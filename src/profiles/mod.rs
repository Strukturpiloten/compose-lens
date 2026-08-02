//! Explicit, non-destructive Compose service-profile selection.

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::merge::{MergeOperation, MergedProject, MergedValue};
use crate::source::{SourceId, SourceSpan};
use std::collections::BTreeSet;

/// A requested or declared profile name does not follow the Compose grammar.
pub const INVALID_PROFILE_NAME: DiagnosticCode = DiagnosticCode::new("compose.profiles.invalid-name");

/// A service `profiles` field is not a sequence.
pub const PROFILES_EXPECTED_SEQUENCE: DiagnosticCode = DiagnosticCode::new("compose.profiles.expected-sequence");

/// A service profile entry is not a scalar string.
pub const PROFILE_EXPECTED_SCALAR: DiagnosticCode = DiagnosticCode::new("compose.profiles.expected-scalar");

/// An explicitly authored profile list is empty.
pub const EMPTY_PROFILE_LIST: DiagnosticCode = DiagnosticCode::new("compose.profiles.empty-list");

/// The explicit active-profile input for one selection operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileRequest {
    active: BTreeSet<String>,
    all: bool,
}

impl ProfileRequest {
    /// Creates a request with no active profiles.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            active: BTreeSet::new(),
            all: false,
        }
    }

    /// Creates a request that activates every valid declared profile.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            active: BTreeSet::new(),
            all: true,
        }
    }

    /// Adds one active profile and reports whether it was newly inserted.
    pub fn activate(&mut self, profile: impl Into<String>) -> bool {
        self.active.insert(profile.into())
    }

    /// Adds one active profile using builder syntax.
    #[must_use]
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        let _ = self.activate(profile);
        self
    }

    /// Returns explicitly active profiles in deterministic order.
    pub fn active(&self) -> impl Iterator<Item = &str> {
        self.active.iter().map(String::as_str)
    }

    /// Reports whether all declared profiles are active.
    #[must_use]
    pub const fn activates_all(&self) -> bool {
        self.all
    }
}

/// Why one service is active or inactive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationReason {
    /// The service has no effective `profiles` restriction.
    Unprofiled,
    /// At least one declared profile matched an explicitly active profile.
    MatchingProfile(String),
    /// The caller explicitly requested all profiles.
    AllProfiles,
    /// None of the declared profiles is active.
    NoMatchingProfile,
}

/// The selection state of one authored service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceStatus {
    /// The service participates in subsequent project processing.
    Active,
    /// The service remains in the merged project but is outside the selected view.
    Inactive,
}

/// One declared service's profile decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSelection {
    name: String,
    source: SourceSpan,
    profiles: Vec<(String, SourceSpan)>,
    status: ServiceStatus,
    reason: ActivationReason,
}

impl ServiceSelection {
    /// Returns the service name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the effective service-key source span.
    #[must_use]
    pub const fn source(&self) -> SourceSpan {
        self.source
    }

    /// Returns declared profile names and source spans in authored merge order.
    #[must_use]
    pub fn profiles(&self) -> &[(String, SourceSpan)] {
        &self.profiles
    }

    /// Returns whether the service is active.
    #[must_use]
    pub const fn status(&self) -> ServiceStatus {
        self.status
    }

    /// Returns the reason for the decision.
    #[must_use]
    pub const fn reason(&self) -> &ActivationReason {
        &self.reason
    }

    /// Reports whether the service is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self.status, ServiceStatus::Active)
    }
}

/// A non-destructive service selection over one merged project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSelection {
    project: MergedProject,
    active_profiles: BTreeSet<String>,
    all_profiles: bool,
    services: Vec<ServiceSelection>,
    diagnostics: Vec<Diagnostic>,
}

impl ProfileSelection {
    /// Returns source documents identifying the merged project used for selection.
    #[must_use]
    pub fn source_ids(&self) -> &[SourceId] {
        self.project.source_ids()
    }

    /// Returns explicitly active profiles in deterministic order.
    pub fn active_profiles(&self) -> impl Iterator<Item = &str> {
        self.active_profiles.iter().map(String::as_str)
    }

    /// Reports whether the request activated all declared profiles.
    #[must_use]
    pub const fn activates_all_profiles(&self) -> bool {
        self.all_profiles
    }

    /// Returns service decisions in merged service order.
    #[must_use]
    pub fn services(&self) -> &[ServiceSelection] {
        &self.services
    }

    /// Finds one service decision.
    #[must_use]
    pub fn service(&self, name: &str) -> Option<&ServiceSelection> {
        self.services.iter().find(|service| service.name == name)
    }

    /// Reports whether a service exists and is active.
    #[must_use]
    pub fn is_active(&self, name: &str) -> bool {
        self.service(name).is_some_and(ServiceSelection::is_active)
    }

