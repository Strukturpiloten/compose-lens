//! Field-level build and deploy section models.

use super::{
    BooleanValue, BuildExtraHosts, BuildNoCache, BuildProvenance, BuildSbom, FieldReference, KeyValueEntry, Labels,
    Located, SecretGrant, ShmSize, Ulimits,
};
use crate::source::SourceSpan;
use std::fmt;
use std::sync::Arc;

/// A Compose build declaration with short and long forms retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Build {
    /// A scalar build context.
    Context(Located<String>),
    /// A mapping of independently classified build fields.
    Definition(BuildDefinition),
}

/// A long-syntax build definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildDefinition {
    span: SourceSpan,
    values: Box<BuildValues>,
    fields: Vec<BuildField>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Arc<Vec<FieldReference>>,
}

/// Heap-stored optional build values keep the public `Build` enum compact without changing its
/// short/long syntax variants.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BuildValues {
    additional_contexts: Option<BuildAdditionalContexts>,
    entitlements: Option<Arc<Vec<Located<String>>>>,
    extra_hosts: Option<BuildExtraHosts>,
    context: Option<Located<String>>,
    args: Option<BuildArgs>,
    cache_from: Option<Arc<Vec<Located<String>>>>,
    cache_to: Option<Arc<Vec<Located<String>>>>,
    dockerfile: Option<Located<String>>,
    dockerfile_inline: Option<Located<String>>,
    target: Option<Located<String>>,
    network: Option<Box<Located<String>>>,
    isolation: Option<Box<Located<String>>>,
    platforms: Option<Arc<Vec<Located<String>>>>,
    no_cache: Option<Box<Located<BuildNoCache>>>,
    no_cache_filter: Option<BuildNoCacheFilter>,
    privileged: Option<Box<Located<BooleanValue>>>,
    sbom: Option<Box<Located<BuildSbom>>>,
    provenance: Option<Box<Located<BuildProvenance>>>,
    pull: Option<Box<Located<BooleanValue>>>,
    shm_size: Option<Box<ShmSize>>,
    tags: Option<Arc<Vec<Located<String>>>>,
    labels: Option<Box<Labels>>,
    secrets: Option<Arc<Vec<SecretGrant>>>,
    ssh: Option<BuildSsh>,
    ulimits: Option<Box<Ulimits>>,
}

