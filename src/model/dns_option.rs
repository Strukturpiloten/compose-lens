//! Raw-preserving service DNS resolver options.

use crate::source::SourceSpan;

use super::Located;

/// An explicitly authored ordered service `dns_opt` sequence.
///
/// Omission is represented by the absence of this value; an empty `items` slice therefore means
/// the author explicitly supplied an empty sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsOptions {
    span: SourceSpan,
    items: Vec<Located<String>>,
}

impl DnsOptions {
    pub(crate) const fn new(span: SourceSpan, items: Vec<Located<String>>) -> Self {
        Self { span, items }
    }

    /// Returns the span of the complete authored sequence.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns every valid authored option in order, including exact duplicates.
    #[must_use]
    pub fn items(&self) -> &[Located<String>] {
        &self.items
    }
}
