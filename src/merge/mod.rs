//! Provenance-preserving Compose multi-file merge behavior.

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::interpolation::DocumentInterpolation;
use crate::loader::{LoadedProject, ProjectInterpolation};
use crate::model::{Located, ShortPort, ShortVolumeMount};
use crate::source::{SourceId, SourceSpan};
use crate::syntax::{MergeScalarKind, MergeSyntaxEntry, MergeSyntaxScalar, MergeSyntaxValue};
use std::fmt;
use std::path::{Path, PathBuf};

/// A loaded document is missing its matching per-file interpolation overlay.
pub const INTERPOLATION_PROJECT_MISMATCH: DiagnosticCode =
    DiagnosticCode::new("compose.merge.interpolation-project-mismatch");

/// A document root cannot participate in a Compose project merge.
pub const INVALID_DOCUMENT_ROOT: DiagnosticCode = DiagnosticCode::new("compose.merge.invalid-document-root");

/// A YAML alias could not be resolved within its source document.
pub const UNRESOLVED_ALIAS: DiagnosticCode = DiagnosticCode::new("compose.merge.unresolved-alias");

/// How a merged value was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MergeOperation {
    /// The value comes directly from the first document that defined it.
    Authored,
    /// A later document added a previously absent field or unique item.
    Added,
    /// A later scalar or incompatible value form replaced the earlier value.
    Replaced,
    /// Mappings or field-specific unique entries were combined.
    Merged,
    /// An ordinary sequence from a later document was appended.
    Appended,
    /// Compose's `!reset` tag cleared the earlier value.
    Reset,
    /// Compose's `!override` tag replaced the earlier value without normal merge behavior.
    Override,
}

/// Source evidence and the last operation applied to one merged value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeProvenance {
    operation: MergeOperation,
    sources: Vec<SourceSpan>,
}

impl MergeProvenance {
    /// Returns the operation that produced the current value.
    #[must_use]
    pub const fn operation(&self) -> MergeOperation {
        self.operation
    }

    /// Returns contributing source spans in processing order.
    #[must_use]
    pub fn sources(&self) -> &[SourceSpan] {
        &self.sources
    }

    /// Returns the most recent contributing span.
    #[must_use]
    pub fn effective_source(&self) -> Option<SourceSpan> {
        self.sources.last().copied()
    }
}

/// The semantic scalar category retained by the merge view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MergedScalarKind {
    /// A YAML string, including quoted numeric or boolean spelling.
    String,
    /// A YAML boolean.
    Boolean,
    /// A YAML integer or floating-point value retained as text.
    Number,
}

/// One scalar after optional per-file interpolation.
#[derive(Clone, PartialEq, Eq)]
pub struct MergedScalar {
    raw: String,
    value: String,
    kind: MergedScalarKind,
    sensitive: bool,
}

impl MergedScalar {
    /// Returns the exact authored scalar spelling.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns the semantic value after optional interpolation.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the scalar category.
    #[must_use]
    pub const fn kind(&self) -> MergedScalarKind {
        self.kind
    }

    /// Reports whether interpolation inserted sensitive content.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

impl fmt::Debug for MergedScalar {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MergedScalar")
            .field("raw", &self.raw)
            .field("value", &if self.sensitive { "<redacted>" } else { &self.value })
            .field("kind", &self.kind)
            .field("sensitive", &self.sensitive)
            .finish()
    }
}

/// Whether a null-like value was explicit or represented by an empty YAML mapping value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NullStyle {
    /// A key was authored without a value.
    Empty,
    /// A YAML null scalar such as `null` or `~` was authored.
    Explicit,
}

/// The authored form that contributed one semantic mapping entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntrySyntax {
    /// A normal YAML mapping entry.
    Mapping,
    /// A `KEY=VALUE` scalar from a Compose list form.
    ListKeyValue,
    /// A key-only scalar from a Compose list form.
    ListKeyOnly,
}

