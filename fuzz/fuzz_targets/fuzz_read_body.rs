#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut reader = std::io::BufReader::new(s.as_bytes());
        // Mode default
        let _ = ssh_frontiere::fuzz_helpers::read_body_default(&mut reader, 65536);

        // Mode size (use first 2 bytes as size if available)
        if data.len() >= 2 {
            let n = u16::from_le_bytes([data[0], data[1]]) as usize;
            let mut reader2 = std::io::BufReader::new(&data[2..]);
            let _ = ssh_frontiere::fuzz_helpers::read_body_size(&mut reader2, n, 65536);
        }
    }
});
