//! Source-aware native values from a merged and optionally profile-selected Compose project.

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::merge::{
    EntrySyntax, MergeProvenance, MergedEntry, MergedProject, MergedScalarKind, MergedValue, MergedValueKind,
};
use crate::model::{
    BindOptions, BooleanValue, Command, ComposeScalar, ConfigDefinition, DependencyCondition, HealthcheckDuration,
    HealthcheckRetries, HealthcheckTest, HealthcheckTestKind, HostAddress, ImageReference, Ipam, IpamConfig,
    KeyValueEntry, Labels, Located, LongPort, LongVolumeMount, MountType, NetworkDefinition, Port, SecretDefinition,
    SelinuxRelabel, ServiceNetwork, ServiceNetworks, ShortExtraHost, ShortPort, ShortVolumeMount, VolumeDefinition,
    VolumeMount,
};
use crate::profiles::ProfileSelection;
use crate::resolution::{SELECTION_PROJECT_MISMATCH, service_in_scope};
use crate::source::{SourceId, SourceSpan};
use std::fmt;
use std::path::{Path, PathBuf};

/// A value in the merged project has an unexpected mapping, sequence, scalar, or null form.
pub const PROJECT_EXPECTED_FORM: DiagnosticCode = DiagnosticCode::new("compose.project.expected-form");

/// A required field is absent from a merged native value.
pub const PROJECT_MISSING_FIELD: DiagnosticCode = DiagnosticCode::new("compose.project.missing-field");

/// A scalar cannot be represented by the requested native value type.
pub const PROJECT_INVALID_VALUE: DiagnosticCode = DiagnosticCode::new("compose.project.invalid-value");

/// A typed value together with every source span that contributed to it during merging.
#[derive(Clone, PartialEq, Eq)]
pub struct ProjectValue<T> {
    value: T,
    provenance: MergeProvenance,
    sensitive: bool,
}

impl<T: fmt::Debug> fmt::Debug for ProjectValue<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("ProjectValue");
        if self.sensitive {
            debug.field("value", &"<redacted>");
        } else {
            debug.field("value", &self.value);
        }
        debug
            .field("provenance", &self.provenance)
            .field("sensitive", &self.sensitive)
            .finish()
    }
}

impl<T> ProjectValue<T> {
    fn new(value: T, source: &MergedValue) -> Self {
        Self {
            value,
            provenance: source.provenance().clone(),
            sensitive: source.is_sensitive(),
        }
    }

    /// Returns the typed effective value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Returns the merge operation and contributing spans in processing order.
    #[must_use]
    pub const fn provenance(&self) -> &MergeProvenance {
        &self.provenance
    }

    /// Returns the most recent source contributing to this value.
    #[must_use]
    pub fn effective_source(&self) -> Option<SourceSpan> {
        self.provenance.effective_source()
    }

    /// Reports whether this value contains sensitive interpolation output.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }

    /// Removes the provenance wrapper and returns the typed value.
    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }
}

/// A merged mapping key and every location at which that key was authored.
#[derive(Clone, PartialEq, Eq)]
pub struct ProjectKey {
    value: String,
    sources: Vec<SourceSpan>,
    sensitive: bool,
}

impl fmt::Debug for ProjectKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProjectKey")
            .field("value", &if self.sensitive { "<redacted>" } else { &self.value })
            .field("sources", &self.sources)
            .field("sensitive", &self.sensitive)
            .finish()
    }
}

impl ProjectKey {
    fn from_entry(entry: &MergedEntry) -> Self {
        Self {
            value: entry.key().to_owned(),
            sources: entry.key_sources().to_vec(),
            sensitive: entry.is_key_sensitive(),
        }
    }

    fn from_value(value: String, source: &MergedValue) -> Self {
        Self {
            value,
            sources: source.provenance().sources().to_vec(),
            sensitive: source.is_sensitive(),
        }
    }

    /// Returns the semantic key text.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns authored key locations in merge order.
    #[must_use]
    pub fn sources(&self) -> &[SourceSpan] {
        &self.sources
    }

    /// Returns the effective key location.
    #[must_use]
    pub fn effective_source(&self) -> Option<SourceSpan> {
        self.sources.last().copied()
    }

    /// Reports whether interpolation inserted sensitive content into this semantic key.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

/// One effective service dependency with source-aware long-form options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectServiceDependency {
    service: ProjectKey,
    condition: Option<ProjectValue<DependencyCondition>>,
    restart: Option<ProjectValue<BooleanValue>>,
    required: Option<ProjectValue<BooleanValue>>,
    unmodeled_fields: Vec<ProjectFieldReference>,
}

impl ProjectServiceDependency {
    /// Returns the referenced service name and all contributing name locations.
    #[must_use]
    pub const fn service(&self) -> &ProjectKey {
        &self.service
    }

    /// Returns the explicitly authored readiness condition.
    #[must_use]
    pub const fn condition(&self) -> Option<&ProjectValue<DependencyCondition>> {
        self.condition.as_ref()
    }

    /// Returns whether Compose-controlled dependency updates restart this service.
    #[must_use]
    pub const fn restart(&self) -> Option<&ProjectValue<BooleanValue>> {
        self.restart.as_ref()
    }

    /// Returns whether the dependency is required.
    #[must_use]
    pub const fn required(&self) -> Option<&ProjectValue<BooleanValue>> {
        self.required.as_ref()
    }

    /// Returns retained long-form fields outside the typed dependency boundary.
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[ProjectFieldReference] {
        &self.unmodeled_fields
    }
}

/// Effective service dependencies with the short or long Compose form retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectDependsOn {
    /// A sequence of service names using Compose defaults.
    Short(Vec<ProjectValue<ProjectServiceDependency>>),
    /// A mapping of service names to dependency options.
    Long(Vec<ProjectValue<ProjectServiceDependency>>),
}

impl ProjectDependsOn {
    /// Returns dependencies in effective merge order.
    #[must_use]
    pub fn services(&self) -> &[ProjectValue<ProjectServiceDependency>] {
        match self {
            Self::Short(services) | Self::Long(services) => services,
        }
    }

    /// Reports whether the effective field uses long mapping syntax.
    #[must_use]
    pub const fn is_long(&self) -> bool {
        matches!(self, Self::Long(_))
    }
}

/// A field retained by the merged tree but outside the first native project-view boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFieldReference {
    path: Vec<String>,
    key: ProjectKey,
    provenance: MergeProvenance,
    extension: bool,
    sensitive: bool,
}

impl ProjectFieldReference {
    /// Returns the semantic path including the field name.
    #[must_use]
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// Returns the retained mapping key and all of its source locations.
    #[must_use]
    pub const fn key(&self) -> &ProjectKey {
        &self.key
    }

    /// Returns the field value's complete merge provenance.
    #[must_use]
    pub const fn provenance(&self) -> &MergeProvenance {
        &self.provenance
    }

    /// Reports whether the field name starts with `x-`.
    #[must_use]
    pub const fn is_extension(&self) -> bool {
        self.extension
    }

    /// Reports whether the retained value contains sensitive interpolation output.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

/// One effective environment variable after field-specific multi-file merging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectEnvironmentEntry {
    name: ProjectKey,
    value: ProjectValue<ComposeScalar>,
    syntax: EntrySyntax,
}

impl ProjectEnvironmentEntry {
    /// Returns the variable name and its contributing key spans.
    #[must_use]
    pub const fn name(&self) -> &ProjectKey {
        &self.name
    }

    /// Returns the effective scalar, including a distinct host-environment null value.
    #[must_use]
    pub const fn value(&self) -> &ProjectValue<ComposeScalar> {
        &self.value
    }