/// One mapping entry in a merged semantic view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedEntry {
    key: String,
    key_sources: Vec<SourceSpan>,
    syntax: EntrySyntax,
    value: MergedValue,
}

impl MergedEntry {
    /// Returns the semantic mapping key.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns every authored key location that participated in this entry.
    #[must_use]
    pub fn key_sources(&self) -> &[SourceSpan] {
        &self.key_sources
    }

    /// Returns the most recent authored syntax form for this entry.
    #[must_use]
    pub const fn syntax(&self) -> EntrySyntax {
        self.syntax
    }

    /// Returns the merged entry value.
    #[must_use]
    pub const fn value(&self) -> &MergedValue {
        &self.value
    }
}

/// A ComposeLens-owned semantic value that does not expose the YAML parser dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergedValueKind {
    /// An empty or explicit null value.
    Null(NullStyle),
    /// A scalar with authored and optionally interpolated forms.
    Scalar(MergedScalar),
    /// An ordered semantic mapping.
    Mapping(Vec<MergedEntry>),
    /// An ordered sequence.
    Sequence(Vec<MergedValue>),
    /// An unresolved YAML alias retained for diagnostics and recovery.
    Alias(String),
    /// A non-Compose YAML tag retained without interpretation.
    Tagged {
        /// The authored tag name.
        tag: String,
        /// The tagged value.
        value: Box<MergedValue>,
    },
}

/// One value in the merged semantic project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedValue {
    kind: MergedValueKind,
    provenance: MergeProvenance,
}

impl MergedValue {
    /// Returns the semantic value kind.
    #[must_use]
    pub const fn kind(&self) -> &MergedValueKind {
        &self.kind
    }

    /// Returns merge provenance.
    #[must_use]
    pub const fn provenance(&self) -> &MergeProvenance {
        &self.provenance
    }

    /// Returns a scalar value when this node is a scalar.
    #[must_use]
    pub const fn as_scalar(&self) -> Option<&MergedScalar> {
        match &self.kind {
            MergedValueKind::Scalar(value) => Some(value),
            _ => None,
        }
    }

    /// Returns mapping entries when this node is a mapping.
    #[must_use]
    pub fn as_mapping(&self) -> Option<&[MergedEntry]> {
        match &self.kind {
            MergedValueKind::Mapping(entries) => Some(entries),
            _ => None,
        }
    }

    /// Returns sequence items when this node is a sequence.
    #[must_use]
    pub fn as_sequence(&self) -> Option<&[MergedValue]> {
        match &self.kind {
            MergedValueKind::Sequence(values) => Some(values),
            _ => None,
        }
    }

    /// Finds a semantic mapping value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&MergedValue> {
        self.as_mapping()?
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| &entry.value)
    }

    /// Reports whether this node or one of its descendants contains interpolated sensitive data.
    #[must_use]
    pub fn is_sensitive(&self) -> bool {
        match &self.kind {
            MergedValueKind::Scalar(value) => value.sensitive,
            MergedValueKind::Mapping(entries) => entries.iter().any(|entry| entry.value.is_sensitive()),
            MergedValueKind::Sequence(values) => values.iter().any(Self::is_sensitive),
            MergedValueKind::Tagged { value, .. } => value.is_sensitive(),
            MergedValueKind::Null(_) | MergedValueKind::Alias(_) => false,
        }
    }
}

/// A merged Compose project and its retained project origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedProject {
    root: MergedValue,
    base_directory: PathBuf,
    source_ids: Vec<SourceId>,
}

impl MergedProject {
    /// Returns the merged root mapping.
    #[must_use]
    pub const fn root(&self) -> &MergedValue {
        &self.root
    }

    /// Returns the project directory inherited from the first loaded document.
    #[must_use]
    pub fn base_directory(&self) -> &Path {
        &self.base_directory
    }

    /// Returns source documents in merge order.
    #[must_use]
    pub fn source_ids(&self) -> &[SourceId] {
        &self.source_ids
    }

