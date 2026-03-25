#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // hex_decode must never panic, only return Ok or Err
    if let Ok(input) = std::str::from_utf8(data) {
        if let Ok(bytes) = ssh_frontiere::crypto::hex_decode(input) {
            // Roundtrip: encode(decode(hex)) == lowercase(hex) for valid hex
            let re_encoded = ssh_frontiere::crypto::hex_encode(&bytes);
            assert_eq!(re_encoded, input.to_lowercase());
        }
    }
});
