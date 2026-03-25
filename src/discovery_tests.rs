#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::discovery::handle_discovery;
    use crate::dispatch::Identity;

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

    // --- Discovery (help/list) ---

    #[test]
    fn discovery_help_full() {
        let config = test_config();
        let identity = Identity::from_args(&["--level=ops"], None);
        let result = handle_discovery(&config, &["help".to_string()], &identity, &[]);
        assert!(result.is_ok());
        let json_str = result.expect("help");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid json");
        assert!(parsed["domains"].is_object());
        assert!(parsed["domains"]["forgejo"].is_object());
        assert!(parsed["domains"]["infra"].is_object());
    }

    #[test]
    fn discovery_help_domain() {
        let config = test_config();
        let identity = Identity::from_args(&["--level=ops"], None);
        let result = handle_discovery(
            &config,
            &["help".to_string(), "forgejo".to_string()],
            &identity,
            &[],
        );
        assert!(result.is_ok());
        let json_str = result.expect("help domain");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid json");
        assert!(parsed["actions"].is_object());
        assert!(parsed["actions"]["backup-config"].is_object());
    }

    #[test]
    fn discovery_help_action() {
        let config = test_config();
        let identity = Identity::from_args(&["--level=ops"], None);
        let result = handle_discovery(
            &config,
            &["help".to_string(), "backup-config".to_string()],
            &identity,
            &[],
        );
        assert!(result.is_ok());
        let json_str = result.expect("help action");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid json");
        assert!(parsed["description"].is_string());
        assert!(parsed["level"].is_string());
    }

    #[test]
    fn discovery_list() {
        let config = test_config();
        let identity = Identity::from_args(&["--level=ops"], None);
        let result = handle_discovery(&config, &["list".to_string()], &identity, &[]);
        assert!(result.is_ok());
        let json_str = result.expect("list");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid json");
        assert!(parsed["actions"].is_array());
    }

    #[test]
    fn discovery_list_filtered_by_level() {
        let config = test_config();
        let identity = Identity::from_args(&["--level=read"], None);
        let result = handle_discovery(&config, &["list".to_string()], &identity, &[]);
        assert!(result.is_ok());
        let json_str = result.expect("list");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid json");
        let actions = parsed["actions"].as_array().expect("array");
        // read level should only see healthcheck (level=read), not ops actions
        for action in actions {
            assert_ne!(action["level"].as_str(), Some("ops"));
        }
    }

    #[test]
    fn discovery_help_filtered_by_level() {
        let config = test_config();
        let identity = Identity::from_args(&["--level=read"], None);
        let result = handle_discovery(&config, &["help".to_string()], &identity, &[]);
        assert!(result.is_ok());
        let json_str = result.expect("help");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid json");
        // infra domain should be visible (has read action)
        assert!(parsed["domains"]["infra"].is_object());
        // forgejo should not be visible (all ops actions)
        assert!(parsed["domains"].get("forgejo").is_none());
    }

    // --- Discovery filtered by tags (ADR 0008) ---

    #[test]
    fn test_help_filters_by_tags() {
        let config = test_config_with_tags();
        let identity = Identity::from_args(&["--level=ops"], None);
        let effective_tags = vec!["mastodon".to_string()];
        let result = handle_discovery(&config, &["help".to_string()], &identity, &effective_tags);
        assert!(result.is_ok());
        let json_str = result.expect("help");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid json");
        // mastodon domain should be visible (tag match)
        assert!(parsed["domains"]["mastodon"].is_object());
        // forgejo actions are tagged "forgejo" -> not matching "mastodon" -> domain omitted
        assert!(parsed["domains"].get("forgejo").is_none());
        // infra has no tags -> always visible
        assert!(parsed["domains"]["infra"].is_object());
    }

    #[test]
    fn test_help_domain_fully_masked() {
        let config = test_config_with_tags();
        let identity = Identity::from_args(&["--level=ops"], None);
        let effective_tags = vec!["mastodon".to_string()];
        let result = handle_discovery(&config, &["help".to_string()], &identity, &effective_tags);
        assert!(result.is_ok());
        let json_str = result.expect("help");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid json");
        // forgejo domain entirely masked (no matching tags)
        assert!(parsed["domains"].get("forgejo").is_none());
    }

    #[test]
    fn test_list_filters_by_tags() {
        let config = test_config_with_tags();
        let identity = Identity::from_args(&["--level=ops"], None);
        let effective_tags = vec!["mastodon".to_string()];
        let result = handle_discovery(&config, &["list".to_string()], &identity, &effective_tags);
        assert!(result.is_ok());
        let json_str = result.expect("list");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid json");
        let actions = parsed["actions"].as_array().expect("array");
        // Should contain mastodon.healthcheck and infra.status, but NOT forgejo.backup
        let domains: Vec<&str> = actions
            .iter()
            .map(|a| a["domain"].as_str().unwrap_or(""))
            .collect();
        assert!(domains.contains(&"mastodon"));
        assert!(domains.contains(&"infra"));
        assert!(!domains.contains(&"forgejo"));
    }

    #[test]
    fn test_help_target_masked_domain() {
        let config = test_config_with_tags();
        let identity = Identity::from_args(&["--level=ops"], None);
        let effective_tags = vec!["mastodon".to_string()];
        let result = handle_discovery(
            &config,
            &["help".to_string(), "forgejo".to_string()],
            &identity,
            &effective_tags,
        );
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("unknown domain or action"));
    }

    // --- Phase 9 : discovery shows max_body_size and free ---

    #[test]
    fn discovery_json_includes_max_body_size_and_free() {
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
            max_body_size = 131072

            [domains.test.actions.notify.args]
            message = { free = true }
        "#;
        let config = Config::from_str(toml_str).expect("parse");
        let identity = Identity::from_args(&["--level=ops"], None);
        let result = handle_discovery(
            &config,
            &["help".to_string(), "test".to_string()],
            &identity,
            &[],
        );
        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.expect("ok")).expect("parse");
        let action = &json["actions"]["notify"];
        assert_eq!(action["max_body_size"], 131072);
        assert_eq!(action["args"]["message"]["free"], true);
    }
}
