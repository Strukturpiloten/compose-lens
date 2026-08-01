//! Built-in compatibility targets, rules, and evidence.

use super::{ImplementationVersion, VersionRange};
use crate::diagnostic::Severity;

const SPEC_URL: &str = "https://github.com/compose-spec/compose-spec/blob/main/spec.md";
const DOCKER_MERGE_URL: &str = "https://docs.docker.com/reference/compose-file/merge/";
const DOCKER_SELINUX_ISSUE_URL: &str = "https://github.com/docker/compose/issues/13396";
const PROVIDER_CONFORMANCE_URL: &str =
    "https://github.com/Strukturpiloten/compose-lens/blob/main/docs/research/provider-config-conformance-2026-07-31.md";

const SPEC_EVIDENCE: &[CompatibilityEvidence] = &[CompatibilityEvidence::new(
    EvidenceKind::Specification,
    SPEC_URL,
    "current Compose Specification syntax",
    None,
    None,
)];
const DOCKER_OVERRIDE_EVIDENCE: &[CompatibilityEvidence] = &[CompatibilityEvidence::new(
    EvidenceKind::OfficialDocumentation,
    DOCKER_MERGE_URL,
    "Docker documents !override as requiring Compose 2.24.4 or newer",
    Some(VersionRange::from_minimum(ImplementationVersion::new(2, 24, 4))),
    None,
)];
const DOCKER_RESET_EVIDENCE: &[CompatibilityEvidence] = &[CompatibilityEvidence::new(
    EvidenceKind::OfficialDocumentation,
    DOCKER_MERGE_URL,
    "current Docker documentation describes !reset but does not identify its first supported version",
    None,
    None,
)];
const DOCKER_PODMAN_SELINUX_EVIDENCE: &[CompatibilityEvidence] = &[CompatibilityEvidence::new(
    EvidenceKind::IssueReproduction,
    DOCKER_SELINUX_ISSUE_URL,
    "Docker Compose 2.40.3 with Podman 5.6.2 applied short-form relabeling but long-form relabeling was ineffective",
    Some(VersionRange::exact(ImplementationVersion::new(2, 40, 3))),
    Some(VersionRange::exact(ImplementationVersion::new(5, 6, 2))),
)];
const DOCKER_2_24_3_PROVIDER_EVIDENCE: &[CompatibilityEvidence] = &[provider_evidence(
    "reviewed feature-specific Docker Compose 2.24.3 config observations",
    ImplementationVersion::new(2, 24, 3),
)];
const DOCKER_2_24_4_PROVIDER_EVIDENCE: &[CompatibilityEvidence] = &[provider_evidence(
    "reviewed feature-specific Docker Compose 2.24.4 config observations",
    ImplementationVersion::new(2, 24, 4),
)];
const DOCKER_2_40_3_PROVIDER_EVIDENCE: &[CompatibilityEvidence] = &[provider_evidence(
    "reviewed feature-specific Docker Compose 2.40.3 config observations",
    ImplementationVersion::new(2, 40, 3),
)];
const DOCKER_5_3_1_PROVIDER_EVIDENCE: &[CompatibilityEvidence] = &[provider_evidence(
    "reviewed feature-specific Docker Compose 5.3.1 config observations",
    ImplementationVersion::new(5, 3, 1),
)];
const PODMAN_COMPOSE_1_3_0_PROVIDER_EVIDENCE: &[CompatibilityEvidence] = &[provider_evidence(
    "reviewed feature-specific podman-compose 1.3.0 config observations",
    ImplementationVersion::new(1, 3, 0),
)];
const PODMAN_COMPOSE_1_5_0_PROVIDER_EVIDENCE: &[CompatibilityEvidence] = &[provider_evidence(
    "reviewed feature-specific podman-compose 1.5.0 config observations",
    ImplementationVersion::new(1, 5, 0),
)];

