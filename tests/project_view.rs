//! Native merged-project behavior and provenance.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::{EntrySyntax, MergeOperation, merge_project};
use compose_lens::model::{
    BooleanValue, Command, ComposeScalar, DependencyCondition, HealthcheckDuration, HealthcheckRetries,
    HealthcheckTest, HealthcheckTestKind, HostAddressKind, Port, SelinuxRelabel, ServiceNetworks, VolumeMount,
};
use compose_lens::profiles::{ProfileRequest, select_profiles};
use compose_lens::project::{
    PROJECT_EXPECTED_FORM, PROJECT_INVALID_VALUE, ProjectDependsOn, ProjectService, ProjectValue, ProjectView,
    build_project_view,
};
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

    let extra_hosts = web.extra_hosts().ok_or("extra_hosts expected")?;
    assert_source_ids(
        extra_hosts.provenance().sources(),
        &[SourceId::new(601), SourceId::new(602)],
    );
    assert_eq!(extra_hosts.value().entries().len(), 3);
    let gateway = &extra_hosts.value().entries()[0];
    assert_eq!(gateway.hostname().value(), "host.docker.internal");
    assert_eq!(gateway.address().value().kind(), HostAddressKind::HostGateway);
    assert_eq!(gateway.syntax(), EntrySyntax::ListKeyValue);
    assert_source_ids(gateway.address().provenance().sources(), &[SourceId::new(601)]);
    let ipv6 = &extra_hosts.value().entries()[1];
    assert_eq!(ipv6.address().value().kind(), HostAddressKind::Ipv6 { bracketed: true });
    assert_eq!(extra_hosts.value().entries()[2].hostname().value(), "database");
    assert_source_ids(
        extra_hosts.value().entries()[2].address().provenance().sources(),
        &[SourceId::new(602)],
    );

    assert_healthcheck(web)?;

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

fn assert_healthcheck(web: &ProjectService) -> Result<(), Box<dyn std::error::Error>> {
    let healthcheck = web.healthcheck().ok_or("healthcheck expected")?;
    assert_source_ids(
        healthcheck.provenance().sources(),
        &[SourceId::new(601), SourceId::new(602)],
    );
    assert!(matches!(
        healthcheck.value().test().map(ProjectValue::value),
        Some(HealthcheckTest::List {
            kind: Some(HealthcheckTestKind::CmdShell),
            values,
            ..
        }) if values.len() == 2
    ));
    assert!(matches!(
        healthcheck.value().interval().map(ProjectValue::value),
        Some(HealthcheckDuration::Value(value)) if value == "10s"
    ));
    assert_source_ids(
        healthcheck
            .value()
            .interval()
            .ok_or("health interval expected")?
            .provenance()
            .sources(),
        &[SourceId::new(601), SourceId::new(602)],
    );
    assert!(matches!(
        healthcheck.value().retries().map(ProjectValue::value),
        Some(HealthcheckRetries::Count(value)) if value == "5"
    ));
    assert_eq!(
        healthcheck
            .value()
            .start_interval()
            .map(ProjectValue::value)
            .map(HealthcheckDuration::raw),
        Some("2s")
    );
    assert!(!healthcheck.value().is_disabled());
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
fn exposes_mapping_extra_hosts_without_losing_address_spelling() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    image: example.invalid/app:1\n",
        "    extra_hosts:\n",
        "      database: 192.0.2.10\n",
        "      ipv6: \"[::1]\"\n",
        "      host.docker.internal: host-gateway\n",
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(635),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = merged.project().ok_or("project expected")?;
    let result = build_project_view(project, None);
    let hosts = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::extra_hosts)
        .ok_or("extra_hosts expected")?;

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert_eq!(hosts.value().entries().len(), 3);
    assert!(
        hosts
            .value()
            .entries()
            .iter()
            .all(|entry| entry.syntax() == EntrySyntax::Mapping)
    );
    assert_eq!(hosts.value().entries()[1].address().value().raw(), "[::1]");
    assert!(hosts.value().entries()[2].address().value().is_host_gateway());
    assert_source_ids(
        hosts.value().entries()[2].address().provenance().sources(),
        &[SourceId::new(635)],
    );
    Ok(())
}

