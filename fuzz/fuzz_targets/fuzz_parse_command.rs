#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // parse_command must never panic on any input string
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = ssh_frontiere::fuzz_helpers::parse_command(input);
    }
});
