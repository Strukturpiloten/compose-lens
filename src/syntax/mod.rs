//! Loss-aware YAML syntax documents.

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::source::{LineColumn, SourceId, SourceSpan, line_column};
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use yaml_edit::{
    AnchorRegistry, AsYaml, Mapping, MappingMergedExt, Parse, ParseErrorKind, Scalar, ScalarType, ScalarValue,
    YamlFile, YamlNode,
};

/// A generic YAML syntax error not covered by a more specific code.
pub const YAML_SYNTAX_ERROR: DiagnosticCode = DiagnosticCode::new("compose.yaml.syntax");

/// The private YAML backend did not include the complete source in the YAML document root.
pub const YAML_UNPARSED_INPUT: DiagnosticCode = DiagnosticCode::new("compose.yaml.unparsed-input");

/// A flow sequence is missing its closing bracket.
pub const YAML_UNCLOSED_FLOW_SEQUENCE: DiagnosticCode = DiagnosticCode::new("compose.yaml.unclosed-flow-sequence");

/// A flow mapping is missing its closing brace.
pub const YAML_UNCLOSED_FLOW_MAPPING: DiagnosticCode = DiagnosticCode::new("compose.yaml.unclosed-flow-mapping");

/// A quoted scalar is missing its closing quote.
pub const YAML_UNTERMINATED_STRING: DiagnosticCode = DiagnosticCode::new("compose.yaml.unterminated-string");

