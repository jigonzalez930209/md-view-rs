# Performance and 120 Hz frame budget

Two independent measurements guard the interaction path:

## Widget benchmark

```sh
cargo xtask perf
```

Runs `crates/mdview-render/tests/frame_budget.rs` in release mode. The harness builds a 500-section
Markdown document (about 6,500 lines), warms up, then performs 120 scroll frames while measuring
the CPU time inside `Editor::show` and `Preview::show`.

Latest result on the Linux development machine:

```text
120Hz frame budget (120 frames):
editor  p50=0.235 ms p95=0.401 ms
preview p50=1.337 ms p95=2.255 ms
combined p50=1.578 ms p95=2.665 ms max=4.022 ms
```

The release test asserts `combined p95 <= 8.33 ms`. Debug builds use a relaxed assertion so the
normal suite remains usable.

## Full-application telemetry

`FrameMetrics` records the CPU time from the start of `logic` to the end of `ui` in windows of 120
frames and logs:

```text
perf frames=... p50=...ms p95=...ms max=...ms over_8.33ms=...
```

Release app under Xvfb, split mode, 120 wheel events:

```text
frames=120  p50=1.427ms p95=1.960ms max=47.189ms over_8.33ms=1  (cold-start frame)
frames=240  p50=1.658ms p95=2.060ms max=2.204ms  over_8.33ms=0
frames=360  p50=1.667ms p95=1.942ms max=2.270ms  over_8.33ms=0
frames=480  p50=1.620ms p95=2.010ms max=2.949ms  over_8.33ms=0
frames=600  p50=1.584ms p95=2.064ms max=2.656ms  over_8.33ms=0
frames=720  p50=1.576ms p95=1.872ms max=2.358ms  over_8.33ms=0
```

The first window contains process/window/GPU initialization; every subsequent measured frame stays
under the 8.33 ms budget.

## Large documents

- Preview blocks are virtualized: only visible blocks plus a small overscan are painted. Measured
  heights are cached; offscreen blocks use estimates. `tests/virtualization.rs` asserts that fewer
  than 60 of more than 800 blocks are painted at the top and at the bottom.
- The complete source map is still built for synchronization, so scrolling maps to the right
  source line without painting the whole document.
- Syntax highlighting in the editor is disabled above 2,000 lines or 1 MB. This is the expensive
  `syntect` path that previously produced 14–20 ms frames while jumping through a large Markdown
  file. Preview code fences still highlight what is visible.
- Markdown preview is disabled above 2 MB or 50,000 lines, with a notice; the editor remains
  virtualized. A 50 MB / 100k-line file is covered by `crates/mdview-editor/tests/large.rs`.
- Scroll synchronization maps editor first-visible line → preview source block, and preview scroll
  offset → editor line. The source map uses the preceding block when the offset lands in the gap
  between two blocks, which fixes the jump-to-end bug.

## Stress tests

- 120 wheel events alternating between editor and preview in a split with a 5,201-line document:
  no panic, aligned panes, zero frames over budget after startup.
- 1,000 simulated scroll-source decisions in `split.rs` assert no oscillation.
- Bidirectional E2E screenshots are kept outside the repository during manual verification; the
  committed snapshots remain the reproducible references.

## Release E2E interaction run (0.11.3)

`MDVIEW_E2E_PROFILE=release cargo xtask e2e` exercises menus, settings, prompts, five-level
folder expansion through a watcher refresh, file operations, tabs, editing, find/goto, split
scrolling, local/remote media, LaTeX, code blocks, a clickable table link, tables, and
Mermaid in one run (11,520 frames). Steady-state windows after the render pass:

```text
frames=11040 p50=0.240ms p95=0.625ms max=1.496ms over_8.33ms=0
frames=11160 p50=0.310ms p95=0.621ms max=1.574ms over_8.33ms=0
frames=11280 p50=0.311ms p95=0.705ms max=0.786ms over_8.33ms=0
frames=11400 p50=0.319ms p95=0.494ms max=0.704ms over_8.33ms=0
frames=11520 p50=0.277ms p95=0.531ms max=0.807ms over_8.33ms=0
```

Transition frames can still exceed 8.33 ms during cold start, the first large media upload,
and the first split-mode layout. There are no over-budget frames in steady-state scrolling,
editing, or rendering.
