//! Native merged-project behavior and provenance.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::{EntrySyntax, MergeOperation, merge_project};
use compose_lens::model::{Command, ComposeScalar, Port, SelinuxRelabel, ServiceNetworks, VolumeMount};
use compose_lens::profiles::{ProfileRequest, select_profiles};
use compose_lens::project::{PROJECT_EXPECTED_FORM, ProjectService, ProjectValue, ProjectView, build_project_view};
use compose_lens::resolution::SELECTION_PROJECT_MISMATCH;
use compose_lens::source::SourceId;

const BASE: &str = include_str!("../fixtures/processing/typed-project-view/compose.yaml");
const OVERRIDE: &str = include_str!("../fixtures/processing/typed-project-view/compose.override.yaml");

#[test]
fn builds_a_profile_selected_native_view_with_multifile_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = loaded_project(SourceId::new(601), SourceId::new(602))?;
    let merged = merge_project(&loaded, None);
    let project = merged.project().ok_or("merged project expected")?;
    let selection = select_profiles(project, &ProfileRequest::new());
    let result = build_project_view(project, Some(&selection));
    let view = result.view().ok_or("typed project view expected")?;

    assert!(loaded.is_valid(), "{:#?}", loaded.diagnostics());
    assert!(merged.is_valid(), "{:#?}", merged.diagnostics());
    assert!(selection.is_valid(), "{:#?}", selection.diagnostics());
    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert_eq!(view.source_ids(), &[SourceId::new(601), SourceId::new(602)]);
    assert_eq!(view.base_directory().to_string_lossy(), "workspace/project");
    assert_eq!(view.name().map(|name| name.value().as_str()), Some("project-view"));
    assert_eq!(view.services().len(), 1);
    assert!(view.service("worker").is_none());

    let web = view.service("web").ok_or("active web service expected")?;
    assert!(matches!(
        web.command().map(ProjectValue::value),
        Some(Command::List { values, .. }) if values.len() == 2
    ));
    let image = web.image().ok_or("image expected")?;
    assert_eq!(image.value().raw(), "example.invalid/web:2@sha256:abcdef");
    assert_eq!(image.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(image.provenance().sources(), &[SourceId::new(601), SourceId::new(602)]);

    let environment = web.environment().ok_or("environment expected")?;
    let shared = environment.value().get("SHARED").ok_or("SHARED expected")?;
    assert_eq!(shared.value().value(), &ComposeScalar::String("override".to_owned()));
    assert_eq!(shared.syntax(), EntrySyntax::ListKeyValue);
    assert_source_ids(
        shared.value().provenance().sources(),
        &[SourceId::new(601), SourceId::new(602)],
    );
    assert!(environment.value().get("BASE_ONLY").is_some());
    assert!(environment.value().get("OVERRIDE_ONLY").is_some());

    let ports = web.ports().ok_or("ports expected")?;
    assert_source_ids(ports.provenance().sources(), &[SourceId::new(601), SourceId::new(602)]);
    assert!(ports.value().iter().any(|port| matches!(port.value(), Port::Long(_))));

    let volumes = web.volumes().ok_or("volumes expected")?;
    let comma_mount = volumes
        .value()
        .iter()
        .find_map(|mount| match mount.value() {
            VolumeMount::Short(value) if value.raw().value().contains(",ro") => Some(value),
            _ => None,
        })
        .ok_or("comma-containing short mount expected")?;
    assert_eq!(comma_mount.options(), &["Z".to_owned(), "ro".to_owned()]);
    assert_eq!(comma_mount.selinux_relabel(), Some(SelinuxRelabel::Private));

    assert!(web.unmodeled_fields().iter().any(|field| {
        field.path() == ["services", "web", "restart"] && field.key().sources()[0].source_id() == SourceId::new(601)
    }));
    assert_networks_and_resources(view, web)?;

    let unselected = build_project_view(project, None);
    let worker = unselected
        .view()
        .and_then(|view| view.service("worker"))
        .ok_or("unselected view must retain worker")?;
    assert_eq!(
        worker
            .profiles()
            .and_then(|profiles| profiles.value().first())
            .map(|profile| profile.value().as_str()),
        Some("workers")
    );
    Ok(())
}

