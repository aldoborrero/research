# vmux Architecture

> Agentic VM Launcher for NixOS — orchestrating WezTerm, microvm.nix, and
> coding agents into sandboxed development environments.

## CLI Interface

```
vmux launch [OPTIONS] [-- COMMAND]
    --name <name>          VM name (matches microvm.nix declaration)
    --workspace <path>     Host directory to mount as /workspace in VM
    --secret <KEY=source>  Secret to inject (repeatable)
                           Sources: op://vault/item/field
                                    sops:path/to/file
                                    env:VAR_NAME
                                    file:path
    --tool <tool>          Agentic tool: claude, aider, codex
    --editor <editor>      Editor inside VM: nvim, hx, emacs
    --layout <layout>      WezTerm layout: editor-agent (default),
                           agent-only, editor-only, full
    --no-wezterm           Print SSH info only, no WezTerm
    --ephemeral            Destroy VM on exit (default: true)
    --reuse                Attach to existing VM if running
    --dry-run              Print resolved config as JSON

vmux list                  List running vmux-managed VMs
vmux stop <name>           Stop a named VM
vmux attach <name>         Re-attach WezTerm to a running VM
```

## Crate Structure

```
vmux/
├── Cargo.toml             (workspace root)
├── crates/
│   ├── vmux-cli/          Main binary, clap arg parsing, vmux.toml config
│   │   └── src/main.rs
│   ├── vmux-core/         Launch orchestration, VM lifecycle coordination
│   │   └── src/lib.rs
│   ├── vmux-wezterm/      WezTerm IPC via `wezterm cli`, pane management
│   │   └── src/lib.rs
│   ├── vmux-secrets/      Secret resolution: 1Password, sops, env, file
│   │   └── src/lib.rs
│   └── vmux-nix/          microvm.nix lifecycle via systemctl + microvm CLI
│       └── src/lib.rs
├── docs/
│   ├── research/          5 research documents
│   └── architecture.md    This file
└── examples/
    └── basic-claude-session.md
```

### Dependency Graph

```
vmux-cli ──→ vmux-core ──→ vmux-wezterm
         │            ├──→ vmux-secrets
         │            └──→ vmux-nix
         ├──→ vmux-wezterm (for attach command)
         ├──→ vmux-secrets (for dry-run resolution display)
         └──→ vmux-nix (for list/stop commands)
```

### Crate Responsibilities

| Crate | Owns | External deps |
|-------|------|---------------|
| `vmux-cli` | Arg parsing, `vmux.toml` config, subcommands | `clap`, `toml`, `anyhow` |
| `vmux-core` | Launch sequence orchestration, cleanup guards | `scopeguard` |
| `vmux-wezterm` | `WezTermSession`, pane spawning/splitting/text | `serde_json` |
| `vmux-secrets` | `SecretResolver`, all source types, `SecretValue` | `zeroize` |
| `vmux-nix` | `MicroVm`, start/stop/ssh_exec, readiness polling | `ipnetwork` |

All library crates use `thiserror` for typed errors. Only `vmux-cli` uses `anyhow`.
All crates use `tracing` for structured logging. All use `tokio` for async.

## VM Launch Sequence

`vmux-core::launch()` executes these steps in order:

```
1. Resolve secrets
   ├── Parse --secret args and vmux.toml [[secrets]]
   ├── For each: call op read, sops decrypt, env::var, or fs::read
   └── Store as Vec<(String, SecretValue)> — zeroized on drop

2. Check VM state
   ├── microvm -l or systemctl is-active microvm@<name>
   └── If running and --reuse: skip to step 4

3. Start VM
   ├── systemctl start microvm@<name>
   └── On failure: capture journal logs, print, abort

4. Wait for SSH readiness
   ├── Poll: ssh -o ConnectTimeout=2 -o BatchMode=yes root@<ip> true
   ├── Retry every 1s, timeout after 30s
   └── Show spinner with elapsed time

5. Inject secrets
   ├── Build env file: export KEY='value' (shell-escaped)
   ├── SSH pipe to /dev/shm/vmux-secrets.env (tmpfs)
   └── chmod 600

6. Open WezTerm layout (if --no-wezterm is not set)
   ├── wezterm cli spawn → first pane (captures pane_id)
   ├── wezterm cli split-pane --right → additional panes
   ├── send-text each pane with SSH command:
   │   ├── Editor pane: ssh -t root@ip 'source secrets; cd /workspace; nvim'
   │   ├── Agent pane:  ssh -t root@ip 'source secrets; cd /workspace; claude'
   │   └── Logs pane:   ssh -t root@ip 'journalctl -f'
   └── Wait for all panes to close (poll wezterm cli list)

7. Cleanup (runs via scopeguard even on panic)
   ├── If ephemeral: systemctl stop microvm@<name>
   └── Zeroize all secrets (automatic via Drop)
```

