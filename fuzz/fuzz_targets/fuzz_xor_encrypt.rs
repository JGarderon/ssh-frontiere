#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Split input into plaintext and secret (need at least 1 byte for secret)
    if data.len() < 2 {
        return;
    }
    let split = data.len() / 2;
    let plaintext = &data[..split];
    let secret = &data[split..];

    // xor_encrypt must never panic
    let ciphertext = ssh_frontiere::crypto::xor_encrypt(plaintext, secret);

    // Invariant: output length == input length
    assert_eq!(ciphertext.len(), plaintext.len());

    // Invariant: XOR is its own inverse (decrypt == encrypt with same key)
    let decrypted = ssh_frontiere::crypto::xor_encrypt(&ciphertext, secret);
    assert_eq!(decrypted, plaintext);
});
