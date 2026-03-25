/// Tests de conformite ADR 0003 v2 — binaire Rust (protocole v2, ADR 0006 v2, ADR 0011)
///
/// Lit les scenarios depuis tests/fixtures/conformance-scenarios.json
/// et verifie que le binaire ssh-frontiere produit les reponses attendues.
/// Protocole v2 + ADR 0011 : commandes en texte brut terminées par ".", réponses ">>> {json 5 champs}".
use std::io::Write;
use std::process::{Command, Stdio};

const SCENARIOS_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/conformance-scenarios.json"
);

const TEST_CONFIG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/test-config.toml"
);

#[derive(Debug)]
struct Scenario {
    id: String,
    description: String,
    level: String,
    command: String,
    expect_exit_code: i32,
    expect_status_code: i32,
    expect_status_message_contains: String,
    expect_stdout_is_null: bool,
    expect_stderr_is_null: bool,
}

fn load_scenarios() -> Vec<Scenario> {
    let content = std::fs::read_to_string(SCENARIOS_PATH).expect("read conformance-scenarios.json");
    let root: serde_json::Value = serde_json::from_str(&content).expect("parse JSON");
    let scenarios = root["scenarios"].as_array().expect("scenarios array");

    scenarios
        .iter()
        .map(|s| Scenario {
            id: s["id"].as_str().expect("id").to_string(),
            description: s["description"].as_str().expect("description").to_string(),
            level: s["expect"]["exit_code"]
                .as_i64()
                .map(|_| s["level"].as_str().expect("level").to_string())
                .expect("level"),
            command: s["command"].as_str().expect("command").to_string(),
            expect_exit_code: s["expect"]["exit_code"].as_i64().expect("exit_code") as i32,
            expect_status_code: s["expect"]["status_code"].as_i64().expect("status_code") as i32,
            expect_status_message_contains: s["expect"]["status_message_contains"]
                .as_str()
                .expect("status_message_contains")
                .to_string(),
            expect_stdout_is_null: s["expect"]["stdout_is_null"]
                .as_bool()
                .expect("stdout_is_null"),
            expect_stderr_is_null: s["expect"]["stderr_is_null"]
                .as_bool()
                .expect("stderr_is_null"),
        })
        .collect()
}

fn run_scenario(scenario: &Scenario) -> (i32, serde_json::Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-frontiere"))
        .arg(format!("--level={}", scenario.level))
        .arg(format!("--config={TEST_CONFIG}"))
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("[{}] failed to spawn: {e}", scenario.id));

    let mut stdin = child.stdin.take().expect("stdin");

    // Send command as plain text + "." terminator (protocol v2)
    writeln!(stdin, "{}", scenario.command).expect("write command");
    writeln!(stdin, ".").expect("write terminator");

    drop(stdin);

    let output = child.wait_with_output().expect("wait");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Find the first >>> response line and parse its JSON (ADR 0011)
    let parsed = stdout
        .lines()
        .find_map(|line| {
            line.strip_prefix(">>> ")
                .and_then(|json| serde_json::from_str(json).ok())
        })
        .unwrap_or_else(|| panic!("[{}] no >>> response in output:\n{stdout}", scenario.id));

    (code, parsed)
}

#[test]
fn conformance_all_scenarios() {
    let scenarios = load_scenarios();
    assert!(
        !scenarios.is_empty(),
        "no scenarios loaded from conformance-scenarios.json"
    );

    for scenario in &scenarios {
        let (code, json) = run_scenario(scenario);

        assert_eq!(
            code, scenario.expect_exit_code,
            "[{}] exit code mismatch: expected {}, got {} — {}",
            scenario.id, scenario.expect_exit_code, code, scenario.description
        );

        assert_eq!(
            json["status_code"].as_i64().unwrap_or(-1) as i32,
            scenario.expect_status_code,
            "[{}] status_code mismatch — {}",
            scenario.id,
            scenario.description
        );

        let status_msg = json["status_message"].as_str().unwrap_or("");
        assert!(
            status_msg.contains(&scenario.expect_status_message_contains),
            "[{}] status_message '{}' does not contain '{}' — {}",
            scenario.id,
            status_msg,
            scenario.expect_status_message_contains,
            scenario.description
        );

        if scenario.expect_stdout_is_null {
            assert!(
                json["stdout"].is_null(),
                "[{}] expected stdout=null — {}",
                scenario.id,
                scenario.description
            );
        } else {
            assert!(
                !json["stdout"].is_null(),
                "[{}] expected stdout non-null — {}",
                scenario.id,
                scenario.description
            );
        }

        if scenario.expect_stderr_is_null {
            assert!(
                json["stderr"].is_null(),
                "[{}] expected stderr=null — {}",
                scenario.id,
                scenario.description
            );
        } else {
            assert!(
                !json["stderr"].is_null(),
                "[{}] expected stderr non-null — {}",
                scenario.id,
                scenario.description
            );
        }
    }
}
