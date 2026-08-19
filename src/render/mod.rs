//! Deterministic rendering and preservation-oriented editing of ComposeLens-owned documents.

mod generated;
mod preserved;

pub use generated::{
    ComposeDocumentBuilder, GeneratedAnnotation, GeneratedCommand, GeneratedComposeDocument,
    GeneratedConfigFileDefinition, GeneratedCpuRtRuntime, GeneratedDevice, GeneratedDns, GeneratedDnsSearch,
    GeneratedEntrypoint, GeneratedEnvironment, GeneratedEnvironmentFile, GeneratedEnvironmentFileFormat,
    GeneratedExtraHost, GeneratedHostname, GeneratedLabel, GeneratedLogging, GeneratedLoggingOption,
    GeneratedLoggingOptionValue, GeneratedLongDevice, GeneratedMemLimit, GeneratedMount, GeneratedNetworkAttachment,
    GeneratedNetworkDefinition, GeneratedNetworkDriverOption, GeneratedNetworkDriverOptionValue, GeneratedPidsLimit,
    GeneratedPort, GeneratedProtocol, GeneratedPullPolicy, GeneratedResource, GeneratedRestartPolicy,
    GeneratedSecretFileDefinition, GeneratedSelinux, GeneratedService, GeneratedServiceRuntimeField, GeneratedShmSize,
    GeneratedString, GeneratedSysctl, GeneratedSysctls, GeneratedTmpfs, GeneratedUlimit, GeneratedUlimitValue,
    GeneratedUlimits, GeneratedVolumeDefinition, GeneratedVolumeDriverOption, GeneratedVolumeDriverOptionValue,
    GenerationError,
};

pub use preserved::{
    EDIT_INVALID_NUMBER, EDIT_OVERLAP, EDIT_SOURCE_MISMATCH, EDIT_TARGET_NOT_SCALAR, EDIT_UNSUPPORTED_SCALAR_STYLE,
    PreservationEditResult, ReplacementScalar, ScalarEdit, apply_preservation_edits,
};

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::merge::{MergedEntry, MergedProject, MergedScalar, MergedScalarKind, MergedValue, MergedValueKind};
use crate::profiles::ProfileSelection;
use crate::resolution::{effective_span, selection_matches};
use std::fmt;
use yaml_edit::{ScalarStyle, ScalarType, ScalarValue, YamlFile};

/// An unresolved alias cannot be represented in a standalone canonical document.
pub const UNRENDERABLE_ALIAS: DiagnosticCode = DiagnosticCode::new("compose.render.unresolved-alias");

/// A retained YAML tag is not safe to emit as a canonical tag token.
pub const UNRENDERABLE_TAG: DiagnosticCode = DiagnosticCode::new("compose.render.invalid-tag");

/// A valid number of spaces used for one canonical YAML indentation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IndentWidth(u8);

impl IndentWidth {
    /// The smallest supported indentation width.
    pub const MIN: u8 = 1;

    /// The fixed width used by canonical-v2 output.
    pub const CANONICAL: Self = Self(2);

    /// Creates a validated indentation width.
    #[must_use]
    pub const fn new(spaces: u8) -> Option<Self> {
        if spaces >= Self::MIN { Some(Self(spaces)) } else { None }
    }

    /// Returns the number of spaces in one indentation level.
    #[must_use]
    pub const fn spaces(self) -> u8 {
        self.0
    }
}

/// A presentation-only line-ending choice for rendered YAML.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineEnding {
    /// A single line-feed byte.
    #[default]
    Lf,
    /// A carriage return followed by a line feed.
    CrLf,
}

/// Presentation-only options for deterministic merged-project rendering.
///
/// These options cannot interpolate, merge, select profiles, apply defaults, reorder mappings,
/// or change retained Compose short/long forms. [`Self::default`] is the fixed canonical-v2 format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CanonicalFormatting {
    indent_width: IndentWidth,
    line_ending: LineEnding,
    document_marker: bool,
    final_newline: bool,
}

impl CanonicalFormatting {
    /// Returns the number of spaces used for each nested YAML level.
    #[must_use]
    pub const fn indent_width(self) -> IndentWidth {
        self.indent_width
    }

    /// Returns the selected line-ending convention.
    #[must_use]
    pub const fn line_ending(self) -> LineEnding {
        self.line_ending
    }

    /// Reports whether output starts with a YAML document marker.
    #[must_use]
    pub const fn document_marker(self) -> bool {
        self.document_marker
    }

    /// Reports whether non-empty output ends with the selected line ending.
    #[must_use]
    pub const fn final_newline(self) -> bool {
        self.final_newline
    }

