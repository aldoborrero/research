//! Bubblewrap process sandbox.
//!
//! Constructs and manages `bwrap` invocations that isolate agent processes
//! with filesystem, network, and PID namespace boundaries.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::proxy::ProxyConfig;
use crate::seccomp::SeccompPolicy;

/// Errors from sandbox operations.
#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("bwrap not found in PATH — install bubblewrap")]
    BwrapNotFound,

    #[error("sandbox failed to start: {0}")]
    StartFailed(String),

    #[error("sandbox process exited with code {0}")]
    ExitCode(i32),

    #[error("proxy error: {0}")]
    Proxy(String),

    #[error("seccomp error: {0}")]
    Seccomp(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Sandbox lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxState {
    /// Not yet started.
    Idle,
    /// bwrap process is running.
    Running,
    /// Process exited.
    Exited,
}

/// Network isolation mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkMode {
    /// Full host network access (no isolation).
    Host,
    /// No network at all (`--unshare-net`, loopback only).
    None,
    /// Network isolated but API access via proxy daemon.
    Proxy(ProxyConfig),
}

/// A path to bind-mount into the sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindMount {
    /// Source path on the host.
    pub src: PathBuf,
    /// Destination path inside the sandbox.
    pub dst: PathBuf,
    /// Whether the mount is read-only.
    pub readonly: bool,
}

/// Configuration for a sandbox instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Human-readable name for this sandbox.
    pub name: String,

    /// Working directory inside the sandbox.
    pub workdir: PathBuf,

    /// Network isolation mode.
    pub network: NetworkMode,

    /// Extra read-only bind mounts (beyond platform defaults).
    pub ro_binds: Vec<BindMount>,

    /// Read-write bind mounts (workspace, tmp).
    pub rw_binds: Vec<BindMount>,

    /// Paths to hide entirely (mounted as tmpfs or /dev/null).
    pub deny_paths: Vec<PathBuf>,

    /// Environment variables to inject.
    pub env: HashMap<String, String>,

    /// Seccomp policy to apply.
    pub seccomp: SeccompPolicy,
}

/// Platform-default read-only bind mounts.
/// Modeled after Codex's `LINUX_PLATFORM_DEFAULT_READ_ROOTS`.
const PLATFORM_READ_ROOTS: &[&str] = &[
    "/bin",
    "/sbin",
    "/usr",
    "/etc",
    "/lib",
    "/lib64",
    "/nix/store",
    "/run/current-system/sw",
];

/// Paths that are always hidden from sandboxed agents.
const DEFAULT_DENY_PATHS: &[&str] = &[
    ".ssh",
    ".gnupg",
    ".aws",
    ".azure",
    ".gcloud",
    ".config/gcloud",
    ".docker",
    ".kube",
    ".npmrc",
    ".pypirc",
    ".netrc",
    ".git-credentials",
];

impl SandboxConfig {
    /// Create a config with sensible defaults for sandboxing an agent
    /// in the given workspace directory.
    pub fn for_workspace(name: impl Into<String>, workspace: impl Into<PathBuf>) -> Self {
        let workspace = workspace.into();

        SandboxConfig {
            name: name.into(),
            workdir: PathBuf::from("/workspace"),
            network: NetworkMode::None,
            ro_binds: Vec::new(),
            rw_binds: vec![
                BindMount {
                    src: workspace,
                    dst: PathBuf::from("/workspace"),
                    readonly: false,
                },
            ],
            deny_paths: DEFAULT_DENY_PATHS
                .iter()
                .map(PathBuf::from)
                .collect(),
            env: HashMap::new(),
            seccomp: SeccompPolicy::Default,
        }
    }
}

/// A running or idle sandbox instance.
pub struct Sandbox {
    pub config: SandboxConfig,
    pub state: SandboxState,
    child: Option<tokio::process::Child>,
}

impl Sandbox {
    /// Create a new sandbox handle from config.
    pub fn new(config: SandboxConfig) -> Self {
        Sandbox {
            config,
            state: SandboxState::Idle,
            child: None,
        }
    }

