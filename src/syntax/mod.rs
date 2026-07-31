//! Loss-aware YAML syntax documents.

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::source::{LineColumn, SourceId, SourceSpan, line_column};
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use yaml_edit::{Parse, ParseErrorKind, YamlFile};

/// A generic YAML syntax error not covered by a more specific code.
pub const YAML_SYNTAX_ERROR: DiagnosticCode = DiagnosticCode::new("compose.yaml.syntax");

/// A flow sequence is missing its closing bracket.
pub const YAML_UNCLOSED_FLOW_SEQUENCE: DiagnosticCode =
    DiagnosticCode::new("compose.yaml.unclosed-flow-sequence");

/// A flow mapping is missing its closing brace.
pub const YAML_UNCLOSED_FLOW_MAPPING: DiagnosticCode =
    DiagnosticCode::new("compose.yaml.unclosed-flow-mapping");

/// A quoted scalar is missing its closing quote.
pub const YAML_UNTERMINATED_STRING: DiagnosticCode =
    DiagnosticCode::new("compose.yaml.unterminated-string");

/// A loss-aware YAML document and its original source text.
///
/// The underlying concrete syntax tree is intentionally private so `ComposeLens` can maintain a
/// stable API independently of its parser dependency. Rendering this initial representation emits
/// the concrete syntax tree without normalization, interpolation, or environment access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxDocument {
    source_id: SourceId,
    source: Arc<str>,
    parse: Parse<YamlFile>,
}

impl SyntaxDocument {
    /// Parses YAML into a loss-aware document and structured diagnostics.
    ///
    /// Recoverable YAML errors are returned in [`SyntaxParse::diagnostics`] while the syntax
    /// document remains available. Only sources too large for the concrete syntax tree produce a
    /// fatal error.
    ///
    /// # Errors
    ///
    /// Returns [`SyntaxParseError`] when the source exceeds the concrete syntax tree's byte-offset
    /// capacity. Malformed YAML is recoverable and is reported through structured diagnostics.
    pub fn parse(
        source_id: SourceId,
        source: impl Into<Arc<str>>,
    ) -> Result<SyntaxParse, SyntaxParseError> {
        let source = source.into();
        if u32::try_from(source.len()).is_err() {
            return Err(SyntaxParseError {
                source_id,
                byte_len: source.len(),
            });
        }

        let parse = YamlFile::parse(&source);
        let diagnostics = parse
            .positioned_errors()
            .iter()
            .map(|error| syntax_diagnostic(source_id, source.len(), error))
            .collect();

        Ok(SyntaxParse {
            document: Self {
                source_id,
                source,
                parse,
            },
            diagnostics,
        })
    }

    /// Returns the source identifier supplied by the caller.
    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// Returns the original source text.
    #[must_use]
    pub fn source_text(&self) -> &str {
        &self.source
    }

    /// Returns the span covering the complete source text.
    #[must_use]
    pub fn source_span(&self) -> SourceSpan {
        SourceSpan::from_valid_offsets(self.source_id, 0, self.source.len())
    }

    /// Returns the source text covered by a span from this document.
    #[must_use]
    pub fn text(&self, span: SourceSpan) -> Option<&str> {
        if span.source_id() != self.source_id || span.end() > self.source.len() {
            return None;
        }

        self.source.get(span.range())
    }

    /// Converts a byte offset in this document to a one-based line and column.
    #[must_use]
    pub fn line_column(&self, byte_offset: usize) -> Option<LineColumn> {
        line_column(&self.source, byte_offset)
    }

    /// Returns the number of YAML documents in the stream.
    #[must_use]
    pub fn document_count(&self) -> usize {
        self.parse.tree().documents().count()
    }

    /// Returns the number of comment tokens retained by the concrete syntax tree.
    #[must_use]
    pub fn comment_count(&self) -> usize {
        self.parse.tree().comments().count()
    }

    /// Renders the concrete syntax tree without semantic changes.
    #[must_use]
    pub fn render_preserved(&self) -> String {
        self.parse.tree().to_string()
    }
}

/// The recoverable result of parsing one YAML source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxParse {
    document: SyntaxDocument,
    diagnostics: Vec<Diagnostic>,
}

impl SyntaxParse {
    /// Returns the loss-aware syntax document.
    #[must_use]
    pub const fn document(&self) -> &SyntaxDocument {
        &self.document
    }

    /// Returns all syntax diagnostics in source order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether parsing produced no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == Severity::Error)
    }

    /// Separates the document and diagnostics.
    #[must_use]
    pub fn into_parts(self) -> (SyntaxDocument, Vec<Diagnostic>) {
        (self.document, self.diagnostics)
    }
}

/// A source that exceeds the concrete syntax tree's byte-offset capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxParseError {
    source_id: SourceId,
    byte_len: usize,
}

impl SyntaxParseError {
    /// Returns the source identifier supplied to the parser.
    #[must_use]
    pub const fn source_id(self) -> SourceId {
        self.source_id
    }

    /// Returns the rejected source length in bytes.
    #[must_use]
    pub const fn byte_len(self) -> usize {
        self.byte_len
    }
}

impl fmt::Display for SyntaxParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} contains {} bytes, exceeding the YAML syntax tree limit",
            self.source_id, self.byte_len
        )
    }
}

impl Error for SyntaxParseError {}

fn syntax_diagnostic(
    source_id: SourceId,
    source_len: usize,
    error: &yaml_edit::PositionedParseError,
) -> Diagnostic {
    let (code, message) = match error.kind {
        ParseErrorKind::UnclosedFlowSequence => (
            YAML_UNCLOSED_FLOW_SEQUENCE,
            "flow sequence is missing a closing `]`",
        ),
        ParseErrorKind::UnclosedFlowMapping => (
            YAML_UNCLOSED_FLOW_MAPPING,
            "flow mapping is missing a closing `}`",
        ),
        ParseErrorKind::UnterminatedString => (
            YAML_UNTERMINATED_STRING,
            "quoted scalar is missing its closing quote",
        ),
        ParseErrorKind::Other => (YAML_SYNTAX_ERROR, "invalid YAML syntax"),
    };
    let start = (error.range.start as usize).min(source_len);
    let end = (error.range.end as usize).clamp(start, source_len);
    let span = SourceSpan::from_valid_offsets(source_id, start, end);

    Diagnostic::new(code, Severity::Error, message)
        .with_label(DiagnosticLabel::primary(span, "syntax error"))
}

#[cfg(test)]
mod tests {
    use super::SyntaxDocument;
    use crate::source::SourceId;

    fn assert_send_and_sync<T: Send + Sync>() {}

    #[test]
    fn syntax_documents_are_send_and_sync() {
        assert_send_and_sync::<SyntaxDocument>();
    }

    #[test]
    fn parsing_never_reads_the_process_environment() -> Result<(), Box<dyn std::error::Error>> {
        let source = "services:\n  app:\n    image: ${COMPOSE_LENS_SECRET}\n";
        let parsed = SyntaxDocument::parse(SourceId::new(1), source)?;

        assert_eq!(parsed.document().render_preserved(), source);
        assert!(parsed.is_valid());
        Ok(())
    }
}