/// A compatibility-sensitive Compose construct recognized by this release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompatibilityFeature {
    /// An image reference combining a tag and a digest.
    ImageTagAndDigest,
    /// `SELinux` relabeling requested through short volume syntax.
    ShortBindSelinuxRelabel,
    /// `SELinux` relabeling requested through long bind syntax.
    LongBindSelinuxRelabel,
    /// Compose's `!reset` merge tag.
    ResetTag,
    /// Compose's `!override` merge tag.
    OverrideTag,
    /// A reserved `x-` extension field.
    ExtensionField,
}

/// How a selected profile classifies one construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompatibilityClassification {
    /// The evidence supports the construct for the selected context.
    Supported,
    /// The construct uses Compose's reserved extension mechanism.
    Extension,
    /// The construct is accepted or meaningful only under implementation-specific behavior.
    ImplementationSpecific,
    /// The construct remains accepted but is deprecated in the selected context.
    Deprecated,
    /// Evidence shows that the construct is unavailable or ineffective.
    Unsupported,
    /// Available evidence is insufficient for the selected versions.
    Unknown,
}

/// The Compose parser/provider whose behavior is being assessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ComposeProvider {
    /// The current Compose Specification, without claiming runtime support.
    Specification,
    /// Docker Compose at an exact released version.
    DockerCompose(ImplementationVersion),
    /// The independent `containers/podman-compose` provider at an exact released version.
    PodmanCompose(ImplementationVersion),
    /// Preservation-oriented handling that deliberately makes no runtime claim.
    Tolerant,
}

/// The backend container runtime used by a Compose provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContainerRuntime {
    /// Docker Engine at an exact released version.
    DockerEngine(ImplementationVersion),
    /// Podman at an exact released version.
    Podman(ImplementationVersion),
}

/// A caller-selected provider and optional backend runtime.
///
/// `podman compose` is intentionally not represented as a provider: Podman documents that command
/// as a wrapper around an external provider. Callers must identify the provider it actually runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompatibilityProfile {
    provider: ComposeProvider,
    runtime: Option<ContainerRuntime>,
}

impl CompatibilityProfile {
    /// Creates the specification-oriented profile.
    #[must_use]
    pub const fn specification() -> Self {
        Self {
            provider: ComposeProvider::Specification,
            runtime: None,
        }
    }

    /// Creates a Docker Compose profile for an exact provider version.
    #[must_use]
    pub const fn docker_compose(version: ImplementationVersion) -> Self {
        Self {
            provider: ComposeProvider::DockerCompose(version),
            runtime: None,
        }
    }

    /// Creates a `containers/podman-compose` profile for an exact provider version.
    #[must_use]
    pub const fn podman_compose(version: ImplementationVersion) -> Self {
        Self {
            provider: ComposeProvider::PodmanCompose(version),
            runtime: None,
        }
    }

    /// Creates a tolerant preservation profile that makes no implementation-support claim.
    #[must_use]
    pub const fn tolerant() -> Self {
        Self {
            provider: ComposeProvider::Tolerant,
            runtime: None,
        }
    }

    /// Attaches the exact backend runtime selected by the caller.
    #[must_use]
    pub const fn with_runtime(mut self, runtime: ContainerRuntime) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Returns the selected Compose provider.
    #[must_use]
    pub const fn provider(self) -> ComposeProvider {
        self.provider
    }

    /// Returns the selected backend runtime, if supplied.
    #[must_use]
    pub const fn runtime(self) -> Option<ContainerRuntime> {
        self.runtime
    }

    /// Classifies one feature using only versioned built-in evidence.
    #[must_use]
    pub fn classify(self, feature: CompatibilityFeature) -> CompatibilityRule {
        match self.provider {
            ComposeProvider::Specification => specification_rule(feature),
            ComposeProvider::DockerCompose(version) => docker_rule(self, version, feature),
            ComposeProvider::PodmanCompose(version) => podman_compose_rule(version, feature),
            ComposeProvider::Tolerant => tolerant_rule(feature),
        }
    }
}

/// The provenance category of one compatibility claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EvidenceKind {
    /// Normative or descriptive Compose Specification text.
    Specification,
    /// Documentation published by the implementation owner.
    OfficialDocumentation,
    /// A versioned public issue containing a reproducible observation.
    IssueReproduction,
    /// A reviewed `ComposeLens` provider-only config observation.
    ProviderConformance,
    /// A ComposeLens-controlled runtime conformance result.
    RuntimeConformance,
}

