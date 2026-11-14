//! Language detection and syntax highlighting.
//!
//! The first implementation uses syntect's Sublime syntaxes. Highlighting is computed for the
//! whole document and cached by the editor; large-file mode disables it.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

/// A colored run of text on one line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// The source text.
    pub text: String,
    /// The foreground color, if the syntax provides one.
    pub color: Option<[u8; 3]>,
}

/// Highlighted lines, parallel to the source lines.
#[derive(Clone, Debug, Default)]
pub struct Highlighted {
    /// One span list per line.
    pub lines: Vec<Vec<Span>>,
}

impl Highlighted {
    /// Plain text for a line.
    pub fn plain(text: &str) -> Vec<Span> {
        vec![Span {
            text: text.to_owned(),
            color: None,
        }]
    }
}

/// A syntax highlighter with cached syntax and theme sets.
pub struct Highlighter {
    syntaxes: SyntaxSet,
    themes: ThemeSet,
}

impl Highlighter {
    /// Build the highlighter.
    pub fn new() -> Self {
        Self {
            syntaxes: two_face::syntax::extra_newlines(),
            themes: ThemeSet::load_defaults(),
        }
    }

    /// A process-wide highlighter shared by every editor and preview.
    pub fn shared() -> Arc<Self> {
        static SHARED: OnceLock<Arc<Highlighter>> = OnceLock::new();
        SHARED.get_or_init(|| Arc::new(Highlighter::new())).clone()
    }

    /// Highlight a document. `light` selects a light or dark syntax palette.
    pub fn highlight(&self, text: &str, path: Option<&Path>, light: bool) -> Highlighted {
        let syntax = detect_syntax(&self.syntaxes, path, text);
        let Some(theme) = theme(&self.themes, light) else {
            return Highlighted {
                lines: text.lines().map(Highlighted::plain).collect(),
            };
        };
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut lines = Vec::new();
        for line in LinesWithEndings::from(text) {
            let line = line.trim_end_matches(['\r', '\n']);
            match highlighter.highlight_line(line, &self.syntaxes) {
                Ok(runs) => {
                    let spans = runs
                        .into_iter()
                        .filter(|(_, text)| !text.is_empty())
                        .map(|(style, text)| Span {
                            text: text.to_owned(),
                            color: color(style),
                        })
                        .collect();
                    lines.push(spans);
                }
                Err(_) => lines.push(Highlighted::plain(line)),
            }
        }
        if lines.is_empty() {
            lines.push(Vec::new());
        }
        Highlighted { lines }
    }

