use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;

use vmux_nix::{MicroVm, MicroVmConfig, MicroVmError};
use vmux_sandbox::{
    BindMount, NamespaceConfig, NetworkMode, Sandbox, SandboxConfig, SandboxError, SeccompPolicy,
};
use vmux_secrets::{SecretError, SecretResolver, SecretSpec, SecretValue};

/// Errors from the vmux launch orchestration.
#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("VM error: {0}")]
    Vm(#[from] MicroVmError),

    #[error("secret resolution error: {0}")]
    Secret(#[from] SecretError),

    #[error("sandbox error: {0}")]
    Sandbox(#[from] SandboxError),

    #[error("tool provisioning failed: {tool} — {reason}")]
    ToolProvision { tool: String, reason: String },

    #[error("configuration error: {0}")]
    Config(String),

    #[error("exec failed: {0}")]
    Exec(String),
}

// ---------------------------------------------------------------------------
// Backend provider trait
// ---------------------------------------------------------------------------

/// A backend that can run commands in an isolated environment.
///
/// Implementors handle environment setup, secret injection, and command
/// wrapping so the orchestration layer stays backend-agnostic.
pub trait BackendProvider: Send + Sync {
    /// Perform any one-time setup (start VM, build sandbox config, etc.).
    /// Called after secrets have been resolved.
    fn prepare(
        &mut self,
        common: &CommonConfig,
        secrets: &[(String, SecretValue)],
    ) -> impl std::future::Future<Output = Result<(), LaunchError>> + Send;

    /// Build the full argv to exec into the environment.
    ///
    /// Returns `(program, args)` suitable for `exec()`. For bwrap this is
    /// `("bwrap", [...bwrap args..., "--", "sh", "-c", command])`, for
    /// microvm it's `("ssh", ["-t", "root@ip", command])`.
    fn exec_argv(&self, command: &str) -> Result<(String, Vec<String>), LaunchError>;

    /// Optional teardown (e.g. stop ephemeral VM).
    fn cleanup(&mut self) -> impl std::future::Future<Output = Result<(), LaunchError>> + Send {
        async { Ok(()) }
    }
}

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/// Backend selector enum (used for config parsing / dispatch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendKind {
    /// Full microvm.nix VM (SSH-based, requires NixOS).
    MicroVm,
    /// Bubblewrap namespace sandbox (direct process, lightweight).
    Bwrap,
}

/// Agentic tool to provision inside the VM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentTool {
    Claude,
    Aider,
    Codex,
    Custom(String),
}

impl AgentTool {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "claude" => AgentTool::Claude,
            "aider" => AgentTool::Aider,
            "codex" => AgentTool::Codex,
            other => AgentTool::Custom(other.to_string()),
        }
    }

    /// The command to launch this agent tool.
    pub fn launch_command(&self) -> &str {
        match self {
            AgentTool::Claude => "claude",
            AgentTool::Aider => "aider",
            AgentTool::Codex => "codex",
            AgentTool::Custom(cmd) => cmd,
        }
    }
}

/// Editor choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Editor {
    Nvim,
    Helix,
    Emacs,
    Custom(String),
}

impl Editor {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "nvim" | "neovim" => Editor::Nvim,
            "hx" | "helix" => Editor::Helix,
            "emacs" => Editor::Emacs,
            other => Editor::Custom(other.to_string()),
        }
    }

    pub fn command(&self) -> &str {
        match self {
            Editor::Nvim => "nvim",
            Editor::Helix => "hx",
            Editor::Emacs => "emacs",
            Editor::Custom(cmd) => cmd,
        }
    }
}

/// Configuration shared across all backends.
#[derive(Debug)]
pub struct CommonConfig {
    /// Session / VM name.
    pub name: String,
    /// Host workspace directory.
    pub workspace: PathBuf,
    /// Mount point for workspace inside the environment.
    pub workspace_mount: PathBuf,
    /// Secrets to inject.
    pub secrets: Vec<SecretSpec>,
    /// Whether to destroy state on exit.
    pub ephemeral: bool,
}

// ---------------------------------------------------------------------------
// Bwrap backend
// ---------------------------------------------------------------------------