/// One source supporting a compatibility rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompatibilityEvidence {
    kind: EvidenceKind,
    source: &'static str,
    summary: &'static str,
    provider_versions: Option<VersionRange>,
    runtime_versions: Option<VersionRange>,
}

impl CompatibilityEvidence {
    const fn new(
        kind: EvidenceKind,
        source: &'static str,
        summary: &'static str,
        provider_versions: Option<VersionRange>,
        runtime_versions: Option<VersionRange>,
    ) -> Self {
        Self {
            kind,
            source,
            summary,
            provider_versions,
            runtime_versions,
        }
    }

    /// Returns the evidence category.
    #[must_use]
    pub const fn kind(self) -> EvidenceKind {
        self.kind
    }

    /// Returns the authoritative or public evidence URL.
    #[must_use]
    pub const fn source(self) -> &'static str {
        self.source
    }

    /// Returns a concise claim supported by the source.
    #[must_use]
    pub const fn summary(self) -> &'static str {
        self.summary
    }

    /// Returns the provider-version scope, when established.
    #[must_use]
    pub const fn provider_versions(self) -> Option<VersionRange> {
        self.provider_versions
    }

    /// Returns the runtime-version scope, when established.
    #[must_use]
    pub const fn runtime_versions(self) -> Option<VersionRange> {
        self.runtime_versions
    }
}

/// A profile's decision for one compatibility feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityRule {
    feature: CompatibilityFeature,
    classification: CompatibilityClassification,
    diagnostic_severity: Option<Severity>,
    explanation: &'static str,
    evidence: &'static [CompatibilityEvidence],
}

impl CompatibilityRule {
    /// Returns the classified feature.
    #[must_use]
    pub const fn feature(&self) -> CompatibilityFeature {
        self.feature
    }

    /// Returns the compatibility classification.
    #[must_use]
    pub const fn classification(&self) -> CompatibilityClassification {
        self.classification
    }

    /// Returns the diagnostic severity, or `None` when no diagnostic should be emitted.
    #[must_use]
    pub const fn diagnostic_severity(&self) -> Option<Severity> {
        self.diagnostic_severity
    }

    /// Returns a value-free explanation suitable for diagnostics.
    #[must_use]
    pub const fn explanation(&self) -> &'static str {
        self.explanation
    }

    /// Returns the evidence supporting the classification.
    #[must_use]
    pub const fn evidence(&self) -> &'static [CompatibilityEvidence] {
        self.evidence
    }
}

fn specification_rule(feature: CompatibilityFeature) -> CompatibilityRule {
    match feature {
        CompatibilityFeature::ImageTagAndDigest => rule(
            feature,
            CompatibilityClassification::ImplementationSpecific,
            Some(Severity::Warning),
            "the documented image grammar selects a tag or a digest, while real implementations may accept both",
            SPEC_EVIDENCE,
        ),
        CompatibilityFeature::ExtensionField => rule(
            feature,
            CompatibilityClassification::Extension,
            None,
            "x- fields use the Compose extension namespace",
            SPEC_EVIDENCE,
        ),
        CompatibilityFeature::ShortBindSelinuxRelabel
        | CompatibilityFeature::LongBindSelinuxRelabel
        | CompatibilityFeature::ResetTag
        | CompatibilityFeature::OverrideTag => rule(
            feature,
            CompatibilityClassification::Supported,
            None,
            "the construct is defined by the current Compose Specification",
            SPEC_EVIDENCE,
        ),
    }
}

