# SSH Access into the VM

## Question

Can we use `russh` for non-interactive command execution and fall back to `ssh` CLI for interactive panes (nvim, claude)?

## Findings

### 1. microvm.nix TAP/Bridge Networking

microvm.nix supports TAP interfaces per VM. The standard pattern is:

- **Bridge mode**: Create a host bridge (e.g., `microbr`), match TAP interfaces with `matchConfig.Name = "vm-*"`, and attach them to the bridge. NAT is configured via `networking.nat`. Common subnets: `10.0.0.0/24` or `192.168.83.0/24`.
- **Routed (bridgeless) mode**: Each VM gets a `/32` host route. More secure but slightly more complex.
- **QEMU user networking**: No TAP needed; use `microvm.forwardPorts` to forward host port 2222 to guest port 22.

### 2. IP Address Determinism

**IP addresses are fully deterministic and statically configured at build time.** There is no DHCP. The official routed-network documentation recommends assigning each VM a numeric index and deriving the IP from it. Stapelberg assigns each VM a unique IP like `192.168.83.3` chosen manually.

For vmux, the IP is known before the VM boots — it's configured in `vmux.toml` or the NixOS flake.

### 3. StrictHostKeyChecking for Ephemeral VMs

microvm.nix VMs on tmpfs generate new SSH host keys on every boot. Recommended flags:

```
ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR
```

This is standard practice for ephemeral infrastructure. Scope narrowly to the VM subnet.

**Alternative**: Stapelberg pre-generates host keys into a virtiofs-shared directory, avoiding regeneration.

### 4. Ephemeral SSH Key Pair Strategy

vmux should generate an ephemeral ed25519 key pair per session:

1. `ssh-keygen -t ed25519 -N "" -f /tmp/vmux-session-XXXX`
2. Inject `.pub` into VM's NixOS config: `users.users.root.openssh.authorizedKeys.keys`
3. Connect with: `ssh -i /tmp/vmux-session-XXXX ...`
4. Delete both keys on session end

This provides cryptographic session isolation with no persistent secrets.

### 5. The `russh` Crate

**Repository**: [Eugeny/russh](https://github.com/Eugeny/russh) (fork of Thrussh)

| Aspect | Details |
|---|---|
| **Async** | Native Tokio async |
| **Auth** | Password, public key, keyboard-interactive, agent forwarding |
| **Channels** | Full SSH channel multiplexing |
| **PTY** | `request_pty()` on channels; `client_exec_interactive.rs` example |
| **SFTP** | Via `russh-sftp` crate |
| **Security** | CryptoVec zeroes memory, mlock for sensitive data |
| **Adopters** | Warpgate, Devolutions Gateway, Yazi, Tabby terminal |

**Maturity**: Production-ready for non-interactive commands. Interactive PTY works but requires manual raw terminal mode, SIGWINCH forwarding, and byte shuttling.

**Limitation**: Does not support `~/.ssh/config`, `ProxyCommand`, or agent forwarding — irrelevant for vmux since we control connection parameters directly.

### 6. `ssh -t` for Interactive Sessions

`ssh -t` forces PTY allocation, required for TUI applications. For `ssh` CLI, terminal negotiation is handled automatically. For russh, you must call `channel.request_pty()`, put the local terminal into raw mode, and shuttle bytes manually.

### 7. SSH Readiness Polling

- Firecracker/cloud-hypervisor reach Linux userspace in ~125ms
- Full NixOS systemd init + sshd: **2-5 seconds** (Stapelberg: "boots and responds to pings within a few seconds")

**Polling strategy**:
```
loop {
    try TCP connect to port 22
    if success -> proceed with SSH handshake
    else -> sleep 100ms, retry
    timeout after 10 seconds -> error
}
```

### 8. `russh` vs `ssh` CLI Comparison

| Aspect | `russh` (in-process) | `ssh` CLI (subprocess) |
|---|---|---|
| **Non-interactive** | Excellent. Typed API, async, structured output | Works but requires pipe management |
| **Interactive PTY** | Possible but manual and fragile for TUI | "Just works". Battle-tested |
| **Connection reuse** | Native multiplexing | ControlMaster/ControlPath |
| **Error handling** | Typed Rust errors | Exit codes + stderr parsing |
| **Startup latency** | Zero | ~10-50ms per subprocess spawn |
| **Terminal compat** | Must handle all terminal semantics | OpenSSH handles everything |

## Decision

**Use a hybrid approach:**

1. **`ssh` CLI for all pane connections** (nvim, claude agent, shell sessions). PTY handling, terminal negotiation, and TUI compatibility all work out of the box. Spawn `ssh -t -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null root@<vm_ip>` as commands sent to WezTerm panes.

2. **`ssh` CLI (non-interactive) for command execution** (secret injection, health checks, tool provisioning). While `russh` would be more elegant, using `ssh` CLI uniformly reduces complexity and dependencies. The `tokio::process::Command` approach works well for capturing output.

3. **Defer `russh` integration**: Add it in a future version for connection pooling and multiplexed command execution if `ssh` CLI subprocess overhead becomes a bottleneck.

**Networking**: Use a bridge with static IPs assigned at Nix build time.

**Key management**: For the initial version, rely on the VM's NixOS-configured authorized keys (part of the flake). Ephemeral keypairs can be added later.

**Readiness**: Poll TCP port 22 with `ssh -o ConnectTimeout=2 -o BatchMode=yes` in a retry loop, timeout after 30 seconds.

## Unknowns / Risks

1. **Exact boot-to-SSH time**: Needs empirical measurement. Stapelberg says "a few seconds" but exact sshd readiness may vary.
2. **Bridge/NAT setup**: The host must have bridge interface and NAT rules pre-configured — a prerequisite vmux cannot set up without root.
3. **IP collision**: If using a `/24` subnet, ~253 addresses available. Sufficient for single-user use.
4. **Firewall**: VM's NixOS config must ensure port 22 is open.

---

*Sources: [russh on crates.io](https://crates.io/crates/russh), [russh docs](https://docs.rs/russh), [Stapelberg blog](https://michael.stapelberg.ch/posts/2026-02-01-coding-agent-microvm-nix/), [microvm.nix networking](https://microvm-nix.github.io/microvm.nix/)*
