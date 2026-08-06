//! Native merged-project behavior and provenance.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::{EntrySyntax, MergeOperation, MergedScalarKind, merge_project};
use compose_lens::model::{
    BooleanValue, Command, ComposeScalar, DependencyCondition, Entrypoint, EnvironmentFileFormatKind,
    HealthcheckDuration, HealthcheckRetries, HealthcheckTest, HealthcheckTestKind, HostAddressKind, HostnameKind,
    IdentityComponent, LimitValue, Port, RestartPolicyKind, SYSCTLS_DUPLICATE_ITEM, SelinuxRelabel, ServiceNetworks,
    StopGracePeriod, ULIMIT_INVALID_NAME, ULIMIT_INVALID_VALUE, ULIMIT_MISSING_RANGE_MEMBER, UserNamespaceModeKind,
    VolumeMount,
};
use compose_lens::profiles::{ProfileRequest, select_profiles};
use compose_lens::project::{
    PROJECT_EXPECTED_FORM, PROJECT_INVALID_VALUE, PROJECT_MISSING_FIELD, ProjectDependsOn, ProjectDevice,
    ProjectEnvironmentFile, ProjectGrant, ProjectService, ProjectSysctls, ProjectTmpfs, ProjectUlimitValue,
    ProjectValue, ProjectView, build_project_view,
};
use compose_lens::resolution::SELECTION_PROJECT_MISMATCH;
use compose_lens::source::SourceId;

const BASE: &str = include_str!("../fixtures/processing/typed-project-view/compose.yaml");
const OVERRIDE: &str = include_str!("../fixtures/processing/typed-project-view/compose.override.yaml");

#[test]
fn exposes_merged_ulimits_with_nested_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let result = merged_ulimits_project_view()?;
    let view = result.view().ok_or("partial project view expected")?;

    assert_merged_ulimits(view)?;
    assert_reset_override_and_mismatch_ulimits(view)?;
    let malformed = view.service("malformed").ok_or("malformed service retained")?;
    assert!(malformed.image().is_some());
    let malformed_limits = malformed.ulimits().ok_or("partial malformed limits expected")?;
    assert_eq!(
        malformed_limits
            .value()
            .entries()
            .iter()
            .map(|entry| entry.value().name().value())
            .collect::<Vec<_>>(),
        ["valid", "missing", "other"]
    );
    assert!(matches!(
        malformed_limits.value().entries()[1].value().value(),
        ProjectUlimitValue::Range(range) if range.hard().is_none()
    ));
    for code in [ULIMIT_INVALID_NAME, ULIMIT_MISSING_RANGE_MEMBER, ULIMIT_INVALID_VALUE] {
        assert!(result.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

fn merged_ulimits_project_view() -> Result<compose_lens::project::ProjectViewResult, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(681),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n",
                "  merged:\n",
                "    ulimits:\n",
                "      nofile:\n",
                "        soft: \"${SOFT}\"\n",
                "        hard: 1024\n",
                "      nproc: 512\n",
                "      core:\n",
                "        soft: 1\n",
                "        hard: 2\n",
                "  reset:\n",
                "    ulimits: {nofile: 1}\n",
                "  overridden:\n",
                "    ulimits: {nofile: 1}\n",
                "  mismatch:\n",
                "    ulimits: {nproc: 1}\n",
                "  malformed:\n",
                "    image: example.invalid/app:1\n",
                "    ulimits:\n",
                "      valid: 9\n",
                "      Bad: 10\n",
                "      missing: {soft: 1}\n",
                "      wrong: [1]\n",
                "      boolean: true\n",
                "      other: host\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(682),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n",
                "  merged:\n",
                "    ulimits:\n",
                "      nofile:\n",
                "        hard: -1\n",
                "      nproc: \"2048\"\n",
                "      core: -1\n",
                "  reset:\n",
                "    ulimits: !reset {}\n",
                "  overridden:\n",
                "    ulimits: !override {nproc: \"3\"}\n",
                "  mismatch:\n",
                "    ulimits:\n",
                "      nproc: {soft: 2, hard: 3}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("SOFT", "4096");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    Ok(build_project_view(
        merged.project().ok_or("merged project expected")?,
        None,
    ))
}

fn assert_merged_ulimits(view: &ProjectView) -> Result<(), Box<dyn std::error::Error>> {
    let limits = view
        .service("merged")
        .and_then(ProjectService::ulimits)
        .ok_or("merged ulimits expected")?;
    assert_eq!(limits.provenance().operation(), MergeOperation::Merged);
    assert_eq!(
        limits
            .value()
            .entries()
            .iter()
            .map(|entry| entry.value().name().value())
            .collect::<Vec<_>>(),
        ["nofile", "nproc", "core"]
    );
    let nofile = limits.value().entries().first().ok_or("nofile expected")?;
    assert_eq!(nofile.provenance().operation(), MergeOperation::Merged);
    let ProjectUlimitValue::Range(range) = nofile.value().value() else {
        return Err("nofile range expected".into());
    };
    let soft = range.soft().ok_or("soft value expected")?;
    assert_eq!(soft.value().authored(), "\"${SOFT}\"");
    assert_eq!(soft.value().value(), &LimitValue::Number("4096".to_owned()));
    assert_eq!(soft.value().kind(), MergedScalarKind::String);
    assert!(soft.is_sensitive());
    assert_eq!(
        range.hard().map(|value| value.provenance().operation()),
        Some(MergeOperation::Replaced)
    );
    let core = &limits.value().entries()[2];
    assert_eq!(core.provenance().operation(), MergeOperation::Replaced);
    assert!(matches!(core.value().value(), ProjectUlimitValue::Single(_)));
    assert!(!format!("{limits:?}").contains("4096"));
    Ok(())
}

fn assert_reset_override_and_mismatch_ulimits(view: &ProjectView) -> Result<(), Box<dyn std::error::Error>> {
    let reset = view
        .service("reset")
        .and_then(ProjectService::ulimits)
        .ok_or("reset expected")?;
    assert!(reset.value().is_empty());
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    let overridden = view
        .service("overridden")
        .and_then(ProjectService::ulimits)
        .ok_or("override expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(overridden.value().entries()[0].value().name().value(), "nproc");
    let mismatch = view
        .service("mismatch")
        .and_then(ProjectService::ulimits)
        .ok_or("mismatch expected")?;
    assert_eq!(
        mismatch.value().entries()[0].provenance().operation(),
        MergeOperation::Replaced
    );
    assert!(matches!(
        mismatch.value().entries()[0].value().value(),
        ProjectUlimitValue::Range(_)
    ));
    Ok(())
}

#[test]
fn exposes_merged_sysctls_forms_provenance_sensitivity_and_duplicate_recovery() -> Result<(), Box<dyn std::error::Error>>
{
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(611),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n",
                "  mapping:\n",
                "    sysctls:\n",
                "      base.only: base\n",
                "      shared: old\n",
                "      sensitive: \"${SECRET}\"\n",
                "      literal.${KEY}: value\n",
                "  list:\n",
                "    sysctls: [same=value, base=value]\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(612),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n",
                "  mapping:\n",
                "    sysctls:\n",
                "      shared: false\n",
                "      added: null\n",
                "  list:\n",
                "    sysctls: [same=value, later=value]\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("SECRET", "project-secret");
    let _ = environment.insert("KEY", "not-a-key-substitution");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = merged.project().ok_or("merged project expected")?;
    let result = build_project_view(project, None);
    let view = result.view().ok_or("partial project view expected")?;

    let mapping = view
        .service("mapping")
        .and_then(ProjectService::sysctls)
        .ok_or("mapping sysctls expected")?;
    let ProjectSysctls::Map(entries) = mapping.value() else {
        return Err("mapping sysctls form expected".into());
    };
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.value().name().value())
            .collect::<Vec<_>>(),
        ["base.only", "shared", "sensitive", "literal.${KEY}", "added"]
    );
    let shared = entries
        .iter()
        .find(|entry| entry.value().name().value() == "shared")
        .ok_or("shared sysctl expected")?;
    assert_eq!(shared.value().value().value(), &ComposeScalar::Boolean(false));
    assert_source_ids(
        shared.value().value().provenance().sources(),
        &[SourceId::new(611), SourceId::new(612)],
    );
    let sensitive = entries
        .iter()
        .find(|entry| entry.value().name().value() == "sensitive")
        .ok_or("sensitive sysctl expected")?;
    assert_eq!(
        sensitive.value().value().value(),
        &ComposeScalar::String("project-secret".to_owned())
    );
    assert!(sensitive.is_sensitive());
    assert!(sensitive.value().value().is_sensitive());
    assert!(mapping.is_sensitive());
    assert!(!format!("{result:?}").contains("project-secret"));
    assert_project_sysctls_list(view, &result)?;
    Ok(())
}

