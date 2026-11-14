//! Applying tokens to an egui context.

use craft_ui::theme::WidgetPalette;
use egui::{CornerRadius, Stroke, TextStyle};

use crate::{Theme, ThemeId, ThemeMode};

/// Resolve a theme against the system preference.
pub fn effective_id(id: ThemeId, follow_system: bool, system_dark: bool) -> ThemeId {
    if follow_system {
        id.follow_system(system_dark)
    } else {
        id
    }
}

/// Apply a base light or dark theme. Kept for callers that do not need the full family.
pub fn apply(ctx: &egui::Context, mode: ThemeMode) {
    let id = match mode {
        ThemeMode::Light => ThemeId::GithubLight,
        ThemeMode::Dark => ThemeId::GithubDark,
    };
    apply_id(ctx, id);
}

/// Apply a specific theme identifier.
pub fn apply_id(ctx: &egui::Context, id: ThemeId) {
    let theme = Theme::for_id(id);
    let mut visuals = match id.mode() {
        ThemeMode::Light => egui::Visuals::light(),
        ThemeMode::Dark => egui::Visuals::dark(),
    };
    visuals.panel_fill = theme.background;
    visuals.window_fill = theme.surface;
    visuals.faint_bg_color = theme.hover;
    visuals.extreme_bg_color = theme.sunken;
    visuals.hyperlink_color = theme.link;
    visuals.selection.bg_fill = theme.selection;
    visuals.selection.stroke = Stroke::new(1.0, theme.text);
    visuals.window_stroke = Stroke::new(1.0, theme.border);

    WidgetPalette {
        noninteractive: theme.background,
        inactive: theme.surface,
        hovered: theme.hover,
        active: theme.accent,
        open: theme.surface,
    }
    .apply_to(
        &mut visuals.widgets,
        CornerRadius::same(6),
        Stroke::new(1.0, theme.focus),
    );

    visuals.widgets.inactive.weak_bg_fill = theme.surface;
    visuals.widgets.inactive.bg_stroke = theme.border_stroke();
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, theme.focus);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, theme.text);
    visuals.widgets.active.bg_stroke = Stroke::new(2.0, theme.focus);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, theme.border);
    visuals.override_text_color = Some(theme.text);
    visuals.window_corner_radius = CornerRadius::same(8);

    ctx.all_styles_mut(|style| {
        style.visuals = visuals.clone();
        style.spacing.item_spacing = egui::vec2(8.0, 4.0);
        style.spacing.button_padding = egui::vec2(8.0, 3.0);
        style.spacing.interact_size = egui::vec2(26.0, 24.0);
        style.spacing.menu_margin = egui::Margin::same(4);
        style.spacing.scroll.bar_width = 6.0;
        style.spacing.scroll.floating_width = 3.0;
        style.spacing.scroll.floating_allocated_width = 4.0;
        style.spacing.scroll.bar_inner_margin = 2.0;
        style.animation_time = 0.10;
        style
            .text_styles
            .insert(TextStyle::Body, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(TextStyle::Button, egui::FontId::proportional(12.5));
        style
            .text_styles
            .insert(TextStyle::Small, egui::FontId::proportional(11.0));
        style
            .text_styles
            .insert(TextStyle::Monospace, egui::FontId::monospace(13.0));
        style
            .text_styles
            .insert(TextStyle::Heading, egui::FontId::proportional(20.0));
    });
}

/// Apply a theme, following the system light/dark preference when requested.
pub fn apply_theme(ctx: &egui::Context, id: ThemeId, follow_system: bool) {
    let system_dark = ctx
        .input(|input| input.raw.system_theme)
        .is_some_and(|theme| theme == egui::Theme::Dark);
    apply_id(ctx, effective_id(id, follow_system, system_dark));
}

/// Set the animation time for reduced-motion preferences.
pub fn set_reduced_motion(ctx: &egui::Context, reduced: bool) {
    ctx.all_styles_mut(|style| {
        style.animation_time = if reduced { 0.0 } else { 0.10 };
    });
}
