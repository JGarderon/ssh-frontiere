// --- Execution engine for command block chaining (protocol v2) ---

use crate::auth::AuthContext;
use crate::chain_parser::{CommandNode, SequenceMode};
use crate::config::Config;
use crate::discovery::handle_discovery;
use crate::dispatch::{
    check_authorization, parse_command, resolve_command, transpose_command, Identity,
};
use crate::executor::{execute_command, ExecuteResult};
use crate::logging::{write_log, LogEntry};
use crate::output::{
    Response, EXIT_INSUFFICIENT_LEVEL, EXIT_REJECTED, EXIT_STDIN_ERROR, EXIT_TIMEOUT,
};
use crate::protocol::{write_comment, write_response};
use std::io::Write;

// Convention: `let _ =` on write_comment/write_response/write_log calls is intentional.
// Protocol output is best-effort — the SSH client may have closed the connection.

/// Execute un arbre de commandes et ecrit les reponses >> sur le writer.
/// Retourne le dernier `status_code`.
/// `body` is passed to the **first** Single command only (ADR 0012 plan P2).
#[must_use = "exit code must be checked"]
pub(crate) fn execute_chain(
    node: &CommandNode,
    config: &Config,
    identity: &Identity,
    auth_ctx: &AuthContext,
    session_id: &str,
    writer: &mut impl Write,
    body: Option<&str>,
) -> i32 {
    match node {
        CommandNode::Single(cmd) => {
            execute_single_command(cmd, config, identity, auth_ctx, session_id, writer, body)
        }
        CommandNode::Sequence(nodes, mode) => {
            let mut last_code = 0;
            let mut remaining_body = body;
            for child in nodes {
                last_code = execute_chain(
                    child,
                    config,
                    identity,
                    auth_ctx,
                    session_id,
                    writer,
                    remaining_body,
                );
                remaining_body = None;
                if *mode == SequenceMode::Strict && last_code != 0 {
                    return last_code;
                }
            }
            last_code
        }
        CommandNode::Recovery(left, right) => {
            let code = execute_chain(left, config, identity, auth_ctx, session_id, writer, body);
            if code != 0 {
                execute_chain(right, config, identity, auth_ctx, session_id, writer, None)
            } else {
                code
            }
        }
    }
}

/// Execute une commande simple : parse, resolve, authorize, transpose, execute, write response.
fn execute_single_command(
    raw_command: &str,
    config: &Config,
    identity: &Identity,
    auth_ctx: &AuthContext,
    session_id: &str,
    writer: &mut impl Write,
    body: Option<&str>,
) -> i32 {
    // Built-in: exit
    if raw_command.trim() == "exit" {
        let resp = Response {
            command: "exit".to_string(),
            status_code: 0,
            status_message: "ok".to_string(),
            stdout: None,
            stderr: None,
        };
        let _ = write_response(writer, &resp.to_json());
        return 0;
    }

    // 1. Parser la commande
    let tokens = match parse_command(raw_command) {
        Ok(t) => t,
        Err(e) => {
            let resp = Response::rejected(raw_command, &format!("rejected: {e}"), EXIT_REJECTED);
            let _ = write_response(writer, &resp.to_json());
            return EXIT_REJECTED;
        }
    };

    // 2. Built-in: help, list
    if tokens[0] == "help" || tokens[0] == "list" {
        let effective = identity.with_level(auth_ctx.effective_level);
        return handle_builtin_chain(
            config,
            &tokens,
            raw_command,
            &effective,
            &auth_ctx.effective_tags,
            writer,
        );
    }

    // 3. Resoudre domaine + action
    let (domain_id, action_id, cmd_args) = match resolve_command(config, &tokens) {
        Ok(r) => r,
        Err(e) => {
            let resp = Response::rejected(raw_command, &format!("rejected: {e}"), EXIT_REJECTED);
            let _ = write_response(writer, &resp.to_json());
            return EXIT_REJECTED;
        }
    };

    let Some(domain_cfg) = config.domains.get(&domain_id) else {
        let resp = Response::rejected(
            raw_command,
            &format!("rejected: unknown domain '{domain_id}'"),
            EXIT_REJECTED,
        );
        let _ = write_response(writer, &resp.to_json());
        return EXIT_REJECTED;
    };
    let Some(action) = domain_cfg.actions.get(&action_id) else {
        let resp = Response::rejected(
            raw_command,
            &format!("rejected: unknown action '{action_id}'"),
            EXIT_REJECTED,
        );
        let _ = write_response(writer, &resp.to_json());
        return EXIT_REJECTED;
    };

    // 4. Verifier l'autorisation RBAC
    let effective = identity.with_level(auth_ctx.effective_level);

    if let Err(e) = check_authorization(&effective, action, &auth_ctx.effective_tags) {
        let resp = Response::rejected(
            raw_command,
            &format!("rejected: {e}"),
            EXIT_INSUFFICIENT_LEVEL,
        );
        let _ = write_response(writer, &resp.to_json());
        log_command(
            config,
            "rejected",
            &domain_id,
            &action_id,
            identity,
            Some(&cmd_args),
        );
        return EXIT_INSUFFICIENT_LEVEL;
    }

    // 5. Transposer et executer
    let resolved = ResolvedCommand {
        domain_id: &domain_id,
        action_id: &action_id,
        args: &cmd_args,
    };
    execute_and_respond(
        raw_command,
        config,
        &resolved,
        identity,
        session_id,
        writer,
        body,
    )
}

