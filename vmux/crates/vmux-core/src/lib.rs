use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;

use vmux_nix::{MicroVm, MicroVmConfig, MicroVmError};
use vmux_sandbox::{
    NetworkMode, Sandbox, SandboxConfig, SandboxError, SeccompPolicy,
};
use vmux_secrets::{SecretError, SecretResolver, SecretSpec};
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

/// Backend for running agents: microvm (legacy) or bwrap sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Backend {
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

/// Full launch configuration (assembled from CLI args + vmux.toml).
#[derive(Debug)]
pub struct LaunchConfig {
    /// VM name.
    pub name: String,
    /// Path to the NixOS flake containing the microvm declaration.
    pub flake: PathBuf,
    /// Static IP of the VM.
    pub ip: std::net::Ipv4Addr,
    /// Host workspace directory.
    pub workspace: PathBuf,
    /// VM mount point for workspace.
    pub workspace_mount: PathBuf,
    /// Secrets to inject.
    pub secrets: Vec<SecretSpec>,
    /// Agentic tool to provision.
    pub tool: Option<AgentTool>,
    /// Editor to use inside the VM.
    pub editor: Option<Editor>,
    /// WezTerm layout.
    pub layout: Layout,
    /// Whether to open WezTerm (false = just print SSH info).
    pub use_wezterm: bool,
    /// Whether to destroy VM state on exit.
    pub ephemeral: bool,
    /// SSH readiness timeout.
    pub ssh_timeout: Duration,
    /// Backend to use (MicroVm or Bwrap).
    pub backend: Backend,
    /// Sandbox network mode (only used with Bwrap backend).
    pub sandbox_network: NetworkMode,
    /// Sandbox seccomp policy (only used with Bwrap backend).
    pub sandbox_seccomp: SeccompPolicy,
    /// Extra paths to deny read access (only used with Bwrap backend).
    pub sandbox_deny_paths: Vec<PathBuf>,
    /// Allowed proxy domains (only used with Bwrap backend + Proxy network).
    pub sandbox_proxy_domains: Vec<String>,
}

/// Inject secrets into the VM via SSH pipe to environment file in tmpfs.
///
/// Writes secrets to `/dev/shm/vmux-secrets.env` inside the VM (tmpfs, no disk).
async fn inject_secrets(
    vm: &MicroVm,
    secrets: &[(String, vmux_secrets::SecretValue)],
) -> Result<(), LaunchError> {
    if secrets.is_empty() {
        return Ok(());
    }

    // Build env file content — never log this.
    let mut env_content = String::new();
    for (name, value) in secrets {
        env_content.push_str(&format!("export {}='{}'\n", name, value.expose().replace('\'', "'\\''")));
    }

    // Pipe to VM via SSH without touching host disk.
    let output = tokio::process::Command::new("ssh")
        .args([
            "-o", "StrictHostKeyChecking=no",
            "-o", "UserKnownHostsFile=/dev/null",
            "-o", "BatchMode=yes",
            &vm.ssh_host(),
            "cat > /dev/shm/vmux-secrets.env && chmod 600 /dev/shm/vmux-secrets.env",
        ])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| LaunchError::Vm(MicroVmError::SshFailed(e.to_string())))?
        .wait_with_output()
        .await
        .map_err(|e| LaunchError::Vm(MicroVmError::SshFailed(e.to_string())))?;

    // Note: we need to actually write to stdin. Let's use a different approach.
    drop(output);

    // Use echo via SSH to write the file.
    let ssh_cmd = format!(
        "cat > /dev/shm/vmux-secrets.env << 'VMUX_EOF'\n{}VMUX_EOF\nchmod 600 /dev/shm/vmux-secrets.env",
        env_content
    );

    vm.ssh_exec(&ssh_cmd)
        .await
        .map_err(|e| LaunchError::Vm(e))?;

    tracing::info!(
        count = secrets.len(),
        "secrets injected into VM at /dev/shm/vmux-secrets.env"
    );

    Ok(())
}

/// Build the SSH command prefix for a pane connecting into the VM.
fn ssh_prefix(vm: &MicroVm) -> String {
    format!(
        "ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -t {}",
        vm.ssh_host()
    )
}

/// Build the command to run the editor inside the VM.
fn editor_cmd(vm: &MicroVm, config: &LaunchConfig) -> String {
    let editor = config
        .editor
        .as_ref()
        .map(|e| e.command())
        .unwrap_or("nvim");
    format!(
        "{} 'source /dev/shm/vmux-secrets.env 2>/dev/null; cd {} && {}'",
        ssh_prefix(vm),
        config.workspace_mount.display(),
        editor
    )
}