    /// Traverses mapping keys from the root.
    #[must_use]
    pub fn value(&self, path: &[&str]) -> Option<&MergedValue> {
        path.iter().try_fold(&self.root, |value, key| value.get(key))
    }
}

/// A recoverable multi-file merge result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeResult {
    project: Option<MergedProject>,
    diagnostics: Vec<Diagnostic>,
}

impl MergeResult {
    /// Returns the merged project when at least one document has a mapping root.
    #[must_use]
    pub const fn project(&self) -> Option<&MergedProject> {
        self.project.as_ref()
    }

    /// Returns upstream and merge diagnostics in processing order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether processing produced a project and no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.project.is_some()
            && self
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }
}

/// Merges loaded documents in order, optionally applying matching per-file interpolation overlays.
///
/// Passing `None` deliberately merges authored values without interpolation. Passing an overlay
/// applies each document's substitutions before its values participate in the merge.
#[must_use]
pub fn merge_project(project: &LoadedProject, interpolation: Option<&ProjectInterpolation>) -> MergeResult {
    let mut diagnostics = project.diagnostics().to_vec();
    if let Some(interpolation) = interpolation {
        diagnostics.extend(interpolation.diagnostics().iter().cloned());
    }

    let mut root = None;
    let mut source_ids = Vec::new();
    for document in project.documents() {
        let overlay = interpolation.and_then(|values| values.document(document.source_id()));
        if interpolation.is_some() && overlay.is_none() {
            diagnostics.push(
                Diagnostic::new(
                    INTERPOLATION_PROJECT_MISMATCH,
                    Severity::Error,
                    "loaded document has no matching interpolation overlay",
                )
                .with_label(DiagnosticLabel::primary(
                    document.syntax().source_span(),
                    "missing per-file overlay",
                )),
            );
        }

        let Some(syntax_root) = document.syntax().merge_root() else {
            continue;
        };
        if !matches!(syntax_root, MergeSyntaxValue::Mapping { .. }) {
            diagnostics.push(
                Diagnostic::new(
                    INVALID_DOCUMENT_ROOT,
                    Severity::Error,
                    "Compose merge inputs must have mapping roots",
                )
                .with_label(DiagnosticLabel::primary(
                    document.syntax().source_span(),
                    "document is not a mapping",
                )),
            );
            continue;
        }

        let value = convert_value(syntax_root, overlay, &mut diagnostics);
        source_ids.push(document.source_id());
        root = Some(match root {
            Some(current) => merge_value(current, value, &[], &mut diagnostics),
            None => activate_tags(value, &[], &mut diagnostics),
        });
    }

    if let Some(interpolation) = interpolation {
        for overlay in interpolation.documents() {
            if project.document(overlay.source_id()).is_none() {
                diagnostics.push(Diagnostic::new(
                    INTERPOLATION_PROJECT_MISMATCH,
                    Severity::Error,
                    "interpolation overlay does not belong to the loaded project",
                ));
            }
        }
    }

    MergeResult {
        project: root.map(|root| MergedProject {
            root,
            base_directory: project.base_directory().to_path_buf(),
            source_ids,
        }),
        diagnostics,
    }
}

