# Existing Adjacent Tools

## Question

What existing tools in the microVM/sandbox/agentic-CLI space should vmux be aware of? Are there crates, patterns, or integrations worth reusing rather than building from scratch?

## Findings

### 1. microsandbox (zerocore-ai/microsandbox)

**What it does:** A self-hosted Rust platform providing microVM sandboxes for AI agent code execution. Uses libkrun for KVM-based (Linux) / HVF-based (macOS) micro-virtualization with sub-200ms boot times.

**Architecture:** Cargo workspace with 5+ crates:

| Crate | Role |
|---|---|
| `microsandbox` | Rust SDK for consumers |
| `microsandbox-core` | VM lifecycle, OCI image distribution, runtime management |
| `microsandbox-server` | HTTP server + built-in MCP server (`/mcp` endpoint) |
| `microsandbox-portal` | Agent-facing entry point |
| `microsandbox-utils` | Shared utilities |

**MCP integration:** The server exposes MCP tools over streamable HTTP at `/mcp`. Tools include sandbox start/stop, run code, execute commands, and get metrics. JSON-RPC with `initialize`, `tools/list`, `tools/call`.

**What vmux can learn:**
- The crate split (core engine vs server vs SDK) is a good model for vmux's own workspace
- MCP-as-HTTP-endpoint design proves the pattern works for agent integration
- The `Sandboxfile` concept (declarative sandbox definition) parallels `vmux.toml`
- Apache-2.0 licensed — patterns can be borrowed freely

**Worth depending on?** No. microsandbox uses libkrun; vmux uses cloud-hypervisor via microvm.nix. Fundamentally different isolation layers.

### 2. bubblewrap-claude (matgawin/bubblewrap-claude)

**What it does:** A Nix flake wrapping Claude Code in a bubblewrap sandbox with per-project profiles, secret injection, and network allowlisting.

**Architecture:**
- `bwLib.mkSandbox { name, packages, env, preStartHooks, args, allowList, customPrompt }` — core builder
- `bwLib.profiles.{go,python,...}` — language-specific presets
- `bwLib.deriveProfile` — extends an existing profile

**Secret injection pattern:** `preStartHooks` run shell commands before the sandbox launches:
```bash
export ANTHROPIC_API_KEY="$(cat /run/secrets/claude-key)"
```
Keeps secrets out of Nix store derivations.

**What vmux can learn:**
- The `mkSandbox`/`deriveProfile` composition pattern for vmux's Nix module
- sops-nix preStartHook pattern solves "how do agents get API keys"
- Proxy-based network allowlist (rather than iptables) is pragmatic
- The `customPrompt` field for injecting system prompts is relevant

**Worth depending on?** Not as a dependency. The profile composition model is worth studying for vmux's Nix module.

### 3. sandbox-shell (agentic-dev3o/sandbox-shell)

**What it does:** A lightweight Rust CLI (`sx`) that wraps commands in macOS Seatbelt sandbox profiles.

**CLI design:**
```
sx [profiles...] [flags] -- <command>
sx online rust -- cargo build      # stack profiles
sx --trace -- cargo build          # real-time violation log
sx --explain rust                  # show rules
sx --dry-run rust                  # preview profile
```

**What vmux can learn:**
- The `--trace`/`--explain`/`--dry-run` debugging triad is excellent UX
- Profile stacking is a composable pattern vmux should support
- The `--` separator cleanly divides sandbox config from the wrapped command

**Worth depending on?** No. macOS-only (Seatbelt). CLI design patterns are the valuable takeaway.

### 4. devenv Claude Code integration (devenv.sh/integrations/claude-code/)

**What it does:** A Nix module that generates `.claude/settings.json` with hooks, commands, agents, MCP servers, and permissions.

**Hook system:** Supports 17 hook types including:

| Hook Type | Trigger | Can Block? |
|---|---|---|
| `PreToolUse` | Before tool execution | Yes (exit 2 = deny) |
| `PostToolUse` | After tool execution | No |
| `SessionStart` / `SessionEnd` | Session lifecycle | No |

**Configuration structure:**
```nix
claude.code.hooks.<name> = {
  enable = true;
  hookType = "PreToolUse";
  matcher = "^(Edit|MultiEdit|Write)$";
  command = "...";
};
```

Also supports: `claude.code.permissions`, `claude.code.mcpServers`, `claude.code.agents`, `claude.code.commands`.

**What vmux can learn:**
- The hook system is the canonical way to intercept Claude Code's tool usage
- Nix module pattern (declare in `.nix`, generate `.claude/settings.json`) is exactly what vmux should do
- `mcpServers` configuration is how vmux would register its own MCP tools
- `matcher` regex pattern on tool names scopes hooks to specific operations

**Worth depending on?** Not as a code dependency, but vmux should integrate with devenv's module system.

## Decision

**No direct crate dependencies are warranted.** The value is in patterns:

1. **Workspace structure**: Follow microsandbox's crate split — separate `vmux-core`, `vmux-cli`, and (future) `vmux-mcp`.

2. **Nix integration**: Adopt bubblewrap-claude's `mkSandbox`/`deriveProfile` composition for vmux's Nix module. Use their sops-nix preStartHook pattern.

3. **CLI UX**: Borrow sandbox-shell's `--dry-run`/`--trace` debugging and profile stacking.

4. **Claude Code integration**: Use devenv's hook system (`PreToolUse`) to intercept tool calls. Generate `.claude/settings.json` from Nix. Register vmux's MCP tools via `mcpServers`.

5. **Future MCP**: microsandbox's HTTP MCP endpoint proves the pattern. vmux could expose VM operations as MCP tools.

## Unknowns / Risks

1. **devenv hook reliability**: Reports suggest `PreToolUse`/`PostToolUse` hooks sometimes fail to fire. If vmux's security model depends on these hooks, this is a reliability risk.

2. **libkrun vs cloud-hypervisor boot times**: microsandbox achieves sub-200ms with libkrun. cloud-hypervisor via microvm.nix may be slower — need to benchmark.

3. **MCP transport choice**: stdio vs HTTP. Stdio is simpler for local use; HTTP for remote/multi-user.

4. **Seatbelt deprecation**: Apple marks `sandbox-exec` as deprecated. vmux's VM-based approach is more future-proof than macOS sandboxing.

---

*Sources: [microsandbox](https://github.com/zerocore-ai/microsandbox), [bubblewrap-claude](https://github.com/matgawin/bubblewrap-claude), [sandbox-shell](https://github.com/agentic-dev3o/sandbox-shell), [devenv Claude Code](https://devenv.sh/integrations/claude-code/), [devenv claude.nix source](https://github.com/cachix/devenv/blob/main/src/modules/integrations/claude.nix)*