/// Build the command to run the agent tool inside the VM.
fn agent_cmd(vm: &MicroVm, config: &LaunchConfig) -> String {
    let tool = config
        .tool
        .as_ref()
        .map(|t| t.launch_command())
        .unwrap_or("bash");
    format!(
        "{} 'source /dev/shm/vmux-secrets.env 2>/dev/null; cd {} && {}'",
        ssh_prefix(vm),
        config.workspace_mount.display(),
        tool
    )
}

/// Send commands to WezTerm panes based on their roles.
async fn send_pane_commands(
    session: &WezTermSession,
    vm: &MicroVm,
    config: &LaunchConfig,
) -> Result<(), LaunchError> {
    if session.panes.contains_key(&PaneRole::Editor) {
        session
            .send_command(PaneRole::Editor, &editor_cmd(vm, config))
            .await?;
    }
    if session.panes.contains_key(&PaneRole::Agent) {
        session
            .send_command(PaneRole::Agent, &agent_cmd(vm, config))
            .await?;
    }
    if session.panes.contains_key(&PaneRole::Logs) {
        let logs_cmd = format!("{} 'journalctl -f'", ssh_prefix(vm));
        session
            .send_command(PaneRole::Logs, &logs_cmd)
            .await?;
    }
    Ok(())
}

/// Re-attach to a running VM's WezTerm session.
///
/// If the original panes are still alive, focuses them.
/// If they're gone, rebuilds the layout with new panes connected to the VM.
pub async fn attach(config: LaunchConfig) -> Result<(), LaunchError> {
    // Create VM handle to check state and get SSH info.
    let vm = MicroVm::from_config(MicroVmConfig {
        name: config.name.clone(),
        flake: config.flake.clone(),
        ip: config.ip,
        workspace_host: config.workspace.clone(),
        workspace_vm: config.workspace_mount.clone(),
    });

    // Verify the VM is actually running.
    let running = vm.is_running().await.unwrap_or(false);
    if !running {
        return Err(LaunchError::Config(format!(
            "VM '{}' is not running. Use `vmux launch` to start it.",
            config.name
        )));
    }

    // Try to restore existing session; if panes are dead, rebuild.
    let session = match WezTermSession::restore(&config.name).await {
        Ok(session) => {
            tracing::info!(
                panes = session.panes.len(),
                "re-attached to existing WezTerm panes"
            );
            session
        }
        Err(_) => {
            tracing::info!("no live panes found, rebuilding WezTerm layout");
            let session = WezTermSession::rebuild_layout(&config.name, config.layout).await?;
            send_pane_commands(&session, &vm, &config).await?;
            session
        }
    };

    // Wait for the session to close.
    session.wait_for_close().await?;
    SessionState::remove(&config.name);

    Ok(())
}

/// Build a `SandboxConfig` from the launch config and resolved secrets.
fn build_sandbox_config(
    config: &LaunchConfig,
    secrets: &[(String, vmux_secrets::SecretValue)],
) -> SandboxConfig {
    let mut sandbox_cfg = SandboxConfig::for_workspace(&config.name, &config.workspace);

    // Network mode.
    sandbox_cfg.network = config.sandbox_network.clone();

    // Inject proxy env vars if using proxy mode.
    if let NetworkMode::Proxy(ref proxy_cfg) = sandbox_cfg.network {
        for (k, v) in proxy_cfg.sandbox_env() {
            sandbox_cfg.env.insert(k, v);
        }
    }

    // Inject secrets as env vars.
    for (name, value) in secrets {
        sandbox_cfg.env.insert(name.clone(), value.expose().to_string());
    }

    // Extra deny paths.
    for path in &config.sandbox_deny_paths {
        sandbox_cfg.deny_paths.push(path.clone());
    }

    // Seccomp policy.
    sandbox_cfg.seccomp = config.sandbox_seccomp.clone();

    sandbox_cfg
}

/// Build a bwrap command string for a given tool, suitable for a WezTerm pane.
fn sandbox_pane_cmd(sandbox: &Sandbox, command: &str) -> Result<String, LaunchError> {
    let cmd = sandbox.command_string(&[command.into()])?;
    Ok(cmd)
}