fn assert_project_sysctls_list(
    view: &ProjectView,
    result: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let list = view
        .service("list")
        .and_then(ProjectService::sysctls)
        .ok_or("list sysctls expected")?;
    let ProjectSysctls::List(items) = list.value() else {
        return Err("list sysctls form expected".into());
    };
    assert_eq!(
        items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>(),
        ["same=value", "base=value", "same=value", "later=value"]
    );
    assert_source_ids(list.provenance().sources(), &[SourceId::new(611), SourceId::new(612)]);
    assert_eq!(
        items[0]
            .effective_source()
            .map(compose_lens::source::SourceSpan::source_id),
        Some(SourceId::new(611))
    );
    assert_eq!(
        items[2]
            .effective_source()
            .map(compose_lens::source::SourceSpan::source_id),
        Some(SourceId::new(612))
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SYSCTLS_DUPLICATE_ITEM)
            .count(),
        1
    );
    Ok(())
}

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
    let container_name = web.container_name().ok_or("container_name expected")?;
    assert_eq!(container_name.value(), "project-view-web-override");
    assert_eq!(container_name.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(
        container_name.provenance().sources(),
        &[SourceId::new(601), SourceId::new(602)],
    );
    assert!(matches!(
        web.command().map(ProjectValue::value),
        Some(Command::List { values, .. }) if values.len() == 2
    ));
    assert_entrypoint(web)?;
    let image = web.image().ok_or("image expected")?;
    assert_eq!(image.value().raw(), "example.invalid/web:2@sha256:abcdef");
    assert_eq!(image.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(image.provenance().sources(), &[SourceId::new(601), SourceId::new(602)]);

    assert_execution_identity(web)?;

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

    assert_labels(web)?;

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

    let restart = web.restart().ok_or("restart policy expected")?;
    assert!(matches!(restart.value().kind(), RestartPolicyKind::UnlessStopped));
    assert_source_ids(restart.provenance().sources(), &[SourceId::new(601)]);
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
    assert_worker_init(worker)?;
    Ok(())
}

fn assert_worker_init(worker: &ProjectService) -> Result<(), &'static str> {
    let init = worker.init().ok_or("worker init value expected")?;
    assert_eq!(init.value(), &BooleanValue::Literal(false));
    assert_eq!(init.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(init.provenance().sources(), &[SourceId::new(601), SourceId::new(602)]);
    Ok(())
}

fn assert_entrypoint(web: &ProjectService) -> Result<(), &'static str> {
    let entrypoint = web.entrypoint().ok_or("entrypoint expected")?;
    assert!(matches!(
        entrypoint.value(),
        Entrypoint::List { values, .. }
            if values.iter().map(compose_lens::model::Located::value).map(String::as_str)
                .eq(["/usr/local/bin/php", "-d", "variables_order=EGPCS"])
    ));
    assert_eq!(entrypoint.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(
        entrypoint.provenance().sources(),
        &[SourceId::new(601), SourceId::new(602)],
    );
    Ok(())
}

#[test]
fn retains_effective_restart_policy_and_complete_replacement_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let base = "services:\n  app:\n    image: example.invalid/app:1\n    restart: always\n";
    let override_source = "services:\n  app:\n    restart: on-failure:003\n";
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(651),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(652),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            override_source,
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let restart = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::restart)
        .ok_or("effective restart policy expected")?;

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert_eq!(restart.value().raw().value(), "on-failure:003");
    assert!(matches!(
        restart.value().kind(),
        RestartPolicyKind::OnFailure { maximum_retries: Some(value) } if value == "003"
    ));
    assert_eq!(restart.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(
        restart.provenance().sources(),
        &[SourceId::new(651), SourceId::new(652)],
    );
    Ok(())
}

#[test]
fn reports_an_invalid_effective_restart_policy_without_dropping_it() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    image: example.invalid/app:1\n    restart: sometimes\n";
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(653),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let restart = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::restart)
        .ok_or("retained invalid restart policy expected")?;

    assert!(!result.is_valid());
    assert!(matches!(restart.value().kind(), RestartPolicyKind::Other));
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_INVALID_VALUE)
    );
    Ok(())
}

#[test]
fn retains_omitted_and_deferred_init_without_coercion_and_recovers_malformed_shapes()
-> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  omitted: {}\n",
        "  deferred:\n",
        "    init: ${USE_INIT:-false}\n",
        "  malformed:\n",
        "    image: example.invalid/malformed:1\n",
        "    init: [true]\n",
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(657),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let view = result.view().ok_or("partial project view expected")?;

    assert!(
        view.service("omitted")
            .ok_or("omitted service expected")?
            .init()
            .is_none()
    );
    let deferred = view
        .service("deferred")
        .and_then(ProjectService::init)
        .ok_or("deferred init expected")?;
    assert_eq!(
        deferred.value(),
        &BooleanValue::Expression("${USE_INIT:-false}".to_owned())
    );
    assert_eq!(deferred.provenance().operation(), MergeOperation::Authored);
    assert_source_ids(deferred.provenance().sources(), &[SourceId::new(657)]);
    assert!(!deferred.is_sensitive());
    let malformed = view.service("malformed").ok_or("malformed service expected")?;
    assert!(malformed.init().is_none());
    assert!(malformed.image().is_some());
    assert!(!result.is_valid());
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