fn docker_rule(
    profile: CompatibilityProfile,
    version: ImplementationVersion,
    feature: CompatibilityFeature,
) -> CompatibilityRule {
    match feature {
        CompatibilityFeature::OverrideTag if version == ImplementationVersion::new(2, 24, 3) => rule(
            feature,
            CompatibilityClassification::Unsupported,
            Some(Severity::Error),
            "Docker Compose 2.24.3 accepted !override syntax but did not apply replacement semantics",
            DOCKER_2_24_3_PROVIDER_EVIDENCE,
        ),
        CompatibilityFeature::OverrideTag if version < ImplementationVersion::new(2, 24, 4) => rule(
            feature,
            CompatibilityClassification::Unsupported,
            Some(Severity::Error),
            "the selected Docker Compose version predates documented !override support",
            DOCKER_OVERRIDE_EVIDENCE,
        ),
        CompatibilityFeature::OverrideTag => rule(
            feature,
            CompatibilityClassification::Supported,
            None,
            "the selected Docker Compose version meets the documented !override minimum",
            DOCKER_OVERRIDE_EVIDENCE,
        ),
        CompatibilityFeature::ResetTag if !docker_provider_evidence(version).is_empty() => rule(
            feature,
            CompatibilityClassification::Supported,
            None,
            "the selected exact Docker Compose version applied !reset in reviewed provider conformance",
            docker_provider_evidence(version),
        ),
        CompatibilityFeature::ResetTag => rule(
            feature,
            CompatibilityClassification::ImplementationSpecific,
            Some(Severity::Warning),
            "Docker documents !reset, but the available evidence does not establish its first supported release",
            DOCKER_RESET_EVIDENCE,
        ),
        CompatibilityFeature::ShortBindSelinuxRelabel if is_reported_selinux_context(profile) => rule(
            feature,
            CompatibilityClassification::Supported,
            None,
            "the exact reported provider/runtime pair applied short-form SELinux relabeling",
            DOCKER_PODMAN_SELINUX_EVIDENCE,
        ),
        CompatibilityFeature::LongBindSelinuxRelabel if is_reported_selinux_context(profile) => rule(
            feature,
            CompatibilityClassification::Unsupported,
            Some(Severity::Error),
            "the exact reported provider/runtime pair accepted this form but did not relabel the host path",
            DOCKER_PODMAN_SELINUX_EVIDENCE,
        ),
        CompatibilityFeature::ShortBindSelinuxRelabel => rule(
            feature,
            CompatibilityClassification::ImplementationSpecific,
            Some(Severity::Warning),
            "SELinux relabeling depends on the backend runtime, host platform, and authored mount form",
            docker_provider_evidence(version),
        ),
        CompatibilityFeature::LongBindSelinuxRelabel => rule(
            feature,
            CompatibilityClassification::Unknown,
            Some(Severity::Warning),
            "no versioned evidence covers long-form SELinux behavior for the selected provider/runtime pair",
            docker_provider_evidence(version),
        ),
        CompatibilityFeature::ImageTagAndDigest if !docker_provider_evidence(version).is_empty() => rule(
            feature,
            CompatibilityClassification::Supported,
            None,
            "the selected exact Docker Compose version retained the combined tag and digest",
            docker_provider_evidence(version),
        ),
        CompatibilityFeature::ImageTagAndDigest => rule(
            feature,
            CompatibilityClassification::ImplementationSpecific,
            Some(Severity::Warning),
            "combined image tags and digests require implementation evidence beyond the documented Compose grammar",
            SPEC_EVIDENCE,
        ),
        CompatibilityFeature::ExtensionField => rule(
            feature,
            CompatibilityClassification::Extension,
            None,
            "x- fields use the Compose extension namespace",
            SPEC_EVIDENCE,
        ),
    }
}

