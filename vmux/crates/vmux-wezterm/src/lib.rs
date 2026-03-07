use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from WezTerm IPC operations.
#[derive(Debug, Error)]
pub enum WezTermError {
    #[error("`wezterm` CLI not found in PATH")]
    CliNotFound,

    #[error("wezterm CLI failed: {0}")]
    CliFailed(String),

    #[error("failed to parse wezterm JSON output: {0}")]
    ParseError(String),

    #[error("pane {0} not found in session")]
    PaneNotFound(PaneId),

    #[error("WEZTERM_UNIX_SOCKET not set — is WezTerm running?")]
    NoSocket,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Opaque pane identifier from WezTerm.
pub type PaneId = u64;

/// Role of a pane in the vmux workspace layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaneRole {
    Editor,
    Agent,
    Logs,
    Shell,
}

/// Layout preset for WezTerm workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Layout {
    /// Editor on the left, agent on the right.
    EditorAgent,
    /// Agent only, full screen.
    AgentOnly,
    /// Editor only, full screen.
    EditorOnly,
    /// Editor | Agent | Logs — three columns.
    Full,
}

impl Layout {
    /// Parse from a string (CLI arg).
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s {
            "editor-agent" => Some(Layout::EditorAgent),
            "agent-only" => Some(Layout::AgentOnly),
            "editor-only" => Some(Layout::EditorOnly),
            "full" => Some(Layout::Full),
            _ => None,
        }
    }

    /// Roles required by this layout.
    pub fn roles(&self) -> Vec<PaneRole> {
        match self {
            Layout::EditorAgent => vec![PaneRole::Editor, PaneRole::Agent],
            Layout::AgentOnly => vec![PaneRole::Agent],
            Layout::EditorOnly => vec![PaneRole::Editor],
            Layout::Full => vec![PaneRole::Editor, PaneRole::Agent, PaneRole::Logs],
        }
    }
}

impl Default for Layout {
    fn default() -> Self {
        Layout::EditorAgent
    }
}

/// A pane entry returned by `wezterm cli list --format json`.
#[derive(Debug, Deserialize)]
struct WezTermPane {
    pane_id: PaneId,
    #[allow(dead_code)]
    tab_id: u64,
    #[allow(dead_code)]
    window_id: u64,
}

/// Persisted session state for re-attach.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub panes: HashMap<PaneRole, PaneId>,
    pub socket: Option<String>,
}

