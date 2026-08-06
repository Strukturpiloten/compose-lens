//! Raw-preserving service capability declarations.

use crate::source::SourceSpan;

use super::Located;

fn is_exact_candidate(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_whitespace)
}

/// One exact service capability name from `cap_add`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAddItem {
    raw: Located<String>,
}

impl CapabilityAddItem {
    pub(crate) const fn new(raw: Located<String>) -> Self {
        Self { raw }
    }

    /// Returns the exact authored string value.
    #[must_use]
    pub fn value(&self) -> &str {
        self.raw.value()
    }

    /// Returns the exact source span of this sequence item.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.raw.span()
    }

    /// Reports whether this value is a conservative future cross-format exact candidate.
    ///
    /// This classification is purely lexical: the value must be non-empty and contain no
    /// whitespace. It does not normalize case or consult a capability whitelist or target.
    #[must_use]
    pub fn is_exact_candidate(&self) -> bool {
        is_exact_candidate(self.value())
    }
}

/// An explicitly authored `cap_add` sequence, including an explicit empty sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAdd {
    span: SourceSpan,
    items: Vec<CapabilityAddItem>,
}

impl CapabilityAdd {
    pub(crate) const fn new(span: SourceSpan, items: Vec<CapabilityAddItem>) -> Self {
        Self { span, items }
    }

    /// Returns the exact span of the authored sequence value.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns items in authored order, including exact duplicates.
    #[must_use]
    pub fn items(&self) -> &[CapabilityAddItem] {
        &self.items
    }
}

/// One exact service capability name from `cap_drop`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDropItem {
    raw: Located<String>,
}

impl CapabilityDropItem {
    pub(crate) const fn new(raw: Located<String>) -> Self {
        Self { raw }
    }

    /// Returns the exact authored string value.
    #[must_use]
    pub fn value(&self) -> &str {
        self.raw.value()
    }

    /// Returns the exact source span of this sequence item.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.raw.span()
    }

    /// Reports whether this value is a conservative future cross-format exact candidate.
    ///
    /// This classification is purely lexical: the value must be non-empty and contain no
    /// whitespace. It does not normalize case or consult a capability whitelist or target.
    #[must_use]
    pub fn is_exact_candidate(&self) -> bool {
        is_exact_candidate(self.value())
    }
}

/// An explicitly authored `cap_drop` sequence, including an explicit empty sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDrop {
    span: SourceSpan,
    items: Vec<CapabilityDropItem>,
}

impl CapabilityDrop {
    pub(crate) const fn new(span: SourceSpan, items: Vec<CapabilityDropItem>) -> Self {
        Self { span, items }
    }

    /// Returns the exact span of the authored sequence value.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns items in authored order, including exact duplicates.
    #[must_use]
    pub fn items(&self) -> &[CapabilityDropItem] {
        &self.items
    }
}