#[test]
fn retains_effective_lifecycle_values_replacement_provenance_and_sensitivity() -> Result<(), Box<dyn std::error::Error>>
{
    let base = concat!(
        "services:\n",
        "  app:\n",
        "    stop_signal: SIGTERM\n",
        "    stop_grace_period: 1s\n",
        "  empty:\n",
        "    stop_signal: \"\"\n",
    );
    let override_source = concat!(
        "services:\n",
        "  app:\n",
        "    stop_signal: ${STOP_SIGNAL}\n",
        "    stop_grace_period: ${STOP_GRACE_PERIOD}\n",
    );
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(654),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(655),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            override_source,
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("STOP_SIGNAL", "15");
    let _ = environment.insert_sensitive("STOP_GRACE_PERIOD", "1m30s");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let service = result
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("app service expected")?;
    let signal = service.stop_signal().ok_or("effective stop signal expected")?;
    let period = service
        .stop_grace_period()
        .ok_or("effective stop grace period expected")?;

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert_eq!(signal.value(), "15");
    assert!(signal.is_sensitive());
    assert_eq!(signal.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(signal.provenance().sources(), &[SourceId::new(654), SourceId::new(655)]);
    assert!(matches!(period.value(), StopGracePeriod::Value(value) if value == "1m30s"));
    assert!(period.is_sensitive());
    assert_eq!(period.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(period.provenance().sources(), &[SourceId::new(654), SourceId::new(655)]);
    assert!(!format!("{signal:?}").contains("15"));
    assert!(!format!("{period:?}").contains("1m30s"));
    let empty_signal = result
        .view()
        .and_then(|view| view.service("empty"))
        .and_then(ProjectService::stop_signal)
        .ok_or("quoted empty stop signal expected")?;
    assert_eq!(empty_signal.value(), "");
    assert!(!empty_signal.is_sensitive());
    assert_eq!(empty_signal.provenance().operation(), MergeOperation::Authored);
    assert_source_ids(empty_signal.provenance().sources(), &[SourceId::new(654)]);
    Ok(())
}

#[test]
fn retains_effective_hostname_replacement_provenance_and_interpolation_sensitivity()
-> Result<(), Box<dyn std::error::Error>> {
    let base = concat!(
        "services:\n",
        "  app:\n",
        "    hostname: base.example\n",
        "  omitted: {}\n",
    );
    let override_source = concat!("services:\n", "  app:\n", "    hostname: ${SERVICE_HOSTNAME}\n",);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(664),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(665),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            override_source,
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("SERVICE_HOSTNAME", "3API.Example-Corp.COM");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let view = result.view().ok_or("project view expected")?;
    let hostname = view
        .service("app")
        .and_then(ProjectService::hostname)
        .ok_or("effective hostname expected")?;

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert_eq!(hostname.value().raw().value(), "3API.Example-Corp.COM");
    assert_eq!(hostname.value().kind(), &HostnameKind::Resolved);
    assert!(hostname.is_sensitive());
    assert_eq!(hostname.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(
        hostname.provenance().sources(),
        &[SourceId::new(664), SourceId::new(665)],
    );
    assert!(!format!("{hostname:?}").contains("3API.Example-Corp.COM"));
    assert!(
        view.service("omitted")
            .ok_or("omitted service expected")?
            .hostname()
            .is_none()
    );
    Ok(())
}

#[test]
fn retains_deferred_and_invalid_effective_hostnames_and_recovers_wrong_shapes() -> Result<(), Box<dyn std::error::Error>>
{
    let source = concat!(
        "services:\n",
        "  deferred:\n",
        "    hostname: literal$marker\n",
        "  invalid:\n",
        "    hostname: invalid_host\n",
        "  boolean:\n",
        "    hostname: true\n",
        "  numeric:\n",
        "    hostname: 123\n",
        "  null:\n",
        "    hostname: null\n",
        "  list:\n",
        "    hostname: [api.example]\n",
        "  map:\n",
        "    hostname: { value: api.example }\n",
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(666),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let view = result.view().ok_or("partial project view expected")?;

    assert_eq!(
        view.service("deferred")
            .and_then(ProjectService::hostname)
            .map(ProjectValue::value)
            .map(compose_lens::model::Hostname::kind),
        Some(&HostnameKind::Expression)
    );
    assert_eq!(
        view.service("invalid")
            .and_then(ProjectService::hostname)
            .map(ProjectValue::value)
            .map(compose_lens::model::Hostname::kind),
        Some(&HostnameKind::Invalid)
    );
    for service in ["boolean", "numeric", "null", "list", "map"] {
        assert!(
            view.service(service)
                .ok_or("malformed service expected")?
                .hostname()
                .is_none()
        );
    }
    assert_eq!(view.services().len(), 7);
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == PROJECT_INVALID_VALUE)
            .count(),
        1
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
            .count(),
        5
    );
    Ok(())
}

#[test]
fn retains_unresolved_and_invalid_effective_stop_grace_periods() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  omitted: {}\n",
        "  deferred:\n",
        "    stop_grace_period: ${STOP_GRACE_PERIOD:-1s}\n",
        "  nanoseconds:\n",
        "    stop_grace_period: 1ns\n",
        "  unicode-microseconds:\n",
        "    stop_grace_period: 1µs\n",
        "  greek-microseconds:\n",
        "    stop_grace_period: 1μs\n",
        "  malformed:\n",
        "    stop_grace_period: 1.s\n",
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(656),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let view = result.view().ok_or("partial project view expected")?;

    assert!(
        view.service("omitted")
            .ok_or("omitted service expected")?
            .stop_grace_period()
            .is_none()
    );
    assert!(matches!(
        view.service("deferred")
            .and_then(ProjectService::stop_grace_period)
            .map(ProjectValue::value),
        Some(StopGracePeriod::Expression(value)) if value == "${STOP_GRACE_PERIOD:-1s}"
    ));
    for (service, expected) in [
        ("nanoseconds", "1ns"),
        ("unicode-microseconds", "1µs"),
        ("greek-microseconds", "1μs"),
        ("malformed", "1.s"),
    ] {
        assert!(matches!(
            view.service(service)
                .and_then(ProjectService::stop_grace_period)
                .map(ProjectValue::value),
            Some(StopGracePeriod::Other(value)) if value == expected
        ));
    }
    assert!(!result.is_valid());
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == PROJECT_INVALID_VALUE)
            .count(),
        4
    );
    Ok(())
}

fn assert_labels(web: &ProjectService) -> Result<(), Box<dyn std::error::Error>> {
    let labels = web.labels().ok_or("labels expected")?;
    assert_source_ids(labels.provenance().sources(), &[SourceId::new(601), SourceId::new(602)]);
    let shared_label = labels
        .value()
        .get("com.example.shared")
        .ok_or("shared label expected")?;
    assert_eq!(
        shared_label.value().value(),
        &ComposeScalar::String("override".to_owned())
    );
    assert_eq!(shared_label.syntax(), EntrySyntax::Mapping);
    assert_source_ids(
        shared_label.value().provenance().sources(),
        &[SourceId::new(601), SourceId::new(602)],
    );
    let empty_label = labels
        .value()
        .get("com.example.empty")
        .ok_or("key-only label expected")?;
    assert_eq!(empty_label.value().value(), &ComposeScalar::String(String::new()));
    assert_eq!(empty_label.syntax(), EntrySyntax::ListKeyOnly);
    assert!(labels.value().get("com.example.base").is_some());
    assert!(labels.value().get("com.example.override").is_some());
    Ok(())
}

