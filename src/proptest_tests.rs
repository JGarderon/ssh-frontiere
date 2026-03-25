#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::crypto::{base64_decode, constant_time_eq, sha256_bytes};
    use crate::dispatch::parse_command;
    use crate::protocol::parse_line;

    proptest! {
        // 1. parse_line never panics on arbitrary input
        #[test]
        fn parse_line_never_panics(input in ".*") {
            let _ = parse_line(&input);
        }

        // 2. parse_command (tokenize) never panics on arbitrary input
        #[test]
        fn parse_command_never_panics(input in ".*") {
            let _ = parse_command(&input);
        }

        // 3. SHA-256 always produces exactly 32 bytes
        #[test]
        fn sha256_always_32_bytes(data in proptest::collection::vec(any::<u8>(), 0..1024)) {
            let digest = sha256_bytes(&data);
            prop_assert_eq!(digest.len(), 32);
        }

        // 4. base64 decode of valid base64 never panics
        #[test]
        fn base64_decode_valid_never_panics(input in "[A-Za-z0-9+/]{0,100}={0,3}") {
            let _ = base64_decode(&input);
        }

        // 5. constant_time_eq is reflexive
        #[test]
        fn constant_time_eq_reflexive(data in proptest::collection::vec(any::<u8>(), 0..256)) {
            prop_assert!(constant_time_eq(&data, &data));
        }

        // 6. read_body default mode never panics (ADR 0012)
        #[test]
        fn read_body_default_never_panics(input in ".*") {
            let mut reader = std::io::BufReader::new(input.as_bytes());
            let _ = crate::protocol::read_body(
                &mut reader,
                &crate::protocol::BodyMode::Default,
                65536,
            );
        }

        // 7. transpose_command preserves token count of template (ADR 0012 fix)
        #[test]
        fn transpose_preserves_token_count(
            template in "[a-z/]{1,20}( [a-z]{1,10}){0,5}",
            value in ".{0,50}"
        ) {
            let expected_tokens = template.split_whitespace().count();
            let mut args = std::collections::HashMap::new();
            args.insert("arg".to_string(), value);
            let result = crate::dispatch::transpose_command(&template, "dom", &args);
            prop_assert_eq!(result.len(), expected_tokens);
        }

        // 8. resolve_arguments with free=true accepts any non-empty string
        #[test]
        fn resolve_free_accepts_any_value(value in "[^\x00]{1,100}") {
            let toml_str = r#"
                [global]
                log_file = "/tmp/test.json"

                [domains.test]
                description = "Test"

                [domains.test.actions.cmd]
                description = "Cmd"
                level = "read"
                timeout = 10
                execute = "/bin/echo {msg}"

                [domains.test.actions.cmd.args]
                msg = { free = true }
            "#;
            let config = crate::config::Config::from_str(toml_str).expect("parse");
            let tokens = vec![
                "test".to_string(),
                "cmd".to_string(),
                format!("msg={value}"),
            ];
            let result = crate::dispatch::resolve_command(&config, &tokens);
            prop_assert!(result.is_ok(), "free arg should accept: {:?}", value);
        }
    }
}
