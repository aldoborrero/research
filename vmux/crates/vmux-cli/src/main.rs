use std::net::Ipv4Addr;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

use vmux_core::{AgentTool, Backend, Editor, LaunchConfig};
use vmux_sandbox::{NetworkMode, SeccompPolicy};
use vmux_secrets::SecretSpec;

/// vmux — sandboxed agent launcher
///
/// Run agentic tools (Claude Code, Aider, Codex) and editors inside
/// isolated bubblewrap sandboxes or microvm.nix VMs with automatic
/// secret injection.
#[derive(Parser, Debug)]
#[command(name = "vmux", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a command inside the sandbox/VM.
    ///
    /// Resolves secrets, prepares the environment, and exec's the command.
    /// Defaults to $SHELL if no command, --tool, or --editor is given.
    Run(RunArgs),

    /// Print resolved configuration as JSON (without launching).
    Config(ConfigArgs),

    /// List running vmux-managed VMs.
    List,

    /// Stop a named VM.
    Stop {
        /// VM name to stop.
        name: String,
    },
}

#[derive(Parser, Debug)]
struct RunArgs {
    /// Session name (matches VM declaration or sandbox label).
    #[arg(long)]
    name: Option<String>,

    /// Host directory to mount as /workspace.
    #[arg(long, default_value = ".")]
    workspace: PathBuf,

    /// Secret to inject. Format: NAME=source (can repeat).
    /// Sources: op://vault/item/field, sops:path, env:VAR, file:path
    #[arg(long = "secret", value_name = "KEY=SOURCE")]
    secrets: Vec<String>,

    /// Agentic tool to run: claude, aider, codex.
    #[arg(long)]
    tool: Option<String>,

    /// Editor to run: nvim, hx, emacs.
    #[arg(long)]
    editor: Option<String>,

    /// Destroy VM state on exit (default: true).
    #[arg(long, default_value_t = true)]
    ephemeral: bool,

    /// Path to vmux.toml config file (default: auto-detect).
    #[arg(long, short = 'c')]
    config: Option<PathBuf>,

    /// Command to run inside the environment (overrides --tool/--editor).
    #[arg(last = true)]
    command: Vec<String>,
}

#[derive(Parser, Debug)]
struct ConfigArgs {
    /// Session name.
    #[arg(long)]
    name: Option<String>,

    /// Host directory to mount as /workspace.
    #[arg(long, default_value = ".")]
    workspace: PathBuf,

    /// Path to vmux.toml config file (default: auto-detect).
    #[arg(long, short = 'c')]
    config: Option<PathBuf>,
}

