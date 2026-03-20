use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;

use vmux_nix::{MicroVm, MicroVmConfig, MicroVmError};
use vmux_sandbox::{
    NetworkMode, Sandbox, SandboxConfig, SandboxError, SeccompPolicy,
};
use vmux_secrets::{SecretError, SecretResolver, SecretSpec, SecretValue};
use vmux_wezterm::{Layout, PaneRole, SessionState, WezTermError, WezTermSession};

/// Errors from the vmux launch orchestration.
#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("VM error: {0}")]
    Vm(#[from] MicroVmError),

    #[error("secret resolution error: {0}")]
    Secret(#[from] SecretError),

    #[error("WezTerm error: {0}")]
    WezTerm(#[from] WezTermError),

    #[error("sandbox error: {0}")]
    Sandbox(#[from] SandboxError),

    #[error("tool provisioning failed: {tool} — {reason}")]
    ToolProvision { tool: String, reason: String },

    #[error("configuration error: {0}")]
    Config(String),
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

    /// Wrap a shell command so it executes inside the environment.
    ///
    /// For bwrap this prepends `bwrap <args> --`, for microvm it prepends
    /// `ssh <host>`. The returned string is suitable for sending to a
    /// WezTerm pane.
    fn wrap_command(&self, command: &str) -> Result<String, LaunchError>;

    /// Optional teardown (e.g. stop ephemeral VM).
    fn cleanup(&mut self) -> impl std::future::Future<Output = Result<(), LaunchError>> + Send {
        async { Ok(()) }
    }

    /// Human-readable info printed when WezTerm is not used.
    fn status_info(&self, common: &CommonConfig, secret_count: usize) -> String;
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

    /// The command to launch this agent tool inside the VM.
    pub fn launch_command(&self) -> &str {
        match self {
            AgentTool::Claude => "claude",
            AgentTool::Aider => "aider",
            AgentTool::Codex => "codex",
            AgentTool::Custom(cmd) => cmd,
        }
    }
}

/// Editor choice for the VM.
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
    /// Agentic tool to provision.
    pub tool: Option<AgentTool>,
    /// Editor to use inside the environment.
    pub editor: Option<Editor>,
    /// WezTerm layout.
    pub layout: Layout,
    /// Whether to open WezTerm (false = just print info).
    pub use_wezterm: bool,
    /// Whether to destroy state on exit.
    pub ephemeral: bool,
}

// ---------------------------------------------------------------------------
// Bwrap backend
// ---------------------------------------------------------------------------

/// Bwrap-specific configuration.
#[derive(Debug)]
pub struct BwrapConfig {
    /// Network isolation mode.
    pub network: NetworkMode,
    /// Seccomp policy.
    pub seccomp: SeccompPolicy,
    /// Extra paths to deny read access.
    pub deny_paths: Vec<PathBuf>,
    /// Allowed proxy domains (when using Proxy network mode).
    pub proxy_domains: Vec<String>,
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

        if let NetworkMode::Proxy(ref proxy_cfg) = sandbox_cfg.network {
            for (k, v) in proxy_cfg.sandbox_env() {
                sandbox_cfg.env.insert(k, v);
            }
        }

        for (name, value) in secrets {
            sandbox_cfg.env.insert(name.clone(), value.expose().to_string());
        }

        for path in &self.config.deny_paths {
            sandbox_cfg.deny_paths.push(path.clone());
        }

        sandbox_cfg.seccomp = self.config.seccomp.clone();

        self.sandbox = Some(Sandbox::new(sandbox_cfg));

        tracing::info!(
            name = %common.name,
            network = ?self.config.network,
            "sandbox configured"
        );

        Ok(())
    }

    fn wrap_command(&self, command: &str) -> Result<String, LaunchError> {
        let sandbox = self.sandbox()?;
        let cmd = sandbox.command_string(&[command.into()])?;
        Ok(cmd)
    }

    fn status_info(&self, common: &CommonConfig, secret_count: usize) -> String {
        let tool = common
            .tool
            .as_ref()
            .map(|t| t.launch_command())
            .unwrap_or("bash");
        let sandbox = self.sandbox.as_ref().unwrap();
        let cmd = sandbox
            .command_string(&[tool.into()])
            .unwrap_or_else(|_| "<error>".into());
        format!(
            "Sandbox command:\n  {}\n\nWorkspace: {} -> /workspace\nNetwork: {:?}\nSecrets injected: {}",
            cmd,
            common.workspace.display(),
            self.config.network,
            secret_count,
        )
    }
}

// ---------------------------------------------------------------------------
// MicroVm backend
// ---------------------------------------------------------------------------

/// MicroVm-specific configuration.
#[derive(Debug)]
pub struct MicroVmBackendConfig {
    /// Path to the NixOS flake containing the microvm declaration.
    pub flake: PathBuf,
    /// Static IP of the VM.
    pub ip: std::net::Ipv4Addr,
    /// SSH readiness timeout.
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

    fn wrap_command(&self, command: &str) -> Result<String, LaunchError> {
        let vm = self.vm()?;
        Ok(format!(
            "ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -t {} 'source /dev/shm/vmux-secrets.env 2>/dev/null; cd {} && {}'",
            vm.ssh_host(),
            "/workspace",
            command,
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

    fn status_info(&self, common: &CommonConfig, secret_count: usize) -> String {
        let vm = self.vm.as_ref().unwrap();
        format!(
            "VM '{}' is running.\nSSH: ssh -o StrictHostKeyChecking=no {}\nSecrets injected: {}\nWorkspace: {} -> {}\n\nUse `vmux stop {}` to shut down the VM.",
            common.name,
            vm.ssh_host(),
            secret_count,
            common.workspace.display(),
            common.workspace_mount.display(),
            common.name,
        )
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

    pub fn wrap_command(&self, command: &str) -> Result<String, LaunchError> {
        match self {
            AnyBackend::Bwrap(b) => b.wrap_command(command),
            AnyBackend::MicroVm(b) => b.wrap_command(command),
        }
    }

    pub async fn cleanup(&mut self) -> Result<(), LaunchError> {
        match self {
            AnyBackend::Bwrap(b) => b.cleanup().await,
            AnyBackend::MicroVm(b) => b.cleanup().await,
        }
    }

    pub fn status_info(&self, common: &CommonConfig, secret_count: usize) -> String {
        match self {
            AnyBackend::Bwrap(b) => b.status_info(common, secret_count),
            AnyBackend::MicroVm(b) => b.status_info(common, secret_count),
        }
    }
}

// ---------------------------------------------------------------------------
// Unified launch orchestration
// ---------------------------------------------------------------------------

/// Send commands to WezTerm panes using the backend to wrap commands.
async fn send_pane_commands(
    session: &WezTermSession,
    common: &CommonConfig,
    backend: &AnyBackend,
) -> Result<(), LaunchError> {
    if session.panes.contains_key(&PaneRole::Editor) {
        let editor = common
            .editor
            .as_ref()
            .map(|e| e.command())
            .unwrap_or("nvim");
        let cmd = backend.wrap_command(editor)?;
        session.send_command(PaneRole::Editor, &cmd).await?;
    }
    if session.panes.contains_key(&PaneRole::Agent) {
        let tool = common
            .tool
            .as_ref()
            .map(|t| t.launch_command())
            .unwrap_or("bash");
        let cmd = backend.wrap_command(tool)?;
        session.send_command(PaneRole::Agent, &cmd).await?;
    }
    if session.panes.contains_key(&PaneRole::Logs) {
        let cmd = backend.wrap_command("journalctl -f").unwrap_or_else(|_| {
            "echo 'No log viewer available for this backend'".to_string()
        });
        session.send_command(PaneRole::Logs, &cmd).await?;
    }
    Ok(())
}

/// Run the full vmux launch sequence with any backend.
pub async fn launch_with_backend(
    common: CommonConfig,
    mut backend: AnyBackend,
) -> Result<(), LaunchError> {
    // Step 1: Resolve secrets.
    let resolver = SecretResolver::new();
    let secrets = resolver.resolve_all(&common.secrets).await?;
    tracing::info!(count = secrets.len(), "secrets resolved");

    // Step 2: Prepare the backend (start VM, build sandbox, etc.).
    backend.prepare(&common, &secrets).await?;

    // Step 3: Open WezTerm or print info.
    if common.use_wezterm {
        let session = WezTermSession::open(common.layout).await?;
        session.save_state(&common.name)?;

        send_pane_commands(&session, &common, &backend).await?;

        tracing::info!("WezTerm session opened, waiting for close...");
        session.wait_for_close().await?;
        SessionState::remove(&common.name);
    } else {
        println!("{}", backend.status_info(&common, secrets.len()));
    }

    // Step 4: Cleanup if ephemeral.
    if common.ephemeral {
        backend.cleanup().await?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Legacy flat config (preserved for CLI compatibility)
// ---------------------------------------------------------------------------

/// Full launch configuration (assembled from CLI args + vmux.toml).
///
/// This flat struct is kept for CLI ergonomics. [`launch`] converts it
/// into [`CommonConfig`] + the appropriate backend.
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
    pub layout: Layout,
    pub use_wezterm: bool,
    pub ephemeral: bool,
    pub ssh_timeout: Duration,
    pub backend: BackendKind,
    pub sandbox_network: NetworkMode,
    pub sandbox_seccomp: SeccompPolicy,
    pub sandbox_deny_paths: Vec<PathBuf>,
    pub sandbox_proxy_domains: Vec<String>,
}

impl LaunchConfig {
    /// Split into common config + backend.
    pub fn into_parts(self) -> (CommonConfig, AnyBackend) {
        let common = CommonConfig {
            name: self.name,
            workspace: self.workspace,
            workspace_mount: self.workspace_mount,
            secrets: self.secrets,
            tool: self.tool,
            editor: self.editor,
            layout: self.layout,
            use_wezterm: self.use_wezterm,
            ephemeral: self.ephemeral,
        };

        let backend = match self.backend {
            BackendKind::Bwrap => AnyBackend::Bwrap(BwrapProvider::new(BwrapConfig {
                network: self.sandbox_network,
                seccomp: self.sandbox_seccomp,
                deny_paths: self.sandbox_deny_paths,
                proxy_domains: self.sandbox_proxy_domains,
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

/// Run the full vmux launch sequence (legacy entry point).
pub async fn launch(config: LaunchConfig) -> Result<(), LaunchError> {
    let (common, backend) = config.into_parts();
    launch_with_backend(common, backend).await
}

/// Re-attach to a running VM's WezTerm session.
pub async fn attach(config: LaunchConfig) -> Result<(), LaunchError> {
    // Attach only works with MicroVm for now.
    let vm = MicroVm::from_config(MicroVmConfig {
        name: config.name.clone(),
        flake: config.flake.clone(),
        ip: config.ip,
        workspace_host: config.workspace.clone(),
        workspace_vm: config.workspace_mount.clone(),
    });

    let running = vm.is_running().await.unwrap_or(false);
    if !running {
        return Err(LaunchError::Config(format!(
            "VM '{}' is not running. Use `vmux launch` to start it.",
            config.name
        )));
    }

    let (common, backend) = config.into_parts();

    let session = match WezTermSession::restore(&common.name).await {
        Ok(session) => {
            tracing::info!(
                panes = session.panes.len(),
                "re-attached to existing WezTerm panes"
            );
            session
        }
        Err(_) => {
            tracing::info!("no live panes found, rebuilding WezTerm layout");
            let session =
                WezTermSession::rebuild_layout(&common.name, common.layout).await?;
            send_pane_commands(&session, &common, &backend).await?;
            session
        }
    };

    session.wait_for_close().await?;
    SessionState::remove(&common.name);

    Ok(())
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
