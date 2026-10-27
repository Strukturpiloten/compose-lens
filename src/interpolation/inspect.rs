//! Bounded inspection of authored expressions without choosing or evaluating any branch.
use super::{
    INVALID_EXPRESSION, InterpolationInput, InterpolationOperator, find_closing_brace, is_name_continue, is_name_start,
    parse_braced_expression,
};
use crate::{
    diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity},
    source::SourceSpan,
    syntax::ScalarPresentation,
};
use std::{error::Error, fmt, ops::Range, sync::Arc};

/// An interpolation inspection exceeded a fixed resource bound.
pub const INSPECTION_LIMIT: DiagnosticCode = DiagnosticCode::new("compose.interpolation.inspection-limit");
/// Maximum decoded input bytes accepted by interpolation inspection.
pub const INTERPOLATION_INSPECTION_MAX_BYTES: usize = 1_048_576;
/// Maximum total segments, including nested operand segments, in an inspection.
pub const INTERPOLATION_INSPECTION_MAX_SEGMENTS: usize = 65_536;
/// Maximum nested operand depth; the containing scalar starts at depth zero.
pub const INTERPOLATION_INSPECTION_MAX_DEPTH: usize = 32;

/// Which fixed inspection resource bound was exceeded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum InterpolationInspectionErrorKind {
    /// The decoded scalar exceeds the input byte bound.
    InputLimit,
    /// Nested operands exceed the depth bound.
    DepthLimit,
    /// The complete operand tree exceeds the segment bound.
    SegmentLimit,
}
/// A content-free, source-aware inspection failure. No scalar text is stored in this error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterpolationInspectionError {
    kind: InterpolationInspectionErrorKind,
    span: SourceSpan,
    range: Option<Range<usize>>,
}
impl InterpolationInspectionError {
    /// Returns the exceeded bound.
    #[must_use]
    pub const fn kind(&self) -> InterpolationInspectionErrorKind {
        self.kind
    }
    /// Returns the containing authored scalar span, not a decoded-to-source offset translation.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns the decoded-root byte range being inspected, when input size permitted inspection.
    #[must_use]
    pub fn range(&self) -> Option<Range<usize>> {
        self.range.clone()
    }
    /// Creates a diagnostic without echoing any name, operand, or scalar content.
    #[must_use]
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic::new(INSPECTION_LIMIT, Severity::Error, self.to_string()).with_label(DiagnosticLabel::primary(
            self.span,
            "interpolation inspection safety limit reached",
        ))
    }
}
impl fmt::Display for InterpolationInspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            InterpolationInspectionErrorKind::InputLimit => "interpolation inspection input byte limit exceeded",
            InterpolationInspectionErrorKind::DepthLimit => "interpolation inspection operand depth limit exceeded",
            InterpolationInspectionErrorKind::SegmentLimit => "interpolation inspection total segment limit exceeded",
        })
    }
}
impl Error for InterpolationInspectionError {}

#[derive(Clone, Eq, PartialEq)]
struct Evidence {
    root: Arc<str>,
    range: Range<usize>,
    span: SourceSpan,
    sensitive: bool,
}
impl Evidence {
    fn original(&self) -> &str {
        &self.root[self.range.clone()]
    }
}
impl fmt::Debug for Evidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InterpolationEvidence")
            .field("original", &if self.sensitive { "<redacted>" } else { self.original() })
            .field("range", &self.range)
            .field("span", &self.span)
            .field("sensitive", &self.sensitive)
            .finish_non_exhaustive()
    }
}
macro_rules! evidence_accessors {
    () => {
        /// Returns the exact original decoded input covered by this item.
        #[must_use]
        pub fn original(&self) -> &str {
            self.evidence.original()
        }
        /// Returns absolute UTF-8 byte offsets in the complete original decoded input.
        ///
        /// Nested operand ranges use the same root coordinates. They are not raw YAML offsets.
        #[must_use]
        pub fn range(&self) -> Range<usize> {
            self.evidence.range.clone()
        }
        /// Returns the containing authored scalar's source span.
        #[must_use]
        pub const fn span(&self) -> SourceSpan {
            self.evidence.span
        }
        /// Reports whether this item carries caller-protected content.
        #[must_use]
        pub const fn is_sensitive(&self) -> bool {
            self.evidence.sensitive
        }
    };
}