fn assert_execution_identity(web: &ProjectService) -> Result<(), Box<dyn std::error::Error>> {
    let user = web.user().ok_or("user expected")?;
    assert_eq!(user.value().raw().value(), "1001:1002");
    assert!(matches!(user.value().user(), IdentityComponent::Numeric(value) if value == "1001"));
    assert!(matches!(user.value().group(), Some(IdentityComponent::Numeric(value)) if value == "1002"));
    assert_source_ids(user.provenance().sources(), &[SourceId::new(601), SourceId::new(602)]);

    let userns = web.userns_mode().ok_or("user namespace mode expected")?;
    assert_eq!(userns.value().kind(), UserNamespaceModeKind::PodmanKeepId);
    assert_source_ids(userns.provenance().sources(), &[SourceId::new(601), SourceId::new(602)]);

    let groups = web.group_add().ok_or("supplementary groups expected")?;
    assert_eq!(
        groups
            .value()
            .iter()
            .map(|group| group.value().as_str())
            .collect::<Vec<_>>(),
        ["audio", "video"]
    );
    assert_source_ids(groups.provenance().sources(), &[SourceId::new(601), SourceId::new(602)]);
    assert_source_ids(groups.value()[0].provenance().sources(), &[SourceId::new(601)]);
    assert_source_ids(groups.value()[1].provenance().sources(), &[SourceId::new(602)]);

    let working_dir = web.working_dir().ok_or("working directory expected")?;
    assert_eq!(working_dir.value(), "/srv/app");
    assert_source_ids(
        working_dir.provenance().sources(),
        &[SourceId::new(601), SourceId::new(602)],
    );

    let read_only = web.read_only().ok_or("read-only value expected")?;
    assert_eq!(read_only.value(), &BooleanValue::Literal(true));
    assert_source_ids(
        read_only.provenance().sources(),
        &[SourceId::new(601), SourceId::new(602)],
    );

    let init = web.init().ok_or("init value expected")?;
    assert_eq!(init.value(), &BooleanValue::Literal(true));
    assert_eq!(init.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(init.provenance().sources(), &[SourceId::new(601), SourceId::new(602)]);

    for field in ["user", "userns_mode", "group_add", "working_dir", "read_only", "init"] {
        assert!(
            !web.unmodeled_fields()
                .iter()
                .any(|reference| reference.path().last().is_some_and(|name| name == field)),
            "{field} must be native in the project view"
        );
    }
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
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    image: []\n",
        "    entrypoint: {invalid: mapping}\n",
        "    group_add: wrong\n",
        "    working_dir: []\n",
        "    read_only: sometimes\n",
        "    init: sometimes\n",
        "    ports: wrong\n",
        "  broken: true\n",
    );
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
            .all(|diagnostic| matches!(diagnostic.code(), PROJECT_EXPECTED_FORM | PROJECT_INVALID_VALUE))
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
fn exposes_merged_config_and_secret_grants_with_nested_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let base = concat!(
        "services:\n",
        "  app:\n",
        "    image: example.invalid/app:1\n",
        "    configs:\n",
        "      - source: base-config\n",
        "        target: /etc/app.conf\n",
        "        mode: \"0444\"\n",
        "    secrets:\n",
        "      - app-secret\n",
    );
    let override_source = concat!(
        "services:\n",
        "  app:\n",
        "    configs:\n",
        "      - source: override-config\n",
        "        target: /etc/app.conf\n",
        "        uid: \"1000\"\n",
        "        x-owner: retained\n",
        "    secrets:\n",
        "      - source: replacement-secret\n",
        "        target: app-secret\n",
        "        gid: \"2000\"\n",
        "        mode: \"0440\"\n",
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
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let service = result
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("app service expected")?;

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert_merged_config_grant(service)?;
    assert_merged_secret_grant(service)?;
    assert!(
        service
            .unmodeled_fields()
            .iter()
            .all(|field| { !matches!(field.path().last().map(String::as_str), Some("configs" | "secrets")) })
    );
    Ok(())
}

fn assert_merged_config_grant(service: &ProjectService) -> Result<(), Box<dyn std::error::Error>> {
    let configs = service.configs().ok_or("configs expected")?;
    assert_source_ids(
        configs.provenance().sources(),
        &[SourceId::new(647), SourceId::new(648)],
    );
    assert_eq!(configs.value().len(), 1);
    assert_source_ids(
        configs.value()[0].provenance().sources(),
        &[SourceId::new(647), SourceId::new(648)],
    );
    let ProjectGrant::Long(config) = configs.value()[0].value() else {
        return Err("long config grant expected".into());
    };
    assert_eq!(
        config.source().map(ProjectValue::value).map(String::as_str),
        Some("override-config")
    );
    assert_eq!(
        config.target().map(ProjectValue::value).map(String::as_str),
        Some("/etc/app.conf")
    );
    assert_eq!(config.uid().map(ProjectValue::value).map(String::as_str), Some("1000"));
    assert_eq!(config.mode().map(ProjectValue::value).map(String::as_str), Some("0444"));
    assert_source_ids(
        config
            .mode()
            .ok_or("retained config mode expected")?
            .provenance()
            .sources(),
        &[SourceId::new(647)],
    );
    assert_source_ids(
        config.source().ok_or("config source expected")?.provenance().sources(),
        &[SourceId::new(647), SourceId::new(648)],
    );
    assert_eq!(
        config.unmodeled_fields()[0].path(),
        ["services", "app", "configs", "0", "x-owner"]
    );
    Ok(())
}

fn assert_merged_secret_grant(service: &ProjectService) -> Result<(), Box<dyn std::error::Error>> {
    let secrets = service.secrets().ok_or("secrets expected")?;
    assert_eq!(secrets.value().len(), 1);
    assert_source_ids(
        secrets.value()[0].provenance().sources(),
        &[SourceId::new(647), SourceId::new(648)],
    );
    let ProjectGrant::Long(secret) = secrets.value()[0].value() else {
        return Err("long secret grant expected".into());
    };
    assert_eq!(
        secret.source().map(ProjectValue::value).map(String::as_str),
        Some("replacement-secret")
    );
    assert_eq!(
        secret.target().map(ProjectValue::value).map(String::as_str),
        Some("app-secret")
    );
    assert_eq!(secret.gid().map(ProjectValue::value).map(String::as_str), Some("2000"));
    assert_eq!(secret.mode().map(ProjectValue::value).map(String::as_str), Some("0440"));
    Ok(())
}

#[test]
fn reports_malformed_service_config_and_secret_grants_without_erasing_other_items()
-> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    image: example.invalid/app:1\n",
        "    configs:\n",
        "      - valid-config\n",
        "      - target: /etc/missing-source.conf\n",
        "      - [invalid]\n",
        "    secrets:\n",
        "      - source: valid-secret\n",
        "        uid: []\n",
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(649),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let service = result
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("app service expected")?;

    assert!(!result.is_valid());
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .map(compose_lens::diagnostic::Diagnostic::code)
            .collect::<Vec<_>>(),
        [PROJECT_MISSING_FIELD, PROJECT_EXPECTED_FORM, PROJECT_EXPECTED_FORM]
    );
    assert_eq!(service.configs().ok_or("partial configs expected")?.value().len(), 2);
    assert_eq!(service.secrets().ok_or("partial secrets expected")?.value().len(), 1);
    Ok(())
}

#[test]
fn redacts_sensitive_interpolation_from_project_value_debug() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    image: example.invalid/app:${TOKEN}\n",
        "    labels: [\"com.example.${LABEL_NAME}=${LABEL_VALUE}\"]\n",
        "    depends_on: [\"${DEPENDENCY}\"]\n",
        "    secrets: [\"${SECRET_GRANT}\"]\n",
        "    env_file: [\"${ENV_FILE_PATH}\"]\n",
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(641),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("TOKEN", "private-tag");
    let _ = environment.insert_sensitive("LABEL_NAME", "private-name");
    let _ = environment.insert_sensitive("LABEL_VALUE", "private-label");
    let _ = environment.insert_sensitive("DEPENDENCY", "private-service");
    let _ = environment.insert_sensitive("SECRET_GRANT", "private-secret");
    let _ = environment.insert_sensitive("ENV_FILE_PATH", "private-environment-file.env");
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
    let label = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::labels)
        .and_then(|labels| labels.value().entries().first())
        .ok_or("label expected")?;
    assert!(label.name().is_sensitive());
    assert!(label.value().is_sensitive());
    let label_debug = format!("{label:?}");
    assert!(!label_debug.contains("private-name"));
    assert!(!label_debug.contains("private-label"));
    assert!(label_debug.contains("<redacted>"));
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
    let secrets = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::secrets)
        .ok_or("secret grants expected")?;
    assert!(secrets.is_sensitive());
    assert!(!format!("{secrets:?}").contains("private-secret"));
    assert!(format!("{secrets:?}").contains("<redacted>"));
    let environment_file = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::environment_files)
        .and_then(|files| files.value().first())
        .ok_or("environment file expected")?;
    assert!(environment_file.is_sensitive());
    assert!(!format!("{environment_file:?}").contains("private-environment-file.env"));
    assert!(format!("{environment_file:?}").contains("<redacted>"));
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

