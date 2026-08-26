//! Explicit, non-destructive Compose variable interpolation.

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::source::{SourceId, SourceSpan};
use crate::syntax::SyntaxDocument;
use std::collections::BTreeMap;
use std::fmt;

/// An unset direct substitution was replaced according to the configured policy.
pub const UNSET_VARIABLE: DiagnosticCode = DiagnosticCode::new("compose.interpolation.unset-variable");

/// A required interpolation variable was unset or empty.
pub const REQUIRED_VARIABLE: DiagnosticCode = DiagnosticCode::new("compose.interpolation.required-variable");

/// A braced interpolation expression is malformed or unsupported.
pub const INVALID_EXPRESSION: DiagnosticCode = DiagnosticCode::new("compose.interpolation.invalid-expression");

/// Nested interpolation exceeded the configured safety limit.
pub const NESTING_LIMIT: DiagnosticCode = DiagnosticCode::new("compose.interpolation.nesting-limit");

/// One value supplied by an explicit interpolation environment.
#[derive(Clone, PartialEq, Eq)]
pub struct EnvironmentValue {
    value: String,
    sensitive: bool,
}

impl fmt::Debug for EnvironmentValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EnvironmentValue")
            .field("value", &if self.sensitive { "<redacted>" } else { &self.value })
            .field("sensitive", &self.sensitive)
            .finish()
    }
}

impl EnvironmentValue {
    /// Creates a non-sensitive value.
    #[must_use]
    pub fn plain(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            sensitive: false,
        }
    }

    /// Creates a value whose use makes the interpolation result sensitive.
    #[must_use]
    pub fn sensitive(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            sensitive: true,
        }
    }

    /// Returns the supplied value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Reports whether callers should redact the value when displaying the result.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

/// Supplies variables to interpolation without granting implicit process-environment access.
pub trait EnvironmentProvider {
    /// Returns a variable value, or `None` when the variable is unset.
    fn get(&self, name: &str) -> Option<EnvironmentValue>;
}

/// An explicit environment that never contains a variable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EmptyEnvironment;

impl EnvironmentProvider for EmptyEnvironment {
    fn get(&self, _name: &str) -> Option<EnvironmentValue> {
        None
    }
}

/// A deterministic caller-owned interpolation environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapEnvironment {
    values: BTreeMap<String, EnvironmentValue>,
}

impl MapEnvironment {
    /// Creates an empty environment map.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    /// Inserts a non-sensitive value and returns the replaced value, if any.
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) -> Option<EnvironmentValue> {
        self.values.insert(name.into(), EnvironmentValue::plain(value))
    }

    /// Inserts a sensitive value and returns the replaced value, if any.
    pub fn insert_sensitive(&mut self, name: impl Into<String>, value: impl Into<String>) -> Option<EnvironmentValue> {
        self.values.insert(name.into(), EnvironmentValue::sensitive(value))
    }

    /// Inserts an already classified value and returns the replaced value, if any.
    pub fn insert_value(&mut self, name: impl Into<String>, value: EnvironmentValue) -> Option<EnvironmentValue> {
        self.values.insert(name.into(), value)
    }

    /// Returns the number of configured variables.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Reports whether no variables are configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl EnvironmentProvider for MapEnvironment {
    fn get(&self, name: &str) -> Option<EnvironmentValue> {
        self.values.get(name).cloned()
    }
}

/// Behavior for an unset direct `$VAR` or `${VAR}` substitution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissingVariablePolicy {
    /// Match Compose's documented behavior: warn and substitute an empty string.
    EmptyWithWarning,
    /// Warn and retain the original expression in the recovered value.
    PreserveWithWarning,
    /// Emit an error and retain the original expression in the recovered value.
    Error,
}

/// Controls one explicit interpolation operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterpolationOptions {
    missing_variable: MissingVariablePolicy,
    max_nesting: usize,
}

impl InterpolationOptions {
    /// Creates options using the requested missing-variable policy.
    #[must_use]
    pub const fn new(missing_variable: MissingVariablePolicy) -> Self {
        Self {
            missing_variable,
            max_nesting: 32,
        }
    }

