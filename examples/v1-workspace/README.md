# V1 feature workspace

This folder is the manual and automated acceptance workspace for everything MD View must
ship before `1.0`. Five folders are nested one inside the next, with Markdown and source or
data files alternating at every level.

## Walk the tree

1. Open `level1/notes.md` to check nested navigation.
2. Continue to `level1/level2/guide.md`.
3. Continue to `level1/level2/level3/reference.md`.
4. Continue to `level1/level2/level3/level4/level5/showcase.md`.

`showcase.md` is the full render matrix: LaTeX, tables, code highlighting, local PNG and
SVG images, Mermaid diagrams, task lists, alerts, and footnotes.

## Media viewers

`media/` holds sample files for the image, animation, PDF, video, and table viewers; its
`README.md` is the checklist.

## Root files

- `config.json` for JSON highlighting.
- `level1/build.sh` for shell highlighting.

> [!TIP]
> Keep this folder open while expanding all five levels; the explorer must not collapse
> folders when the watcher refreshes the tree.
