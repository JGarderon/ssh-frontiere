use crate::config::{Config, TrustLevel};
use crate::crypto;

/// Authentication context for a connection
pub(crate) struct AuthContext {
    pub(crate) base_level: TrustLevel,
    pub(crate) effective_level: TrustLevel,
    pub(crate) effective_tags: Vec<String>,
    pub(crate) failures: u32,
    pub(crate) max_failures: u32,
}

impl AuthContext {
    pub(crate) fn new(base_level: TrustLevel, max_failures: u32) -> Self {
        AuthContext {
            base_level,
            effective_level: base_level,
            effective_tags: Vec::new(),
            failures: 0,
            max_failures,
        }
    }

    /// Validate a +auth attempt and update context
    /// Returns `Ok(new_level)` or `Err(message)`
    /// nonce: None = simple mode (SHA-256 brut), Some = nonce mode (challenge-response)
    #[must_use = "auth validation result must be checked"]
    pub(crate) fn validate_auth(
        &mut self,
        config: &Config,
        token_id: &str,
        proof_hex: &str,
        nonce: Option<&[u8]>,
    ) -> Result<TrustLevel, String> {
        let auth = config
            .auth
            .as_ref()
            .ok_or_else(|| "auth not configured".to_string())?;

        let Some(token) = auth.tokens.get(token_id) else {
            self.failures += 1;
            return Err(format!(
                "authentication failed ({}/{})",
                self.failures, self.max_failures
            ));
        };

        let Ok(secret) = crypto::decode_b64_secret(&token.secret) else {
            self.failures += 1;
            return Err(format!(
                "authentication failed ({}/{})",
                self.failures, self.max_failures
            ));
        };

        let valid = match nonce {
            Some(n) => crypto::verify_proof(&secret, n, proof_hex),
            None => crypto::verify_simple_proof(&secret, proof_hex),
        };

        if valid {
            // Auth success: effective level = max(base, token)
            let new_level = self.base_level.max(token.level);
            self.effective_level = new_level;

            // Merge token tags into effective_tags (union, deduplicated)
            for tag in &token.tags {
                if !self.effective_tags.contains(tag) {
                    self.effective_tags.push(tag.clone());
                }
            }
            self.effective_tags.sort();

            Ok(new_level)
        } else {
            self.failures += 1;
            Err(format!(
                "authentication failed ({}/{})",
                self.failures, self.max_failures
            ))
        }
    }

    #[must_use]
    pub(crate) fn is_locked_out(&self) -> bool {
        self.failures >= self.max_failures
    }
}
