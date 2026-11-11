#!/usr/bin/env bash
# Tests for the macOS app bundle packaging scripts (scripts/make-app.sh and
# scripts/macos-install.sh). Runs fully isolated in a temp directory: it never
# touches /Applications or the real $HOME/bin.
#
# Usage: bash scripts/test-macos-bundle.sh
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MAKE_APP="$ROOT/scripts/make-app.sh"
INSTALL="$ROOT/scripts/macos-install.sh"
ICON="$ROOT/data/meld_rs.icns"
PLISTBUDDY=/usr/libexec/PlistBuddy

if [ ! -x "$PLISTBUDDY" ]; then
	echo "SKIP - these tests require macOS ($PLISTBUDDY not found)"
	exit 1
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

FAIL=0
TESTS=0
ok() {
	TESTS=$((TESTS + 1))
	echo "ok   - $1"
}
bad() {
	TESTS=$((TESTS + 1))
	FAIL=$((FAIL + 1))
	echo "FAIL - $1"
}

check() { # check <description> <command...>
	local desc="$1"
	shift
	if "$@" >/dev/null 2>&1; then ok "$desc"; else bad "$desc"; fi
}

eq() { # eq <description> <expected> <actual>
	if [ "$2" = "$3" ]; then ok "$1"; else bad "$1 (expected '$2', got '$3')"; fi
}

plist_get() { "$PLISTBUDDY" -c "Print $1" "$2" 2>/dev/null; }

# ---------------------------------------------------------------------------
# Stub binary: records its argv, PATH and PWD so the launcher can be tested
# without building the real meld.
# ---------------------------------------------------------------------------
STUB="$WORK/meld-stub"
cat >"$STUB" <<'EOF'
#!/usr/bin/env bash
{
  printf 'ARG:%s\n' "$@"
  echo "PATH:$PATH"
  echo "PWD:$PWD"
} > "${STUB_OUT:?STUB_OUT not set}"
EOF
chmod +x "$STUB"

# ---------------------------------------------------------------------------
# make-app.sh: bundle structure, Info.plist identity, icon, version
# ---------------------------------------------------------------------------
OUT="$WORK/out"
bash "$MAKE_APP" --binary "$STUB" --out "$OUT" >/dev/null 2>&1
APP="$OUT/Meld_rs.app"
PLIST="$APP/Contents/Info.plist"

check "make-app creates Meld_rs.app" test -d "$APP"
check "launcher is executable" test -x "$APP/Contents/MacOS/Meld_rs"
check "real binary is executable" test -x "$APP/Contents/MacOS/meld-rs"
check "icon resource present" test -f "$APP/Contents/Resources/meld_rs.icns"
check "icon matches data/meld_rs.icns" cmp -s "$ICON" "$APP/Contents/Resources/meld_rs.icns"
check "Info.plist passes plutil -lint" plutil -lint "$PLIST"

eq "CFBundleName is 'Meld RS'" "Meld RS" "$(plist_get :CFBundleName "$PLIST")"
eq "CFBundleDisplayName is 'Meld RS'" "Meld RS" "$(plist_get :CFBundleDisplayName "$PLIST")"
eq "CFBundleIdentifier is org.gnome.Meld.Rust" "org.gnome.Meld.Rust" "$(plist_get :CFBundleIdentifier "$PLIST")"
eq "CFBundleExecutable is Meld_rs" "Meld_rs" "$(plist_get :CFBundleExecutable "$PLIST")"
eq "version derives from Cargo.toml (4.0.0-alpha.1 -> 4.0.0)" "4.0.0" "$(plist_get :CFBundleShortVersionString "$PLIST")"

DT="$(plist_get :CFBundleDocumentTypes "$PLIST")"
case "$DT" in
*public.data*) ok "document types cover files (public.data)" ;;
*) bad "document types cover files (public.data)" ;;
esac
case "$DT" in
*public.folder*) ok "document types cover folders (public.folder)" ;;
*) bad "document types cover folders (public.folder)" ;;
esac

check "scripts pass bash -n (make-app)" bash -n "$MAKE_APP"
check "scripts pass bash -n (install)" bash -n "$INSTALL"

# ---------------------------------------------------------------------------
# Launcher: PATH from ~/.bashrc, fallback dirs, argv forwarding, CWD restore
# ---------------------------------------------------------------------------
FAKEHOME="$WORK/home"
mkdir -p "$FAKEHOME/fakebin"
# .bashrc moves the CWD and prepends a marker dir to PATH on purpose.
cat >"$FAKEHOME/.bashrc" <<'EOF'
cd /
export PATH="$HOME/fakebin:$PATH"
EOF

