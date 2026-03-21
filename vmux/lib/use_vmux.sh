# vmux direnv stdlib — source this from your ~/.config/direnv/direnvrc
# or use `source_url` / `source_up` in your .envrc.
#
# Usage in .envrc:
#   use_vmux            # auto-detect vmux.toml
#   use_vmux ./vmux.toml  # explicit config path
#
# This function:
# 1. Finds vmux.toml in the project tree
# 2. Calls `vmux direnv` to resolve secrets and configuration
# 3. Exports VMUX_* metadata and resolved secrets into the direnv environment
# 4. Watches vmux.toml for changes (triggers re-eval on edit)

use_vmux() {
  local config_arg=""

  if [[ -n "${1:-}" ]]; then
    config_arg="--config $1"
    watch_file "$1"
  else
    # Find vmux.toml by walking up from PWD.
    local dir="$PWD"
    while [[ "$dir" != "/" ]]; do
      if [[ -f "$dir/vmux.toml" ]]; then
        watch_file "$dir/vmux.toml"
        config_arg="--config $dir/vmux.toml"
        break
      fi
      dir="$(dirname "$dir")"
    done
  fi

  if ! has vmux; then
    log_error "vmux not found in PATH. Install vmux or add it to your devShell."
    return 1
  fi

  local exports
  # shellcheck disable=SC2086
  exports="$(vmux direnv $config_arg 2>&1)"
  local rc=$?

  if [[ $rc -ne 0 ]]; then
    log_error "vmux direnv failed (exit $rc):"
    log_error "$exports"
    return 1
  fi

  # Separate warnings (stderr lines forwarded by vmux) from exports.
  local line
  while IFS= read -r line; do
    if [[ "$line" == export\ * ]]; then
      eval "$line"
    elif [[ -n "$line" ]]; then
      log_status "vmux: $line"
    fi
  done <<< "$exports"

  log_status "vmux environment loaded"
}