fn convert_value(
    value: MergeSyntaxValue,
    interpolation: Option<&DocumentInterpolation>,
    diagnostics: &mut Vec<Diagnostic>,
) -> MergedValue {
    match value {
        MergeSyntaxValue::Empty(span) => authored(MergedValueKind::Null(NullStyle::Empty), span),
        MergeSyntaxValue::Scalar(value) if value.kind == MergeScalarKind::Null => {
            authored(MergedValueKind::Null(NullStyle::Explicit), value.span)
        }
        MergeSyntaxValue::Scalar(value) => convert_scalar(value, interpolation),
        MergeSyntaxValue::Mapping { entries, span } => {
            let entries = entries
                .into_iter()
                .map(|entry| convert_entry(entry, interpolation, diagnostics))
                .collect();
            authored(MergedValueKind::Mapping(entries), span)
        }
        MergeSyntaxValue::Sequence { values, span } => {
            let values = values
                .into_iter()
                .map(|value| convert_value(value, interpolation, diagnostics))
                .collect();
            authored(MergedValueKind::Sequence(values), span)
        }
        MergeSyntaxValue::Alias { name, span } => {
            diagnostics.push(
                Diagnostic::new(UNRESOLVED_ALIAS, Severity::Warning, "YAML alias could not be resolved")
                    .with_label(DiagnosticLabel::primary(span, "unresolved alias")),
            );
            authored(MergedValueKind::Alias(name), span)
        }
        MergeSyntaxValue::Tagged { tag, value, span } => {
            let value = convert_value(*value, interpolation, diagnostics);
            authored(
                MergedValueKind::Tagged {
                    tag,
                    value: Box::new(value),
                },
                span,
            )
        }
    }
}

fn convert_entry(
    entry: MergeSyntaxEntry,
    interpolation: Option<&DocumentInterpolation>,
    diagnostics: &mut Vec<Diagnostic>,
) -> MergedEntry {
    MergedEntry {
        key: entry.key.value,
        key_sources: vec![entry.key.span],
        syntax: EntrySyntax::Mapping,
        value: convert_value(entry.value, interpolation, diagnostics),
    }
}

fn convert_scalar(value: MergeSyntaxScalar, interpolation: Option<&DocumentInterpolation>) -> MergedValue {
    let resolved = interpolation.and_then(|overlay| overlay.value(value.span));
    let semantic = resolved.map_or_else(|| value.value.clone(), |result| result.resolved().to_owned());
    let sensitive = resolved.is_some_and(crate::interpolation::InterpolationResult::is_sensitive);
    let kind = match value.kind {
        MergeScalarKind::String => MergedScalarKind::String,
        MergeScalarKind::Boolean => MergedScalarKind::Boolean,
        MergeScalarKind::Number => MergedScalarKind::Number,
        MergeScalarKind::Null => unreachable!("null scalars are converted before convert_scalar"),
    };
    authored(
        MergedValueKind::Scalar(MergedScalar {
            raw: value.raw,
            value: semantic,
            kind,
            sensitive,
        }),
        value.span,
    )
}

fn authored(kind: MergedValueKind, span: SourceSpan) -> MergedValue {
    MergedValue {
        kind,
        provenance: MergeProvenance {
            operation: MergeOperation::Authored,
            sources: vec![span],
        },
    }
}

fn activate_tags(mut value: MergedValue, path: &[String], diagnostics: &mut Vec<Diagnostic>) -> MergedValue {
    value = match value.kind {
        MergedValueKind::Tagged { tag, value: inner } if tag == "!reset" => reset_value(None, &inner, value.provenance),
        MergedValueKind::Tagged { tag, value: inner } if tag == "!override" => {
            override_value(None, *inner, value.provenance)
        }
        kind => MergedValue {
            kind,
            provenance: value.provenance,
        },
    };

    match &mut value.kind {
        MergedValueKind::Mapping(entries) => {
            for entry in entries {
                let mut child_path = path.to_vec();
                child_path.push(entry.key.clone());
                entry.value = activate_tags(entry.value.clone(), &child_path, diagnostics);
            }
        }
        MergedValueKind::Sequence(values) => {
            for item in values {
                *item = activate_tags(item.clone(), path, diagnostics);
            }
        }
        MergedValueKind::Tagged { value: inner, .. } => {
            **inner = activate_tags((**inner).clone(), path, diagnostics);
        }
        MergedValueKind::Null(_) | MergedValueKind::Scalar(_) | MergedValueKind::Alias(_) => {}
    }
    let _ = diagnostics;
    value
}

