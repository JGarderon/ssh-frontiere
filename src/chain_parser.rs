// --- AST and parser for command block chaining (protocol v2) ---

/// Structured error type for chain parsing operations
#[derive(Debug)]
pub(crate) enum ChainError {
    /// Empty command block
    EmptyBlock,
    /// Syntax error in the command block (unclosed quotes, parens, etc.)
    SyntaxError(String),
}

impl std::fmt::Display for ChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainError::EmptyBlock => write!(f, "empty command block"),
            ChainError::SyntaxError(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ChainError {}

// --- AST ---

/// Mode de sequence : strict (arret au premier echec) ou permissif (continue toujours)
#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum SequenceMode {
    /// Operateur `;` — arret au premier echec
    Strict,
    /// Operateur `&` — continue quoi qu'il arrive
    Permissive,
}

/// Noeud de l'arbre syntaxique du bloc commande multi-lignes (protocole v2)
#[derive(Debug, PartialEq)]
pub(crate) enum CommandNode {
    /// Une commande simple : "mastodon healthcheck"
    Single(String),
    /// Sequence de commandes avec mode strict ou permissif
    Sequence(Vec<CommandNode>, SequenceMode),
    /// Rattrapage : si left echoue, execute right
    Recovery(Box<CommandNode>, Box<CommandNode>),
}

// --- Tokens internes pour le parseur ---

#[derive(Debug, PartialEq)]
enum Token {
    Command(String),
    Semicolon,
    Ampersand,
    Pipe,
    OpenParen,
    CloseParen,
}

// --- Parsing du bloc commande ---

/// Parse un bloc commande multi-lignes en AST.
///
/// Pre-traitement : les sauts de ligne hors guillemets deviennent ";".
/// Operateurs : ";" (strict), "&" (permissif), "|" (rattrapage), "()" (groupement).
/// Priorite : () > | > & = ; (meme priorite, gauche a droite).
#[must_use = "parsing result must be checked"]
pub(crate) fn parse_block(block: &str) -> Result<CommandNode, ChainError> {
    let normalized = normalize_newlines(block);
    let trimmed = normalized.trim();

    if trimmed.is_empty() {
        return Err(ChainError::EmptyBlock);
    }

    let tokens = tokenize_block(trimmed)?;

    if tokens.is_empty() {
        return Err(ChainError::EmptyBlock);
    }

    parse_sequence(&tokens, 0).map(|(node, _)| node)
}

/// Remplace les sauts de ligne hors guillemets par " ; "
fn normalize_newlines(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_double_quote = false;
    let mut in_single_quote = false;

    for ch in input.chars() {
        match ch {
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                result.push(ch);
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                result.push(ch);
            }
            '\n' if !in_double_quote && !in_single_quote => {
                result.push_str(" ; ");
            }
            _ => result.push(ch),
        }
    }

    result
}

/// Tokenise le bloc en une sequence de Token (Command, operateurs, parentheses)
fn tokenize_block(input: &str) -> Result<Vec<Token>, ChainError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_double_quote = false;
    let mut in_single_quote = false;
    let mut paren_depth: i32 = 0;

    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        match ch {
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                current.push(ch);
                i += 1;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                current.push(ch);
                i += 1;
            }
            _ if in_double_quote || in_single_quote => {
                current.push(ch);
                i += 1;
            }
            '(' => {
                flush_command(&mut current, &mut tokens);
                tokens.push(Token::OpenParen);
                paren_depth += 1;
                i += 1;
            }
            ')' => {
                if paren_depth <= 0 {
                    return Err(ChainError::SyntaxError(
                        "unexpected closing parenthesis".to_string(),
                    ));
                }
                flush_command(&mut current, &mut tokens);
                tokens.push(Token::CloseParen);
                paren_depth -= 1;
                i += 1;
            }
            ';' => {
                flush_command(&mut current, &mut tokens);
                tokens.push(Token::Semicolon);
                i += 1;
            }
            '&' => {
                flush_command(&mut current, &mut tokens);
                tokens.push(Token::Ampersand);
                i += 1;
            }
            '|' => {
                flush_command(&mut current, &mut tokens);
                tokens.push(Token::Pipe);
                i += 1;
            }
            _ => {
                current.push(ch);
                i += 1;
            }
        }
    }

    if in_double_quote {
        return Err(ChainError::SyntaxError("unclosed double quote".to_string()));
    }
    if in_single_quote {
        return Err(ChainError::SyntaxError("unclosed single quote".to_string()));
    }
    if paren_depth != 0 {
        return Err(ChainError::SyntaxError("unclosed parenthesis".to_string()));
    }

    flush_command(&mut current, &mut tokens);

    // Nettoyage : supprimer les operateurs consecutifs en tete et en queue
    strip_leading_trailing_operators(&mut tokens);

    Ok(tokens)
}

/// Pousse la commande accumulee dans la liste de tokens (si non vide)
fn flush_command(current: &mut String, tokens: &mut Vec<Token>) {
    let taken = std::mem::take(current);
    let trimmed = taken.trim();
    if !trimmed.is_empty() {
        tokens.push(Token::Command(trimmed.to_string()));
    }
}

