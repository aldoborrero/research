# Basic Claude Code Session with vmux

This example shows how to launch a sandboxed Claude Code session inside a
microvm.nix VM using vmux.

## Prerequisites

1. NixOS host with `microvm.nix` configured
2. A microVM declared in your flake (e.g., `myproject-vm`)
3. WezTerm installed and running
4. 1Password CLI (`op`) installed (if using `op://` secret sources)

## Using CLI flags

```bash
cd ~/projects/myproject

vmux launch \
  --name myproject-vm \
  --workspace . \
  --secret "ANTHROPIC_API_KEY=op://Personal/Anthropic/api_key" \
  --secret "GITHUB_TOKEN=env:GITHUB_TOKEN" \
  --tool claude \
  --editor nvim \
  --layout editor-agent
```

This will:
1. Start the `myproject-vm` microVM
2. Wait for SSH readiness
3. Inject both secrets into the VM (in-memory only)
4. Open WezTerm with nvim on the left and Claude Code on the right
5. Both panes are SSH-ed into the VM with `/workspace` as the CWD
6. On WezTerm close, the VM is destroyed (ephemeral by default)

## Using vmux.toml

Create a `vmux.toml` in your project root:

```toml
[vm]
name = "myproject-vm"
flake = "/home/aldo/machines/rhea"
ip = "192.168.83.10"

[workspace]
host_path = "."
vm_mount = "/workspace"

[tools]
agent = "claude"
editor = "nvim"

[layout]
type = "editor-agent"

[[secrets]]
name = "ANTHROPIC_API_KEY"
source = "op://Personal/Anthropic/api_key"

[[secrets]]
name = "GITHUB_TOKEN"
source = "env:GITHUB_TOKEN"
```

Then simply run:

```bash
vmux launch
```

## Dry-run mode

To see the resolved configuration without launching:

```bash
vmux launch --dry-run
```

## Headless mode (CI / non-WezTerm)

```bash
vmux launch --no-wezterm --tool claude --secret "KEY=env:KEY"
# Prints SSH connection info. VM stays running.
# Use `vmux stop myproject-vm` to shut down.
```

## Listing and stopping VMs

```bash
vmux list           # Show running VMs
vmux stop myproject-vm  # Stop a specific VM
```