/// A loss-aware YAML document and its original source text.
///
/// The underlying concrete syntax tree is intentionally private so `ComposeLens` can maintain a
/// stable API independently of its parser dependency. Preservation rendering emits the original
/// source without normalization, interpolation, or environment access.
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
    pub fn parse(source_id: SourceId, source: impl Into<Arc<str>>) -> Result<SyntaxParse, SyntaxParseError> {
        let source = source.into();
        if u32::try_from(source.len()).is_err() {
            return Err(SyntaxParseError {
                source_id,
                byte_len: source.len(),
            });
        }

        let parser_source = parser_compatible_source(&source);
        let parse = YamlFile::parse(parser_source.as_deref().unwrap_or(&source));
        let mut diagnostics: Vec<_> = parse
            .positioned_errors()
            .iter()
            .map(|error| syntax_diagnostic(source_id, source.len(), error))
            .collect();
        if diagnostics.is_empty() {
            if let Some(document_end) = unparsed_input_offset(&parse, &source) {
                diagnostics.push(unparsed_input_diagnostic(source_id, source.len(), document_end));
            }
        }

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

    /// Returns the original source as preservation-oriented output.
    #[must_use]
    pub fn render_preserved(&self) -> String {
        self.source.to_string()
    }

    pub(crate) fn yaml_file(&self) -> YamlFile {
        self.parse.tree()
    }

    pub(crate) fn interpolatable_value_scalars(&self) -> Vec<ValueScalar> {
        let mut values = Vec::new();
        if let Some(document) = self.parse.tree().document() {
            if let Some(mapping) = document.as_mapping() {
                collect_value_scalars(self.source_id, &self.source, YamlNode::Mapping(mapping), &mut values);
            } else if let Some(sequence) = document.as_sequence() {
                collect_value_scalars(self.source_id, &self.source, YamlNode::Sequence(sequence), &mut values);
            } else if let Some(scalar) = document.as_scalar() {
                collect_value_scalars(self.source_id, &self.source, YamlNode::Scalar(scalar), &mut values);
            }
        }
        values
    }

    pub(crate) fn editable_value_scalars(&self) -> Vec<EditableValueScalar> {
        let mut values = Vec::new();
        if let Some(document) = self.parse.tree().document() {
            if let Some(mapping) = document.as_mapping() {
                collect_editable_value_scalars(self.source_id, &self.source, YamlNode::Mapping(mapping), &mut values);
            } else if let Some(sequence) = document.as_sequence() {
                collect_editable_value_scalars(self.source_id, &self.source, YamlNode::Sequence(sequence), &mut values);
            } else if let Some(scalar) = document.as_scalar() {
                collect_editable_value_scalars(self.source_id, &self.source, YamlNode::Scalar(scalar), &mut values);
            }
        }
        values
    }

    pub(crate) fn merge_root(&self) -> Option<MergeSyntaxValue> {
        let document = self.parse.tree().document()?;
        let root = if let Some(mapping) = document.as_mapping() {
            YamlNode::Mapping(mapping)
        } else if let Some(sequence) = document.as_sequence() {
            YamlNode::Sequence(sequence)
        } else {
            YamlNode::Scalar(document.as_scalar()?)
        };
        let registry = AnchorRegistry::from_document(&document);
        Some(extract_merge_value(
            self.source_id,
            &self.source,
            root,
            &registry,
            &mut Vec::new(),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValueScalar {
    pub(crate) value: String,
    pub(crate) span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditableValueScalar {
    pub(crate) raw: String,
    pub(crate) span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MergeScalarKind {
    String,
    Boolean,
    Number,
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MergeSyntaxScalar {
    pub(crate) raw: String,
    pub(crate) value: String,
    pub(crate) kind: MergeScalarKind,
    pub(crate) span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MergeSyntaxEntry {
    pub(crate) key: MergeSyntaxScalar,
    pub(crate) value: MergeSyntaxValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MergeSyntaxValue {
    Empty(SourceSpan),
    Scalar(MergeSyntaxScalar),
    Mapping {
        entries: Vec<MergeSyntaxEntry>,
        span: SourceSpan,
    },
    Sequence {
        values: Vec<MergeSyntaxValue>,
        span: SourceSpan,
    },
    Alias {
        name: String,
        span: SourceSpan,
    },
    Tagged {
        tag: String,
        value: Box<MergeSyntaxValue>,
        span: SourceSpan,
    },
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

fn syntax_diagnostic(source_id: SourceId, source_len: usize, error: &yaml_edit::PositionedParseError) -> Diagnostic {
    let (code, message) = match error.kind {
        ParseErrorKind::UnclosedFlowSequence => (YAML_UNCLOSED_FLOW_SEQUENCE, "flow sequence is missing a closing `]`"),
        ParseErrorKind::UnclosedFlowMapping => (YAML_UNCLOSED_FLOW_MAPPING, "flow mapping is missing a closing `}`"),
        ParseErrorKind::UnterminatedString => (YAML_UNTERMINATED_STRING, "quoted scalar is missing its closing quote"),
        ParseErrorKind::Other => (YAML_SYNTAX_ERROR, "invalid YAML syntax"),
    };
    let start = (error.range.start as usize).min(source_len);
    let end = (error.range.end as usize).clamp(start, source_len);
    let span = SourceSpan::from_valid_offsets(source_id, start, end);

    Diagnostic::new(code, Severity::Error, message).with_label(DiagnosticLabel::primary(span, "syntax error"))
}

fn unparsed_input_diagnostic(source_id: SourceId, source_len: usize, document_end: usize) -> Diagnostic {
    let span = SourceSpan::from_valid_offsets(source_id, document_end.min(source_len), source_len);

    Diagnostic::new(
        YAML_UNPARSED_INPUT,
        Severity::Error,
        "YAML parser did not include the complete input in the document root",
    )
    .with_label(DiagnosticLabel::primary(span, "input omitted from YAML document"))
    .with_note("the original source remains available, but typed processing must not continue silently")
}

fn unparsed_input_offset(parse: &Parse<YamlFile>, source: &str) -> Option<usize> {
    let document_end = parse
        .tree()
        .document()
        .and_then(|document| {
            document
                .as_node()
                .map(|node| u32::from(node.text_range().end()) as usize)
        })
        .unwrap_or_default();
    source
        .as_bytes()
        .get(document_end..)
        .is_some_and(|suffix| suffix.iter().any(|byte| !byte.is_ascii_whitespace()))
        .then_some(document_end)
}

/// Produces a same-length input for the private YAML backend when it would otherwise treat a
/// comma in a block-style plain scalar as a collection delimiter. Commas remain structural in
/// flow collections and remain untouched in quoted, commented, and block-scalar content.
fn parser_compatible_source(source: &str) -> Option<String> {
    let mut compatible = source.as_bytes().to_vec();
    let mut changed = false;
    let mut offset = 0;
    let mut block_scalar_indent = None;
    let mut state = ParserCompatibilityState::default();

    for line in source.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        let indent = content.bytes().take_while(|byte| *byte == b' ').count();
        let blank = content[indent..].trim().is_empty();

        if let Some(header_indent) = block_scalar_indent {
            if blank || indent > header_indent {
                offset += line.len();
                continue;
            }
            block_scalar_indent = None;
        }

        let (line_changed, starts_block_scalar) = mask_block_plain_commas(content, offset, &mut compatible, &mut state);
        changed |= line_changed;
        if starts_block_scalar {
            block_scalar_indent = Some(indent);
        }
        offset += line.len();
    }

    if changed {
        String::from_utf8(compatible).ok()
    } else {
        None
    }
}

#[derive(Debug, Default)]
struct ParserCompatibilityState {
    flow_depth: u32,
    quote: Option<u8>,
    escaped: bool,
}

fn mask_block_plain_commas(
    content: &str,
    offset: usize,
    compatible: &mut [u8],
    state: &mut ParserCompatibilityState,
) -> (bool, bool) {
    let bytes = content.as_bytes();
    let mut index = 0;
    let mut first_token = true;
    let mut plain_started = false;
    let mut eligible_plain_value = false;
    let mut changed = false;

    while index < bytes.len() {
        let byte = bytes[index];

        if let Some(delimiter) = state.quote {
            if delimiter == b'"' && state.escaped {
                state.escaped = false;
            } else if delimiter == b'"' && byte == b'\\' {
                state.escaped = true;
            } else if byte == delimiter {
                if delimiter == b'\'' && bytes.get(index + 1) == Some(&b'\'') {
                    index += 1;
                } else {
                    state.quote = None;
                }
            }
            index += 1;
            continue;
        }

        if byte == b'#' && (index == 0 || bytes[index - 1].is_ascii_whitespace()) {
            break;
        }

        if eligible_plain_value && !plain_started && matches!(byte, b'!' | b'&') {
            index += 1;
            while bytes.get(index).is_some_and(|byte| !byte.is_ascii_whitespace()) {
                index += 1;
            }
            first_token = false;
            continue;
        }

        if matches!(byte, b'\'' | b'"') && !plain_started {
            state.quote = Some(byte);
            first_token = false;
            index += 1;
            continue;
        }

        if state.flow_depth > 0 {
            match byte {
                b'[' | b'{' => state.flow_depth += 1,
                b']' | b'}' => state.flow_depth -= 1,
                _ => {}
            }
            index += 1;
            continue;
        }

        let token_boundary = index == 0 || bytes[index - 1].is_ascii_whitespace();
        if matches!(byte, b'[' | b'{') && (!plain_started || (eligible_plain_value && token_boundary)) {
            state.flow_depth = 1;
            first_token = false;
            index += 1;
            continue;
        }

        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }

        if first_token && byte == b'-' && bytes.get(index + 1).is_none_or(u8::is_ascii_whitespace) {
            eligible_plain_value = true;
            plain_started = false;
            first_token = false;
            index += 1;
            continue;
        }

        if byte == b':' && bytes.get(index + 1).is_none_or(u8::is_ascii_whitespace) {
            eligible_plain_value = true;
            plain_started = false;
            first_token = false;
            index += 1;
            continue;
        }

        if eligible_plain_value && !plain_started && matches!(byte, b'|' | b'>') {
            return (changed, true);
        }

        if eligible_plain_value && plain_started && byte == b',' {
            compatible[offset + index] = b'_';
            changed = true;
        }

        plain_started = true;
        first_token = false;
        index += 1;
    }

    state.escaped = false;
    (changed, false)
}

pub(crate) fn scalar_raw_from_source(source: &str, scalar: &Scalar) -> String {
    let range = scalar.byte_range();
    source
        .get(range.start as usize..range.end as usize)
        .map_or_else(|| scalar.value(), str::to_owned)
}

pub(crate) fn scalar_string_from_source(source: &str, scalar: &Scalar) -> String {
    let authored = scalar_raw_from_source(source, scalar);
    if authored == scalar.value() {
        scalar.as_string()
    } else {
        // The compatibility overlay only rewrites complete, single-line block plain scalars.
        authored
    }
}

fn collect_value_scalars(source_id: SourceId, source: &str, node: YamlNode, values: &mut Vec<ValueScalar>) {
    match node {
        YamlNode::Scalar(scalar) => collect_scalar(source_id, source, &scalar, values),
        YamlNode::Mapping(mapping) => {
            for value in mapping.entries().filter_map(|entry| entry.value_node()) {
                collect_value_scalars(source_id, source, value, values);
            }
        }
        YamlNode::Sequence(sequence) => {
            for value in sequence.values() {
                collect_value_scalars(source_id, source, value, values);
            }
        }
        YamlNode::TaggedNode(tagged) => {
            if let Some(node) = tagged
                .as_node()
                .and_then(|node| node.children().find_map(YamlNode::from_syntax))
            {
                collect_value_scalars(source_id, source, node, values);
            }
        }
        YamlNode::Alias(_) => {}
    }
}

fn collect_editable_value_scalars(
    source_id: SourceId,
    source: &str,
    node: YamlNode,
    values: &mut Vec<EditableValueScalar>,
) {
    match node {
        YamlNode::Scalar(scalar) => {
            values.push(EditableValueScalar {
                raw: scalar_raw_from_source(source, &scalar),
                span: position_span(source_id, scalar.byte_range()),
            });
        }
        YamlNode::Mapping(mapping) => {
            for value in mapping.entries().filter_map(|entry| entry.value_node()) {
                collect_editable_value_scalars(source_id, source, value, values);
            }
        }
        YamlNode::Sequence(sequence) => {
            for value in sequence.values() {
                collect_editable_value_scalars(source_id, source, value, values);
            }
        }
        YamlNode::TaggedNode(tagged) => {
            if let Some(node) = tagged
                .as_node()
                .and_then(|node| node.children().find_map(YamlNode::from_syntax))
            {
                collect_editable_value_scalars(source_id, source, node, values);
            }
        }
        YamlNode::Alias(_) => {}
    }
}

fn extract_merge_value(
    source_id: SourceId,
    source: &str,
    node: YamlNode,
    registry: &AnchorRegistry,
    aliases: &mut Vec<String>,
) -> MergeSyntaxValue {
    match node {
        YamlNode::Scalar(scalar) => MergeSyntaxValue::Scalar(extract_merge_scalar(source_id, source, &scalar)),
        YamlNode::Mapping(mapping) => extract_merge_mapping(source_id, source, &mapping, registry, aliases),
        YamlNode::Sequence(sequence) => {
            let span = position_span(source_id, sequence.byte_range());
            let values = sequence
                .values()
                .map(|value| extract_merge_value(source_id, source, value, registry, aliases))
                .collect();
            MergeSyntaxValue::Sequence { values, span }
        }
        YamlNode::Alias(alias) => {
            let name = alias.name();
            let span = yaml_node_span(source_id, &YamlNode::Alias(alias.clone()));
            if aliases.contains(&name) || aliases.len() >= 64 {
                return MergeSyntaxValue::Alias { name, span };
            }
            if let Some(target) = registry.resolve(&name).and_then(|node| {
                YamlNode::from_syntax(node.clone()).or_else(|| node.children().find_map(YamlNode::from_syntax))
            }) {
                aliases.push(name);
                let value = extract_merge_value(source_id, source, target, registry, aliases);
                let _ = aliases.pop();
                value
            } else {
                MergeSyntaxValue::Alias { name, span }
            }
        }
        YamlNode::TaggedNode(tagged) => {
            let span = yaml_node_span(source_id, &YamlNode::TaggedNode(tagged.clone()));
            let value = tagged
                .as_node()
                .and_then(|node| node.children().find_map(YamlNode::from_syntax))
                .map_or_else(
                    || MergeSyntaxValue::Empty(span),
                    |value| extract_merge_value(source_id, source, value, registry, aliases),
                );
            MergeSyntaxValue::Tagged {
                tag: tagged.tag().unwrap_or_default(),
                value: Box::new(value),
                span,
            }
        }
    }
}

fn extract_merge_mapping(
    source_id: SourceId,
    source: &str,
    mapping: &Mapping,
    registry: &AnchorRegistry,
    aliases: &mut Vec<String>,
) -> MergeSyntaxValue {
    let span = position_span(source_id, mapping.byte_range());
    let direct = flatten_merge_fields(source_id, source, raw_merge_fields(source_id, source, mapping));
    let mut entries = Vec::new();
    let mut direct_keys = Vec::new();

    for field in direct {
        if field.key.value == "<<" {
            continue;
        }
        direct_keys.push(field.key.value.clone());
        let value = field.value.map_or_else(
            || {
                MergeSyntaxValue::Empty(SourceSpan::from_valid_offsets(
                    source_id,
                    field.key.span.end(),
                    field.key.span.end(),
                ))
            },
            |value| extract_merge_value(source_id, source, resolve_alias(value, registry), registry, aliases),
        );
        entries.push(MergeSyntaxEntry { key: field.key, value });
    }

    for (key, value) in mapping.merged(registry).iter() {
        let Some(key) = key
            .as_scalar()
            .map(|scalar| extract_merge_scalar(source_id, source, scalar))
        else {
            continue;
        };
        if direct_keys.contains(&key.value) {
            continue;
        }
        entries.push(MergeSyntaxEntry {
            key,
            value: extract_merge_value(source_id, source, value, registry, aliases),
        });
    }

    MergeSyntaxValue::Mapping { entries, span }
}

#[derive(Debug)]
struct RawMergeField {
    key: MergeSyntaxScalar,
    value: Option<YamlNode>,
}

fn raw_merge_fields(source_id: SourceId, source: &str, mapping: &Mapping) -> Vec<RawMergeField> {
    mapping
        .entries()
        .filter_map(|entry| {
            let key = entry.key_node()?.as_scalar().cloned()?;
            Some(RawMergeField {
                key: extract_merge_scalar(source_id, source, &key),
                value: entry.value_node(),
            })
        })
        .collect()
}

fn flatten_merge_fields(source_id: SourceId, source: &str, fields: Vec<RawMergeField>) -> Vec<RawMergeField> {
    let Some(target_column) = fields
        .first()
        .map(|field| source_column(source, field.key.span.start()))
    else {
        return fields;
    };
    recover_merge_fields(source_id, source, fields, target_column)
}

fn recover_merge_fields(
    source_id: SourceId,
    source: &str,
    fields: Vec<RawMergeField>,
    target_column: usize,
) -> Vec<RawMergeField> {
    let mut flattened = Vec::new();
    for mut field in fields {
        let field_column = source_column(source, field.key.span.start());
        let nested_mapping = field.value.as_ref().and_then(YamlNode::as_mapping).cloned();
        let continuation = nested_mapping.as_ref().is_some_and(|mapping| {
            !is_flow_mapping(source, mapping)
                && mapping
                    .entries()
                    .find_map(|entry| entry.key_node()?.as_scalar().map(Scalar::byte_range))
                    .is_some_and(|position| source_column(source, position.start as usize) <= field_column)
        });
        if continuation {
            field.value = None;
        }
        if field_column == target_column {
            flattened.push(field);
        }
        if let Some(mapping) = nested_mapping.filter(|mapping| !is_flow_mapping(source, mapping)) {
            let nested = raw_merge_fields(source_id, source, &mapping);
            flattened.extend(recover_merge_fields(source_id, source, nested, target_column));
        }
    }
    flattened
}

fn is_flow_mapping(source: &str, mapping: &Mapping) -> bool {
    let position = mapping.byte_range();
    source
        .get(position.start as usize..position.end as usize)
        .is_some_and(|text| text.trim_start().starts_with('{'))
}

fn source_column(source: &str, offset: usize) -> usize {
    let prefix = &source[..offset.min(source.len())];
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    source[line_start..offset.min(source.len())].chars().count()
}

fn resolve_alias(node: YamlNode, registry: &AnchorRegistry) -> YamlNode {
    let YamlNode::Alias(alias) = &node else {
        return node;
    };
    registry
        .resolve(&alias.name())
        .and_then(|target| {
            YamlNode::from_syntax(target.clone()).or_else(|| target.children().find_map(YamlNode::from_syntax))
        })
        .unwrap_or(node)
}

fn extract_merge_scalar(source_id: SourceId, source: &str, scalar: &Scalar) -> MergeSyntaxScalar {
    let kind = match ScalarValue::from_scalar(scalar).scalar_type() {
        ScalarType::Boolean => MergeScalarKind::Boolean,
        ScalarType::Integer | ScalarType::Float => MergeScalarKind::Number,
        ScalarType::Null => MergeScalarKind::Null,
        ScalarType::String | ScalarType::Timestamp | ScalarType::Regex => MergeScalarKind::String,
    };
    MergeSyntaxScalar {
        raw: scalar_raw_from_source(source, scalar),
        value: scalar_string_from_source(source, scalar),
        kind,
        span: position_span(source_id, scalar.byte_range()),
    }
}

fn yaml_node_span(source_id: SourceId, node: &YamlNode) -> SourceSpan {
    let Some(syntax) = node.as_node() else {
        return SourceSpan::from_valid_offsets(source_id, 0, 0);
    };
    let range = syntax.text_range();
    SourceSpan::from_valid_offsets(
        source_id,
        u32::from(range.start()) as usize,
        u32::from(range.end()) as usize,
    )
}

fn position_span(source_id: SourceId, position: yaml_edit::TextPosition) -> SourceSpan {
    SourceSpan::from_valid_offsets(source_id, position.start as usize, position.end as usize)
}

fn collect_scalar(source_id: SourceId, source: &str, scalar: &Scalar, values: &mut Vec<ValueScalar>) {
    let raw = scalar_raw_from_source(source, scalar);
    let eligible_style = !raw.starts_with('\'') && !raw.starts_with('|') && !raw.starts_with('>');
    if !eligible_style || !raw.contains('$') {
        return;
    }
    let position = scalar.byte_range();
    values.push(ValueScalar {
        value: scalar_string_from_source(source, scalar),
        span: SourceSpan::from_valid_offsets(source_id, position.start as usize, position.end as usize),
    });
}

#[cfg(test)]
mod tests {
    use super::{SyntaxDocument, unparsed_input_offset};
    use crate::source::SourceId;
    use yaml_edit::YamlFile;

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

    #[test]
    fn complete_root_guard_detects_the_private_backends_raw_comma_omission() {
        let source = "services:\n  app:\n    volumes:\n      - ./data:/data:Z,ro\n  later:\n    image: later\n";
        let backend = YamlFile::parse(source);

        assert!(backend.positioned_errors().is_empty());
        assert!(unparsed_input_offset(&backend, source).is_some());
    }
}
