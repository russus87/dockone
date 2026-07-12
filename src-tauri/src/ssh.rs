//! SSH port-forwarding: connect to a remote Docker daemon over SSH without
//! exposing its API. We shell out to the system `ssh` client and forward a
//! local TCP port to the remote unix socket; bollard then speaks plain HTTP
//! to `127.0.0.1:<port>`. Authentication uses the user's SSH agent / keys /
//! `~/.ssh/config` (no interactive password prompt).

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Parse `ssh://[user@]host[:port]` and open a tunnel to the remote Docker
/// socket. Returns the spawned child plus the local `tcp://…` endpoint.
pub fn open(endpoint: &str) -> Result<(Child, String), String> {
    let rest = endpoint
        .trim()
        .strip_prefix("ssh://")
        .ok_or("endpoint non SSH")?;
    // strip an optional trailing path (we always target the default socket)
    let rest = rest.split('/').next().unwrap_or(rest);

    let (userhost, port) = match rest.rsplit_once(':') {
        Some((uh, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => {
            (uh, p.to_string())
        }
        _ => (rest, "22".to_string()),
    };
    if userhost.is_empty() {
        return Err("host SSH mancante".into());
    }

    let local_port = free_port()?;
    let forward = format!("127.0.0.1:{local_port}:/var/run/docker.sock");

    let child = Command::new("ssh")
        .args([
            "-N",
            "-o",
            "ExitOnForwardFailure=yes",
            "-o",
            "ConnectTimeout=10",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "BatchMode=yes",
            "-L",
            &forward,
            "-p",
            &port,
            userhost,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("impossibile avviare ssh: {e}"))?;

    // Wait until the forwarded port accepts connections.
    let addr = format!("127.0.0.1:{local_port}");
    let deadline = Instant::now() + Duration::from_secs(12);
    let mut child = child;
    loop {
        if TcpStream::connect(&addr).is_ok() {
            return Ok((child, format!("tcp://{addr}")));
        }
        // ssh exited early → auth/host failure
        if let Ok(Some(_)) = child.try_wait() {
            return Err(
                "connessione SSH fallita (verifica host, porta e chiavi SSH — serve accesso senza password)"
                    .into(),
            );
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            return Err("tunnel SSH non stabilito entro il timeout".into());
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

fn free_port() -> Result<u16, String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    Ok(port) // listener dropped here → port is free for ssh to bind
}