impl BuildDefinition {
    pub(super) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            values: Box::new(BuildValues {
                additional_contexts: None,
                entitlements: None,
                extra_hosts: None,
                context: None,
                args: None,
                cache_from: None,
                cache_to: None,
                dockerfile: None,
                dockerfile_inline: None,
                target: None,
                network: None,
                isolation: None,
                platforms: None,
                no_cache: None,
                no_cache_filter: None,
                privileged: None,
                sbom: None,
                provenance: None,
                pull: None,
                shm_size: None,
                tags: None,
                labels: None,
                secrets: None,
                ssh: None,
                ulimits: None,
            }),
            fields: Vec::new(),
            extension_fields: Vec::new(),
            unknown_fields: Arc::new(Vec::new()),
        }
    }

    pub(super) fn push_field(&mut self, field: BuildField) {
        self.fields.push(field);
    }

    pub(super) fn set_context(&mut self, context: Located<String>) {
        self.values.context = Some(context);
    }

    pub(super) fn set_additional_contexts(&mut self, additional_contexts: Option<BuildAdditionalContexts>) {
        self.values.additional_contexts = additional_contexts;
    }

    pub(super) fn set_entitlements(&mut self, entitlements: Vec<Located<String>>) {
        self.values.entitlements = Some(Arc::new(entitlements));
    }

    pub(super) fn set_extra_hosts(&mut self, extra_hosts: BuildExtraHosts) {
        self.values.extra_hosts = Some(extra_hosts);
    }

    pub(super) fn set_args(&mut self, args: BuildArgs) {
        self.values.args = Some(args);
    }

    pub(super) fn set_cache_from(&mut self, cache_from: Vec<Located<String>>) {
        self.values.cache_from = Some(Arc::new(cache_from));
    }

    pub(super) fn set_cache_to(&mut self, cache_to: Vec<Located<String>>) {
        self.values.cache_to = Some(Arc::new(cache_to));
    }

    pub(super) fn set_dockerfile(&mut self, dockerfile: Located<String>) {
        self.values.dockerfile = Some(dockerfile);
    }

    pub(super) fn set_dockerfile_inline(&mut self, dockerfile_inline: Located<String>) {
        self.values.dockerfile_inline = Some(dockerfile_inline);
    }

    pub(super) fn set_target(&mut self, target: Located<String>) {
        self.values.target = Some(target);
    }

    pub(super) fn set_network(&mut self, network: Located<String>) {
        self.values.network = Some(Box::new(network));
    }

    pub(super) fn set_isolation(&mut self, isolation: Located<String>) {
        self.values.isolation = Some(Box::new(isolation));
    }

    pub(super) fn set_platforms(&mut self, platforms: Vec<Located<String>>) {
        self.values.platforms = Some(Arc::new(platforms));
    }

    pub(super) fn set_no_cache(&mut self, no_cache: Located<BuildNoCache>) {
        self.values.no_cache = Some(Box::new(no_cache));
    }
    pub(super) fn set_no_cache_filter(&mut self, value: BuildNoCacheFilter) {
        self.values.no_cache_filter = Some(value);
    }
    pub(super) fn set_privileged(&mut self, value: Located<BooleanValue>) {
        self.values.privileged = Some(Box::new(value));
    }

    pub(super) fn set_sbom(&mut self, sbom: Located<BuildSbom>) {
        self.values.sbom = Some(Box::new(sbom));
    }
    pub(super) fn set_provenance(&mut self, value: Located<BuildProvenance>) {
        self.values.provenance = Some(Box::new(value));
    }

    pub(super) fn set_pull(&mut self, pull: Located<BooleanValue>) {
        self.values.pull = Some(Box::new(pull));
    }

    pub(super) fn set_shm_size(&mut self, shm_size: ShmSize) {
        self.values.shm_size = Some(Box::new(shm_size));
    }

    pub(super) fn set_tags(&mut self, tags: Vec<Located<String>>) {
        self.values.tags = Some(Arc::new(tags));
    }

    pub(super) fn set_labels(&mut self, labels: Labels) {
        self.values.labels = Some(Box::new(labels));
    }

    pub(super) fn set_secrets(&mut self, secrets: Vec<SecretGrant>) {
        self.values.secrets = Some(Arc::new(secrets));
    }

    pub(super) fn set_ssh(&mut self, ssh: BuildSsh) {
        self.values.ssh = Some(ssh);
    }

    pub(super) fn set_ulimits(&mut self, ulimits: Ulimits) {
        self.values.ulimits = Some(Box::new(ulimits));
    }

    pub(super) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(super) fn push_unknown(&mut self, field: FieldReference) {
        Arc::make_mut(&mut self.unknown_fields).push(field);
    }

    /// Returns the complete mapping span.
    /// Returns the complete update-config mapping span.
    /// Returns the complete update-config mapping span.
    /// Returns the complete update-config mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the explicitly authored long-syntax build context when it is a string scalar.
    ///
    /// Other build subfields remain source-addressable references and are not semantically
    /// interpreted by this model.
    /// Returns the raw update parallelism scalar.
    #[must_use]
    pub const fn context(&self) -> Option<&Located<String>> {
        self.values.context.as_ref()
    }

    /// Returns authored additional build contexts without normalizing mapping and list syntax.
    ///
    /// List entries remain raw ordered strings, including duplicates and `NAME=VALUE` spelling.
    /// Mapping entries retain scalar kinds and authored order. This model does not interpret
    /// names, paths, URLs, images, service schemes, or builder behavior.
    /// Returns the raw update delay string.
    #[must_use]
    pub const fn additional_contexts(&self) -> Option<&BuildAdditionalContexts> {
        self.values.additional_contexts.as_ref()
    }

    /// Returns authored build entitlements in order as opaque raw strings.
    ///
    /// Explicit emptiness remains distinct from omission. This model retains duplicates and does
    /// not infer entitlement allowlists, privilege state, `BuildKit` or platform support, build
    /// execution, or runtime effect.
    /// Returns the raw update monitor string.
    #[must_use]
    pub fn entitlements(&self) -> Option<&[Located<String>]> {
        self.values.entitlements.as_deref().map(Vec::as_slice)
    }

    /// Returns authored build-time host mappings without using service `extra_hosts` semantics.
    ///
    /// List entries retain raw `=`/`:` spelling, IPv4/IPv6 brackets, `host-gateway`, and unknown
    /// values. Mapping values retain either a scalar string or ordered string list; no address
    /// normalization, validation, DNS lookup, host inspection, or build behavior is performed.
    /// Returns the raw update failure-action string.
    #[must_use]
    pub const fn extra_hosts(&self) -> Option<&BuildExtraHosts> {
        self.values.extra_hosts.as_ref()
    }

    /// Returns explicitly authored build arguments without normalizing mapping and list syntax.
    ///
    /// Mapping entries retain their string, number, boolean, or null scalar kinds. List entries
    /// remain raw ordered strings, including duplicates and bare argument names.
    /// Returns the raw update maximum-failure-ratio scalar.
    #[must_use]
    pub const fn args(&self) -> Option<&BuildArgs> {
        self.values.args.as_ref()
    }

    /// Returns authored external build-cache sources in order.
    ///
    /// An explicit empty sequence remains distinct from omission. Entries are raw string scalars;
    /// this model preserves duplicates and does not parse cache type, reference, source, path,
    /// image, credentials, or builder behavior.
    /// Returns the update order.
    #[must_use]
    pub fn cache_from(&self) -> Option<&[Located<String>]> {
        self.values.cache_from.as_deref().map(Vec::as_slice)
    }

    /// Returns authored external build-cache destinations in order.
    ///
    /// An explicit empty sequence remains distinct from omission. Entries are raw string scalars;
    /// this model preserves duplicates and does not parse cache type, reference, destination,
    /// path, image, credentials, or builder behavior.
    /// Returns retained update-config extensions.
    #[must_use]
    pub fn cache_to(&self) -> Option<&[Located<String>]> {
        self.values.cache_to.as_deref().map(Vec::as_slice)
    }

    /// Returns the explicitly authored long-syntax Dockerfile when it is a non-empty scalar.
    ///
    /// Other build subfields remain source-addressable references and are not semantically
    /// interpreted by this model.
    /// Returns retained update-config malformed or unknown fields.
    #[must_use]
    pub const fn dockerfile(&self) -> Option<&Located<String>> {
        self.values.dockerfile.as_ref()
    }

    /// Returns the authored inline Dockerfile as an exact string scalar.
    ///
    /// Empty and multiline content remains distinct from omission. `ComposeLens` does not parse
    /// Containerfile syntax, resolve paths or contexts, scan content for secrets, build images,
    /// or infer Docker, `BuildKit`, or runtime behavior.
    #[must_use]
    pub const fn dockerfile_inline(&self) -> Option<&Located<String>> {
        self.values.dockerfile_inline.as_ref()
    }

    /// Returns the explicitly authored long-syntax build target as an opaque scalar.
    ///
    /// An empty scalar remains an authored target; this model does not infer stage-name grammar.
    #[must_use]
    pub const fn target(&self) -> Option<&Located<String>> {
        self.values.target.as_ref()
    }

    /// Returns the explicitly authored long-syntax build network as an opaque scalar.
    ///
    /// An empty scalar remains authored; this model does not infer network names, defaults, or
    /// runtime behavior.
    #[must_use]
    pub fn network(&self) -> Option<&Located<String>> {
        self.values.network.as_deref()
    }

    /// Returns the explicitly authored long-syntax build isolation as an opaque YAML string.
    ///
    /// This model preserves only YAML string scalars and does not validate isolation modes,
    /// platforms, defaults, privileges, or `BUILDAH_ISOLATION` behavior. It is unrelated to the
    /// service-level `isolation` field.
    #[must_use]
    pub fn isolation(&self) -> Option<&Located<String>> {
        self.values.isolation.as_deref()
    }

    /// Returns explicitly authored build platforms in order.
    ///
    /// An explicit empty sequence remains distinct from omission. Platforms are raw scalar
    /// values; this model does not parse OCI platform grammar or validate availability.
    #[must_use]
    pub fn platforms(&self) -> Option<&[Located<String>]> {
        self.values.platforms.as_deref().map(Vec::as_slice)
    }

    /// Returns the explicitly authored build cache-disable choice with YAML scalar type retained.
    ///
    /// Omission does not imply a default. String values, including empty and interpolation-shaped
    /// strings, are not coerced or resolved as booleans; this model does not infer builder or
    /// cache behavior.
    #[must_use]
    pub fn no_cache(&self) -> Option<&Located<BuildNoCache>> {
        self.values.no_cache.as_deref()
    }
    /// Returns raw no-cache filter scalar or list syntax.
    #[must_use]
    pub const fn no_cache_filter(&self) -> Option<&BuildNoCacheFilter> {
        self.values.no_cache_filter.as_ref()
    }
    /// Returns the explicit build privileged boolean or deferred expression.
    #[must_use]
    pub fn privileged(&self) -> Option<&Located<BooleanValue>> {
        self.values.privileged.as_deref()
    }

    /// Returns the explicitly authored build SBOM choice with YAML scalar type retained.
    ///
    /// Omission does not imply a default. String values, including empty, generator-shaped, and
    /// interpolation-shaped strings, are not coerced or interpreted; this model does not generate
    /// an SBOM or infer builder behavior.
    #[must_use]
    pub fn sbom(&self) -> Option<&Located<BuildSbom>> {
        self.values.sbom.as_deref()
    }
    /// Returns authored Build provenance as a boolean or opaque string scalar.
    #[must_use]
    pub fn provenance(&self) -> Option<&Located<BuildProvenance>> {
        self.values.provenance.as_deref()
    }

    /// Returns whether this build should pull referenced images before building.
    ///
    /// A literal boolean and a deferred interpolation expression remain distinct. Omission is not
    /// treated as an implicit default, and this model does not resolve expressions or infer build
    /// execution behavior.
    #[must_use]
    pub fn pull(&self) -> Option<&Located<BooleanValue>> {
        self.values.pull.as_deref()
    }

    /// Returns the explicitly authored build-container shared-memory size.
    ///
    /// This retains the same YAML number/string spelling, documented lowercase-unit
    /// classification, lexical-zero, deferred-expression, and provider-dependent states as
    /// service `shm_size`. Omission does not infer a builder default, allocation, host setting,
    /// or runtime behavior.
    #[must_use]
    pub fn shm_size(&self) -> Option<&ShmSize> {
        self.values.shm_size.as_deref()
    }

    /// Returns explicitly authored additional build tags in order.
    ///
    /// An explicit empty sequence remains distinct from omission. Tags are opaque scalar values;
    /// this model does not apply image-reference grammar or duplicate handling.
    #[must_use]
    pub fn tags(&self) -> Option<&[Located<String>]> {
        self.values.tags.as_deref().map(Vec::as_slice)
    }

    /// Returns explicitly authored build labels without normalizing mapping and list syntax.
    ///
    /// List entries remain ordered raw strings, including duplicates and bare labels. Mapping
    /// entries retain their scalar kinds and authored order.
    #[must_use]
    pub fn labels(&self) -> Option<&Labels> {
        self.values.labels.as_deref()
    }

    /// Returns explicitly authored build secret grants in order.
    ///
    /// Short resource-name and long mapping syntax remain distinct. An explicit empty sequence,
    /// duplicate entries, raw scalar spellings, and unknown long-form fields are retained; this
    /// model neither resolves top-level secrets nor materializes secret contents.
    #[must_use]
    pub fn secrets(&self) -> Option<&[SecretGrant]> {
        self.values.secrets.as_deref().map(Vec::as_slice)
    }

    /// Returns authored `BuildKit` SSH grants without normalizing mapping and list syntax.
    ///
    /// SSH identifiers, paths, agent sockets, and material are opaque sensitive data. This
    /// model neither parses them nor accesses the host, an agent, a socket, or a build runtime.
    #[must_use]
    pub const fn ssh(&self) -> Option<&BuildSsh> {
        self.values.ssh.as_ref()
    }

    /// Returns explicitly authored build-container resource limits.
    ///
    /// The same single and soft/hard range forms as service `ulimits` are retained. This model
    /// does not inject defaults, normalize unlimited values, validate host limits, or infer
    /// builder or runtime behavior.
    #[must_use]
    pub fn ulimits(&self) -> Option<&Ulimits> {
        self.values.ulimits.as_deref()
    }

    /// Returns recognized build fields in authored order.
    #[must_use]
    pub fn fields(&self) -> &[BuildField] {
        &self.fields
    }

    /// Finds the first recognized field of the requested kind.
    #[must_use]
    pub fn field(&self, kind: BuildFieldKind) -> Option<&BuildField> {
        self.fields.iter().find(|field| field.kind == kind)
    }

    /// Returns retained `x-` fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns fields not recognized by this release.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        self.unknown_fields.as_slice()
    }
}

