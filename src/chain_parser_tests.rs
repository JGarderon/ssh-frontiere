#[cfg(test)]
mod tests {
    use crate::chain_parser::*;

    // --- Tests parse_block : commande simple ---

    #[test]
    fn parse_single_command() {
        let node = parse_block("mastodon healthcheck").expect("parse");
        assert_eq!(
            node,
            CommandNode::Single("mastodon healthcheck".to_string())
        );
    }

    #[test]
    fn parse_single_command_trimmed() {
        let node = parse_block("  mastodon healthcheck  ").expect("parse");
        assert_eq!(
            node,
            CommandNode::Single("mastodon healthcheck".to_string())
        );
    }

    // --- Tests parse_block : sequentiel strict (;) ---

    #[test]
    fn parse_strict_sequence() {
        let node = parse_block("cmd1 ; cmd2").expect("parse");
        assert_eq!(
            node,
            CommandNode::Sequence(
                vec![
                    CommandNode::Single("cmd1".to_string()),
                    CommandNode::Single("cmd2".to_string()),
                ],
                SequenceMode::Strict
            )
        );
    }

    #[test]
    fn parse_strict_sequence_three_commands() {
        let node = parse_block("cmd1 ; cmd2 ; cmd3").expect("parse");
        assert_eq!(
            node,
            CommandNode::Sequence(
                vec![
                    CommandNode::Single("cmd1".to_string()),
                    CommandNode::Single("cmd2".to_string()),
                    CommandNode::Single("cmd3".to_string()),
                ],
                SequenceMode::Strict
            )
        );
    }

    // --- Tests parse_block : saut de ligne = ; ---

    #[test]
    fn parse_newline_as_strict_sequence() {
        let node = parse_block("cmd1\ncmd2").expect("parse");
        assert_eq!(
            node,
            CommandNode::Sequence(
                vec![
                    CommandNode::Single("cmd1".to_string()),
                    CommandNode::Single("cmd2".to_string()),
                ],
                SequenceMode::Strict
            )
        );
    }

    #[test]
    fn parse_multiple_newlines() {
        let node = parse_block("cmd1\n\ncmd2").expect("parse");
        assert_eq!(
            node,
            CommandNode::Sequence(
                vec![
                    CommandNode::Single("cmd1".to_string()),
                    CommandNode::Single("cmd2".to_string()),
                ],
                SequenceMode::Strict
            )
        );
    }

    // --- Tests parse_block : sequentiel permissif (&) ---

    #[test]
    fn parse_permissive_sequence() {
        let node = parse_block("cmd1 & cmd2").expect("parse");
        assert_eq!(
            node,
            CommandNode::Sequence(
                vec![
                    CommandNode::Single("cmd1".to_string()),
                    CommandNode::Single("cmd2".to_string()),
                ],
                SequenceMode::Permissive
            )
        );
    }

    #[test]
    fn parse_permissive_sequence_three_commands() {
        let node = parse_block("cmd1 & cmd2 & cmd3").expect("parse");
        assert_eq!(
            node,
            CommandNode::Sequence(
                vec![
                    CommandNode::Single("cmd1".to_string()),
                    CommandNode::Single("cmd2".to_string()),
                    CommandNode::Single("cmd3".to_string()),
                ],
                SequenceMode::Permissive
            )
        );
    }

    // --- Tests parse_block : rattrapage (|) ---

    #[test]
    fn parse_recovery() {
        let node = parse_block("cmd1 | cmd2").expect("parse");
        assert_eq!(
            node,
            CommandNode::Recovery(
                Box::new(CommandNode::Single("cmd1".to_string())),
                Box::new(CommandNode::Single("cmd2".to_string())),
            )
        );
    }

    #[test]
    fn parse_recovery_chain() {
        // | est associatif a droite : cmd1 | cmd2 | cmd3 = cmd1 | (cmd2 | cmd3)
        let node = parse_block("cmd1 | cmd2 | cmd3").expect("parse");
        assert_eq!(
            node,
            CommandNode::Recovery(
                Box::new(CommandNode::Single("cmd1".to_string())),
                Box::new(CommandNode::Recovery(
                    Box::new(CommandNode::Single("cmd2".to_string())),
                    Box::new(CommandNode::Single("cmd3".to_string())),
                )),
            )
        );
    }

    // --- Tests parse_block : groupement () --- D6: parens are passthrough

    #[test]
    fn parse_group_simple() {
        // D6: (cmd1) returns inner directly, no Group wrapper
        let node = parse_block("(cmd1)").expect("parse");
        assert_eq!(node, CommandNode::Single("cmd1".to_string()));
    }

    #[test]
    fn parse_group_with_sequence() {
        // D6: (cmd1 ; cmd2) returns the Sequence directly
        let node = parse_block("(cmd1 ; cmd2)").expect("parse");
        assert_eq!(
            node,
            CommandNode::Sequence(
                vec![
                    CommandNode::Single("cmd1".to_string()),
                    CommandNode::Single("cmd2".to_string()),
                ],
                SequenceMode::Strict
            )
        );
    }

    // --- Tests parse_block : combinaisons complexes ---

