---
layout: home

hero:
  name: MD View
  text: Markdown, without a webview.
  tagline: A native Markdown reading view, code editor, workspace explorer, and rich-media renderer for Windows, macOS, and Linux. Built with Rust and egui.
  image:
    src: /images/hero.png
    alt: MD View rendering a Markdown document with math, tables, SVG, and Mermaid
  actions:
    - theme: brand
      text: Get started
      link: /guide/
    - theme: alt
      text: Download 1.0.0-rc.6
      link: /guide/install

features:
  - title: Native and fast
    details: A Rust desktop application with a virtualized editor and reading view. No browser engine, no webview, no JavaScript runtime.
  - title: GitHub-style Markdown
    details: GFM tables, tasks, alerts, footnotes, safe HTML, syntax-colored code, LaTeX math, Mermaid diagrams, local and remote images, and animated SVG.
  - title: A real workspace
    details: Lazy folder tree, tabs, recent files, session restore, file watching, conflict handling, and a fixed external-files group for tabs opened outside the workspace.
  - title: Reading and editing
    details: View, Edit, and Split modes with block-accurate scroll sync, search and replace, go-to-line, encoding detection, mixed EOL preservation, and large-file mode.
  - title: Nine themes
    details: Calibrated light and dark palettes, a GitHub-parity reading layout, typography controls for body, headings, code, tables, and equations, and a zoomable image lightbox.
  - title: Local-first
    details: Files stay on your machine. Remote images are opt-in, private network requests are separate, and the update flow verifies SHA-256 before installing.
---

## A desktop workspace, not a browser tab

MD View puts the explorer, document tabs, reading view, and editor in one native window. Open a folder, switch between documents, and keep the active file visible in the tree.

<div class="shot-grid">
  <figure class="shot">
    <img src="/images/reading.png" alt="MD View reading a Markdown document">
    <figcaption>Reading view with GitHub-style typography.</figcaption>
  </figure>
  <figure class="shot">
    <img src="/images/editor.png" alt="MD View code editor with syntax highlighting">
    <figcaption>The virtualized editor with syntax colors.</figcaption>
  </figure>
  <figure class="shot">
    <img src="/images/explorer.png" alt="MD View workspace explorer">
    <figcaption>Lazy workspace tree and document tabs.</figcaption>
  </figure>
</div>

## Markdown that looks like GitHub

The preview follows GitHub's measured Markdown body: 1em block spacing, 1.5 line height, heading ratios, alert colors, table borders and zebra rows, footnote backlinks, inline code chips, and GitHub's Primer syntax colors. The layout is compared against GitHub's own render in the repository's parity fixtures.

<div class="shot-grid">
  <figure class="shot">
    <img src="/images/preview-light.png" alt="Light Markdown preview">
    <figcaption>GitHub light reading view.</figcaption>
  </figure>
  <figure class="shot">
    <img src="/images/preview-dark.png" alt="Dark Markdown preview">
    <figcaption>Dark reading view.</figcaption>
  </figure>
</div>

## Rich content, local files

Images, SVG, math, and Mermaid are rendered in the document with stable intrinsic boxes, so loading a texture never reflows the reader's position. Remote media is opt-in and cached; the image viewer has zoom, pan, fit, actual size, and a lightbox.

<div class="shot-grid">
  <figure class="shot">
    <img src="/images/media-light.png" alt="Markdown preview with images, math, and Mermaid">
    <figcaption>Local media, formulas, and diagrams.</figcaption>
  </figure>
  <figure class="shot">
    <img src="/images/split.png" alt="Split view with editor and preview">
    <figcaption>Split view with bidirectional scroll sync.</figcaption>
  </figure>
</div>

## Find what you need

Search the active document or the whole workspace from a floating panel. Results are grouped by file, preview the matching line, and activate at the exact source line. The find/replace overlay and the command palette are always one shortcut away.

<div class="shot-grid">
  <figure class="shot">
    <img src="/images/find.png" alt="MD View find overlay">
    <figcaption>Find and replace in the active document.</figcaption>
  </figure>
  <figure class="shot">
    <img src="/images/palette.png" alt="MD View command palette">
    <figcaption>Command palette and workspace Quick Open.</figcaption>
  </figure>
</div>

## Tabs outside the workspace

Files opened from anywhere appear in a fixed **External files** group at the end of the explorer. The group cannot be renamed, deleted, or used to create files; external documents can be renamed and the active tab is marked in the tree.

<figure class="shot shot-wide">
  <img src="/images/external-files.png" alt="MD View external files folder in the explorer">
  <figcaption>Open documents outside the workspace stay reachable from the tree.</figcaption>
</figure>

## Ready to try it?

The 1.0.0-rc.6 builds are published for Linux, macOS, and Windows. The RC is unsigned; the release page explains the current signing and notarization status.

<p class="home-actions">
  <a class="VPButton brand" href="/md-view-rs/guide/install">Download the RC</a>
  <a class="VPButton alt" href="https://github.com/jigonzalez930209/md-view-rs">View the source</a>
</p>
