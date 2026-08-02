//! Atomic preservation-oriented edits over exact YAML value-scalar spans.

use super::write_quoted;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::source::SourceSpan;
use crate::syntax::SyntaxDocument;
use std::fmt;
use yaml_edit::{ScalarType, ScalarValue, YamlFile};

/// An edit targets a source document other than the supplied syntax document.
pub const EDIT_SOURCE_MISMATCH: DiagnosticCode = DiagnosticCode::new("compose.edit.source-mismatch");

/// An edit target is not exactly one YAML value scalar.
pub const EDIT_TARGET_NOT_SCALAR: DiagnosticCode = DiagnosticCode::new("compose.edit.target-not-scalar");

/// Two requested edits modify overlapping source ranges.
pub const EDIT_OVERLAP: DiagnosticCode = DiagnosticCode::new("compose.edit.overlap");

/// The authored scalar style cannot yet be replaced without risking surrounding syntax.
pub const EDIT_UNSUPPORTED_SCALAR_STYLE: DiagnosticCode = DiagnosticCode::new("compose.edit.unsupported-scalar-style");

/// A requested numeric spelling is not one complete YAML number scalar.
pub const EDIT_INVALID_NUMBER: DiagnosticCode = DiagnosticCode::new("compose.edit.invalid-number");

/// A typed scalar value to place at an existing YAML value-scalar span.
#[derive(Clone, PartialEq, Eq)]
pub struct ReplacementScalar {
    kind: ReplacementKind,
    sensitive: bool,
}

#[derive(Clone, PartialEq, Eq)]
enum ReplacementKind {
    String(String),
    Boolean(bool),
    Number(String),
    Null,
}

impl ReplacementScalar {
    /// Creates a non-sensitive YAML string replacement.
    #[must_use]
    pub fn string(value: impl Into<String>) -> Self {
        Self {
            kind: ReplacementKind::String(value.into()),
            sensitive: false,
        }
    }

    /// Creates a sensitive YAML string replacement.
    ///
    /// The value remains available only through the explicit edit operation. `Debug` output for
    /// the replacement, containing edit, and successful result is redacted.
    #[must_use]
    pub fn sensitive_string(value: impl Into<String>) -> Self {
        Self {
            kind: ReplacementKind::String(value.into()),
            sensitive: true,
        }
    }

    /// Creates a YAML boolean replacement.
    #[must_use]
    pub const fn boolean(value: bool) -> Self {
        Self {
            kind: ReplacementKind::Boolean(value),
            sensitive: false,
        }
    }

    /// Creates a YAML number replacement whose spelling is validated when the edit is applied.
    #[must_use]
    pub fn number(value: impl Into<String>) -> Self {
        Self {
            kind: ReplacementKind::Number(value.into()),
            sensitive: false,
        }
    }

    /// Creates an explicit YAML null replacement.
    #[must_use]
    pub const fn null() -> Self {
        Self {
            kind: ReplacementKind::Null,
            sensitive: false,
        }
    }

    /// Reports whether the replacement contains sensitive caller-supplied content.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

impl fmt::Debug for ReplacementScalar {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("ReplacementScalar");
        match &self.kind {
            ReplacementKind::String(value) => {
                debug.field("kind", &"String");
                debug.field("value", &if self.sensitive { "<redacted>" } else { value });
            }
            ReplacementKind::Boolean(value) => {
                debug.field("kind", &"Boolean");
                debug.field("value", value);
            }
            ReplacementKind::Number(value) => {
                debug.field("kind", &"Number");
                debug.field("value", value);
            }
            ReplacementKind::Null => {
                debug.field("kind", &"Null");
            }
        }
        debug.field("sensitive", &self.sensitive).finish()
    }
}

/// One exact value-scalar replacement in a syntax document.
#[derive(Clone, PartialEq, Eq)]
pub struct ScalarEdit {
    span: SourceSpan,
    replacement: ReplacementScalar,
}

impl ScalarEdit {
    /// Creates an edit for one exact source span.
    #[must_use]
    pub const fn new(span: SourceSpan, replacement: ReplacementScalar) -> Self {
        Self { span, replacement }
    }

    /// Returns the exact scalar span to replace.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the typed replacement value.
    #[must_use]
    pub const fn replacement(&self) -> &ReplacementScalar {
        &self.replacement
    }
}

impl fmt::Debug for ScalarEdit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ScalarEdit")
            .field("span", &self.span)
            .field("replacement", &self.replacement)
            .finish()
    }
}

/// The result of applying an atomic batch of preservation-oriented scalar edits.
///
/// Any error rejects the complete batch and returns the original source bytes. Successful output
/// can contain sensitive replacement values and is therefore exposed through explicit accessors.
#[derive(Clone, PartialEq, Eq)]
pub struct PreservationEditResult {
    output: String,
    diagnostics: Vec<Diagnostic>,
    sensitive: bool,
}

impl PreservationEditResult {
    /// Returns the edited text, or the unchanged original text when the batch was rejected.
    #[must_use]
    pub fn output(&self) -> &str {
        &self.output
    }

    /// Consumes the result and returns the edited or unchanged original text.
    #[must_use]
    pub fn into_output(self) -> String {
        self.output
    }

    /// Returns edit diagnostics in deterministic validation order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether the complete edit batch was applied.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }

    /// Reports whether successfully edited output contains a sensitive replacement.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

impl fmt::Debug for PreservationEditResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreservationEditResult")
            .field(
                "output",
                &if self.sensitive {
                    "<redacted>"
                } else {
                    self.output.as_str()
                },
            )
            .field("diagnostics", &self.diagnostics)
            .field("sensitive", &self.sensitive)
            .finish()
    }
}

