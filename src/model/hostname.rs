//! Raw-preserving service hostnames.

use super::Located;

/// A service-level Compose `hostname` value with its authored string scalar retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hostname {
    raw: Located<String>,
    kind: HostnameKind,
}

impl Hostname {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let kind = if raw.value().contains('$') {
            HostnameKind::Expression
        } else if valid_hostname(raw.value()) {
            HostnameKind::Resolved
        } else {
            HostnameKind::Invalid
        };
        Self { raw, kind }
    }

    /// Returns the complete authored scalar value and its source span.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns the conservative, non-destructive hostname classification.
    #[must_use]
    pub const fn kind(&self) -> &HostnameKind {
        &self.kind
    }

    /// Reports whether the hostname is a resolved RFC-1123 literal.
    #[must_use]
    pub const fn is_resolved(&self) -> bool {
        matches!(self.kind, HostnameKind::Resolved)
    }
}

/// The semantic family of a service-level Compose hostname.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HostnameKind {
    /// A resolved ASCII hostname satisfying the conservative RFC-1123 grammar.
    Resolved,
    /// A scalar containing a dollar marker and therefore deferred to interpolation.
    Expression,
    /// A resolved literal that does not satisfy the conservative hostname grammar.
    Invalid,
}

pub(crate) fn valid_hostname(value: &str) -> bool {
    if !(1..=253).contains(&value.len()) || !value.is_ascii() {
        return false;
    }
    value.split('.').all(|label| {
        (1..=63).contains(&label.len())
            && label.bytes().next().is_some_and(|byte| byte.is_ascii_alphanumeric())
            && label.bytes().last().is_some_and(|byte| byte.is_ascii_alphanumeric())
            && label.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

#[cfg(test)]
mod tests {
    use super::{Hostname, HostnameKind, valid_hostname};
    use crate::model::Located;
    use crate::source::{SourceId, SourceSpan};

    #[test]
    fn validates_conservative_ascii_rfc_1123_hostnames() {
        let label_63 = "a".repeat(63);
        let maximum = format!("{label_63}.{label_63}.{label_63}.{}", "a".repeat(61));
        for value in ["a", "3api", "API.Example-Corp.COM", maximum.as_str()] {
            assert!(valid_hostname(value), "expected valid hostname {value}");
        }
        let label_64 = "a".repeat(64);
        let too_long = format!("{maximum}.a");
        for value in [
            "",
            ".",
            "example.",
            ".example",
            "example..com",
            "-example",
            "example-",
            "example_com",
            "café.example",
            label_64.as_str(),
            too_long.as_str(),
        ] {
            assert!(!valid_hostname(value), "expected invalid hostname {value}");
        }
    }

    #[test]
    fn classifies_every_dollar_bearing_value_as_deferred() -> Result<(), &'static str> {
        let value = "invalid_$_hostname";
        let span = SourceSpan::new(SourceId::new(1), 0, value.len()).ok_or("valid test span expected")?;
        let hostname = Hostname::parse(Located::new(value.to_owned(), span));
        assert_eq!(hostname.raw().value(), value);
        assert_eq!(hostname.kind(), &HostnameKind::Expression);
        assert!(!hostname.is_resolved());
        Ok(())
    }
}
