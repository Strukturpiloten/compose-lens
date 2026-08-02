//! Public post-merge profile, path, reference, and default behavior.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::{MergedProject, merge_project};
use compose_lens::profiles::{INVALID_PROFILE_NAME, ProfileRequest, ServiceStatus, select_profiles};
use compose_lens::resolution::{
    ComposeDefaults, ContainerPlatform, DISABLED_DEPENDENCY_HEALTHCHECK, DefaultKind, DefaultLocation, DefaultValue,
    HOME_DIRECTORY_REQUIRED, HostPathKind, INACTIVE_SERVICE_REFERENCE, MISSING_REFERENCE, NoDefaults, PathContext,
    PathPurpose, ReferenceKind, ReferenceStatus, SELECTION_PROJECT_MISMATCH, UNVERIFIED_DEPENDENCY_HEALTHCHECK,
    resolve_defaults, resolve_paths, validate_references,
};
use compose_lens::source::SourceId;
use std::collections::BTreeMap;
use std::path::Path;

const PROFILE_BASE: &str = include_str!("../fixtures/processing/profile-selection/compose.yaml");
const PROFILE_OVERRIDE: &str = include_str!("../fixtures/processing/profile-selection/compose.override.yaml");
const RESOLUTION_PROJECT: &str = include_str!("../fixtures/processing/project-resolution/compose.yaml");
const ISSUE_BACKLOG: &str = include_str!("../fixtures/typed-model/post-01-issue-backlog/compose.yaml");

#[test]
fn selects_profiles_without_mutating_the_merged_project() -> Result<(), Box<dyn std::error::Error>> {
    let merged = profile_project()?;
    let default_selection = select_profiles(&merged, &ProfileRequest::new());

    assert!(default_selection.is_valid(), "{:#?}", default_selection.diagnostics());
    assert_eq!(
        service_states(&default_selection),
        BTreeMap::from([
            ("always", ServiceStatus::Active),
            ("debug", ServiceStatus::Inactive),
            ("reset", ServiceStatus::Active),
            ("tools", ServiceStatus::Inactive),
        ])
    );
    assert!(default_selection.active_profiles().next().is_none());
    assert_eq!(
        merged
            .value(&["services", "debug", "profiles"])
            .and_then(compose_lens::merge::MergedValue::as_sequence)
            .map(<[compose_lens::merge::MergedValue]>::len),
        Some(1)
    );

    let debug = select_profiles(&merged, &ProfileRequest::new().with_profile("debug"));
    assert!(
        debug
            .services()
            .iter()
            .all(compose_lens::profiles::ServiceSelection::is_active)
    );
    assert_eq!(debug.active_profiles().collect::<Vec<_>>(), vec!["debug"]);

    let all = select_profiles(&merged, &ProfileRequest::all());
    assert!(all.activates_all_profiles());
    assert!(
        all.services()
            .iter()
            .all(compose_lens::profiles::ServiceSelection::is_active)
    );
    Ok(())
}

#[test]
fn diagnoses_invalid_requested_and_declared_profile_names() -> Result<(), Box<dyn std::error::Error>> {
    let merged = one_file_project(
        "services:\n  app:\n    image: example/app\n    profiles: [x, valid-profile]\n",
        203,
    )?;
    let selection = select_profiles(&merged, &ProfileRequest::new().with_profile("x"));

    assert!(!selection.is_valid());
    assert_eq!(
        selection
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == INVALID_PROFILE_NAME)
            .count(),
        2
    );
    assert!(!selection.is_active("app"));
    Ok(())
}