/// Whether a native reference used braces.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum InterpolationReferenceForm {
    /// `$NAME`.
    Unbraced,
    /// `${NAME}` or a braced expression with an operator.
    Braced,
}
/// Literal decoded text after collapsing each native `$$` escape exactly once.
#[derive(Clone, Eq, PartialEq)]
pub struct InterpolationLiteral {
    evidence: Evidence,
    value: String,
}
impl InterpolationLiteral {
    evidence_accessors!();
    /// Returns literal text after one escape pass. This text must not be inspected a second time.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}
impl fmt::Debug for InterpolationLiteral {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InterpolationLiteral")
            .field("evidence", &self.evidence)
            .field(
                "value",
                &if self.is_sensitive() {
                    "<redacted>"
                } else {
                    self.value()
                },
            )
            .finish()
    }
}
/// An unevaluated native reference, with exact name, operator, and operand evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterpolationReference {
    evidence: Evidence,
    name: Range<usize>,
    operator: InterpolationOperator,
    form: InterpolationReferenceForm,
    operator_range: Option<Range<usize>>,
    operand: Option<Range<usize>>,
    operand_segments: Vec<InterpolationSegment>,
}
impl InterpolationReference {
    evidence_accessors!();
    /// Returns the uninterpreted native variable name through an explicit access boundary.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.evidence.root[self.name.clone()]
    }
    /// Returns the existing native operator category; no branch has been chosen.
    #[must_use]
    pub const fn operator(&self) -> InterpolationOperator {
        self.operator
    }
    /// Returns authored braced or unbraced form.
    #[must_use]
    pub const fn form(&self) -> InterpolationReferenceForm {
        self.form
    }
    /// Returns the exact decoded operand, including nested syntax and uncollapsed escapes.
    ///
    /// Direct references return `None`; an explicitly empty non-direct operand returns `Some("")`.
    #[must_use]
    pub fn operand(&self) -> Option<&str> {
        self.operand.as_ref().map(|range| &self.evidence.root[range.clone()])
    }
    /// Returns the syntax-only operand segments, without evaluating any of them.
    #[must_use]
    pub fn operand_segments(&self) -> Option<&[InterpolationSegment]> {
        self.operand.as_ref().map(|_| self.operand_segments.as_slice())
    }
    /// Returns the variable name's absolute decoded-root byte range.
    #[must_use]
    pub fn name_range(&self) -> Range<usize> {
        self.name.clone()
    }
    /// Returns the operator's absolute decoded-root byte range, absent for direct references.
    #[must_use]
    pub fn operator_range(&self) -> Option<Range<usize>> {
        self.operator_range.clone()
    }
    /// Returns the operand's absolute decoded-root byte range, absent for direct references.
    #[must_use]
    pub fn operand_range(&self) -> Option<Range<usize>> {
        self.operand.clone()
    }
}
/// The native syntax issue retained by an invalid segment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum InterpolationIssueKind {
    /// A real `${` expression has no matching closing brace.
    UnclosedBracedExpression,
    /// A braced expression has an empty or invalid variable-name prefix.
    InvalidVariableName,
    /// A valid variable-name prefix is followed by an unsupported operator spelling.
    UnsupportedOperator,
}
/// Original malformed or unsupported braced syntax, retained without evaluation or escape collapse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidInterpolation {
    evidence: Evidence,
    kind: InterpolationIssueKind,
}
impl InvalidInterpolation {
    evidence_accessors!();
    /// Returns a typed syntax issue without requiring a diagnostic-message parser.
    #[must_use]
    pub const fn kind(&self) -> InterpolationIssueKind {
        self.kind
    }
}
/// One ordered native interpolation segment.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum InterpolationSegment {
    /// Literal text, with native dollar escapes collapsed once.
    Literal(InterpolationLiteral),
    /// An unevaluated native variable reference.
    Reference(InterpolationReference),
    /// Retained malformed or unsupported expression syntax.
    Invalid(InvalidInterpolation),
}
impl InterpolationSegment {
    fn evidence(&self) -> &Evidence {
        match self {
            Self::Literal(value) => &value.evidence,
            Self::Reference(value) => &value.evidence,
            Self::Invalid(value) => &value.evidence,
        }
    }
    /// Returns the exact original decoded input covered by this segment.
    #[must_use]
    pub fn original(&self) -> &str {
        self.evidence().original()
    }
    /// Returns absolute byte offsets in the complete original decoded input, not raw YAML offsets.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.evidence().range.clone()
    }
    /// Returns the containing authored scalar's source span.
    #[must_use]
    pub fn span(&self) -> SourceSpan {
        self.evidence().span
    }
    /// Reports whether this segment carries caller-protected content.
    #[must_use]
    pub fn is_sensitive(&self) -> bool {
        self.evidence().sensitive
    }
}
/// A source-aware, unevaluated scalar inspection. All nested ranges share its decoded input.
#[derive(Clone, Eq, PartialEq)]
pub struct InterpolationInspection {
    evidence: Evidence,
    segments: Vec<InterpolationSegment>,
    diagnostics: Vec<Diagnostic>,
}
impl InterpolationInspection {
    evidence_accessors!();
    /// Returns ordered literal, reference, and invalid syntax segments.
    #[must_use]
    pub fn segments(&self) -> &[InterpolationSegment] {
        &self.segments
    }
    /// Returns syntax diagnostics only; absence of a provider never produces missing-variable findings.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    /// Reports whether all authored expression syntax is supported and well formed.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }
}
impl fmt::Debug for InterpolationInspection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InterpolationInspection")
            .field("evidence", &self.evidence)
            .field("segments", &self.segments)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}