fn merge_value(
    base: MergedValue,
    incoming: MergedValue,
    path: &[String],
    diagnostics: &mut Vec<Diagnostic>,
) -> MergedValue {
    if let MergedValueKind::Tagged { tag, value } = incoming.kind {
        if tag == "!reset" {
            return reset_value(Some(&base), &value, incoming.provenance);
        }
        if tag == "!override" {
            return override_value(Some(&base), *value, incoming.provenance);
        }
        return replace_value(
            base,
            MergedValue {
                kind: MergedValueKind::Tagged { tag, value },
                provenance: incoming.provenance,
            },
            MergeOperation::Replaced,
        );
    }

    if is_shell_command(path) {
        return replace_value(base, incoming, MergeOperation::Replaced);
    }

    if is_keyed_mapping(path) {
        if let (Some(base), Some(incoming)) = (normalize_keyed(base.clone()), normalize_keyed(incoming.clone())) {
            return merge_mappings(base, incoming, path, diagnostics);
        }
    }

    match (&base.kind, &incoming.kind) {
        (MergedValueKind::Mapping(_), MergedValueKind::Mapping(_)) => merge_mappings(base, incoming, path, diagnostics),
        (MergedValueKind::Sequence(_), MergedValueKind::Sequence(_)) if unique_field(path).is_some() => {
            merge_unique_sequences(base, incoming, path, diagnostics)
        }
        (MergedValueKind::Sequence(_), MergedValueKind::Sequence(_)) => append_sequences(base, incoming),
        _ => replace_value(
            base,
            activate_tags(incoming, path, diagnostics),
            MergeOperation::Replaced,
        ),
    }
}

fn merge_mappings(
    base: MergedValue,
    incoming: MergedValue,
    path: &[String],
    diagnostics: &mut Vec<Diagnostic>,
) -> MergedValue {
    let MergedValueKind::Mapping(mut base_entries) = base.kind else {
        return base;
    };
    let MergedValueKind::Mapping(incoming_entries) = incoming.kind else {
        return MergedValue {
            kind: MergedValueKind::Mapping(base_entries),
            provenance: base.provenance,
        };
    };

    for incoming_entry in incoming_entries {
        if let Some(index) = base_entries.iter().position(|entry| entry.key == incoming_entry.key) {
            let mut child_path = path.to_vec();
            child_path.push(incoming_entry.key.clone());
            let existing = &mut base_entries[index];
            existing.value = merge_value(existing.value.clone(), incoming_entry.value, &child_path, diagnostics);
            extend_sources(&mut existing.key_sources, &incoming_entry.key_sources);
            existing.syntax = incoming_entry.syntax;
        } else {
            let mut incoming_entry = incoming_entry;
            incoming_entry.value = activate_tags(incoming_entry.value, path, diagnostics);
            mark_added(&mut incoming_entry.value);
            base_entries.push(incoming_entry);
        }
    }

    MergedValue {
        kind: MergedValueKind::Mapping(base_entries),
        provenance: combined_provenance(base.provenance, &incoming.provenance, MergeOperation::Merged),
    }
}

fn append_sequences(base: MergedValue, incoming: MergedValue) -> MergedValue {
    let MergedValueKind::Sequence(mut base_values) = base.kind else {
        return base;
    };
    let MergedValueKind::Sequence(mut incoming_values) = incoming.kind else {
        return MergedValue {
            kind: MergedValueKind::Sequence(base_values),
            provenance: base.provenance,
        };
    };
    for value in &mut incoming_values {
        mark_added(value);
    }
    base_values.append(&mut incoming_values);
    MergedValue {
        kind: MergedValueKind::Sequence(base_values),
        provenance: combined_provenance(base.provenance, &incoming.provenance, MergeOperation::Appended),
    }
}

