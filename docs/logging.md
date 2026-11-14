# Diagnostics and logs

MD View writes a diagnostic log alongside the normal stderr output.

## Location

```
$XDG_STATE_HOME/mdview/logs/mdview.log
$XDG_STATE_HOME/mdview/logs/mdview.1.log
```

`XDG_STATE_HOME` defaults to `~/.local/state`. `MDVIEW_LOG_DIR` overrides the folder for tests.
The About tab shows the current usage and opens the folder.

## Rotation and size

- Each part is capped at 512 KiB.
- The active file rotates to `mdview.1.log` before it would exceed the cap.
- The two parts therefore never exceed 1 MiB in total.
- Existing oversized parts are truncated at startup.

## Format

```
2026-11-14T15:50:46.514618168Z INFO  mdview::commands src/commands.rs:14: command SetMode(Split)
```

Every line has a UTC RFC 3339 timestamp, level, target, source location, and message.

## Coverage

- startup version, arguments, and active log path
- commands and shortcuts
- workspace open and index timing
- file open/save bytes, lines, and elapsed time
- large-file worker load time and errors
- media load source and elapsed time; failures and slow loads
- explorer toggles, create/rename/delete, collapse
- settings changes through commands
- 120-frame performance windows with p50/p95/max/over-budget counts
- user-facing errors
- close requests

`RUST_LOG=debug` adds transport and per-frame details; the default is `warn,mdview=info`.
