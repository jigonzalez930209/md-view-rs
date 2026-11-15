# File and folder icons

The workspace tree uses the **VS Code Material Icon Theme** specification, vendored from
`material-extensions/vscode-material-icon-theme` under its MIT license.

- `assets/material-icons/material-icons.json` is the upstream manifest.
- `assets/material-icons/icons/` contains the upstream SVGs, including the generated open-folder
  variants.
- `assets/material-icons/LICENSE` is the upstream MIT license.
- `crates/mdview-ui/src/material_icons.rs` resolves icons in the same order as the extension:
  exact file name, extension, compound extension, folder name, expanded folder name, then the
  default file/folder/root icons.
- Light-theme overrides from the manifest (`light.fileNames`, `light.fileExtensions`,
  `light.folderNames`, `light.folderNamesExpanded`) are applied; dark themes use the base mapping.
- Icons are rasterized at the device pixel ratio for their requested display size (16–18 pt in
  the tree) and cached per size. The tree only requests icons for loaded rows.
- `README.md` intentionally uses the Material “readme” icon (an info mark). It is a file-kind
  icon, not a status badge; every other Markdown file uses the Markdown icon.
- Special folder names (`src`, `docs`, `assets`, …) use their colored Material variants; ordinary
  folders use the neutral default. This is the upstream specification, not an inconsistency.
- UI action icons live in `assets/ui-icons/` and are tinted with the active theme text color.
- The title bar, tabs, tree, and dialogs use the size tokens in
  `crates/mdview-ui/src/metrics.rs` (`ICON_SMALL` 14, `ICON_MEDIUM` 16, `ICON_LARGE` 20).

The sidebar snapshot is regenerated with `UPDATE_SNAPSHOTS=1 cargo test -p mdview-app --bin mdview`.
