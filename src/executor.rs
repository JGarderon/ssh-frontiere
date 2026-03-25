use crate::protocol::{write_stderr_line, write_stdout_line};
use std::io::{BufRead, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::Command;
use std::sync::mpsc;

// Convention: `let _ =` on write/join/signal calls is intentional (best-effort I/O/cleanup).
// - Streaming writes (write_stdout_line, write_stderr_line): client may have disconnected
// - Thread joins: a panicked reader thread is non-recoverable
// - Signal sends (kill): best-effort process cleanup

const SAFE_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// Resultat de l'execution d'une commande streamee (ADR 0011)
#[derive(Debug)]
pub(crate) enum ExecuteResult {
    /// Processus termine normalement (code de sortie)
    Exited(i32),
    /// Processus tue par signal (numero de signal)
    Signaled(i32),
    /// Timeout depasse
    Timeout,
    /// Erreur au spawn
    SpawnError(String),
    /// ADR 0012 D3 — stdin closed before body fully written
    StdinError,
}

/// Ligne streamee depuis un processus enfant
enum StreamLine {
    Stdout(String),
    Stderr(String),
}

/// Execute une commande avec streaming stdout/stderr ligne par ligne (ADR 0011)
/// If `body` is Some, it is written to the child process's stdin (ADR 0012 D3).
#[must_use = "execution result must be checked"]
pub(crate) fn execute_command(
    cmd_parts: &[&str],
    timeout_secs: u64,
    session_id: &str,
    writer: &mut impl Write,
    max_stream_bytes: usize,
    body: Option<&str>,
) -> ExecuteResult {
    if cmd_parts.is_empty() {
        return ExecuteResult::SpawnError("empty command".to_string());
    }

    // PANIC-SAFE: cmd_parts.is_empty() checked above with early return
    let stdin_mode = if body.is_some() {
        std::process::Stdio::piped()
    } else {
        std::process::Stdio::null()
    };
    let mut child = match Command::new(cmd_parts[0])
        .args(&cmd_parts[1..])
        .env_clear()
        .env("PATH", SAFE_PATH)
        .env("SSH_FRONTIERE_SESSION", session_id)
        .process_group(0)
        .stdin(stdin_mode)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return ExecuteResult::SpawnError(format!("failed to spawn: {e}")),
    };

    // ADR 0012 D3 — write body to child stdin in a separate thread (plan P3)
    let stdin_thread = spawn_stdin_thread(&mut child, body);

    // Channel partage pour les deux threads de lecture
    let (tx, rx) = mpsc::channel::<StreamLine>();

    // Thread stdout
    let stdout_thread = spawn_reader_thread(child.stdout.take(), tx.clone(), StreamLine::Stdout);

    // Thread stderr
    let stderr_thread = spawn_reader_thread(child.stderr.take(), tx, StreamLine::Stderr);

    // Boucle principale : consommer les lignes + surveiller le timeout
    let threads = IoThreads {
        stdin: stdin_thread,
        stdout: Some(stdout_thread),
        stderr: Some(stderr_thread),
        rx,
    };
    run_event_loop(
        &mut child,
        threads,
        writer,
        timeout_secs,
        max_stream_bytes,
        body.is_some(),
    )
}

/// Groups I/O threads and the shared channel for the event loop
struct IoThreads {
    stdin: Option<std::thread::JoinHandle<bool>>,
    stdout: Option<std::thread::JoinHandle<()>>,
    stderr: Option<std::thread::JoinHandle<()>>,
    rx: mpsc::Receiver<StreamLine>,
}

impl IoThreads {
    /// Join all remaining threads (best-effort)
    fn join_all(&mut self) {
        if let Some(h) = self.stdin.take() {
            let _ = h.join();
        }
        if let Some(h) = self.stdout.take() {
            let _ = h.join();
        }
        if let Some(h) = self.stderr.take() {
            let _ = h.join();
        }
    }
}

/// Spawn a thread to write body to child stdin (ADR 0012 D3)
fn spawn_stdin_thread(
    child: &mut std::process::Child,
    body: Option<&str>,
) -> Option<std::thread::JoinHandle<bool>> {
    let body_str = body?;
    let body_owned = body_str.to_string();
    let child_stdin = child.stdin.take();
    Some(std::thread::spawn(move || -> bool {
        if let Some(mut stdin) = child_stdin {
            use std::io::Write as _;
            if stdin.write_all(body_owned.as_bytes()).is_err() {
                return false;
            }
            // Drop closes stdin, signaling EOF to the child
        }
        true
    }))
}

