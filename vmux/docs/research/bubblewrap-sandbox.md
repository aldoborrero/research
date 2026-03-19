# Bubblewrap as Agent Sandbox — Research

## Motivation

vmux currently uses microvm.nix to run agents (Claude Code, pi-agent, Codex)
inside a full NixOS virtual machine. This is heavyweight: ~5-10s boot, 4GB+ RAM,
root-only TAP networking, a complete NixOS flake config per VM.

The actual threat model is simpler: prevent an AI agent from reading sensitive
files (`~/.ssh`, `~/.gnupg`, `~/.aws`) or modifying the system outside the
project workspace. Both Anthropic and OpenAI chose bubblewrap (`bwrap`) for
exactly this — namespace-based sandboxing with zero overhead.

**Conclusion: microvm is not needed.** bwrap provides sufficient isolation for
agent sandboxing at a fraction of the complexity.

## How Claude Code's sandbox-runtime works

Source: [anthropic-experimental/sandbox-runtime](https://github.com/anthropic-experimental/sandbox-runtime)

### bwrap invocation

```
bwrap \
  --new-session \
  --die-with-parent \
  --unshare-net \
  --unshare-pid \
  --ro-bind / / \                          # read-only root filesystem
  --bind <writable-path> <writable-path> \ # each allowed write path
  --ro-bind <deny-path> <deny-path> \      # deny writes within allowed paths
  --ro-bind /dev/null <dangerous-file> \   # block specific files
  --tmpfs <read-deny-dir> \                # hide directories entirely
  --dev /dev \
  --proc /proc \
  --bind <http-socket> <http-socket> \     # proxy socket into sandbox
  --bind <socks-socket> <socks-socket> \
  --setenv HTTP_PROXY http://localhost:3128 \
  --setenv HTTPS_PROXY http://localhost:3128 \
  --setenv ALL_PROXY socks5h://localhost:1080 \
  -- <shell> -c <command>
```

### Network proxy pattern (socat bridge)

```
                    Host                          │  Sandbox (--unshare-net)
                                                  │
  ┌─────────────┐     ┌──────────────────┐        │  ┌──────────────────┐
  │ HTTP proxy   │◄────│ socat            │◄───UDS─┼──│ socat            │──► localhost:3128
  │ (domain      │     │ UNIX-LISTEN:sock │        │  │ TCP-LISTEN:3128  │
  │  filtering)  │     │ → TCP:proxy-port │        │  │ → UNIX:sock      │
  └─────────────┘     └──────────────────┘        │  └──────────────────┘
                                                  │
  ┌─────────────┐     ┌──────────────────┐        │  ┌──────────────────┐
  │ SOCKS5 proxy │◄────│ socat            │◄───UDS─┼──│ socat            │──► localhost:1080
  │ (SSH, DB,    │     │ UNIX-LISTEN:sock │        │  │ TCP-LISTEN:1080  │
  │  all TCP)    │     │ → TCP:proxy-port │        │  │ → UNIX:sock      │
  └─────────────┘     └──────────────────┘        │  └──────────────────┘
```

1. Host runs HTTP + SOCKS5 proxy servers with domain allow/deny lists
2. socat bridges: host TCP → Unix domain socket (UDS)
3. bwrap bind-mounts the UDS into the sandbox
4. Inside sandbox: socat bridges UDS → localhost TCP ports
5. Agent sees `HTTP_PROXY=http://localhost:3128` and routes all traffic through it

### Filesystem policy

- **Default**: `--ro-bind / /` (full filesystem read-only)
- **Writable**: only explicitly allowed paths get `--bind` (read-write)
- **Mandatory denies** (always write-protected):
  `.gitconfig`, `.bashrc`, `.zshrc`, `.profile`, `.mcp.json`,
  `.vscode/`, `.idea/`, `.claude/commands/`, `.git/hooks/`, `.git/config`
- **Read denies**: `--tmpfs` hides dirs, `--ro-bind /dev/null` hides files

### Secrets

Injected via `--setenv KEY VALUE` on the bwrap command line.
Only proxy-related vars are injected by the runtime itself.

## How OpenAI Codex's sandbox works

Source: [openai/codex — codex-rs/linux-sandbox](https://github.com/openai/codex/tree/main/codex-rs)

### bwrap invocation

```
bwrap \
  --new-session \
  --die-with-parent \
  --unshare-user \
  --unshare-pid \
  --unshare-net \
  --ro-bind / / \                          # or --tmpfs / for restricted read
  --dev /dev \
  --proc /proc \
  --bind <writable-root> <writable-root> \
  --ro-bind <protected> <protected> \      # .git, .codex made read-only
  --perms 000 --tmpfs <denied-dir> --remount-ro <denied-dir> \
  --chdir <cwd> \
  -- <command>
```

### Key differences from Claude Code

| Aspect | Claude Code | Codex |
|--------|-------------|-------|
| Language | TypeScript | Rust |
| User namespace | not unshared | `--unshare-user` always |
| Network proxy | Own HTTP+SOCKS5 proxy + socat | Piggybacks on existing proxy env vars; Rust TCP/UDS bridge |
| Nix store | Implicit (full `/` bind) | Explicit: `/nix/store` and `/run/current-system/sw` in defaults |
| Unreadable paths | `--tmpfs` / `--ro-bind /dev/null` | `--perms 000 --tmpfs --remount-ro` |
| Seccomp | Pre-generated BPF via C | In-process via `seccompiler` Rust crate |

### Nix store handling

Codex explicitly includes Nix paths in its platform defaults:
```rust
const LINUX_PLATFORM_DEFAULT_READ_ROOTS: &[&str] = &[
    "/bin", "/sbin", "/usr", "/etc", "/lib", "/lib64",
    "/nix/store",
    "/run/current-system/sw",
];
```

## microvm vs bwrap comparison

| Feature | microvm.nix | bubblewrap |
|---------|------------|------------|
| Startup time | ~5-10s (VM boot + SSH ready) | ~5ms (instant) |
| Memory overhead | 4GB+ reserved | Zero (shared kernel) |
| Root required | Yes (TAP, bridge, systemd units) | No (unprivileged namespaces) |
| Config complexity | Full NixOS flake + modules | Single bwrap command line |
| Filesystem sharing | virtiofs (kernel module) | Bind mounts (native) |
| Network isolation | TAP + bridge + static IP | `--unshare-net` (loopback only) |
| SSH access | Yes (full sshd) | Not needed (direct process spawn) |
| Init system | Full systemd | None (not needed) |
| Isolation boundary | Hardware (separate kernel) | Namespace (shared kernel) |
| Process spawning | SSH per pane | Direct bwrap per pane |
| Dependencies | cloud-hypervisor, microvm.nix, NixOS | Just `bwrap` binary (~3k lines C) |

## Proposed vmux architecture with bwrap

### Sandbox wrapper (Nix derivation)

```nix
(pkgs.writeShellScriptBin "sandbox" ''
  exec ${pkgs.bubblewrap}/bin/bwrap \
    --ro-bind /nix/store /nix/store \
    --ro-bind /run/current-system/sw /run/current-system/sw \
    --ro-bind /etc/resolv.conf /etc/resolv.conf \
    --ro-bind /etc/ssl /etc/ssl \
    --ro-bind /etc/passwd /etc/passwd \
    --ro-bind /etc/group /etc/group \
    --bind "''${VMUX_WORKSPACE:-.}" /workspace \
    --tmpfs /tmp \
    --tmpfs /home \
    --dev /dev \
    --proc /proc \
    --unshare-pid \
    --unshare-ipc \
    --unshare-net \
    --new-session \
    --die-with-parent \
    "$@"
'')
```

### Revised launch flow

```
vmux launch
├── Resolve secrets
├── For each pane:
│   └── wezterm cli spawn -- sandbox \
│       --setenv ANTHROPIC_API_KEY "..." \
│       --setenv HOME /home/agent \
│       -- claude --workspace /workspace
└── Wait for panes to close, cleanup
```

### Network access (when needed)

For agents that need API access, use the socat proxy pattern:

```
vmux launch
├── Start HTTP proxy on host (domain filtering)
├── Start socat: TCP → UDS bridge on host
├── For each pane:
│   └── wezterm cli spawn -- sandbox \
│       --bind /tmp/vmux-proxy.sock /tmp/vmux-proxy.sock \
│       --setenv HTTP_PROXY http://localhost:3128 \
│       -- sh -c 'socat TCP-LISTEN:3128,fork UNIX:/tmp/vmux-proxy.sock & exec claude'
└── Cleanup proxy + socat on exit
```

### vmux.toml config

```toml
[sandbox]
enabled = true
network = "proxy"              # none | proxy | host
allow_write = ["/workspace"]
deny_read = ["~/.ssh", "~/.gnupg", "~/.aws"]
proxy_allow = [                # domains reachable via proxy
  "api.anthropic.com",
  "api.openai.com",
  "github.com",
]
```

## What agents CANNOT do in the sandbox

- Read `~/.ssh/`, `~/.gnupg/`, `~/.aws/`, `~/.config/`
- See host processes (`--unshare-pid`)
- Access network directly (`--unshare-net`)
- Write anywhere outside `/workspace` and `/tmp`
- Escalate privileges (`PR_SET_NO_NEW_PRIVS`)
- Escape terminal session (`--new-session`)

## What agents CAN do

- Read/write project files in `/workspace`
- Use tools from Nix store (git, avim, curl — read-only)
- Call APIs through the controlled proxy
- Use `/tmp` for scratch

## References

- [containers/bubblewrap](https://github.com/containers/bubblewrap)
- [anthropic-experimental/sandbox-runtime](https://github.com/anthropic-experimental/sandbox-runtime)
- [openai/codex — codex-rs/linux-sandbox](https://github.com/openai/codex/tree/main/codex-rs/linux-sandbox)
- [Arch Wiki — Bubblewrap](https://wiki.archlinux.org/title/Bubblewrap)
- [Claude Code sandbox bwrap analysis (sambaiz.net)](https://www.sambaiz.net/en/article/547/)
- [nixpkgs buildFHSEnv bubblewrap](https://github.com/NixOS/nixpkgs/blob/master/pkgs/build-support/build-fhsenv-bubblewrap/default.nix)
