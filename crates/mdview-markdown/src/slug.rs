//! GitHub-style heading anchors.

use std::collections::HashMap;

/// Converts heading text to unique, stable anchors.
#[derive(Debug, Default)]
pub struct Slugger {
    used: HashMap<String, usize>,
}

impl Slugger {
    /// Build an anchor for heading text, appending a suffix when it collides.
    pub fn slug(&mut self, text: &str) -> String {
        let base = github_slug(text);
        let base = if base.is_empty() { "section".to_owned() } else { base };
        self.unique(base)
    }

    /// Reserve an explicit anchor, appending a suffix when it collides.
    pub fn unique(&mut self, base: String) -> String {
        let count = self.used.entry(base.clone()).or_insert(0);
        let slug = if *count == 0 {
            base.clone()
        } else {
            format!("{base}-{count}")
        };
        *count += 1;
        slug
    }
}

/// GitHub's anchor transformation: lowercase, strip punctuation, spaces to hyphens.
pub fn github_slug(text: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for character in text.chars() {
        if character.is_whitespace() {
            pending_dash = !slug.is_empty();
            continue;
        }
        if character.is_alphanumeric() || character == '-' || character == '_' {
            if pending_dash && !slug.ends_with('-') {
                slug.push('-');
            }
            pending_dash = false;
            for lower in character.to_lowercase() {
                slug.push(lower);
            }
        }
    }
    slug.trim_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_follow_github_rules() {
        assert_eq!(github_slug("Hello, World!"), "hello-world");
        assert_eq!(github_slug("  Spaced  Out  "), "spaced-out");
        assert_eq!(github_slug("CamelCase_and-dash"), "camelcase_and-dash");
        assert_eq!(github_slug("Acentos: ñandú"), "acentos-ñandú");
        assert_eq!(github_slug("!!!"), "");
    }

    #[test]
    fn duplicate_headings_get_numeric_suffixes() {
        let mut slugger = Slugger::default();
        assert_eq!(slugger.slug("Intro"), "intro");
        assert_eq!(slugger.slug("Intro"), "intro-1");
        assert_eq!(slugger.slug("Intro"), "intro-2");
        assert_eq!(slugger.slug("Other"), "other");
    }
}
