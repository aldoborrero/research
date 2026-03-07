# Secret Injection Strategies

## Question

Which approach for passing secrets into a microVM at launch time works without modifying the NixOS flake each time a new secret is needed?

## Findings

### Approach 1: sops-nix with virtiofs `/run/secrets` share

**How it works**: sops-nix decrypts secrets on the host and places them at `/run/secrets/<name>`. A virtiofs share exposes `/run/secrets` (or a subset) into the VM.

**NixOS config (host)**:
```nix
sops.secrets.anthropic-api-key = {
  sopsFile = ./secrets.yaml;
};
microvm.shares = [{
  proto = "virtiofs";
  tag = "secrets";
  source = "/run/secrets";
  mountPoint = "/run/host-secrets";
}];
```

**Rust code**: No special handling — secrets are available as files inside the VM at `/run/host-secrets/<name>`.

| Pros | Cons |
|------|------|
| Declarative, NixOS-native | Requires sops setup on host |
| Secrets never touch disk unencrypted outside tmpfs | New secrets require editing sops config + nixos-rebuild |
| Well-documented, widely used | Shares entire `/run/secrets` — may over-expose |

### Approach 2: `op run --` wrapper around the VM process

**How it works**: The 1Password CLI's `op run` injects secrets as environment variables into a child process. `op read` retrieves a single secret value.

**Rust code**:
```rust
let output = Command::new("op")
    .args(["read", "op://Personal/Anthropic/api_key"])
    .output()?;
let secret = String::from_utf8(output.stdout)?.trim().to_string();
```

| Pros | Cons |
|------|------|
| Simple, 1Password-integrated | Requires `op` CLI + 1Password account |
| No NixOS config changes for new secrets | Must authenticate to 1Password first |
| Works on any platform | Secrets are in Rust process memory |

### Approach 3: Environment variable injection via systemd

**How it works**: Set environment variables in the microVM's systemd service.

**NixOS config**:
```nix
systemd.services."microvm@myvm".environment = {
  ANTHROPIC_API_KEY = "..."; # BAD: plaintext in nix store
};
```

| Pros | Cons |
|------|------|
| Simple, built-in | Env visible in `/proc/<pid>/environ` |
| No extra tools | Secrets end up in Nix store (world-readable) |
| | New secrets require nixos-rebuild |

**Verdict**: Not suitable for sensitive secrets. Only appropriate for non-sensitive config.

### Approach 4: SSH-based secret push after VM boot

**How it works**: After the VM boots and SSH is ready, pipe secrets into the VM via SSH, writing to tmpfs (`/dev/shm` or `/run`).

**Rust code**:
```rust
// Write secrets to tmpfs inside VM — never touches disk
let ssh_cmd = format!(
    "cat > /dev/shm/vmux-secrets.env << 'EOF'\nexport ANTHROPIC_API_KEY='{}'\nEOF\nchmod 600 /dev/shm/vmux-secrets.env",
    secret_value
);
Command::new("ssh")
    .args(["-o", "StrictHostKeyChecking=no", &vm_ssh_host, &ssh_cmd])
    .status()?;
```

| Pros | Cons |
|------|------|
| Flexible — any secret source | Timing/race: must wait for SSH readiness |
| No NixOS config changes needed | Small window where secrets aren't available |
| No disk persistence | Secret in Rust process memory during transfer |
| Works with any secret backend | Requires SSH connection |

### Approach 5: tmpfs virtiofs share populated at runtime

**How it works**: Create a tmpfs on the host, populate it with secrets, share into VM via virtiofs, clean up on session end.

**NixOS config (host)**:
```nix
microvm.shares = [{
  proto = "virtiofs";
  tag = "runtime-secrets";
  source = "/run/vmux/myvm/secrets"; # host-side tmpfs
  mountPoint = "/run/secrets";
}];
```

**Rust code**:
```rust
// Create tmpfs-backed directory on host
fs::create_dir_all("/run/vmux/myvm/secrets")?;
fs::write("/run/vmux/myvm/secrets/api_key", secret_value)?;
// VM sees it at /run/secrets/api_key
// Cleanup on session end:
fs::remove_dir_all("/run/vmux/myvm/secrets")?;
```

| Pros | Cons |
|------|------|
| No disk persistence (tmpfs) | virtiofs source path must be pre-declared in NixOS config |
| Files available before VM userspace boots | Requires pre-configured share mount point |
| Clean, filesystem-based interface | Host-side tmpfs requires root to manage |

## Decision

**Use Approach 4 (SSH-based push) as the primary method, with Approach 2 (1Password) and env vars as secret sources.**

Rationale:
- **SSH push does not require NixOS flake modifications** when adding new secrets — this directly answers the key question.
- It works with any secret backend (1Password, sops, env vars, files) as the secret source.
- The Rust code resolves secrets from their sources (via `vmux-secrets`), then pipes the resolved values into the VM over SSH.
- Secrets are written to `/dev/shm/vmux-secrets.env` (tmpfs, never touches disk).
- The agent process inside the VM sources this file: `source /dev/shm/vmux-secrets.env`.

**Secret resolution chain**:
1. User specifies `--secret "API_KEY=op://vault/item/field"` or in `vmux.toml`
2. `vmux-secrets` resolves the source (calls `op read`, reads env var, etc.)
3. `vmux-core` pushes resolved values into VM via SSH after boot
4. Agent process sources `/dev/shm/vmux-secrets.env`

**Fallback for pre-boot secrets**: If a secret must be available before the VM's userspace boots (rare), use Approach 5 (tmpfs virtiofs share) with a pre-declared mount point.

## Unknowns / Risks

1. **SSH readiness timing**: Secrets can't be injected until SSH is ready. For most agentic workflows this is fine (the agent starts after secrets are available), but if a systemd service inside the VM needs secrets at boot time, the SSH approach won't work.

2. **Memory exposure**: Secrets exist in Rust process memory during resolution and transfer. The `zeroize` crate mitigates this by zeroing memory on drop, but the secret is briefly in memory.

3. **`/dev/shm` size**: The default tmpfs size for `/dev/shm` is typically 50% of RAM. For a few KB of secrets this is never a problem.

4. **Shell escaping in env file**: The SSH heredoc approach must properly escape single quotes in secret values. The implementation uses `'\''` escaping for single quotes within single-quoted strings.

5. **Multiple secret injection runs**: If `vmux launch` is called multiple times against the same running VM, each invocation overwrites `/dev/shm/vmux-secrets.env`. This is intentional — latest wins.

6. **sops-nix integration**: While not the primary path, users who already use sops-nix can use `file:/run/secrets/<name>` as a secret source, letting vmux-secrets read the decrypted file.

---

*Sources: [sops-nix](https://github.com/Mic92/sops-nix), [1Password CLI docs](https://developer.1password.com/docs/cli), [bubblewrap-claude](https://github.com/matgawin/bubblewrap-claude), [Stapelberg blog](https://michael.stapelberg.ch/posts/2026-02-01-coding-agent-microvm-nix/)*
