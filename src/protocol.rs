use crate::config::Config;
use std::io::{BufRead, Write};

const MAX_LINE_LEN: usize = 4096;
const VERSION: &str = env!("CARGO_PKG_VERSION");

// --- Protocol types (ADR 0006 §1) ---

/// A parsed line from the protocol
#[derive(Debug, PartialEq)]
pub(crate) enum ProtocolLine {
    Configure(Directive),
    Comment(String),
    Text(String),
    EndOfBlock,
    EmptyLine,
}

/// Body delimitation mode (ADR 0012 D1)
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BodyMode {
    /// `+body` — read until `\n.\n`
    Default,
    /// `+body size=N` — read exactly N bytes
    Size(usize),
    /// `+body stop="X"` — read until line X
    Stop(String),
    /// `+body size=N stop="X"` — first reached terminates
    SizeAndStop(usize, String),
}

/// A parsed + directive
#[derive(Debug, PartialEq)]
pub(crate) enum Directive {
    Capabilities(Vec<String>),
    Challenge { nonce: String },
    Auth { token: String, proof: String },
    Session,
    Body(BodyMode),
    Unknown(String),
}

/// Result of reading the client header phase
pub(crate) struct HeadersResult {
    pub(crate) auth_token: Option<String>,
    pub(crate) auth_proof: Option<String>,
    pub(crate) session_mode: bool,
    pub(crate) comments: Vec<String>,
    /// ADR 0012 D1 — body mode declared by client via `+body`
    pub(crate) body_mode: Option<BodyMode>,
}

/// Protocol error types
#[derive(Debug)]
pub(crate) enum ProtocolError {
    InvalidLine(String),
    LineTooLong(usize),
    UnexpectedEof,
    IoError(String),
    /// ADR 0012 D2 — body exceeds `max_body_size`
    BodyTooLarge {
        size: usize,
        max: usize,
    },
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolError::InvalidLine(line) => write!(f, "invalid protocol line: {line}"),
            ProtocolError::LineTooLong(len) => {
                write!(f, "line too long ({len} chars, max {MAX_LINE_LEN})")
            }
            ProtocolError::UnexpectedEof => write!(f, "unexpected EOF"),
            ProtocolError::IoError(e) => write!(f, "I/O error: {e}"),
            ProtocolError::BodyTooLarge { size, max } => {
                write!(f, "body too large ({size} bytes, max {max})")
            }
        }
    }
}

impl std::error::Error for ProtocolError {}

// --- Line parser ---

/// Parse a single protocol line
#[must_use = "parsing result must be checked"]
pub(crate) fn parse_line(line: &str) -> Result<ProtocolLine, ProtocolError> {
    if line.len() > MAX_LINE_LEN {
        return Err(ProtocolError::LineTooLong(line.len()));
    }

    let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');

    if trimmed.is_empty() {
        return Ok(ProtocolLine::EmptyLine);
    }

    if trimmed == "." {
        return Ok(ProtocolLine::EndOfBlock);
    }

    if let Some(content) = trimmed.strip_prefix("+ ") {
        Ok(ProtocolLine::Configure(parse_directive(content)))
    } else if let Some(content) = trimmed.strip_prefix("# ") {
        Ok(ProtocolLine::Comment(content.to_string()))
    } else if trimmed == "#" {
        Ok(ProtocolLine::Comment(String::new()))
    } else {
        Ok(ProtocolLine::Text(trimmed.to_string()))
    }
}

/// Parse a + directive content
fn parse_directive(content: &str) -> Directive {
    if let Some(caps) = content.strip_prefix("capabilities ") {
        let items: Vec<String> = caps.split(", ").map(|s| s.trim().to_string()).collect();
        return Directive::Capabilities(items);
    }

    if let Some(rest) = content.strip_prefix("challenge ") {
        if let Some(nonce) = rest.strip_prefix("nonce=") {
            return Directive::Challenge {
                nonce: nonce.to_string(),
            };
        }
    }

    if let Some(rest) = content.strip_prefix("auth ") {
        if let (Some(token), Some(proof)) = (parse_kv(rest, "token"), parse_kv(rest, "proof")) {
            return Directive::Auth { token, proof };
        }
    }

    if content == "session keepalive" {
        return Directive::Session;
    }

    // ADR 0012 D1 — body directive
    if content == "body" {
        return Directive::Body(BodyMode::Default);
    }
    if let Some(params) = content.strip_prefix("body ") {
        if let Some(mode) = parse_body_params(params) {
            return Directive::Body(mode);
        }
    }

    Directive::Unknown(content.to_string())
}