    /// Returns selection diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether selection emitted no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }

    pub(crate) fn belongs_to(&self, project: &MergedProject) -> bool {
        &self.project == project
    }
}

/// Selects active services from a merged project using only caller-supplied profiles.
///
/// The merged project is not modified. Explicit runtime service targeting and dependency startup
/// are command concerns and are intentionally not inferred by this operation.
#[must_use]
pub fn select_profiles(project: &MergedProject, request: &ProfileRequest) -> ProfileSelection {
    let mut diagnostics = Vec::new();
    for profile in &request.active {
        if !valid_profile_name(profile) {
            diagnostics.push(Diagnostic::new(
                INVALID_PROFILE_NAME,
                Severity::Error,
                "active profile name does not follow the Compose profile grammar",
            ));
        }
    }

    let services = project
        .root()
        .get("services")
        .and_then(MergedValue::as_mapping)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let source = entry.key_sources().last().copied()?;
            let profiles_value = entry.value().get("profiles");
            let (profiles, unrestricted) = read_profiles(profiles_value, &mut diagnostics);
            let (status, reason) = if unrestricted {
                (ServiceStatus::Active, ActivationReason::Unprofiled)
            } else if request.all {
                (ServiceStatus::Active, ActivationReason::AllProfiles)
            } else if let Some(profile) = profiles
                .iter()
                .find(|(profile, _)| request.active.contains(profile) && valid_profile_name(profile))
            {
                (
                    ServiceStatus::Active,
                    ActivationReason::MatchingProfile(profile.0.clone()),
                )
            } else {
                (ServiceStatus::Inactive, ActivationReason::NoMatchingProfile)
            };
            Some(ServiceSelection {
                name: entry.key().to_owned(),
                source,
                profiles,
                status,
                reason,
            })
        })
        .collect();

    ProfileSelection {
        project: project.clone(),
        active_profiles: request.active.clone(),
        all_profiles: request.all,
        services,
        diagnostics,
    }
}

fn read_profiles(value: Option<&MergedValue>, diagnostics: &mut Vec<Diagnostic>) -> (Vec<(String, SourceSpan)>, bool) {
    let Some(value) = value else {
        return (Vec::new(), true);
    };
    let Some(values) = value.as_sequence() else {
        diagnostics.push(
            Diagnostic::new(
                PROFILES_EXPECTED_SEQUENCE,
                Severity::Error,
                "service profiles must be a sequence",
            )
            .with_label(DiagnosticLabel::primary(
                value
                    .provenance()
                    .effective_source()
                    .unwrap_or_else(|| fallback_span(value)),
                "not a profile sequence",
            )),
        );
        return (Vec::new(), false);
    };
    if values.is_empty() {
        if value.provenance().operation() == MergeOperation::Reset {
            return (Vec::new(), true);
        }
        diagnostics.push(
            Diagnostic::new(
                EMPTY_PROFILE_LIST,
                Severity::Error,
                "an explicitly authored profiles list must not be empty",
            )
            .with_label(DiagnosticLabel::primary(
                value
                    .provenance()
                    .effective_source()
                    .unwrap_or_else(|| fallback_span(value)),
                "empty profile list",
            )),
        );
        return (Vec::new(), false);
    }

    let mut profiles = Vec::new();
    for profile in values {
        let Some(scalar) = profile.as_scalar() else {
            diagnostics.push(
                Diagnostic::new(
                    PROFILE_EXPECTED_SCALAR,
                    Severity::Error,
                    "profile names must be scalar strings",
                )
                .with_label(DiagnosticLabel::primary(
                    profile
                        .provenance()
                        .effective_source()
                        .unwrap_or_else(|| fallback_span(profile)),
                    "not a profile name",
                )),
            );
            continue;
        };
        let span = profile
            .provenance()
            .effective_source()
            .unwrap_or_else(|| fallback_span(profile));
        if scalar.kind() != crate::merge::MergedScalarKind::String || !valid_profile_name(scalar.value()) {
            diagnostics.push(
                Diagnostic::new(
                    INVALID_PROFILE_NAME,
                    Severity::Error,
                    "declared profile name does not follow the Compose profile grammar",
                )
                .with_label(DiagnosticLabel::primary(span, "invalid profile name")),
            );
        }
        profiles.push((scalar.value().to_owned(), span));
    }
    (profiles, false)
}

fn valid_profile_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes.next().is_some_and(valid_profile_tail)
        && bytes.all(valid_profile_tail)
}

fn valid_profile_tail(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-')
}

fn fallback_span(value: &MergedValue) -> SourceSpan {
    value
        .provenance()
        .sources()
        .first()
        .copied()
        .unwrap_or_else(|| SourceSpan::from_valid_offsets(SourceId::new(0), 0, 0))
}
