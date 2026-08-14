use super::paths::is_path_source;
use super::{entry_span, selection_matches, service_entries, service_in_scope};
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::merge::{MergedEntry, MergedProject, MergedScalar, MergedValue};
use crate::model::{Located, ShortVolumeMount};
use crate::profiles::ProfileSelection;
use crate::source::SourceSpan;
use std::collections::BTreeSet;
use std::fmt;

/// A selected service references an undefined service or top-level resource.
pub const MISSING_REFERENCE: DiagnosticCode = DiagnosticCode::new("compose.references.missing");

/// A selected service references a service excluded by the active profiles.
pub const INACTIVE_SERVICE_REFERENCE: DiagnosticCode = DiagnosticCode::new("compose.references.inactive-service");

/// A healthy dependency has no Compose health check and requires image/runtime verification.
pub const UNVERIFIED_DEPENDENCY_HEALTHCHECK: DiagnosticCode =
    DiagnosticCode::new("compose.references.healthcheck-unverified");

/// A healthy dependency explicitly disables its health check.
pub const DISABLED_DEPENDENCY_HEALTHCHECK: DiagnosticCode =
    DiagnosticCode::new("compose.references.healthcheck-disabled");

/// The semantic kind of a Compose cross-reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceKind {
    /// A service network attachment.
    Network,
    /// A named service volume mount.
    Volume,
    /// A service config grant.
    Config,
    /// A service secret grant.
    Secret,
    /// A `depends_on` service edge.
    Dependency,
    /// A `service:name` namespace edge from `network_mode`, `ipc`, or `pid`.
    ServiceNamespace,
    /// A legacy `links` service edge.
    Link,
    /// A local `extends.service` edge without an external file.
    Extends,
}

/// Whether the referenced project object is available to the selected view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceStatus {
    /// The target exists and participates in the selected view.
    Found,
    /// No target with that name is declared.
    Missing,
    /// The target service exists but its profiles are inactive.
    Inactive,
}

/// One source-aware Compose cross-reference.
#[derive(Clone, PartialEq, Eq)]
pub struct Reference {
    source_service: String,
    target: String,
    source: SourceSpan,
    kind: ReferenceKind,
    status: ReferenceStatus,
    sensitive: bool,
    required: bool,
}

impl Reference {
    /// Returns the service containing the reference.
    #[must_use]
    pub fn source_service(&self) -> &str {
        &self.source_service
    }

    /// Returns the referenced model key.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Returns the reference source span.
    #[must_use]
    pub const fn source(&self) -> SourceSpan {
        self.source
    }

    /// Returns the reference kind.
    #[must_use]
    pub const fn kind(&self) -> ReferenceKind {
        self.kind
    }

    /// Returns whether the target is available.
    #[must_use]
    pub const fn status(&self) -> ReferenceStatus {
        self.status
    }

    /// Reports whether interpolation inserted sensitive content into the target name.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }

    /// Reports whether an unavailable dependency is an error rather than a warning.
    ///
    /// Non-dependency reference kinds are always required.
    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.required
    }
}

impl fmt::Debug for Reference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Reference")
            .field("source_service", &self.source_service)
            .field("target", &if self.sensitive { "<redacted>" } else { &self.target })
            .field("source", &self.source)
            .field("kind", &self.kind)
            .field("status", &self.status)
            .field("sensitive", &self.sensitive)
            .field("required", &self.required)
            .finish()
    }
}

/// Cross-reference validation for one selected project view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceValidation {
    references: Vec<Reference>,
    diagnostics: Vec<Diagnostic>,
}

impl ReferenceValidation {
    /// Returns discovered references in deterministic traversal order.
    #[must_use]
    pub fn references(&self) -> &[Reference] {
        &self.references
    }

    /// Returns missing and inactive-reference diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether every discovered reference is available.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }
}

/// Validates top-level resource and service references for active services.
#[must_use]
pub fn validate_references(project: &MergedProject, selection: Option<&ProfileSelection>) -> ReferenceValidation {
    let mut diagnostics = Vec::new();
    if !selection_matches(project, selection, &mut diagnostics) {
        return ReferenceValidation {
            references: Vec::new(),
            diagnostics,
        };
    }

    let services = service_entries(project);
    let service_names: BTreeSet<_> = services.iter().map(MergedEntry::key).collect();
    let networks = resource_names(project, "networks");
    let volumes = resource_names(project, "volumes");
    let configs = resource_names(project, "configs");
    let secrets = resource_names(project, "secrets");
    let models = resource_names(project, "models");
    let mut references = Vec::new();

    for service in services {
        if !service_in_scope(selection, service.key()) {
            continue;
        }
        collect_networks(service, &networks, &mut references, &mut diagnostics);
        collect_volumes(service, &volumes, &mut references, &mut diagnostics);
        collect_grants(
            service,
            "configs",
            ReferenceKind::Config,
            &configs,
            &mut references,
            &mut diagnostics,
        );
        collect_grants(
            service,
            "secrets",
            ReferenceKind::Secret,
            &secrets,
            &mut references,
            &mut diagnostics,
        );
        collect_service_references(service, &service_names, selection, &mut references, &mut diagnostics);
        validate_service_models(service, &models, &mut diagnostics);
        validate_dependency_healthchecks(service, services, selection, &mut diagnostics);
    }

    ReferenceValidation {
        references,
        diagnostics,
    }
}

