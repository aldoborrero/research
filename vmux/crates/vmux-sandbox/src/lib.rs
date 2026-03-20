//! Bubblewrap sandbox for vmux.
//!
//! Provides three layers of agent isolation:
//!
//! 1. **Filesystem** — bwrap bind mounts: read-only root, read-write workspace only
//! 2. **Network** — `--unshare-net` + proxy daemon for controlled API access
//! 3. **Seccomp** — BPF syscall filtering to prevent namespace escapes

mod bwrap;
mod proxy;
mod seccomp;

pub use bwrap::{BindMount, NetworkMode, Sandbox, SandboxConfig, SandboxError, SandboxState};
pub use proxy::{ProxyConfig, ProxyDaemon};
pub use seccomp::SeccompPolicy;
