use super::{selection_matches, service_entries, service_in_scope};
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::loader::{
    IncludeCompositionResult, IncludeDefinitionEvidence, IncludeIdentity, IncludeProjectDirectoryPlan,
};
use crate::merge::{MergedProject, MergedScalar, MergedValue};
use crate::model::{Located, MountType, ShortVolumeMount, VolumeMount};
use crate::profiles::ProfileSelection;
use crate::source::SourceSpan;
use std::fmt;
use std::path::{Path, PathBuf};

/// A home-relative path cannot be expanded without explicit caller context.
pub const HOME_DIRECTORY_REQUIRED: DiagnosticCode = DiagnosticCode::new("compose.paths.home-directory-required");
/// A selected included resource has no authorized occurrence base directory.
pub const INCLUDE_RESOURCE_PATH_BASE_UNAVAILABLE: DiagnosticCode =
    DiagnosticCode::new("compose.include.resource-path-base-unavailable");
/// Included composition evidence and the supplied directory plan do not describe the same occurrence.
pub const INCLUDE_RESOURCE_PATH_PLAN_MISMATCH: DiagnosticCode =
    DiagnosticCode::new("compose.include.resource-path-plan-mismatch");

/// The lexical category of one authored host path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostPathKind {
    /// A path interpreted relative to the project base directory.
    Relative,
    /// A Unix-style absolute path.
    UnixAbsolute,
    /// A Windows drive-letter absolute path.
    WindowsDriveAbsolute,
    /// A Windows UNC path.
    WindowsUnc,
    /// `~` or a `~/`-prefixed path requiring an explicit home directory.
    HomeRelative,
}

/// Why a host path participates in the Compose project.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathPurpose {
    /// A service bind-mount source.
    ServiceBind {
        /// Service name.
        service: String,
        /// Zero-based merged mount index.
        index: usize,
    },
    /// A top-level config `file` source.
    ConfigFile {
        /// Config model name.
        config: String,
    },
    /// A top-level secret `file` source.
    SecretFile {
        /// Secret model name.
        secret: String,
    },
}

/// Explicit context for host-path interpretation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PathContext {
    home_directory: Option<PathBuf>,
}

impl PathContext {
    /// Creates context without a home-directory assumption.
    #[must_use]
    pub const fn new() -> Self {
        Self { home_directory: None }
    }

    /// Supplies a caller-owned home directory for `~` expansion.
    #[must_use]
    pub fn with_home_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.home_directory = Some(directory.into());
        self
    }

    /// Returns the explicit home directory, if supplied.
    #[must_use]
    pub fn home_directory(&self) -> Option<&Path> {
        self.home_directory.as_deref()
    }
}

/// One classified host path and its explicit resolution origin.
#[derive(Clone, PartialEq, Eq)]
pub struct ResolvedHostPath {
    raw: String,
    kind: HostPathKind,
    purpose: PathPurpose,
    source: SourceSpan,
    origin: PathBuf,
    resolved: Option<PathBuf>,
    sensitive: bool,
}

impl ResolvedHostPath {
    /// Returns the interpolated but otherwise unmodified path value.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns the lexical path category.
    #[must_use]
    pub const fn kind(&self) -> HostPathKind {
        self.kind
    }

    /// Returns why the path is used.
    #[must_use]
    pub const fn purpose(&self) -> &PathPurpose {
        &self.purpose
    }

    /// Returns the authored value span.
    #[must_use]
    pub const fn source(&self) -> SourceSpan {
        self.source
    }

    /// Returns the first-file project directory used as the path origin.
    #[must_use]
    pub fn origin(&self) -> &Path {
        &self.origin
    }

    /// Returns the path with its explicit relative or home origin applied.
    ///
    /// This is lexical resolution only. It does not canonicalize, follow symlinks, or access the
    /// file system. Windows absolute paths remain representable on non-Windows hosts.
    #[must_use]
    pub fn resolved(&self) -> Option<&Path> {
        self.resolved.as_deref()
    }

    /// Reports whether interpolation inserted sensitive content.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

impl fmt::Debug for ResolvedHostPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let raw = if self.sensitive { "<redacted>" } else { &self.raw };
        let resolved = if self.sensitive { None } else { self.resolved.as_deref() };
        formatter
            .debug_struct("ResolvedHostPath")
            .field("raw", &raw)
            .field("kind", &self.kind)
            .field("purpose", &self.purpose)
            .field("source", &self.source)
            .field("origin", &self.origin)
            .field("resolved", &resolved)
            .field("sensitive", &self.sensitive)
            .finish()
    }
}

/// A recoverable, non-destructive host-path resolution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathResolution {
    paths: Vec<ResolvedHostPath>,
    diagnostics: Vec<Diagnostic>,
}

