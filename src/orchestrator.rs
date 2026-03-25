use crate::auth::AuthContext;
use crate::chain_exec;
use crate::config::Config;
use crate::crypto;
use crate::dispatch::Identity;
use crate::logging::{self, LogEntry};
use crate::output::{
    stderr_message, Response, EXIT_CONFIG_ERROR, EXIT_PROTOCOL_ERROR, EXIT_REJECTED,
};
use crate::protocol::{
    self, write_banner, write_comment, write_response, ProtocolError, SessionInput,
};

// Convention: `let _ =` on write_comment/write_response/write_log calls is intentional.
// Protocol output is best-effort — the SSH client may disconnect at any time.
// Log writes are best-effort — must not block or panic the dispatcher.
const DEFAULT_CONFIG_PATH: &str = "/etc/ssh-frontiere/config.toml";

/// En mode production, masque les details internes. En mode --diagnostic, affiche tout.
fn opaque_error(diagnostic: bool, detail: &str) -> String {
    if diagnostic {
        detail.to_string()
    } else {
        "service unavailable".to_string()
    }
}

/// Main protocol orchestrator — reads config, runs banner, headers, auth, command, session.
pub(crate) fn run(args: &[String]) -> i32 {
    let diagnostic = args.iter().any(|a| a == "--diagnostic");
    let check_config = args.iter().any(|a| a == "--check-config");

    // 1. Charger la configuration
    let config_path = resolve_config_path(args);
    let config = match Config::from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            let detail = e.to_string();
            eprintln!(
                "{}",
                stderr_message("error", &opaque_error(diagnostic, &detail))
            );
            return EXIT_CONFIG_ERROR;
        }
    };

    // --check-config : validation dry-run, exit immédiat sans démarrer le protocole
    if check_config {
        println!("Config OK: {config_path}");
        return 0;
    }

    // 2. Construire l'identite de base (--level via authorized_keys)
    let ssh_client = std::env::var("SSH_CLIENT").ok();
    let identity = build_identity(args, ssh_client.as_deref());

    // 3-4. Setup connection: nonce + banner
    let stdout = std::io::stdout();
    let mut writer = std::io::BufWriter::new(stdout.lock());
    let (nonce, session_id) = match setup_connection(&config, diagnostic, &mut writer) {
        Ok(result) => result,
        Err(code) => return code,
    };

    // 5. Lire les entetes client depuis stdin
    let stdin = std::io::stdin();
    let mut reader = std::io::BufReader::new(stdin.lock());

    let (headers, first_cmd_line) = match protocol::read_headers(&mut reader) {
        Ok(h) => h,
        Err(ProtocolError::UnexpectedEof) => return 0,
        Err(e) => {
            let msg = opaque_error(diagnostic, &format!("protocol error: {e}"));
            let _ = write_comment(&mut writer, &msg);
            eprintln!("{}", stderr_message("protocol", &msg));
            return EXIT_PROTOCOL_ERROR;
        }
    };

    // 6. Log les commentaires client si configure
    log_client_comments(&config, &headers.comments);

    // 7. Creer le contexte d'auth et valider +auth si present
    let mut auth_ctx = AuthContext::new(identity.level, config.global.max_auth_failures);

    if let (Some(ref token), Some(ref proof)) = (&headers.auth_token, &headers.auth_proof) {
        let nonce_slice = nonce.as_ref().map(<[u8; 16]>::as_slice);
        if let Some(code) = handle_auth_result(
            &mut auth_ctx,
            &config,
            token,
            proof,
            nonce_slice,
            &identity,
            &mut writer,
        ) {
            ban_if_configured(&config, ssh_client.as_deref());
            return code;
        }
    }

    // 8. Executer la premiere commande (help ou bloc)
    let exit_code = if first_cmd_line.as_deref() == Some("help") {
        let _ = protocol::read_command_block(&mut reader, first_cmd_line);
        emit_help(&config, &auth_ctx, &identity, &mut writer);
        0
    } else {
        read_and_execute(
            &config,
            &identity,
            &auth_ctx,
            &session_id,
            diagnostic,
            &headers,
            first_cmd_line,
            &mut reader,
            &mut writer,
        )
    };

    // 9. Mode session : boucle si +session keepalive
    if headers.session_mode {
        let mut ctx = SessionContext {
            config: &config,
            identity: &identity,
            auth_ctx: &mut auth_ctx,
            nonce,
            ssh_client: ssh_client.as_deref(),
            session_id: &session_id,
        };
        run_session_loop(&mut ctx, &mut reader, &mut writer);
    }

    exit_code
}

