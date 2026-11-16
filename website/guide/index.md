# MD View

MD View is a native Markdown, text, and code editor for Windows, macOS, and Linux. It is built with
Rust and egui, renders Markdown itself, and does not embed a browser or a webview.

The 1.0.0-rc.2 release is the third public release candidate. It ships the GitHub-parity reading
view, the virtualized editor, the workspace explorer, nine themes, media rendering, search, and
the release packaging for the three desktop platforms.

![MD View reading view](/images/hero.png)

## What it does

- Opens a folder as a workspace with a lazy tree, tabs, recent files, session restore, file
  watching, and conflict handling.
- Reads Markdown with CommonMark and GFM: tables, task items, alerts, footnotes, safe HTML,
  syntax-colored fences, LaTeX, Mermaid, images, SVG, and animated SVG.
- Edits text in a virtualized editor with syntax highlighting, regex find/replace, go-to-line,
  encoding and EOL detection, mixed-EOL preservation, and large-file mode.
- Shows a GitHub-style preview with measured block spacing and a source map that keeps the editor
  and reading view on the same line.
- Marks the active tab in the explorer, expands its folders, and keeps files opened outside the
  workspace in a fixed **External files** group.
- Runs entirely on the desktop. Remote images are opt-in, private-network requests are a separate
  setting, and updates verify their SHA-256 before installation.

## At a glance

| Area | What is included |
|---|---|
| Reading view | GFM, GitHub layout, alerts, tasks, footnotes, tables, safe HTML, math, Mermaid, media |
| Editor | Virtualized rows, syntax highlighting, find/replace, go-to-line, wrap, line numbers |
| Workspace | Lazy tree, tabs, watcher, conflict diff, recent files, session restore, external files |
| Media | PNG, JPEG, GIF, WebP, BMP, TIFF, SVG, animated SVG, local and cached remote images |
| Platform | Linux (deb, RPM, AppImage, tarball), macOS (app/dmg), Windows (MSI/zip) |
| UI | Nine themes, English and Spanish, command palette, workspace search, keyboard menus |

## Next steps

- [Install the release candidate](/guide/install)
- [Learn the workspace and tabs](/guide/workspace)
- [Read about Markdown support](/guide/markdown)
- [See the full feature matrix](/guide/features)