    /// Returns the most recent mapping or list syntax contributing this entry.
    #[must_use]
    pub const fn syntax(&self) -> EntrySyntax {
        self.syntax
    }
}

/// A normalized-by-key environment view that retains each entry's authored syntax form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectEnvironment {
    entries: Vec<ProjectEnvironmentEntry>,
}

/// One effective hostname-to-address mapping after field-specific project merging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectExtraHost {
    hostname: ProjectKey,
    address: ProjectValue<HostAddress>,
    syntax: EntrySyntax,
}

impl ProjectExtraHost {
    /// Returns the hostname and every contributing source location.
    #[must_use]
    pub const fn hostname(&self) -> &ProjectKey {
        &self.hostname
    }

    /// Returns the raw-preserving IP address or implementation token.
    #[must_use]
    pub const fn address(&self) -> &ProjectValue<HostAddress> {
        &self.address
    }

    /// Returns the most recent mapping or list syntax contributing this entry.
    #[must_use]
    pub const fn syntax(&self) -> EntrySyntax {
        self.syntax
    }
}

/// Ordered effective `extra_hosts` entries with field and item provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectExtraHosts {
    entries: Vec<ProjectExtraHost>,
}

impl ProjectExtraHosts {
    /// Returns host mappings in effective merge order.
    #[must_use]
    pub fn entries(&self) -> &[ProjectExtraHost] {
        &self.entries
    }
}

impl ProjectEnvironment {
    /// Returns environment variables in effective merge order.
    #[must_use]
    pub fn entries(&self) -> &[ProjectEnvironmentEntry] {
        &self.entries
    }

    /// Finds an effective environment variable by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ProjectEnvironmentEntry> {
        self.entries.iter().find(|entry| entry.name.value == name)
    }
}

/// One effective service health check with field-level merge provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectHealthcheck {
    test: Option<ProjectValue<HealthcheckTest>>,
    interval: Option<ProjectValue<HealthcheckDuration>>,
    timeout: Option<ProjectValue<HealthcheckDuration>>,
    retries: Option<ProjectValue<HealthcheckRetries>>,
    start_period: Option<ProjectValue<HealthcheckDuration>>,
    start_interval: Option<ProjectValue<HealthcheckDuration>>,
    disable: Option<ProjectValue<BooleanValue>>,
    unmodeled_fields: Vec<ProjectFieldReference>,
}

impl ProjectHealthcheck {
    /// Returns the effective health command without collapsing scalar and list forms.
    #[must_use]
    pub const fn test(&self) -> Option<&ProjectValue<HealthcheckTest>> {
        self.test.as_ref()
    }

    /// Returns the effective regular-check interval.
    #[must_use]
    pub const fn interval(&self) -> Option<&ProjectValue<HealthcheckDuration>> {
        self.interval.as_ref()
    }

    /// Returns the effective per-check timeout.
    #[must_use]
    pub const fn timeout(&self) -> Option<&ProjectValue<HealthcheckDuration>> {
        self.timeout.as_ref()
    }

    /// Returns the effective unhealthy retry count.
    #[must_use]
    pub const fn retries(&self) -> Option<&ProjectValue<HealthcheckRetries>> {
        self.retries.as_ref()
    }

    /// Returns the effective startup grace period.
    #[must_use]
    pub const fn start_period(&self) -> Option<&ProjectValue<HealthcheckDuration>> {
        self.start_period.as_ref()
    }

    /// Returns the effective interval used during the startup grace period.
    #[must_use]
    pub const fn start_interval(&self) -> Option<&ProjectValue<HealthcheckDuration>> {
        self.start_interval.as_ref()
    }

    /// Returns whether the image health check is explicitly disabled.
    #[must_use]
    pub const fn disable(&self) -> Option<&ProjectValue<BooleanValue>> {
        self.disable.as_ref()
    }

    /// Reports whether the effective definition explicitly disables health checks.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        matches!(
            self.disable.as_ref().map(ProjectValue::value),
            Some(BooleanValue::Literal(true))
        ) || matches!(
            self.test.as_ref().and_then(|test| test.value().kind()),
            Some(HealthcheckTestKind::None)
        )
    }

    /// Returns retained health-check fields outside the typed project-view boundary.
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[ProjectFieldReference] {
        &self.unmodeled_fields
    }
}

/// One selected service with the native fields needed by the first conversion boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectService {
    name: ProjectKey,
    provenance: MergeProvenance,
    image: Option<ProjectValue<ImageReference>>,
    command: Option<ProjectValue<Command>>,
    environment: Option<ProjectValue<ProjectEnvironment>>,
    extra_hosts: Option<ProjectValue<ProjectExtraHosts>>,
    healthcheck: Option<ProjectValue<ProjectHealthcheck>>,
    depends_on: Option<ProjectValue<ProjectDependsOn>>,
    ports: Option<ProjectValue<Vec<ProjectValue<Port>>>>,
    volumes: Option<ProjectValue<Vec<ProjectValue<VolumeMount>>>>,
    networks: Option<ProjectValue<ServiceNetworks>>,
    profiles: Option<ProjectValue<Vec<ProjectValue<String>>>>,
    unmodeled_fields: Vec<ProjectFieldReference>,
}

impl ProjectService {
    /// Returns the service name and all contributing key spans.
    #[must_use]
    pub const fn name(&self) -> &ProjectKey {
        &self.name
    }

    /// Returns provenance for the complete effective service mapping.
    #[must_use]
    pub const fn provenance(&self) -> &MergeProvenance {
        &self.provenance
    }

    /// Returns the effective image reference.
    #[must_use]
    pub const fn image(&self) -> Option<&ProjectValue<ImageReference>> {
        self.image.as_ref()
    }

    /// Returns the effective command without normalizing scalar and list forms.
    #[must_use]
    pub const fn command(&self) -> Option<&ProjectValue<Command>> {
        self.command.as_ref()
    }

    /// Returns environment entries normalized by key with per-entry syntax retained.
    #[must_use]
    pub const fn environment(&self) -> Option<&ProjectValue<ProjectEnvironment>> {
        self.environment.as_ref()
    }

    /// Returns effective service host mappings with per-entry provenance and syntax.
    #[must_use]
    pub const fn extra_hosts(&self) -> Option<&ProjectValue<ProjectExtraHosts>> {
        self.extra_hosts.as_ref()
    }

    /// Returns the effective health check with per-field merge provenance.
    #[must_use]
    pub const fn healthcheck(&self) -> Option<&ProjectValue<ProjectHealthcheck>> {
        self.healthcheck.as_ref()
    }

    /// Returns effective service dependencies with authored form and field-level provenance.
    #[must_use]
    pub const fn depends_on(&self) -> Option<&ProjectValue<ProjectDependsOn>> {
        self.depends_on.as_ref()
    }

    /// Returns the effective port collection and per-item provenance.
    #[must_use]
    pub const fn ports(&self) -> Option<&ProjectValue<Vec<ProjectValue<Port>>>> {
        self.ports.as_ref()
    }

    /// Returns the effective volume-mount collection and per-item provenance.
    #[must_use]
    pub const fn volumes(&self) -> Option<&ProjectValue<Vec<ProjectValue<VolumeMount>>>> {
        self.volumes.as_ref()
    }

    /// Returns effective network attachments with short and long forms retained.
    #[must_use]
    pub const fn networks(&self) -> Option<&ProjectValue<ServiceNetworks>> {
        self.networks.as_ref()
    }

    /// Returns effective profile names and their individual provenance.
    #[must_use]
    pub const fn profiles(&self) -> Option<&ProjectValue<Vec<ProjectValue<String>>>> {
        self.profiles.as_ref()
    }

