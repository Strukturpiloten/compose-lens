//! Raw-preserving service-level temporary-filesystem declarations.

use crate::source::SourceSpan;

use super::Located;

/// One exact service-level `tmpfs` item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmpfsItem {
    raw: Located<String>,
    kind: TmpfsItemKind,
}

impl TmpfsItem {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let kind = classify_tmpfs_item(raw.value());
        Self { raw, kind }
    }

    /// Returns the exact string value without normalizing its path or options.
    #[must_use]
    pub fn value(&self) -> &str {
        self.raw.value()
    }

    /// Returns the exact source span of this scalar item.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.raw.span()
    }

    /// Returns the conservative raw-preserving item classification.
    #[must_use]
    pub const fn kind(&self) -> TmpfsItemKind {
        self.kind
    }
}

/// The conservative semantic family of one service-level `tmpfs` item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TmpfsItemKind {
    /// A dollar-bearing item whose effective spelling depends on Compose interpolation.
    Expression,
    /// A non-empty path with only non-empty `mode`, `uid`, or `gid` option assignments.
    Documented,
    /// An empty or malformed item, or one using provider- or target-specific options.
    ProviderDependent,
}

/// The exact authored scalar or sequence form of service-level `tmpfs`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TmpfsForm {
    /// One scalar temporary-filesystem declaration.
    Scalar(TmpfsItem),
    /// An explicitly authored sequence, including an explicit empty sequence.
    List(Vec<TmpfsItem>),
}

/// An explicitly authored service-level Compose `tmpfs` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tmpfs {
    span: SourceSpan,
    form: TmpfsForm,
}

impl Tmpfs {
    pub(crate) const fn new(span: SourceSpan, form: TmpfsForm) -> Self {
        Self { span, form }
    }

    /// Returns the exact span of the complete authored field value.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the authored scalar or list form without normalizing it.
    #[must_use]
    pub const fn form(&self) -> &TmpfsForm {
        &self.form
    }
}

pub(crate) fn classify_tmpfs_item(value: &str) -> TmpfsItemKind {
    if value.contains('$') {
        return TmpfsItemKind::Expression;
    }

    let (path, options) = value
        .split_once(':')
        .map_or((value, None), |(path, options)| (path, Some(options)));
    if path.is_empty() || path.contains(['\r', '\n']) {
        return TmpfsItemKind::ProviderDependent;
    }

    let Some(options) = options else {
        return TmpfsItemKind::Documented;
    };
    if options.is_empty() {
        return TmpfsItemKind::ProviderDependent;
    }

    for option in options.split(',') {
        let Some((key, option_value)) = option.split_once('=') else {
            return TmpfsItemKind::ProviderDependent;
        };
        if !matches!(key, "mode" | "uid" | "gid") || option_value.is_empty() || option_value.contains(['\r', '\n']) {
            return TmpfsItemKind::ProviderDependent;
        }
    }

    TmpfsItemKind::Documented
}

pub(crate) fn valid_generated_tmpfs_item(value: &str) -> bool {
    if value.contains(['$', '\r', '\n']) {
        return false;
    }
    let (path, options) = value
        .split_once(':')
        .map_or((value, None), |(path, options)| (path, Some(options)));
    if path.is_empty() {
        return false;
    }
    options.is_none_or(|options| {
        !options.is_empty()
            && options.split(',').all(|option| {
                if let Some((key, option_value)) = option.split_once('=') {
                    !key.is_empty() && !option_value.is_empty()
                } else {
                    !option.is_empty()
                }
            })
    })
}

#[cfg(test)]
mod tests {
    use super::{TmpfsItemKind, classify_tmpfs_item, valid_generated_tmpfs_item};

    #[test]
    fn classifies_exact_documented_expression_and_provider_dependent_spellings() {
        for value in [
            "/run",
            "/path,with,commas",
            "/run:mode=1777",
            "relative:uid=user,gid=group",
            "/run:mode=1=2",
        ] {
            assert_eq!(classify_tmpfs_item(value), TmpfsItemKind::Documented);
        }
        for value in ["${TMPFS}", "/run:mode=${MODE}"] {
            assert_eq!(classify_tmpfs_item(value), TmpfsItemKind::Expression);
        }
        for value in [
            "",
            ":mode=1777",
            "/run:",
            "/run:mode",
            "/run:mode=",
            "/run:size=64m",
            "/run:exec,mode=1777",
            "/run\nnext",
        ] {
            assert_eq!(classify_tmpfs_item(value), TmpfsItemKind::ProviderDependent);
        }
    }

    #[test]
    fn generated_items_accept_well_shaped_raw_options_and_duplicates_are_a_collection_concern() {
        for value in [
            "/run",
            "/path,with,commas",
            "/run:mode=1777,uid=1000,gid=1000",
            "/run:size=64m",
            "/run:exec,nosuid,nodev",
        ] {
            assert!(
                valid_generated_tmpfs_item(value),
                "expected valid generated item {value:?}"
            );
        }
        for value in [
            "",
            "${TMPFS}",
            ":mode=1777",
            "/run:",
            "/run:,mode=1777",
            "/run:mode=",
            "/run\nnext",
        ] {
            assert!(
                !valid_generated_tmpfs_item(value),
                "expected invalid generated item {value:?}"
            );
        }
    }
}
