#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // SHA-256 must never panic on any input
    let hex = ssh_frontiere::crypto::sha256(data);

    // Basic invariants: always 64 hex chars, lowercase hex only
    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
});
