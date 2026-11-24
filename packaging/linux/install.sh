#!/bin/sh
# Install Wordsmith Manuscript for the current user from the unpacked release folder:
# the programs, the menu entry, the icon and the project file types.
set -e
here=$(cd "$(dirname "$0")" && pwd)
bin="${XDG_BIN_HOME:-$HOME/.local/bin}"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
mkdir -p "$bin" "$data/applications" "$data/mime/packages" "$data/icons/hicolor/256x256/apps"
cp "$here/wordsmith-manuscript" "$here/manuscript-cli" "$bin/"
cp "$here/org.jcjfoundation.manuscript.desktop" "$data/applications/"
cp "$here/org.jcjfoundation.manuscript.xml" "$data/mime/packages/"
cp "$here/icon-256.png" "$data/icons/hicolor/256x256/apps/org.jcjfoundation.manuscript.png"
update-mime-database "$data/mime" 2>/dev/null || true
update-desktop-database "$data/applications" 2>/dev/null || true
echo "Wordsmith Manuscript is installed in $bin."
