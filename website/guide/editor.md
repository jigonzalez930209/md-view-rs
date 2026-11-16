# Editing

The editor is virtualized: only the visible rows are laid out and painted. Syntax highlighting
keeps a per-line parse cache, so scrolling forward and editing near the cursor reuse the previous
state instead of re-parsing the document.

![Editor](/images/editor.png)

## Modes

The tab bar has **View**, **Editor**, and **Split** segments:

- **View** reads Markdown with the GitHub-style renderer.
- **Editor** shows the virtualized text editor.
- **Split** shows both panes with block-accurate bidirectional scroll sync.

Images and code files open in their supported presentation; modes that do not apply are disabled
for that tab.

Entering **Editor** or **Split** mode, activating a tab, and closing the find or go-to-line panel
all return keyboard focus to the editor. Arrow keys, typing, undo, and copy/paste work immediately
without clicking the text pane first.

## Syntax highlighting

`mdview-syntax` uses syntect's Oniguruma engine with two-face's matching syntax dumps. The first
highlight of a cold language compiles its grammar once; later files of the same language reuse the
shared cache. TypeScript's first cold highlight in a debug build dropped from about 2.8 s to 33 ms
after moving from the `regex-fancy` backend to Oniguruma.

Highlighting is disabled above 2,000 lines or 1 MB. Large files use the virtualized editor without
syntax colors and show a **Large file** status.

## Find and replace

`Ctrl/Cmd+F` opens the floating find panel; `Ctrl/Cmd+H` opens replace. The panel supports regular
expressions, case and whole-word matching, match counts, and next/previous navigation. The
document and workspace search share the same top-center panel anchored to the window.

![Find overlay](/images/find.png)

## Go to line

`Ctrl/Cmd+G` opens the line prompt. Enter a one-based line number to move the cursor and center it.

## LaTeX blocks

`Ctrl/Cmd+Shift+M`, **Edit → Insert LaTeX Block**, or the command palette inserts a display-math
block on its own lines and places the cursor inside it:

```markdown
$$
E = mc^2
$$
```

The preview renders the block with the reading view's equation size. Inline math uses a single
`$…$` pair.

## Quick Open and the palette

`Ctrl/Cmd+P` opens workspace Quick Open. `Ctrl/Cmd+Shift+P` opens the command palette with every
menu command, including mode switches, wrap, line numbers, theme changes, and file operations.

![Command palette](/images/palette.png)

## Split view

Split view keeps the editor and preview on the same source line. The anchor maps editor rows to
Markdown blocks and the preview back to the editor, tolerating files whose layout changes while
media finishes measuring. The per-tab ratio is restored with the session.

![Split view](/images/split.png)

## Large files

Files larger than 10 MB or 200,000 lines load on a worker with progress and cancellation. The
editor stays virtualized, wrap is disabled, and syntax highlighting is turned off so scrolling and
typing stay inside the frame budget.

For everyday editing, use a release build. Debug builds are unoptimized and are not a
representative measure of typing, highlighting, or preview latency.