    /// The language name for the status bar.
    pub fn language_name(&self, path: Option<&Path>, text: &str) -> String {
        detect_syntax(&self.syntaxes, path, text).name.clone()
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

fn theme(themes: &ThemeSet, light: bool) -> Option<&Theme> {
    let name = if light { "InspiredGitHub" } else { "base16-ocean.dark" };
    themes.themes.get(name).or_else(|| themes.themes.values().next())
}

fn color(style: Style) -> Option<[u8; 3]> {
    let color = style.foreground;
    if color.a == 0 {
        None
    } else {
        Some([color.r, color.g, color.b])
    }
}

fn detect_syntax<'a>(syntaxes: &'a SyntaxSet, path: Option<&Path>, text: &str) -> &'a SyntaxReference {
    if let Some(path) = path {
        if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
            let manual = match extension.to_ascii_lowercase().as_str() {
                "kt" | "kts" => Some("Kotlin"),
                "swift" => Some("Swift"),
                "rb" => Some("Ruby"),
                "php" => Some("PHP"),
                "graphql" | "gql" => Some("GraphQL"),
                "ini" => Some("INI"),
                "diff" | "patch" => Some("Diff"),
                "jsx" => Some("JavaScript (Babel)"),
                "md" | "markdown" => Some("Markdown"),
                "txt" => Some("Plain Text"),
                _ => None,
            };
            if let Some(syntax) = manual.and_then(|name| syntaxes.find_syntax_by_name(name)) {
                return syntax;
            }
            if let Some(syntax) = syntaxes.find_syntax_by_extension(extension) {
                return syntax;
            }
        }
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            if let Some(syntax) = syntaxes.find_syntax_by_name(name) {
                return syntax;
            }
            let lower = name.to_ascii_lowercase();
            if lower == "dockerfile"
                && let Some(syntax) = syntaxes.find_syntax_by_extension("dockerfile")
            {
                return syntax;
            }
            if lower == "makefile"
                && let Some(syntax) = syntaxes.find_syntax_by_extension("makefile")
            {
                return syntax;
            }
        }
    }
    if let Some(first) = text.lines().next()
        && let Some(command) = first.strip_prefix("#!")
        && let Some(syntax) = syntaxes.find_syntax_by_first_line(command)
    {
        return syntax;
    }
    syntaxes.find_syntax_plain_text()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rust_and_highlights_keywords() {
        let highlighter = Highlighter::new();
        let highlighted = highlighter.highlight("fn main() { let x = 1; }\n", Some(Path::new("x.rs")), false);
        assert_eq!(highlighted.lines.len(), 1);
        assert!(highlighted.lines[0].iter().any(|span| span.text.contains("fn")));
        assert_eq!(highlighter.language_name(Some(Path::new("x.rs")), ""), "Rust");
    }

    #[test]
    fn falls_back_to_plain_text() {
        let highlighter = Highlighter::new();
        let highlighted = highlighter.highlight("hello\n", Some(Path::new("x.unknown")), true);
        assert_eq!(highlighted.lines.len(), 1);
        assert!(!highlighted.lines[0].is_empty());
        assert!(
            highlighter
                .language_name(Some(Path::new("x.unknown")), "")
                .contains("Plain")
        );
    }
}

#[cfg(test)]
mod language_tests {
    use std::path::Path;

    use super::Highlighter;

    #[test]
    fn curated_languages_are_detected() {
        let highlighter = Highlighter::new();
        let cases = [
            ("README.md", "Markdown"),
            ("notes.txt", "Plain Text"),
            ("main.rs", "Rust"),
            ("app.py", "Python"),
            ("app.js", "JavaScript"),
            ("app.ts", "TypeScript"),
            ("app.tsx", "TypeScript"),
            ("data.json", "JSON"),
            ("config.yaml", "YAML"),
            ("Cargo.toml", "TOML"),
            ("index.html", "HTML"),
            ("style.css", "CSS"),
            ("style.scss", "SCSS"),
            ("data.xml", "XML"),
            ("query.sql", "SQL"),
            ("run.sh", "Bourne Again Shell (bash)"),
            ("main.c", "C"),
            ("main.cpp", "C++"),
            ("Program.cs", "C#"),
            ("main.go", "Go"),
            ("Main.java", "Java"),
            ("Main.kt", "Kotlin"),
            ("Main.swift", "Swift"),
            ("index.php", "PHP"),
            ("app.rb", "Ruby"),
            ("init.lua", "Lua"),
            ("Dockerfile", "Dockerfile"),
            ("Makefile", "Makefile"),
            ("config.ini", "INI"),
            ("change.diff", "Diff"),
            ("schema.graphql", "GraphQL"),
        ];
        for (file, expected) in cases {
            let name = highlighter.language_name(Some(Path::new(file)), "");
            assert!(
                name.to_ascii_lowercase().contains(&expected.to_ascii_lowercase()),
                "{file}: expected {expected}, got {name}"
            );
        }
    }
}

/// A lazy per-line highlight cache.
///
/// Syntect needs the parse state of every preceding line. The cache keeps the state after each
/// parsed line, so scrolling forward and editing near the cursor only replays the affected range
/// instead of the whole document.
#[derive(Default)]
pub struct HighlightCache {
    lines: Vec<Option<Vec<Span>>>,
    states: Vec<Option<(syntect::highlighting::HighlightState, syntect::parsing::ParseState)>>,
    key: Option<(String, bool)>,
}