impl PathResolution {
    /// Returns paths in deterministic project traversal order.
    #[must_use]
    pub fn paths(&self) -> &[ResolvedHostPath] {
        &self.paths
    }

    /// Returns path diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether path processing emitted no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }
}

/// One selected included service bind, config, or secret file path and its occurrence-specific
/// lexical result.
#[derive(Clone, PartialEq, Eq)]
pub struct IncludedResourcePath {
    raw: String,
    kind: HostPathKind,
    purpose: PathPurpose,
    source: SourceSpan,
    occurrence_index: usize,
    identity: IncludeIdentity,
    base_directory: Option<PathBuf>,
    resolved: Option<PathBuf>,
}

impl IncludedResourcePath {
    /// Returns the authored, uninterpolated path value.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns the lexical path category.
    #[must_use]
    pub const fn kind(&self) -> HostPathKind {
        self.kind
    }

    /// Returns the selected resource and namespace that supplied this path.
    #[must_use]
    pub const fn purpose(&self) -> &PathPurpose {
        &self.purpose
    }

    /// Returns the source span that anchors this path finding.
    ///
    /// Long-syntax bind sources and top-level config/secret `file` values retain the exact
    /// authored value-scalar span. A short-syntax service bind is decoded from one colon-delimited
    /// mount scalar, so its source component has no independently retained byte range; this getter
    /// deliberately returns the containing authored mount-scalar span instead of guessing through
    /// YAML quoting or escape spelling.
    #[must_use]
    pub const fn source(&self) -> SourceSpan {
        self.source
    }

    /// Returns the retained include occurrence index.
    #[must_use]
    pub const fn occurrence_index(&self) -> usize {
        self.occurrence_index
    }

    /// Returns the caller-defined identity of the retained occurrence.
    #[must_use]
    pub const fn identity(&self) -> &IncludeIdentity {
        &self.identity
    }

    /// Returns the authorized occurrence base, when planning supplied one.
    #[must_use]
    pub fn base_directory(&self) -> Option<&Path> {
        self.base_directory.as_deref()
    }

    /// Returns the lexical result, when its base and any required home directory were available.
    #[must_use]
    pub fn resolved(&self) -> Option<&Path> {
        self.resolved.as_deref()
    }
}

impl fmt::Debug for IncludedResourcePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IncludedResourcePath")
            .field("raw", &"<redacted-path>")
            .field("kind", &self.kind)
            .field("purpose", &self.purpose)
            .field("source", &self.source)
            .field("occurrence_index", &self.occurrence_index)
            .field("identity", &"<redacted-identity>")
            .field(
                "base_directory",
                &self.base_directory.as_ref().map(|_| "<authorized-directory>"),
            )
            .field("resolved", &self.resolved.as_ref().map(|_| "<resolved-path>"))
            .finish()
    }
}

/// Recoverable lexical resolution for selected included service binds, config, and secret file
/// paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludedResourcePathResolution {
    paths: Vec<IncludedResourcePath>,
    diagnostics: Vec<Diagnostic>,
    upstream_complete: bool,
}

impl IncludedResourcePathResolution {
    /// Returns selected paths in service-then-config-then-secret composition order.
    #[must_use]
    pub fn paths(&self) -> &[IncludedResourcePath] {
        &self.paths
    }

    /// Returns composition, directory-plan, and lexical path diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether no error diagnostic was retained or emitted.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }

    /// Reports whether every selected file path resolved and no error occurred.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.upstream_complete && self.is_valid() && self.paths.iter().all(|path| path.resolved.is_some())
    }
}