/// Authored `build.no_cache_filter` form retaining exact string values.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildNoCacheFilter {
    /// One exact scalar stage name.
    Scalar(Located<String>),
    /// Ordered exact stage names.
    List(Vec<Located<String>>),
}

/// Additional build contexts with mapping and list syntax retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildAdditionalContexts {
    /// List syntax such as `name=path`, retained as raw strings.
    List {
        /// The complete sequence span.
        span: SourceSpan,
        /// Context entries in authored order.
        values: Vec<Located<String>>,
    },
    /// Mapping syntax with scalar context values.
    Map {
        /// The complete mapping span.
        span: SourceSpan,
        /// Context entries in authored order.
        entries: Vec<KeyValueEntry>,
    },
}

impl BuildAdditionalContexts {
    /// Returns the authored collection span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::List { span, .. } | Self::Map { span, .. } => *span,
        }
    }
}

/// Compose build arguments with mapping and list syntax retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildArgs {
    /// List syntax such as `HTTP_PROXY=http://proxy` or a bare `HTTP_PROXY`.
    List {
        /// The complete sequence span.
        span: SourceSpan,
        /// Raw argument strings in authored order.
        values: Vec<Located<String>>,
    },
    /// Mapping syntax with scalar argument values.
    Map {
        /// The complete mapping span.
        span: SourceSpan,
        /// Argument entries in authored order.
        entries: Vec<KeyValueEntry>,
    },
}

impl BuildArgs {
    /// Returns the authored collection span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::List { span, .. } | Self::Map { span, .. } => *span,
        }
    }
}

/// Sensitive `BuildKit` SSH grants with mapping and list syntax retained.
///
/// List entries remain raw ordered strings, including duplicates. Mapping entries retain scalar
/// string, number, boolean, or null values in authored order. Neither form parses identifiers,
/// paths, PEM material, sockets, `SSH_AUTH_SOCK`, Containerfile mounts, or agent behavior.
///
/// Sensitive storage cannot be destructured by downstream callers; use [`Self::as_list`] or
/// [`Self::as_map`] explicitly when raw inspection is required.
///
/// ```compile_fail
/// use compose_lens::model::BuildSsh;
///
/// fn forbidden(ssh: BuildSsh) {
///     let BuildSsh { storage } = ssh;
/// }
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct BuildSsh {
    form: BuildSshForm,
    span: SourceSpan,
    storage: BuildSshStorage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
/// The authored `build.ssh` collection form.
pub enum BuildSshForm {
    /// List syntax with raw SSH grant strings in authored order.
    List,
    /// Mapping syntax with sensitive scalar SSH grant values.
    Map,
}

#[derive(Clone, PartialEq, Eq)]
enum BuildSshStorage {
    List(Vec<Located<String>>),
    Map(Vec<KeyValueEntry>),
}

impl fmt::Debug for BuildSsh {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BuildSsh")
            .field("form", &self.form)
            .field("span", &self.span)
            .field("storage", &"<redacted>")
            .finish()
    }
}

impl BuildSsh {
    /// Returns the authored collection span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the authored syntax form without exposing sensitive storage.
    #[must_use]
    pub const fn form(&self) -> BuildSshForm {
        self.form
    }

    /// Explicitly returns raw ordered list-form grants.
    #[must_use]
    pub fn as_list(&self) -> Option<&[Located<String>]> {
        let BuildSshStorage::List(values) = &self.storage else {
            return None;
        };
        Some(values)
    }

    /// Explicitly returns raw ordered mapping-form grants.
    #[must_use]
    pub fn as_map(&self) -> Option<&[KeyValueEntry]> {
        let BuildSshStorage::Map(entries) = &self.storage else {
            return None;
        };
        Some(entries)
    }

    pub(super) fn list(span: SourceSpan, values: Vec<Located<String>>) -> Self {
        Self {
            form: BuildSshForm::List,
            span,
            storage: BuildSshStorage::List(values),
        }
    }

    pub(super) fn map(span: SourceSpan, entries: Vec<KeyValueEntry>) -> Self {
        Self {
            form: BuildSshForm::Map,
            span,
            storage: BuildSshStorage::Map(entries),
        }
    }
}

/// One recognized build subfield and its source reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildField {
    kind: BuildFieldKind,
    reference: FieldReference,
}

impl BuildField {
    pub(super) const fn new(kind: BuildFieldKind, reference: FieldReference) -> Self {
        Self { kind, reference }
    }

    /// Returns the field's specification-level identity.
    #[must_use]
    pub const fn kind(&self) -> BuildFieldKind {
        self.kind
    }

    /// Returns source spans for reading or editing the retained value.
    #[must_use]
    pub const fn reference(&self) -> &FieldReference {
        &self.reference
    }
}

/// Recognized fields from the current Compose Build Specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BuildFieldKind {
    /// Additional named build contexts.
    AdditionalContexts,
    /// Dockerfile build arguments.
    Args,
    /// External cache sources.
    CacheFrom,
    /// External cache destinations.
    CacheTo,
    /// Build context.
    Context,
    /// Dockerfile path.
    Dockerfile,
    /// Inline Dockerfile content.
    DockerfileInline,
    /// Build entitlements.
    Entitlements,
    /// Build-time host mappings.
    ExtraHosts,
    /// Container isolation technology.
    Isolation,
    /// Build image labels.
    Labels,
    /// Build network mode.
    Network,
    /// Disable build cache.
    NoCache,
    /// Target platforms.
    Platforms,
    /// Privileged build mode.
    Privileged,
    /// Supply-chain provenance.
    Provenance,
    /// Pull referenced images.
    Pull,
    /// Software bill of materials.
    Sbom,
    /// Build-time secret grants.
    Secrets,
    /// SSH agent/socket grants.
    Ssh,
    /// Build shared-memory size.
    ShmSize,
    /// Additional output tags.
    Tags,
    /// Dockerfile target stage.
    Target,
    /// Build-container resource limits.
    Ulimits,
    /// Select build stages excluded from cache.
    NoCacheFilter,
}

impl BuildFieldKind {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "additional_contexts" => Self::AdditionalContexts,
            "args" => Self::Args,
            "cache_from" => Self::CacheFrom,
            "cache_to" => Self::CacheTo,
            "context" => Self::Context,
            "dockerfile" => Self::Dockerfile,
            "dockerfile_inline" => Self::DockerfileInline,
            "entitlements" => Self::Entitlements,
            "extra_hosts" => Self::ExtraHosts,
            "isolation" => Self::Isolation,
            "labels" => Self::Labels,
            "network" => Self::Network,
            "no_cache" => Self::NoCache,
            "no_cache_filter" => Self::NoCacheFilter,
            "platforms" => Self::Platforms,
            "privileged" => Self::Privileged,
            "provenance" => Self::Provenance,
            "pull" => Self::Pull,
            "sbom" => Self::Sbom,
            "secrets" => Self::Secrets,
            "ssh" => Self::Ssh,
            "shm_size" => Self::ShmSize,
            "tags" => Self::Tags,
            "target" => Self::Target,
            "ulimits" => Self::Ulimits,
            _ => return None,
        })
    }
}

