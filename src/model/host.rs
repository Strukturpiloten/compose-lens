//! Source-aware extra-host mappings.

use super::Located;
use crate::source::SourceSpan;
use std::net::IpAddr;

/// The authored collection form of `extra_hosts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtraHosts {
    /// Sequence-based short syntax.
    Short {
        /// The complete sequence span.
        span: SourceSpan,
        /// Host mappings in authored order.
        entries: Vec<ShortExtraHost>,
    },
    /// Mapping-based long syntax.
    Long {
        /// The complete mapping span.
        span: SourceSpan,
        /// Host mappings in authored order.
        entries: Vec<LongExtraHost>,
    },
}

impl ExtraHosts {
    /// Returns the complete collection span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Short { span, .. } | Self::Long { span, .. } => *span,
        }
    }

    /// Reports whether any entry uses the implementation token `host-gateway`.
    #[must_use]
    pub fn contains_host_gateway(&self) -> bool {
        match self {
            Self::Short { entries, .. } => entries
                .iter()
                .any(|entry| entry.address().is_some_and(HostAddress::is_host_gateway)),
            Self::Long { entries, .. } => entries.iter().any(|entry| entry.address().value().is_host_gateway()),
        }
    }
}

/// The separator used by one short `extra_hosts` entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtraHostSeparator {
    /// Preferred `HOST=ADDRESS` spelling.
    Equals,
    /// Compatibility `HOST:ADDRESS` spelling.
    Colon,
}

/// One short-syntax `extra_hosts` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortExtraHost {
    raw: Located<String>,
    hostname: Option<String>,
    address: Option<HostAddress>,
    separator: Option<ExtraHostSeparator>,
}

impl ShortExtraHost {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let (hostname, address, separator) =
            split_short_entry(raw.value()).map_or((None, None, None), |(hostname, address, separator)| {
                (
                    Some(hostname.to_owned()),
                    Some(HostAddress::parse(address.to_owned())),
                    Some(separator),
                )
            });
        Self {
            raw,
            hostname,
            address,
            separator,
        }
    }

    /// Returns the complete unquoted scalar and its source span.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns the conservatively extracted hostname.
    #[must_use]
    pub fn hostname(&self) -> Option<&str> {
        self.hostname.as_deref()
    }

    /// Returns the raw-preserving address or implementation token.
    #[must_use]
    pub const fn address(&self) -> Option<&HostAddress> {
        self.address.as_ref()
    }

    /// Returns the authored separator.
    #[must_use]
    pub const fn separator(&self) -> Option<ExtraHostSeparator> {
        self.separator
    }

    /// Reports whether this entry contains a hostname and address.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.hostname.is_some() && self.address.is_some()
    }
}

/// One mapping-syntax `extra_hosts` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongExtraHost {
    hostname: Located<String>,
    address: Located<HostAddress>,
    span: SourceSpan,
}

impl LongExtraHost {
    pub(super) const fn new(hostname: Located<String>, address: Located<HostAddress>, span: SourceSpan) -> Self {
        Self {
            hostname,
            address,
            span,
        }
    }

    /// Returns the hostname mapping key.
    #[must_use]
    pub const fn hostname(&self) -> &Located<String> {
        &self.hostname
    }

    /// Returns the raw-preserving address or implementation token.
    #[must_use]
    pub const fn address(&self) -> &Located<HostAddress> {
        &self.address
    }

    /// Returns the complete entry span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// A host address classified without normalizing its authored spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAddress {
    raw: String,
    kind: HostAddressKind,
}

impl HostAddress {
    pub(super) fn parse(raw: String) -> Self {
        let unbracketed = raw
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
            .unwrap_or(&raw);
        let kind = if raw == "host-gateway" {
            HostAddressKind::HostGateway
        } else {
            match unbracketed.parse::<IpAddr>() {
                Ok(IpAddr::V4(_)) => HostAddressKind::Ipv4,
                Ok(IpAddr::V6(_)) => HostAddressKind::Ipv6 {
                    bracketed: raw.starts_with('[') && raw.ends_with(']'),
                },
                Err(_) => HostAddressKind::Other,
            }
        };
        Self { raw, kind }
    }

    /// Returns the address exactly as represented by the YAML scalar.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns the non-destructive address classification.
    #[must_use]
    pub const fn kind(&self) -> HostAddressKind {
        self.kind
    }

    /// Reports whether this is the implementation token `host-gateway`.
    #[must_use]
    pub const fn is_host_gateway(&self) -> bool {
        matches!(self.kind, HostAddressKind::HostGateway)
    }
}

/// The lexical kind of a host address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostAddressKind {
    /// An IPv4 address.
    Ipv4,
    /// An IPv6 address, retaining whether brackets were authored.
    Ipv6 {
        /// Whether the address used `[::1]` spelling.
        bracketed: bool,
    },
    /// The runtime-specific `host-gateway` token.
    HostGateway,
    /// A deferred expression or implementation-specific value.
    Other,
}

fn split_short_entry(value: &str) -> Option<(&str, &str, ExtraHostSeparator)> {
    if let Some((hostname, address)) = value.split_once('=') {
        return (!hostname.is_empty() && !address.is_empty()).then_some((
            hostname,
            address,
            ExtraHostSeparator::Equals,
        ));
    }
    let (hostname, address) = value.split_once(':')?;
    (!hostname.is_empty() && !address.is_empty()).then_some((hostname, address, ExtraHostSeparator::Colon))
}

#[cfg(test)]
mod tests {
    use super::{ExtraHostSeparator, HostAddressKind, ShortExtraHost};
    use crate::model::Located;
    use crate::source::{SourceId, SourceSpan};

    fn entry(value: &str) -> Result<ShortExtraHost, &'static str> {
        let span = SourceSpan::new(SourceId::new(1), 0, value.len()).ok_or("valid test span expected")?;
        Ok(ShortExtraHost::parse(Located::new(value.to_owned(), span)))
    }

    #[test]
    fn preserves_ipv6_and_legacy_separator_spelling() -> Result<(), &'static str> {
        let unbracketed = entry("myhostv6:::1")?;
        assert_eq!(unbracketed.hostname(), Some("myhostv6"));
        assert_eq!(unbracketed.address().map(super::HostAddress::raw), Some("::1"));
        assert_eq!(unbracketed.separator(), Some(ExtraHostSeparator::Colon));
        assert_eq!(
            unbracketed.address().map(super::HostAddress::kind),
            Some(HostAddressKind::Ipv6 { bracketed: false })
        );

        let bracketed = entry("myhostv6=[::1]")?;
        assert_eq!(
            bracketed.address().map(super::HostAddress::kind),
            Some(HostAddressKind::Ipv6 { bracketed: true })
        );
        Ok(())
    }
}
