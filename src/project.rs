//! Source-aware native values from a merged and optionally profile-selected Compose project.

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::merge::{
    EntrySyntax, MergeProvenance, MergedEntry, MergedProject, MergedScalarKind, MergedValue, MergedValueKind,
};
use crate::model::{
    ANNOTATIONS_DUPLICATE_NAME, ANNOTATIONS_EMPTY_NAME, ANNOTATIONS_EXPECTED_STRING, ANNOTATIONS_KEY_ONLY,
    BUILD_DOCKERFILE_INLINE_CONFLICT, BUILD_NO_CACHE_FILTER_DUPLICATE_ITEM, BindOptions, BooleanValue, BuildNoCache,
    BuildProvenance, BuildSbom, CAP_ADD_DUPLICATE_ITEM, CAP_DROP_DUPLICATE_ITEM, CapabilityAddItem, CapabilityDropItem,
    Command, ComposeScalar, ConfigDefinition, DEPLOY_ENDPOINT_MODE_PORTABILITY, DEPLOY_MODE_PORTABILITY,
    DEVICE_EXPECTED_FORM, DEVICE_EXPECTED_STRING, DNS_EXPECTED_FORM, DNS_EXPECTED_STRING, DNS_OPT_DUPLICATE_ITEM,
    DNS_OPT_EXPECTED_SEQUENCE, DNS_OPT_EXPECTED_STRING, DNS_SEARCH_DUPLICATE_ITEM, DNS_SEARCH_EXPECTED_FORM,
    DNS_SEARCH_EXPECTED_STRING, DependencyCondition, DeployEndpointMode, DeployMode, DeployPlacementMaxReplicasPerNode,
    DeployReplicas, DeployRestartCondition, DeployRestartDuration, DeployRestartMaxAttempts, EXPOSE_DUPLICATE_ITEM,
    EXPOSE_EXPECTED_SCALAR, EXPOSE_EXPECTED_SEQUENCE, EXPOSE_INVALID_ITEM, EXPOSE_PROVIDER_DEPENDENT, Entrypoint,
    EnvironmentFileFormat, EnvironmentFileFormatKind, ExposeItemKind, ExposeScalarKind, HealthcheckDuration,
    HealthcheckRetries, HealthcheckTest, HealthcheckTestKind, HostAddress, Hostname, HostnameKind, ImageReference,
    Ipam, IpamConfig, KeyValueEntry, LOGGING_DRIVER_EXPECTED_STRING, LOGGING_EXPECTED_MAPPING,
    LOGGING_OPTION_EMPTY_KEY, LOGGING_OPTION_EXPECTED_SCALAR, LOGGING_OPTIONS_EXPECTED_MAPPING, Labels, LimitValue,
    Located, LongPort, LongVolumeMount, MEM_LIMIT_AMBIGUOUS_ZERO, MEM_LIMIT_EXPECTED_VALUE,
    MEM_LIMIT_PROVIDER_DEPENDENT_STRING, MEM_LIMIT_SCHEMA_NUMBER, MemLimit, MemLimitKind, MemLimitScalarKind,
    MountType, NetworkDefinition, PIDS_LIMIT_AMBIGUOUS_ZERO, PidsLimit, PidsLimitKind, Port, PullPolicy, RestartPolicy,
    SECURITY_OPT_APPARMOR_CONFLICT, SECURITY_OPT_APPARMOR_NEAR_MISS, SECURITY_OPT_EMPTY_ITEM,
    SECURITY_OPT_EXPECTED_SEQUENCE, SECURITY_OPT_EXPECTED_STRING, SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT,
    SECURITY_OPT_NO_NEW_PRIVILEGES_NEAR_MISS, SECURITY_OPT_SECCOMP_CONFLICT, SECURITY_OPT_SECCOMP_NEAR_MISS,
    SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT, SECURITY_OPT_SECURITY_LABEL_DISABLE_NEAR_MISS,
    SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT, SECURITY_OPT_SECURITY_LABEL_FILETYPE_NEAR_MISS,
    SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT, SECURITY_OPT_SECURITY_LABEL_LEVEL_NEAR_MISS,
    SECURITY_OPT_SECURITY_LABEL_NESTED_CONFLICT, SECURITY_OPT_SECURITY_LABEL_NESTED_NEAR_MISS,
    SECURITY_OPT_SECURITY_LABEL_TYPE_CONFLICT, SECURITY_OPT_SECURITY_LABEL_TYPE_NEAR_MISS, SHM_SIZE_AMBIGUOUS_ZERO,
    SHM_SIZE_EXPECTED_VALUE, SHM_SIZE_PROVIDER_DEPENDENT_NUMBER, SHM_SIZE_PROVIDER_DEPENDENT_STRING,
    SYSCTLS_DUPLICATE_ITEM, SYSCTLS_EMPTY_KEY, SYSCTLS_EXPECTED_FORM, SYSCTLS_EXPECTED_SCALAR, SYSCTLS_EXPECTED_STRING,
    SecretDefinition, SecurityOptionCandidateCounts, SecurityOptionKind, SelinuxRelabel, ServiceNetwork,
    ServiceNetworks, ShmSize, ShmSizeKind, ShmSizeScalarKind, ShortDevice, ShortExtraHost, ShortPort, ShortVolumeMount,
    StopGracePeriod, TMPFS_EXPECTED_FORM, TMPFS_EXPECTED_STRING, TMPFS_PROVIDER_DEPENDENT, TmpfsItem, TmpfsItemKind,
    ULIMIT_INVALID_NAME, ULIMIT_INVALID_VALUE, ULIMIT_MISSING_RANGE_MEMBER, UserNamespaceMode, UserSpec,
    VOLUME_EXTERNAL_DRIVER_CONFIGURATION, VOLUME_EXTERNAL_LABELS_CONFIGURATION, VolumeDefinition, VolumeMount,
    classify_expose_item, classify_security_option, security_path_option_diagnostic, valid_ulimit_name,
};
use crate::profiles::ProfileSelection;
use crate::resolution::{SELECTION_PROJECT_MISMATCH, service_in_scope};
use crate::source::{SourceId, SourceSpan};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

    fn new_sensitive(value: T, source: &MergedValue) -> Self {
        Self {
            value,
            provenance: source.provenance().clone(),
            sensitive: true,
        }
    }

    /// Returns the typed effective value.
    /// Returns effective no-cache filter form with collection provenance.
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

    fn from_sensitive_entry(entry: &MergedEntry) -> Self {
        Self {
            value: entry.key().to_owned(),
            sources: entry.key_sources().to_vec(),
            sensitive: true,
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

/// One effective service environment-file entry with syntax and item provenance retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectEnvironmentFile {
    /// Scalar path syntax.
    Short(String),
    /// Mapping syntax with field-level provenance.
    Long(Box<ProjectLongEnvironmentFile>),
}

/// Effective long-syntax service environment-file options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLongEnvironmentFile {
    path: Option<ProjectValue<String>>,
    required: Option<ProjectValue<BooleanValue>>,
    format: Option<ProjectValue<EnvironmentFileFormat>>,
    unmodeled_fields: Vec<ProjectFieldReference>,
}

impl ProjectLongEnvironmentFile {
    /// Returns the required environment-file path.
    #[must_use]
    pub const fn path(&self) -> Option<&ProjectValue<String>> {
        self.path.as_ref()
    }

    /// Returns the explicit required-file choice; absence means Compose's default `true`.
    #[must_use]
    pub const fn required(&self) -> Option<&ProjectValue<BooleanValue>> {
        self.required.as_ref()
    }

    /// Returns the explicit parser format; absence means Compose's default format.
    #[must_use]
    pub const fn format(&self) -> Option<&ProjectValue<EnvironmentFileFormat>> {
        self.format.as_ref()
    }

    /// Returns retained long-form fields outside the typed project-view boundary.
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[ProjectFieldReference] {
        &self.unmodeled_fields
    }
}

/// One effective service or deployment metadata label after field-specific multi-file merging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLabelEntry {
    name: ProjectKey,
    value: ProjectValue<ComposeScalar>,
    syntax: EntrySyntax,
}

impl ProjectLabelEntry {
    /// Returns the label name and its contributing key spans.
    #[must_use]
    pub const fn name(&self) -> &ProjectKey {
        &self.name
    }

    /// Returns the effective label scalar.
    ///
    /// A key-only list entry has an explicit empty-string value while retaining
    /// [`EntrySyntax::ListKeyOnly`] as its authored form.
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

/// An effective service or deployment label view retaining collection and entry syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLabels {
    form: ProjectLabelsForm,
    entries: Vec<ProjectLabelEntry>,
}

/// The effective syntax form of a service or deployment label collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectLabelsForm {
    /// Mapping syntax with scalar or null values.
    Map,
    /// Ordered scalar list syntax.
    List,
}

/// Effective build labels retaining their mapping or list syntax.
///
/// Mapping entries use the same key/value representation as service labels. List entries remain
/// raw strings so order, duplicates, and bare-label spelling are not normalized away.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectBuildLabels {
    /// Mapping syntax with keyed label entries.
    Map(Vec<ProjectLabelEntry>),
    /// List syntax with raw ordered label strings.
    List(Vec<ProjectValue<String>>),
}

/// One effective mapping-form build argument with key and scalar provenance retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBuildArgEntry {
    name: ProjectKey,
    value: ProjectValue<ComposeScalar>,
}

impl ProjectBuildArgEntry {
    /// Returns the argument name and every contributing key location.
    #[must_use]
    pub const fn name(&self) -> &ProjectKey {
        &self.name
    }

    /// Returns the effective string, number, boolean, or null value with merge provenance.
    #[must_use]
    pub const fn value(&self) -> &ProjectValue<ComposeScalar> {
        &self.value
    }
}

/// Effective build arguments retaining their mapping or list syntax.
///
/// Mapping entries merge by exact argument name. List entries remain raw ordered strings, so
/// duplicates and bare argument names are not normalized or resolved from the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectBuildArgs {
    /// Mapping syntax with keyed argument entries.
    Map(Vec<ProjectBuildArgEntry>),
    /// List syntax with raw ordered argument strings.
    List(Vec<ProjectValue<String>>),
}

/// One effective mapping-form additional build context with key and scalar provenance retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBuildAdditionalContextEntry {
    name: ProjectKey,
    value: ProjectValue<ComposeScalar>,
}

impl ProjectBuildAdditionalContextEntry {
    /// Returns the context name and every contributing key location.
    #[must_use]
    pub const fn name(&self) -> &ProjectKey {
        &self.name
    }

    /// Returns the effective scalar context value with merge provenance.
    #[must_use]
    pub const fn value(&self) -> &ProjectValue<ComposeScalar> {
        &self.value
    }
}

/// Effective additional build contexts retaining their mapping or list syntax.
///
/// List entries remain raw ordered strings, including duplicates and `NAME=VALUE` spelling.
/// Mapping entries merge by exact key. Neither form interprets context names, paths, URLs,
/// images, service schemes, or builder behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectBuildAdditionalContexts {
    /// Mapping syntax with keyed scalar entries.
    Map(Vec<ProjectBuildAdditionalContextEntry>),
    /// List syntax with raw ordered entries.
    List(Vec<ProjectValue<String>>),
}

/// One effective mapping-form build host entry with complete key and address provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBuildExtraHostEntry {
    hostname: ProjectKey,
    addresses: ProjectBuildExtraHostAddresses,
}

impl ProjectBuildExtraHostEntry {
    /// Returns the raw hostname key and every contributing key span.
    #[must_use]
    pub const fn hostname(&self) -> &ProjectKey {
        &self.hostname
    }

    /// Returns the scalar or ordered-list address form with nested provenance.
    #[must_use]
    pub const fn addresses(&self) -> &ProjectBuildExtraHostAddresses {
        &self.addresses
    }
}

/// Effective build-host addresses for one mapping key.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectBuildExtraHostAddresses {
    /// One raw string address.
    Scalar(ProjectValue<String>),
    /// Ordered raw string addresses.
    List(Vec<ProjectValue<String>>),
}

impl ProjectBuildExtraHostAddresses {
    /// Returns the scalar address when that effective form is present.
    #[must_use]
    pub const fn as_scalar(&self) -> Option<&ProjectValue<String>> {
        match self {
            Self::Scalar(value) => Some(value),
            Self::List(_) => None,
        }
    }

    /// Returns the ordered address list when that effective form is present.
    #[must_use]
    pub fn as_list(&self) -> Option<&[ProjectValue<String>]> {
        let Self::List(values) = self else {
            return None;
        };
        Some(values)
    }
}

/// Effective build-time host mappings, intentionally distinct from service [`ProjectExtraHosts`].
///
/// List entries remain raw ordered strings, including duplicates. Mapping entries retain raw
/// hostname keys and scalar/list address syntax without applying service-host parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectBuildExtraHosts {
    /// Ordered raw string entries.
    List(Vec<ProjectValue<String>>),
    /// Ordered hostname keys with scalar or list addresses.
    Map(Vec<ProjectBuildExtraHostEntry>),
}

impl ProjectBuildExtraHosts {
    /// Returns ordered raw entries when the effective form is a list.
    #[must_use]
    pub fn as_list(&self) -> Option<&[ProjectValue<String>]> {
        let Self::List(entries) = self else {
            return None;
        };
        Some(entries)
    }

    /// Returns ordered hostname entries when the effective form is a mapping.
    #[must_use]
    pub fn as_map(&self) -> Option<&[ProjectBuildExtraHostEntry]> {
        let Self::Map(entries) = self else {
            return None;
        };
        Some(entries)
    }
}

/// One effective mapping-form `BuildKit` SSH grant with sensitive key and value provenance.
#[derive(Clone, PartialEq, Eq)]
pub struct ProjectBuildSshEntry {
    name: ProjectKey,
    value: ProjectValue<ComposeScalar>,
}

impl fmt::Debug for ProjectBuildSshEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProjectBuildSshEntry")
            .field("name", &"<redacted>")
            .field("value", &"<redacted>")
            .finish()
    }
}

impl ProjectBuildSshEntry {
    /// Explicitly returns the raw SSH grant name and its key provenance.
    #[must_use]
    pub const fn name(&self) -> &ProjectKey {
        &self.name
    }

    /// Explicitly returns the raw SSH grant scalar and its merge provenance.
    #[must_use]
    pub const fn value(&self) -> &ProjectValue<ComposeScalar> {
        &self.value
    }
}

/// Effective sensitive `BuildKit` SSH grants retaining mapping or list syntax.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectBuildSsh {
    /// Mapping syntax with sensitive keyed scalar entries.
    Map(Vec<ProjectBuildSshEntry>),
    /// List syntax with sensitive raw ordered grant strings.
    List(Vec<ProjectValue<String>>),
}

impl fmt::Debug for ProjectBuildSsh {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let form = match self {
            Self::Map(_) => "Map",
            Self::List(_) => "List",
        };
        formatter
            .debug_struct("ProjectBuildSsh")
            .field("form", &form)
            .field("entries", &"<redacted>")
            .finish()
    }
}

impl ProjectBuildSsh {
    /// Explicitly returns raw mapping-form SSH grants.
    #[must_use]
    pub fn as_map(&self) -> Option<&[ProjectBuildSshEntry]> {
        let Self::Map(entries) = self else {
            return None;
        };
        Some(entries)
    }

    /// Explicitly returns raw list-form SSH grants.
    #[must_use]
    pub fn as_list(&self) -> Option<&[ProjectValue<String>]> {
        let Self::List(entries) = self else {
            return None;
        };
        Some(entries)
    }
}

impl ProjectBuildAdditionalContexts {
    /// Returns mapping entries when the effective additional contexts use mapping syntax.
    #[must_use]
    pub fn as_map(&self) -> Option<&[ProjectBuildAdditionalContextEntry]> {
        let Self::Map(entries) = self else {
            return None;
        };
        Some(entries)
    }

    /// Returns raw ordered entries when the effective additional contexts use list syntax.
    #[must_use]
    pub fn as_list(&self) -> Option<&[ProjectValue<String>]> {
        let Self::List(entries) = self else {
            return None;
        };
        Some(entries)
    }
}

impl ProjectBuildArgs {
    /// Returns mapping entries when the effective build arguments use mapping syntax.
    #[must_use]
    pub fn as_map(&self) -> Option<&[ProjectBuildArgEntry]> {
        let Self::Map(entries) = self else {
            return None;
        };
        Some(entries)
    }

    /// Returns raw ordered entries when the effective build arguments use list syntax.
    #[must_use]
    pub fn as_list(&self) -> Option<&[ProjectValue<String>]> {
        let Self::List(entries) = self else {
            return None;
        };
        Some(entries)
    }
}

impl ProjectBuildLabels {
    /// Returns mapping entries when the effective build labels use mapping syntax.
    #[must_use]
    pub fn as_map(&self) -> Option<&[ProjectLabelEntry]> {
        let Self::Map(entries) = self else {
            return None;
        };
        Some(entries)
    }

    /// Returns raw ordered entries when the effective build labels use list syntax.
    #[must_use]
    pub fn as_list(&self) -> Option<&[ProjectValue<String>]> {
        let Self::List(entries) = self else {
            return None;
        };
        Some(entries)
    }
}

/// One raw/effective annotation scalar without erasing authored spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAnnotationScalar {
    authored: String,
    effective: ComposeScalar,
}

impl ProjectAnnotationScalar {
    /// Returns the exact authored scalar spelling retained by the merge layer.
    #[must_use]
    pub fn authored(&self) -> &str {
        &self.authored
    }

    /// Returns the effective scalar after optional per-file interpolation.
    #[must_use]
    pub const fn effective(&self) -> &ComposeScalar {
        &self.effective
    }
}

/// One effective service annotation after keyed merge and duplicate replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAnnotationEntry {
    name: ProjectKey,
    value: Option<ProjectValue<ProjectAnnotationScalar>>,
    raw_list_item: Option<ProjectValue<ProjectAnnotationScalar>>,
    syntax: EntrySyntax,
    contributors: Vec<MergeProvenance>,
}

impl ProjectAnnotationEntry {
    /// Returns the effective annotation name and every contributing key/item location.
    #[must_use]
    pub const fn name(&self) -> &ProjectKey {
        &self.name
    }

    /// Returns the explicit effective annotation value.
    ///
    /// A key-only list item returns `None`; it is diagnosed and never coerced to an empty string.
    #[must_use]
    pub const fn value(&self) -> Option<&ProjectValue<ProjectAnnotationScalar>> {
        self.value.as_ref()
    }

    /// Returns the complete raw list scalar when list syntax supplied the effective entry.
    #[must_use]
    pub const fn raw_list_item(&self) -> Option<&ProjectValue<ProjectAnnotationScalar>> {
        self.raw_list_item.as_ref()
    }

    /// Returns the most recent mapping or list syntax contributing this entry.
    #[must_use]
    pub const fn syntax(&self) -> EntrySyntax {
        self.syntax
    }

    /// Returns every replaced contributor's merge provenance in authored order.
    #[must_use]
    pub fn contributors(&self) -> &[MergeProvenance] {
        &self.contributors
    }
}

/// Effective service annotations keyed by semantic name with authored evidence retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAnnotations {
    entries: Vec<ProjectAnnotationEntry>,
}

impl ProjectAnnotations {
    /// Returns effective annotations in first-key order.
    #[must_use]
    pub fn entries(&self) -> &[ProjectAnnotationEntry] {
        &self.entries
    }

    /// Finds an effective annotation by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ProjectAnnotationEntry> {
        self.entries.iter().find(|entry| entry.name.value == name)
    }
}

impl ProjectLabels {
    /// Returns whether the effective collection uses mapping or list syntax.
    #[must_use]
    pub const fn form(&self) -> ProjectLabelsForm {
        self.form
    }

    /// Returns labels in effective merge order.
    #[must_use]
    pub fn entries(&self) -> &[ProjectLabelEntry] {
        &self.entries
    }

    /// Finds an effective label by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ProjectLabelEntry> {
        self.entries.iter().find(|entry| entry.name.value == name)
    }
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

/// Effective long-form service config or secret grant with field-level merge provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLongGrant {
    source: Option<ProjectValue<String>>,
    target: Option<ProjectValue<String>>,
    uid: Option<ProjectValue<String>>,
    gid: Option<ProjectValue<String>>,
    mode: Option<ProjectValue<String>>,
    unmodeled_fields: Vec<ProjectFieldReference>,
}

impl ProjectLongGrant {
    /// Returns the referenced top-level resource name.
    #[must_use]
    pub const fn source(&self) -> Option<&ProjectValue<String>> {
        self.source.as_ref()
    }

    /// Returns the requested container path or name.
    #[must_use]
    pub const fn target(&self) -> Option<&ProjectValue<String>> {
        self.target.as_ref()
    }

    /// Returns the requested container user-ID spelling.
    #[must_use]
    pub const fn uid(&self) -> Option<&ProjectValue<String>> {
        self.uid.as_ref()
    }

    /// Returns the requested container group-ID spelling.
    #[must_use]
    pub const fn gid(&self) -> Option<&ProjectValue<String>> {
        self.gid.as_ref()
    }

    /// Returns the requested permission-mode spelling.
    #[must_use]
    pub const fn mode(&self) -> Option<&ProjectValue<String>> {
        self.mode.as_ref()
    }

    /// Returns retained long-form fields outside the typed project-view boundary.
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[ProjectFieldReference] {
        &self.unmodeled_fields
    }
}

/// One effective service config or secret grant with its Compose syntax form retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectGrant {
    /// Resource-name short syntax.
    Short(String),
    /// Mapping-based long syntax.
    Long(Box<ProjectLongGrant>),
}

/// Effective long-form service device with nested merge provenance retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLongDevice {
    source: Option<ProjectValue<String>>,
    target: Option<ProjectValue<String>>,
    permissions: Option<ProjectValue<String>>,
    extension_fields: Vec<ProjectFieldReference>,
    unknown_fields: Vec<ProjectFieldReference>,
}

impl ProjectLongDevice {
    /// Returns the required raw source when it was valid and present.
    #[must_use]
    pub const fn source(&self) -> Option<&ProjectValue<String>> {
        self.source.as_ref()
    }

    /// Returns the optional raw target without path interpretation.
    #[must_use]
    pub const fn target(&self) -> Option<&ProjectValue<String>> {
        self.target.as_ref()
    }

    /// Returns the optional raw permissions string without validating runtime meaning.
    #[must_use]
    pub const fn permissions(&self) -> Option<&ProjectValue<String>> {
        self.permissions.as_ref()
    }

    /// Returns retained `x-` options with their complete source evidence.
    #[must_use]
    pub fn extension_fields(&self) -> &[ProjectFieldReference] {
        &self.extension_fields
    }

