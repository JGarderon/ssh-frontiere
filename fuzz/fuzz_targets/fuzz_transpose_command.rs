#![no_main]
use libfuzzer_sys::fuzz_target;
use std::collections::HashMap;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Split input into template and value
        let parts: Vec<&str> = s.splitn(2, '\n').collect();
        let template = parts.first().copied().unwrap_or("");
        let value = parts.get(1).copied().unwrap_or("");

        let mut args = HashMap::new();
        args.insert("arg".to_string(), value.to_string());

        let result = ssh_frontiere::fuzz_helpers::transpose_command(template, "dom", &args);

        // Invariant: first token is always the command (never empty if template non-empty)
        if !template.trim().is_empty() {
            assert!(!result.is_empty(), "non-empty template must produce tokens");
        }
    }
});