    /// Returns options with a different validated indentation width.
    #[must_use]
    pub const fn with_indent_width(mut self, indent_width: IndentWidth) -> Self {
        self.indent_width = indent_width;
        self
    }

    /// Returns options with a different line-ending convention.
    #[must_use]
    pub const fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Returns options with YAML document-marker emission enabled or disabled.
    #[must_use]
    pub const fn with_document_marker(mut self, document_marker: bool) -> Self {
        self.document_marker = document_marker;
        self
    }

    /// Returns options with a final line ending enabled or disabled.
    #[must_use]
    pub const fn with_final_newline(mut self, final_newline: bool) -> Self {
        self.final_newline = final_newline;
        self
    }
}

impl Default for CanonicalFormatting {
    fn default() -> Self {
        Self {
            indent_width: IndentWidth::CANONICAL,
            line_ending: LineEnding::Lf,
            document_marker: true,
            final_newline: true,
        }
    }
}

/// The result of deterministic canonical rendering.
///
/// The output can contain interpolated secrets and is therefore available only through explicit
/// accessors. Its `Debug` representation redacts the complete output when any rendered value is
/// sensitive.
#[derive(Clone, PartialEq, Eq)]
pub struct CanonicalRender {
    output: String,
    diagnostics: Vec<Diagnostic>,
    sensitive: bool,
}

impl CanonicalRender {
    /// Returns the rendered UTF-8 YAML document.
    #[must_use]
    pub fn output(&self) -> &str {
        &self.output
    }

    /// Consumes the result and returns the rendered document.
    #[must_use]
    pub fn into_output(self) -> String {
        self.output
    }

    /// Returns canonical-rendering diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether the renderer emitted no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }

    /// Reports whether any rendered value contains sensitive interpolation output.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

impl fmt::Debug for CanonicalRender {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CanonicalRender")
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

/// Renders a merged project as deterministic `ComposeLens` canonical YAML.
///
/// The renderer preserves merged mapping/sequence order and retained short/long forms. It does not
/// interpolate, resolve paths, apply defaults, normalize syntax variants, or invoke a Compose
/// implementation. When a matching selection is supplied, profile-inactive services are omitted
/// while top-level resources remain available.
#[must_use]
pub fn render_canonical(project: &MergedProject, selection: Option<&ProfileSelection>) -> CanonicalRender {
    render_canonical_with_formatting(project, selection, &CanonicalFormatting::default())
}

/// Renders a merged project with explicit presentation-only formatting choices.
///
/// Semantic processing remains identical to [`render_canonical`]. The default formatting value
/// produces byte-identical canonical-v2 output.
#[must_use]
pub fn render_canonical_with_formatting(
    project: &MergedProject,
    selection: Option<&ProfileSelection>,
    formatting: &CanonicalFormatting,
) -> CanonicalRender {
    let mut diagnostics = Vec::new();
    if !selection_matches(project, selection, &mut diagnostics) {
        return CanonicalRender {
            output: String::new(),
            diagnostics,
            sensitive: false,
        };
    }

    let mut renderer = Renderer {
        output: String::new(),
        diagnostics,
        sensitive: false,
        formatting: *formatting,
    };
    renderer.write_project(project, selection);
    renderer.finish_formatting();
    CanonicalRender {
        output: renderer.output,
        diagnostics: renderer.diagnostics,
        sensitive: renderer.sensitive,
    }
}

struct Renderer {
    output: String,
    diagnostics: Vec<Diagnostic>,
    sensitive: bool,
    formatting: CanonicalFormatting,
}

impl Renderer {
    fn write_project(&mut self, project: &MergedProject, selection: Option<&ProfileSelection>) {
        if self.formatting.document_marker {
            self.output.push_str("---\n");
        }
        let Some(entries) = project.root().as_mapping() else {
            self.write_inline(project.root());
            self.output.push('\n');
            return;
        };
        if entries.is_empty() {
            self.output.push_str("{}\n");
            return;
        }
        for entry in entries {
            if entry.key() == "services" && selection.is_some() {
                self.write_selected_services(entry, selection);
            } else {
                self.write_entry(entry, 0);
            }
        }
    }

    fn write_selected_services(&mut self, entry: &MergedEntry, selection: Option<&ProfileSelection>) {
        let Some(services) = entry.value().as_mapping() else {
            self.write_entry(entry, 0);
            return;
        };
        self.write_indent(0);
        write_quoted(&mut self.output, entry.key());
        self.output.push(':');
        let active: Vec<_> = services
            .iter()
            .filter(|service| selection.is_none_or(|selection| selection.is_active(service.key())))
            .collect();
        if active.is_empty() {
            self.output.push_str(" {}\n");
            return;
        }
        self.output.push('\n');
        for service in active {
            self.write_entry(service, self.indent_width());
        }
    }

