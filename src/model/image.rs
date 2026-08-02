//! Tolerant Compose image references.

/// A parsed digest suffix that remains tolerant of implementation extensions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageDigest {
    algorithm: Option<String>,
    value: String,
}

impl ImageDigest {
    /// Returns the digest algorithm when the suffix contains `algorithm:value`.
    #[must_use]
    pub fn algorithm(&self) -> Option<&str> {
        self.algorithm.as_deref()
    }

    /// Returns the digest payload without validating an OCI grammar.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// A loss-aware, tolerant image reference.
///
/// `ComposeLens` deliberately accepts combined tags and digests such as
/// `registry.example/app:1.2@sha256:...` and does not reject implementation-supported values
/// merely because a stricter external grammar would.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageReference {
    raw: String,
    name: String,
    tag: Option<String>,
    digest: Option<ImageDigest>,
}

impl ImageReference {
    pub(super) fn parse(raw: String) -> Self {
        let (name_and_tag, digest) = raw.split_once('@').map_or((raw.as_str(), None), |(left, right)| {
            let (algorithm, value) = right
                .split_once(':')
                .map_or((None, right.to_owned()), |(algorithm, value)| {
                    (Some(algorithm.to_owned()), value.to_owned())
                });
            (left, Some(ImageDigest { algorithm, value }))
        });

        let last_slash = name_and_tag.rfind('/');
        let tag_separator = name_and_tag
            .rfind(':')
            .filter(|index| last_slash.is_none_or(|slash| *index > slash));
        let (name, tag) = tag_separator.map_or_else(
            || (name_and_tag.to_owned(), None),
            |index| {
                (
                    name_and_tag[..index].to_owned(),
                    Some(name_and_tag[index + 1..].to_owned()),
                )
            },
        );

        Self { raw, name, tag, digest }
    }

    /// Returns the complete unmodified semantic reference.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns the repository/name portion before a tag or digest.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the tag without the leading colon.
    #[must_use]
    pub fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    /// Returns the digest suffix without the leading at-sign.
    #[must_use]
    pub const fn digest(&self) -> Option<&ImageDigest> {
        self.digest.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::ImageReference;

    #[test]
    fn accepts_registry_ports_tags_and_digests_together() {
        let reference = ImageReference::parse("registry.example:5000/team/app:1.2@sha256:abcdef".to_owned());

        assert_eq!(reference.name(), "registry.example:5000/team/app");
        assert_eq!(reference.tag(), Some("1.2"));
        assert_eq!(
            reference.digest().and_then(super::ImageDigest::algorithm),
            Some("sha256")
        );
        assert_eq!(reference.digest().map(super::ImageDigest::value), Some("abcdef"));
    }
}
