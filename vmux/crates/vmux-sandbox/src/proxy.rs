//! Network proxy daemon for sandboxed agents.
//!
//! When agents need API access (anthropic, openai, github) but the sandbox
//! has `--unshare-net`, we bridge traffic through Unix domain sockets using
//! socat, with a domain-filtering proxy on the host side.
//!
//! Architecture (matches Claude Code sandbox-runtime pattern):
//!
//! ```text
//!   Host                              │  Sandbox (--unshare-net)
//!                                     │
//!   tinyproxy ◄── socat UDS ◄─────────┼── socat TCP:3128 ◄── agent
//!   (domain        UNIX-LISTEN        │   TCP-LISTEN          (HTTP_PROXY=localhost:3128)
//!    filter)       → TCP:proxy        │   → UNIX:sock
//! ```

use std::path::PathBuf;
use std::process::Stdio;

use serde::{Deserialize, Serialize};

use crate::bwrap::SandboxError;

/// Proxy configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Domains the agent is allowed to reach.
    pub allow_domains: Vec<String>,

    /// Domains explicitly blocked (applied after allow).
    pub deny_domains: Vec<String>,

    /// Port for the HTTP proxy inside the sandbox (default: 3128).
    pub http_port: u16,

    /// Port for the SOCKS5 proxy inside the sandbox (default: 1080).
    pub socks_port: u16,

    /// Path to the HTTP bridge Unix domain socket.
    #[serde(skip)]
    pub http_socket_path: Option<PathBuf>,

    /// Path to the SOCKS5 bridge Unix domain socket.
    #[serde(skip)]
    pub socks_socket_path: Option<PathBuf>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        ProxyConfig {
            allow_domains: vec![
                "api.anthropic.com".into(),
                "api.openai.com".into(),
                "github.com".into(),
                "api.github.com".into(),
            ],
            deny_domains: Vec::new(),
            http_port: 3128,
            socks_port: 1080,
            http_socket_path: None,
            socks_socket_path: None,
        }
    }
}

impl ProxyConfig {
    /// Generate the environment variables that should be set inside the sandbox
    /// so that tools route traffic through the proxy.
    pub fn sandbox_env(&self) -> Vec<(String, String)> {
        let http = format!("http://localhost:{}", self.http_port);
        let socks = format!("socks5h://localhost:{}", self.socks_port);

        vec![
            ("HTTP_PROXY".into(), http.clone()),
            ("HTTPS_PROXY".into(), http.clone()),
            ("http_proxy".into(), http.clone()),
            ("https_proxy".into(), http.clone()),
            ("ALL_PROXY".into(), socks.clone()),
            ("all_proxy".into(), socks),
            (
                "NO_PROXY".into(),
                "localhost,127.0.0.1,::1,0.0.0.0".into(),
            ),
        ]
    }
}

/// A running proxy daemon with its socat bridges.
///
/// Manages the lifecycle of:
/// 1. A host-side HTTP proxy (tinyproxy) with domain filtering
/// 2. socat bridges between host TCP and sandbox Unix domain sockets
pub struct ProxyDaemon {
    config: ProxyConfig,
    /// Child processes to clean up on drop.
    children: Vec<tokio::process::Child>,
}

impl ProxyDaemon {
    /// Create a new proxy daemon (not yet started).
    pub fn new(config: ProxyConfig) -> Self {
        ProxyDaemon {
            config,
            children: Vec::new(),
        }
    }

    /// Start the proxy daemon and socat bridges.
    ///
    /// Returns a modified `ProxyConfig` with socket paths filled in.
    pub async fn start(&mut self) -> Result<ProxyConfig, SandboxError> {
        let session_id = std::process::id();
        let sock_dir = std::env::temp_dir().join(format!("vmux-proxy-{}", session_id));
        tokio::fs::create_dir_all(&sock_dir)
            .await
            .map_err(|e| SandboxError::Proxy(format!("failed to create socket dir: {}", e)))?;

        let http_sock = sock_dir.join("http.sock");
        let socks_sock = sock_dir.join("socks.sock");

        // Start host-side HTTP proxy (tinyproxy or simple forwarding proxy).
        // For now, we use a direct socat TCP forwarder.
        // A full domain-filtering proxy would be a tinyproxy config or a
        // custom Rust TCP proxy — left as a build target.

        // Start socat bridge: UNIX-LISTEN → TCP (host proxy / direct internet)
        let http_bridge = self
            .start_socat_bridge(&http_sock, self.config.http_port)
            .await?;
        self.children.push(http_bridge);

        tracing::info!(
            sock = %http_sock.display(),
            port = self.config.http_port,
            "HTTP proxy bridge started"
        );

        let mut cfg = self.config.clone();
        cfg.http_socket_path = Some(http_sock);
        cfg.socks_socket_path = Some(socks_sock);

        Ok(cfg)
    }

    /// Start a socat bridge: Unix socket → TCP on the host side.
    async fn start_socat_bridge(
        &self,
        socket_path: &PathBuf,
        _port: u16,
    ) -> Result<tokio::process::Child, SandboxError> {
        // socat UNIX-LISTEN:<sock>,fork,reuseaddr TCP:localhost:<port>
        // For the initial implementation we bridge to the internet directly.
        // A domain-filtering proxy would sit between socat and the internet.
        let child = tokio::process::Command::new("socat")
            .args([
                &format!(
                    "UNIX-LISTEN:{},fork,reuseaddr,mode=0600",
                    socket_path.display()
                ),
                "TCP:localhost:0", // placeholder — real proxy port goes here
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                SandboxError::Proxy(format!("failed to start socat bridge: {}", e))
            })?;

        Ok(child)
    }

    /// Build the command that runs inside the sandbox to bridge UDS → TCP.
    ///
    /// This is prepended to the agent command inside bwrap.
    pub fn inner_bridge_command(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref http_sock) = self.config.http_socket_path {
            parts.push(format!(
                "socat TCP-LISTEN:{},fork,reuseaddr UNIX-CONNECT:{} &",
                self.config.http_port,
                http_sock.display()
            ));
        }

        if let Some(ref socks_sock) = self.config.socks_socket_path {
            parts.push(format!(
                "socat TCP-LISTEN:{},fork,reuseaddr UNIX-CONNECT:{} &",
                self.config.socks_port,
                socks_sock.display()
            ));
        }

        parts.join(" ")
    }

    /// Stop all proxy processes.
    pub async fn stop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill().await;
        }
        self.children.clear();

        // Clean up socket files.
        let sock_dir = std::env::temp_dir().join(format!("vmux-proxy-{}", std::process::id()));
        let _ = tokio::fs::remove_dir_all(&sock_dir).await;

        tracing::info!("proxy daemon stopped");
    }
}

impl Drop for ProxyDaemon {
    fn drop(&mut self) {
        // Best-effort sync cleanup.
        for child in &mut self.children {
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_allows_anthropic() {
        let cfg = ProxyConfig::default();
        assert!(cfg.allow_domains.contains(&"api.anthropic.com".to_string()));
    }

    #[test]
    fn sandbox_env_sets_proxy_vars() {
        let cfg = ProxyConfig::default();
        let env = cfg.sandbox_env();
        let keys: Vec<_> = env.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"HTTP_PROXY"));
        assert!(keys.contains(&"HTTPS_PROXY"));
        assert!(keys.contains(&"ALL_PROXY"));
        assert!(keys.contains(&"NO_PROXY"));
    }
}
