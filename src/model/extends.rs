//! Source-aware raw service `extends` directive.

use crate::source::SourceSpan;

use super::{FieldReference, Located};

/// An authored service `extends` directive without referenced-service resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Extends {
    /// A short raw service reference.
    Short(Located<String>),
    /// A long reference mapping.
    Long(ExtendsReference),
}

/// A long service `extends` reference mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendsReference {
    span: SourceSpan,
    service: Option<Located<String>>,
    file: Option<Located<String>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl ExtendsReference {
    pub(crate) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            service: None,
            file: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(crate) fn set_service(&mut self, value: Located<String>) {
        self.service = Some(value);
    }

    pub(crate) fn set_file(&mut self, value: Located<String>) {
        self.file = Some(value);
    }

    pub(crate) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(crate) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the complete authored long-form mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the raw referenced service without resolution.
    #[must_use]
    pub const fn service(&self) -> Option<&Located<String>> {
        self.service.as_ref()
    }

    /// Returns the raw referenced file without path access.
    #[must_use]
    pub const fn file(&self) -> Option<&Located<String>> {
        self.file.as_ref()
    }

    /// Returns retained `x-*` members.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns retained unknown or malformed members.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}