/// Project-level config file (`vmux.toml`).
#[derive(Debug, Default, Serialize, Deserialize)]
struct ProjectConfig {
    #[serde(default)]
    backend: Option<String>,
    #[serde(default)]
    vm: Option<VmConfig>,
    #[serde(default)]
    workspace: Option<WorkspaceConfig>,
    #[serde(default)]
    tools: Option<ToolsConfig>,
    #[serde(default)]
    sandbox: Option<SandboxSection>,
    #[serde(default)]
    secrets: Vec<SecretEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SandboxSection {
    network: Option<String>,
    seccomp: Option<String>,
    proxy_allow: Option<Vec<String>>,
    deny_paths: Option<Vec<String>>,
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
    #[serde(alias = "mount")]
    vm_mount: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolsConfig {
    agent: Option<String>,
    editor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SecretEntry {
    name: String,
    source: String,
}

/// Resolved configuration for JSON output.
#[derive(Debug, Serialize)]
struct ResolvedConfig {
    name: String,
    backend: String,
    workspace: String,
    workspace_mount: String,
    command: String,
    ephemeral: bool,
    secrets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flake: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sandbox_network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sandbox_seccomp: Option<String>,
}

fn find_config_file(explicit: Option<&PathBuf>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        if path.exists() {
            return Some(path.clone());
        }
        return None;
    }

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

fn build_launch_config(
    name: Option<String>,
    workspace: PathBuf,
    secrets_raw: &[String],
    tool: Option<&str>,
    editor: Option<&str>,
    ephemeral: bool,
    config_path: Option<&PathBuf>,
) -> Result<LaunchConfig> {
    let project = load_project_config(config_path);
    let vm = project.vm.as_ref();
    let ws = project.workspace.as_ref();
    let tools = project.tools.as_ref();

    let name = name
        .or_else(|| vm.and_then(|v| v.name.clone()))
        .context("session name is required (--name or vm.name in vmux.toml)")?;

    let flake = vm
        .and_then(|v| v.flake.clone())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let ip: Ipv4Addr = vm
        .and_then(|v| v.ip.clone())
        .unwrap_or_else(|| "192.168.83.10".to_string())
        .parse()
        .context("invalid IP address in config")?;

    let workspace = if workspace != PathBuf::from(".") {
        workspace
    } else {
        ws.and_then(|w| w.host_path.as_ref())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    };
    let workspace = std::fs::canonicalize(&workspace).unwrap_or(workspace);

    let workspace_mount = ws
        .and_then(|w| w.vm_mount.as_ref())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/workspace"));

    let tool_parsed = tool
        .or_else(|| tools.and_then(|t| t.agent.as_deref()))
        .map(AgentTool::from_str);

    let editor_parsed = editor
        .or_else(|| tools.and_then(|t| t.editor.as_deref()))
        .map(Editor::from_str);

    // Secrets: CLI + vmux.toml (merged).
    let mut secrets = Vec::new();
    for s in secrets_raw {
        secrets.push(SecretSpec::parse_cli(s).context(format!("invalid secret: {}", s))?);
    }
    for entry in &project.secrets {
        if !secrets.iter().any(|s| s.name == entry.name) {
            let source = vmux_secrets::SecretSource::parse(&entry.source)
                .context(format!("invalid secret source in config: {}", entry.source))?;
            secrets.push(SecretSpec {
                name: entry.name.clone(),
                source,
            });
        }
    }

    let backend = project
        .backend
        .as_deref()
        .map(|b| match b {
            "bwrap" | "sandbox" => Backend::Bwrap,
            _ => Backend::MicroVm,
        })
        .unwrap_or(Backend::Bwrap);

    let sandbox_section = project.sandbox.as_ref();
    let sandbox_network = sandbox_section
        .and_then(|s| s.network.as_deref())
        .map(|n| match n {
            "none" => NetworkMode::None,
            "host" => NetworkMode::Host,
            "proxy" => NetworkMode::Proxy(vmux_sandbox::ProxyConfig {
                allow_domains: sandbox_section
                    .and_then(|s| s.proxy_allow.clone())
                    .unwrap_or_default(),
                ..vmux_sandbox::ProxyConfig::default()
            }),
            _ => NetworkMode::None,
        })
        .unwrap_or(NetworkMode::None);

    let sandbox_seccomp = sandbox_section
        .and_then(|s| s.seccomp.as_deref())
        .map(|s| match s {
            "strict" => SeccompPolicy::Strict,
            "default" => SeccompPolicy::Default,
            _ => SeccompPolicy::None,
        })
        .unwrap_or(SeccompPolicy::Default);

    let sandbox_deny_paths = sandbox_section
        .and_then(|s| s.deny_paths.clone())
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect();

    let sandbox_proxy_domains = sandbox_section
        .and_then(|s| s.proxy_allow.clone())
        .unwrap_or_default();

    Ok(LaunchConfig {
        name,
        flake,
        ip,
        workspace,
        workspace_mount,
        secrets,
        tool: tool_parsed,
        editor: editor_parsed,
        ephemeral,
        ssh_timeout: Duration::from_secs(30),
        backend,
        sandbox_network,
        sandbox_seccomp,
        sandbox_deny_paths,
        sandbox_proxy_domains,
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
        Commands::Run(args) => {
            let config = build_launch_config(
                args.name,
                args.workspace,
                &args.secrets,
                args.tool.as_deref(),
                args.editor.as_deref(),
                args.ephemeral,
                args.config.as_ref(),
            )?;

            // Determine the command to run.
            let command = if !args.command.is_empty() {
                args.command.join(" ")
            } else {
                config.command()
            };

            let (common, mut backend) = config.into_parts();

            // Resolve secrets and prepare backend.
            vmux_core::prepare_exec(&common, &mut backend).await?;

            // Build exec argv.
            let (program, argv) = backend.exec_argv(&command)?;

            tracing::info!(
                program = %program,
                "exec into environment"
            );

            // Replace this process with the sandboxed command.
            let err = std::process::Command::new(&program)
                .args(&argv)
                .exec();

            // exec() only returns on error.
            anyhow::bail!("exec failed: {}", err);
        }

        Commands::Config(args) => {
            let config = build_launch_config(
                args.name,
                args.workspace,
                &[],
                None,
                None,
                true,
                args.config.as_ref(),
            )?;

            let resolved = ResolvedConfig {
                name: config.name.clone(),
                backend: match config.backend {
                    Backend::Bwrap => "bwrap".to_string(),
                    Backend::MicroVm => "microvm".to_string(),
                },
                workspace: config.workspace.display().to_string(),
                workspace_mount: config.workspace_mount.display().to_string(),
                command: config.command(),
                ephemeral: config.ephemeral,
                secrets: config.secrets.iter().map(|s| s.name.clone()).collect(),
                flake: if config.backend == Backend::MicroVm {
                    Some(config.flake.display().to_string())
                } else {
                    None
                },
                ip: if config.backend == Backend::MicroVm {
                    Some(config.ip.to_string())
                } else {
                    None
                },
                sandbox_network: if config.backend == Backend::Bwrap {
                    Some(format!("{:?}", config.sandbox_network))
                } else {
                    None
                },
                sandbox_seccomp: if config.backend == Backend::Bwrap {
                    Some(format!("{:?}", config.sandbox_seccomp))
                } else {
                    None
                },
            };
            println!("{}", serde_json::to_string_pretty(&resolved)?);
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
    }

    Ok(())
}
