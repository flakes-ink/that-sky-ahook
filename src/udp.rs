//! Lightweight UDP listener that feeds Lua scripts into the hook queue.
//!
//! Binds to a UDP port on all interfaces. Every incoming datagram is
//! treated as a Lua script and pushed onto the script queue for
//! execution on the next game frame.

use crate::log_error;
use crate::log_info;
use std::net::UdpSocket;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// UDP port the server listens on.
const LISTEN_PORT: u16 = 17222;

/// Maximum datagram size (Lua scripts longer than this will be truncated).
const MAX_DATAGRAM: usize = 65507; // practical UDP limit (64 KiB minus headers)

// ---------------------------------------------------------------------------
// Listener thread
// ---------------------------------------------------------------------------

/// Spawn a detached thread that binds to `0.0.0.0:LISTEN_PORT` and
/// forwards every received datagram to [`crate::hooks::queue_script`].
///
/// # Errors
///
/// Logs and returns immediately if the socket cannot be bound (e.g. port
/// already in use or process lacks network permission).  Individual
/// receive errors are logged without tearing down the listener.
pub fn start_udp_listener() {
    let bind_addr = format!("0.0.0.0:{LISTEN_PORT}");

    let socket = match UdpSocket::bind(&bind_addr) {
        Ok(s) => s,
        Err(e) => {
            log_error!("udp: failed to bind {} — {}", bind_addr, e);
            return;
        }
    };

    log_info!(
        "udp: listening on {} (max {} bytes/datagram)",
        bind_addr,
        MAX_DATAGRAM
    );

    std::thread::spawn(move || {
        let mut buf = vec![0u8; MAX_DATAGRAM];

        loop {
            match socket.recv_from(&mut buf) {
                Ok((n, src)) => {
                    if n == 0 {
                        continue;
                    }

                    let script = String::from_utf8_lossy(&buf[..n]);
                    log_info!("udp: recv {} bytes from {} — queuing script", n, src);
                    crate::hooks::queue_script(&script);
                }
                Err(e) => {
                    log_error!("udp: recv_from failed — {}", e);
                }
            }
        }
    });
}