    /// Sets the maximum nested-expression depth; zero is promoted to one.
    #[must_use]
    pub fn with_max_nesting(mut self, max_nesting: usize) -> Self {
        self.max_nesting = max_nesting.max(1);
        self
    }

    /// Returns the direct missing-variable policy.
    #[must_use]
    pub const fn missing_variable(self) -> MissingVariablePolicy {
        self.missing_variable
    }

    /// Returns the maximum nested-expression depth.
    #[must_use]
    pub const fn max_nesting(self) -> usize {
        self.max_nesting
    }
}

impl Default for InterpolationOptions {
    fn default() -> Self {
        Self::new(MissingVariablePolicy::EmptyWithWarning)
    }
}

/// A source-aware scalar supplied to the interpolation kernel.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct InterpolationInput<'a> {
    value: &'a str,
    span: SourceSpan,
    sensitive: bool,
}

impl fmt::Debug for InterpolationInput<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InterpolationInput")
            .field("value", &if self.sensitive { "<redacted>" } else { self.value })
            .field("span", &self.span)
            .field("sensitive", &self.sensitive)
            .finish()
    }
}

impl<'a> InterpolationInput<'a> {
    /// Creates a non-sensitive input.
    #[must_use]
    pub const fn new(value: &'a str, span: SourceSpan) -> Self {
        Self {
            value,
            span,
            sensitive: false,
        }
    }

    /// Marks the authored scalar as sensitive.
    #[must_use]
    pub const fn sensitive(mut self) -> Self {
        self.sensitive = true;
        self
    }

    /// Returns the uninterpolated semantic scalar.
    #[must_use]
    pub const fn value(self) -> &'a str {
        self.value
    }

    /// Returns the scalar's source span.
    #[must_use]
    pub const fn span(self) -> SourceSpan {
        self.span
    }

    /// Reports whether the authored scalar is sensitive.
    #[must_use]
    pub const fn is_sensitive(self) -> bool {
        self.sensitive
    }
}

/// The operator used by one interpolation expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterpolationOperator {
    /// `$VAR` or `${VAR}`.
    Direct,
    /// `${VAR:-default}`.
    DefaultIfUnsetOrEmpty,
    /// `${VAR-default}`.
    DefaultIfUnset,
    /// `${VAR:?message}`.
    RequiredIfUnsetOrEmpty,
    /// `${VAR?message}`.
    RequiredIfUnset,
    /// `${VAR:+alternative}`.
    AlternativeIfSetAndNonEmpty,
    /// `${VAR+alternative}`.
    AlternativeIfSet,
}

/// How one expression contributed to the resolved value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubstitutionOutcome {
    /// An environment value was inserted.
    Environment,
    /// A default operand was inserted.
    Default,
    /// An alternative operand was inserted.
    Alternative,
    /// The expression intentionally produced an empty string.
    Empty,
    /// A direct variable was unset.
    Missing,
    /// A required variable was unset or empty.
    RequiredMissing,
}

/// Provenance for one evaluated variable expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Substitution {
    name: String,
    operator: InterpolationOperator,
    outcome: SubstitutionOutcome,
    span: SourceSpan,
    sensitive: bool,
}

impl Substitution {
    /// Returns the referenced variable name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the expression operator.
    #[must_use]
    pub const fn operator(&self) -> InterpolationOperator {
        self.operator
    }

    /// Returns how the expression contributed to the result.
    #[must_use]
    pub const fn outcome(&self) -> SubstitutionOutcome {
        self.outcome
    }

    /// Returns the containing scalar's source span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Reports whether this substitution inserted sensitive content.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

/// A recoverable, non-destructive interpolation result.
#[derive(Clone, PartialEq, Eq)]
pub struct InterpolationResult {
    original: String,
    resolved: String,
    span: SourceSpan,
    sensitive: bool,
    substitutions: Vec<Substitution>,
    diagnostics: Vec<Diagnostic>,
}

impl fmt::Debug for InterpolationResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("InterpolationResult");
        if self.sensitive {
            debug.field("original", &"<redacted>").field("resolved", &"<redacted>");
        } else {
            debug
                .field("original", &self.original)
                .field("resolved", &self.resolved);
        }
        debug
            .field("span", &self.span)
            .field("sensitive", &self.sensitive)
            .field("substitutions", &self.substitutions)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}