impl HighlightCache {
    /// A cached line, if it has been highlighted.
    pub fn line(&self, index: usize) -> Option<&[Span]> {
        self.lines.get(index)?.as_deref()
    }

    /// Whether every line in the inclusive range is cached.
    pub fn covers(&self, start: usize, end: usize) -> bool {
        start <= end && (start..=end).all(|line| self.lines.get(line).is_some_and(Option::is_some))
    }

    /// Drop cached lines and states at and after `line`.
    pub fn invalidate_from(&mut self, line: usize) {
        let line = line.min(self.lines.len());
        self.lines.truncate(line);
        self.states.truncate(line);
    }

    /// Forget everything, for example after opening another document.
    pub fn reset(&mut self) {
        self.lines.clear();
        self.states.clear();
        self.key = None;
    }

    fn prepare(&mut self, path: Option<&Path>, light: bool, count: usize) {
        let key = (
            path.map(|path| path.to_string_lossy().into_owned()).unwrap_or_default(),
            light,
        );
        if self.key.as_ref() != Some(&key) {
            self.reset();
            self.key = Some(key);
        }
        self.lines.resize(count, None);
        self.states.resize(count, None);
    }
}

impl Highlighter {
    /// Ensure the lines in `[start, end]` are cached.
    pub fn ensure_range(
        &self,
        text: &str,
        path: Option<&Path>,
        light: bool,
        start: usize,
        end: usize,
        cache: &mut HighlightCache,
    ) {
        let source_lines: Vec<&str> = text.split_inclusive('\n').collect();
        let count = source_lines.len().max(1);
        let end = end.min(count.saturating_sub(1));
        let start = start.min(end);
        cache.prepare(path, light, count);
        if (start..=end).all(|line| cache.lines.get(line).is_some_and(Option::is_some)) {
            return;
        }

        let syntax = detect_syntax(&self.syntaxes, path, text);
        let Some(theme) = theme(&self.themes, light) else {
            for line in start..=end {
                if let Some(slot) = cache.lines.get_mut(line) {
                    *slot = Some(Highlighted::plain(source_lines.get(line).copied().unwrap_or("")));
                }
            }
            return;
        };

        let first_uncached = (0..=start)
            .rev()
            .find(|line| cache.states.get(*line).is_some_and(Option::is_some))
            .map_or(start, |line| line + 1);
        // Jumping far from the cached viewport must not re-highlight every line in between.
        // Start from the visible line with a fresh parser state; colours are approximate for
        // that frame but scrolling stays inside the frame budget.
        let first_uncached = if start.saturating_sub(first_uncached) > 128 {
            start
        } else {
            first_uncached.min(start)
        };
        let mut highlighter = if first_uncached == 0 {
            HighlightLines::new(syntax, theme)
        } else if let Some(Some((highlight_state, parse_state))) = cache.states.get(first_uncached - 1).cloned() {
            HighlightLines::from_state(theme, highlight_state, parse_state)
        } else {
            HighlightLines::new(syntax, theme)
        };

        for line in first_uncached..=end {
            let source = source_lines.get(line).copied().unwrap_or("");
            let spans = match highlighter.highlight_line(source.trim_end_matches(['\r', '\n']), &self.syntaxes) {
                Ok(runs) => runs
                    .into_iter()
                    .filter(|(_, text)| !text.is_empty())
                    .map(|(style, text)| Span {
                        text: text.to_owned(),
                        color: color(style),
                    })
                    .collect(),
                Err(_) => Highlighted::plain(source),
            };
            if let Some(slot) = cache.lines.get_mut(line) {
                *slot = Some(spans);
            }
            let (highlight_state, parse_state) = highlighter.state();
            if let Some(slot) = cache.states.get_mut(line) {
                *slot = Some((highlight_state.clone(), parse_state.clone()));
            }
            highlighter = HighlightLines::from_state(theme, highlight_state, parse_state);
        }
    }
}
