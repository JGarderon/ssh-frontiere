use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

/// Structured error type for configuration loading and validation
#[derive(Debug)]
pub(crate) enum ConfigError {
    /// File I/O error (file not found, permission denied)
    Io(String),
    /// TOML parsing error (syntax)
    Parse(String),
    /// Semantic validation error (missing fields, invalid values)
    Validation(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "config not found: {e}"),
            ConfigError::Parse(e) => write!(f, "invalid TOML: {e}"),
            ConfigError::Validation(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Concept 2.0.4 — Niveaux de confiance RBAC
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TrustLevel {
    Read,
    Ops,
    Admin,
}

impl fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrustLevel::Read => write!(f, "read"),
            TrustLevel::Ops => write!(f, "ops"),
            TrustLevel::Admin => write!(f, "admin"),
        }
    }
}

impl FromStr for TrustLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read" => Ok(TrustLevel::Read),
            "ops" => Ok(TrustLevel::Ops),
            "admin" => Ok(TrustLevel::Admin),
            other => Err(format!("invalid trust level: '{other}'")),
        }
    }
}

/// Definition d'un argument d'action (ADR 0009 — nom = clé TOML)
/// `free = true` (ADR 0012 D5) : argument texte libre, valeur arbitraire
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ArgDef {
    #[serde(rename = "type", default)]
    pub(crate) arg_type: String,
    #[serde(default)]
    pub(crate) values: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) sensitive: bool,
    #[serde(default)]
    pub(crate) default: Option<String>,
    /// ADR 0012 D5 — texte libre : accepte toute valeur sans contrainte
    #[serde(default)]
    pub(crate) free: bool,
}

/// Concept 2.0.2 — Action dans un domaine
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ActionConfig {
    pub(crate) description: String,
    pub(crate) level: TrustLevel,
    pub(crate) timeout: Option<u64>,
    pub(crate) execute: String,
    #[serde(default)]
    pub(crate) args: BTreeMap<String, ArgDef>,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    /// ADR 0012 D2 — taille max du body (octets), defaut 64 Ko
    #[serde(default = "default_max_body_size")]
    pub(crate) max_body_size: usize,
}

impl ActionConfig {
    /// Verifie si l'action est visible pour un niveau de confiance et des tags donnes.
    /// Action sans tags = publique (visible pour tous les niveaux suffisants).
    /// Sinon, au moins un tag effectif doit correspondre aux tags de l'action.
    #[must_use = "visibility check result must be used"]
    pub(crate) fn is_visible_to(&self, level: TrustLevel, tags: &[String]) -> bool {
        level >= self.level && (self.tags.is_empty() || tags.iter().any(|t| self.tags.contains(t)))
    }
}

/// Concept 2.0.1 — Domaine : perimetre fonctionnel
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DomainConfig {
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) actions: BTreeMap<String, ActionConfig>,
}

/// Section [global] de la configuration
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GlobalConfig {
    pub(crate) log_file: String,
    #[serde(default = "default_timeout")]
    pub(crate) default_timeout: u64,
    #[serde(default = "default_max_stdout")]
    pub(crate) max_stdout_chars: usize,
    #[serde(default = "default_max_stderr")]
    pub(crate) max_stderr_chars: usize,
    #[serde(default = "default_max_output")]
    pub(crate) max_output_chars: usize,
    // Phase 3 — protocole d'entêtes (ADR 0006)
    #[serde(default = "default_timeout_session")]
    pub(crate) timeout_session: u64,
    #[serde(default = "default_max_auth_failures")]
    pub(crate) max_auth_failures: u32,
    #[serde(default)]
    pub(crate) log_comments: bool,
    #[serde(default)]
    pub(crate) ban_command: String,
    // Protocole v2 — UUID de session (alignement 003)
    #[serde(default)]
    pub(crate) expose_session_id: bool,
    // ADR 0011 — streaming: limite volume total streamé (stdout + stderr)
    #[serde(default = "default_max_stream_bytes")]
    pub(crate) max_stream_bytes: usize,
    // Capture legacy TOML fields (log_level, default_level, mask_sensitive) without error
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, toml::Value>,
}