/// Lexically resolves selected included service bind sources and config and secret `file` paths.
///
/// This consumes only typed, authored composition values and caller-authorized occurrence bases.
/// It considers short-form bind sources only when their spelling is already path-like and
/// long-form sources only when `type: bind` is selected. It does not interpolate, canonicalize,
/// access the file system, or resolve any other path family.
#[must_use]
pub fn resolve_included_resource_paths(
    composition: &IncludeCompositionResult,
    directory_plan: &IncludeProjectDirectoryPlan,
    context: &PathContext,
) -> IncludedResourcePathResolution {
    let common_prefix = composition
        .diagnostics()
        .iter()
        .zip(directory_plan.diagnostics())
        .take_while(|(left, right)| left == right)
        .count();
    let mut diagnostics = composition.diagnostics().to_vec();
    diagnostics.extend_from_slice(&directory_plan.diagnostics()[common_prefix..]);
    let mut paths = Vec::new();

    if let Some(root) = composition.root() {
        for definition in root.services() {
            let Some(volumes) = definition.definition().volumes() else {
                continue;
            };
            for (index, mount) in volumes.value().iter().enumerate() {
                let Some((raw, source)) = included_bind_source(mount.value()) else {
                    continue;
                };
                push_included_resource_path(
                    &mut paths,
                    &mut diagnostics,
                    directory_plan,
                    context,
                    definition.evidence(),
                    raw,
                    source,
                    PathPurpose::ServiceBind {
                        service: definition.name().to_owned(),
                        index,
                    },
                );
            }
        }
        for definition in root.configs() {
            if let Some(file) = definition.definition().file() {
                push_included_resource_path(
                    &mut paths,
                    &mut diagnostics,
                    directory_plan,
                    context,
                    definition.evidence(),
                    file.value(),
                    file.span(),
                    PathPurpose::ConfigFile {
                        config: definition.name().to_owned(),
                    },
                );
            }
        }
        for definition in root.secrets() {
            if let Some(file) = definition.definition().file() {
                push_included_resource_path(
                    &mut paths,
                    &mut diagnostics,
                    directory_plan,
                    context,
                    definition.evidence(),
                    file.value(),
                    file.span(),
                    PathPurpose::SecretFile {
                        secret: definition.name().to_owned(),
                    },
                );
            }
        }
    }

    IncludedResourcePathResolution {
        paths,
        diagnostics,
        upstream_complete: composition.is_complete() && directory_plan.is_complete(),
    }
}

