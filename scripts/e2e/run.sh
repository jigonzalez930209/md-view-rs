#!/usr/bin/env bash
# End-to-end UI and rendering test for MD View.
#
# Requires: Xvfb, ffmpeg, python3 with libX11/libXtst, curl.
# Run with: cargo xtask e2e   (or bash scripts/e2e/run.sh)
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
TMP=${MDVIEW_E2E_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/mdview-e2e.XXXXXX")}
WS="$TMP/workspace"
WEB="$TMP/web"
CONFIG="$TMP/config"
LOGS="$TMP/logs"
SHOTS="$ROOT/target/e2e"
DISPLAY=:97
PORT=8899
APP_PID=""
SERVER_PID=""
XVFB_PID=""

cleanup() {
  [[ -n "$APP_PID" ]] && kill "$APP_PID" 2>/dev/null || true
  [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" 2>/dev/null || true
  [[ -n "$XVFB_PID" ]] && kill "$XVFB_PID" 2>/dev/null || true
}
trap cleanup EXIT

fail() {
  echo "E2E FAIL: $*" >&2
  echo "Artifacts: $TMP" >&2
  exit 1
}

x() { DISPLAY="$DISPLAY" python3 "$ROOT/scripts/e2e/x11.py" "$@"; }
click() { x click "$1" "$2"; sleep 0.45; }
rclick() { x rclick "$1" "$2"; sleep 0.6; }
key() { x key "$1"; sleep 0.35; }
type_text() { x text "$1"; sleep 0.25; }
shot() {
  mkdir -p "$SHOTS"
  ffmpeg -y -loglevel error -f x11grab -video_size 1280x900 -i "$DISPLAY" \
    -frames:v 1 "$SHOTS/$1.png"
}
title() {
  DISPLAY="$DISPLAY" xwininfo -root -tree 2>/dev/null |
    sed -n 's/.*"MD View — \([^"]*\)".*/\1/p' | head -1
}
wait_title() {
  local expected=$1
  for _ in $(seq 1 140); do
    [[ "$(title)" == "$expected" ]] && return 0
    sleep 0.15
  done
  fail "expected window title '$expected', got '$(title)'"
}
wait_file() {
  local path=$1
  for _ in $(seq 1 140); do
    [[ -e "$path" ]] && return 0
    sleep 0.15
  done
  fail "expected file $path"
}
cfg() {
  awk -F' = ' -v key="$1" '$1 == key {gsub(/"/, "", $2); print $2}' \
    "$CONFIG/config.toml" | tail -1
}
expect_cfg() {
  local key=$1 expected=$2 actual
  actual=$(cfg "$key")
  [[ "$actual" == "$expected" ]] || fail "config $key expected '$expected', got '$actual'"
}
toggle_to() {
  local key=$1 desired=$2 x=$3 y=$4
  local attempt
  for attempt in 1 2 3; do
    [[ "$(cfg "$key")" == "$desired" ]] && return 0
    click "$x" "$y"
    for _ in $(seq 1 20); do
      [[ "$(cfg "$key")" == "$desired" ]] && return 0
      sleep 0.05
    done
  done
  fail "switch $key did not reach '$desired' (got '$(cfg "$key")')"
}
expand_folder() {
  local rel=$1 x=$2 y=$3 before after
  before=$(grep -c "explorer toggle path=$WS/$rel" "$LOGS/mdview.log" || true)
  for _ in 1 2 3; do
    click "$x" "$y"
    sleep 0.35
    after=$(grep -c "explorer toggle path=$WS/$rel" "$LOGS/mdview.log" || true)
    (( after > before )) && return 0
  done
  fail "folder $rel did not toggle"
}

expect_zoom() {
  local expected=$1 actual
  actual=$(cfg zoom)
  python3 - "$actual" "$expected" <<'PY' || fail "zoom expected $expected, got $actual"
import sys
raise SystemExit(0 if abs(float(sys.argv[1]) - float(sys.argv[2])) < 0.001 else 1)
PY
}
log_size() {
  local total=0 file
  for file in "$LOGS/mdview.log" "$LOGS/mdview.1.log"; do
    [[ -f "$file" ]] && total=$((total + $(stat -c %s "$file")))
  done
  echo "$total"
}
log_has() { grep -qE "$1" "$LOGS/mdview.log" || fail "log missing /$1/"; }

mkdir -p "$WS"/{assets,notes,docs,src,deep/nested} "$WEB" "$CONFIG" "$SHOTS"
cat > "$CONFIG/config.toml" <<'EOF'
schema_version = 1

[media]
remote_images = true
private_networks = true
animate_svg = true
reduce_motion = false
EOF
rm -rf "${SHOTS:?}"/*

cat > "$WS/showcase.md" <<EOF
# Render showcase

Inline math \$a^2 + b^2 = c^2\$ and a display integral:

\$\$
\\int_0^1 x^2\\,dx = \\frac{1}{3}
\$\$

## Table

| Feature | Local | Web | Status |
|---|---|---:|---|
| LaTeX | yes | yes | ready |
| Tables | yes | yes | ready |
| SVG | yes | yes | animated |
| Mermaid | yes | yes | ready |

## Local media

![Local PNG](assets/photo.png)

![Local SVG](assets/logo.svg)

![Animated local SVG](assets/pulse.svg)

## Web media

![Remote PNG](http://127.0.0.1:$PORT/photo.png)

![Remote SVG](http://127.0.0.1:$PORT/logo.svg)

![Remote animated SVG](http://127.0.0.1:$PORT/pulse.svg)

## Mermaid

\`\`\`mermaid
flowchart LR
    A[Open folder] --> B[Pick file]
    B --> C[Render Markdown]
    C --> D{All checks?}
    D -- yes --> E[Ship]
    D -- no --> B
\`\`\`

\`\`\`mermaid
sequenceDiagram
    participant U as User
    participant A as MD View
    U->>A: open showcase.md
    A-->>U: LaTeX + tables + media
\`\`\`

## Code and tasks

\`\`\`rust
fn main() {
    println!("all renderers");
}
\`\`\`

- [x] LaTeX
- [x] tables
- [x] local images
- [x] web images
- [x] Mermaid
EOF
printf '# Pixel lab\n\nWorkspace root for the E2E run.\n' > "$WS/README.md"
printf '# Guide\n' > "$WS/docs/guide.md"
printf '# Note one\n' > "$WS/notes/one.md"
printf '# Note two\n' > "$WS/notes/two.md"
printf 'fn main() {}\n' > "$WS/src/main.rs"
printf '# Leaf\n' > "$WS/deep/nested/leaf.md"
cp "$ROOT/crates/mdview-render/tests/snapshots/theme-github-light.png" "$WS/assets/photo.png"
cp "$ROOT/crates/mdview-render/tests/snapshots/theme-github-light.png" "$WEB/photo.png"
cat > "$WS/assets/logo.svg" <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="220" height="90" viewBox="0 0 220 90">
  <rect width="220" height="90" rx="14" fill="#0d4f8b"/>
  <circle cx="48" cy="45" r="24" fill="#7cc4ff"/>
  <path d="M36 45h24M48 33v24" stroke="#0d4f8b" stroke-width="6" stroke-linecap="round"/>
  <text x="86" y="53" font-family="sans-serif" font-size="20" fill="#ffffff">local svg</text>
</svg>
EOF
cat > "$WS/assets/pulse.svg" <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="180" height="80" viewBox="0 0 180 80">
  <rect width="180" height="80" rx="12" fill="#123b2a"/>
  <circle cx="90" cy="40" r="18" fill="#5ce1a0">
    <animate attributeName="r" values="12;22;12" dur="1.6s" repeatCount="indefinite"/>
  </circle>
</svg>
EOF
cp "$WS/assets/logo.svg" "$WEB/logo.svg"
cp "$WS/assets/pulse.svg" "$WEB/pulse.svg"

mkdir -p "$TMP/runtime"
chmod 700 "$TMP/runtime"

PROFILE=${MDVIEW_E2E_PROFILE:-debug}
echo "== build ($PROFILE) =="
if [[ "$PROFILE" == "release" ]]; then
  cargo build --release -p mdview-app --manifest-path "$ROOT/Cargo.toml"
else
  cargo build -p mdview-app --manifest-path "$ROOT/Cargo.toml"
fi
BIN="$ROOT/target/$PROFILE/mdview"

echo "== web server =="
python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$WEB" >"$TMP/web.log" 2>&1 &
SERVER_PID=$!
for _ in $(seq 1 40); do
  curl -fsS "http://127.0.0.1:$PORT/photo.png" -o /dev/null && break
  sleep 0.1
done

echo "== display =="
Xvfb "$DISPLAY" -screen 0 1280x900x24 >"$TMP/xvfb.log" 2>&1 &
XVFB_PID=$!
sleep 0.7

echo "== launch =="
env -u WAYLAND_DISPLAY XDG_SESSION_TYPE=x11 DISPLAY="$DISPLAY" USER=mdview_e2e \
  XDG_RUNTIME_DIR="$TMP/runtime" MDVIEW_CONFIG_DIR="$CONFIG" MDVIEW_LOG_DIR="$LOGS" \
  RUST_LOG=warn,mdview=debug "$BIN" "$WS" "$WS/showcase.md" \
  >"$TMP/app.log" 2>&1 &
APP_PID=$!
wait_title "showcase.md"
WID=$(DISPLAY="$DISPLAY" xwininfo -root -tree | awk '/MD View/{print $1; exit}')
DISPLAY="$DISPLAY" python3 "$ROOT/scripts/e2e/x11.py" focus "$WID"
sleep 0.4
shot "01-initial"
log_has "startup version="
log_has "workspace ready"
log_has "file open.*showcase.md"

echo "== title menus =="
for menu in "File 57" "Edit 94" "View 138" "Go 174" "Help 213"; do
  set -- $menu
  click "$2" 16
  shot "02-menu-$1"
  key Escape
done

echo "== settings: open and close paths =="
click 850 16
shot "03-settings-appearance"
click 240 224
shot "04-settings-editor"
toggle_to wrap true 767 243
toggle_to wrap false 767 243
toggle_to line_numbers false 767 296
toggle_to line_numbers true 767 296
toggle_to sync_scroll false 767 349
toggle_to sync_scroll true 767 349
click 240 252
shot "05-settings-explorer"
toggle_to show_hidden true 767 242
toggle_to show_hidden false 767 242
toggle_to respect_gitignore false 767 296
toggle_to respect_gitignore true 767 296
click 240 280
shot "06-settings-media"
toggle_to remote_images false 767 242
toggle_to remote_images true 767 242
toggle_to private_networks false 767 296
toggle_to private_networks true 767 296
toggle_to animate_svg false 767 348
toggle_to animate_svg true 767 348
toggle_to reduce_motion true 767 396
toggle_to reduce_motion false 767 396
click 240 308
shot "07-settings-about"
click 756 576
expect_cfg private_networks true
shot "07b-after-settings"

echo "== explorer: expand, context menu, rename, delete, create =="
expand_folder notes 120 203
shot "08-explorer-expanded"
rclick 120 203
shot "08b-context-menu"
key Escape
click 115 237
wait_title "one.md"
key ctrl+shift+p
type_text "Rename File"
key Enter
key ctrl+a
type_text "one-e2e.md"
key Return
wait_file "$WS/notes/one-e2e.md"
wait_title "one-e2e.md"
key ctrl+shift+p
type_text "Rename File"
key Enter
key ctrl+a
type_text "one.md"
key Return
wait_file "$WS/notes/one.md"
key ctrl+p
shot "08c-quickopen"
type_text "two.md"
shot "08d-quickopen-typed"
key Enter
wait_title "two.md"
key ctrl+shift+p
type_text "Delete File"
key Enter
click 507 408
[[ -f "$WS/notes/two.md" ]] || fail "delete cancel removed two.md"
key ctrl+shift+p
type_text "Delete File"
key Enter
click 615 408
for _ in $(seq 1 40); do
  [[ ! -f "$WS/notes/two.md" ]] && break
  sleep 0.1
done
[[ ! -f "$WS/notes/two.md" ]] || fail "delete did not remove two.md"
shot "09-deleted"

click 193 85
key Escape
[[ ! -f "$WS/untitled.md" ]] || fail "escape created untitled.md"
click 193 85
click 572 436
[[ ! -f "$WS/untitled.md" ]] || fail "cancel created untitled.md"
click 193 85
click 679 292
[[ ! -f "$WS/untitled.md" ]] || fail "close created untitled.md"
click 193 85
key ctrl+a
type_text "e2e-created.md"
key Return
wait_file "$WS/e2e-created.md"
wait_title "e2e-created.md"
click 222 85
key ctrl+a
type_text "e2e-folder"
key Return
wait_file "$WS/e2e-folder"
shot "10-created"
click 280 85
shot "11-collapsed"
click 251 85

echo "== palette, tabs, quick open =="
key ctrl+shift+p
type_text "settings"
shot "12-palette"
key Escape
key ctrl+p
type_text "showcase"
key Enter
wait_title "showcase.md"
click 903 50
shot "13-tab-menu"
click 750 117
[[ "$(title)" == "showcase.md" ]] || fail "close other tabs changed active title"
click 875 50
wait_title "Untitled"
key ctrl+w
wait_title "showcase.md"

echo "== editor, find, replace, goto, save =="
key ctrl+e
click 640 400
type_text "E2E "
key ctrl+s
sleep 0.7
grep -q "E2E" "$WS/showcase.md" || fail "Ctrl+S did not save the edit"
key ctrl+f
type_text "E2E"
shot "14-find"
key Escape
key ctrl+h
shot "15-replace"
key Escape
key ctrl+g
key ctrl+a
type_text "10"
shot "16-goto"
click 593 409
shot "17-goto-result"

echo "== split and scroll stress =="
key ctrl+shift+e
shot "18-split"
x wheel 320 400 60
x wheel 700 400 60
sleep 2
shot "19-scroll-stress"
key ctrl+e
key ctrl+e
wait_title "showcase.md"

echo "== render pass =="
for _ in $(seq 1 40); do x wheelup 640 400 2; done
sleep 1
shot "20-render-top"
x wheel 640 400 5
sleep 1
shot "21-render-media"
x wheel 640 400 4
sleep 2
shot "22-render-web"
x wheel 640 400 5
sleep 2
shot "23-render-mermaid"
x wheel 640 400 6
sleep 1
shot "24-render-tasks"

echo "== shutdown =="
click 958 16
for _ in $(seq 1 140); do
  kill -0 "$APP_PID" 2>/dev/null || break
  sleep 0.1
done
if kill -0 "$APP_PID" 2>/dev/null; then fail "application did not close on Alt+F4"; fi
APP_PID=""

echo "== logs =="
[[ -f "$LOGS/mdview.log" ]] || fail "log file missing"
size=$(log_size)
(( size > 0 )) || fail "log file empty"
(( size <= 1024 * 1024 )) || fail "logs exceed 1 MiB: $size"
log_has "file save.*showcase.md"
log_has "command SetMode"
log_has "perf frames="
grep -q "panicked" "$LOGS/mdview.log" && fail "panic in log"
grep -q "media load failed" "$LOGS/mdview.log" && fail "media failed to load"
log_has "media load"
log_has "source=http://127.0.0.1:8899/photo.png"
log_has "source=http://127.0.0.1:8899/logo.svg"
log_has "source=http://127.0.0.1:8899/pulse.svg"
grep 'perf frames=' "$LOGS/mdview.log" | tail -3 | grep -v 'over_8.33ms=0' && \
  fail "final frame windows exceeded the 120 Hz budget"

echo
echo "E2E PASS"
echo "  log bytes: $size (limit 1048576)"
echo "  screenshots: $SHOTS"
echo "  logs: $LOGS"
echo "  workspace: $WS"
echo
tail -n 8 "$LOGS/mdview.log"
