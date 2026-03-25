use serde::Serialize;

// Codes de sortie reserves (ADR 0003)
pub(crate) const EXIT_REJECTED: i32 = 128;
pub(crate) const EXIT_CONFIG_ERROR: i32 = 129;
pub(crate) const EXIT_TIMEOUT: i32 = 130;
pub(crate) const EXIT_INSUFFICIENT_LEVEL: i32 = 131;
pub(crate) const EXIT_PROTOCOL_ERROR: i32 = 132;
/// ADR 0012 D3 — child process closed stdin before body was fully written
pub(crate) const EXIT_STDIN_ERROR: i32 = 133;

/// Reponse JSON structuree a 5 champs (ADR 0003 v2, alignement 003)
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Response {
    pub(crate) command: String,
    pub(crate) status_code: i32,
    pub(crate) status_message: String,
    pub(crate) stdout: Option<String>,
    pub(crate) stderr: Option<String>,
}

impl Response {
    /// Commande rejetee (stdout/stderr sont null)
    pub(crate) fn rejected(command: &str, reason: &str, code: i32) -> Self {
        Response {
            command: command.to_string(),
            status_code: code,
            status_message: reason.to_string(),
            stdout: None,
            stderr: None,
        }
    }

    /// Commande executee avec streaming (stdout/stderr deja envoyes via >> / >>!)
    pub(crate) fn streamed(command: &str, code: i32) -> Self {
        Response {
            command: command.to_string(),
            status_code: code,
            status_message: "executed".to_string(),
            stdout: None,
            stderr: None,
        }
    }

    /// Timeout (stdout/stderr null)
    pub(crate) fn timeout(command_desc: &str, timeout_secs: u64) -> Self {
        Response {
            command: command_desc.to_string(),
            status_code: EXIT_TIMEOUT,
            status_message: format!("timeout after {timeout_secs}s: {command_desc}"),
            stdout: None,
            stderr: None,
        }
    }

    pub(crate) fn to_json(&self) -> String {
        // INVARIANT: Response est toujours serializable
        serde_json::to_string(self).expect("Response serialization cannot fail")
    }
}

/// Formate un message stderr court (ADR 0003)
pub(crate) fn stderr_message(msg_type: &str, detail: &str) -> String {
    format!("ssh-frontiere: {msg_type}: {detail}")
}
