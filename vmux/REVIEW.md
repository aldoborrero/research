# vmux Code Review

## Summary

vmux is a Rust-based orchestration tool (~3,900 lines across 6 crates) that launches sandboxed development environments for AI coding agents (Claude Code, Aider, Codex). It combines WezTerm (terminal multiplexer), microvm.nix (NixOS VMs), and Bubblewrap (process-level sandboxing) into a unified CLI. The codebase is well-structured, idiomatic Rust, and demonstrates thoughtful architecture decisions.

**Overall assessment: Solid foundation with good separation of concerns. Several issues need attention before production use, primarily around security, error handling gaps, and incomplete implementations.**

---

## Architecture & Design

### Strengths

- **Clean crate decomposition**: The 6-crate workspace (`cli`, `core`, `wezterm`, `secrets`, `nix`, `sandbox`) follows single-responsibility well. Each crate owns one concern with minimal cross-contamination.

- **BackendProvider trait**: The pluggable backend model (`BackendProvider` + `AnyBackend` enum dispatch) is a pragmatic solution to async trait object limitations. It keeps orchestration logic backend-agnostic while avoiding `dyn` + async complexity.

- **Error handling discipline**: Consistent `thiserror` in library crates, `anyhow` in the binary. Error types are well-named and informative. The `LaunchError` enum composes sub-crate errors cleanly via `#[from]`.

- **Secret hygiene**: `SecretValue` with `Zeroize`/`ZeroizeOnDrop`, redacted `Debug` impl, tmpfs-only storage in VMs, and no disk writes on the host. This is above-average for a tool at this maturity level.

- **Config merging**: The CLI > vmux.toml > defaults precedence chain is clear and the `find_config_file()` walk-up-from-cwd pattern is user-friendly.

### Concerns

- **Architecture doc is stale**: The doc says "systemctl start microvm@\<name\>" but the code uses `microvm -r` (foreground mode). The crate structure diagram is also missing `vmux-sandbox`. These discrepancies will confuse contributors.

- **No integration tests**: Every test is a unit test on pure functions (parsing, formatting). There are no integration tests that exercise the launch sequence, even with mocked externals. The surface area most likely to break (process spawning, IPC, state management) has zero coverage.

---

## Security Issues

### HIGH: Shell injection in SSH command construction

`vmux-core/src/lib.rs:327-331` — `MicroVmProvider::wrap_command()`:

```rust
Ok(format!(
    "ssh ... -t {} 'source /dev/shm/vmux-secrets.env 2>/dev/null; cd {} && {}'",
    vm.ssh_host(),
    "/workspace",
    command,  // <-- unescaped user input
))
```

The `command` parameter is interpolated directly into a shell string wrapped in single quotes. If `command` contains a single quote (e.g., a custom tool name like `my'tool`), it breaks out of the quoting. The same issue exists in `BwrapProvider::wrap_command()` where the command flows through `shell_join`, but then gets embedded in `sh -c "cd ...; exec {}"` — the `exec` target is still unescaped.

**Recommendation**: Use `shell_join()` (or a proper shell-escaping function) on the `command` parameter before interpolation, or pass commands as separate SSH arguments instead of embedding in a shell string.

### HIGH: Secret injection heredoc delimiter collision

`vmux-core/src/lib.rs:376-378` — `inject_secrets_ssh()`:

```rust
let ssh_cmd = format!(
    "cat > /dev/shm/vmux-secrets.env << 'VMUX_EOF'\n{}VMUX_EOF\n...",
    env_content
);
```

If any secret value contains the literal string `VMUX_EOF` on its own line, the heredoc terminates early. While the single-quote escaping on line 372 handles most shell metacharacters, it doesn't handle heredoc delimiter collisions.

**Recommendation**: Use a randomized delimiter (e.g., `VMUX_EOF_<random_hex>`) or pipe the content via stdin instead of a heredoc.

### MEDIUM: Proxy bypass — domain filtering not implemented

`vmux-sandbox/src/proxy.rs:154-161` — `start_socat_bridge()` bridges traffic with `TCP:localhost:0` (a placeholder). The `allow_domains` and `deny_domains` fields in `ProxyConfig` are never actually enforced — traffic goes straight through socat to the internet. An agent in "proxy" network mode has unrestricted internet access despite the configuration suggesting otherwise.