/// Parse body parameters: `size=N`, `stop="X"`, or both
fn parse_body_params(params: &str) -> Option<BodyMode> {
    let mut size: Option<usize> = None;
    let mut stop: Option<String> = None;

    let mut rest = params.trim();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("size=") {
            let end = after.find(' ').unwrap_or(after.len());
            size = Some(after[..end].parse().ok()?);
            rest = after[end..].trim_start();
        } else if let Some(after) = rest.strip_prefix("stop=\"") {
            let end = after.find('"')?;
            stop = Some(after[..end].to_string());
            rest = after[end + 1..].trim_start();
        } else {
            return None;
        }
    }

    match (size, stop) {
        (Some(s), Some(st)) => Some(BodyMode::SizeAndStop(s, st)),
        (Some(s), None) => Some(BodyMode::Size(s)),
        (None, Some(st)) => Some(BodyMode::Stop(st)),
        (None, None) => None,
    }
}

/// Extract key=value from "key1=val1 key2=val2" format
fn parse_kv(input: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    for part in input.split_whitespace() {
        if let Some(val) = part.strip_prefix(&prefix) {
            return Some(val.to_string());
        }
    }
    None
}

// --- Body reading (ADR 0012 D1) ---

/// Read a body from the reader according to the given mode.
///
/// # Errors
/// Returns `ProtocolError::BodyTooLarge` if body exceeds `max_size`.
/// Returns `ProtocolError::UnexpectedEof` if stream ends before body is complete.
pub(crate) fn read_body(
    reader: &mut impl BufRead,
    mode: &BodyMode,
    max_size: usize,
) -> Result<String, ProtocolError> {
    match mode {
        BodyMode::Default => read_body_line_terminated(reader, ".", max_size),
        BodyMode::Size(n) => read_body_exact(reader, *n, max_size),
        BodyMode::Stop(stop) => read_body_line_terminated(reader, stop, max_size),
        BodyMode::SizeAndStop(n, stop) => read_body_combined(reader, *n, stop, max_size),
    }
}

/// Read body line by line until a terminator line (`.` or custom stop)
fn read_body_line_terminated(
    reader: &mut impl BufRead,
    terminator: &str,
    max_size: usize,
) -> Result<String, ProtocolError> {
    let mut body = String::new();

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return Err(ProtocolError::UnexpectedEof),
            Ok(_) => {}
            Err(e) => return Err(ProtocolError::IoError(e.to_string())),
        }

        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        if trimmed == terminator {
            return Ok(body);
        }

        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str(trimmed);

        if body.len() > max_size {
            return Err(ProtocolError::BodyTooLarge {
                size: body.len(),
                max: max_size,
            });
        }
    }
}

/// Read exactly N bytes from the reader
fn read_body_exact(
    reader: &mut impl BufRead,
    n: usize,
    max_size: usize,
) -> Result<String, ProtocolError> {
    if n > max_size {
        return Err(ProtocolError::BodyTooLarge {
            size: n,
            max: max_size,
        });
    }
    if n == 0 {
        return Ok(String::new());
    }

    let mut buf = vec![0u8; n];
    reader.read_exact(&mut buf).map_err(|e| match e.kind() {
        std::io::ErrorKind::UnexpectedEof => ProtocolError::UnexpectedEof,
        _ => ProtocolError::IoError(e.to_string()),
    })?;

    String::from_utf8(buf).map_err(|e| ProtocolError::IoError(format!("invalid UTF-8: {e}")))
}

/// Read body with combined size + stop: first reached terminates
fn read_body_combined(
    reader: &mut impl BufRead,
    max_bytes: usize,
    stop: &str,
    max_size: usize,
) -> Result<String, ProtocolError> {
    let effective_max = max_bytes.min(max_size);
    let mut body = String::new();

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return Err(ProtocolError::UnexpectedEof),
            Ok(_) => {}
            Err(e) => return Err(ProtocolError::IoError(e.to_string())),
        }

        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        if trimmed == stop {
            return Ok(body);
        }

        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str(trimmed);

        if body.len() >= effective_max {
            body.truncate(effective_max);
            return Ok(body);
        }
    }
}

