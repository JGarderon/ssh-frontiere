#[cfg(test)]
mod tests {
    use crate::crypto::sha256;
    use crate::logging::*;

    // --- SHA-256 NIST test vectors ---

    #[test]
    fn sha256_empty_string() {
        let hash = sha256(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_abc() {
        let hash = sha256(b"abc");
        assert_eq!(
            hash,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_longer_message() {
        let hash = sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");
        assert_eq!(
            hash,
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    // --- SHA-256 NIST FIPS 180-4 supplementary vectors (TODO-008) ---
    // Sources :
    //   - NIST FIPS 180-4 (Secure Hash Standard), Section 6.2
    //   - Valeurs cross-verifiees avec Node.js (crypto/OpenSSL) et Perl (Digest::SHA)

    #[test]
    fn sha256_nist_55_bytes_padding_boundary() {
        // 55 octets : juste sous la frontiere de padding (55+1+8=64 = 1 bloc exact)
        let input = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; // 55 × 'a'
        assert_eq!(input.len(), 55);
        let hash = sha256(input);
        assert_eq!(
            hash,
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
        );
    }

    #[test]
    fn sha256_nist_64_bytes_full_block() {
        // 64 octets : un bloc complet, le padding deborde sur un second bloc
        let input: Vec<u8> = vec![b'a'; 64];
        assert_eq!(input.len(), 64);
        let hash = sha256(&input);
        assert_eq!(
            hash,
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
    }

    #[test]
    fn sha256_nist_112_bytes_two_blocks() {
        // Vecteur officiel NIST FIPS 180-4 (896 bits, 2 blocs)
        let input = b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";
        assert_eq!(input.len(), 112);
        let hash = sha256(input);
        assert_eq!(
            hash,
            "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1"
        );
    }

    #[test]
    fn sha256_nist_one_million_a() {
        // Vecteur officiel NIST FIPS 180-4 : 1 000 000 repetitions de 'a' (stress test)
        let input: Vec<u8> = vec![b'a'; 1_000_000];
        let hash = sha256(&input);
        assert_eq!(
            hash,
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    // --- Log events ---

    #[test]
    fn log_event_executed_is_valid_json() {
        let mut entry = LogEntry::new("executed")
            .with_domain("forgejo")
            .with_action("backup-config");
        entry.duration_ms = Some(150);
        let json = entry.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert_eq!(parsed["event"], "executed");
        assert_eq!(parsed["domain"], "forgejo");
        assert_eq!(parsed["action"], "backup-config");
        assert_eq!(parsed["duration_ms"], 150);
    }

    #[test]
    fn log_event_rejected() {
        let entry = LogEntry::new("rejected").with_reason("unknown action 'foo'");
        let json = entry.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert_eq!(parsed["event"], "rejected");
        assert_eq!(parsed["reason"], "unknown action 'foo'");
    }

    #[test]
    fn log_event_with_ssh_client() {
        let entry = LogEntry::new("executed").with_ssh_client("192.168.1.1 12345 22");
        let json = entry.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert_eq!(parsed["ssh_client"], "192.168.1.1 12345 22");
    }

    #[test]
    fn log_event_has_timestamp_and_pid() {
        let entry = LogEntry::new("test");
        let json = entry.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert!(parsed["timestamp"].is_string());
        assert!(parsed["pid"].is_number());
    }

    #[test]
    fn test_log_entry_with_tags() {
        let mut entry = LogEntry::new("executed");
        entry.effective_tags = vec!["forgejo".to_string(), "infra".to_string()];
        entry.action_tags = vec!["forgejo".to_string()];
        let json = entry.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        let eff = parsed["effective_tags"].as_array().expect("array");
        assert_eq!(eff.len(), 2);
        assert!(eff.iter().any(|v| v.as_str() == Some("forgejo")));
        assert!(eff.iter().any(|v| v.as_str() == Some("infra")));
        let act = parsed["action_tags"].as_array().expect("array");
        assert_eq!(act.len(), 1);
        assert_eq!(act[0].as_str(), Some("forgejo"));
    }

    #[test]
    fn test_log_entry_empty_tags() {
        let entry = LogEntry::new("executed");
        let json = entry.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        let eff = parsed["effective_tags"].as_array().expect("array");
        assert!(eff.is_empty());
        let act = parsed["action_tags"].as_array().expect("array");
        assert!(act.is_empty());
    }

    // --- Write to file ---

    #[test]
    fn log_entry_defaults_applied_present() {
        let mut entry = LogEntry::new("executed");
        entry.defaults_applied = vec!["tag".to_string(), "env".to_string()];
        let json = entry.to_json();
        assert!(json.contains("defaults_applied"));
        assert!(json.contains("tag"));
        assert!(json.contains("env"));
    }

    #[test]
    fn log_entry_defaults_applied_empty_omitted() {
        let entry = LogEntry::new("executed");
        let json = entry.to_json();
        assert!(
            !json.contains("defaults_applied"),
            "empty defaults_applied should be omitted"
        );
    }

    #[test]
    fn write_log_to_temp_file() {
        let dir = std::env::temp_dir().join("ssh-frontiere-test-log");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.json");
        let entry = LogEntry::new("test_event");
        let result = write_log(&path.to_string_lossy(), &entry);
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&path).expect("read log");
        assert!(content.contains("test_event"));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
}
