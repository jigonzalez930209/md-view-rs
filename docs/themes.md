# Themes, UI, and accessibility (`0.8.0`)

## Theme families

| Family | Variants | Mode |
|---|---|---|
| GitHub | GitHub Light, GitHub Dark, GitHub Dark Dimmed | light, dark, dark |
| Dracula | Dracula, Dracula Soft, Alucard | dark, dark, light |
| One Dark Pro | One Dark Pro, One Dark Pro Darker, One Dark Pro Light | dark, dark, light (derived) |

- IDs are stable and stored in settings: `github-light`, `github-dark`, `github-dark-dimmed`,
  `dracula`, `dracula-soft`, `alucard`, `one-dark-pro`, `one-dark-pro-darker`,
  `one-dark-pro-light`.
- Old `light`/`dark` values migrate to the GitHub variants.
- **Follow system** keeps the active family and switches between its light and dark base
  variants when the desktop reports a system theme change.
- Every token lives in `mdview-theme`. The shell, editor, preview, tables, code blocks, and media
  badges read from the token set; no UI crate invents a literal color.
- `ThemeMode` remains available for the editor's syntax palette and for tests.

## Contrast

`mdview-theme` computes WCAG 2.2 relative luminance and contrast ratios. Tests cover all nine
variants for:

- primary text on background and raised surface (≥ 4.5:1),
- muted text on background (≥ 4.5:1),
- links and danger text on background (≥ 4.5:1),
- accent text on the accent fill (≥ 4.5:1),
- focus rings on background (≥ 3:1).

## Layout and typography

- Reading content is centered in a 760-point column, roughly 70–80 characters at the default
  text size.
- The sidebar is collapsible and resizable; split mode stacks vertically below 760 points.
- Controls use a 28-point interaction size, 6-point radii, a 10-point item rhythm, and a visible
  focus stroke.
- Animation is limited to 100 ms hover/focus transitions and is disabled by **Reduce motion**.

## Settings

**Help → Settings…** (or `Cmd+,`) opens a single window for:

- theme variant and follow-system;
- text scale from 80% to 200%;
- language (follow system, English, Spanish, pseudo-locale);
- word wrap, line numbers, and split scroll synchronization;
- remote images, private-network images, SVG animations, and reduced motion.

Text scale uses egui's zoom factor and persists in `config.toml`. `Cmd+=`, `Cmd+-`, and `Cmd+0`
change or reset it without opening settings.

## Command palette and quick open

- `Cmd+Shift+P` opens the command palette with New, Open, Save, Settings, Find, Replace,
  Go to Line, modes, wrap, sidebar, About, and Quit.
- `Cmd+P` opens quick open with the current tabs and recent files.
- Both filter case-insensitively, support arrow-free selection, Enter for the first result, and
  Escape to close.
- English and Spanish catalogs are embedded; the pseudo-locale wraps every UI string in `[!…!]`
  so missing translations are visible.

## Snapshots

- `crates/mdview-render/tests/snapshots/theme-github-light.png`,
  `theme-dracula.png`, and `theme-one-dark-light.png` cover the reading view in three families.
- `crates/mdview-ui/tests/snapshots/settings.png` covers the settings window.
- `crates/mdview-ui/tests/snapshots/palette.png` covers the command palette in Dracula.
- The existing editor, preview, split, media, sidebar, tab, and conflict snapshots were
  regenerated for the new token palette.

## Known limits

- The typography stack uses the egui default fonts until the optional font subsets are vendored;
  the token layer already exposes UI, mono, and reading families for that change.
- Content-level media badges and syntax diagnostics use short English labels; the shell, menus,
  dialogs, settings, palette, and status strings are catalogued.
- Screen-reader verification on macOS and Windows remains part of the deferred 1.0.0 release gate.
