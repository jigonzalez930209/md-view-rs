//! Media access policy and log helpers.

/// Media policy from preferences.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaPolicy {
    /// Fetch remote images referenced by a document.
    pub remote_images: bool,
    /// Allow URLs that resolve to private network addresses.
    pub private_networks: bool,
}

impl Default for MediaPolicy {
    fn default() -> Self {
        Self {
            remote_images: true,
            private_networks: false,
        }
    }
}

/// Truncate a media source for logs.
pub(crate) fn media_source(source: &str) -> String {
    source.chars().take(120).collect()
}
