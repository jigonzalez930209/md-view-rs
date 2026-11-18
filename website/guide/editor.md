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

## Markdown formatting

Markdown files (`.md`, `.markdown`, `.mdx`, …) show a formatting bar above the editor in
**Editor** and **Split** modes, modelled on GitHub's comment toolbar:

| Button | Markdown | Shortcut |
|---|---|---|
| Heading | `#` … `######`, or normal text, from a menu | |
| Bold | `**text**` | `Ctrl/Cmd+B` |
| Italic | `_text_` | `Ctrl/Cmd+I` |
| Underline | `<ins>text</ins>` (GitHub shows it underlined) | `Ctrl/Cmd+U` |
| Quote | `> ` before each line | |
| Code | `` `code` ``, or a fenced block for several lines | |
| Link | `[text](url)` with `url` selected | `Ctrl/Cmd+K` |
| Table | a table picked on a grid of up to 10 × 8 cells | |
| Bulleted, numbered, task list | `- `, `1. `, `- [ ] ` on each selected line | |
| Equation (Σ) | a gallery of common formulas, inserted as `$$…$$` | |
| Equation editor (√x) | the formula editor, on the formula at the cursor or a new one | `Alt+=` |

Every button toggles: pressing it on formatted text removes the format, and the buttons for the
formats under the cursor are shown pressed. Formats combine (`**_<ins>text</ins>_**`) and each
comes off on its own. Without a selection, Bold, Italic, and Underline format the word under the
cursor, or insert an empty pair to type into. Each action is a single undo step. The same actions
are in the command palette as **Format: …**. In a focused Markdown editor `Ctrl/Cmd+B` is Bold;
elsewhere it still toggles the sidebar.

Lists continue as you type:

- **Enter** at the end of an item starts the next one: numbers increase (later items are
  renumbered), and a new task starts unchecked. Enter in the middle of an item moves the rest of
  the text into the new item.
- **Enter** on an empty item leaves the list. A nested item first moves up to its parent's level
  and continues the parent's numbering; a top-level item loses its marker.
- **Tab** on an empty item nests it under the item above; **Shift+Tab** moves an item back up.

The **Table** button opens a grid: hover or use the arrow keys to choose columns × rows, then
click or press Enter. The header cells are named `Column 1`, `Column 2`, … and the first one is
selected so you can type over it. Inside a table, **Tab** moves to the next cell and selects its
text (the `| --- |` row is skipped), **Shift+Tab** goes back, and Tab in the last cell adds a row.
Each Tab also realigns the table's columns.

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

## Equation editor

The **Σ** button opens a gallery of common equations, as in Word: the area of a circle, the
binomial theorem, the quadratic formula, Taylor and Fourier series, Euler's identity, and more.
Click one to insert it as a display formula, or use its pencil to open it in the formula editor
first.

The **√x** button, `Alt+=`, **Edit → Equation Editor…**, or the command palette opens the formula
editor on the formula under the cursor, or for a new one. **Double-click** any formula to edit it:
`$…$` and `$$…$$` in the editor or the preview, and ```` ```math ```` blocks in the editor. In the
preview, a formula's context menu also has **Edit equation** (a single click on a math block
opens the lightbox).

The dialog has:

- **A template palette** in categories (structures, integrals, sums and limits, matrices, Greek,
  operators, arrows, functions, delimiters). Every template is shown at the same type size, so
  the buttons compare as they will in the formula. A template is inserted at the cursor with its
  first `\square` placeholder selected; **Tab** moves to the next placeholder.
- **The LaTeX source**, written in the same editor as documents, with highlighting and undo.
- **A live preview.** While the source does not parse, the last valid formula stays dimmed and
  the error is shown under it.
- **Source-to-formula hover.** Point at any part of the source and the part of the formula it
  draws lights up in the theme accent, while the source under it is underlined. Coloring never
  changes the formula's shape: when a part cannot be colored without moving something (the base
  of a scripted symbol, an operator with limits), the enclosing construct is highlighted
  instead.
- **Inline / Display** to write `$…$` or a `$$…$$` block. **Insert** (or **Update** for an
  existing formula, `Ctrl/Cmd+Enter`) writes the formula in one undo step; a ```` ```math ````
  block keeps its fence.

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
