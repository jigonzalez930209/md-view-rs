//! GitHub's code colors (Primer "prettylights") as syntect themes.
//!
//! github.com colors highlighted code with the `pl-*` classes of Primer's prettylights
//! palette. These themes map the same TextMate scopes to the same colors, light and
//! dark, so fenced code in the preview reads like code on GitHub.

use std::str::FromStr;

use syntect::highlighting::{Color, FontStyle, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSettings};

/// Prettylights colors: (comment, constant, entity, tag, keyword, string, variable,
/// regexp/markup-inserted, invalid, text).
struct Palette {
    comment: Color,
    constant: Color,
    entity: Color,
    tag: Color,
    keyword: Color,
    string: Color,
    variable: Color,
    regexp: Color,
    invalid: Color,
    text: Color,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r, g, b, a: 0xff }
}

const LIGHT: Palette = Palette {
    comment: rgb(0x59, 0x63, 0x6e),
    constant: rgb(0x05, 0x50, 0xae),
    entity: rgb(0x66, 0x39, 0xba),
    tag: rgb(0x05, 0x50, 0xae),
    keyword: rgb(0xcf, 0x22, 0x2e),
    string: rgb(0x0a, 0x30, 0x69),
    variable: rgb(0x95, 0x38, 0x00),
    regexp: rgb(0x11, 0x63, 0x29),
    invalid: rgb(0x82, 0x07, 0x1e),
    text: rgb(0x1f, 0x23, 0x28),
};

const DARK: Palette = Palette {
    comment: rgb(0x91, 0x98, 0xa1),
    constant: rgb(0x79, 0xc0, 0xff),
    entity: rgb(0xd2, 0xa8, 0xff),
    tag: rgb(0x7e, 0xe7, 0x87),
    keyword: rgb(0xff, 0x7b, 0x72),
    string: rgb(0xa5, 0xd6, 0xff),
    variable: rgb(0xff, 0xa6, 0x57),
    regexp: rgb(0x7e, 0xe7, 0x87),
    invalid: rgb(0xff, 0xa1, 0x98),
    text: rgb(0xe6, 0xed, 0xf3),
};

/// The GitHub light or dark code theme.
pub(crate) fn github_theme(light: bool) -> Theme {
    let palette = if light { &LIGHT } else { &DARK };
    // Scope lists follow prettylights: pl-c, pl-c1, pl-e/pl-en, pl-ent, pl-k, pl-s,
    // pl-v, pl-sr, pl-ii. Later, more specific selectors win in syntect.
    let rules: [(&str, Color, FontStyle); 14] = [
        (
            "comment, punctuation.definition.comment",
            palette.comment,
            FontStyle::empty(),
        ),
        (
            "constant, entity.name.constant, variable.other.constant, variable.language, support, \
             meta.property-name, support.constant, support.function, support.type, \
             support.class, meta.module-reference, storage.modifier.import",
            palette.constant,
            FontStyle::empty(),
        ),
        (
            "entity, entity.name, entity.name.function, meta.function-call entity.name",
            palette.entity,
            FontStyle::empty(),
        ),
        (
            "entity.name.tag, entity.other.attribute-name",
            palette.tag,
            FontStyle::empty(),
        ),
        (
            "keyword, storage, storage.type, storage.modifier",
            palette.keyword,
            FontStyle::empty(),
        ),
        (
            "string, punctuation.definition.string, string punctuation.section.embedded source, \
             string.quoted, constant.other.placeholder",
            palette.string,
            FontStyle::empty(),
        ),
        (
            "variable, variable.parameter.function",
            palette.variable,
            FontStyle::empty(),
        ),
        (
            "variable.other, variable.parameter, meta.function-call.arguments",
            palette.text,
            FontStyle::empty(),
        ),
        (
            "string.regexp, markup.inserted, constant.character.escape",
            palette.regexp,
            FontStyle::empty(),
        ),
        (
            "markup.deleted, invalid, invalid.illegal",
            palette.invalid,
            FontStyle::empty(),
        ),
        (
            "markup.heading, markup.heading entity.name",
            palette.constant,
            FontStyle::BOLD,
        ),
        ("markup.bold", palette.text, FontStyle::BOLD),
        // Interpolations read as code inside the string (pl-s1); GitHub colors their
        // braces with the variable color, measured from its render.
        (
            "meta.interpolation, string meta.interpolation, source.python.embedded",
            palette.text,
            FontStyle::empty(),
        ),
        (
            "punctuation.section.interpolation, punctuation.section.embedded",
            palette.variable,
            FontStyle::empty(),
        ),
    ];
    let scopes = rules
        .iter()
        .filter_map(|(selector, color, font_style)| {
            Some(ThemeItem {
                scope: ScopeSelectors::from_str(selector).ok()?,
                style: StyleModifier {
                    foreground: Some(*color),
                    background: None,
                    font_style: Some(*font_style),
                },
            })
        })
        .collect();
    Theme {
        name: Some(if light { "GitHub Light" } else { "GitHub Dark" }.to_owned()),
        author: Some("Primer prettylights".to_owned()),
        settings: ThemeSettings {
            foreground: Some(palette.text),
            ..ThemeSettings::default()
        },
        scopes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_parses() {
        assert_eq!(github_theme(true).scopes.len(), 14);
        assert_eq!(github_theme(false).scopes.len(), 14);
    }
}