/// Setup nonce + banner. Returns (`nonce`, `session_id`) or exit code on error.
fn setup_connection(
    config: &Config,
    diagnostic: bool,
    writer: &mut impl std::io::Write,
) -> Result<(Option<[u8; 16]>, String), i32> {
    let has_auth_tokens = config.auth.as_ref().is_some_and(|a| !a.tokens.is_empty());
    let challenge_nonce = config.auth.as_ref().is_some_and(|a| a.challenge_nonce);
    let nonce = if has_auth_tokens && challenge_nonce {
        match crypto::generate_nonce() {
            Ok(n) => Some(n),
            Err(e) => {
                eprintln!(
                    "{}",
                    stderr_message(
                        "error",
                        &opaque_error(diagnostic, &format!("nonce generation: {e}")),
                    )
                );
                return Err(EXIT_CONFIG_ERROR);
            }
        }
    } else {
        None
    };

    let nonce_hex = nonce.as_ref().map(|n| crypto::hex_encode(n));
    let session_id = generate_session_id();

    if let Err(e) = write_banner(
        writer,
        config,
        nonce_hex.as_deref(),
        Some(&session_id),
        config.global.expose_session_id,
    ) {
        eprintln!("{}", stderr_message("error", &opaque_error(diagnostic, &e)));
        return Err(EXIT_CONFIG_ERROR);
    }

    Ok((nonce, session_id))
}

/// Emit help text to writer
fn emit_help(
    config: &Config,
    auth_ctx: &AuthContext,
    identity: &Identity,
    writer: &mut impl std::io::Write,
) {
    let effective = identity.with_level(auth_ctx.effective_level);
    chain_exec::write_help_text(
        config,
        &["help".to_string()],
        &effective,
        &auth_ctx.effective_tags,
        writer,
    );
}

/// Read command block + body, execute, return exit code
#[allow(clippy::too_many_arguments)]
fn read_and_execute(
    config: &Config,
    identity: &Identity,
    auth_ctx: &AuthContext,
    session_id: &str,
    diagnostic: bool,
    headers: &protocol::HeadersResult,
    first_cmd_line: Option<String>,
    reader: &mut impl std::io::BufRead,
    writer: &mut impl std::io::Write,
) -> i32 {
    let cmd_block = match protocol::read_command_block(reader, first_cmd_line) {
        Ok(Some(block)) => block,
        Ok(None) | Err(ProtocolError::UnexpectedEof) => return 0,
        Err(e) => {
            let msg = opaque_error(diagnostic, &format!("protocol error: {e}"));
            let _ = write_comment(writer, &msg);
            eprintln!("{}", stderr_message("protocol", &msg));
            return EXIT_PROTOCOL_ERROR;
        }
    };

    let body = if let Some(ref mode) = headers.body_mode {
        match protocol::read_body(reader, mode, protocol::DEFAULT_MAX_BODY_SIZE) {
            Ok(b) => Some(b),
            Err(e) => {
                let msg = opaque_error(diagnostic, &format!("body error: {e}"));
                let _ = write_comment(writer, &msg);
                return EXIT_PROTOCOL_ERROR;
            }
        }
    } else {
        None
    };

    execute_command_block(
        config,
        &cmd_block,
        identity,
        auth_ctx,
        session_id,
        writer,
        body.as_deref(),
    )
}

/// Handle auth validation result. Returns `Some(exit_code)` if lockout, `None` to continue.
fn handle_auth_result(
    auth_ctx: &mut AuthContext,
    config: &Config,
    token: &str,
    proof: &str,
    nonce_slice: Option<&[u8]>,
    identity: &Identity,
    writer: &mut impl std::io::Write,
) -> Option<i32> {
    match auth_ctx.validate_auth(config, token, proof, nonce_slice) {
        Ok(level) => {
            let _ = write_comment(writer, &format!("auth ok, level={level}"));
            None
        }
        Err(msg) => {
            let _ = write_comment(writer, &msg);
            if auth_ctx.is_locked_out() {
                let _ = write_comment(writer, "session terminated");
                log_event(config, "auth_lockout", identity, None, None, Some(&msg));
                Some(EXIT_PROTOCOL_ERROR)
            } else {
                None
            }
        }
    }
}

