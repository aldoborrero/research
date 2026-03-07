use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

use vmux_core::{AgentTool, Editor, LaunchConfig};
use vmux_secrets::SecretSpec;
use vmux_wezterm::Layout;

/// vmux — Agentic VM Launcher for NixOS
///
/// Orchestrates WezTerm, microvm.nix, and agentic tools (Claude Code, etc.)
/// into sandboxed development environments with secret injection.
#[derive(Parser, Debug)]
#[command(name = "vmux", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Launch a new vmux session (start VM, inject secrets, open WezTerm).
    Launch(LaunchArgs),
    /// List running vmux-managed VMs.
    List,
    /// Stop a named VM.
    Stop {
        /// VM name to stop.
        name: String,
    },
    /// Re-attach WezTerm layout to a running VM.
    Attach {
        /// VM name to attach to.
        name: String,
    },
}

#[derive(Parser, Debug)]
struct LaunchArgs {
    /// VM name (matches microvm.nix declaration).
    #[arg(long)]
    name: Option<String>,

    /// Host directory to mount as /workspace in VM.
    #[arg(long, default_value = ".")]
    workspace: PathBuf,

    /// Secret to inject. Format: NAME=source (can repeat).
    /// Sources: op://vault/item/field, sops:path, env:VAR, file:path
    #[arg(long = "secret", value_name = "KEY=SOURCE")]
    secrets: Vec<String>,

    /// Agentic tool to provision: claude, aider, codex.
    #[arg(long)]
    tool: Option<String>,

    /// Editor binary inside VM: nvim, hx, emacs.
    #[arg(long)]
    editor: Option<String>,

    /// WezTerm layout: editor-agent, agent-only, editor-only, full.
    #[arg(long, default_value = "editor-agent")]
    layout: String,

    /// Don't open WezTerm; just print SSH info.
    #[arg(long)]
    no_wezterm: bool,

    /// Destroy VM state on exit (default: true).
    #[arg(long, default_value_t = true)]
    ephemeral: bool,

    /// Attach to existing VM with same name if running.
    #[arg(long)]
    reuse: bool,

    /// Print resolved config as JSON and exit (don't launch).
    #[arg(long)]
    dry_run: bool,

    /// Path to vmux.toml config file (default: auto-detect).
    #[arg(long, short = 'c')]
    config: Option<PathBuf>,
}

