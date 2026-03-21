# vmux Code Review

**Reviewer:** Claude (automated)
**Date:** 2026-03-21
**Scope:** Full codebase review — 6 crates, ~3,100 lines of Rust
**Build status:** Compiles cleanly, 28/28 tests pass

---

## Executive Summary

vmux is a well-structured Rust tool for launching isolated development environments with agentic AI tools (Claude Code, Aider, Codex) inside either bubblewrap sandboxes or NixOS microVMs, with WezTerm as the terminal multiplexer.

The architecture is clean, the crate boundaries are sensible, and the code is readable. The main areas for improvement are: **security hardening** (shell injection vectors, secret handling), **incomplete implementations** (seccomp BPF, proxy domain filtering), and **robustness** gaps (error recovery, concurrent secret resolution).

---

## Architecture Assessment

### Strengths

1. **Clean crate separation** — Each concern (sandbox, secrets, nix, wezterm, orchestration) is isolated with typed error enums and minimal cross-dependencies. The dependency graph flows strictly downward: `cli → core → {nix, sandbox, secrets, wezterm}`.

2. **Backend abstraction** — The `BackendProvider` trait + `AnyBackend` enum dispatch pattern avoids dyn-safety issues with async traits while keeping the orchestration layer backend-agnostic. This is pragmatic Rust.

3. **Security defaults** — Sandbox defaults to network-isolated (`--unshare-net`), denies sensitive paths (`.ssh`, `.gnupg`, `.aws`, etc.), and uses `--new-session` + `--die-with-parent` for process lifecycle safety.

4. **Secret hygiene** — `SecretValue` uses `zeroize` for memory safety, `Debug` impl redacts values, and secrets are injected into tmpfs (`/dev/shm`) with `chmod 600`.

5. **Config ergonomics** — The CLI-over-TOML precedence system with auto-discovery of `vmux.toml` walking up from CWD is user-friendly.

### Structural Issues

1. **`LaunchConfig` duplication** — The flat `LaunchConfig` struct duplicates fields that belong to specific backends (e.g., `flake`, `ip`, `ssh_timeout` are MicroVM-only; `sandbox_*` fields are bwrap-only). `into_parts()` then splits it apart. Consider making CLI parse directly into `CommonConfig` + backend-specific config.

2. **`Attach` command duplicates config resolution** — `main.rs:434-503` repeats most of the logic from `merge_config()` (IP parsing, workspace resolution, layout parsing). This should be factored into a shared function.

3. **`Backend` vs `BackendKind` naming** — The type alias `pub use BackendKind as Backend` at `vmux-core/src/lib.rs:622` is confusing since `BackendProvider` trait also exists. Consider committing to one naming scheme.

---

## Security Issues

### High Priority

**1. Shell injection in `inject_secrets_ssh` (`vmux-core/src/lib.rs:358-391`)**

The secret injection builds a shell command via string formatting:

```rust
env_content.push_str(&format!(
    "export {}='{}'\n",
    name,
    value.expose().replace('\'', "'\\''")
));
```

The `name` field is **not sanitized**. A malicious secret name like `FOO$(rm -rf /)` would be interpreted by the shell. The value escaping (single-quote replacement) is correct for values, but the name needs validation (alphanumeric + underscore only).

**2. Shell injection in `wrap_command` (`vmux-core/src/lib.rs:324-331`)**

```rust
Ok(format!(
    "ssh ... 'source /dev/shm/vmux-secrets.env 2>/dev/null; cd {} && {}'",
    "/workspace",
    command,
))
```

The `command` parameter is interpolated directly into a single-quoted SSH command string. If a command contains a single quote, this breaks out of the quoting. The bwrap backend uses `shell_join()` for escaping, but the MicroVM backend does not.

**3. Seccomp is not actually applied (`vmux-sandbox/src/seccomp.rs:111-128`)**

