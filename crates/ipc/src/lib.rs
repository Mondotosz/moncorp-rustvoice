pub mod client;
pub mod protocol;
pub mod server;

pub use client::IpcError;

/// Returns the IPC socket path, resolved in priority order:
/// 1. `IPC_SOCKET_PATH` env var
/// 2. `$XDG_RUNTIME_DIR/rustvoice.sock`
/// 3. `~/.local/run/rustvoice.sock`
/// 4. `/tmp/rustvoice.sock`
pub fn default_socket_path() -> String {
    if let Ok(p) = std::env::var("IPC_SOCKET_PATH") {
        if !p.is_empty() {
            return p;
        }
    }
    format!("{}/rustvoice.sock", runtime_dir())
}

/// Returns the PID file path, resolved with the same XDG priority as the socket.
pub fn default_pid_path() -> String {
    format!("{}/rustvoice.pid", runtime_dir())
}

fn runtime_dir() -> String {
    if let Ok(d) = std::env::var("XDG_RUNTIME_DIR") {
        return d;
    }
    if let Ok(home) = std::env::var("HOME") {
        return format!("{home}/.local/run");
    }
    "/tmp".to_owned()
}