/// Project-level config file (`vmux.toml`).
#[derive(Debug, Default, Serialize, Deserialize)]
struct ProjectConfig {
    #[serde(default)]
    vm: Option<VmConfig>,
    #[serde(default)]
    workspace: Option<WorkspaceConfig>,
    #[serde(default)]
    tools: Option<ToolsConfig>,
    #[serde(default)]
    layout: Option<LayoutConfig>,
    #[serde(default)]
    secrets: Vec<SecretEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VmConfig {
    name: Option<String>,
    flake: Option<String>,
    ip: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkspaceConfig {
    host_path: Option<String>,
    vm_mount: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolsConfig {
    agent: Option<String>,
    editor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LayoutConfig {
    #[serde(rename = "type")]
    layout_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SecretEntry {
    name: String,
    source: String,
}

/// Resolved configuration for JSON output in dry-run mode.
#[derive(Debug, Serialize)]
struct ResolvedConfig {
    name: String,
    flake: String,
    ip: String,
    workspace: String,
    workspace_mount: String,
    tool: Option<String>,
    editor: Option<String>,
    layout: String,
    use_wezterm: bool,
    ephemeral: bool,
    secrets: Vec<String>,
}

fn find_config_file(explicit: Option<&PathBuf>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        if path.exists() {
            return Some(path.clone());
        }
        return None;
    }

    // Walk up from cwd looking for vmux.toml.
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("vmux.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

fn load_project_config(path: Option<&PathBuf>) -> ProjectConfig {
    match find_config_file(path) {
        Some(config_path) => {
            tracing::debug!(path = %config_path.display(), "loading vmux.toml");
            match std::fs::read_to_string(&config_path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(config) => config,
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to parse vmux.toml, using defaults");
                        ProjectConfig::default()
                    }
                },
                Err(e) => {
                    tracing::warn!(error = %e, "failed to read vmux.toml, using defaults");
                    ProjectConfig::default()
                }
            }
        }
        None => ProjectConfig::default(),
    }
}

fn merge_config(args: &LaunchArgs, project: &ProjectConfig) -> Result<LaunchConfig> {
    let vm = project.vm.as_ref();
    let ws = project.workspace.as_ref();
    let tools = project.tools.as_ref();

    // VM name: CLI > vmux.toml > error.
    let name = args
        .name
        .clone()
        .or_else(|| vm.and_then(|v| v.name.clone()))
        .context("VM name is required (--name or vm.name in vmux.toml)")?;

    // Flake path: vmux.toml only (no CLI flag for now).
    let flake = vm
        .and_then(|v| v.flake.clone())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    // IP: vmux.toml only.
    let ip: Ipv4Addr = vm
        .and_then(|v| v.ip.clone())
        .unwrap_or_else(|| "192.168.83.10".to_string())
        .parse()
        .context("invalid IP address in config")?;

    // Workspace: CLI > vmux.toml.
    let workspace = if args.workspace != PathBuf::from(".") {
        args.workspace.clone()
    } else {
        ws.and_then(|w| w.host_path.as_ref())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    };
    let workspace = std::fs::canonicalize(&workspace)
        .unwrap_or(workspace);

    let workspace_mount = ws
        .and_then(|w| w.vm_mount.as_ref())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/workspace"));

    // Tool: CLI > vmux.toml.
    let tool = args
        .tool
        .as_deref()
        .or_else(|| tools.and_then(|t| t.agent.as_deref()))
        .map(AgentTool::from_str);

    // Editor: CLI > vmux.toml.
    let editor = args
        .editor
        .as_deref()
        .or_else(|| tools.and_then(|t| t.editor.as_deref()))
        .map(Editor::from_str);

    // Layout: CLI > vmux.toml.
    let layout_str = if args.layout != "editor-agent" {
        args.layout.as_str()
    } else {
        project
            .layout
            .as_ref()
            .and_then(|l| l.layout_type.as_deref())
            .unwrap_or("editor-agent")
    };
    let layout = Layout::from_str_loose(layout_str)
        .context(format!("unknown layout: {}", layout_str))?;

    // Secrets: CLI + vmux.toml (merged).
    let mut secrets = Vec::new();
    for s in &args.secrets {
        secrets.push(SecretSpec::parse_cli(s).context(format!("invalid secret: {}", s))?);
    }
    for entry in &project.secrets {
        // Only add from config if not already specified on CLI.
        if !secrets.iter().any(|s| s.name == entry.name) {
            let source = vmux_secrets::SecretSource::parse(&entry.source)
                .context(format!("invalid secret source in config: {}", entry.source))?;
            secrets.push(SecretSpec {
                name: entry.name.clone(),
                source,
            });
        }
    }

    Ok(LaunchConfig {
        name,
        flake,
        ip,
        workspace,
        workspace_mount,
        secrets,
        tool,
        editor,
        layout,
        use_wezterm: !args.no_wezterm,
        ephemeral: args.ephemeral,
        ssh_timeout: Duration::from_secs(30),
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Launch(args) => {
            let project = load_project_config(args.config.as_ref());
            let config = merge_config(&args, &project)?;

            if args.dry_run {
                let resolved = ResolvedConfig {
                    name: config.name.clone(),
                    flake: config.flake.display().to_string(),
                    ip: config.ip.to_string(),
                    workspace: config.workspace.display().to_string(),
                    workspace_mount: config.workspace_mount.display().to_string(),
                    tool: config.tool.as_ref().map(|t| t.launch_command().to_string()),
                    editor: config.editor.as_ref().map(|e| e.command().to_string()),
                    layout: format!("{:?}", config.layout),
                    use_wezterm: config.use_wezterm,
                    ephemeral: config.ephemeral,
                    secrets: config
                        .secrets
                        .iter()
                        .map(|s| s.name.clone())
                        .collect(),
                };
                println!("{}", serde_json::to_string_pretty(&resolved)?);
                return Ok(());
            }

            vmux_core::launch(config).await?;
        }
        Commands::List => {
            let vms = vmux_nix::list_running_vms().await?;
            if vms.is_empty() {
                println!("No vmux-managed VMs running.");
            } else {
                println!("Running VMs:");
                for vm in vms {
                    println!("  - {}", vm);
                }
            }
        }
        Commands::Stop { name } => {
            tracing::info!(name = %name, "stopping VM");
            let mut vm = vmux_nix::MicroVm::from_config(vmux_nix::MicroVmConfig {
                name: name.clone(),
                flake: PathBuf::from("."),
                ip: Ipv4Addr::UNSPECIFIED,
                workspace_host: PathBuf::from("."),
                workspace_vm: PathBuf::from("/workspace"),
            });
            vm.stop().await?;
            println!("VM '{}' stopped.", name);
        }
        Commands::Attach { name } => {
            // TODO: Re-attach to a running VM's WezTerm session.
            println!("Attaching to VM '{}'... (not yet implemented)", name);
        }
    }

    Ok(())
}