`SeccompPolicy::Default` and `SeccompPolicy::Strict` both return `Ok(None)` — meaning **no seccomp filter is ever applied**. The `build_args()` method in `bwrap.rs` never calls `seccomp.bwrap_args()` at all. This means the documented security guarantee of syscall filtering is not enforced.

**4. Proxy domain filtering is not implemented (`vmux-sandbox/src/proxy.rs:153-168`)**

The socat bridge connects to `TCP:localhost:0` (a placeholder). The `allow_domains` and `deny_domains` fields in `ProxyConfig` are never used for actual filtering. A sandboxed agent can currently reach any host if proxy mode is enabled.

### Medium Priority

**5. `StrictHostKeyChecking=no` everywhere**

Both `wait_for_ssh` and `ssh_exec` disable host key checking. This is expected for ephemeral VMs, but the SSH command in `wrap_command` also disables it for interactive sessions. Consider generating and trusting a known host key during VM provisioning.

**6. Secrets written to shared tmpfs (`/dev/shm`)**

`/dev/shm/vmux-secrets.env` is readable by any process running as the same user in the VM. If multiple vmux sessions share a VM, secrets leak across sessions. Consider using per-session filenames (e.g., `/dev/shm/vmux-secrets-{session_id}.env`).

**7. `VMUX_EOF` heredoc delimiter collision**

```rust
let ssh_cmd = format!(
    "cat > /dev/shm/vmux-secrets.env << 'VMUX_EOF'\n{}VMUX_EOF\n...",
    env_content
);
```

If a secret value contains the literal string `VMUX_EOF` on its own line, the heredoc terminates early and the rest of the secret becomes shell commands. Use a randomized delimiter or base64-encode the content.

---

## Correctness Issues

**8. `ephemeral` flag default is problematic (`vmux-cli/src/main.rs:84`)**

```rust
#[arg(long, default_value_t = true)]
ephemeral: bool,
```

`--ephemeral` defaults to `true`, but there's no `--no-ephemeral` or `--persistent` flag. With clap's default bool handling, `--ephemeral` is a flag that's always true — users cannot disable it via CLI. Use `--persistent` as an opt-in alternative or use `--ephemeral=false`.

**9. Workspace path comparison is fragile (`vmux-cli/src/main.rs:245`)**

```rust
if args.workspace != PathBuf::from(".") {
```

This checks if the CLI default was overridden by comparing against the default value string. If a user explicitly passes `--workspace .`, it won't use the TOML config. Use `Option<PathBuf>` and check `is_some()` instead.

**10. `MicroVmProvider::cleanup` uses sync `Command` (`vmux-core/src/lib.rs:337-339`)**

```rust
let _ = std::process::Command::new("microvm")
    .args(["-s", &vm.config.name])
    .status();
```

This is an async function that uses `std::process::Command` (blocking). It should use `tokio::process::Command` to avoid blocking the tokio runtime. The `MicroVm::stop()` method already does this correctly.

**11. `is_running` state drift**

`MicroVm::is_running()` checks the external `microvm -l` command but doesn't update `self.state`. After calling `is_running()`, the struct's `state` field may be `Stopped` even though the VM is actually running.

**12. `attach` ignores backend from config**

`vmux-cli/src/main.rs:495` hardcodes `backend: Backend::MicroVm` for the attach command, but a user could be attaching to a bwrap session. The backend should be read from the config file.

---

## Code Quality

**13. `from_str` methods shadow the `FromStr` trait**

`AgentTool::from_str` and `Editor::from_str` are inherent methods that shadow Rust's `std::str::FromStr` trait. Implement the trait instead — this enables `.parse::<AgentTool>()` and integrates with clap's value parsing.

**14. `resolve_all` is sequential**

`SecretResolver::resolve_all()` resolves secrets one at a time. Since secrets come from independent external sources (1Password, sops, env, files), they could be resolved concurrently with `futures::join_all` or `tokio::join!`. This matters when multiple 1Password or sops calls are involved.