    /// Returns unrecognized long-form options with their complete source evidence.
    #[must_use]
    pub fn unknown_fields(&self) -> &[ProjectFieldReference] {
        &self.unknown_fields
    }
}

/// One effective service device with short and long syntax kept distinct.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectDevice {
    /// A raw short scalar, including path, CDI, deferred, and opaque spellings.
    Short(ShortDevice),
    /// A mapping-form device whose nested values retain their own provenance.
    Long(ProjectLongDevice),
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

/// Effective service-level `tmpfs` syntax with per-item merge provenance retained.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectTmpfs {
    /// One effective scalar declaration.
    Scalar(ProjectValue<TmpfsItem>),
    /// One effective list, including an explicit empty or reset list.
    List(Vec<ProjectValue<TmpfsItem>>),
}

/// Effective service `dns` syntax with collection and per-item merge provenance retained.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectDns {
    /// One effective raw scalar server string.
    Scalar(ProjectValue<String>),
    /// One effective ordered list, including an explicit empty or reset list.
    List(Vec<ProjectValue<String>>),
}

/// Effective service `dns_search` syntax with collection and per-item merge provenance retained.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectDnsSearch {
    /// One effective raw scalar search-domain string.
    Scalar(ProjectValue<String>),
    /// One effective ordered list, including an explicit empty or reset list.
    List(Vec<ProjectValue<String>>),
}

/// One effective service `expose` scalar with authored spelling and YAML kind retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectExposeItem {
    authored: String,
    value: String,
    scalar_kind: ExposeScalarKind,
    kind: ExposeItemKind,
}

/// One effective raw service security option with authored and interpolated spelling retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSecurityOptionItem {
    authored: String,
    value: String,
    scalar_kind: MergedScalarKind,
    kind: SecurityOptionKind,
}

impl ProjectSecurityOptionItem {
    /// Returns the exact scalar spelling before optional interpolation.
    #[must_use]
    pub fn authored(&self) -> &str {
        &self.authored
    }

    /// Returns the effective scalar spelling after optional interpolation.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the retained YAML scalar category.
    #[must_use]
    pub const fn scalar_kind(&self) -> MergedScalarKind {
        self.scalar_kind
    }

    /// Returns the narrow classification of the effective spelling.
    #[must_use]
    pub const fn kind(&self) -> &SecurityOptionKind {
        &self.kind
    }
}

impl ProjectExposeItem {
    /// Returns the exact scalar spelling before optional interpolation.
    #[must_use]
    pub fn authored(&self) -> &str {
        &self.authored
    }

    /// Returns the exact effective scalar spelling after optional interpolation.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns whether the YAML scalar was authored as a string or number.
    #[must_use]
    pub const fn scalar_kind(&self) -> ExposeScalarKind {
        self.scalar_kind
    }

    /// Returns the conservative classification of the effective spelling.
    #[must_use]
    pub const fn kind(&self) -> &ExposeItemKind {
        &self.kind
    }
}

/// One effective ulimit scalar with authored spelling and YAML scalar kind retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectUlimitScalar {
    authored: String,
    value: LimitValue,
    kind: MergedScalarKind,
}

impl ProjectUlimitScalar {
    /// Returns the exact authored scalar spelling before optional interpolation.
    #[must_use]
    pub fn authored(&self) -> &str {
        &self.authored
    }

    /// Returns the classified effective spelling after optional interpolation.
    #[must_use]
    pub const fn value(&self) -> &LimitValue {
        &self.value
    }

    /// Returns whether the authored YAML scalar was a string or number.
    #[must_use]
    pub const fn kind(&self) -> MergedScalarKind {
        self.kind
    }
}

/// Effective long-syntax ulimit members with independent merge provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectUlimitRange {
    soft: Option<ProjectValue<ProjectUlimitScalar>>,
    hard: Option<ProjectValue<ProjectUlimitScalar>>,
    unmodeled_fields: Vec<ProjectFieldReference>,
}

impl ProjectUlimitRange {
    /// Returns the effective soft limit, or `None` when the required member was omitted or malformed.
    #[must_use]
    pub const fn soft(&self) -> Option<&ProjectValue<ProjectUlimitScalar>> {
        self.soft.as_ref()
    }

    /// Returns the effective hard limit, or `None` when the required member was omitted or malformed.
    #[must_use]
    pub const fn hard(&self) -> Option<&ProjectValue<ProjectUlimitScalar>> {
        self.hard.as_ref()
    }

    /// Returns retained range fields outside the `soft` and `hard` boundary.
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[ProjectFieldReference] {
        &self.unmodeled_fields
    }
}

/// The effective single or soft/hard form of one named ulimit.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectUlimitValue {
    /// One scalar applies to both the soft and hard limit.
    Single(ProjectValue<ProjectUlimitScalar>),
    /// Soft and hard members remain independently source-aware.
    Range(ProjectUlimitRange),
}

/// One ordered effective named ulimit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectUlimit {
    name: ProjectKey,
    value: ProjectUlimitValue,
}

impl ProjectUlimit {
    /// Returns the lowercase limit name and every authored key location.
    #[must_use]
    pub const fn name(&self) -> &ProjectKey {
        &self.name
    }

    /// Returns the effective single or soft/hard form.
    #[must_use]
    pub const fn value(&self) -> &ProjectUlimitValue {
        &self.value
    }
}

/// Effective service `ulimits`, including an explicitly empty or reset mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectUlimits {
    entries: Vec<ProjectValue<ProjectUlimit>>,
}

impl ProjectUlimits {
    /// Returns named limits in effective mapping order.
    #[must_use]
    pub fn entries(&self) -> &[ProjectValue<ProjectUlimit>] {
        &self.entries
    }

    /// Reports whether the effective mapping is explicitly empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// One effective mapping-form service sysctl with key and scalar-value provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSysctl {
    name: ProjectKey,
    value: ProjectValue<ComposeScalar>,
}

impl ProjectSysctl {
    /// Returns the exact sysctl name and every authored key location.
    #[must_use]
    pub const fn name(&self) -> &ProjectKey {
        &self.name
    }

    /// Returns the exact scalar kind and spelling with complete merge provenance.
    #[must_use]
    pub const fn value(&self) -> &ProjectValue<ComposeScalar> {
        &self.value
    }
}

/// Effective service `sysctls` with mapping/list form and per-entry provenance retained.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectSysctls {
    /// Ordered mapping entries merged by exact key.
    Map(Vec<ProjectValue<ProjectSysctl>>),
    /// Ordered list items appended without implicit deduplication.
    List(Vec<ProjectValue<String>>),
}

/// One effective logging-option scalar with authored and interpolated spelling retained.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectLoggingOptionValue {
    /// A YAML string scalar before and after optional per-file interpolation.
    String {
        /// Exact authored string before interpolation.
        authored: String,
        /// Effective string after interpolation.
        value: String,
    },
    /// A YAML number scalar with exact spelling retained.
    Number(String),
    /// An explicit or empty YAML null.
    Null,
}

/// One ordered effective logging option with complete key and value provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLoggingOption {
    name: ProjectKey,
    value: ProjectValue<ProjectLoggingOptionValue>,
}

impl ProjectLoggingOption {
    /// Returns the non-empty option name and every authored key location.
    #[must_use]
    pub const fn name(&self) -> &ProjectKey {
        &self.name
    }

    /// Returns the exact string, number, or null option value with merge provenance.
    #[must_use]
    pub const fn value(&self) -> &ProjectValue<ProjectLoggingOptionValue> {
        &self.value
    }
}

/// Effective ordered logging options, including an explicitly empty or reset mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLoggingOptions {
    entries: Vec<ProjectValue<ProjectLoggingOption>>,
    unmodeled_entries: Vec<ProjectFieldReference>,
}

impl ProjectLoggingOptions {
    /// Returns valid option entries in effective mapping order.
    #[must_use]
    pub fn entries(&self) -> &[ProjectValue<ProjectLoggingOption>] {
        &self.entries
    }

    /// Returns malformed entries retained outside the typed string/number/null boundary.
    #[must_use]
    pub fn unmodeled_entries(&self) -> &[ProjectFieldReference] {
        &self.unmodeled_entries
    }

    /// Reports whether the effective options mapping has no valid or malformed entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.unmodeled_entries.is_empty()
    }
}

/// Effective service logging configuration with nested merge provenance retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLogging {
    driver: Option<ProjectValue<String>>,
    options: Option<ProjectValue<ProjectLoggingOptions>>,
    unmodeled_fields: Vec<ProjectFieldReference>,
}

/// An effective Compose build declaration retaining scalar and mapping syntax.
///
/// Only `context`, `args`, `cache_from`, `cache_to`, `dockerfile`, `dockerfile_inline`, `entitlements`, `extra_hosts`, `target`, `network`, `isolation`, `platforms`, `no_cache`,
/// `sbom`, `pull`, `shm_size`, `tags`, `labels`, `secrets`, and `ulimits` are promoted to native values in this slice.
/// Every other build subfield stays source-addressable as unmodeled evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectBuild {
    /// A short-syntax scalar build context.
    Context(ProjectValue<String>),
    /// A long-syntax build declaration.
    Definition(ProjectBuildDefinition),
}
/// Effective `build.no_cache_filter` scalar or list form.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectBuildNoCacheFilter {
    /// One exact scalar stage name.
    Scalar(ProjectValue<String>),
    /// Ordered exact stage names.
    List(Vec<ProjectValue<String>>),
}

/// Private grouping keeps the public Build declaration compact while retaining independent values.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectBuildCacheOptions {
    cache_from: Option<ProjectValue<Vec<ProjectValue<String>>>>,
    cache_to: Option<ProjectValue<Vec<ProjectValue<String>>>>,
    no_cache: Option<ProjectValue<BuildNoCache>>,
    pull: Option<ProjectValue<BooleanValue>>,
}

/// An effective long-syntax build declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBuildDefinition {
    additional_contexts: Option<Box<ProjectValue<ProjectBuildAdditionalContexts>>>,
    context: Option<Box<ProjectValue<String>>>,
    args: Option<Box<ProjectValue<ProjectBuildArgs>>>,
    entitlements: Option<Box<ProjectValue<Vec<ProjectValue<String>>>>>,
    extra_hosts: Option<Box<ProjectValue<ProjectBuildExtraHosts>>>,
    dockerfile: Option<Box<ProjectValue<String>>>,
    dockerfile_inline: Option<Box<ProjectValue<String>>>,
    target: Option<Box<ProjectValue<String>>>,
    network: Option<Box<ProjectValue<String>>>,
    isolation: Option<Box<ProjectValue<String>>>,
    platforms: Option<Box<ProjectValue<Vec<ProjectValue<String>>>>>,
    cache_options: Option<Box<ProjectBuildCacheOptions>>,
    no_cache_filter: Option<Box<ProjectValue<ProjectBuildNoCacheFilter>>>,
    privileged: Option<Box<ProjectValue<BooleanValue>>>,
    sbom: Option<Box<ProjectValue<BuildSbom>>>,
    provenance: Option<Box<ProjectValue<BuildProvenance>>>,
    shm_size: Option<Box<ProjectValue<ShmSize>>>,
    tags: Option<Box<ProjectValue<Vec<ProjectValue<String>>>>>,
    labels: Option<Box<ProjectValue<ProjectBuildLabels>>>,
    secrets: Option<Box<ProjectValue<Vec<ProjectValue<ProjectGrant>>>>>,
    ssh: Option<Box<ProjectValue<ProjectBuildSsh>>>,
    ulimits: Option<Box<ProjectValue<ProjectUlimits>>>,
    unmodeled_fields: Arc<Vec<ProjectFieldReference>>,
}

impl ProjectBuildDefinition {
    /// Returns effective additional build contexts without normalizing mapping and list syntax.
    ///
    /// The collection, list items, mapping keys, and mapping values each retain merge provenance
    /// and interpolation sensitivity. No builder or reference interpretation is performed.
    /// Returns effective no-cache filter form with collection provenance.
    #[must_use]
    pub fn additional_contexts(&self) -> Option<&ProjectValue<ProjectBuildAdditionalContexts>> {
        self.additional_contexts.as_deref()
    }

    /// Returns the effective long-syntax build context when it is a scalar.
    #[must_use]
    pub const fn context(&self) -> Option<&ProjectValue<String>> {
        match &self.context {
            Some(context) => Some(context),
            None => None,
        }
    }

    /// Returns effective build arguments without resolving list entries from an environment.
    ///
    /// Mapping entries retain exact-key replacement provenance and scalar kind. List entries
    /// retain append/reset/override provenance, ordering, duplicates, and bare forms.
    #[must_use]
    pub fn args(&self) -> Option<&ProjectValue<ProjectBuildArgs>> {
        self.args.as_deref()
    }

    /// Returns effective build entitlements in append order with per-item provenance.
    ///
    /// Explicit empty and reset sequences remain distinct from omission. Entries are opaque raw
    /// strings: this view retains duplicates and does not infer allowlists, privilege state,
    /// BuildKit/platform support, execution, or runtime effect.
    #[must_use]
    pub fn entitlements(&self) -> Option<&ProjectValue<Vec<ProjectValue<String>>>> {
        self.entitlements.as_deref()
    }

    /// Returns effective build-time host mappings without applying service-host parsing.
    ///
    /// Generic project merging appends list entries, recursively merges mapping keys and nested
    /// address lists, and replaces mixed forms. Collection, key, scalar, and nested-list-item
    /// provenance plus per-file interpolation sensitivity remain available. This view performs no
    /// address normalization or validation, DNS/host access, build generation, or conversion.
    #[must_use]
    pub fn extra_hosts(&self) -> Option<&ProjectValue<ProjectBuildExtraHosts>> {
        self.extra_hosts.as_deref()
    }

    /// Returns the effective long-syntax Dockerfile when it is a non-empty scalar.
    #[must_use]
    pub const fn dockerfile(&self) -> Option<&ProjectValue<String>> {
        match &self.dockerfile {
            Some(dockerfile) => Some(dockerfile),
            None => None,
        }
    }

    /// Returns the effective inline Dockerfile as an exact string scalar.
    ///
    /// Empty and multiline content remains distinct from omission. `ComposeLens` does not parse
    /// Containerfile syntax, resolve paths or contexts, scan content for secrets, build images,
    /// or infer Docker, `BuildKit`, or runtime behavior.
    #[must_use]
    pub const fn dockerfile_inline(&self) -> Option<&ProjectValue<String>> {
        match &self.dockerfile_inline {
            Some(dockerfile_inline) => Some(dockerfile_inline),
            None => None,
        }
    }

    /// Returns the effective long-syntax build target as an opaque scalar.
    ///
    /// An empty scalar remains an authored target; this view does not infer stage-name grammar.
    #[must_use]
    pub const fn target(&self) -> Option<&ProjectValue<String>> {
        match &self.target {
            Some(target) => Some(target),
            None => None,
        }
    }

    /// Returns the effective long-syntax build network as an opaque scalar.
    ///
    /// An empty scalar remains authored; this view does not infer network names, defaults, or
    /// runtime behavior.
    #[must_use]
    pub fn network(&self) -> Option<&ProjectValue<String>> {
        self.network.as_deref()
    }

    /// Returns the effective long-syntax build isolation as an opaque YAML string.
    ///
    /// The scalar is interpolated per file before merging and retains complete replacement,
    /// reset, or override provenance and sensitivity. This view does not validate isolation
    /// modes, platforms, defaults, privileges, or `BUILDAH_ISOLATION` behavior, and does not
    /// represent service-level `isolation`.
    #[must_use]
    pub fn isolation(&self) -> Option<&ProjectValue<String>> {
        self.isolation.as_deref()
    }

    /// Returns effective build platforms in append order with per-item provenance.
    ///
    /// An explicit empty or reset sequence remains distinct from omission. Platforms are raw
    /// scalars; this view does not parse OCI platform grammar or validate availability.
    #[must_use]
    pub fn platforms(&self) -> Option<&ProjectValue<Vec<ProjectValue<String>>>> {
        self.platforms.as_deref()
    }

    /// Returns effective external build-cache sources in append order with per-item provenance.
    ///
    /// An explicit empty or reset sequence remains distinct from omission. Entries are opaque
    /// strings: this view preserves duplicates and does not interpret cache type, reference,
    /// source, path, image, credentials, or builder behavior.
    #[must_use]
    pub fn cache_from(&self) -> Option<&ProjectValue<Vec<ProjectValue<String>>>> {
        self.cache_options
            .as_ref()
            .and_then(|options| options.cache_from.as_ref())
    }

    /// Returns effective external build-cache destinations in append order with per-item provenance.
    ///
    /// An explicit empty or reset sequence remains distinct from omission. Entries are opaque
    /// strings: this view preserves duplicates and does not interpret cache type, reference,
    /// destination, path, image, credentials, or builder behavior.
    #[must_use]
    pub fn cache_to(&self) -> Option<&ProjectValue<Vec<ProjectValue<String>>>> {
        self.cache_options
            .as_ref()
            .and_then(|options| options.cache_to.as_ref())
    }

    /// Returns the effective build cache-disable choice with YAML scalar type retained.
    ///
    /// String values, including empty and interpolation-shaped values, remain strings after the
    /// explicit per-file interpolation stage and are never coerced to booleans. Omission and a
    /// reset null imply no default, builder, or cache behavior.
    #[must_use]
    pub fn no_cache(&self) -> Option<&ProjectValue<BuildNoCache>> {
        self.cache_options
            .as_ref()
            .and_then(|options| options.no_cache.as_ref())
    }
    /// Returns effective no-cache filter form with collection provenance.
    #[must_use]
    pub fn no_cache_filter(&self) -> Option<&ProjectValue<ProjectBuildNoCacheFilter>> {
        self.no_cache_filter.as_deref()
    }
    /// Returns the explicit build privileged boolean or deferred expression.
    #[must_use]
    pub fn privileged(&self) -> Option<&ProjectValue<BooleanValue>> {
        self.privileged.as_deref()
    }

    /// Returns the effective build SBOM choice with YAML scalar type retained.
    ///
    /// Strings, including empty, generator-shaped, and interpolation-derived values, remain
    /// strings. The value retains scalar replacement/reset/override provenance and sensitivity;
    /// this view neither parses generator data nor generates an SBOM or infers builder behavior.
    #[must_use]
    pub fn sbom(&self) -> Option<&ProjectValue<BuildSbom>> {
        self.sbom.as_deref()
    }
    /// Returns effective Build provenance as a boolean or opaque string scalar.
    #[must_use]
    pub fn provenance(&self) -> Option<&ProjectValue<BuildProvenance>> {
        self.provenance.as_deref()
    }

    /// Returns the effective build-image pull choice with complete scalar merge provenance.
    ///
    /// Literal booleans and deferred expressions remain distinct. Omission or a reset null does
    /// not imply a default, and this view neither resolves expressions nor infers build execution
    /// behavior.
    #[must_use]
    pub fn pull(&self) -> Option<&ProjectValue<BooleanValue>> {
        self.cache_options.as_ref().and_then(|options| options.pull.as_ref())
    }

    /// Returns the effective raw-preserving build-container shared-memory size.
    ///
    /// The value retains YAML number/string spelling, documented lowercase-unit classification,
    /// lexical zero, deferred expressions, scalar replacement/reset/override provenance, and
    /// interpolation sensitivity. This view does not infer builder defaults, allocations, host
    /// settings, or runtime behavior.
    #[must_use]
    pub fn shm_size(&self) -> Option<&ProjectValue<ShmSize>> {
        self.shm_size.as_deref()
    }

    /// Returns effective additional build tags in append order with per-item provenance.
    ///
    /// An explicit empty or reset sequence remains distinct from omission. Tags are opaque
    /// scalars; this view does not apply image-reference grammar or duplicate handling.
    #[must_use]
    pub fn tags(&self) -> Option<&ProjectValue<Vec<ProjectValue<String>>>> {
        self.tags.as_deref()
    }

    /// Returns effective build labels without normalizing mapping and list syntax.
    ///
    /// Mapping entries retain key/value provenance after per-key replacement. List entries retain
    /// append/reset/override provenance, order, duplicates, and bare-label spelling.
    #[must_use]
    pub fn labels(&self) -> Option<&ProjectValue<ProjectBuildLabels>> {
        self.labels.as_deref()
    }

    /// Returns effective build secret grants in append order with item and nested-field provenance.
    ///
    /// Short resource-name and long mapping syntax remain distinct. An explicit empty or reset
    /// sequence remains distinct from omission; this view does not resolve top-level secrets,
    /// read secret values, or infer build execution behavior.
    #[must_use]
    pub fn secrets(&self) -> Option<&ProjectValue<Vec<ProjectValue<ProjectGrant>>>> {
        self.secrets.as_deref()
    }

    /// Returns effective sensitive `BuildKit` SSH grants without normalizing their form.
    ///
    /// Collection, item, mapping key, and mapping value provenance are retained. Every value is
    /// sensitive even without interpolation; callers must use the explicit form accessors to
    /// inspect raw grants. No identifier, path, PEM, socket, agent, mount, or runtime behavior is
    /// parsed or accessed.
    #[must_use]
    pub fn ssh(&self) -> Option<&ProjectValue<ProjectBuildSsh>> {
        self.ssh.as_deref()
    }

    /// Returns effective build-container resource limits with nested merge provenance.
    ///
    /// Single and soft/hard forms retain exact scalar spelling, YAML string/number identity,
    /// interpolation sensitivity, explicit empty/reset/override mappings, and independent range
    /// members. This view injects no defaults, normalizes neither `-1` nor names, and does not
    /// validate host limits or infer builder or runtime behavior.
    #[must_use]
    pub fn ulimits(&self) -> Option<&ProjectValue<ProjectUlimits>> {
        self.ulimits.as_deref()
    }

    /// Returns all non-build-context/args/Dockerfile/entitlements/extra_hosts/target/network/isolation/platforms/no_cache/sbom/pull/shm_size/tags/labels/secrets/ulimits,
    /// extension, unknown, and malformed known fields.
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[ProjectFieldReference] {
        self.unmodeled_fields.as_slice()
    }

    fn push_unmodeled(&mut self, field: ProjectFieldReference) {
        Arc::make_mut(&mut self.unmodeled_fields).push(field);
    }

    fn cache_options(&mut self) -> &mut ProjectBuildCacheOptions {
        self.cache_options.get_or_insert_with(|| {
            Box::new(ProjectBuildCacheOptions {
                cache_from: None,
                cache_to: None,
                no_cache: None,
                pull: None,
            })
        })
    }
}

impl ProjectLogging {
    /// Returns the uninterpreted effective YAML string driver.
    #[must_use]
    pub const fn driver(&self) -> Option<&ProjectValue<String>> {
        self.driver.as_ref()
    }