/// Bwrap-specific configuration.
#[derive(Debug)]
pub struct BwrapConfig {
    pub network: NetworkMode,
    pub seccomp: SeccompPolicy,
    pub namespaces: NamespaceConfig,
    pub deny_paths: Vec<PathBuf>,
    pub use_default_deny_paths: bool,
    pub proxy_domains: Vec<String>,
    pub ro_binds: Vec<BindMount>,
    pub rw_binds: Vec<BindMount>,
    pub tmpfs_mounts: Vec<PathBuf>,
    pub use_default_tmpfs: bool,
    pub extra_env: HashMap<String, String>,
}

/// Bwrap backend provider.
pub struct BwrapProvider {
    pub config: BwrapConfig,
    sandbox: Option<Sandbox>,
}

impl BwrapProvider {
    pub fn new(config: BwrapConfig) -> Self {
        BwrapProvider {
            config,
            sandbox: None,
        }
    }

    fn sandbox(&self) -> Result<&Sandbox, LaunchError> {
        self.sandbox
            .as_ref()
            .ok_or_else(|| LaunchError::Config("sandbox not prepared".into()))
    }
}

impl BackendProvider for BwrapProvider {
    async fn prepare(
        &mut self,
        common: &CommonConfig,
        secrets: &[(String, SecretValue)],
    ) -> Result<(), LaunchError> {
        let mut sandbox_cfg = SandboxConfig::for_workspace(&common.name, &common.workspace);

        sandbox_cfg.network = self.config.network.clone();
        sandbox_cfg.namespaces = self.config.namespaces.clone();
        sandbox_cfg.seccomp = self.config.seccomp.clone();
        sandbox_cfg.use_default_deny_paths = self.config.use_default_deny_paths;
        sandbox_cfg.use_default_tmpfs = self.config.use_default_tmpfs;

        // Extra bind mounts from config.
        sandbox_cfg.ro_binds.extend(self.config.ro_binds.iter().cloned());
        sandbox_cfg.rw_binds.extend(self.config.rw_binds.iter().cloned());

        // Extra tmpfs mounts.
        sandbox_cfg.tmpfs_mounts.extend(self.config.tmpfs_mounts.iter().cloned());

        // Extra deny paths.
        for path in &self.config.deny_paths {
            sandbox_cfg.deny_paths.push(path.clone());
        }

        // Extra environment variables from config.
        for (k, v) in &self.config.extra_env {
            sandbox_cfg.env.insert(k.clone(), v.clone());
        }

        // Proxy environment variables.
        if let NetworkMode::Proxy(ref proxy_cfg) = sandbox_cfg.network {
            for (k, v) in proxy_cfg.sandbox_env() {
                sandbox_cfg.env.insert(k, v);
            }
        }

        // Secrets as environment variables.
        for (name, value) in secrets {
            sandbox_cfg.env.insert(name.clone(), value.expose().to_string());
        }

        self.sandbox = Some(Sandbox::new(sandbox_cfg));

        tracing::info!(
            name = %common.name,
            network = ?self.config.network,
            "sandbox configured"
        );

        Ok(())
    }

    fn exec_argv(&self, command: &str) -> Result<(String, Vec<String>), LaunchError> {
        let sandbox = self.sandbox()?;
        let args = sandbox.build_args(&[command.into()]);
        let bwrap = sandbox.bwrap_path()?;
        Ok((bwrap, args))
    }
}

// ---------------------------------------------------------------------------
// MicroVm backend
// ---------------------------------------------------------------------------

/// MicroVm-specific configuration.
#[derive(Debug)]
pub struct MicroVmBackendConfig {
    pub flake: PathBuf,
    pub ip: std::net::Ipv4Addr,
    pub ssh_timeout: Duration,
}

/// MicroVm backend provider.
pub struct MicroVmProvider {
    pub config: MicroVmBackendConfig,
    vm: Option<MicroVm>,
}

impl MicroVmProvider {
    pub fn new(config: MicroVmBackendConfig) -> Self {
        MicroVmProvider { config, vm: None }
    }

    fn vm(&self) -> Result<&MicroVm, LaunchError> {
        self.vm
            .as_ref()
            .ok_or_else(|| LaunchError::Config("VM not prepared".into()))
    }
}