fn validate_service_models(service: &MergedEntry, models: &BTreeSet<&str>, diagnostics: &mut Vec<Diagnostic>) {
    let Some(value) = service.value().get("models") else {
        return;
    };
    match value.kind() {
        crate::merge::MergedValueKind::Sequence(items) => {
            for item in items {
                if let Some(scalar) = item.as_scalar() {
                    diagnose_missing_model(
                        service.key(),
                        scalar.value(),
                        super::effective_span(item),
                        models,
                        diagnostics,
                    );
                }
            }
        }
        crate::merge::MergedValueKind::Mapping(entries) => {
            for entry in entries {
                diagnose_missing_model(service.key(), entry.key(), entry_span(entry), models, diagnostics);
            }
        }
        _ => {}
    }
}

fn diagnose_missing_model(
    _service: &str,
    model: &str,
    source: SourceSpan,
    models: &BTreeSet<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if models.contains(model) {
        return;
    }
    diagnostics.push(
        Diagnostic::new(
            MISSING_REFERENCE,
            Severity::Error,
            "selected service model binding references an undefined model",
        )
        .with_label(DiagnosticLabel::primary(source, "model is not declared")),
    );
}

fn resource_names<'a>(project: &'a MergedProject, field: &str) -> BTreeSet<&'a str> {
    project
        .root()
        .get(field)
        .and_then(MergedValue::as_mapping)
        .into_iter()
        .flatten()
        .map(MergedEntry::key)
        .collect()
}

fn collect_networks(
    service: &MergedEntry,
    definitions: &BTreeSet<&str>,
    references: &mut Vec<Reference>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(networks) = service.value().get("networks") else {
        return;
    };
    if let Some(values) = networks.as_sequence() {
        for value in values {
            if let Some(scalar) = value.as_scalar() {
                push_resource(
                    service.key(),
                    scalar.value(),
                    super::effective_span(value),
                    ReferenceKind::Network,
                    definitions.contains(scalar.value()) || scalar.value() == "default",
                    scalar.is_sensitive(),
                    references,
                    diagnostics,
                );
            }
        }
    } else if let Some(entries) = networks.as_mapping() {
        for network in entries {
            push_reference(
                service.key(),
                network.key(),
                entry_span(network),
                ReferenceKind::Network,
                if definitions.contains(network.key()) || network.key() == "default" {
                    ReferenceStatus::Found
                } else {
                    ReferenceStatus::Missing
                },
                false,
                true,
                references,
                diagnostics,
            );
        }
    }
}

fn collect_volumes(
    service: &MergedEntry,
    definitions: &BTreeSet<&str>,
    references: &mut Vec<Reference>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(values) = service.value().get("volumes").and_then(MergedValue::as_sequence) else {
        return;
    };
    for value in values {
        if let Some(scalar) = value.as_scalar() {
            let span = super::effective_span(value);
            let mount = ShortVolumeMount::new(Located::new(scalar.value().to_owned(), span));
            if let Some(source) = mount.source().filter(|source| !is_path_source(source)) {
                push_resource(
                    service.key(),
                    source,
                    span,
                    ReferenceKind::Volume,
                    definitions.contains(source),
                    scalar.is_sensitive(),
                    references,
                    diagnostics,
                );
            }
            continue;
        }
        let mount_type = value
            .get("type")
            .and_then(MergedValue::as_scalar)
            .map_or("volume", MergedScalar::value);
        if mount_type != "volume" {
            continue;
        }
        if let Some(source) = value.get("source") {
            if let Some(scalar) = source.as_scalar() {
                push_resource(
                    service.key(),
                    scalar.value(),
                    super::effective_span(source),
                    ReferenceKind::Volume,
                    definitions.contains(scalar.value()),
                    scalar.is_sensitive(),
                    references,
                    diagnostics,
                );
            }
        }
    }
}