#[test]
fn exposes_environment_files_in_effective_append_order_with_nested_provenance() -> Result<(), Box<dyn std::error::Error>>
{
    let base = concat!(
        "services:\n",
        "  app:\n",
        "    env_file:\n",
        "      - base.env\n",
        "      - path: optional.env\n",
        "        required: false\n",
    );
    let override_source = concat!(
        "services:\n",
        "  app:\n",
        "    env_file:\n",
        "      - override.env\n",
        "      - path: raw.env\n",
        "        format: raw\n",
    );
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(649),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(650),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            override_source,
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let environment_files = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::environment_files)
        .ok_or("environment files expected")?;

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert_eq!(environment_files.provenance().operation(), MergeOperation::Appended);
    assert_source_ids(
        environment_files.provenance().sources(),
        &[SourceId::new(649), SourceId::new(650)],
    );
    assert_eq!(environment_files.value().len(), 4);
    assert!(matches!(
        environment_files.value()[0].value(),
        ProjectEnvironmentFile::Short(path) if path == "base.env"
    ));
    assert_source_ids(
        environment_files.value()[0].provenance().sources(),
        &[SourceId::new(649)],
    );
    let ProjectEnvironmentFile::Long(optional) = environment_files.value()[1].value() else {
        return Err("optional long environment file expected".into());
    };
    assert_eq!(
        optional.path().map(ProjectValue::value).map(String::as_str),
        Some("optional.env")
    );
    assert_eq!(
        optional.required().map(ProjectValue::value),
        Some(&BooleanValue::Literal(false))
    );
    let ProjectEnvironmentFile::Long(raw) = environment_files.value()[3].value() else {
        return Err("raw long environment file expected".into());
    };
    assert_eq!(
        raw.format()
            .map(ProjectValue::value)
            .map(compose_lens::model::EnvironmentFileFormat::kind),
        Some(EnvironmentFileFormatKind::Raw)
    );
    assert_source_ids(
        raw.path().ok_or("raw path expected")?.provenance().sources(),
        &[SourceId::new(650)],
    );
    Ok(())
}

#[test]
fn reports_malformed_environment_files_without_erasing_valid_entries() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    env_file:\n",
        "      - good.env\n",
        "      - required: true\n",
        "      - path: invalid-required.env\n",
        "        required: []\n",
        "      - path: invalid-format.env\n",
        "        format: dotenv\n",
        "      - [invalid]\n",
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(651),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let environment_files = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::environment_files)
        .ok_or("partial environment files expected")?;

    assert!(!result.is_valid());
    assert_eq!(environment_files.value().len(), 4);
    for code in [PROJECT_MISSING_FIELD, PROJECT_EXPECTED_FORM, PROJECT_INVALID_VALUE] {
        assert!(result.diagnostics().iter().any(|diagnostic| {
            diagnostic.code() == code
                && diagnostic
                    .labels()
                    .iter()
                    .all(|label| label.span().source_id() == SourceId::new(651))
        }));
    }
    let ProjectEnvironmentFile::Long(invalid_format) = environment_files.value()[3].value() else {
        return Err("retained invalid-format entry expected".into());
    };
    assert_eq!(
        invalid_format
            .format()
            .map(ProjectValue::value)
            .map(compose_lens::model::EnvironmentFileFormat::kind),
        Some(EnvironmentFileFormatKind::Other)
    );
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

#[test]
fn retains_effective_pull_policy_replacement_provenance_sensitivity_and_refresh_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::PullPolicyKind;

    let base = concat!(
        "services:\n",
        "  app:\n",
        "    pull_policy: missing\n",
        "    pull_refresh_after: 24h\n",
    );
    let override_source = concat!(
        "services:\n",
        "  app:\n",
        "    pull_policy: ${PULL_POLICY}\n",
        "    pull_refresh_after: 12h\n",
    );
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(660),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(661),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            override_source,
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("PULL_POLICY", "every_12h");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let service = result
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("app service expected")?;
    let policy = service.pull_policy().ok_or("effective pull policy expected")?;

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert_eq!(policy.value().raw().value(), "every_12h");
    assert_eq!(
        policy.value().kind(),
        &PullPolicyKind::Every {
            duration: "12h".to_owned(),
        }
    );
    assert!(policy.is_sensitive());
    assert_eq!(policy.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(policy.provenance().sources(), &[SourceId::new(660), SourceId::new(661)]);
    assert!(!format!("{policy:?}").contains("every_12h"));
    let refresh = service
        .unmodeled_fields()
        .iter()
        .find(|field| field.path().ends_with(&["pull_refresh_after".to_owned()]))
        .ok_or("pull_refresh_after evidence expected")?;
    assert_eq!(refresh.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(
        refresh.provenance().sources(),
        &[SourceId::new(660), SourceId::new(661)],
    );
    Ok(())
}

#[test]
fn retains_effective_pids_limit_provenance_sensitivity_and_malformed_services() -> Result<(), Box<dyn std::error::Error>>
{
    use compose_lens::model::{PIDS_LIMIT_AMBIGUOUS_ZERO, PidsLimitKind};

    let base = concat!("services:\n", "  app:\n", "    pids_limit: 64\n",);
    let override_source = concat!(
        "services:\n",
        "  app:\n",
        "    pids_limit: ${PIDS_LIMIT}\n",
        "  zero:\n",
        "    pids_limit: 000\n",
        "  fraction:\n",
        "    pids_limit: 1.5\n",
        "  boolean:\n",
        "    pids_limit: true\n",
        "  null:\n",
        "    pids_limit: null\n",
        "  list:\n",
        "    pids_limit: [64]\n",
        "  map:\n",
        "    pids_limit: { value: 64 }\n",
    );
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(662),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(663),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            override_source,
        ),
    ])?;
    let arbitrary_precision = "18446744073709551616000000000000000000000000000000";
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("PIDS_LIMIT", arbitrary_precision);
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let view = result.view().ok_or("project view expected")?;
    let limit = view
        .service("app")
        .and_then(ProjectService::pids_limit)
        .ok_or("effective PID limit expected")?;

    assert_eq!(limit.value().raw().value(), arbitrary_precision);
    assert_eq!(
        limit.value().kind(),
        &PidsLimitKind::Finite {
            decimal: arbitrary_precision.to_owned(),
        }
    );
    assert!(limit.is_sensitive());
    assert_eq!(limit.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(limit.provenance().sources(), &[SourceId::new(662), SourceId::new(663)]);
    assert!(!format!("{limit:?}").contains(arbitrary_precision));
    assert!(matches!(
        view.service("zero")
            .and_then(ProjectService::pids_limit)
            .map(ProjectValue::value)
            .map(compose_lens::model::PidsLimit::kind),
        Some(PidsLimitKind::Zero)
    ));
    assert!(matches!(
        view.service("fraction")
            .and_then(ProjectService::pids_limit)
            .map(ProjectValue::value)
            .map(compose_lens::model::PidsLimit::kind),
        Some(PidsLimitKind::Other)
    ));
    for service in ["boolean", "null", "list", "map"] {
        assert!(
            view.service(service)
                .ok_or("malformed service expected")?
                .pids_limit()
                .is_none()
        );
    }
    assert_eq!(view.services().len(), 7);
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == PIDS_LIMIT_AMBIGUOUS_ZERO)
            .count(),
        1
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_INVALID_VALUE)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
            .count()
            >= 4
    );
    Ok(())
}

