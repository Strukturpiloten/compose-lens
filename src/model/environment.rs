//! Service environment forms.

use super::{ComposeScalar, Located};
use crate::source::SourceSpan;

/// One array-syntax environment entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentListEntry {
    raw: Located<String>,
    name: String,
    value: Option<String>,
}

impl EnvironmentListEntry {
    pub(super) fn parse(raw: Located<String>) -> Self {
        let (name, value) = raw.value().split_once('=').map_or_else(
            || (raw.value().clone(), None),
            |(name, value)| (name.to_owned(), Some(value.to_owned())),
        );
        Self { raw, name, value }
    }

    /// Returns the complete semantic entry and its source span.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns the variable name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the value after the first equals sign.
    ///
    /// `None` means no equals sign was authored; `Some("")` means an explicitly empty value.
    #[must_use]
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }
}

/// One mapping-syntax environment entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentMapEntry {
    name: Located<String>,
    value: Located<ComposeScalar>,
    span: SourceSpan,
}

impl EnvironmentMapEntry {
    pub(super) const fn new(name: Located<String>, value: Located<ComposeScalar>, span: SourceSpan) -> Self {
        Self { name, value, span }
    }

    /// Returns the environment-variable name.
    #[must_use]
    pub const fn name(&self) -> &Located<String> {
        &self.name
    }

    /// Returns the unprocessed scalar value; null remains distinct from an empty string.
    #[must_use]
    pub const fn value(&self) -> &Located<ComposeScalar> {
        &self.value
    }

    /// Returns the complete mapping-entry span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// A service environment with array or mapping syntax retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Environment {
    /// Array syntax.
    List {
        /// The complete sequence span.
        span: SourceSpan,
        /// Entries in authored order.
        entries: Vec<EnvironmentListEntry>,
    },
    /// Mapping syntax.
    Map {
        /// The complete mapping span.
        span: SourceSpan,
        /// Entries in authored order.
        entries: Vec<EnvironmentMapEntry>,
    },
}

impl Environment {
    /// Returns the complete environment value span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::List { span, .. } | Self::Map { span, .. } => *span,
        }
    }
}