/// Spawn a reader thread for stdout or stderr
fn spawn_reader_thread<F>(
    pipe: Option<impl std::io::Read + Send + 'static>,
    tx: mpsc::Sender<StreamLine>,
    wrapper: F,
) -> std::thread::JoinHandle<()>
where
    F: Fn(String) -> StreamLine + Send + 'static,
{
    std::thread::spawn(move || {
        if let Some(pipe) = pipe {
            let reader = std::io::BufReader::new(pipe);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if tx.send(wrapper(l)).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    })
}

/// Main event loop: consume lines + monitor timeout
fn run_event_loop(
    child: &mut std::process::Child,
    mut io: IoThreads,
    writer: &mut impl Write,
    timeout_secs: u64,
    max_stream_bytes: usize,
    has_body: bool,
) -> ExecuteResult {
    let timeout = std::time::Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();
    let mut streamed_bytes: usize = 0;
    let mut truncated = false;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Processus termine — join les threads puis drain le channel
                let stdin_ok = io.stdin.take().is_none_or(|h| h.join().unwrap_or(false));
                io.join_all();
                drain_channel(
                    &io.rx,
                    writer,
                    max_stream_bytes,
                    &mut streamed_bytes,
                    &mut truncated,
                );

                // ADR 0012 D3 — stdin closed = error 133
                if has_body && !stdin_ok {
                    return ExecuteResult::StdinError;
                }

                return if let Some(code) = status.code() {
                    ExecuteResult::Exited(code)
                } else {
                    let signal = status.signal().unwrap_or(9);
                    ExecuteResult::Signaled(signal)
                };
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    kill_process(child);
                    io.join_all();
                    drain_channel(
                        &io.rx,
                        writer,
                        max_stream_bytes,
                        &mut streamed_bytes,
                        &mut truncated,
                    );
                    let _ = write_stderr_line(writer, "ssh-frontiere: command timed out");
                    return ExecuteResult::Timeout;
                }
            }
            Err(_) => {
                kill_process(child);
                io.join_all();
                return ExecuteResult::SpawnError("wait error".to_string());
            }
        }

        // Consommer les lignes disponibles (non bloquant)
        while let Ok(line) = io.rx.try_recv() {
            write_stream_line(
                writer,
                &line,
                max_stream_bytes,
                &mut streamed_bytes,
                &mut truncated,
            );
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// Ecrit une ligne streamee sur le writer, avec gestion du volume max
fn write_stream_line(
    writer: &mut impl Write,
    line: &StreamLine,
    max_bytes: usize,
    streamed_bytes: &mut usize,
    truncated: &mut bool,
) {
    if *truncated {
        return;
    }

    let content = match line {
        StreamLine::Stdout(s) | StreamLine::Stderr(s) => s,
    };

    *streamed_bytes += content.len() + 1; // +1 for \n
    if *streamed_bytes > max_bytes {
        *truncated = true;
        let _ = write_stderr_line(
            writer,
            &format!(
                "ssh-frontiere: output truncated (max {}MB exceeded)",
                max_bytes / 1_048_576
            ),
        );
        return;
    }

    match line {
        StreamLine::Stdout(_) => {
            let _ = write_stdout_line(writer, content);
        }
        StreamLine::Stderr(_) => {
            let _ = write_stderr_line(writer, content);
        }
    }
}

/// Drain le channel apres join des threads
fn drain_channel(
    rx: &mpsc::Receiver<StreamLine>,
    writer: &mut impl Write,
    max_bytes: usize,
    streamed_bytes: &mut usize,
    truncated: &mut bool,
) {
    while let Ok(line) = rx.try_recv() {
        write_stream_line(writer, &line, max_bytes, streamed_bytes, truncated);
    }
}

const GRACEFUL_SHUTDOWN_SECS: u64 = 5;

/// Envoie un signal a un process group via /bin/kill
fn send_signal_to_group(pid: u32, signal: &str) {
    let _ = Command::new("/bin/kill")
        .arg(signal)
        .arg("--")
        .arg(format!("-{pid}"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Arret gracieux : SIGTERM → delai → SIGKILL (process group)
fn kill_process(child: &mut std::process::Child) {
    let pid = child.id();

    // Etape 1 : SIGTERM au process group
    send_signal_to_group(pid, "-TERM");

    // Etape 2 : attendre l'arret gracieux (max GRACEFUL_SHUTDOWN_SECS)
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(GRACEFUL_SHUTDOWN_SECS);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return, // Processus termine proprement
            Ok(None) if std::time::Instant::now() >= deadline => break,
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(_) => break,
        }
    }

    // Etape 3 : SIGKILL si toujours vivant
    send_signal_to_group(pid, "-KILL");
    // Reap zombie — result intentionally discarded after SIGKILL
    let _ = child.wait();
}
