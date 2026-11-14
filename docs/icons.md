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
- Icons are rasterized once at 32 px and cached as egui textures. The tree only requests icons for
  loaded rows.
- UI action icons live in `assets/ui-icons/` and are tinted with the active theme text color.

The sidebar snapshot is regenerated with `UPDATE_SNAPSHOTS=1 cargo test -p mdview-app --bin mdview`.