/// Applies exact value-scalar replacements while retaining every unrelated source byte.
///
/// The operation is pure, performs no I/O or implicit processing, and is atomic: any invalid
/// target, replacement, or overlap rejects the complete batch. Mapping keys, collections, aliases,
/// empty values, and block or multiline scalars are not scalar-edit targets in this initial API.
#[must_use]
pub fn apply_preservation_edits(document: &SyntaxDocument, edits: &[ScalarEdit]) -> PreservationEditResult {
    let editable = document.editable_value_scalars();
    let mut diagnostics = Vec::new();
    let mut planned = Vec::new();

    for edit in edits {
        if edit.span.source_id() != document.source_id() {
            diagnostics.push(error(
                EDIT_SOURCE_MISMATCH,
                edit.span,
                "edit target belongs to another source document",
                "source does not match the edited document",
            ));
            continue;
        }
        let Some(target) = editable.iter().find(|target| target.span == edit.span) else {
            diagnostics.push(error(
                EDIT_TARGET_NOT_SCALAR,
                edit.span,
                "edit target is not exactly one YAML value scalar",
                "expected an exact value-scalar span",
            ));
            continue;
        };
        if unsupported_style(&target.raw) {
            diagnostics.push(error(
                EDIT_UNSUPPORTED_SCALAR_STYLE,
                edit.span,
                "authored scalar style is not supported for preservation editing",
                "block and multiline scalars require a dedicated edit operation",
            ));
            continue;
        }
        let Some(replacement) = render_replacement(&edit.replacement, &target.raw) else {
            diagnostics.push(error(
                EDIT_INVALID_NUMBER,
                edit.span,
                "numeric replacement is not one complete YAML number scalar",
                "invalid numeric replacement",
            ));
            continue;
        };
        planned.push(PlannedEdit {
            span: edit.span,
            replacement,
            sensitive: edit.replacement.sensitive,
        });
    }

    planned.sort_by_key(|edit| (edit.span.start(), edit.span.end()));
    for pair in planned.windows(2) {
        if pair[1].span.start() < pair[0].span.end() || pair[1].span == pair[0].span {
            diagnostics.push(
                Diagnostic::new(
                    EDIT_OVERLAP,
                    Severity::Error,
                    "preservation edits must target disjoint source ranges",
                )
                .with_label(DiagnosticLabel::primary(pair[1].span, "overlapping edit"))
                .with_label(DiagnosticLabel::secondary(pair[0].span, "earlier edit")),
            );
        }
    }

    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity() == Severity::Error)
    {
        return PreservationEditResult {
            output: document.source_text().to_owned(),
            diagnostics,
            sensitive: false,
        };
    }

    let sensitive = planned.iter().any(|edit| edit.sensitive);
    let mut output = document.source_text().to_owned();
    for edit in planned.iter().rev() {
        output.replace_range(edit.span.range(), &edit.replacement);
    }
    PreservationEditResult {
        output,
        diagnostics,
        sensitive,
    }
}

#[derive(Debug)]
struct PlannedEdit {
    span: SourceSpan,
    replacement: String,
    sensitive: bool,
}

fn error(code: DiagnosticCode, span: SourceSpan, message: &'static str, label: &'static str) -> Diagnostic {
    Diagnostic::new(code, Severity::Error, message).with_label(DiagnosticLabel::primary(span, label))
}

fn unsupported_style(raw: &str) -> bool {
    raw.starts_with('|') || raw.starts_with('>') || raw.contains(['\n', '\r'])
}

fn render_replacement(replacement: &ReplacementScalar, authored: &str) -> Option<String> {
    match &replacement.kind {
        ReplacementKind::String(value) => Some(render_string(value, authored)),
        ReplacementKind::Boolean(value) => Some(value.to_string()),
        ReplacementKind::Number(value) if scalar_has_type(value, &[ScalarType::Integer, ScalarType::Float]) => {
            Some(value.clone())
        }
        ReplacementKind::Number(_) => None,
        ReplacementKind::Null => Some("null".to_owned()),
    }
}

fn render_string(value: &str, authored: &str) -> String {
    if authored.starts_with('"') {
        return double_quoted(value);
    }
    if authored.starts_with('\'') && single_quoted_safe(value) {
        return format!("'{}'", value.replace('\'', "''"));
    }
    if plain_string_safe(value) {
        return value.to_owned();
    }
    double_quoted(value)
}

fn single_quoted_safe(value: &str) -> bool {
    !value
        .chars()
        .any(|character| character.is_control() || matches!(character, '\u{85}' | '\u{2028}' | '\u{2029}' | '\u{feff}'))
}

fn plain_string_safe(value: &str) -> bool {
    !value.is_empty() && !value.contains(['\n', '\r']) && scalar_has_type(value, &[ScalarType::String])
}

fn scalar_has_type(value: &str, expected: &[ScalarType]) -> bool {
    let parse = YamlFile::parse(value);
    if !parse.ok() {
        return false;
    }
    let file = parse.tree();
    let Some(document) = file.document() else {
        return false;
    };
    let Some(scalar) = document.as_scalar() else {
        return false;
    };
    let position = scalar.byte_range();
    position.start == 0
        && position.end as usize == value.len()
        && expected.contains(&ScalarValue::from_scalar(&scalar).scalar_type())
}

fn double_quoted(value: &str) -> String {
    let mut output = String::new();
    write_quoted(&mut output, value);
    output
}