/// A non-destructive interpolation overlay for one syntax document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentInterpolation {
    source_id: SourceId,
    values: Vec<InterpolationResult>,
    diagnostics: Vec<Diagnostic>,
}

impl DocumentInterpolation {
    /// Returns the interpolated source-document identifier.
    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// Returns eligible value-scalar results in source order.
    #[must_use]
    pub fn values(&self) -> &[InterpolationResult] {
        &self.values
    }

    /// Finds the result for an exact value-scalar span.
    #[must_use]
    pub fn value(&self, span: SourceSpan) -> Option<&InterpolationResult> {
        self.values.iter().find(|value| value.span == span)
    }

    /// Returns aggregated interpolation diagnostics in source order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether no value produced an error diagnostic.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == Severity::Error)
    }
}

impl InterpolationResult {
    /// Returns the uninterpolated semantic scalar.
    #[must_use]
    pub fn original(&self) -> &str {
        &self.original
    }

    /// Returns the recovered resolved value.
    ///
    /// Required-variable and invalid-expression errors retain the offending expression so callers
    /// can continue analysis without silently losing source intent.
    #[must_use]
    pub fn resolved(&self) -> &str {
        &self.resolved
    }

    /// Returns the source span of the containing scalar.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Reports whether the input or an inserted value is sensitive.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }

    /// Returns evaluated expressions in evaluation order.
    #[must_use]
    pub fn substitutions(&self) -> &[Substitution] {
        &self.substitutions
    }

    /// Returns interpolation diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether interpolation emitted no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == Severity::Error)
    }
}

/// Interpolates one scalar using Compose's documented direct, default, required, and alternative
/// operators.
#[must_use]
pub fn interpolate(input: InterpolationInput<'_>, environment: &dyn EnvironmentProvider) -> InterpolationResult {
    interpolate_with_options(input, environment, InterpolationOptions::default())
}

/// Interpolates one scalar with explicit missing-variable and nesting policies.
#[must_use]
pub fn interpolate_with_options(
    input: InterpolationInput<'_>,
    environment: &dyn EnvironmentProvider,
    options: InterpolationOptions,
) -> InterpolationResult {
    let mut context = Context {
        environment,
        options,
        span: input.span,
        diagnostics: Vec::new(),
        substitutions: Vec::new(),
    };
    let fragment = context.interpolate_text(input.value, 0);
    InterpolationResult {
        original: input.value.to_owned(),
        resolved: fragment.value,
        span: input.span,
        sensitive: input.sensitive || fragment.sensitive,
        substitutions: context.substitutions,
        diagnostics: context.diagnostics,
    }
}

/// Interpolates eligible YAML value scalars in one syntax document without modifying its source.
///
/// Mapping keys, single-quoted scalars, literal block scalars, and folded block scalars are not
/// eligible. The returned overlay contains only values that included a dollar sign.
#[must_use]
pub fn interpolate_document(document: &SyntaxDocument, environment: &dyn EnvironmentProvider) -> DocumentInterpolation {
    interpolate_document_with_options(document, environment, InterpolationOptions::default())
}

/// Interpolates eligible YAML value scalars with explicit options.
#[must_use]
pub fn interpolate_document_with_options(
    document: &SyntaxDocument,
    environment: &dyn EnvironmentProvider,
    options: InterpolationOptions,
) -> DocumentInterpolation {
    let values: Vec<_> = document
        .interpolatable_value_scalars()
        .into_iter()
        .map(|value| interpolate_with_options(InterpolationInput::new(&value.value, value.span), environment, options))
        .collect();
    let diagnostics = values
        .iter()
        .flat_map(|value| value.diagnostics.iter().cloned())
        .collect();
    DocumentInterpolation {
        source_id: document.source_id(),
        values,
        diagnostics,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Fragment {
    value: String,
    sensitive: bool,
}

impl Fragment {
    fn plain(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            sensitive: false,
        }
    }
}

struct Context<'a> {
    environment: &'a dyn EnvironmentProvider,
    options: InterpolationOptions,
    span: SourceSpan,
    diagnostics: Vec<Diagnostic>,
    substitutions: Vec<Substitution>,
}

