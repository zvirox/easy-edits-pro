//! Per-user local control socket shared by the desktop app and MCP adapter.

use std::path::PathBuf;

/// Resolve the same private runtime socket path in the desktop app and MCP process.
pub fn mcp_socket_path() -> PathBuf {
    if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir).join("easy-edits-pro-mcp.sock");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".cache")
            .join("easy-edits-pro")
            .join("runtime")
            .join("mcp.sock");
    }
    let user = std::env::var("USER")
        .unwrap_or_else(|_| "default".to_owned())
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    std::env::temp_dir().join(format!("easy-edits-pro-mcp-{user}.sock"))
}