#[test]
fn exposes_disabled_healthcheck_and_reports_malformed_fields() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  disabled:\n",
        "    image: example.invalid/disabled:1\n",
        "    healthcheck:\n",
        "      test: [NONE]\n",
        "  malformed:\n",
        "    image: example.invalid/malformed:1\n",
        "    healthcheck:\n",
        "      test: {command: true}\n",
        "      retries: []\n",
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(637),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = merged.project().ok_or("project expected")?;
    let result = build_project_view(project, None);
    let disabled = result
        .view()
        .and_then(|view| view.service("disabled"))
        .and_then(ProjectService::healthcheck)
        .ok_or("disabled healthcheck expected")?;

    assert!(disabled.value().is_disabled());
    assert!(!result.is_valid());
    assert_eq!(result.diagnostics().len(), 2);
    assert!(
        result
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

#[test]
fn exposes_merged_long_dependencies_with_nested_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let base = concat!(
        "services:\n",
        "  app:\n",
        "    image: example.invalid/app:1\n",
        "    depends_on:\n",
        "      database:\n",
        "        condition: service_started\n",
        "        required: true\n",
        "  database:\n",
        "    image: example.invalid/database:1\n",
        "  cache:\n",
        "    image: example.invalid/cache:1\n",
    );
    let override_source = concat!(
        "services:\n",
        "  app:\n",
        "    depends_on:\n",
        "      database:\n",
        "        condition: service_healthy\n",
        "        restart: true\n",
        "        x-note: retained\n",
        "      cache:\n",
        "        required: false\n",
    );
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(638),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(639),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            override_source,
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let dependencies = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::depends_on)
        .ok_or("depends_on expected")?;

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert_source_ids(
        dependencies.provenance().sources(),
        &[SourceId::new(638), SourceId::new(639)],
    );
    let ProjectDependsOn::Long(services) = dependencies.value() else {
        return Err("long dependency form expected".into());
    };
    assert_eq!(services.len(), 2);
    let database = &services[0];
    assert_eq!(database.value().service().value(), "database");
    assert_source_ids(
        database.value().service().sources(),
        &[SourceId::new(638), SourceId::new(639)],
    );
    assert!(matches!(
        database.value().condition().map(ProjectValue::value),
        Some(DependencyCondition::ServiceHealthy)
    ));
    assert_source_ids(
        database
            .value()
            .condition()
            .ok_or("condition expected")?
            .provenance()
            .sources(),
        &[SourceId::new(638), SourceId::new(639)],
    );
    assert!(matches!(
        database.value().restart().map(ProjectValue::value),
        Some(BooleanValue::Literal(true))
    ));
    assert!(matches!(
        database.value().required().map(ProjectValue::value),
        Some(BooleanValue::Literal(true))
    ));
    assert_eq!(
        database.value().unmodeled_fields()[0].path(),
        ["services", "app", "depends_on", "database", "x-note"]
    );
    assert_eq!(services[1].value().service().value(), "cache");
    assert!(matches!(
        services[1].value().required().map(ProjectValue::value),
        Some(BooleanValue::Literal(false))
    ));
    assert!(
        result
            .view()
            .and_then(|view| view.service("app"))
            .is_some_and(|service| !service
                .unmodeled_fields()
                .iter()
                .any(|field| field.path().last().is_some_and(|name| name == "depends_on")))
    );
    Ok(())
}

