//! A system fallback font for Chinese, Japanese, and Korean text.
//!
//! egui's bundled fonts have no CJK glyphs, so such text was drawn as empty boxes in the
//! editor, the preview, and the shell. A CJK font is far too large to bundle; the installed
//! one is found and loaded on a background thread so startup does not wait for the font
//! scan, and registered as the lowest-priority fallback of every family.

use std::sync::atomic::{AtomicBool, Ordering};

use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use egui::{FontData, FontFamily};

use crate::ReadingFace;

/// Families tried in order: Noto/Source Han first (Linux, many installs), then the
/// platform defaults of Windows and macOS, then common Linux fallbacks.
const CANDIDATES: &[&str] = &[
    "Noto Sans CJK JP",
    "Noto Sans CJK SC",
    "Noto Sans JP",
    "Source Han Sans",
    "Source Han Sans JP",
    "Microsoft YaHei",
    "Yu Gothic",
    "Meiryo",
    "Malgun Gothic",
    "PingFang SC",
    "Hiragino Sans",
    "Hiragino Kaku Gothic ProN",
    "Apple SD Gothic Neo",
    "WenQuanYi Micro Hei",
    "Droid Sans Fallback",
];

const NAME: &str = "mdview-cjk-fallback";

static STARTED: AtomicBool = AtomicBool::new(false);

/// Find an installed CJK font and add it as a fallback, once per process.
///
/// Returns immediately; the font becomes active on a later frame. Set
/// `MDVIEW_NO_SYSTEM_FONTS=1` to skip it, for example for pixel-exact snapshots.
pub fn add_cjk_fallback(ctx: &egui::Context) {
    if std::env::var_os("MDVIEW_NO_SYSTEM_FONTS").is_some() || STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let ctx = ctx.clone();
    let spawned = std::thread::Builder::new()
        .name("mdview-cjk-font".to_owned())
        .spawn(move || {
            if let Some((data, index)) = find_font() {
                insert(&ctx, data, index);
                ctx.request_repaint();
            }
        });
    if spawned.is_err() {
        STARTED.store(false, Ordering::SeqCst);
    }
}

fn find_font() -> Option<(Vec<u8>, u32)> {
    let mut database = fontdb::Database::new();
    database.load_system_fonts();
    for name in CANDIDATES {
        let query = fontdb::Query {
            families: &[fontdb::Family::Name(name)],
            ..fontdb::Query::default()
        };
        let Some(id) = database.query(&query) else {
            continue;
        };
        let found = database.with_face_data(id, |data, index| (data.to_vec(), index));
        if found.is_some() {
            return found;
        }
    }
    None
}

fn insert(ctx: &egui::Context, data: Vec<u8>, index: u32) {
    let mut families = vec![FontFamily::Proportional, FontFamily::Monospace];
    families.extend(
        [
            ReadingFace::Regular,
            ReadingFace::SemiBold,
            ReadingFace::Bold,
            ReadingFace::Italic,
            ReadingFace::BoldItalic,
        ]
        .map(ReadingFace::family),
    );
    let mut font = FontData::from_owned(data);
    font.index = index;
    ctx.add_font(FontInsert {
        name: NAME.to_owned(),
        data: font,
        families: families
            .into_iter()
            .map(|family| InsertFontFamily {
                family,
                priority: FontPriority::Lowest,
            })
            .collect(),
    });
}