    /// Build the full bwrap argument list from config.
    pub fn build_args(&self, command: &[String]) -> Vec<String> {
        let mut args = Vec::new();

        // -- Session & lifecycle --
        args.extend(["--new-session".into(), "--die-with-parent".into()]);

        // -- Namespace isolation --
        args.push("--unshare-pid".into());
        args.push("--unshare-ipc".into());

        match &self.config.network {
            NetworkMode::Host => {}
            NetworkMode::None | NetworkMode::Proxy(_) => {
                args.push("--unshare-net".into());
            }
        }

        // -- Platform read-only roots --
        for root in PLATFORM_READ_ROOTS {
            let path = Path::new(root);
            if path.exists() {
                args.extend([
                    "--ro-bind".into(),
                    root.to_string(),
                    root.to_string(),
                ]);
            }
        }

        // -- /dev and /proc --
        args.extend(["--dev".into(), "/dev".into()]);
        args.extend(["--proc".into(), "/proc".into()]);

        // -- tmpfs for /tmp and /home --
        args.extend(["--tmpfs".into(), "/tmp".into()]);
        args.extend(["--tmpfs".into(), "/home".into()]);

        // -- TLS certs (needed for API calls) --
        for cert_path in &["/etc/ssl", "/etc/pki", "/etc/ca-certificates"] {
            if Path::new(cert_path).exists() {
                args.extend([
                    "--ro-bind".into(),
                    cert_path.to_string(),
                    cert_path.to_string(),
                ]);
            }
        }

        // -- DNS resolution --
        if Path::new("/etc/resolv.conf").exists() {
            args.extend([
                "--ro-bind".into(),
                "/etc/resolv.conf".into(),
                "/etc/resolv.conf".into(),
            ]);
        }

        // -- User extra read-only binds --
        for bind in &self.config.ro_binds {
            args.extend([
                "--ro-bind".into(),
                bind.src.display().to_string(),
                bind.dst.display().to_string(),
            ]);
        }

        // -- Read-write binds (workspace) --
        for bind in &self.config.rw_binds {
            if bind.readonly {
                args.extend([
                    "--ro-bind".into(),
                    bind.src.display().to_string(),
                    bind.dst.display().to_string(),
                ]);
            } else {
                args.extend([
                    "--bind".into(),
                    bind.src.display().to_string(),
                    bind.dst.display().to_string(),
                ]);
            }
        }

        // -- Deny paths (hide sensitive dirs from home) --
        for deny in &self.config.deny_paths {
            let home_path = dirs::home_dir()
                .map(|h| h.join(deny))
                .unwrap_or_else(|| PathBuf::from("/nonexistent"));

            if home_path.is_dir() {
                args.extend(["--tmpfs".into(), home_path.display().to_string()]);
            } else if home_path.exists() {
                args.extend([
                    "--ro-bind".into(),
                    "/dev/null".into(),
                    home_path.display().to_string(),
                ]);
            }
        }

        // -- Proxy socket mounts (if proxy mode) --
        if let NetworkMode::Proxy(ref proxy_cfg) = self.config.network {
            if let Some(ref http_sock) = proxy_cfg.http_socket_path {
                args.extend([
                    "--bind".into(),
                    http_sock.display().to_string(),
                    http_sock.display().to_string(),
                ]);
            }
            if let Some(ref socks_sock) = proxy_cfg.socks_socket_path {
                args.extend([
                    "--bind".into(),
                    socks_sock.display().to_string(),
                    socks_sock.display().to_string(),
                ]);
            }
        }

        // -- Environment variables --
        for (key, value) in &self.config.env {
            args.extend(["--setenv".into(), key.clone(), value.clone()]);
        }

        // -- Working directory --
        // bwrap doesn't have --chdir in all versions, so we use sh -c
        // to cd into the workspace before running the command.

        // -- Separator and command --
        args.push("--".into());

        if command.is_empty() {
            args.push("sh".into());
        } else {
            // Wrap in sh -c to handle cd + command
            args.push("sh".into());
            args.push("-c".into());
            let cmd_str = format!(
                "cd {} 2>/dev/null; exec {}",
                self.config.workdir.display(),
                shell_join(command),
            );
            args.push(cmd_str);
        }

        args
    }

    /// Launch a command inside the sandbox. Returns immediately.
    pub async fn spawn(&mut self, command: &[String]) -> Result<(), SandboxError> {
        // Verify bwrap exists.
        let bwrap = which_bwrap()?;

        let args = self.build_args(command);

        tracing::info!(
            name = %self.config.name,
            cmd = %command.join(" "),
            "spawning sandbox"
        );
        tracing::debug!(bwrap_args = ?args, "full bwrap invocation");

        let child = tokio::process::Command::new(&bwrap)
            .args(&args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| SandboxError::StartFailed(e.to_string()))?;

        self.child = Some(child);
        self.state = SandboxState::Running;

        Ok(())
    }