// --- Banner generation (ADR 0006 §2) ---

/// Write the server banner to the given writer
#[must_use = "I/O result must be checked"]
pub(crate) fn write_banner(
    writer: &mut impl Write,
    config: &Config,
    nonce: Option<&str>,
    session_id: Option<&str>,
    expose_session_id: bool,
) -> Result<(), String> {
    writeln!(writer, "#> ssh-frontiere {VERSION}")
        .map_err(|e| format!("banner write error: {e}"))?;

    // Capabilities: always include session, help. Include rbac only if auth configured.
    let mut caps = Vec::new();
    if config.auth.is_some() {
        caps.push("rbac");
    }
    caps.push("session");
    caps.push("help");
    caps.push("body");
    writeln!(writer, "+> capabilities {}", caps.join(", "))
        .map_err(|e| format!("banner write error: {e}"))?;

    // Challenge nonce (only if auth configured)
    if let Some(nonce_hex) = nonce {
        writeln!(writer, "+> challenge nonce={nonce_hex}")
            .map_err(|e| format!("banner write error: {e}"))?;
    }

    // Session ID (only if expose_session_id is true and session_id is Some)
    if expose_session_id {
        if let Some(sid) = session_id {
            writeln!(writer, "+> session {sid}").map_err(|e| format!("banner write error: {e}"))?;
        }
    }

    writeln!(writer, "#> type \"help\" for available commands")
        .map_err(|e| format!("banner write error: {e}"))?;

    writer
        .flush()
        .map_err(|e| format!("banner flush error: {e}"))?;
    Ok(())
}

// --- Header reading (ADR 0006 §2) ---

/// Read client headers from stdin until a Text line (first command) or `EndOfBlock` (end of connection)
#[must_use = "headers result must be checked"]
pub(crate) fn read_headers(
    reader: &mut impl BufRead,
) -> Result<(HeadersResult, Option<String>), ProtocolError> {
    let mut result = HeadersResult {
        auth_token: None,
        auth_proof: None,
        session_mode: false,
        comments: Vec::new(),
        body_mode: None,
    };

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return Err(ProtocolError::UnexpectedEof),
            Ok(_) => {}
            Err(e) => return Err(ProtocolError::IoError(e.to_string())),
        }

        match parse_line(&line)? {
            ProtocolLine::Configure(Directive::Auth { token, proof }) => {
                result.auth_token = Some(token);
                result.auth_proof = Some(proof);
            }
            ProtocolLine::Configure(Directive::Session) => {
                result.session_mode = true;
            }
            ProtocolLine::Configure(Directive::Body(mode)) => {
                result.body_mode = Some(mode);
            }
            ProtocolLine::Comment(text) => {
                result.comments.push(text);
            }
            ProtocolLine::Text(text) => {
                // First command line — return it to the caller
                return Ok((result, Some(text)));
            }
            ProtocolLine::EndOfBlock => {
                // "." alone = end of connection
                return Ok((result, None));
            }
            // Unknown directives, capabilities, challenge from client, empty lines: ignored
            ProtocolLine::Configure(_) | ProtocolLine::EmptyLine => {}
        }
    }
}

// --- Command block reading ---

/// Read a command block from the reader.
/// The `first_line` is the first Text line already read.
/// Returns the full command block as a single string, or None if `first_line` is None (end of connection).
#[must_use = "command block result must be checked"]
pub(crate) fn read_command_block(
    reader: &mut impl BufRead,
    first_line: Option<String>,
) -> Result<Option<String>, ProtocolError> {
    let Some(first) = first_line else {
        return Ok(None); // "." sans commande = fin de connexion
    };

    let mut block = first;

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return Err(ProtocolError::UnexpectedEof),
            Ok(_) => {}
            Err(e) => return Err(ProtocolError::IoError(e.to_string())),
        }

        match parse_line(&line)? {
            ProtocolLine::EndOfBlock => return Ok(Some(block)),
            ProtocolLine::Text(text) => {
                block.push('\n');
                block.push_str(&text);
            }
            ProtocolLine::EmptyLine => {
                // Ligne vide dans un bloc commande = saut de ligne (= ";")
                block.push('\n');
            }
            ProtocolLine::Configure(_) | ProtocolLine::Comment(_) => {
                return Err(ProtocolError::InvalidLine(
                    "expected command text or '.', got header/comment".to_string(),
                ))
            }
        }
    }
}