#[test]
fn retains_effective_shm_size_scalar_kind_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>>
{
    use compose_lens::model::{ShmSizeKind, ShmSizeScalarKind, ShmSizeUnit};

    let base = concat!("services:\n", "  app:\n", "    shm_size: 64m\n");
    let override_source = concat!(
        "services:\n",
        "  app:\n",
        "    shm_size: ${SHM_SIZE}\n",
        "  zero:\n",
        "    shm_size: 00m\n",
        "  number:\n",
        "    shm_size: 64\n",
        "  string:\n",
        "    shm_size: \"64\"\n",
        "  boolean:\n",
        "    shm_size: true\n",
        "  null:\n",
        "    shm_size: null\n",
        "  list:\n",
        "    shm_size: [64m]\n",
        "  map:\n",
        "    shm_size: { value: 64m }\n",
    );
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(664),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(665),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            override_source,
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("SHM_SIZE", "128mb");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let view = result.view().ok_or("project view expected")?;
    let size = view
        .service("app")
        .and_then(ProjectService::shm_size)
        .ok_or("effective shared-memory size expected")?;

    assert_eq!(size.value().raw().value(), "128mb");
    assert_eq!(size.value().scalar_kind(), ShmSizeScalarKind::String);
    assert!(matches!(
        size.value().kind(),
        ShmSizeKind::Documented { amount_raw, unit: ShmSizeUnit::Mb } if amount_raw == "128"
    ));
    assert!(size.is_sensitive());
    assert_eq!(size.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(size.provenance().sources(), &[SourceId::new(664), SourceId::new(665)]);
    assert!(!format!("{size:?}").contains("128mb"));
    assert!(matches!(
        view.service("zero")
            .and_then(ProjectService::shm_size)
            .map(ProjectValue::value)
            .map(compose_lens::model::ShmSize::kind),
        Some(ShmSizeKind::Zero { amount_raw, unit: Some(ShmSizeUnit::M) }) if amount_raw == "00"
    ));
    let number = view
        .service("number")
        .and_then(ProjectService::shm_size)
        .ok_or("numeric shared-memory size expected")?;
    assert_eq!(number.value().scalar_kind(), ShmSizeScalarKind::Number);
    assert_eq!(number.value().kind(), &ShmSizeKind::ProviderDependentNumber);
    let string = view
        .service("string")
        .and_then(ProjectService::shm_size)
        .ok_or("string shared-memory size expected")?;
    assert_eq!(string.value().scalar_kind(), ShmSizeScalarKind::String);
    assert_eq!(string.value().kind(), &ShmSizeKind::ProviderDependentString);
    for service in ["boolean", "null", "list", "map"] {
        assert!(
            view.service(service)
                .ok_or("malformed service expected")?
                .shm_size()
                .is_none()
        );
    }
    assert_eq!(view.services().len(), 8);
    assert_shm_size_project_diagnostics(result.diagnostics());
    Ok(())
}

fn assert_shm_size_project_diagnostics(diagnostics: &[compose_lens::diagnostic::Diagnostic]) {
    use compose_lens::model::{
        SHM_SIZE_AMBIGUOUS_ZERO, SHM_SIZE_EXPECTED_VALUE, SHM_SIZE_PROVIDER_DEPENDENT_NUMBER,
        SHM_SIZE_PROVIDER_DEPENDENT_STRING,
    };

    for (code, expected) in [
        (SHM_SIZE_AMBIGUOUS_ZERO, 1),
        (SHM_SIZE_PROVIDER_DEPENDENT_NUMBER, 1),
        (SHM_SIZE_PROVIDER_DEPENDENT_STRING, 1),
        (SHM_SIZE_EXPECTED_VALUE, 4),
    ] {
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code() == code)
                .count(),
            expected,
            "unexpected diagnostic count for {code}"
        );
    }
}

#[test]
fn retains_effective_mem_limit_scalar_provenance_sensitivity_override_and_reset_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{MEM_LIMIT_EXPECTED_VALUE, MemLimitKind, MemLimitScalarKind, MemLimitUnit};

    let base = concat!(
        "services:\n",
        "  app:\n    mem_limit: 64m\n",
        "  reset:\n    mem_limit: 32m\n",
        "  number:\n    mem_limit: 64\n",
        "  string:\n    mem_limit: \"64\"\n",
        "  malformed:\n    mem_limit: [64m]\n",
    );
    let override_source = concat!(
        "services:\n",
        "  app:\n    mem_limit: !override ${MEM_LIMIT}\n",
        "  reset:\n    mem_limit: !reset null\n",
    );
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(766),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(767),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            override_source,
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("MEM_LIMIT", "128b");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let view = result.view().ok_or("project view expected")?;
    let limit = view
        .service("app")
        .and_then(ProjectService::mem_limit)
        .ok_or("effective memory limit expected")?;
    assert_eq!(limit.value().raw().value(), "128b");
    assert_eq!(limit.value().scalar_kind(), MemLimitScalarKind::String);
    assert!(matches!(
        limit.value().kind(),
        MemLimitKind::Documented { amount_raw, unit: MemLimitUnit::B } if amount_raw == "128"
    ));
    assert!(limit.is_sensitive());
    assert_eq!(limit.provenance().operation(), MergeOperation::Override);
    assert_source_ids(
        limit.provenance().sources(),
        &[SourceId::new(766), SourceId::new(767), SourceId::new(767)],
    );
    assert!(!format!("{limit:?}").contains("128b"));

    assert!(
        view.service("reset")
            .ok_or("reset service expected")?
            .mem_limit()
            .is_none()
    );
    let number = view
        .service("number")
        .and_then(ProjectService::mem_limit)
        .ok_or("numeric memory limit expected")?;
    assert_eq!(number.value().scalar_kind(), MemLimitScalarKind::Number);
    assert_eq!(number.value().kind(), &MemLimitKind::SchemaNumber);
    assert_eq!(
        view.service("string")
            .and_then(ProjectService::mem_limit)
            .map(ProjectValue::value)
            .map(compose_lens::model::MemLimit::kind),
        Some(&MemLimitKind::ProviderDependentString)
    );
    assert!(
        view.service("malformed")
            .ok_or("malformed expected")?
            .mem_limit()
            .is_none()
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == MEM_LIMIT_EXPECTED_VALUE)
            .count(),
        2
    );
    Ok(())
}

const CAP_DROP_BASE: &str = concat!(
    "services:\n",
    "  omitted:\n",
    "    image: example.invalid/omitted:1\n",
    "  empty:\n",
    "    cap_drop: []\n",
    "  normal:\n",
    "    cap_drop: [NET_ADMIN, CHOWN]\n",
    "  reset:\n",
    "    cap_drop: [NET_ADMIN]\n",
    "  replacement:\n",
    "    cap_drop: [NET_ADMIN]\n",
    "  sensitive:\n",
    "    cap_drop: [\"${DROP_CAP}\"]\n",
    "  malformed:\n",
    "    cap_drop: [CHOWN, true, [SYS_TIME], SYS_NICE]\n",
    "    image: example.invalid/recovered:1\n",
);

const CAP_DROP_OVERRIDE: &str = concat!(
    "services:\n",
    "  normal:\n",
    "    cap_drop: [CHOWN, net_admin]\n",
    "  reset:\n",
    "    cap_drop: !reset []\n",
    "  replacement:\n",
    "    cap_drop: !override [CHOWN, CHOWN, chown]\n",
    "  sensitive:\n",
    "    cap_drop: [NET_ADMIN]\n",
);

