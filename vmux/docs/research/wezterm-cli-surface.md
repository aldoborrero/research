# WezTerm CLI Surface

## Question

Can we reliably drive WezTerm from a Rust `std::process::Command` without embedding Lua?

## Findings

### 1. `wezterm cli spawn` — Spawn a new pane/tab

Spawns a command into a new window or tab. **Outputs the pane-id (integer) on stdout on success**, which is critical for programmatic chaining.

```
wezterm cli spawn [OPTIONS] [PROG]...
```

Key flags:
- `--new-window` — Spawn into a new window instead of a new tab
- `--cwd <CWD>` — Set the working directory for the spawned program
- `--pane-id <PANE_ID>` — Specify the reference pane (defaults to `$WEZTERM_PANE`)
- `--workspace <NAME>` — Set the workspace name (with `--new-window`)
- `--domain-name <DOMAIN>` — Target a specific mux domain
- `[PROG]...` — The command to run; omit for the default shell

Example capturing the pane ID:
```bash
pane_id=$(wezterm cli spawn --cwd /my/project -- bash -l)
```

### 2. `wezterm cli split-pane` — Split an existing pane

Creates a split and spawns a command into it. **Also outputs the new pane-id on stdout.**

```
wezterm cli split-pane [OPTIONS] [PROG]...
```

Direction flags (default is `--bottom`):
- `--right` / `--horizontal` — Horizontal split, new pane on the right
- `--left` — Horizontal split, new pane on the left
- `--top` — Vertical split, new pane on top
- `--bottom` — Vertical split, new pane on bottom
- `--top-level` — Split the entire window, not just the active pane

Sizing:
- `--percent <PERCENT>` — Percentage of available space (default 50)
- `--cells <CELLS>` — Absolute number of cells

Other:
- `--pane-id <PANE_ID>` — Which pane to split
- `--cwd <CWD>` — Working directory
- `--move-pane-id <ID>` — Move an existing pane into the new split instead of spawning

Example:
```bash
right_pane=$(wezterm cli split-pane --right --percent 30 --pane-id "$main_pane" -- htop)
```

### 3. `wezterm cli send-text` — Send keystrokes/text to a pane

Sends text to a pane as though it were pasted. By default uses bracketed paste mode if the target pane has it enabled.

```
wezterm cli send-text [OPTIONS] [TEXT]
```

Key flags:
- `--pane-id <PANE_ID>` — Target pane
- `--no-paste` — Skip bracketed paste wrapping; text is sent as raw input (important for commands that should execute immediately)

Input sources:
- **Argument**: `wezterm cli send-text --no-paste $'ls -la\r'` (use ANSI-C quoting for special chars)
- **Stdin**: `echo -e "ls -la\n" | wezterm cli send-text --no-paste --pane-id "$pane"` — reads until EOF then sends

**Important nuances for Rust `std::process::Command`:**
- When piping via stdin, the entire buffer is sent on EOF (when stdin is closed). This is the most reliable approach from Rust: write to the child's stdin, then drop it.
- The `--no-paste` flag is essential for sending commands that should execute (otherwise they get wrapped in bracketed paste escape sequences).
- Escape handling differs between argument and stdin modes. From Rust, piping bytes to stdin is more predictable than shell escaping in arguments.

### 4. `wezterm cli list` — List open panes (JSON output)

```
wezterm cli list --format json
```

Returns a JSON array of pane objects:

```json
[
  {
    "window_id": 0,
    "tab_id": 0,
    "pane_id": 0,
    "workspace": "default",
    "size": { "rows": 24, "cols": 80 },
    "title": "...",
    "cwd": "file://hostname/home/user/"
  }
]
```

Extended fields in newer versions: `cursor_x`, `cursor_y`, `cursor_shape`, `cursor_visibility`, `left_col`, `top_row`, `tab_title`, `window_title`, `is_active`, `is_zoomed`, `tty_name`, and `size.pixel_width`/`pixel_height`/`dpi`.

Also available: `wezterm cli list-clients --format json` which includes a `focused_pane_id` field per connected GUI client.

### 5. `wezterm cli get-pane-direction` — Navigate panes

```
wezterm cli get-pane-direction [OPTIONS] <DIRECTION>
```

Prints the pane-id of the pane in the specified direction relative to the current pane. Directions (case-insensitive): `Up`, `Down`, `Left`, `Right`, `Next`, `Prev`.

Additional navigation/manipulation subcommands:
- `wezterm cli activate-pane --pane-id <ID>` — Focus a specific pane
- `wezterm cli activate-pane-direction <DIR>` — Focus adjacent pane
- `wezterm cli activate-tab --tab-id <ID>` — Focus a tab
- `wezterm cli zoom-pane --pane-id <ID>` — Toggle zoom
- `wezterm cli move-pane-to-new-tab --pane-id <ID>` — Move pane to its own tab

### 6. Environment Variables: `WEZTERM_UNIX_SOCKET` and `WEZTERM_PANE`

**`WEZTERM_UNIX_SOCKET`**: Set by WezTerm in all spawned child processes. Points to the Unix domain socket of the parent WezTerm instance (e.g., `~/.local/share/wezterm/gui-sock-50325`). The `wezterm cli` subcommands use this to locate the running instance.