    fn write_entry(&mut self, entry: &MergedEntry, indent: usize) {
        self.write_indent(indent);
        write_quoted(&mut self.output, entry.key());
        self.output.push(':');
        self.write_after_indicator(entry.value(), indent + self.indent_width());
    }

    fn write_sequence_item(&mut self, value: &MergedValue, indent: usize) {
        self.write_indent(indent);
        self.output.push('-');
        self.sensitive |= value.is_sensitive();
        let core = self.write_tag_prefixes(value);
        if let MergedValueKind::Mapping(entries) = core.kind() {
            if let Some((first, remaining)) = entries.split_first() {
                self.output.push(' ');
                write_quoted(&mut self.output, first.key());
                self.output.push(':');
                self.write_after_indicator(first.value(), indent + (self.indent_width() * 2));
                for entry in remaining {
                    self.write_entry(entry, indent + self.indent_width());
                }
                return;
            }
        }
        self.write_after_indicator(value, indent + self.indent_width());
    }

    fn write_after_indicator(&mut self, value: &MergedValue, nested_indent: usize) {
        self.sensitive |= value.is_sensitive();
        let core = self.write_tag_prefixes(value);
        if non_empty_collection(core) {
            self.output.push('\n');
            self.write_block(core, nested_indent);
        } else {
            self.output.push(' ');
            self.write_inline(core);
            self.output.push('\n');
        }
    }

    fn write_tag_prefixes<'a>(&mut self, mut value: &'a MergedValue) -> &'a MergedValue {
        while let MergedValueKind::Tagged { tag, value: inner } = value.kind() {
            if valid_tag(tag) {
                self.output.push(' ');
                self.output.push_str(tag);
            } else {
                self.diagnostics.push(
                    Diagnostic::new(
                        UNRENDERABLE_TAG,
                        Severity::Error,
                        "retained YAML tag cannot be emitted canonically",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(value),
                        "invalid canonical tag token",
                    )),
                );
            }
            value = inner;
        }
        value
    }

    fn write_block(&mut self, value: &MergedValue, indent: usize) {
        match value.kind() {
            MergedValueKind::Mapping(entries) => {
                for entry in entries {
                    self.write_entry(entry, indent);
                }
            }
            MergedValueKind::Sequence(values) => {
                for value in values {
                    self.write_sequence_item(value, indent);
                }
            }
            MergedValueKind::Tagged { .. } => {
                self.write_indent(indent);
                self.write_after_indicator(value, indent + self.indent_width());
            }
            MergedValueKind::Null(_) | MergedValueKind::Scalar(_) | MergedValueKind::Alias(_) => {
                self.write_indent(indent);
                self.write_inline(value);
                self.output.push('\n');
            }
        }
    }

    fn write_inline(&mut self, value: &MergedValue) {
        match value.kind() {
            MergedValueKind::Null(_) => self.output.push_str("null"),
            MergedValueKind::Scalar(scalar) => self.write_scalar(scalar),
            MergedValueKind::Mapping(entries) if entries.is_empty() => self.output.push_str("{}"),
            MergedValueKind::Sequence(values) if values.is_empty() => self.output.push_str("[]"),
            MergedValueKind::Alias(_) => {
                self.diagnostics.push(
                    Diagnostic::new(
                        UNRENDERABLE_ALIAS,
                        Severity::Error,
                        "unresolved YAML alias cannot be emitted in a standalone canonical document",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(value),
                        "alias has no resolved canonical value",
                    )),
                );
                self.output.push_str("null");
            }
            MergedValueKind::Tagged { .. } => {
                let core = self.write_tag_prefixes(value);
                self.output.push(' ');
                self.write_inline(core);
            }
            MergedValueKind::Mapping(_) | MergedValueKind::Sequence(_) => {}
        }
    }

    fn write_scalar(&mut self, scalar: &MergedScalar) {
        self.sensitive |= scalar.is_sensitive();
        match scalar.kind() {
            MergedScalarKind::Boolean if scalar.value().eq_ignore_ascii_case("true") => {
                self.output.push_str("true");
            }
            MergedScalarKind::Boolean if scalar.value().eq_ignore_ascii_case("false") => {
                self.output.push_str("false");
            }
            MergedScalarKind::String | MergedScalarKind::Boolean => {
                write_quoted(&mut self.output, scalar.value());
            }
            MergedScalarKind::Number => self.output.push_str(scalar.value()),
        }
    }

    fn write_indent(&mut self, indent: usize) {
        self.output.extend(std::iter::repeat_n(' ', indent));
    }

    fn indent_width(&self) -> usize {
        usize::from(self.formatting.indent_width.spaces())
    }

    fn finish_formatting(&mut self) {
        if !self.formatting.final_newline && self.output.ends_with('\n') {
            let _ = self.output.pop();
        }
        if self.formatting.line_ending == LineEnding::CrLf {
            self.output = self.output.replace('\n', "\r\n");
        }
    }
}

