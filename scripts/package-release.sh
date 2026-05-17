#!/usr/bin/env bash
# Build production distribution archive for Nexus-Transe.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${VERSION:-$(tr -d ' \r\n' < "$ROOT/VERSION")}"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64) ARCH="x64" ;;
  aarch64|arm64) ARCH="arm64" ;;
esac
NAME="Nexus-Transe-${VERSION}-${OS}-${ARCH}"
DIST="$ROOT/dist"
STAGE="$DIST/$NAME"

echo "Nexus-Transe production package v$VERSION"

if [[ "${SKIP_TESTS:-0}" != "1" ]]; then
  (cd "$ROOT" && cargo test -p nexus-core -q && cargo test -p nexus-cli -q)
fi

(cd "$ROOT" && cargo build -p nexus-cli --release)
command -v uv >/dev/null || { echo "uv required: https://docs.astral.sh/uv/"; exit 1; }

rm -rf "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/engine" "$STAGE/docs" "$STAGE/assets"

cp "$ROOT/target/release/nexus" "$STAGE/bin/nexus"
cp "$STAGE/bin/nexus" "$STAGE/bin/nx"
chmod +x "$STAGE/bin/nexus" "$STAGE/bin/nx"

rsync -a --delete \
  --exclude .pytest_cache --exclude __pycache__ --exclude .venv --exclude dist \
  "$ROOT/packages/nexus-engine/" "$STAGE/engine/"

echo "Syncing engine venv..."
uv sync --directory "$STAGE/engine" --extra dev

cat > "$STAGE/bin/nexus-wrap" <<EOF
#!/usr/bin/env bash
export NEXUS_ENGINE_DIR="$STAGE/engine"
exec "\$(dirname "\$0")/nexus" "\$@"
EOF
chmod +x "$STAGE/bin/nexus-wrap"
ln -sf nexus-wrap "$STAGE/bin/nx-wrap"

cp "$ROOT/LICENSE" "$ROOT/NOTICE" "$ROOT/README.md" "$ROOT/VERSION" "$ROOT/CHANGELOG.md" "$STAGE/" 2>/dev/null || true
cp "$ROOT/assets/logo.png" "$STAGE/assets/"
cp "$ROOT/docs/"{INSTALL,CLI,DISTRIBUTION,PRODUCTION,RELEASE,known-issues}.md "$STAGE/docs/" 2>/dev/null || true

cat > "$STAGE/config.example.toml" <<'EOF'
engine_url = "http://127.0.0.1:8765"
default_model = "gpt-4o-mini"
EOF

mkdir -p "$DIST"
ARCHIVE="$DIST/${NAME}.tar.gz"
tar -czf "$ARCHIVE" -C "$DIST" "$NAME"
echo ""
echo "Package ready: $ARCHIVE"
echo "  Run: $STAGE/bin/nexus-wrap"