**`WEZTERM_PANE`**: Set by WezTerm in each spawned pane's environment. Contains the numeric pane ID. Used as the default `--pane-id` when that flag is omitted.

**For an external Rust process**: If your Rust process runs *outside* WezTerm, you must either:
1. Discover the socket path (it follows a predictable pattern in `$XDG_RUNTIME_DIR` or `~/.local/share/wezterm/`)
2. Pass `--pane-id` explicitly (discovered via `wezterm cli list`)
3. Set `WEZTERM_UNIX_SOCKET` in the environment before invoking `wezterm cli`

### 7. Headless / GUI Requirement

**WezTerm can run headless via `wezterm-mux-server`.** The architecture cleanly separates:
- `wezterm-gui` — The GUI frontend
- `wezterm-mux-server` — Headless multiplexer server (no display needed)
- `wezterm cli` — Client that talks to either

The `wezterm cli` commands work against both the GUI and the headless mux server over the same Unix socket RPC protocol. For a typical desktop use case (driving the terminal a user is looking at), the GUI must be running.

### 8. Lua `wezterm.mux` vs. External CLI

| Aspect | Lua (`wezterm.mux`) | CLI (`wezterm cli`) |
|---|---|---|
| **Execution context** | Inside WezTerm's Lua VM (config hooks, event callbacks) | External process via Unix socket RPC |
| **Latency** | In-process; negligible | Process spawn + socket round-trip (~5-20ms typical) |
| **Synchronicity** | Async / not awaitable from Lua; timing heuristics needed | Synchronous from caller's perspective |
| **Pane ID return** | Must query via `mux:get_*` methods | Returned on stdout from `spawn`/`split-pane` |
| **Error handling** | Lua pcall / limited | Exit code + stderr from the process |
| **Dependencies** | Requires modifying `~/.wezterm.lua` | Zero config; works out of the box |
| **Composability** | Tightly coupled to WezTerm config lifecycle | Fully composable from any language via `std::process::Command` |

**The CLI is strictly superior for external Rust process control.**

## Decision

**Yes, we can reliably drive WezTerm from Rust `std::process::Command` without embedding Lua.** The `wezterm cli` surface provides everything needed:

1. **Spawn panes/tabs**: `spawn` and `split-pane` return pane IDs on stdout, parseable as `u64`.
2. **Send commands**: `send-text --no-paste` via stdin pipe is the most reliable method from Rust.
3. **Query state**: `list --format json` gives structured pane data, deserializable with `serde_json`.
4. **Navigate**: `get-pane-direction`, `activate-pane`, `activate-pane-direction` for focus management.
5. **Targeting**: Pass `--pane-id` explicitly or set `WEZTERM_UNIX_SOCKET` in the `Command` environment.

Recommended Rust pattern:

```rust
use std::process::Command;
use std::io::Write;

// Spawn a pane and capture its ID
let output = Command::new("wezterm")
    .args(["cli", "spawn", "--cwd", "/my/project", "--", "bash", "-l"])
    .output()?;
let pane_id: u64 = String::from_utf8(output.stdout)?.trim().parse()?;

// Send a command to that pane
let mut child = Command::new("wezterm")
    .args(["cli", "send-text", "--no-paste", "--pane-id", &pane_id.to_string()])
    .stdin(std::process::Stdio::piped())
    .spawn()?;
child.stdin.take().unwrap().write_all(b"cargo build\r")?;
// stdin is dropped here, triggering EOF and send
child.wait()?;

// Query all panes
let output = Command::new("wezterm")
    .args(["cli", "list", "--format", "json"])
    .output()?;
let panes: serde_json::Value = serde_json::from_slice(&output.stdout)?;
```

## Unknowns / Risks

1. **Socket discovery outside WezTerm**: If the Rust process is not spawned inside a WezTerm pane, `WEZTERM_UNIX_SOCKET` will not be set. You must either hardcode a discovery heuristic (scanning `~/.local/share/wezterm/gui-sock-*`) or require the user to set the variable. Multiple running WezTerm instances produce multiple sockets.

2. **`send-text` is fire-and-forget**: There is no built-in way to confirm that a command sent via `send-text` has completed execution. The CLI has no `read-text` or `get-pane-output` counterpart.

3. **Race conditions on spawn**: Although `spawn` and `split-pane` are synchronous in returning the pane ID, the shell inside the new pane may not be fully initialized when the command returns. A small delay or readiness check may be needed.

4. **No atomic multi-operation**: Each CLI call is an independent RPC. There is no transaction or batch mode. Complex layouts require multiple sequential calls.

5. **Version skew**: JSON output fields have expanded across versions. Parsing should be tolerant of missing fields.

6. **`--no-paste` and `\r` vs `\n`**: To simulate pressing Enter, send `\r` (carriage return), not `\n`. When piping from Rust, write the literal byte `0x0D`.

---

*Sources: [WezTerm CLI docs](https://wezterm.org/cli/cli/index.html), [spawn](https://wezterm.org/cli/cli/spawn.html), [split-pane](https://wezterm.org/cli/cli/split-pane.html), [send-text](https://wezterm.org/cli/cli/send-text.html), [list](https://wezterm.org/cli/cli/list.html), [Multiplexing](https://wezterm.org/multiplexing.html)*
