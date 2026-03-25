#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::auth::AuthContext;
    use crate::chain_exec::*;
    use crate::chain_parser::{CommandNode, SequenceMode};
    use crate::config::{Config, TrustLevel};
    use crate::dispatch::Identity;
    use crate::output::{EXIT_INSUFFICIENT_LEVEL, EXIT_REJECTED, EXIT_STDIN_ERROR, EXIT_TIMEOUT};

    // --- Tests execute_chain avec /bin/true et /bin/false ---

    fn exec_config() -> Config {
        let toml_str = r#"
            [global]
            log_file = "/tmp/chain-exec-test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.ok]
            description = "Always succeeds"
            level = "read"
            timeout = 10
            execute = "/bin/true"


            [domains.test.actions.fail]
            description = "Always fails"
            level = "read"
            timeout = 10
            execute = "/bin/false"


            [domains.test.actions.echo]
            description = "Echo"
            level = "read"
            timeout = 10
            execute = "/bin/echo hello"

        "#;
        Config::from_str(toml_str).expect("exec test config")
    }

    fn exec_identity() -> Identity {
        Identity {
            level: TrustLevel::Admin,
            ssh_client: None,
        }
    }

    fn exec_auth_ctx() -> AuthContext {
        AuthContext::new(TrustLevel::Admin, 3)
    }

    #[test]
    fn execute_single_success() {
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Single("test ok".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, 0);
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains(">>>"));
    }

    #[test]
    fn execute_single_failure() {
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Single("test fail".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_ne!(code, 0);
    }

    #[test]
    fn execute_strict_sequence_stops_on_failure() {
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Sequence(
            vec![
                CommandNode::Single("test fail".to_string()),
                CommandNode::Single("test ok".to_string()),
            ],
            SequenceMode::Strict,
        );
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_ne!(code, 0);
        let output = String::from_utf8(buf).expect("utf8");
        // Seulement une reponse >> (la premiere commande fail, pas la deuxieme)
        let response_count = output.matches(">>> {").count();
        assert_eq!(response_count, 1);
    }

    #[test]
    fn execute_permissive_sequence_continues_on_failure() {
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Sequence(
            vec![
                CommandNode::Single("test fail".to_string()),
                CommandNode::Single("test ok".to_string()),
            ],
            SequenceMode::Permissive,
        );
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        // Dernier code = 0 (test ok)
        assert_eq!(code, 0);
        let output = String::from_utf8(buf).expect("utf8");
        // Deux reponses >> (les deux commandes executees)
        let response_count = output.matches(">>> {").count();
        assert_eq!(response_count, 2);
    }

    #[test]
    fn execute_recovery_on_failure() {
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Recovery(
            Box::new(CommandNode::Single("test fail".to_string())),
            Box::new(CommandNode::Single("test ok".to_string())),
        );
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, 0);
        let output = String::from_utf8(buf).expect("utf8");
        // Deux reponses (fail + ok)
        let response_count = output.matches(">>> {").count();
        assert_eq!(response_count, 2);
    }

    #[test]
    fn execute_recovery_skips_right_on_success() {
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Recovery(
            Box::new(CommandNode::Single("test ok".to_string())),
            Box::new(CommandNode::Single("test fail".to_string())),
        );
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, 0);
        let output = String::from_utf8(buf).expect("utf8");
        // Une seule reponse (ok, pas le rattrapage)
        let response_count = output.matches(">>> {").count();
        assert_eq!(response_count, 1);
    }

    #[test]
    fn execute_rejected_command_emits_response() {
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Single("unknown domain".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_ne!(code, 0);
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains(">>>"));
        assert!(output.contains("rejected"));
    }

    #[test]
    fn execute_rbac_rejected_emits_response() {
        // read level cannot access ops action
        let identity = Identity {
            level: TrustLevel::Read,
            ssh_client: None,
        };
        let auth_ctx = AuthContext::new(TrustLevel::Read, 3);
        let node = CommandNode::Single("mastodon restart".to_string());

        // Use config with mastodon.restart (ops level)
        let full_config = {
            let toml_str = r#"
                [global]
                log_file = "/tmp/chain-rbac-test.json"

                [domains.mastodon]
                description = "Mastodon"

                [domains.mastodon.actions.restart]
                description = "Restart"
                level = "ops"
                timeout = 30
                execute = "/bin/true"

            "#;
            Config::from_str(toml_str).expect("rbac test config")
        };

        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &full_config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_ne!(code, 0);
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains("rejected"));
    }

    // --- Built-in: exit ---

    #[test]
    fn execute_exit_builtin_returns_zero() {
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Single("exit".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, 0);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains(">>> {"));
        assert!(output.contains("\"exit\""));
    }

    // --- Parse error path ---

    #[test]
    fn execute_parse_error_returns_rejected() {
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        // Unclosed quote triggers parse error
        let node = CommandNode::Single("test \"unclosed".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, EXIT_REJECTED);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("rejected"));
    }

    // --- Built-in: help overview ---

    fn rich_config() -> Config {
        let toml_str = r#"
            [global]
            log_file = "/tmp/chain-exec-cov-test.json"

            [domains.app]
            description = "Application"

            [domains.app.actions.status]
            description = "Check status"
            level = "read"
            timeout = 10
            execute = "/bin/true"

            [domains.app.actions.deploy]
            description = "Deploy app"
            level = "ops"
            timeout = 10
            execute = "/bin/echo {domain} {tag}"
            max_body_size = 1024

            [domains.app.actions.deploy.args]
            tag = { type = "enum", values = ["latest", "stable"] }

            [domains.app.actions.notify]
            description = "Send notification"
            level = "read"
            timeout = 10
            execute = "/bin/echo {msg}"

            [domains.app.actions.notify.args]
            msg = { free = true, default = "hello" }

            [domains.app.actions.tagged]
            description = "Tagged action"
            level = "read"
            timeout = 10
            execute = "/bin/true"
            tags = ["deploy"]
        "#;
        Config::from_str(toml_str).unwrap()
    }

    #[test]
    fn execute_help_overview_returns_zero() {
        let config = rich_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Single("help".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, 0);
        let output = String::from_utf8(buf).unwrap();
        // Help writes #> comments with protocol info
        assert!(output.contains("#> "));
        assert!(output.contains("Protocol"));
        assert!(output.contains("Available domains"));
        assert!(output.contains("app"));
        // Final >>> response
        assert!(output.contains(">>> {"));
    }

    #[test]
    fn execute_help_domain_detail() {
        let config = rich_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Single("help app".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, 0);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Domain: app"));
        assert!(output.contains("required level"));
        // deploy has args
        assert!(output.contains("arguments"));
        // deploy has custom max_body_size (1024 != 65536)
        assert!(output.contains("body: max 1 KB"));
    }

    #[test]
    fn execute_help_unknown_domain() {
        let config = rich_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Single("help nonexistent".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, 0);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("unknown domain"));
    }

    // --- Built-in: list ---

    #[test]
    fn execute_list_returns_json() {
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Single("list".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, 0);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains(">>> {"));
        assert!(output.contains("\"ok\""));
    }

    // --- SpawnError path ---

    fn spawn_error_config() -> Config {
        let toml_str = r#"
            [global]
            log_file = "/tmp/chain-exec-spawn-test.json"

            [domains.bad]
            description = "Bad commands"

            [domains.bad.actions.missing]
            description = "Missing binary"
            level = "read"
            timeout = 10
            execute = "/nonexistent/binary"
        "#;
        Config::from_str(toml_str).unwrap()
    }

    #[test]
    fn execute_spawn_error_in_chain() {
        let config = spawn_error_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Single("bad missing".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, EXIT_REJECTED);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("execution error"));
    }

    // --- Timeout path ---

    fn timeout_config() -> Config {
        let toml_str = r#"
            [global]
            log_file = "/tmp/chain-exec-timeout-test.json"

            [domains.slow]
            description = "Slow commands"

            [domains.slow.actions.wait]
            description = "Wait forever"
            level = "read"
            timeout = 1
            execute = "/bin/sleep 100"
        "#;
        Config::from_str(toml_str).unwrap()
    }

    #[test]
    fn execute_timeout_in_chain() {
        let config = timeout_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Single("slow wait".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, EXIT_TIMEOUT);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("timeout"));
    }

    // --- Signaled path ---

    fn signaled_config() -> Config {
        let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/self-signal.sh");
        let toml_str = format!(
            r#"
            [global]
            log_file = "/tmp/chain-exec-signal-test.json"

            [domains.sig]
            description = "Signal test"

            [domains.sig.actions.die]
            description = "Die by signal"
            level = "read"
            timeout = 10
            execute = "/bin/sh {script}"
        "#,
            script = script
        );
        Config::from_str(&toml_str).unwrap()
    }

    #[test]
    fn execute_signaled_in_chain() {
        let config = signaled_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Single("sig die".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        // SIGTERM = 15, so exit code = 128 + 15 = 143
        assert!(code > 128, "expected signal exit code > 128, got {code}");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains(">>> {"));
    }

    // --- StdinError path ---

    #[test]
    fn execute_stdin_error_in_chain() {
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        // /bin/true exits immediately without reading stdin; large body triggers EPIPE
        let node = CommandNode::Single("test ok".to_string());
        let large_body = "x".repeat(256 * 1024);
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            Some(&large_body),
        );
        // Either StdinError (133) or Exited(0) depending on race condition
        // We just verify no panic and a valid response
        assert!(code == EXIT_STDIN_ERROR || code == 0);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains(">>> {"));
    }

    // --- RBAC rejected with ssh_client (covers log_command with ssh_client) ---

    #[test]
    fn execute_rbac_rejected_with_ssh_client_logs() {
        let identity = Identity {
            level: TrustLevel::Read,
            ssh_client: Some("192.168.1.1 12345 22".to_string()),
        };
        let auth_ctx = AuthContext::new(TrustLevel::Read, 3);
        let config = {
            let toml_str = r#"
                [global]
                log_file = "/tmp/chain-exec-sshclient-test.json"

                [domains.svc]
                description = "Service"

                [domains.svc.actions.restart]
                description = "Restart"
                level = "ops"
                timeout = 10
                execute = "/bin/true"
            "#;
            Config::from_str(toml_str).unwrap()
        };
        let node = CommandNode::Single("svc restart".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, EXIT_INSUFFICIENT_LEVEL);
    }

    // --- write_help_text direct tests ---

    #[test]
    fn write_help_text_overview_includes_operators() {
        let config = rich_config();
        let identity = Identity {
            level: TrustLevel::Admin,
            ssh_client: None,
        };
        let tokens = vec!["help".to_string()];
        let mut buf = Vec::new();
        write_help_text(&config, &tokens, &identity, &[], &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Operators"));
        assert!(output.contains("Strict sequential"));
        assert!(output.contains("Fallback"));
        assert!(output.contains("Grouping"));
        assert!(output.contains("list"));
    }

    #[test]
    fn write_help_text_domain_detail_shows_args_and_body() {
        let config = rich_config();
        let identity = Identity {
            level: TrustLevel::Admin,
            ssh_client: None,
        };
        let tokens = vec!["help".to_string(), "app".to_string()];
        let mut buf = Vec::new();
        write_help_text(&config, &tokens, &identity, &[], &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Domain: app"));
        assert!(output.contains("arguments"));
        assert!(output.contains("body: max 1 KB"));
        assert!(output.contains("optional"));
        assert!(output.contains("[free text]"));
    }

    #[test]
    fn write_help_text_unknown_domain_message() {
        let config = rich_config();
        let identity = Identity {
            level: TrustLevel::Admin,
            ssh_client: None,
        };
        let tokens = vec!["help".to_string(), "nope".to_string()];
        let mut buf = Vec::new();
        write_help_text(&config, &tokens, &identity, &[], &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("unknown domain: nope"));
    }

    #[test]
    fn write_help_text_filters_by_tags() {
        let config = rich_config();
        let identity = Identity {
            level: TrustLevel::Admin,
            ssh_client: None,
        };
        // Without deploy tag, deploy action should be hidden
        let tokens = vec!["help".to_string(), "app".to_string()];
        let mut buf_no_tag = Vec::new();
        write_help_text(&config, &tokens, &identity, &[], &mut buf_no_tag);
        let output_no_tag = String::from_utf8(buf_no_tag).unwrap();

        // With deploy tag, deploy action should be visible
        let mut buf_with_tag = Vec::new();
        let tags = vec!["deploy".to_string()];
        write_help_text(&config, &tokens, &identity, &tags, &mut buf_with_tag);
        let output_with_tag = String::from_utf8(buf_with_tag).unwrap();

        // The tagged version should have more content (deploy action visible)
        assert!(output_with_tag.len() > output_no_tag.len());
    }

    // --- Sequence body consumed by first command only ---

    #[test]
    fn sequence_body_consumed_by_first_only() {
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Sequence(
            vec![
                CommandNode::Single("test ok".to_string()),
                CommandNode::Single("test ok".to_string()),
            ],
            SequenceMode::Permissive,
        );
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            Some("body-data"),
        );
        assert_eq!(code, 0);
    }

    // --- list with filter args ---

    #[test]
    fn execute_list_domain_filter() {
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Single("list test".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, 0);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains(">>> {"));
    }

    #[test]
    fn execute_list_with_extra_args_still_succeeds() {
        // list ignores extra arguments (no domain filter in discovery)
        let config = exec_config();
        let identity = exec_identity();
        let auth_ctx = exec_auth_ctx();
        let node = CommandNode::Single("list extra".to_string());
        let mut buf = Vec::new();
        let code = execute_chain(
            &node,
            &config,
            &identity,
            &auth_ctx,
            "test-sess",
            &mut buf,
            None,
        );
        assert_eq!(code, 0);
    }
}
