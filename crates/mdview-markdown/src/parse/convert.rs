//! Event-to-AST value conversions.

use pulldown_cmark::{Alignment as CmAlignment, BlockQuoteKind, HeadingLevel};

use crate::ast::{Alignment, QuoteKind};

pub(super) fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

pub(super) fn quote_kind(kind: Option<BlockQuoteKind>) -> QuoteKind {
    match kind {
        Some(BlockQuoteKind::Note) => QuoteKind::Note,
        Some(BlockQuoteKind::Tip) => QuoteKind::Tip,
        Some(BlockQuoteKind::Important) => QuoteKind::Important,
        Some(BlockQuoteKind::Warning) => QuoteKind::Warning,
        Some(BlockQuoteKind::Caution) => QuoteKind::Caution,
        None => QuoteKind::Quote,
    }
}

pub(super) fn alignment(alignment: CmAlignment) -> Alignment {
    match alignment {
        CmAlignment::None => Alignment::None,
        CmAlignment::Left => Alignment::Left,
        CmAlignment::Center => Alignment::Center,
        CmAlignment::Right => Alignment::Right,
    }
}

pub(super) fn empty_to_none(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}