**Recommendation**: Either implement the tinyproxy integration (as the architecture diagram shows) or clearly document that proxy mode is not yet functional and should not be relied on for security.

### MEDIUM: Seccomp not applied

`vmux-sandbox/src/seccomp.rs:111-128` — `SeccompPolicy::bwrap_args()` returns `Ok(None)` for both `Default` and `Strict` policies. The blocked syscall lists exist but are never compiled into BPF programs or passed to bwrap. The `build_args()` method in `bwrap.rs` never calls `seccomp.bwrap_args()` at all.

An agent running under `SeccompPolicy::Strict` has zero syscall filtering applied. Combined with the proxy bypass above, the sandbox provides only filesystem and PID namespace isolation.

**Recommendation**: Add a TODO/warning that's visible at runtime (not just a debug log), or integrate the `seccompiler` crate to generate BPF filters.

---

## Code Quality Issues

### Duplicated config resolution in `Attach` command

`vmux-cli/src/main.rs:434-502` — The `Attach` handler duplicates ~60 lines of config resolution logic that already exists in `merge_config()`. Fields like `flake`, `ip`, `workspace`, `workspace_mount`, `tool`, `editor`, and `layout` are resolved with identical patterns. This violates DRY and means bug fixes to config resolution must be applied in two places.

**Recommendation**: Extract a shared config builder or refactor `merge_config()` to accept optional `LaunchArgs` (with `None` for attach scenarios).

### `--ephemeral` defaults to `true` but can't be negated

`vmux-cli/src/main.rs:84` — `#[arg(long, default_value_t = true)]` means `--ephemeral` is always true. Clap doesn't generate `--no-ephemeral` for boolean args by default. Users cannot set `ephemeral = false` from the CLI.

**Recommendation**: Use `#[arg(long, default_value_t = true, action = clap::ArgAction::Set)]` or add an explicit `--persistent` flag.

### `--reuse` flag is parsed but never used

`vmux-cli/src/main.rs:89` — The `reuse` field is declared in `LaunchArgs` but never read in `main()` or passed to `LaunchConfig`. The architecture doc mentions "If running and --reuse: skip to step 4" but this is unimplemented.

### `from_str` methods shadow the `FromStr` trait

`vmux-core/src/lib.rs:91,121` — Both `AgentTool::from_str()` and `Editor::from_str()` are inherent methods that shadow the standard `FromStr` trait. This prevents using these types with `.parse()` or `clap`'s automatic parsing. It also violates Rust API conventions.

**Recommendation**: Implement `std::str::FromStr` instead of inherent methods.

### `BackendKind` vs `Backend` naming confusion

`vmux-core/src/lib.rs:622` — `pub use BackendKind as Backend;` creates two names for the same type. The CLI imports `Backend`, the core defines `BackendKind`. This creates confusion when reading cross-crate code.

**Recommendation**: Pick one name. Since the CLI is the public interface, `Backend` is probably better.

### Missing `--unshare-user` in bwrap args

`vmux-sandbox/src/bwrap.rs:181-182` — The sandbox unshares PID and IPC namespaces but not the user namespace. Without `--unshare-user`, the process runs as the host user with the host's UID, which means it has the same file permissions as the user. This weakens the filesystem isolation provided by bind mounts (the process could potentially access files through `/proc/self/root` or other escapes).

### HashMap iteration order for env vars

`vmux-sandbox/src/bwrap.rs:293` — Environment variables are iterated from a `HashMap`, which has non-deterministic order in Rust. This means the `bwrap` command line changes between runs, making debugging harder and potentially causing issues with tools that are sensitive to environment variable ordering.

**Recommendation**: Use `BTreeMap` or sort the keys before iteration.

### `status_info` unwraps without checking

`vmux-core/src/lib.rs:250,345` — Both `BwrapProvider::status_info()` and `MicroVmProvider::status_info()` call `.unwrap()` on `self.sandbox`/`self.vm`, which panics if called before `prepare()`. Since `status_info()` is part of the `BackendProvider` trait interface, callers could trigger this at any lifecycle stage.

