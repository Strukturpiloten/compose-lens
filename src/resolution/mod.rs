//! Explicit project paths, references, and default decisions.

mod defaults;
mod paths;
mod references;

pub use defaults::{
    AppliedDefault, ComposeDefaults, ContainerPlatform, DefaultKind, DefaultLocation, DefaultProvider, DefaultRequest,
    DefaultResolution, DefaultValue, NoDefaults, resolve_defaults,
};
pub use paths::{
    HOME_DIRECTORY_REQUIRED, HostPathKind, PathContext, PathPurpose, PathResolution, ResolvedHostPath, resolve_paths,
};
pub use references::{
    INACTIVE_SERVICE_REFERENCE, MISSING_REFERENCE, Reference, ReferenceKind, ReferenceStatus, ReferenceValidation,
    validate_references,
};

use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity};
use crate::merge::{MergedEntry, MergedProject, MergedValue};
use crate::profiles::ProfileSelection;
use crate::source::{SourceId, SourceSpan};

/// A profile selection was created from another merged project.
pub const SELECTION_PROJECT_MISMATCH: DiagnosticCode =
    DiagnosticCode::new("compose.resolution.selection-project-mismatch");

pub(crate) fn selection_matches(
    project: &MergedProject,
    selection: Option<&ProfileSelection>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if selection.is_none_or(|selection| selection.belongs_to(project)) {
        return true;
    }
    diagnostics.push(Diagnostic::new(
        SELECTION_PROJECT_MISMATCH,
        Severity::Error,
        "profile selection does not belong to the merged project",
    ));
    false
}

pub(crate) fn service_entries(project: &MergedProject) -> &[MergedEntry] {
    project
        .root()
        .get("services")
        .and_then(MergedValue::as_mapping)
        .unwrap_or_default()
}

pub(crate) fn service_in_scope(selection: Option<&ProfileSelection>, name: &str) -> bool {
    selection.is_none_or(|selection| selection.is_active(name))
}

pub(crate) fn effective_span(value: &MergedValue) -> SourceSpan {
    value
        .provenance()
        .effective_source()
        .or_else(|| value.provenance().sources().first().copied())
        .unwrap_or_else(|| SourceSpan::from_valid_offsets(SourceId::new(0), 0, 0))
}

pub(crate) fn entry_span(entry: &MergedEntry) -> SourceSpan {
    entry
        .key_sources()
        .last()
        .copied()
        .or_else(|| entry.key_sources().first().copied())
        .unwrap_or_else(|| SourceSpan::from_valid_offsets(SourceId::new(0), 0, 0))
}
