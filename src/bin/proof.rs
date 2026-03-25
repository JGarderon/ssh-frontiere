//! SSH Frontière proof calculator — computes challenge-response proofs for E2E tests.
//!
//! Usage:
//!   ssh-frontiere-proof --secret `<raw-secret>` --nonce `<hex-nonce>`   # nonce mode
//!   ssh-frontiere-proof --secret `<raw-secret>`                         # simple mode (SHA-256)
//! Output: hex proof string on stdout

// Access the library modules from the main crate
use ssh_frontiere::crypto;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let secret = get_required_arg(&args, "--secret");
    let nonce_hex = get_optional_arg(&args, "--nonce");

    let proof = match nonce_hex {
        Some(hex) => {
            let nonce_bytes = match crypto::hex_decode(&hex) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("error: invalid nonce hex: {e}");
                    std::process::exit(1);
                }
            };
            crypto::compute_proof(secret.as_bytes(), &nonce_bytes)
        }
        None => crypto::compute_simple_proof(secret.as_bytes()),
    };
    println!("{proof}");
}

fn get_required_arg(args: &[String], name: &str) -> String {
    if let Some(val) = get_optional_arg(args, name) {
        val
    } else {
        eprintln!("error: missing required argument: {name}");
        eprintln!("usage: ssh-frontiere-proof --secret <raw-secret> [--nonce <hex-nonce>]");
        std::process::exit(1);
    }
}

fn get_optional_arg(args: &[String], name: &str) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == name {
            if let Some(val) = args.get(i + 1) {
                return Some(val.clone());
            }
        }
        if let Some(val) = arg.strip_prefix(&format!("{name}=")) {
            return Some(val.to_string());
        }
    }
    None
}