fn default_timeout() -> u64 {
    300
}

fn default_max_stdout() -> usize {
    65536
}

fn default_max_stderr() -> usize {
    16384
}

fn default_max_output() -> usize {
    131072
}

fn default_timeout_session() -> u64 {
    3600
}

fn default_max_auth_failures() -> u32 {
    3
}

fn default_max_stream_bytes() -> usize {
    10_485_760 // 10 Mo
}

fn default_max_body_size() -> usize {
    65536 // 64 Ko (ADR 0012 D2)
}

/// Token d'authentification RBAC (ADR 0006 §11)
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TokenConfig {
    pub(crate) secret: String,
    pub(crate) level: TrustLevel,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
}

/// Section [auth] optionnelle (ADR 0006 §11, ADR 0010)
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AuthConfig {
    #[serde(default)]
    pub(crate) challenge_nonce: bool,
    #[serde(default)]
    pub(crate) tokens: BTreeMap<String, TokenConfig>,
}

/// Configuration complete
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Config {
    pub(crate) global: GlobalConfig,
    #[serde(default)]
    pub(crate) domains: BTreeMap<String, DomainConfig>,
    pub(crate) auth: Option<AuthConfig>,
}

impl Config {
    /// Charge et valide la configuration depuis un fichier TOML
    pub(crate) fn from_file(path: &str) -> Result<Self, ConfigError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ConfigError::Io(format!("{path}: {e}")))?;
        Self::from_str(&content)
    }

    /// Parse et valide la configuration depuis une chaine TOML
    pub(crate) fn from_str(toml_content: &str) -> Result<Self, ConfigError> {
        let mut config: Config =
            toml::from_str(toml_content).map_err(|e| ConfigError::Parse(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&mut self) -> Result<(), ConfigError> {
        // Au moins un domaine
        if self.domains.is_empty() {
            return Err(ConfigError::Validation(
                "no domain defined in configuration".to_string(),
            ));
        }

        // Chaque domaine a au moins une action
        for (domain_id, domain) in &mut self.domains {
            if domain.actions.is_empty() {
                return Err(ConfigError::Validation(format!(
                    "domain '{domain_id}' has no action defined"
                )));
            }

            for (action_id, action) in &mut domain.actions {
                Self::validate_and_normalize_tags(
                    &mut action.tags,
                    &format!("{domain_id}.{action_id}"),
                )?;
                Self::validate_action_fields(domain_id, action_id, action)?;
            }
        }

        // Collect all action tags for orphan check
        let all_action_tags: std::collections::BTreeSet<&str> = self
            .domains
            .values()
            .flat_map(|d| d.actions.values())
            .flat_map(|a| a.tags.iter())
            .map(String::as_str)
            .collect();

        Self::validate_auth_tokens(&mut self.auth, &all_action_tags)?;
        Self::validate_output_limits(&self.global)?;

        Ok(())
    }

    /// Validate auth tokens: name format, secret encoding, tags
    fn validate_auth_tokens(
        auth: &mut Option<AuthConfig>,
        all_action_tags: &std::collections::BTreeSet<&str>,
    ) -> Result<(), ConfigError> {
        let Some(ref mut auth) = auth else {
            return Ok(());
        };

        for (token_id, token) in &mut auth.tokens {
            if !token_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
            {
                return Err(ConfigError::Validation(format!(
                    "invalid token name '{token_id}': must be alphanumeric or hyphen"
                )));
            }
            if !token.secret.starts_with("b64:") {
                return Err(ConfigError::Validation(format!(
                    "token '{token_id}' secret must start with 'b64:' prefix"
                )));
            }
            crate::crypto::decode_b64_secret(&token.secret).map_err(|e| {
                ConfigError::Validation(format!(
                    "token '{token_id}' has invalid base64 secret: {e}"
                ))
            })?;

            Self::validate_and_normalize_tags(&mut token.tags, &format!("token '{token_id}'"))?;

            for tag in &token.tags {
                if !all_action_tags.contains(tag.as_str()) {
                    eprintln!(
                        "ssh-frontiere: warning: tag '{tag}' on token '{token_id}' \
                         does not match any action"
                    );
                }
            }
        }
        Ok(())
    }

    /// Validate output size limits coherence
    fn validate_output_limits(global: &GlobalConfig) -> Result<(), ConfigError> {
        if global.max_stdout_chars > global.max_output_chars {
            return Err(ConfigError::Validation(
                "max_stdout_chars exceeds max_output_chars hard limit".to_string(),
            ));
        }
        if global.max_stderr_chars > global.max_output_chars {
            return Err(ConfigError::Validation(
                "max_stderr_chars exceeds max_output_chars hard limit".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate and normalize a tags list: lowercase, dedup, format check
    fn validate_and_normalize_tags(
        tags: &mut Vec<String>,
        context: &str,
    ) -> Result<(), ConfigError> {
        for tag in tags.iter_mut() {
            *tag = tag.to_lowercase();
        }
        for tag in tags.iter() {
            if tag.is_empty() {
                return Err(ConfigError::Validation(format!("empty tag in {context}")));
            }
            if !tag
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(ConfigError::Validation(format!(
                    "invalid tag '{tag}' in {context}: must be alphanumeric, hyphen or underscore"
                )));
            }
        }
        tags.sort();
        tags.dedup();
        Ok(())
    }

    fn validate_action_fields(
        domain_id: &str,
        action_id: &str,
        action: &ActionConfig,
    ) -> Result<(), ConfigError> {
        // ADR 0012 D2 — max_body_size must be > 0
        if action.max_body_size == 0 {
            return Err(ConfigError::Validation(format!(
                "max_body_size must be > 0 in {domain_id}.{action_id}"
            )));
        }

        for (arg_name, arg) in &action.args {
            // Validate argument name format
            if !arg_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(ConfigError::Validation(format!(
                    "invalid argument name '{arg_name}' in {domain_id}.{action_id}: \
                     must be alphanumeric, hyphen or underscore"
                )));
            }

            // free = true: skip enum validation (ADR 0012 D5)
            if arg.free {
                continue;
            }

            // Enum arguments must have values
            if arg.arg_type == "enum" {
                match &arg.values {
                    None => {
                        return Err(ConfigError::Validation(format!(
                            "enum argument '{arg_name}' in {domain_id}.{action_id} has no values",
                        )));
                    }
                    Some(vals) if vals.is_empty() => {
                        return Err(ConfigError::Validation(format!(
                            "enum argument '{arg_name}' in {domain_id}.{action_id} has empty values list",
                        )));
                    }
                    Some(vals) => {
                        // Default must be in values if present
                        if let Some(ref default) = arg.default {
                            if !vals.contains(default) {
                                return Err(ConfigError::Validation(format!(
                                    "default value '{default}' for enum argument '{arg_name}' \
                                     in {domain_id}.{action_id} is not in allowed values",
                                )));
                            }
                        }
                    }
                }
            }
        }

        Self::validate_placeholders(domain_id, action_id, action)
    }

    /// Check that all `{arg}` placeholders in `execute` have matching argument definitions
    fn validate_placeholders(
        domain_id: &str,
        action_id: &str,
        action: &ActionConfig,
    ) -> Result<(), ConfigError> {
        let mut pos = 0;
        let execute = &action.execute;
        while let Some(start) = execute[pos..].find('{') {
            let abs_start = pos + start;
            if let Some(end) = execute[abs_start..].find('}') {
                let placeholder = &execute[abs_start + 1..abs_start + end];
                if placeholder != "domain" && !action.args.contains_key(placeholder) {
                    return Err(ConfigError::Validation(format!(
                        "placeholder '{{{placeholder}}}' in {domain_id}.{action_id} \
                         has no matching argument definition"
                    )));
                }
                pos = abs_start + end + 1;
            } else {
                break;
            }
        }
        Ok(())
    }
}
