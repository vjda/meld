#!/usr/bin/env bash
# Assemble the Meld_rs.app macOS application bundle from a built meld binary.
#
# Usage: scripts/make-app.sh --binary PATH [--out DIR] [--version X.Y.Z]
#
#   --binary    built meld binary to embed (required)
#   --out       directory that will contain Meld_rs.app (default: target/release/bundle)
#   --version   version override (default: derived from the workspace Cargo.toml)
#
# The bundle is self-contained: meld embeds its styles, icons and translations
# at compile time, so only the binary, the dock icon and Info.plist are needed.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BINARY=""
OUT="$ROOT/target/release/bundle"
VERSION=""

usage() { sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
	case "$1" in
	--binary)
		BINARY="$2"
		shift 2
		;;
	--out)
		OUT="$2"
		shift 2
		;;
	--version)
		VERSION="$2"
		shift 2
		;;
	-h | --help)
		usage
		exit 0
		;;
	*)
		echo "make-app.sh: unknown argument '$1'" >&2
		exit 2
		;;
	esac
done

if [ -z "$BINARY" ]; then
	echo "make-app.sh: --binary is required" >&2
	exit 2
fi
if [ ! -f "$BINARY" ]; then
	echo "make-app.sh: binary not found: $BINARY" >&2
	exit 2
fi
if [ ! -f "$ROOT/data/meld_rs.icns" ]; then
	echo "make-app.sh: icon not found: $ROOT/data/meld_rs.icns" >&2
	exit 2
fi

if [ -z "$VERSION" ]; then
	VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
fi
if [ -z "$VERSION" ]; then
	echo "make-app.sh: cannot derive version from $ROOT/Cargo.toml" >&2
	exit 2
fi
# CFBundleShortVersionString must be numeric (x.y.z): strip pre-release/build suffix.
SHORT_VERSION="${VERSION%%-*}"
SHORT_VERSION="${SHORT_VERSION%%+*}"

APP="$OUT/Meld_rs.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BINARY" "$APP/Contents/MacOS/meld-rs"
chmod +x "$APP/Contents/MacOS/meld-rs"
cp "$ROOT/data/meld_rs.icns" "$APP/Contents/Resources/meld_rs.icns"

cat >"$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>Meld RS</string>
  <key>CFBundleDisplayName</key>
  <string>Meld RS</string>
  <key>CFBundleIdentifier</key>
  <string>org.gnome.Meld.Rust</string>
  <key>CFBundleExecutable</key>
  <string>Meld_rs</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleIconFile</key>
  <string>meld_rs</string>
  <key>CFBundleShortVersionString</key>
  <string>$SHORT_VERSION</string>
  <key>CFBundleVersion</key>
  <string>$SHORT_VERSION</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.developer-tools</string>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key>
      <string>Files and folders to compare</string>
      <key>CFBundleTypeRole</key>
      <string>Editor</string>
      <key>LSHandlerRank</key>
      <string>Alternate</string>
      <key>CFBundleTypeContentTypes</key>
      <array>
        <string>public.data</string>
        <string>public.folder</string>
      </array>
    </dict>
  </array>
</dict>
</plist>
EOF

# Launcher: Finder-launched GUIs inherit launchd's minimal PATH, which lacks
# Homebrew tools (gh, glab, claude) that meld invokes at runtime. Resolve PATH
# from the user's ~/.bashrc, append fallback tool dirs, restore the working
# directory the launcher was started from (sourcing .bashrc may change it) and
# exec the real binary so argv and signals pass through unchanged.
cat >"$APP/Contents/MacOS/Meld_rs" <<'EOF'
#!/usr/bin/env bash
# Resolve this script's real directory through any symlink chain: deriving it
# from $0 alone made the launcher exec itself forever when invoked through an
# external symlink such as ~/bin/meld-rs (the install shape).
SELF="$0"
while [ -L "$SELF" ]; do
  TARGET="$(readlink "$SELF")"
  case "$TARGET" in
    /*) SELF="$TARGET" ;;
    *) SELF="$(dirname "$SELF")/$TARGET" ;;
  esac
done
HERE="$(cd "$(dirname "$SELF")" && pwd)"
ORIG_PWD="$(pwd)"
if [ -f "$HOME/.bashrc" ]; then
  # shellcheck disable=SC1090,SC1091
  . "$HOME/.bashrc" >/dev/null 2>&1
fi
for dir in /opt/homebrew/bin /usr/local/bin "$HOME/bin" "$HOME/.local/bin"; do
  case ":$PATH:" in
    *":$dir:"*) ;;
    *) PATH="$PATH:$dir" ;;
  esac
done
export PATH
cd "$ORIG_PWD" 2>/dev/null || true
exec "$HERE/meld-rs" "$@"
EOF
chmod +x "$APP/Contents/MacOS/Meld_rs"

echo "Built $APP (version $SHORT_VERSION)"