/// Send commands to WezTerm panes using bwrap sandbox.
async fn send_sandbox_pane_commands(
    session: &WezTermSession,
    sandbox: &Sandbox,
    config: &LaunchConfig,
) -> Result<(), LaunchError> {
    if session.panes.contains_key(&PaneRole::Editor) {
        let editor = config
            .editor
            .as_ref()
            .map(|e| e.command())
            .unwrap_or("nvim");
        let cmd = sandbox_pane_cmd(sandbox, editor)?;
        session.send_command(PaneRole::Editor, &cmd).await?;
    }
    if session.panes.contains_key(&PaneRole::Agent) {
        let tool = config
            .tool
            .as_ref()
            .map(|t| t.launch_command())
            .unwrap_or("bash");
        let cmd = sandbox_pane_cmd(sandbox, tool)?;
        session.send_command(PaneRole::Agent, &cmd).await?;
    }
    if session.panes.contains_key(&PaneRole::Logs) {
        // Logs pane runs unsandboxed — it's just watching the host process.
        let cmd = "echo 'Sandbox logs — no journalctl in bwrap mode'";
        session.send_command(PaneRole::Logs, cmd).await?;
    }
    Ok(())
}

/// Launch using the bwrap sandbox backend.
async fn launch_sandbox(config: LaunchConfig) -> Result<(), LaunchError> {
    // Step 1: Resolve secrets.
    let resolver = SecretResolver::new();
    let secrets = resolver.resolve_all(&config.secrets).await?;
    tracing::info!(count = secrets.len(), "secrets resolved");

    // Step 2: Build sandbox config.
    let sandbox_cfg = build_sandbox_config(&config, &secrets);
    let sandbox = Sandbox::new(sandbox_cfg);

    tracing::info!(
        name = %config.name,
        network = ?config.sandbox_network,
        "sandbox configured"
    );

    // Step 3: Open WezTerm or run directly.
    if config.use_wezterm {
        let session = WezTermSession::open(config.layout).await?;
        session.save_state(&config.name)?;

        send_sandbox_pane_commands(&session, &sandbox, &config).await?;

        tracing::info!("WezTerm session opened, waiting for close...");
        session.wait_for_close().await?;
        SessionState::remove(&config.name);
    } else {
        // No WezTerm — run agent directly in the sandbox.
        let tool = config
            .tool
            .as_ref()
            .map(|t| t.launch_command())
            .unwrap_or("bash");

        let cmd = sandbox.command_string(&[tool.into()])?;
        println!("Sandbox command:\n  {}", cmd);
        println!("\nWorkspace: {} -> /workspace", config.workspace.display());
        println!("Network: {:?}", config.sandbox_network);
        println!("Secrets injected: {}", secrets.len());
    }

    Ok(())
}

/// Run the full vmux launch sequence.
pub async fn launch(config: LaunchConfig) -> Result<(), LaunchError> {
    // Dispatch to the appropriate backend.
    if config.backend == Backend::Bwrap {
        return launch_sandbox(config).await;
    }

    // Step 1: Resolve secrets.
    let resolver = SecretResolver::new();
    let secrets = resolver.resolve_all(&config.secrets).await?;
    tracing::info!(count = secrets.len(), "secrets resolved");

    // Step 2: Create VM handle.
    let mut vm = MicroVm::from_config(MicroVmConfig {
        name: config.name.clone(),
        flake: config.flake.clone(),
        ip: config.ip,
        workspace_host: config.workspace.clone(),
        workspace_vm: config.workspace_mount.clone(),
    });

    // Step 3: Start VM (if not already running).
    let already_running = vm.is_running().await.unwrap_or(false);
    if !already_running {
        vm.start().await?;
    }

    // Ensure cleanup on any exit path.
    let vm_name = config.name.clone();
    let ephemeral = config.ephemeral;
    let _guard = scopeguard::guard((), move |_| {
        if ephemeral {
            tracing::info!(name = %vm_name, "cleaning up VM (ephemeral)");
            // Synchronous cleanup — best effort.
            let _ = std::process::Command::new("microvm")
                .args(["-s", &vm_name])
                .status();
        }
    });

    // Step 4: Wait for SSH.
    vm.wait_for_ssh(config.ssh_timeout).await?;

    // Step 5: Inject secrets.
    inject_secrets(&vm, &secrets).await?;

    // Step 6: Open WezTerm or print SSH info.
    if config.use_wezterm {
        let session = WezTermSession::open(config.layout).await?;

        // Persist session state for re-attach.
        session.save_state(&config.name)?;

        // Send commands to panes based on layout.
        send_pane_commands(&session, &vm, &config).await?;

        tracing::info!("WezTerm session opened, waiting for close...");
        session.wait_for_close().await?;

        // Clean up saved state when session ends.
        SessionState::remove(&config.name);
    } else {
        // No WezTerm mode — print connection info.
        println!("VM '{}' is running.", config.name);
        println!("SSH: ssh -o StrictHostKeyChecking=no {}", vm.ssh_host());
        println!("Secrets injected: {}", secrets.len());
        println!("Workspace: {} -> {}", config.workspace.display(), config.workspace_mount.display());
        println!("\nUse `vmux stop {}` to shut down the VM.", config.name);
    }

    Ok(())
}

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
