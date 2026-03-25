// --- SHA-256 (FIPS 180-4) implementation en Rust pur ---
// Relocated from logging.rs for shared use (auth + log masking)

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

const H0: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// Compute SHA-256 digest, return hex string
pub fn sha256(data: &[u8]) -> String {
    let digest = sha256_bytes(data);
    digest.iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write;
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

/// Compute SHA-256 digest, return raw bytes
pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let padded = sha256_pad(data);
    let mut h = H0;

    for chunk in padded.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            // PANIC-SAFE: chunk is exactly 64 bytes from sha256_pad (padded to multiple of 64); i*4+3 max = 15*4+3 = 63
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        // PANIC-SAFE: w is [u32; 64], i ranges 16..64, all indices i-16..i are within 0..64
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        // PANIC-SAFE: h is [u32; 8], indices 0..7 always valid
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        // PANIC-SAFE: K is [u32; 64] and w is [u32; 64], i ranges 0..64
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        // PANIC-SAFE: h is [u32; 8], indices 0..7 always valid
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut result = [0u8; 32];
    for (i, val) in h.iter().enumerate() {
        // PANIC-SAFE: h has 8 elements, so i ranges 0..8; i*4+4 max = 32 = result.len()
        result[i * 4..i * 4 + 4].copy_from_slice(&val.to_be_bytes());
    }
    result
}

fn sha256_pad(data: &[u8]) -> Vec<u8> {
    let bit_len = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    padded
}

// --- Base64 decoder (RFC 4648) ---

fn b64_decode_char(c: u8) -> Result<u8, String> {
    match c {
        b'A'..=b'Z' => Ok(c - b'A'),
        b'a'..=b'z' => Ok(c - b'a' + 26),
        b'0'..=b'9' => Ok(c - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(format!("invalid base64 character: {}", c as char)),
    }
}

/// Decode base64 string to bytes (RFC 4648)
#[must_use = "decode result must be checked"]
pub fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(Vec::new());
    }

    // Strip padding and whitespace
    let clean: Vec<u8> = input.bytes().filter(|&b| b != b'=').collect();

    let mut output = Vec::with_capacity(clean.len() * 3 / 4);
    let chunks = clean.chunks(4);

    for chunk in chunks {
        let len = chunk.len();
        if len < 2 {
            return Err("invalid base64: truncated input".to_string());
        }

        // PANIC-SAFE: checked len >= 2 above (early return if len < 2)
        let a = b64_decode_char(chunk[0])?;
        let b = b64_decode_char(chunk[1])?;
        output.push((a << 2) | (b >> 4));

        if len > 2 {
            // PANIC-SAFE: len > 2 checked by if condition
            let c = b64_decode_char(chunk[2])?;
            output.push((b << 4) | (c >> 2));

            if len > 3 {
                // PANIC-SAFE: len > 3 checked by if condition
                let d = b64_decode_char(chunk[3])?;
                output.push((c << 6) | d);
            }
        }
    }

    Ok(output)
}

/// Decode a b64:-prefixed string, or return error if prefix is missing
#[must_use = "decode result must be checked"]
pub fn decode_b64_secret(value: &str) -> Result<Vec<u8>, String> {
    let stripped = value
        .strip_prefix("b64:")
        .ok_or_else(|| "secret must start with 'b64:' prefix".to_string())?;
    base64_decode(stripped)
}

// --- Hex encoding ---

/// Encode bytes as lowercase hex string
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write;
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

/// Decode hex string to bytes
// Used by lib.rs → proof.rs (ssh-frontiere-proof binary), dead for the main binary
#[must_use = "decode result must be checked"]
#[allow(dead_code)]
pub fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err("hex string must have even length".to_string());
    }

    (0..hex.len())
        .step_by(2)
        .map(|i| {
            // PANIC-SAFE: hex has even length (checked above); step_by(2) ensures i+2 <= hex.len()
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("invalid hex at position {i}: {e}"))
        })
        .collect()
}

// --- Nonce generation ---

/// Generate 16 random bytes from /dev/urandom
#[must_use = "nonce generation result must be checked"]
pub fn generate_nonce() -> Result<[u8; 16], String> {
    use std::io::Read;
    let mut file = std::fs::File::open("/dev/urandom")
        .map_err(|e| format!("cannot open /dev/urandom: {e}"))?;
    let mut buf = [0u8; 16];
    file.read_exact(&mut buf)
        .map_err(|e| format!("cannot read /dev/urandom: {e}"))?;
    Ok(buf)
}

// --- XOR stream cipher (ADR 0006 §3) ---

/// XOR encrypt/decrypt using SHA-256 CTR keystream
/// keystream = SHA-256(secret || 0x00) || SHA-256(secret || 0x01) || ...
pub fn xor_encrypt(plaintext: &[u8], secret: &[u8]) -> Vec<u8> {
    debug_assert!(
        plaintext.len() < 8192,
        "XOR-CTR keystream repeats after 8192 bytes"
    );
    let mut ciphertext = Vec::with_capacity(plaintext.len());
    let mut counter: u8 = 0;
    let mut keystream_pos = 32; // Force first block generation
    let mut keystream_block = [0u8; 32];

    for &byte in plaintext {
        if keystream_pos >= 32 {
            // Generate next keystream block: SHA-256(secret || counter)
            let mut input = Vec::with_capacity(secret.len() + 1);
            input.extend_from_slice(secret);
            input.push(counter);
            keystream_block = sha256_bytes(&input);
            keystream_pos = 0;
            counter = counter.wrapping_add(1);
        }
        // PANIC-SAFE: keystream_pos is reset to 0 when >= 32, so always in 0..31; keystream_block is [u8; 32]
        ciphertext.push(byte ^ keystream_block[keystream_pos]);
        keystream_pos += 1;
    }

    ciphertext
}

// --- Challenge-response (ADR 0006 §3) ---

/// Compute challenge-response proof: SHA-256(XOR_encrypt(secret || nonce))
pub fn compute_proof(secret: &[u8], nonce: &[u8]) -> String {
    let mut plaintext = Vec::with_capacity(secret.len() + nonce.len());
    plaintext.extend_from_slice(secret);
    plaintext.extend_from_slice(nonce);

    let ciphertext = xor_encrypt(&plaintext, secret);
    sha256(&ciphertext)
}

// --- Simple proof (ADR 0010 — mode sans nonce) ---

/// Compute simple proof: SHA-256(secret) — no nonce
pub fn compute_simple_proof(secret: &[u8]) -> String {
    sha256(secret)
}

/// Verify simple proof (constant-time comparison)
#[must_use]
pub fn verify_simple_proof(secret: &[u8], proof_hex: &str) -> bool {
    let expected = compute_simple_proof(secret);
    constant_time_eq(expected.as_bytes(), proof_hex.as_bytes())
}

/// Verify a challenge-response proof (constant-time comparison)
#[must_use]
pub fn verify_proof(secret: &[u8], nonce: &[u8], proof_hex: &str) -> bool {
    let expected = compute_proof(secret, nonce);
    constant_time_eq(expected.as_bytes(), proof_hex.as_bytes())
}

/// Constant-time byte comparison (protection against timing side-channels)
/// `#[inline(never)]` + `black_box` prevent LLVM from optimizing the loop
/// into a short-circuit comparison (ANSSI recommendation).
#[must_use]
#[inline(never)]
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    core::hint::black_box(diff) == 0
}
