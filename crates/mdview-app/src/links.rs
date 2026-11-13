//! Internal and external link resolution for the rendered view.

use std::path::{Path, PathBuf};

use craft_docpath::resolve;
use mdview_markdown::LinkKind;

/// What activating a link should do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkAction {
    /// Scroll inside the current document.
    Anchor(String),
    /// Open a local document, optionally at an anchor.
    Open {
        /// Resolved path.
        path: PathBuf,
        /// Anchor inside the target document.
        anchor: Option<String>,
    },
    /// Hand an external URL to the platform browser.
    External(String),
    /// Reject the destination with an actionable reason.
    Rejected(String),
}

/// Resolve a Markdown link against the current document path.
pub fn resolve_link(current: Option<&Path>, destination: &str, kind: LinkKind) -> LinkAction {
    let destination = destination.trim();
    if destination.is_empty() {
        return LinkAction::Rejected("The link has no destination.".to_owned());
    }
    if destination.starts_with('#') {
        return LinkAction::Anchor(destination.trim_start_matches('#').to_owned());
    }
    match kind {
        LinkKind::External => {
            let lower = destination.to_ascii_lowercase();
            if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:") {
                LinkAction::External(destination.to_owned())
            } else {
                LinkAction::Rejected(format!("Unsupported link scheme: {destination}"))
            }
        }
        LinkKind::Invalid => LinkAction::Rejected(format!("Unsafe link destination: {destination}")),
        LinkKind::Data => LinkAction::Rejected("Embedded data links are not opened.".to_owned()),
        LinkKind::Anchor | LinkKind::LocalFile | LinkKind::FileUrl => {
            let (path_part, anchor) = split_fragment(destination);
            if path_part.is_empty() {
                return match anchor {
                    Some(anchor) => LinkAction::Anchor(anchor),
                    None => LinkAction::Rejected("The link has no destination.".to_owned()),
                };
            }
            match resolve(path_part, current) {
                Ok(path) => LinkAction::Open { path, anchor },
                Err(refused) => LinkAction::Rejected(refused.to_string()),
            }
        }
    }
}

fn split_fragment(destination: &str) -> (&str, Option<String>) {
    match destination.split_once('#') {
        Some((path, fragment)) if !fragment.is_empty() => (path, Some(fragment.to_owned())),
        Some((path, _)) => (path, None),
        None => (destination, None),
    }
}

/// The title shown when a link cannot be opened.
pub fn link_error(destination: &str, reason: &str) -> String {
    format!("Could not open link '{destination}': {reason}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_links_resolve_against_the_document() {
        let current = Path::new("/tmp/project/docs/readme.md");
        assert_eq!(
            resolve_link(Some(current), "other.md#intro", LinkKind::LocalFile),
            LinkAction::Open {
                path: PathBuf::from("/tmp/project/docs/other.md"),
                anchor: Some("intro".to_owned()),
            }
        );
        assert_eq!(
            resolve_link(Some(current), "../assets/picture.png", LinkKind::LocalFile),
            LinkAction::Open {
                path: PathBuf::from("/tmp/project/assets/picture.png"),
                anchor: None,
            }
        );
    }

    #[test]
    fn anchors_stay_in_the_document() {
        assert_eq!(
            resolve_link(None, "#section", LinkKind::Anchor),
            LinkAction::Anchor("section".to_owned())
        );
    }

    #[test]
    fn external_and_unsafe_links_are_separated() {
        assert_eq!(
            resolve_link(None, "https://example.com", LinkKind::External),
            LinkAction::External("https://example.com".to_owned())
        );
        assert!(matches!(
            resolve_link(None, "javascript:alert(1)", LinkKind::Invalid),
            LinkAction::Rejected(_)
        ));
        assert!(matches!(
            resolve_link(None, "\\\\server\\share\\a.md", LinkKind::LocalFile),
            LinkAction::Rejected(_)
        ));
    }

    #[test]
    fn file_urls_are_local() {
        let action = resolve_link(
            Some(Path::new("/tmp/project/a.md")),
            "file:///tmp/project/b.md",
            LinkKind::FileUrl,
        );
        assert!(matches!(action, LinkAction::Open { path, .. } if path == Path::new("/tmp/project/b.md")));
    }
}