If `--no-wezterm` is set, step 6 is replaced with printing SSH connection info.

## Error Handling Strategy

- All errors are typed via `thiserror` in library crates
- `vmux-cli` uses `anyhow` for the binary entrypoint
- VM start failures print the systemd journal output
- SSH timeout shows a spinner with elapsed time
- Secret resolution failures **never** print the secret value
- Cleanup (VM stop) runs even on panic via `scopeguard::guard`

### Error Types

```rust
// vmux-core
enum LaunchError { Vm(MicroVmError), Secret(SecretError), WezTerm(WezTermError), ... }

// vmux-nix
enum MicroVmError { VmNotDeclared, StartFailed, StopFailed, SshTimeout, ... }

// vmux-secrets
enum SecretError { OpCliNotFound, EnvVarNotSet, FileNotFound, InvalidSource, ... }

// vmux-wezterm
enum WezTermError { CliNotFound, CliFailed, ParseError, PaneNotFound, ... }
```

## Configuration

### vmux.toml (per-project)

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

### Config Precedence

CLI flags override `vmux.toml` values. Secrets from both sources are merged
(CLI takes priority on name collision).

```
CLI flags > vmux.toml > defaults
```

Auto-discovery: `vmux.toml` is found by walking up from CWD.

## WezTerm Integration

Based on research findings, WezTerm is driven exclusively via `wezterm cli`
subcommands from Rust's `tokio::process::Command`:

- **Spawn**: `wezterm cli spawn` → captures pane ID from stdout
- **Split**: `wezterm cli split-pane --right --pane-id <id>` → captures new pane ID
- **Send text**: `wezterm cli send-text --no-paste --pane-id <id>` via stdin pipe
- **Query**: `wezterm cli list --format json` → parse with serde_json
- **Targeting**: Set `WEZTERM_UNIX_SOCKET` env var or use `--pane-id`

No Lua embedding is needed.

## Secret Management

### Resolution Chain

```
User input              vmux-secrets            VM
─────────              ────────────            ──
op://vault/item/field  → op read → value ──┐
sops:path/to/file      → sops -d → value  ─┤
env:VAR_NAME           → env::var → value  ─┤→ SSH pipe → /dev/shm/vmux-secrets.env
file:/path             → fs::read → value ──┘
```

### Security Properties

- `SecretValue` uses `zeroize` — memory is zeroed on drop
- `Debug` impl prints `SecretValue(***)` — no leaks in logs
- Secrets are never written to host disk
- Secrets in VM live on tmpfs (`/dev/shm`) — not persisted
- `tracing` never logs secret values at any level

## microvm.nix Integration

Based on research, vmux uses the **pre-declared + `autostart = false`** pattern:

1. User declares VM in their NixOS flake with `autostart = false`
2. `nixos-rebuild switch` creates the systemd unit (one-time)
3. vmux calls `systemctl start microvm@<name>` (fast, no Nix evaluation)
4. vmux calls `systemctl stop microvm@<name>` on cleanup

IP addresses are statically assigned in the NixOS flake and mirrored in
`vmux.toml`.

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| `wezterm cli` not Lua | CLI returns pane IDs synchronously; Lua is async/not-awaitable |
| SSH CLI not `russh` crate | TUI apps (nvim, claude) need proper PTY; ssh CLI "just works" |
| SSH push for secrets | No flake changes needed for new secrets; flexible backend |
| `systemctl` not `microvm -r` | systemd units are the production path; foreground is for debug |
| `tokio` not `async-std` | Ecosystem standard; required by russh if added later |
| `thiserror` in libs | Typed errors, composable; `anyhow` only in binary crate |
| `zeroize` for secrets | Memory safety for sensitive data; defense in depth |
| `scopeguard` for cleanup | VM teardown must happen even on panic |

## Future Directions

1. **MCP server**: Expose vmux operations (start/stop/exec) as MCP tools for Claude Code
2. **NixOS module**: `vmux.vms` option mirroring `microvm.vms` with vmux-specific fields
3. **Ephemeral keypairs**: Generate per-session SSH keys for stronger isolation
4. **Connection pooling**: Use `russh` for multiplexed non-interactive commands
5. **`vmux attach`**: Full implementation for re-attaching to running sessions
6. **Warm pool**: Pre-booted VMs for instant session start
