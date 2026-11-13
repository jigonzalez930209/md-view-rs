# Animated SVG (`0.7.0`)

MD View supports a bounded SMIL and CSS animation subset without a webview or JavaScript. The
animation engine rewrites animated attributes into the SVG source for a requested time and
rasterizes that frame with `resvg`. Static SVG already uses the same parser and limits.

## Supported subset

### SMIL

| Feature | Details |
|---|---|
| `<animate>` | `opacity`, `fill`, `stroke`, `x`, `y`, `width`, `height`, `cx`, `cy`, `r` |
| `<animateTransform>` | `translate`, `scale`, `rotate` (the values keep their extra arguments) |
| `<set>` | Supported attributes, applied from `begin` onward |
| Timing | `begin`, `dur` (`s`, `ms`), `repeatCount` (`indefinite` or a number) |
| Interpolation | linear and discrete; `keyTimes`; numeric and `#rrggbb` color interpolation |

### CSS

- `@keyframes` with percentage, `from`, and `to` selectors.
- `opacity`, `fill`, `stroke`, and `transform` declarations.
- Element `style="animation: name 2s infinite"` and simple class or ID rules:
  `.name { animation: ... }` / `#name { animation: ... }`.
- No transitions, `:hover`, events, media queries, or external stylesheets.

### Not supported

`animateMotion`, `animateColor`, `discard`, `mpath`, path morphing, event-based timing, script,
`foreignObject`, and external resources. A document that mixes an unsupported feature with a
supported clip shows the first frame with an “Animation not fully supported” badge; the document
does not break.

## Playback

- Frames advance at 30 fps. The frame clock only moves while the image is painted inside the
  viewport; offscreen images are marked invisible and stop consuming work.
- **View → Media → Play SVG animations** pauses or resumes every player.
- **View → Media → Reduce motion** keeps the first frame and stops the clock.
- Closing a tab clears the cache and increments the media generation, so queued frames from the
  closed document are discarded.
- Clicking an animated image opens the lightbox; it shows the current frame and the normal
  animation toggles apply.

## Limits and safety

The animation parser uses `roxmltree` on already validated SVG source. It does not execute
scripts, events, or external resources, and the existing SVG guards still bound bytes, depth,
entities, nodes, and embedded documents before rasterization. Each frame is bounded by the same
image side and source-size limits as static media. The media worker has a 16 MB stack.

## Verification

- `crates/mdview-media/src/svg_anim/tests.rs` contains 19 fixtures covering SMIL opacity,
  discrete and keyed interpolation, repeat, delay, set, fill/stroke colors, translate/rotate/scale,
  geometry, CSS keyframes by class and ID, inline CSS animation, static documents, unsupported
  motion, and raster frame differences.
- `crates/mdview-app/src/media_runtime.rs` covers detection, play/pause, reduced motion,
  visibility pause, cache clearing, and the worker round trip.
- `crates/mdview-markdown/tests/fuzz.rs` and `crates/mdview-media/tests/hostile.rs` cover random
  and hostile inputs; the Markdown parser flattens nesting beyond 64 levels instead of
  overflowing the stack.
- `crates/mdview-editor/tests/large.rs` opens a 50 MB / >100k-line file in large mode.
- `crates/mdview-app/src/tabs_tests.rs` covers 100 open/close cycles and preview refusal for
  large Markdown files.