/// A deploy definition split into independently classifiable fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployDefinition {
    span: SourceSpan,
    endpoint_mode: Option<Located<DeployEndpointMode>>,
    labels: Option<Box<Labels>>,
    mode: Option<Located<DeployMode>>,
    placement: Option<Box<DeployPlacement>>,
    replicas: Option<Located<DeployReplicas>>,
    resources: Option<Box<DeployResources>>,
    restart_policy: Option<Box<DeployRestartPolicy>>,
    rollback_config: Option<Box<DeployRollbackConfig>>,
    update_config: Option<Box<DeployUpdateConfig>>,
    fields: Vec<DeployField>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl DeployDefinition {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            endpoint_mode: None,
            labels: None,
            mode: None,
            placement: None,
            replicas: None,
            resources: None,
            restart_policy: None,
            rollback_config: None,
            update_config: None,
            fields: Vec::new(),
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn push_field(&mut self, field: DeployField) {
        self.fields.push(field);
    }

    pub(super) fn set_endpoint_mode(&mut self, endpoint_mode: Located<DeployEndpointMode>) {
        self.endpoint_mode = Some(endpoint_mode);
    }

    pub(super) fn set_labels(&mut self, labels: Labels) {
        self.labels = Some(Box::new(labels));
    }

    pub(super) fn set_mode(&mut self, mode: Located<DeployMode>) {
        self.mode = Some(mode);
    }

    pub(super) fn set_placement(&mut self, placement: DeployPlacement) {
        self.placement = Some(Box::new(placement));
    }

    pub(super) fn set_replicas(&mut self, replicas: Located<DeployReplicas>) {
        self.replicas = Some(replicas);
    }

    pub(super) fn set_resources(&mut self, resources: DeployResources) {
        self.resources = Some(Box::new(resources));
    }

    pub(super) fn set_restart_policy(&mut self, restart_policy: DeployRestartPolicy) {
        self.restart_policy = Some(Box::new(restart_policy));
    }
    pub(super) fn set_rollback_config(&mut self, rollback_config: DeployRollbackConfig) {
        self.rollback_config = Some(Box::new(rollback_config));
    }
    pub(super) fn set_update_config(&mut self, update_config: DeployUpdateConfig) {
        self.update_config = Some(Box::new(update_config));
    }

    pub(super) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(super) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the complete deploy mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the authored service-discovery endpoint mode.
    #[must_use]
    pub const fn endpoint_mode(&self) -> Option<&Located<DeployEndpointMode>> {
        self.endpoint_mode.as_ref()
    }

    /// Returns authored deployment labels without conflating them with service container labels.
    #[must_use]
    pub fn labels(&self) -> Option<&Labels> {
        self.labels.as_deref()
    }

    /// Returns the authored deployment mode.
    #[must_use]
    pub const fn mode(&self) -> Option<&Located<DeployMode>> {
        self.mode.as_ref()
    }

    /// Returns authored deploy placement without scheduling interpretation.
    #[must_use]
    pub fn placement(&self) -> Option<&DeployPlacement> {
        self.placement.as_deref()
    }

    /// Returns the authored replica-count spelling and YAML scalar category.
    #[must_use]
    pub const fn replicas(&self) -> Option<&Located<DeployReplicas>> {
        self.replicas.as_ref()
    }

    /// Returns authored deploy resources without resource-policy interpretation.
    #[must_use]
    pub fn resources(&self) -> Option<&DeployResources> {
        self.resources.as_deref()
    }

    /// Returns the authored deploy restart policy without using service `restart` semantics.
    #[must_use]
    pub fn restart_policy(&self) -> Option<&DeployRestartPolicy> {
        self.restart_policy.as_deref()
    }
    /// Returns the authored rollback configuration without rollout interpretation.
    #[must_use]
    pub fn rollback_config(&self) -> Option<&DeployRollbackConfig> {
        self.rollback_config.as_deref()
    }
    /// Returns the authored rolling-update configuration without rollout interpretation.
    #[must_use]
    pub fn update_config(&self) -> Option<&DeployUpdateConfig> {
        self.update_config.as_deref()
    }

    /// Returns recognized deploy fields in authored order.
    #[must_use]
    pub fn fields(&self) -> &[DeployField] {
        &self.fields
    }

    /// Finds the first recognized field of the requested kind.
    #[must_use]
    pub fn field(&self, kind: DeployFieldKind) -> Option<&DeployField> {
        self.fields.iter().find(|field| field.kind == kind)
    }

    /// Returns retained `x-` fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns fields not recognized by this release.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// A deploy endpoint mode with unknown provider values retained verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployEndpointMode {
    /// Use the virtual-IP service-discovery mode.
    Vip,
    /// Use DNS round-robin service discovery.
    Dnsrr,
    /// A value outside the documented Compose endpoint modes.
    Other(String),
}

impl DeployEndpointMode {
    pub(crate) fn parse(value: String) -> Self {
        match value.as_str() {
            "vip" => Self::Vip,
            "dnsrr" => Self::Dnsrr,
            _ => Self::Other(value),
        }
    }

    /// Returns whether the mode is one of Compose's documented endpoint modes.
    #[must_use]
    pub const fn is_documented(&self) -> bool {
        matches!(self, Self::Vip | Self::Dnsrr)
    }
}

/// A deploy mode with unknown provider values retained verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployMode {
    /// Run one task on every eligible node.
    Global,
    /// Run a caller-specified replica count.
    Replicated,
    /// A value outside the documented Compose deploy modes.
    Other(String),
}

impl DeployMode {
    pub(crate) fn parse(value: String) -> Self {
        match value.as_str() {
            "global" => Self::Global,
            "replicated" => Self::Replicated,
            _ => Self::Other(value),
        }
    }

    /// Returns whether the mode is one of Compose's documented deployment modes.
    #[must_use]
    pub const fn is_documented(&self) -> bool {
        matches!(self, Self::Global | Self::Replicated)
    }
}

/// A raw deploy replica-count scalar with its YAML category preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployReplicas {
    /// A YAML numeric scalar, retained without integer validation or normalization.
    YamlNumber(String),
    /// A YAML string scalar, including empty and deferred expressions.
    String(String),
}

/// Authored deploy resources with source-aware child fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployResources {
    span: SourceSpan,
    limits: Option<Box<DeployResourceLimits>>,
    reservations: Option<Box<DeployResourceReservations>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl DeployResources {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            limits: None,
            reservations: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_limits(&mut self, limits: DeployResourceLimits) {
        self.limits = Some(Box::new(limits));
    }

    pub(super) fn set_reservations(&mut self, reservations: DeployResourceReservations) {
        self.reservations = Some(Box::new(reservations));
    }

    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }

    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete resources mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns authored resource limits without default or runtime interpretation.
    #[must_use]
    pub fn limits(&self) -> Option<&DeployResourceLimits> {
        self.limits.as_deref()
    }

    /// Returns authored resource reservations without scheduling interpretation.
    #[must_use]
    pub fn reservations(&self) -> Option<&DeployResourceReservations> {
        self.reservations.as_deref()
    }

    /// Returns retained resources extensions.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns retained unknown resources fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// Authored deploy resource reservations with source-aware child fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployResourceReservations {
    span: SourceSpan,
    cpus: Option<Located<DeployResourceCpus>>,
    memory: Option<Located<DeployResourceMemory>>,
    generic_resources: Option<DeployGenericResources>,
    devices: Option<DeployReservationDevices>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl DeployResourceReservations {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            cpus: None,
            memory: None,
            generic_resources: None,
            devices: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_cpus(&mut self, cpus: Located<DeployResourceCpus>) {
        self.cpus = Some(cpus);
    }

    pub(super) fn set_memory(&mut self, memory: Located<DeployResourceMemory>) {
        self.memory = Some(memory);
    }

    pub(super) fn set_generic_resources(&mut self, generic_resources: DeployGenericResources) {
        self.generic_resources = Some(generic_resources);
    }

    pub(super) fn set_devices(&mut self, devices: DeployReservationDevices) {
        self.devices = Some(devices);
    }

    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }

    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete resource-reservations mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the authored deploy resource reservation CPU scalar spelling and category.
    #[must_use]
    pub const fn cpus(&self) -> Option<&Located<DeployResourceCpus>> {
        self.cpus.as_ref()
    }

    /// Returns the authored deploy resource reservation memory value and its source location.
    #[must_use]
    pub const fn memory(&self) -> Option<&Located<DeployResourceMemory>> {
        self.memory.as_ref()
    }

    /// Returns authored ordered generic-resource reservations, including an explicit empty list.
    #[must_use]
    pub const fn generic_resources(&self) -> Option<&DeployGenericResources> {
        self.generic_resources.as_ref()
    }

    /// Returns authored reservation devices, including an explicit empty list.
    #[must_use]
    pub const fn devices(&self) -> Option<&DeployReservationDevices> {
        self.devices.as_ref()
    }

    /// Returns retained resource-reservation extensions.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns retained unknown resource-reservation fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// Ordered schema-backed deploy resource-reservation devices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployReservationDevices {
    span: SourceSpan,
    items: Vec<DeployReservationDevice>,
}