ORIGCWD="$WORK/origcwd"
mkdir -p "$ORIGCWD"
STUB_OUT="$WORK/launcher-run.txt"
export STUB_OUT
(
	cd "$ORIGCWD" || exit 1
	HOME="$FAKEHOME" "$APP/Contents/MacOS/Meld_rs" "file one" "file two" >/dev/null 2>&1
)

check "launcher forwards arguments (incl. spaces)" grep -Fx 'ARG:file one' "$STUB_OUT"
check "launcher forwards a second argument" grep -Fx 'ARG:file two' "$STUB_OUT"
if grep -q 'fakebin' "$STUB_OUT"; then ok "PATH marker dir reaches the binary"; else bad "PATH marker dir reaches the binary"; fi
if grep -q "PWD:$ORIGCWD" "$STUB_OUT"; then ok "launcher restores the original CWD"; else bad "launcher restores the original CWD"; fi

# Without ~/.bashrc the launcher must still add the fallback tool dirs.
NOHOME="$WORK/nohome"
mkdir -p "$NOHOME"
STUB_OUT="$WORK/launcher-nobashrc.txt"
HOME="$NOHOME" "$APP/Contents/MacOS/Meld_rs" >/dev/null 2>&1
if grep -q '/opt/homebrew/bin' "$STUB_OUT"; then ok "launcher adds /opt/homebrew/bin fallback"; else bad "launcher adds /opt/homebrew/bin fallback"; fi
if grep -q "$NOHOME/bin" "$STUB_OUT"; then ok "launcher adds ~/bin fallback"; else bad "launcher adds ~/bin fallback"; fi

# Invoking the launcher through an external symlink (the ~/bin/meld-rs install
# shape) must run the real binary. Regression guard: the launcher used to
# derive its own directory from $0, so via a symlink it exec'ed itself in an
# infinite loop and the app never started.
LINKDIR="$WORK/linkdir"
mkdir -p "$LINKDIR"
ln -s "$APP/Contents/MacOS/Meld_rs" "$LINKDIR/meld-rs"
STUB_OUT="$WORK/via-symlink.txt"
("$LINKDIR/meld-rs" arg1 >/dev/null 2>&1) &
LK=$!
sleep 3
if kill -0 "$LK" 2>/dev/null; then
	kill "$LK" 2>/dev/null
	wait "$LK" 2>/dev/null
	bad "launcher terminates when invoked via external symlink"
else
	wait "$LK" 2>/dev/null
	ok "launcher terminates when invoked via external symlink"
fi
if [ -s "$STUB_OUT" ] && grep -Fx 'ARG:arg1' "$STUB_OUT" >/dev/null; then
	ok "real binary runs with args via external symlink"
else
	bad "real binary runs with args via external symlink"
fi

# ---------------------------------------------------------------------------
# macos-install.sh: installs the app, replaces the old ~/bin copy with a
# symlink, is idempotent, and honours --no-build (never calls cargo).
# ---------------------------------------------------------------------------
APPDIR="$WORK/Applications"
BINDIR="$WORK/home2/bin"
mkdir -p "$BINDIR"
echo "old manual copy" >"$BINDIR/meld-rs"

FAKEBIN="$WORK/fakebin"
mkdir -p "$FAKEBIN"
cat >"$FAKEBIN/cargo" <<EOF
#!/usr/bin/env bash
echo "cargo must not run with --no-build" >> "$WORK/cargo-called.txt"
exit 1
EOF
chmod +x "$FAKEBIN/cargo"

PATH="$FAKEBIN:$PATH" bash "$INSTALL" --no-build --binary "$STUB" \
	--app-dir "$APPDIR" --bin-dir "$BINDIR" >/dev/null 2>&1
check "install exits successfully" test -d "$APPDIR/Meld_rs.app"
check "install places the launcher in app" test -x "$APPDIR/Meld_rs.app/Contents/MacOS/Meld_rs"
check "install creates the CLI symlink" test -L "$BINDIR/meld-rs"
eq "symlink points at the installed launcher" \
	"$APPDIR/Meld_rs.app/Contents/MacOS/Meld_rs" "$(readlink "$BINDIR/meld-rs")"
check "--no-build never calls cargo" test ! -e "$WORK/cargo-called.txt"

PATH="$FAKEBIN:$PATH" bash "$INSTALL" --no-build --binary "$STUB" \
	--app-dir "$APPDIR" --bin-dir "$BINDIR" >/dev/null 2>&1
check "install is idempotent (second run)" test -L "$BINDIR/meld-rs"
check "--no-build never calls cargo (2nd)" test ! -e "$WORK/cargo-called.txt"

echo
echo "$TESTS checks run, failures: $FAIL"
exit "$FAIL"
