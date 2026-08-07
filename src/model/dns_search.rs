//! Raw-preserving service DNS search-domain declarations.

use crate::source::SourceSpan;

use super::Located;

/// The exact authored scalar or ordered-list form of service `dns_search`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DnsSearchForm {
    /// One exact DNS search-domain string scalar.
    Scalar(Located<String>),
    /// An explicitly authored ordered list, including an explicit empty list.
    List(Vec<Located<String>>),
}

/// An explicitly authored raw service `dns_search` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsSearch {
    span: SourceSpan,
    form: DnsSearchForm,
}

impl DnsSearch {
    pub(crate) const fn new(span: SourceSpan, form: DnsSearchForm) -> Self {
        Self { span, form }
    }

    /// Returns the exact span of the complete authored field value.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the authored scalar or ordered-list form without interpreting domain strings.
    #[must_use]
    pub const fn form(&self) -> &DnsSearchForm {
        &self.form
    }
}
