//! Source-aware service provider configuration.

use crate::source::SourceSpan;

use super::{ComposeScalar, FieldReference, Located};

/// One authored scalar or sequence value in a service provider option.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderOptionValue {
    /// One YAML string, number, or boolean scalar.
    Scalar(Located<ComposeScalar>),
    /// An ordered sequence of provider-option items.
    Sequence {
        /// Complete sequence span.
        span: SourceSpan,
        /// Valid and malformed items in authored order.
        items: Vec<ProviderOptionItem>,
    },
}

impl ProviderOptionValue {
    /// Returns the complete value span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Scalar(value) => value.span(),
            Self::Sequence { span, .. } => *span,
        }
    }
}

/// One item in an authored provider-option sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderOptionItem {
    /// A YAML string, number, or boolean scalar.
    Scalar(Located<ComposeScalar>),
    /// A malformed item retained by source span.
    Unmodeled {
        /// Complete malformed-item span.
        span: SourceSpan,
    },
}

impl ProviderOptionItem {
    /// Returns the complete item span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Scalar(value) => value.span(),
            Self::Unmodeled { span } => *span,
        }
    }
}

/// One source-aware provider option mapping entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderOption {
    name: Located<String>,
    value: ProviderOptionValue,
    span: SourceSpan,
}

impl ProviderOption {
    pub(crate) const fn new(name: Located<String>, value: ProviderOptionValue, span: SourceSpan) -> Self {
        Self { name, value, span }
    }

    /// Returns the non-empty option name and source span.
    #[must_use]
    pub const fn name(&self) -> &Located<String> {
        &self.name
    }

    /// Returns the raw scalar or ordered sequence option value.
    #[must_use]
    pub const fn value(&self) -> &ProviderOptionValue {
        &self.value
    }

    /// Returns the complete key/value entry span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// An ordered service provider-options mapping, including an explicitly empty mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderOptions {
    span: SourceSpan,
    entries: Vec<ProviderOption>,
    unmodeled_entries: Vec<FieldReference>,
}

impl ProviderOptions {
    pub(crate) const fn new(
        span: SourceSpan,
        entries: Vec<ProviderOption>,
        unmodeled_entries: Vec<FieldReference>,
    ) -> Self {
        Self {
            span,
            entries,
            unmodeled_entries,
        }
    }

    /// Returns the complete options mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns valid option entries in authored order.
    #[must_use]
    pub fn entries(&self) -> &[ProviderOption] {
        &self.entries
    }

    /// Returns malformed or duplicate option entries retained as evidence.
    #[must_use]
    pub fn unmodeled_entries(&self) -> &[FieldReference] {
        &self.unmodeled_entries
    }
}

/// An authored service `provider` mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    span: SourceSpan,
    type_: Option<Located<String>>,
    options: Option<ProviderOptions>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl Provider {
    pub(crate) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            type_: None,
            options: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(crate) fn set_type(&mut self, value: Located<String>) {
        self.type_ = Some(value);
    }

    pub(crate) fn set_options(&mut self, value: ProviderOptions) {
        self.options = Some(value);
    }

    pub(crate) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(crate) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the complete provider mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the required strict YAML-string provider type when validly authored.
    #[must_use]
    pub const fn type_(&self) -> Option<&Located<String>> {
        self.type_.as_ref()
    }

    /// Returns the optional ordered provider options mapping.
    #[must_use]
    pub const fn options(&self) -> Option<&ProviderOptions> {
        self.options.as_ref()
    }

    /// Returns retained parent `x-*` members.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns retained unknown or malformed parent members.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}
