#[cfg(test)]
mod tests {
    use crate::config::*;

    #[test]
    fn trust_level_ordering() {
        assert!(TrustLevel::Read < TrustLevel::Ops);
        assert!(TrustLevel::Ops < TrustLevel::Admin);
        assert!(TrustLevel::Read < TrustLevel::Admin);
        assert!(TrustLevel::Ops >= TrustLevel::Ops);
        assert!(TrustLevel::Admin >= TrustLevel::Read);
    }

    #[test]
    fn trust_level_from_str() {
        assert_eq!(
            "read".parse::<TrustLevel>().expect("parse read"),
            TrustLevel::Read
        );
        assert_eq!(
            "ops".parse::<TrustLevel>().expect("parse ops"),
            TrustLevel::Ops
        );
        assert_eq!(
            "admin".parse::<TrustLevel>().expect("parse admin"),
            TrustLevel::Admin
        );
        assert!("invalid".parse::<TrustLevel>().is_err());
    }

    #[test]
    fn trust_level_display() {
        assert_eq!(TrustLevel::Read.to_string(), "read");
        assert_eq!(TrustLevel::Ops.to_string(), "ops");
        assert_eq!(TrustLevel::Admin.to_string(), "admin");
    }

    #[test]
    fn argdef_without_sensitive() {
        let toml_str = r#"
            type = "enum"
            values = ["latest", "stable"]
        "#;
        let arg: ArgDef = toml::from_str(toml_str).expect("parse argdef");
        assert_eq!(arg.arg_type, "enum");
        assert_eq!(
            arg.values,
            Some(vec!["latest".to_string(), "stable".to_string()])
        );
        assert!(!arg.sensitive);
    }

    #[test]
    fn argdef_with_sensitive() {
        let toml_str = r#"
            type = "string"
            sensitive = true
        "#;
        let arg: ArgDef = toml::from_str(toml_str).expect("parse argdef");
        assert_eq!(arg.arg_type, "string");
        assert!(arg.sensitive);
        assert!(arg.values.is_none());
    }

    #[test]
    fn action_config_deserialization() {
        let toml_str = r#"
            description = "Sauvegarde config"
            level = "ops"
            timeout = 600
            execute = "sudo /usr/local/bin/backup.sh {domain}"
        "#;
        let action: ActionConfig = toml::from_str(toml_str).expect("parse action");
        assert_eq!(action.description, "Sauvegarde config");
        assert_eq!(action.level, TrustLevel::Ops);
        assert_eq!(action.timeout, Some(600));
        assert_eq!(action.execute, "sudo /usr/local/bin/backup.sh {domain}");
        assert!(action.args.is_empty());
    }

    #[test]
    fn domain_config_deserialization() {
        let toml_str = r#"
            description = "Forge Git"

            [actions.backup]
            description = "Backup"
            level = "ops"
            timeout = 600
            execute = "/usr/local/bin/backup.sh"
        "#;
        let domain: DomainConfig = toml::from_str(toml_str).expect("parse domain");
        assert_eq!(domain.description, "Forge Git");
        assert_eq!(domain.actions.len(), 1);
        assert!(domain.actions.contains_key("backup"));
    }

    #[test]
    fn global_config_defaults() {
        let toml_str = r#"
            log_file = "/var/log/test.json"
        "#;
        let global: GlobalConfig = toml::from_str(toml_str).expect("parse global");
        assert_eq!(global.log_file, "/var/log/test.json");
        assert_eq!(global.default_timeout, 300);
        // default_level, mask_sensitive: TOML compat fields (deserialized, not used at runtime)
        assert_eq!(global.max_stdout_chars, 65536);
        assert_eq!(global.max_stderr_chars, 16384);
        assert_eq!(global.max_output_chars, 131072);
    }

    #[test]
    fn full_config_deserialization() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/ssh-frontiere/commands.json"
            default_timeout = 300
            default_level = "ops"
            mask_sensitive = false
            max_stdout_chars = 65536
            max_stderr_chars = 16384
            max_output_chars = 131072

            [domains.forgejo]
            description = "Forge Git infrastructure"

            [domains.forgejo.actions.backup-config]
            description = "Sauvegarde la configuration"
            level = "ops"
            timeout = 600
            execute = "sudo /usr/local/bin/backup-config.sh {domain}"

