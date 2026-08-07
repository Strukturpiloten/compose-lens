//! Source-aware service annotation declarations.

use crate::source::SourceSpan;

use super::{ComposeScalar, KeyValueEntry, Located};

/// The exact authored mapping or list form of service `annotations`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnnotationsForm {
    /// Ordered mapping syntax with scalar kinds and spelling retained.
    Map(Vec<KeyValueEntry>),
    /// Ordered raw scalar list items, including invalid scalar kinds retained for diagnostics.
    List(Vec<Located<ComposeScalar>>),
}

/// An explicitly authored service-level Compose `annotations` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotations {
    span: SourceSpan,
    form: AnnotationsForm,
}

impl Annotations {
    pub(crate) const fn new(span: SourceSpan, form: AnnotationsForm) -> Self {
        Self { span, form }
    }

    /// Returns the exact span of the complete authored collection.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the authored mapping or list form without semantic normalization.
    #[must_use]
    pub const fn form(&self) -> &AnnotationsForm {
        &self.form
    }
}