#[test]
fn exposes_cap_drop_omission_empty_merge_override_recovery_and_sensitivity() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(660),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            CAP_DROP_BASE,
        ),
        DocumentInput::new(
            SourceId::new(661),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            CAP_DROP_OVERRIDE,
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("DROP_CAP", "NET_ADMIN");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert!(
        view.service("omitted")
            .ok_or("omitted service expected")?
            .cap_drop()
            .is_none()
    );
    let empty = view
        .service("empty")
        .and_then(ProjectService::cap_drop)
        .ok_or("explicit empty cap_drop expected")?;
    assert!(empty.value().is_empty());
    assert_eq!(empty.provenance().operation(), MergeOperation::Authored);

    let normal = view
        .service("normal")
        .and_then(ProjectService::cap_drop)
        .ok_or("merged cap_drop expected")?;
    assert_eq!(
        normal
            .value()
            .iter()
            .map(|item| item.value().value())
            .collect::<Vec<_>>(),
        ["NET_ADMIN", "CHOWN", "net_admin"]
    );
    assert_source_ids(normal.provenance().sources(), &[SourceId::new(660), SourceId::new(661)]);
    assert_source_ids(
        normal.value()[1].provenance().sources(),
        &[SourceId::new(660), SourceId::new(661)],
    );
    assert!(normal.value()[0].value().is_exact_candidate());

    let reset = view
        .service("reset")
        .and_then(ProjectService::cap_drop)
        .ok_or("reset cap_drop expected")?;
    assert!(reset.value().is_empty());
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);

    let replacement = view
        .service("replacement")
        .and_then(ProjectService::cap_drop)
        .ok_or("overridden cap_drop expected")?;
    assert_eq!(replacement.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        replacement
            .value()
            .iter()
            .map(|item| item.value().value())
            .collect::<Vec<_>>(),
        ["CHOWN", "CHOWN", "chown"]
    );

    let sensitive_service = view.service("sensitive").ok_or("sensitive service expected")?;
    let sensitive = sensitive_service.cap_drop().ok_or("sensitive cap_drop expected")?;
    assert_eq!(sensitive.value()[0].value().value(), "NET_ADMIN");
    assert!(sensitive.is_sensitive());
    assert!(sensitive.value()[0].is_sensitive());
    assert!(!format!("{sensitive_service:?}").contains("NET_ADMIN"));

    let malformed_service = view.service("malformed").ok_or("malformed service expected")?;
    assert_eq!(
        malformed_service
            .cap_drop()
            .ok_or("recovered cap_drop expected")?
            .value()
            .iter()
            .map(|item| item.value().value())
            .collect::<Vec<_>>(),
        ["CHOWN", "SYS_NICE"]
    );
    assert!(malformed_service.image().is_some());
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == compose_lens::model::CAP_DROP_DUPLICATE_ITEM)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

const CAP_ADD_BASE: &str = concat!(
    "services:\n",
    "  omitted:\n",
    "    image: example.invalid/omitted:1\n",
    "  empty:\n",
    "    cap_add: []\n",
    "  normal:\n",
    "    cap_add: [NET_ADMIN, CHOWN]\n",
    "    cap_drop: [MKNOD]\n",
    "  reset:\n",
    "    cap_add: [NET_ADMIN]\n",
    "  replacement:\n",
    "    cap_add: [NET_ADMIN]\n",
    "  sensitive:\n",
    "    cap_add: [\"${ADD_CAP}\"]\n",
    "  malformed:\n",
    "    cap_add: [CHOWN, true, [SYS_TIME], SYS_NICE]\n",
    "    image: example.invalid/recovered:1\n",
);

const CAP_ADD_OVERRIDE: &str = concat!(
    "services:\n",
    "  normal:\n",
    "    cap_add: [CHOWN, net_admin]\n",
    "    cap_drop: [SYS_ADMIN]\n",
    "  reset:\n",
    "    cap_add: !reset []\n",
    "  replacement:\n",
    "    cap_add: !override [CHOWN, CHOWN, chown]\n",
    "  sensitive:\n",
    "    cap_add: [NET_ADMIN]\n",
);

#[test]
fn exposes_cap_add_omission_merge_recovery_and_independent_cap_drop() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(662),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            CAP_ADD_BASE,
        ),
        DocumentInput::new(
            SourceId::new(663),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            CAP_ADD_OVERRIDE,
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("ADD_CAP", "NET_ADMIN");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert!(
        view.service("omitted")
            .ok_or("omitted service expected")?
            .cap_add()
            .is_none()
    );
    let empty = view
        .service("empty")
        .and_then(ProjectService::cap_add)
        .ok_or("explicit empty cap_add expected")?;
    assert!(empty.value().is_empty());
    assert_eq!(empty.provenance().operation(), MergeOperation::Authored);

    let normal = view
        .service("normal")
        .and_then(ProjectService::cap_add)
        .ok_or("merged cap_add expected")?;
    assert_eq!(
        normal
            .value()
            .iter()
            .map(|item| item.value().value())
            .collect::<Vec<_>>(),
        ["NET_ADMIN", "CHOWN", "net_admin"]
    );
    assert_source_ids(normal.provenance().sources(), &[SourceId::new(662), SourceId::new(663)]);
    assert_source_ids(
        normal.value()[1].provenance().sources(),
        &[SourceId::new(662), SourceId::new(663)],
    );
    assert!(normal.value()[0].value().is_exact_candidate());
    assert_eq!(
        view.service("normal")
            .and_then(ProjectService::cap_drop)
            .ok_or("independent cap_drop expected")?
            .value()
            .iter()
            .map(|item| item.value().value())
            .collect::<Vec<_>>(),
        ["MKNOD", "SYS_ADMIN"]
    );

    let reset = view
        .service("reset")
        .and_then(ProjectService::cap_add)
        .ok_or("reset cap_add expected")?;
    assert!(reset.value().is_empty());
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);

    let replacement = view
        .service("replacement")
        .and_then(ProjectService::cap_add)
        .ok_or("overridden cap_add expected")?;
    assert_eq!(replacement.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        replacement
            .value()
            .iter()
            .map(|item| item.value().value())
            .collect::<Vec<_>>(),
        ["CHOWN", "CHOWN", "chown"]
    );

    assert_cap_add_sensitive_and_recovery(&result)?;
    Ok(())
}

fn assert_cap_add_sensitive_and_recovery(
    result: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let view = result.view().ok_or("project view expected")?;
    let sensitive_service = view.service("sensitive").ok_or("sensitive service expected")?;
    let sensitive = sensitive_service.cap_add().ok_or("sensitive cap_add expected")?;
    assert_eq!(sensitive.value()[0].value().value(), "NET_ADMIN");
    assert!(sensitive.is_sensitive());
    assert!(sensitive.value()[0].is_sensitive());
    assert!(!format!("{sensitive_service:?}").contains("NET_ADMIN"));

    let malformed_service = view.service("malformed").ok_or("malformed service expected")?;
    assert_eq!(
        malformed_service
            .cap_add()
            .ok_or("recovered cap_add expected")?
            .value()
            .iter()
            .map(|item| item.value().value())
            .collect::<Vec<_>>(),
        ["CHOWN", "SYS_NICE"]
    );
    assert!(malformed_service.image().is_some());
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == compose_lens::model::CAP_ADD_DUPLICATE_ITEM)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