#[test]
fn resolves_only_selected_host_paths_from_explicit_origins() -> Result<(), Box<dyn std::error::Error>> {
    let merged = resolution_project()?;
    let selection = select_profiles(&merged, &ProfileRequest::new());
    let context = PathContext::new().with_home_directory("/home/tester");
    let resolution = resolve_paths(&merged, Some(&selection), &context);

    assert!(resolution.is_valid(), "{:#?}", resolution.diagnostics());
    assert_eq!(resolution.paths().len(), 4);
    let relative = path_by_raw(&resolution, "./data")?;
    assert_eq!(relative.kind(), HostPathKind::Relative);
    assert_eq!(relative.origin(), Path::new("workspace/project"));
    assert_eq!(relative.resolved(), Some(Path::new("workspace/project/./data")));
    assert!(matches!(
        relative.purpose(),
        PathPurpose::ServiceBind { service, index: 0 } if service == "app"
    ));

    let home = path_by_raw(&resolution, "~/cache")?;
    assert_eq!(home.kind(), HostPathKind::HomeRelative);
    assert_eq!(home.resolved(), Some(Path::new("/home/tester/cache")));
    assert!(resolution.paths().iter().all(|path| path.raw() != "./debug"));
    assert!(matches!(
        path_by_raw(&resolution, "./config/app.conf")?.purpose(),
        PathPurpose::ConfigFile { config } if config == "app-config"
    ));
    assert!(matches!(
        path_by_raw(&resolution, "./secrets/app.secret")?.purpose(),
        PathPurpose::SecretFile { secret } if secret == "app-secret"
    ));

    let without_home = resolve_paths(&merged, Some(&selection), &PathContext::new());
    assert_eq!(path_by_raw(&without_home, "~/cache")?.resolved(), None);
    assert!(
        without_home
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == HOME_DIRECTORY_REQUIRED)
    );
    Ok(())
}

#[test]
fn validates_resource_and_selected_service_references() -> Result<(), Box<dyn std::error::Error>> {
    let merged = resolution_project()?;
    let selection = select_profiles(&merged, &ProfileRequest::new());
    let validation = validate_references(&merged, Some(&selection));

    assert!(!validation.is_valid());
    assert_reference(&validation, ReferenceKind::Network, "front", ReferenceStatus::Found);
    assert_reference(
        &validation,
        ReferenceKind::Network,
        "missing-network",
        ReferenceStatus::Missing,
    );
    assert_reference(&validation, ReferenceKind::Volume, "named-data", ReferenceStatus::Found);
    assert_reference(
        &validation,
        ReferenceKind::Volume,
        "missing-data",
        ReferenceStatus::Missing,
    );
    assert_reference(&validation, ReferenceKind::Config, "app-config", ReferenceStatus::Found);
    assert_reference(
        &validation,
        ReferenceKind::Config,
        "missing-config",
        ReferenceStatus::Missing,
    );
    assert_reference(&validation, ReferenceKind::Secret, "app-secret", ReferenceStatus::Found);
    assert_reference(&validation, ReferenceKind::Dependency, "db", ReferenceStatus::Found);
    assert_reference(
        &validation,
        ReferenceKind::Dependency,
        "debug-helper",
        ReferenceStatus::Inactive,
    );
    assert_reference(
        &validation,
        ReferenceKind::Dependency,
        "ghost",
        ReferenceStatus::Missing,
    );
    assert_reference(
        &validation,
        ReferenceKind::ServiceNamespace,
        "debug-helper",
        ReferenceStatus::Inactive,
    );
    assert_reference(&validation, ReferenceKind::Link, "db", ReferenceStatus::Found);
    assert_reference(&validation, ReferenceKind::Extends, "db", ReferenceStatus::Found);
    assert!(
        validation
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == MISSING_REFERENCE)
    );
    assert!(
        validation
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == INACTIVE_SERVICE_REFERENCE)
    );
    Ok(())
}

