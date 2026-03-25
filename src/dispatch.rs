use crate::config::{Config, TrustLevel};
use std::collections::HashMap;

const MAX_COMMAND_LEN: usize = 4096;
const MAX_TOKEN_LEN: usize = 256;

/// Structured error type for dispatch operations
#[derive(Debug)]
pub(crate) enum DispatchError {
    EmptyCommand,
    CommandTooLong { len: usize, max: usize },
    TokenTooLong { len: usize, max: usize },
    UnclosedQuote(char),
    UnknownDomain(String),
    UnknownAction { domain: String, action: String },
    InvalidSyntax(String),
    ArgumentError(String),
    Unauthorized(String),
    TagMismatch,
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchError::EmptyCommand => write!(f, "empty command"),
            DispatchError::CommandTooLong { len, max } => {
                write!(f, "command too long ({len} chars, max {max})")
            }
            DispatchError::TokenTooLong { len, max } => {
                write!(f, "token too long ({len} chars, max {max})")
            }
            DispatchError::UnclosedQuote(ch) => write!(f, "unclosed {ch} quote"),
            DispatchError::UnknownDomain(d) => write!(f, "unknown domain '{d}'"),
            DispatchError::UnknownAction { domain, action } => {
                write!(f, "unknown action '{action}' in domain '{domain}'")
            }
            DispatchError::InvalidSyntax(msg)
            | DispatchError::ArgumentError(msg)
            | DispatchError::Unauthorized(msg) => write!(f, "{msg}"),
            DispatchError::TagMismatch => write!(f, "access denied (tag mismatch)"),
        }
    }
}

impl std::error::Error for DispatchError {}

/// Concept 2.0.3 — Identite : qui demande
#[derive(Debug, Clone)]
pub(crate) struct Identity {
    pub(crate) level: TrustLevel,
    pub(crate) ssh_client: Option<String>,
}

impl Identity {
    pub(crate) fn from_args(args: &[&str], ssh_client: Option<&str>) -> Self {
        let mut level = TrustLevel::Read;
        for arg in args {
            if let Some(val) = arg.strip_prefix("--level=") {
                if let Ok(l) = val.parse::<TrustLevel>() {
                    level = l;
                }
            }
        }
        Identity {
            level,
            ssh_client: ssh_client.map(std::string::ToString::to_string),
        }
    }

    /// Create a new Identity with a different trust level (for effective identity after auth)
    pub(crate) fn with_level(&self, level: TrustLevel) -> Self {
        Identity {
            level,
            ssh_client: self.ssh_client.clone(),
        }
    }
}

/// Parse et valide la commande brute (supporte les guillemets pour les espaces)
#[must_use = "parsing result must be checked"]
pub(crate) fn parse_command(raw: &str) -> Result<Vec<String>, DispatchError> {
    let raw = raw.trim();

    if raw.is_empty() {
        return Err(DispatchError::EmptyCommand);
    }

    if raw.len() > MAX_COMMAND_LEN {
        return Err(DispatchError::CommandTooLong {
            len: raw.len(),
            max: MAX_COMMAND_LEN,
        });
    }

    let tokens = tokenize_with_quotes(raw)?;

    if tokens.is_empty() {
        return Err(DispatchError::EmptyCommand);
    }

    // Verification longueur de chaque token
    for token in &tokens {
        if token.len() > MAX_TOKEN_LEN {
            return Err(DispatchError::TokenTooLong {
                len: token.len(),
                max: MAX_TOKEN_LEN,
            });
        }
    }

    Ok(tokens)
}

