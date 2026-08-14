//! Service environment forms.

use super::{BooleanValue, ComposeScalar, FieldReference, Located};
use crate::source::SourceSpan;

/// One array-syntax environment entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentListEntry {
    raw: Located<String>,
    name: String,
    value: Option<String>,
}

impl EnvironmentListEntry {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let (name, value) = raw.value().split_once('=').map_or_else(
            || (raw.value().clone(), None),
            |(name, value)| (name.to_owned(), Some(value.to_owned())),
        );
        Self { raw, name, value }
    }

    /// Returns the complete semantic entry and its source span.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns the variable name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the value after the first equals sign.
    ///
    /// `None` means no equals sign was authored; `Some("")` means an explicitly empty value.
    #[must_use]
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }
}

/// One mapping-syntax environment entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentMapEntry {
    name: Located<String>,
    value: Located<ComposeScalar>,
    span: SourceSpan,
}

impl EnvironmentMapEntry {
    pub(crate) const fn new(name: Located<String>, value: Located<ComposeScalar>, span: SourceSpan) -> Self {
        Self { name, value, span }
    }

    /// Returns the environment-variable name.
    #[must_use]
    pub const fn name(&self) -> &Located<String> {
        &self.name
    }

    /// Returns the unprocessed scalar value; null remains distinct from an empty string.
    #[must_use]
    pub const fn value(&self) -> &Located<ComposeScalar> {
        &self.value
    }

    /// Returns the complete mapping-entry span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// A service environment with array or mapping syntax retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Environment {
    /// Array syntax.
    List {
        /// The complete sequence span.
        span: SourceSpan,
        /// Entries in authored order.
        entries: Vec<EnvironmentListEntry>,
    },
    /// Mapping syntax.
    Map {
        /// The complete mapping span.
        span: SourceSpan,
        /// Entries in authored order.
        entries: Vec<EnvironmentMapEntry>,
    },
}

impl Environment {
    /// Returns the complete environment value span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::List { span, .. } | Self::Map { span, .. } => *span,
        }
    }
}

/// One service `env_file` entry with short or long syntax retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentFile {
    /// Scalar path syntax.
    Short(Located<String>),
    /// Mapping syntax with path options.
    Long(Box<LongEnvironmentFile>),
}

impl EnvironmentFile {
    /// Returns the environment-file path in either syntax form.
    #[must_use]
    pub const fn path(&self) -> Option<&Located<String>> {
        match self {
            Self::Short(path) => Some(path),
            Self::Long(value) => value.path(),
        }
    }
}

/// One mapping-syntax service `env_file` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongEnvironmentFile {
    span: SourceSpan,
    path: Option<Located<String>>,
    required: Option<Located<BooleanValue>>,
    format: Option<EnvironmentFileFormat>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl LongEnvironmentFile {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            path: None,
            required: None,
            format: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_path(&mut self, value: Located<String>) {
        self.path = Some(value);
    }

    pub(super) fn set_required(&mut self, value: Located<BooleanValue>) {
        self.required = Some(value);
    }

    pub(super) fn set_format(&mut self, value: EnvironmentFileFormat) {
        self.format = Some(value);
    }

    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }

    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete long-syntax entry span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the required environment-file path.
    #[must_use]
    pub const fn path(&self) -> Option<&Located<String>> {
        self.path.as_ref()
    }

    /// Returns the explicit required-file choice; absence means Compose's default `true`.
    #[must_use]
    pub const fn required(&self) -> Option<&Located<BooleanValue>> {
        self.required.as_ref()
    }

    /// Returns the explicit file format; absence means Compose's default parser.
    #[must_use]
    pub const fn format(&self) -> Option<&EnvironmentFileFormat> {
        self.format.as_ref()
    }

    /// Returns retained `x-` fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns unrecognized long-syntax fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// Raw-preserving long-syntax `env_file.format` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentFileFormat {
    raw: Located<String>,
    kind: EnvironmentFileFormatKind,
}

impl EnvironmentFileFormat {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let kind = EnvironmentFileFormatKind::classify(raw.value());
        Self { raw, kind }
    }

    /// Returns the authored format scalar and its source span.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns the non-destructive format classification.
    #[must_use]
    pub const fn kind(&self) -> EnvironmentFileFormatKind {
        self.kind
    }

    /// Reports whether Compose defines this value or interpolation still defers it.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        !matches!(self.kind, EnvironmentFileFormatKind::Other)
    }
}

/// Classification of an `env_file.format` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EnvironmentFileFormatKind {
    /// Compose's raw environment-file parser.
    Raw,
    /// A value that still contains interpolation.
    Expression,
    /// An invalid or provider-specific value retained for diagnostics.
    Other,
}

impl EnvironmentFileFormatKind {
    pub(crate) fn classify(value: &str) -> Self {
        match value {
            "raw" => Self::Raw,
            value if value.contains('$') => Self::Expression,
            _ => Self::Other,
        }
    }
}