impl DeployReservationDevices {
    pub(super) const fn new(span: SourceSpan, items: Vec<DeployReservationDevice>) -> Self {
        Self { span, items }
    }

    /// Returns the complete devices sequence span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns items in authored order, including duplicate and partially recovered entries.
    #[must_use]
    pub fn items(&self) -> &[DeployReservationDevice] {
        &self.items
    }
}

/// One schema-backed deploy resource-reservation device item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployReservationDevice {
    span: SourceSpan,
    form: DeployReservationDeviceForm,
    capabilities: Option<DeployReservationDeviceCapabilities>,
    driver: Option<Located<String>>,
    count: Option<Located<DeployReservationDeviceCount>>,
    device_ids: Option<DeployReservationDeviceIds>,
    options: Option<DeployReservationDeviceOptions>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl DeployReservationDevice {
    pub(super) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            form: DeployReservationDeviceForm::Mapping,
            capabilities: None,
            driver: None,
            count: None,
            device_ids: None,
            options: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn unmodeled(span: SourceSpan) -> Self {
        Self {
            span,
            form: DeployReservationDeviceForm::Unmodeled,
            capabilities: None,
            driver: None,
            count: None,
            device_ids: None,
            options: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_capabilities(&mut self, capabilities: DeployReservationDeviceCapabilities) {
        self.capabilities = Some(capabilities);
    }

    pub(super) fn set_driver(&mut self, driver: Located<String>) {
        self.driver = Some(driver);
    }

    pub(super) fn set_count(&mut self, count: Located<DeployReservationDeviceCount>) {
        self.count = Some(count);
    }

    pub(super) fn set_device_ids(&mut self, device_ids: DeployReservationDeviceIds) {
        self.device_ids = Some(device_ids);
    }

    pub(super) fn set_options(&mut self, options: DeployReservationDeviceOptions) {
        self.options = Some(options);
    }

    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }

    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete item span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns whether the item was a mapping or an unmodeled sequence entry.
    #[must_use]
    pub const fn form(&self) -> DeployReservationDeviceForm {
        self.form
    }

    /// Returns the required capabilities list when its form was valid.
    #[must_use]
    pub const fn capabilities(&self) -> Option<&DeployReservationDeviceCapabilities> {
        self.capabilities.as_ref()
    }

    /// Returns the optional raw device driver when its form was valid.
    #[must_use]
    pub const fn driver(&self) -> Option<&Located<String>> {
        self.driver.as_ref()
    }

    /// Returns the optional raw device allocation count when its scalar form was valid.
    #[must_use]
    pub const fn count(&self) -> Option<&Located<DeployReservationDeviceCount>> {
        self.count.as_ref()
    }

    /// Returns optional ordered device allocation IDs, including an explicit empty list.
    #[must_use]
    pub const fn device_ids(&self) -> Option<&DeployReservationDeviceIds> {
        self.device_ids.as_ref()
    }

    /// Returns optional raw device options without provider-specific interpretation.
    #[must_use]
    pub const fn options(&self) -> Option<&DeployReservationDeviceOptions> {
        self.options.as_ref()
    }

    /// Returns retained extensions.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns retained unknown or malformed fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// Authored resource-reservation device item shape retained without coercion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployReservationDeviceForm {
    /// A mapping-form item.
    Mapping,
    /// A non-mapping sequence item retained as evidence.
    Unmodeled,
}

/// Raw resource-reservation device allocation-count scalar spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployReservationDeviceCount {
    /// An exact YAML integer scalar spelling.
    YamlInteger(String),
    /// An exact YAML string scalar spelling.
    String(String),
}

/// Ordered raw resource-reservation device allocation IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployReservationDeviceIds {
    span: SourceSpan,
    items: Vec<DeployReservationDeviceId>,
}

impl DeployReservationDeviceIds {
    pub(super) const fn new(span: SourceSpan, items: Vec<DeployReservationDeviceId>) -> Self {
        Self { span, items }
    }

    /// Returns the complete device-IDs sequence span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns IDs in authored order, including duplicates and unmodeled entries.
    #[must_use]
    pub fn items(&self) -> &[DeployReservationDeviceId] {
        &self.items
    }
}

/// One resource-reservation device allocation ID retained without interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployReservationDeviceId {
    span: SourceSpan,
    form: DeployReservationDeviceIdForm,
    value: Option<Located<String>>,
}

impl DeployReservationDeviceId {
    pub(super) fn string(value: Located<String>) -> Self {
        Self {
            span: value.span(),
            form: DeployReservationDeviceIdForm::String,
            value: Some(value),
        }
    }

    pub(super) const fn unmodeled(span: SourceSpan) -> Self {
        Self {
            span,
            form: DeployReservationDeviceIdForm::Unmodeled,
            value: None,
        }
    }

    /// Returns the complete item span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns whether the item was a YAML string or retained unmodeled value.
    #[must_use]
    pub const fn form(&self) -> DeployReservationDeviceIdForm {
        self.form
    }

    /// Returns the exact YAML string when the item had string form.
    #[must_use]
    pub const fn value(&self) -> Option<&Located<String>> {
        self.value.as_ref()
    }
}

/// Resource-reservation device allocation-ID item shape retained without coercion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployReservationDeviceIdForm {
    /// A YAML string item.
    String,
    /// A non-string sequence item retained as evidence.
    Unmodeled,
}

/// Schema-shaped resource-reservation device options retaining map or list syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployReservationDeviceOptions {
    /// Ordered mapping entries plus malformed or duplicate fields retained as evidence.
    Map {
        /// Complete mapping span.
        span: SourceSpan,
        /// Valid non-empty strict-string keyed scalar entries.
        entries: Vec<KeyValueEntry>,
        /// Malformed or duplicate mapping fields.
        unmodeled_entries: Vec<FieldReference>,
    },
    /// Ordered list entries, including malformed entries.
    List {
        /// Complete sequence span.
        span: SourceSpan,
        /// Items in authored order.
        items: Vec<DeployReservationDeviceOptionItem>,
    },
}

impl DeployReservationDeviceOptions {
    /// Returns the complete authored collection span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Map { span, .. } | Self::List { span, .. } => *span,
        }
    }

    /// Returns valid map entries when the authored form was a mapping.
    #[must_use]
    pub fn as_map(&self) -> Option<&[KeyValueEntry]> {
        let Self::Map { entries, .. } = self else { return None };
        Some(entries)
    }

    /// Returns malformed or duplicate map fields when the authored form was a mapping.
    #[must_use]
    pub fn unmodeled_entries(&self) -> Option<&[FieldReference]> {
        let Self::Map { unmodeled_entries, .. } = self else {
            return None;
        };
        Some(unmodeled_entries)
    }

    /// Returns ordered list entries when the authored form was a sequence.
    #[must_use]
    pub fn as_list(&self) -> Option<&[DeployReservationDeviceOptionItem]> {
        let Self::List { items, .. } = self else { return None };
        Some(items)
    }
}

/// One device-options list item retained without splitting or coercion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployReservationDeviceOptionItem {
    span: SourceSpan,
    form: DeployReservationDeviceOptionItemForm,
    value: Option<Located<String>>,
}

impl DeployReservationDeviceOptionItem {
    pub(super) fn string(value: Located<String>) -> Self {
        Self {
            span: value.span(),
            form: DeployReservationDeviceOptionItemForm::String,
            value: Some(value),
        }
    }
    pub(super) const fn unmodeled(span: SourceSpan) -> Self {
        Self {
            span,
            form: DeployReservationDeviceOptionItemForm::Unmodeled,
            value: None,
        }
    }
    /// Returns the complete item span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns whether the item was a strict YAML string or retained unmodeled value.
    #[must_use]
    pub const fn form(&self) -> DeployReservationDeviceOptionItemForm {
        self.form
    }
    /// Returns the exact string when the item had string form.
    #[must_use]
    pub const fn value(&self) -> Option<&Located<String>> {
        self.value.as_ref()
    }
}

/// Resource-reservation device-options list item shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployReservationDeviceOptionItemForm {
    /// A strict YAML string list item.
    String,
    /// A non-string list item retained as evidence.
    Unmodeled,
}