fn podman_compose_rule(version: ImplementationVersion, feature: CompatibilityFeature) -> CompatibilityRule {
    if feature == CompatibilityFeature::ExtensionField {
        return rule(
            feature,
            CompatibilityClassification::Extension,
            None,
            "x- fields use the Compose extension namespace",
            SPEC_EVIDENCE,
        );
    }
    let evidence = podman_compose_provider_evidence(version);
    if !evidence.is_empty() {
        return match feature {
            CompatibilityFeature::ImageTagAndDigest => rule(
                feature,
                CompatibilityClassification::Supported,
                None,
                "the selected exact podman-compose version retained the combined tag and digest",
                evidence,
            ),
            CompatibilityFeature::ResetTag => rule(
                feature,
                CompatibilityClassification::Unsupported,
                Some(Severity::Error),
                "the selected exact podman-compose version failed while processing !reset",
                evidence,
            ),
            CompatibilityFeature::OverrideTag if version == ImplementationVersion::new(1, 3, 0) => rule(
                feature,
                CompatibilityClassification::Unsupported,
                Some(Severity::Error),
                "podman-compose 1.3.0 rejected !override",
                evidence,
            ),
            CompatibilityFeature::OverrideTag => rule(
                feature,
                CompatibilityClassification::Supported,
                None,
                "podman-compose 1.5.0 applied !override replacement semantics",
                evidence,
            ),
            CompatibilityFeature::ShortBindSelinuxRelabel | CompatibilityFeature::LongBindSelinuxRelabel => rule(
                feature,
                CompatibilityClassification::Unknown,
                Some(Severity::Warning),
                "provider config accepted the SELinux form, but no reviewed runtime-effect record establishes relabeling",
                evidence,
            ),
            CompatibilityFeature::ExtensionField => unreachable!("extension fields returned above"),
        };
    }
    rule(
        feature,
        CompatibilityClassification::Unknown,
        Some(Severity::Warning),
        "no versioned podman-compose conformance evidence covers this construct yet",
        &[],
    )
}

const fn provider_evidence(summary: &'static str, version: ImplementationVersion) -> CompatibilityEvidence {
    CompatibilityEvidence::new(
        EvidenceKind::ProviderConformance,
        PROVIDER_CONFORMANCE_URL,
        summary,
        Some(VersionRange::exact(version)),
        None,
    )
}

fn docker_provider_evidence(version: ImplementationVersion) -> &'static [CompatibilityEvidence] {
    match version {
        value if value == ImplementationVersion::new(2, 24, 3) => DOCKER_2_24_3_PROVIDER_EVIDENCE,
        value if value == ImplementationVersion::new(2, 24, 4) => DOCKER_2_24_4_PROVIDER_EVIDENCE,
        value if value == ImplementationVersion::new(2, 40, 3) => DOCKER_2_40_3_PROVIDER_EVIDENCE,
        value if value == ImplementationVersion::new(5, 3, 1) => DOCKER_5_3_1_PROVIDER_EVIDENCE,
        _ => &[],
    }
}

fn podman_compose_provider_evidence(version: ImplementationVersion) -> &'static [CompatibilityEvidence] {
    match version {
        value if value == ImplementationVersion::new(1, 3, 0) => PODMAN_COMPOSE_1_3_0_PROVIDER_EVIDENCE,
        value if value == ImplementationVersion::new(1, 5, 0) => PODMAN_COMPOSE_1_5_0_PROVIDER_EVIDENCE,
        _ => &[],
    }
}

fn tolerant_rule(feature: CompatibilityFeature) -> CompatibilityRule {
    if feature == CompatibilityFeature::ExtensionField {
        return rule(
            feature,
            CompatibilityClassification::Extension,
            None,
            "x- fields are preserved as Compose extensions",
            SPEC_EVIDENCE,
        );
    }
    rule(
        feature,
        CompatibilityClassification::Unknown,
        Some(Severity::Note),
        "tolerant preservation deliberately makes no runtime-support claim",
        &[],
    )
}

fn is_reported_selinux_context(profile: CompatibilityProfile) -> bool {
    profile.provider == ComposeProvider::DockerCompose(ImplementationVersion::new(2, 40, 3))
        && profile.runtime == Some(ContainerRuntime::Podman(ImplementationVersion::new(5, 6, 2)))
}

fn rule(
    feature: CompatibilityFeature,
    classification: CompatibilityClassification,
    diagnostic_severity: Option<Severity>,
    explanation: &'static str,
    evidence: &'static [CompatibilityEvidence],
) -> CompatibilityRule {
    CompatibilityRule {
        feature,
        classification,
        diagnostic_severity,
        explanation,
        evidence,
    }
}
