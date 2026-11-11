#!/usr/bin/env bash
# Build meld and install it as the Meld_rs.app macOS application, exposing the
# meld-rs command-line entry point through a symlink into the app bundle.
#
# Usage: scripts/macos-install.sh [--binary PATH] [--no-build] [--app-dir DIR] [--bin-dir DIR]
#
#   --binary    package this existing binary instead of building (implies --no-build)
#   --no-build  skip 'cargo build --release' (requires --binary)
#   --app-dir   where to install Meld_rs.app (default: /Applications)
#   --bin-dir   where to create the meld-rs symlink (default: $HOME/bin)
#
# The ~/bin/meld-rs symlink points at the app launcher, so terminal and Finder
# launches share the same behaviour. An existing ~/bin/meld-rs file (for example
# a manually copied binary) is replaced by the symlink.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD=1
BINARY=""
APP_DIR="/Applications"
BIN_DIR="${HOME:?HOME is not set}/bin"

usage() { sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
	case "$1" in
	--binary)
		BINARY="$2"
		BUILD=0
		shift 2
		;;
	--no-build)
		BUILD=0
		shift
		;;
	--app-dir)
		APP_DIR="$2"
		shift 2
		;;
	--bin-dir)
		BIN_DIR="$2"
		shift 2
		;;
	-h | --help)
		usage
		exit 0
		;;
	*)
		echo "macos-install.sh: unknown argument '$1'" >&2
		exit 2
		;;
	esac
done

if [ "$BUILD" = 1 ]; then
	(cd "$ROOT" && cargo build --release --package meld)
	BINARY="$ROOT/target/release/meld"
elif [ -z "$BINARY" ]; then
	echo "macos-install.sh: --no-build requires --binary" >&2
	exit 2
fi

if [ ! -f "$BINARY" ]; then
	echo "macos-install.sh: binary not found: $BINARY" >&2
	exit 2
fi

if [ ! -d "$APP_DIR" ]; then
	mkdir -p "$APP_DIR" || {
		echo "macos-install.sh: cannot create $APP_DIR." >&2
		exit 1
	}
fi
if [ ! -w "$APP_DIR" ]; then
	echo "macos-install.sh: $APP_DIR is missing or not writable." >&2
	echo "Re-run with write access (for example: sudo $0) or pass --app-dir." >&2
	exit 1
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
"$ROOT/scripts/make-app.sh" --binary "$BINARY" --out "$STAGE"

rm -rf "${APP_DIR:?}/Meld_rs.app"
cp -R "$STAGE/Meld_rs.app" "$APP_DIR/"

mkdir -p "$BIN_DIR"
ln -sfn "$APP_DIR/Meld_rs.app/Contents/MacOS/Meld_rs" "$BIN_DIR/meld-rs"

echo "Installed: $APP_DIR/Meld_rs.app"
echo "CLI:       $BIN_DIR/meld-rs -> $APP_DIR/Meld_rs.app/Contents/MacOS/Meld_rs"
