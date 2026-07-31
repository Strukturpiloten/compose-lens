//! Stable, source-aware diagnostics.

use crate::source::SourceSpan;
use std::fmt;

/// A stable machine-readable diagnostic identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagnosticCode(&'static str);

impl DiagnosticCode {
    /// Creates a diagnostic code from a stable static string.
    #[must_use]
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    /// Returns the code as text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// The severity assigned to a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// The input cannot be interpreted as requested.
    Error,
    /// The input is usable but deserves attention.
    Warning,
    /// Additional information that does not indicate a problem.
    Note,
}

/// The role of a source label in a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LabelKind {
    /// The location primarily responsible for the diagnostic.
    Primary,
    /// A related location that adds context.
    Secondary,
}

/// A message attached to a source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    kind: LabelKind,
    span: SourceSpan,
    message: String,
}

impl DiagnosticLabel {
    /// Creates a primary source label.
    #[must_use]
    pub fn primary(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            kind: LabelKind::Primary,
            span,
            message: message.into(),
        }
    }

    /// Creates a secondary source label.
    #[must_use]
    pub fn secondary(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            kind: LabelKind::Secondary,
            span,
            message: message.into(),
        }
    }

    /// Returns the label role.
    #[must_use]
    pub const fn kind(&self) -> LabelKind {
        self.kind
    }

    /// Returns the labeled source span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the human-readable label.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// A structured diagnostic with stable identity and source labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    code: DiagnosticCode,
    severity: Severity,
    message: String,
    labels: Vec<DiagnosticLabel>,
    notes: Vec<String>,
}

impl Diagnostic {
    /// Creates a diagnostic without source labels.
    #[must_use]
    pub fn new(code: DiagnosticCode, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            code,
            severity,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Appends a source label.
    #[must_use]
    pub fn with_label(mut self, label: DiagnosticLabel) -> Self {
        self.labels.push(label);
        self
    }

    /// Appends a human-readable note.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Returns the stable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    /// Returns the severity.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Returns the human-readable summary.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the source labels in display order.
    #[must_use]
    pub fn labels(&self) -> &[DiagnosticLabel] {
        &self.labels
    }

    /// Returns the additional notes in display order.
    #[must_use]
    pub fn notes(&self) -> &[String] {
        &self.notes
    }
}