    /// Returns the ordered options mapping, including an explicitly empty one.
    #[must_use]
    pub const fn options(&self) -> Option<&ProjectValue<ProjectLoggingOptions>> {
        self.options.as_ref()
    }

    /// Returns extensions, unknown fields, and malformed known siblings retained in the mapping.
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[ProjectFieldReference] {
        &self.unmodeled_fields
    }
}

/// The effective deploy mapping, with fields outside the native boundary retained as evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDeploy {
    endpoint_mode: Option<ProjectValue<DeployEndpointMode>>,
    labels: Option<ProjectValue<ProjectLabels>>,
    mode: Option<ProjectValue<DeployMode>>,
    placement: Option<ProjectValue<ProjectDeployPlacement>>,
    replicas: Option<ProjectValue<DeployReplicas>>,
    restart_policy: Option<ProjectValue<ProjectDeployRestartPolicy>>,
    unmodeled_fields: Vec<ProjectFieldReference>,
}

impl ProjectDeploy {
    /// Returns the effective service-discovery endpoint mode.
    #[must_use]
    pub const fn endpoint_mode(&self) -> Option<&ProjectValue<DeployEndpointMode>> {
        self.endpoint_mode.as_ref()
    }

    /// Returns effective deployment labels, distinct from service container labels.
    #[must_use]
    pub const fn labels(&self) -> Option<&ProjectValue<ProjectLabels>> {
        self.labels.as_ref()
    }

    /// Returns the effective deployment mode.
    #[must_use]
    pub const fn mode(&self) -> Option<&ProjectValue<DeployMode>> {
        self.mode.as_ref()
    }

    /// Returns the effective deployment placement without scheduling interpretation.
    #[must_use]
    pub const fn placement(&self) -> Option<&ProjectValue<ProjectDeployPlacement>> {
        self.placement.as_ref()
    }

    /// Returns the effective replica-count spelling and YAML scalar category.
    #[must_use]
    pub const fn replicas(&self) -> Option<&ProjectValue<DeployReplicas>> {
        self.replicas.as_ref()
    }

    /// Returns the effective deploy restart policy, separate from service `restart`.
    #[must_use]
    pub const fn restart_policy(&self) -> Option<&ProjectValue<ProjectDeployRestartPolicy>> {
        self.restart_policy.as_ref()
    }

    /// Returns immediate deploy children outside the current native boundary.
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[ProjectFieldReference] {
        &self.unmodeled_fields
    }
}

/// Effective deploy restart-policy members with independent provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDeployRestartPolicy {
    condition: Option<ProjectValue<DeployRestartCondition>>,
    delay: Option<ProjectValue<DeployRestartDuration>>,
    max_attempts: Option<ProjectValue<DeployRestartMaxAttempts>>,
    window: Option<ProjectValue<DeployRestartDuration>>,
    unmodeled_fields: Vec<ProjectFieldReference>,
}

impl ProjectDeployRestartPolicy {
    /// Returns the effective restart condition.
    #[must_use]
    pub const fn condition(&self) -> Option<&ProjectValue<DeployRestartCondition>> {
        self.condition.as_ref()
    }
    /// Returns the effective raw delay spelling.
    #[must_use]
    pub const fn delay(&self) -> Option<&ProjectValue<DeployRestartDuration>> {
        self.delay.as_ref()
    }
    /// Returns the effective maximum-attempts scalar.
    #[must_use]
    pub const fn max_attempts(&self) -> Option<&ProjectValue<DeployRestartMaxAttempts>> {
        self.max_attempts.as_ref()
    }
    /// Returns the effective raw window spelling.
    #[must_use]
    pub const fn window(&self) -> Option<&ProjectValue<DeployRestartDuration>> {
        self.window.as_ref()
    }
    /// Returns malformed and unknown retained members.
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[ProjectFieldReference] {
        &self.unmodeled_fields
    }
}

/// Effective deploy placement with item and nested-member provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDeployPlacement {
    constraints: Option<ProjectValue<Vec<ProjectValue<String>>>>,
    preferences: Option<ProjectValue<Vec<ProjectValue<ProjectDeployPlacementPreference>>>>,
    max_replicas_per_node: Option<ProjectValue<DeployPlacementMaxReplicasPerNode>>,
    unmodeled_fields: Vec<ProjectFieldReference>,
}

impl ProjectDeployPlacement {
    /// Returns ordered raw constraints, including duplicates and empty strings.
    #[must_use]
    pub const fn constraints(&self) -> Option<&ProjectValue<Vec<ProjectValue<String>>>> {
        self.constraints.as_ref()
    }

    /// Returns ordered placement preferences, including explicit empty mappings.
    #[must_use]
    pub const fn preferences(&self) -> Option<&ProjectValue<Vec<ProjectValue<ProjectDeployPlacementPreference>>>> {
        self.preferences.as_ref()
    }

    /// Returns the effective max-replicas-per-node scalar spelling and category.
    #[must_use]
    pub const fn max_replicas_per_node(&self) -> Option<&ProjectValue<DeployPlacementMaxReplicasPerNode>> {
        self.max_replicas_per_node.as_ref()
    }

    /// Returns extensions, unknown fields, and malformed children retained as evidence.
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[ProjectFieldReference] {
        &self.unmodeled_fields
    }
}

/// One effective placement preference mapping with nested provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDeployPlacementPreference {
    spread: Option<ProjectValue<String>>,
    unmodeled_fields: Vec<ProjectFieldReference>,
}

impl ProjectDeployPlacementPreference {
    /// Returns the raw effective spread expression without scheduling interpretation.
    #[must_use]
    pub const fn spread(&self) -> Option<&ProjectValue<String>> {
        self.spread.as_ref()
    }

    /// Returns extensions, unknown fields, and malformed members retained as evidence.
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
    hostname: Option<ProjectValue<Hostname>>,
    container_name: Option<ProjectValue<String>>,
    image: Option<ProjectValue<ImageReference>>,
    entrypoint: Option<ProjectValue<Entrypoint>>,
    command: Option<ProjectValue<Command>>,
    init: Option<ProjectValue<BooleanValue>>,
    stdin_open: Option<ProjectValue<BooleanValue>>,
    tty: Option<ProjectValue<BooleanValue>>,
    privileged: Option<ProjectValue<BooleanValue>>,
    environment: Option<ProjectValue<ProjectEnvironment>>,
    environment_files: Option<ProjectValue<Vec<ProjectValue<ProjectEnvironmentFile>>>>,
    labels: Option<ProjectValue<ProjectLabels>>,
    annotations: Option<ProjectValue<ProjectAnnotations>>,
    extra_hosts: Option<ProjectValue<ProjectExtraHosts>>,
    user: Option<ProjectValue<UserSpec>>,
    userns_mode: Option<ProjectValue<UserNamespaceMode>>,
    group_add: Option<ProjectValue<Vec<ProjectValue<String>>>>,
    cap_add: Option<ProjectValue<Vec<ProjectValue<CapabilityAddItem>>>>,
    cap_drop: Option<ProjectValue<Vec<ProjectValue<CapabilityDropItem>>>>,
    devices: Option<ProjectValue<Vec<ProjectValue<ProjectDevice>>>>,
    dns: Option<ProjectValue<ProjectDns>>,
    dns_options: Option<ProjectValue<Vec<ProjectValue<String>>>>,
    dns_search: Option<ProjectValue<ProjectDnsSearch>>,
    expose: Option<ProjectValue<Vec<ProjectValue<ProjectExposeItem>>>>,
    security_options: Option<ProjectValue<Vec<ProjectValue<ProjectSecurityOptionItem>>>>,
    working_dir: Option<ProjectValue<String>>,
    read_only: Option<ProjectValue<BooleanValue>>,
    pids_limit: Option<ProjectValue<PidsLimit>>,
    shm_size: Option<ProjectValue<ShmSize>>,
    mem_limit: Option<ProjectValue<MemLimit>>,
    tmpfs: Option<ProjectValue<ProjectTmpfs>>,
    sysctls: Option<ProjectValue<ProjectSysctls>>,
    logging: Option<ProjectValue<ProjectLogging>>,
    ulimits: Option<ProjectValue<ProjectUlimits>>,
    pull_policy: Option<ProjectValue<PullPolicy>>,
    restart: Option<ProjectValue<RestartPolicy>>,
    stop_signal: Option<ProjectValue<String>>,
    stop_grace_period: Option<ProjectValue<StopGracePeriod>>,
    healthcheck: Option<ProjectValue<ProjectHealthcheck>>,
    build: Option<ProjectValue<ProjectBuild>>,
    deploy: Option<ProjectValue<ProjectDeploy>>,
    depends_on: Option<ProjectValue<ProjectDependsOn>>,
    ports: Option<ProjectValue<Vec<ProjectValue<Port>>>>,
    volumes: Option<ProjectValue<Vec<ProjectValue<VolumeMount>>>>,
    configs: Option<ProjectValue<Vec<ProjectValue<ProjectGrant>>>>,
    secrets: Option<ProjectValue<Vec<ProjectValue<ProjectGrant>>>>,
    networks: Option<ProjectValue<ServiceNetworks>>,
    profiles: Option<ProjectValue<Vec<ProjectValue<String>>>>,
    unmodeled_fields: Vec<ProjectFieldReference>,
}

impl ProjectService {
    fn from_entry(entry: &MergedEntry) -> Self {
        Self {
            name: ProjectKey::from_entry(entry),
            provenance: entry.value().provenance().clone(),
            hostname: None,
            container_name: None,
            image: None,
            entrypoint: None,
            command: None,
            init: None,
            stdin_open: None,
            tty: None,
            privileged: None,
            environment: None,
            environment_files: None,
            labels: None,
            annotations: None,
            extra_hosts: None,
            user: None,
            userns_mode: None,
            group_add: None,
            cap_add: None,
            cap_drop: None,
            devices: None,
            dns: None,
            dns_options: None,
            dns_search: None,
            expose: None,
            security_options: None,
            working_dir: None,
            read_only: None,
            pids_limit: None,
            shm_size: None,
            mem_limit: None,
            tmpfs: None,
            sysctls: None,
            logging: None,
            ulimits: None,
            pull_policy: None,
            restart: None,
            stop_signal: None,
            stop_grace_period: None,
            healthcheck: None,
            build: None,
            deploy: None,
            depends_on: None,
            ports: None,
            volumes: None,
            configs: None,
            secrets: None,
            networks: None,
            profiles: None,
            unmodeled_fields: Vec::new(),
        }
    }

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

    /// Returns the effective raw-preserving service hostname.
    #[must_use]
    pub const fn hostname(&self) -> Option<&ProjectValue<Hostname>> {
        self.hostname.as_ref()
    }

    /// Returns the effective explicit runtime container name.
    #[must_use]
    pub const fn container_name(&self) -> Option<&ProjectValue<String>> {
        self.container_name.as_ref()
    }

    /// Returns the effective image reference.
    #[must_use]
    pub const fn image(&self) -> Option<&ProjectValue<ImageReference>> {
        self.image.as_ref()
    }

    /// Returns the effective entrypoint without normalizing scalar and list forms.
    #[must_use]
    pub const fn entrypoint(&self) -> Option<&ProjectValue<Entrypoint>> {
        self.entrypoint.as_ref()
    }

    /// Returns the effective command without normalizing scalar and list forms.
    #[must_use]
    pub const fn command(&self) -> Option<&ProjectValue<Command>> {
        self.command.as_ref()
    }

    /// Returns the effective platform-specific init-process choice.
    #[must_use]
    pub const fn init(&self) -> Option<&ProjectValue<BooleanValue>> {
        self.init.as_ref()
    }

    /// Returns whether Compose should keep standard input open for the effective service.
    #[must_use]
    pub const fn stdin_open(&self) -> Option<&ProjectValue<BooleanValue>> {
        self.stdin_open.as_ref()
    }

    /// Returns whether Compose should allocate a terminal for the effective service.
    #[must_use]
    pub const fn tty(&self) -> Option<&ProjectValue<BooleanValue>> {
        self.tty.as_ref()
    }

    /// Returns the effective privileged choice for the service.
    #[must_use]
    pub const fn privileged(&self) -> Option<&ProjectValue<BooleanValue>> {
        self.privileged.as_ref()
    }

    /// Returns environment entries normalized by key with per-entry syntax retained.
    #[must_use]
    pub const fn environment(&self) -> Option<&ProjectValue<ProjectEnvironment>> {
        self.environment.as_ref()
    }

    /// Returns effective service environment files in merge order with per-item provenance.
    #[must_use]
    pub const fn environment_files(&self) -> Option<&ProjectValue<Vec<ProjectValue<ProjectEnvironmentFile>>>> {
        self.environment_files.as_ref()
    }

    /// Returns effective service labels normalized by key with entry syntax retained.
    #[must_use]
    pub const fn labels(&self) -> Option<&ProjectValue<ProjectLabels>> {
        self.labels.as_ref()
    }

    /// Returns effective service annotations keyed by name with ambiguous key-only entries retained.
    #[must_use]
    pub const fn annotations(&self) -> Option<&ProjectValue<ProjectAnnotations>> {
        self.annotations.as_ref()
    }

    /// Returns effective service host mappings with per-entry provenance and syntax.
    #[must_use]
    pub const fn extra_hosts(&self) -> Option<&ProjectValue<ProjectExtraHosts>> {
        self.extra_hosts.as_ref()
    }

    /// Returns the effective container user and optional group spelling.
    #[must_use]
    pub const fn user(&self) -> Option<&ProjectValue<UserSpec>> {
        self.user.as_ref()
    }

    /// Returns the effective user-namespace mode.
    #[must_use]
    pub const fn userns_mode(&self) -> Option<&ProjectValue<UserNamespaceMode>> {
        self.userns_mode.as_ref()
    }

    /// Returns supplementary groups in effective merge order.
    #[must_use]
    pub const fn group_add(&self) -> Option<&ProjectValue<Vec<ProjectValue<String>>>> {
        self.group_add.as_ref()
    }

    /// Returns the effective capability-add sequence with full field and per-item provenance.
    ///
    /// `None` means the field was omitted; `Some` with an empty vector means it was explicitly
    /// configured empty or reset.
    #[must_use]
    pub const fn cap_add(&self) -> Option<&ProjectValue<Vec<ProjectValue<CapabilityAddItem>>>> {
        self.cap_add.as_ref()
    }

    /// Returns the effective capability-drop sequence with full field and per-item provenance.
    ///
    /// `None` means the field was omitted; `Some` with an empty vector means it was explicitly
    /// configured empty or reset.
    #[must_use]
    pub const fn cap_drop(&self) -> Option<&ProjectValue<Vec<ProjectValue<CapabilityDropItem>>>> {
        self.cap_drop.as_ref()
    }

    /// Returns effective ordered mixed short/long service devices with complete provenance.
    ///
    /// `None` means omission; `Some` with an empty vector means an explicit empty sequence or reset.
    #[must_use]
    pub const fn devices(&self) -> Option<&ProjectValue<Vec<ProjectValue<ProjectDevice>>>> {
        self.devices.as_ref()
    }

    /// Returns effective raw service DNS servers with source form and provenance.
    #[must_use]
    pub const fn dns(&self) -> Option<&ProjectValue<ProjectDns>> {
        self.dns.as_ref()
    }

    /// Returns the effective ordered service DNS resolver options.
    ///
    /// Omission remains `None`; an explicitly empty or reset sequence remains `Some` with no
    /// items. Exact duplicate items are retained and diagnosed.
    #[must_use]
    pub const fn dns_options(&self) -> Option<&ProjectValue<Vec<ProjectValue<String>>>> {
        self.dns_options.as_ref()
    }

    /// Returns effective raw DNS search domains with source form and provenance.
    #[must_use]
    pub const fn dns_search(&self) -> Option<&ProjectValue<ProjectDnsSearch>> {
        self.dns_search.as_ref()
    }

    /// Returns the effective ordered service `expose` sequence.
    ///
    /// Omission remains `None`; an explicitly empty or reset sequence remains `Some` with no
    /// items. Exact scalar identity includes both value text and YAML string/number kind.
    #[must_use]
    pub const fn expose(&self) -> Option<&ProjectValue<Vec<ProjectValue<ProjectExposeItem>>>> {
        self.expose.as_ref()
    }

    /// Returns the effective ordered raw service security options.
    ///
    /// Omission remains `None`; an explicitly empty or reset sequence remains `Some` with no
    /// items. Duplicates remain ordered evidence.
    #[must_use]
    pub const fn security_options(&self) -> Option<&ProjectValue<Vec<ProjectValue<ProjectSecurityOptionItem>>>> {
        self.security_options.as_ref()
    }

    /// Returns the effective container working-directory override.
    #[must_use]
    pub const fn working_dir(&self) -> Option<&ProjectValue<String>> {
        self.working_dir.as_ref()
    }

    /// Returns the effective read-only root-filesystem choice.
    #[must_use]
    pub const fn read_only(&self) -> Option<&ProjectValue<BooleanValue>> {
        self.read_only.as_ref()
    }

    /// Returns the effective raw-preserving service PID limit.
    #[must_use]
    pub const fn pids_limit(&self) -> Option<&ProjectValue<PidsLimit>> {
        self.pids_limit.as_ref()
    }

    /// Returns the effective raw-preserving service shared-memory size.
    #[must_use]
    pub const fn shm_size(&self) -> Option<&ProjectValue<ShmSize>> {
        self.shm_size.as_ref()
    }

    /// Returns the effective raw-preserving service memory limit.
    #[must_use]
    pub const fn mem_limit(&self) -> Option<&ProjectValue<MemLimit>> {
        self.mem_limit.as_ref()
    }

    /// Returns effective service-level temporary filesystems with source form and provenance.
    #[must_use]
    pub const fn tmpfs(&self) -> Option<&ProjectValue<ProjectTmpfs>> {
        self.tmpfs.as_ref()
    }

    /// Returns effective service sysctls with source form and per-entry provenance.
    #[must_use]
    pub const fn sysctls(&self) -> Option<&ProjectValue<ProjectSysctls>> {
        self.sysctls.as_ref()
    }

    /// Returns effective service logging configuration with nested provenance and recovery data.
    #[must_use]
    pub const fn logging(&self) -> Option<&ProjectValue<ProjectLogging>> {
        self.logging.as_ref()
    }

    /// Returns effective ordered service limits with nested and field-level merge provenance.
    ///
    /// `None` means the field was omitted; an empty mapping remains present and can carry reset or
    /// override provenance.
    #[must_use]
    pub const fn ulimits(&self) -> Option<&ProjectValue<ProjectUlimits>> {
        self.ulimits.as_ref()
    }

    /// Returns the effective raw-preserving service image pull policy.
    #[must_use]
    pub const fn pull_policy(&self) -> Option<&ProjectValue<PullPolicy>> {
        self.pull_policy.as_ref()
    }

    /// Returns the effective service-level container restart policy.
    #[must_use]
    pub const fn restart(&self) -> Option<&ProjectValue<RestartPolicy>> {
        self.restart.as_ref()
    }

    /// Returns the effective explicitly authored service stop signal.
    #[must_use]
    pub const fn stop_signal(&self) -> Option<&ProjectValue<String>> {
        self.stop_signal.as_ref()
    }

    /// Returns the effective raw-preserving service stop grace period.
    #[must_use]
    pub const fn stop_grace_period(&self) -> Option<&ProjectValue<StopGracePeriod>> {
        self.stop_grace_period.as_ref()
    }

    /// Returns the effective health check with per-field merge provenance.
    #[must_use]
    pub const fn healthcheck(&self) -> Option<&ProjectValue<ProjectHealthcheck>> {
        self.healthcheck.as_ref()
    }

    /// Returns the effective build declaration with scalar and mapping context forms retained.
    ///
    /// Only build context, arguments, Dockerfile, target, network, platforms, no-cache, pull,
    /// tags, labels, secrets, and ulimits are natively
    /// modeled. All sibling build fields remain available as source-addressable unmodeled references.
    #[must_use]
    pub const fn build(&self) -> Option<&ProjectValue<ProjectBuild>> {
        self.build.as_ref()
    }