/// Handle auth result in session loop. Returns true if session should terminate.
fn handle_session_auth_result(
    ctx: &mut SessionContext<'_>,
    token: &str,
    proof: &str,
    writer: &mut impl std::io::Write,
) -> bool {
    let nonce_slice = ctx.nonce.as_ref().map(<[u8; 16]>::as_slice);
    match ctx
        .auth_ctx
        .validate_auth(ctx.config, token, proof, nonce_slice)
    {
        Ok(level) => {
            let _ = write_comment(writer, &format!("auth ok, level={level}"));
            // Regenerate nonce to prevent replay (TODO-016)
            if ctx.nonce.is_some() {
                if let Ok(new_nonce) = crypto::generate_nonce() {
                    let hex = crypto::hex_encode(&new_nonce);
                    let _ = write_comment(writer, &format!("new challenge nonce={hex}"));
                    ctx.nonce = Some(new_nonce);
                }
            }
            false
        }
        Err(msg) => {
            let _ = write_comment(writer, &msg);
            if ctx.auth_ctx.is_locked_out() {
                let _ = write_comment(writer, "session terminated");
                log_event(
                    ctx.config,
                    "auth_lockout",
                    ctx.identity,
                    None,
                    None,
                    Some(&msg),
                );
                ban_if_configured(ctx.config, ctx.ssh_client);
                true
            } else {
                false
            }
        }
    }
}

/// Genere un UUID v4 a partir de /dev/urandom (zero dependance)
fn generate_session_id() -> String {
    match crypto::generate_nonce() {
        Ok(bytes) => {
            // Format UUID v4 : xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
            // PANIC-SAFE: hex_encode(16 bytes) always produces exactly 32 hex chars; all slices are within 0..32
            let hex = crypto::hex_encode(&bytes);
            format!(
                "{}-{}-4{}-{}{}-{}",
                &hex[0..8],
                &hex[8..12],
                &hex[13..16],
                match u8::from_str_radix(&hex[16..17], 16) {
                    Ok(v) => format!("{:x}", (v & 0x3) | 0x8),
                    Err(_) => "8".to_string(),
                },
                &hex[17..20],
                &hex[20..32],
            )
        }
        Err(e) => {
            eprintln!(
                "{}",
                stderr_message("warning", &format!("session ID generation failed: {e}"))
            );
            "00000000-0000-4000-8000-000000000000".to_string()
        }
    }
}

/// Execute un bloc de commande (potentiellement avec operateurs de chainage)
fn execute_command_block(
    config: &Config,
    block: &str,
    identity: &Identity,
    auth_ctx: &AuthContext,
    session_id: &str,
    writer: &mut impl std::io::Write,
    body: Option<&str>,
) -> i32 {
    match chain_exec::parse_block(block) {
        Ok(node) => {
            chain_exec::execute_chain(&node, config, identity, auth_ctx, session_id, writer, body)
        }
        Err(e) => {
            let msg = e.to_string();
            let resp = Response::rejected(block, &format!("rejected: {msg}"), EXIT_REJECTED);
            let _ = write_response(writer, &resp.to_json());
            eprintln!("{}", stderr_message("rejected", &msg));
            EXIT_REJECTED
        }
    }
}

/// Session context — groups the 6 state parameters of `run_session_loop` (I-06)
pub(crate) struct SessionContext<'a> {
    pub(crate) config: &'a Config,
    pub(crate) identity: &'a Identity,
    pub(crate) auth_ctx: &'a mut AuthContext,
    pub(crate) nonce: Option<[u8; 16]>,
    pub(crate) ssh_client: Option<&'a str>,
    pub(crate) session_id: &'a str,
}

/// Execute ban command if configured and SSH client IP is available
fn ban_if_configured(config: &Config, ssh_client: Option<&str>) {
    if let Some(client) = ssh_client {
        let ip = extract_ip_from_ssh_client(client);
        execute_ban_command(&config.global.ban_command, ip);
    }
}

