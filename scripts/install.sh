#!/usr/bin/env bash
# Install Nexus CLI + engine to a fixed prefix (portable across directories).
# Usage:
#   ./scripts/install.sh
#   ./scripts/install.sh --prefix "$HOME/.local/nexus-ide"
#   ./scripts/install.sh --add-to-path

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local/nexus-ide}"
ADD_TO_PATH=0
SKIP_BUILD=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --prefix) PREFIX="$2"; shift 2 ;;
    --add-to-path) ADD_TO_PATH=1; shift ;;
    --skip-build) SKIP_BUILD=1; shift ;;
    *) echo "Unknown: $1"; exit 1 ;;
  esac
done

BIN_DIR="$PREFIX/bin"
ENGINE_DIR="$PREFIX/engine"

echo "NexusIDE install -> $PREFIX"

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  echo "Building nexus-cli (release)..."
  (cd "$ROOT" && cargo build -p nexus-cli --release)
fi

mkdir -p "$BIN_DIR" "$ENGINE_DIR"
cp "$ROOT/target/release/nexus" "$BIN_DIR/nexus"
cp "$ROOT/target/release/nexus" "$BIN_DIR/nx"
chmod +x "$BIN_DIR/nexus" "$BIN_DIR/nx"

echo "Copying engine..."
rsync -a --delete \
  --exclude .pytest_cache --exclude __pycache__ --exclude .venv \
  "$ROOT/packages/nexus-engine/" "$ENGINE_DIR/"

if command -v uv >/dev/null 2>&1; then
  echo "Syncing engine venv..."
  uv sync --directory "$ENGINE_DIR" --extra dev
else
  echo "WARNING: uv not found. Install uv then run:"
  echo "  uv sync --directory \"$ENGINE_DIR\" --extra dev"
fi

cat > "$BIN_DIR/nexus-wrap" <<EOF
#!/usr/bin/env bash
export NEXUS_ENGINE_DIR="$ENGINE_DIR"
exec "$BIN_DIR/nexus" "\$@"
EOF
chmod +x "$BIN_DIR/nexus-wrap"
ln -sf nexus-wrap "$BIN_DIR/nx-wrap" 2>/dev/null || cp "$BIN_DIR/nexus-wrap" "$BIN_DIR/nx-wrap"

cat > "$PREFIX/config.example.toml" <<'EOF'
engine_url = "http://127.0.0.1:8765"
default_model = "gpt-4o-mini"
EOF

echo ""
echo "Installed:"
echo "  CLI:    $BIN_DIR/nexus"
echo "  Engine: $ENGINE_DIR"
echo ""
echo "Run from any directory:"
echo "  $BIN_DIR/nexus-wrap"
echo "  # or: export PATH=\"$BIN_DIR:\$PATH\" && export NEXUS_ENGINE_DIR=\"$ENGINE_DIR\""

if [[ "$ADD_TO_PATH" -eq 1 ]]; then
  LINE="export PATH=\"$BIN_DIR:\$PATH\""
  LINE2="export NEXUS_ENGINE_DIR=\"$ENGINE_DIR\""
  for f in "$HOME/.bashrc" "$HOME/.zshrc"; do
    if [[ -f "$f" ]] && ! grep -q "NEXUS_ENGINE_DIR" "$f" 2>/dev/null; then
      echo "$LINE" >> "$f"
      echo "$LINE2" >> "$f"
      echo "Updated $f"
    fi
  done
fi