fn merge_unique_sequences(
    base: MergedValue,
    incoming: MergedValue,
    path: &[String],
    diagnostics: &mut Vec<Diagnostic>,
) -> MergedValue {
    let Some(field) = unique_field(path) else {
        return append_sequences(base, incoming);
    };
    let MergedValueKind::Sequence(mut base_values) = base.kind else {
        return base;
    };
    let MergedValueKind::Sequence(incoming_values) = incoming.kind else {
        return MergedValue {
            kind: MergedValueKind::Sequence(base_values),
            provenance: base.provenance,
        };
    };

    for mut incoming_value in incoming_values {
        let key = unique_key(&incoming_value, field);
        let existing = key.as_ref().and_then(|key| {
            base_values
                .iter()
                .position(|value| unique_key(value, field).as_ref() == Some(key))
        });
        if let Some(index) = existing {
            base_values[index] = merge_value(base_values[index].clone(), incoming_value, path, diagnostics);
        } else {
            incoming_value = activate_tags(incoming_value, path, diagnostics);
            mark_added(&mut incoming_value);
            base_values.push(incoming_value);
        }
    }

    MergedValue {
        kind: MergedValueKind::Sequence(base_values),
        provenance: combined_provenance(base.provenance, &incoming.provenance, MergeOperation::Merged),
    }
}

fn replace_value(base: MergedValue, mut incoming: MergedValue, operation: MergeOperation) -> MergedValue {
    incoming.provenance = combined_provenance(base.provenance, &incoming.provenance, operation);
    incoming
}

fn reset_value(base: Option<&MergedValue>, tagged: &MergedValue, tag: MergeProvenance) -> MergedValue {
    let kind = match &tagged.kind {
        MergedValueKind::Mapping(_) => MergedValueKind::Mapping(Vec::new()),
        MergedValueKind::Sequence(_) => MergedValueKind::Sequence(Vec::new()),
        MergedValueKind::Null(_) => match base.map(|value| &value.kind) {
            Some(MergedValueKind::Mapping(_)) => MergedValueKind::Mapping(Vec::new()),
            Some(MergedValueKind::Sequence(_)) => MergedValueKind::Sequence(Vec::new()),
            _ => MergedValueKind::Null(NullStyle::Empty),
        },
        _ => MergedValueKind::Null(NullStyle::Empty),
    };
    let provenance = match base {
        Some(base) => combined_provenance(base.provenance.clone(), &tag, MergeOperation::Reset),
        None => MergeProvenance {
            operation: MergeOperation::Reset,
            sources: tag.sources,
        },
    };
    MergedValue { kind, provenance }
}

fn override_value(base: Option<&MergedValue>, mut tagged: MergedValue, tag: MergeProvenance) -> MergedValue {
    tagged.provenance = match base {
        Some(base) => {
            let prior = combined_provenance(base.provenance.clone(), &tag, MergeOperation::Override);
            combined_provenance(prior, &tagged.provenance, MergeOperation::Override)
        }
        None => combined_provenance(tag, &tagged.provenance, MergeOperation::Override),
    };
    tagged
}

fn combined_provenance(
    mut base: MergeProvenance,
    incoming: &MergeProvenance,
    operation: MergeOperation,
) -> MergeProvenance {
    extend_sources(&mut base.sources, &incoming.sources);
    base.operation = operation;
    base
}

fn extend_sources(target: &mut Vec<SourceSpan>, incoming: &[SourceSpan]) {
    for span in incoming {
        if !target.contains(span) {
            target.push(*span);
        }
    }
}

fn mark_added(value: &mut MergedValue) {
    if value.provenance.operation == MergeOperation::Authored {
        value.provenance.operation = MergeOperation::Added;
    }
}

fn is_shell_command(path: &[String]) -> bool {
    matches!(path, [services, _, field] if services == "services" && (field == "command" || field == "entrypoint"))
        || matches!(path, [services, _, healthcheck, test] if services == "services" && healthcheck == "healthcheck" && test == "test")
}

fn is_keyed_mapping(path: &[String]) -> bool {
    matches!(path, [services, _, field] if services == "services" && (field == "environment" || field == "labels"))
}

