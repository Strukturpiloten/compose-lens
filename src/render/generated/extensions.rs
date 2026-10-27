//! Bounded inert extension construction; no Compose or application interpretation.
use super::{GeneratedString, GenerationError, valid_yaml_number, write_indent, write_quoted};
use crate::{
    source::SourceId,
    syntax::{MergeSyntaxValue, SyntaxDocument},
};
use std::collections::BTreeSet;

/// Maximum collection nesting in one generated extension value (a scalar has depth zero).
pub const GENERATED_EXTENSION_MAX_DEPTH: usize = 64;
/// Maximum values and mapping keys in one generated extension tree.
pub const GENERATED_EXTENSION_MAX_NODES: usize = 65_536;

/// One ordered string-keyed entry inside an inert extension mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedExtensionEntry {
    key: GeneratedString,
    value: GeneratedExtensionValue,
}
impl GeneratedExtensionEntry {
    /// Creates an entry. Empty and multiline string keys remain explicit YAML strings.
    #[must_use]
    pub const fn new(key: GeneratedString, value: GeneratedExtensionValue) -> Self {
        Self { key, value }
    }
}

/// One validated `x-*` field at document or direct service scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedExtension {
    key: GeneratedString,
    value: GeneratedExtensionValue,
}
impl GeneratedExtension {
    /// Creates an extension, retaining exact key casing.
    ///
    /// # Errors
    /// Rejects names without the exact lowercase `x-` prefix, an empty suffix, or line breaks.
    pub fn new(key: GeneratedString, value: GeneratedExtensionValue) -> Result<Self, GenerationError> {
        if !key.expose().starts_with("x-") || key.expose().len() == 2 || key.expose().contains(['\n', '\r']) {
            return Err(GenerationError::InvalidExtensionName);
        }
        Ok(Self { key, value })
    }
    pub(super) fn is_sensitive(&self) -> bool {
        self.key.is_sensitive() || self.value.is_sensitive()
    }
}

/// A bounded recursive YAML value for inert native extensions.
///
/// Mappings and sequences retain insertion order. Numbers keep their validated spelling; no
/// floating-point conversion, interpolation, environment access, or provider interpretation occurs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedExtensionValue {
    kind: ExtensionKind,
    depth: usize,
    nodes: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
