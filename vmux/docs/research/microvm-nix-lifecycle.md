# microvm.nix Lifecycle Management

## Question

Can vmux start/stop microvm.nix VMs without requiring the user to call `nixos-rebuild` first, or must the VM be pre-declared?

## Findings

### Two Management Modes: Declarative vs Imperative

microvm.nix offers two distinct management paths:

**Declarative** — VMs are declared in the host's NixOS configuration via `microvm.vms.<name>` and deployed with `nixos-rebuild switch`. The host module (`microvm.nixosModules.host`) generates systemd units automatically.

**Imperative** — VMs are created from any flake via the `microvm` CLI tool, without touching the host NixOS configuration. State lives under `/var/lib/microvms/$NAME/`.

Both paths result in `microvm@<name>.service` systemd units.

### The `microvm` CLI

| Flag | Action |
|------|--------|
| `microvm -c <name>` | Create a MicroVM (builds from flake, installs systemd unit) |
| `microvm -u <name>` | Rebuild/update a MicroVM |
| `microvm -r <name>` | Run a MicroVM in the foreground |
| `microvm -l` | List all MicroVMs (compares running vs flake versions; slow) |
| `microvm -f <flake>` | Specify source flake |
| `microvm -R` | Restart after update |

There is **no** `microvm -s <name>` for stop. Stopping is done via systemd.

### Systemd Integration

- **Unit template**: `microvm@.service`
- **Start**: `systemctl start microvm@<name>`
- **Stop**: `systemctl stop microvm@<name>`
- **Restart**: `systemctl restart microvm@<name>`
- **Status**: `systemctl status microvm@<name>`

### Declaring VMs in `flake.nix` (Declarative Path)

On the host, after enabling `microvm.nixosModules.host`:

```nix
microvm.vms.myvm = {
  autostart = false;  # do not start on boot
  flake = self;       # or a remote flake ref
  config = { ... };   # inline NixOS module configuration
};
```

This requires `nixos-rebuild switch` on the host to deploy changes.

### Imperative Path (No `nixos-rebuild`)

```bash
# Create from any flake that has nixosConfigurations.<name>:
microvm -c my-agent-vm -f /path/to/my/agent-flake

# The VM is now registered; start it:
systemctl start microvm@my-agent-vm

# Later, stop it:
systemctl stop microvm@my-agent-vm
```

### `autostart = false` and Manual Start

Setting `autostart = false` in the declarative `microvm.vms` block means the systemd unit is created but not enabled. The VM must be started manually with `systemctl start microvm@<name>`. This is the pattern Stapelberg uses for coding agent VMs.

### virtiofs Share Configuration (`microvm.shares`)

Inside the VM's NixOS configuration:

```nix
microvm.shares = [
  {
    proto = "virtiofs";
    tag = "ro-store";
    source = "/nix/store";
    mountPoint = "/nix/.ro-store";
  }
  {
    proto = "virtiofs";
    tag = "workspace";
    source = "/home/user/microvm/project-name";
    mountPoint = "/home/agent/workspace";
  }
];
```

Key points:
- `proto = "virtiofs"` is required for VMs started via systemd
- The host-side `virtiofsd` process is managed automatically by the host module
- virtiofsd config is tunable: `microvm.virtiofsd.group`, `microvm.virtiofsd.inodeFileHandles`, `microvm.virtiofsd.threadPoolSize`

### TAP Network Setup and Deterministic SSH IP

Inside the VM config:

```nix
microvm.interfaces = [
  {
    type = "tap";
    id = "vm-myvm";
    mac = "52:54:00:xx:xx:xx";
  }
];
```

On the host, TAP interfaces are bridged:

```nix
systemd.network.netdevs."10-microvm".netdevConfig = {
  Kind = "bridge";
  Name = "microbr";
};
systemd.network.networks."10-microvm" = {
  matchConfig.Name = "microbr";
  addresses = [{ Address = "192.168.83.1/24"; }];
};
systemd.network.networks."11-microvm" = {
  matchConfig.Name = "vm-*";
  networkConfig.Bridge = "microbr";
};
networking.nat = {
  enable = true;
  internalInterfaces = [ "microbr" ];
};
```

Inside the guest, a static IP is assigned (e.g., `192.168.83.6/24` with gateway `192.168.83.1`). Because both the MAC address and the guest static IP are set in Nix configuration, the SSH IP is **fully deterministic**.

### Stapelberg's Coding Agent Pattern

From [Coding Agent VMs on NixOS with microvm.nix](https://michael.stapelberg.ch/posts/2026-02-01-coding-agent-microvm-nix/):

- VMs are declared in a `microvm.nix` file imported by the host config
- Each VM uses `autostart = false` and a `microvm-base.nix` helper
- Workspace dirs are shared via virtiofs
- The Nix store is shared read-only via virtiofs
- Network is a bridge at `192.168.83.1/24` with static IPs per VM
- VMs are started with `systemctl start microvm@<name>` and accessed via `ssh 192.168.83.x`
- VMs use cloud-hypervisor with 8 vCPUs and 4 GB RAM

## Decision

**Yes, with caveats.** Two viable approaches:

### Approach A: Imperative (`microvm -c` + `systemctl`)
- A VM flake with `nixosConfigurations.<name>` must exist
- `microvm -c <name> -f <flake>` creates the VM without `nixos-rebuild`
- `systemctl start/stop microvm@<name>` controls lifecycle
- **Caveat**: Requires Nix evaluation + build time

### Approach B: Pre-declared with `autostart = false` (Recommended)
- VMs are declared in the host config with `autostart = false`
- After one `nixos-rebuild switch`, all VM systemd units exist but are stopped
- vmux simply calls `systemctl start/stop microvm@<name>`
- **This is the simpler, more reliable path**

**Recommendation for vmux**: Use Approach B. Pre-declare a pool of VM configurations in the user's NixOS flake with `autostart = false`. vmux then:
1. Calls `systemctl start microvm@<name>` to boot a VM
2. Calls `ssh <deterministic-ip>` to connect
3. Calls `systemctl stop microvm@<name>` to tear down

## Unknowns / Risks

1. **Root required**: `systemctl start/stop microvm@*` requires root or polkit authorization.
2. **No `microvm -s` (stop) command**: Stopping is exclusively via `systemctl`.
3. **VM pool sizing**: With Approach B, the number of available VMs is fixed at `nixos-rebuild` time.
4. **virtiofsd lifecycle**: If vmux needs to change workspace share paths between runs, the `source` path is baked into the NixOS config. Workaround: use a stable mount point and symlink.
5. **State under `/var/lib/microvms/`**: If this directory is removed, the VM is effectively deleted.
6. **macOS support**: microvm.nix recently added macOS support (vfkit), but lifecycle management is Linux/NixOS-only.

---

*Sources: [microvm.nix GitHub](https://github.com/microvm-nix/microvm.nix), [Documentation](https://microvm-nix.github.io/microvm.nix/), [Imperative Management](https://microvm-nix.github.io/microvm.nix/microvm-command.html), [Declaring MicroVMs](https://microvm-nix.github.io/microvm.nix/declaring.html), [Stapelberg Blog](https://michael.stapelberg.ch/posts/2026-02-01-coding-agent-microvm-nix/)*
