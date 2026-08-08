//! Build-specific `extra_hosts` values.

use super::Located;
use crate::source::SourceSpan;

/// Build-time host mappings with list and mapping syntax retained separately from service
/// [`super::ExtraHosts`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildExtraHosts {
    /// Ordered raw string entries, including duplicates.
    List {
        /// The complete sequence span.
        span: SourceSpan,
        /// Raw host-mapping strings in authored order.
        values: Vec<Located<String>>,
    },
    /// Ordered hostname entries with scalar or list addresses.
    Map {
        /// The complete mapping span.
        span: SourceSpan,
        /// Hostname entries in authored order.
        entries: Vec<BuildExtraHostEntry>,
    },
}

impl BuildExtraHosts {
    /// Returns the complete collection span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::List { span, .. } | Self::Map { span, .. } => *span,
        }
    }
}

/// One mapping-form build host entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildExtraHostEntry {
    hostname: Located<String>,
    addresses: BuildExtraHostAddresses,
    span: SourceSpan,
}

impl BuildExtraHostEntry {
    pub(crate) const fn new(hostname: Located<String>, addresses: BuildExtraHostAddresses, span: SourceSpan) -> Self {
        Self {
            hostname,
            addresses,
            span,
        }
    }

    /// Returns the raw hostname key and its source span.
    #[must_use]
    pub const fn hostname(&self) -> &Located<String> {
        &self.hostname
    }

    /// Returns the scalar or ordered-list address form without parsing it as an IP address.
    #[must_use]
    pub const fn addresses(&self) -> &BuildExtraHostAddresses {
        &self.addresses
    }

    /// Returns the complete mapping-entry span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// Addresses authored for one build host mapping key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildExtraHostAddresses {
    /// One raw string address.
    Scalar(Located<String>),
    /// Ordered raw string addresses.
    List {
        /// The complete nested sequence span.
        span: SourceSpan,
        /// Raw addresses in authored order.
        values: Vec<Located<String>>,
    },
}

impl BuildExtraHostAddresses {
    /// Returns the scalar address when that form was authored.
    #[must_use]
    pub const fn as_scalar(&self) -> Option<&Located<String>> {
        match self {
            Self::Scalar(value) => Some(value),
            Self::List { .. } => None,
        }
    }

    /// Returns ordered addresses when the nested list form was authored.
    #[must_use]
    pub fn as_list(&self) -> Option<&[Located<String>]> {
        let Self::List { values, .. } = self else {
            return None;
        };
        Some(values)
    }

    /// Returns the complete address-form span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Scalar(value) => value.span(),
            Self::List { span, .. } => *span,
        }
    }
}