impl BackendProvider for MicroVmProvider {
    async fn prepare(
        &mut self,
        common: &CommonConfig,
        secrets: &[(String, SecretValue)],
    ) -> Result<(), LaunchError> {
        let mut vm = MicroVm::from_config(MicroVmConfig {
            name: common.name.clone(),
            flake: self.config.flake.clone(),
            ip: self.config.ip,
            workspace_host: common.workspace.clone(),
            workspace_vm: common.workspace_mount.clone(),
        });

        let already_running = vm.is_running().await.unwrap_or(false);
        if !already_running {
            vm.start().await?;
        }

        vm.wait_for_ssh(self.config.ssh_timeout).await?;

        inject_secrets_ssh(&vm, secrets).await?;

        self.vm = Some(vm);
        Ok(())
    }

    fn exec_argv(&self, command: &str) -> Result<(String, Vec<String>), LaunchError> {
        let vm = self.vm()?;
        let shell_cmd = format!(
            "source /dev/shm/vmux-secrets.env 2>/dev/null; cd /workspace && exec {}",
            command,
        );
        Ok((
            "ssh".into(),
            vec![
                "-o".into(), "StrictHostKeyChecking=no".into(),
                "-o".into(), "UserKnownHostsFile=/dev/null".into(),
                "-t".into(),
                vm.ssh_host(),
                shell_cmd,
            ],
        ))
    }

    async fn cleanup(&mut self) -> Result<(), LaunchError> {
        if let Some(ref vm) = self.vm {
            tracing::info!(name = %vm.config.name, "cleaning up VM (ephemeral)");
            let _ = std::process::Command::new("microvm")
                .args(["-s", &vm.config.name])
                .status();
        }
        Ok(())
    }
}

