//! Typed service resource limits.

use super::{FieldReference, Located};
use crate::source::SourceSpan;

/// A service `ulimits` mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ulimits {
    span: SourceSpan,
    entries: Vec<Ulimit>,
}

impl Ulimits {
    pub(super) const fn new(span: SourceSpan, entries: Vec<Ulimit>) -> Self {
        Self { span, entries }
    }

    /// Returns the complete mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns limits in authored order.
    #[must_use]
    pub fn entries(&self) -> &[Ulimit] {
        &self.entries
    }
}

/// One named service limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ulimit {
    name: Located<String>,
    span: SourceSpan,
    value: UlimitValue,
}

impl Ulimit {
    pub(super) const fn new(name: Located<String>, span: SourceSpan, value: UlimitValue) -> Self {
        Self { name, span, value }
    }

    /// Returns the limit name.
    #[must_use]
    pub const fn name(&self) -> &Located<String> {
        &self.name
    }

    /// Returns the complete entry span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the short or long authored value.
    #[must_use]
    pub const fn value(&self) -> &UlimitValue {
        &self.value
    }
}

/// The authored form of one service limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UlimitValue {
    /// One scalar applies to both limits.
    Single(Located<LimitValue>),
    /// Separate soft and hard limits.
    Range(UlimitRange),
}

/// A scalar resource-limit value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitValue {
    /// Unlimited, authored as `-1`.
    Unlimited,
    /// A non-negative integer with its spelling retained.
    Number(String),
    /// A deferred interpolation expression.
    Expression(String),
    /// An invalid or provider-specific scalar retained for diagnostics.
    Other(String),
}

impl LimitValue {
    pub(super) fn parse(value: String) -> Self {
        if value == "-1" {
            Self::Unlimited
        } else if value.bytes().all(|byte| byte.is_ascii_digit()) && !value.is_empty() {
            Self::Number(value)
        } else if value.contains('$') {
            Self::Expression(value)
        } else {
            Self::Other(value)
        }
    }

    /// Reports whether the value follows a specification form or remains deferred.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        !matches!(self, Self::Other(_))
    }

    /// Returns a source-independent semantic spelling.
    #[must_use]
    pub fn raw(&self) -> &str {
        match self {
            Self::Unlimited => "-1",
            Self::Number(value) | Self::Expression(value) | Self::Other(value) => value,
        }
    }
}

/// Long syntax with independent soft and hard limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UlimitRange {
    span: SourceSpan,
    soft: Option<Located<LimitValue>>,
    hard: Option<Located<LimitValue>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl UlimitRange {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            soft: None,
            hard: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_soft(&mut self, value: Located<LimitValue>) {
        self.soft = Some(value);
    }

    pub(super) fn set_hard(&mut self, value: Located<LimitValue>) {
        self.hard = Some(value);
    }

    pub(super) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(super) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the complete range mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the explicitly authored soft limit.
    #[must_use]
    pub const fn soft(&self) -> Option<&Located<LimitValue>> {
        self.soft.as_ref()
    }

    /// Returns the explicitly authored hard limit.
    #[must_use]
    pub const fn hard(&self) -> Option<&Located<LimitValue>> {
        self.hard.as_ref()
    }

    /// Returns retained `x-` fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns unrecognized range fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}