    /// Wait for the sandboxed process to exit.
    pub async fn wait(&mut self) -> Result<i32, SandboxError> {
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| SandboxError::StartFailed("sandbox not started".into()))?;

        let status = child.wait().await?;
        self.state = SandboxState::Exited;

        Ok(status.code().unwrap_or(-1))
    }

    /// Spawn and wait in one call.
    pub async fn run(&mut self, command: &[String]) -> Result<i32, SandboxError> {
        self.spawn(command).await?;
        self.wait().await
    }

    /// Build a command string suitable for a WezTerm pane.
    ///
    /// Returns the full `bwrap ...` command as a single string that can
    /// be sent to a terminal pane.
    pub fn command_string(&self, command: &[String]) -> Result<String, SandboxError> {
        let bwrap = which_bwrap()?;
        let args = self.build_args(command);
        Ok(format!("{} {}", bwrap, shell_join(&args)))
    }
}

/// Find the bwrap binary.
fn which_bwrap() -> Result<String, SandboxError> {
    // Check common locations.
    for path in &[
        "/usr/bin/bwrap",
        "/usr/local/bin/bwrap",
        "/run/current-system/sw/bin/bwrap",
    ] {
        if Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    // Fall back to PATH lookup.
    let output = std::process::Command::new("which")
        .arg("bwrap")
        .output()
        .map_err(|_| SandboxError::BwrapNotFound)?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }

    Err(SandboxError::BwrapNotFound)
}

/// Shell-escape and join a command slice.
fn shell_join(parts: &[String]) -> String {
    parts
        .iter()
        .map(|s| {
            if s.contains(' ')
                || s.contains('\'')
                || s.contains('"')
                || s.contains('$')
                || s.contains('\\')
                || s.contains(';')
                || s.contains('&')
                || s.contains('|')
                || s.contains('>')
                || s.contains('<')
                || s.contains('(')
                || s.contains(')')
                || s.is_empty()
            {
                format!("'{}'", s.replace('\'', "'\\''"))
            } else {
                s.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_workspace_bind() {
        let cfg = SandboxConfig::for_workspace("test", "/home/user/project");
        assert_eq!(cfg.rw_binds.len(), 1);
        assert_eq!(cfg.rw_binds[0].dst, PathBuf::from("/workspace"));
        assert!(!cfg.rw_binds[0].readonly);
    }

    #[test]
    fn build_args_contains_unshare_pid() {
        let cfg = SandboxConfig::for_workspace("test", "/tmp/ws");
        let sb = Sandbox::new(cfg);
        let args = sb.build_args(&["bash".into()]);
        assert!(args.contains(&"--unshare-pid".to_string()));
        assert!(args.contains(&"--new-session".to_string()));
        assert!(args.contains(&"--die-with-parent".to_string()));
    }

    #[test]
    fn network_none_unshares_net() {
        let cfg = SandboxConfig::for_workspace("test", "/tmp/ws");
        let sb = Sandbox::new(cfg);
        let args = sb.build_args(&["bash".into()]);
        assert!(args.contains(&"--unshare-net".to_string()));
    }

    #[test]
    fn network_host_does_not_unshare() {
        let mut cfg = SandboxConfig::for_workspace("test", "/tmp/ws");
        cfg.network = NetworkMode::Host;
        let sb = Sandbox::new(cfg);
        let args = sb.build_args(&["bash".into()]);
        assert!(!args.contains(&"--unshare-net".to_string()));
    }

    #[test]
    fn env_vars_injected() {
        let mut cfg = SandboxConfig::for_workspace("test", "/tmp/ws");
        cfg.env.insert("FOO".into(), "bar".into());
        let sb = Sandbox::new(cfg);
        let args = sb.build_args(&["bash".into()]);
        let idx = args.iter().position(|a| a == "--setenv").unwrap();
        assert_eq!(args[idx + 1], "FOO");
        assert_eq!(args[idx + 2], "bar");
    }

    #[test]
    fn shell_join_escapes_spaces() {
        let parts = vec!["hello world".to_string(), "simple".to_string()];
        assert_eq!(shell_join(&parts), "'hello world' simple");
    }
}
