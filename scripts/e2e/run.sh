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
DISPLAY=${MDVIEW_E2E_DISPLAY:-:97}
PORT=8899
APP_PID=""
SERVER_PID=""
XVFB_PID=""

mkdir -p "$TMP"
# shellcheck source=lifecycle.sh
source "$ROOT/scripts/e2e/lifecycle.sh"
reap_stale

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
  for _ in $(seq 1 40); do
    actual=$(cfg "$key")
    [[ "$actual" == "$expected" ]] && return 0
    sleep 0.05
  done
  fail "config $key expected '$expected', got '$actual'"
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
set_mode() {
  local mode=$1
  key ctrl+shift+p
  type_text "$mode"
  key Enter
  sleep 0.5
}
set_wrap() {
  local desired=$1 current attempt
  for attempt in 1 2 3 4; do
    current=$(cfg wrap)
    [[ -z "$current" ]] && current=false
    [[ "$current" == "$desired" ]] && return 0
    set_mode "Word wrap"
  done
  fail "word wrap did not reach '$desired'"
}
assert_end_stable() {
  python3 - "$LOGS/mdview.log" "$1" "$2" <<'PY' || fail "editor end offsets are unstable for $1 wrap=$2"
import re
import sys
text = open(sys.argv[1], encoding="utf-8").read()
mode, wrap = sys.argv[2], sys.argv[3]
values = [
    tuple(map(float, match))
    for match in re.findall(
        rf"editor scroll end mode={re.escape(mode)} wrap={re.escape(wrap)} offset=([0-9.]+) max=([0-9.]+) clearance=([0-9.]+)",
        text,
    )
]
if len(values) < 2:
    raise SystemExit(f"only {len(values)} end diagnostics for {mode} wrap={wrap}")
recent = values[-3:]
offsets = [offset for offset, _, _ in recent[-2:]]
clearances = [clearance for _, _, clearance in recent]
if max(offsets) - min(offsets) > 0.1:
    raise SystemExit(f"unstable offsets: {recent}")
if min(clearances) < 6.0:
    raise SystemExit(f"scrollbar clearance below 6 px: {recent}")
PY
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

printf "# Format\n\n" > "$WS/src/format.md"
printf 'heading\nitalic\nunderline\nquote\ncode\nlink\nbullets\nnumbered\ntask\n' > "$WS/src/toolbar.md"
printf 'bold\nitalic\nunderline\nhone\nhtwo\nhthree\n## normal\nquote\ncode\nlink\nbullets\nnumbered\ntask\nend\n' > "$WS/src/palette.md"
printf '# Interact\n\nalpha_beta gamma\na=1\nb=22\nf(x[y])\n\n%s\n' "$(printf 'word %.0s' $(seq 1 40))" > "$WS/src/interact.md"
python3 -c "import sys; open(sys.argv[1], 'w').write(''.join(f'line {i} of the large file\\n' for i in range(210000)))" "$WS/src/big.txt"
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
| [Open guide](docs/guide.md) | table link | ready | ready |

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
for index in 1 2 3 4; do
  printf '# Extra %s\n\nAnother document for tab scrolling.\n' "$index" > "$WS/extra$index.md"
done
printf 'fn main() {}\n' > "$WS/src/main.rs"
printf 'fn segmented() {}\n' > "$WS/src/segmented.rs"
mkdir -p "$WS/deep/nested/three/four/five"
printf '# Leaf\n' > "$WS/deep/nested/leaf.md"
printf '# Level three\n' > "$WS/deep/nested/three/three.md"
printf '# Level four\n' > "$WS/deep/nested/three/four/four.md"
printf '# Level five\n' > "$WS/deep/nested/three/four/five/final.md"
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
register_pid "$SERVER_PID" "http.server $PORT"
for _ in $(seq 1 40); do
  curl -fsS "http://127.0.0.1:$PORT/photo.png" -o /dev/null && break
  sleep 0.1
done

echo "== display =="
if ! xdpyinfo -display "$DISPLAY" >/dev/null 2>&1; then
  Xvfb "$DISPLAY" -screen 0 1280x900x24 >"$TMP/xvfb.log" 2>&1 &
  XVFB_PID=$!
  register_pid "$XVFB_PID" "Xvfb $DISPLAY"
  sleep 0.7
fi

echo "== launch =="
# A recording stand-in for the desktop opener: Reveal must ask for the right folder, and the
# run must not start a real file manager.
mkdir -p "$TMP/bin"
printf '#!/bin/sh\nprintf "%%s\\n" "$*" >> "%s/opened.log"\n' "$TMP" > "$TMP/bin/xdg-open"
chmod +x "$TMP/bin/xdg-open"
env -u WAYLAND_DISPLAY XDG_SESSION_TYPE=x11 DISPLAY="$DISPLAY" USER=mdview_e2e PATH="$TMP/bin:$PATH" \
  XDG_RUNTIME_DIR="$TMP/runtime" MDVIEW_CONFIG_DIR="$CONFIG" MDVIEW_LOG_DIR="$LOGS" \
  RUST_LOG=warn,mdview=debug "$BIN" "$WS" "$WS/showcase.md" \
  >"$TMP/app.log" 2>&1 &
APP_PID=$!
register_pid "$APP_PID" "$BIN"
start_ram_guard "$APP_PID"
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
click 240 252
shot "03b-settings-reading"
x drag 705 243 665 243
sleep 0.4
expect_cfg margin_percent 5.0
shot "03c-settings-reading-wide"
x drag 665 243 805 243
sleep 0.4
expect_cfg margin_percent 40.0
shot "03d-settings-reading-narrow"
x drag 780 243 702 243
sleep 0.4
expect_cfg margin_percent 15.0
click 773 286
sleep 0.4
expect_cfg body_size 14.5
click 773 300
sleep 0.4
expect_cfg body_size 14.0
click 773 485
sleep 0.4
expect_cfg math_size 20.5
click 773 500
sleep 0.4
expect_cfg math_size 20.0
click 240 224
shot "04-settings-editor"
toggle_to wrap true 767 243
toggle_to wrap false 767 243
toggle_to line_numbers false 767 296
toggle_to line_numbers true 767 296
toggle_to sync_scroll false 767 349
toggle_to sync_scroll true 767 349
toggle_to scroll_beyond_last_line false 767 506
toggle_to scroll_beyond_last_line true 767 506
click 771 398
sleep 0.4
expect_cfg font_size 13.5
click 771 412
sleep 0.4
expect_cfg font_size 13.0
click 240 280
shot "05-settings-explorer"
toggle_to show_hidden true 767 242
toggle_to show_hidden false 767 242
toggle_to respect_gitignore false 767 296
toggle_to respect_gitignore true 767 296
click 240 308
shot "06-settings-media"
toggle_to remote_images false 767 242
toggle_to remote_images true 767 242
toggle_to private_networks false 767 296
toggle_to private_networks true 767 296
toggle_to animate_svg false 767 348
toggle_to animate_svg true 767 348
toggle_to reduce_motion true 767 396
toggle_to reduce_motion false 767 396
click 240 336
shot "07-settings-about"
click 756 576
expect_cfg private_networks true
shot "07b-after-settings"

echo "== explorer: five nested folders survive refreshes =="
# deep is the second root row (assets, deep, docs, ...). Expanding each level must survive
# the debounced watcher refresh triggered by a change in the workspace root.
expand_folder deep 120 105
expand_folder deep/nested 120 129
expand_folder deep/nested/three 120 153
expand_folder deep/nested/three/four 120 177
expand_folder deep/nested/three/four/five 120 201
touch "$WS/README.md"
sleep 1.4
shot "08a-nested-folders"
click 200 225
wait_title "final.md"
click 120 105
sleep 0.5

echo "== explorer: expand, context menu, rename, delete, create =="
expand_folder notes 120 153
shot "08-explorer-expanded"
rclick 120 153
shot "08b-context-menu"
log_has "explorer context menu path="
key Escape
click 115 177
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
sleep 0.6
shot "09-deleted"

click 158 48
key Escape
[[ ! -f "$WS/untitled.md" ]] || fail "escape created untitled.md"
click 158 48
click 572 436
[[ ! -f "$WS/untitled.md" ]] || fail "cancel created untitled.md"
click 158 48
click 679 292
[[ ! -f "$WS/untitled.md" ]] || fail "close created untitled.md"
key ctrl+shift+p
type_text "New File in Workspace"
key Enter
sleep 0.5
click 470 390
key ctrl+a
type_text "e2e-created.md"
key Return
wait_file "$WS/e2e-created.md"
wait_title "e2e-created.md"
click 185 48
key ctrl+a
type_text "e2e-folder"
key Return
wait_file "$WS/e2e-folder"
shot "10-created"
click 240 48
shot "11-collapsed"
click 213 48

echo "== explorer context menu: rename, delete, new file in a folder =="
# Collapsed tree: e2e-folder is the fourth row (y=156) and e2e-created.md the first file (y=231).
# Menu rows sit below the clicked row: a file's Rename at +80 and Delete at +103, a folder's
# New file at +27.
sleep 0.6
rclick 60 231
click 57 311
key ctrl+a
type_text "ctx-renamed.md"
key Return
wait_file "$WS/ctx-renamed.md"
[[ ! -e "$WS/e2e-created.md" ]] || fail "context-menu rename kept the old name"
sleep 0.6
rclick 60 231
click 57 334
click 615 408
for _ in $(seq 1 40); do
  [[ ! -e "$WS/ctx-renamed.md" ]] && break
  sleep 0.1
done
[[ ! -e "$WS/ctx-renamed.md" ]] || fail "context-menu delete did not remove the file"
sleep 0.6
rclick 70 156
click 60 183
sleep 0.5
click 470 390
key ctrl+a
type_text "inside.md"
key Return
wait_file "$WS/e2e-folder/inside.md"
shot "11b-context-menu-actions"
# Reveal: a folder opens itself, a file opens its folder. The tree now shows e2e-folder
# expanded with inside.md below it.
sleep 0.6
rclick 70 156
click 60 229
rclick 80 181
click 60 231
for _ in $(seq 1 30); do
  [[ $(wc -l < "$TMP/opened.log" 2>/dev/null || echo 0) -ge 2 ]] && break
  sleep 0.1
done
[[ "$(cat "$TMP/opened.log" 2>/dev/null)" == "$WS/e2e-folder"$'\n'"$WS/e2e-folder" ]] ||
  fail "Reveal opened: $(cat "$TMP/opened.log" 2>/dev/null)"

echo "== palette, tabs, quick open =="
key ctrl+shift+p
type_text "settings"
shot "12-palette"
key Escape
key ctrl+p
type_text "showcase"
key Enter
wait_title "showcase.md"
x press 926 50
sleep 0.25
shot "13a-tab-overflow-pressed"
log_has "tab overflow pressed"
x release 926 50
sleep 0.4
shot "13-tab-menu"
key Escape
key ctrl+shift+p
type_text "Close Other Tabs"
key Enter
sleep 0.5
[[ "$(title)" == "showcase.md" ]] || fail "close other tabs changed active title"

echo "== editor, find, replace, goto, save =="
key ctrl+shift+p
type_text "Edit mode"
key Enter
sleep 0.8
# Entering Edit mode must focus the editor: move and type without clicking the pane.
key ctrl+End
type_text "E2E "
sleep 1.0
shot "14a-editor-typed"
key ctrl+s
sleep 0.8
if ! grep -q "E2E" "$WS/showcase.md"; then
  # Debug builds can still be highlighting the first editor frame when Ctrl+S arrives.
  key ctrl+s
  sleep 0.8
fi
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

echo "== markdown formatting and lists =="
key ctrl+p
type_text "format.md"
key Enter
wait_title "format.md"
key ctrl+shift+p
type_text "Edit mode"
key Enter
sleep 0.8
key ctrl+End
# Enter continues a list, Enter on an empty item ends it, Tab nests an empty item.
type_text "- one"; key Enter; type_text "two"; key Enter; key Enter
type_text "1. first"; key Enter; type_text "second"; key Enter; key Tab
type_text "sub"; key Enter; key Enter; type_text "third"
# Bold the selected word with the Markdown shortcut (Ctrl+B is Bold in a Markdown editor).
key ctrl+shift+Left
key ctrl+b
sleep 0.4
shot "17a-markdown-lists"
key ctrl+s
sleep 0.8
python3 - "$WS/src/format.md" <<'PY_FORMAT' || fail "Markdown list editing produced unexpected text"
import sys
text = open(sys.argv[1], encoding="utf-8").read()
expected = "# Format\n\n- one\n- two\n1. first\n2. second\n   1. sub\n3. **third**"
if text.rstrip("\n") != expected:
    raise SystemExit(f"got {text!r}")
PY_FORMAT
key ctrl+p
type_text "showcase.md"
key Enter
wait_title "showcase.md"

echo "== editor interactions: mouse, clipboard, find, toolbar, palette =="
# Coordinates follow the Markdown editor in Edit mode: the formatting bar sits above the
# text, so line N is at y = 117 + (N - 1) * 18.8 and column C at x = 317 + C * 7.83.
key ctrl+p
type_text "interact.md"
key Enter
wait_title "interact.md"
key ctrl+shift+p
type_text "Edit mode"
key Enter
sleep 0.8
x clicks 345 155 2            # double-click selects alpha_beta
key ctrl+b                    # Bold in a Markdown editor
x clicks 330 192 3            # triple-click selects the line b=22
key ctrl+c
key End
key Enter
key ctrl+v                    # the copied line is pasted below
key ctrl+h
type_text '(\w)=(\d+)'
click 772 141                 # regex option
click 625 180                 # replacement field
type_text '$2:$1'
click 840 180                 # Replace All
key Escape
set_wrap true
x drag 318 268 357 287        # select across two wrapped rows
type_text "Z"
key ctrl+End
click 479 91                  # toolbar table button
click 520 170                 # 3 columns x 4 rows on the grid
type_text "Head"
key ctrl+Home
key ctrl+shift+p
type_text "Format: Heading 2"
key Enter
sleep 0.4
click 360 230                 # cursor inside f(x[y]): brackets are highlighted
shot "17b-editor-interactions"
python3 - "$SHOTS/17b-editor-interactions.png" <<'PY_BRACKETS' || fail "the bracket pair around the cursor is not highlighted"
import subprocess, sys
def blueness(x0, x1, y0=223, y1=238):
    raw = subprocess.run(["ffmpeg", "-loglevel", "error", "-i", sys.argv[1], "-vf",
                          f"crop={x1 - x0}:{y1 - y0}:{x0}:{y0}", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"],
                         capture_output=True, check=True).stdout
    pixels = [raw[i:i + 3] for i in range(0, len(raw), 3)]
    return sum(p[2] - p[0] for p in pixels) / len(pixels)
cells = {col: blueness(int(317 + col * 7.83) + 1, int(317 + col * 7.83) + 7) for col in range(7)}
# f(x[y]) with the cursor before `]`: `[` (3) and `]` (5) are highlighted, `(` and `)` are not.
plain = max(cells[1], cells[2], cells[4], cells[6])
if not (cells[3] > plain + 6 and cells[5] > plain + 6):
    raise SystemExit(f"cells {cells}")
PY_BRACKETS
key ctrl+s
sleep 0.8
python3 - "$WS/src/interact.md" <<'PY_INTERACT' || fail "editor interactions produced unexpected text"
import sys
lines = open(sys.argv[1], encoding="utf-8").read().split("\n")
expected_head = ["## Interact", "", "**alpha_beta** gamma", "1:a", "22:b", "22:b", "f(x[y])", ""]
if lines[:8] != expected_head:
    raise SystemExit(f"head: {lines[:8]!r}")
paragraph = lines[8]
# The drag replaced everything from the paragraph start to column 5 of the second row.
if not paragraph.startswith("Zword ") or not 10 < paragraph.count("word") < 40:
    raise SystemExit(f"paragraph: {paragraph!r}")
table = [line for line in lines if line.startswith("|")]
if len(table) != 5 or not table[0].startswith("| Head | Column 2 | Column 3 |"):
    raise SystemExit(f"table: {table!r}")
PY_INTERACT
set_wrap false

echo "== formatting bar: every button =="
key ctrl+p
type_text "toolbar.md"
key Enter
wait_title "toolbar.md"
key ctrl+shift+p
type_text "Edit mode"
key Enter
sleep 0.8
click 330 117; click 285 91; click 325 171   # Heading menu: Heading 2
x clicks 330 136 2; click 338 91             # Italic on the double-clicked word
x clicks 330 155 2; click 364 91             # Underline
click 330 174; click 401 91                  # Quote
x clicks 330 192 2; click 427 91             # Code
x clicks 330 211 2; click 453 91             # Link: the url placeholder is selected
type_text "https://x.dev"
click 330 230; click 516 91                  # Bulleted list
click 330 249; click 542 91                  # Numbered list
click 330 268; click 568 91                  # Task list
shot "17c-formatting-bar"
key ctrl+s
sleep 0.8
expected=$'## heading\n_italic_\n<ins>underline</ins>\n> quote\n`code`\n[link](https://x.dev)\n- bullets\n1. numbered\n- [ ] task'
[[ "$(cat "$WS/src/toolbar.md")" == "$expected" ]] || fail "formatting bar produced: $(cat "$WS/src/toolbar.md")"
key ctrl+w

echo "== command palette: every Format command =="
key ctrl+p
type_text "palette.md"
key Enter
wait_title "palette.md"
key ctrl+shift+p
type_text "Edit mode"
key Enter
sleep 0.8
format_cmd() { key ctrl+shift+p; type_text "Format: $1"; key Enter; sleep 0.3; }
y_of() { echo $((117 + ($1 - 1) * 1883 / 100)); }
x clicks 330 "$(y_of 1)" 2; format_cmd "Bold"
x clicks 330 "$(y_of 2)" 2; format_cmd "Italic"
x clicks 330 "$(y_of 3)" 2; format_cmd "Underline"
click 330 "$(y_of 4)"; format_cmd "Heading 1"
click 330 "$(y_of 5)"; format_cmd "Heading 2"
click 330 "$(y_of 6)"; format_cmd "Heading 3"
click 340 "$(y_of 7)"; format_cmd "Normal text"
click 330 "$(y_of 8)"; format_cmd "Quote"
x clicks 330 "$(y_of 9)" 2; format_cmd "Code"
x clicks 330 "$(y_of 10)" 2; format_cmd "Link"; type_text "u"
click 330 "$(y_of 11)"; format_cmd "Bulleted list"
click 330 "$(y_of 12)"; format_cmd "Numbered list"
click 330 "$(y_of 13)"; format_cmd "Task list"
click 330 "$(y_of 14)"; key End; format_cmd "Table 3"
shot "17d-palette-format"
key ctrl+s
sleep 0.8
python3 - "$WS/src/palette.md" <<'PY_PALETTE' || fail "palette Format commands produced unexpected text"
import sys
lines = open(sys.argv[1], encoding="utf-8").read().split("\n")
expected = ["**bold**", "_italic_", "<ins>underline</ins>", "# hone", "## htwo", "### hthree", "normal",
            "> quote", "`code`", "[link](u)", "- bullets", "1. numbered", "- [ ] task", "end", ""]
if lines[:15] != expected:
    raise SystemExit(f"got {lines[:15]!r}")
table = [line for line in lines[15:] if line.startswith("|")]
if len(table) != 4 or not table[0].startswith("| Column 1 | Column 2 | Column 3 |"):
    raise SystemExit(f"table {table!r}")
PY_PALETTE
key ctrl+w

echo "== large file mode =="
key ctrl+p
type_text "big.txt"
key Enter
wait_title "big.txt"
sleep 1
key ctrl+End
type_text "TAIL"
sleep 0.6
key ctrl+s
sleep 1
[[ "$(tail -c 4 "$WS/src/big.txt")" == "TAIL" ]] || fail "typing at the end of a large file was not saved"
log_has "file open path=$WS/src/big.txt bytes=[0-9]+ lines=210001"
key ctrl+w
key ctrl+p
type_text "showcase.md"
key Enter
wait_title "showcase.md"

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
x wheelup 640 400 200
sleep 1.5
shot "20-render-top"
click 422 473
wait_title "guide.md"
key ctrl+p
type_text "logo.svg"
key Enter
wait_title "logo.svg"
shot "20b-image-svg"
key ctrl+p
type_text "photo.png"
key Enter
wait_title "photo.png"
shot "20c-image-png"
key ctrl+p
type_text "showcase"
key Enter
wait_title "showcase.md"
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

echo "== tab scrolling =="
for file in README.md guide.md leaf.md main.rs extra1.md extra2.md extra3.md extra4.md showcase.md; do
  key ctrl+p
  type_text "$file"
  key Enter
  sleep 0.3
done
x wheel 500 50 8
sleep 0.8
shot "25-tab-scroll"
grep -q "tab wheel delta=" "$LOGS/mdview.log" || fail "tab wheel did not scroll"
tab_offset=$(grep -o 'offset=[0-9.]*' "$LOGS/mdview.log" | tail -1 | cut -d= -f2)
python3 - "$tab_offset" <<'PY' || fail "tab wheel moved $tab_offset px instead of 320 px"
import sys
raise SystemExit(0 if abs(float(sys.argv[1]) - 320.0) <= 1.0 else 1)
PY

echo "== search, history, and segmented modes =="
key ctrl+shift+f
sleep 0.5
type_text "Pixel"
sleep 1.5
log_has "workspace search query=Pixel"
log_has "workspace search hits="
shot "26-search-workspace"
key Enter
wait_title "README.md"
key alt+Left
wait_title "showcase.md"
click 802 16
sleep 0.5
log_has "command Search"
shot "27-search-document"
key Escape
key ctrl+f
sleep 0.4
key ctrl+a
type_text "Table"
sleep 0.8
shot "28-find-overlay"
key Escape

key ctrl+shift+e
sleep 0.6
click 813 51
sleep 0.5
click 849 51
sleep 0.5
click 885 51
sleep 0.5
log_has "command SetMode\(View\)"
log_has "command SetMode\(Edit\)"
log_has "command SetMode\(Split\)"

echo "== scroll-end matrix: Edit/Split × unwrapped/wrapped =="
set_wrap false
set_mode "Edit mode"
click 640 400
key ctrl+Home
sleep 0.4
x wheel 640 400 300
sleep 1.4
assert_end_stable edit false
set_wrap true
click 640 400
key ctrl+Home
sleep 0.4
x wheel 640 400 300
sleep 1.4
assert_end_stable edit true
set_mode "Split view"
set_wrap false
click 320 400
key ctrl+Home
sleep 0.4
x wheel 320 400 300
sleep 1.4
assert_end_stable split false
set_wrap true
click 320 400
key ctrl+Home
sleep 0.4
x wheel 320 400 300
sleep 1.4
assert_end_stable split true
set_wrap false

echo "== End key reaches the final preview block =="
set_mode "Reading view"
key End
sleep 1.2
log_has "preview end via End"
shot "35-end-key"
python3 - "$LOGS/mdview.log" <<'PY' || fail "End key did not reach the preview end"
import re
import sys
text = open(sys.argv[1], encoding="utf-8").read()
matches = [
    tuple(map(float, match))
    for match in re.findall(
        r"preview end via End offset=([0-9.]+) max=([0-9.]+) last_bottom=([0-9.]+) content=([0-9.]+)",
        text,
    )
]
if not matches:
    raise SystemExit("no End diagnostic")
offset, maximum, bottom, content = matches[-1]
if offset < maximum - 2:
    raise SystemExit(f"End stopped at {offset} of {maximum}")
if bottom > content + 1:
    raise SystemExit(f"last block {bottom} exceeds content {content}")
PY

echo "== image viewer, zoom, pan, and lightbox =="
key ctrl+p
type_text "logo.svg"
key Enter
wait_title "logo.svg"
shot "20b-image-svg"
x drag 490 360 560 410
sleep 0.5
log_has "image pan="
x wheelup 490 360 10
sleep 1.5
log_has "image zoom"
log_has "image reraster"
click 490 360
sleep 0.8
log_has "lightbox open"
click 276 70
click 276 70
click 276 70
sleep 1.2
log_has "lightbox reraster"
x drag 490 360 560 410
sleep 0.6
log_has "lightbox pan="
python3 - "$LOGS/mdview.log" <<'PY' || fail "lightbox drag did not move the image"
import re
import sys

text = open(sys.argv[1], encoding="utf-8").read()
pans = [
    tuple(map(float, match))
    for match in re.findall(r"lightbox pan=\[(-?[0-9.]+) (-?[0-9.]+)\]", text)
]
if not pans:
    raise SystemExit("no lightbox pan diagnostics")
if not any(abs(x) > 1.0 or abs(y) > 1.0 for x, y in pans):
    raise SystemExit(f"lightbox pan did not move: {pans[-3:]}")
PY
shot "29-lightbox"
key Escape
sleep 0.4
# Disabled segmented segments: an image cannot enter the editor or split mode.
before_modes=$(grep -c 'command SetMode' "$LOGS/mdview.log" || true)
click 849 51
click 885 51
sleep 0.4
after_modes=$(grep -c 'command SetMode' "$LOGS/mdview.log" || true)
[[ "$before_modes" == "$after_modes" ]] || fail "image document accepted Edit/Split mode"
key ctrl+p
type_text "showcase"
key Enter
wait_title "showcase.md"

echo "== code document segmented modes =="
key ctrl+p
type_text "segmented.rs"
key Enter
wait_title "segmented.rs"
log_has "document mode.*segmented.rs mode=edit can_edit=true can_view=false can_split=false"
shot "36-code-modes"
before_modes=$(grep -c 'command SetMode' "$LOGS/mdview.log" || true)
click 813 51
click 885 51
sleep 0.5
after_modes=$(grep -c 'command SetMode' "$LOGS/mdview.log" || true)
[[ "$before_modes" == "$after_modes" ]] || fail "code document accepted View/Split mode"
key ctrl+p
type_text "showcase"
key Enter
wait_title "showcase.md"

echo "== split end stability =="
key ctrl+shift+e
sleep 0.6
x wheel 700 400 300
sleep 2
log_has "preview scroll end"
shot "30-split-end"

echo "== split end hold with continued editor wheel input =="
set_mode "Split view"
set_wrap false
click 320 400
key ctrl+Home
sleep 0.6
before_end=$(grep -c 'split sync editor->preview end' "$LOGS/mdview.log" || true)
x wheel 320 400 400
sleep 1.6
log_has "editor scroll end mode=split wrap=false"
after_end=$(grep -c 'split sync editor->preview end' "$LOGS/mdview.log" || true)
(( after_end - before_end <= 1 )) || fail "preview end target was rewritten while the editor stayed at the end"
shot "30a-split-end-hold"

echo "== split zoom: both ends reached together, locked, no overshoot paint =="
# Zoom keeps the window size in pixels, so the split area in points changes: at 1.3 the
# split stacks the editor over the preview, at 0.9 it stays side by side.
assert_split_settled() {
  python3 - "$LOGS/mdview.log" "$1" "$2" <<'PY' || fail "split zoom $2 with $1 leading is not locked at both ends"
import re
import sys
text = open(sys.argv[1], encoding="utf-8").read()
leader, zoom = sys.argv[2], float(sys.argv[3])
pattern = (r"split settled leader=(\w+) error=([0-9.]+) zoom=([0-9.]+) "
           r"editor=([0-9.]+)/([0-9.]+) preview=([0-9.]+)/([0-9.]+)")
matches = re.findall(pattern, text)
if not matches:
    raise SystemExit("no split settled line")
name, error, actual_zoom, e, e_max, p, p_max = matches[-1]
error, actual_zoom, e, e_max, p, p_max = map(float, (error, actual_zoom, e, e_max, p, p_max))
print(f"split zoom={actual_zoom} leader={name} error={error} editor={e}/{e_max} preview={p}/{p_max}")
assert name == leader, name
assert abs(actual_zoom - zoom) < 0.01, actual_zoom
assert error <= 0.005, error
assert e_max > 0 and p_max > 0
assert abs(e - e_max) <= 0.005 and abs(p - p_max) <= 0.005, (e, e_max, p, p_max)
PY
}
before_over=$(grep -c 'split overshoot' "$LOGS/mdview.log" || true)
for zoom in 1.3 0.9; do
  key ctrl+0
  if [[ $zoom == 1.3 ]]; then
    key ctrl+equal; key ctrl+equal; key ctrl+equal
    EP=(650 230); PP=(650 540)
  else
    key ctrl+minus
    EP=(420 400); PP=(800 400)
  fi
  sleep 0.8
  click "${EP[@]}"
  key ctrl+Home
  sleep 0.6
  x wheel "${EP[@]}" 400
  sleep 1.4
  assert_split_settled Editor "$zoom"
  shot "30b-split-zoom-$zoom-editor"
  x wheelup "${PP[@]}" 600
  x wheel "${PP[@]}" 400
  sleep 1.4
  assert_split_settled Preview "$zoom"
  shot "30c-split-zoom-$zoom-preview"
done
key ctrl+0
sleep 0.5
after_over=$(grep -c 'split overshoot' "$LOGS/mdview.log" || true)
[[ "$before_over" == "$after_over" ]] || fail "a split pane painted past its end (flicker)"

echo "== external conflict and reload =="
key ctrl+shift+p
type_text "Edit mode"
key Enter
sleep 0.5
click 640 400
type_text "CONFLICT "
printf '\nExternal change\n' >> "$WS/showcase.md"
sleep 2
log_has "disk conflict for dirty file"
shot "31-conflict"
key alt+r
sleep 2
log_has "reloaded externally changed clean file"

echo "== Spanish dialogs and keyboard menus =="
click 850 16
sleep 0.5
click 240 185
click 700 350
click 700 435
sleep 0.6
shot "32-settings-spanish"
key Escape
sleep 0.3
key alt+f
sleep 0.4
log_has "keyboard menu group=0"
shot "37-menu-file-spanish"
key Down
sleep 0.3
shot "38-menu-item-spanish"
key Enter
sleep 0.5
log_has "command NewFile"
shot "39-prompt-spanish"
key Escape
sleep 0.3
key ctrl+shift+p
sleep 0.4
shot "40-palette-spanish"
key Escape
sleep 0.3
key ctrl+f
sleep 0.4
shot "41-find-spanish"
key Escape
sleep 0.3
key alt+h
sleep 0.3
key Down
key Down
sleep 0.2
key Enter
sleep 0.6
log_has "command About"
shot "42-about-spanish"
key Escape
sleep 0.3
key ctrl+shift+p
type_text "Renombrar"
key Enter
sleep 0.6
log_has "rename prompt title=Cambiar nombre confirm=Cambiar nombre cancel=Cancelar"
shot "48-rename-spanish"
key Escape
sleep 0.3
key ctrl+shift+p
type_text "Eliminar"
key Enter
sleep 0.6
log_has "delete prompt title=Eliminar confirm=Eliminar cancel=Cancelar"
shot "49-delete-spanish"
key Escape
sleep 0.4
key Escape
sleep 0.4
key ctrl+p
type_text "showcase"
key Enter
wait_title "showcase.md"
key ctrl+shift+p
type_text "Modo"
key Enter
sleep 0.5
click 640 400
click 640 400
type_text "unsaved "
sleep 0.5
wait_title "showcase.md •"
click 950 16
sleep 0.7
log_has "close prompt title=¿Guardar cambios\\? save=Guardar discard=Descartar cancel=Cancelar"
shot "50-close-spanish"
key Escape
sleep 0.4
key ctrl+s
sleep 0.6

echo "== pseudo-locale dialogs and keyboard menus =="
key ctrl+shift+p
type_text "Pseudo"
key Enter
sleep 0.5
click 850 16
sleep 0.4
shot "33-settings-pseudo"
key Escape
sleep 0.3
key alt+f
sleep 0.4
shot "43-menu-file-pseudo"
key Escape
sleep 0.2
key ctrl+shift+p
sleep 0.4
shot "44-palette-pseudo"
key Escape
sleep 0.3
key ctrl+f
sleep 0.4
shot "45-find-pseudo"
key Escape
sleep 0.3
key alt+h
sleep 0.3
key Down
key Down
sleep 0.2
key Enter
sleep 0.6
shot "46-about-pseudo"
key Escape
sleep 0.3
key ctrl+shift+p
type_text "Rename"
key Enter
sleep 0.6
log_has "rename prompt title=\\[!Rename!\\]"
shot "51-rename-pseudo"
key Escape
sleep 0.3
key ctrl+shift+p
type_text "Delete File"
key Enter
sleep 0.6
log_has "delete prompt title=\\[!Delete!\\]"
shot "52-delete-pseudo"
key Escape
sleep 0.4
key Escape
sleep 0.4
key ctrl+p
type_text "showcase"
key Enter
wait_title "showcase.md"
key ctrl+shift+p
type_text "Edit mode"
key Enter
sleep 0.5
click 640 400
click 640 400
type_text "unsaved "
sleep 0.5
wait_title "showcase.md •"
sleep 0.4
click 950 16
sleep 0.7
log_has "close prompt title="
shot "53-close-pseudo"
key Escape
sleep 0.4
key ctrl+s
sleep 0.6
key alt+f
sleep 0.2
key Down
key Enter
sleep 0.5
shot "47-prompt-pseudo"
key Escape
sleep 0.3

echo "== restore locale and dark theme =="
key ctrl+shift+p
type_text "Follow"
key Enter
sleep 0.4
click 850 16
sleep 0.4
click 700 242
click 700 301
sleep 0.6
click 756 576
sleep 0.8
expect_cfg theme github-dark
key Escape
sleep 0.3
click 813 51
sleep 0.6
shot "34-dark-render"
key Escape
key ctrl+shift+p
type_text "Pause SVG animations"
key Enter
sleep 2
for _ in $(seq 1 12); do
  clean=$(grep 'perf frames=' "$LOGS/mdview.log" | tail -3 | grep -c 'over_8.33ms=0' || true)
  [[ "$clean" == 3 ]] && break
  sleep 2
done
perf_before_shutdown=$(grep -c 'perf frames=' "$LOGS/mdview.log" || true)

echo "== shutdown =="
click 950 16
for _ in $(seq 1 140); do
  kill -0 "$APP_PID" 2>/dev/null || break
  sleep 0.1
done
if kill -0 "$APP_PID" 2>/dev/null; then fail "application did not close on Alt+F4"; fi
APP_PID=""
kill "$GUARD_PID" 2>/dev/null || true
GUARD_PID=""
rss_peak=$(cat "$TMP/rss-peak-mb")
(( rss_peak > 0 )) || fail "RAM guard recorded no samples"

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
log_has "viewer open.*logo.svg"
log_has "viewer open.*photo.png"
log_has "source=http://127.0.0.1:8899/photo.png"
log_has "source=http://127.0.0.1:8899/logo.svg"
log_has "source=http://127.0.0.1:8899/pulse.svg"
if [[ "$PROFILE" == "release" ]]; then
  # Exclude the shutdown window; the frame-budget gate is the steady-state p95
  # (individual Xvfb/scheduler spikes are not a 120 Hz regression).
  python3 - "$LOGS/mdview.log" "${perf_before_shutdown:-0}" <<'PY' || fail "final frame windows exceeded the 120 Hz p95 budget"
import re
import sys

lines = [line for line in open(sys.argv[1], encoding="utf-8") if "perf frames=" in line]
limit = int(sys.argv[2]) if sys.argv[2].isdigit() else len(lines)
for line in lines[max(0, limit - 3):limit]:
    match = re.search(r"p95=([0-9.]+)ms", line)
    if not match or float(match.group(1)) > 8.33:
        raise SystemExit(line.strip())
PY
fi

echo
echo "E2E PASS"
echo "  log bytes: $size (limit 1048576)"
echo "  peak app memory: ${rss_peak} MiB (limit ${MAX_RSS_MB} MiB)"
echo "  screenshots: $SHOTS"
echo "  logs: $LOGS"
echo "  workspace: $WS"
echo
tail -n 8 "$LOGS/mdview.log"