fn included_bind_source(mount: &VolumeMount) -> Option<(&str, SourceSpan)> {
    match mount {
        VolumeMount::Short(mount) => {
            let source = mount.source()?;
            is_path_source(source).then_some((source, mount.raw().span()))
        }
        VolumeMount::Long(mount) if mount.mount_type().is_some_and(|kind| *kind.value() == MountType::Bind) => {
            mount.source().map(|source| (source.value().as_str(), source.span()))
        }
        VolumeMount::Long(_) => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn push_included_resource_path(
    paths: &mut Vec<IncludedResourcePath>,
    diagnostics: &mut Vec<Diagnostic>,
    directory_plan: &IncludeProjectDirectoryPlan,
    context: &PathContext,
    evidence: &IncludeDefinitionEvidence,
    raw: &str,
    source: SourceSpan,
    purpose: PathPurpose,
) {
    let occurrence_index = evidence.occurrence_index();
    let plan_entry = directory_plan.entry(occurrence_index);
    let aligned_entry =
        plan_entry.filter(|entry| entry.node_index() == occurrence_index && entry.identity() == evidence.identity());
    let base_directory = match (plan_entry, aligned_entry) {
        (Some(_), None) | (None, _) => {
            diagnostics.push(
                Diagnostic::new(
                    INCLUDE_RESOURCE_PATH_PLAN_MISMATCH,
                    Severity::Error,
                    "included resource path and directory plan describe different occurrences",
                )
                .with_label(DiagnosticLabel::primary(source, "directory plan occurrence mismatch")),
            );
            None
        }
        (_, Some(entry)) => {
            if let Some(directory) = entry.effective_directory() {
                Some(directory.to_path_buf())
            } else {
                diagnostics.push(
                    Diagnostic::new(
                        INCLUDE_RESOURCE_PATH_BASE_UNAVAILABLE,
                        Severity::Error,
                        "included resource path has no authorized project directory",
                    )
                    .with_label(DiagnosticLabel::primary(source, "project directory unavailable")),
                );
                None
            }
        }
    };
    let kind = classify(raw);
    let resolved = base_directory
        .as_deref()
        .and_then(|base| resolve_lexically(base, context, raw, kind));
    if base_directory.is_some() && kind == HostPathKind::HomeRelative && resolved.is_none() {
        diagnostics.push(
            Diagnostic::new(
                HOME_DIRECTORY_REQUIRED,
                Severity::Warning,
                "home-relative path requires an explicit home directory",
            )
            .with_label(DiagnosticLabel::primary(source, "home directory not supplied")),
        );
    }
    paths.push(IncludedResourcePath {
        raw: raw.to_owned(),
        kind,
        purpose,
        source,
        occurrence_index,
        identity: evidence.identity().clone(),
        base_directory,
        resolved,
    });
}

/// Finds and lexically resolves paths covered by the initial conversion boundary.
#[must_use]
pub fn resolve_paths(
    project: &MergedProject,
    selection: Option<&ProfileSelection>,
    context: &PathContext,
) -> PathResolution {
    let mut diagnostics = Vec::new();
    if !selection_matches(project, selection, &mut diagnostics) {
        return PathResolution {
            paths: Vec::new(),
            diagnostics,
        };
    }

    let mut paths = Vec::new();
    for service in service_entries(project) {
        if !service_in_scope(selection, service.key()) {
            continue;
        }
        let Some(volumes) = service.value().get("volumes").and_then(MergedValue::as_sequence) else {
            continue;
        };
        for (index, volume) in volumes.iter().enumerate() {
            let source = bind_source(volume);
            if let Some((source, span, sensitive)) = source {
                push_path(
                    &mut paths,
                    &mut diagnostics,
                    project.base_directory(),
                    context,
                    &source,
                    span,
                    sensitive,
                    PathPurpose::ServiceBind {
                        service: service.key().to_owned(),
                        index,
                    },
                );
            }
        }
    }

    collect_resource_files(project, "configs", true, context, &mut paths, &mut diagnostics);
    collect_resource_files(project, "secrets", false, context, &mut paths, &mut diagnostics);
    PathResolution { paths, diagnostics }
}

fn bind_source(volume: &MergedValue) -> Option<(String, SourceSpan, bool)> {
    if let Some(scalar) = volume.as_scalar() {
        let span = super::effective_span(volume);
        let mount = ShortVolumeMount::new(Located::new(scalar.value().to_owned(), span));
        let source = mount.source()?;
        return is_path_source(source).then_some((source.to_owned(), span, scalar.is_sensitive()));
    }
    if volume
        .get("type")
        .and_then(MergedValue::as_scalar)
        .map(MergedScalar::value)
        != Some("bind")
    {
        return None;
    }
    let source = volume.get("source")?;
    let scalar = source.as_scalar()?;
    Some((
        scalar.value().to_owned(),
        super::effective_span(source),
        scalar.is_sensitive(),
    ))
}

fn collect_resource_files(
    project: &MergedProject,
    field: &str,
    config: bool,
    context: &PathContext,
    paths: &mut Vec<ResolvedHostPath>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(resources) = project.root().get(field).and_then(MergedValue::as_mapping) else {
        return;
    };
    for resource in resources {
        let Some(file) = resource.value().get("file") else {
            continue;
        };
        let Some(scalar) = file.as_scalar() else {
            continue;
        };
        let purpose = if config {
            PathPurpose::ConfigFile {
                config: resource.key().to_owned(),
            }
        } else {
            PathPurpose::SecretFile {
                secret: resource.key().to_owned(),
            }
        };
        push_path(
            paths,
            diagnostics,
            project.base_directory(),
            context,
            scalar.value(),
            super::effective_span(file),
            scalar.is_sensitive(),
            purpose,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_path(
    paths: &mut Vec<ResolvedHostPath>,
    diagnostics: &mut Vec<Diagnostic>,
    base: &Path,
    context: &PathContext,
    raw: &str,
    source: SourceSpan,
    sensitive: bool,
    purpose: PathPurpose,
) {
    let kind = classify(raw);
    let resolved = resolve_lexically(base, context, raw, kind);
    if kind == HostPathKind::HomeRelative && resolved.is_none() {
        diagnostics.push(
            Diagnostic::new(
                HOME_DIRECTORY_REQUIRED,
                Severity::Warning,
                "home-relative path requires an explicit home directory",
            )
            .with_label(DiagnosticLabel::primary(source, "home directory not supplied")),
        );
    }
    paths.push(ResolvedHostPath {
        raw: raw.to_owned(),
        kind,
        purpose,
        source,
        origin: base.to_path_buf(),
        resolved,
        sensitive,
    });
}

fn resolve_lexically(base: &Path, context: &PathContext, raw: &str, kind: HostPathKind) -> Option<PathBuf> {
    match kind {
        HostPathKind::Relative => Some(base.join(raw)),
        HostPathKind::UnixAbsolute | HostPathKind::WindowsDriveAbsolute | HostPathKind::WindowsUnc => {
            Some(PathBuf::from(raw))
        }
        HostPathKind::HomeRelative => context.home_directory.as_ref().map(|home| {
            raw.strip_prefix("~/")
                .map_or_else(|| home.clone(), |suffix| home.join(suffix))
        }),
    }
}

fn classify(value: &str) -> HostPathKind {
    if value == "~" || value.starts_with("~/") {
        HostPathKind::HomeRelative
    } else if is_windows_drive_absolute(value) {
        HostPathKind::WindowsDriveAbsolute
    } else if value.starts_with("\\\\") {
        HostPathKind::WindowsUnc
    } else if value.starts_with('/') {
        HostPathKind::UnixAbsolute
    } else {
        HostPathKind::Relative
    }
}

pub(crate) fn is_path_source(value: &str) -> bool {
    value == "."
        || value == ".."
        || value.starts_with("./")
        || value.starts_with("../")
        || value.starts_with('/')
        || value == "~"
        || value.starts_with("~/")
        || value.starts_with("\\\\")
        || is_windows_drive_absolute(value)
}

fn is_windows_drive_absolute(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && matches!(bytes[2], b'\\' | b'/')
}