**Recommendation**: Return `Result<String, LaunchError>` or handle the `None` case gracefully.

### `PaneNotFound` always reports pane 0

`vmux-wezterm/src/lib.rs:191` — When a pane role isn't found, the error reports pane ID 0 regardless of which role was requested:

```rust
.ok_or(WezTermError::PaneNotFound(0))?;
```

**Recommendation**: Either include the role in the error or use the correct pane ID.

---

## Missing Functionality

1. **No `scopeguard` usage**: The architecture doc mentions cleanup via `scopeguard::guard` on panic, but the actual code doesn't use `scopeguard` anywhere. If `launch_with_backend()` panics between VM start and the cleanup step, the VM is left running. The crate is listed as a dependency but never imported.

2. **No `--reuse` support**: The flag is defined but the feature is unimplemented.

3. **No `ipnetwork` usage**: Listed as a workspace dependency but never imported by any crate. The code uses `std::net::Ipv4Addr` directly.

4. **Proxy SOCKS5 bridge not started**: `ProxyDaemon::start()` only starts the HTTP bridge. The SOCKS5 socket path is set but no socat process is spawned for it.

5. **`BwrapProvider::cleanup()` is a no-op**: Uses the trait default (`async { Ok(()) }`). If proxy daemons or other resources are started during `prepare()`, they leak on exit.

6. **Session state for bwrap backend**: `save_state()` and `restore()` are only meaningful for MicroVm sessions, but the code calls them regardless of backend.

---

## Testing

The existing tests are good for what they cover — parsing, formatting, and basic invariants. Missing:

- No tests for `merge_config()` (the most complex function in the CLI)
- No tests for `build_args()` output correctness (the most security-critical function)
- No tests for `inject_secrets_ssh()` shell escaping
- No tests for `shell_join()` with adversarial inputs (quotes, newlines, null bytes)
- No async tests for WezTerm session lifecycle

The `env::set_var` / `env::remove_var` usage in `vmux-secrets` tests (`lib.rs:266-275`) is unsound in multi-threaded test execution (Rust 1.75+ warns about this).

---

## Dependencies

Dependencies are well-chosen and minimal. A few notes:

- `tokio = { features = ["full"] }` — Consider scoping to only needed features (`rt-multi-thread`, `process`, `time`, `fs`, `io-util`) to reduce compile times.
- `thiserror = "2"` — Good, using the latest version.
- `zeroize` with `derive` — Correct usage for secret handling.
- `ipnetwork = "0.20"` — Unused; remove it.
- `scopeguard = "1"` — Unused; either implement the panic-safe cleanup or remove it.

---

## Recommendations Summary

| Priority | Issue | Location |
|----------|-------|----------|
| **P0** | Shell injection in `wrap_command()` | `vmux-core/src/lib.rs:327` |
| **P0** | Heredoc delimiter collision in secret injection | `vmux-core/src/lib.rs:376` |
| **P1** | Proxy domain filtering not implemented (false sense of security) | `vmux-sandbox/src/proxy.rs` |
| **P1** | Seccomp policy never applied | `vmux-sandbox/src/seccomp.rs` |
| **P1** | No `scopeguard` cleanup on panic (doc claims it exists) | `vmux-core/src/lib.rs` |
| **P2** | Duplicated config resolution in Attach | `vmux-cli/src/main.rs:434-502` |
| **P2** | `--ephemeral` can't be set to false | `vmux-cli/src/main.rs:84` |
| **P2** | `from_str` shadows `FromStr` trait | `vmux-core/src/lib.rs:91,121` |
| **P2** | Remove unused deps (`ipnetwork`, `scopeguard`) | `Cargo.toml` |
| **P3** | Non-deterministic env var ordering | `vmux-sandbox/src/bwrap.rs:293` |
| **P3** | `PaneNotFound` always reports pane 0 | `vmux-wezterm/src/lib.rs:191` |
| **P3** | Stale architecture doc | `docs/architecture.md` |
| **P3** | Add integration tests for `build_args()` and `merge_config()` | Tests |
