#!/usr/bin/env bash
# Launch MD View in a private Xvfb display with a given theme.
# Usage: launch.sh <theme> <display> <workdir>
set -euo pipefail
THEME=$1; DISP=$2; WORK=$3
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
mkdir -p "$WORK"/{config,logs,runtime}
chmod 700 "$WORK/runtime"
rm -rf "$WORK/ws"; cp -r "$ROOT/examples/v1-workspace" "$WORK/ws"
SHOW="$WORK/ws/level1/level2/level3/level4/level5/showcase.md"
cat > "$WORK/config/config.toml" <<EOF
schema_version = 1

[appearance]
theme = "$THEME"
follow_system = false
zoom = 1.5

[language]
language = "en"

[window]
width = 1280.0
height = 667.0

[editor]
wrap = true
line_numbers = true
sync_scroll = true
font_size = 13.0
scroll_beyond_last_line = true
indent_guides = true
render_whitespace = "selection"

[session]
sidebar_open = true
sidebar_width = 215.0

[updates]
manifest_url = ""
enabled = false
EOF
Xvfb "$DISP" -screen 0 1920x1000x24 >"$WORK/xvfb.log" 2>&1 &
echo $! > "$WORK/xvfb.pid"
sleep 0.8
env -u WAYLAND_DISPLAY XDG_SESSION_TYPE=x11 DISPLAY="$DISP" USER=mdview_promo \
  XDG_RUNTIME_DIR="$WORK/runtime" MDVIEW_CONFIG_DIR="$WORK/config" MDVIEW_LOG_DIR="$WORK/logs" \
  RUST_LOG=warn "$ROOT/target/release/mdview" "$WORK/ws" "$SHOW" >"$WORK/app.log" 2>&1 &
echo $! > "$WORK/app.pid"
sleep 2.5
