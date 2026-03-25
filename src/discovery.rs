use crate::config::Config;
use crate::dispatch::{DispatchError, Identity};
use serde_json::json;

/// Gere les sous-commandes de decouverte (help, list)
#[must_use = "discovery result must be checked"]
pub(crate) fn handle_discovery(
    config: &Config,
    tokens: &[String],
    identity: &Identity,
    effective_tags: &[String],
) -> Result<String, DispatchError> {
    // PANIC-SAFE: callers (handle_builtin_chain) guarantee tokens is non-empty (checked tokens[0] == "help"/"list")
    match tokens[0].as_str() {
        "help" => {
            if tokens.len() == 1 {
                Ok(help_full(config, identity, effective_tags))
            } else {
                // PANIC-SAFE: tokens.len() > 1 from the else branch
                help_target(config, &tokens[1], identity, effective_tags)
            }
        }
        "list" => Ok(list_actions(config, identity, effective_tags)),
        _ => Err(DispatchError::InvalidSyntax(format!(
            "unknown discovery command '{}'",
            tokens[0]
        ))),
    }
}

fn help_full(config: &Config, identity: &Identity, effective_tags: &[String]) -> String {
    let mut domains = serde_json::Map::new();

    for (domain_id, domain) in &config.domains {
        let mut actions = serde_json::Map::new();
        for (action_id, action) in &domain.actions {
            if action.is_visible_to(identity.level, effective_tags) {
                actions.insert(action_id.clone(), action_to_json(action));
            }
        }
        if !actions.is_empty() {
            domains.insert(
                domain_id.clone(),
                json!({
                    "description": domain.description,
                    "actions": actions,
                }),
            );
        }
    }

    json!({ "domains": domains }).to_string()
}

fn help_target(
    config: &Config,
    target: &str,
    identity: &Identity,
    effective_tags: &[String],
) -> Result<String, DispatchError> {
    // D'abord chercher comme domaine
    if let Some(domain) = config.domains.get(target) {
        let mut actions = serde_json::Map::new();
        for (action_id, action) in &domain.actions {
            if action.is_visible_to(identity.level, effective_tags) {
                actions.insert(action_id.clone(), action_to_json(action));
            }
        }
        if actions.is_empty() {
            return Err(DispatchError::InvalidSyntax(format!(
                "unknown domain or action '{target}'"
            )));
        }
        return Ok(json!({
            "domain": target,
            "description": domain.description,
            "actions": actions,
        })
        .to_string());
    }

    // Sinon chercher comme action (resolution implicite si domaine unique)
    for domain in config.domains.values() {
        if let Some(action) = domain.actions.get(target) {
            if action.is_visible_to(identity.level, effective_tags) {
                return Ok(action_to_json(action).to_string());
            }
        }
    }

    Err(DispatchError::InvalidSyntax(format!(
        "unknown domain or action '{target}'"
    )))
}

fn list_actions(config: &Config, identity: &Identity, effective_tags: &[String]) -> String {
    let mut actions_list = Vec::new();

    for (domain_id, domain) in &config.domains {
        for (action_id, action) in &domain.actions {
            if action.is_visible_to(identity.level, effective_tags) {
                actions_list.push(json!({
                    "domain": domain_id,
                    "action": action_id,
                    "description": action.description,
                    "level": action.level.to_string(),
                }));
            }
        }
    }

    json!({ "actions": actions_list }).to_string()
}

fn action_to_json(action: &crate::config::ActionConfig) -> serde_json::Value {
    let mut args_map = serde_json::Map::new();
    for (name, a) in &action.args {
        let mut obj = json!({
            "type": a.arg_type,
        });
        if let Some(ref vals) = a.values {
            obj["values"] = json!(vals);
        }
        if a.sensitive {
            obj["sensitive"] = json!(true);
        }
        if let Some(ref default) = a.default {
            obj["default"] = json!(default);
        }
        if a.free {
            obj["free"] = json!(true);
        }
        args_map.insert(name.clone(), obj);
    }

    json!({
        "description": action.description,
        "level": action.level.to_string(),
        "args": args_map,
        "max_body_size": action.max_body_size,
    })
}