    /// Returns the effective deploy mapping without applying deployment semantics.
    #[must_use]
    pub const fn deploy(&self) -> Option<&ProjectValue<ProjectDeploy>> {
        self.deploy.as_ref()
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

    /// Returns effective service config grants with syntax and field-level provenance retained.
    #[must_use]
    pub const fn configs(&self) -> Option<&ProjectValue<Vec<ProjectValue<ProjectGrant>>>> {
        self.configs.as_ref()
    }

    /// Returns effective service secret grants with syntax and field-level provenance retained.
    #[must_use]
    pub const fn secrets(&self) -> Option<&ProjectValue<Vec<ProjectValue<ProjectGrant>>>> {
        self.secrets.as_ref()
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
        let mut service = ProjectService::from_entry(entry);
        let path = ["services".to_owned(), entry.key().to_owned()];
        for field in fields {
            match field.key() {
                "hostname" => service.hostname = self.hostname(field.value()),
                "container_name" => {
                    service.container_name = self.project_string(field.value(), "service container name");
                }
                "image" => {
                    service.image = self
                        .project_string(field.value(), "service image")
                        .map(|value| ProjectValue {
                            value: ImageReference::parse(value.value),
                            provenance: value.provenance,
                            sensitive: value.sensitive,
                        });
                }
                "entrypoint" => service.entrypoint = self.entrypoint(field.value()),
                "command" => service.command = self.command(field.value()),
                "init" => service.init = self.boolean(field.value(), "service init must be a boolean"),
                "stdin_open" => {
                    service.stdin_open = self.boolean(field.value(), "service stdin_open must be a boolean");
                }
                "tty" => service.tty = self.boolean(field.value(), "service tty must be a boolean"),
                "privileged" => {
                    service.privileged = self.boolean(field.value(), "service privileged must be a boolean");
                }
                "environment" => service.environment = self.environment(field.value()),
                "env_file" => service.environment_files = self.environment_files(field.value(), &path),
                "labels" => service.labels = self.service_labels(field.value()),
                "annotations" => service.annotations = self.service_annotations(field.value()),
                "extra_hosts" => service.extra_hosts = self.extra_hosts(field.value()),
                "user" => service.user = self.user(field.value()),
                "userns_mode" => service.userns_mode = self.userns_mode(field.value()),
                "group_add" => {
                    service.group_add = self.string_collection(field.value(), "group_add must be a sequence");
                }
                "cap_add" => service.cap_add = self.capability_add(field.value()),
                "cap_drop" => service.cap_drop = self.capability_drop(field.value()),
                "devices" => service.devices = self.devices(field.value(), &path),
                "dns" => service.dns = self.dns(field.value()),
                "dns_opt" => service.dns_options = self.dns_options(field.value()),
                "dns_search" => service.dns_search = self.dns_search(field.value()),
                "expose" => service.expose = self.expose(field.value()),
                "security_opt" => service.security_options = self.security_options(field.value()),
                "working_dir" => service.working_dir = self.project_string(field.value(), "service working directory"),
                "read_only" => {
                    service.read_only = self
                        .located_boolean(field.value(), "service read_only must be a boolean")
                        .map(|value| ProjectValue::new(value.into_value(), field.value()));
                }
                "pids_limit" => service.pids_limit = self.pids_limit(field.value()),
                "shm_size" => service.shm_size = self.shm_size(field.value()),
                "mem_limit" => service.mem_limit = self.mem_limit(field.value()),
                "tmpfs" => service.tmpfs = self.tmpfs(field.value()),
                "sysctls" => service.sysctls = self.sysctls(field.value()),
                "logging" => {
                    service.logging = self.logging(field.value(), &path);
                    if service.logging.is_none() {
                        service.unmodeled_fields.push(field_reference(&path, field));
                    }
                }
                "ulimits" => {
                    let (ulimits, unmodeled) = self.ulimits(field.value(), &path);
                    service.ulimits = ulimits;
                    service.unmodeled_fields.extend(unmodeled);
                    if service.ulimits.is_none() {
                        service.unmodeled_fields.push(field_reference(&path, field));
                    }
                }
                "pull_policy" => service.pull_policy = self.pull_policy(field.value()),
                "restart" => service.restart = self.restart_policy(field.value()),
                "stop_signal" => {
                    service.stop_signal = self.project_string(field.value(), "service stop signal");
                }
                "stop_grace_period" => {
                    service.stop_grace_period = self.stop_grace_period(field.value());
                }
                "healthcheck" => service.healthcheck = self.healthcheck(field.value(), &path),
                "build" => service.build = self.service_build(field.value(), &path),
                "deploy" => self.set_service_deploy(&mut service, field, &path),
                "depends_on" => service.depends_on = self.depends_on(field.value(), &path),
                "ports" => service.ports = self.ports(field.value(), &path),
                "volumes" => service.volumes = self.volumes(field.value(), &path),
                "configs" => service.configs = self.grants(field.value(), &path, "config"),
                "secrets" => service.secrets = self.grants(field.value(), &path, "secret"),
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

    fn boolean(&mut self, field: &MergedValue, message: &'static str) -> Option<ProjectValue<BooleanValue>> {
        self.located_boolean(field, message)
            .map(|value| ProjectValue::new(value.into_value(), field))
    }

    fn set_service_deploy(&mut self, service: &mut ProjectService, field: &MergedEntry, path: &[String]) {
        service.deploy = self.service_deploy(field, path);
        if service.deploy.is_none() {
            service.unmodeled_fields.push(field_reference(path, field));
        }
    }

    fn service_deploy(&mut self, field: &MergedEntry, path: &[String]) -> Option<ProjectValue<ProjectDeploy>> {
        let mut deploy_path = path.to_vec();
        deploy_path.push(field.key().to_owned());
        let fields = self.mapping(field.value(), "deploy must be a mapping")?;
        let mut deploy = ProjectDeploy {
            endpoint_mode: None,
            labels: None,
            mode: None,
            placement: None,
            replicas: None,
            restart_policy: None,
            unmodeled_fields: Vec::new(),
        };
        for nested in fields {
            match nested.key() {
                "endpoint_mode" => {
                    if let Some(endpoint_mode) = self.deploy_endpoint_mode(nested.value()) {
                        deploy.endpoint_mode = Some(endpoint_mode);
                    } else {
                        deploy.unmodeled_fields.push(field_reference(&deploy_path, nested));
                    }
                }
                "labels" => {
                    let (labels, malformed) = self.deploy_labels(nested.value());
                    if let Some(labels) = labels {
                        deploy.labels = Some(labels);
                    }
                    if malformed {
                        deploy.unmodeled_fields.push(field_reference(&deploy_path, nested));
                    }
                }
                "mode" => {
                    if let Some(mode) = self.deploy_mode(nested.value()) {
                        deploy.mode = Some(mode);
                    } else {
                        deploy.unmodeled_fields.push(field_reference(&deploy_path, nested));
                    }
                }
                "placement" => {
                    if let Some(placement) = self.deploy_placement(nested.value(), &deploy_path) {
                        deploy.placement = Some(placement);
                    } else {
                        deploy.unmodeled_fields.push(field_reference(&deploy_path, nested));
                    }
                }
                "replicas" => {
                    if let Some(replicas) = self.deploy_replicas(nested.value()) {
                        deploy.replicas = Some(replicas);
                    } else {
                        deploy.unmodeled_fields.push(field_reference(&deploy_path, nested));
                    }
                }
                "restart_policy" => {
                    if let Some(policy) = self.deploy_restart_policy(nested.value(), &deploy_path) {
                        deploy.restart_policy = Some(policy);
                    } else {
                        deploy.unmodeled_fields.push(field_reference(&deploy_path, nested));
                    }
                }
                _ => deploy.unmodeled_fields.push(field_reference(&deploy_path, nested)),
            }
        }
        Some(ProjectValue::new(deploy, field.value()))
    }

    fn deploy_labels(&mut self, value: &MergedValue) -> (Option<ProjectValue<ProjectLabels>>, bool) {
        let mut entries = Vec::new();
        let mut malformed = false;
        let form = match value.kind() {
            MergedValueKind::Mapping(values) => {
                for entry in values {
                    let scalar = if entry.syntax() == EntrySyntax::ListKeyOnly {
                        Some(ComposeScalar::String(String::new()))
                    } else {
                        self.compose_scalar(entry.value(), "deploy label value must be a scalar or null")
                    };
                    let Some(scalar) = scalar else {
                        malformed = true;
                        continue;
                    };
                    entries.push(ProjectLabelEntry {
                        name: ProjectKey::from_entry(entry),
                        value: ProjectValue::new(scalar, entry.value()),
                        syntax: entry.syntax(),
                    });
                }
                ProjectLabelsForm::Map
            }
            MergedValueKind::Sequence(values) => {
                for item in values {
                    let Some(raw) = self.located_string(item, "deploy label list item must be a scalar") else {
                        malformed = true;
                        continue;
                    };
                    let (name, value, syntax) = raw.value().split_once('=').map_or_else(
                        || (raw.value().clone(), String::new(), EntrySyntax::ListKeyOnly),
                        |(name, value)| (name.to_owned(), value.to_owned(), EntrySyntax::ListKeyValue),
                    );
                    entries.push(ProjectLabelEntry {
                        name: ProjectKey::from_value(name, item),
                        value: ProjectValue::new(ComposeScalar::String(value), item),
                        syntax,
                    });
                }
                ProjectLabelsForm::List
            }
            _ => {
                self.expected(value, "deploy labels must be a mapping or sequence");
                return (None, true);
            }
        };
        (
            Some(ProjectValue::new(ProjectLabels { form, entries }, value)),
            malformed,
        )
    }

    fn deploy_endpoint_mode(&mut self, value: &MergedValue) -> Option<ProjectValue<DeployEndpointMode>> {
        let scalar = match value.kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => scalar,
            _ => {
                self.expected(value, "deploy endpoint_mode must be a YAML string scalar");
                return None;
            }
        };
        let endpoint_mode = DeployEndpointMode::parse(scalar.value().to_owned());
        if !endpoint_mode.is_documented() {
            self.diagnostics.push(
                Diagnostic::new(
                    DEPLOY_ENDPOINT_MODE_PORTABILITY,
                    Severity::Warning,
                    "deploy endpoint_mode is outside Compose's documented portable values",
                )
                .with_label(DiagnosticLabel::primary(
                    effective_span(value),
                    "retained provider-specific endpoint mode",
                )),
            );
        }
        Some(ProjectValue::new(endpoint_mode, value))
    }

    fn deploy_mode(&mut self, value: &MergedValue) -> Option<ProjectValue<DeployMode>> {
        let scalar = match value.kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => scalar,
            _ => {
                self.expected(value, "deploy mode must be a YAML string scalar");
                return None;
            }
        };
        let mode = DeployMode::parse(scalar.value().to_owned());
        if !mode.is_documented() {
            self.diagnostics.push(
                Diagnostic::new(
                    DEPLOY_MODE_PORTABILITY,
                    Severity::Warning,
                    "deploy mode is outside Compose's documented portable values",
                )
                .with_label(DiagnosticLabel::primary(
                    effective_span(value),
                    "retained provider-specific deploy mode",
                )),
            );
        }
        Some(ProjectValue::new(mode, value))
    }

    fn deploy_replicas(&mut self, value: &MergedValue) -> Option<ProjectValue<DeployReplicas>> {
        let MergedValueKind::Scalar(scalar) = value.kind() else {
            self.expected(value, "deploy replicas must be a YAML number or string scalar");
            return None;
        };
        let replicas = match scalar.kind() {
            MergedScalarKind::Number => DeployReplicas::YamlNumber(scalar.value().to_owned()),
            MergedScalarKind::String => DeployReplicas::String(scalar.value().to_owned()),
            MergedScalarKind::Boolean => {
                self.expected(value, "deploy replicas must be a YAML number or string scalar");
                return None;
            }
        };
        Some(ProjectValue::new(replicas, value))
    }

    fn deploy_restart_policy(
        &mut self,
        value: &MergedValue,
        path: &[String],
    ) -> Option<ProjectValue<ProjectDeployRestartPolicy>> {
        let fields = self.mapping(value, "deploy restart_policy must be a mapping")?;
        let mut member_path = path.to_vec();
        member_path.push("restart_policy".to_owned());
        let mut policy = ProjectDeployRestartPolicy {
            condition: None,
            delay: None,
            max_attempts: None,
            window: None,
            unmodeled_fields: Vec::new(),
        };
        for field in fields {
            let typed = match field.key() {
                "condition" => self
                    .project_string(field.value(), "deploy restart-policy condition")
                    .map(|raw| ProjectValue {
                        value: DeployRestartCondition::parse(raw.value),
                        provenance: raw.provenance,
                        sensitive: raw.sensitive,
                    })
                    .map(|value| {
                        policy.condition = Some(value);
                    }),
                "delay" => self
                    .project_string(field.value(), "deploy restart-policy delay")
                    .map(|raw| ProjectValue {
                        value: DeployRestartDuration::new(raw.value),
                        provenance: raw.provenance,
                        sensitive: raw.sensitive,
                    })
                    .map(|value| {
                        policy.delay = Some(value);
                    }),
                "window" => self
                    .project_string(field.value(), "deploy restart-policy window")
                    .map(|raw| ProjectValue {
                        value: DeployRestartDuration::new(raw.value),
                        provenance: raw.provenance,
                        sensitive: raw.sensitive,
                    })
                    .map(|value| {
                        policy.window = Some(value);
                    }),
                "max_attempts" => self.deploy_restart_max_attempts(field.value()).map(|value| {
                    policy.max_attempts = Some(value);
                }),
                _ => {
                    policy.unmodeled_fields.push(field_reference(&member_path, field));
                    Some(())
                }
            };
            if typed.is_none() {
                policy.unmodeled_fields.push(field_reference(&member_path, field));
            }
        }
        Some(ProjectValue::new(policy, value))
    }

    fn deploy_restart_max_attempts(&mut self, value: &MergedValue) -> Option<ProjectValue<DeployRestartMaxAttempts>> {
        let MergedValueKind::Scalar(scalar) = value.kind() else {
            self.expected(
                value,
                "deploy restart-policy max_attempts must be a YAML integer or string scalar",
            );
            return None;
        };
        let parsed = match scalar.kind() {
            MergedScalarKind::Number if Self::yaml_integer_spelling(scalar.value()) => {
                DeployRestartMaxAttempts::YamlNumber(scalar.value().to_owned())
            }
            MergedScalarKind::String => DeployRestartMaxAttempts::String(scalar.value().to_owned()),
            MergedScalarKind::Number | MergedScalarKind::Boolean => {
                self.expected(
                    value,
                    "deploy restart-policy max_attempts must be a YAML integer or string scalar",
                );
                return None;
            }
        };
        Some(ProjectValue::new(parsed, value))
    }

    fn deploy_placement(
        &mut self,
        value: &MergedValue,
        path: &[String],
    ) -> Option<ProjectValue<ProjectDeployPlacement>> {
        let fields = self.mapping(value, "deploy placement must be a mapping")?;
        let mut placement_path = path.to_vec();
        placement_path.push("placement".to_owned());
        let mut placement = ProjectDeployPlacement {
            constraints: None,
            preferences: None,
            max_replicas_per_node: None,
            unmodeled_fields: Vec::new(),
        };
        for field in fields {
            match field.key() {
                "constraints" => {
                    let (constraints, malformed) = self.deploy_placement_constraints(field.value());
                    placement.constraints = constraints;
                    if malformed {
                        placement.unmodeled_fields.push(field_reference(&placement_path, field));
                    }
                }
                "preferences" => {
                    let (preferences, malformed) = self.deploy_placement_preferences(field.value(), &placement_path);
                    placement.preferences = preferences;
                    if malformed {
                        placement.unmodeled_fields.push(field_reference(&placement_path, field));
                    }
                }
                "max_replicas_per_node" => {
                    if let Some(maximum) = self.deploy_placement_max_replicas_per_node(field.value()) {
                        placement.max_replicas_per_node = Some(maximum);
                    } else {
                        placement.unmodeled_fields.push(field_reference(&placement_path, field));
                    }
                }
                _ => placement.unmodeled_fields.push(field_reference(&placement_path, field)),
            }
        }
        Some(ProjectValue::new(placement, value))
    }

    fn deploy_placement_constraints(
        &mut self,
        value: &MergedValue,
    ) -> (Option<ProjectValue<Vec<ProjectValue<String>>>>, bool) {
        let Some(values) = value.as_sequence() else {
            self.expected(
                value,
                "deploy placement constraints must be a sequence of YAML string scalars",
            );
            return (None, true);
        };
        let mut constraints = Vec::new();
        let mut malformed = false;
        for item in values {
            match item.kind() {
                MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => {
                    constraints.push(ProjectValue::new(scalar.value().to_owned(), item));
                }
                _ => {
                    self.expected(item, "deploy placement constraints must contain YAML string scalars");
                    malformed = true;
                }
            }
        }
        (Some(ProjectValue::new(constraints, value)), malformed)
    }

    fn deploy_placement_preferences(
        &mut self,
        value: &MergedValue,
        path: &[String],
    ) -> (
        Option<ProjectValue<Vec<ProjectValue<ProjectDeployPlacementPreference>>>>,
        bool,
    ) {
        let Some(values) = value.as_sequence() else {
            self.expected(value, "deploy placement preferences must be a sequence of mappings");
            return (None, true);
        };
        let mut preferences = Vec::new();
        let mut malformed = false;
        for (index, item) in values.iter().enumerate() {
            let Some(fields) = item.as_mapping() else {
                self.expected(item, "deploy placement preferences must contain mappings");
                malformed = true;
                continue;
            };
            let mut preference_path = path.to_vec();
            preference_path.extend(["preferences".to_owned(), index.to_string()]);
            let mut preference = ProjectDeployPlacementPreference {
                spread: None,
                unmodeled_fields: Vec::new(),
            };
            for field in fields {
                match field.key() {
                    "spread" => match field.value().kind() {
                        MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => {
                            preference.spread = Some(ProjectValue::new(scalar.value().to_owned(), field.value()));
                        }
                        _ => {
                            self.expected(
                                field.value(),
                                "deploy placement preference spread must be a YAML string scalar",
                            );
                            preference
                                .unmodeled_fields
                                .push(field_reference(&preference_path, field));
                        }
                    },
                    _ => preference
                        .unmodeled_fields
                        .push(field_reference(&preference_path, field)),
                }
            }
            preferences.push(ProjectValue::new(preference, item));
        }
        (Some(ProjectValue::new(preferences, value)), malformed)
    }

    fn deploy_placement_max_replicas_per_node(
        &mut self,
        value: &MergedValue,
    ) -> Option<ProjectValue<DeployPlacementMaxReplicasPerNode>> {
        let MergedValueKind::Scalar(scalar) = value.kind() else {
            self.expected(
                value,
                "deploy placement max_replicas_per_node must be a YAML integer or string scalar",
            );
            return None;
        };
        let maximum = match scalar.kind() {
            MergedScalarKind::Number if Self::yaml_integer_spelling(scalar.value()) => {
                DeployPlacementMaxReplicasPerNode::YamlInteger(scalar.value().to_owned())
            }
            MergedScalarKind::String => DeployPlacementMaxReplicasPerNode::String(scalar.value().to_owned()),
            MergedScalarKind::Number | MergedScalarKind::Boolean => {
                self.expected(
                    value,
                    "deploy placement max_replicas_per_node must be a YAML integer or string scalar",
                );
                return None;
            }
        };
        Some(ProjectValue::new(maximum, value))
    }

    /// Recognizes raw YAML integer spellings after merge has coalesced integer and float scalars.
    ///
    /// This deliberately preserves spelling and accepts only decimal, binary, octal, and
    /// hexadecimal integer forms with optional signs and digit separators. It therefore never
    /// promotes a float-shaped merged number to either integer-only deploy field.
    fn yaml_integer_spelling(value: &str) -> bool {
        let digits = value
            .strip_prefix('+')
            .or_else(|| value.strip_prefix('-'))
            .unwrap_or(value);
        let (radix, digits) = if let Some(value) = digits.strip_prefix("0b") {
            (2, value)
        } else if let Some(value) = digits.strip_prefix("0o") {
            (8, value)
        } else if let Some(value) = digits.strip_prefix("0x") {
            (16, value)
        } else {
            (10, digits)
        };
        let mut saw_digit = false;
        let mut previous_separator = false;
        for byte in digits.bytes() {
            if byte == b'_' {
                if !saw_digit || previous_separator {
                    return false;
                }
                previous_separator = true;
            } else if if radix == 16 {
                byte.is_ascii_hexdigit()
            } else {
                byte.is_ascii_digit() && (byte - b'0') < radix
            } {
                saw_digit = true;
                previous_separator = false;
            } else {
                return false;
            }
        }
        saw_digit && !previous_separator
    }

    fn hostname(&mut self, value: &MergedValue) -> Option<ProjectValue<Hostname>> {
        let scalar = match value.kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => scalar,
            _ => {
                self.expected(value, "hostname must be a YAML string scalar");
                return None;
            }
        };
        let hostname = Hostname::parse(Located::new(scalar.value().to_owned(), effective_span(value)));
        if hostname.kind() == &HostnameKind::Invalid {
            self.invalid(
                effective_span(value),
                "hostname must be an ASCII RFC-1123 name of 1 to 253 characters with dot-separated labels of 1 to 63 alphanumeric or hyphen characters; each label must start and end alphanumeric",
            );
        }
        Some(ProjectValue::new(hostname, value))
    }

    fn restart_policy(&mut self, value: &MergedValue) -> Option<ProjectValue<RestartPolicy>> {
        let policy = RestartPolicy::parse(self.located_string(value, "restart must be a non-null scalar")?);
        if !policy.is_valid() {
            self.invalid(
                effective_span(value),
                "restart must be `no`, `always`, `on-failure[:max-retries]`, or `unless-stopped`",
            );
        }
        Some(ProjectValue::new(policy, value))
    }

    fn service_build(&mut self, value: &MergedValue, path: &[String]) -> Option<ProjectValue<ProjectBuild>> {
        let build = match value.kind() {
            MergedValueKind::Scalar(_) => ProjectBuild::Context(self.project_string(value, "build context")?),
            MergedValueKind::Mapping(fields) => {
                let mut definition = ProjectBuildDefinition {
                    additional_contexts: None,
                    context: None,
                    args: None,
                    entitlements: None,
                    extra_hosts: None,
                    dockerfile: None,
                    dockerfile_inline: None,
                    target: None,
                    network: None,
                    isolation: None,
                    platforms: None,
                    cache_options: None,
                    no_cache_filter: None,
                    privileged: None,
                    sbom: None,
                    provenance: None,
                    shm_size: None,
                    tags: None,
                    labels: None,
                    secrets: None,
                    ssh: None,
                    ulimits: None,
                    unmodeled_fields: Arc::new(Vec::new()),
                };
                let (mut dockerfile, mut dockerfile_inline) = (None, None);
                for field in fields {
                    match field.key() {
                        "additional_contexts" => self.set_build_additional_contexts(&mut definition, field, path),
                        "args" => {
                            let (args, malformed) = self.build_args(field.value());
                            definition.args = args.map(Box::new);
                            if malformed {
                                definition.push_unmodeled(field_reference(path, field));
                            }
                        }
                        "cache_from" | "cache_to" => self.cache_locations(&mut definition, field, path),
                        "context" => {
                            if let Some(context) = self.project_string(field.value(), "build context") {
                                definition.context = Some(Box::new(context));
                            } else {
                                definition.push_unmodeled(field_reference(path, field));
                            }
                        }
                        "dockerfile" => {
                            dockerfile = Some(field);
                            if let Some(dockerfile) = self.non_empty_project_string(field.value(), "build dockerfile") {
                                definition.dockerfile = Some(Box::new(dockerfile));
                            } else {
                                definition.push_unmodeled(field_reference(path, field));
                            }
                        }
                        "dockerfile_inline" => self.set_inline(&mut definition, field, path, &mut dockerfile_inline),
                        "entitlements" => self.set_build_entitlements(&mut definition, field, path),
                        "extra_hosts" => self.set_build_extra_hosts(&mut definition, field, path),
                        "target" => {
                            if let Some(target) = self.project_string(field.value(), "build target") {
                                definition.target = Some(Box::new(target));
                            } else {
                                definition.push_unmodeled(field_reference(path, field));
                            }
                        }
                        "network" => self.set_build_network(&mut definition, field, path),
                        "isolation" => self.set_build_isolation(&mut definition, field, path),
                        "platforms" => self.set_build_platforms(&mut definition, field, path),
                        "no_cache" => self.set_build_no_cache(&mut definition, field, path),
                        "no_cache_filter" => self.set_build_no_cache_filter(&mut definition, field, path),
                        "privileged" => self.set_build_privileged(&mut definition, field, path),
                        "sbom" | "provenance" => self.set_build_attestation(&mut definition, field, path),
                        "pull" => self.set_build_pull(&mut definition, field, path),
                        "shm_size" => self.set_build_shm_size(&mut definition, field, path),
                        "tags" => {
                            let (tags, malformed) = self.build_tags(field.value());
                            definition.tags = tags.map(Box::new);
                            if malformed {
                                definition.push_unmodeled(field_reference(path, field));
                            }
                        }
                        "labels" => {
                            let (labels, malformed) = self.build_labels(field.value());
                            definition.labels = labels.map(Box::new);
                            if malformed {
                                definition.push_unmodeled(field_reference(path, field));
                            }
                        }
                        "secrets" => self.set_build_secrets(&mut definition, field, path),
                        "ssh" => self.set_build_ssh(&mut definition, field, path),
                        "ulimits" => self.set_build_ulimits(&mut definition, field, path),
                        _ => definition.push_unmodeled(field_reference(path, field)),
                    }
                }
                self.report_build_dockerfile_conflict(dockerfile, dockerfile_inline);
                ProjectBuild::Definition(definition)
            }
            _ => return self.invalid_project_build_form(value),
        };
        Some(ProjectValue::new(build, value))
    }

    fn invalid_project_build_form(&mut self, value: &MergedValue) -> Option<ProjectValue<ProjectBuild>> {
        self.expected(value, "build must be a scalar context or mapping");
        None
    }

    fn report_build_dockerfile_conflict(
        &mut self,
        dockerfile: Option<&MergedEntry>,
        dockerfile_inline: Option<&MergedEntry>,
    ) {
        let (Some(dockerfile), Some(dockerfile_inline)) = (dockerfile, dockerfile_inline) else {
            return;
        };
        self.diagnostics.push(
            Diagnostic::new(
                BUILD_DOCKERFILE_INLINE_CONFLICT,
                Severity::Error,
                "build `dockerfile` and `dockerfile_inline` are mutually exclusive",
            )
            .with_label(DiagnosticLabel::primary(entry_span(dockerfile), "dockerfile retained"))
            .with_label(DiagnosticLabel::secondary(
                entry_span(dockerfile_inline),
                "dockerfile_inline retained",
            )),
        );
    }

    fn set_build_secrets(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        let (secrets, malformed) = self.build_secret_grants(field.value(), path);
        definition.secrets = secrets.map(Box::new);
        if malformed {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn set_build_ssh(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        let (ssh, malformed) = self.build_ssh(field.value());
        definition.ssh = ssh.map(Box::new);
        if malformed {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn set_build_ulimits(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        let (ulimits, unmodeled) = self.ulimits(field.value(), path);
        definition.ulimits = ulimits.map(Box::new);
        for field in unmodeled {
            definition.push_unmodeled(field);
        }
        if definition.ulimits.is_none() {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn set_build_additional_contexts(
        &mut self,
        definition: &mut ProjectBuildDefinition,
        field: &MergedEntry,
        path: &[String],
    ) {
        let (additional_contexts, malformed) = self.build_additional_contexts(field.value());
        definition.additional_contexts = additional_contexts.map(Box::new);
        if malformed {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn set_build_entitlements(
        &mut self,
        definition: &mut ProjectBuildDefinition,
        field: &MergedEntry,
        path: &[String],
    ) {
        let (entitlements, malformed) = self.build_entitlements(field.value());
        definition.entitlements = entitlements.map(Box::new);
        if malformed {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn set_build_dockerfile_inline<'field>(
        &mut self,
        definition: &mut ProjectBuildDefinition,
        field: &'field MergedEntry,
        path: &[String],
        dockerfile_inline: &mut Option<&'field MergedEntry>,
    ) {
        *dockerfile_inline = Some(field);
        let dockerfile_inline = match field.value().kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => {
                Some(ProjectValue::new(scalar.value().to_owned(), field.value()))
            }
            MergedValueKind::Scalar(_) => {
                self.invalid(
                    effective_span(field.value()),
                    "build dockerfile_inline must be a YAML string scalar",
                );
                None
            }
            _ => {
                self.expected(field.value(), "build dockerfile_inline must be a YAML string scalar");
                None
            }
        };
        if let Some(dockerfile_inline) = dockerfile_inline {
            definition.dockerfile_inline = Some(Box::new(dockerfile_inline));
        } else {
            definition.push_unmodeled(field_reference(path, field));
        }
    }
    fn set_inline<'field>(
        &mut self,
        definition: &mut ProjectBuildDefinition,
        field: &'field MergedEntry,
        path: &[String],
        inline: &mut Option<&'field MergedEntry>,
    ) {
        self.set_build_dockerfile_inline(definition, field, path, inline);
    }

    fn set_build_extra_hosts(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        let pending_start = self.pending_unmodeled.len();
        let (extra_hosts, malformed) = self.build_extra_hosts(field.value(), path);
        definition.extra_hosts = extra_hosts.map(Box::new);
        for nested in self.pending_unmodeled.drain(pending_start..) {
            definition.push_unmodeled(nested);
        }
        if malformed {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn set_build_platforms(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        let (platforms, malformed) = self.build_platforms(field.value());
        definition.platforms = platforms.map(Box::new);
        if malformed {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn set_build_cache_locations(
        &mut self,
        definition: &mut ProjectBuildDefinition,
        field: &MergedEntry,
        path: &[String],
        name: &str,
    ) {
        let (locations, malformed) = self.build_cache_locations(field.value(), name);
        if name == "cache_from" {
            definition.cache_options().cache_from = locations;
        } else {
            definition.cache_options().cache_to = locations;
        }
        if malformed {
            definition.push_unmodeled(field_reference(path, field));
        }
    }
    fn cache_locations(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        self.set_build_cache_locations(definition, field, path, field.key());
    }

    fn set_build_network(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        if let Some(network) = self.project_string(field.value(), "build network") {
            definition.network = Some(Box::new(network));
        } else {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn set_build_isolation(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        let isolation = match field.value().kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => {
                Some(ProjectValue::new(scalar.value().to_owned(), field.value()))
            }
            MergedValueKind::Scalar(_) => {
                self.invalid(
                    effective_span(field.value()),
                    "build isolation must be a YAML string scalar",
                );
                None
            }
            _ => {
                self.expected(field.value(), "build isolation must be a YAML string scalar");
                None
            }
        };
        if let Some(isolation) = isolation {
            definition.isolation = Some(Box::new(isolation));
        } else {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn set_build_pull(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        if let Some(pull) = self.boolean(field.value(), "build pull must be a boolean") {
            definition.cache_options().pull = Some(pull);
        } else {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn set_build_no_cache(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        let value = match field.value().kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::Boolean => {
                Some(BuildNoCache::Boolean(scalar.value().eq_ignore_ascii_case("true")))
            }
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => {
                Some(BuildNoCache::String(scalar.value().to_owned()))
            }
            MergedValueKind::Scalar(_) => {
                self.invalid(
                    effective_span(field.value()),
                    "build no_cache must be a YAML boolean or string scalar",
                );
                None
            }
            _ => {
                self.expected(field.value(), "build no_cache must be a YAML boolean or string scalar");
                None
            }
        };
        if let Some(value) = value {
            definition.cache_options().no_cache = Some(ProjectValue::new(value, field.value()));
        } else {
            definition.push_unmodeled(field_reference(path, field));
        }
    }
    fn set_build_no_cache_filter(
        &mut self,
        definition: &mut ProjectBuildDefinition,
        field: &MergedEntry,
        path: &[String],
    ) {
        let value = match field.value().kind() {
            MergedValueKind::Scalar(s) if s.kind() == MergedScalarKind::String => Some(
                ProjectBuildNoCacheFilter::Scalar(ProjectValue::new(s.value().to_owned(), field.value())),
            ),
            MergedValueKind::Sequence(items) => {
                let mut values = Vec::new();
                let mut bad = false;
                for item in items {
                    if let MergedValueKind::Scalar(s) = item.kind() {
                        if s.kind() == MergedScalarKind::String {
                            if values
                                .iter()
                                .any(|value: &ProjectValue<String>| value.value() == s.value())
                            {
                                self.diagnostics.push(
                                    Diagnostic::new(
                                        BUILD_NO_CACHE_FILTER_DUPLICATE_ITEM,
                                        Severity::Warning,
                                        "build no_cache_filter retains duplicate stage",
                                    )
                                    .with_label(DiagnosticLabel::primary(effective_span(item), "duplicate retained")),
                                );
                            }
                            values.push(ProjectValue::new(s.value().to_owned(), item));
                            continue;
                        }
                    }
                    self.expected(item, "build no_cache_filter entries must be string scalars");
                    bad = true;
                }
                if bad {
                    definition.push_unmodeled(field_reference(path, field));
                }
                Some(ProjectBuildNoCacheFilter::List(values))
            }
            _ => {
                self.expected(
                    field.value(),
                    "build no_cache_filter must be a string scalar or sequence",
                );
                None
            }
        };
        if let Some(value) = value {
            definition.no_cache_filter = Some(Box::new(ProjectValue::new(value, field.value())));
        } else {
            definition.push_unmodeled(field_reference(path, field));
        }
    }
    fn set_build_privileged(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        if let Some(value) = self.boolean(field.value(), "build privileged must be a boolean") {
            definition.privileged = Some(Box::new(value));
        } else {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn set_build_sbom(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        let value = match field.value().kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::Boolean => {
                Some(BuildSbom::Boolean(scalar.value().eq_ignore_ascii_case("true")))
            }
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => {
                Some(BuildSbom::String(scalar.value().to_owned()))
            }
            MergedValueKind::Scalar(_) => {
                self.invalid(
                    effective_span(field.value()),
                    "build sbom must be a YAML boolean or string scalar",
                );
                None
            }
            _ => {
                self.expected(field.value(), "build sbom must be a YAML boolean or string scalar");
                None
            }
        };
        if let Some(value) = value {
            definition.sbom = Some(Box::new(ProjectValue::new(value, field.value())));
        } else {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn set_build_attestation(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        if field.key() == "sbom" {
            self.set_build_sbom(definition, field, path);
        } else {
            self.set_build_provenance(definition, field, path);
        }
    }
    fn set_build_provenance(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        let value = match field.value().kind() {
            MergedValueKind::Scalar(s) if s.kind() == MergedScalarKind::Boolean => {
                Some(BuildProvenance::Boolean(s.value().eq_ignore_ascii_case("true")))
            }
            MergedValueKind::Scalar(s) if s.kind() == MergedScalarKind::String => {
                Some(BuildProvenance::String(s.value().to_owned()))
            }
            MergedValueKind::Scalar(_) => {
                self.invalid(
                    effective_span(field.value()),
                    "build provenance must be a YAML boolean or string scalar",
                );
                None
            }
            _ => {
                self.expected(
                    field.value(),
                    "build provenance must be a YAML boolean or string scalar",
                );
                None
            }
        };
        if let Some(value) = value {
            definition.provenance = Some(Box::new(ProjectValue::new(value, field.value())));
        } else {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn set_build_shm_size(&mut self, definition: &mut ProjectBuildDefinition, field: &MergedEntry, path: &[String]) {
        if let Some(shm_size) = self.shm_size(field.value()) {
            definition.shm_size = Some(Box::new(shm_size));
        } else {
            definition.push_unmodeled(field_reference(path, field));
        }
    }

    fn build_tags(&mut self, value: &MergedValue) -> (Option<ProjectValue<Vec<ProjectValue<String>>>>, bool) {
        let Some(values) = value.as_sequence() else {
            self.expected(value, "build tags must be a sequence of non-null scalars");
            return (None, true);
        };
        let mut tags = Vec::new();
        let mut malformed = false;
        for item in values {
            let Some(scalar) = item.as_scalar() else {
                self.expected(item, "build tag entries must be non-null scalars");
                malformed = true;
                continue;
            };
            tags.push(ProjectValue::new(scalar.value().to_owned(), item));
        }
        (Some(ProjectValue::new(tags, value)), malformed)
    }

    fn build_entitlements(&mut self, value: &MergedValue) -> (Option<ProjectValue<Vec<ProjectValue<String>>>>, bool) {
        let Some(values) = value.as_sequence() else {
            self.expected(value, "build entitlements must be a sequence of string scalars");
            return (None, true);
        };
        let mut entitlements = Vec::new();
        let mut malformed = false;
        for item in values {
            let MergedValueKind::Scalar(scalar) = item.kind() else {
                self.expected(item, "build entitlement entries must be string scalars");
                malformed = true;
                continue;
            };
            if scalar.kind() != MergedScalarKind::String {
                self.expected(item, "build entitlement entries must be string scalars");
                malformed = true;
                continue;
            }
            entitlements.push(ProjectValue::new(scalar.value().to_owned(), item));
        }
        (Some(ProjectValue::new(entitlements, value)), malformed)
    }

    fn build_platforms(&mut self, value: &MergedValue) -> (Option<ProjectValue<Vec<ProjectValue<String>>>>, bool) {
        let Some(values) = value.as_sequence() else {
            self.expected(value, "build platforms must be a sequence of non-null scalars");
            return (None, true);
        };
        let mut platforms = Vec::new();
        let mut malformed = false;
        for item in values {
            let Some(scalar) = item.as_scalar() else {
                self.expected(item, "build platform entries must be non-null scalars");
                malformed = true;
                continue;
            };
            platforms.push(ProjectValue::new(scalar.value().to_owned(), item));
        }
        (Some(ProjectValue::new(platforms, value)), malformed)
    }

    fn build_cache_locations(
        &mut self,
        value: &MergedValue,
        name: &str,
    ) -> (Option<ProjectValue<Vec<ProjectValue<String>>>>, bool) {
        let Some(values) = value.as_sequence() else {
            self.expected(value, &format!("build {name} must be a sequence of string scalars"));
            return (None, true);
        };
        let mut locations = Vec::new();
        let mut malformed = false;
        for item in values {
            let MergedValueKind::Scalar(scalar) = item.kind() else {
                self.expected(item, &format!("build {name} entries must be string scalars"));
                malformed = true;
                continue;
            };
            if scalar.kind() != MergedScalarKind::String {
                self.expected(item, &format!("build {name} entries must be string scalars"));
                malformed = true;
                continue;
            }
            locations.push(ProjectValue::new(scalar.value().to_owned(), item));
        }
        (Some(ProjectValue::new(locations, value)), malformed)
    }

    fn build_args(&mut self, value: &MergedValue) -> (Option<ProjectValue<ProjectBuildArgs>>, bool) {
        match value.kind() {
            MergedValueKind::Mapping(entries) => {
                let mut args = Vec::new();
                let mut malformed = false;
                for entry in entries {
                    let Some(value) =
                        self.compose_scalar(entry.value(), "build argument value must be a scalar or null")
                    else {
                        malformed = true;
                        continue;
                    };
                    args.push(ProjectBuildArgEntry {
                        name: ProjectKey::from_entry(entry),
                        value: ProjectValue::new(value, entry.value()),
                    });
                }
                (Some(ProjectValue::new(ProjectBuildArgs::Map(args), value)), malformed)
            }
            MergedValueKind::Sequence(values) => {
                let mut args = Vec::new();
                let mut malformed = false;
                for item in values {
                    let MergedValueKind::Scalar(argument) = item.kind() else {
                        self.expected(item, "build argument list item must be a string scalar");
                        malformed = true;
                        continue;
                    };
                    if argument.kind() != MergedScalarKind::String {
                        self.expected(item, "build argument list item must be a string scalar");
                        malformed = true;
                        continue;
                    }
                    args.push(ProjectValue::new(argument.value().to_owned(), item));
                }
                (Some(ProjectValue::new(ProjectBuildArgs::List(args), value)), malformed)
            }
            _ => {
                self.expected(value, "build args must be a mapping or sequence");
                (None, true)
            }
        }
    }

    fn build_additional_contexts(
        &mut self,
        value: &MergedValue,
    ) -> (Option<ProjectValue<ProjectBuildAdditionalContexts>>, bool) {
        match value.kind() {
            MergedValueKind::Mapping(entries) => {
                let mut contexts = Vec::new();
                let mut malformed = false;
                let mut names = BTreeSet::new();
                for entry in entries {
                    if !names.insert(entry.key().to_owned()) {
                        self.invalid(
                            entry_span(entry),
                            "build additional context mapping names must be unique",
                        );
                        malformed = true;
                        continue;
                    }
                    let Some(value) = self.compose_scalar(
                        entry.value(),
                        "build additional context mapping values must be scalars or null",
                    ) else {
                        malformed = true;
                        continue;
                    };
                    contexts.push(ProjectBuildAdditionalContextEntry {
                        name: ProjectKey::from_entry(entry),
                        value: ProjectValue::new(value, entry.value()),
                    });
                }
                (
                    Some(ProjectValue::new(ProjectBuildAdditionalContexts::Map(contexts), value)),
                    malformed,
                )
            }
            MergedValueKind::Sequence(values) => {
                let mut contexts = Vec::new();
                let mut malformed = false;
                for item in values {
                    let MergedValueKind::Scalar(context) = item.kind() else {
                        self.expected(item, "build additional context list items must be string scalars");
                        malformed = true;
                        continue;
                    };
                    if context.kind() != MergedScalarKind::String {
                        self.expected(item, "build additional context list items must be string scalars");
                        malformed = true;
                        continue;
                    }
                    contexts.push(ProjectValue::new(context.value().to_owned(), item));
                }
                (
                    Some(ProjectValue::new(ProjectBuildAdditionalContexts::List(contexts), value)),
                    malformed,
                )
            }
            _ => {
                self.expected(value, "build additional_contexts must be a mapping or sequence");
                (None, true)
            }
        }
    }

    fn build_extra_hosts(
        &mut self,
        value: &MergedValue,
        path: &[String],
    ) -> (Option<ProjectValue<ProjectBuildExtraHosts>>, bool) {
        match value.kind() {
            MergedValueKind::Sequence(values) => {
                let mut entries = Vec::new();
                let mut malformed = false;
                for item in values {
                    let MergedValueKind::Scalar(scalar) = item.kind() else {
                        self.expected(item, "build extra_hosts list entries must be string scalars");
                        malformed = true;
                        continue;
                    };
                    if scalar.kind() != MergedScalarKind::String {
                        self.expected(item, "build extra_hosts list entries must be string scalars");
                        malformed = true;
                        continue;
                    }
                    entries.push(ProjectValue::new(scalar.value().to_owned(), item));
                }
                (
                    Some(ProjectValue::new(ProjectBuildExtraHosts::List(entries), value)),
                    malformed,
                )
            }
            MergedValueKind::Mapping(entries) => {
                let mut hosts = Vec::new();
                let mut malformed = false;
                for entry in entries {
                    let mut entry_path = path.to_vec();
                    entry_path.push("extra_hosts".to_owned());
                    entry_path.push(entry.key().to_owned());
                    let (addresses, invalid_addresses) = self.build_extra_host_addresses(entry.value(), &entry_path);
                    malformed |= invalid_addresses;
                    let Some(addresses) = addresses else {
                        continue;
                    };
                    hosts.push(ProjectBuildExtraHostEntry {
                        hostname: ProjectKey::from_entry(entry),
                        addresses,
                    });
                }
                (
                    Some(ProjectValue::new(ProjectBuildExtraHosts::Map(hosts), value)),
                    malformed,
                )
            }
            _ => {
                self.expected(value, "build extra_hosts must be a mapping or sequence");
                (None, true)
            }
        }
    }

    fn build_extra_host_addresses(
        &mut self,
        value: &MergedValue,
        path: &[String],
    ) -> (Option<ProjectBuildExtraHostAddresses>, bool) {
        match value.kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => (
                Some(ProjectBuildExtraHostAddresses::Scalar(ProjectValue::new(
                    scalar.value().to_owned(),
                    value,
                ))),
                false,
            ),
            MergedValueKind::Sequence(values) => {
                let mut addresses = Vec::new();
                let mut malformed = false;
                for (index, item) in values.iter().enumerate() {
                    let MergedValueKind::Scalar(scalar) = item.kind() else {
                        self.expected(item, "build extra_hosts address-list entries must be string scalars");
                        let mut item_path = path.to_vec();
                        item_path.push(index.to_string());
                        self.pending_unmodeled.push(value_reference(&item_path, item));
                        malformed = true;
                        continue;
                    };
                    if scalar.kind() != MergedScalarKind::String {
                        self.expected(item, "build extra_hosts address-list entries must be string scalars");
                        let mut item_path = path.to_vec();
                        item_path.push(index.to_string());
                        self.pending_unmodeled.push(value_reference(&item_path, item));
                        malformed = true;
                        continue;
                    }
                    addresses.push(ProjectValue::new(scalar.value().to_owned(), item));
                }
                (Some(ProjectBuildExtraHostAddresses::List(addresses)), malformed)
            }
            _ => {
                self.expected(
                    value,
                    "build extra_hosts mapping addresses must be string scalars or sequences of string scalars",
                );
                self.pending_unmodeled.push(value_reference(path, value));
                (None, true)
            }
        }
    }

    fn build_ssh(&mut self, value: &MergedValue) -> (Option<ProjectValue<ProjectBuildSsh>>, bool) {
        match value.kind() {
            MergedValueKind::Mapping(entries) => {
                let mut ssh = Vec::new();
                let mut malformed = false;
                for entry in entries {
                    let Some(value) =
                        self.compose_scalar(entry.value(), "build ssh mapping values must be scalars or null")
                    else {
                        malformed = true;
                        continue;
                    };
                    ssh.push(ProjectBuildSshEntry {
                        name: ProjectKey::from_sensitive_entry(entry),
                        value: ProjectValue::new_sensitive(value, entry.value()),
                    });
                }
                (
                    Some(ProjectValue::new_sensitive(ProjectBuildSsh::Map(ssh), value)),
                    malformed,
                )
            }
            MergedValueKind::Sequence(values) => {
                let mut ssh = Vec::new();
                let mut malformed = false;
                for item in values {
                    let MergedValueKind::Scalar(scalar) = item.kind() else {
                        self.expected(item, "build ssh list entries must be string scalars");
                        malformed = true;
                        continue;
                    };
                    if scalar.kind() != MergedScalarKind::String {
                        self.expected(item, "build ssh list entries must be string scalars");
                        malformed = true;
                        continue;
                    }
                    ssh.push(ProjectValue::new_sensitive(scalar.value().to_owned(), item));
                }
                (
                    Some(ProjectValue::new_sensitive(ProjectBuildSsh::List(ssh), value)),
                    malformed,
                )
            }
            _ => {
                self.expected(value, "build ssh must be a mapping or sequence");
                (None, true)
            }
        }
    }

    fn build_labels(&mut self, value: &MergedValue) -> (Option<ProjectValue<ProjectBuildLabels>>, bool) {
        match value.kind() {
            MergedValueKind::Mapping(entries) => {
                let mut labels = Vec::new();
                let mut malformed = false;
                for entry in entries {
                    let Some(value) = self.compose_scalar(entry.value(), "build label value must be a scalar or null")
                    else {
                        malformed = true;
                        continue;
                    };
                    labels.push(ProjectLabelEntry {
                        name: ProjectKey::from_entry(entry),
                        value: ProjectValue::new(value, entry.value()),
                        syntax: entry.syntax(),
                    });
                }
                (
                    Some(ProjectValue::new(ProjectBuildLabels::Map(labels), value)),
                    malformed,
                )
            }
            MergedValueKind::Sequence(values) => {
                let mut labels = Vec::new();
                let mut malformed = false;
                for item in values {
                    let Some(label) = self.located_string(item, "build label list item must be a scalar") else {
                        malformed = true;
                        continue;
                    };
                    labels.push(ProjectValue::new(label.value().to_owned(), item));
                }
                (
                    Some(ProjectValue::new(ProjectBuildLabels::List(labels), value)),
                    malformed,
                )
            }
            _ => {
                self.expected(value, "build labels must be a mapping or sequence");
                (None, true)
            }
        }
    }

    fn pids_limit(&mut self, value: &MergedValue) -> Option<ProjectValue<PidsLimit>> {
        let scalar = match value.kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() != MergedScalarKind::Boolean => scalar,
            _ => {
                self.expected(value, "pids_limit must be a number or string scalar");
                return None;
            }
        };
        let limit = PidsLimit::parse(Located::new(scalar.value().to_owned(), effective_span(value)));
        match limit.kind() {
            PidsLimitKind::Zero => self.diagnostics.push(
                Diagnostic::new(
                    PIDS_LIMIT_AMBIGUOUS_ZERO,
                    Severity::Warning,
                    "pids_limit zero is preserved as an ambiguous and unportable native state",
                )
                .with_label(DiagnosticLabel::primary(
                    effective_span(value),
                    "ambiguous zero PID limit",
                )),
            ),
            PidsLimitKind::Other => self.invalid(
                effective_span(value),
                "pids_limit must be `-1`, a positive integral decimal, or interpolation",
            ),
            _ => {}
        }
        Some(ProjectValue::new(limit, value))
    }

    fn shm_size(&mut self, value: &MergedValue) -> Option<ProjectValue<ShmSize>> {
        let scalar = match value.kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::Number => {
                (scalar, ShmSizeScalarKind::Number)
            }
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => {
                (scalar, ShmSizeScalarKind::String)
            }
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        SHM_SIZE_EXPECTED_VALUE,
                        Severity::Error,
                        "shm_size must be a YAML number or string scalar",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(value),
                        "unexpected shared-memory-size form",
                    )),
                );
                return None;
            }
        };
        let size = ShmSize::parse(
            Located::new(scalar.0.value().to_owned(), effective_span(value)),
            scalar.1,
        );
        let (code, message, label, note) = match size.kind() {
            ShmSizeKind::Zero { .. } => (
                SHM_SIZE_AMBIGUOUS_ZERO,
                "shm_size zero is preserved because Compose does not define its semantics",
                "ambiguous zero shared-memory size",
                "choose a positive size with an explicit documented lowercase unit",
            ),
            ShmSizeKind::ProviderDependentNumber => (
                SHM_SIZE_PROVIDER_DEPENDENT_NUMBER,
                "numeric shm_size is schema-accepted but lacks a documented explicit unit",
                "provider-dependent numeric shared-memory size",
                "use a positive quoted value with `b`, `k`, `kb`, `m`, `mb`, `g`, or `gb` for portable intent",
            ),
            ShmSizeKind::ProviderDependentString => (
                SHM_SIZE_PROVIDER_DEPENDENT_STRING,
                "string shm_size is schema-accepted but falls outside the documented lowercase suffix family",
                "provider-dependent string shared-memory size",
                "use an explicit lowercase `b`, `k`, `kb`, `m`, `mb`, `g`, or `gb` suffix when that is the intended unit",
            ),
            ShmSizeKind::Documented { .. } | ShmSizeKind::Expression => {
                return Some(ProjectValue::new(size, value));
            }
        };
        self.diagnostics.push(
            Diagnostic::new(code, Severity::Warning, message)
                .with_label(DiagnosticLabel::primary(effective_span(value), label))
                .with_note(note),
        );
        Some(ProjectValue::new(size, value))
    }

    fn mem_limit(&mut self, value: &MergedValue) -> Option<ProjectValue<MemLimit>> {
        let scalar = match value.kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::Number => {
                (scalar, MemLimitScalarKind::Number)
            }
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => {
                (scalar, MemLimitScalarKind::String)
            }
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        MEM_LIMIT_EXPECTED_VALUE,
                        Severity::Error,
                        "mem_limit must be a YAML number or string scalar",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(value),
                        "unexpected memory-limit form",
                    )),
                );
                return None;
            }
        };
        let limit = MemLimit::parse(
            Located::new(scalar.0.value().to_owned(), effective_span(value)),
            scalar.1,
        );
        let (code, message, label, note) = match limit.kind() {
            MemLimitKind::Zero { .. } => (
                MEM_LIMIT_AMBIGUOUS_ZERO,
                "mem_limit zero is preserved without inferring portable runtime behavior",
                "ambiguous zero memory limit",
                "choose a positive size with an explicit documented lowercase unit",
            ),
            MemLimitKind::SchemaNumber => (
                MEM_LIMIT_SCHEMA_NUMBER,
                "numeric mem_limit is schema-accepted but lacks a documented explicit unit",
                "schema-only numeric memory limit",
                "use a positive quoted value with `b`, `k`, `kb`, `m`, `mb`, `g`, or `gb` for explicit intent",
            ),
            MemLimitKind::ProviderDependentString => (
                MEM_LIMIT_PROVIDER_DEPENDENT_STRING,
                "string mem_limit is schema-accepted but falls outside the documented lowercase suffix family",
                "provider-dependent string memory limit",
                "use an explicit lowercase `b`, `k`, `kb`, `m`, `mb`, `g`, or `gb` suffix when that is the intended unit",
            ),
            MemLimitKind::Documented { .. } | MemLimitKind::Expression => {
                return Some(ProjectValue::new(limit, value));
            }
        };
        self.diagnostics.push(
            Diagnostic::new(code, Severity::Warning, message)
                .with_label(DiagnosticLabel::primary(effective_span(value), label))
                .with_note(note),
        );
        Some(ProjectValue::new(limit, value))
    }

    fn dns(&mut self, value: &MergedValue) -> Option<ProjectValue<ProjectDns>> {
        let form = match value.kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => {
                ProjectDns::Scalar(ProjectValue::new(scalar.value().to_owned(), value))
            }
            MergedValueKind::Sequence(values) => {
                let mut items = Vec::new();
                for item_value in values {
                    let MergedValueKind::Scalar(scalar) = item_value.kind() else {
                        self.diagnostics.push(
                            Diagnostic::new(
                                DNS_EXPECTED_STRING,
                                Severity::Error,
                                "dns entries must be string scalars",
                            )
                            .with_label(DiagnosticLabel::primary(
                                effective_span(item_value),
                                "unexpected DNS server list item",
                            )),
                        );
                        continue;
                    };
                    if scalar.kind() != MergedScalarKind::String {
                        self.diagnostics.push(
                            Diagnostic::new(
                                DNS_EXPECTED_STRING,
                                Severity::Error,
                                "dns entries must be string scalars",
                            )
                            .with_label(DiagnosticLabel::primary(
                                effective_span(item_value),
                                "unexpected DNS server list item",
                            )),
                        );
                        continue;
                    }
                    items.push(ProjectValue::new(scalar.value().to_owned(), item_value));
                }
                ProjectDns::List(items)
            }
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        DNS_EXPECTED_FORM,
                        Severity::Error,
                        "dns must be a string scalar or a sequence of string scalars",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(value),
                        "unexpected service DNS form",
                    )),
                );
                return None;
            }
        };
        Some(ProjectValue::new(form, value))
    }

    fn dns_options(&mut self, value: &MergedValue) -> Option<ProjectValue<Vec<ProjectValue<String>>>> {
        let MergedValueKind::Sequence(values) = value.kind() else {
            self.diagnostics.push(
                Diagnostic::new(
                    DNS_OPT_EXPECTED_SEQUENCE,
                    Severity::Error,
                    "dns_opt must be a sequence of string scalars",
                )
                .with_label(DiagnosticLabel::primary(
                    effective_span(value),
                    "unexpected service DNS option form",
                )),
            );
            return None;
        };
        let mut items = Vec::new();
        let mut seen = BTreeSet::new();
        for item_value in values {
            let MergedValueKind::Scalar(scalar) = item_value.kind() else {
                self.diagnostics.push(
                    Diagnostic::new(
                        DNS_OPT_EXPECTED_STRING,
                        Severity::Error,
                        "dns_opt entries must be string scalars",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(item_value),
                        "unexpected DNS option list item",
                    )),
                );
                continue;
            };
            if scalar.kind() != MergedScalarKind::String {
                self.diagnostics.push(
                    Diagnostic::new(
                        DNS_OPT_EXPECTED_STRING,
                        Severity::Error,
                        "dns_opt entries must be string scalars",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(item_value),
                        "unexpected DNS option list item",
                    )),
                );
                continue;
            }
            if !seen.insert(scalar.value().to_owned()) {
                self.diagnostics.push(
                    Diagnostic::new(
                        DNS_OPT_DUPLICATE_ITEM,
                        Severity::Warning,
                        "dns_opt entries must be unique exact strings",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(item_value),
                        "duplicate DNS option retained",
                    )),
                );
            }
            items.push(ProjectValue::new(scalar.value().to_owned(), item_value));
        }
        Some(ProjectValue::new(items, value))
    }

    fn dns_search(&mut self, value: &MergedValue) -> Option<ProjectValue<ProjectDnsSearch>> {
        let form = match value.kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => {
                ProjectDnsSearch::Scalar(ProjectValue::new(scalar.value().to_owned(), value))
            }
            MergedValueKind::Sequence(values) => {
                let mut items = Vec::new();
                let mut seen = BTreeSet::new();
                for item_value in values {
                    let MergedValueKind::Scalar(scalar) = item_value.kind() else {
                        self.diagnostics.push(
                            Diagnostic::new(
                                DNS_SEARCH_EXPECTED_STRING,
                                Severity::Error,
                                "dns_search entries must be string scalars",
                            )
                            .with_label(DiagnosticLabel::primary(
                                effective_span(item_value),
                                "unexpected DNS search-domain list item",
                            )),
                        );
                        continue;
                    };
                    if scalar.kind() != MergedScalarKind::String {
                        self.diagnostics.push(
                            Diagnostic::new(
                                DNS_SEARCH_EXPECTED_STRING,
                                Severity::Error,
                                "dns_search entries must be string scalars",
                            )
                            .with_label(DiagnosticLabel::primary(
                                effective_span(item_value),
                                "unexpected DNS search-domain list item",
                            )),
                        );
                        continue;
                    }
                    if !seen.insert(scalar.value().to_owned()) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                DNS_SEARCH_DUPLICATE_ITEM,
                                Severity::Warning,
                                "dns_search schema entries are unique, but duplicate merge behavior is ambiguous",
                            )
                            .with_label(DiagnosticLabel::primary(
                                effective_span(item_value),
                                "duplicate DNS search domain retained",
                            )),
                        );
                    }
                    items.push(ProjectValue::new(scalar.value().to_owned(), item_value));
                }
                ProjectDnsSearch::List(items)
            }
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        DNS_SEARCH_EXPECTED_FORM,
                        Severity::Error,
                        "dns_search must be a string scalar or a sequence of string scalars",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(value),
                        "unexpected service DNS search-domain form",
                    )),
                );
                return None;
            }
        };
        Some(ProjectValue::new(form, value))
    }

    fn expose(&mut self, value: &MergedValue) -> Option<ProjectValue<Vec<ProjectValue<ProjectExposeItem>>>> {
        let MergedValueKind::Sequence(values) = value.kind() else {
            self.diagnostics.push(
                Diagnostic::new(
                    EXPOSE_EXPECTED_SEQUENCE,
                    Severity::Error,
                    "expose must be a sequence of string or number scalars",
                )
                .with_label(DiagnosticLabel::primary(
                    effective_span(value),
                    "unexpected service expose form",
                )),
            );
            return None;
        };
        let mut items = Vec::new();
        let mut seen = Vec::new();
        for item_value in values {
            let MergedValueKind::Scalar(scalar) = item_value.kind() else {
                self.diagnostics.push(
                    Diagnostic::new(
                        EXPOSE_EXPECTED_SCALAR,
                        Severity::Error,
                        "expose entries must be string or number scalars",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(item_value),
                        "unexpected exposed-port item",
                    )),
                );
                continue;
            };
            let scalar_kind = match scalar.kind() {
                MergedScalarKind::String => ExposeScalarKind::String,
                MergedScalarKind::Number => ExposeScalarKind::Number,
                MergedScalarKind::Boolean => {
                    self.diagnostics.push(
                        Diagnostic::new(
                            EXPOSE_EXPECTED_SCALAR,
                            Severity::Error,
                            "expose entries must be string or number scalars",
                        )
                        .with_label(DiagnosticLabel::primary(
                            effective_span(item_value),
                            "unexpected exposed-port item",
                        )),
                    );
                    continue;
                }
            };
            if seen.contains(&(scalar_kind, scalar.value().to_owned())) {
                self.diagnostics.push(
                    Diagnostic::new(
                        EXPOSE_DUPLICATE_ITEM,
                        Severity::Warning,
                        "expose entries must be unique by exact scalar identity",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(item_value),
                        "duplicate exposed-port item retained",
                    )),
                );
            } else {
                seen.push((scalar_kind, scalar.value().to_owned()));
            }
            let kind = classify_expose_item(scalar.value(), scalar_kind);
            self.diagnose_expose_item(&kind, item_value);
            items.push(ProjectValue::new(
                ProjectExposeItem {
                    authored: scalar.raw().to_owned(),
                    value: scalar.value().to_owned(),
                    scalar_kind,
                    kind,
                },
                item_value,
            ));
        }
        Some(ProjectValue::new(items, value))
    }

    fn diagnose_expose_item(&mut self, kind: &ExposeItemKind, value: &MergedValue) {
        let (code, severity, message, label) = match kind {
            ExposeItemKind::Documented { .. } | ExposeItemKind::Expression => return,
            ExposeItemKind::Sctp { .. } | ExposeItemKind::UnknownProtocol { .. } => (
                EXPOSE_PROVIDER_DEPENDENT,
                Severity::Warning,
                "expose protocol is outside the documented portable `tcp` and `udp` set",
                "provider-dependent exposed-port protocol retained",
            ),
            ExposeItemKind::Malformed => (
                EXPOSE_INVALID_ITEM,
                Severity::Error,
                "expose item must be a decimal port or range with an optional protocol",
                "malformed exposed-port item retained",
            ),
        };
        self.diagnostics.push(
            Diagnostic::new(code, severity, message).with_label(DiagnosticLabel::primary(effective_span(value), label)),
        );
    }

    fn security_options(
        &mut self,
        value: &MergedValue,
    ) -> Option<ProjectValue<Vec<ProjectValue<ProjectSecurityOptionItem>>>> {
        let MergedValueKind::Sequence(values) = value.kind() else {
            self.diagnostics.push(
                Diagnostic::new(
                    SECURITY_OPT_EXPECTED_SEQUENCE,
                    Severity::Error,
                    "security_opt must be a sequence of string scalars",
                )
                .with_label(DiagnosticLabel::primary(
                    effective_span(value),
                    "unexpected service security-option form",
                )),
            );
            return None;
        };
        let mut items = Vec::new();
        let mut candidates = SecurityOptionCandidateCounts::default();
        for item_value in values {
            let MergedValueKind::Scalar(scalar) = item_value.kind() else {
                self.diagnostics.push(
                    Diagnostic::new(
                        SECURITY_OPT_EXPECTED_STRING,
                        Severity::Error,
                        "security_opt entries must be string scalars",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(item_value),
                        "unexpected security-option item retained in source evidence",
                    )),
                );
                continue;
            };
            if scalar.kind() != MergedScalarKind::String {
                self.diagnostics.push(
                    Diagnostic::new(
                        SECURITY_OPT_EXPECTED_STRING,
                        Severity::Error,
                        "security_opt entries must be string scalars",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(item_value),
                        "unexpected security-option scalar kind retained in source evidence",
                    )),
                );
                continue;
            }
            let kind = classify_security_option(scalar.value());
            self.diagnose_security_option_item(&kind, effective_span(item_value), &mut candidates);
            items.push(ProjectValue::new(
                ProjectSecurityOptionItem {
                    authored: scalar.raw().to_owned(),
                    value: scalar.value().to_owned(),
                    scalar_kind: scalar.kind(),
                    kind,
                },
                item_value,
            ));
        }
        Some(ProjectValue::new(items, value))
    }

    fn diagnose_security_option_item(
        &mut self,
        kind: &SecurityOptionKind,
        span: SourceSpan,
        candidates: &mut SecurityOptionCandidateCounts,
    ) {
        let diagnostic = match kind {
            SecurityOptionKind::AppArmor { .. } => {
                candidates.apparmor += 1;
                (candidates.apparmor > 1).then(|| {
                    Diagnostic::new(
                        SECURITY_OPT_APPARMOR_CONFLICT,
                        Severity::Warning,
                        "multiple AppArmor candidates are retained; a consumer must resolve the conflict explicitly",
                    )
                    .with_label(DiagnosticLabel::primary(
                        span,
                        "additional effective AppArmor candidate retained",
                    ))
                })
            }
            SecurityOptionKind::AppArmorNearMiss => Some(
                Diagnostic::new(
                    SECURITY_OPT_APPARMOR_NEAR_MISS,
                    Severity::Warning,
                    "AppArmor candidates require exact lowercase `apparmor=<profile>` spelling without whitespace",
                )
                .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
            ),
            SecurityOptionKind::Seccomp { .. } => {
                candidates.seccomp += 1;
                (candidates.seccomp > 1).then(|| {
                    Diagnostic::new(
                        SECURITY_OPT_SECCOMP_CONFLICT,
                        Severity::Warning,
                        "multiple seccomp candidates are retained; a consumer must resolve the conflict explicitly",
                    )
                    .with_label(DiagnosticLabel::primary(
                        span,
                        "additional effective seccomp candidate retained",
                    ))
                })
            }
            SecurityOptionKind::SeccompNearMiss => Some(
                Diagnostic::new(
                    SECURITY_OPT_SECCOMP_NEAR_MISS,
                    Severity::Warning,
                    "seccomp candidates require exact lowercase `seccomp=<profile>` spelling without whitespace",
                )
                .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
            ),
            SecurityOptionKind::NoNewPrivileges { .. } => {
                candidates.no_new_privileges += 1;
                (candidates.no_new_privileges > 1).then(|| {
                    Diagnostic::new(
                        SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT,
                        Severity::Warning,
                        "multiple no-new-privileges candidates are retained; a consumer must resolve the conflict explicitly",
                    )
                    .with_label(DiagnosticLabel::primary(
                        span,
                        "additional effective no-new-privileges candidate retained",
                    ))
                })
            }
            SecurityOptionKind::NoNewPrivilegesNearMiss => Some(
                Diagnostic::new(
                    SECURITY_OPT_NO_NEW_PRIVILEGES_NEAR_MISS,
                    Severity::Warning,
                    "no-new-privileges candidates require exact lowercase `no-new-privileges:true` or `no-new-privileges:false` spelling without whitespace",
                )
                .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
            ),
            SecurityOptionKind::Mask { .. }
            | SecurityOptionKind::MaskNearMiss
            | SecurityOptionKind::Unmask { .. }
            | SecurityOptionKind::UnmaskNearMiss => security_path_option_diagnostic(kind, span),
            SecurityOptionKind::SecurityLabelDisable { .. }
            | SecurityOptionKind::SecurityLabelDisableNearMiss
            | SecurityOptionKind::SecurityLabelFileType { .. }
            | SecurityOptionKind::SecurityLabelFileTypeNearMiss
            | SecurityOptionKind::SecurityLabelLevel { .. }
            | SecurityOptionKind::SecurityLabelLevelNearMiss
            | SecurityOptionKind::SecurityLabelNested { .. }
            | SecurityOptionKind::SecurityLabelNestedNearMiss
            | SecurityOptionKind::SecurityLabelType { .. }
            | SecurityOptionKind::SecurityLabelTypeNearMiss => {
                effective_security_label_diagnostic(kind, span, candidates)
            }
            SecurityOptionKind::Empty => Some(
                Diagnostic::new(
                    SECURITY_OPT_EMPTY_ITEM,
                    Severity::Error,
                    "security_opt entries must not be empty strings",
                )
                .with_label(DiagnosticLabel::primary(span, "empty security option retained")),
            ),
            SecurityOptionKind::Expression | SecurityOptionKind::Other => None,
        };
        if let Some(diagnostic) = diagnostic {
            self.diagnostics.push(diagnostic);
        }
    }

    fn tmpfs(&mut self, value: &MergedValue) -> Option<ProjectValue<ProjectTmpfs>> {
        let form = match value.kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => {
                let item = self.tmpfs_item(value, scalar.value());
                ProjectTmpfs::Scalar(ProjectValue::new(item, value))
            }
            MergedValueKind::Sequence(values) => {
                let mut items = Vec::new();
                for item_value in values {
                    let MergedValueKind::Scalar(scalar) = item_value.kind() else {
                        self.diagnostics.push(
                            Diagnostic::new(
                                TMPFS_EXPECTED_STRING,
                                Severity::Error,
                                "tmpfs entries must be string scalars",
                            )
                            .with_label(DiagnosticLabel::primary(
                                effective_span(item_value),
                                "unexpected temporary-filesystem list item",
                            )),
                        );
                        continue;
                    };
                    if scalar.kind() != MergedScalarKind::String {
                        self.diagnostics.push(
                            Diagnostic::new(
                                TMPFS_EXPECTED_STRING,
                                Severity::Error,
                                "tmpfs entries must be string scalars",
                            )
                            .with_label(DiagnosticLabel::primary(
                                effective_span(item_value),
                                "unexpected temporary-filesystem list item",
                            )),
                        );
                        continue;
                    }
                    let item = self.tmpfs_item(item_value, scalar.value());
                    items.push(ProjectValue::new(item, item_value));
                }
                ProjectTmpfs::List(items)
            }
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        TMPFS_EXPECTED_FORM,
                        Severity::Error,
                        "tmpfs must be a string scalar or a sequence of string scalars",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(value),
                        "unexpected service-level temporary-filesystem form",
                    )),
                );
                return None;
            }
        };
        Some(ProjectValue::new(form, value))
    }

    fn tmpfs_item(&mut self, source: &MergedValue, raw: &str) -> TmpfsItem {
        let item = TmpfsItem::parse(Located::new(raw.to_owned(), effective_span(source)));
        if item.kind() == TmpfsItemKind::ProviderDependent {
            self.diagnostics.push(
                Diagnostic::new(
                    TMPFS_PROVIDER_DEPENDENT,
                    Severity::Warning,
                    "tmpfs item is malformed or uses provider- or target-specific options",
                )
                .with_label(DiagnosticLabel::primary(
                    effective_span(source),
                    "provider-dependent temporary-filesystem item",
                ))
                .with_note("use a non-empty path with only non-empty `mode`, `uid`, or `gid` assignments for documented portable syntax"),
            );
        }
        item
    }

    fn sysctls(&mut self, value: &MergedValue) -> Option<ProjectValue<ProjectSysctls>> {
        let form = match value.kind() {
            MergedValueKind::Mapping(entries) => ProjectSysctls::Map(self.sysctls_map(entries)),
            MergedValueKind::Sequence(items) => ProjectSysctls::List(self.sysctls_list(items)),
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        SYSCTLS_EXPECTED_FORM,
                        Severity::Error,
                        "sysctls must be a mapping or a sequence of string scalars",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(value),
                        "unexpected service sysctls form",
                    )),
                );
                return None;
            }
        };
        Some(ProjectValue::new(form, value))
    }

    fn logging(&mut self, value: &MergedValue, service_path: &[String]) -> Option<ProjectValue<ProjectLogging>> {
        let Some(fields) = value.as_mapping() else {
            self.diagnostics.push(
                Diagnostic::new(
                    LOGGING_EXPECTED_MAPPING,
                    Severity::Error,
                    "logging must be a mapping with optional driver and options fields",
                )
                .with_label(DiagnosticLabel::primary(
                    effective_span(value),
                    "unexpected logging form",
                )),
            );
            return None;
        };
        let mut driver = None;
        let mut options = None;
        let mut unmodeled_fields = Vec::new();
        let mut path = service_path.to_vec();
        path.push("logging".to_owned());
        for field in fields {
            match field.key() {
                "driver" => {
                    let Some(scalar) = field.value().as_scalar() else {
                        self.logging_driver_finding(field.value());
                        unmodeled_fields.push(field_reference(&path, field));
                        continue;
                    };
                    if scalar.kind() != MergedScalarKind::String {
                        self.logging_driver_finding(field.value());
                        unmodeled_fields.push(field_reference(&path, field));
                        continue;
                    }
                    driver = Some(ProjectValue::new(scalar.value().to_owned(), field.value()));
                }
                "options" => {
                    let Some(entries) = field.value().as_mapping() else {
                        self.diagnostics.push(
                            Diagnostic::new(
                                LOGGING_OPTIONS_EXPECTED_MAPPING,
                                Severity::Error,
                                "logging options must be a mapping",
                            )
                            .with_label(DiagnosticLabel::primary(
                                effective_span(field.value()),
                                "unexpected logging options form",
                            )),
                        );
                        unmodeled_fields.push(field_reference(&path, field));
                        continue;
                    };
                    options = Some(self.logging_options(field.value(), entries, &path));
                }
                _ => unmodeled_fields.push(field_reference(&path, field)),
            }
        }
        Some(ProjectValue::new(
            ProjectLogging {
                driver,
                options,
                unmodeled_fields,
            },
            value,
        ))
    }

    fn logging_driver_finding(&mut self, value: &MergedValue) {
        self.diagnostics.push(
            Diagnostic::new(
                LOGGING_DRIVER_EXPECTED_STRING,
                Severity::Error,
                "logging driver must be a YAML string scalar",
            )
            .with_label(DiagnosticLabel::primary(
                effective_span(value),
                "non-string logging driver retained as unmodeled evidence",
            )),
        );
    }

    fn logging_options(
        &mut self,
        source: &MergedValue,
        entries: &[MergedEntry],
        logging_path: &[String],
    ) -> ProjectValue<ProjectLoggingOptions> {
        let mut values = Vec::new();
        let mut unmodeled_entries = Vec::new();
        let mut path = logging_path.to_vec();
        path.push("options".to_owned());
        for entry in entries {
            if entry.key().is_empty() {
                self.diagnostics.push(
                    Diagnostic::new(
                        LOGGING_OPTION_EMPTY_KEY,
                        Severity::Error,
                        "logging option keys must not be empty",
                    )
                    .with_label(DiagnosticLabel::primary(entry_span(entry), "empty logging option key")),
                );
                unmodeled_entries.push(field_reference(&path, entry));
                continue;
            }
            let Some(value) = self.logging_option_value(entry.value()) else {
                unmodeled_entries.push(field_reference(&path, entry));
                continue;
            };
            let option = ProjectLoggingOption {
                name: ProjectKey::from_entry(entry),
                value: ProjectValue::new(value, entry.value()),
            };
            values.push(ProjectValue::new(option, entry.value()));
        }
        ProjectValue::new(
            ProjectLoggingOptions {
                entries: values,
                unmodeled_entries,
            },
            source,
        )
    }

    fn logging_option_value(&mut self, value: &MergedValue) -> Option<ProjectLoggingOptionValue> {
        let parsed = match value.kind() {
            MergedValueKind::Null(_) => ProjectLoggingOptionValue::Null,
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => {
                ProjectLoggingOptionValue::String {
                    authored: scalar.raw().to_owned(),
                    value: scalar.value().to_owned(),
                }
            }
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::Number => {
                ProjectLoggingOptionValue::Number(scalar.value().to_owned())
            }
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        LOGGING_OPTION_EXPECTED_SCALAR,
                        Severity::Error,
                        "logging option values must be YAML string, number, or null scalars",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(value),
                        "unsupported logging option value retained as unmodeled evidence",
                    )),
                );
                return None;
            }
        };
        Some(parsed)
    }

    fn sysctls_map(&mut self, entries: &[MergedEntry]) -> Vec<ProjectValue<ProjectSysctl>> {
        let mut sysctls = Vec::new();
        for entry in entries {
            if entry.key().is_empty() {
                self.diagnostics.push(
                    Diagnostic::new(
                        SYSCTLS_EMPTY_KEY,
                        Severity::Error,
                        "sysctls mapping keys must not be empty",
                    )
                    .with_label(DiagnosticLabel::primary(entry_span(entry), "empty sysctl name")),
                );
                continue;
            }
            let Some(scalar) = self.sysctl_scalar(entry.value()) else {
                continue;
            };
            let sysctl = ProjectSysctl {
                name: ProjectKey::from_entry(entry),
                value: ProjectValue::new(scalar, entry.value()),
            };
            sysctls.push(ProjectValue::new(sysctl, entry.value()));
        }
        sysctls
    }

    fn sysctl_scalar(&mut self, value: &MergedValue) -> Option<ComposeScalar> {
        match value.kind() {
            MergedValueKind::Null(_) => Some(ComposeScalar::Null),
            MergedValueKind::Scalar(scalar) => Some(match scalar.kind() {
                MergedScalarKind::String => ComposeScalar::String(scalar.value().to_owned()),
                MergedScalarKind::Boolean => ComposeScalar::Boolean(scalar.value().eq_ignore_ascii_case("true")),
                MergedScalarKind::Number => ComposeScalar::Number(scalar.value().to_owned()),
            }),
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        SYSCTLS_EXPECTED_SCALAR,
                        Severity::Error,
                        "sysctls mapping values must be scalar strings, numbers, booleans, or null",
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(value),
                        "non-scalar sysctl value",
                    )),
                );
                None
            }
        }
    }

    fn sysctls_list(&mut self, items: &[MergedValue]) -> Vec<ProjectValue<String>> {
        let mut sysctls = Vec::new();
        let mut seen = BTreeMap::new();
        for item in items {
            let MergedValueKind::Scalar(scalar) = item.kind() else {
                self.invalid_sysctl_list_item(item);
                continue;
            };
            if scalar.kind() != MergedScalarKind::String {
                self.invalid_sysctl_list_item(item);
                continue;
            }
            let span = effective_span(item);
            if let Some(first) = seen.get(scalar.value()) {
                self.diagnostics.push(
                    Diagnostic::new(
                        SYSCTLS_DUPLICATE_ITEM,
                        Severity::Error,
                        "effective sysctls list entries must be unique exact strings",
                    )
                    .with_label(DiagnosticLabel::primary(span, "duplicate sysctl string"))
                    .with_label(DiagnosticLabel::secondary(*first, "first identical string")),
                );
            } else {
                seen.insert(scalar.value().to_owned(), span);
            }
            sysctls.push(ProjectValue::new(scalar.value().to_owned(), item));
        }
        sysctls
    }

    fn invalid_sysctl_list_item(&mut self, value: &MergedValue) {
        self.diagnostics.push(
            Diagnostic::new(
                SYSCTLS_EXPECTED_STRING,
                Severity::Error,
                "sysctls list entries must be YAML string scalars",
            )
            .with_label(DiagnosticLabel::primary(
                effective_span(value),
                "non-string sysctl list item",
            )),
        );
    }

    fn ulimits(
        &mut self,
        value: &MergedValue,
        service_path: &[String],
    ) -> (Option<ProjectValue<ProjectUlimits>>, Vec<ProjectFieldReference>) {
        let Some(entries) = value.as_mapping() else {
            self.expected(value, "ulimits must be a mapping");
            return (None, Vec::new());
        };
        let mut limits = Vec::new();
        let mut unmodeled = Vec::new();
        let mut path = service_path.to_vec();
        path.push("ulimits".to_owned());
        for entry in entries {
            if !valid_ulimit_name(entry.key()) {
                self.diagnostics.push(
                    Diagnostic::new(
                        ULIMIT_INVALID_NAME,
                        Severity::Error,
                        "ulimit names must contain only lowercase ASCII letters",
                    )
                    .with_label(DiagnosticLabel::primary(entry_span(entry), "invalid ulimit name")),
                );
                unmodeled.push(field_reference(&path, entry));
                continue;
            }
            let Some(limit) = self.ulimit(entry, &path) else {
                unmodeled.push(field_reference(&path, entry));
                continue;
            };
            limits.push(ProjectValue::new(limit, entry.value()));
        }
        (
            Some(ProjectValue::new(ProjectUlimits { entries: limits }, value)),
            unmodeled,
        )
    }

    fn ulimit(&mut self, entry: &MergedEntry, parent_path: &[String]) -> Option<ProjectUlimit> {
        let value = match entry.value().kind() {
            MergedValueKind::Scalar(_) => ProjectUlimitValue::Single(
                self.ulimit_scalar(entry.value())
                    .map(|scalar| ProjectValue::new(scalar, entry.value()))?,
            ),
            MergedValueKind::Mapping(fields) => {
                let mut soft = None;
                let mut hard = None;
                let mut unmodeled_fields = Vec::new();
                let mut range_path = parent_path.to_vec();
                range_path.push(entry.key().to_owned());
                for field in fields {
                    match field.key() {
                        "soft" => {
                            let parsed = self
                                .ulimit_scalar(field.value())
                                .map(|scalar| ProjectValue::new(scalar, field.value()));
                            if parsed.is_none() {
                                unmodeled_fields.push(field_reference(&range_path, field));
                            }
                            soft = parsed;
                        }
                        "hard" => {
                            let parsed = self
                                .ulimit_scalar(field.value())
                                .map(|scalar| ProjectValue::new(scalar, field.value()));
                            if parsed.is_none() {
                                unmodeled_fields.push(field_reference(&range_path, field));
                            }
                            hard = parsed;
                        }
                        _ => unmodeled_fields.push(field_reference(&range_path, field)),
                    }
                }
                if soft.is_none() {
                    self.diagnostics.push(
                        Diagnostic::new(
                            ULIMIT_MISSING_RANGE_MEMBER,
                            Severity::Error,
                            "ulimit range is missing required `soft`",
                        )
                        .with_label(DiagnosticLabel::primary(
                            effective_span(entry.value()),
                            "missing soft limit",
                        )),
                    );
                }
                if hard.is_none() {
                    self.diagnostics.push(
                        Diagnostic::new(
                            ULIMIT_MISSING_RANGE_MEMBER,
                            Severity::Error,
                            "ulimit range is missing required `hard`",
                        )
                        .with_label(DiagnosticLabel::primary(
                            effective_span(entry.value()),
                            "missing hard limit",
                        )),
                    );
                }
                ProjectUlimitValue::Range(ProjectUlimitRange {
                    soft,
                    hard,
                    unmodeled_fields,
                })
            }
            _ => {
                self.expected(
                    entry.value(),
                    "ulimit must be a number/string scalar or a soft/hard mapping",
                );
                return None;
            }
        };
        Some(ProjectUlimit {
            name: ProjectKey::from_entry(entry),
            value,
        })
    }

    fn ulimit_scalar(&mut self, value: &MergedValue) -> Option<ProjectUlimitScalar> {
        let Some(scalar) = value.as_scalar() else {
            self.expected(value, "ulimit values must be number or string scalars");
            return None;
        };
        if !matches!(scalar.kind(), MergedScalarKind::String | MergedScalarKind::Number) {
            self.diagnostics.push(
                Diagnostic::new(
                    ULIMIT_INVALID_VALUE,
                    Severity::Error,
                    "ulimit values must be number or string scalars",
                )
                .with_label(DiagnosticLabel::primary(
                    effective_span(value),
                    "invalid ulimit scalar kind",
                )),
            );
            return None;
        }
        let parsed = LimitValue::parse(scalar.value().to_owned());
        if !parsed.is_valid() {
            self.diagnostics.push(
                Diagnostic::new(
                    ULIMIT_INVALID_VALUE,
                    Severity::Error,
                    "ulimit must be -1, a non-negative integer, or an interpolation expression",
                )
                .with_label(DiagnosticLabel::primary(effective_span(value), "invalid ulimit value")),
            );
        }
        Some(ProjectUlimitScalar {
            authored: scalar.raw().to_owned(),
            value: parsed,
            kind: scalar.kind(),
        })
    }

    fn pull_policy(&mut self, value: &MergedValue) -> Option<ProjectValue<PullPolicy>> {
        let policy = PullPolicy::parse(self.located_string(value, "pull_policy must be a non-null scalar")?);
        if !policy.is_recognized() {
            self.invalid(
                effective_span(value),
                "pull_policy must be a documented Compose policy, the retained `if_not_present` alias, schema-only `refresh`, an `every_` interval matching integer `w`, `d`, `h`, `m`, and `s` components, or interpolation",
            );
        }
        Some(ProjectValue::new(policy, value))
    }

    fn stop_grace_period(&mut self, value: &MergedValue) -> Option<ProjectValue<StopGracePeriod>> {
        let scalar = self.scalar(value, "stop_grace_period must be a non-null scalar")?;
        let period = StopGracePeriod::parse(scalar.value().to_owned());
        if !period.is_valid() {
            self.invalid(
                effective_span(value),
                "stop_grace_period must match the ComposeLens duration policy using `us`, `ms`, `s`, `m`, or `h`, or contain an interpolation marker",
            );
        }
        Some(ProjectValue::new(period, value))
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

    fn entrypoint(&mut self, value: &MergedValue) -> Option<ProjectValue<Entrypoint>> {
        let span = effective_span(value);
        let entrypoint = match value.kind() {
            MergedValueKind::Null(_) => Entrypoint::Null(span),
            MergedValueKind::Scalar(scalar) => Entrypoint::String(Located::new(scalar.value().to_owned(), span)),
            MergedValueKind::Sequence(values) => {
                let mut arguments = Vec::new();
                for value in values {
                    arguments.push(self.located_string(value, "entrypoint list item must be a scalar")?);
                }
                Entrypoint::List {
                    span,
                    values: arguments,
                }
            }
            _ => {
                self.expected(value, "entrypoint must be null, a scalar, or a sequence");
                return None;
            }
        };
        Some(ProjectValue::new(entrypoint, value))
    }

    fn user(&mut self, value: &MergedValue) -> Option<ProjectValue<UserSpec>> {
        let raw = self.project_string(value, "service user")?;
        Some(ProjectValue {
            value: UserSpec::parse(Located::new(raw.value, effective_span(value))),
            provenance: raw.provenance,
            sensitive: raw.sensitive,
        })
    }

    fn userns_mode(&mut self, value: &MergedValue) -> Option<ProjectValue<UserNamespaceMode>> {
        let raw = self.project_string(value, "service user namespace mode")?;
        Some(ProjectValue {
            value: UserNamespaceMode::parse(Located::new(raw.value, effective_span(value))),
            provenance: raw.provenance,
            sensitive: raw.sensitive,
        })
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

    fn environment_files(
        &mut self,
        value: &MergedValue,
        service_path: &[String],
    ) -> Option<ProjectValue<Vec<ProjectValue<ProjectEnvironmentFile>>>> {
        let values = match value.kind() {
            MergedValueKind::Scalar(_) => std::slice::from_ref(value),
            MergedValueKind::Sequence(values) => values,
            _ => {
                self.expected(
                    value,
                    "env_file must be a scalar path or sequence of short/long entries",
                );
                return None;
            }
        };
        let mut environment_files = Vec::new();
        for (index, item) in values.iter().enumerate() {
            let mut path = service_path.to_vec();
            path.push("env_file".to_owned());
            path.push(index.to_string());
            let environment_file = match item.kind() {
                MergedValueKind::Scalar(scalar) => ProjectEnvironmentFile::Short(scalar.value().to_owned()),
                MergedValueKind::Mapping(fields) => {
                    ProjectEnvironmentFile::Long(Box::new(self.long_environment_file(item, fields, &path)))
                }
                _ => {
                    self.expected(
                        item,
                        "env_file item must use scalar short syntax or mapping long syntax",
                    );
                    continue;
                }
            };
            environment_files.push(ProjectValue::new(environment_file, item));
        }
        Some(ProjectValue::new(environment_files, value))
    }

    fn long_environment_file(
        &mut self,
        value: &MergedValue,
        fields: &[MergedEntry],
        path: &[String],
    ) -> ProjectLongEnvironmentFile {
        let mut environment_file = ProjectLongEnvironmentFile {
            path: None,
            required: None,
            format: None,
            unmodeled_fields: Vec::new(),
        };
        for field in fields {
            match field.key() {
                "path" => {
                    environment_file.path = self.project_string(field.value(), "environment-file path");
                }
                "required" => {
                    environment_file.required = self
                        .located_boolean(field.value(), "environment-file required option must be a boolean")
                        .map(|value| ProjectValue::new(value.into_value(), field.value()));
                }
                "format" => {
                    environment_file.format = self.environment_file_format(field.value());
                }
                _ => environment_file.unmodeled_fields.push(field_reference(path, field)),
            }
        }
        if environment_file.path.is_none() {
            self.missing(value, "long-syntax environment file is missing `path`");
        }
        environment_file
    }

    fn environment_file_format(&mut self, value: &MergedValue) -> Option<ProjectValue<EnvironmentFileFormat>> {
        let raw = self.project_string(value, "environment-file format")?;
        let format = EnvironmentFileFormat::parse(Located::new(raw.value, effective_span(value)));
        if matches!(format.kind(), EnvironmentFileFormatKind::Other) {
            self.invalid(
                effective_span(value),
                "environment-file format must be `raw` or interpolation",
            );
        }
        Some(ProjectValue {
            value: format,
            provenance: raw.provenance,
            sensitive: raw.sensitive,
        })
    }

    fn service_labels(&mut self, value: &MergedValue) -> Option<ProjectValue<ProjectLabels>> {
        let mut entries = Vec::new();
        let form = match value.kind() {
            MergedValueKind::Mapping(values) => {
                for entry in values {
                    let scalar = if entry.syntax() == EntrySyntax::ListKeyOnly {
                        ComposeScalar::String(String::new())
                    } else {
                        self.compose_scalar(entry.value(), "label value must be a scalar or null")?
                    };
                    entries.push(ProjectLabelEntry {
                        name: ProjectKey::from_entry(entry),
                        value: ProjectValue::new(scalar, entry.value()),
                        syntax: entry.syntax(),
                    });
                }
                ProjectLabelsForm::Map
            }
            MergedValueKind::Sequence(values) => {
                for item in values {
                    let raw = self.located_string(item, "label list item must be a scalar")?;
                    let (name, value, syntax) = raw.value().split_once('=').map_or_else(
                        || (raw.value().clone(), String::new(), EntrySyntax::ListKeyOnly),
                        |(name, value)| (name.to_owned(), value.to_owned(), EntrySyntax::ListKeyValue),
                    );
                    entries.push(ProjectLabelEntry {
                        name: ProjectKey::from_value(name, item),
                        value: ProjectValue::new(ComposeScalar::String(value), item),
                        syntax,
                    });
                }
                ProjectLabelsForm::List
            }
            _ => {
                self.expected(value, "labels must be a mapping or sequence");
                return None;
            }
        };
        Some(ProjectValue::new(ProjectLabels { form, entries }, value))
    }

    fn service_annotations(&mut self, value: &MergedValue) -> Option<ProjectValue<ProjectAnnotations>> {
        let entries = match value.kind() {
            MergedValueKind::Mapping(values) => self.annotation_mapping(values),
            MergedValueKind::Sequence(values) => self.annotation_sequence(values),
            _ => {
                self.expected(value, "annotations must be a mapping or sequence");
                return None;
            }
        };
        Some(ProjectValue::new(ProjectAnnotations { entries }, value))
    }

    fn annotation_mapping(&mut self, values: &[MergedEntry]) -> Vec<ProjectAnnotationEntry> {
        let mut entries = Vec::new();
        for entry in values {
            if entry.key().is_empty() {
                self.annotation_finding(
                    ANNOTATIONS_EMPTY_NAME,
                    Severity::Error,
                    entry.value(),
                    "service annotation name must not be empty",
                    "empty annotation name",
                );
            }
            if entry
                .raw_list_item()
                .is_some_and(|raw| raw.kind() != MergedScalarKind::String)
            {
                self.annotation_finding(
                    ANNOTATIONS_EXPECTED_STRING,
                    Severity::Error,
                    entry.value(),
                    "annotation list entries must be string scalars",
                    "non-string annotation item retained",
                );
            }
            let raw_list_item = entry.raw_list_item().map(|raw| ProjectValue {
                value: ProjectAnnotationScalar {
                    authored: raw.raw().to_owned(),
                    effective: compose_scalar_from_merged(raw),
                },
                provenance: entry.value().provenance().clone(),
                sensitive: raw.is_sensitive(),
            });
            let value = self.annotation_mapping_value(entry);
            let candidate = ProjectAnnotationEntry {
                name: ProjectKey::from_entry(entry),
                value,
                raw_list_item,
                syntax: entry.syntax(),
                contributors: vec![entry.value().provenance().clone()],
            };
            self.upsert_annotation(&mut entries, candidate, entry.value());
        }
        entries
    }

    fn annotation_mapping_value(&mut self, entry: &MergedEntry) -> Option<ProjectValue<ProjectAnnotationScalar>> {
        if entry.syntax() == EntrySyntax::ListKeyOnly {
            self.annotation_finding(
                ANNOTATIONS_KEY_ONLY,
                Severity::Warning,
                entry.value(),
                "key-only service annotation has no explicit value",
                "ambiguous key-only annotation",
            );
            return None;
        }
        self.annotation_scalar(entry.value()).map(|scalar| ProjectValue {
            value: scalar,
            provenance: entry.value().provenance().clone(),
            sensitive: entry.value().is_sensitive(),
        })
    }

    fn annotation_sequence(&mut self, values: &[MergedValue]) -> Vec<ProjectAnnotationEntry> {
        let mut entries = Vec::new();
        for item in values {
            let Some(candidate) = self.annotation_list_item(item) else {
                continue;
            };
            self.upsert_annotation(&mut entries, candidate, item);
        }
        entries
    }

    fn annotation_list_item(&mut self, item: &MergedValue) -> Option<ProjectAnnotationEntry> {
        let Some(scalar) = item.as_scalar() else {
            self.invalid_annotation_list_item(item);
            return None;
        };
        if scalar.kind() != MergedScalarKind::String {
            self.invalid_annotation_list_item(item);
            return None;
        }
        let raw_list_item = Some(ProjectValue {
            value: ProjectAnnotationScalar {
                authored: scalar.raw().to_owned(),
                effective: ComposeScalar::String(scalar.value().to_owned()),
            },
            provenance: item.provenance().clone(),
            sensitive: scalar.is_sensitive(),
        });
        let (name, value, syntax) = if let Some((name, value)) = scalar.value().split_once('=') {
            (
                name.to_owned(),
                Some(ProjectValue {
                    value: ProjectAnnotationScalar {
                        authored: scalar.raw().to_owned(),
                        effective: ComposeScalar::String(value.to_owned()),
                    },
                    provenance: item.provenance().clone(),
                    sensitive: scalar.is_sensitive(),
                }),
                EntrySyntax::ListKeyValue,
            )
        } else {
            self.annotation_finding(
                ANNOTATIONS_KEY_ONLY,
                Severity::Warning,
                item,
                "key-only service annotation has no explicit value",
                "ambiguous key-only annotation",
            );
            (scalar.value().to_owned(), None, EntrySyntax::ListKeyOnly)
        };
        if name.is_empty() {
            self.annotation_finding(
                ANNOTATIONS_EMPTY_NAME,
                Severity::Error,
                item,
                "service annotation name must not be empty",
                "empty annotation name",
            );
        }
        Some(ProjectAnnotationEntry {
            name: ProjectKey::from_value(name, item),
            value,
            raw_list_item,
            syntax,
            contributors: vec![item.provenance().clone()],
        })
    }

    fn invalid_annotation_list_item(&mut self, item: &MergedValue) {
        self.annotation_finding(
            ANNOTATIONS_EXPECTED_STRING,
            Severity::Error,
            item,
            "annotation list entries must be string scalars",
            "non-string annotation item retained in merged source",
        );
    }

    fn annotation_scalar(&mut self, value: &MergedValue) -> Option<ProjectAnnotationScalar> {
        let effective = self.compose_scalar(
            value,
            "annotation mapping values must be scalar strings, numbers, booleans, or null",
        )?;
        let authored = match value.kind() {
            MergedValueKind::Null(crate::merge::NullStyle::Empty) => String::new(),
            MergedValueKind::Null(crate::merge::NullStyle::Explicit) => "null".to_owned(),
            MergedValueKind::Scalar(scalar) => scalar.raw().to_owned(),
            _ => unreachable!("compose_scalar accepted only scalar or null"),
        };
        Some(ProjectAnnotationScalar { authored, effective })
    }

    fn upsert_annotation(
        &mut self,
        entries: &mut Vec<ProjectAnnotationEntry>,
        mut candidate: ProjectAnnotationEntry,
        source: &MergedValue,
    ) {
        if let Some(existing) = entries
            .iter_mut()
            .find(|entry| entry.name.value == candidate.name.value)
        {
            self.annotation_finding(
                ANNOTATIONS_DUPLICATE_NAME,
                Severity::Error,
                source,
                "service annotation names must be unique",
                "later annotation value replaces earlier effective value",
            );
            for span in candidate.name.sources.drain(..) {
                if !existing.name.sources.contains(&span) {
                    existing.name.sources.push(span);
                }
            }
            existing.name.sensitive |= candidate.name.sensitive;
            existing.contributors.append(&mut candidate.contributors);
            existing.value = candidate.value;
            existing.raw_list_item = candidate.raw_list_item;
            existing.syntax = candidate.syntax;
        } else {
            entries.push(candidate);
        }
    }

    fn annotation_finding(
        &mut self,
        code: DiagnosticCode,
        severity: Severity,
        value: &MergedValue,
        message: &'static str,
        label: &'static str,
    ) {
        self.diagnostics.push(
            Diagnostic::new(code, severity, message).with_label(DiagnosticLabel::primary(effective_span(value), label)),
        );
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

    fn non_empty_project_string(&mut self, value: &MergedValue, description: &str) -> Option<ProjectValue<String>> {
        let string = self.project_string(value, description)?;
        if string.value().is_empty() {
            self.invalid(effective_span(value), "build dockerfile must be a non-empty scalar");
            return None;
        }
        Some(string)
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

    fn capability_drop(&mut self, value: &MergedValue) -> Option<ProjectValue<Vec<ProjectValue<CapabilityDropItem>>>> {
        let Some(values) = value.as_sequence() else {
            self.expected(value, "cap_drop must be a sequence of string scalars");
            return None;
        };
        let mut items = Vec::new();
        let mut seen = BTreeMap::new();
        for item in values {
            let scalar = match item.kind() {
                MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => scalar,
                _ => {
                    self.expected(item, "cap_drop entries must be string scalars");
                    continue;
                }
            };
            let span = effective_span(item);
            if let Some(first) = seen.get(scalar.value()) {
                self.diagnostics.push(
                    Diagnostic::new(
                        CAP_DROP_DUPLICATE_ITEM,
                        Severity::Error,
                        "cap_drop entries must be unique exact strings",
                    )
                    .with_label(DiagnosticLabel::primary(span, "duplicate capability string"))
                    .with_label(DiagnosticLabel::secondary(*first, "first identical string")),
                );
            } else {
                seen.insert(scalar.value().to_owned(), span);
            }
            let typed = CapabilityDropItem::new(Located::new(scalar.value().to_owned(), span));
            items.push(ProjectValue::new(typed, item));
        }
        Some(ProjectValue::new(items, value))
    }

    fn capability_add(&mut self, value: &MergedValue) -> Option<ProjectValue<Vec<ProjectValue<CapabilityAddItem>>>> {
        let Some(values) = value.as_sequence() else {
            self.expected(value, "cap_add must be a sequence of string scalars");
            return None;
        };
        let mut items = Vec::new();
        let mut seen = BTreeMap::new();
        for item in values {
            let scalar = match item.kind() {
                MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => scalar,
                _ => {
                    self.expected(item, "cap_add entries must be string scalars");
                    continue;
                }
            };
            let span = effective_span(item);
            if let Some(first) = seen.get(scalar.value()) {
                self.diagnostics.push(
                    Diagnostic::new(
                        CAP_ADD_DUPLICATE_ITEM,
                        Severity::Error,
                        "cap_add entries must be unique exact strings",
                    )
                    .with_label(DiagnosticLabel::primary(span, "duplicate capability string"))
                    .with_label(DiagnosticLabel::secondary(*first, "first identical string")),
                );
            } else {
                seen.insert(scalar.value().to_owned(), span);
            }
            let typed = CapabilityAddItem::new(Located::new(scalar.value().to_owned(), span));
            items.push(ProjectValue::new(typed, item));
        }
        Some(ProjectValue::new(items, value))
    }

    fn devices(
        &mut self,
        value: &MergedValue,
        service_path: &[String],
    ) -> Option<ProjectValue<Vec<ProjectValue<ProjectDevice>>>> {
        let Some(values) = value.as_sequence() else {
            self.expected(value, "service devices must be a sequence");
            return None;
        };
        let mut devices = Vec::new();
        for (index, item) in values.iter().enumerate() {
            let mut path = service_path.to_vec();
            path.push("devices".to_owned());
            path.push(index.to_string());
            let device = match item.kind() {
                MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => ProjectDevice::Short(
                    ShortDevice::new(Located::new(scalar.value().to_owned(), effective_span(item))),
                ),
                MergedValueKind::Mapping(fields) => ProjectDevice::Long(self.long_device(item, fields, &path)),
                _ => {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DEVICE_EXPECTED_FORM,
                            Severity::Error,
                            "service device must use string short syntax or mapping long syntax",
                        )
                        .with_label(DiagnosticLabel::primary(
                            effective_span(item),
                            "unsupported device form",
                        )),
                    );
                    continue;
                }
            };
            devices.push(ProjectValue::new(device, item));
        }
        Some(ProjectValue::new(devices, value))
    }

    fn long_device(&mut self, value: &MergedValue, fields: &[MergedEntry], path: &[String]) -> ProjectLongDevice {
        let mut device = ProjectLongDevice {
            source: None,
            target: None,
            permissions: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        };
        for field in fields {
            let parsed = match field.key() {
                "source" | "target" | "permissions" => self.device_string(field.value(), field.key()),
                name if name.starts_with("x-") => {
                    device.extension_fields.push(field_reference(path, field));
                    continue;
                }
                _ => {
                    device.unknown_fields.push(field_reference(path, field));
                    continue;
                }
            };
            match field.key() {
                "source" => device.source = parsed,
                "target" => device.target = parsed,
                "permissions" => device.permissions = parsed,
                _ => unreachable!("unrecognized device fields continue before assignment"),
            }
        }
        if device.source.is_none() {
            self.missing(value, "long-syntax device is missing required string `source`");
        }
        device
    }

    fn device_string(&mut self, value: &MergedValue, member: &str) -> Option<ProjectValue<String>> {
        let scalar = match value.kind() {
            MergedValueKind::Scalar(scalar) if scalar.kind() == MergedScalarKind::String => scalar,
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        DEVICE_EXPECTED_STRING,
                        Severity::Error,
                        format!("device {member} must be a string scalar"),
                    )
                    .with_label(DiagnosticLabel::primary(
                        effective_span(value),
                        "unexpected long-device member form",
                    )),
                );
                return None;
            }
        };
        Some(ProjectValue::new(scalar.value().to_owned(), value))
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

