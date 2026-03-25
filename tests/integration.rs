use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_test_dir(prefix: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    format!("/tmp/{prefix}-{pid}-{id}")
}

/// Protocol v2 test helper: spawns ssh-frontiere and interacts via stdin/stdout.
///
/// Protocol v2 changes:
/// - Banner uses `#>` and `+>` prefixes
/// - Responses use `>>> ` prefix (ADR 0011: was `>> ` before streaming)
/// - Streaming: stdout lines prefixed `>> `, stderr lines prefixed `>>! `
/// - Commands are plain text (no `$` prefix), terminated by `.` alone on a line
/// - No mandatory empty line between headers and command (implicit transition)
/// - Session end = `.` alone (no preceding command)
/// - JSON response has 5 fields: command, `status_code`, `status_message`, stdout, stderr
/// - For executed commands, stdout/stderr are `null` in JSON (content was streamed)
fn run_protocol(command: &str) -> (i32, serde_json::Value, Vec<String>) {
    run_protocol_with_level(command, "ops")
}

fn run_protocol_with_level(command: &str, level: &str) -> (i32, serde_json::Value, Vec<String>) {
    run_protocol_full(command, level, &[], false)
}

fn run_protocol_full(
    command: &str,
    level: &str,
    headers: &[&str],
    session_mode: bool,
) -> (i32, serde_json::Value, Vec<String>) {
    let config = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/test-config.toml"
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg(format!("--level={level}"))
        .arg(format!("--config={config}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn");

    let mut stdin = child.stdin.take().expect("stdin");

    // Write headers (+ directives, # comments)
    for header in headers {
        writeln!(stdin, "{header}").expect("write header");
    }
    if session_mode {
        writeln!(stdin, "+ session keepalive").expect("write session");
    }
    // v2: no mandatory empty line — write command as plain text + "." terminator
    writeln!(stdin, "{command}").expect("write command");
    writeln!(stdin, ".").expect("write block terminator");

    if session_mode {
        // v2: "." alone = end of session (replaces "$ exit")
        writeln!(stdin, ".").expect("write session end");
    }

    drop(stdin); // Close stdin to signal EOF

    let output = child.wait_with_output().expect("wait");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse all lines
    let all_lines: Vec<String> = stdout
        .lines()
        .map(std::string::ToString::to_string)
        .collect();

    // Find the first >>> response line (ADR 0011: final JSON uses >>> prefix)
    let response_json = all_lines
        .iter()
        .find_map(|line| {
            line.strip_prefix(">>> ")
                .and_then(|json| serde_json::from_str(json).ok())
        })
        .unwrap_or_else(|| panic!("no >>> response found in output:\n{stdout}"));

    (code, response_json, all_lines)
}

/// Helper for commands that produce raw output lines (like help)
fn run_protocol_raw(command: &str, level: &str) -> (i32, Vec<String>) {
    let config = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/test-config.toml"
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg(format!("--level={level}"))
        .arg(format!("--config={config}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn");

    let mut stdin = child.stdin.take().expect("stdin");
    writeln!(stdin, "{command}").expect("write command");
    writeln!(stdin, ".").expect("write terminator");
    drop(stdin);

    let output = child.wait_with_output().expect("wait");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let all_lines: Vec<String> = stdout
        .lines()
        .map(std::string::ToString::to_string)
        .collect();

    (code, all_lines)
}

fn run_protocol_with_config(
    command: &str,
    config_content: &str,
) -> (i32, serde_json::Value, Vec<String>) {
    let dir = unique_test_dir("ssh-frontiere-test-proto");
    let _ = std::fs::create_dir_all(&dir);
    let config_path = format!("{dir}/config.toml");
    std::fs::write(&config_path, config_content).expect("write config");

    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg("--level=ops")
        .arg(format!("--config={config_path}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn");

    let mut stdin = child.stdin.take().expect("stdin");
    // v2: plain text command + "." terminator, no empty line needed
    writeln!(stdin, "{command}").expect("command");
    writeln!(stdin, ".").expect("block terminator");
    drop(stdin);

    let output = child.wait_with_output().expect("wait");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let all_lines: Vec<String> = stdout
        .lines()
        .map(std::string::ToString::to_string)
        .collect();
    let response_json = all_lines
        .iter()
        .find_map(|line| {
            line.strip_prefix(">>> ")
                .and_then(|j| serde_json::from_str(j).ok())
        })
        .unwrap_or_else(|| panic!("no >>> response in:\n{stdout}"));

    let _ = std::fs::remove_dir_all(&dir);

    (code, response_json, all_lines)
}

// --- Scenario 1 : Execution simple ---
#[test]
fn e2e_simple_execution() {
    let (code, json, all_lines) = run_protocol("test echo");
    assert_eq!(code, 0);
    assert_eq!(json["status_code"], 0);
    assert_eq!(json["status_message"], "executed");
    // ADR 0011: stdout is null in JSON (content was streamed via >> lines)
    assert!(json["stdout"].is_null());
    // Verify streamed output contains expected content
    assert!(
        all_lines
            .iter()
            .any(|l| l.strip_prefix(">> ").is_some_and(|s| s.contains("test"))),
        "expected streamed >> line containing 'test'"
    );
}

// --- Scenario 2 : Execution avec argument enum valide ---
#[test]
fn e2e_execution_with_valid_enum_arg() {
    let (code, json, all_lines) = run_protocol("test greet name=world");
    assert_eq!(code, 0);
    assert_eq!(json["status_code"], 0);
    // ADR 0011: stdout is null in JSON (content was streamed via >> lines)
    assert!(json["stdout"].is_null());
    // Verify streamed output contains expected content
    let streamed: Vec<&str> = all_lines
        .iter()
        .filter_map(|l| l.strip_prefix(">> "))
        .collect();
    assert!(
        streamed.iter().any(|s| s.contains("hello")),
        "expected streamed line containing 'hello'"
    );
    assert!(
        streamed.iter().any(|s| s.contains("world")),
        "expected streamed line containing 'world'"
    );
}

// --- Scenario 3 : Argument invalide -> rejet ---
#[test]
fn e2e_invalid_enum_arg_rejected() {
    let (code, json, _) = run_protocol("test greet name=invalid");
    assert_eq!(code, 128);
    assert_eq!(json["status_code"], 128);
    assert!(json["stdout"].is_null());
}

// --- Scenario 4 : Commande inconnue -> rejet ---
#[test]
fn e2e_unknown_action_rejected() {
    let (code, json, _) = run_protocol("test unknown");
    assert_eq!(code, 128);
    assert_eq!(json["status_code"], 128);
    assert!(json["stdout"].is_null());
}

// --- Scenario 5 : Domaine inconnu -> rejet ---
#[test]
fn e2e_unknown_domain_rejected() {
    let (code, json, _) = run_protocol("nonexistent echo");
    assert_eq!(code, 128);
    assert_eq!(json["status_code"], 128);
}

// --- Scenario 6 : Operateur | = rattrapage (protocole v2) ---
#[test]
fn e2e_chaining_recovery_operator() {
    // "test echo | cat" = "test echo" | "cat"
    // test echo succeeds (code 0), so "cat" is not executed (recovery only on failure)
    let (code, json, _) = run_protocol("test echo | cat");
    assert_eq!(code, 0);
    assert_eq!(json["status_code"], 0);
}

// --- Scenario 6a : Tokens excedentaires -> rejet grammatical ---
#[test]
fn e2e_grammatical_rejection_extra_tokens() {
    // "test echo extra1 extra2" has 2 extra tokens for an action expecting 0 args
    let (code, json, _) = run_protocol("test echo extra1 extra2");
    assert_eq!(code, 128);
    assert_eq!(json["status_code"], 128);
}

// --- Scenario 6b : Caracteres speciaux entre guillemets -> acceptes ---
#[test]
fn e2e_special_chars_in_quotes_accepted() {
    let (code, json, all_lines) = run_protocol(r#"test say "message=hello|world;test&foo$bar""#);
    assert_eq!(code, 0);
    assert_eq!(json["status_code"], 0);
    // ADR 0011: stdout is null in JSON (content was streamed via >> lines)
    assert!(json["stdout"].is_null());
    let streamed: Vec<&str> = all_lines
        .iter()
        .filter_map(|l| l.strip_prefix(">> "))
        .collect();
    assert!(
        streamed
            .iter()
            .any(|s| s.contains("hello|world;test&foo$bar")),
        "expected streamed line containing special chars"
    );
}

// --- Scenario 6c : Variable shell non interpretee ---
#[test]
fn e2e_shell_variable_not_expanded() {
    let (code, json, all_lines) = run_protocol(r#"test say "message=$HOME""#);
    assert_eq!(code, 0);
    assert_eq!(json["status_code"], 0);
    // ADR 0011: stdout is null in JSON (content was streamed via >> lines)
    assert!(json["stdout"].is_null());
    let streamed: Vec<&str> = all_lines
        .iter()
        .filter_map(|l| l.strip_prefix(">> "))
        .collect();
    assert!(
        streamed.iter().any(|s| s.contains("$HOME")),
        "expected streamed line containing '$HOME' (not expanded)"
    );
}

// --- Scenario 7 : Commande trop longue -> rejet protocole (line too long) ---
#[test]
fn e2e_command_too_long_rejected() {
    let config = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/test-config.toml"
    );

    let long = format!("test {}", "a".repeat(4093));

    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg("--level=ops")
        .arg(format!("--config={config}"))
        .arg("--diagnostic")
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let mut stdin = child.stdin.take().expect("stdin");
    // v2: plain text command (no $ prefix) + terminator
    writeln!(stdin, "{long}").expect("write command");
    writeln!(stdin, ".").expect("write terminator");
    drop(stdin);

    let output = child.wait_with_output().expect("wait");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Protocol rejects lines > 4096 chars with exit code 132
    // --diagnostic mode shows error details
    assert_eq!(code, 132);
    assert!(stdout.lines().any(|l| l.contains("line too long")));
}

// --- Scenario 8 : Niveau insuffisant -> code 131 ---
#[test]
fn e2e_insufficient_level_rejected() {
    let (code, json, _) = run_protocol_with_level("test greet name=world", "read");
    assert_eq!(code, 131);
    assert_eq!(json["status_code"], 131);
    assert!(json["stdout"].is_null());
}

// --- Scenario 9 : Timeout -> code 130 ---
#[test]
fn e2e_timeout() {
    let (code, json, _) = run_protocol("test slow");
    assert_eq!(code, 130);
    assert_eq!(json["status_code"], 130);
    assert!(json["status_message"]
        .as_str()
        .unwrap_or("")
        .contains("timeout"));
}

// --- Scenario 10 : Exit code non-zero -> passthrough ---
#[test]
fn e2e_nonzero_exit_passthrough() {
    let (code, json, _) = run_protocol("test fail");
    assert_eq!(code, 1);
    assert_eq!(json["status_code"], 1);
    assert_eq!(json["status_message"], "executed");
}

// --- Scenario 11 : help -> texte #> + >>> JSON final (ADR 0011) ---
#[test]
fn e2e_help_full() {
    let (code, lines) = run_protocol_raw("help", "ops");
    assert_eq!(code, 0);
    // help retourne du texte via #> puis un >>> JSON final (ADR 0011)
    let help_lines: Vec<&str> = lines.iter().filter_map(|l| l.strip_prefix("#> ")).collect();
    assert!(help_lines.iter().any(|l| l.contains("ssh-frontiere")));
    assert!(help_lines.iter().any(|l| l.contains("Protocol")));
    assert!(help_lines.iter().any(|l| l.contains("Available domains")));
    // ADR 0011: help now produces a >>> final JSON response
    let response = lines.iter().find_map(|l| {
        l.strip_prefix(">>> ")
            .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
    });
    assert!(
        response.is_some(),
        "help should produce a >>> JSON response"
    );
    let resp = response.expect("response");
    assert_eq!(resp["status_code"], 0);
}

// --- Scenario 12 : help <domaine> -> texte #> + >>> JSON final (ADR 0011) ---
#[test]
fn e2e_help_domain() {
    let (code, lines) = run_protocol_raw("help test", "ops");
    assert_eq!(code, 0);
    let help_lines: Vec<&str> = lines.iter().filter_map(|l| l.strip_prefix("#> ")).collect();
    assert!(help_lines.iter().any(|l| l.contains("Domain: test")));
    assert!(help_lines.iter().any(|l| l.contains("echo")));
    // ADR 0011: help now produces a >>> final JSON response
    let response = lines.iter().find_map(|l| {
        l.strip_prefix(">>> ")
            .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
    });
    assert!(
        response.is_some(),
        "help domain should produce a >>> JSON response"
    );
    let resp = response.expect("response");
    assert_eq!(resp["status_code"], 0);
}

// --- Scenario 13 : help filtre par niveau ---
#[test]
fn e2e_help_action() {
    // help echo : en v2, pas de help par action, c'est un domaine inconnu
    let (code, lines) = run_protocol_raw("help echo", "ops");
    assert_eq!(code, 0);
    let help_lines: Vec<&str> = lines.iter().filter_map(|l| l.strip_prefix("#> ")).collect();
    assert!(help_lines.iter().any(|l| l.contains("unknown domain")));
    // ADR 0011: help now produces a >>> final JSON response
    assert!(
        lines.iter().any(|l| l.starts_with(">>> ")),
        "help action should produce a >>> JSON response"
    );
}

// --- Scenario 14 : list ---
#[test]
fn e2e_list() {
    let (code, json, _) = run_protocol("list");
    assert_eq!(code, 0);
    let inner: serde_json::Value =
        serde_json::from_str(json["stdout"].as_str().unwrap_or("{}")).expect("inner json");
    assert!(inner["actions"].is_array());
}

// --- Scenario 15 : help filtre par niveau read ---
#[test]
fn e2e_help_filtered_by_read_level() {
    let (code, lines) = run_protocol_raw("help", "read");
    assert_eq!(code, 0);
    let help_lines: Vec<&str> = lines.iter().filter_map(|l| l.strip_prefix("#> ")).collect();
    // Level read : should not see admin domain (level admin)
    assert!(!help_lines.iter().any(|l| l.contains("  admin")));
    // Should see test domain (has read-level actions)
    assert!(help_lines.iter().any(|l| l.contains("  test")));
}

// --- Scenario 16 : list filtre par niveau read ---
#[test]
fn e2e_list_filtered_by_read_level() {
    let (code, json, _) = run_protocol_with_level("list", "read");
    assert_eq!(code, 0);
    let inner: serde_json::Value =
        serde_json::from_str(json["stdout"].as_str().unwrap_or("{}")).expect("inner json");
    let actions = inner["actions"].as_array().expect("array");
    for action in actions {
        assert_ne!(action["level"].as_str(), Some("ops"));
    }
}

// --- Scenario 17 : Streaming remplace la troncature (ADR 0011) ---
#[test]
fn e2e_output_streamed_not_truncated() {
    let long_output = "x".repeat(2000);
    let config = format!(
        r#"
[global]
log_file = "/tmp/ssh-frontiere-test-trunc-proto/commands.json"
max_stdout_chars = 100
max_stderr_chars = 200
max_output_chars = 200
[domains.test]
description = "Test"
[domains.test.actions.big]
description = "Big output"
level = "read"
timeout = 10
execute = "/bin/echo {long_output}"
"#
    );

    let (_, json, all_lines) = run_protocol_with_config("test big", &config);
    // ADR 0011: stdout is null in JSON (content was streamed via >> lines)
    assert!(json["stdout"].is_null(), "stdout should be null (streamed)");
    assert_eq!(json["status_message"], "executed");
    // Verify streamed output exists and contains the long output
    let streamed: Vec<&str> = all_lines
        .iter()
        .filter_map(|l| l.strip_prefix(">> "))
        .collect();
    assert!(
        !streamed.is_empty(),
        "expected at least one streamed >> line"
    );
    assert!(
        streamed.iter().any(|s| s.contains("xxx")),
        "streamed output should contain the long string"
    );
}

// --- Protocol-specific tests ---

// Scenario P1 : Banniere contient version et capabilities (v2 prefixes)
#[test]
fn e2e_banner_contains_version_and_capabilities() {
    let (_, _, lines) = run_protocol("test echo");
    // v2: banner uses #> and +> prefixes
    assert!(lines.iter().any(|l| l.starts_with("#> ssh-frontiere")));
    assert!(lines.iter().any(|l| l.starts_with("+> capabilities")));
    assert!(lines.iter().any(|l| l.contains("session, help")));
}

// Scenario P2 : Banniere sans rbac si pas de [auth]
#[test]
fn e2e_banner_no_rbac_without_auth() {
    let (_, _, lines) = run_protocol("test echo");
    // test-config.toml has no [auth] section
    // v2: capabilities line uses +> prefix
    let caps_line = lines
        .iter()
        .find(|l| l.starts_with("+> capabilities"))
        .expect("capabilities line");
    assert!(!caps_line.contains("rbac"));
}

// Scenario P3 : Mode session — 2 commandes
#[test]
fn e2e_session_two_commands() {
    let config = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/test-config.toml"
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg("--level=ops")
        .arg(format!("--config={config}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let mut stdin = child.stdin.take().expect("stdin");
    // v2: session header
    writeln!(stdin, "+ session keepalive").expect("session");
    // v2: first command block (plain text + "." terminator)
    writeln!(stdin, "test echo").expect("cmd1");
    writeln!(stdin, ".").expect("cmd1 terminator");
    // v2: second command block
    writeln!(stdin, "test echo").expect("cmd2");
    writeln!(stdin, ".").expect("cmd2 terminator");
    // v2: "." alone = end of session (replaces "$ exit")
    writeln!(stdin, ".").expect("session end");
    drop(stdin);

    let output = child.wait_with_output().expect("wait");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // ADR 0011: final JSON responses use >>> prefix
    let response_count = stdout.lines().filter(|l| l.starts_with(">>> ")).count();
    // Should have at least 2 responses (for the 2 commands)
    assert!(
        response_count >= 2,
        "expected >= 2 >>> responses, got {response_count}. Output:\n{stdout}"
    );
}

// Scenario P4 : exit command -> fermeture propre
#[test]
fn e2e_exit_command() {
    let (code, json, _) = run_protocol("exit");
    assert_eq!(code, 0);
    assert_eq!(json["status_code"], 0);
}

// Scenario P5 : Reponse JSON a 5 champs (v2)
#[test]
fn e2e_response_has_five_fields() {
    let (_, json, _) = run_protocol("test echo");
    // v2: JSON response must have exactly these 5 fields
    assert!(json.get("command").is_some(), "missing 'command' field");
    assert!(
        json.get("status_code").is_some(),
        "missing 'status_code' field"
    );
    assert!(
        json.get("status_message").is_some(),
        "missing 'status_message' field"
    );
    assert!(json.get("stdout").is_some(), "missing 'stdout' field");
    assert!(json.get("stderr").is_some(), "missing 'stderr' field");
}

// Scenario P6 : Reponse rejetee a aussi le champ command (v2)
#[test]
fn e2e_rejected_response_has_command_field() {
    let (_, json, _) = run_protocol("nonexistent echo");
    assert_eq!(json["command"].as_str().unwrap_or(""), "nonexistent echo");
    assert_eq!(json["status_code"], 128);
}

// ================================================================
// Phase 5 — Tags integration tests (ADR 0008)
// ================================================================

/// Helper: spawn ssh-frontiere with the tags config, perform interactive auth, execute command.
/// Returns (`exit_code`, `all_stdout_lines`, `response_jsons`).
// Multi-step protocol interaction (spawn, banner, auth, command, session) forms a clear sequence
#[allow(clippy::too_many_lines)]
fn run_with_auth_tags(
    level: &str,
    secret: &str,
    token_name: &str,
    command: &str,
    session_mode: bool,
    extra_auths: &[(&str, &str)], // Additional (token_name, secret) for session auth
    extra_commands: &[&str],
) -> (i32, Vec<String>, Vec<serde_json::Value>) {
    let config = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/test-config-tags.toml"
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg(format!("--level={level}"))
        .arg(format!("--config={config}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn");

    let child_stdout = child.stdout.take().expect("stdout");
    let stdin = child.stdin.take().expect("stdin");

    // Read banner from stdout in a background thread to avoid deadlock
    let stdout_thread = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(child_stdout);
        let mut all_lines = Vec::new();
        for line in reader.lines() {
            match line {
                Ok(l) => all_lines.push(l),
                Err(_) => break,
            }
        }
        all_lines
    });

    // Extract nonce: we need to read just the banner first, but since the thread
    // reads all output, we use a brief delay to let the banner arrive, then
    // compute proof based on the known nonce from the challenge line.
    // Alternative: use the ssh-frontiere-proof binary.
    //
    // Simpler approach: since stdout is consumed by thread, we use the proof binary
    // to compute the proof. But we need the nonce first...
    //
    // Best approach: use a pipe with line-by-line reading on the main thread.

    // Actually, let's not use the thread approach. Instead, use pipe reading directly.
    drop(stdout_thread); // Cancel the thread approach

    // Re-spawn for the correct approach
    drop(stdin);
    let _ = child.wait();

    // --- Correct approach: synchronous line-by-line reading ---
    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg(format!("--level={level}"))
        .arg(format!("--config={config}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn");

    let child_stdout = child.stdout.take().expect("stdout");
    let mut stdin = child.stdin.take().expect("stdin");

    // Read banner line by line to extract nonce
    let mut reader = std::io::BufReader::new(child_stdout);
    let mut banner_lines = Vec::new();
    let mut nonce_hex = String::new();

    // Read banner lines (4-5 lines, ends with #> type "help"...)
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let trimmed = line.trim_end().to_string();
                if let Some(rest) = trimmed.strip_prefix("+> challenge nonce=") {
                    nonce_hex = rest.to_string();
                }
                let is_help_hint = trimmed.contains("type \"help\"");
                banner_lines.push(trimmed);
                if is_help_hint {
                    break; // End of banner
                }
            }
        }
    }

    // Compute proof and send +auth
    if !nonce_hex.is_empty() {
        let nonce_bytes = ssh_frontiere::crypto::hex_decode(&nonce_hex).expect("decode nonce hex");
        let proof = ssh_frontiere::crypto::compute_proof(secret.as_bytes(), &nonce_bytes);
        writeln!(stdin, "+ auth token={token_name} proof={proof}").expect("write auth");
    }

    // Session mode
    if session_mode {
        writeln!(stdin, "+ session keepalive").expect("write session");
    }

    // Write first command
    writeln!(stdin, "{command}").expect("write command");
    writeln!(stdin, ".").expect("write terminator");

    // Extra auths and commands in session mode
    for (extra_token, extra_secret) in extra_auths {
        // For extra auths in session, we need to read the new nonce from server comments
        // The server sends "#> new challenge nonce=..." after each successful auth
        // Read server response lines to find new nonce
        let mut new_nonce_hex = String::new();
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let trimmed = line.trim_end().to_string();
                    // Check for new nonce in server comments
                    if let Some(rest) = trimmed.strip_prefix("#> new challenge nonce=") {
                        new_nonce_hex = rest.to_string();
                    }
                    banner_lines.push(trimmed.clone());
                    // Stop after we see a >>> response (command result, ADR 0011)
                    if trimmed.starts_with(">>> ") {
                        break;
                    }
                }
            }
        }
        if !new_nonce_hex.is_empty() {
            let nonce_bytes =
                ssh_frontiere::crypto::hex_decode(&new_nonce_hex).expect("decode new nonce");
            let proof = ssh_frontiere::crypto::compute_proof(extra_secret.as_bytes(), &nonce_bytes);
            writeln!(stdin, "+ auth token={extra_token} proof={proof}").expect("write extra auth");
        }
    }

    for extra_cmd in extra_commands {
        writeln!(stdin, "{extra_cmd}").expect("write extra cmd");
        writeln!(stdin, ".").expect("write extra terminator");
    }

    if session_mode {
        writeln!(stdin, ".").expect("write session end");
    }
    drop(stdin);

    // Read remaining output
    let mut remaining_lines = Vec::new();
    for line in reader.lines() {
        match line {
            Ok(l) => remaining_lines.push(l),
            Err(_) => break,
        }
    }

    let output = child.wait().expect("wait");
    let code = output.code().unwrap_or(-1);

    let mut all_lines = banner_lines;
    all_lines.extend(remaining_lines);

    // Extract all >>> responses (ADR 0011: final JSON uses >>> prefix)
    let responses: Vec<serde_json::Value> = all_lines
        .iter()
        .filter_map(|line| {
            line.strip_prefix(">>> ")
                .and_then(|json| serde_json::from_str(json).ok())
        })
        .collect();

    (code, all_lines, responses)
}