/// A scalar deliberately excluded by native document-interpolation policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExcludedInterpolation {
    evidence: Evidence,
    presentation: ScalarPresentation,
}
impl ExcludedInterpolation {
    evidence_accessors!();
    /// Returns the authored presentation that excluded this scalar.
    #[must_use]
    pub const fn presentation(&self) -> ScalarPresentation {
        self.presentation
    }
    /// Reports any dollar syntax without parsing it or collapsing its escapes.
    #[must_use]
    pub fn contains_dollar(&self) -> bool {
        self.original().contains('$')
    }
}
/// Explicit presentation disposition; excluded content is never disguised as an inspected literal.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ScalarInterpolationInspection {
    /// Plain or double-quoted input was inspected without evaluation.
    Inspected(InterpolationInspection),
    /// Single-quoted or block input remains unchanged and explicitly excluded.
    Excluded(ExcludedInterpolation),
}

/// Inspects a caller-selected decoded scalar without a provider, substitution, or environment access.
///
/// Known operators retain their operands regardless of whether evaluation would select that branch.
/// This generic kernel does not infer YAML style or automatically traverse metadata. Use
/// [`inspect_scalar_interpolation`] when native document presentation eligibility matters.
///
/// # Errors
/// Rejects input, depth, or total segment counts exceeding the fixed inspection bounds.
pub fn inspect_interpolation(
    input: InterpolationInput<'_>,
) -> Result<InterpolationInspection, InterpolationInspectionError> {
    check_input(input)?;
    let evidence = Evidence {
        root: Arc::from(input.value()),
        range: 0..input.value().len(),
        span: input.span(),
        sensitive: input.is_sensitive(),
    };
    let mut inspector = Inspector {
        evidence: evidence.clone(),
        nodes: 0,
        diagnostics: Vec::new(),
    };
    let segments = inspector.scan(0..input.value().len(), 0)?;
    Ok(InterpolationInspection {
        evidence,
        segments,
        diagnostics: inspector.diagnostics,
    })
}
/// Inspects eligible decoded input or returns an explicit native presentation exclusion.
///
/// Single-quoted and literal/folded block input is retained unchanged, including all dollar escapes.
/// This preserves the document-interpolation policy; applications decide how excluded dollar syntax
/// affects their own conversion mode. Mapping keys are not made eligible by this scalar-only API.
///
/// # Errors
/// Rejects oversized input for every presentation, and excessive depth/segments for eligible input.
pub fn inspect_scalar_interpolation(
    input: InterpolationInput<'_>,
    presentation: ScalarPresentation,
) -> Result<ScalarInterpolationInspection, InterpolationInspectionError> {
    match presentation {
        ScalarPresentation::Plain | ScalarPresentation::DoubleQuoted => {
            inspect_interpolation(input).map(ScalarInterpolationInspection::Inspected)
        }
        ScalarPresentation::SingleQuoted | ScalarPresentation::Literal | ScalarPresentation::Folded => {
            check_input(input)?;
            Ok(ScalarInterpolationInspection::Excluded(ExcludedInterpolation {
                evidence: Evidence {
                    root: Arc::from(input.value()),
                    range: 0..input.value().len(),
                    span: input.span(),
                    sensitive: input.is_sensitive(),
                },
                presentation,
            }))
        }
    }
}
fn check_input(input: InterpolationInput<'_>) -> Result<(), InterpolationInspectionError> {
    if input.value().len() > INTERPOLATION_INSPECTION_MAX_BYTES {
        return Err(InterpolationInspectionError {
            kind: InterpolationInspectionErrorKind::InputLimit,
            span: input.span(),
            range: None,
        });
    }
    Ok(())
}
struct Inspector {
    evidence: Evidence,
    nodes: usize,
    diagnostics: Vec<Diagnostic>,
}
impl Inspector {
    fn evidence(&self, range: Range<usize>) -> Evidence {
        Evidence {
            range,
            ..self.evidence.clone()
        }
    }
    fn node(&mut self, range: Range<usize>) -> Result<(), InterpolationInspectionError> {
        if self.nodes >= INTERPOLATION_INSPECTION_MAX_SEGMENTS {
            return Err(InterpolationInspectionError {
                kind: InterpolationInspectionErrorKind::SegmentLimit,
                span: self.evidence.span,
                range: Some(range),
            });
        }
        self.nodes += 1;
        Ok(())
    }
    fn literal(
        &mut self,
        segments: &mut Vec<InterpolationSegment>,
        range: Range<usize>,
        value: String,
    ) -> Result<(), InterpolationInspectionError> {
        if !range.is_empty() {
            self.node(range.clone())?;
            segments.push(InterpolationSegment::Literal(InterpolationLiteral {
                evidence: self.evidence(range),
                value,
            }));
        }
        Ok(())
    }
    fn invalid(
        &mut self,
        range: Range<usize>,
        kind: InterpolationIssueKind,
    ) -> Result<InterpolationSegment, InterpolationInspectionError> {
        self.node(range.clone())?;
        self.diagnostics.push(
            Diagnostic::new(
                INVALID_EXPRESSION,
                Severity::Error,
                "interpolation expression is malformed or unsupported",
            )
            .with_label(DiagnosticLabel::primary(
                self.evidence.span,
                "invalid interpolation expression",
            )),
        );
        Ok(InterpolationSegment::Invalid(InvalidInterpolation {
            evidence: self.evidence(range),
            kind,
        }))
    }
    fn braced(
        &mut self,
        token: Range<usize>,
        depth: usize,
    ) -> Result<InterpolationSegment, InterpolationInspectionError> {
        let root = Arc::clone(&self.evidence.root);
        let expression = &root[token.start + 2..token.end - 1];
        let Some((name, operator, operand)) = parse_braced_expression(expression) else {
            let kind = if expression.as_bytes().first().is_none_or(|byte| !is_name_start(*byte)) {
                InterpolationIssueKind::InvalidVariableName
            } else {
                InterpolationIssueKind::UnsupportedOperator
            };
            return self.invalid(token, kind);
        };
        self.node(token.clone())?;
        let name_start = token.start + 2;
        let name_end = name_start + name.len();
        let operand_start = token.end - 1 - operand.len();
        let has_operand = operator != InterpolationOperator::Direct;
        let operand_range = has_operand.then_some(operand_start..token.end - 1);
        let operand_segments = if let Some(operand_range) = &operand_range {
            self.scan(operand_range.clone(), depth + 1)?
        } else {
            Vec::new()
        };
        Ok(InterpolationSegment::Reference(InterpolationReference {
            evidence: self.evidence(token),
            name: name_start..name_end,
            operator,
            form: InterpolationReferenceForm::Braced,
            operator_range: has_operand.then_some(name_end..operand_start),
            operand: operand_range,
            operand_segments,
        }))
    }
    fn scan(
        &mut self,
        range: Range<usize>,
        depth: usize,
    ) -> Result<Vec<InterpolationSegment>, InterpolationInspectionError> {
        if depth > INTERPOLATION_INSPECTION_MAX_DEPTH {
            return Err(InterpolationInspectionError {
                kind: InterpolationInspectionErrorKind::DepthLimit,
                span: self.evidence.span,
                range: Some(range),
            });
        }
        let root = Arc::clone(&self.evidence.root);
        let text = &root[range.clone()];
        let mut segments = Vec::new();
        let mut cursor = 0;
        let mut literal_start = 0;
        let mut literal = String::new();
        while let Some(relative) = text[cursor..].find('$') {
            let dollar = cursor + relative;
            literal.push_str(&text[cursor..dollar]);
            let next = dollar + 1;
            if next == text.len() {
                literal.push('$');
                cursor = next;
                break;
            }
            if text.as_bytes()[next] == b'$' {
                literal.push('$');
                cursor = next + 1;
                continue;
            }
            if text.as_bytes()[next] == b'{' {
                self.literal(
                    &mut segments,
                    range.start + literal_start..range.start + dollar,
                    std::mem::take(&mut literal),
                )?;
                let Some(close) = find_closing_brace(text, next + 1) else {
                    segments.push(self.invalid(
                        range.start + dollar..range.end,
                        InterpolationIssueKind::UnclosedBracedExpression,
                    )?);
                    cursor = text.len();
                    literal_start = cursor;
                    break;
                };
                let token = range.start + dollar..range.start + close + 1;
                segments.push(self.braced(token, depth)?);
                cursor = close + 1;
                literal_start = cursor;
                continue;
            }
            if is_name_start(text.as_bytes()[next]) {
                self.literal(
                    &mut segments,
                    range.start + literal_start..range.start + dollar,
                    std::mem::take(&mut literal),
                )?;
                let mut end = next + 1;
                while end < text.len() && is_name_continue(text.as_bytes()[end]) {
                    end += 1;
                }
                let token = range.start + dollar..range.start + end;
                self.node(token.clone())?;
                segments.push(InterpolationSegment::Reference(InterpolationReference {
                    evidence: self.evidence(token),
                    name: range.start + next..range.start + end,
                    operator: InterpolationOperator::Direct,
                    form: InterpolationReferenceForm::Unbraced,
                    operator_range: None,
                    operand: None,
                    operand_segments: Vec::new(),
                }));
                cursor = end;
                literal_start = cursor;
                continue;
            }
            literal.push('$');
            cursor = next;
        }
        literal.push_str(&text[cursor..]);
        self.literal(&mut segments, range.start + literal_start..range.end, literal)?;
        Ok(segments)
    }
}
