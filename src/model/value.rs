//! Shared scalar and collection values used by the typed Compose model.

use super::Located;
use crate::source::SourceSpan;

/// A Compose boolean before optional interpolation is evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BooleanValue {
    /// A YAML boolean literal.
    Literal(bool),
    /// A scalar expression whose result must later be validated as a boolean.
    Expression(String),
}

/// A YAML scalar retained without applying Compose interpolation or coercion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposeScalar {
    /// An explicit YAML null.
    Null,
    /// A YAML boolean.
    Boolean(bool),
    /// A numeric scalar with its authored semantic spelling retained.
    Number(String),
    /// A string scalar, which may still contain interpolation expressions.
    String(String),
}

/// One source-aware key/value entry from a Compose mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyValueEntry {
    key: Located<String>,
    value: Located<ComposeScalar>,
    span: SourceSpan,
}

impl KeyValueEntry {
    pub(super) const fn new(key: Located<String>, value: Located<ComposeScalar>, span: SourceSpan) -> Self {
        Self { key, value, span }
    }

    /// Returns the entry key.
    #[must_use]
    pub const fn key(&self) -> &Located<String> {
        &self.key
    }

    /// Returns the scalar entry value.
    #[must_use]
    pub const fn value(&self) -> &Located<ComposeScalar> {
        &self.value
    }

    /// Returns the complete key/value span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// A Compose labels value with its list or mapping form retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Labels {
    /// List syntax such as `com.example.role=database`.
    List {
        /// The complete sequence span.
        span: SourceSpan,
        /// Label strings in authored order.
        values: Vec<Located<String>>,
    },
    /// Mapping syntax with scalar values.
    Map {
        /// The complete mapping span.
        span: SourceSpan,
        /// Label entries in authored order.
        entries: Vec<KeyValueEntry>,
    },
}

impl Labels {
    /// Returns the authored collection span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::List { span, .. } | Self::Map { span, .. } => *span,
        }
    }
}