impl SessionState {
    /// Default state directory: `~/.local/state/vmux/sessions/`.
    fn state_dir() -> Option<PathBuf> {
        dirs::state_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".local/state")))
            .map(|d| d.join("vmux/sessions"))
    }

    fn path_for(vm_name: &str) -> Option<PathBuf> {
        Self::state_dir().map(|d| d.join(format!("{}.json", vm_name)))
    }

    /// Save session state to disk.
    pub fn save(&self, vm_name: &str) -> Result<(), WezTermError> {
        let path = Self::path_for(vm_name)
            .ok_or_else(|| WezTermError::CliFailed("cannot determine state directory".into()))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| WezTermError::ParseError(e.to_string()))?;
        std::fs::write(&path, json)?;
        tracing::debug!(path = %path.display(), "saved session state");
        Ok(())
    }

    /// Load session state from disk.
    pub fn load(vm_name: &str) -> Result<Self, WezTermError> {
        let path = Self::path_for(vm_name)
            .ok_or_else(|| WezTermError::CliFailed("cannot determine state directory".into()))?;
        let json = std::fs::read_to_string(&path).map_err(|_| {
            WezTermError::CliFailed(format!("no saved session for VM '{}'", vm_name))
        })?;
        let state: SessionState =
            serde_json::from_str(&json).map_err(|e| WezTermError::ParseError(e.to_string()))?;
        Ok(state)
    }

    /// Remove session state from disk.
    pub fn remove(vm_name: &str) {
        if let Some(path) = Self::path_for(vm_name) {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// An active WezTerm session with tracked panes.
pub struct WezTermSession {
    pub panes: HashMap<PaneRole, PaneId>,
    socket: Option<String>,
}

impl WezTermSession {
    /// Open a new WezTerm workspace with the given layout.
    ///
    /// Each pane is spawned via `wezterm cli` and its ID is tracked.
    pub async fn open(layout: Layout) -> Result<Self, WezTermError> {
        let socket = env::var("WEZTERM_UNIX_SOCKET").ok();

        let mut session = WezTermSession {
            panes: HashMap::new(),
            socket,
        };

        let roles = layout.roles();
        if roles.is_empty() {
            return Ok(session);
        }

        // Spawn the first pane in a new tab.
        let first_pane_id = session.spawn_tab().await?;
        session.panes.insert(roles[0], first_pane_id);

        // Split for additional panes.
        for role in roles.iter().skip(1) {
            let pane_id = session
                .split_pane(first_pane_id, SplitDirection::Right)
                .await?;
            session.panes.insert(*role, pane_id);
        }

        Ok(session)
    }

    /// Send a command (text + newline) to a specific pane by role.
    pub async fn send_command(&self, role: PaneRole, cmd: &str) -> Result<(), WezTermError> {
        let pane_id = self
            .panes
            .get(&role)
            .ok_or(WezTermError::PaneNotFound(0))?;

        let text = format!("{}\n", cmd);
        let mut args = self.base_args();
        args.extend([
            "cli".to_string(),
            "send-text".to_string(),
            "--pane-id".to_string(),
            pane_id.to_string(),
            "--no-paste".to_string(),
            text,
        ]);

        self.run_wezterm(&args).await?;
        Ok(())
    }

    /// Close all tracked panes.
    pub async fn close_all(&self) -> Result<(), WezTermError> {
        for (_role, pane_id) in &self.panes {
            // Send exit to each pane; ignore errors for already-closed panes.
            let text = "exit\n".to_string();
            let mut args = self.base_args();
            args.extend([
                "cli".to_string(),
                "send-text".to_string(),
                "--pane-id".to_string(),
                pane_id.to_string(),
                "--no-paste".to_string(),
                text,
            ]);
            let _ = self.run_wezterm(&args).await;
        }
        Ok(())
    }

    /// List all panes visible to the WezTerm instance.
    pub async fn list_panes(&self) -> Result<Vec<PaneId>, WezTermError> {
        let mut args = self.base_args();
        args.extend(["cli".to_string(), "list".to_string(), "--format".to_string(), "json".to_string()]);

        let output = self.run_wezterm_output(&args).await?;
        let panes: Vec<WezTermPane> = serde_json::from_str(&output)
            .map_err(|e| WezTermError::ParseError(e.to_string()))?;
        Ok(panes.into_iter().map(|p| p.pane_id).collect())
    }

    /// Wait for the user to close the WezTerm session.
    ///
    /// Polls `wezterm cli list` until our tracked panes disappear.
    pub async fn wait_for_close(&self) -> Result<(), WezTermError> {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let active_panes = self.list_panes().await.unwrap_or_default();
            let any_alive = self
                .panes
                .values()
                .any(|id| active_panes.contains(id));
            if !any_alive {
                break;
            }
        }
        Ok(())
    }

    /// Save the current session state to disk for later re-attach.
    pub fn save_state(&self, vm_name: &str) -> Result<(), WezTermError> {
        let state = SessionState {
            panes: self.panes.clone(),
            socket: self.socket.clone(),
        };
        state.save(vm_name)
    }

    /// Restore a session from saved state, validating that panes still exist.
    ///
    /// Returns the session with only the panes that are still alive in WezTerm.
    pub async fn restore(vm_name: &str) -> Result<Self, WezTermError> {
        let state = SessionState::load(vm_name)?;

        let session = WezTermSession {
            panes: state.panes.clone(),
            socket: state.socket,
        };

        // Validate which panes are still alive.
        let active = session.list_panes().await.unwrap_or_default();
        let alive: HashMap<PaneRole, PaneId> = session
            .panes
            .into_iter()
            .filter(|(_, id)| active.contains(id))
            .collect();

        if alive.is_empty() {
            SessionState::remove(vm_name);
            return Err(WezTermError::CliFailed(format!(
                "no live panes found for VM '{}' — session has ended",
                vm_name
            )));
        }

        Ok(WezTermSession {
            panes: alive,
            socket: session.socket,
        })
    }

    /// Re-create missing panes for the given layout, reusing any still-alive panes.
    ///
    /// Returns the rebuilt session with all roles populated.
    pub async fn rebuild_layout(
        vm_name: &str,
        layout: Layout,
    ) -> Result<Self, WezTermError> {
        let socket = env::var("WEZTERM_UNIX_SOCKET").ok();

        // Try to restore existing panes.
        let existing = match Self::restore(vm_name).await {
            Ok(s) => s.panes,
            Err(_) => HashMap::new(),
        };

        let mut session = WezTermSession {
            panes: HashMap::new(),
            socket,
        };

        let roles = layout.roles();
        if roles.is_empty() {
            return Ok(session);
        }

        // For roles that already have a live pane, reuse them.
        // For missing roles, spawn new panes.
        let anchor_pane = if let Some(&id) = existing.values().next() {
            // Reuse an existing pane as anchor for splits.
            id
        } else {
            // No existing panes — spawn a fresh tab.
            session.spawn_tab().await?
        };

        for (i, role) in roles.iter().enumerate() {
            if let Some(&id) = existing.get(role) {
                session.panes.insert(*role, id);
            } else if i == 0 && existing.is_empty() {
                // First role gets the anchor pane.
                session.panes.insert(*role, anchor_pane);
            } else {
                let pane_id = session
                    .split_pane(anchor_pane, SplitDirection::Right)
                    .await?;
                session.panes.insert(*role, pane_id);
            }
        }

        // Save the new state.
        session.save_state(vm_name)?;

        Ok(session)
    }

    // --- Private helpers ---

    fn base_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        if let Some(ref sock) = self.socket {
            args.push("--unix-socket".to_string());
            args.push(sock.clone());
        }
        args
    }

    async fn spawn_tab(&self) -> Result<PaneId, WezTermError> {
        let mut args = self.base_args();
        args.extend(["cli".to_string(), "spawn".to_string()]);
        let output = self.run_wezterm_output(&args).await?;
        parse_pane_id(&output)
    }

    async fn split_pane(
        &self,
        target: PaneId,
        direction: SplitDirection,
    ) -> Result<PaneId, WezTermError> {
        let mut args = self.base_args();
        args.extend([
            "cli".to_string(),
            "split-pane".to_string(),
            "--pane-id".to_string(),
            target.to_string(),
            direction.as_flag().to_string(),
        ]);
        let output = self.run_wezterm_output(&args).await?;
        parse_pane_id(&output)
    }

    async fn run_wezterm(&self, args: &[String]) -> Result<(), WezTermError> {
        let status = tokio::process::Command::new("wezterm")
            .args(args)
            .status()
            .await
            .map_err(|_| WezTermError::CliNotFound)?;

        if !status.success() {
            return Err(WezTermError::CliFailed(format!(
                "wezterm exited with {}",
                status
            )));
        }
        Ok(())
    }

    async fn run_wezterm_output(&self, args: &[String]) -> Result<String, WezTermError> {
        let output = tokio::process::Command::new("wezterm")
            .args(args)
            .output()
            .await
            .map_err(|_| WezTermError::CliNotFound)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WezTermError::CliFailed(stderr.into_owned()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[derive(Debug, Clone, Copy)]
enum SplitDirection {
    Right,
    #[allow(dead_code)]
    Bottom,
}

impl SplitDirection {
    fn as_flag(&self) -> &str {
        match self {
            SplitDirection::Right => "--right",
            SplitDirection::Bottom => "--bottom",
        }
    }
}

fn parse_pane_id(output: &str) -> Result<PaneId, WezTermError> {
    output
        .trim()
        .parse::<PaneId>()
        .map_err(|_| WezTermError::ParseError(format!("expected pane ID, got: {}", output.trim())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_roles() {
        assert_eq!(Layout::EditorAgent.roles().len(), 2);
        assert_eq!(Layout::AgentOnly.roles().len(), 1);
        assert_eq!(Layout::Full.roles().len(), 3);
    }

    #[test]
    fn layout_from_str() {
        assert_eq!(Layout::from_str_loose("editor-agent"), Some(Layout::EditorAgent));
        assert_eq!(Layout::from_str_loose("full"), Some(Layout::Full));
        assert_eq!(Layout::from_str_loose("bogus"), None);
    }

    #[test]
    fn parse_pane_id_valid() {
        assert_eq!(parse_pane_id("42\n").unwrap(), 42);
    }

    #[test]
    fn parse_pane_id_invalid() {
        assert!(parse_pane_id("not-a-number").is_err());
    }
}