fn effective_security_label_diagnostic(
    kind: &SecurityOptionKind,
    span: SourceSpan,
    candidates: &mut SecurityOptionCandidateCounts,
) -> Option<Diagnostic> {
    match kind {
        SecurityOptionKind::SecurityLabelDisable { .. } | SecurityOptionKind::SecurityLabelDisableNearMiss => {
            security_label_disable_diagnostic(kind, span, &mut candidates.security_label_disable)
        }
        SecurityOptionKind::SecurityLabelFileType { .. } | SecurityOptionKind::SecurityLabelFileTypeNearMiss => {
            security_label_filetype_diagnostic(kind, span, &mut candidates.security_label_filetype)
        }
        SecurityOptionKind::SecurityLabelLevel { .. } | SecurityOptionKind::SecurityLabelLevelNearMiss => {
            security_label_level_diagnostic(kind, span, &mut candidates.security_label_level)
        }
        SecurityOptionKind::SecurityLabelNested { .. } | SecurityOptionKind::SecurityLabelNestedNearMiss => {
            security_label_nested_diagnostic(kind, span, &mut candidates.security_label_nested)
        }
        SecurityOptionKind::SecurityLabelType { .. } | SecurityOptionKind::SecurityLabelTypeNearMiss => {
            security_label_type_diagnostic(kind, span, &mut candidates.security_label_type)
        }
        _ => None,
    }
}

