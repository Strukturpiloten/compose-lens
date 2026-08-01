//! Field-level build and deploy section models.

use super::{FieldReference, Located};
use crate::source::SourceSpan;

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
    fields: Vec<BuildField>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl BuildDefinition {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            fields: Vec::new(),
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn push_field(&mut self, field: BuildField) {
        self.fields.push(field);
    }

    pub(super) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(super) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the complete mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
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
        &self.unknown_fields
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
    fields: Vec<DeployField>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl DeployDefinition {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            fields: Vec::new(),
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn push_field(&mut self, field: DeployField) {
        self.fields.push(field);
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