fn non_empty_collection(value: &MergedValue) -> bool {
    match value.kind() {
        MergedValueKind::Mapping(entries) => !entries.is_empty(),
        MergedValueKind::Sequence(values) => !values.is_empty(),
        _ => false,
    }
}

fn valid_tag(tag: &str) -> bool {
    if let Some(verbatim) = tag.strip_prefix("!<").and_then(|tag| tag.strip_suffix('>')) {
        return !verbatim.is_empty()
            && verbatim
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && !matches!(byte, b'<' | b'>'));
    }
    tag.starts_with('!')
        && tag.len() > 1
        && tag
            .bytes()
            .skip(1)
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'!' | b'_' | b'-' | b'.' | b':' | b'/'))
}

/// Writes a YAML string with the least quoting needed to keep it a string.
///
/// A plain candidate must round-trip through the private YAML parser as one complete plain string.
/// YAML 1.1's boolean and null spellings are additionally quoted even though the parser uses newer
/// scalar rules: generated Compose must remain a string when read by older YAML consumers.
fn write_quoted(output: &mut String, value: &str) {
    if plain_string_safe(value) {
        output.push_str(value);
        return;
    }

    write_double_quoted(output, value);
}

fn plain_string_safe(value: &str) -> bool {
    if value.is_empty() || value.trim() != value || value.contains(['\n', '\r']) || yaml_1_1_ambiguous_string(value) {
        return false;
    }

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
    let range = scalar.byte_range();
    range.start == 0
        && range.end as usize == value.len()
        && ScalarValue::from_scalar(&scalar).style() == ScalarStyle::Plain
        && ScalarValue::from_scalar(&scalar).scalar_type() == ScalarType::String
}

fn yaml_1_1_ambiguous_string(value: &str) -> bool {
    matches!(
        value,
        "y" | "Y"
            | "yes"
            | "Yes"
            | "YES"
            | "n"
            | "N"
            | "no"
            | "No"
            | "NO"
            | "true"
            | "True"
            | "TRUE"
            | "false"
            | "False"
            | "FALSE"
            | "on"
            | "On"
            | "ON"
            | "off"
            | "Off"
            | "OFF"
            | "null"
            | "Null"
            | "NULL"
            | "~"
    ) || yaml_1_1_sexagesimal(value)
        || yaml_1_1_special_float(value)
        || yaml_1_1_timestamp(value)
}

fn yaml_1_1_sexagesimal(value: &str) -> bool {
    let mut components = value.split(':');
    let Some(first) = components.next() else {
        return false;
    };
    if components.clone().next().is_none() || first.is_empty() || !first.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    components.all(|component| !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit()))
}

fn yaml_1_1_special_float(value: &str) -> bool {
    matches!(value.to_ascii_lowercase().as_str(), ".inf" | "+.inf" | "-.inf" | ".nan")
}

fn yaml_1_1_timestamp(value: &str) -> bool {
    let date_end = value.find(['T', 't', ' ']).unwrap_or(value.len());
    let date = &value[..date_end];
    let mut components = date.split('-');
    let (Some(year), Some(month), Some(day), None) = (
        components.next(),
        components.next(),
        components.next(),
        components.next(),
    ) else {
        return false;
    };

    year.len() == 4
        && (1..=2).contains(&month.len())
        && (1..=2).contains(&day.len())
        && year.bytes().all(|byte| byte.is_ascii_digit())
        && month.bytes().all(|byte| byte.is_ascii_digit())
        && day.bytes().all(|byte| byte.is_ascii_digit())
}

fn write_double_quoted(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\t' => output.push_str("\\t"),
            '\n' => output.push_str("\\n"),
            '\u{0c}' => output.push_str("\\f"),
            '\r' => output.push_str("\\r"),
            character
                if character.is_control() || matches!(character, '\u{85}' | '\u{2028}' | '\u{2029}' | '\u{feff}') =>
            {
                push_unicode_escape(output, character);
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

fn push_unicode_escape(output: &mut String, character: char) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let value = character as u32;
    if value <= 0xffff {
        output.push_str("\\u");
        for shift in [12, 8, 4, 0] {
            output.push(HEX[((value >> shift) & 0xf) as usize] as char);
        }
    } else {
        output.push_str("\\U");
        for shift in [28, 24, 20, 16, 12, 8, 4, 0] {
            output.push(HEX[((value >> shift) & 0xf) as usize] as char);
        }
    }
}