**15. `Layout::from_str_loose` returns `Option` but is used with `.context()`**

The naming suggests a "loose" parse that might return `None`, but callers always treat `None` as an error. This should return `Result<Layout, E>` directly or implement `FromStr`.

**16. `send_command` reports wrong pane ID on error**

```rust
let pane_id = self
    .panes
    .get(&role)
    .ok_or(WezTermError::PaneNotFound(0))?;
```

The error always reports pane ID `0` instead of something meaningful. It should report the role name.

**17. No `#[must_use]` on key types**

`LaunchConfig`, `SandboxConfig`, `SecretValue`, and other builder-pattern types should have `#[must_use]` to prevent silent drops.

---

## Missing Functionality

18. **No `cargo clippy` in CI** — Several patterns (e.g., `&PathBuf` parameters, needless `.clone()`) would be caught by clippy. Consider running `cargo clippy -- -W clippy::pedantic`.

19. **No integration tests** — All tests are unit tests that don't exercise the actual bwrap/SSH/WezTerm interaction. Even a basic smoke test with `--dry-run` would catch regressions.

20. **No signal handling** — If the user Ctrl-C's during launch, the VM may be left running and secrets may remain in `/dev/shm`. The `scopeguard` dependency is imported but not used for cleanup in `launch_with_backend`.

21. **No log redaction** — While `SecretValue::Debug` is redacted, `tracing::debug!` calls that log bwrap args (`bwrap_args = ?args`) will include `--setenv SECRET_NAME secret_value` in debug logs.

---

## Minor Nits

- `vmux-sandbox/src/bwrap.rs:212`: `/etc/ssl` is already covered by `/etc` in `PLATFORM_READ_ROOTS` — the TLS cert bind mounts are redundant when `/etc` is already mounted.
- `vmux-sandbox/src/proxy.rs:149`: `_port` parameter is unused (placeholder implementation).
- `vmux-core/src/lib.rs:250`: `status_info` calls `.unwrap()` on `self.sandbox` which could panic if called before `prepare()`. The `sandbox()` helper method exists for this — use it.
- `vmux-core/src/lib.rs:345`: Same `.unwrap()` issue on `self.vm` in `MicroVmProvider::status_info`.
- `shell_join` in bwrap.rs doesn't handle newlines, tabs, or backtick characters, which could break quoting.

---

## Recommendations (Priority Order)

1. **Fix shell injection in secret name validation** — Add `fn validate_env_name(s: &str) -> bool` that enforces `[A-Za-z_][A-Za-z0-9_]*`.
2. **Fix shell injection in MicroVM `wrap_command`** — Apply proper escaping or use the same `shell_join` approach as bwrap.
3. **Implement seccomp BPF generation** — Use the `seccompiler` crate (as noted in the TODO) or at minimum wire `bwrap_args()` into `build_args()`.
4. **Implement proxy domain filtering** — Replace the socat placeholder with tinyproxy or a minimal Rust HTTP proxy.
5. **Factor out config resolution** — Eliminate duplication between `merge_config` and the `Attach` command.
6. **Add integration tests** — At minimum, test `--dry-run` end-to-end and bwrap arg generation with various configs.
7. **Implement `FromStr` trait** — Replace inherent `from_str` methods on `AgentTool`, `Editor`, and `Layout`.
8. **Add signal handling** — Use `tokio::signal` to catch SIGINT/SIGTERM and run cleanup.

---

## Summary

| Category | Count |
|----------|-------|
| Security (High) | 4 |
| Security (Medium) | 3 |
| Correctness | 5 |
| Code Quality | 5 |
| Missing Functionality | 4 |
| Minor Nits | 5 |

The codebase is well-designed for an early-stage project. The architecture is sound and the crate boundaries are clean. The main risk area is the gap between documented security guarantees and actual enforcement — seccomp and proxy filtering are documented but not implemented, and shell injection vectors exist in secret injection. These should be addressed before any production use.