#[test]
fn exposes_tmpfs_form_item_provenance_append_reset_override_recovery_and_redaction()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::TmpfsItemKind;

    let base = concat!(
        "services:\n",
        "  omitted: {}\n",
        "  appended:\n    tmpfs: [/base, /same]\n",
        "  sensitive:\n    tmpfs: /old\n",
        "  reset:\n    tmpfs: [/old]\n",
        "  replacement:\n    tmpfs: [/old]\n",
        "  malformed:\n    image: example.invalid/recovered:1\n    tmpfs: [/valid, true, [/nested], /later]\n",
        "  provider:\n    tmpfs: /run:size=64m\n",
        "  invalid-form:\n    image: example.invalid/invalid:1\n    tmpfs: { path: /run }\n",
    );
    let override_source = concat!(
        "services:\n",
        "  appended:\n    tmpfs: [/same, /later]\n",
        "  sensitive:\n    tmpfs: \"${TMPFS_SECRET}\"\n",
        "  reset:\n    tmpfs: !reset []\n",
        "  replacement:\n    tmpfs: !override [/same, /same, /case, /CASE]\n",
    );
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(664),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(665),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            override_source,
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("TMPFS_SECRET", "/secret:mode=0700");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert!(view.service("omitted").is_some_and(|service| service.tmpfs().is_none()));
    let appended = view
        .service("appended")
        .and_then(ProjectService::tmpfs)
        .ok_or("appended tmpfs expected")?;
    let ProjectTmpfs::List(items) = appended.value() else {
        return Err("effective tmpfs list expected".into());
    };
    assert_eq!(
        items.iter().map(|item| item.value().value()).collect::<Vec<_>>(),
        ["/base", "/same", "/same", "/later"]
    );
    assert_eq!(appended.provenance().operation(), MergeOperation::Appended);
    assert_source_ids(
        appended.provenance().sources(),
        &[SourceId::new(664), SourceId::new(665)],
    );
    assert_source_ids(items[1].provenance().sources(), &[SourceId::new(664)]);
    assert_source_ids(items[2].provenance().sources(), &[SourceId::new(665)]);

    let sensitive_service = view.service("sensitive").ok_or("sensitive service expected")?;
    let sensitive = sensitive_service.tmpfs().ok_or("sensitive scalar tmpfs expected")?;
    let ProjectTmpfs::Scalar(item) = sensitive.value() else {
        return Err("effective scalar tmpfs expected".into());
    };
    assert_eq!(item.value().value(), "/secret:mode=0700");
    assert_eq!(item.value().kind(), TmpfsItemKind::Documented);
    assert!(item.is_sensitive());
    assert!(sensitive.is_sensitive());
    assert_eq!(sensitive.provenance().operation(), MergeOperation::Replaced);
    assert!(!format!("{sensitive_service:?}").contains("/secret"));

    let reset = view
        .service("reset")
        .and_then(ProjectService::tmpfs)
        .ok_or("reset tmpfs expected")?;
    assert!(matches!(reset.value(), ProjectTmpfs::List(items) if items.is_empty()));
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);

    let replacement = view
        .service("replacement")
        .and_then(ProjectService::tmpfs)
        .ok_or("overridden tmpfs expected")?;
    assert_eq!(replacement.provenance().operation(), MergeOperation::Override);
    assert!(matches!(
        replacement.value(),
        ProjectTmpfs::List(items)
            if items.iter().map(|item| item.value().value()).collect::<Vec<_>>()
                == ["/same", "/same", "/case", "/CASE"]
    ));

    let malformed = view.service("malformed").ok_or("malformed service retained")?;
    assert!(malformed.image().is_some());
    assert!(matches!(
        malformed.tmpfs().map(ProjectValue::value),
        Some(ProjectTmpfs::List(items))
            if items.iter().map(|item| item.value().value()).collect::<Vec<_>>() == ["/valid", "/later"]
    ));
    assert!(
        view.service("invalid-form")
            .is_some_and(|service| service.image().is_some() && service.tmpfs().is_none())
    );

    assert_tmpfs_diagnostics(&result);
    Ok(())
}

fn assert_tmpfs_diagnostics(result: &compose_lens::project::ProjectViewResult) {
    use compose_lens::model::{TMPFS_EXPECTED_FORM, TMPFS_EXPECTED_STRING, TMPFS_PROVIDER_DEPENDENT};

    for (code, count) in [
        (TMPFS_EXPECTED_FORM, 1),
        (TMPFS_EXPECTED_STRING, 2),
        (TMPFS_PROVIDER_DEPENDENT, 1),
    ] {
        assert_eq!(
            result
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.code() == code)
                .count(),
            count,
            "unexpected project diagnostic count for {code}"
        );
    }
}

#[test]
fn exposes_device_target_replacement_nested_provenance_reset_override_and_redaction()
-> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(683);
    let override_id = SourceId::new(684);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            DEVICE_MERGE_BASE,
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            DEVICE_MERGE_OVERRIDE,
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("DEVICE_SECRET", "/dev/private:/dev/private:r");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert!(
        view.service("omitted")
            .is_some_and(|service| service.devices().is_none())
    );
    let merged_devices = view
        .service("merged")
        .and_then(ProjectService::devices)
        .ok_or("merged devices expected")?;
    assert_eq!(merged_devices.value().len(), 3);
    assert_eq!(merged_devices.provenance().operation(), MergeOperation::Merged);
    assert_source_ids(merged_devices.provenance().sources(), &[base_id, override_id]);
    let ProjectDevice::Short(replaced) = merged_devices.value()[0].value() else {
        return Err("replaced short device expected".into());
    };
    assert_eq!(replaced.raw().value(), "/dev/override:/dev/shared:rw");
    assert_eq!(
        merged_devices.value()[0].provenance().operation(),
        MergeOperation::Replaced
    );
    assert_source_ids(
        merged_devices.value()[0].provenance().sources(),
        &[base_id, override_id],
    );

    let ProjectDevice::Long(long) = merged_devices.value()[1].value() else {
        return Err("merged long device expected".into());
    };
    let source = long.source().ok_or("inherited source expected")?;
    assert_eq!(source.value(), "/dev/base-long");
    assert_source_ids(source.provenance().sources(), &[base_id]);
    let target = long.target().ok_or("merged target expected")?;
    assert_eq!(target.value(), "/dev/long");
    assert_source_ids(target.provenance().sources(), &[base_id, override_id]);
    let permissions = long.permissions().ok_or("replacement permissions expected")?;
    assert_eq!(permissions.value(), "provider-specific");
    assert_source_ids(permissions.provenance().sources(), &[base_id, override_id]);
    assert_eq!(long.extension_fields().len(), 2);
    assert_eq!(long.unknown_fields().len(), 2);

    let sensitive_service = view.service("sensitive").ok_or("sensitive service expected")?;
    let sensitive = sensitive_service.devices().ok_or("sensitive devices expected")?;
    assert!(sensitive.is_sensitive());
    assert!(sensitive.value()[1].is_sensitive());
    assert!(!format!("{sensitive_service:?}").contains("/dev/private"));

    let reset = view
        .service("reset")
        .and_then(ProjectService::devices)
        .ok_or("reset expected")?;
    assert!(reset.value().is_empty());
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    let overridden = view
        .service("overridden")
        .and_then(ProjectService::devices)
        .ok_or("override expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(overridden.value().len(), 3, "override preserves exact duplicate items");
    assert_device_shape_replacement(view, base_id, override_id)?;
    assert!(result.is_valid());
    Ok(())
}

fn assert_device_shape_replacement(
    view: &ProjectView,
    base_id: SourceId,
    override_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let shape = view
        .service("shape")
        .and_then(ProjectService::devices)
        .and_then(|devices| devices.value().first())
        .ok_or("shape-replaced device expected")?;
    assert!(matches!(shape.value(), ProjectDevice::Long(device)
        if device.source().is_some_and(|source| source.value() == "/dev/override-shape")));
    assert_eq!(shape.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(shape.provenance().sources(), &[base_id, override_id]);
    Ok(())
}

const DEVICE_MERGE_BASE: &str = concat!(
    "services:\n",
    "  omitted: {}\n",
    "  merged:\n",
    "    devices:\n",
    "      - /dev/base:/dev/shared:r\n",
    "      - source: /dev/base-long\n",
    "        target: /dev/long\n",
    "        permissions: r\n",
    "        x-base: retained\n",
    "        base-unknown: retained\n",
    "      - vendor.example/device=gpu\n",
    "  sensitive:\n",
    "    devices: [/dev/public]\n",
    "  shape:\n",
    "    devices: [/dev/base-shape:/dev/shape:r]\n",
    "  reset:\n",
    "    devices: [/dev/old]\n",
    "  overridden:\n",
    "    devices: [/dev/old]\n",
);

const DEVICE_MERGE_OVERRIDE: &str = concat!(
    "services:\n",
    "  merged:\n",
    "    devices:\n",
    "      - /dev/override:/dev/shared:rw\n",
    "      - target: /dev/long\n",
    "        permissions: provider-specific\n",
    "        x-later: retained\n",
    "        later-unknown: retained\n",
    "      - vendor.example/device=gpu\n",
    "  sensitive:\n",
    "    devices: [\"${DEVICE_SECRET}\"]\n",
    "  shape:\n",
    "    devices:\n",
    "      - source: /dev/override-shape\n",
    "        target: /dev/shape\n",
    "        permissions: rw\n",
    "  reset:\n",
    "    devices: !reset []\n",
    "  overridden:\n",
    "    devices: !override [/dev/same, /dev/same, vendor.example/device=gpu]\n",
);
