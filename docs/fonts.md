# Reading fonts

The Markdown preview follows GitHub's Markdown body, whose font stack
(`-apple-system, "Segoe UI", "Noto Sans", Helvetica, Arial, sans-serif`) resolves to
Noto Sans on Linux. The preview bundles five static Noto Sans v2.015 faces so semibold, bold, and italic
text use real weights and slants:

- `assets/fonts/NotoSans-Regular.ttf`
- `assets/fonts/NotoSans-SemiBold.ttf`
- `assets/fonts/NotoSans-Bold.ttf`
- `assets/fonts/NotoSans-Italic.ttf`
- `assets/fonts/NotoSans-BoldItalic.ttf`

They are licensed under the SIL Open Font License 1.1; see `assets/fonts/OFL.txt`.

`mdview-theme` embeds the files and registers them as the `ReadingFace` families
(`add_reading_fonts`, called by every theme apply). The application shell keeps egui's
default UI fonts; only preview text uses the reading faces. `mdview-render` checks that
the faces are active before laying out a document and waits one frame when they were
just registered.