// --- Scenario T1 : Sans +auth → seules actions publiques (sans tags) accessibles ---
#[test]
fn e2e_tags_no_auth_only_public() {
    // Without auth, effective_tags = empty → only actions without tags are accessible
    let config = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/test-config-tags.toml"
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg("--level=ops")
        .arg(format!("--config={config}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn");

    let stdin = child.stdin.take().expect("stdin");
    // No +auth, just send command to tagged action
    let mut writer = std::io::BufWriter::new(stdin);
    writeln!(writer, "forgejo backup").expect("write command");
    writeln!(writer, ".").expect("write terminator");
    drop(writer);

    let output = child.wait_with_output().expect("wait");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<String> = stdout
        .lines()
        .map(std::string::ToString::to_string)
        .collect();

    let response = lines
        .iter()
        .find_map(|line| {
            line.strip_prefix(">>> ")
                .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        })
        .expect("response");

    // forgejo.backup has tags=["forgejo"], no auth → tag mismatch → rejected
    assert!(response["status_message"]
        .as_str()
        .unwrap_or("")
        .contains("tag mismatch"));
}

// --- Scenario T2 : +auth avec token tagué forgejo → actions forgejo accessibles ---
#[test]
fn e2e_tags_auth_forgejo_can_access_forgejo() {
    let (code, _lines, responses) = run_with_auth_tags(
        "read",
        "secret",
        "runner-forge",
        "forgejo backup",
        false,
        &[],
        &[],
    );
    assert!(!responses.is_empty(), "expected at least one response");
    let resp = &responses[0];
    assert_eq!(resp["status_code"], 0, "forgejo backup should succeed");
    assert_eq!(code, 0);
}

// --- Scenario T3 : +auth token forgejo tente action mastodon → rejeté ---
#[test]
fn e2e_tags_auth_forgejo_cannot_access_mastodon() {
    let (code, _lines, responses) = run_with_auth_tags(
        "read",
        "secret",
        "runner-forge",
        "mastodon healthcheck",
        false,
        &[],
        &[],
    );
    assert!(!responses.is_empty(), "expected at least one response");
    let resp = &responses[0];
    assert!(
        resp["status_message"]
            .as_str()
            .unwrap_or("")
            .contains("tag mismatch"),
        "mastodon access with forgejo token should be denied"
    );
    assert_ne!(code, 0);
}

// --- Scenario T4 : +auth forgejo → action publique (sans tags) aussi accessible ---
#[test]
fn e2e_tags_auth_forgejo_can_access_public() {
    let (code, _lines, responses) = run_with_auth_tags(
        "read",
        "secret",
        "runner-forge",
        "infra status",
        false,
        &[],
        &[],
    );
    assert!(!responses.is_empty(), "expected at least one response");
    let resp = &responses[0];
    assert_eq!(
        resp["status_code"], 0,
        "infra.status (no tags) should be accessible"
    );
    assert_eq!(code, 0);
}

// --- TODO-028 : help sans préfixe → texte humain ---

// B4a : help sans préfixe → texte humain + >>> JSON final (ADR 0011)
#[test]
fn e2e_help_without_prefix_returns_text() {
    let (code, lines) = run_protocol_raw("help", "ops");
    assert_eq!(code, 0);
    // Doit contenir du texte humain
    let help_lines: Vec<&str> = lines.iter().filter_map(|l| l.strip_prefix("#> ")).collect();
    assert!(
        help_lines.iter().any(|l| l.contains("ssh-frontiere")),
        "help text should contain 'ssh-frontiere'"
    );
    assert!(
        help_lines.iter().any(|l| l.contains("Protocol")),
        "help text should contain 'Protocol'"
    );
    // ADR 0011: help now produces a >>> final JSON response
    assert!(
        lines.iter().any(|l| l.starts_with(">>> ")),
        "help should produce a >>> JSON response (ADR 0011)"
    );
}

// B4b : help avec auth → affiche les actions visibles au niveau authentifié
#[test]
fn e2e_help_with_auth_shows_auth_level_actions() {
    let (code, lines, _) = run_with_simple_auth("read", "secret", "runner", "help", false);
    assert_eq!(code, 0);
    let help_lines: Vec<&str> = lines.iter().filter_map(|l| l.strip_prefix("#> ")).collect();
    // With auth level=ops, should see ops-level actions
    assert!(
        help_lines.iter().any(|l| l.contains("greet")),
        "ops-level help should show greet action"
    );
}

// --- TODO-027 : ligne vide optionnelle entre entêtes et commande ---

// C2a : Commande sans ligne vide après entêtes → exécution OK
#[test]
fn e2e_no_empty_line_before_command() {
    // run_protocol already sends command without empty line — explicit regression test
    let (code, json, _) = run_protocol("test echo");
    assert_eq!(code, 0);
    assert_eq!(json["status_code"], 0);
    assert_eq!(json["status_message"], "executed");
}

// C2b : Commande avec ligne vide après entêtes → même résultat (rétrocompatible)
#[test]
fn e2e_with_empty_line_before_command() {
    let config = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/test-config.toml"
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg("--level=ops")
        .arg(format!("--config={config}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn");

    let mut stdin = child.stdin.take().expect("stdin");
    // Explicit empty line before command (rétrocompatible)
    writeln!(stdin).expect("write empty line");
    writeln!(stdin, "test echo").expect("write command");
    writeln!(stdin, ".").expect("write terminator");
    drop(stdin);

    let output = child.wait_with_output().expect("wait");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response_json: serde_json::Value = stdout
        .lines()
        .find_map(|line| {
            line.strip_prefix(">>> ")
                .and_then(|json| serde_json::from_str(json).ok())
        })
        .expect("response");

    assert_eq!(code, 0);
    assert_eq!(response_json["status_code"], 0);
    assert_eq!(response_json["status_message"], "executed");
}

// --- Phase 5.5 : tests intégration nonce optionnel (ADR 0010) ---

/// Helper for simple auth mode (no nonce): computes SHA-256(secret) directly
fn run_with_simple_auth(
    level: &str,
    secret: &str,
    token_name: &str,
    command: &str,
    session_mode: bool,
) -> (i32, Vec<String>, Vec<serde_json::Value>) {
    let config = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/test-config-simple-auth.toml"
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg(format!("--level={level}"))
        .arg(format!("--config={config}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn");

    let child_stdout = child.stdout.take().expect("stdout");
    let mut stdin = child.stdin.take().expect("stdin");

    // Read banner line by line (no nonce expected in simple mode)
    let mut reader = std::io::BufReader::new(child_stdout);
    let mut banner_lines = Vec::new();

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let trimmed = line.trim_end().to_string();
                let is_help_hint = trimmed.contains("type \"help\"");
                banner_lines.push(trimmed);
                if is_help_hint {
                    break;
                }
            }
        }
    }

    // Compute simple proof: SHA-256(secret)
    let proof = ssh_frontiere::crypto::compute_simple_proof(secret.as_bytes());
    writeln!(stdin, "+ auth token={token_name} proof={proof}").expect("write auth");

    if session_mode {
        writeln!(stdin, "+ session keepalive").expect("write session");
    }

    writeln!(stdin, "{command}").expect("write command");
    writeln!(stdin, ".").expect("write terminator");

    if session_mode {
        writeln!(stdin, ".").expect("write session end");
    }
    drop(stdin);

    let mut remaining_lines = Vec::new();
    for line in reader.lines() {
        match line {
            Ok(l) => remaining_lines.push(l),
            Err(_) => break,
        }
    }

    let output = child.wait().expect("wait");
    let code = output.code().unwrap_or(-1);

    let mut all_lines = banner_lines;
    all_lines.extend(remaining_lines);

    let responses: Vec<serde_json::Value> = all_lines
        .iter()
        .filter_map(|line| {
            line.strip_prefix(">>> ")
                .and_then(|json| serde_json::from_str(json).ok())
        })
        .collect();

    (code, all_lines, responses)
}

