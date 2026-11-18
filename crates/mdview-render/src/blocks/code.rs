//! Fenced code, display math, and Mermaid blocks.

use super::*;

pub(super) fn show_code(
    ui: &mut Ui,
    language: Option<&str>,
    text: &str,
    math: bool,
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    if math && show_media_block(ui, MediaKind::Math, text, ctx, media) {
        return;
    }
    if language.is_some_and(|language| language.eq_ignore_ascii_case("mermaid")) {
        if show_media_block(ui, MediaKind::Mermaid, text, ctx, media) {
            return;
        }
        ui.label(
            RichText::new("Could not render this Mermaid diagram; showing the source.")
                .small()
                .color(ctx.theme.danger),
        );
    }
    show_plain_code(ui, language, text, math, ctx);
}

fn show_media_block(
    ui: &mut Ui,
    kind: MediaKind,
    source: &str,
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) -> bool {
    let Some(media_context) = media.as_mut().map(|context| &mut **context) else {
        return false;
    };
    let width = ui.available_width().max(64.0);
    // Vector media are rasterized once at a stable width. Split resizing then rescales
    // the existing texture instead of re-rendering diagrams and formulas.
    let request_width = match kind {
        MediaKind::Mermaid => 1_200.0,
        MediaKind::Math => 1_024.0,
        MediaKind::Image | MediaKind::Svg => width,
    };
    let request = media_context.request(ui.ctx(), kind, source, request_width, ctx.theme.is_dark());
    match media_context.provider.resolve(ui.ctx(), &request) {
        MediaState::Loading => {
            let height = crate::layout::reserved_media_height(kind, source, width, ctx.typography, ctx.media_sizes);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Rendering...",
                FontId::proportional(12.0),
                ctx.theme.text_muted,
            );
            true
        }
        MediaState::Ready(texture) => {
            // Formula rasters are one pixel per SVG unit and scale with the math font
            // size. Diagrams fit the column like GitHub's rendered Mermaid blocks.
            let points = if kind == MediaKind::Math {
                texture.size_vec2() * crate::layout::math_scale(ctx.typography)
            } else {
                texture.size_vec2() / ui.ctx().pixels_per_point().max(1.0)
            };
            // Reserve one stable box per media kind. Both the placeholder and the
            // finished texture occupy the same height, so lazy rasterization cannot
            // reflow the document or shift the reader's position.
            let reserved = crate::layout::reserved_media_height(kind, source, width, ctx.typography, ctx.media_sizes);
            if kind == MediaKind::Math {
                // The texture keeps its padding and is centered on the TeX box, so it
                // may overhang the reserved height by that transparent margin.
                let size = points * (width / points.x).min(1.0);
                let (rect, response) = ui.allocate_exact_size(egui::vec2(width, reserved), egui::Sense::click());
                ui.painter().image(
                    texture.id(),
                    egui::Rect::from_center_size(rect.center(), size),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                // Click inspects the formula; the context menu edits it (a double-click
                // cannot, because the first click already opened the lightbox).
                crate::media::math_edit_request(&response, source, ctx.output);
                if response.on_hover_cursor(egui::CursorIcon::ZoomIn).clicked() && ctx.output.math.is_none() {
                    ctx.output.lightbox = Some(MediaLightbox {
                        request,
                        alt: format!("{} view", kind.label()),
                    });
                }
                return true;
            }
            let scale = (width / points.x).min(reserved / points.y).min(1.0);
            let size = points * scale;
            let response = ui
                .allocate_ui_with_layout(egui::vec2(width, reserved), Layout::top_down(Align::Center), |ui| {
                    ui.add_space(((reserved - size.y) * 0.5).max(0.0));
                    ui.add(
                        egui::Image::new(&texture)
                            .fit_to_exact_size(size)
                            .sense(egui::Sense::click()),
                    )
                    .on_hover_cursor(egui::CursorIcon::ZoomIn)
                })
                .inner;
            if response.clicked() {
                ctx.output.lightbox = Some(MediaLightbox {
                    request,
                    alt: format!("{} view", kind.label()),
                });
            }
            true
        }
        MediaState::Error(error) => {
            ui.label(RichText::new(error).small().color(ctx.theme.danger));
            false
        }
    }
}

