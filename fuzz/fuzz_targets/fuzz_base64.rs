#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // base64_decode must never panic, only return Ok or Err
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = ssh_frontiere::crypto::base64_decode(input);
    }
});