    /// Returns fields retained outside this initial native project-view boundary.
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[ProjectFieldReference] {
        &self.unmodeled_fields
    }
}

/// One named top-level resource with key and definition provenance kept separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectResource<T> {
    name: ProjectKey,
    definition: ProjectValue<T>,
}

impl<T> ProjectResource<T> {
    /// Returns the model name and all authored key locations.
    #[must_use]
    pub const fn name(&self) -> &ProjectKey {
        &self.name
    }

    /// Returns the native effective definition and its merge provenance.
    #[must_use]
    pub const fn definition(&self) -> &ProjectValue<T> {
        &self.definition
    }
}

/// The native consumer view of one merged and optionally profile-selected Compose project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    source_ids: Vec<SourceId>,
    base_directory: PathBuf,
    provenance: MergeProvenance,
    name: Option<ProjectValue<String>>,
    services: Vec<ProjectService>,
    networks: Vec<ProjectResource<NetworkDefinition>>,
    volumes: Vec<ProjectResource<VolumeDefinition>>,
    configs: Vec<ProjectResource<ConfigDefinition>>,
    secrets: Vec<ProjectResource<SecretDefinition>>,
    unmodeled_fields: Vec<ProjectFieldReference>,
}

impl ProjectView {
    /// Returns source documents in merge order.
    #[must_use]
    pub fn source_ids(&self) -> &[SourceId] {
        &self.source_ids
    }

    /// Returns the project directory inherited from the first loaded document.
    #[must_use]
    pub fn base_directory(&self) -> &Path {
        &self.base_directory
    }

    /// Returns provenance for the complete merged root.
    #[must_use]
    pub const fn provenance(&self) -> &MergeProvenance {
        &self.provenance
    }

    /// Returns the effective explicit project name.
    #[must_use]
    pub const fn name(&self) -> Option<&ProjectValue<String>> {
        self.name.as_ref()
    }

    /// Returns profile-active services in merged order.
    #[must_use]
    pub fn services(&self) -> &[ProjectService] {
        &self.services
    }

    /// Finds one profile-active service.
    #[must_use]
    pub fn service(&self, name: &str) -> Option<&ProjectService> {
        self.services.iter().find(|service| service.name.value == name)
    }

    /// Returns effective top-level network definitions.
    #[must_use]
    pub fn networks(&self) -> &[ProjectResource<NetworkDefinition>] {
        &self.networks
    }

    /// Returns effective top-level volume definitions.
    #[must_use]
    pub fn volumes(&self) -> &[ProjectResource<VolumeDefinition>] {
        &self.volumes
    }

    /// Returns effective top-level config definitions.
    #[must_use]
    pub fn configs(&self) -> &[ProjectResource<ConfigDefinition>] {
        &self.configs
    }

    /// Returns effective top-level secret definitions.
    #[must_use]
    pub fn secrets(&self) -> &[ProjectResource<SecretDefinition>] {
        &self.secrets
    }

    /// Returns root fields retained outside this initial native project-view boundary.
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[ProjectFieldReference] {
        &self.unmodeled_fields
    }
}

/// Recoverable result of building a typed merged project view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectViewResult {
    view: Option<ProjectView>,
    diagnostics: Vec<Diagnostic>,
}

impl ProjectViewResult {
    /// Returns the typed view when the profile selection belongs to the project.
    #[must_use]
    pub const fn view(&self) -> Option<&ProjectView> {
        self.view.as_ref()
    }

    /// Returns project-view diagnostics in traversal order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether a view exists and contains no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.view.is_some()
            && self
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }

    /// Separates the view and diagnostics.
    #[must_use]
    pub fn into_parts(self) -> (Option<ProjectView>, Vec<Diagnostic>) {
        (self.view, self.diagnostics)
    }
}

/// Builds native values directly from a merged project without canonical rendering or reparsing.
///
/// A matching selection filters inactive services. Omitting it includes every service. The
/// operation performs no file, environment, provider, or runtime access.
#[must_use]
pub fn build_project_view(project: &MergedProject, selection: Option<&ProfileSelection>) -> ProjectViewResult {
    if selection.is_some_and(|selection| !selection.belongs_to(project)) {
        return ProjectViewResult {
            view: None,
            diagnostics: vec![Diagnostic::new(
                SELECTION_PROJECT_MISMATCH,
                Severity::Error,
                "profile selection does not belong to the merged project",
            )],
        };
    }

    Builder::new(project, selection).build()
}

struct Builder<'a> {
    project: &'a MergedProject,
    selection: Option<&'a ProfileSelection>,
    diagnostics: Vec<Diagnostic>,
    root_unmodeled: Vec<ProjectFieldReference>,
    pending_unmodeled: Vec<ProjectFieldReference>,
}

impl<'a> Builder<'a> {
    const fn new(project: &'a MergedProject, selection: Option<&'a ProfileSelection>) -> Self {
        Self {
            project,
            selection,
            diagnostics: Vec::new(),
            root_unmodeled: Vec::new(),
            pending_unmodeled: Vec::new(),
        }
    }

    fn build(mut self) -> ProjectViewResult {
        let root = self.project.root();
        let entries = root.as_mapping().unwrap_or_default();
        let mut name = None;
        let mut services = Vec::new();
        let mut networks = Vec::new();
        let mut volumes = Vec::new();
        let mut configs = Vec::new();
        let mut secrets = Vec::new();

        for entry in entries {
            match entry.key() {
                "name" => name = self.project_string(entry.value(), "project name"),
                "services" => services = self.services(entry.value()),
                "networks" => networks = self.network_definitions(entry.value()),
                "volumes" => volumes = self.volume_definitions(entry.value()),
                "configs" => configs = self.config_definitions(entry.value()),
                "secrets" => secrets = self.secret_definitions(entry.value()),
                _ => self.record_root_unmodeled(&[], entry),
            }
        }

        ProjectViewResult {
            view: Some(ProjectView {
                source_ids: self.project.source_ids().to_vec(),
                base_directory: self.project.base_directory().to_path_buf(),
                provenance: root.provenance().clone(),
                name,
                services,
                networks,
                volumes,
                configs,
                secrets,
                unmodeled_fields: self.root_unmodeled,
            }),
            diagnostics: self.diagnostics,
        }
    }

    fn services(&mut self, value: &MergedValue) -> Vec<ProjectService> {
        let Some(entries) = self.mapping(value, "services must be a mapping") else {
            return Vec::new();
        };
        let selection = self.selection;
        let mut services = Vec::new();
        for entry in entries {
            if service_in_scope(selection, entry.key()) {
                services.extend(self.service(entry));
            }
        }
        services
    }

    fn service(&mut self, entry: &MergedEntry) -> Option<ProjectService> {
        let pending_start = self.pending_unmodeled.len();
        let value = entry.value();
        let fields = self.mapping(value, "service definition must be a mapping")?;
        let mut service = ProjectService {
            name: ProjectKey::from_entry(entry),
            provenance: value.provenance().clone(),
            image: None,
            command: None,
            environment: None,
            extra_hosts: None,
            healthcheck: None,
            depends_on: None,
            ports: None,
            volumes: None,
            networks: None,
            profiles: None,
            unmodeled_fields: Vec::new(),
        };
        let path = ["services".to_owned(), entry.key().to_owned()];

        for field in fields {
            match field.key() {
                "image" => {
                    service.image = self
                        .project_string(field.value(), "service image")
                        .map(|value| ProjectValue {
                            value: ImageReference::parse(value.value),
                            provenance: value.provenance,
                            sensitive: value.sensitive,
                        });
                }
                "command" => service.command = self.command(field.value()),
                "environment" => service.environment = self.environment(field.value()),
                "extra_hosts" => service.extra_hosts = self.extra_hosts(field.value()),
                "healthcheck" => service.healthcheck = self.healthcheck(field.value(), &path),
                "depends_on" => service.depends_on = self.depends_on(field.value(), &path),
                "ports" => service.ports = self.ports(field.value(), &path),
                "volumes" => service.volumes = self.volumes(field.value(), &path),
                "networks" => service.networks = self.service_networks(field.value(), &path),
                "profiles" => service.profiles = self.string_collection(field.value(), "profiles must be a sequence"),
                _ => service.unmodeled_fields.push(field_reference(&path, field)),
            }
        }
        service
            .unmodeled_fields
            .extend(self.pending_unmodeled.drain(pending_start..));
        Some(service)
    }