/// A GitHub-style code block: a subtle rounded surface with 16 px padding, a 1.45
/// line height, and a copy button that appears on hover.
fn show_plain_code(ui: &mut Ui, language: Option<&str>, text: &str, math: bool, ctx: &mut BlockCtx<'_>) {
    let code_size = ctx.typography.code;
    // GitHub: line-height 1.45 without rounding, so long blocks keep the same pitch.
    let line_height = code_size * 1.45;
    // GitHub: 16 px padding at a 16 px body.
    let padding = ctx.typography.body.round();
    let frame = egui::Frame::new()
        .fill(ctx.theme.code_background)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::same(padding as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            if math {
                ui.label(
                    RichText::new(text.trim_end())
                        .monospace()
                        .size(code_size)
                        .line_height(Some(line_height))
                        .color(ctx.theme.text),
                );
                return;
            }
            let path = language_path(language);
            let highlighted = ctx.highlighter.highlight(text, path.as_deref(), ctx.light);
            let mut job = egui::text::LayoutJob::default();
            // Code lines keep their indentation and scroll horizontally, like GitHub and
            // desktop editors, instead of soft-wrapping in the middle of a token.
            job.wrap.max_width = f32::INFINITY;
            let format = |color| TextFormat {
                font_id: FontId::monospace(code_size),
                color,
                line_height: Some(line_height),
                ..Default::default()
            };
            for (index, line) in highlighted.lines.iter().enumerate() {
                for span in line {
                    let color = span
                        .color
                        .map(|[r, g, b]| Color32::from_rgb(r, g, b))
                        .unwrap_or(ctx.theme.text);
                    let color = if mdview_theme::contrast_ratio(color, ctx.theme.code_background) < 3.0 {
                        ctx.theme.text
                    } else {
                        color
                    };
                    job.append(&span.text, 0.0, format(color));
                }
                if index + 1 < highlighted.lines.len() {
                    job.append("\n", 0.0, format(ctx.theme.text));
                }
            }
            egui::ScrollArea::horizontal()
                .id_salt((language, text.len(), text.as_ptr()))
                .auto_shrink([false, true])
                .scroll_source(egui::scroll_area::ScrollSource {
                    scroll_bar: true,
                    drag: egui::scroll_area::DragScroll::Always,
                    mouse_wheel: false,
                })
                .show(ui, |ui| {
                    ui.add(Label::new(job).selectable(true).wrap_mode(egui::TextWrapMode::Extend));
                });
        });
    let rect = frame.response.rect;
    if ui.rect_contains_pointer(rect) {
        let button_rect = egui::Rect::from_min_size(
            egui::pos2(rect.right() - 52.0, rect.top() + 6.0),
            egui::vec2(46.0, 22.0),
        );
        let copy = ui
            .put(button_rect, egui::Button::new(RichText::new("Copy").size(11.5)))
            .on_hover_text("Copy code");
        if copy.clicked() {
            ui.ctx().copy_text(text.to_owned());
            ctx.output.copied = true;
        }
    }
}

fn language_path(language: Option<&str>) -> Option<PathBuf> {
    let language = language?;
    let extension = match language.to_ascii_lowercase().as_str() {
        "rust" | "rs" => "rs",
        "javascript" | "js" => "js",
        "typescript" | "ts" => "ts",
        "python" | "py" => "py",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "html" => "html",
        "css" => "css",
        "scss" => "scss",
        "xml" => "xml",
        "sql" => "sql",
        "sh" | "bash" | "shell" => "sh",
        "c" => "c",
        "cpp" | "c++" => "cpp",
        "csharp" | "cs" => "cs",
        "go" => "go",
        "java" => "java",
        "kotlin" | "kt" => "kt",
        "swift" => "swift",
        "php" => "php",
        "ruby" | "rb" => "rb",
        "lua" => "lua",
        "dockerfile" => "dockerfile",
        "makefile" => "makefile",
        "ini" => "ini",
        "diff" | "patch" => "diff",
        "graphql" | "gql" => "graphql",
        "markdown" | "md" => "md",
        _ => return None,
    };
    Some(PathBuf::from(format!("code.{extension}")))
}