    #[test]
    fn parse_group_recovery() {
        // (cmd1 ; cmd2) | cmd3 — D6: group is passthrough
        let node = parse_block("(cmd1 ; cmd2) | cmd3").expect("parse");
        assert_eq!(
            node,
            CommandNode::Recovery(
                Box::new(CommandNode::Sequence(
                    vec![
                        CommandNode::Single("cmd1".to_string()),
                        CommandNode::Single("cmd2".to_string()),
                    ],
                    SequenceMode::Strict
                )),
                Box::new(CommandNode::Single("cmd3".to_string())),
            )
        );
    }

    #[test]
    fn parse_recovery_higher_priority_than_sequence() {
        // cmd1 ; cmd2 | cmd3 = cmd1 ; (cmd2 | cmd3)
        let node = parse_block("cmd1 ; cmd2 | cmd3").expect("parse");
        assert_eq!(
            node,
            CommandNode::Sequence(
                vec![
                    CommandNode::Single("cmd1".to_string()),
                    CommandNode::Recovery(
                        Box::new(CommandNode::Single("cmd2".to_string())),
                        Box::new(CommandNode::Single("cmd3".to_string())),
                    ),
                ],
                SequenceMode::Strict
            )
        );
    }

    #[test]
    fn parse_sequence_then_recovery() {
        // cmd1 & cmd2 | cmd3 = cmd1 & (cmd2 | cmd3)
        let node = parse_block("cmd1 & cmd2 | cmd3").expect("parse");
        assert_eq!(
            node,
            CommandNode::Sequence(
                vec![
                    CommandNode::Single("cmd1".to_string()),
                    CommandNode::Recovery(
                        Box::new(CommandNode::Single("cmd2".to_string())),
                        Box::new(CommandNode::Single("cmd3".to_string())),
                    ),
                ],
                SequenceMode::Permissive
            )
        );
    }

    #[test]
    fn parse_nested_groups() {
        // D6: ((cmd1)) — both layers of parens are passthrough
        let node = parse_block("((cmd1))").expect("parse");
        assert_eq!(node, CommandNode::Single("cmd1".to_string()));
    }

    // --- Tests parse_block : operateurs dans les guillemets ---

    #[test]
    fn parse_operators_in_double_quotes_are_content() {
        let node = parse_block(r#"cmd1 "arg with ; inside""#).expect("parse");
        assert_eq!(
            node,
            CommandNode::Single(r#"cmd1 "arg with ; inside""#.to_string())
        );
    }

    #[test]
    fn parse_operators_in_single_quotes_are_content() {
        let node = parse_block("cmd1 'arg with | inside'").expect("parse");
        assert_eq!(
            node,
            CommandNode::Single("cmd1 'arg with | inside'".to_string())
        );
    }

    #[test]
    fn parse_ampersand_in_quotes_is_content() {
        let node = parse_block(r#"cmd1 "a & b""#).expect("parse");
        assert_eq!(node, CommandNode::Single(r#"cmd1 "a & b""#.to_string()));
    }

    #[test]
    fn parse_parens_in_quotes_are_content() {
        let node = parse_block(r#"cmd1 "value (test)""#).expect("parse");
        assert_eq!(
            node,
            CommandNode::Single(r#"cmd1 "value (test)""#.to_string())
        );
    }

    #[test]
    fn parse_newline_in_double_quotes_is_content() {
        let input = "cmd1 \"line1\nline2\"";
        let node = parse_block(input).expect("parse");
        assert_eq!(
            node,
            CommandNode::Single("cmd1 \"line1\nline2\"".to_string())
        );
    }

    // --- Tests parse_block : erreurs ---

    #[test]
    fn parse_empty_block() {
        let result = parse_block("");
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("empty"));
    }

    #[test]
    fn parse_whitespace_only_block() {
        let result = parse_block("   \n  \n  ");
        assert!(result.is_err());
    }

    #[test]
    fn parse_unclosed_parenthesis() {
        let result = parse_block("(cmd1 ; cmd2");
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("parenthesis"));
    }

    #[test]
    fn parse_unexpected_close_parenthesis() {
        let result = parse_block("cmd1) ; cmd2");
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("parenthesis"));
    }

    #[test]
    fn parse_unclosed_double_quote() {
        let result = parse_block(r#"cmd1 "unclosed"#);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("quote"));
    }

    #[test]
    fn parse_unclosed_single_quote() {
        let result = parse_block("cmd1 'unclosed");
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("quote"));
    }

    // --- Tests parse_block : cas limites ---

    #[test]
    fn parse_trailing_semicolon_ignored() {
        let node = parse_block("cmd1 ;").expect("parse");
        assert_eq!(node, CommandNode::Single("cmd1".to_string()));
    }

    #[test]
    fn parse_leading_semicolon_ignored() {
        let node = parse_block("; cmd1").expect("parse");
        assert_eq!(node, CommandNode::Single("cmd1".to_string()));
    }

    #[test]
    fn parse_trailing_newline_ignored() {
        let node = parse_block("cmd1\n").expect("parse");
        assert_eq!(node, CommandNode::Single("cmd1".to_string()));
    }
}