    fn command(&mut self, value: &MergedValue) -> Option<ProjectValue<Command>> {
        let span = effective_span(value);
        let command = match value.kind() {
            MergedValueKind::Null(_) => Command::Null(span),
            MergedValueKind::Scalar(scalar) => Command::String(Located::new(scalar.value().to_owned(), span)),
            MergedValueKind::Sequence(values) => {
                let mut arguments = Vec::new();
                for value in values {
                    arguments.push(self.located_string(value, "command list item must be a scalar")?);
                }
                Command::List {
                    span,
                    values: arguments,
                }
            }
            _ => {
                self.expected(value, "command must be null, a scalar, or a sequence");
                return None;
            }
        };
        Some(ProjectValue::new(command, value))
    }

    fn environment(&mut self, value: &MergedValue) -> Option<ProjectValue<ProjectEnvironment>> {
        let mut entries = Vec::new();
        match value.kind() {
            MergedValueKind::Mapping(values) => {
                for entry in values {
                    let scalar = self.compose_scalar(entry.value(), "environment value must be a scalar or null")?;
                    entries.push(ProjectEnvironmentEntry {
                        name: ProjectKey::from_entry(entry),
                        value: ProjectValue::new(scalar, entry.value()),
                        syntax: entry.syntax(),
                    });
                }
            }
            MergedValueKind::Sequence(values) => {
                for item in values {
                    let raw = self.located_string(item, "environment list item must be a scalar")?;
                    let (name, scalar, syntax) = raw.value().split_once('=').map_or_else(
                        || (raw.value().clone(), ComposeScalar::Null, EntrySyntax::ListKeyOnly),
                        |(name, value)| {
                            (
                                name.to_owned(),
                                ComposeScalar::String(value.to_owned()),
                                EntrySyntax::ListKeyValue,
                            )
                        },
                    );
                    entries.push(ProjectEnvironmentEntry {
                        name: ProjectKey {
                            value: name,
                            sources: item.provenance().sources().to_vec(),
                            sensitive: item.is_sensitive(),
                        },
                        value: ProjectValue::new(scalar, item),
                        syntax,
                    });
                }
            }
            _ => {
                self.expected(value, "environment must be a mapping or sequence");
                return None;
            }
        }
        Some(ProjectValue::new(ProjectEnvironment { entries }, value))
    }

    fn healthcheck(&mut self, value: &MergedValue, parent_path: &[String]) -> Option<ProjectValue<ProjectHealthcheck>> {
        let fields = self.mapping(value, "healthcheck must be a mapping")?;
        let mut healthcheck = ProjectHealthcheck {
            test: None,
            interval: None,
            timeout: None,
            retries: None,
            start_period: None,
            start_interval: None,
            disable: None,
            unmodeled_fields: Vec::new(),
        };
        let mut path = parent_path.to_vec();
        path.push("healthcheck".to_owned());
        for field in fields {
            match field.key() {
                "test" => healthcheck.test = self.healthcheck_test(field.value()),
                "interval" => {
                    healthcheck.interval =
                        self.healthcheck_duration(field.value(), "healthcheck interval must be a scalar");
                }
                "timeout" => {
                    healthcheck.timeout =
                        self.healthcheck_duration(field.value(), "healthcheck timeout must be a scalar");
                }
                "retries" => healthcheck.retries = self.healthcheck_retries(field.value()),
                "start_period" => {
                    healthcheck.start_period =
                        self.healthcheck_duration(field.value(), "healthcheck start_period must be a scalar");
                }
                "start_interval" => {
                    healthcheck.start_interval =
                        self.healthcheck_duration(field.value(), "healthcheck start_interval must be a scalar");
                }
                "disable" => {
                    healthcheck.disable = self
                        .located_boolean(field.value(), "healthcheck disable must be a boolean")
                        .map(|value| ProjectValue::new(value.into_value(), field.value()));
                }
                _ => healthcheck.unmodeled_fields.push(field_reference(&path, field)),
            }
        }
        Some(ProjectValue::new(healthcheck, value))
    }

    fn depends_on(&mut self, value: &MergedValue, parent_path: &[String]) -> Option<ProjectValue<ProjectDependsOn>> {
        let dependencies = match value.kind() {
            MergedValueKind::Sequence(values) => {
                let mut dependencies = Vec::new();
                for value in values {
                    let Some(service) = self.project_string(value, "dependency service name") else {
                        continue;
                    };
                    let dependency = ProjectServiceDependency {
                        service: ProjectKey::from_value(service.value, value),
                        condition: None,
                        restart: None,
                        required: None,
                        unmodeled_fields: Vec::new(),
                    };
                    dependencies.push(ProjectValue::new(dependency, value));
                }
                ProjectDependsOn::Short(dependencies)
            }
            MergedValueKind::Mapping(entries) => {
                let mut dependencies = Vec::new();
                let mut path = parent_path.to_vec();
                path.push("depends_on".to_owned());
                for entry in entries {
                    let mut dependency = ProjectServiceDependency {
                        service: ProjectKey::from_entry(entry),
                        condition: None,
                        restart: None,
                        required: None,
                        unmodeled_fields: Vec::new(),
                    };
                    let fields = match entry.value().kind() {
                        MergedValueKind::Null(_) => &[][..],
                        MergedValueKind::Mapping(fields) => fields.as_slice(),
                        _ => {
                            self.expected(entry.value(), "long dependency options must be a mapping or null");
                            continue;
                        }
                    };
                    let mut dependency_path = path.clone();
                    dependency_path.push(entry.key().to_owned());
                    for field in fields {
                        match field.key() {
                            "condition" => {
                                let Some(condition) = self.project_string(field.value(), "dependency condition") else {
                                    continue;
                                };
                                let parsed = DependencyCondition::parse(condition.value);
                                if !parsed.is_known() {
                                    self.invalid(
                                        effective_span(field.value()),
                                        "dependency condition is not defined by Compose",
                                    );
                                }
                                dependency.condition = Some(ProjectValue {
                                    value: parsed,
                                    provenance: condition.provenance,
                                    sensitive: condition.sensitive,
                                });
                            }
                            "restart" => {
                                dependency.restart = self
                                    .located_boolean(field.value(), "dependency restart must be a boolean")
                                    .map(|value| ProjectValue::new(value.into_value(), field.value()));
                            }
                            "required" => {
                                dependency.required = self
                                    .located_boolean(field.value(), "dependency required must be a boolean")
                                    .map(|value| ProjectValue::new(value.into_value(), field.value()));
                            }
                            _ => dependency
                                .unmodeled_fields
                                .push(field_reference(&dependency_path, field)),
                        }
                    }
                    dependencies.push(ProjectValue::new(dependency, entry.value()));
                }
                ProjectDependsOn::Long(dependencies)
            }
            _ => {
                self.expected(value, "depends_on must be a sequence or mapping");
                return None;
            }
        };
        Some(ProjectValue::new(dependencies, value))
    }

