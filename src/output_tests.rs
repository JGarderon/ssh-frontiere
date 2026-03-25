#[cfg(test)]
mod tests {
    use crate::output::*;

    #[test]
    fn response_rejected_has_null_output() {
        let resp = Response::rejected("unknown foo", "unknown action 'foo'", EXIT_REJECTED);
        assert_eq!(resp.command, "unknown foo");
        assert_eq!(resp.status_code, EXIT_REJECTED);
        assert!(resp.stdout.is_none());
        assert!(resp.stderr.is_none());
    }

    #[test]
    fn response_config_error() {
        let resp = Response::rejected("", "config file not found", EXIT_CONFIG_ERROR);
        assert_eq!(resp.status_code, EXIT_CONFIG_ERROR);
        assert_eq!(resp.status_message, "config file not found");
    }

    #[test]
    fn response_timeout() {
        let resp = Response::timeout("mastodon backup-full", 300);
        assert_eq!(resp.command, "mastodon backup-full");
        assert_eq!(resp.status_code, EXIT_TIMEOUT);
        assert!(resp.status_message.contains("timeout"));
        assert!(resp.status_message.contains("300s"));
    }

    #[test]
    fn response_insufficient_level() {
        let resp = Response::rejected(
            "forgejo deploy latest",
            "insufficient level (ops required, read granted)",
            EXIT_INSUFFICIENT_LEVEL,
        );
        assert_eq!(resp.command, "forgejo deploy latest");
        assert_eq!(resp.status_code, EXIT_INSUFFICIENT_LEVEL);
    }

    #[test]
    fn response_serializes_to_5_fields() {
        let resp = Response::streamed("test echo", 0);
        let json = resp.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert!(parsed.get("command").is_some());
        assert!(parsed.get("status_code").is_some());
        assert!(parsed.get("status_message").is_some());
        assert!(parsed.get("stdout").is_some());
        assert!(parsed.get("stderr").is_some());
        // Exactly 5 top-level keys
        assert_eq!(parsed.as_object().expect("object").len(), 5);
        assert_eq!(parsed["command"].as_str(), Some("test echo"));
    }

    #[test]
    fn response_null_vs_empty_string() {
        // Rejected: null stdout/stderr
        let rejected = Response::rejected("test cmd", "denied", EXIT_REJECTED);
        let json = rejected.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert!(parsed["stdout"].is_null());
        assert!(parsed["stderr"].is_null());
        assert_eq!(parsed["command"].as_str(), Some("test cmd"));
    }

    #[test]
    fn response_streamed_has_null_output() {
        let resp = Response::streamed("gitlab backup-full", 0);
        assert_eq!(resp.command, "gitlab backup-full");
        assert_eq!(resp.status_code, 0);
        assert_eq!(resp.status_message, "executed");
        assert!(resp.stdout.is_none());
        assert!(resp.stderr.is_none());
    }

    #[test]
    fn response_streamed_nonzero_code() {
        let resp = Response::streamed("gitlab backup-full", 1);
        assert_eq!(resp.status_code, 1);
        assert_eq!(resp.status_message, "executed");
        assert!(resp.stdout.is_none());
    }

    #[test]
    fn stderr_message_format() {
        let msg = stderr_message("rejected", "unknown action 'foo'");
        assert_eq!(msg, "ssh-frontiere: rejected: unknown action 'foo'");
    }

    // --- Phase 9 : EXIT_STDIN_ERROR ---

    #[test]
    fn exit_codes_complete() {
        assert_eq!(EXIT_REJECTED, 128);
        assert_eq!(EXIT_CONFIG_ERROR, 129);
        assert_eq!(EXIT_TIMEOUT, 130);
        assert_eq!(EXIT_INSUFFICIENT_LEVEL, 131);
        assert_eq!(EXIT_PROTOCOL_ERROR, 132);
        assert_eq!(EXIT_STDIN_ERROR, 133);
    }
}