fn normalize_keyed(value: MergedValue) -> Option<MergedValue> {
    match value.kind {
        MergedValueKind::Mapping(_) => Some(value),
        MergedValueKind::Sequence(values) => {
            let mut entries = Vec::with_capacity(values.len());
            for value in values {
                let scalar = value.as_scalar()?;
                let (key, entry_value, syntax) = if let Some((key, entry_value)) = scalar.value.split_once('=') {
                    let scalar_value = MergedValue {
                        kind: MergedValueKind::Scalar(MergedScalar {
                            raw: entry_value.to_owned(),
                            value: entry_value.to_owned(),
                            kind: MergedScalarKind::String,
                            sensitive: scalar.sensitive,
                        }),
                        provenance: value.provenance.clone(),
                    };
                    (key.to_owned(), scalar_value, EntrySyntax::ListKeyValue)
                } else {
                    let null = MergedValue {
                        kind: MergedValueKind::Null(NullStyle::Empty),
                        provenance: value.provenance.clone(),
                    };
                    (scalar.value.clone(), null, EntrySyntax::ListKeyOnly)
                };
                let key_source = value.provenance.effective_source()?;
                entries.push(MergedEntry {
                    key,
                    key_sources: vec![key_source],
                    syntax,
                    value: entry_value,
                });
            }
            Some(MergedValue {
                kind: MergedValueKind::Mapping(entries),
                provenance: value.provenance,
            })
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UniqueField {
    Volume,
    Device,
    Config,
    Secret,
    Port,
}

fn unique_field(path: &[String]) -> Option<UniqueField> {
    let [services, _, field] = path else {
        return None;
    };
    if services != "services" {
        return None;
    }
    match field.as_str() {
        "volumes" => Some(UniqueField::Volume),
        "devices" => Some(UniqueField::Device),
        "configs" => Some(UniqueField::Config),
        "secrets" => Some(UniqueField::Secret),
        "ports" => Some(UniqueField::Port),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum UniqueKey {
    Target(String),
    Port {
        ip: String,
        target: String,
        published: String,
        protocol: String,
    },
}

fn unique_key(value: &MergedValue, field: UniqueField) -> Option<UniqueKey> {
    match field {
        UniqueField::Volume | UniqueField::Device => target_key(value, true).map(UniqueKey::Target),
        UniqueField::Config | UniqueField::Secret => target_key(value, false).map(UniqueKey::Target),
        UniqueField::Port => port_key(value),
    }
}

fn target_key(value: &MergedValue, colon_syntax: bool) -> Option<String> {
    if let Some(scalar) = value.as_scalar() {
        if colon_syntax {
            let span = value.provenance.effective_source()?;
            return ShortVolumeMount::new(Located::new(scalar.value.clone(), span))
                .target()
                .map(str::to_owned);
        }
        return Some(scalar.value.clone());
    }
    mapping_scalar(value, "target")
        .or_else(|| (!colon_syntax).then(|| mapping_scalar(value, "source")).flatten())
        .map(str::to_owned)
}

fn port_key(value: &MergedValue) -> Option<UniqueKey> {
    if let Some(scalar) = value.as_scalar() {
        let span = value.provenance.effective_source()?;
        let port = ShortPort::parse(Located::new(scalar.value.clone(), span));
        return Some(UniqueKey::Port {
            ip: port.host_ip().unwrap_or_default().to_owned(),
            target: port.target().to_owned(),
            published: port.published().unwrap_or_default().to_owned(),
            protocol: port.protocol().unwrap_or("tcp").to_owned(),
        });
    }
    Some(UniqueKey::Port {
        ip: mapping_scalar(value, "host_ip").unwrap_or_default().to_owned(),
        target: mapping_scalar(value, "target")?.to_owned(),
        published: mapping_scalar(value, "published").unwrap_or_default().to_owned(),
        protocol: mapping_scalar(value, "protocol").unwrap_or("tcp").to_owned(),
    })
}

fn mapping_scalar<'a>(value: &'a MergedValue, key: &str) -> Option<&'a str> {
    value.get(key)?.as_scalar().map(MergedScalar::value)
}