/// Write a >>> response line (ADR 0011 — final JSON response)
#[must_use = "I/O result must be checked"]
pub(crate) fn write_response(writer: &mut impl Write, json: &str) -> Result<(), String> {
    writeln!(writer, ">>> {json}").map_err(|e| format!("response write error: {e}"))?;
    writer
        .flush()
        .map_err(|e| format!("response flush error: {e}"))?;
    Ok(())
}

/// Write a >> stdout line (ADR 0011 — streaming)
#[must_use = "I/O result must be checked"]
pub(crate) fn write_stdout_line(writer: &mut impl Write, line: &str) -> Result<(), String> {
    writeln!(writer, ">> {line}").map_err(|e| format!("stdout line write error: {e}"))?;
    writer
        .flush()
        .map_err(|e| format!("stdout line flush error: {e}"))?;
    Ok(())
}

/// Write a >>! stderr line (ADR 0011 — streaming)
#[must_use = "I/O result must be checked"]
pub(crate) fn write_stderr_line(writer: &mut impl Write, line: &str) -> Result<(), String> {
    writeln!(writer, ">>! {line}").map_err(|e| format!("stderr line write error: {e}"))?;
    writer
        .flush()
        .map_err(|e| format!("stderr line flush error: {e}"))?;
    Ok(())
}

/// Write a #> comment line from the server
#[must_use = "I/O result must be checked"]
pub(crate) fn write_comment(writer: &mut impl Write, text: &str) -> Result<(), String> {
    writeln!(writer, "#> {text}").map_err(|e| format!("comment write error: {e}"))?;
    writer
        .flush()
        .map_err(|e| format!("comment flush error: {e}"))?;
    Ok(())
}

// --- Session loop (ADR 0006 §6) ---

/// Read the next input in session mode
/// Returns either a command block (with optional body), an auth update, a comment, or signals end/eof
#[derive(Debug)]
pub(crate) enum SessionInput {
    CommandBlock { block: String, body: Option<String> },
    Auth { token: String, proof: String },
    Comment(String),
    EndOfConnection,
    Eof,
}

/// Default body size limit for session reads (64 KB — ADR 0012 plan decision P1)
pub(crate) const DEFAULT_MAX_BODY_SIZE: usize = 65536;

/// Read the next input in session mode.
/// Between command blocks, the server accepts headers (+ and #) or the start of a new block (Text)
/// or the end (EndOfBlock/EOF).
/// If `+body` appears before a command, the body is read after the command block terminator.
#[must_use = "session input result must be checked"]
pub(crate) fn read_session_input(reader: &mut impl BufRead) -> Result<SessionInput, ProtocolError> {
    let mut pending_body_mode: Option<BodyMode> = None;

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return Ok(SessionInput::Eof),
            Ok(_) => {}
            Err(e) => return Err(ProtocolError::IoError(e.to_string())),
        }

        match parse_line(&line)? {
            ProtocolLine::Configure(Directive::Auth { token, proof }) => {
                return Ok(SessionInput::Auth { token, proof });
            }
            ProtocolLine::Configure(Directive::Body(mode)) => {
                pending_body_mode = Some(mode);
            }
            ProtocolLine::Comment(text) => return Ok(SessionInput::Comment(text)),
            ProtocolLine::Text(text) => {
                // Start of a command block — read the rest
                match read_command_block(reader, Some(text))? {
                    Some(block) => {
                        let body = if let Some(ref mode) = pending_body_mode {
                            Some(read_body(reader, mode, DEFAULT_MAX_BODY_SIZE)?)
                        } else {
                            None
                        };
                        return Ok(SessionInput::CommandBlock { block, body });
                    }
                    None => return Ok(SessionInput::EndOfConnection),
                }
            }
            ProtocolLine::EndOfBlock => {
                // "." alone = end of connection
                return Ok(SessionInput::EndOfConnection);
            }
            ProtocolLine::Configure(_) | ProtocolLine::EmptyLine => {}
        }
    }
}
