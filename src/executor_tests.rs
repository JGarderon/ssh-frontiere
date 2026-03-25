#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::executor::*;

    // --- Execution (ADR 0011 — streaming) ---

    #[test]
    fn execute_echo_command_streams_stdout() {
        let mut buf: Vec<u8> = Vec::new();
        let result = execute_command(
            &["/bin/echo", "hello"],
            30,
            "test-session",
            &mut buf,
            10_485_760,
            None,
        );
        assert!(matches!(result, ExecuteResult::Exited(0)));
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains(">> hello\n"));
    }

    #[test]
    fn execute_nonzero_exit() {
        let mut buf: Vec<u8> = Vec::new();
        let result = execute_command(
            &["/bin/sh", "-c", "exit 42"],
            30,
            "test-session",
            &mut buf,
            10_485_760,
            None,
        );
        assert!(matches!(result, ExecuteResult::Exited(42)));
    }

    #[test]
    fn execute_timeout() {
        let mut buf: Vec<u8> = Vec::new();
        let result = execute_command(
            &["/bin/sleep", "10"],
            1,
            "test-session",
            &mut buf,
            10_485_760,
            None,
        );
        assert!(matches!(result, ExecuteResult::Timeout));
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains(">>! ssh-frontiere: command timed out"));
    }

    #[test]
    fn execute_timeout_sends_sigterm_first() {
        let marker = format!("/tmp/ssh-frontiere-sigterm-test-{}", std::process::id());
        let _ = std::fs::remove_file(&marker);

        let script = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/trap-sigterm.sh"
        );
        let mut buf: Vec<u8> = Vec::new();
        let result = execute_command(
            &["/bin/bash", script, &marker],
            1,
            "test-session",
            &mut buf,
            10_485_760,
            None,
        );
        assert!(matches!(result, ExecuteResult::Timeout));

        let marker_exists = std::path::Path::new(&marker).exists();
        let _ = std::fs::remove_file(&marker);
        assert!(
            marker_exists,
            "SIGTERM marker file should exist — SIGTERM was not sent before SIGKILL"
        );
    }

    #[test]
    fn execute_env_has_session_id() {
        let mut buf: Vec<u8> = Vec::new();
        let result = execute_command(
            &["/usr/bin/env"],
            10,
            "my-test-uuid-1234",
            &mut buf,
            10_485_760,
            None,
        );
        assert!(matches!(result, ExecuteResult::Exited(0)));
        let output = String::from_utf8(buf).expect("utf8");
        // Chaque ligne d'env est streamee via >>
        assert!(output.contains(">> PATH="));
        assert!(output.contains(">> SSH_FRONTIERE_SESSION=my-test-uuid-1234"));
    }

    #[test]
    fn execute_stderr_streams_with_bang_prefix() {
        let mut buf: Vec<u8> = Vec::new();
        let result = execute_command(
            &["/bin/sh", "-c", "echo error_msg >&2"],
            10,
            "test-session",
            &mut buf,
            10_485_760,
            None,
        );
        assert!(matches!(result, ExecuteResult::Exited(0)));
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains(">>! error_msg\n"));
    }

    #[test]
    fn execute_no_output_no_stream_lines() {
        let mut buf: Vec<u8> = Vec::new();
        let result = execute_command(
            &["/bin/true"],
            10,
            "test-session",
            &mut buf,
            10_485_760,
            None,
        );
        assert!(matches!(result, ExecuteResult::Exited(0)));
        let output = String::from_utf8(buf).expect("utf8");
        // No >> or >>! lines expected
        assert!(!output.contains(">> "));
        assert!(!output.contains(">>! "));
    }

    #[test]
    fn execute_spawn_error() {
        let mut buf: Vec<u8> = Vec::new();
        let result = execute_command(
            &["/nonexistent/binary"],
            10,
            "test-session",
            &mut buf,
            10_485_760,
            None,
        );
        assert!(matches!(result, ExecuteResult::SpawnError(_)));
    }

    #[test]
    fn execute_max_stream_bytes_truncates() {
        let mut buf: Vec<u8> = Vec::new();
        // Generer beaucoup de sortie, limiter a 100 octets
        let result = execute_command(
            &[
                "/bin/sh",
                "-c",
                "for i in $(seq 1 100); do echo line$i; done",
            ],
            10,
            "test-session",
            &mut buf,
            100,
            None,
        );
        assert!(matches!(result, ExecuteResult::Exited(0)));
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains(">>! ssh-frontiere: output truncated"));
    }

    // --- Phase 9 : body → stdin piped (3.1) ---

    #[test]
    fn execute_command_with_body_sends_to_stdin() {
        let mut buf: Vec<u8> = Vec::new();
        let result = execute_command(
            &["/bin/cat"],
            10,
            "test-session",
            &mut buf,
            10_485_760,
            Some("hello from body"),
        );
        assert!(matches!(result, ExecuteResult::Exited(0)));
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains(">> hello from body"));
    }

    #[test]
    fn execute_command_without_body_stdin_null() {
        let mut buf: Vec<u8> = Vec::new();
        let result = execute_command(
            &["/bin/cat"],
            10,
            "test-session",
            &mut buf,
            10_485_760,
            None,
        );
        assert!(matches!(result, ExecuteResult::Exited(0)));
        let output = String::from_utf8(buf).expect("utf8");
        // cat with null stdin produces no output
        assert!(!output.contains(">> "));
    }

    // --- execute_command: empty cmd_parts ---

    #[test]
    fn execute_empty_command_returns_spawn_error() {
        let mut buf: Vec<u8> = Vec::new();
        let result = execute_command(&[], 10, "test-session", &mut buf, 10_485_760, None);
        assert!(matches!(result, ExecuteResult::SpawnError(_)));
    }

    // --- execute_command: process killed by signal ---

    #[test]
    fn execute_command_signaled_process() {
        let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/self-signal.sh");
        let mut buf: Vec<u8> = Vec::new();
        let result = execute_command(
            &["/bin/sh", script],
            10,
            "test-session",
            &mut buf,
            10_485_760,
            None,
        );
        assert!(
            matches!(result, ExecuteResult::Signaled(_)),
            "expected Signaled, got {result:?}"
        );
    }
}