/// Inject secrets into the VM via SSH pipe to environment file in tmpfs.
async fn inject_secrets_ssh(
    vm: &MicroVm,
    secrets: &[(String, SecretValue)],
) -> Result<(), LaunchError> {
    if secrets.is_empty() {
        return Ok(());
    }

    let mut env_content = String::new();
    for (name, value) in secrets {
        env_content.push_str(&format!(
            "export {}='{}'\n",
            name,
            value.expose().replace('\'', "'\\''")
        ));
    }

    let ssh_cmd = format!(
        "cat > /dev/shm/vmux-secrets.env << 'VMUX_EOF'\n{}VMUX_EOF\nchmod 600 /dev/shm/vmux-secrets.env",
        env_content
    );

    vm.ssh_exec(&ssh_cmd)
        .await
        .map_err(LaunchError::Vm)?;

    tracing::info!(
        count = secrets.len(),
        "secrets injected into VM at /dev/shm/vmux-secrets.env"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Enum-based dispatch (avoids dyn-compatibility issues with async trait)
// ---------------------------------------------------------------------------

/// Type-erased backend that dispatches to the concrete provider.
pub enum AnyBackend {
    Bwrap(BwrapProvider),
    MicroVm(MicroVmProvider),
}

impl AnyBackend {
    pub async fn prepare(
        &mut self,
        common: &CommonConfig,
        secrets: &[(String, SecretValue)],
    ) -> Result<(), LaunchError> {
        match self {
            AnyBackend::Bwrap(b) => b.prepare(common, secrets).await,
            AnyBackend::MicroVm(b) => b.prepare(common, secrets).await,
        }
    }

    pub fn exec_argv(&self, command: &str) -> Result<(String, Vec<String>), LaunchError> {
        match self {
            AnyBackend::Bwrap(b) => b.exec_argv(command),
            AnyBackend::MicroVm(b) => b.exec_argv(command),
        }
    }

    pub async fn cleanup(&mut self) -> Result<(), LaunchError> {
        match self {
            AnyBackend::Bwrap(b) => b.cleanup().await,
            AnyBackend::MicroVm(b) => b.cleanup().await,
        }
    }
}

// ---------------------------------------------------------------------------
// Launch orchestration
// ---------------------------------------------------------------------------

/// Resolve secrets, prepare the backend, and return the argv to exec.
///
/// The caller is responsible for actually calling `exec()` with the
/// returned `(program, args)`. This separation keeps the core testable
/// and lets the CLI handle the Unix exec.
pub async fn prepare_exec(
    common: &CommonConfig,
    backend: &mut AnyBackend,
) -> Result<(), LaunchError> {
    // Step 1: Resolve secrets.
    let resolver = SecretResolver::new();
    let secrets = resolver.resolve_all(&common.secrets).await?;
    tracing::info!(count = secrets.len(), "secrets resolved");

    // Step 2: Prepare the backend.
    backend.prepare(common, &secrets).await?;

    Ok(())
}

/// Convenience: resolve secrets + prepare + build exec argv.
pub async fn resolve_and_build_argv(
    common: &CommonConfig,
    backend: &mut AnyBackend,
    command: &str,
) -> Result<(String, Vec<String>), LaunchError> {
    prepare_exec(common, backend).await?;
    backend.exec_argv(command)
}

// ---------------------------------------------------------------------------
// Legacy flat config (preserved for CLI ergonomics)
// ---------------------------------------------------------------------------

/// Full launch configuration (assembled from CLI args + vmux.toml).
///
/// This flat struct is kept for CLI ergonomics. Use [`into_parts`] to
/// split into [`CommonConfig`] + [`AnyBackend`].
#[derive(Debug)]
pub struct LaunchConfig {
    pub name: String,
    pub flake: PathBuf,
    pub ip: std::net::Ipv4Addr,
    pub workspace: PathBuf,
    pub workspace_mount: PathBuf,
    pub secrets: Vec<SecretSpec>,
    pub tool: Option<AgentTool>,
    pub editor: Option<Editor>,
    pub ephemeral: bool,
    pub ssh_timeout: Duration,
    pub backend: BackendKind,
    pub sandbox_network: NetworkMode,
    pub sandbox_seccomp: SeccompPolicy,
    pub sandbox_namespaces: NamespaceConfig,
    pub sandbox_deny_paths: Vec<PathBuf>,
    pub sandbox_use_default_deny_paths: bool,
    pub sandbox_proxy_domains: Vec<String>,
    pub sandbox_ro_binds: Vec<BindMount>,
    pub sandbox_rw_binds: Vec<BindMount>,
    pub sandbox_tmpfs_mounts: Vec<PathBuf>,
    pub sandbox_use_default_tmpfs: bool,
    pub sandbox_env: HashMap<String, String>,
}

impl LaunchConfig {
    /// The command to run inside the environment.
    pub fn command(&self) -> String {
        if let Some(ref tool) = self.tool {
            return tool.launch_command().to_string();
        }
        if let Some(ref editor) = self.editor {
            return editor.command().to_string();
        }
        std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string())
    }

    /// Split into common config + backend.
    pub fn into_parts(self) -> (CommonConfig, AnyBackend) {
        let common = CommonConfig {
            name: self.name,
            workspace: self.workspace,
            workspace_mount: self.workspace_mount,
            secrets: self.secrets,
            ephemeral: self.ephemeral,
        };

        let backend = match self.backend {
            BackendKind::Bwrap => AnyBackend::Bwrap(BwrapProvider::new(BwrapConfig {
                network: self.sandbox_network,
                seccomp: self.sandbox_seccomp,
                namespaces: self.sandbox_namespaces,
                deny_paths: self.sandbox_deny_paths,
                use_default_deny_paths: self.sandbox_use_default_deny_paths,
                proxy_domains: self.sandbox_proxy_domains,
                ro_binds: self.sandbox_ro_binds,
                rw_binds: self.sandbox_rw_binds,
                tmpfs_mounts: self.sandbox_tmpfs_mounts,
                use_default_tmpfs: self.sandbox_use_default_tmpfs,
                extra_env: self.sandbox_env,
            })),
            BackendKind::MicroVm => AnyBackend::MicroVm(MicroVmProvider::new(MicroVmBackendConfig {
                flake: self.flake,
                ip: self.ip,
                ssh_timeout: self.ssh_timeout,
            })),
        };

        (common, backend)
    }
}

// Re-export BackendKind as Backend for CLI compatibility.
pub use BackendKind as Backend;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_tool_parsing() {
        assert_eq!(AgentTool::from_str("claude"), AgentTool::Claude);
        assert_eq!(AgentTool::from_str("AIDER"), AgentTool::Aider);
        assert_eq!(AgentTool::from_str("codex"), AgentTool::Codex);
        assert!(matches!(AgentTool::from_str("custom-agent"), AgentTool::Custom(_)));
    }

    #[test]
    fn editor_parsing() {
        assert_eq!(Editor::from_str("nvim"), Editor::Nvim);
        assert_eq!(Editor::from_str("hx"), Editor::Helix);
        assert_eq!(Editor::from_str("helix"), Editor::Helix);
        assert_eq!(Editor::from_str("emacs"), Editor::Emacs);
    }
}