/// Ordered required resource-reservation device capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployReservationDeviceCapabilities {
    span: SourceSpan,
    items: Vec<DeployReservationDeviceCapability>,
}

impl DeployReservationDeviceCapabilities {
    pub(super) const fn new(span: SourceSpan, items: Vec<DeployReservationDeviceCapability>) -> Self {
        Self { span, items }
    }

    /// Returns the complete capabilities sequence span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns items in authored order, including duplicates and unmodeled entries.
    #[must_use]
    pub fn items(&self) -> &[DeployReservationDeviceCapability] {
        &self.items
    }
}

/// One resource-reservation device capability retained without name interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployReservationDeviceCapability {
    span: SourceSpan,
    form: DeployReservationDeviceCapabilityForm,
    value: Option<Located<String>>,
}

impl DeployReservationDeviceCapability {
    pub(super) fn string(value: Located<String>) -> Self {
        Self {
            span: value.span(),
            form: DeployReservationDeviceCapabilityForm::String,
            value: Some(value),
        }
    }

    pub(super) const fn unmodeled(span: SourceSpan) -> Self {
        Self {
            span,
            form: DeployReservationDeviceCapabilityForm::Unmodeled,
            value: None,
        }
    }

    /// Returns the complete item span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns whether the item was a YAML string or retained unmodeled value.
    #[must_use]
    pub const fn form(&self) -> DeployReservationDeviceCapabilityForm {
        self.form
    }

    /// Returns the exact YAML string when the item had string form.
    #[must_use]
    pub const fn value(&self) -> Option<&Located<String>> {
        self.value.as_ref()
    }
}

/// Resource-reservation device capability item shape retained without coercion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployReservationDeviceCapabilityForm {
    /// A YAML string item.
    String,
    /// A non-string sequence item retained as evidence.
    Unmodeled,
}

/// Ordered schema-backed deploy generic-resource reservations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployGenericResources {
    span: SourceSpan,
    items: Vec<DeployGenericResource>,
}

impl DeployGenericResources {
    pub(super) const fn new(span: SourceSpan, items: Vec<DeployGenericResource>) -> Self {
        Self { span, items }
    }

    /// Returns the complete generic-resources sequence span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns items in authored order, including duplicates and partially recovered entries.
    #[must_use]
    pub fn items(&self) -> &[DeployGenericResource] {
        &self.items
    }
}

/// One schema-backed generic-resource reservation item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployGenericResource {
    span: SourceSpan,
    form: DeployGenericResourceForm,
    discrete_resource_spec: Option<DeployDiscreteResourceSpec>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl DeployGenericResource {
    pub(super) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            form: DeployGenericResourceForm::Mapping,
            discrete_resource_spec: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }
    pub(super) fn unmodeled(span: SourceSpan) -> Self {
        Self {
            span,
            form: DeployGenericResourceForm::Unmodeled,
            discrete_resource_spec: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }
    pub(super) fn set_discrete_resource_spec(&mut self, value: DeployDiscreteResourceSpec) {
        self.discrete_resource_spec = Some(value);
    }
    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }
    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }
    /// Returns this item mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns whether the item was a mapping or an unmodeled sequence entry.
    #[must_use]
    pub const fn form(&self) -> DeployGenericResourceForm {
        self.form
    }
    /// Returns the optional schema-backed discrete resource specification.
    #[must_use]
    pub const fn discrete_resource_spec(&self) -> Option<&DeployDiscreteResourceSpec> {
        self.discrete_resource_spec.as_ref()
    }
    /// Returns retained extensions.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }
    /// Returns retained unknown or malformed fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// Authored generic-resource item shape retained without coercion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployGenericResourceForm {
    /// A mapping-form item.
    Mapping,
    /// A non-mapping sequence item retained as evidence.
    Unmodeled,
}

/// Schema-backed discrete generic-resource specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployDiscreteResourceSpec {
    span: SourceSpan,
    kind: Option<Located<String>>,
    value: Option<Located<DeployDiscreteResourceValue>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl DeployDiscreteResourceSpec {
    pub(super) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            kind: None,
            value: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }
    pub(super) fn set_kind(&mut self, value: Located<String>) {
        self.kind = Some(value);
    }
    pub(super) fn set_value(&mut self, value: Located<DeployDiscreteResourceValue>) {
        self.value = Some(value);
    }
    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }
    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }
    /// Returns the complete discrete-resource-spec mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns the optional raw kind scalar.
    #[must_use]
    pub const fn kind(&self) -> Option<&Located<String>> {
        self.kind.as_ref()
    }
    /// Returns the optional raw value scalar category.
    #[must_use]
    pub const fn value(&self) -> Option<&Located<DeployDiscreteResourceValue>> {
        self.value.as_ref()
    }
    /// Returns retained extensions.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }
    /// Returns retained unknown or malformed members.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// Raw scalar category for a discrete generic-resource value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployDiscreteResourceValue {
    /// A YAML numeric scalar retained without numeric interpretation.
    YamlNumber(String),
    /// A YAML string scalar retained without schema-specific interpretation.
    String(String),
}

/// Authored deploy resource limits with source-aware child fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployResourceLimits {
    span: SourceSpan,
    cpus: Option<Located<DeployResourceCpus>>,
    memory: Option<Located<DeployResourceMemory>>,
    pids: Option<Located<DeployResourcePids>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl DeployResourceLimits {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            cpus: None,
            memory: None,
            pids: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_pids(&mut self, pids: Located<DeployResourcePids>) {
        self.pids = Some(pids);
    }

    pub(super) fn set_cpus(&mut self, cpus: Located<DeployResourceCpus>) {
        self.cpus = Some(cpus);
    }

    pub(super) fn set_memory(&mut self, memory: Located<DeployResourceMemory>) {
        self.memory = Some(memory);
    }

    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }

    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete resource-limits mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the authored deploy resource PID scalar spelling and category.
    #[must_use]
    pub const fn pids(&self) -> Option<&Located<DeployResourcePids>> {
        self.pids.as_ref()
    }

    /// Returns the authored deploy resource CPU scalar spelling and category.
    #[must_use]
    pub const fn cpus(&self) -> Option<&Located<DeployResourceCpus>> {
        self.cpus.as_ref()
    }

    /// Returns the authored deploy resource memory value and its source location.
    #[must_use]
    pub const fn memory(&self) -> Option<&Located<DeployResourceMemory>> {
        self.memory.as_ref()
    }

    /// Returns retained resource-limit extensions.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns retained unknown resource-limit fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// Raw deploy resource PID scalar category and spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployResourcePids {
    /// A YAML integer scalar without range validation.
    YamlInteger(String),
    /// A YAML string scalar without numeric validation.
    String(String),
}

/// Raw deploy resource CPU scalar category and spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployResourceCpus {
    /// A YAML integer or floating-point scalar without numeric validation or normalization.
    YamlNumber(String),
    /// A YAML string scalar without numeric validation.
    String(String),
}

/// Raw-preserving deploy resource memory value with deploy-specific classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployResourceMemory {
    raw: String,
    kind: DeployResourceMemoryKind,
}

impl DeployResourceMemory {
    pub(crate) fn parse(raw: String) -> Self {
        let kind = if raw.contains('$') {
            DeployResourceMemoryKind::Expression
        } else if let Some((amount_raw, unit)) = split_deploy_resource_memory_unit(&raw) {
            if deploy_resource_memory_lexical_zero(amount_raw) {
                DeployResourceMemoryKind::Zero {
                    amount_raw: amount_raw.to_owned(),
                    unit: Some(unit),
                }
            } else {
                DeployResourceMemoryKind::Documented {
                    amount_raw: amount_raw.to_owned(),
                    unit,
                }
            }
        } else if deploy_resource_memory_lexical_zero(&raw) {
            DeployResourceMemoryKind::Zero {
                amount_raw: raw.clone(),
                unit: None,
            }
        } else {
            DeployResourceMemoryKind::ProviderDependentString
        };
        Self { raw, kind }
    }

    /// Returns the exact deploy resource memory scalar text without normalization.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns the non-destructive deploy resource memory classification.
    #[must_use]
    pub const fn kind(&self) -> &DeployResourceMemoryKind {
        &self.kind
    }
}