            [domains.infra]
            description = "Serveur hote"

            [domains.infra.actions.healthcheck]
            description = "Healthcheck"
            level = "read"
            timeout = 30
            execute = "sudo /usr/local/bin/healthcheck.sh"
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse config");
        assert_eq!(config.domains.len(), 2);
        assert!(config.domains.contains_key("forgejo"));
        assert!(config.domains.contains_key("infra"));
        let forgejo = &config.domains["forgejo"];
        assert_eq!(forgejo.actions.len(), 1);
        assert!(forgejo.actions.contains_key("backup-config"));
    }

    // --- Étape 2 : chargement fichier et validation ---

    #[test]
    fn load_config_from_fixture_file() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/config.toml");
        let config = Config::from_file(path).expect("load fixture");
        assert_eq!(config.domains.len(), 3);
        assert!(config.domains.contains_key("forgejo"));
        assert!(config.domains.contains_key("mastodon"));
        assert!(config.domains.contains_key("infra"));
        // forgejo has 4 actions
        assert_eq!(config.domains["forgejo"].actions.len(), 4);
        // default_timeout applied when action has no timeout
        assert_eq!(config.global.default_timeout, 300);
    }

    #[test]
    fn load_config_file_not_found() {
        let result = Config::from_file("/nonexistent/config.toml");
        assert!(result.is_err());
        let err = result.expect_err("should fail").to_string();
        assert!(err.contains("not found") || err.contains("No such file"));
    }

    #[test]
    fn validate_rejects_empty_domains() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"
        "#;
        let result = Config::from_str(toml_str);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("domain"));
    }

    #[test]
    fn validate_rejects_domain_without_actions() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.empty]
            description = "Empty domain"
        "#;
        let result = Config::from_str(toml_str);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("action"));
    }

    #[test]
    fn validate_rejects_enum_without_values() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test cmd"
            level = "ops"
            timeout = 30
            execute = "/bin/echo {arg}"

            [domains.test.actions.cmd.args]
            arg = { type = "enum" }
        "#;
        let result = Config::from_str(toml_str);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("enum"));
    }

    #[test]
    fn validate_rejects_placeholder_mismatch() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test cmd"
            level = "ops"
            timeout = 30
            execute = "/bin/echo {missing}"
        "#;
        let result = Config::from_str(toml_str);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("missing"));
    }

    #[test]
    fn validate_rejects_output_limits_exceeded() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"
            max_stdout_chars = 200000
            max_output_chars = 131072

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test cmd"
            level = "ops"
            timeout = 30
            execute = "/bin/echo"
        "#;
        let result = Config::from_str(toml_str);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("max_output"));
    }

    // --- Phase 3 : [global] extensions ---

    #[test]
    fn global_config_protocol_defaults() {
        let toml_str = r#"
            log_file = "/var/log/test.json"
        "#;
        let global: GlobalConfig = toml::from_str(toml_str).expect("parse global");
        assert_eq!(global.timeout_session, 3600);
        assert_eq!(global.max_auth_failures, 3);
        assert!(!global.log_comments);
        assert_eq!(global.ban_command, "");
        assert!(!global.expose_session_id);
    }

    #[test]
    fn global_config_protocol_custom_values() {
        let toml_str = r#"
            log_file = "/var/log/test.json"
            timeout_session = 7200
            max_auth_failures = 5
            log_comments = true
            ban_command = "/usr/local/bin/ban-ip.sh {ip}"
            expose_session_id = true
        "#;
        let global: GlobalConfig = toml::from_str(toml_str).expect("parse global");
        assert_eq!(global.timeout_session, 7200);
        assert_eq!(global.max_auth_failures, 5);
        assert!(global.log_comments);
        assert_eq!(global.ban_command, "/usr/local/bin/ban-ip.sh {ip}");
        assert!(global.expose_session_id);
    }

    #[test]
    fn existing_config_parses_with_new_defaults() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/test-config.toml"
        );
        let config = Config::from_file(path).expect("load existing fixture");
        // New fields should have their defaults
        assert_eq!(config.global.timeout_session, 3600);
        assert_eq!(config.global.max_auth_failures, 3);
        assert!(!config.global.log_comments);
        assert_eq!(config.global.ban_command, "");
    }

    // --- Phase 3 : [auth.tokens] ---

    #[test]
    fn config_without_auth_section() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"

        "#;
        let config = Config::from_str(toml_str).expect("parse");
        assert!(config.auth.is_none());
    }

    #[test]
    fn config_with_auth_tokens() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"


            [auth.tokens.runner-forge]
            secret = "b64:Zm9vYmFy"
            level = "ops"

            [auth.tokens.agent-claude]
            secret = "b64:c2VjcmV0"
            level = "read"
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let auth = config.auth.expect("auth present");
        assert_eq!(auth.tokens.len(), 2);
        assert!(auth.tokens.contains_key("runner-forge"));
        assert!(auth.tokens.contains_key("agent-claude"));
        assert_eq!(auth.tokens["runner-forge"].level, TrustLevel::Ops);
        assert_eq!(auth.tokens["agent-claude"].level, TrustLevel::Read);
    }

    #[test]
    fn config_auth_token_invalid_base64() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"


            [auth.tokens.bad-token]
            secret = "b64:!!!invalid"
            level = "ops"
        "#;
        let result = Config::from_str(toml_str);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("base64"));
    }

    #[test]
    fn config_auth_token_missing_b64_prefix() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"


            [auth.tokens.no-prefix]
            secret = "just-plain-text"
            level = "ops"
        "#;
        let result = Config::from_str(toml_str);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("b64:"));
    }

    #[test]
    fn validate_allows_domain_placeholder() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test cmd"
            level = "ops"
            timeout = 30
            execute = "/bin/echo {domain}"

        "#;
        let result = Config::from_str(toml_str);
        assert!(result.is_ok());
    }

    // --- Phase 5 : tags de visibilité (ADR 0008) ---

    #[test]
    fn test_config_with_action_tags() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.forgejo]
            description = "Forge Git"

            [domains.forgejo.actions.backup]
            description = "Backup"
            level = "ops"
            timeout = 600
            execute = "/usr/local/bin/backup.sh"

            tags = ["forgejo", "infra"]
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let action = &config.domains["forgejo"].actions["backup"];
        assert_eq!(action.tags, vec!["forgejo", "infra"]);
    }

    #[test]
    fn test_config_without_tags_retrocompat() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"

        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let action = &config.domains["test"].actions["cmd"];
        assert!(action.tags.is_empty());
    }

    #[test]
    fn test_config_empty_tag_rejected() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"

            tags = ["forgejo", ""]
        "#;
        let result = Config::from_str(toml_str);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("empty tag"));
    }

    #[test]
    fn test_config_invalid_tag_format() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"

            tags = ["valid-tag", "invalid tag!"]
        "#;
        let result = Config::from_str(toml_str);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("invalid tag"));
    }

    #[test]
    fn test_config_tags_deduplicated() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"

            tags = ["forgejo", "forgejo", "infra"]
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let action = &config.domains["test"].actions["cmd"];
        assert_eq!(action.tags.len(), 2);
        assert!(action.tags.contains(&"forgejo".to_string()));
        assert!(action.tags.contains(&"infra".to_string()));
    }

    #[test]
    fn test_config_tags_normalized_lowercase() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"

            tags = ["Forgejo", "INFRA"]
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let action = &config.domains["test"].actions["cmd"];
        assert_eq!(action.tags, vec!["forgejo", "infra"]);
    }

    #[test]
    fn test_config_token_tags() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"

            tags = ["forgejo"]

            [auth.tokens.runner-forge]
            secret = "b64:Zm9vYmFy"
            level = "ops"
            tags = ["forgejo", "infra"]
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let auth = config.auth.expect("auth present");
        let token = &auth.tokens["runner-forge"];
        assert_eq!(token.tags, vec!["forgejo", "infra"]);
    }

    // --- Phase 5.5 : challenge_nonce (ADR 0010) ---

    #[test]
    fn test_config_challenge_nonce_true() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"


            [auth]
            challenge_nonce = true

            [auth.tokens.runner]
            secret = "b64:Zm9vYmFy"
            level = "ops"
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let auth = config.auth.expect("auth present");
        assert!(auth.challenge_nonce);
    }

    #[test]
    fn test_config_challenge_nonce_false() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"


            [auth]
            challenge_nonce = false

            [auth.tokens.runner]
            secret = "b64:Zm9vYmFy"
            level = "ops"
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let auth = config.auth.expect("auth present");
        assert!(!auth.challenge_nonce);
    }

    #[test]
    fn test_config_challenge_nonce_default_false() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"


            [auth.tokens.runner]
            secret = "b64:Zm9vYmFy"
            level = "ops"
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let auth = config.auth.expect("auth present");
        assert!(!auth.challenge_nonce);
    }

    // --- Phase 5.5 : arguments nommés (ADR 0009) ---

    #[test]
    fn test_config_named_args_inline_tables() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.deploy]
            description = "Deploy"
            level = "ops"
            timeout = 30
            execute = "/bin/echo {tag}"

            [domains.test.actions.deploy.args]
            tag = { type = "enum", values = ["latest", "stable"] }
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let action = &config.domains["test"].actions["deploy"];
        assert_eq!(action.args.len(), 1);
        assert!(action.args.contains_key("tag"));
        let arg = &action.args["tag"];
        assert_eq!(arg.arg_type, "enum");
        assert_eq!(
            arg.values,
            Some(vec!["latest".to_string(), "stable".to_string()])
        );
    }

    #[test]
    fn test_config_named_args_with_default() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.deploy]
            description = "Deploy"
            level = "ops"
            timeout = 30
            execute = "/bin/echo {tag}"

            [domains.test.actions.deploy.args]
            tag = { type = "enum", values = ["latest", "stable"], default = "latest" }
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let arg = &config.domains["test"].actions["deploy"].args["tag"];
        assert_eq!(arg.default, Some("latest".to_string()));
    }

    #[test]
    fn test_config_named_args_default_enum_invalid() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.deploy]
            description = "Deploy"
            level = "ops"
            timeout = 30
            execute = "/bin/echo {tag}"

            [domains.test.actions.deploy.args]
            tag = { type = "enum", values = ["latest", "stable"], default = "invalid" }
        "#;
        let result = Config::from_str(toml_str);
        assert!(result.is_err());
        let err = result.expect_err("should fail").to_string();
        assert!(
            err.contains("default"),
            "error should mention default: {err}"
        );
    }

    #[test]
    fn test_config_named_args_invalid_name() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.deploy]
            description = "Deploy"
            level = "ops"
            timeout = 30
            execute = "/bin/echo {bad name}"

            [domains.test.actions.deploy.args]
            "bad name" = { type = "string" }
        "#;
        let result = Config::from_str(toml_str);
        assert!(result.is_err());
        let err = result.expect_err("should fail").to_string();
        assert!(
            err.contains("argument name"),
            "error should mention arg name: {err}"
        );
    }

    #[test]
    fn test_config_no_args_retrocompat() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        assert!(config.domains["test"].actions["cmd"].args.is_empty());
    }

    // --- is_visible_to ---

    /// Helper pour creer une `ActionConfig` minimale pour les tests `is_visible_to`
    fn make_action(level: TrustLevel, tags: Vec<String>) -> ActionConfig {
        ActionConfig {
            description: "test".to_string(),
            level,
            timeout: None,
            execute: "/bin/echo".to_string(),
            args: std::collections::BTreeMap::new(),
            tags,
            max_body_size: 65536,
        }
    }

    #[test]
    fn is_visible_to_level_and_tags_match() {
        let action = make_action(TrustLevel::Ops, vec!["forgejo".to_string()]);
        let tags = vec!["forgejo".to_string(), "infra".to_string()];
        assert!(action.is_visible_to(TrustLevel::Ops, &tags));
        assert!(action.is_visible_to(TrustLevel::Admin, &tags));
    }

    #[test]
    fn is_visible_to_level_too_low() {
        let action = make_action(TrustLevel::Ops, vec!["forgejo".to_string()]);
        let tags = vec!["forgejo".to_string()];
        assert!(!action.is_visible_to(TrustLevel::Read, &tags));
    }

    #[test]
    fn is_visible_to_tags_mismatch() {
        let action = make_action(TrustLevel::Read, vec!["forgejo".to_string()]);
        let tags = vec!["infra".to_string()];
        assert!(!action.is_visible_to(TrustLevel::Admin, &tags));
    }

    #[test]
    fn is_visible_to_no_action_tags_public() {
        let action = make_action(TrustLevel::Read, vec![]);
        let tags = vec!["forgejo".to_string()];
        assert!(action.is_visible_to(TrustLevel::Read, &tags));
        assert!(action.is_visible_to(TrustLevel::Admin, &tags));
    }

    #[test]
    fn is_visible_to_empty_effective_tags_no_action_tags() {
        let action = make_action(TrustLevel::Read, vec![]);
        let tags: Vec<String> = vec![];
        assert!(action.is_visible_to(TrustLevel::Read, &tags));
    }

    #[test]
    fn is_visible_to_empty_effective_tags_with_action_tags() {
        let action = make_action(TrustLevel::Read, vec!["forgejo".to_string()]);
        let tags: Vec<String> = vec![];
        assert!(!action.is_visible_to(TrustLevel::Admin, &tags));
    }

    #[test]
    fn test_config_token_without_tags() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Test"
            level = "read"
            timeout = 10
            execute = "/bin/echo"


            [auth.tokens.runner-forge]
            secret = "b64:Zm9vYmFy"
            level = "ops"
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let auth = config.auth.expect("auth present");
        let token = &auth.tokens["runner-forge"];
        assert!(token.tags.is_empty());
    }

    // --- Phase 9 : free = true (ADR 0012 D5) ---

    #[test]
    fn test_argdef_free_true_without_values() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.notify]
            description = "Notify"
            level = "ops"
            timeout = 30
            execute = "/bin/echo {message}"

            [domains.test.actions.notify.args]
            message = { free = true }
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let arg = &config.domains["test"].actions["notify"].args["message"];
        assert!(arg.free);
    }

    #[test]
    fn test_argdef_free_true_with_default() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.notify]
            description = "Notify"
            level = "ops"
            timeout = 30
            execute = "/bin/echo {message}"

            [domains.test.actions.notify.args]
            message = { free = true, default = "hello" }
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let arg = &config.domains["test"].actions["notify"].args["message"];
        assert!(arg.free);
        assert_eq!(arg.default, Some("hello".to_string()));
    }

    #[test]
    fn test_argdef_free_false_default() {
        let toml_str = r#"
            type = "enum"
            values = ["a", "b"]
        "#;
        let arg: ArgDef = toml::from_str(toml_str).expect("parse argdef");
        assert!(!arg.free);
    }

    // --- Phase 9 : max_body_size (ADR 0012 D2) ---

    #[test]
    fn test_action_max_body_size_default() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.deploy]
            description = "Deploy"
            level = "ops"
            timeout = 30
            execute = "/bin/echo"
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let action = &config.domains["test"].actions["deploy"];
        assert_eq!(action.max_body_size, 65536);
    }

    #[test]
    fn test_action_max_body_size_custom() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.deploy]
            description = "Deploy"
            level = "ops"
            timeout = 30
            execute = "/bin/echo"
            max_body_size = 10485760
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let action = &config.domains["test"].actions["deploy"];
        assert_eq!(action.max_body_size, 10485760);
    }

    #[test]
    fn test_action_max_body_size_zero_rejected() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.deploy]
            description = "Deploy"
            level = "ops"
            timeout = 30
            execute = "/bin/echo"
            max_body_size = 0
        "#;
        let result = Config::from_str(toml_str);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("max_body_size"));
    }

    // --- Phase 9 : validation croisée free + type ---

    #[test]
    fn test_argdef_free_true_with_type_enum_accepted() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Cmd"
            level = "ops"
            timeout = 30
            execute = "/bin/echo {arg}"

            [domains.test.actions.cmd.args]
            arg = { free = true, type = "enum", values = ["a", "b"] }
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let arg = &config.domains["test"].actions["cmd"].args["arg"];
        assert!(arg.free);
    }

    #[test]
    fn test_argdef_free_true_without_type() {
        let toml_str = r#"
            [global]
            log_file = "/var/log/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.cmd]
            description = "Cmd"
            level = "ops"
            timeout = 30
            execute = "/bin/echo {msg}"

            [domains.test.actions.cmd.args]
            msg = { free = true }
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let arg = &config.domains["test"].actions["cmd"].args["msg"];
        assert!(arg.free);
    }
}