fn collect_grants(
    service: &MergedEntry,
    field: &str,
    kind: ReferenceKind,
    definitions: &BTreeSet<&str>,
    references: &mut Vec<Reference>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(values) = service.value().get(field).and_then(MergedValue::as_sequence) else {
        return;
    };
    for value in values {
        let source = value
            .as_scalar()
            .map(|scalar| (scalar, super::effective_span(value)))
            .or_else(|| {
                let source = value.get("source")?;
                Some((source.as_scalar()?, super::effective_span(source)))
            });
        if let Some((scalar, span)) = source {
            push_resource(
                service.key(),
                scalar.value(),
                span,
                kind,
                definitions.contains(scalar.value()),
                scalar.is_sensitive(),
                references,
                diagnostics,
            );
        }
    }
}

fn collect_service_references(
    service: &MergedEntry,
    service_names: &BTreeSet<&str>,
    selection: Option<&ProfileSelection>,
    references: &mut Vec<Reference>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(depends_on) = service.value().get("depends_on") {
        collect_service_collection(
            service,
            depends_on,
            ReferenceKind::Dependency,
            service_names,
            selection,
            references,
            diagnostics,
        );
    }
    for field in ["network_mode", "ipc", "pid"] {
        let Some(value) = service.value().get(field) else {
            continue;
        };
        let Some(scalar) = value.as_scalar() else {
            continue;
        };
        if let Some(target) = scalar.value().strip_prefix("service:") {
            push_service(
                service.key(),
                target,
                super::effective_span(value),
                ReferenceKind::ServiceNamespace,
                scalar.is_sensitive(),
                true,
                service_names,
                selection,
                references,
                diagnostics,
            );
        }
    }
    if let Some(values) = service.value().get("volumes_from").and_then(MergedValue::as_sequence) {
        for value in values {
            let Some(scalar) = value.as_scalar() else {
                continue;
            };
            let source = scalar.value().rsplit_once(':').map_or(scalar.value(), |(name, mode)| {
                if matches!(mode, "ro" | "rw") {
                    name
                } else {
                    scalar.value()
                }
            });
            let source = source.strip_prefix("service:").unwrap_or(source);
            if !source.starts_with("container:") {
                push_service(
                    service.key(),
                    source,
                    super::effective_span(value),
                    ReferenceKind::ServiceNamespace,
                    scalar.is_sensitive(),
                    true,
                    service_names,
                    selection,
                    references,
                    diagnostics,
                );
            }
        }
    }
    collect_service_links(service, service_names, selection, references, diagnostics);
    if let Some(extends) = service.value().get("extends") {
        if extends.get("file").is_none() {
            if let Some(target) = extends.get("service") {
                if let Some(scalar) = target.as_scalar() {
                    push_service(
                        service.key(),
                        scalar.value(),
                        super::effective_span(target),
                        ReferenceKind::Extends,
                        scalar.is_sensitive(),
                        true,
                        service_names,
                        selection,
                        references,
                        diagnostics,
                    );
                }
            }
        }
    }
}

fn collect_service_links(
    service: &MergedEntry,
    service_names: &BTreeSet<&str>,
    selection: Option<&ProfileSelection>,
    references: &mut Vec<Reference>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(links) = service.value().get("links").and_then(MergedValue::as_sequence) else {
        return;
    };
    for link in links {
        let Some(scalar) = link.as_scalar() else {
            continue;
        };
        let target = scalar
            .value()
            .split_once(':')
            .map_or(scalar.value(), |(target, _)| target);
        push_service(
            service.key(),
            target,
            super::effective_span(link),
            ReferenceKind::Link,
            scalar.is_sensitive(),
            true,
            service_names,
            selection,
            references,
            diagnostics,
        );
    }
}