/// Boucle de session v2 (ADR 0006 §9)
fn run_session_loop(
    ctx: &mut SessionContext<'_>,
    reader: &mut impl std::io::BufRead,
    writer: &mut impl std::io::Write,
) {
    let timeout_deadline = std::time::Instant::now()
        + std::time::Duration::from_secs(ctx.config.global.timeout_session);

    loop {
        if std::time::Instant::now() >= timeout_deadline {
            let _ = write_comment(writer, "session timeout");
            return;
        }

        match protocol::read_session_input(reader) {
            Ok(SessionInput::CommandBlock { block, body }) => {
                if block.trim() == "help" {
                    let effective = ctx.identity.with_level(ctx.auth_ctx.effective_level);
                    chain_exec::write_help_text(
                        ctx.config,
                        &["help".to_string()],
                        &effective,
                        &ctx.auth_ctx.effective_tags,
                        writer,
                    );
                } else {
                    execute_command_block(
                        ctx.config,
                        &block,
                        ctx.identity,
                        ctx.auth_ctx,
                        ctx.session_id,
                        writer,
                        body.as_deref(),
                    );
                }
            }
            Ok(SessionInput::Auth { token, proof }) => {
                if handle_session_auth_result(ctx, &token, &proof, writer) {
                    return;
                }
            }
            Ok(SessionInput::Comment(text)) => {
                if ctx.config.global.log_comments {
                    let entry = LogEntry::new("client_comment").with_reason(&text);
                    let _ = logging::write_log(&ctx.config.global.log_file, &entry);
                }
            }
            Ok(SessionInput::EndOfConnection) => {
                let _ = write_comment(writer, "session closed");
                return;
            }
            Ok(SessionInput::Eof) => return,
            Err(e) => {
                let _ = write_comment(writer, &format!("protocol error: {e}"));
                return;
            }
        }
    }
}

/// Log client comments if configured
fn log_client_comments(config: &Config, comments: &[String]) {
    if config.global.log_comments {
        for comment in comments {
            let entry = LogEntry::new("client_comment").with_reason(comment);
            let _ = logging::write_log(&config.global.log_file, &entry);
        }
    }
}

fn resolve_config_path(args: &[String]) -> String {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--config" {
            if let Some(path) = args.get(i + 1) {
                return path.clone();
            }
        }
        if let Some(path) = arg.strip_prefix("--config=") {
            return path.to_string();
        }
    }
    std::env::var("SSH_FRONTIERE_CONFIG").unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_string())
}

fn build_identity(args: &[String], ssh_client: Option<&str>) -> Identity {
    let str_args: Vec<&str> = args.iter().map(String::as_str).collect();
    Identity::from_args(&str_args, ssh_client)
}

fn log_event(
    config: &Config,
    event: &str,
    identity: &Identity,
    domain: Option<&str>,
    action: Option<&str>,
    reason: Option<&str>,
) {
    let mut entry = LogEntry::new(event);
    if let Some(d) = domain {
        entry = entry.with_domain(d);
    }
    if let Some(a) = action {
        entry = entry.with_action(a);
    }
    if let Some(r) = reason {
        entry = entry.with_reason(r);
    }
    if let Some(ref client) = identity.ssh_client {
        entry = entry.with_ssh_client(client);
    }
    let _ = logging::write_log(&config.global.log_file, &entry);
}

// --- Ban command execution (moved from protocol.rs — ADR 0006 §7) ---

/// Execute the ban command with {ip} placeholder substitution
pub(crate) fn execute_ban_command(ban_command: &str, ip: &str) {
    if ban_command.is_empty() {
        return;
    }

    // Safety: ensure ip contains no whitespace (use only first word)
    let ip = ip.split_whitespace().next().unwrap_or(ip);

    let parts: Vec<String> = ban_command
        .split_whitespace()
        .map(|p| p.replace("{ip}", ip))
        .collect();

    if parts.is_empty() {
        return;
    }

    // Execute via std::process::Command (no shell — injection safe)
    // Result intentionally discarded — ban is best-effort, must not block the dispatcher
    // PANIC-SAFE: parts.is_empty() checked above with early return
    let _ = std::process::Command::new(&parts[0])
        .args(&parts[1..])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Extract IP from `SSH_CLIENT` ("ip port" format)
pub(crate) fn extract_ip_from_ssh_client(ssh_client: &str) -> &str {
    ssh_client.split_whitespace().next().unwrap_or(ssh_client)
}
