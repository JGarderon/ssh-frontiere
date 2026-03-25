#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::config::{Config, TrustLevel};
    use crate::dispatch::*;
    use std::collections::HashMap;

    fn test_config() -> Config {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.forgejo]
            description = "Forge Git"

            [domains.forgejo.actions.backup-config]
            description = "Backup config"
            level = "ops"
            timeout = 600
            execute = "/usr/local/bin/backup.sh {domain}"

            [domains.forgejo.actions.deploy]
            description = "Deploy"
            level = "ops"
            timeout = 300
            execute = "/usr/local/bin/deploy.sh {domain} {tag}"

            [domains.forgejo.actions.deploy.args]
            tag = { type = "enum", values = ["latest", "stable"] }

            [domains.infra]
            description = "Infra"

            [domains.infra.actions.healthcheck]
            description = "Healthcheck"
            level = "read"
            timeout = 30
            execute = "/usr/local/bin/healthcheck.sh"
        "#;
        Config::from_str(toml_str).expect("test config")
    }

    // --- Identity ---

    #[test]
    fn parse_level_from_args() {
        let identity = Identity::from_args(&["--level=ops"], None);
        assert_eq!(identity.level, TrustLevel::Ops);
    }

    #[test]
    fn parse_level_default_to_read() {
        let identity = Identity::from_args(&[], None);
        assert_eq!(identity.level, TrustLevel::Read);
    }

    #[test]
    fn parse_level_with_ssh_client() {
        let identity = Identity::from_args(&["--level=ops"], Some("192.168.1.1 12345 22"));
        assert_eq!(
            identity.ssh_client,
            Some("192.168.1.1 12345 22".to_string())
        );
    }

    // --- Command parsing ---

    #[test]
    fn parse_command_domain_action() {
        let tokens = parse_command("forgejo backup-config").expect("parse");
        assert_eq!(tokens, vec!["forgejo", "backup-config"]);
    }

    #[test]
    fn parse_command_with_args() {
        let tokens = parse_command("forgejo deploy latest").expect("parse");
        assert_eq!(tokens, vec!["forgejo", "deploy", "latest"]);
    }

    #[test]
    fn parse_command_help() {
        let tokens = parse_command("help").expect("parse");
        assert_eq!(tokens, vec!["help"]);
    }

    // Special chars without quotes are valid tokens for parse_command.
    // Rejection happens at resolve_command (grammar: extra tokens).
    #[test]
    fn parse_command_tokenizes_pipe_as_token() {
        let tokens = parse_command("forgejo backup | cat").expect("parse");
        assert_eq!(tokens, vec!["forgejo", "backup", "|", "cat"]);
    }

    #[test]
    fn parse_command_tokenizes_semicolon_as_token() {
        // ";" is attached to the previous token (no space before)
        let tokens = parse_command("forgejo backup; rm -rf /").expect("parse");
        assert_eq!(tokens, vec!["forgejo", "backup;", "rm", "-rf", "/"]);
    }

    #[test]
    fn parse_command_tokenizes_ampersand_as_token() {
        let tokens = parse_command("forgejo backup & echo").expect("parse");
        assert_eq!(tokens, vec!["forgejo", "backup", "&", "echo"]);
    }

    #[test]
    fn parse_command_tokenizes_special_chars() {
        let tokens = parse_command("forgejo backup > /tmp/out").expect("parse");
        assert_eq!(tokens, vec!["forgejo", "backup", ">", "/tmp/out"]);
    }

    #[test]
    fn parse_command_tokenizes_dollar() {
        let tokens = parse_command("forgejo $HOME").expect("parse");
        assert_eq!(tokens, vec!["forgejo", "$HOME"]);
    }

    #[test]
    fn parse_command_tokenizes_backtick() {
        let tokens = parse_command("forgejo `whoami`").expect("parse");
        assert_eq!(tokens, vec!["forgejo", "`whoami`"]);
    }

    #[test]
    fn parse_command_tokenizes_backslash() {
        let tokens = parse_command("forgejo back\\up").expect("parse");
        assert_eq!(tokens, vec!["forgejo", "back\\up"]);
    }

    #[test]
    fn parse_command_tokenizes_exclamation() {
        let tokens = parse_command("forgejo !cmd").expect("parse");
        assert_eq!(tokens, vec!["forgejo", "!cmd"]);
    }

    // Special chars inside quotes are content, not syntax
    #[test]
    fn parse_command_special_chars_in_double_quotes() {
        let tokens = parse_command(r#"forgejo deploy "hello|world;test&foo""#).expect("parse");
        assert_eq!(tokens, vec!["forgejo", "deploy", "hello|world;test&foo"]);
    }

    #[test]
    fn parse_command_dollar_in_quotes_not_expanded() {
        let tokens = parse_command(r#"forgejo deploy "$HOME""#).expect("parse");
        assert_eq!(tokens, vec!["forgejo", "deploy", "$HOME"]);
    }

    #[test]
    fn parse_command_rejects_too_long() {
        let long_cmd = "a".repeat(4097);
        assert!(parse_command(&long_cmd).is_err());
    }

    #[test]
    fn parse_command_rejects_long_token() {
        let long_token = "a".repeat(257);
        assert!(parse_command(&long_token).is_err());
    }

    #[test]
    fn parse_command_rejects_empty() {
        assert!(parse_command("").is_err());
    }

    // --- Resolution ---

    #[test]
    fn resolve_domain_and_action() {
        let config = test_config();
        let tokens = vec!["forgejo".to_string(), "backup-config".to_string()];
        let (domain_id, action_id, args) = resolve_command(&config, &tokens).expect("resolve");
        assert_eq!(domain_id, "forgejo");
        assert_eq!(action_id, "backup-config");
        assert!(args.is_empty());
    }

    #[test]
    fn resolve_with_args() {
        let config = test_config();
        let tokens = vec![
            "forgejo".to_string(),
            "deploy".to_string(),
            "tag=latest".to_string(),
        ];
        let (domain_id, action_id, args) = resolve_command(&config, &tokens).expect("resolve");
        assert_eq!(domain_id, "forgejo");
        assert_eq!(action_id, "deploy");
        assert_eq!(args.get("tag"), Some(&"latest".to_string()));
    }

    #[test]
    fn resolve_unknown_domain() {
        let config = test_config();
        let tokens = vec!["unknown".to_string(), "backup".to_string()];
        assert!(resolve_command(&config, &tokens).is_err());
    }

    #[test]
    fn resolve_unknown_action() {
        let config = test_config();
        let tokens = vec!["forgejo".to_string(), "unknown".to_string()];
        assert!(resolve_command(&config, &tokens).is_err());
    }

    #[test]
    fn resolve_rejects_invalid_enum_value() {
        let config = test_config();
        let tokens = vec![
            "forgejo".to_string(),
            "deploy".to_string(),
            "tag=invalid".to_string(),
        ];
        assert!(resolve_command(&config, &tokens).is_err());
    }

    #[test]
    fn resolve_rejects_missing_required_args() {
        let config = test_config();
        let tokens = vec!["forgejo".to_string(), "deploy".to_string()];
        assert!(resolve_command(&config, &tokens).is_err());
    }

    #[test]
    fn resolve_rejects_extra_args() {
        let config = test_config();
        let tokens = vec![
            "forgejo".to_string(),
            "backup-config".to_string(),
            "extra".to_string(),
        ];
        assert!(resolve_command(&config, &tokens).is_err());
    }

    // --- Quoted arguments (TODO-004) ---

    #[test]
    fn parse_command_double_quotes() {
        let tokens = parse_command(r#"test deploy "my version tag""#).expect("parse");
        assert_eq!(tokens, vec!["test", "deploy", "my version tag"]);
    }

    #[test]
    fn parse_command_single_quotes() {
        let tokens = parse_command("test deploy 'my version tag'").expect("parse");
        assert_eq!(tokens, vec!["test", "deploy", "my version tag"]);
    }

    #[test]
    fn parse_command_no_quotes() {
        let tokens = parse_command("test deploy latest").expect("parse");
        assert_eq!(tokens, vec!["test", "deploy", "latest"]);
    }

    #[test]
    fn parse_command_unclosed_double_quote() {
        let result = parse_command(r#"test deploy "unclosed"#);
        assert!(matches!(result, Err(DispatchError::UnclosedQuote('"'))));
    }

    #[test]
    fn parse_command_unclosed_single_quote() {
        let result = parse_command("test deploy 'unclosed");
        assert!(matches!(result, Err(DispatchError::UnclosedQuote('\''))));
    }

    #[test]
    fn parse_command_mixed_quotes() {
        let tokens = parse_command(r#"test deploy "hello world" 'foo bar'"#).expect("parse");
        assert_eq!(tokens, vec!["test", "deploy", "hello world", "foo bar"]);
    }

    // --- RBAC authorization ---

    #[test]
    fn rbac_ops_can_access_ops_action() {
        let config = test_config();
        let identity = Identity::from_args(&["--level=ops"], None);
        let result = check_authorization(
            &identity,
            &config.domains["forgejo"].actions["backup-config"],
            &[],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn rbac_read_cannot_access_ops_action() {
        let config = test_config();
        let identity = Identity::from_args(&["--level=read"], None);
        let result = check_authorization(
            &identity,
            &config.domains["forgejo"].actions["backup-config"],
            &[],
        );
        assert!(result.is_err());
    }

    #[test]
    fn rbac_read_can_access_read_action() {
        let config = test_config();
        let identity = Identity::from_args(&["--level=read"], None);
        let result = check_authorization(
            &identity,
            &config.domains["infra"].actions["healthcheck"],
            &[],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn rbac_admin_can_access_ops_action() {
        let config = test_config();
        let identity = Identity::from_args(&["--level=admin"], None);
        let result = check_authorization(
            &identity,
            &config.domains["forgejo"].actions["backup-config"],
            &[],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn rbac_admin_can_access_read_action() {
        let config = test_config();
        let identity = Identity::from_args(&["--level=admin"], None);
        let result = check_authorization(
            &identity,
            &config.domains["infra"].actions["healthcheck"],
            &[],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn rbac_ops_can_access_read_action() {
        let config = test_config();
        let identity = Identity::from_args(&["--level=ops"], None);
        let result = check_authorization(
            &identity,
            &config.domains["infra"].actions["healthcheck"],
            &[],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn rbac_error_message_contains_levels() {
        let config = test_config();
        let identity = Identity::from_args(&["--level=read"], None);
        let err = check_authorization(
            &identity,
            &config.domains["forgejo"].actions["backup-config"],
            &[],
        )
        .expect_err("should fail");
        let msg = err.to_string();
        assert!(msg.contains("ops"));
        assert!(msg.contains("read"));
    }

    // --- Transposition ---

    #[test]
    fn transpose_with_domain() {
        let args = HashMap::new();
        let result = transpose_command("/usr/local/bin/backup.sh {domain}", "forgejo", &args);
        assert_eq!(result, vec!["/usr/local/bin/backup.sh", "forgejo"]);
    }

    #[test]
    fn transpose_with_args() {
        let mut args = HashMap::new();
        args.insert("tag".to_string(), "latest".to_string());
        let result = transpose_command("/usr/local/bin/deploy.sh {domain} {tag}", "forgejo", &args);
        assert_eq!(
            result,
            vec!["/usr/local/bin/deploy.sh", "forgejo", "latest"]
        );
    }

    // --- Phase 5: Tags authorization (ADR 0008) ---

    fn test_config_with_tags() -> Config {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.forgejo]
            description = "Forge Git"

            [domains.forgejo.actions.backup]
            description = "Backup"
            level = "ops"
            timeout = 600
            execute = "/bin/echo backup"
            tags = ["forgejo"]

            [domains.forgejo.actions.deploy]
            description = "Deploy"
            level = "ops"
            timeout = 300
            execute = "/bin/echo deploy"
            tags = ["forgejo", "deploy"]

            [domains.mastodon]
            description = "Mastodon"

            [domains.mastodon.actions.healthcheck]
            description = "Health"
            level = "read"
            timeout = 30
            execute = "/bin/echo health"
            tags = ["mastodon"]

            [domains.infra]
            description = "Infra"

            [domains.infra.actions.status]
            description = "Status"
            level = "read"
            timeout = 30
            execute = "/bin/echo status"
        "#;
        Config::from_str(toml_str).expect("test config with tags")
    }

    // --- check_tags tests ---

    #[test]
    fn test_check_tags_no_action_tags() {
        // Action without tags = public → always authorized
        assert!(check_tags(&["forgejo".to_string()], &[]));
    }

    #[test]
    fn test_check_tags_no_identity_tags() {
        // Action with tags, identity without → denied
        assert!(!check_tags(&[], &["forgejo".to_string()]));
    }

    #[test]
    fn test_check_tags_matching() {
        assert!(check_tags(
            &["forgejo".to_string()],
            &["forgejo".to_string()]
        ));
    }

    #[test]
    fn test_check_tags_no_match() {
        assert!(!check_tags(
            &["mastodon".to_string()],
            &["forgejo".to_string()]
        ));
    }

    #[test]
    fn test_check_tags_multiple_one_match() {
        assert!(check_tags(
            &["mastodon".to_string(), "forgejo".to_string()],
            &["forgejo".to_string(), "deploy".to_string()]
        ));
    }

    // --- check_authorization with tags ---

    #[test]
    fn test_auth_level_ok_tags_ok() {
        let config = test_config_with_tags();
        let identity = Identity::from_args(&["--level=ops"], None);
        let action = &config.domains["forgejo"].actions["backup"];
        let effective_tags = vec!["forgejo".to_string()];
        let result = check_authorization(&identity, action, &effective_tags);
        assert!(result.is_ok());
    }

    #[test]
    fn test_auth_level_ok_tags_ko() {
        let config = test_config_with_tags();
        let identity = Identity::from_args(&["--level=ops"], None);
        let action = &config.domains["forgejo"].actions["backup"];
        let effective_tags = vec!["mastodon".to_string()];
        let result = check_authorization(&identity, action, &effective_tags);
        // Error should not reveal tag details (security)
        assert!(matches!(result, Err(DispatchError::TagMismatch)));
    }

    #[test]
    fn test_auth_level_ko_tags_ok() {
        let config = test_config_with_tags();
        let identity = Identity::from_args(&["--level=read"], None);
        let action = &config.domains["forgejo"].actions["backup"];
        let effective_tags = vec!["forgejo".to_string()];
        let result = check_authorization(&identity, action, &effective_tags);
        assert!(result.is_err());
        // Tags pass (forgejo match), then level check fails
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("insufficient level"));
    }

    #[test]
    fn test_auth_level_ko_tags_ko_reports_tag_mismatch() {
        // SEC-015 regression: when both level AND tags fail,
        // tag mismatch must be reported (not insufficient level)
        let config = test_config_with_tags();
        let identity = Identity::from_args(&["--level=read"], None);
        let action = &config.domains["forgejo"].actions["backup"];
        let effective_tags = vec!["mastodon".to_string()];
        let result = check_authorization(&identity, action, &effective_tags);
        assert!(matches!(result, Err(DispatchError::TagMismatch)));
    }

    // --- Phase 9 : resolve_arguments free = true (ADR 0012 D5) ---

    // --- Phase 9 : transpose_command fix for spaces (3.3) ---

    #[test]
    fn test_transpose_command_value_with_spaces() {
        let mut args = std::collections::HashMap::new();
        args.insert("msg".to_string(), "hello world".to_string());
        let result = transpose_command("/usr/bin/echo {msg}", "dom", &args);
        assert_eq!(result, vec!["/usr/bin/echo", "hello world"]);
        assert_eq!(result.len(), 2); // NOT 3
    }

    #[test]
    fn test_transpose_command_preserves_token_count() {
        let args = std::collections::HashMap::new();
        let result = transpose_command("/usr/bin/echo {domain}", "mydom", &args);
        assert_eq!(result, vec!["/usr/bin/echo", "mydom"]);
    }

    // --- Phase 9 : EXIT_STDIN_ERROR (3.2) ---

    #[test]
    fn test_exit_stdin_error_constant() {
        assert_eq!(crate::output::EXIT_STDIN_ERROR, 133);
    }

    // --- Phase 9 : resolve_arguments free = true (ADR 0012 D5) ---

    #[test]
    fn test_resolve_free_arg_accepts_arbitrary_value() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.notify]
            description = "Notify"
            level = "read"
            timeout = 10
            execute = "/bin/echo {message}"

            [domains.test.actions.notify.args]
            message = { free = true }
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let tokens: Vec<String> = vec![
            "test".to_string(),
            "notify".to_string(),
            "message=hello world".to_string(),
        ];
        let result = resolve_command(&config, &tokens);
        assert!(result.is_ok());
        let (_, _, args) = result.expect("resolve");
        assert_eq!(args["message"], "hello world");
    }

    #[test]
    fn test_resolve_free_arg_missing_without_default_rejected() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.notify]
            description = "Notify"
            level = "read"
            timeout = 10
            execute = "/bin/echo {message}"

            [domains.test.actions.notify.args]
            message = { free = true }
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let tokens: Vec<String> = vec!["test".to_string(), "notify".to_string()];
        let result = resolve_command(&config, &tokens);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("missing required"));
    }

    #[test]
    fn test_resolve_free_arg_with_default() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.notify]
            description = "Notify"
            level = "read"
            timeout = 10
            execute = "/bin/echo {message}"

            [domains.test.actions.notify.args]
            message = { free = true, default = "bonjour" }
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let tokens: Vec<String> = vec!["test".to_string(), "notify".to_string()];
        let result = resolve_command(&config, &tokens);
        assert!(result.is_ok());
        let (_, _, args) = result.expect("resolve");
        assert_eq!(args["message"], "bonjour");
    }

    // --- DispatchError Display coverage ---

    #[test]
    fn dispatch_error_display_empty_command() {
        let err = DispatchError::EmptyCommand;
        assert_eq!(err.to_string(), "empty command");
    }

    #[test]
    fn dispatch_error_display_command_too_long() {
        let err = DispatchError::CommandTooLong {
            len: 5000,
            max: 4096,
        };
        let msg = err.to_string();
        assert!(msg.contains("5000"));
        assert!(msg.contains("4096"));
    }

    #[test]
    fn dispatch_error_display_token_too_long() {
        let err = DispatchError::TokenTooLong { len: 300, max: 256 };
        let msg = err.to_string();
        assert!(msg.contains("300"));
        assert!(msg.contains("256"));
    }

    #[test]
    fn dispatch_error_display_unclosed_quote() {
        let err = DispatchError::UnclosedQuote('"');
        assert!(err.to_string().contains("unclosed"));
    }

    #[test]
    fn dispatch_error_display_unknown_domain() {
        let err = DispatchError::UnknownDomain("foo".to_string());
        assert!(err.to_string().contains("foo"));
    }

    #[test]
    fn dispatch_error_display_unknown_action() {
        let err = DispatchError::UnknownAction {
            domain: "foo".to_string(),
            action: "bar".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("bar"));
        assert!(msg.contains("foo"));
    }

    #[test]
    fn dispatch_error_display_invalid_syntax() {
        let err = DispatchError::InvalidSyntax("bad syntax".to_string());
        assert_eq!(err.to_string(), "bad syntax");
    }

    #[test]
    fn dispatch_error_display_argument_error() {
        let err = DispatchError::ArgumentError("bad arg".to_string());
        assert_eq!(err.to_string(), "bad arg");
    }

    #[test]
    fn dispatch_error_display_unauthorized() {
        let err = DispatchError::Unauthorized("not allowed".to_string());
        assert_eq!(err.to_string(), "not allowed");
    }

    #[test]
    fn dispatch_error_display_tag_mismatch() {
        let err = DispatchError::TagMismatch;
        assert!(err.to_string().contains("tag mismatch"));
    }

    // --- resolve_command: single token (InvalidSyntax) ---

    #[test]
    fn resolve_single_token_invalid_syntax() {
        let config = test_config();
        let tokens = vec!["forgejo".to_string()];
        let result = resolve_command(&config, &tokens);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("expected"));
    }

    // --- resolve_arguments: unknown argument ---

    #[test]
    fn resolve_unknown_argument_rejected() {
        let config = test_config();
        let tokens = vec![
            "forgejo".to_string(),
            "deploy".to_string(),
            "unknown=value".to_string(),
        ];
        let result = resolve_command(&config, &tokens);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("unknown argument"));
    }

    // --- resolve_arguments: duplicate argument ---

    #[test]
    fn resolve_duplicate_argument_rejected() {
        let config = test_config();
        let tokens = vec![
            "forgejo".to_string(),
            "deploy".to_string(),
            "tag=latest".to_string(),
            "tag=stable".to_string(),
        ];
        let result = resolve_command(&config, &tokens);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("duplicate"));
    }

    // --- parse_command: whitespace-only input ---

    #[test]
    fn parse_command_whitespace_only_is_empty() {
        let result = parse_command("   \t  ");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DispatchError::EmptyCommand));
    }
}
