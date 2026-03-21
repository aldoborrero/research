# claude-code-lsp-nix

A [Claude Code](https://docs.anthropic.com/en/docs/claude-code) plugin that provides Nix language intelligence via the [nixd](https://github.com/nix-community/nixd) language server.

When installed, Claude gets real-time diagnostics, go-to-definition, find-references, and hover information for `.nix` files.

## Prerequisites

- [Nix](https://nixos.org/) with flakes enabled
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code)

## Installation

### Option 1: Local plugin directory

Point Claude Code at the plugin directory:

```bash
claude --plugin-dir /path/to/claude-code-lsp-nix
```

### Option 2: Install to user scope

```bash
claude plugin install claude-code-lsp-nix
```

### Ensuring `nixd` is in PATH

The plugin configures how Claude Code talks to the `nixd` language server, but the binary must be available in your `$PATH`. You have several options:

**Using the bundled devshell** (recommended):

```bash
cd claude-code-lsp-nix
nix develop
```

This drops you into a shell with `nixd` available. You can then run `claude` from within that shell.

**Using the bundled package**:

```bash
nix shell github:aldoborrero/research#nixd
```

**Using nixpkgs directly**:

```bash
nix shell nixpkgs#nixd
```

**System-wide install** (NixOS / home-manager):

```nix
# configuration.nix or home.nix
environment.systemPackages = [ pkgs.nixd ];
```

## What it provides

### LSP capabilities

Once installed and with `nixd` in PATH, Claude Code gains:

- **Instant diagnostics** — syntax errors and warnings surface immediately after each edit
- **Go to definition** — navigate to where functions, variables, and attributes are defined
- **Find references** — locate all usages of a symbol across the codebase
- **Hover information** — type info and documentation for Nix expressions

### Flake outputs

This project uses [numtide/blueprint](https://github.com/numtide/blueprint) to map the directory structure to flake outputs:

| Directory | Flake output | Description |
|---|---|---|
| `packages/nixd/` | `packages.<system>.nixd` | The `nixd` binary |
| `devshells/` | `devShells.<system>.default` | Development shell with `nixd` |

## Project structure

```
claude-code-lsp-nix/
├── .claude-plugin/
│   └── plugin.json        # Plugin manifest
├── .lsp.json              # LSP server configuration
├── flake.nix              # Nix flake (numtide/blueprint)
├── packages/
│   └── nixd/
│       └── default.nix    # nixd package
├── devshells/
│   └── default.nix        # Dev shell with nixd
└── README.md
```

## Configuration

The `.lsp.json` configures `nixd` with default settings:

```json
{
  "nixd": {
    "command": "nixd",
    "args": [],
    "extensionToLanguage": {
      ".nix": "nix"
    }
  }
}
```

To customize `nixd` behavior (e.g., setting the nixpkgs path for option completion), you can add `initializationOptions` or `settings` fields. See the [nixd configuration docs](https://github.com/nix-community/nixd/blob/main/nixd/docs/configuration.md) for available options.

## License

MIT
