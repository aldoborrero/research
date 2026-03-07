use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from microvm.nix operations.
#[derive(Debug, Error)]
pub enum MicroVmError {
    #[error("VM `{name}` is not declared in the flake at {flake}")]
    VmNotDeclared { name: String, flake: String },

    #[error("VM `{name}` failed to start: {reason}")]
    StartFailed { name: String, reason: String },

    #[error("VM `{name}` failed to stop: {reason}")]
    StopFailed { name: String, reason: String },

    #[error("SSH to {ip} timed out after {elapsed:?}")]
    SshTimeout { ip: Ipv4Addr, elapsed: Duration },

    #[error("SSH command failed: {0}")]
    SshFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("`microvm` CLI not found in PATH")]
    MicrovmCliNotFound,

    #[error("failed to parse microvm output: {0}")]
    ParseError(String),
}

/// State of a microVM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VmState {
    Stopped,
    Starting,
    Running,
    Stopping,
}

/// Configuration for a microVM managed by vmux.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroVmConfig {
    /// Name of the VM (matches microvm.nix declaration).
    pub name: String,
    /// Path to the NixOS flake containing the microvm declaration.
    pub flake: PathBuf,
    /// Static IP address assigned to the VM.
    pub ip: Ipv4Addr,
    /// Host workspace path to mount via virtiofs.
    pub workspace_host: PathBuf,
    /// Mount point inside the VM.
    pub workspace_vm: PathBuf,
}

/// A running (or stopped) microVM handle.
pub struct MicroVm {
    pub config: MicroVmConfig,
    pub state: VmState,
}

impl MicroVm {
    /// Create a new MicroVm handle from config.
    pub fn from_config(config: MicroVmConfig) -> Self {
        MicroVm {
            config,
            state: VmState::Stopped,
        }
    }

    /// Start the VM using `microvm -r <name>`.
    pub async fn start(&mut self) -> Result<(), MicroVmError> {
        if self.state == VmState::Running {
            tracing::info!(name = %self.config.name, "VM already running");
            return Ok(());
        }

        self.state = VmState::Starting;
        tracing::info!(name = %self.config.name, "starting microVM");

        let output = tokio::process::Command::new("microvm")
            .args(["-r", &self.config.name])
            .current_dir(&self.config.flake)
            .output()
            .await
            .map_err(|_| MicroVmError::MicrovmCliNotFound)?;

        if !output.status.success() {
            self.state = VmState::Stopped;
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MicroVmError::StartFailed {
                name: self.config.name.clone(),
                reason: stderr.into_owned(),
            });
        }

        self.state = VmState::Running;
        tracing::info!(name = %self.config.name, "microVM started");
        Ok(())
    }

    /// Stop the VM using `microvm -s <name>`.
    pub async fn stop(&mut self) -> Result<(), MicroVmError> {
        if self.state == VmState::Stopped {
            return Ok(());
        }

        self.state = VmState::Stopping;
        tracing::info!(name = %self.config.name, "stopping microVM");

        let output = tokio::process::Command::new("microvm")
            .args(["-s", &self.config.name])
            .output()
            .await
            .map_err(|_| MicroVmError::MicrovmCliNotFound)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MicroVmError::StopFailed {
                name: self.config.name.clone(),
                reason: stderr.into_owned(),
            });
        }

        self.state = VmState::Stopped;
        tracing::info!(name = %self.config.name, "microVM stopped");
        Ok(())
    }

    /// Check if the VM is running using `microvm -l`.
    pub async fn is_running(&self) -> Result<bool, MicroVmError> {
        let output = tokio::process::Command::new("microvm")
            .arg("-l")
            .output()
            .await
            .map_err(|_| MicroVmError::MicrovmCliNotFound)?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().any(|line| line.trim() == self.config.name))
    }

    /// Poll SSH readiness with a timeout.
    pub async fn wait_for_ssh(&self, timeout: Duration) -> Result<(), MicroVmError> {
        let start = std::time::Instant::now();
        let ip = self.config.ip;

        tracing::info!(%ip, ?timeout, "waiting for SSH readiness");

        loop {
            if start.elapsed() > timeout {
                return Err(MicroVmError::SshTimeout {
                    ip,
                    elapsed: start.elapsed(),
                });
            }

            let result = tokio::process::Command::new("ssh")
                .args([
                    "-o", "StrictHostKeyChecking=no",
                    "-o", "UserKnownHostsFile=/dev/null",
                    "-o", "ConnectTimeout=2",
                    "-o", "BatchMode=yes",
                    &format!("root@{}", ip),
                    "true",
                ])
                .output()
                .await;

            match result {
                Ok(output) if output.status.success() => {
                    tracing::info!(%ip, elapsed = ?start.elapsed(), "SSH is ready");
                    return Ok(());
                }
                _ => {
                    tracing::debug!(%ip, elapsed = ?start.elapsed(), "SSH not ready yet, retrying...");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Run a command on the VM via SSH and return stdout.
    pub async fn ssh_exec(&self, command: &str) -> Result<String, MicroVmError> {
        let output = tokio::process::Command::new("ssh")
            .args([
                "-o", "StrictHostKeyChecking=no",
                "-o", "UserKnownHostsFile=/dev/null",
                "-o", "BatchMode=yes",
                &format!("root@{}", self.config.ip),
                command,
            ])
            .output()
            .await
            .map_err(|e| MicroVmError::SshFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MicroVmError::SshFailed(stderr.into_owned()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get the SSH connection string (user@ip).
    pub fn ssh_host(&self) -> String {
        format!("root@{}", self.config.ip)
    }
}

/// List all running microvm.nix VMs.
pub async fn list_running_vms() -> Result<Vec<String>, MicroVmError> {
    let output = tokio::process::Command::new("microvm")
        .arg("-l")
        .output()
        .await
        .map_err(|_| MicroVmError::MicrovmCliNotFound)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_host_format() {
        let vm = MicroVm::from_config(MicroVmConfig {
            name: "test-vm".to_string(),
            flake: PathBuf::from("/tmp/flake"),
            ip: "192.168.83.10".parse().unwrap(),
            workspace_host: PathBuf::from("/home/user/project"),
            workspace_vm: PathBuf::from("/workspace"),
        });
        assert_eq!(vm.ssh_host(), "root@192.168.83.10");
    }

    #[test]
    fn vm_initial_state() {
        let vm = MicroVm::from_config(MicroVmConfig {
            name: "test".to_string(),
            flake: PathBuf::from("."),
            ip: "10.0.0.1".parse().unwrap(),
            workspace_host: PathBuf::from("."),
            workspace_vm: PathBuf::from("/workspace"),
        });
        assert_eq!(vm.state, VmState::Stopped);
    }
}
