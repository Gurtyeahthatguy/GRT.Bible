#!/usr/bin/env bash
# Puts a locally built GRT Bible in this user's application menu, without root.
#
#   ./scripts/install-local.sh            install
#   ./scripts/install-local.sh --remove   undo
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="$HERE/target/release/grt-bible"
LIB_DIR="$HOME/.local/lib/grt-bible"
BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor"
DESKTOP="$APP_DIR/org.grt.bible.desktop"

if [[ "${1:-}" == "--remove" ]]; then
  rm -rf "$LIB_DIR"
  rm -f "$BIN_DIR/grt-bible" "$DESKTOP" "$ICON_DIR"/*/apps/grt-bible.png
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" 2>/dev/null
  echo "Removed. Your notes are still in ${XDG_DATA_HOME:-$HOME/.local/share}/org.grt.bible/user.grt"
  exit 0
fi

if [[ ! -x "$BINARY" ]]; then
  echo "No release binary at $BINARY. Build it first: npm run build:binary" >&2
  exit 1
fi

mkdir -p "$LIB_DIR/catalog" "$BIN_DIR" "$APP_DIR"
install -m 755 "$BINARY" "$LIB_DIR/grt-bible"
rm -rf "$LIB_DIR/modules"
cp -r "$HERE/modules" "$LIB_DIR/modules"
install -m 644 "$HERE/catalog/videos.grt" "$LIB_DIR/catalog/videos.grt"
ln -sf "$LIB_DIR/grt-bible" "$BIN_DIR/grt-bible"

for size in 32x32 128x128; do
  mkdir -p "$ICON_DIR/$size/apps"
  install -m 644 "$HERE/src-tauri/icons/$size.png" "$ICON_DIR/$size/apps/grt-bible.png"
done

cat > "$DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Name=GRT Bible
GenericName=Bible reader
Comment=Read, search and annotate the Bible on this computer
Exec=$LIB_DIR/grt-bible
Icon=grt-bible
Terminal=false
Categories=Education;Literature;
StartupNotify=true
EOF
chmod 644 "$DESKTOP"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" 2>/dev/null
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$ICON_DIR" 2>/dev/null

echo "Installed in $LIB_DIR, with a menu entry. To undo: $0 --remove"