impl Context<'_> {
    fn interpolate_text(&mut self, text: &str, depth: usize) -> Fragment {
        if depth > self.options.max_nesting {
            self.diagnostics.push(
                Diagnostic::new(
                    NESTING_LIMIT,
                    Severity::Error,
                    "interpolation nesting exceeds the configured safety limit",
                )
                .with_label(DiagnosticLabel::primary(self.span, "nested expression limit reached")),
            );
            return Fragment::plain(text);
        }

        let mut resolved = String::with_capacity(text.len());
        let mut sensitive = false;
        let mut cursor = 0;
        while let Some(relative) = text[cursor..].find('$') {
            let dollar = cursor + relative;
            resolved.push_str(&text[cursor..dollar]);
            let after_dollar = dollar + 1;
            if after_dollar == text.len() {
                resolved.push('$');
                cursor = after_dollar;
                break;
            }

            let next = text.as_bytes()[after_dollar];
            if next == b'$' {
                resolved.push('$');
                cursor = after_dollar + 1;
                continue;
            }
            if next == b'{' {
                let expression_start = after_dollar + 1;
                let Some(close) = find_closing_brace(text, expression_start) else {
                    self.invalid_expression();
                    resolved.push_str(&text[dollar..]);
                    cursor = text.len();
                    break;
                };
                let expression = &text[expression_start..close];
                let original = &text[dollar..=close];
                let fragment = self.evaluate_braced(expression, original, depth);
                resolved.push_str(&fragment.value);
                sensitive |= fragment.sensitive;
                cursor = close + 1;
                continue;
            }
            if is_name_start(next) {
                let mut end = after_dollar + 1;
                while end < text.len() && is_name_continue(text.as_bytes()[end]) {
                    end += 1;
                }
                let name = &text[after_dollar..end];
                let original = &text[dollar..end];
                let fragment = self.evaluate(name, InterpolationOperator::Direct, "", original, depth);
                resolved.push_str(&fragment.value);
                sensitive |= fragment.sensitive;
                cursor = end;
                continue;
            }

            resolved.push('$');
            cursor = after_dollar;
        }
        resolved.push_str(&text[cursor..]);
        Fragment {
            value: resolved,
            sensitive,
        }
    }

    fn evaluate_braced(&mut self, expression: &str, original: &str, depth: usize) -> Fragment {
        let Some((name, operator, operand)) = parse_braced_expression(expression) else {
            self.invalid_expression();
            return Fragment::plain(original);
        };
        self.evaluate(name, operator, operand, original, depth)
    }

    fn evaluate(
        &mut self,
        name: &str,
        operator: InterpolationOperator,
        operand: &str,
        original: &str,
        depth: usize,
    ) -> Fragment {
        let environment = self.environment.get(name);
        let is_set = environment.is_some();
        let is_non_empty = environment.as_ref().is_some_and(|value| !value.value.is_empty());

        let (fragment, outcome) = match operator {
            InterpolationOperator::Direct => environment.map_or_else(
                || self.missing_direct(name, original),
                |value| {
                    let fragment = Fragment {
                        value: value.value,
                        sensitive: value.sensitive,
                    };
                    (fragment, SubstitutionOutcome::Environment)
                },
            ),
            InterpolationOperator::DefaultIfUnsetOrEmpty if !is_non_empty => {
                (self.interpolate_text(operand, depth + 1), SubstitutionOutcome::Default)
            }
            InterpolationOperator::DefaultIfUnset if !is_set => {
                (self.interpolate_text(operand, depth + 1), SubstitutionOutcome::Default)
            }
            InterpolationOperator::RequiredIfUnsetOrEmpty if !is_non_empty => {
                self.required_missing(name);
                (Fragment::plain(original), SubstitutionOutcome::RequiredMissing)
            }
            InterpolationOperator::RequiredIfUnset if !is_set => {
                self.required_missing(name);
                (Fragment::plain(original), SubstitutionOutcome::RequiredMissing)
            }
            InterpolationOperator::AlternativeIfSetAndNonEmpty if is_non_empty => (
                self.interpolate_text(operand, depth + 1),
                SubstitutionOutcome::Alternative,
            ),
            InterpolationOperator::AlternativeIfSet if is_set => (
                self.interpolate_text(operand, depth + 1),
                SubstitutionOutcome::Alternative,
            ),
            InterpolationOperator::AlternativeIfSetAndNonEmpty | InterpolationOperator::AlternativeIfSet => {
                (Fragment::plain(""), SubstitutionOutcome::Empty)
            }
            InterpolationOperator::DefaultIfUnsetOrEmpty
            | InterpolationOperator::DefaultIfUnset
            | InterpolationOperator::RequiredIfUnsetOrEmpty
            | InterpolationOperator::RequiredIfUnset => {
                let value = environment.unwrap_or_else(|| EnvironmentValue::plain(""));
                (
                    Fragment {
                        value: value.value,
                        sensitive: value.sensitive,
                    },
                    SubstitutionOutcome::Environment,
                )
            }
        };

        self.substitutions.push(Substitution {
            name: name.to_owned(),
            operator,
            outcome,
            span: self.span,
            sensitive: fragment.sensitive,
        });
        fragment
    }

    fn missing_direct(&mut self, name: &str, original: &str) -> (Fragment, SubstitutionOutcome) {
        let (severity, value) = match self.options.missing_variable {
            MissingVariablePolicy::EmptyWithWarning => (Severity::Warning, ""),
            MissingVariablePolicy::PreserveWithWarning => (Severity::Warning, original),
            MissingVariablePolicy::Error => (Severity::Error, original),
        };
        self.diagnostics.push(
            Diagnostic::new(
                UNSET_VARIABLE,
                severity,
                format!("interpolation variable `{name}` is not set"),
            )
            .with_label(DiagnosticLabel::primary(self.span, "unresolved variable expression")),
        );
        (Fragment::plain(value), SubstitutionOutcome::Missing)
    }

    fn required_missing(&mut self, name: &str) {
        self.diagnostics.push(
            Diagnostic::new(
                REQUIRED_VARIABLE,
                Severity::Error,
                format!("required interpolation variable `{name}` is unset or empty"),
            )
            .with_label(DiagnosticLabel::primary(self.span, "required variable is unavailable")),
        );
    }

    fn invalid_expression(&mut self) {
        self.diagnostics.push(
            Diagnostic::new(
                INVALID_EXPRESSION,
                Severity::Error,
                "interpolation expression is malformed or unsupported",
            )
            .with_label(DiagnosticLabel::primary(self.span, "invalid interpolation expression")),
        );
    }
}

