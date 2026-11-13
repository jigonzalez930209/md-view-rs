//! UI translations for MD View.
//!
//! The source strings are English. Spanish is embedded as a small catalog and resolved through
//! `craft-i18n`, which also provides the system locale and the language fallback chain.

mod catalog;

use craft_i18n::{I18n, Lang};
use log::warn;

/// The user's language preference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LanguagePreference {
    /// Follow the system locale.
    #[default]
    Auto,
    /// English.
    English,
    /// Spanish.
    Spanish,
}

impl LanguagePreference {
    /// Parse a value stored in settings.
    pub fn from_setting(value: &str) -> Self {
        match value {
            "en" => Self::English,
            "es" => Self::Spanish,
            _ => Self::Auto,
        }
    }

    /// The value stored in settings.
    pub fn as_setting(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::English => "en",
            Self::Spanish => "es",
        }
    }

    /// A stable identifier for UI radio buttons.
    pub fn id(self) -> &'static str {
        self.as_setting()
    }
}

/// The translated UI for the current session.
pub struct Translations {
    i18n: I18n,
}

impl std::fmt::Debug for Translations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Translations").finish_non_exhaustive()
    }
}

impl Translations {
    /// Build the translator from the embedded catalogs.
    pub fn new() -> Self {
        static EMBEDDED: [(&str, &str, &str); 1] = [("es", "", catalog::SPANISH)];
        let (i18n, problems) = I18n::new(&EMBEDDED, &["en", "es"]);
        for problem in problems {
            warn!("translation catalog: {problem}");
        }
        Self { i18n }
    }

    /// Resolve a preference to a concrete language.
    pub fn lang(&self, preference: LanguagePreference) -> Lang {
        match preference {
            LanguagePreference::Auto => self.i18n.system_lang(),
            LanguagePreference::English => Lang::EN,
            LanguagePreference::Spanish => self.i18n.from_code("es").unwrap_or(Lang::EN),
        }
    }

    /// Translate an English source string.
    ///
    /// The result is owned so callers can build small UI closures without tying the source
    /// literal to the translator's borrow. Translation rows are short and this remains cheap.
    pub fn t(&self, lang: Lang, source: &str) -> String {
        self.i18n.t(lang, source).to_owned()
    }

    /// The language name in its own language.
    pub fn language_name(&self, lang: Lang) -> &str {
        self.i18n.name(lang)
    }

    /// The language code shown by diagnostics and settings.
    pub fn code(&self, lang: Lang) -> &str {
        self.i18n.code(lang)
    }

    /// The languages offered by this build.
    pub fn languages(&self) -> Vec<Lang> {
        self.i18n.languages()
    }

    /// Parse all catalogs. Intended for tests and startup diagnostics.
    pub fn check_all(&self) -> Vec<String> {
        self.i18n.check_all()
    }
}

impl Default for Translations {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spanish_catalog_is_valid_and_resolves() {
        let translations = Translations::new();
        assert!(translations.check_all().is_empty());
        let spanish = translations.lang(LanguagePreference::Spanish);
        assert_eq!(translations.t(spanish, "File"), "Archivo");
        assert_eq!(translations.t(spanish, "Save"), "Guardar");
        assert_eq!(translations.t(spanish, "Not translated"), "Not translated");
        assert_eq!(translations.language_name(spanish), "Español");
    }

    #[test]
    fn language_preference_round_trips() {
        for preference in [
            LanguagePreference::Auto,
            LanguagePreference::English,
            LanguagePreference::Spanish,
        ] {
            assert_eq!(LanguagePreference::from_setting(preference.as_setting()), preference);
        }
        assert_eq!(LanguagePreference::from_setting("unknown"), LanguagePreference::Auto);
    }
}