    fn healthcheck_test(&mut self, value: &MergedValue) -> Option<ProjectValue<HealthcheckTest>> {
        let span = effective_span(value);
        let test = match value.kind() {
            MergedValueKind::Scalar(scalar) => HealthcheckTest::String(Located::new(scalar.value().to_owned(), span)),
            MergedValueKind::Sequence(values) => {
                let mut items = Vec::new();
                for value in values {
                    items.push(self.located_string(value, "healthcheck test item must be a scalar")?);
                }
                let kind = items.first().map(|item| HealthcheckTestKind::parse(item.value()));
                HealthcheckTest::List {
                    span,
                    kind,
                    values: items,
                }
            }
            _ => {
                self.expected(value, "healthcheck test must be a scalar or sequence");
                return None;
            }
        };
        Some(ProjectValue::new(test, value))
    }

    fn healthcheck_duration(
        &mut self,
        value: &MergedValue,
        message: &str,
    ) -> Option<ProjectValue<HealthcheckDuration>> {
        let scalar = self.scalar(value, message)?;
        Some(ProjectValue::new(
            HealthcheckDuration::parse(scalar.value().to_owned()),
            value,
        ))
    }

    fn healthcheck_retries(&mut self, value: &MergedValue) -> Option<ProjectValue<HealthcheckRetries>> {
        let scalar = self.scalar(value, "healthcheck retries must be a scalar")?;
        Some(ProjectValue::new(
            HealthcheckRetries::parse(scalar.value().to_owned()),
            value,
        ))
    }

    fn extra_hosts(&mut self, value: &MergedValue) -> Option<ProjectValue<ProjectExtraHosts>> {
        let mut entries = Vec::new();
        match value.kind() {
            MergedValueKind::Mapping(values) => {
                for entry in values {
                    let scalar = self.scalar(entry.value(), "extra_hosts address must be a scalar")?;
                    entries.push(ProjectExtraHost {
                        hostname: ProjectKey::from_entry(entry),
                        address: ProjectValue::new(HostAddress::parse(scalar.value().to_owned()), entry.value()),
                        syntax: EntrySyntax::Mapping,
                    });
                }
            }
            MergedValueKind::Sequence(values) => {
                for item in values {
                    let raw = self.located_string(item, "extra_hosts list item must be a scalar")?;
                    let parsed = ShortExtraHost::parse(raw);
                    let (Some(hostname), Some(address)) = (parsed.hostname(), parsed.address()) else {
                        self.invalid(
                            effective_span(item),
                            "extra_hosts entry must contain a hostname and address",
                        );
                        continue;
                    };
                    entries.push(ProjectExtraHost {
                        hostname: ProjectKey {
                            value: hostname.to_owned(),
                            sources: item.provenance().sources().to_vec(),
                            sensitive: item.is_sensitive(),
                        },
                        address: ProjectValue::new(address.clone(), item),
                        syntax: EntrySyntax::ListKeyValue,
                    });
                }
            }
            _ => {
                self.expected(value, "extra_hosts must be a mapping or sequence");
                return None;
            }
        }
        Some(ProjectValue::new(ProjectExtraHosts { entries }, value))
    }

    fn project_string(&mut self, value: &MergedValue, description: &str) -> Option<ProjectValue<String>> {
        let scalar = self.scalar(value, &format!("{description} must be a non-null scalar"))?;
        Some(ProjectValue::new(scalar.value().to_owned(), value))
    }

    fn string_collection(
        &mut self,
        value: &MergedValue,
        message: &str,
    ) -> Option<ProjectValue<Vec<ProjectValue<String>>>> {
        let Some(values) = value.as_sequence() else {
            self.expected(value, message);
            return None;
        };
        let mut strings = Vec::new();
        for value in values {
            let scalar = self.scalar(value, "sequence item must be a non-null scalar")?;
            strings.push(ProjectValue::new(scalar.value().to_owned(), value));
        }
        Some(ProjectValue::new(strings, value))
    }

    fn scalar<'value>(
        &mut self,
        value: &'value MergedValue,
        message: &str,
    ) -> Option<&'value crate::merge::MergedScalar> {
        let Some(scalar) = value.as_scalar() else {
            self.expected(value, message);
            return None;
        };
        Some(scalar)
    }

    fn located_string(&mut self, value: &MergedValue, message: &str) -> Option<Located<String>> {
        let scalar = self.scalar(value, message)?;
        Some(Located::new(scalar.value().to_owned(), effective_span(value)))
    }

    fn compose_scalar(&mut self, value: &MergedValue, message: &str) -> Option<ComposeScalar> {
        match value.kind() {
            MergedValueKind::Null(_) => Some(ComposeScalar::Null),
            MergedValueKind::Scalar(scalar) => Some(match scalar.kind() {
                MergedScalarKind::String => ComposeScalar::String(scalar.value().to_owned()),
                MergedScalarKind::Boolean => ComposeScalar::Boolean(scalar.value().eq_ignore_ascii_case("true")),
                MergedScalarKind::Number => ComposeScalar::Number(scalar.value().to_owned()),
            }),
            _ => {
                self.expected(value, message);
                None
            }
        }
    }

    fn mapping<'value>(&mut self, value: &'value MergedValue, message: &str) -> Option<&'value [MergedEntry]> {
        let Some(entries) = value.as_mapping() else {
            self.expected(value, message);
            return None;
        };
        Some(entries)
    }

    fn expected(&mut self, value: &MergedValue, message: &str) {
        self.diagnostics.push(
            Diagnostic::new(PROJECT_EXPECTED_FORM, Severity::Error, message).with_label(DiagnosticLabel::primary(
                effective_span(value),
                "unexpected merged value form",
            )),
        );
    }

    fn missing(&mut self, value: &MergedValue, message: &str) {
        self.diagnostics.push(
            Diagnostic::new(PROJECT_MISSING_FIELD, Severity::Error, message).with_label(DiagnosticLabel::primary(
                effective_span(value),
                "required field is missing",
            )),
        );
    }

    fn invalid(&mut self, span: SourceSpan, message: &str) {
        self.diagnostics.push(
            Diagnostic::new(PROJECT_INVALID_VALUE, Severity::Error, message)
                .with_label(DiagnosticLabel::primary(span, "invalid native value")),
        );
    }

    fn record_root_unmodeled(&mut self, path: &[String], entry: &MergedEntry) {
        self.root_unmodeled.push(field_reference(path, entry));
    }

    fn record_pending_unmodeled(&mut self, path: &[String], entry: &MergedEntry) {
        self.pending_unmodeled.push(field_reference(path, entry));
    }
}