fn security_label_disable_diagnostic(
    kind: &SecurityOptionKind,
    span: SourceSpan,
    candidates: &mut usize,
) -> Option<Diagnostic> {
    match kind {
        SecurityOptionKind::SecurityLabelDisable { .. } => {
            *candidates += 1;
            (*candidates > 1).then(|| {
                Diagnostic::new(
                    SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT,
                    Severity::Warning,
                    "multiple SELinux label-disable candidates are retained; a consumer must resolve the conflict explicitly",
                )
                .with_label(DiagnosticLabel::primary(
                    span,
                    "additional effective SELinux label-disable candidate retained",
                ))
            })
        }
        SecurityOptionKind::SecurityLabelDisableNearMiss => Some(
            Diagnostic::new(
                SECURITY_OPT_SECURITY_LABEL_DISABLE_NEAR_MISS,
                Severity::Warning,
                "SELinux label-disable candidates require exact lowercase `label:disable` spelling without whitespace",
            )
            .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
        ),
        _ => None,
    }
}

fn security_label_filetype_diagnostic(
    kind: &SecurityOptionKind,
    span: SourceSpan,
    candidates: &mut usize,
) -> Option<Diagnostic> {
    match kind {
        SecurityOptionKind::SecurityLabelFileType { .. } => {
            *candidates += 1;
            (*candidates > 1).then(|| {
                Diagnostic::new(
                    SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT,
                    Severity::Warning,
                    "multiple SELinux label-filetype candidates are retained; a consumer must resolve the conflict explicitly",
                )
                .with_label(DiagnosticLabel::primary(
                    span,
                    "additional effective SELinux label-filetype candidate retained",
                ))
            })
        }
        SecurityOptionKind::SecurityLabelFileTypeNearMiss => Some(
            Diagnostic::new(
                SECURITY_OPT_SECURITY_LABEL_FILETYPE_NEAR_MISS,
                Severity::Warning,
                "SELinux label-filetype candidates require exact lowercase `label:filetype:<type>` spelling without whitespace",
            )
            .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
        ),
        _ => None,
    }
}

