mod auth;
mod auth_tests;
mod chain_exec;
mod chain_exec_tests;
mod chain_parser;
mod chain_parser_tests;
mod config;
mod config_tests;
mod crypto;
mod crypto_tests;
mod discovery;
mod discovery_tests;
mod dispatch;
mod dispatch_tests;
mod executor;
mod executor_tests;
mod logging;
mod logging_tests;
mod orchestrator;
mod output;
mod output_tests;
mod proptest_tests;
mod protocol;
mod protocol_tests;

fn main() {
    let exit_code = orchestrator::run(&flatten_args());
    std::process::exit(exit_code);
}

/// Flatten args: when invoked via sshd with command= in `authorized_keys`,
/// sshd runs: <shell> -c "<command string>". Split the -c value to extract
/// --level and --config flags.
fn flatten_args() -> Vec<String> {
    let raw: Vec<String> = std::env::args().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        // PANIC-SAFE: i < raw.len() checked by while loop condition
        if raw[i] == "-c" {
            if let Some(cmd_str) = raw.get(i + 1) {
                result.extend(
                    cmd_str
                        .split_whitespace()
                        .map(std::string::ToString::to_string),
                );
                i += 2;
                continue;
            }
        }
        // PANIC-SAFE: i < raw.len() checked by while loop condition
        result.push(raw[i].clone());
        i += 1;
    }
    result
}
