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
    /// Returns the complete restart-policy mapping span.
    /// Returns the complete restart-policy mapping span.
    /// Returns the complete restart-policy mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the explicitly authored long-syntax build context when it is a string scalar.
    ///
    /// Other build subfields remain source-addressable references and are not semantically
    /// interpreted by this model.
    /// Returns the explicit restart condition.
    #[must_use]
    pub const fn context(&self) -> Option<&Located<String>> {
        self.values.context.as_ref()
    }

    /// Returns authored additional build contexts without normalizing mapping and list syntax.
    ///
    /// List entries remain raw ordered strings, including duplicates and `NAME=VALUE` spelling.
    /// Mapping entries retain scalar kinds and authored order. This model does not interpret
    /// names, paths, URLs, images, service schemes, or builder behavior.
    /// Returns the raw delay spelling.
    #[must_use]
    pub const fn additional_contexts(&self) -> Option<&BuildAdditionalContexts> {
        self.values.additional_contexts.as_ref()
    }

    /// Returns authored build entitlements in order as opaque raw strings.
    ///
    /// Explicit emptiness remains distinct from omission. This model retains duplicates and does
    /// not infer entitlement allowlists, privilege state, `BuildKit` or platform support, build
    /// execution, or runtime effect.
    /// Returns the raw maximum-attempts YAML scalar.
    #[must_use]
    pub fn entitlements(&self) -> Option<&[Located<String>]> {
        self.values.entitlements.as_deref().map(Vec::as_slice)
    }

    /// Returns authored build-time host mappings without using service `extra_hosts` semantics.
    ///
    /// List entries retain raw `=`/`:` spelling, IPv4/IPv6 brackets, `host-gateway`, and unknown
    /// values. Mapping values retain either a scalar string or ordered string list; no address
    /// normalization, validation, DNS lookup, host inspection, or build behavior is performed.
    /// Returns the raw restart window spelling.
    #[must_use]
    pub const fn extra_hosts(&self) -> Option<&BuildExtraHosts> {
        self.values.extra_hosts.as_ref()
    }

    /// Returns explicitly authored build arguments without normalizing mapping and list syntax.
    ///
    /// Mapping entries retain their string, number, boolean, or null scalar kinds. List entries
    /// remain raw ordered strings, including duplicates and bare argument names.
    /// Returns retained restart-policy extensions.
    #[must_use]
    pub const fn args(&self) -> Option<&BuildArgs> {
        self.values.args.as_ref()
    }

    /// Returns authored external build-cache sources in order.
    ///
    /// An explicit empty sequence remains distinct from omission. Entries are raw string scalars;
    /// this model preserves duplicates and does not parse cache type, reference, source, path,
    /// image, credentials, or builder behavior.
    /// Returns retained unknown restart-policy fields.
    #[must_use]
    pub fn cache_from(&self) -> Option<&[Located<String>]> {
        self.values.cache_from.as_deref().map(Vec::as_slice)
    }

    /// Returns authored external build-cache destinations in order.
    ///
    /// An explicit empty sequence remains distinct from omission. Entries are raw string scalars;
    /// this model preserves duplicates and does not parse cache type, reference, destination,
    /// path, image, credentials, or builder behavior.
    #[must_use]
    pub fn cache_to(&self) -> Option<&[Located<String>]> {
        self.values.cache_to.as_deref().map(Vec::as_slice)
    }

    /// Returns the explicitly authored long-syntax Dockerfile when it is a non-empty scalar.
    ///
    /// Other build subfields remain source-addressable references and are not semantically
    /// interpreted by this model.
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
    /// Select build stages excluded from cache.
    NoCacheFilter,
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
    restart_policy: Option<Box<DeployRestartPolicy>>,
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
            restart_policy: None,
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

    pub(super) fn set_restart_policy(&mut self, restart_policy: DeployRestartPolicy) {
        self.restart_policy = Some(Box::new(restart_policy));
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

    /// Returns the authored deploy restart policy without using service `restart` semantics.
    #[must_use]
    pub fn restart_policy(&self) -> Option<&DeployRestartPolicy> {
        self.restart_policy.as_deref()
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