// --- SA1 : Mode simple : auth + commande → exécution ok ---
#[test]
fn e2e_simple_auth_succeeds() {
    let (code, lines, responses) =
        run_with_simple_auth("read", "secret", "runner", "test echo", false);
    // Verify no challenge line in banner
    assert!(
        !lines.iter().any(|l| l.contains("+> challenge")),
        "simple mode should not have challenge line"
    );
    assert!(!responses.is_empty(), "expected response");
    assert_eq!(responses[0]["status_code"], 0, "test echo should succeed");
    assert_eq!(code, 0);
}

// --- SA2 : Mode simple : mauvaise preuve → rejet ---
#[test]
fn e2e_simple_auth_bad_proof_rejected() {
    let config = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/test-config-simple-auth.toml"
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg("--level=read")
        .arg(format!("--config={config}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn");

    let child_stdout = child.stdout.take().expect("stdout");
    let mut stdin = child.stdin.take().expect("stdin");
    let mut reader = std::io::BufReader::new(child_stdout);

    // Read banner
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                if line.contains("type \"help\"") {
                    break;
                }
            }
        }
    }

    // Send bad proof
    writeln!(stdin, "+ auth token=runner proof=0000000000000000000000000000000000000000000000000000000000000000").expect("write auth");
    writeln!(stdin, "test echo").expect("write command");
    writeln!(stdin, ".").expect("write terminator");
    drop(stdin);

    let mut all_lines = Vec::new();
    for line in reader.lines() {
        match line {
            Ok(l) => all_lines.push(l),
            Err(_) => break,
        }
    }

    // Auth should have failed — command should still execute at base level (read)
    // since echo is level=read, it should succeed even without auth elevation
    let output = child.wait().expect("wait");
    let _code = output.code().unwrap_or(-1);

    // Check that auth failed comment is present
    assert!(
        all_lines
            .iter()
            .any(|l| l.contains("authentication failed")),
        "should report auth failure"
    );
}

