#!/usr/bin/env bash
set -euo pipefail

repo_root="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
ui_dir="$repo_root/crates/brain-brew-workbench-ui"
embedded_dir="$repo_root/crates/brain-brew-cli/assets/workbench"
generated_dir="$(mktemp -d "${TMPDIR:-/tmp}/brainbrew-workbench-embed-check.XXXXXX")"

cleanup() {
  rm -rf "$generated_dir"
}
trap cleanup EXIT

(
  cd "$ui_dir"
  # Keep in sync with devenv.nix workbench-ui-embed. Leptos' wasm_split_helpers
  # hashes its absolute CARGO_MANIFEST_PATH into the wasm, so the registry must
  # live at the same absolute path on every builder.
  CARGO_HOME=/tmp/brainbrew-workbench-cargo-home \
    trunk build --release --dist "$generated_dir" --public-url /
)

if diff -ruN "$embedded_dir" "$generated_dir"; then
  echo "Workbench embedded release assets are fresh."
else
  cat >&2 <<EOF

Workbench embedded release assets are stale.
Regenerate them with:
  devenv shell workbench-ui-embed

Then commit the updated files under:
  crates/brain-brew-cli/assets/workbench
EOF
  exit 1
fi