fn parse_braced_expression(expression: &str) -> Option<(&str, InterpolationOperator, &str)> {
    let bytes = expression.as_bytes();
    let first = *bytes.first()?;
    if !is_name_start(first) {
        return None;
    }
    let mut name_end = 1;
    while name_end < bytes.len() && is_name_continue(bytes[name_end]) {
        name_end += 1;
    }
    let name = &expression[..name_end];
    let remainder = &expression[name_end..];
    if remainder.is_empty() {
        return Some((name, InterpolationOperator::Direct, ""));
    }

    for (prefix, operator) in [
        (":-", InterpolationOperator::DefaultIfUnsetOrEmpty),
        (":?", InterpolationOperator::RequiredIfUnsetOrEmpty),
        (":+", InterpolationOperator::AlternativeIfSetAndNonEmpty),
        ("-", InterpolationOperator::DefaultIfUnset),
        ("?", InterpolationOperator::RequiredIfUnset),
        ("+", InterpolationOperator::AlternativeIfSet),
    ] {
        if let Some(operand) = remainder.strip_prefix(prefix) {
            return Some((name, operator, operand));
        }
    }
    None
}

fn find_closing_brace(text: &str, start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut cursor = start;
    while cursor < text.len() {
        if text[cursor..].starts_with("${") {
            depth += 1;
            cursor += 2;
            continue;
        }
        let character = text[cursor..].chars().next()?;
        if character == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(cursor);
            }
        }
        cursor += character.len_utf8();
    }
    None
}

const fn is_name_start(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphabetic()
}

const fn is_name_continue(byte: u8) -> bool {
    is_name_start(byte) || byte.is_ascii_digit()
}