/// Raw-preserving semantic family of a deploy resource memory string.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployResourceMemoryKind {
    /// A string ending in one documented lowercase byte suffix.
    Documented {
        /// Exact text before the suffix; no amount grammar is inferred.
        amount_raw: String,
        /// Exact documented suffix family.
        unit: DeployResourceMemoryUnit,
    },
    /// An all-zero amount spelling whose runtime meaning is not inferred.
    Zero {
        /// Exact all-zero amount spelling.
        amount_raw: String,
        /// Documented suffix when one was present.
        unit: Option<DeployResourceMemoryUnit>,
    },
    /// A dollar-bearing string deferred to Compose interpolation.
    Expression,
    /// A string outside the documented lowercase-suffix family.
    ProviderDependentString,
}

/// One lowercase byte-unit suffix documented for deploy resource memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeployResourceMemoryUnit {
    /// Bytes (`b`).
    B,
    /// Kilobytes (`k`).
    K,
    /// Kilobytes (`kb`).
    Kb,
    /// Megabytes (`m`).
    M,
    /// Megabytes (`mb`).
    Mb,
    /// Gigabytes (`g`).
    G,
    /// Gigabytes (`gb`).
    Gb,
}

impl DeployResourceMemoryUnit {
    /// Returns the exact lowercase documented suffix.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::B => "b",
            Self::K => "k",
            Self::Kb => "kb",
            Self::M => "m",
            Self::Mb => "mb",
            Self::G => "g",
            Self::Gb => "gb",
        }
    }
}

fn split_deploy_resource_memory_unit(value: &str) -> Option<(&str, DeployResourceMemoryUnit)> {
    for (suffix, unit) in [
        ("kb", DeployResourceMemoryUnit::Kb),
        ("mb", DeployResourceMemoryUnit::Mb),
        ("gb", DeployResourceMemoryUnit::Gb),
        ("b", DeployResourceMemoryUnit::B),
        ("k", DeployResourceMemoryUnit::K),
        ("m", DeployResourceMemoryUnit::M),
        ("g", DeployResourceMemoryUnit::G),
    ] {
        if let Some(amount) = value.strip_suffix(suffix) {
            if !amount.is_empty() {
                return Some((amount, unit));
            }
        }
    }
    None
}

fn deploy_resource_memory_lexical_zero(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte == b'0')
}

/// A deploy restart-policy mapping with independent raw-preserving members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployRestartPolicy {
    span: SourceSpan,
    condition: Option<Located<DeployRestartCondition>>,
    delay: Option<Located<DeployRestartDuration>>,
    max_attempts: Option<Located<DeployRestartMaxAttempts>>,
    window: Option<Located<DeployRestartDuration>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl DeployRestartPolicy {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            condition: None,
            delay: None,
            max_attempts: None,
            window: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }
    pub(super) fn set_condition(&mut self, value: Located<DeployRestartCondition>) {
        self.condition = Some(value);
    }
    pub(super) fn set_delay(&mut self, value: Located<DeployRestartDuration>) {
        self.delay = Some(value);
    }
    pub(super) fn set_max_attempts(&mut self, value: Located<DeployRestartMaxAttempts>) {
        self.max_attempts = Some(value);
    }
    pub(super) fn set_window(&mut self, value: Located<DeployRestartDuration>) {
        self.window = Some(value);
    }
    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }
    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }
    /// Returns the complete restart-policy mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns the explicit restart condition.
    #[must_use]
    pub const fn condition(&self) -> Option<&Located<DeployRestartCondition>> {
        self.condition.as_ref()
    }
    /// Returns the raw delay spelling.
    #[must_use]
    pub const fn delay(&self) -> Option<&Located<DeployRestartDuration>> {
        self.delay.as_ref()
    }
    /// Returns the raw maximum-attempts YAML scalar.
    #[must_use]
    pub const fn max_attempts(&self) -> Option<&Located<DeployRestartMaxAttempts>> {
        self.max_attempts.as_ref()
    }
    /// Returns the raw restart window spelling.
    #[must_use]
    pub const fn window(&self) -> Option<&Located<DeployRestartDuration>> {
        self.window.as_ref()
    }
    /// Returns retained restart-policy extensions.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }
    /// Returns retained unknown restart-policy fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// A deploy restart condition with unknown and deferred values retained.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployRestartCondition {
    /// Do not restart failed tasks.
    None,
    /// Restart after failure.
    OnFailure,
    /// Restart regardless of termination result.
    Any,
    /// A deferred expression.
    Expression(String),
    /// An unknown retained condition.
    Other(String),
}
impl DeployRestartCondition {
    pub(crate) fn parse(value: String) -> Self {
        match value.as_str() {
            "none" => Self::None,
            "on-failure" => Self::OnFailure,
            "any" => Self::Any,
            _ if value.contains('$') => Self::Expression(value),
            _ => Self::Other(value),
        }
    }
}

/// A raw deploy restart duration spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployRestartDuration(String);
impl DeployRestartDuration {
    pub(crate) const fn new(value: String) -> Self {
        Self(value)
    }
    /// Returns the exact duration spelling.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.0
    }
}

/// A raw deploy max-attempts YAML integer/string scalar category and spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployRestartMaxAttempts {
    /// A YAML integer scalar without range validation.
    YamlNumber(String),
    /// A YAML string scalar without numeric validation.
    String(String),
}

/// A deploy rollback configuration with independent raw-preserving members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployRollbackConfig {
    span: SourceSpan,
    parallelism: Option<Located<DeployRollbackParallelism>>,
    delay: Option<Located<String>>,
    monitor: Option<Located<String>>,
    failure_action: Option<Located<String>>,
    max_failure_ratio: Option<Located<DeployRollbackMaxFailureRatio>>,
    order: Option<Located<DeployRollbackOrder>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}
impl DeployRollbackConfig {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            parallelism: None,
            delay: None,
            monitor: None,
            failure_action: None,
            max_failure_ratio: None,
            order: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }
    pub(super) fn set_parallelism(&mut self, value: Located<DeployRollbackParallelism>) {
        self.parallelism = Some(value);
    }
    pub(super) fn set_delay(&mut self, value: Located<String>) {
        self.delay = Some(value);
    }
    pub(super) fn set_monitor(&mut self, value: Located<String>) {
        self.monitor = Some(value);
    }
    pub(super) fn set_failure_action(&mut self, value: Located<String>) {
        self.failure_action = Some(value);
    }
    pub(super) fn set_max_failure_ratio(&mut self, value: Located<DeployRollbackMaxFailureRatio>) {
        self.max_failure_ratio = Some(value);
    }
    pub(super) fn set_order(&mut self, value: Located<DeployRollbackOrder>) {
        self.order = Some(value);
    }
    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }
    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }
    /// Returns the complete rollback-config mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns the raw rollback parallelism scalar.
    #[must_use]
    pub const fn parallelism(&self) -> Option<&Located<DeployRollbackParallelism>> {
        self.parallelism.as_ref()
    }
    /// Returns the raw rollback delay string.
    #[must_use]
    pub const fn delay(&self) -> Option<&Located<String>> {
        self.delay.as_ref()
    }
    /// Returns the raw rollback monitor string.
    #[must_use]
    pub const fn monitor(&self) -> Option<&Located<String>> {
        self.monitor.as_ref()
    }
    /// Returns the raw rollback failure action.
    #[must_use]
    pub const fn failure_action(&self) -> Option<&Located<String>> {
        self.failure_action.as_ref()
    }
    /// Returns the raw rollback maximum failure ratio.
    #[must_use]
    pub const fn max_failure_ratio(&self) -> Option<&Located<DeployRollbackMaxFailureRatio>> {
        self.max_failure_ratio.as_ref()
    }
    /// Returns the rollback order.
    #[must_use]
    pub const fn order(&self) -> Option<&Located<DeployRollbackOrder>> {
        self.order.as_ref()
    }
    /// Returns retained rollback-config extensions.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }
    /// Returns retained rollback-config malformed or unknown fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}