/// Result of command resolution (domain, action, args)
struct ResolvedCommand<'a> {
    domain_id: &'a str,
    action_id: &'a str,
    args: &'a std::collections::HashMap<String, String>,
}

/// Transpose, execute, and write the response for a resolved command (ADR 0011 — streaming)
#[allow(clippy::too_many_lines)]
fn execute_and_respond(
    raw_command: &str,
    config: &Config,
    resolved: &ResolvedCommand<'_>,
    identity: &Identity,
    session_id: &str,
    writer: &mut impl Write,
    body: Option<&str>,
) -> i32 {
    let domain_id = resolved.domain_id;
    let action_id = resolved.action_id;
    let Some(domain_cfg) = config.domains.get(domain_id) else {
        let resp = Response::rejected(
            raw_command,
            &format!("rejected: unknown domain '{domain_id}'"),
            EXIT_REJECTED,
        );
        let _ = write_response(writer, &resp.to_json());
        return EXIT_REJECTED;
    };
    let Some(action) = domain_cfg.actions.get(action_id) else {
        let resp = Response::rejected(
            raw_command,
            &format!("rejected: unknown action '{action_id}'"),
            EXIT_REJECTED,
        );
        let _ = write_response(writer, &resp.to_json());
        return EXIT_REJECTED;
    };
    let timeout = action.timeout.unwrap_or(config.global.default_timeout);
    let cmd_parts = transpose_command(&action.execute, domain_id, resolved.args);
    let cmd_refs: Vec<&str> = cmd_parts.iter().map(String::as_str).collect();

    match execute_command(
        &cmd_refs,
        timeout,
        session_id,
        writer,
        config.global.max_stream_bytes,
        body,
    ) {
        ExecuteResult::Exited(code) => {
            let resp = Response::streamed(raw_command, code);
            let _ = write_response(writer, &resp.to_json());
            log_command(
                config,
                "executed",
                domain_id,
                action_id,
                identity,
                Some(resolved.args),
            );
            code
        }
        ExecuteResult::Signaled(signal) => {
            let resp = Response::streamed(raw_command, 128 + signal);
            let _ = write_response(writer, &resp.to_json());
            log_command(
                config,
                "executed",
                domain_id,
                action_id,
                identity,
                Some(resolved.args),
            );
            128 + signal
        }
        ExecuteResult::Timeout => {
            let desc = format!("{domain_id} {action_id}");
            let resp = Response::timeout(&desc, timeout);
            let _ = write_response(writer, &resp.to_json());
            log_command(
                config,
                "timeout",
                domain_id,
                action_id,
                identity,
                Some(resolved.args),
            );
            EXIT_TIMEOUT
        }
        ExecuteResult::SpawnError(e) => {
            let resp =
                Response::rejected(raw_command, &format!("execution error: {e}"), EXIT_REJECTED);
            let _ = write_response(writer, &resp.to_json());
            log_command(
                config,
                "rejected",
                domain_id,
                action_id,
                identity,
                Some(resolved.args),
            );
            EXIT_REJECTED
        }
        ExecuteResult::StdinError => {
            let resp = Response::rejected(
                raw_command,
                "stdin closed: target process closed stdin before body was fully written",
                EXIT_STDIN_ERROR,
            );
            let _ = write_response(writer, &resp.to_json());
            log_command(
                config,
                "stdin_error",
                domain_id,
                action_id,
                identity,
                Some(resolved.args),
            );
            EXIT_STDIN_ERROR
        }
    }
}

/// Gere les commandes built-in (help, list) dans le contexte du chainage
fn handle_builtin_chain(
    config: &Config,
    tokens: &[String],
    raw_command: &str,
    identity: &Identity,
    effective_tags: &[String],
    writer: &mut impl Write,
) -> i32 {
    if tokens[0] == "help" {
        write_help_text(config, tokens, identity, effective_tags, writer);
        return 0;
    }

    // list : retourne du JSON via >> (format 5 champs)
    match handle_discovery(config, tokens, identity, effective_tags) {
        Ok(json_str) => {
            let resp = Response {
                command: raw_command.to_string(),
                status_code: 0,
                status_message: "ok".to_string(),
                stdout: Some(json_str),
                stderr: None,
            };
            let _ = write_response(writer, &resp.to_json());
            0
        }
        Err(e) => {
            let resp = Response::rejected(raw_command, &format!("rejected: {e}"), EXIT_REJECTED);
            let _ = write_response(writer, &resp.to_json());
            EXIT_REJECTED
        }
    }
}