fn assert_networks_and_resources(view: &ProjectView, web: &ProjectService) -> Result<(), Box<dyn std::error::Error>> {
    let Some(ServiceNetworks::Long { networks, .. }) = web.networks().map(ProjectValue::value) else {
        return Err("long service networks expected".into());
    };
    assert_eq!(networks[0].name().value(), "appnet");
    assert_eq!(
        networks[0]
            .aliases()
            .iter()
            .map(|alias| alias.value().as_str())
            .collect::<Vec<_>>(),
        ["web-base", "web-override"]
    );
    assert_source_ids(
        web.networks().ok_or("networks expected")?.provenance().sources(),
        &[SourceId::new(601), SourceId::new(602)],
    );

    let data = view
        .volumes()
        .iter()
        .find(|resource| resource.name().value() == "data")
        .ok_or("data volume expected")?;
    assert_source_ids(
        data.definition().provenance().sources(),
        &[SourceId::new(601), SourceId::new(602)],
    );
    assert_eq!(
        data.definition().value().driver().map(|driver| driver.value().as_str()),
        Some("local")
    );
    assert!(data.definition().value().labels().is_some());
    assert_eq!(
        view.networks()[0]
            .definition()
            .value()
            .driver()
            .map(|driver| driver.value().as_str()),
        Some("bridge")
    );
    assert_eq!(
        view.configs()[0]
            .definition()
            .value()
            .file()
            .map(|file| file.value().as_str()),
        Some("./settings.conf")
    );
    assert_eq!(
        view.secrets()[0]
            .definition()
            .value()
            .environment()
            .map(|environment| environment.value().as_str()),
        Some("APP_PASSWORD")
    );
    Ok(())
}

#[test]
fn rejects_a_profile_selection_from_another_merged_project() -> Result<(), Box<dyn std::error::Error>> {
    let first = loaded_project(SourceId::new(611), SourceId::new(612))?;
    let second = loaded_project(SourceId::new(621), SourceId::new(622))?;
    let first_merge = merge_project(&first, None);
    let second_merge = merge_project(&second, None);
    let first_project = first_merge.project().ok_or("first project expected")?;
    let second_project = second_merge.project().ok_or("second project expected")?;
    let selection = select_profiles(first_project, &ProfileRequest::new());
    let result = build_project_view(second_project, Some(&selection));

    assert!(result.view().is_none());
    assert!(!result.is_valid());
    assert_eq!(result.diagnostics()[0].code(), SELECTION_PROJECT_MISMATCH);
    Ok(())
}

#[test]
fn malformed_native_forms_return_a_partial_view_and_stable_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    image: []\n    ports: wrong\n  broken: true\n";
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(631),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = merged.project().ok_or("project expected")?;
    let result = build_project_view(project, None);

    assert!(result.view().is_some());
    assert!(!result.is_valid());
    assert!(
        result
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    assert!(result.diagnostics().iter().all(|diagnostic| {
        diagnostic
            .labels()
            .iter()
            .all(|label| label.span().source_id() == SourceId::new(631) && label.span().end() <= source.len())
    }));
    Ok(())
}

#[test]
fn redacts_sensitive_interpolation_from_project_value_debug() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    image: example.invalid/app:${TOKEN}\n";
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(641),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("TOKEN", "private-tag");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = merged.project().ok_or("project expected")?;
    let result = build_project_view(project, None);
    let image = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::image)
        .ok_or("image expected")?;
    let debug = format!("{image:?}");

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert!(image.is_sensitive());
    assert!(!debug.contains("private-tag"));
    assert!(debug.contains("<redacted>"));
    Ok(())
}

fn loaded_project(base_id: SourceId, override_id: SourceId) -> Result<LoadedProject, Box<dyn std::error::Error>> {
    Ok(LoadedProject::load([
        DocumentInput::new(base_id, DocumentOrigin::new("compose.yaml", "workspace/project"), BASE),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace/overrides"),
            OVERRIDE,
        ),
    ])?)
}

fn assert_source_ids(sources: &[compose_lens::source::SourceSpan], expected: &[SourceId]) {
    let actual: Vec<_> = sources.iter().map(|source| source.source_id()).collect();
    assert_eq!(actual, expected);
}