impl Builder<'_> {
    fn ports(&mut self, value: &MergedValue, service_path: &[String]) -> Option<ProjectValue<Vec<ProjectValue<Port>>>> {
        let Some(values) = value.as_sequence() else {
            self.expected(value, "service ports must be a sequence");
            return None;
        };
        let mut ports = Vec::new();
        for (index, item) in values.iter().enumerate() {
            let mut path = service_path.to_vec();
            path.push("ports".to_owned());
            path.push(index.to_string());
            let port = match item.kind() {
                MergedValueKind::Scalar(scalar) => Port::Short(ShortPort::parse(Located::new(
                    scalar.value().to_owned(),
                    effective_span(item),
                ))),
                MergedValueKind::Mapping(fields) => Port::Long(Box::new(self.long_port(item, fields, &path))),
                _ => {
                    self.expected(item, "service port must use scalar short syntax or mapping long syntax");
                    continue;
                }
            };
            ports.push(ProjectValue::new(port, item));
        }
        Some(ProjectValue::new(ports, value))
    }

    fn long_port(&mut self, value: &MergedValue, fields: &[MergedEntry], path: &[String]) -> LongPort {
        let mut port = LongPort::new(effective_span(value));
        let mut has_target = false;
        for field in fields {
            match field.key() {
                "target" => {
                    if let Some(value) = self.located_string(field.value(), "port target must be a scalar") {
                        port.set_target(value);
                        has_target = true;
                    }
                }
                "published" => self
                    .located_string(field.value(), "published port must be a scalar")
                    .into_iter()
                    .for_each(|value| port.set_published(value)),
                "host_ip" => self
                    .located_string(field.value(), "port host_ip must be a scalar")
                    .into_iter()
                    .for_each(|value| port.set_host_ip(value)),
                "protocol" => self
                    .located_string(field.value(), "port protocol must be a scalar")
                    .into_iter()
                    .for_each(|value| port.set_protocol(value)),
                "app_protocol" => self
                    .located_string(field.value(), "port app_protocol must be a scalar")
                    .into_iter()
                    .for_each(|value| port.set_app_protocol(value)),
                "mode" => self
                    .located_string(field.value(), "port mode must be a scalar")
                    .into_iter()
                    .for_each(|value| port.set_mode(value)),
                "name" => self
                    .located_string(field.value(), "port name must be a scalar")
                    .into_iter()
                    .for_each(|value| port.set_name(value)),
                _ => self.record_pending_unmodeled(path, field),
            }
        }
        if !has_target {
            self.missing(value, "long-syntax port is missing `target`");
        }
        port
    }

    fn volumes(
        &mut self,
        value: &MergedValue,
        service_path: &[String],
    ) -> Option<ProjectValue<Vec<ProjectValue<VolumeMount>>>> {
        let Some(values) = value.as_sequence() else {
            self.expected(value, "service volumes must be a sequence");
            return None;
        };
        let mut mounts = Vec::new();
        for (index, item) in values.iter().enumerate() {
            let mut path = service_path.to_vec();
            path.push("volumes".to_owned());
            path.push(index.to_string());
            let mount = match item.kind() {
                MergedValueKind::Scalar(scalar) => VolumeMount::Short(ShortVolumeMount::new(Located::new(
                    scalar.value().to_owned(),
                    effective_span(item),
                ))),
                MergedValueKind::Mapping(fields) => VolumeMount::Long(Box::new(self.long_volume(item, fields, &path))),
                _ => {
                    self.expected(
                        item,
                        "service volume must use scalar short syntax or mapping long syntax",
                    );
                    continue;
                }
            };
            mounts.push(ProjectValue::new(mount, item));
        }
        Some(ProjectValue::new(mounts, value))
    }

    fn long_volume(&mut self, value: &MergedValue, fields: &[MergedEntry], path: &[String]) -> LongVolumeMount {
        let mut mount = LongVolumeMount::new(effective_span(value));
        let mut has_type = false;
        let mut has_target = false;
        for field in fields {
            match field.key() {
                "type" => {
                    if let Some(value) = self.located_string(field.value(), "volume type must be a scalar") {
                        mount.set_mount_type(Located::new(MountType::from_text(value.value().clone()), value.span()));
                        has_type = true;
                    }
                }
                "source" => self
                    .located_string(field.value(), "volume source must be a scalar")
                    .into_iter()
                    .for_each(|value| mount.set_source(value)),
                "target" => {
                    if let Some(value) = self.located_string(field.value(), "volume target must be a scalar") {
                        mount.set_target(value);
                        has_target = true;
                    }
                }
                "read_only" => self
                    .located_boolean(field.value(), "volume read_only must be a boolean")
                    .into_iter()
                    .for_each(|value| mount.set_read_only(value)),
                "bind" => self
                    .bind_options(field.value(), path)
                    .into_iter()
                    .for_each(|value| mount.set_bind(value)),
                _ => self.record_pending_unmodeled(path, field),
            }
        }
        if !has_type {
            self.missing(value, "long-syntax volume is missing `type`");
        }
        if !has_target {
            self.missing(value, "long-syntax volume is missing `target`");
        }
        mount
    }

    fn bind_options(&mut self, value: &MergedValue, parent_path: &[String]) -> Option<BindOptions> {
        let fields = self.mapping(value, "volume bind options must be a mapping")?;
        let mut bind = BindOptions::new(effective_span(value));
        let mut path = parent_path.to_vec();
        path.push("bind".to_owned());
        for field in fields {
            match field.key() {
                "propagation" => self
                    .located_string(field.value(), "bind propagation must be a scalar")
                    .into_iter()
                    .for_each(|value| bind.set_propagation(value)),
                "create_host_path" => self
                    .located_boolean(field.value(), "bind create_host_path must be a boolean")
                    .into_iter()
                    .for_each(|value| bind.set_create_host_path(value)),
                "selinux" => {
                    if let Some(value) = self.located_string(field.value(), "bind SELinux mode must be a scalar") {
                        let mode = match value.value().as_str() {
                            "z" => Some(SelinuxRelabel::Shared),
                            "Z" => Some(SelinuxRelabel::Private),
                            _ => None,
                        };
                        if let Some(mode) = mode {
                            bind.set_selinux(Located::new(mode, value.span()));
                        } else {
                            self.invalid(value.span(), "bind SELinux mode must be `z` or `Z`");
                        }
                    }
                }
                _ => self.record_pending_unmodeled(&path, field),
            }
        }
        Some(bind)
    }

    fn service_networks(
        &mut self,
        value: &MergedValue,
        service_path: &[String],
    ) -> Option<ProjectValue<ServiceNetworks>> {
        let span = effective_span(value);
        let networks = match value.kind() {
            MergedValueKind::Sequence(values) => {
                let mut names = Vec::new();
                for value in values {
                    names.push(self.located_string(value, "service network name must be a scalar")?);
                }
                ServiceNetworks::Short { span, names }
            }
            MergedValueKind::Mapping(entries) => {
                let mut networks = Vec::new();
                for entry in entries {
                    let mut path = service_path.to_vec();
                    path.push("networks".to_owned());
                    path.push(entry.key().to_owned());
                    networks.push(self.service_network(entry, &path)?);
                }
                ServiceNetworks::Long { span, networks }
            }
            _ => {
                self.expected(value, "service networks must be a sequence or mapping");
                return None;
            }
        };
        Some(ProjectValue::new(networks, value))
    }

    fn service_network(&mut self, entry: &MergedEntry, path: &[String]) -> Option<ServiceNetwork> {
        let value = entry.value();
        let span = effective_span(value);
        let mut network = ServiceNetwork::new(Located::new(entry.key().to_owned(), entry_span(entry)), span);
        let fields = match value.kind() {
            MergedValueKind::Null(_) => return Some(network),
            MergedValueKind::Mapping(fields) => fields,
            _ => {
                self.expected(value, "service network attachment must be a mapping or null");
                return None;
            }
        };
        for field in fields {
            match field.key() {
                "aliases" => self
                    .located_string_sequence(field.value(), "network aliases must be a sequence")
                    .into_iter()
                    .for_each(|value| network.set_aliases(value)),
                "interface_name" => self
                    .located_string(field.value(), "network interface_name must be a scalar")
                    .into_iter()
                    .for_each(|value| network.set_interface_name(value)),
                "ipv4_address" => self
                    .located_string(field.value(), "network ipv4_address must be a scalar")
                    .into_iter()
                    .for_each(|value| network.set_ipv4_address(value)),
                "ipv6_address" => self
                    .located_string(field.value(), "network ipv6_address must be a scalar")
                    .into_iter()
                    .for_each(|value| network.set_ipv6_address(value)),
                "link_local_ips" => self
                    .located_string_sequence(field.value(), "link_local_ips must be a sequence")
                    .into_iter()
                    .for_each(|value| network.set_link_local_ips(value)),
                "mac_address" => self
                    .located_string(field.value(), "network mac_address must be a scalar")
                    .into_iter()
                    .for_each(|value| network.set_mac_address(value)),
                "driver_opts" => self
                    .key_value_mapping(field.value(), "network driver_opts must be a mapping")
                    .into_iter()
                    .for_each(|value| network.set_driver_opts(value)),
                "gw_priority" => self
                    .located_string(field.value(), "network gw_priority must be a scalar")
                    .into_iter()
                    .for_each(|value| network.set_gw_priority(value)),
                "priority" => self
                    .located_string(field.value(), "network priority must be a scalar")
                    .into_iter()
                    .for_each(|value| network.set_priority(value)),
                _ => self.record_pending_unmodeled(path, field),
            }
        }
        Some(network)
    }

    fn located_boolean(&mut self, value: &MergedValue, message: &str) -> Option<Located<BooleanValue>> {
        let scalar = self.scalar(value, message)?;
        let boolean = if scalar.kind() == MergedScalarKind::Boolean {
            BooleanValue::Literal(scalar.value().eq_ignore_ascii_case("true"))
        } else if scalar.value().contains('$') {
            BooleanValue::Expression(scalar.value().to_owned())
        } else {
            self.invalid(effective_span(value), message);
            return None;
        };
        Some(Located::new(boolean, effective_span(value)))
    }

    fn located_string_sequence(&mut self, value: &MergedValue, message: &str) -> Option<Vec<Located<String>>> {
        let Some(values) = value.as_sequence() else {
            self.expected(value, message);
            return None;
        };
        let mut strings = Vec::new();
        for value in values {
            strings.push(self.located_string(value, "sequence item must be a scalar")?);
        }
        Some(strings)
    }

    fn key_value_mapping(&mut self, value: &MergedValue, message: &str) -> Option<Vec<KeyValueEntry>> {
        let Some(entries) = value.as_mapping() else {
            self.expected(value, message);
            return None;
        };
        let mut values = Vec::new();
        for entry in entries {
            let scalar = self.compose_scalar(entry.value(), "mapping value must be a scalar or null")?;
            let value_span = effective_span(entry.value());
            values.push(KeyValueEntry::new(
                Located::new(entry.key().to_owned(), entry_span(entry)),
                Located::new(scalar, value_span),
                value_span,
            ));
        }
        Some(values)
    }

    fn network_definitions(&mut self, value: &MergedValue) -> Vec<ProjectResource<NetworkDefinition>> {
        let Some(entries) = self.mapping(value, "top-level networks must be a mapping") else {
            return Vec::new();
        };
        entries
            .iter()
            .filter_map(|entry| {
                let definition = self.network_definition(entry)?;
                Some(ProjectResource {
                    name: ProjectKey::from_entry(entry),
                    definition: ProjectValue::new(definition, entry.value()),
                })
            })
            .collect()
    }

    fn network_definition(&mut self, entry: &MergedEntry) -> Option<NetworkDefinition> {
        let value = entry.value();
        let span = effective_span(value);
        let mut network = NetworkDefinition::new(Located::new(entry.key().to_owned(), entry_span(entry)), span);
        let fields = match value.kind() {
            MergedValueKind::Null(_) => return Some(network),
            MergedValueKind::Mapping(fields) => fields,
            _ => {
                self.expected(value, "network definition must be a mapping or null");
                return None;
            }
        };
        let path = ["networks".to_owned(), entry.key().to_owned()];
        for field in fields {
            match field.key() {
                "driver" => self
                    .located_string(field.value(), "network driver must be a scalar")
                    .into_iter()
                    .for_each(|value| network.set_driver(value)),
                "driver_opts" => self
                    .key_value_mapping(field.value(), "network driver_opts must be a mapping")
                    .into_iter()
                    .for_each(|value| network.set_driver_opts(value)),
                "attachable" => self
                    .located_boolean(field.value(), "network attachable must be a boolean")
                    .into_iter()
                    .for_each(|value| network.set_attachable(value)),
                "enable_ipv4" => self
                    .located_boolean(field.value(), "network enable_ipv4 must be a boolean")
                    .into_iter()
                    .for_each(|value| network.set_enable_ipv4(value)),
                "enable_ipv6" => self
                    .located_boolean(field.value(), "network enable_ipv6 must be a boolean")
                    .into_iter()
                    .for_each(|value| network.set_enable_ipv6(value)),
                "external" => self
                    .located_boolean(field.value(), "network external must be a boolean")
                    .into_iter()
                    .for_each(|value| network.set_external(value)),
                "internal" => self
                    .located_boolean(field.value(), "network internal must be a boolean")
                    .into_iter()
                    .for_each(|value| network.set_internal(value)),
                "ipam" => self
                    .ipam(field.value(), &path)
                    .into_iter()
                    .for_each(|value| network.set_ipam(value)),
                "labels" => self
                    .labels(field.value())
                    .into_iter()
                    .for_each(|value| network.set_labels(value)),
                "name" => self
                    .located_string(field.value(), "network custom name must be a scalar")
                    .into_iter()
                    .for_each(|value| network.set_custom_name(value)),
                _ => self.record_root_unmodeled(&path, field),
            }
        }
        Some(network)
    }

    fn ipam(&mut self, value: &MergedValue, parent_path: &[String]) -> Option<Ipam> {
        let fields = self.mapping(value, "network IPAM must be a mapping")?;
        let mut ipam = Ipam::new(effective_span(value));
        let mut path = parent_path.to_vec();
        path.push("ipam".to_owned());
        for field in fields {
            match field.key() {
                "driver" => self
                    .located_string(field.value(), "IPAM driver must be a scalar")
                    .into_iter()
                    .for_each(|value| ipam.set_driver(value)),
                "config" => self
                    .ipam_configs(field.value(), &path)
                    .into_iter()
                    .for_each(|value| ipam.set_config(value)),
                "options" => self
                    .key_value_mapping(field.value(), "IPAM options must be a mapping")
                    .into_iter()
                    .for_each(|value| ipam.set_options(value)),
                _ => self.record_root_unmodeled(&path, field),
            }
        }
        Some(ipam)
    }

    fn ipam_configs(&mut self, value: &MergedValue, parent_path: &[String]) -> Option<Vec<IpamConfig>> {
        let Some(values) = value.as_sequence() else {
            self.expected(value, "IPAM config must be a sequence");
            return None;
        };
        let mut configs = Vec::new();
        for (index, value) in values.iter().enumerate() {
            let Some(fields) = value.as_mapping() else {
                self.expected(value, "IPAM config entry must be a mapping");
                continue;
            };
            let mut config = IpamConfig::new(effective_span(value));
            let mut path = parent_path.to_vec();
            path.push("config".to_owned());
            path.push(index.to_string());
            for field in fields {
                match field.key() {
                    "subnet" => self
                        .located_string(field.value(), "IPAM subnet must be a scalar")
                        .into_iter()
                        .for_each(|value| config.set_subnet(value)),
                    "ip_range" => self
                        .located_string(field.value(), "IPAM ip_range must be a scalar")
                        .into_iter()
                        .for_each(|value| config.set_ip_range(value)),
                    "gateway" => self
                        .located_string(field.value(), "IPAM gateway must be a scalar")
                        .into_iter()
                        .for_each(|value| config.set_gateway(value)),
                    "aux_addresses" => self
                        .key_value_mapping(field.value(), "IPAM aux_addresses must be a mapping")
                        .into_iter()
                        .for_each(|value| config.set_aux_addresses(value)),
                    _ => self.record_root_unmodeled(&path, field),
                }
            }
            configs.push(config);
        }
        Some(configs)
    }

    fn volume_definitions(&mut self, value: &MergedValue) -> Vec<ProjectResource<VolumeDefinition>> {
        let Some(entries) = self.mapping(value, "top-level volumes must be a mapping") else {
            return Vec::new();
        };
        entries
            .iter()
            .filter_map(|entry| {
                let definition = self.volume_definition(entry)?;
                Some(ProjectResource {
                    name: ProjectKey::from_entry(entry),
                    definition: ProjectValue::new(definition, entry.value()),
                })
            })
            .collect()
    }

    fn volume_definition(&mut self, entry: &MergedEntry) -> Option<VolumeDefinition> {
        let value = entry.value();
        let span = effective_span(value);
        let mut volume = VolumeDefinition::new(Located::new(entry.key().to_owned(), entry_span(entry)), span);
        let fields = match value.kind() {
            MergedValueKind::Null(_) => return Some(volume),
            MergedValueKind::Mapping(fields) => fields,
            _ => {
                self.expected(value, "volume definition must be a mapping or null");
                return None;
            }
        };
        let path = ["volumes".to_owned(), entry.key().to_owned()];
        for field in fields {
            match field.key() {
                "driver" => self
                    .located_string(field.value(), "volume driver must be a scalar")
                    .into_iter()
                    .for_each(|value| volume.set_driver(value)),
                "driver_opts" => self
                    .key_value_mapping(field.value(), "volume driver_opts must be a mapping")
                    .into_iter()
                    .for_each(|value| volume.set_driver_opts(value)),
                "external" => self
                    .located_boolean(field.value(), "volume external must be a boolean")
                    .into_iter()
                    .for_each(|value| volume.set_external(value)),
                "labels" => self
                    .labels(field.value())
                    .into_iter()
                    .for_each(|value| volume.set_labels(value)),
                "name" => self
                    .located_string(field.value(), "volume custom name must be a scalar")
                    .into_iter()
                    .for_each(|value| volume.set_custom_name(value)),
                _ => self.record_root_unmodeled(&path, field),
            }
        }
        Some(volume)
    }

    fn config_definitions(&mut self, value: &MergedValue) -> Vec<ProjectResource<ConfigDefinition>> {
        let Some(entries) = self.mapping(value, "top-level configs must be a mapping") else {
            return Vec::new();
        };
        entries
            .iter()
            .filter_map(|entry| {
                let definition = self.config_definition(entry)?;
                Some(ProjectResource {
                    name: ProjectKey::from_entry(entry),
                    definition: ProjectValue::new(definition, entry.value()),
                })
            })
            .collect()
    }

    fn config_definition(&mut self, entry: &MergedEntry) -> Option<ConfigDefinition> {
        let value = entry.value();
        let span = effective_span(value);
        let mut config = ConfigDefinition::new(Located::new(entry.key().to_owned(), entry_span(entry)), span);
        let fields = match value.kind() {
            MergedValueKind::Null(_) => return Some(config),
            MergedValueKind::Mapping(fields) => fields,
            _ => {
                self.expected(value, "config definition must be a mapping or null");
                return None;
            }
        };
        let path = ["configs".to_owned(), entry.key().to_owned()];
        for field in fields {
            match field.key() {
                "file" => self
                    .located_string(field.value(), "config file must be a scalar")
                    .into_iter()
                    .for_each(|value| config.set_file(value)),
                "environment" => self
                    .located_string(field.value(), "config environment must be a scalar")
                    .into_iter()
                    .for_each(|value| config.set_environment(value)),
                "content" => self
                    .located_string(field.value(), "config content must be a scalar")
                    .into_iter()
                    .for_each(|value| config.set_content(value)),
                "external" => self
                    .located_boolean(field.value(), "config external must be a boolean")
                    .into_iter()
                    .for_each(|value| config.set_external(value)),
                "name" => self
                    .located_string(field.value(), "config custom name must be a scalar")
                    .into_iter()
                    .for_each(|value| config.set_custom_name(value)),
                _ => self.record_root_unmodeled(&path, field),
            }
        }
        Some(config)
    }

    fn secret_definitions(&mut self, value: &MergedValue) -> Vec<ProjectResource<SecretDefinition>> {
        let Some(entries) = self.mapping(value, "top-level secrets must be a mapping") else {
            return Vec::new();
        };
        entries
            .iter()
            .filter_map(|entry| {
                let definition = self.secret_definition(entry)?;
                Some(ProjectResource {
                    name: ProjectKey::from_entry(entry),
                    definition: ProjectValue::new(definition, entry.value()),
                })
            })
            .collect()
    }

    fn secret_definition(&mut self, entry: &MergedEntry) -> Option<SecretDefinition> {
        let value = entry.value();
        let span = effective_span(value);
        let mut secret = SecretDefinition::new(Located::new(entry.key().to_owned(), entry_span(entry)), span);
        let fields = match value.kind() {
            MergedValueKind::Null(_) => return Some(secret),
            MergedValueKind::Mapping(fields) => fields,
            _ => {
                self.expected(value, "secret definition must be a mapping or null");
                return None;
            }
        };
        let path = ["secrets".to_owned(), entry.key().to_owned()];
        for field in fields {
            match field.key() {
                "file" => self
                    .located_string(field.value(), "secret file must be a scalar")
                    .into_iter()
                    .for_each(|value| secret.set_file(value)),
                "environment" => self
                    .located_string(field.value(), "secret environment must be a scalar")
                    .into_iter()
                    .for_each(|value| secret.set_environment(value)),
                "external" => self
                    .located_boolean(field.value(), "secret external must be a boolean")
                    .into_iter()
                    .for_each(|value| secret.set_external(value)),
                "name" => self
                    .located_string(field.value(), "secret custom name must be a scalar")
                    .into_iter()
                    .for_each(|value| secret.set_custom_name(value)),
                _ => self.record_root_unmodeled(&path, field),
            }
        }
        Some(secret)
    }

    fn labels(&mut self, value: &MergedValue) -> Option<Labels> {
        let span = effective_span(value);
        match value.kind() {
            MergedValueKind::Sequence(_) => self
                .located_string_sequence(value, "labels must be a scalar sequence")
                .map(|values| Labels::List { span, values }),
            MergedValueKind::Mapping(_) => self
                .key_value_mapping(value, "labels must be a scalar mapping")
                .map(|entries| Labels::Map { span, entries }),
            _ => {
                self.expected(value, "labels must be a sequence or mapping");
                None
            }
        }
    }
}

fn field_reference(path: &[String], entry: &MergedEntry) -> ProjectFieldReference {
    let mut complete_path = path.to_vec();
    complete_path.push(entry.key().to_owned());
    ProjectFieldReference {
        path: complete_path,
        key: ProjectKey::from_entry(entry),
        provenance: entry.value().provenance().clone(),
        extension: entry.key().starts_with("x-"),
        sensitive: entry.value().is_sensitive(),
    }
}

fn effective_span(value: &MergedValue) -> SourceSpan {
    value
        .provenance()
        .effective_source()
        .or_else(|| value.provenance().sources().first().copied())
        .unwrap_or_else(|| SourceSpan::from_valid_offsets(SourceId::new(0), 0, 0))
}

fn entry_span(entry: &MergedEntry) -> SourceSpan {
    entry
        .key_sources()
        .last()
        .copied()
        .or_else(|| entry.key_sources().first().copied())
        .unwrap_or_else(|| effective_span(entry.value()))
}
