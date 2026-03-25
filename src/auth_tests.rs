#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::auth::AuthContext;
    use crate::config::{Config, TrustLevel};
    use crate::orchestrator::{execute_ban_command, extract_ip_from_ssh_client};

    fn test_config_with_auth() -> Config {
        let toml_str = r#"
            [global]
            log_file = "/tmp/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.echo]
            description = "Echo"
            level = "read"
            timeout = 10
            execute = "/bin/echo"

            [auth]
            challenge_nonce = true

            [auth.tokens.runner]
            secret = "b64:c2VjcmV0"
            level = "ops"
        "#;
        Config::from_str(toml_str).expect("test config with auth")
    }

    fn test_config_no_auth() -> Config {
        let toml_str = r#"
            [global]
            log_file = "/tmp/test.json"

            [domains.test]
            description = "Test"

            [domains.test.actions.echo]
            description = "Echo"
            level = "read"
            timeout = 10
            execute = "/bin/echo"
        "#;
        Config::from_str(toml_str).expect("test config no auth")
    }

    fn test_config_with_tagged_tokens() -> Config {
        let toml_str = r#"
            [global]
            log_file = "/tmp/test.json"

            [domains.forgejo]
            description = "Forge"

            [domains.forgejo.actions.backup]
            description = "Backup"
            level = "ops"
            timeout = 10
            execute = "/bin/echo"

            tags = ["forgejo"]

            [domains.infra]
            description = "Infra"

            [domains.infra.actions.healthcheck]
            description = "Health"
            level = "read"
            timeout = 10
            execute = "/bin/echo"


            [auth.tokens.runner-forge]
            secret = "b64:c2VjcmV0"
            level = "ops"
            tags = ["forgejo", "infra"]

            [auth.tokens.agent-mastodon]
            secret = "b64:c2VjcmV0Mg=="
            level = "ops"
            tags = ["mastodon"]

            [auth.tokens.no-tags-token]
            secret = "b64:c2VjcmV0Mw=="
            level = "read"
        "#;
        Config::from_str(toml_str).expect("test config with tagged tokens")
    }

    // --- Auth context tests ---

    #[test]
    fn auth_context_starts_at_base_level() {
        let ctx = AuthContext::new(TrustLevel::Read, 3);
        assert_eq!(ctx.effective_level, TrustLevel::Read);
        assert_eq!(ctx.failures, 0);
        assert!(!ctx.is_locked_out());
    }

    #[test]
    fn auth_context_valid_auth_elevates_level() {
        let config = test_config_with_auth();
        let mut ctx = AuthContext::new(TrustLevel::Read, 3);
        let secret = b"secret";
        let nonce = [0x42u8; 16];
        let proof = crate::crypto::compute_proof(secret, &nonce);
        let result = ctx.validate_auth(&config, "runner", &proof, Some(&nonce));
        assert!(result.is_ok());
        assert_eq!(ctx.effective_level, TrustLevel::Ops);
    }

    #[test]
    fn auth_context_invalid_proof_increments_failures() {
        let config = test_config_with_auth();
        let mut ctx = AuthContext::new(TrustLevel::Read, 3);
        let nonce = [0x42u8; 16];
        let result = ctx.validate_auth(&config, "runner", "bad-proof", Some(&nonce));
        assert!(result.is_err());
        assert_eq!(ctx.failures, 1);
        assert_eq!(ctx.effective_level, TrustLevel::Read);
    }

    #[test]
    fn auth_context_lockout_after_max_failures() {
        let config = test_config_with_auth();
        let mut ctx = AuthContext::new(TrustLevel::Read, 3);
        let nonce = [0x42u8; 16];
        for _ in 0..3 {
            let _ = ctx.validate_auth(&config, "runner", "bad", Some(&nonce));
        }
        assert!(ctx.is_locked_out());
    }

    #[test]
    fn auth_context_unknown_token_fails() {
        let config = test_config_with_auth();
        let mut ctx = AuthContext::new(TrustLevel::Read, 3);
        let nonce = [0x42u8; 16];
        let result = ctx.validate_auth(&config, "nonexistent", "proof", Some(&nonce));
        assert!(result.is_err());
    }

    #[test]
    fn auth_context_no_auth_configured_fails() {
        let config = test_config_no_auth();
        let mut ctx = AuthContext::new(TrustLevel::Read, 3);
        let nonce = [0x42u8; 16];
        let result = ctx.validate_auth(&config, "any", "proof", Some(&nonce));
        assert!(result.is_err());
    }

    #[test]
    fn auth_context_base_level_higher_than_token() {
        let config = test_config_with_auth();
        let mut ctx = AuthContext::new(TrustLevel::Admin, 3);
        let secret = b"secret";
        let nonce = [0x42u8; 16];
        let proof = crate::crypto::compute_proof(secret, &nonce);
        let result = ctx.validate_auth(&config, "runner", &proof, Some(&nonce));
        assert!(result.is_ok());
        // Admin > Ops, so effective stays Admin
        assert_eq!(ctx.effective_level, TrustLevel::Admin);
    }

    #[test]
    fn auth_context_nonce_replay_rejected() {
        let config = test_config_with_auth();
        let mut ctx = AuthContext::new(TrustLevel::Read, 3);
        let secret = b"secret";
        let nonce1 = [0x42u8; 16];
        let nonce2 = [0xABu8; 16];
        let proof = crate::crypto::compute_proof(secret, &nonce1);

        let result = ctx.validate_auth(&config, "runner", &proof, Some(&nonce1));
        assert!(result.is_ok());

        let mut ctx2 = AuthContext::new(TrustLevel::Read, 3);
        let result = ctx2.validate_auth(&config, "runner", &proof, Some(&nonce2));
        assert!(result.is_err());
    }

    // --- Phase 5 : effective_tags on AuthContext (ADR 0008) ---

    #[test]
    fn test_auth_effective_tags_empty_before_auth() {
        let ctx = AuthContext::new(TrustLevel::Read, 3);
        assert!(ctx.effective_tags.is_empty());
    }

    #[test]
    fn test_auth_effective_tags_from_token() {
        let config = test_config_with_tagged_tokens();
        let mut ctx = AuthContext::new(TrustLevel::Read, 3);
        let secret = b"secret";
        let nonce = [0x42u8; 16];
        let proof = crate::crypto::compute_proof(secret, &nonce);
        let result = ctx.validate_auth(&config, "runner-forge", &proof, Some(&nonce));
        assert!(result.is_ok());
        assert_eq!(ctx.effective_tags.len(), 2);
        assert!(ctx.effective_tags.contains(&"forgejo".to_string()));
        assert!(ctx.effective_tags.contains(&"infra".to_string()));
    }

    #[test]
    fn test_auth_effective_tags_union_multiple() {
        let config = test_config_with_tagged_tokens();
        let mut ctx = AuthContext::new(TrustLevel::Read, 3);

        let secret1 = b"secret";
        let nonce1 = [0x42u8; 16];
        let proof1 = crate::crypto::compute_proof(secret1, &nonce1);
        let _ = ctx.validate_auth(&config, "runner-forge", &proof1, Some(&nonce1));

        let secret2 = b"secret2";
        let nonce2 = [0xABu8; 16];
        let proof2 = crate::crypto::compute_proof(secret2, &nonce2);
        let _ = ctx.validate_auth(&config, "agent-mastodon", &proof2, Some(&nonce2));

        assert_eq!(ctx.effective_tags.len(), 3);
        assert!(ctx.effective_tags.contains(&"forgejo".to_string()));
        assert!(ctx.effective_tags.contains(&"infra".to_string()));
        assert!(ctx.effective_tags.contains(&"mastodon".to_string()));
    }

    // --- Phase 5.5 : validate_auth nonce optionnel (ADR 0010) ---

    #[test]
    fn test_validate_auth_simple_mode_correct() {
        let config = test_config_with_auth();
        let mut ctx = AuthContext::new(TrustLevel::Read, 3);
        let secret = b"secret";
        let proof = crate::crypto::compute_simple_proof(secret);
        let result = ctx.validate_auth(&config, "runner", &proof, None);
        assert!(result.is_ok());
        assert_eq!(ctx.effective_level, TrustLevel::Ops);
    }

    #[test]
    fn test_validate_auth_simple_mode_incorrect() {
        let config = test_config_with_auth();
        let mut ctx = AuthContext::new(TrustLevel::Read, 3);
        let result = ctx.validate_auth(&config, "runner", "bad-proof", None);
        assert!(result.is_err());
        assert_eq!(ctx.failures, 1);
    }

    #[test]
    fn test_validate_auth_nonce_mode_regression() {
        let config = test_config_with_auth();
        let mut ctx = AuthContext::new(TrustLevel::Read, 3);
        let secret = b"secret";
        let nonce = [0x42u8; 16];
        let proof = crate::crypto::compute_proof(secret, &nonce);
        let result = ctx.validate_auth(&config, "runner", &proof, Some(&nonce));
        assert!(result.is_ok());
        assert_eq!(ctx.effective_level, TrustLevel::Ops);
    }

    #[test]
    fn test_validate_auth_simple_mode_tags_merged() {
        let config = test_config_with_tagged_tokens();
        let mut ctx = AuthContext::new(TrustLevel::Read, 3);
        let secret = b"secret";
        let proof = crate::crypto::compute_simple_proof(secret);
        let result = ctx.validate_auth(&config, "runner-forge", &proof, None);
        assert!(result.is_ok());
        assert_eq!(ctx.effective_tags.len(), 2);
        assert!(ctx.effective_tags.contains(&"forgejo".to_string()));
        assert!(ctx.effective_tags.contains(&"infra".to_string()));
    }

    #[test]
    fn test_auth_effective_tags_no_token_tags() {
        let config = test_config_with_tagged_tokens();
        let mut ctx = AuthContext::new(TrustLevel::Read, 3);
        let secret = b"secret3";
        let nonce = [0x42u8; 16];
        let proof = crate::crypto::compute_proof(secret, &nonce);
        let result = ctx.validate_auth(&config, "no-tags-token", &proof, Some(&nonce));
        assert!(result.is_ok());
        assert!(ctx.effective_tags.is_empty());
    }

    // --- Ban command tests ---

    #[test]
    fn extract_ip_from_ssh_client_standard() {
        assert_eq!(
            extract_ip_from_ssh_client("192.168.1.100 54321 22"),
            "192.168.1.100"
        );
    }

    #[test]
    fn extract_ip_from_ssh_client_ipv6() {
        assert_eq!(extract_ip_from_ssh_client("::1 54321 22"), "::1");
    }

    #[test]
    fn extract_ip_from_ssh_client_ip_only() {
        assert_eq!(extract_ip_from_ssh_client("10.0.0.1"), "10.0.0.1");
    }

    #[test]
    fn test_execute_ban_command_empty_is_noop() {
        execute_ban_command("", "192.168.1.1");
    }

    #[test]
    fn test_execute_ban_command_with_placeholder() {
        execute_ban_command("/bin/true {ip}", "10.0.0.1");
    }

    #[test]
    fn test_execute_ban_command_ip_with_spaces_sanitized() {
        execute_ban_command("/bin/echo {ip}", "192.168.1.1 12345 22");
    }
}