fn validate_dependency_healthchecks(
    service: &MergedEntry,
    services: &[MergedEntry],
    selection: Option<&ProfileSelection>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(dependencies) = service.value().get("depends_on").and_then(MergedValue::as_mapping) else {
        return;
    };
    for dependency in dependencies {
        let Some(condition) = dependency.value().get("condition") else {
            continue;
        };
        if condition.as_scalar().map(MergedScalar::value) != Some("service_healthy") {
            continue;
        }
        let Some(target) = services.iter().find(|candidate| candidate.key() == dependency.key()) else {
            continue;
        };
        if !service_in_scope(selection, target.key()) {
            continue;
        }
        let span = super::effective_span(condition);
        let required = dependency
            .value()
            .get("required")
            .and_then(MergedValue::as_scalar)
            .is_none_or(|value| value.value() != "false");
        match target.value().get("healthcheck") {
            None => diagnostics.push(
                Diagnostic::new(
                    UNVERIFIED_DEPENDENCY_HEALTHCHECK,
                    Severity::Warning,
                    "service_healthy dependency has no Compose healthcheck to validate",
                )
                .with_label(DiagnosticLabel::primary(span, "image health metadata is not available"))
                .with_note("the dependency image may still define a health check; verify it at build or runtime"),
            ),
            Some(healthcheck) if healthcheck_is_disabled(healthcheck) => diagnostics.push(
                Diagnostic::new(
                    DISABLED_DEPENDENCY_HEALTHCHECK,
                    if required { Severity::Error } else { Severity::Warning },
                    if required {
                        "service_healthy dependency explicitly disables its health check"
                    } else {
                        "optional service_healthy dependency explicitly disables its health check"
                    },
                )
                .with_label(DiagnosticLabel::primary(span, "dependency cannot become healthy")),
            ),
            Some(_) => {}
        }
    }
}

fn healthcheck_is_disabled(healthcheck: &MergedValue) -> bool {
    if healthcheck
        .get("disable")
        .and_then(MergedValue::as_scalar)
        .is_some_and(|value| value.value() == "true")
    {
        return true;
    }
    healthcheck
        .get("test")
        .and_then(MergedValue::as_sequence)
        .and_then(|values| values.first())
        .and_then(MergedValue::as_scalar)
        .is_some_and(|value| value.value() == "NONE")
}

#[allow(clippy::too_many_arguments)]
fn collect_service_collection(
    service: &MergedEntry,
    value: &MergedValue,
    kind: ReferenceKind,
    service_names: &BTreeSet<&str>,
    selection: Option<&ProfileSelection>,
    references: &mut Vec<Reference>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(values) = value.as_sequence() {
        for dependency in values {
            if let Some(scalar) = dependency.as_scalar() {
                push_service(
                    service.key(),
                    scalar.value(),
                    super::effective_span(dependency),
                    kind,
                    scalar.is_sensitive(),
                    true,
                    service_names,
                    selection,
                    references,
                    diagnostics,
                );
            }
        }
    } else if let Some(entries) = value.as_mapping() {
        for dependency in entries {
            let required = dependency
                .value()
                .get("required")
                .and_then(MergedValue::as_scalar)
                .is_none_or(|value| value.value() != "false");
            push_service(
                service.key(),
                dependency.key(),
                entry_span(dependency),
                kind,
                false,
                required,
                service_names,
                selection,
                references,
                diagnostics,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_resource(
    source_service: &str,
    target: &str,
    source: SourceSpan,
    kind: ReferenceKind,
    found: bool,
    sensitive: bool,
    references: &mut Vec<Reference>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    push_reference(
        source_service,
        target,
        source,
        kind,
        if found {
            ReferenceStatus::Found
        } else {
            ReferenceStatus::Missing
        },
        sensitive,
        true,
        references,
        diagnostics,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_service(
    source_service: &str,
    target: &str,
    source: SourceSpan,
    kind: ReferenceKind,
    sensitive: bool,
    required: bool,
    service_names: &BTreeSet<&str>,
    selection: Option<&ProfileSelection>,
    references: &mut Vec<Reference>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let status = if !service_names.contains(target) {
        ReferenceStatus::Missing
    } else if selection.is_some_and(|selection| !selection.is_active(target)) {
        ReferenceStatus::Inactive
    } else {
        ReferenceStatus::Found
    };
    push_reference(
        source_service,
        target,
        source,
        kind,
        status,
        sensitive,
        required,
        references,
        diagnostics,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_reference(
    source_service: &str,
    target: &str,
    source: SourceSpan,
    kind: ReferenceKind,
    status: ReferenceStatus,
    sensitive: bool,
    required: bool,
    references: &mut Vec<Reference>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if status != ReferenceStatus::Found {
        let (code, message, label) = if status == ReferenceStatus::Missing {
            (
                MISSING_REFERENCE,
                "selected service has an undefined reference",
                "target is not declared",
            )
        } else {
            (
                INACTIVE_SERVICE_REFERENCE,
                "selected service references a profile-disabled service",
                "target service is inactive",
            )
        };
        diagnostics.push(
            Diagnostic::new(
                code,
                if required { Severity::Error } else { Severity::Warning },
                if required {
                    message
                } else {
                    "optional service dependency is unavailable"
                },
            )
            .with_label(DiagnosticLabel::primary(source, label)),
        );
    }
    references.push(Reference {
        source_service: source_service.to_owned(),
        target: target.to_owned(),
        source,
        kind,
        status,
        sensitive,
        required,
    });
}