/// Ecrit le texte d'aide humain via #> (protocole v2, alignement 003, TODO-028)
/// Filtre par niveau d'acces et tags : sans +auth, seules les actions publiques sont affichees
pub(crate) fn write_help_text(
    config: &Config,
    tokens: &[String],
    identity: &Identity,
    effective_tags: &[String],
    writer: &mut impl Write,
) {
    let version = env!("CARGO_PKG_VERSION");
    let _ = write_comment(
        writer,
        &format!("ssh-frontiere {version} — SSH command dispatcher"),
    );
    let _ = write_comment(writer, "");

    if tokens.len() == 1 {
        write_help_overview(config, identity, effective_tags, writer);
    } else {
        write_help_domain(config, &tokens[1], identity, effective_tags, writer);
    }

    // ADR 0011 §5 : help emet un >>> final pour uniformite protocole
    let resp = Response {
        command: "help".to_string(),
        status_code: 0,
        status_message: "ok".to_string(),
        stdout: None,
        stderr: None,
    };
    let _ = write_response(writer, &resp.to_json());
}

/// Write the overview help text (no argument)
fn write_help_overview(
    config: &Config,
    identity: &Identity,
    effective_tags: &[String],
    writer: &mut impl Write,
) {
    let _ = write_comment(writer, "Protocol:");
    let _ = write_comment(writer, "  +     Client configuration (headers)");
    let _ = write_comment(writer, "  #     Client comment");
    let _ = write_comment(writer, "  .     End of command block (alone on its line)");
    let _ = write_comment(writer, "  +>    Server configuration");
    let _ = write_comment(writer, "  #>    Server comment");
    let _ = write_comment(writer, "  >>    Stdout output (streaming)");
    let _ = write_comment(writer, "  >>!   Stderr output (streaming)");
    let _ = write_comment(writer, "  >>>   Final JSON response");
    let _ = write_comment(writer, "");
    let _ = write_comment(writer, "Operators:");
    let _ = write_comment(writer, "  ;     Strict sequential (stop on first failure)");
    let _ = write_comment(
        writer,
        "  &     Permissive sequential (continue regardless)",
    );
    let _ = write_comment(writer, "  |     Fallback (execute on failure)");
    let _ = write_comment(writer, "  ()    Grouping");
    let _ = write_comment(writer, "");
    let _ = write_comment(writer, "Available domains:");
    let _ = write_comment(writer, "");

    for (domain_id, domain) in &config.domains {
        let visible_actions: Vec<(&String, &crate::config::ActionConfig)> = domain
            .actions
            .iter()
            .filter(|(_, a)| a.is_visible_to(identity.level, effective_tags))
            .collect();

        if !visible_actions.is_empty() {
            let _ = write_comment(writer, &format!("  {domain_id}"));
            for (action_id, action) in &visible_actions {
                let _ = write_comment(
                    writer,
                    &format!("    {action_id:<20} {}", action.description),
                );
            }
            let _ = write_comment(writer, "");
        }
    }

    let _ = write_comment(writer, "For the full list (JSON): list");
    let _ = write_comment(writer, "");
}

/// Write help text for a specific domain
fn write_help_domain(
    config: &Config,
    target: &str,
    identity: &Identity,
    effective_tags: &[String],
    writer: &mut impl Write,
) {
    if let Some(domain) = config.domains.get(target) {
        let _ = write_comment(
            writer,
            &format!("Domain: {target} — {}", domain.description),
        );
        let _ = write_comment(writer, "");

        for (action_id, action) in &domain.actions {
            if action.is_visible_to(identity.level, effective_tags) {
                let _ = write_comment(writer, &format!("  {action_id:<20} {}", action.description));
                let _ = write_comment(writer, &format!("    required level: {}", action.level));
                if !action.args.is_empty() {
                    let args_desc: Vec<String> = action
                        .args
                        .iter()
                        .map(|(name, a)| {
                            let type_desc = if a.free {
                                "[free text]".to_string()
                            } else {
                                format!("{{{}}}", a.arg_type)
                            };
                            let suffix = if a.default.is_some() {
                                " [optional]"
                            } else {
                                ""
                            };
                            format!("{name}={type_desc}{suffix}")
                        })
                        .collect();
                    let _ =
                        write_comment(writer, &format!("    arguments: {}", args_desc.join(", ")));
                }
                if action.max_body_size != 65536 {
                    let _ = write_comment(
                        writer,
                        &format!("    body: max {} KB", action.max_body_size / 1024),
                    );
                }
                let _ = write_comment(writer, "");
            }
        }
    } else {
        let _ = write_comment(writer, &format!("unknown domain: {target}"));
    }
}

/// Log a command event (executed, rejected, timeout)
fn log_command(
    config: &Config,
    event: &str,
    domain: &str,
    action: &str,
    identity: &Identity,
    args: Option<&std::collections::HashMap<String, String>>,
) {
    let mut entry = LogEntry::new(event).with_domain(domain).with_action(action);
    if let Some(ref client) = identity.ssh_client {
        entry = entry.with_ssh_client(client);
    }
    // NOTE: args intentionally not logged to avoid leaking sensitive values.
    // TODO: log non-sensitive args and hash sensitive ones (requires ActionConfig).
    let _ = args;
    let _ = write_log(&config.global.log_file, &entry);
}

// Re-export parse_block for orchestrator convenience
pub(crate) use crate::chain_parser::parse_block;
