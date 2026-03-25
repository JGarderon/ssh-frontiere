pub mod crypto;

// --- Modules exposed for fuzz testing (cargo-fuzz sets --cfg fuzzing) ---

#[cfg(fuzzing)]
mod auth;
#[cfg(fuzzing)]
mod chain_exec;
#[cfg(fuzzing)]
mod chain_parser;
#[cfg(fuzzing)]
mod config;
#[cfg(fuzzing)]
mod discovery;
#[cfg(fuzzing)]
mod dispatch;
#[cfg(fuzzing)]
mod executor;
#[cfg(fuzzing)]
mod logging;
#[cfg(fuzzing)]
mod orchestrator;
#[cfg(fuzzing)]
mod output;
#[cfg(fuzzing)]
mod protocol;

/// Thin public wrappers for fuzz harnesses (only compiled with cargo-fuzz)
#[cfg(fuzzing)]
pub mod fuzz_helpers {
    /// Parse a single protocol line and discard the result.
    /// Exercises the protocol line parser with arbitrary input.
    pub fn parse_line(line: &str) -> Result<(), String> {
        crate::protocol::parse_line(line)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Parse a raw command string (tokenize + validate) and discard the result.
    /// Exercises the command parser with arbitrary input.
    pub fn parse_command(raw: &str) -> Result<(), String> {
        crate::dispatch::parse_command(raw)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Read body in default mode (until `\n.\n`) from arbitrary input.
    pub fn read_body_default(
        reader: &mut impl std::io::BufRead,
        max_size: usize,
    ) -> Result<String, String> {
        crate::protocol::read_body(reader, &crate::protocol::BodyMode::Default, max_size)
            .map_err(|e| e.to_string())
    }

    /// Read body in size mode from arbitrary input.
    pub fn read_body_size(
        reader: &mut impl std::io::BufRead,
        n: usize,
        max_size: usize,
    ) -> Result<String, String> {
        crate::protocol::read_body(reader, &crate::protocol::BodyMode::Size(n), max_size)
            .map_err(|e| e.to_string())
    }

    /// Transpose command with arbitrary args.
    pub fn transpose_command(
        template: &str,
        domain: &str,
        args: &std::collections::HashMap<String, String>,
    ) -> Vec<String> {
        crate::dispatch::transpose_command(template, domain, args)
    }

    /// Parse a TOML config string and discard the result.
    pub fn parse_config(toml_content: &str) -> Result<(), String> {
        crate::config::Config::from_str(toml_content)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
