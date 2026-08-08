//! Source-aware service logging configuration.

use crate::source::SourceSpan;

use super::{FieldReference, Located};

/// An authored logging-option scalar with its exact Compose-supported YAML kind retained.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoggingOptionValue {
    /// A YAML string scalar, including interpolation-shaped text.
    String(String),
    /// A YAML number scalar with exact semantic spelling retained.
    Number(String),
    /// An explicit or empty YAML null.
    Null,
}

/// One source-aware logging option entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggingOption {
    name: Located<String>,
    value: Located<LoggingOptionValue>,
    span: SourceSpan,
}

impl LoggingOption {
    pub(crate) const fn new(name: Located<String>, value: Located<LoggingOptionValue>, span: SourceSpan) -> Self {
        Self { name, value, span }
    }

    /// Returns the non-empty option key and its exact source span.
    #[must_use]
    pub const fn name(&self) -> &Located<String> {
        &self.name
    }

    /// Returns the exact string, number, or null value and its source span.
    #[must_use]
    pub const fn value(&self) -> &Located<LoggingOptionValue> {
        &self.value
    }

    /// Returns the complete key/value entry span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// An ordered service logging-options mapping, including an explicitly empty mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggingOptions {
    span: SourceSpan,
    entries: Vec<LoggingOption>,
    unmodeled_entries: Vec<FieldReference>,
}

impl LoggingOptions {
    pub(crate) const fn new(
        span: SourceSpan,
        entries: Vec<LoggingOption>,
        unmodeled_entries: Vec<FieldReference>,
    ) -> Self {
        Self {
            span,
            entries,
            unmodeled_entries,
        }
    }

    /// Returns the exact span of the authored options mapping.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns valid option entries in authored order.
    #[must_use]
    pub fn entries(&self) -> &[LoggingOption] {
        &self.entries
    }

    /// Returns malformed option entries retained for source-aware recovery.
    #[must_use]
    pub fn unmodeled_entries(&self) -> &[FieldReference] {
        &self.unmodeled_entries
    }
}

/// An explicitly authored service-level Compose `logging` mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Logging {
    span: SourceSpan,
    driver: Option<Located<String>>,
    options: Option<LoggingOptions>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl Logging {
    pub(crate) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            driver: None,
            options: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(crate) fn set_driver(&mut self, driver: Located<String>) {
        self.driver = Some(driver);
    }

    pub(crate) fn set_options(&mut self, options: LoggingOptions) {
        self.options = Some(options);
    }

    pub(crate) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(crate) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the exact span of the complete authored logging mapping.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the exact uninterpreted string driver when validly authored.
    #[must_use]
    pub const fn driver(&self) -> Option<&Located<String>> {
        self.driver.as_ref()
    }

    /// Returns the ordered options mapping, including an explicitly empty one.
    #[must_use]
    pub const fn options(&self) -> Option<&LoggingOptions> {
        self.options.as_ref()
    }

    /// Returns retained `x-*` fields from the logging mapping.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns unrecognized fields from the logging mapping.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}