fn security_label_level_diagnostic(
    kind: &SecurityOptionKind,
    span: SourceSpan,
    candidates: &mut usize,
) -> Option<Diagnostic> {
    match kind {
        SecurityOptionKind::SecurityLabelLevel { .. } => {
            *candidates += 1;
            (*candidates > 1).then(|| {
                Diagnostic::new(
                    SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT,
                    Severity::Warning,
                    "multiple SELinux label-level candidates are retained; a consumer must resolve the conflict explicitly",
                )
                .with_label(DiagnosticLabel::primary(
                    span,
                    "additional effective SELinux label-level candidate retained",
                ))
            })
        }
        SecurityOptionKind::SecurityLabelLevelNearMiss => Some(
            Diagnostic::new(
                SECURITY_OPT_SECURITY_LABEL_LEVEL_NEAR_MISS,
                Severity::Warning,
                "SELinux label-level candidates require exact lowercase `label:level:<level>` spelling without whitespace",
            )
            .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
        ),
        _ => None,
    }
}

fn security_label_nested_diagnostic(
    kind: &SecurityOptionKind,
    span: SourceSpan,
    candidates: &mut usize,
) -> Option<Diagnostic> {
    match kind {
        SecurityOptionKind::SecurityLabelNested { .. } => {
            *candidates += 1;
            (*candidates > 1).then(|| {
                Diagnostic::new(
                    SECURITY_OPT_SECURITY_LABEL_NESTED_CONFLICT,
                    Severity::Warning,
                    "multiple SELinux label-nested candidates are retained; a consumer must resolve the conflict explicitly",
                )
                .with_label(DiagnosticLabel::primary(
                    span,
                    "additional effective SELinux label-nested candidate retained",
                ))
            })
        }
        SecurityOptionKind::SecurityLabelNestedNearMiss => Some(
            Diagnostic::new(
                SECURITY_OPT_SECURITY_LABEL_NESTED_NEAR_MISS,
                Severity::Warning,
                "SELinux label-nested candidates require exact lowercase `label:nested` spelling without whitespace",
            )
            .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
        ),
        _ => None,
    }
}