enum ExtensionKind {
    Null,
    Boolean(bool),
    Number(GeneratedString),
    String(GeneratedString),
    Folded(GeneratedString),
    Sequence(Vec<GeneratedExtensionValue>),
    Mapping(Vec<GeneratedExtensionEntry>),
}
impl GeneratedExtensionValue {
    const fn scalar(kind: ExtensionKind) -> Self {
        Self {
            kind,
            depth: 0,
            nodes: 1,
        }
    }
    /// Creates an explicit YAML null.
    #[must_use]
    pub const fn null() -> Self {
        Self::scalar(ExtensionKind::Null)
    }
    /// Creates a YAML boolean.
    #[must_use]
    pub const fn boolean(value: bool) -> Self {
        Self::scalar(ExtensionKind::Boolean(value))
    }
    /// Creates a YAML number without changing its spelling or precision.
    ///
    /// # Errors
    /// Rejects anything other than one complete native YAML number scalar.
    pub fn number(value: GeneratedString) -> Result<Self, GenerationError> {
        if !valid_yaml_number(value.expose()) {
            return Err(GenerationError::InvalidExtensionNumber);
        }
        Ok(Self::scalar(ExtensionKind::Number(value)))
    }
    /// Creates a string with native minimal-safe quoting.
    #[must_use]
    pub const fn string(value: GeneratedString) -> Self {
        Self::scalar(ExtensionKind::String(value))
    }
    /// Creates a folded (`>`) string with native indentation and chomping selection.
    ///
    /// # Errors
    /// Rejects carriage returns and controls other than LF/tab, or any value whose exact semantic
    /// parse-back cannot be represented by this folded presentation.
    pub fn folded_string(value: GeneratedString) -> Result<Self, GenerationError> {
        if value
            .expose()
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\t'))
        {
            return Err(GenerationError::InvalidFoldedExtensionString);
        }
        let mut text = String::from("value: ");
        render_folded(&mut text, value.expose(), 0);
        let parsed =
            SyntaxDocument::parse(SourceId::new(0), text).map_err(|_| GenerationError::InvalidFoldedExtensionString)?;
        let exact = parsed.is_valid()
            && matches!(parsed.document().merge_root(),
            Some(MergeSyntaxValue::Mapping { entries, .. }) if entries.len() == 1
            && matches!(&entries[0].value, MergeSyntaxValue::Scalar(scalar) if scalar.value == value.expose()));
        if !exact {
            return Err(GenerationError::InvalidFoldedExtensionString);
        }
        Ok(Self::scalar(ExtensionKind::Folded(value)))
    }
    /// Creates an ordered sequence, including an explicit empty sequence.
    ///
    /// # Errors
    /// Rejects values exceeding [`GENERATED_EXTENSION_MAX_DEPTH`] or [`GENERATED_EXTENSION_MAX_NODES`].
    pub fn sequence(values: Vec<Self>) -> Result<Self, GenerationError> {
        let (depth, nodes) = bounds(values.iter(), 0)?;
        Ok(Self {
            kind: ExtensionKind::Sequence(values),
            depth,
            nodes,
        })
    }
    /// Creates an ordered string-keyed mapping, including an explicit empty mapping.
    ///
    /// # Errors
    /// Rejects duplicate exact keys and trees exceeding the generated extension bounds. Errors
    /// never disclose keys, including keys whose other occurrence was marked sensitive.
    pub fn mapping(entries: Vec<GeneratedExtensionEntry>) -> Result<Self, GenerationError> {
        let mut seen = BTreeSet::new();
        for entry in &entries {
            if !seen.insert(entry.key.expose()) {
                return Err(GenerationError::DuplicateExtensionKey);
            }
        }
        let (depth, nodes) = bounds(entries.iter().map(|entry| &entry.value), entries.len())?;
        Ok(Self {
            kind: ExtensionKind::Mapping(entries),
            depth,
            nodes,
        })
    }
    /// Reports whether this tree contains any caller-protected key or scalar.
    #[must_use]
    pub fn is_sensitive(&self) -> bool {
        match &self.kind {
            ExtensionKind::Null | ExtensionKind::Boolean(_) => false,
            ExtensionKind::Number(value) | ExtensionKind::String(value) | ExtensionKind::Folded(value) => {
                value.is_sensitive()
            }
            ExtensionKind::Sequence(values) => values.iter().any(Self::is_sensitive),
            ExtensionKind::Mapping(entries) => entries
                .iter()
                .any(|entry| entry.key.is_sensitive() || entry.value.is_sensitive()),
        }
    }
}
fn bounds<'a>(
    values: impl Iterator<Item = &'a GeneratedExtensionValue>,
    keys: usize,
) -> Result<(usize, usize), GenerationError> {
    let mut depth = 1;
    let mut nodes = keys.checked_add(1).ok_or(GenerationError::ExtensionLimit)?;
    for value in values {
        depth = depth.max(value.depth + 1);
        nodes = nodes.checked_add(value.nodes).ok_or(GenerationError::ExtensionLimit)?;
        if depth > GENERATED_EXTENSION_MAX_DEPTH || nodes > GENERATED_EXTENSION_MAX_NODES {
            return Err(GenerationError::ExtensionLimit);
        }
    }
    if nodes > GENERATED_EXTENSION_MAX_NODES {
        return Err(GenerationError::ExtensionLimit);
    }
    Ok((depth, nodes))
}
pub(super) fn add_extension(
    extensions: &mut Vec<GeneratedExtension>,
    extension: GeneratedExtension,
) -> Result<(), GenerationError> {
    if extensions
        .iter()
        .any(|existing| existing.key.expose() == extension.key.expose())
    {
        return Err(GenerationError::DuplicateExtensionKey);
    }
    extensions.push(extension);
    Ok(())
}
pub(super) fn render_extensions(output: &mut String, extensions: &[GeneratedExtension], depth: usize) {
    for extension in extensions {
        write_indent(output, depth);
        write_extension_key(output, extension.key.expose());
        output.push_str(": ");
        render_value(output, &extension.value, depth);
    }
}
fn render_value(output: &mut String, value: &GeneratedExtensionValue, depth: usize) {
    match &value.kind {
        ExtensionKind::Null => output.push_str("null\n"),
        ExtensionKind::Boolean(value) => output.push_str(if *value { "true\n" } else { "false\n" }),
        ExtensionKind::Number(value) => {
            output.push_str(value.expose());
            output.push('\n');
        }
        ExtensionKind::String(value) => {
            write_quoted(output, value.expose());
            output.push('\n');
        }
        ExtensionKind::Folded(value) => render_folded(output, value.expose(), depth),
        ExtensionKind::Sequence(values) if values.is_empty() => output.push_str("[]\n"),
        ExtensionKind::Mapping(entries) if entries.is_empty() => output.push_str("{}\n"),
        ExtensionKind::Sequence(values) => {
            let _ = output.pop();
            output.push('\n');
            for value in values {
                write_indent(output, depth + 1);
                output.push_str("- ");
                render_value(output, value, depth + 1);
            }
        }
        ExtensionKind::Mapping(entries) => {
            let _ = output.pop();
            output.push('\n');
            for entry in entries {
                write_indent(output, depth + 1);
                write_extension_key(output, entry.key.expose());
                output.push_str(": ");
                render_value(output, &entry.value, depth + 1);
            }
        }
    }
}
fn render_folded(output: &mut String, value: &str, depth: usize) {
    let trailing = value.len() - value.trim_end_matches('\n').len();
    let core = value.trim_end_matches('\n');
    output.push_str(if trailing == 0 {
        ">2-\n"
    } else if trailing == 1 && !core.is_empty() {
        ">2\n"
    } else {
        ">2+\n"
    });
    if !core.is_empty() {
        let lines: Vec<_> = core.split('\n').collect();
        for (index, line) in lines.iter().enumerate() {
            write_indent(output, depth + 1);
            output.push_str(line);
            output.push('\n');
            let ordinary = |line: &str| !line.is_empty() && !line.starts_with([' ', '\t']);
            if ordinary(line)
                && lines[index + 1..]
                    .iter()
                    .find(|next| !next.is_empty())
                    .is_some_and(|next| ordinary(next))
            {
                output.push('\n');
            }
        }
    }
    for _ in 0..trailing.saturating_sub(usize::from(!core.is_empty())) {
        output.push('\n');
    }
}

fn write_extension_key(output: &mut String, key: &str) {
    if key == "<<" {
        output.push_str("\"<<\"");
    } else {
        write_quoted(output, key);
    }
}
