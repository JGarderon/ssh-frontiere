#[cfg(test)]
mod tests {
    use crate::crypto::*;

    // --- SHA-256 NIST vectors (relocated from logging_tests.rs) ---

    #[test]
    fn sha256_empty() {
        assert_eq!(
            sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_abc() {
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_448bit() {
        assert_eq!(
            sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn sha256_55_bytes() {
        let input = vec![0x61u8; 55];
        assert_eq!(
            sha256(&input),
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
        );
    }

    #[test]
    fn sha256_64_bytes() {
        let input = vec![0x61u8; 64];
        assert_eq!(
            sha256(&input),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
    }

    #[test]
    fn sha256_bytes_returns_raw() {
        let digest = sha256_bytes(b"abc");
        assert_eq!(digest[0], 0xba);
        assert_eq!(digest[1], 0x78);
        assert_eq!(digest.len(), 32);
    }

    // --- Base64 RFC 4648 vectors ---

    #[test]
    fn base64_decode_empty() {
        assert_eq!(base64_decode("").expect("empty"), Vec::<u8>::new());
    }

    #[test]
    fn base64_decode_f() {
        assert_eq!(base64_decode("Zg==").expect("f"), b"f");
    }

    #[test]
    fn base64_decode_fo() {
        assert_eq!(base64_decode("Zm8=").expect("fo"), b"fo");
    }

    #[test]
    fn base64_decode_foo() {
        assert_eq!(base64_decode("Zm9v").expect("foo"), b"foo");
    }

    #[test]
    fn base64_decode_foob() {
        assert_eq!(base64_decode("Zm9vYg==").expect("foob"), b"foob");
    }

    #[test]
    fn base64_decode_fooba() {
        assert_eq!(base64_decode("Zm9vYmE=").expect("fooba"), b"fooba");
    }

    #[test]
    fn base64_decode_foobar() {
        assert_eq!(base64_decode("Zm9vYmFy").expect("foobar"), b"foobar");
    }

    #[test]
    fn base64_decode_invalid_char() {
        assert!(base64_decode("Zm9v!!!").is_err());
    }

    #[test]
    fn base64_decode_no_padding() {
        // base64 without padding should also work
        assert_eq!(base64_decode("Zg").expect("f no pad"), b"f");
        assert_eq!(base64_decode("Zm8").expect("fo no pad"), b"fo");
    }

    #[test]
    fn decode_b64_secret_with_prefix() {
        let decoded = decode_b64_secret("b64:Zm9vYmFy").expect("decode");
        assert_eq!(decoded, b"foobar");
    }

    #[test]
    fn decode_b64_secret_without_prefix() {
        assert!(decode_b64_secret("Zm9vYmFy").is_err());
    }

    #[test]
    fn decode_b64_secret_invalid_base64() {
        assert!(decode_b64_secret("b64:!!!invalid").is_err());
    }

    // --- Hex encoding/decoding ---

    #[test]
    fn hex_encode_roundtrip() {
        let data = [0xde, 0xad, 0xbe, 0xef];
        assert_eq!(hex_encode(&data), "deadbeef");
    }

    #[test]
    fn hex_decode_valid() {
        assert_eq!(
            hex_decode("deadbeef").expect("decode"),
            vec![0xde, 0xad, 0xbe, 0xef]
        );
    }

    #[test]
    fn hex_decode_odd_length() {
        assert!(hex_decode("abc").is_err());
    }

    #[test]
    fn hex_decode_invalid_chars() {
        assert!(hex_decode("zzzz").is_err());
    }

    // --- Nonce generation ---

    #[test]
    fn generate_nonce_produces_16_bytes() {
        let nonce = generate_nonce().expect("nonce");
        assert_eq!(nonce.len(), 16);
    }

    #[test]
    fn generate_nonce_not_all_zeros() {
        let nonce = generate_nonce().expect("nonce");
        // Probability of all zeros from /dev/urandom is ~2^-128
        assert!(nonce.iter().any(|&b| b != 0));
    }

    #[test]
    fn generate_nonce_unique() {
        let n1 = generate_nonce().expect("n1");
        let n2 = generate_nonce().expect("n2");
        assert_ne!(n1, n2);
    }

    // --- XOR stream cipher ---

    #[test]
    fn xor_encrypt_deterministic() {
        let secret = b"my-secret-key";
        let plaintext = b"hello world";
        let ct1 = xor_encrypt(plaintext, secret);
        let ct2 = xor_encrypt(plaintext, secret);
        assert_eq!(ct1, ct2);
    }

    #[test]
    fn xor_encrypt_reversible() {
        let secret = b"my-secret-key";
        let plaintext = b"hello world";
        let ciphertext = xor_encrypt(plaintext, secret);
        let decrypted = xor_encrypt(&ciphertext, secret);
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn xor_encrypt_different_key_different_output() {
        let plaintext = b"hello";
        let ct1 = xor_encrypt(plaintext, b"key1");
        let ct2 = xor_encrypt(plaintext, b"key2");
        assert_ne!(ct1, ct2);
    }

    #[test]
    fn xor_encrypt_longer_than_32_bytes() {
        // Plaintext > 32 bytes requires CTR mode (multiple keystream blocks)
        let secret = b"key";
        let plaintext = vec![0x42u8; 100];
        let ciphertext = xor_encrypt(&plaintext, secret);
        assert_eq!(ciphertext.len(), 100);
        // Verify reversibility
        let decrypted = xor_encrypt(&ciphertext, secret);
        assert_eq!(decrypted, plaintext);
    }

    // --- Challenge-response ---

    #[test]
    fn compute_proof_deterministic() {
        let secret = b"my-secret";
        let nonce = [1u8; 16];
        let p1 = compute_proof(secret, &nonce);
        let p2 = compute_proof(secret, &nonce);
        assert_eq!(p1, p2);
    }

    #[test]
    fn compute_proof_is_hex_64_chars() {
        let proof = compute_proof(b"secret", &[0u8; 16]);
        assert_eq!(proof.len(), 64);
        assert!(proof.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn compute_proof_different_nonce_different_proof() {
        let secret = b"secret";
        let p1 = compute_proof(secret, &[1u8; 16]);
        let p2 = compute_proof(secret, &[2u8; 16]);
        assert_ne!(p1, p2);
    }

    #[test]
    fn compute_proof_different_secret_different_proof() {
        let nonce = [1u8; 16];
        let p1 = compute_proof(b"secret1", &nonce);
        let p2 = compute_proof(b"secret2", &nonce);
        assert_ne!(p1, p2);
    }

    #[test]
    fn verify_proof_valid() {
        let secret = b"test-secret";
        let nonce = [0xab; 16];
        let proof = compute_proof(secret, &nonce);
        assert!(verify_proof(secret, &nonce, &proof));
    }

    #[test]
    fn verify_proof_invalid() {
        let secret = b"test-secret";
        let nonce = [0xab; 16];
        assert!(!verify_proof(
            secret,
            &nonce,
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));
    }

    #[test]
    fn verify_proof_wrong_length() {
        let secret = b"test-secret";
        let nonce = [0xab; 16];
        assert!(!verify_proof(secret, &nonce, "too-short"));
    }

    // --- Simple proof (ADR 0010) ---

    #[test]
    fn compute_simple_proof_known_vector() {
        // SHA-256("secret") = known hex value
        let proof = compute_simple_proof(b"secret");
        assert_eq!(proof, sha256(b"secret"));
        assert_eq!(proof.len(), 64);
    }

    #[test]
    fn verify_simple_proof_correct() {
        let secret = b"test-secret";
        let proof = compute_simple_proof(secret);
        assert!(verify_simple_proof(secret, &proof));
    }

    #[test]
    fn verify_simple_proof_incorrect() {
        let secret = b"test-secret";
        assert!(!verify_simple_proof(
            secret,
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));
    }

    // --- Constant-time comparison ---

    #[test]
    fn constant_time_eq_equal() {
        assert!(constant_time_eq(b"hello", b"hello"));
    }

    #[test]
    fn constant_time_eq_different() {
        assert!(!constant_time_eq(b"hello", b"world"));
    }

    #[test]
    fn constant_time_eq_different_length() {
        assert!(!constant_time_eq(b"hello", b"hell"));
    }

    #[test]
    fn constant_time_eq_empty() {
        assert!(constant_time_eq(b"", b""));
    }
}