/// Raw rollback parallelism scalar category and spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployRollbackParallelism {
    /// A YAML integer scalar without range validation.
    YamlInteger(String),
    /// A strict YAML string scalar without numeric validation.
    String(String),
}
/// Raw rollback maximum-failure-ratio scalar category and spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployRollbackMaxFailureRatio {
    /// A YAML number scalar without range validation.
    YamlNumber(String),
    /// A strict YAML string scalar without numeric validation.
    String(String),
}
/// Rollback order with unsupported values retained.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployRollbackOrder {
    /// Stop the old task before starting the new task.
    StopFirst,
    /// Start the new task before stopping the old task.
    StartFirst,
    /// A retained provider-specific order.
    Other(String),
}
impl DeployRollbackOrder {
    pub(crate) fn parse(value: String) -> Self {
        match value.as_str() {
            "stop-first" => Self::StopFirst,
            "start-first" => Self::StartFirst,
            _ => Self::Other(value),
        }
    }
    pub(crate) const fn is_documented(&self) -> bool {
        matches!(self, Self::StopFirst | Self::StartFirst)
    }
}

/// A deploy rolling-update configuration with independent raw-preserving members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployUpdateConfig {
    span: SourceSpan,
    parallelism: Option<Located<DeployUpdateParallelism>>,
    delay: Option<Located<String>>,
    monitor: Option<Located<String>>,
    failure_action: Option<Located<String>>,
    max_failure_ratio: Option<Located<DeployUpdateMaxFailureRatio>>,
    order: Option<Located<DeployUpdateOrder>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}
impl DeployUpdateConfig {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            parallelism: None,
            delay: None,
            monitor: None,
            failure_action: None,
            max_failure_ratio: None,
            order: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }
    pub(super) fn set_parallelism(&mut self, value: Located<DeployUpdateParallelism>) {
        self.parallelism = Some(value);
    }
    pub(super) fn set_delay(&mut self, value: Located<String>) {
        self.delay = Some(value);
    }
    pub(super) fn set_monitor(&mut self, value: Located<String>) {
        self.monitor = Some(value);
    }
    pub(super) fn set_failure_action(&mut self, value: Located<String>) {
        self.failure_action = Some(value);
    }
    pub(super) fn set_max_failure_ratio(&mut self, value: Located<DeployUpdateMaxFailureRatio>) {
        self.max_failure_ratio = Some(value);
    }
    pub(super) fn set_order(&mut self, value: Located<DeployUpdateOrder>) {
        self.order = Some(value);
    }
    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }
    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }
    /// Returns the complete update-config mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns the raw update parallelism scalar.
    #[must_use]
    pub const fn parallelism(&self) -> Option<&Located<DeployUpdateParallelism>> {
        self.parallelism.as_ref()
    }
    /// Returns the raw update delay string.
    #[must_use]
    pub const fn delay(&self) -> Option<&Located<String>> {
        self.delay.as_ref()
    }
    /// Returns the raw update monitor string.
    #[must_use]
    pub const fn monitor(&self) -> Option<&Located<String>> {
        self.monitor.as_ref()
    }
    /// Returns the raw update failure action.
    #[must_use]
    pub const fn failure_action(&self) -> Option<&Located<String>> {
        self.failure_action.as_ref()
    }
    /// Returns the raw maximum failure ratio.
    #[must_use]
    pub const fn max_failure_ratio(&self) -> Option<&Located<DeployUpdateMaxFailureRatio>> {
        self.max_failure_ratio.as_ref()
    }
    /// Returns the update order.
    #[must_use]
    pub const fn order(&self) -> Option<&Located<DeployUpdateOrder>> {
        self.order.as_ref()
    }
    /// Returns retained update-config extensions.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }
    /// Returns retained update-config malformed or unknown fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}
/// Raw update parallelism scalar category and spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployUpdateParallelism {
    /// A YAML integer scalar without range validation.
    YamlInteger(String),
    /// A strict YAML string scalar without numeric validation.
    String(String),
}
/// Raw update maximum-failure-ratio scalar category and spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployUpdateMaxFailureRatio {
    /// A YAML number scalar without range validation.
    YamlNumber(String),
    /// A strict YAML string scalar without numeric validation.
    String(String),
}
/// Update order with unsupported values retained.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployUpdateOrder {
    /// Stop the old task before starting the new task.
    StopFirst,
    /// Start the new task before stopping the old task.
    StartFirst,
    /// A retained provider-specific order.
    Other(String),
}
impl DeployUpdateOrder {
    pub(crate) fn parse(value: String) -> Self {
        match value.as_str() {
            "stop-first" => Self::StopFirst,
            "start-first" => Self::StartFirst,
            _ => Self::Other(value),
        }
    }
    pub(crate) const fn is_documented(&self) -> bool {
        matches!(self, Self::StopFirst | Self::StartFirst)
    }
}

/// Authored deploy placement with source-aware child fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployPlacement {
    span: SourceSpan,
    constraints: Option<Vec<Located<String>>>,
    preferences: Option<Vec<DeployPlacementPreference>>,
    max_replicas_per_node: Option<Located<DeployPlacementMaxReplicasPerNode>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl DeployPlacement {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            constraints: None,
            preferences: None,
            max_replicas_per_node: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_constraints(&mut self, constraints: Vec<Located<String>>) {
        self.constraints = Some(constraints);
    }

    pub(super) fn set_preferences(&mut self, preferences: Vec<DeployPlacementPreference>) {
        self.preferences = Some(preferences);
    }

    pub(super) fn set_max_replicas_per_node(&mut self, value: Located<DeployPlacementMaxReplicasPerNode>) {
        self.max_replicas_per_node = Some(value);
    }

    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }

    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete placement mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns ordered raw constraints, including duplicates and empty strings.
    #[must_use]
    pub fn constraints(&self) -> Option<&[Located<String>]> {
        self.constraints.as_deref()
    }

    /// Returns ordered placement preferences, including explicit empty mappings.
    #[must_use]
    pub fn preferences(&self) -> Option<&[DeployPlacementPreference]> {
        self.preferences.as_deref()
    }

    /// Returns the authored max-replicas-per-node scalar spelling and category.
    #[must_use]
    pub const fn max_replicas_per_node(&self) -> Option<&Located<DeployPlacementMaxReplicasPerNode>> {
        self.max_replicas_per_node.as_ref()
    }

    /// Returns retained placement extensions.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns retained unknown placement fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// One authored placement preference mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployPlacementPreference {
    span: SourceSpan,
    spread: Option<Located<String>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl DeployPlacementPreference {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            spread: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_spread(&mut self, spread: Located<String>) {
        self.spread = Some(spread);
    }

    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }

    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete preference mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the raw optional spread expression without evaluating it.
    #[must_use]
    pub const fn spread(&self) -> Option<&Located<String>> {
        self.spread.as_ref()
    }

    /// Returns retained preference extensions.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns retained unknown preference fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// Raw deploy placement max-replicas-per-node scalar category and spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeployPlacementMaxReplicasPerNode {
    /// A YAML integer scalar without range validation.
    YamlInteger(String),
    /// A YAML string scalar without numeric validation.
    String(String),
}

/// One recognized deploy subfield and its source reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployField {
    kind: DeployFieldKind,
    reference: FieldReference,
}

impl DeployField {
    pub(super) const fn new(kind: DeployFieldKind, reference: FieldReference) -> Self {
        Self { kind, reference }
    }

    /// Returns the field's specification-level identity.
    #[must_use]
    pub const fn kind(&self) -> DeployFieldKind {
        self.kind
    }

    /// Returns source spans for reading or editing the retained value.
    #[must_use]
    pub const fn reference(&self) -> &FieldReference {
        &self.reference
    }
}

/// Recognized fields from the current Compose Deploy Specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeployFieldKind {
    /// Service-discovery endpoint mode.
    EndpointMode,
    /// Platform-service labels.
    Labels,
    /// Replication or job mode.
    Mode,
    /// Node-placement rules.
    Placement,
    /// Desired replica count.
    Replicas,
    /// Resource limits and reservations.
    Resources,
    /// Deploy-level restart policy.
    RestartPolicy,
    /// Rollback behavior.
    RollbackConfig,
    /// Rolling-update behavior.
    UpdateConfig,
}

impl DeployFieldKind {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "endpoint_mode" => Self::EndpointMode,
            "labels" => Self::Labels,
            "mode" => Self::Mode,
            "placement" => Self::Placement,
            "replicas" => Self::Replicas,
            "resources" => Self::Resources,
            "restart_policy" => Self::RestartPolicy,
            "rollback_config" => Self::RollbackConfig,
            "update_config" => Self::UpdateConfig,
            _ => return None,
        })
    }
}
