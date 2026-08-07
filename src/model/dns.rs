//! Raw-preserving service DNS server declarations.

use crate::source::SourceSpan;

use super::Located;

/// The exact authored scalar or ordered-list form of service `dns`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DnsForm {
    /// One exact DNS server string scalar.
    Scalar(Located<String>),
    /// An explicitly authored ordered list, including an explicit empty list.
    List(Vec<Located<String>>),
}

/// An explicitly authored raw service `dns` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dns {
    span: SourceSpan,
    form: DnsForm,
}

impl Dns {
    pub(crate) const fn new(span: SourceSpan, form: DnsForm) -> Self {
        Self { span, form }
    }

    /// Returns the exact span of the complete authored field value.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the authored scalar or ordered-list form without interpreting server strings.
    #[must_use]
    pub const fn form(&self) -> &DnsForm {
        &self.form
    }
}