#[test]
fn validates_healthy_dependencies_after_merge_without_assuming_image_metadata() -> Result<(), Box<dyn std::error::Error>>
{
    let merged = one_file_project(ISSUE_BACKLOG, 215)?;
    let validation = validate_references(&merged, None);

    assert!(validation.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == UNVERIFIED_DEPENDENCY_HEALTHCHECK
            && diagnostic.severity() == compose_lens::diagnostic::Severity::Warning
    }));
    assert!(validation.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == DISABLED_DEPENDENCY_HEALTHCHECK
            && diagnostic.severity() == compose_lens::diagnostic::Severity::Error
    }));
    assert!(
        validation
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == MISSING_REFERENCE)
    );
    let optional = validation
        .references()
        .iter()
        .find(|reference| reference.target() == "optional-missing")
        .ok_or("optional missing dependency expected")?;
    assert!(!optional.is_required());
    assert!(validation.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == MISSING_REFERENCE && diagnostic.severity() == compose_lens::diagnostic::Severity::Warning
    }));
    Ok(())
}

#[test]
fn requests_documented_defaults_from_an_explicit_policy() -> Result<(), Box<dyn std::error::Error>> {
    let merged = resolution_project()?;
    let selection = select_profiles(&merged, &ProfileRequest::new());
    let none = resolve_defaults(&merged, Some(&selection), &NoDefaults);
    assert!(none.defaults().is_empty());

    let defaults = resolve_defaults(
        &merged,
        Some(&selection),
        &ComposeDefaults::new(ContainerPlatform::Linux),
    );
    assert!(defaults.is_valid(), "{:#?}", defaults.diagnostics());
    assert_eq!(defaults.defaults().len(), 14);
    assert_eq!(kind_count(&defaults, DefaultKind::ImplicitNetwork), 1);
    assert_eq!(kind_count(&defaults, DefaultKind::ServiceNetwork), 1);
    assert_eq!(kind_count(&defaults, DefaultKind::PortProtocol), 1);
    assert_eq!(kind_count(&defaults, DefaultKind::PortMode), 2);
    assert_eq!(kind_count(&defaults, DefaultKind::VolumeReadOnly), 3);
    assert_eq!(kind_count(&defaults, DefaultKind::ConfigTarget), 1);
    assert_eq!(kind_count(&defaults, DefaultKind::ConfigMode), 2);
    assert_eq!(kind_count(&defaults, DefaultKind::SecretTarget), 1);
    assert_eq!(kind_count(&defaults, DefaultKind::SecretMode), 1);
    assert_eq!(kind_count(&defaults, DefaultKind::RestartPolicy), 1);
    assert!(defaults.defaults().iter().any(|default| {
        default.request().location() == &DefaultLocation::Project
            && default.value() == &DefaultValue::String("default".to_owned())
    }));
    assert!(defaults.defaults().iter().any(|default| {
        default.request().kind() == DefaultKind::ConfigTarget
            && default.value() == &DefaultValue::String("/app-config".to_owned())
    }));
    assert!(defaults.defaults().iter().any(|default| {
        default.request().kind() == DefaultKind::SecretTarget
            && default.value() == &DefaultValue::String("app-secret".to_owned())
    }));
    Ok(())
}

#[test]
fn rejects_profile_selections_created_for_another_project() -> Result<(), Box<dyn std::error::Error>> {
    let profile = profile_project()?;
    let selection = select_profiles(&profile, &ProfileRequest::new());
    let resolution = resolution_project()?;

    for diagnostics in [
        resolve_paths(&resolution, Some(&selection), &PathContext::new())
            .diagnostics()
            .to_vec(),
        validate_references(&resolution, Some(&selection))
            .diagnostics()
            .to_vec(),
        resolve_defaults(&resolution, Some(&selection), &NoDefaults)
            .diagnostics()
            .to_vec(),
    ] {
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), SELECTION_PROJECT_MISMATCH);
    }

    let same_source_id = one_file_project("services:\n  other:\n    image: example/other\n", 211)?;
    let same_id_selection = select_profiles(&same_source_id, &ProfileRequest::new());
    let paths = resolve_paths(&resolution, Some(&same_id_selection), &PathContext::new());
    assert_eq!(paths.diagnostics()[0].code(), SELECTION_PROJECT_MISMATCH);
    Ok(())
}

