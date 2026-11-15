//! Media warm-up before the first preview layout.
//!
//! Layout estimates for images, diagrams, and formulas are only as good as the
//! intrinsic size of the finished raster. Warming every request before the first
//! paint keeps lazy rasterization from reflowing the document while the reader is
//! at the end of a split view.

use mdview_markdown::{Block, BlockKind, Inline};
use mdview_media::MediaKind;

use crate::{MediaContext, MediaState, Preview};

impl Preview {
    /// Resolve every media request in the document once. Returns `false` while any
    /// request is still loading, or the reading fonts are not active yet, so the caller
    /// can avoid painting an unstable layout.
    pub(crate) fn warm_up_media(&mut self, ui: &egui::Ui, media: Option<&mut MediaContext<'_>>, dark: bool) -> bool {
        // Reading fonts registered during this pass are active from the next one.
        if !mdview_theme::install_reading_fonts(ui.ctx()) {
            ui.ctx().request_repaint();
            return false;
        }
        if !self.media_warmup {
            return true;
        }
        let Some(context) = media else {
            self.media_warmup = false;
            return true;
        };
        let Some(cached) = &self.cached else {
            return true;
        };
        let mut sources = Vec::new();
        for block in &cached.document.blocks {
            collect_block(block, &mut sources);
        }
        let mut pending = false;
        for (kind, source) in sources {
            let request_width = match kind {
                MediaKind::Image | MediaKind::Svg => 1_600.0,
                MediaKind::Math => 1_024.0,
                MediaKind::Mermaid => 1_200.0,
            };
            if !self.media_sizes.contains_key(&source)
                && let Some(size) = mdview_media::intrinsic_size(kind, &source, context.base.as_deref(), dark)
            {
                self.media_sizes.insert(source.clone(), size);
            }
            let request = context.request(ui.ctx(), kind, &source, request_width, dark);
            if matches!(context.provider.resolve(ui.ctx(), &request), MediaState::Loading) {
                pending = true;
            }
        }
        if pending {
            return false;
        }
        self.media_warmup = false;
        true
    }
}

fn collect_block(block: &Block, out: &mut Vec<(MediaKind, String)>) {
    match &block.kind {
        BlockKind::Paragraph(inlines) | BlockKind::Heading { inlines, .. } => {
            collect_inlines(inlines, out);
        }
        BlockKind::Code { language, text, math } => {
            if *math {
                out.push((MediaKind::Math, text.clone()));
            } else if language
                .as_deref()
                .is_some_and(|language| language.eq_ignore_ascii_case("mermaid"))
            {
                out.push((MediaKind::Mermaid, text.clone()));
            }
        }
        BlockKind::BlockQuote { blocks, .. } | BlockKind::FootnoteDefinition { blocks, .. } => {
            for block in blocks {
                collect_block(block, out);
            }
        }
        BlockKind::List { items, .. } => {
            for item in items {
                for block in &item.blocks {
                    collect_block(block, out);
                }
            }
        }
        BlockKind::Table { head, rows, .. } => {
            for cell in head {
                collect_inlines(cell, out);
            }
            for row in rows {
                for cell in row {
                    collect_inlines(cell, out);
                }
            }
        }
        BlockKind::Html(_) | BlockKind::Rule => {}
    }
}

fn collect_inlines(inlines: &[Inline], out: &mut Vec<(MediaKind, String)>) {
    for inline in inlines {
        match inline {
            Inline::Image { source, .. } => out.push((MediaKind::Image, source.clone())),
            Inline::Math { tex, display } => {
                out.push((MediaKind::Math, crate::layout::math_source(tex, *display).into_owned()))
            }
            Inline::Emphasis(children) | Inline::Strong(children) | Inline::Strikethrough(children) => {
                collect_inlines(children, out)
            }
            Inline::Link { children, .. } => collect_inlines(children, out),
            _ => {}
        }
    }
}