/// Tokenize a command string, supporting double and single quotes for spaces in args
fn tokenize_with_quotes(input: &str) -> Result<Vec<String>, DispatchError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_double_quote = false;
    let mut in_single_quote = false;
    let chars = input.chars();

    for ch in chars {
        match ch {
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            ' ' | '\t' if !in_double_quote && !in_single_quote => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if in_double_quote {
        return Err(DispatchError::UnclosedQuote('"'));
    }
    if in_single_quote {
        return Err(DispatchError::UnclosedQuote('\''));
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

/// Resout domaine + action + arguments depuis les tokens
#[must_use = "resolution result must be checked"]
pub(crate) fn resolve_command(
    config: &Config,
    tokens: &[String],
) -> Result<(String, String, HashMap<String, String>), DispatchError> {
    if tokens.len() < 2 {
        return Err(DispatchError::InvalidSyntax(
            "expected: <domain> <action> [args...]".to_string(),
        ));
    }

    // PANIC-SAFE: checked tokens.len() >= 2 above
    let domain_id = &tokens[0];
    let action_id = &tokens[1];
    let arg_tokens = &tokens[2..];

    // Resolution du domaine
    let domain = config
        .domains
        .get(domain_id.as_str())
        .ok_or_else(|| DispatchError::UnknownDomain(domain_id.clone()))?;

    // Resolution de l'action
    let action =
        domain
            .actions
            .get(action_id.as_str())
            .ok_or_else(|| DispatchError::UnknownAction {
                domain: domain_id.clone(),
                action: action_id.clone(),
            })?;

    // Resolution des arguments nommés (ADR 0009)
    let args = resolve_arguments(arg_tokens, &action.args, domain_id, action_id)?;

    Ok((domain_id.clone(), action_id.clone(), args))
}

/// Resolve named arguments: key=value parsing, defaults, validation (ADR 0009)
fn resolve_arguments(
    arg_tokens: &[String],
    arg_defs: &std::collections::BTreeMap<String, crate::config::ArgDef>,
    domain_id: &str,
    action_id: &str,
) -> Result<HashMap<String, String>, DispatchError> {
    let mut args = HashMap::new();

    for token in arg_tokens {
        // Split on first '='
        let (name, value) = match token.find('=') {
            // PANIC-SAFE: pos is a valid byte index returned by find('='), and pos+1 is safe because '=' is ASCII (1 byte)
            Some(pos) => (&token[..pos], &token[pos + 1..]),
            None => {
                return Err(DispatchError::ArgumentError(format!(
                    "argument '{token}' must use key=value syntax \
                     (positional arguments not supported)"
                )));
            }
        };

        // Check name is defined
        if !arg_defs.contains_key(name) {
            return Err(DispatchError::ArgumentError(format!(
                "unknown argument '{name}' for '{domain_id} {action_id}'"
            )));
        }

        // Check for duplicates
        if args.contains_key(name) {
            return Err(DispatchError::ArgumentError(format!(
                "duplicate argument '{name}'"
            )));
        }

        args.insert(name.to_string(), value.to_string());
    }

    // Apply defaults and check mandatory args
    for (name, arg_def) in arg_defs {
        if !args.contains_key(name.as_str()) {
            match &arg_def.default {
                Some(default_val) => {
                    args.insert(name.clone(), default_val.clone());
                }
                None => {
                    return Err(DispatchError::ArgumentError(format!(
                        "missing required argument '{name}' for '{domain_id} {action_id}'"
                    )));
                }
            }
        }
    }

    // Validate enum values (skip for free args — ADR 0012 D5)
    for (name, value) in &args {
        if let Some(arg_def) = arg_defs.get(name) {
            if arg_def.free {
                continue;
            }
            if arg_def.arg_type == "enum" {
                if let Some(ref values) = arg_def.values {
                    if !values.contains(value) {
                        return Err(DispatchError::ArgumentError(format!(
                            "invalid value '{value}' for argument '{name}' \
                             (allowed: {})",
                            values.join(", ")
                        )));
                    }
                }
            }
        }
    }

    Ok(args)
}

/// Verifie l'intersection entre les tags effectifs et les tags de l'action
/// Action sans tags → true (publique). Intersection vide → false.
#[must_use = "tag check result must be used"]
pub(crate) fn check_tags(effective_tags: &[String], action_tags: &[String]) -> bool {
    if action_tags.is_empty() {
        return true;
    }
    effective_tags.iter().any(|t| action_tags.contains(t))
}

/// Verifie que l'identite a les tags requis et le niveau suffisant pour l'action.
/// Les tags sont verifies en premier : un token cross-domaine est rejete avant
/// meme de tester le niveau, ce qui empeche toute fuite d'information sur les
/// niveaux requis d'un domaine auquel on n'a pas acces.
#[must_use = "authorization result must be checked"]
pub(crate) fn check_authorization(
    identity: &Identity,
    action: &crate::config::ActionConfig,
    effective_tags: &[String],
) -> Result<(), DispatchError> {
    if !check_tags(effective_tags, &action.tags) {
        return Err(DispatchError::TagMismatch);
    }
    if identity.level < action.level {
        return Err(DispatchError::Unauthorized(format!(
            "insufficient level ({} required, {} granted)",
            action.level, identity.level
        )));
    }
    Ok(())
}

/// Transpose la commande : substitue {domain} et {arg} par leurs valeurs.
/// Each token in the template is substituted individually — a value containing
/// spaces stays as a single token (security fix for `free = true` args).
pub(crate) fn transpose_command(
    execute_template: &str,
    domain_id: &str,
    args: &HashMap<String, String>,
) -> Vec<String> {
    execute_template
        .split_whitespace()
        .map(|token| {
            let mut t = token.replace("{domain}", domain_id);
            for (name, value) in args {
                t = t.replace(&format!("{{{name}}}"), value);
            }
            t
        })
        .collect()
}
