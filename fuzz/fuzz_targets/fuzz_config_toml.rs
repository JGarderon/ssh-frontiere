#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Must not panic regardless of input
        let _ = ssh_frontiere::fuzz_helpers::parse_config(s);
    }
});
