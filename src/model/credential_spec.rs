//! Source-aware service credential-spec configuration.

use crate::source::SourceSpan;

use super::{FieldReference, Located};

/// An explicitly authored service `credential_spec` mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialSpec {
    span: SourceSpan,
    config: Option<Located<String>>,
    file: Option<Located<String>>,
    registry: Option<Located<String>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl CredentialSpec {
    pub(crate) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            config: None,
            file: None,
            registry: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(crate) fn set_config(&mut self, value: Located<String>) {
        self.config = Some(value);
    }

    pub(crate) fn set_file(&mut self, value: Located<String>) {
        self.file = Some(value);
    }

    pub(crate) fn set_registry(&mut self, value: Located<String>) {
        self.registry = Some(value);
    }

    pub(crate) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(crate) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the complete authored credential-spec mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the raw authored config reference without resolving top-level configs.
    #[must_use]
    pub const fn config(&self) -> Option<&Located<String>> {
        self.config.as_ref()
    }

    /// Returns the raw authored file reference without accessing the filesystem.
    #[must_use]
    pub const fn file(&self) -> Option<&Located<String>> {
        self.file.as_ref()
    }

    /// Returns the raw authored registry reference without account or registry access.
    #[must_use]
    pub const fn registry(&self) -> Option<&Located<String>> {
        self.registry.as_ref()
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
