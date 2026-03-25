#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::protocol::*;
    use std::io::Cursor;

    // --- Line parser tests ---

    #[test]
    fn parse_configure_capabilities() {
        let line = parse_line("+ capabilities rbac, session, help").expect("parse");
        assert_eq!(
            line,
            ProtocolLine::Configure(Directive::Capabilities(vec![
                "rbac".to_string(),
                "session".to_string(),
                "help".to_string(),
            ]))
        );
    }

    #[test]
    fn parse_configure_challenge() {
        let line = parse_line("+ challenge nonce=abcdef0123456789").expect("parse");
        assert_eq!(
            line,
            ProtocolLine::Configure(Directive::Challenge {
                nonce: "abcdef0123456789".to_string(),
            })
        );
    }

    #[test]
    fn parse_configure_auth() {
        let line = parse_line("+ auth token=runner proof=abc123def").expect("parse");
        assert_eq!(
            line,
            ProtocolLine::Configure(Directive::Auth {
                token: "runner".to_string(),
                proof: "abc123def".to_string(),
            })
        );
    }

    #[test]
    fn parse_configure_session() {
        let line = parse_line("+ session keepalive").expect("parse");
        assert_eq!(line, ProtocolLine::Configure(Directive::Session));
    }

    #[test]
    fn parse_configure_unknown() {
        let line = parse_line("+ future-directive something").expect("parse");
        assert_eq!(
            line,
            ProtocolLine::Configure(Directive::Unknown("future-directive something".to_string()))
        );
    }

    #[test]
    fn parse_comment() {
        let line = parse_line("# this is a comment").expect("parse");
        assert_eq!(line, ProtocolLine::Comment("this is a comment".to_string()));
    }

    #[test]
    fn parse_comment_empty() {
        let line = parse_line("#").expect("parse");
        assert_eq!(line, ProtocolLine::Comment(String::new()));
    }

    #[test]
    fn parse_dollar_prefix_is_text() {
        // v2: "$ xxx" is no longer Command, it's Text
        let line = parse_line("$ forgejo healthcheck").expect("parse");
        assert_eq!(
            line,
            ProtocolLine::Text("$ forgejo healthcheck".to_string())
        );
    }

    #[test]
    fn parse_response_prefix_is_text() {
        // v2: "> xxx" is no longer Response, it's Text (server doesn't receive > from client)
        let json = r#"{"status_code": 0, "status_message": "ok"}"#;
        let line = parse_line(&format!("> {json}")).expect("parse");
        assert_eq!(line, ProtocolLine::Text(format!("> {json}")));
    }

    #[test]
    fn parse_empty_line() {
        let line = parse_line("").expect("parse");
        assert_eq!(line, ProtocolLine::EmptyLine);
    }

    #[test]
    fn parse_empty_line_with_newline() {
        let line = parse_line("\n").expect("parse");
        assert_eq!(line, ProtocolLine::EmptyLine);
    }

    #[test]
    fn parse_no_prefix_is_text() {
        // v2: lines without a prefix are Text, not errors
        let line = parse_line("no prefix here").expect("parse");
        assert_eq!(line, ProtocolLine::Text("no prefix here".to_string()));
    }

    #[test]
    fn parse_end_of_block() {
        // v2: "." alone on a line is EndOfBlock
        let line = parse_line(".").expect("parse");
        assert_eq!(line, ProtocolLine::EndOfBlock);
    }

    #[test]
    fn parse_end_of_block_with_newline() {
        let line = parse_line(".\n").expect("parse");
        assert_eq!(line, ProtocolLine::EndOfBlock);
    }

    #[test]
    fn parse_dot_with_content_is_text() {
        // ".something" is not EndOfBlock, it's Text
        let line = parse_line(".something").expect("parse");
        assert_eq!(line, ProtocolLine::Text(".something".to_string()));
    }

    #[test]
    fn parse_line_too_long() {
        let long = format!("+ {}", "x".repeat(5000));
        let result = parse_line(&long);
        assert!(matches!(result, Err(ProtocolError::LineTooLong(_))));
    }

    // --- Banner tests ---

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
        Config::from_str(toml_str).expect("test config")
    }

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


            [auth.tokens.runner]
            secret = "b64:c2VjcmV0"
            level = "ops"
        "#;
        Config::from_str(toml_str).expect("test config with auth")
    }

    #[test]
    fn banner_without_auth() {
        let config = test_config_no_auth();
        let mut buf = Vec::new();
        write_banner(&mut buf, &config, None, None, false).expect("write banner");
        let output = String::from_utf8(buf).expect("utf8");
        // v2: banner uses #> and +> prefixes
        assert!(output.contains("#> ssh-frontiere"));
        assert!(output.contains("+> capabilities session, help"));
        assert!(!output.contains("rbac"));
        assert!(!output.contains("+> challenge"));
    }

    #[test]
    fn banner_with_auth() {
        let config = test_config_with_auth();
        let mut buf = Vec::new();
        write_banner(&mut buf, &config, Some("abcdef1234567890"), None, false)
            .expect("write banner");
        let output = String::from_utf8(buf).expect("utf8");
        // v2: banner uses #> and +> prefixes
        assert!(output.contains("#> ssh-frontiere"));
        assert!(output.contains("rbac, session, help"));
        assert!(output.contains("+> challenge nonce=abcdef1234567890"));
    }

    #[test]
    fn banner_with_session_id_exposed() {
        let config = test_config_no_auth();
        let mut buf = Vec::new();
        write_banner(&mut buf, &config, None, Some("sid-12345"), true).expect("write banner");
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains("+> session sid-12345"));
    }

    #[test]
    fn banner_with_session_id_not_exposed() {
        let config = test_config_no_auth();
        let mut buf = Vec::new();
        write_banner(&mut buf, &config, None, Some("sid-12345"), false).expect("write banner");
        let output = String::from_utf8(buf).expect("utf8");
        assert!(!output.contains("sid-12345"));
    }

    #[test]
    fn banner_help_hint_uses_server_comment_prefix() {
        let config = test_config_no_auth();
        let mut buf = Vec::new();
        write_banner(&mut buf, &config, None, None, false).expect("write banner");
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains("#> type \"help\" for available commands"));
    }

    // --- Phase 5.5 : bannière conditionnelle nonce (ADR 0010) ---

    #[test]
    fn banner_simple_mode_no_challenge_line() {
        let config = test_config_with_auth();
        let mut buf = Vec::new();
        // nonce = None → mode simple, pas de ligne +> challenge
        write_banner(&mut buf, &config, None, None, false).expect("write banner");
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains("rbac, session, help"));
        assert!(!output.contains("+> challenge"));
    }

    #[test]
    fn banner_nonce_mode_has_challenge_line() {
        let config = test_config_with_auth();
        let mut buf = Vec::new();
        write_banner(&mut buf, &config, Some("abcdef0123456789"), None, false)
            .expect("write banner");
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains("rbac, session, help"));
        assert!(output.contains("+> challenge nonce=abcdef0123456789"));
    }

    #[test]
    fn banner_no_auth_no_rbac() {
        let config = test_config_no_auth();
        let mut buf = Vec::new();
        write_banner(&mut buf, &config, None, None, false).expect("write banner");
        let output = String::from_utf8(buf).expect("utf8");
        assert!(!output.contains("rbac"));
        assert!(!output.contains("+> challenge"));
    }

    // --- Header reading tests ---

    #[test]
    fn read_headers_with_end_of_block() {
        // "." alone signals end of connection, no command
        let input = ".\n";
        let mut reader = Cursor::new(input.as_bytes());
        let (result, first_line) = read_headers(&mut reader).expect("read headers");
        assert!(result.auth_token.is_none());
        assert!(!result.session_mode);
        assert!(result.comments.is_empty());
        assert!(first_line.is_none());
    }

    #[test]
    fn read_headers_with_auth_and_session() {
        // v2: headers end when a Text line is encountered (first command)
        let input = "+ auth token=runner proof=abc123\n+ session keepalive\nforgejo healthcheck\n";
        let mut reader = Cursor::new(input.as_bytes());
        let (result, first_line) = read_headers(&mut reader).expect("read headers");
        assert_eq!(result.auth_token, Some("runner".to_string()));
        assert_eq!(result.auth_proof, Some("abc123".to_string()));
        assert!(result.session_mode);
        assert_eq!(first_line, Some("forgejo healthcheck".to_string()));
    }

    #[test]
    fn read_headers_with_comments() {
        let input = "# hello\n# world\nsome command\n";
        let mut reader = Cursor::new(input.as_bytes());
        let (result, first_line) = read_headers(&mut reader).expect("read headers");
        assert_eq!(result.comments, vec!["hello", "world"]);
        assert_eq!(first_line, Some("some command".to_string()));
    }

    #[test]
    fn read_headers_unknown_directive_ignored() {
        let input = "+ unknown-future foo=bar\nmy command\n";
        let mut reader = Cursor::new(input.as_bytes());
        let (result, first_line) = read_headers(&mut reader).expect("read headers");
        assert!(result.auth_token.is_none());
        assert_eq!(first_line, Some("my command".to_string()));
    }

    #[test]
    fn read_headers_eof_error() {
        let input = "+ auth token=test proof=abc";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_headers(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn read_headers_text_line_ends_headers() {
        // v2: a line without prefix (Text) ends headers and is returned as first_line
        let input = "forgejo healthcheck\n";
        let mut reader = Cursor::new(input.as_bytes());
        let (result, first_line) = read_headers(&mut reader).expect("read headers");
        assert!(result.auth_token.is_none());
        assert!(!result.session_mode);
        assert_eq!(first_line, Some("forgejo healthcheck".to_string()));
    }

    #[test]
    fn read_headers_empty_lines_ignored() {
        // v2: empty lines in headers are ignored (readability)
        let input = "\n\n# comment\n\nmy command\n";
        let mut reader = Cursor::new(input.as_bytes());
        let (result, first_line) = read_headers(&mut reader).expect("read headers");
        assert_eq!(result.comments, vec!["comment"]);
        assert_eq!(first_line, Some("my command".to_string()));
    }

    #[test]
    fn read_headers_dollar_prefix_is_text() {
        // v2: "$ xxx" is a Text line, so it ends headers and returns as first_line
        let input = "$ forgejo healthcheck\n";
        let mut reader = Cursor::new(input.as_bytes());
        let (result, first_line) = read_headers(&mut reader).expect("read headers");
        assert!(result.auth_token.is_none());
        assert_eq!(first_line, Some("$ forgejo healthcheck".to_string()));
    }

    // --- Command block reading tests ---

    #[test]
    fn read_command_block_single_line() {
        // first_line provided, next line is "." (end of block)
        let input = ".\n";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_command_block(&mut reader, Some("forgejo healthcheck".to_string()))
            .expect("read command block");
        assert_eq!(result, Some("forgejo healthcheck".to_string()));
    }

    #[test]
    fn read_command_block_multi_line() {
        // Multi-line block: first_line + additional Text lines + "."
        let input = "second line\nthird line\n.\n";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_command_block(&mut reader, Some("first line".to_string()))
            .expect("read command block");
        assert_eq!(
            result,
            Some("first line\nsecond line\nthird line".to_string())
        );
    }

    #[test]
    fn read_command_block_with_empty_lines() {
        // Empty lines within a block are preserved as newlines
        let input = "\nsecond part\n.\n";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_command_block(&mut reader, Some("first part".to_string()))
            .expect("read command block");
        assert_eq!(result, Some("first part\n\nsecond part".to_string()));
    }

    #[test]
    fn read_command_block_none_first_line() {
        // None first_line means end of connection ("." was received with no command)
        let input = "";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_command_block(&mut reader, None).expect("read command block");
        assert_eq!(result, None);
    }

    #[test]
    fn read_command_block_eof_is_error() {
        // EOF before "." is an error
        let input = "more text";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_command_block(&mut reader, Some("start".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn read_command_block_rejects_header_in_block() {
        // A + directive inside a command block is an error
        let input = "+ auth token=x proof=y\n.\n";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_command_block(&mut reader, Some("start".to_string()));
        assert!(matches!(result, Err(ProtocolError::InvalidLine(_))));
    }

    #[test]
    fn read_command_block_rejects_comment_in_block() {
        // A # comment inside a command block is an error
        let input = "# not allowed\n.\n";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_command_block(&mut reader, Some("start".to_string()));
        assert!(matches!(result, Err(ProtocolError::InvalidLine(_))));
    }

    // --- Response and comment writing tests ---

    #[test]
    fn write_response_format() {
        let mut buf = Vec::new();
        write_response(&mut buf, r#"{"status_code": 0}"#).expect("write");
        let output = String::from_utf8(buf).expect("utf8");
        // ADR 0011: response uses >>> prefix (triple chevron)
        assert_eq!(output, ">>> {\"status_code\": 0}\n");
    }

    #[test]
    fn write_stdout_line_format() {
        let mut buf = Vec::new();
        write_stdout_line(&mut buf, "hello world").expect("write");
        let output = String::from_utf8(buf).expect("utf8");
        assert_eq!(output, ">> hello world\n");
    }

    #[test]
    fn write_stderr_line_format() {
        let mut buf = Vec::new();
        write_stderr_line(&mut buf, "error occurred").expect("write");
        let output = String::from_utf8(buf).expect("utf8");
        assert_eq!(output, ">>! error occurred\n");
    }

    #[test]
    fn write_stdout_line_empty() {
        let mut buf = Vec::new();
        write_stdout_line(&mut buf, "").expect("write");
        let output = String::from_utf8(buf).expect("utf8");
        assert_eq!(output, ">> \n");
    }

    #[test]
    fn write_stderr_line_empty() {
        let mut buf = Vec::new();
        write_stderr_line(&mut buf, "").expect("write");
        let output = String::from_utf8(buf).expect("utf8");
        assert_eq!(output, ">>! \n");
    }

    #[test]
    fn write_comment_format() {
        let mut buf = Vec::new();
        write_comment(&mut buf, "authentication failed (1/3)").expect("write");
        let output = String::from_utf8(buf).expect("utf8");
        // v2: server comment uses #> prefix
        assert_eq!(output, "#> authentication failed (1/3)\n");
    }

    // --- Session input tests ---

    #[test]
    fn session_input_command_block() {
        // v2: Text line starts a command block, "." ends it -> CommandBlock
        let input = "forgejo backup\n.\n";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_session_input(&mut reader).expect("read");
        assert!(
            matches!(result, SessionInput::CommandBlock { block, body: None } if block == "forgejo backup")
        );
    }

    #[test]
    fn session_input_multi_line_command_block() {
        let input = "forgejo backup\n--full\n.\n";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_session_input(&mut reader).expect("read");
        assert!(
            matches!(result, SessionInput::CommandBlock { block, body: None } if block == "forgejo backup\n--full")
        );
    }

    #[test]
    fn session_input_end_of_connection() {
        // v2: "." alone (EndOfBlock) without command = EndOfConnection
        let input = ".\n";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_session_input(&mut reader).expect("read");
        assert!(matches!(result, SessionInput::EndOfConnection));
    }

    #[test]
    fn session_input_eof() {
        let input = "";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_session_input(&mut reader).expect("read");
        assert!(matches!(result, SessionInput::Eof));
    }

    #[test]
    fn session_input_auth_change() {
        let input = "+ auth token=admin proof=xyz\n";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_session_input(&mut reader).expect("read");
        assert!(
            matches!(result, SessionInput::Auth { token, proof } if token == "admin" && proof == "xyz")
        );
    }

    #[test]
    fn session_input_comment() {
        let input = "# my comment\n";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_session_input(&mut reader).expect("read");
        assert!(matches!(result, SessionInput::Comment(text) if text == "my comment"));
    }

    #[test]
    fn session_input_empty_lines_skipped() {
        // Empty lines between commands in session mode are skipped
        let input = "\n\nforgejo healthcheck\n.\n";
        let mut reader = Cursor::new(input.as_bytes());
        let result = read_session_input(&mut reader).expect("read");
        assert!(
            matches!(result, SessionInput::CommandBlock { block, body: None } if block == "forgejo healthcheck")
        );
    }

    // --- TODO-027 : ligne vide optionnelle entre entêtes et commande ---

    #[test]
    fn read_headers_command_without_empty_line() {
        // TODO-027: la commande $ suit directement les entêtes, sans ligne vide
        let input = "+ auth token=runner proof=abc123\n$ forgejo backup\n";
        let mut reader = Cursor::new(input.as_bytes());
        let (result, first_line) = read_headers(&mut reader).expect("read headers");
        assert_eq!(result.auth_token, Some("runner".to_string()));
        assert_eq!(first_line, Some("$ forgejo backup".to_string()));
    }

    #[test]
    fn read_headers_command_with_empty_line() {
        // TODO-027: la ligne vide entre entêtes et commande reste valide (rétrocompatible)
        let input = "+ auth token=runner proof=abc123\n\n$ forgejo backup\n";
        let mut reader = Cursor::new(input.as_bytes());
        let (result, first_line) = read_headers(&mut reader).expect("read headers");
        assert_eq!(result.auth_token, Some("runner".to_string()));
        assert_eq!(first_line, Some("$ forgejo backup".to_string()));
    }

    #[test]
    fn read_headers_session_without_empty_line() {
        // TODO-027: +session keepalive suivi directement de la commande, pas de ligne vide
        let input = "+ session keepalive\n$ test echo\n";
        let mut reader = Cursor::new(input.as_bytes());
        let (result, first_line) = read_headers(&mut reader).expect("read headers");
        assert!(result.session_mode);
        assert_eq!(first_line, Some("$ test echo".to_string()));
    }

    // --- Phase 5.5 : couverture protocol.rs (TODO-023) ---

    #[test]
    fn test_protocol_error_display_io() {
        let err = ProtocolError::IoError("connection reset".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("connection reset"));
    }

    // --- Phase 9 : BodyMode + Directive::Body (ADR 0012 D1) ---

    #[test]
    fn parse_body_default() {
        let line = parse_line("+ body").expect("parse");
        assert_eq!(
            line,
            ProtocolLine::Configure(Directive::Body(BodyMode::Default))
        );
    }

    #[test]
    fn parse_body_size() {
        let line = parse_line("+ body size=1024").expect("parse");
        assert_eq!(
            line,
            ProtocolLine::Configure(Directive::Body(BodyMode::Size(1024)))
        );
    }

    #[test]
    fn parse_body_stop() {
        let line = parse_line("+ body stop=\"FIN\"").expect("parse");
        assert_eq!(
            line,
            ProtocolLine::Configure(Directive::Body(BodyMode::Stop("FIN".to_string())))
        );
    }

    #[test]
    fn parse_body_size_and_stop() {
        let line = parse_line("+ body size=4096 stop=\"---END---\"").expect("parse");
        assert_eq!(
            line,
            ProtocolLine::Configure(Directive::Body(BodyMode::SizeAndStop(
                4096,
                "---END---".to_string()
            )))
        );
    }

    #[test]
    fn parse_body_stop_then_size() {
        let line = parse_line("+ body stop=\"FIN\" size=2048").expect("parse");
        assert_eq!(
            line,
            ProtocolLine::Configure(Directive::Body(BodyMode::SizeAndStop(
                2048,
                "FIN".to_string()
            )))
        );
    }

    // --- Phase 9 : read_body (ADR 0012 D1) ---

    #[test]
    fn read_body_default_multiline() {
        let input = "line 1\nline 2\nline 3\n.\n";
        let mut reader = Cursor::new(input);
        let body = read_body(&mut reader, &BodyMode::Default, 65536).expect("read");
        assert_eq!(body, "line 1\nline 2\nline 3");
    }

    #[test]
    fn read_body_default_empty() {
        let input = ".\n";
        let mut reader = Cursor::new(input);
        let body = read_body(&mut reader, &BodyMode::Default, 65536).expect("read");
        assert_eq!(body, "");
    }

    #[test]
    fn read_body_default_single_line() {
        let input = "hello\n.\n";
        let mut reader = Cursor::new(input);
        let body = read_body(&mut reader, &BodyMode::Default, 65536).expect("read");
        assert_eq!(body, "hello");
    }

    #[test]
    fn read_body_size_exact() {
        let input = "hello world!extra";
        let mut reader = Cursor::new(input);
        let body = read_body(&mut reader, &BodyMode::Size(11), 65536).expect("read");
        assert_eq!(body, "hello world");
    }

    #[test]
    fn read_body_size_zero() {
        let input = "anything";
        let mut reader = Cursor::new(input);
        let body = read_body(&mut reader, &BodyMode::Size(0), 65536).expect("read");
        assert_eq!(body, "");
    }

    #[test]
    fn read_body_size_exceeds_max() {
        let input = "data";
        let mut reader = Cursor::new(input);
        let result = read_body(&mut reader, &BodyMode::Size(100), 50);
        assert!(result.is_err());
        let err = format!("{}", result.expect_err("should fail"));
        assert!(err.contains("too large"));
    }

    #[test]
    fn read_body_size_short_stream() {
        let input = "short";
        let mut reader = Cursor::new(input);
        let result = read_body(&mut reader, &BodyMode::Size(100), 65536);
        assert!(result.is_err());
        let err = format!("{}", result.expect_err("should fail"));
        assert!(err.contains("EOF") || err.contains("eof"));
    }

    #[test]
    fn read_body_stop_custom() {
        let input = "line 1\nline 2\n---END---\n";
        let mut reader = Cursor::new(input);
        let body =
            read_body(&mut reader, &BodyMode::Stop("---END---".to_string()), 65536).expect("read");
        assert_eq!(body, "line 1\nline 2");
    }

    #[test]
    fn read_body_stop_not_a_prefix() {
        let input = "x---END---x\n---END---\n";
        let mut reader = Cursor::new(input);
        let body =
            read_body(&mut reader, &BodyMode::Stop("---END---".to_string()), 65536).expect("read");
        assert_eq!(body, "x---END---x");
    }

    #[test]
    fn read_body_combined_stop_first() {
        let input = "line 1\nFIN\n";
        let mut reader = Cursor::new(input);
        let body = read_body(
            &mut reader,
            &BodyMode::SizeAndStop(65536, "FIN".to_string()),
            65536,
        )
        .expect("read");
        assert_eq!(body, "line 1");
    }

    #[test]
    fn read_body_combined_size_first() {
        let input = "abcdefghij";
        let mut reader = Cursor::new(input);
        let body = read_body(
            &mut reader,
            &BodyMode::SizeAndStop(5, "FIN".to_string()),
            65536,
        )
        .expect("read");
        assert_eq!(body, "abcde");
    }

    #[test]
    fn read_body_default_exceeds_max() {
        let input = "aaaa\nbbbb\n.\n";
        let mut reader = Cursor::new(input);
        let result = read_body(&mut reader, &BodyMode::Default, 5);
        assert!(result.is_err());
        let err = format!("{}", result.expect_err("should fail"));
        assert!(err.contains("too large"));
    }

    #[test]
    fn read_body_stop_exceeds_max() {
        let input = "aaaa\nbbbb\nFIN\n";
        let mut reader = Cursor::new(input);
        let result = read_body(&mut reader, &BodyMode::Stop("FIN".to_string()), 5);
        assert!(result.is_err());
    }

    // --- Phase 9 : HeadersResult body_mode ---

    #[test]
    fn headers_with_body_mode_default() {
        let input = "+ body\nhello world\n.\n";
        let mut reader = Cursor::new(input);
        let (headers, first_line) = read_headers(&mut reader).expect("read headers");
        assert_eq!(headers.body_mode, Some(BodyMode::Default));
        assert_eq!(first_line, Some("hello world".to_string()));
    }

    #[test]
    fn headers_without_body() {
        let input = "hello world\n.\n";
        let mut reader = Cursor::new(input);
        let (headers, first_line) = read_headers(&mut reader).expect("read headers");
        assert!(headers.body_mode.is_none());
        assert_eq!(first_line, Some("hello world".to_string()));
    }

    #[test]
    fn headers_with_body_size() {
        let input = "+ body size=1024\ntest cmd\n.\n";
        let mut reader = Cursor::new(input);
        let (headers, _) = read_headers(&mut reader).expect("read headers");
        assert_eq!(headers.body_mode, Some(BodyMode::Size(1024)));
    }

    // --- Phase 9 : read_session_input + body ---

    #[test]
    fn session_input_body_then_command() {
        let input = "+ body\ntest cmd\n.\nline1\nline2\n.\n";
        let mut reader = Cursor::new(input);
        let result = read_session_input(&mut reader).expect("read");
        match result {
            SessionInput::CommandBlock { block, body } => {
                assert_eq!(block, "test cmd");
                assert_eq!(body, Some("line1\nline2".to_string()));
            }
            other => panic!("expected CommandBlock, got: {other:?}"),
        }
    }

    #[test]
    fn session_input_no_body() {
        let input = "test cmd\n.\n";
        let mut reader = Cursor::new(input);
        let result = read_session_input(&mut reader).expect("read");
        match result {
            SessionInput::CommandBlock { block, body } => {
                assert_eq!(block, "test cmd");
                assert!(body.is_none());
            }
            other => panic!("expected CommandBlock, got: {other:?}"),
        }
    }
}