fn security_label_type_diagnostic(
    kind: &SecurityOptionKind,
    span: SourceSpan,
    candidates: &mut usize,
) -> Option<Diagnostic> {
    match kind {
        SecurityOptionKind::SecurityLabelType { .. } => {
            *candidates += 1;
            (*candidates > 1).then(|| {
                Diagnostic::new(
                    SECURITY_OPT_SECURITY_LABEL_TYPE_CONFLICT,
                    Severity::Warning,
                    "multiple SELinux label-type candidates are retained; a consumer must resolve the conflict explicitly",
                )
                .with_label(DiagnosticLabel::primary(
                    span,
                    "additional effective SELinux label-type candidate retained",
                ))
            })
        }
        SecurityOptionKind::SecurityLabelTypeNearMiss => Some(
            Diagnostic::new(
                SECURITY_OPT_SECURITY_LABEL_TYPE_NEAR_MISS,
                Severity::Warning,
                "SELinux label-type candidates require exact lowercase `label:type:<type>` spelling with one non-empty whitespace-free type",
            )
            .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
        ),
        _ => None,
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

    fn grants(
        &mut self,
        value: &MergedValue,
        service_path: &[String],
        kind: &str,
    ) -> Option<ProjectValue<Vec<ProjectValue<ProjectGrant>>>> {
        self.grants_at(value, service_path, &format!("{kind}s"), kind).0
    }

    fn build_secret_grants(
        &mut self,
        value: &MergedValue,
        build_path: &[String],
    ) -> (Option<ProjectValue<Vec<ProjectValue<ProjectGrant>>>>, bool) {
        self.grants_at(value, build_path, "secrets", "build secret")
    }

    fn grants_at(
        &mut self,
        value: &MergedValue,
        parent_path: &[String],
        field_name: &str,
        kind: &str,
    ) -> (Option<ProjectValue<Vec<ProjectValue<ProjectGrant>>>>, bool) {
        let Some(values) = value.as_sequence() else {
            self.expected(value, &format!("{kind}s must be a sequence"));
            return (None, true);
        };
        let mut grants = Vec::new();
        let mut malformed = false;
        for (index, item) in values.iter().enumerate() {
            let mut path = parent_path.to_vec();
            path.push(field_name.to_owned());
            path.push(index.to_string());
            let grant = match item.kind() {
                MergedValueKind::Scalar(scalar) => ProjectGrant::Short(scalar.value().to_owned()),
                MergedValueKind::Mapping(fields) => {
                    let grant = self.long_grant(item, fields, &path, kind);
                    malformed |= grant.source().is_none();
                    ProjectGrant::Long(Box::new(grant))
                }
                _ => {
                    self.expected(
                        item,
                        &format!("{kind} must use scalar short syntax or mapping long syntax"),
                    );
                    malformed = true;
                    continue;
                }
            };
            grants.push(ProjectValue::new(grant, item));
        }
        (Some(ProjectValue::new(grants, value)), malformed)
    }

    fn long_grant(
        &mut self,
        value: &MergedValue,
        fields: &[MergedEntry],
        path: &[String],
        kind: &str,
    ) -> ProjectLongGrant {
        let mut grant = ProjectLongGrant {
            source: None,
            target: None,
            uid: None,
            gid: None,
            mode: None,
            unmodeled_fields: Vec::new(),
        };
        for field in fields {
            let parsed = match field.key() {
                "source" => self.project_string(field.value(), &format!("{kind} source")),
                "target" => self.project_string(field.value(), &format!("{kind} target")),
                "uid" => self.project_string(field.value(), &format!("{kind} uid")),
                "gid" => self.project_string(field.value(), &format!("{kind} gid")),
                "mode" => self.project_string(field.value(), &format!("{kind} mode")),
                _ => {
                    grant.unmodeled_fields.push(field_reference(path, field));
                    continue;
                }
            };
            if parsed.is_none() {
                grant.unmodeled_fields.push(field_reference(path, field));
            }
            match field.key() {
                "source" => grant.source = parsed,
                "target" => grant.target = parsed,
                "uid" => grant.uid = parsed,
                "gid" => grant.gid = parsed,
                "mode" => grant.mode = parsed,
                _ => unreachable!("unrecognized grant fields continue before assignment"),
            }
        }
        if grant.source.is_none() {
            self.missing(value, &format!("long-syntax {kind} is missing `source`"));
        }
        grant
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
        } else if value.is_sensitive() && matches!(scalar.value(), "true" | "false") {
            BooleanValue::Literal(scalar.value() == "true")
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
        self.validate_external_volume_driver_configuration(&volume);
        self.validate_external_volume_labels_configuration(&volume);
        Some(volume)
    }

    fn validate_external_volume_driver_configuration(&mut self, volume: &VolumeDefinition) {
        if !matches!(volume.external().map(Located::value), Some(BooleanValue::Literal(true)))
            || (volume.driver().is_none() && volume.driver_opts().is_empty())
        {
            return;
        }
        let span = volume
            .driver()
            .map(Located::span)
            .or_else(|| volume.driver_opts().first().map(KeyValueEntry::span))
            .unwrap_or_else(|| volume.span());
        self.diagnostics.push(
            Diagnostic::new(
                VOLUME_EXTERNAL_DRIVER_CONFIGURATION,
                Severity::Error,
                "external volume cannot also configure `driver` or `driver_opts`",
            )
            .with_label(DiagnosticLabel::primary(
                span,
                "driver configuration remains retained for review",
            )),
        );
    }

    fn validate_external_volume_labels_configuration(&mut self, volume: &VolumeDefinition) {
        if !matches!(volume.external().map(Located::value), Some(BooleanValue::Literal(true)))
            || volume.labels().is_none()
        {
            return;
        }
        let span = volume.labels().map_or_else(|| volume.span(), Labels::span);
        self.diagnostics.push(
            Diagnostic::new(
                VOLUME_EXTERNAL_LABELS_CONFIGURATION,
                Severity::Error,
                "external volume cannot also configure `labels`",
            )
            .with_label(DiagnosticLabel::primary(span, "labels remain retained for review")),
        );
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

fn compose_scalar_from_merged(scalar: &crate::merge::MergedScalar) -> ComposeScalar {
    match scalar.kind() {
        MergedScalarKind::String => ComposeScalar::String(scalar.value().to_owned()),
        MergedScalarKind::Boolean => ComposeScalar::Boolean(scalar.value().eq_ignore_ascii_case("true")),
        MergedScalarKind::Number => ComposeScalar::Number(scalar.value().to_owned()),
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

fn value_reference(path: &[String], value: &MergedValue) -> ProjectFieldReference {
    let key = path.last().cloned().unwrap_or_default();
    ProjectFieldReference {
        path: path.to_vec(),
        key: ProjectKey::from_value(key, value),
        provenance: value.provenance().clone(),
        extension: false,
        sensitive: value.is_sensitive(),
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