#[test]
fn retains_short_dependencies_and_reports_invalid_long_options() -> Result<(), Box<dyn std::error::Error>> {
    let short_source = concat!(
        "services:\n",
        "  app:\n",
        "    image: example.invalid/app:1\n",
        "    depends_on: [database, cache]\n",
        "  database:\n",
        "    image: example.invalid/database:1\n",
        "  cache:\n",
        "    image: example.invalid/cache:1\n",
    );
    let short_loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(645),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        short_source,
    )])?;
    let short_merge = merge_project(&short_loaded, None);
    let short_result = build_project_view(short_merge.project().ok_or("short project expected")?, None);
    let short = short_result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::depends_on)
        .ok_or("short dependencies expected")?;
    let ProjectDependsOn::Short(services) = short.value() else {
        return Err("short dependency form expected".into());
    };
    assert!(short_result.is_valid(), "{:#?}", short_result.diagnostics());
    assert_eq!(
        services
            .iter()
            .map(|dependency| dependency.value().service().value())
            .collect::<Vec<_>>(),
        ["database", "cache"]
    );
    assert!(services.iter().all(|dependency| {
        dependency.value().condition().is_none()
            && dependency.value().restart().is_none()
            && dependency.value().required().is_none()
    }));

    let malformed_source = concat!(
        "services:\n",
        "  app:\n",
        "    image: example.invalid/app:1\n",
        "    depends_on:\n",
        "      invalid-options: service_started\n",
        "      invalid-condition:\n",
        "        condition: provider_ready\n",
        "      invalid-condition-form:\n",
        "        condition: []\n",
        "      invalid-required:\n",
        "        required: yes\n",
    );
    let malformed_loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(646),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        malformed_source,
    )])?;
    let malformed_merge = merge_project(&malformed_loaded, None);
    let malformed_result = build_project_view(malformed_merge.project().ok_or("malformed project expected")?, None);

    assert!(!malformed_result.is_valid());
    assert_eq!(
        malformed_result
            .diagnostics()
            .iter()
            .map(compose_lens::diagnostic::Diagnostic::code)
            .collect::<Vec<_>>(),
        [
            PROJECT_EXPECTED_FORM,
            PROJECT_INVALID_VALUE,
            PROJECT_EXPECTED_FORM,
            PROJECT_INVALID_VALUE
        ]
    );
    let malformed_dependencies = malformed_result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::depends_on)
        .ok_or("partial dependency view expected")?;
    assert_eq!(malformed_dependencies.value().services().len(), 3);
    assert!(matches!(
        malformed_dependencies.value().services()[0]
            .value()
            .condition()
            .map(ProjectValue::value),
        Some(DependencyCondition::Other(value)) if value == "provider_ready"
    ));
    assert!(
        malformed_dependencies.value().services()[1]
            .value()
            .condition()
            .is_none()
    );
    Ok(())
}

#[test]
fn redacts_sensitive_interpolation_from_project_value_debug() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    image: example.invalid/app:${TOKEN}\n",
        "    depends_on: [\"${DEPENDENCY}\"]\n",
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(641),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("TOKEN", "private-tag");
    let _ = environment.insert_sensitive("DEPENDENCY", "private-service");
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
    let dependency = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::depends_on)
        .and_then(|dependencies| dependencies.value().services().first())
        .ok_or("dependency expected")?;
    let dependency_name = dependency.value().service();
    assert!(dependency_name.is_sensitive());
    assert!(!format!("{dependency_name:?}").contains("private-service"));
    assert!(format!("{dependency_name:?}").contains("<redacted>"));
    Ok(())
}

#[test]
fn redacts_sensitive_semantic_keys_across_keyed_merges() -> Result<(), Box<dyn std::error::Error>> {
    let base = concat!(
        "services:\n",
        "  app:\n",
        "    image: example.invalid/app:1\n",
        "    environment:\n",
        "      - \"${SECRET_NAME}=base\"\n",
    );
    let override_source = concat!(
        "services:\n",
        "  app:\n",
        "    environment:\n",
        "      private-name: override\n",
    );
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(647),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(648),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            override_source,
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("SECRET_NAME", "private-name");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let name = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::environment)
        .and_then(|environment| environment.value().get("private-name"))
        .map(compose_lens::project::ProjectEnvironmentEntry::name)
        .ok_or("merged environment key expected")?;

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert!(name.is_sensitive());
    assert!(!format!("{name:?}").contains("private-name"));
    assert!(!format!("{merged:?}").contains("private-name"));
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