#[test]
fn redacts_sensitive_processing_values_from_debug_and_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    image: example/app\n    networks: [\"${PRIVATE}\"]\n    volumes:\n      - type: bind\n        source: ${PRIVATE}\n        target: /data\n    configs: [\"${PRIVATE}\"]\n    secrets: [\"${PRIVATE}\"]\n";
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(221),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("PRIVATE", "private-processing-value");
    let interpolation = loaded.interpolate(&environment);
    let merge = merge_project(&loaded, Some(&interpolation));
    let project = merge.project().ok_or("merged project expected")?;
    let selection = select_profiles(project, &ProfileRequest::new());
    let paths = resolve_paths(project, Some(&selection), &PathContext::new());
    let references = validate_references(project, Some(&selection));
    let defaults = resolve_defaults(
        project,
        Some(&selection),
        &ComposeDefaults::new(ContainerPlatform::Linux),
    );

    assert_eq!(paths.paths()[0].raw(), "private-processing-value");
    assert!(
        references
            .references()
            .iter()
            .any(|reference| { reference.target() == "private-processing-value" && reference.is_sensitive() }),
        "{:#?}",
        references.references()
    );
    assert!(defaults.defaults().iter().any(|default| {
        default.request().source_name() == Some("private-processing-value") && default.request().is_sensitive()
    }));
    let debug = format!("{selection:?} {paths:?} {references:?} {defaults:?}");
    assert!(!debug.contains("private-processing-value"));
    for diagnostic in paths
        .diagnostics()
        .iter()
        .chain(references.diagnostics())
        .chain(defaults.diagnostics())
    {
        assert!(!diagnostic_text(diagnostic).contains("private-processing-value"));
    }
    Ok(())
}

fn profile_project() -> Result<MergedProject, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(201),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            PROFILE_BASE,
        ),
        DocumentInput::new(
            SourceId::new(202),
            DocumentOrigin::new("compose.override.yaml", "workspace/overrides"),
            PROFILE_OVERRIDE,
        ),
    ])?;
    merged(&loaded)
}

fn resolution_project() -> Result<MergedProject, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(211),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        RESOLUTION_PROJECT,
    )])?;
    merged(&loaded)
}

fn one_file_project(source: &'static str, source_id: u32) -> Result<MergedProject, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(source_id),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    merged(&loaded)
}

fn merged(project: &LoadedProject) -> Result<MergedProject, Box<dyn std::error::Error>> {
    let result = merge_project(project, None);
    result
        .project()
        .cloned()
        .ok_or_else(|| "merged project expected".into())
}

fn service_states(selection: &compose_lens::profiles::ProfileSelection) -> BTreeMap<&str, ServiceStatus> {
    selection
        .services()
        .iter()
        .map(|service| (service.name(), service.status()))
        .collect()
}

fn path_by_raw<'a>(
    resolution: &'a compose_lens::resolution::PathResolution,
    raw: &str,
) -> Result<&'a compose_lens::resolution::ResolvedHostPath, Box<dyn std::error::Error>> {
    resolution
        .paths()
        .iter()
        .find(|path| path.raw() == raw)
        .ok_or_else(|| format!("path {raw:?} expected").into())
}

fn assert_reference(
    validation: &compose_lens::resolution::ReferenceValidation,
    kind: ReferenceKind,
    target: &str,
    status: ReferenceStatus,
) {
    assert!(
        validation
            .references()
            .iter()
            .any(|reference| reference.kind() == kind && reference.target() == target && reference.status() == status),
        "missing {kind:?} reference to {target:?} with {status:?}: {:#?}",
        validation.references()
    );
}

fn kind_count(resolution: &compose_lens::resolution::DefaultResolution, kind: DefaultKind) -> usize {
    resolution
        .defaults()
        .iter()
        .filter(|default| default.request().kind() == kind)
        .count()
}

fn diagnostic_text(diagnostic: &compose_lens::diagnostic::Diagnostic) -> String {
    let mut text = diagnostic.message().to_owned();
    for label in diagnostic.labels() {
        text.push_str(label.message());
    }
    for note in diagnostic.notes() {
        text.push_str(note);
    }
    text
}