/// Nettoie la liste de tokens : supprime les operateurs en tete/queue
/// et fusionne les operateurs consecutifs (garde le premier)
fn strip_leading_trailing_operators(tokens: &mut Vec<Token>) {
    // Supprimer en tete (drain O(n) au lieu de remove(0) en boucle O(n^2))
    let skip = tokens
        .iter()
        .take_while(|t| matches!(t, Token::Semicolon | Token::Ampersand | Token::Pipe))
        .count();
    if skip > 0 {
        tokens.drain(..skip);
    }
    // Supprimer en queue
    while matches!(
        tokens.last(),
        Some(Token::Semicolon | Token::Ampersand | Token::Pipe)
    ) {
        tokens.pop();
    }
    // Fusionner les operateurs consecutifs (garder le premier)
    let mut i = 0;
    while i + 1 < tokens.len() {
        let curr_is_op = is_operator(&tokens[i]);
        let next_is_op = is_operator(&tokens[i + 1]);
        if curr_is_op && next_is_op {
            tokens.remove(i + 1);
        } else {
            i += 1;
        }
    }
}

/// Retourne true si le token est un operateur (; & |)
fn is_operator(token: &Token) -> bool {
    matches!(token, Token::Semicolon | Token::Ampersand | Token::Pipe)
}

// --- Parseur recursif descendant ---
//
// Grammaire (priorite croissante) :
//   sequence  = recovery ((';' | '&') recovery)*
//   recovery  = primary ('|' primary)*
//   primary   = '(' sequence ')' | command

/// Parse une sequence (niveau le plus bas de priorite : ; et &)
fn parse_sequence(tokens: &[Token], pos: usize) -> Result<(CommandNode, usize), ChainError> {
    let (first, mut pos) = parse_recovery(tokens, pos)?;
    let mut nodes = vec![first];
    let mut ops: Vec<SequenceMode> = Vec::new();

    loop {
        if pos >= tokens.len() {
            break;
        }
        let mode = match tokens[pos] {
            Token::Semicolon => SequenceMode::Strict,
            Token::Ampersand => SequenceMode::Permissive,
            _ => break,
        };
        ops.push(mode);
        pos += 1;
        if pos >= tokens.len() || matches!(tokens[pos], Token::CloseParen) {
            break;
        }
        let (node, new_pos) = parse_recovery(tokens, pos)?;
        nodes.push(node);
        pos = new_pos;
    }

    if nodes.len() == 1 {
        return Ok((nodes.remove(0), pos));
    }

    // Determiner si la sequence est homogene (tout strict ou tout permissif)
    // ou heterogene (mixte) — dans ce cas on construit gauche a droite
    let node = build_sequence_tree(nodes, ops);
    Ok((node, pos))
}

/// Construit l'arbre de sequence gauche a droite en respectant les operateurs
fn build_sequence_tree(mut nodes: Vec<CommandNode>, ops: Vec<SequenceMode>) -> CommandNode {
    // Si tous les operateurs sont identiques, une seule Sequence
    if ops.iter().all(|&o| o == ops[0]) {
        return CommandNode::Sequence(nodes, ops[0]);
    }

    // Mixte : construire gauche a droite en imbriquant
    let mut result = nodes.remove(0);
    for op in ops {
        let right = nodes.remove(0);
        result = CommandNode::Sequence(vec![result, right], op);
    }
    result
}

/// Parse un rattrapage (priorite moyenne : |)
fn parse_recovery(tokens: &[Token], pos: usize) -> Result<(CommandNode, usize), ChainError> {
    let (left, mut pos) = parse_primary(tokens, pos)?;

    if pos < tokens.len() && tokens[pos] == Token::Pipe {
        pos += 1;
        let (right, pos) = parse_recovery(tokens, pos)?;
        Ok((CommandNode::Recovery(Box::new(left), Box::new(right)), pos))
    } else {
        Ok((left, pos))
    }
}

/// Parse un primaire (priorite haute : () ou commande simple)
/// D6: parentheses are passthrough — they return the inner node directly (no Group wrapper)
fn parse_primary(tokens: &[Token], pos: usize) -> Result<(CommandNode, usize), ChainError> {
    if pos >= tokens.len() {
        return Err(ChainError::SyntaxError(
            "unexpected end of block".to_string(),
        ));
    }

    match &tokens[pos] {
        Token::OpenParen => {
            let (inner, pos) = parse_sequence(tokens, pos + 1)?;
            if pos >= tokens.len() || tokens[pos] != Token::CloseParen {
                return Err(ChainError::SyntaxError("unclosed parenthesis".to_string()));
            }
            Ok((inner, pos + 1))
        }
        Token::Command(cmd) => Ok((CommandNode::Single(cmd.clone()), pos + 1)),
        _ => Err(ChainError::SyntaxError(format!(
            "unexpected token at position {pos}"
        ))),
    }
}