// --- SA3 : Mode simple en session : re-auth sans nonce → ok ---
#[test]
fn e2e_simple_auth_session_reauth() {
    let (code, lines, responses) =
        run_with_simple_auth("read", "secret", "runner", "test echo", true);
    assert!(
        !lines.iter().any(|l| l.contains("+> challenge")),
        "simple mode should not have challenge line"
    );
    assert!(!responses.is_empty(), "expected response");
    assert_eq!(responses[0]["status_code"], 0, "echo should succeed");
    assert_eq!(code, 0);
}

// --- SA4 : Mode nonce : tests existants passent toujours (régression) ---
// Couvert par e2e_tags_auth_forgejo_can_access_forgejo et les tests existants
// qui utilisent test-config-tags.toml avec challenge_nonce = true

// --- Bloc D : --check-config validation dry-run ---

// D2a : Config valide → exit 0, stdout contient "Config OK"
#[test]
fn e2e_check_config_valid() {
    let config = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/test-config.toml"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg("--check-config")
        .arg(format!("--config={config}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .output()
        .expect("failed to spawn");

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(code, 0, "valid config should exit 0");
    assert!(
        stdout.contains("Config OK"),
        "stdout should contain 'Config OK', got: {stdout}"
    );
}

// D2b : Config invalide (TOML cassé) → exit 129, stderr contient message d'erreur
#[test]
fn e2e_check_config_invalid_toml() {
    let dir = unique_test_dir("ssh-frontiere-test-check-config");
    let _ = std::fs::create_dir_all(&dir);
    let config_path = format!("{dir}/broken.toml");
    std::fs::write(&config_path, "[global\nbroken syntax").expect("write broken config");

    let output = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg("--check-config")
        .arg(format!("--config={config_path}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .output()
        .expect("failed to spawn");

    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(code, 129, "invalid config should exit 129");
    assert!(!stderr.is_empty(), "stderr should contain error message");

    let _ = std::fs::remove_dir_all(&dir);
}

// D2c : Fichier inexistant → exit 129
#[test]
fn e2e_check_config_file_not_found() {
    let output = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg("--check-config")
        .arg("--config=/tmp/nonexistent-ssh-frontiere-config.toml")
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .output()
        .expect("failed to spawn");

    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(code, 129, "missing config should exit 129");
    assert!(!stderr.is_empty(), "stderr should contain error message");
}

// --- Phase 5.5 : tests intégration arguments nommés (ADR 0009) ---

fn run_named_args(command: &str) -> (i32, serde_json::Value, Vec<String>) {
    let config = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/test-config-named-args.toml"
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg("--level=ops")
        .arg(format!("--config={config}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn");

    let stdin = child.stdin.take().expect("stdin");
    let mut writer = std::io::BufWriter::new(stdin);
    writeln!(writer, "{command}").expect("write command");
    writeln!(writer, ".").expect("write terminator");
    drop(writer);

    let output = child.wait_with_output().expect("wait");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<String> = stdout
        .lines()
        .map(std::string::ToString::to_string)
        .collect();

    let response = lines
        .iter()
        .find_map(|line| {
            line.strip_prefix(">>> ")
                .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        })
        .unwrap_or_default();

    let code = output.status.code().unwrap_or(-1);
    (code, response, lines)
}

// --- NA1 : Commande avec args nommés → exécution OK ---
#[test]
fn e2e_named_args_explicit() {
    let (code, resp, all_lines) = run_named_args("test deploy tag=canary env=prod");
    assert_eq!(
        resp["status_code"], 0,
        "deploy with named args should succeed"
    );
    // ADR 0011: stdout is null in JSON (content was streamed via >> lines)
    assert!(resp["stdout"].is_null(), "stdout should be null (streamed)");
    let streamed: Vec<&str> = all_lines
        .iter()
        .filter_map(|l| l.strip_prefix(">> "))
        .collect();
    assert!(
        streamed.iter().any(|s| s.contains("deploy canary prod")),
        "should contain transposed args in streamed output"
    );
    assert_eq!(code, 0);
}

// --- NA2 : Commande avec defaults omis → defaults substitués ---
#[test]
fn e2e_named_args_defaults_applied() {
    let (code, resp, all_lines) = run_named_args("test deploy");
    assert_eq!(
        resp["status_code"], 0,
        "deploy with all defaults should succeed"
    );
    // ADR 0011: stdout is null in JSON (content was streamed via >> lines)
    assert!(resp["stdout"].is_null(), "stdout should be null (streamed)");
    let streamed: Vec<&str> = all_lines
        .iter()
        .filter_map(|l| l.strip_prefix(">> "))
        .collect();
    assert!(
        streamed.iter().any(|s| s.contains("deploy latest staging")),
        "should use default values in streamed output"
    );
    assert_eq!(code, 0);
}

// --- NA3 : Arg positionnel (sans =) → erreur ---
#[test]
fn e2e_named_args_positional_rejected() {
    let (code, resp, _) = run_named_args("test greet world");
    assert_ne!(code, 0, "positional arg should be rejected");
    assert!(
        resp["status_message"]
            .as_str()
            .unwrap_or("")
            .contains("key=value"),
        "should mention key=value syntax"
    );
}

// --- NA4 : Arg obligatoire manquant → erreur ---
#[test]
fn e2e_named_args_mandatory_missing() {
    let (code, resp, _) = run_named_args("test greet");
    assert_ne!(code, 0, "missing mandatory arg should be rejected");
    assert!(
        resp["status_message"]
            .as_str()
            .unwrap_or("")
            .contains("missing"),
        "should mention missing argument"
    );
}

// =====================================================================
// Phase 9 : Body integration tests (ADR 0012)
// =====================================================================

const BODY_CONFIG: &str = r#"
[global]
log_file = "/tmp/ssh-frontiere-test-body/commands.json"

[domains.test]
description = "Body test"

[domains.test.actions.cat]
description = "Cat stdin"
level = "read"
timeout = 10
execute = "/bin/cat"

[domains.test.actions.echo]
description = "Echo"
level = "read"
timeout = 10
execute = "/bin/echo hello"

[domains.test.actions.notify]
description = "Notify"
level = "read"
timeout = 10
execute = "/bin/echo {message}"

[domains.test.actions.notify.args]
message = { free = true }

[domains.test.actions.limited]
description = "Limited body"
level = "read"
timeout = 10
execute = "/bin/cat"
max_body_size = 32
"#;

fn run_body_test(
    headers: &[&str],
    command: &str,
    body: &str,
    body_mode: &str,
) -> (i32, serde_json::Value, Vec<String>) {
    let dir = unique_test_dir("ssh-frontiere-test-body");
    let _ = std::fs::create_dir_all(&dir);
    let config_path = format!("{dir}/config.toml");
    std::fs::write(&config_path, BODY_CONFIG).expect("write config");

    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg("--level=ops")
        .arg(format!("--config={config_path}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn");

    let mut stdin = child.stdin.take().expect("stdin");

    // Write headers
    for h in headers {
        writeln!(stdin, "{h}").expect("write header");
    }
    // +body directive
    writeln!(stdin, "+ {body_mode}").expect("write body mode");
    // Command
    writeln!(stdin, "{command}").expect("write command");
    writeln!(stdin, ".").expect("write terminator");
    // Body content
    write!(stdin, "{body}").expect("write body");

    drop(stdin);

    let output = child.wait_with_output().expect("wait");
    let code = output.status.code().unwrap_or(-1);
    let stdout_str = String::from_utf8_lossy(&output.stdout);

    let all_lines: Vec<String> = stdout_str
        .lines()
        .map(std::string::ToString::to_string)
        .collect();

    let response_json = all_lines
        .iter()
        .find_map(|line| {
            line.strip_prefix(">>> ")
                .and_then(|json| serde_json::from_str(json).ok())
        })
        .unwrap_or_else(|| panic!("no >>> response found in output:\n{stdout_str}"));

    let _ = std::fs::remove_dir_all(&dir);

    (code, response_json, all_lines)
}

// --- BOD-IT-1 : Body mode default ---
#[test]
fn e2e_body_default_mode() {
    let (code, resp, lines) = run_body_test(&[], "test cat", "hello body\n.\n", "body");
    assert_eq!(code, 0);
    assert_eq!(resp["status_code"], 0);
    assert!(
        lines.iter().any(|l| l.contains(">> hello body")),
        "stdout should contain body content: {lines:?}"
    );
}

// --- BOD-IT-2 : Body mode size=N ---
#[test]
fn e2e_body_size_mode() {
    let (code, resp, lines) = run_body_test(&[], "test cat", "hello", "body size=5");
    assert_eq!(code, 0);
    assert_eq!(resp["status_code"], 0);
    assert!(
        lines.iter().any(|l| l.contains(">> hello")),
        "stdout should contain body: {lines:?}"
    );
}

// --- BOD-IT-3 : Body mode stop="FIN" ---
#[test]
fn e2e_body_stop_mode() {
    let (code, resp, lines) =
        run_body_test(&[], "test cat", "line1\nline2\nFIN\n", "body stop=\"FIN\"");
    assert_eq!(code, 0);
    assert_eq!(resp["status_code"], 0);
    assert!(
        lines.iter().any(|l| l.contains(">> line1")),
        "stdout should contain body lines: {lines:?}"
    );
}

// --- BOD-IT-4 : free=true argument ---
#[test]
fn e2e_free_arg() {
    let (code, resp, lines) =
        run_protocol_with_config("test notify message=\"bonjour le monde\"", BODY_CONFIG);
    assert_eq!(code, 0);
    assert_eq!(resp["status_code"], 0);
    assert!(
        lines.iter().any(|l| l.contains(">> bonjour le monde")),
        "stdout should contain free text: {lines:?}"
    );
}

// --- BOD-IT-5 : Banner contains body capability ---
#[test]
fn e2e_banner_has_body_capability() {
    let (_, lines) = run_protocol_raw("test echo", "ops");
    let caps_line = lines
        .iter()
        .find(|l| l.contains("+> capabilities"))
        .expect("should have capabilities line");
    assert!(
        caps_line.contains("body"),
        "capabilities should include body: {caps_line}"
    );
}

// --- BOD-IT-6 : Help text shows free args ---
#[test]
fn e2e_help_text_shows_free_args() {
    let dir = unique_test_dir("ssh-frontiere-test-body");
    let _ = std::fs::create_dir_all(&dir);
    let config_path = format!("{dir}/config.toml");
    std::fs::write(&config_path, BODY_CONFIG).expect("write config");

    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg("--level=ops")
        .arg(format!("--config={config_path}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn");

    let mut stdin = child.stdin.take().expect("stdin");
    writeln!(stdin, "help test").expect("write cmd");
    writeln!(stdin, ".").expect("write term");
    drop(stdin);

    let output = child.wait_with_output().expect("wait");
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout_str.lines().collect();

    // Help text should mention [free text] for free args
    assert!(
        lines.iter().any(|l| l.contains("[free text]")),
        "help should show [free text] for free args: {lines:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
