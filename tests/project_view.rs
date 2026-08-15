//! Native merged-project behavior and provenance.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::{EntrySyntax, MergeOperation, MergedScalarKind, merge_project};
use compose_lens::model::{
    BUILD_DOCKERFILE_INLINE_CONFLICT, BooleanValue, BuildNoCache, BuildSbom, Command, ComposeScalar,
    DEPLOY_ENDPOINT_MODE_PORTABILITY, DEPLOY_MODE_PORTABILITY, DNS_EXPECTED_FORM, DNS_EXPECTED_STRING,
    DNS_OPT_DUPLICATE_ITEM, DNS_OPT_EXPECTED_SEQUENCE, DNS_OPT_EXPECTED_STRING, DNS_SEARCH_DUPLICATE_ITEM,
    DNS_SEARCH_EXPECTED_FORM, DNS_SEARCH_EXPECTED_STRING, DependencyCondition, DeployEndpointMode, DeployMode,
    DeployPlacementMaxReplicasPerNode, DeployReplicas, DeployReservationDeviceCount, DeployResourceCpus,
    DeployResourceMemoryKind, DeployResourceMemoryUnit, DeployResourcePids, DeployRestartCondition, Entrypoint,
    EnvironmentFileFormatKind, HealthcheckDuration, HealthcheckRetries, HealthcheckTest, HealthcheckTestKind,
    HostAddressKind, HostnameKind, IdentityComponent, Labels, LimitValue, PidsLimitKind, Port, RestartPolicyKind,
    SYSCTLS_DUPLICATE_ITEM, SelinuxRelabel, ServiceInteger, ServiceNetworks, StopGracePeriod, ULIMIT_INVALID_NAME,
    ULIMIT_INVALID_VALUE, ULIMIT_MISSING_RANGE_MEMBER, UserNamespaceModeKind, VOLUME_EXTERNAL_DRIVER_CONFIGURATION,
    VOLUME_EXTERNAL_LABELS_CONFIGURATION, VolumeMount,
};
use compose_lens::profiles::{ProfileRequest, select_profiles};
use compose_lens::project::{
    PROJECT_EXPECTED_FORM, PROJECT_INVALID_VALUE, PROJECT_MISSING_FIELD, ProjectBuild, ProjectBuildAdditionalContexts,
    ProjectBuildArgs, ProjectBuildExtraHostAddresses, ProjectBuildExtraHosts, ProjectBuildLabels,
    ProjectBuildNoCacheFilter, ProjectBuildSsh, ProjectDependsOn, ProjectDevice, ProjectDns, ProjectDnsSearch,
    ProjectEnvironmentFile, ProjectFieldReference, ProjectGrant, ProjectLabelsForm, ProjectLoggingOptionValue,
    ProjectService, ProjectSysctls, ProjectTmpfs, ProjectUlimitValue, ProjectValue, ProjectView, build_project_view,
};
use compose_lens::resolution::SELECTION_PROJECT_MISMATCH;
use compose_lens::source::SourceId;

const BASE: &str = include_str!("../fixtures/processing/typed-project-view/compose.yaml");
const OVERRIDE: &str = include_str!("../fixtures/processing/typed-project-view/compose.override.yaml");

#[test]
fn retains_effective_deploy_endpoint_mode_merge_provenance_and_unmodeled_children()
-> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(2502);
    let override_id = SourceId::new(2503);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    deploy: {endpoint_mode: vip}\n",
                "  reset:\n    deploy: {endpoint_mode: vip}\n",
                "  overridden:\n    deploy: {endpoint_mode: vip}\n",
                "  sensitive:\n    deploy: {endpoint_mode: \"${ENDPOINT_MODE}\"}\n",
                "  provider:\n    deploy: {endpoint_mode: mesh}\n",
                "  siblings:\n    deploy: {endpoint_mode: dnsrr, replicas: 2, resources: {limits: {cpus: \"0.5\"}}}\n",
                "  malformed:\n    deploy: {endpoint_mode: true, replicas: 4}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    deploy: {endpoint_mode: dnsrr}\n",
                "  reset:\n    deploy: {endpoint_mode: !reset null}\n",
                "  overridden:\n    deploy: !override {endpoint_mode: dnsrr, replicas: 3}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("ENDPOINT_MODE", "vip");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;
    let deploy = |name| {
        view.service(name)
            .and_then(ProjectService::deploy)
            .ok_or("effective deploy expected")
    };
    let replaced = deploy("replaced")?
        .value()
        .endpoint_mode()
        .ok_or("replaced endpoint mode expected")?;
    assert!(matches!(replaced.value(), DeployEndpointMode::Dnsrr));
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    let reset = deploy("reset")?;
    assert!(reset.value().endpoint_mode().is_none());
    assert!(reset.value().unmodeled_fields().iter().any(|field| {
        field.path() == ["services", "reset", "deploy", "endpoint_mode"]
            && field.provenance().operation() == MergeOperation::Reset
    }));
    let overridden = deploy("overridden")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(
        overridden.value().endpoint_mode().map(ProjectValue::value),
        Some(DeployEndpointMode::Dnsrr)
    ));
    assert!(matches!(
        overridden.value().replicas().map(ProjectValue::value),
        Some(DeployReplicas::YamlNumber(value)) if value == "3"
    ));
    let sensitive = deploy("sensitive")?
        .value()
        .endpoint_mode()
        .ok_or("sensitive endpoint mode expected")?;
    assert!(matches!(sensitive.value(), DeployEndpointMode::Vip) && sensitive.is_sensitive());
    assert!(!format!("{sensitive:?}").contains("vip"));
    assert!(matches!(
        deploy("provider")?.value().endpoint_mode().map(ProjectValue::value),
        Some(DeployEndpointMode::Other(value)) if value == "mesh"
    ));
    let siblings = deploy("siblings")?.value();
    assert!(matches!(
        siblings.endpoint_mode().map(ProjectValue::value),
        Some(DeployEndpointMode::Dnsrr)
    ));
    assert!(matches!(
        siblings.replicas().map(ProjectValue::value),
        Some(DeployReplicas::YamlNumber(value)) if value == "2"
    ));
    assert!(siblings.resources().is_some());
    assert!(siblings.unmodeled_fields().is_empty());
    let malformed = deploy("malformed")?.value();
    assert!(malformed.endpoint_mode().is_none());
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| { field.path() == ["services", "malformed", "deploy", "endpoint_mode"] })
    );
    assert_deploy_endpoint_mode_diagnostics(&result);
    Ok(())
}

fn assert_deploy_endpoint_mode_diagnostics(result: &compose_lens::project::ProjectViewResult) {
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DEPLOY_ENDPOINT_MODE_PORTABILITY)
    );
}

#[test]
fn retains_effective_deploy_mode_merge_provenance_and_unmodeled_siblings() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(2602);
    let override_id = SourceId::new(2603);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    deploy: {mode: global}\n",
                "  reset:\n    deploy: {mode: global}\n",
                "  overridden:\n    deploy: {mode: global}\n",
                "  sensitive:\n    deploy: {mode: \"${DEPLOY_MODE}\"}\n",
                "  global-replicas:\n    deploy: {mode: global, replicas: 2}\n    scale: 3\n",
                "  omitted:\n    deploy: {replicas: 2}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    deploy: {mode: replicated}\n",
                "  reset:\n    deploy: {mode: !reset null}\n",
                "  overridden:\n    deploy: !override {mode: replicated, placement: {}}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("DEPLOY_MODE", "global");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;
    let deploy = |name| {
        view.service(name)
            .and_then(ProjectService::deploy)
            .ok_or("effective deploy expected")
    };
    let replaced = deploy("replaced")?
        .value()
        .mode()
        .ok_or("replaced deploy mode expected")?;
    assert!(matches!(replaced.value(), DeployMode::Replicated));
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    let reset = deploy("reset")?;
    assert!(reset.value().mode().is_none());
    assert!(
        reset
            .value()
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "reset", "deploy", "mode"]
                && field.provenance().operation() == MergeOperation::Reset)
    );
    let overridden = deploy("overridden")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(
        overridden.value().mode().map(ProjectValue::value),
        Some(DeployMode::Replicated)
    ));
    assert!(overridden.value().placement().is_some_and(|placement| {
        placement.value().constraints().is_none()
            && placement.value().preferences().is_none()
            && placement.value().max_replicas_per_node().is_none()
    }));
    let sensitive = deploy("sensitive")?.value().mode().ok_or("sensitive mode expected")?;
    assert!(matches!(sensitive.value(), DeployMode::Global) && sensitive.is_sensitive());
    let global = deploy("global-replicas")?.value();
    assert!(matches!(
        global.mode().map(ProjectValue::value),
        Some(DeployMode::Global)
    ));
    assert!(matches!(
        global.replicas().map(ProjectValue::value),
        Some(DeployReplicas::YamlNumber(value)) if value == "2"
    ));
    assert!(view.service("global-replicas").is_some_and(|service| {
        matches!(
            service.scale().map(ProjectValue::value),
            Some(ServiceInteger::Valid(value)) if value == "3"
        ) && !service
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "global-replicas", "scale"])
    }));
    let omitted = deploy("omitted")?.value();
    assert!(omitted.mode().is_none());
    assert!(matches!(
        omitted.replicas().map(ProjectValue::value),
        Some(DeployReplicas::YamlNumber(value)) if value == "2"
    ));
    assert!(
        !result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DEPLOY_MODE_PORTABILITY)
    );
    Ok(())
}

#[test]
fn retains_effective_deploy_replicas_merge_provenance_and_scalar_category() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(2702);
    let override_id = SourceId::new(2703);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n  replaced:\n    deploy: {replicas: 2}\n  reset:\n    deploy: {replicas: 2}\n",
                "  overridden:\n    deploy: {replicas: 2}\n  sensitive:\n    deploy: {replicas: \"${REPLICAS}\"}\n",
                "  global:\n    deploy: {mode: global, replicas: 2}\n    scale: 3\n",
                "  mode-only:\n    deploy: {mode: global}\n  invalid:\n    deploy:\n      replicas: false\n      mode: global\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n  replaced:\n    deploy: {replicas: 3}\n  reset:\n    deploy: {replicas: !reset null}\n",
                "  overridden:\n    deploy: !override {replicas: 4}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("REPLICAS", "private");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;
    let deploy = |name| {
        view.service(name)
            .and_then(ProjectService::deploy)
            .ok_or("effective deploy expected")
    };
    let replaced = deploy("replaced")?
        .value()
        .replicas()
        .ok_or("replaced replicas expected")?;
    assert!(matches!(replaced.value(), DeployReplicas::YamlNumber(value) if value == "3"));
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    let reset = deploy("reset")?.value();
    assert!(reset.replicas().is_none());
    assert!(
        reset
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "reset", "deploy", "replicas"]
                && field.provenance().operation() == MergeOperation::Reset)
    );
    let overridden = deploy("overridden")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(
        matches!(overridden.value().replicas().map(ProjectValue::value), Some(DeployReplicas::YamlNumber(value)) if value == "4")
    );
    let sensitive = deploy("sensitive")?
        .value()
        .replicas()
        .ok_or("sensitive replicas expected")?;
    assert!(
        matches!(sensitive.value(), DeployReplicas::String(value) if value == "private") && sensitive.is_sensitive()
    );
    assert!(!format!("{sensitive:?}").contains("private"));
    let global = deploy("global")?.value();
    assert!(
        matches!(global.replicas().map(ProjectValue::value), Some(DeployReplicas::YamlNumber(value)) if value == "2")
    );
    assert!(view.service("global").is_some_and(|service| {
        matches!(
            service.scale().map(ProjectValue::value),
            Some(ServiceInteger::Valid(value)) if value == "3"
        ) && !service
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "global", "scale"])
    }));
    assert!(deploy("mode-only")?.value().replicas().is_none());
    let invalid = deploy("invalid")?.value();
    assert!(invalid.replicas().is_none() && invalid.mode().is_some());
    assert!(
        invalid
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "invalid", "deploy", "replicas"])
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
fn retains_effective_deploy_labels_forms_merge_provenance_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(2802);
    let override_id = SourceId::new(2803);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n  map:\n    deploy:\n      labels: {kept: base, replaced: base, number: 2, sensitive: \"${LABEL}\"}\n",
                "  list:\n    deploy: {labels: [bare, pair=base, pair=base]}\n  reset:\n    deploy: {labels: {old: base}}\n",
                "  overridden:\n    deploy: {labels: {old: base}}\n  malformed:\n    deploy:\n      labels:\n        kept: value\n        bad: {nested: value}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n  map:\n    deploy: {labels: {replaced: override}}\n  list:\n    deploy: {labels: [pair=override]}\n",
                "  reset:\n    deploy: {labels: !reset {}}\n  overridden:\n    deploy: {labels: !override {only: override}}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("LABEL", "private");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;
    let labels = |name| {
        view.service(name)
            .and_then(ProjectService::deploy)
            .and_then(|deploy| deploy.value().labels())
            .ok_or("effective deploy labels expected")
    };
    let map = labels("map")?;
    assert_eq!(map.value().form(), ProjectLabelsForm::Map);
    let replaced = map.value().get("replaced").ok_or("replaced deploy label expected")?;
    assert!(matches!(replaced.value().value(), ComposeScalar::String(value) if value == "override"));
    assert_eq!(replaced.value().provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.value().provenance().sources(), &[base_id, override_id]);
    assert!(
        matches!(map.value().get("number").map(|entry| entry.value().value()), Some(ComposeScalar::Number(value)) if value == "2")
    );
    assert_eq!(map.provenance().operation(), MergeOperation::Merged);
    assert!(
        map.value()
            .get("sensitive")
            .is_some_and(|entry| entry.value().is_sensitive())
    );
    assert!(!format!("{map:?}").contains("private"));
    let list = labels("list")?;
    assert_eq!(list.value().form(), ProjectLabelsForm::List);
    assert_eq!(list.value().entries().len(), 4);
    assert_eq!(list.value().entries()[0].syntax(), EntrySyntax::ListKeyOnly);
    assert_eq!(
        list.value().entries()[1].value().value(),
        &ComposeScalar::String("base".into())
    );
    assert_eq!(
        list.value().entries()[2].value().value(),
        &ComposeScalar::String("base".into())
    );
    assert_eq!(
        list.value().entries()[3].value().value(),
        &ComposeScalar::String("override".into())
    );
    let reset = labels("reset")?;
    assert!(
        reset.value().entries().is_empty()
            && reset.value().form() == ProjectLabelsForm::Map
            && reset.provenance().operation() == MergeOperation::Reset
    );
    let overridden = labels("overridden")?;
    assert!(overridden.value().get("old").is_none());
    assert!(
        overridden.value().get("only").is_some() && overridden.provenance().operation() == MergeOperation::Override
    );
    let malformed = labels("malformed")?;
    assert_eq!(malformed.value().entries().len(), 1);
    assert!(
        view.service("malformed")
            .and_then(ProjectService::deploy)
            .is_some_and(|deploy| deploy
                .value()
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", "malformed", "deploy", "labels"]))
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
fn retains_effective_deploy_update_config_merge_reset_override_and_sensitivity()
-> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(3151),
            DocumentOrigin::new("base", "workspace"),
            "services:\n  merged:\n    deploy: {update_config: {parallelism: 1, delay: \"${DELAY}\", failure_action: pause}}\n  reset:\n    deploy: {update_config: {parallelism: 1}}\n  override:\n    deploy: {update_config: {parallelism: 1, delay: old}}\n",
        ),
        DocumentInput::new(
            SourceId::new(3152),
            DocumentOrigin::new("override", "workspace"),
            "services:\n  merged:\n    deploy: {update_config: {monitor: new, order: vendor}}\n  reset:\n    deploy: {update_config: !reset {}}\n  override:\n    deploy: {update_config: !override {order: start-first}}\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("DELAY", "private-delay");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let update = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::deploy)
            .and_then(|deploy| deploy.value().update_config())
            .ok_or("update config")
    };
    let value = update("merged")?;
    assert!(
        matches!(value.value().parallelism().map(ProjectValue::value), Some(compose_lens::model::DeployUpdateParallelism::YamlInteger(raw)) if raw == "1")
    );
    assert_eq!(
        value.value().monitor().map(ProjectValue::value).map(String::as_str),
        Some("new")
    );
    assert!(value.value().delay().is_some_and(ProjectValue::is_sensitive));
    assert!(!format!("{value:?}").contains("private-delay"));
    assert!(
        matches!(value.value().order().map(ProjectValue::value), Some(compose_lens::model::DeployUpdateOrder::Other(raw)) if raw == "vendor")
    );
    let reset = update("reset")?;
    assert!(reset.value().parallelism().is_none());
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    let overridden = update("override")?;
    assert!(overridden.value().delay().is_none());
    assert!(matches!(
        overridden.value().order().map(ProjectValue::value),
        Some(compose_lens::model::DeployUpdateOrder::StartFirst)
    ));
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    Ok(())
}

#[test]
fn retains_effective_deploy_rollback_config_merge_reset_override_and_sensitivity()
-> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(3155),
            DocumentOrigin::new("base", "workspace"),
            "services:\n  merged:\n    deploy:\n      rollback_config:\n        parallelism: 1\n        delay: \"${DELAY}\"\n        failure_action: pause\n      update_config:\n        order: start-first\n  reset:\n    deploy:\n      rollback_config:\n        parallelism: 1\n  override:\n    deploy:\n      rollback_config:\n        parallelism: 1\n        delay: old\n",
        ),
        DocumentInput::new(
            SourceId::new(3156),
            DocumentOrigin::new("override", "workspace"),
            "services:\n  merged:\n    deploy:\n      rollback_config:\n        monitor: new\n        order: vendor\n  reset:\n    deploy:\n      rollback_config: !reset {}\n  override:\n    deploy:\n      rollback_config: !override\n        order: stop-first\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("DELAY", "private-delay");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let rollback = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::deploy)
            .and_then(|deploy| deploy.value().rollback_config())
            .ok_or("rollback config")
    };
    let value = rollback("merged")?;
    assert!(matches!(
        value.value().parallelism().map(ProjectValue::value),
        Some(compose_lens::model::DeployRollbackParallelism::YamlInteger(raw)) if raw == "1"
    ));
    assert_eq!(
        value.value().monitor().map(ProjectValue::value).map(String::as_str),
        Some("new")
    );
    assert!(value.value().delay().is_some_and(ProjectValue::is_sensitive));
    assert!(!format!("{value:?}").contains("private-delay"));
    assert!(matches!(
        value.value().order().map(ProjectValue::value),
        Some(compose_lens::model::DeployRollbackOrder::Other(raw)) if raw == "vendor"
    ));
    assert!(
        result
            .view()
            .and_then(|view| view.service("merged"))
            .and_then(ProjectService::deploy)
            .and_then(|deploy| deploy.value().update_config())
            .is_some()
    );
    let reset = rollback("reset")?;
    assert!(reset.value().parallelism().is_none());
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    let overridden = rollback("override")?;
    assert!(overridden.value().delay().is_none());
    assert!(matches!(
        overridden.value().order().map(ProjectValue::value),
        Some(compose_lens::model::DeployRollbackOrder::StopFirst)
    ));
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.code() == compose_lens::model::DEPLOY_ROLLBACK_CONFIG_ORDER_PORTABILITY })
    );
    Ok(())
}

#[test]
fn retains_effective_credential_spec_merge_reset_override_and_sensitivity() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(3159),
            DocumentOrigin::new("base", "workspace"),
            "services:\n  merged:\n    credential_spec:\n      config: old\n      file: \"${SECRET_FILE}\"\n  reset:\n    credential_spec: {config: old}\n  override:\n    credential_spec: {config: old, file: old-file}\n  null:\n    credential_spec: {config: old}\n",
        ),
        DocumentInput::new(
            SourceId::new(3160),
            DocumentOrigin::new("override", "workspace"),
            "services:\n  merged:\n    credential_spec:\n      config: new\n      registry: registry://account\n  reset:\n    credential_spec: !reset {}\n  override:\n    credential_spec: !override {registry: registry://override}\n  null:\n    credential_spec: !reset null\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("SECRET_FILE", "private-file");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let credential_spec = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::credential_spec)
            .ok_or("credential spec")
    };
    let value = credential_spec("merged")?;
    assert_eq!(
        value.value().config().map(ProjectValue::value).map(String::as_str),
        Some("new")
    );
    assert_eq!(
        value.value().registry().map(ProjectValue::value).map(String::as_str),
        Some("registry://account")
    );
    assert!(value.value().file().is_some_and(ProjectValue::is_sensitive));
    assert!(!format!("{value:?}").contains("private-file"));
    let reset = credential_spec("reset")?;
    assert!(reset.value().config().is_none());
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    let overridden = credential_spec("override")?;
    assert!(overridden.value().config().is_none());
    assert_eq!(
        overridden
            .value()
            .registry()
            .map(ProjectValue::value)
            .map(String::as_str),
        Some("registry://override")
    );
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    let reset_null = credential_spec("null")?;
    assert!(reset_null.value().config().is_none());
    assert_eq!(reset_null.provenance().operation(), MergeOperation::Reset);
    Ok(())
}

#[test]
fn retains_effective_extends_without_expansion_or_file_lookup() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(3163),
            DocumentOrigin::new("base", "workspace"),
            "services:\n  merged:\n    extends:\n      service: \"${PARENT}\"\n      file: base.yml\n  reset:\n    extends: {service: old}\n  override:\n    extends: {service: old, file: old.yml}\n  null:\n    extends: {service: old}\n  shape:\n    extends: short-parent\n",
        ),
        DocumentInput::new(
            SourceId::new(3164),
            DocumentOrigin::new("override", "workspace"),
            "services:\n  merged:\n    extends:\n      file: override.yml\n  reset:\n    extends: !reset {}\n  override:\n    extends: !override {service: override-parent}\n  null:\n    extends: !reset null\n  shape:\n    extends: {service: map-parent}\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("PARENT", "private-parent");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let extends = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::extends)
            .ok_or("extends")
    };
    let merged = extends("merged")?;
    let compose_lens::project::ProjectExtends::Long(reference) = merged.value() else {
        return Err("merged long form expected".into());
    };
    assert!(reference.service().is_some_and(ProjectValue::is_sensitive));
    assert_eq!(
        reference.file().map(ProjectValue::value).map(String::as_str),
        Some("override.yml")
    );
    assert!(!format!("{merged:?}").contains("private-parent"));
    let reset = extends("reset")?;
    assert!(matches!(reset.value(), compose_lens::project::ProjectExtends::Long(value) if value.service().is_none()));
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    let overridden = extends("override")?;
    assert!(matches!(
        overridden.value(),
        compose_lens::project::ProjectExtends::Long(value)
            if value.service().map(ProjectValue::value).map(String::as_str) == Some("override-parent")
    ));
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    let reset_null = extends("null")?;
    assert!(
        matches!(reset_null.value(), compose_lens::project::ProjectExtends::Long(value) if value.service().is_none())
    );
    assert_eq!(reset_null.provenance().operation(), MergeOperation::Reset);
    assert!(matches!(
        extends("shape")?.value(),
        compose_lens::project::ProjectExtends::Long(_)
    ));
    Ok(())
}

#[test]
fn retains_effective_provider_merge_reset_override_and_sensitivity() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(3169),
            DocumentOrigin::new("base", "workspace"),
            "services:\n  merged:\n    provider:\n      type: \"${TYPE}\"\n      options:\n        secret: \"${TOKEN}\"\n        replaced: first\n        values: [one, true]\n  reset:\n    provider:\n      type: old\n      options: {old: value}\n  override:\n    provider:\n      type: old\n      options: {old: value}\n  missing:\n    provider: {options: {kept: value}}\n  shape:\n    provider:\n      type: example\n      options: {value: scalar}\n",
        ),
        DocumentInput::new(
            SourceId::new(3170),
            DocumentOrigin::new("override", "workspace"),
            "services:\n  merged:\n    provider:\n      options:\n        replaced: second\n        values: [2, false]\n        added: true\n  reset:\n    provider:\n      options: !reset {}\n  override:\n    provider: !override {type: override, options: {only: true}}\n  shape:\n    provider:\n      options: {value: [one]}\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("TYPE", "private-provider");
    let _ = environment.insert_sensitive("TOKEN", "private-option");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let provider = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::provider)
            .ok_or("provider")
    };
    let merged = provider("merged")?;
    assert!(merged.value().type_().is_some_and(ProjectValue::is_sensitive));
    assert!(!format!("{merged:?}").contains("private-provider"));
    let options = merged.value().options().ok_or("merged options")?.value();
    let option = |name| {
        options
            .entries()
            .iter()
            .find(|entry| entry.value().name().value() == name)
            .ok_or("option")
    };
    assert!(option("secret")?.is_sensitive());
    assert!(!format!("{:?}", option("secret")?).contains("private-option"));
    assert!(
        matches!(option("replaced")?.value().value().value(), compose_lens::project::ProjectProviderOptionValue::Scalar(ComposeScalar::String(value)) if value == "second")
    );
    assert!(matches!(
        option("added")?.value().value().value(),
        compose_lens::project::ProjectProviderOptionValue::Scalar(ComposeScalar::Boolean(true))
    ));
    assert!(
        matches!(option("values")?.value().value().value(), compose_lens::project::ProjectProviderOptionValue::Sequence(items)
            if matches!(items.as_slice(), [first, second, third, fourth]
                if matches!(first.value(), compose_lens::project::ProjectProviderOptionItem::Scalar(ComposeScalar::String(value)) if value == "one")
                    && matches!(second.value(), compose_lens::project::ProjectProviderOptionItem::Scalar(ComposeScalar::Boolean(true)))
                    && matches!(third.value(), compose_lens::project::ProjectProviderOptionItem::Scalar(ComposeScalar::Number(value)) if value == "2")
                    && matches!(fourth.value(), compose_lens::project::ProjectProviderOptionItem::Scalar(ComposeScalar::Boolean(false)))
            )
        )
    );
    let reset = provider("reset")?;
    assert!(
        reset
            .value()
            .options()
            .is_some_and(|options| options.value().is_empty())
    );
    let overridden = provider("override")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(
        overridden.value().type_().map(ProjectValue::value).map(String::as_str),
        Some("override")
    ));
    assert!(matches!(
        provider("shape")?
            .value()
            .options()
            .map(ProjectValue::value)
            .and_then(|options| options.entries().first())
            .map(|option| option.value().value().value()),
        Some(compose_lens::project::ProjectProviderOptionValue::Sequence(_))
    ));
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == compose_lens::model::PROVIDER_MISSING_TYPE)
    );
    Ok(())
}

#[test]
fn retains_effective_post_start_append_reset_override_and_sensitivity() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3174);
    let override_id = SourceId::new(3175);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("base", "workspace"),
            "services:\n  merged:\n    post_start:\n      - command: \"${HOOK}\"\n        environment: {ONE: base}\n      - command: [echo, base]\n  reset:\n    post_start: [{command: old}]\n  override:\n    post_start: [{command: old}]\n  malformed:\n    post_start: [{command: old}]\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  merged:\n    post_start:\n      - command: later\n        environment: [TWO=two]\n        privileged: true\n        user: hook-user\n        working_dir: /hook\n  reset:\n    post_start: !reset []\n  override:\n    post_start: !override [{command: override}]\n  malformed:\n    post_start: [{environment: [ONE=one]}, invalid]\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("HOOK", "private-hook");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let hooks = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::post_start)
            .ok_or("post-start hooks")
    };
    let merged = hooks("merged")?;
    assert_eq!(merged.value().len(), 3);
    let compose_lens::project::ProjectPostStartHook::Hook(first) = merged.value()[0].value() else {
        return Err("first hook expected".into());
    };
    assert!(first.command().is_some_and(ProjectValue::is_sensitive));
    assert!(!format!("{:?}", merged.value()[0]).contains("private-hook"));
    assert!(matches!(
        merged.value()[1].value(),
        compose_lens::project::ProjectPostStartHook::Hook(hook)
            if matches!(hook.command().map(ProjectValue::value), Some(Command::List { values, .. }) if values.len() == 2)
    ));
    let compose_lens::project::ProjectPostStartHook::Hook(last) = merged.value()[2].value() else {
        return Err("last hook expected".into());
    };
    assert!(last.environment().is_some());
    assert!(matches!(
        last.privileged().map(ProjectValue::value),
        Some(BooleanValue::Literal(true))
    ));
    assert_eq!(
        last.user().map(ProjectValue::value).map(String::as_str),
        Some("hook-user")
    );
    assert_eq!(
        last.working_dir().map(ProjectValue::value).map(String::as_str),
        Some("/hook")
    );
    assert_source_ids(merged.provenance().sources(), &[base_id, override_id]);
    let reset = hooks("reset")?;
    assert!(reset.value().is_empty());
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    let overridden = hooks("override")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(overridden.value().len(), 1);
    assert!(matches!(
        overridden.value()[0].value(),
        compose_lens::project::ProjectPostStartHook::Hook(hook)
            if matches!(hook.command().map(ProjectValue::value), Some(Command::String(value)) if value.value() == "override")
    ));
    let malformed = hooks("malformed")?;
    assert_eq!(malformed.value().len(), 3);
    assert!(matches!(
        malformed.value()[1].value(),
        compose_lens::project::ProjectPostStartHook::Hook(hook) if hook.command().is_none()
    ));
    assert!(matches!(
        malformed.value()[2].value(),
        compose_lens::project::ProjectPostStartHook::Unmodeled
    ));
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == compose_lens::model::POST_START_MISSING_COMMAND)
    );
    Ok(())
}

#[test]
fn retains_effective_pre_stop_append_reset_override_and_null_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3181);
    let override_id = SourceId::new(3182);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("base", "workspace"),
            "services:\n  merged:\n    pre_stop: [{command: \"${HOOK}\", environment: {LOCAL: base}}]\n  reset:\n    pre_stop: [{command: old}]\n  override:\n    pre_stop: [{command: old}]\n  null:\n    pre_stop: [{command: old}]\n  missing:\n    pre_stop: [{environment: [LOCAL=base]}]\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  merged:\n    pre_stop: [{command: later, privileged: true, user: hook-user, working_dir: /hook}]\n  reset:\n    pre_stop: !reset []\n  override:\n    pre_stop: !override [{command: replacement}]\n  null:\n    pre_stop: !reset null\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("HOOK", "private-hook");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let hooks = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::pre_stop)
            .ok_or("pre-stop hooks")
    };
    let merged = hooks("merged")?;
    assert_eq!(merged.value().len(), 2);
    let compose_lens::project::ProjectPreStopHook::Hook(first) = merged.value()[0].value() else {
        return Err("first pre-stop hook expected".into());
    };
    assert!(first.command().is_some_and(ProjectValue::is_sensitive));
    assert!(!format!("{:?}", merged.value()[0]).contains("private-hook"));
    let compose_lens::project::ProjectPreStopHook::Hook(last) = merged.value()[1].value() else {
        return Err("last pre-stop hook expected".into());
    };
    assert!(matches!(
        last.privileged().map(ProjectValue::value),
        Some(BooleanValue::Literal(true))
    ));
    assert_eq!(
        last.user().map(ProjectValue::value).map(String::as_str),
        Some("hook-user")
    );
    assert_source_ids(merged.provenance().sources(), &[base_id, override_id]);
    let reset = hooks("reset")?;
    assert!(reset.value().is_empty());
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    let overridden = hooks("override")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(
        overridden.value()[0].value(),
        compose_lens::project::ProjectPreStopHook::Hook(hook)
            if matches!(hook.command().map(ProjectValue::value), Some(Command::String(value)) if value.value() == "replacement")
    ));
    let reset_null = hooks("null")?;
    assert!(reset_null.value().is_empty());
    assert_eq!(reset_null.provenance().operation(), MergeOperation::Reset);
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == compose_lens::model::PRE_STOP_MISSING_COMMAND)
    );
    Ok(())
}

#[test]
fn retains_effective_pre_start_optional_members_and_merge_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3192);
    let override_id = SourceId::new(3193);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("base", "workspace"),
            "services:\n  merged:\n    pre_start: [{}, {command: \"${COMMAND}\", image: \"${IMAGE}\", environment: {LOCAL: base}}]\n  reset:\n    pre_start: [{command: old}]\n  override:\n    pre_start: [{command: old}]\n  null:\n    pre_start: [{command: old}]\n  malformed:\n    pre_start: [{command: old}]\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  merged:\n    pre_start: [{command: later, privileged: true, per_replica: true, user: hook-user, working_dir: /hook}]\n  reset:\n    pre_start: !reset []\n  override:\n    pre_start: !override [{image: replacement}]\n  null:\n    pre_start: !reset null\n  malformed:\n    pre_start: [invalid, {privileged: sometimes, per_replica: maybe, image: 1, user: 1000, working_dir: false}]\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("COMMAND", "private-command");
    let _ = environment.insert_sensitive("IMAGE", "private image @ all");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let hooks = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::pre_start)
            .ok_or("pre-start hooks")
    };
    let merged = hooks("merged")?;
    assert_eq!(merged.value().len(), 3);
    assert!(
        matches!(merged.value()[0].value(), compose_lens::project::ProjectPreStartHook::Hook(hook) if hook.command().is_none())
    );
    let compose_lens::project::ProjectPreStartHook::Hook(second) = merged.value()[1].value() else {
        return Err("second pre-start hook expected".into());
    };
    assert!(second.command().is_some_and(ProjectValue::is_sensitive));
    assert!(second.image().is_some_and(ProjectValue::is_sensitive));
    assert!(!format!("{:?}", merged.value()[1]).contains("private image @ all"));
    let compose_lens::project::ProjectPreStartHook::Hook(last) = merged.value()[2].value() else {
        return Err("last pre-start hook expected".into());
    };
    assert!(matches!(
        last.privileged().map(ProjectValue::value),
        Some(BooleanValue::Literal(true))
    ));
    assert!(matches!(
        last.per_replica().map(ProjectValue::value),
        Some(BooleanValue::Literal(true))
    ));
    assert_eq!(
        last.user().map(ProjectValue::value).map(String::as_str),
        Some("hook-user")
    );
    assert_source_ids(merged.provenance().sources(), &[base_id, override_id]);
    let reset = hooks("reset")?;
    assert!(reset.value().is_empty());
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    let overridden = hooks("override")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(
        overridden.value()[0].value(),
        compose_lens::project::ProjectPreStartHook::Hook(hook)
            if hook.command().is_none()
                && hook.image().map(ProjectValue::value).map(String::as_str) == Some("replacement")
    ));
    let reset_null = hooks("null")?;
    assert!(reset_null.value().is_empty());
    assert_eq!(reset_null.provenance().operation(), MergeOperation::Reset);
    let malformed = hooks("malformed")?;
    assert_eq!(malformed.value().len(), 3);
    assert!(matches!(
        malformed.value()[1].value(),
        compose_lens::project::ProjectPreStartHook::Unmodeled
    ));
    assert!(matches!(
        malformed.value()[2].value(),
        compose_lens::project::ProjectPreStartHook::Hook(hook)
            if hook.privileged().is_none()
                && hook.per_replica().is_none()
                && hook.image().is_none()
                && hook.unmodeled_fields().len() == 5
    ));
    assert!(!result.diagnostics().iter().any(|diagnostic| {
        matches!(
            diagnostic.code(),
            compose_lens::model::POST_START_MISSING_COMMAND | compose_lens::model::PRE_STOP_MISSING_COMMAND
        )
    }));
    Ok(())
}

#[test]
fn retains_effective_service_runtime_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3199);
    let override_id = SourceId::new(3200);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("base", "workspace"),
            "services:\n  replaced:\n    runtime: first\n  sensitive:\n    runtime: \"${RUNTIME}\"\n  reset:\n    runtime: first\n  override:\n    runtime: first\n  malformed:\n    runtime: first\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  replaced:\n    runtime: second\n  reset:\n    runtime: !reset null\n  override:\n    runtime: !override replacement\n  malformed:\n    runtime: 1\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("RUNTIME", "private-runtime");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let runtime = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::runtime)
            .ok_or("runtime")
    };
    let replaced = runtime("replaced")?;
    assert_eq!(replaced.value(), "second");
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    let sensitive = runtime("sensitive")?;
    assert!(sensitive.is_sensitive());
    assert!(!format!("{sensitive:?}").contains("private-runtime"));
    assert!(
        result
            .view()
            .and_then(|view| view.service("reset"))
            .is_some_and(|service| service.runtime().is_none())
    );
    let overridden = runtime("override")?;
    assert_eq!(overridden.value(), "replacement");
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(
        result
            .view()
            .and_then(|view| view.service("malformed"))
            .is_some_and(|service| service.runtime().is_none())
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
fn retains_effective_cgroup_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3234);
    let override_id = SourceId::new(3235);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("base", "workspace"),
            "services:\n  replaced: {cgroup: host}\n  sensitive: {cgroup: \"${CGROUP}\"}\n  reset: {cgroup: host}\n  override: {cgroup: host}\n  invalid: {cgroup: none}\n  malformed: {cgroup: host}\n  omitted: {image: example/app}\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  replaced: {cgroup: private}\n  reset: {cgroup: !reset null}\n  override: {cgroup: !override private}\n  malformed: {cgroup: [private]}\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CGROUP", "host");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let cgroup = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::cgroup)
            .ok_or("effective cgroup")
    };
    let replaced = cgroup("replaced")?;
    assert!(matches!(
        replaced.value().kind(),
        compose_lens::model::CgroupNamespaceKind::Private
    ));
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    let sensitive = cgroup("sensitive")?;
    assert!(sensitive.is_sensitive());
    assert!(matches!(
        sensitive.value().kind(),
        compose_lens::model::CgroupNamespaceKind::Host
    ));
    assert!(!format!("{sensitive:?}").contains("host"));
    assert!(
        result
            .view()
            .and_then(|view| view.service("reset"))
            .is_some_and(|service| service.cgroup().is_none())
    );
    assert_eq!(cgroup("override")?.provenance().operation(), MergeOperation::Override);
    let invalid = cgroup("invalid")?;
    assert!(!invalid.value().is_valid());
    assert!(matches!(
        invalid.value().kind(),
        compose_lens::model::CgroupNamespaceKind::Other(value) if value == "none"
    ));
    let malformed = result
        .view()
        .and_then(|view| view.service("malformed"))
        .ok_or("malformed service")?;
    assert!(malformed.cgroup().is_none());
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "malformed", "cgroup"])
    );
    assert!(
        result
            .view()
            .and_then(|view| view.service("omitted"))
            .is_some_and(|service| service.cgroup().is_none())
    );
    for code in [PROJECT_EXPECTED_FORM, compose_lens::model::CGROUP_NAMESPACE_INVALID] {
        assert!(result.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_effective_cgroup_parent_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3244);
    let override_id = SourceId::new(3245);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("base", "workspace"),
            "services:\n  replaced: {cgroup_parent: base}\n  sensitive: {cgroup_parent: \"${PARENT}\"}\n  reset: {cgroup_parent: base}\n  override: {cgroup_parent: base}\n  malformed: {cgroup_parent: base}\n  omitted: {image: example/app}\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  replaced: {cgroup_parent: replacement}\n  reset: {cgroup_parent: !reset null}\n  override: {cgroup_parent: !override explicit}\n  malformed: {cgroup_parent: [parent]}\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("PARENT", "sensitive-parent");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let parent = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::cgroup_parent)
            .ok_or("effective cgroup parent")
    };
    let replaced = parent("replaced")?;
    assert_eq!(replaced.value(), "replacement");
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    let sensitive = parent("sensitive")?;
    assert_eq!(sensitive.value(), "sensitive-parent");
    assert!(sensitive.is_sensitive());
    assert!(!format!("{sensitive:?}").contains("sensitive-parent"));
    assert!(
        result
            .view()
            .and_then(|view| view.service("reset"))
            .is_some_and(|service| service.cgroup_parent().is_none())
    );
    assert_eq!(parent("override")?.provenance().operation(), MergeOperation::Override);
    let malformed = result
        .view()
        .and_then(|view| view.service("malformed"))
        .ok_or("malformed service")?;
    assert!(malformed.cgroup_parent().is_none());
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "malformed", "cgroup_parent"])
    );
    assert!(
        result
            .view()
            .and_then(|view| view.service("omitted"))
            .is_some_and(|service| service.cgroup_parent().is_none())
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
fn retains_effective_cpu_count_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3254);
    let override_id = SourceId::new(3255);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("base", "workspace"),
            "services:\n  replaced: {cpu_count: 1}\n  reset: {cpu_count: 1}\n  override: {cpu_count: 1}\n  sensitive: {cpu_count: \"${CPU_COUNT}\"}\n  block:\n    cpu_count: |-\n      007\n  negative: {cpu_count: -1}\n  malformed: {cpu_count: 1}\n  timestamp: {cpu_count: !!timestamp 2024-01-01}\n  regex: {cpu_count: !!regex '007'}\n  omitted: {image: example/app}\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  replaced: {cpu_count: 0xCA_FE}\n  reset: {cpu_count: !reset null}\n  override: {cpu_count: !override \"custom\"}\n  malformed: {cpu_count: [1]}\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CPU_COUNT", "private-cpu-count");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let count = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::cpu_count)
            .ok_or("effective cpu count")
    };
    let replaced = count("replaced")?;
    assert!(matches!(replaced.value(), compose_lens::model::CpuCount::YamlInteger(value) if value == "0xCA_FE"));
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    assert!(count("sensitive")?.is_sensitive());
    assert!(
        matches!(count("sensitive")?.value(), compose_lens::model::CpuCount::String(value) if value == "private-cpu-count")
    );
    assert!(!format!("{:?}", count("sensitive")?).contains("private-cpu-count"));
    assert!(matches!(count("block")?.value(), compose_lens::model::CpuCount::String(value) if value == "007"));
    assert!(
        result
            .view()
            .and_then(|view| view.service("reset"))
            .is_some_and(|service| service.cpu_count().is_none())
    );
    assert!(matches!(count("override")?.value(), compose_lens::model::CpuCount::String(value) if value == "custom"));
    assert!(
        matches!(count("negative")?.value(), compose_lens::model::CpuCount::NegativeYamlInteger(value) if value == "-1")
    );
    let malformed = result
        .view()
        .and_then(|view| view.service("malformed"))
        .ok_or("malformed service")?;
    assert!(malformed.cpu_count().is_none());
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "malformed", "cpu_count"])
    );
    for name in ["timestamp", "regex"] {
        let service = result
            .view()
            .and_then(|view| view.service(name))
            .ok_or("tagged service")?;
        assert!(service.cpu_count().is_none());
        assert!(
            service
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", name, "cpu_count"])
        );
    }
    assert!(
        result
            .view()
            .and_then(|view| view.service("omitted"))
            .is_some_and(|service| service.cpu_count().is_none())
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
            .count(),
        4
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == compose_lens::model::CPU_COUNT_NEGATIVE)
    );
    Ok(())
}

#[test]
fn retains_effective_cpu_percent_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3267);
    let override_id = SourceId::new(3268);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("base", "workspace"),
            "services:\n  replaced: {cpu_percent: 1}\n  reset: {cpu_percent: 1}\n  override: {cpu_percent: 1}\n  sensitive: {cpu_percent: \"${CPU_PERCENT}\"}\n  block:\n    cpu_percent: |-\n      101\n  over: {cpu_percent: 0x65}\n  malformed: {cpu_percent: 1}\n  timestamp: {cpu_percent: !!timestamp 2024-01-01}\n  regex: {cpu_percent: !!regex '101'}\n  tagged: {cpu_percent: !opaque 101}\n  omitted: {image: example/app}\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  replaced: {cpu_percent: 0x64}\n  reset: {cpu_percent: !reset null}\n  override: {cpu_percent: !override \"101\"}\n  malformed: {cpu_percent: [1]}\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CPU_PERCENT", "private-cpu-percent");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let percent = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::cpu_percent)
            .ok_or("effective cpu percent")
    };
    let replaced = percent("replaced")?;
    assert!(matches!(
        replaced.value(),
        compose_lens::model::CpuPercent::YamlInteger(value) if value == "0x64"
    ));
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    assert!(percent("sensitive")?.is_sensitive());
    assert!(matches!(
        percent("sensitive")?.value(),
        compose_lens::model::CpuPercent::String(value) if value == "private-cpu-percent"
    ));
    assert!(!format!("{:?}", percent("sensitive")?).contains("private-cpu-percent"));
    assert!(matches!(
        percent("block")?.value(),
        compose_lens::model::CpuPercent::String(value) if value == "101"
    ));
    assert!(matches!(
        percent("over")?.value(),
        compose_lens::model::CpuPercent::OutOfRangeYamlInteger(value) if value == "0x65"
    ));
    assert!(
        result
            .view()
            .and_then(|view| view.service("reset"))
            .is_some_and(|service| service.cpu_percent().is_none())
    );
    assert!(matches!(
        percent("override")?.value(),
        compose_lens::model::CpuPercent::String(value) if value == "101"
    ));
    for name in ["malformed", "timestamp", "regex", "tagged"] {
        let service = result
            .view()
            .and_then(|view| view.service(name))
            .ok_or("malformed service")?;
        assert!(service.cpu_percent().is_none());
        assert!(
            service
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", name, "cpu_percent"])
        );
    }
    assert!(
        result
            .view()
            .and_then(|view| view.service("omitted"))
            .is_some_and(|service| service.cpu_percent().is_none())
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == compose_lens::model::CPU_PERCENT_OUT_OF_RANGE)
    );
    Ok(())
}

#[test]
fn retains_effective_cpu_period_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3275);
    let override_id = SourceId::new(3276);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("base", "workspace"),
            "services:\n  replaced: {cpu_period: 1}\n  reset: {cpu_period: 1}\n  override: {cpu_period: 1}\n  sensitive: {cpu_period: \"${CPU_PERIOD}\"}\n  literal:\n    cpu_period: |-\n      1e6\n  folded:\n    cpu_period: >-\n      1e6\n  malformed: {cpu_period: 1}\n  tagged: {cpu_period: !opaque 1}\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  replaced: {cpu_period: 1e+6}\n  reset: {cpu_period: !reset null}\n  override: {cpu_period: !override \"opaque\"}\n  malformed: {cpu_period: [1]}\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CPU_PERIOD", "private-cpu-period");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let period = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::cpu_period)
            .ok_or("effective cpu period")
    };
    let replaced = period("replaced")?;
    assert!(matches!(
        replaced.value(),
        compose_lens::model::CpuPeriod::YamlNumber(value) if value == "1e+6"
    ));
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    assert!(period("sensitive")?.is_sensitive());
    assert!(matches!(
        period("sensitive")?.value(),
        compose_lens::model::CpuPeriod::String(value) if value == "private-cpu-period"
    ));
    assert!(!format!("{:?}", period("sensitive")?).contains("private-cpu-period"));
    for name in ["literal", "folded"] {
        assert!(matches!(
            period(name)?.value(),
            compose_lens::model::CpuPeriod::String(value) if value == "1e6"
        ));
    }
    assert!(
        result
            .view()
            .and_then(|view| view.service("reset"))
            .is_some_and(|service| service.cpu_period().is_none())
    );
    assert!(matches!(
        period("override")?.value(),
        compose_lens::model::CpuPeriod::String(value) if value == "opaque"
    ));
    for name in ["malformed", "tagged"] {
        let service = result
            .view()
            .and_then(|view| view.service(name))
            .ok_or("malformed service")?;
        assert!(service.cpu_period().is_none());
        assert!(
            service
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", name, "cpu_period"])
        );
    }
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

#[test]
fn retains_effective_cpu_quota_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3281);
    let override_id = SourceId::new(3282);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("base", "workspace"),
            "services:\n  replaced: {cpu_quota: 1}\n  reset: {cpu_quota: 1}\n  override: {cpu_quota: 1}\n  sensitive: {cpu_quota: \"${CPU_QUOTA}\"}\n  literal:\n    cpu_quota: |-\n      1e6\n  folded:\n    cpu_quota: >-\n      1e6\n  malformed: {cpu_quota: 1}\n  tagged: {cpu_quota: !opaque 1}\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  replaced: {cpu_quota: 1e+6}\n  reset: {cpu_quota: !reset null}\n  override: {cpu_quota: !override \"opaque\"}\n  malformed: {cpu_quota: [1]}\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CPU_QUOTA", "private-cpu-quota");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let quota = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::cpu_quota)
            .ok_or("effective cpu quota")
    };
    let replaced = quota("replaced")?;
    assert!(matches!(
        replaced.value(),
        compose_lens::model::CpuQuota::YamlNumber(value) if value == "1e+6"
    ));
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    assert!(quota("sensitive")?.is_sensitive());
    assert!(matches!(
        quota("sensitive")?.value(),
        compose_lens::model::CpuQuota::String(value) if value == "private-cpu-quota"
    ));
    assert!(!format!("{:?}", quota("sensitive")?).contains("private-cpu-quota"));
    for name in ["literal", "folded"] {
        assert!(matches!(
            quota(name)?.value(),
            compose_lens::model::CpuQuota::String(value) if value == "1e6"
        ));
    }
    assert!(
        result
            .view()
            .and_then(|view| view.service("reset"))
            .is_some_and(|service| service.cpu_quota().is_none())
    );
    assert!(matches!(
        quota("override")?.value(),
        compose_lens::model::CpuQuota::String(value) if value == "opaque"
    ));
    for name in ["malformed", "tagged"] {
        let service = result
            .view()
            .and_then(|view| view.service(name))
            .ok_or("malformed service")?;
        assert!(service.cpu_quota().is_none());
        assert!(
            service
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", name, "cpu_quota"])
        );
    }
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

#[test]
fn retains_effective_cpu_rt_period_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3289);
    let override_id = SourceId::new(3290);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("base", "workspace"),
            "services:\n  replaced: {cpu_rt_period: 1s}\n  reset: {cpu_rt_period: 1s}\n  override: {cpu_rt_period: 1s}\n  sensitive: {cpu_rt_period: \"${CPU_RT_PERIOD}\"}\n  literal:\n    cpu_rt_period: |-\n      1m30s\n  other: {cpu_rt_period: 1ns}\n  malformed: {cpu_rt_period: 1s}\n  tagged: {cpu_rt_period: !opaque 1s}\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  replaced: {cpu_rt_period: 1.5s}\n  reset: {cpu_rt_period: !reset null}\n  override: {cpu_rt_period: !override \"opaque\"}\n  malformed: {cpu_rt_period: [1]}\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CPU_RT_PERIOD", "1m30s");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let period = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::cpu_rt_period)
            .ok_or("effective cpu rt period")
    };
    let replaced = period("replaced")?;
    assert!(matches!(
        replaced.value(),
        compose_lens::model::CpuRtPeriod::Duration(value) if value == "1.5s"
    ));
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    assert!(period("sensitive")?.is_sensitive());
    assert!(matches!(
        period("sensitive")?.value(),
        compose_lens::model::CpuRtPeriod::Duration(value) if value == "1m30s"
    ));
    assert!(!format!("{:?}", period("sensitive")?).contains("1m30s"));
    assert!(matches!(
        period("literal")?.value(),
        compose_lens::model::CpuRtPeriod::Duration(value) if value == "1m30s"
    ));
    assert!(matches!(
        period("other")?.value(),
        compose_lens::model::CpuRtPeriod::Other(value) if value == "1ns"
    ));
    assert!(
        result
            .view()
            .and_then(|view| view.service("reset"))
            .is_some_and(|service| service.cpu_rt_period().is_none())
    );
    assert!(matches!(
        period("override")?.value(),
        compose_lens::model::CpuRtPeriod::Other(value) if value == "opaque"
    ));
    for name in ["malformed", "tagged"] {
        let service = result
            .view()
            .and_then(|view| view.service(name))
            .ok_or("malformed service")?;
        assert!(service.cpu_rt_period().is_none());
        assert!(
            service
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", name, "cpu_rt_period"])
        );
    }
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_INVALID_VALUE)
    );
    Ok(())
}

#[test]
fn retains_effective_deploy_restart_policy_member_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let base = SourceId::new(2902);
    let override_id = SourceId::new(2903);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base,
            DocumentOrigin::new("compose.yaml", "workspace"),
            "services:\n  app:\n    deploy:\n      restart_policy: {condition: any, delay: 1s, max_attempts: 003, window: 1m}\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override.yaml", "workspace"),
            "services:\n  app:\n    deploy:\n      restart_policy: {condition: on-failure, max_attempts: !reset null}\n",
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let policy = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::deploy)
        .and_then(|deploy| deploy.value().restart_policy())
        .ok_or("effective restart policy")?;
    assert!(matches!(
        policy.value().condition().map(ProjectValue::value),
        Some(DeployRestartCondition::OnFailure)
    ));
    assert_eq!(
        policy.value().condition().map(|value| value.provenance().operation()),
        Some(MergeOperation::Replaced)
    );
    assert_eq!(policy.value().delay().map(|value| value.value().raw()), Some("1s"));
    assert!(policy.value().max_attempts().is_none());
    assert!(policy.value().unmodeled_fields().iter().any(|field| field.path()
        == ["services", "app", "deploy", "restart_policy", "max_attempts"]
        && field.provenance().operation() == MergeOperation::Reset));
    Ok(())
}

#[test]
fn retains_effective_deploy_restart_policy_reset_override_invalid_and_sensitive_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let base = SourceId::new(2906);
    let override_id = SourceId::new(2907);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  reset:\n    deploy: {restart_policy: {condition: any, delay: 1s}}\n",
                "  overridden:\n    deploy: {restart_policy: {condition: any, delay: 1s}}\n",
                "  sensitive:\n    deploy: {restart_policy: {delay: \"${RESTART_DELAY}\"}}\n",
                "  malformed:\n    deploy: {restart_policy: {condition: any, max_attempts: 1.5, delay: []}}\n",
                "  retained:\n    deploy: {restart_policy: {x-map: {nested: value}, future: [value]}}\n",
                "  independent:\n    restart: always\n    deploy: {restart_policy: {condition: none}}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  reset:\n    deploy: {restart_policy: !reset {}}\n",
                "  overridden:\n    deploy: {restart_policy: !override {condition: on-failure, max_attempts: 003}}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("RESTART_DELAY", "private-delay");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;
    let policy = |service| {
        view.service(service)
            .and_then(ProjectService::deploy)
            .and_then(|deploy| deploy.value().restart_policy())
            .ok_or("effective deploy restart policy expected")
    };

    let reset = policy("reset")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.value().condition().is_none() && reset.value().delay().is_none());

    let overridden = policy("overridden")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(
        overridden.value().condition().map(ProjectValue::value),
        Some(DeployRestartCondition::OnFailure)
    ));
    assert!(matches!(
        overridden.value().max_attempts().map(ProjectValue::value),
        Some(compose_lens::model::DeployRestartMaxAttempts::YamlNumber(value)) if value == "003"
    ));

    let sensitive = policy("sensitive")?.value().delay().ok_or("sensitive delay expected")?;
    assert!(sensitive.is_sensitive());
    assert_eq!(sensitive.value().raw(), "private-delay");
    assert!(!format!("{sensitive:?}").contains("private-delay"));

    let malformed = policy("malformed")?.value();
    assert!(malformed.max_attempts().is_none() && malformed.delay().is_none());
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| { field.path() == ["services", "malformed", "deploy", "restart_policy", "max_attempts"] })
    );
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| { field.path() == ["services", "malformed", "deploy", "restart_policy", "delay"] })
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );

    let retained = policy("retained")?.value().unmodeled_fields();
    assert!(retained.iter().any(|field| {
        field.path() == ["services", "retained", "deploy", "restart_policy", "x-map"] && field.is_extension()
    }));
    assert!(retained.iter().any(|field| {
        field.path() == ["services", "retained", "deploy", "restart_policy", "future"] && !field.is_extension()
    }));

    let independent = view.service("independent").ok_or("independent service expected")?;
    assert!(matches!(
        independent
            .restart()
            .map(ProjectValue::value)
            .map(compose_lens::model::RestartPolicy::kind),
        Some(RestartPolicyKind::Always)
    ));
    assert!(matches!(
        policy("independent")?.value().condition().map(ProjectValue::value),
        Some(DeployRestartCondition::None)
    ));
    Ok(())
}

fn deploy_placement_project_view() -> Result<compose_lens::project::ProjectViewResult, Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3002);
    let override_id = SourceId::new(3003);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  merged:\n    deploy:\n      mode: global\n      placement:\n        constraints: [zone=east, \"\", zone=east]\n        preferences: [{spread: rack}, {}]\n        max_replicas_per_node: 003\n",
                "  child-reset:\n    deploy: {placement: {constraints: [old], preferences: [{spread: old}], max_replicas_per_node: 2}}\n",
                "  whole-reset:\n    deploy: {placement: {constraints: [old]}}\n",
                "  whole-override:\n    deploy: {placement: {constraints: [old], max_replicas_per_node: 2}}\n",
                "  sensitive:\n    deploy: {placement: {constraints: [\"${PLACEMENT_CONSTRAINT}\"], preferences: [{spread: \"${PLACEMENT_SPREAD}\"}], max_replicas_per_node: \"${PLACEMENT_MAX}\"}}\n",
                "  malformed:\n    deploy: {placement: {constraints: [valid, 1.5, {bad: value}, later], preferences: [{spread: 1}, [], {future: retained}], max_replicas_per_node: 1.5, x-retained: {nested: value}}}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  merged:\n    deploy: {placement: {constraints: [zone=west, zone=east], preferences: [{spread: zone}, {spread: zone}], max_replicas_per_node: 004}}\n",
                "  child-reset:\n    deploy: {placement: {constraints: !reset [], preferences: !reset [], max_replicas_per_node: !reset null}}\n",
                "  whole-reset:\n    deploy: {placement: !reset {}}\n",
                "  whole-override:\n    deploy: {placement: !override {constraints: [only], max_replicas_per_node: \"three\"}}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("PLACEMENT_CONSTRAINT", "private-constraint");
    let _ = environment.insert_sensitive("PLACEMENT_SPREAD", "private-spread");
    let _ = environment.insert_sensitive("PLACEMENT_MAX", "private-max");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    Ok(build_project_view(
        merged.project().ok_or("merged project expected")?,
        None,
    ))
}

fn effective_deploy_placement<'view>(
    view: &'view ProjectView,
    service: &str,
) -> Result<&'view ProjectValue<compose_lens::project::ProjectDeployPlacement>, Box<dyn std::error::Error>> {
    view.service(service)
        .and_then(ProjectService::deploy)
        .and_then(|deploy| deploy.value().placement())
        .ok_or_else(|| "effective deploy placement expected".into())
}

#[test]
fn retains_effective_deploy_placement_append_and_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_placement_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let merged = effective_deploy_placement(view, "merged")?;

    assert!(matches!(
        view.service("merged")
            .and_then(ProjectService::deploy)
            .and_then(|deploy| deploy.value().mode())
            .map(ProjectValue::value),
        Some(DeployMode::Global)
    ));
    let constraints = merged.value().constraints().ok_or("merged constraints expected")?;
    assert_eq!(constraints.provenance().operation(), MergeOperation::Appended);
    assert_source_ids(
        constraints.provenance().sources(),
        &[SourceId::new(3002), SourceId::new(3003)],
    );
    assert_eq!(
        constraints
            .value()
            .iter()
            .map(ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["zone=east", "", "zone=east", "zone=west", "zone=east"]
    );
    assert_eq!(
        constraints.value()[0].provenance().operation(),
        MergeOperation::Authored
    );
    assert_eq!(constraints.value()[4].provenance().operation(), MergeOperation::Added);
    let preferences = merged.value().preferences().ok_or("merged preferences expected")?;
    assert_eq!(preferences.provenance().operation(), MergeOperation::Appended);
    assert_eq!(preferences.value().len(), 4);
    assert_eq!(
        preferences.value()[0].provenance().operation(),
        MergeOperation::Authored
    );
    assert_eq!(preferences.value()[3].provenance().operation(), MergeOperation::Added);
    assert_eq!(
        preferences.value()[2]
            .value()
            .spread()
            .map(ProjectValue::value)
            .map(String::as_str),
        Some("zone")
    );
    let maximum = merged
        .value()
        .max_replicas_per_node()
        .ok_or("merged maximum expected")?;
    assert!(matches!(maximum.value(), DeployPlacementMaxReplicasPerNode::YamlInteger(value) if value == "004"));
    assert_eq!(maximum.provenance().operation(), MergeOperation::Replaced);
    Ok(())
}

#[test]
fn retains_effective_deploy_placement_reset_and_override() -> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_placement_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let child_reset = effective_deploy_placement(view, "child-reset")?.value();
    assert!(
        matches!(child_reset.constraints(), Some(value) if value.value().is_empty() && value.provenance().operation() == MergeOperation::Reset)
    );
    assert!(
        matches!(child_reset.preferences(), Some(value) if value.value().is_empty() && value.provenance().operation() == MergeOperation::Reset)
    );
    assert!(child_reset.max_replicas_per_node().is_none());
    assert!(child_reset.unmodeled_fields().iter().any(|field| {
        field.path()
            == [
                "services",
                "child-reset",
                "deploy",
                "placement",
                "max_replicas_per_node",
            ]
            && field.provenance().operation() == MergeOperation::Reset
    }));

    let whole_reset = effective_deploy_placement(view, "whole-reset")?;
    assert_eq!(whole_reset.provenance().operation(), MergeOperation::Reset);
    assert!(whole_reset.value().constraints().is_none() && whole_reset.value().preferences().is_none());
    let whole_override = effective_deploy_placement(view, "whole-override")?;
    assert_eq!(whole_override.provenance().operation(), MergeOperation::Override);
    assert!(matches!(
        whole_override.value().constraints().map(ProjectValue::value),
        Some(values) if values.len() == 1 && values[0].value() == "only"
    ));
    assert!(matches!(
        whole_override.value().max_replicas_per_node().map(ProjectValue::value),
        Some(DeployPlacementMaxReplicasPerNode::String(value)) if value == "three"
    ));
    Ok(())
}

#[test]
fn retains_effective_deploy_placement_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_placement_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let sensitive = effective_deploy_placement(view, "sensitive")?.value();
    let constraint = &sensitive.constraints().ok_or("sensitive constraints expected")?.value()[0];
    let spread = sensitive.preferences().ok_or("sensitive preferences expected")?.value()[0]
        .value()
        .spread()
        .ok_or("sensitive spread expected")?;
    let maximum = sensitive.max_replicas_per_node().ok_or("sensitive maximum expected")?;
    assert!(constraint.is_sensitive() && spread.is_sensitive() && maximum.is_sensitive());
    for value in [format!("{constraint:?}"), format!("{spread:?}"), format!("{maximum:?}")] {
        for secret in ["private-constraint", "private-spread", "private-max"] {
            assert!(!value.contains(secret), "sensitive placement value leaked: {secret}");
        }
    }

    let malformed = effective_deploy_placement(view, "malformed")?.value();
    assert!(matches!(malformed.constraints().map(ProjectValue::value), Some(values)
        if values.iter().map(ProjectValue::value).map(String::as_str).collect::<Vec<_>>() == ["valid", "later"]));
    assert!(malformed.max_replicas_per_node().is_none());
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| { field.path() == ["services", "malformed", "deploy", "placement", "constraints"] })
    );
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| { field.path() == ["services", "malformed", "deploy", "placement", "preferences"] })
    );
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| { field.path() == ["services", "malformed", "deploy", "placement", "max_replicas_per_node"] })
    );
    assert!(malformed.unmodeled_fields().iter().any(|field| {
        field.path() == ["services", "malformed", "deploy", "placement", "x-retained"] && field.is_extension()
    }));
    let preferences = malformed.preferences().ok_or("partial preferences expected")?;
    assert!(preferences.value()[0].value().unmodeled_fields().iter().any(|field| {
        field.path()
            == [
                "services",
                "malformed",
                "deploy",
                "placement",
                "preferences",
                "0",
                "spread",
            ]
    }));
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

fn deploy_resource_pids_project_view() -> Result<compose_lens::project::ProjectViewResult, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(3102),
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  merged:\n    deploy:\n      resources:\n        limits:\n          pids: 003\n          x-limit: kept\n          future-limit: kept\n        x-resources: kept\n        future-resources: kept\n",
                "  leaf-reset:\n    deploy: {resources: {limits: {pids: 2}}}\n",
                "  limits-reset:\n    deploy: {resources: {limits: {pids: 2}}}\n",
                "  resources-reset:\n    deploy: {resources: {limits: {pids: 2}}}\n",
                "  resources-override:\n    deploy: {resources: {limits: {pids: 2}}}\n",
                "  sensitive:\n    pids_limit: -1\n    deploy: {resources: {limits: {pids: \"${DEPLOY_PIDS}\"}}}\n",
                "  malformed:\n    deploy: {resources: {limits: {pids: 1.5, x-limit: kept}, x-resources: kept}}\n",
                "  cpu-merged:\n    mem_limit: 512m\n    deploy:\n      resources:\n        limits:\n          cpus: 0.50\n          x-limit: kept\n          future-limit: kept\n        x-resources: kept\n        future-resources: kept\n",
                "  cpu-leaf-reset:\n    deploy: {resources: {limits: {cpus: 2}}}\n",
                "  cpu-limits-reset:\n    deploy: {resources: {limits: {cpus: 2}}}\n",
                "  cpu-resources-reset:\n    deploy: {resources: {limits: {cpus: 2}}}\n",
                "  cpu-resources-override:\n    deploy: {resources: {limits: {cpus: 2}}}\n",
                "  cpu-sensitive:\n    deploy: {resources: {limits: {cpus: \"${DEPLOY_CPUS}\"}}}\n",
                "  cpu-malformed:\n    deploy: {resources: {limits: {cpus: true}}}\n",
                "  cpu-malformed-map:\n    deploy: {resources: {limits: {cpus: {bad: value}}}}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(3103),
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  merged:\n    deploy: {resources: {limits: {pids: 004}}}\n",
                "  leaf-reset:\n    deploy: {resources: {limits: {pids: !reset null}}}\n",
                "  limits-reset:\n    deploy: {resources: {limits: !reset {}}}\n",
                "  resources-reset:\n    deploy: {resources: !reset {}}\n",
                "  resources-override:\n    deploy: {resources: !override {limits: {pids: \"six\"}}}\n",
                "  cpu-merged:\n    deploy: {resources: {limits: {cpus: 1e-3}}}\n",
                "  cpu-leaf-reset:\n    deploy: {resources: {limits: {cpus: !reset null}}}\n",
                "  cpu-limits-reset:\n    deploy: {resources: {limits: !reset {}}}\n",
                "  cpu-resources-reset:\n    deploy: {resources: !reset {}}\n",
                "  cpu-resources-override:\n    deploy: {resources: !override {limits: {cpus: \"one\"}}}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("DEPLOY_PIDS", "private-deploy-pids");
    let _ = environment.insert_sensitive("DEPLOY_CPUS", "private-deploy-cpus");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    Ok(build_project_view(
        merged.project().ok_or("merged project expected")?,
        None,
    ))
}

fn effective_deploy_resources<'view>(
    view: &'view ProjectView,
    service: &str,
) -> Result<&'view ProjectValue<compose_lens::project::ProjectDeployResources>, Box<dyn std::error::Error>> {
    view.service(service)
        .and_then(ProjectService::deploy)
        .and_then(|deploy| deploy.value().resources())
        .ok_or_else(|| "effective deploy resources expected".into())
}

fn deploy_resource_memory_project_view() -> Result<compose_lens::project::ProjectViewResult, Box<dyn std::error::Error>>
{
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(3106),
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  merged:\n    mem_limit: \"512m\"\n    deploy:\n      resources:\n        limits:\n          memory: \"50m\"\n          x-limit: kept\n          future-limit: kept\n        reservations: {memory: \"1g\"}\n        x-resources: kept\n        future-resources: kept\n",
                "  zero:\n    deploy: {resources: {limits: {memory: \"000mb\"}}}\n",
                "  leaf-reset:\n    deploy: {resources: {limits: {memory: \"2m\"}}}\n",
                "  limits-reset:\n    deploy: {resources: {limits: {memory: \"2m\"}}}\n",
                "  resources-reset:\n    deploy: {resources: {limits: {memory: \"2m\"}}}\n",
                "  resources-override:\n    deploy: {resources: {limits: {memory: \"2m\"}}}\n",
                "  sensitive:\n    deploy: {resources: {limits: {memory: \"${DEPLOY_MEMORY}\"}}}\n",
                "  malformed-number:\n    deploy: {resources: {limits: {memory: 64}}}\n",
                "  malformed-map:\n    deploy: {resources: {limits: {memory: {bad: value}}}}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(3107),
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  merged:\n    deploy: {resources: {limits: {memory: \"001kb\"}}}\n",
                "  leaf-reset:\n    deploy: {resources: {limits: {memory: !reset null}}}\n",
                "  limits-reset:\n    deploy: {resources: {limits: !reset {}}}\n",
                "  resources-reset:\n    deploy: {resources: !reset {}}\n",
                "  resources-override:\n    deploy: {resources: !override {limits: {memory: \"64\"}}}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("DEPLOY_MEMORY", "private-deploy-memory");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    Ok(build_project_view(
        merged.project().ok_or("merged project expected")?,
        None,
    ))
}

fn deploy_resource_reservation_project_view()
-> Result<compose_lens::project::ProjectViewResult, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(3110),
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  merged:\n    cpus: 3\n    mem_limit: \"100m\"\n    deploy:\n      resources:\n        limits: {cpus: 9, memory: \"99m\"}\n        reservations:\n          cpus: 0.50\n          memory: \"50m\"\n          x-reservation: kept\n          future: kept\n        x-resources: kept\n        later: kept\n",
                "  leaf-reset:\n    deploy: {resources: {reservations: {cpus: 2, memory: \"2m\"}}}\n",
                "  reservations-reset:\n    deploy: {resources: {reservations: {cpus: 2, memory: \"2m\"}}}\n",
                "  resources-reset:\n    deploy: {resources: {reservations: {cpus: 2, memory: \"2m\"}}}\n",
                "  override:\n    deploy: {resources: {reservations: {cpus: 2, memory: \"2m\"}}}\n",
                "  sensitive:\n    deploy: {resources: {reservations: {cpus: \"${RESERVED_CPUS}\", memory: \"${RESERVED_MEMORY}\"}}}\n",
                "  malformed-bool:\n    deploy: {resources: {reservations: {cpus: true}}}\n",
                "  malformed-map:\n    deploy: {resources: {reservations: {cpus: {bad: value}}}}\n",
                "  memory-malformed-number:\n    deploy: {resources: {reservations: {memory: 64}}}\n",
                "  memory-malformed-map:\n    deploy: {resources: {reservations: {memory: {bad: value}}}}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(3111),
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  merged:\n    deploy: {resources: {reservations: {cpus: 1e-3, memory: \"001kb\"}}}\n",
                "  leaf-reset:\n    deploy: {resources: {reservations: {cpus: !reset null, memory: !reset null}}}\n",
                "  reservations-reset:\n    deploy: {resources: {reservations: !reset {}}}\n",
                "  resources-reset:\n    deploy: {resources: !reset {}}\n",
                "  override:\n    deploy: {resources: !override {reservations: {cpus: \"one\", memory: \"64\"}}}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("RESERVED_CPUS", "private-reserved-cpus");
    let _ = environment.insert_sensitive("RESERVED_MEMORY", "private-reserved-memory");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    Ok(build_project_view(
        merged.project().ok_or("merged project expected")?,
        None,
    ))
}

#[test]
fn retains_effective_deploy_resource_pids_nested_merge_and_service_independence()
-> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_resource_pids_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let resources = effective_deploy_resources(view, "merged")?;
    assert_eq!(resources.provenance().operation(), MergeOperation::Merged);
    assert_source_ids(
        resources.provenance().sources(),
        &[SourceId::new(3102), SourceId::new(3103)],
    );
    let limits = resources.value().limits().ok_or("limits expected")?;
    assert_eq!(limits.provenance().operation(), MergeOperation::Merged);
    let pids = limits.value().pids().ok_or("pids expected")?;
    assert!(matches!(pids.value(), DeployResourcePids::YamlInteger(value) if value == "004"));
    assert_eq!(pids.provenance().operation(), MergeOperation::Replaced);
    assert!(
        resources
            .value()
            .unmodeled_fields()
            .iter()
            .any(ProjectFieldReference::is_extension)
    );
    assert!(
        resources
            .value()
            .unmodeled_fields()
            .iter()
            .any(|field| { field.path() == ["services", "merged", "deploy", "resources", "future-resources"] })
    );
    assert!(
        limits
            .value()
            .unmodeled_fields()
            .iter()
            .any(ProjectFieldReference::is_extension)
    );
    assert!(
        limits
            .value()
            .unmodeled_fields()
            .iter()
            .any(|field| { field.path() == ["services", "merged", "deploy", "resources", "limits", "future-limit"] })
    );
    let service_pids = view
        .service("sensitive")
        .and_then(ProjectService::pids_limit)
        .ok_or("service pids_limit expected")?;
    assert!(matches!(service_pids.value().kind(), PidsLimitKind::Unlimited));
    Ok(())
}

#[test]
fn retains_effective_deploy_resource_pids_reset_and_override_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_resource_pids_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let leaf_limits = effective_deploy_resources(view, "leaf-reset")?
        .value()
        .limits()
        .ok_or("leaf limits expected")?;
    assert!(leaf_limits.value().pids().is_none());
    assert!(leaf_limits.value().unmodeled_fields().iter().any(|field| {
        field.path() == ["services", "leaf-reset", "deploy", "resources", "limits", "pids"]
            && field.provenance().operation() == MergeOperation::Reset
    }));
    let limits_reset = effective_deploy_resources(view, "limits-reset")?
        .value()
        .limits()
        .ok_or("reset limits expected")?;
    assert!(limits_reset.value().pids().is_none());
    assert_eq!(limits_reset.provenance().operation(), MergeOperation::Reset);
    let resources_reset = effective_deploy_resources(view, "resources-reset")?;
    assert!(resources_reset.value().limits().is_none());
    assert_eq!(resources_reset.provenance().operation(), MergeOperation::Reset);
    let overridden = effective_deploy_resources(view, "resources-override")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    let pids = overridden
        .value()
        .limits()
        .and_then(|limits| limits.value().pids())
        .ok_or("overridden pids expected")?;
    assert!(matches!(pids.value(), DeployResourcePids::String(value) if value == "six"));
    Ok(())
}

#[test]
fn retains_effective_deploy_resource_pids_sensitivity_and_malformed_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let result = deploy_resource_pids_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let sensitive = effective_deploy_resources(view, "sensitive")?
        .value()
        .limits()
        .and_then(|limits| limits.value().pids())
        .ok_or("sensitive deploy pids expected")?;
    assert!(sensitive.is_sensitive());
    assert!(!format!("{sensitive:?}").contains("private-deploy-pids"));
    let malformed = effective_deploy_resources(view, "malformed")?
        .value()
        .limits()
        .ok_or("malformed limits expected")?;
    assert!(malformed.value().pids().is_none());
    assert!(
        malformed
            .value()
            .unmodeled_fields()
            .iter()
            .any(|field| { field.path() == ["services", "malformed", "deploy", "resources", "limits", "pids"] })
    );
    assert!(
        malformed
            .value()
            .unmodeled_fields()
            .iter()
            .any(ProjectFieldReference::is_extension)
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
fn retains_effective_deploy_resource_cpus_nested_merge_and_independence() -> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_resource_pids_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let resources = effective_deploy_resources(view, "cpu-merged")?;
    assert_eq!(resources.provenance().operation(), MergeOperation::Merged);
    let limits = resources.value().limits().ok_or("limits expected")?;
    assert_eq!(limits.provenance().operation(), MergeOperation::Merged);
    let cpus = limits.value().cpus().ok_or("cpus expected")?;
    assert!(matches!(cpus.value(), DeployResourceCpus::YamlNumber(value) if value == "1e-3"));
    assert_eq!(cpus.provenance().operation(), MergeOperation::Replaced);
    assert!(
        resources
            .value()
            .unmodeled_fields()
            .iter()
            .any(ProjectFieldReference::is_extension)
    );
    assert!(
        resources
            .value()
            .unmodeled_fields()
            .iter()
            .any(|field| { field.path() == ["services", "cpu-merged", "deploy", "resources", "future-resources"] })
    );
    assert!(
        limits
            .value()
            .unmodeled_fields()
            .iter()
            .any(ProjectFieldReference::is_extension)
    );
    assert!(limits.value().unmodeled_fields().iter().any(|field| {
        field.path()
            == [
                "services",
                "cpu-merged",
                "deploy",
                "resources",
                "limits",
                "future-limit",
            ]
    }));
    assert!(view.service("cpu-merged").and_then(ProjectService::mem_limit).is_some());
    Ok(())
}

#[test]
fn retains_effective_deploy_resource_cpus_reset_and_override_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_resource_pids_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let leaf_limits = effective_deploy_resources(view, "cpu-leaf-reset")?
        .value()
        .limits()
        .ok_or("leaf limits expected")?;
    assert!(leaf_limits.value().cpus().is_none());
    assert!(leaf_limits.value().unmodeled_fields().iter().any(|field| {
        field.path() == ["services", "cpu-leaf-reset", "deploy", "resources", "limits", "cpus"]
            && field.provenance().operation() == MergeOperation::Reset
    }));
    let limits_reset = effective_deploy_resources(view, "cpu-limits-reset")?
        .value()
        .limits()
        .ok_or("reset limits expected")?;
    assert!(limits_reset.value().cpus().is_none());
    assert_eq!(limits_reset.provenance().operation(), MergeOperation::Reset);
    let resources_reset = effective_deploy_resources(view, "cpu-resources-reset")?;
    assert!(resources_reset.value().limits().is_none());
    assert_eq!(resources_reset.provenance().operation(), MergeOperation::Reset);
    let overridden = effective_deploy_resources(view, "cpu-resources-override")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    let cpus = overridden
        .value()
        .limits()
        .and_then(|limits| limits.value().cpus())
        .ok_or("overridden cpus expected")?;
    assert!(matches!(cpus.value(), DeployResourceCpus::String(value) if value == "one"));
    Ok(())
}

#[test]
fn retains_effective_deploy_resource_cpus_sensitivity_and_malformed_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let result = deploy_resource_pids_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let sensitive = effective_deploy_resources(view, "cpu-sensitive")?
        .value()
        .limits()
        .and_then(|limits| limits.value().cpus())
        .ok_or("sensitive deploy cpus expected")?;
    assert!(sensitive.is_sensitive());
    assert!(!format!("{sensitive:?}").contains("private-deploy-cpus"));
    for service in ["cpu-malformed", "cpu-malformed-map"] {
        let limits = effective_deploy_resources(view, service)?
            .value()
            .limits()
            .ok_or("malformed limits expected")?;
        assert!(limits.value().cpus().is_none());
        assert!(
            limits
                .value()
                .unmodeled_fields()
                .iter()
                .any(|field| { field.path() == ["services", service, "deploy", "resources", "limits", "cpus"] })
        );
    }
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

#[test]
fn retains_effective_deploy_resource_memory_merge_and_independence() -> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_resource_memory_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let resources = effective_deploy_resources(view, "merged")?;
    assert_eq!(resources.provenance().operation(), MergeOperation::Merged);
    let limits = resources.value().limits().ok_or("limits expected")?;
    assert_eq!(limits.provenance().operation(), MergeOperation::Merged);
    let memory = limits.value().memory().ok_or("memory expected")?;
    assert_eq!(memory.value().raw(), "001kb");
    assert!(matches!(
        memory.value().kind(),
        DeployResourceMemoryKind::Documented { amount_raw, unit: DeployResourceMemoryUnit::Kb } if amount_raw == "001"
    ));
    assert_eq!(memory.provenance().operation(), MergeOperation::Replaced);
    assert!(
        resources
            .value()
            .unmodeled_fields()
            .iter()
            .any(ProjectFieldReference::is_extension)
    );
    assert!(matches!(
        resources
            .value()
            .reservations()
            .and_then(|reservations| reservations.value().memory())
            .map(ProjectValue::value)
            .map(compose_lens::model::DeployResourceMemory::kind),
        Some(DeployResourceMemoryKind::Documented { amount_raw, unit: DeployResourceMemoryUnit::G }) if amount_raw == "1"
    ));
    assert!(
        limits
            .value()
            .unmodeled_fields()
            .iter()
            .any(ProjectFieldReference::is_extension)
    );
    assert!(view.service("merged").and_then(ProjectService::mem_limit).is_some());
    let zero = effective_deploy_resources(view, "zero")?
        .value()
        .limits()
        .and_then(|limits| limits.value().memory());
    assert!(matches!(
        zero.map(ProjectValue::value).map(compose_lens::model::DeployResourceMemory::kind),
        Some(DeployResourceMemoryKind::Zero { amount_raw, unit: Some(DeployResourceMemoryUnit::Mb) }) if amount_raw == "000"
    ));
    Ok(())
}

#[test]
fn retains_effective_deploy_resource_memory_reset_and_override_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_resource_memory_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let leaf_limits = effective_deploy_resources(view, "leaf-reset")?
        .value()
        .limits()
        .ok_or("leaf limits expected")?;
    assert!(leaf_limits.value().memory().is_none());
    assert!(leaf_limits.value().unmodeled_fields().iter().any(|field| {
        field.path() == ["services", "leaf-reset", "deploy", "resources", "limits", "memory"]
            && field.provenance().operation() == MergeOperation::Reset
    }));
    let limits_reset = effective_deploy_resources(view, "limits-reset")?
        .value()
        .limits()
        .ok_or("limits reset")?;
    assert!(limits_reset.value().memory().is_none());
    assert_eq!(limits_reset.provenance().operation(), MergeOperation::Reset);
    let resources_reset = effective_deploy_resources(view, "resources-reset")?;
    assert!(resources_reset.value().limits().is_none());
    assert_eq!(resources_reset.provenance().operation(), MergeOperation::Reset);
    let overridden = effective_deploy_resources(view, "resources-override")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(
        overridden
            .value()
            .limits()
            .and_then(|limits| limits.value().memory())
            .map(ProjectValue::value)
            .map(compose_lens::model::DeployResourceMemory::kind),
        Some(DeployResourceMemoryKind::ProviderDependentString)
    ));
    Ok(())
}

#[test]
fn retains_effective_deploy_resource_memory_sensitivity_and_malformed_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_resource_memory_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let sensitive = effective_deploy_resources(view, "sensitive")?
        .value()
        .limits()
        .and_then(|limits| limits.value().memory())
        .ok_or("sensitive memory expected")?;
    assert!(sensitive.is_sensitive());
    assert!(!format!("{sensitive:?}").contains("private-deploy-memory"));
    for service in ["malformed-number", "malformed-map"] {
        let limits = effective_deploy_resources(view, service)?
            .value()
            .limits()
            .ok_or("malformed limits expected")?;
        assert!(limits.value().memory().is_none());
        assert!(
            limits
                .value()
                .unmodeled_fields()
                .iter()
                .any(|field| { field.path() == ["services", service, "deploy", "resources", "limits", "memory"] })
        );
    }
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

#[test]
fn retains_effective_deploy_resource_reservation_cpu_merge_and_independence() -> Result<(), Box<dyn std::error::Error>>
{
    let result = deploy_resource_reservation_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let resources = effective_deploy_resources(view, "merged")?;
    assert_eq!(resources.provenance().operation(), MergeOperation::Merged);
    let reservations = resources.value().reservations().ok_or("reservations expected")?;
    assert_eq!(reservations.provenance().operation(), MergeOperation::Merged);
    let cpus = reservations.value().cpus().ok_or("reservation CPUs expected")?;
    assert!(matches!(cpus.value(), DeployResourceCpus::YamlNumber(value) if value == "1e-3"));
    assert_eq!(cpus.provenance().operation(), MergeOperation::Replaced);
    assert!(
        resources
            .value()
            .unmodeled_fields()
            .iter()
            .any(ProjectFieldReference::is_extension)
    );
    assert!(
        resources
            .value()
            .unmodeled_fields()
            .iter()
            .any(|field| { field.path() == ["services", "merged", "deploy", "resources", "later"] })
    );
    assert!(
        reservations
            .value()
            .unmodeled_fields()
            .iter()
            .any(ProjectFieldReference::is_extension)
    );
    assert!(
        reservations
            .value()
            .unmodeled_fields()
            .iter()
            .any(|field| { field.path() == ["services", "merged", "deploy", "resources", "reservations", "future"] })
    );
    let limits = resources
        .value()
        .limits()
        .and_then(|limits| limits.value().cpus())
        .ok_or("limit CPUs expected")?;
    assert!(matches!(limits.value(), DeployResourceCpus::YamlNumber(value) if value == "9"));
    Ok(())
}

#[test]
fn retains_effective_deploy_resource_reservation_cpu_reset_and_override_provenance()
-> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_resource_reservation_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let leaf = effective_deploy_resources(view, "leaf-reset")?
        .value()
        .reservations()
        .ok_or("leaf reservations")?;
    assert!(leaf.value().cpus().is_none());
    assert!(leaf.value().unmodeled_fields().iter().any(|field| {
        field.path() == ["services", "leaf-reset", "deploy", "resources", "reservations", "cpus"]
            && field.provenance().operation() == MergeOperation::Reset
    }));
    let reset = effective_deploy_resources(view, "reservations-reset")?
        .value()
        .reservations()
        .ok_or("reset reservations")?;
    assert!(reset.value().cpus().is_none());
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    let resources_reset = effective_deploy_resources(view, "resources-reset")?;
    assert!(resources_reset.value().reservations().is_none());
    assert_eq!(resources_reset.provenance().operation(), MergeOperation::Reset);
    let overridden = effective_deploy_resources(view, "override")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(
        overridden.value().reservations().and_then(|value| value.value().cpus()).map(ProjectValue::value),
        Some(DeployResourceCpus::String(value)) if value == "one"
    ));
    Ok(())
}

#[test]
fn retains_effective_deploy_resource_reservation_cpu_sensitivity_and_malformed_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_resource_reservation_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let sensitive = effective_deploy_resources(view, "sensitive")?
        .value()
        .reservations()
        .and_then(|reservations| reservations.value().cpus())
        .ok_or("sensitive reservation CPUs expected")?;
    assert!(sensitive.is_sensitive());
    assert!(!format!("{sensitive:?}").contains("private-reserved-cpus"));
    for service in ["malformed-bool", "malformed-map"] {
        let reservations = effective_deploy_resources(view, service)?
            .value()
            .reservations()
            .ok_or("malformed reservations")?;
        assert!(reservations.value().cpus().is_none());
        assert!(
            reservations
                .value()
                .unmodeled_fields()
                .iter()
                .any(|field| { field.path() == ["services", service, "deploy", "resources", "reservations", "cpus"] })
        );
    }
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == PROJECT_EXPECTED_FORM
            && diagnostic.message() == "deploy resource reservations cpus must be a YAML number or string scalar"
    }));
    assert!(!result.diagnostics().iter().any(|diagnostic| {
        diagnostic.message() == "deploy resource limits cpus must be a YAML number or string scalar"
    }));
    Ok(())
}

#[test]
fn retains_effective_deploy_resource_reservation_memory_merge_and_independence()
-> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_resource_reservation_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let resources = effective_deploy_resources(view, "merged")?;
    assert_eq!(resources.provenance().operation(), MergeOperation::Merged);
    let reservations = resources.value().reservations().ok_or("reservations expected")?;
    assert_eq!(reservations.provenance().operation(), MergeOperation::Merged);
    let memory = reservations.value().memory().ok_or("reservation memory expected")?;
    assert_eq!(memory.value().raw(), "001kb");
    assert!(matches!(
        memory.value().kind(),
        DeployResourceMemoryKind::Documented { amount_raw, unit: DeployResourceMemoryUnit::Kb } if amount_raw == "001"
    ));
    assert_eq!(memory.provenance().operation(), MergeOperation::Replaced);
    assert!(matches!(
        resources
            .value()
            .limits()
            .and_then(|limits| limits.value().memory())
            .map(ProjectValue::value)
            .map(compose_lens::model::DeployResourceMemory::raw),
        Some("99m")
    ));
    assert!(matches!(
        view.service("merged")
            .and_then(ProjectService::mem_limit)
            .map(ProjectValue::value)
            .map(|memory| memory.raw().value().as_str()),
        Some("100m")
    ));
    assert!(
        reservations
            .value()
            .unmodeled_fields()
            .iter()
            .any(ProjectFieldReference::is_extension)
    );
    assert!(
        reservations
            .value()
            .unmodeled_fields()
            .iter()
            .any(|field| { field.path() == ["services", "merged", "deploy", "resources", "reservations", "future"] })
    );
    Ok(())
}

#[test]
fn retains_effective_deploy_resource_reservation_memory_reset_and_override_provenance()
-> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_resource_reservation_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let leaf = effective_deploy_resources(view, "leaf-reset")?
        .value()
        .reservations()
        .ok_or("leaf reservations")?;
    assert!(leaf.value().memory().is_none());
    assert!(leaf.value().unmodeled_fields().iter().any(|field| {
        field.path()
            == [
                "services",
                "leaf-reset",
                "deploy",
                "resources",
                "reservations",
                "memory",
            ]
            && field.provenance().operation() == MergeOperation::Reset
    }));
    let reset = effective_deploy_resources(view, "reservations-reset")?
        .value()
        .reservations()
        .ok_or("reset reservations")?;
    assert!(reset.value().memory().is_none());
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    let resources_reset = effective_deploy_resources(view, "resources-reset")?;
    assert!(resources_reset.value().reservations().is_none());
    assert_eq!(resources_reset.provenance().operation(), MergeOperation::Reset);
    let overridden = effective_deploy_resources(view, "override")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(
        overridden
            .value()
            .reservations()
            .and_then(|reservations| reservations.value().memory())
            .map(ProjectValue::value)
            .map(compose_lens::model::DeployResourceMemory::kind),
        Some(DeployResourceMemoryKind::ProviderDependentString)
    ));
    Ok(())
}

#[test]
fn retains_effective_deploy_resource_reservation_memory_sensitivity_and_malformed_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let result = deploy_resource_reservation_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let sensitive = effective_deploy_resources(view, "sensitive")?
        .value()
        .reservations()
        .and_then(|reservations| reservations.value().memory())
        .ok_or("sensitive reservation memory expected")?;
    assert!(sensitive.is_sensitive());
    assert!(!format!("{sensitive:?}").contains("private-reserved-memory"));
    for service in ["memory-malformed-number", "memory-malformed-map"] {
        let reservations = effective_deploy_resources(view, service)?
            .value()
            .reservations()
            .ok_or("malformed reservations")?;
        assert!(reservations.value().memory().is_none());
        assert!(
            reservations.value().unmodeled_fields().iter().any(|field| {
                field.path() == ["services", service, "deploy", "resources", "reservations", "memory"]
            })
        );
    }
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == PROJECT_EXPECTED_FORM
            && diagnostic.message() == "deploy resource reservations memory must be a YAML string scalar"
    }));
    assert!(
        !result
            .diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.message() == "deploy resource limits memory must be a YAML string scalar" })
    );
    Ok(())
}

#[test]
fn retains_effective_reservation_generic_resources_append_reset_override_and_sensitivity()
-> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(3114),
            DocumentOrigin::new("base", "workspace"),
            concat!(
                "services:\n  merged:\n    deploy: {resources: {reservations: {cpus: 2, memory: \"2m\", generic_resources: [{discrete_resource_spec: {kind: gpu, value: 001}}]}}}\n",
                "  reset:\n    deploy: {resources: {reservations: {generic_resources: [{discrete_resource_spec: {value: 1}}]}}}\n",
                "  override:\n    deploy: {resources: {reservations: {generic_resources: [{discrete_resource_spec: {value: 1}}]}}}\n",
                "  sensitive:\n    deploy: {resources: {reservations: {generic_resources: [{discrete_resource_spec: {value: \"${COUNT}\"}}]}}}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(3115),
            DocumentOrigin::new("override", "workspace"),
            concat!(
                "services:\n  merged:\n    deploy: {resources: {reservations: {generic_resources: [{discrete_resource_spec: {kind: fpga, value: \"two\"}}]}}}\n",
                "  reset:\n    deploy: {resources: {reservations: {generic_resources: !reset []}}}\n",
                "  override:\n    deploy: {resources: {reservations: {generic_resources: !override [{discrete_resource_spec: {value: 1e-3}}]}}}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("COUNT", "private-count");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let view = result.view().ok_or("view")?;
    let generic = |service| {
        effective_deploy_resources(view, service)
            .ok()
            .and_then(|resources| resources.value().reservations())
            .and_then(|reservations| reservations.value().generic_resources())
    };
    let merged_values = generic("merged").ok_or("merged generic resources")?;
    assert_eq!(merged_values.provenance().operation(), MergeOperation::Appended);
    assert_eq!(merged_values.value().len(), 2);
    let first = merged_values
        .value()
        .first()
        .and_then(|item| item.value().discrete_resource_spec())
        .and_then(|spec| spec.value().value())
        .ok_or("first value")?;
    assert!(
        matches!(first.value(), compose_lens::model::DeployDiscreteResourceValue::YamlNumber(value) if value == "001")
    );
    let reset = generic("reset").ok_or("reset generic resources")?;
    assert!(reset.value().is_empty() && reset.provenance().operation() == MergeOperation::Reset);
    let overridden = generic("override").ok_or("override generic resources")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    let sensitive = generic("sensitive")
        .and_then(|items| items.value().first())
        .ok_or("sensitive generic resource")?;
    assert!(sensitive.is_sensitive() && !format!("{sensitive:?}").contains("private-count"));
    Ok(())
}

#[test]
fn retains_effective_malformed_reservation_generic_resource_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  mixed:\n    deploy:\n      resources:\n        reservations:\n          generic_resources:\n            - discrete_resource_spec: {kind: gpu, value: 1}\n            - malformed-item\n            - discrete_resource_spec: malformed-spec\n            - discrete_resource_spec:\n                kind: {broken: mapping}\n                value: 1\n            - discrete_resource_spec:\n                kind: fpga\n                value: {broken: mapping}\n            - discrete_resource_spec: {kind: tpu, value: \"ready\"}\n  outer:\n    deploy: {resources: {reservations: {generic_resources: malformed-collection}}}\n";
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3117),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let view = result.view().ok_or("project view")?;
    let reservations = effective_deploy_resources(view, "mixed")?
        .value()
        .reservations()
        .ok_or("mixed reservations")?;
    let items = reservations
        .value()
        .generic_resources()
        .map(ProjectValue::value)
        .ok_or("mixed generic resources")?;
    assert_eq!(items.len(), 6);
    assert!(matches!(
        items
            .iter()
            .map(|item| item.value().form())
            .collect::<Vec<_>>()
            .as_slice(),
        [
            compose_lens::project::ProjectDeployGenericResourceForm::Mapping,
            compose_lens::project::ProjectDeployGenericResourceForm::Unmodeled,
            compose_lens::project::ProjectDeployGenericResourceForm::Mapping,
            compose_lens::project::ProjectDeployGenericResourceForm::Mapping,
            compose_lens::project::ProjectDeployGenericResourceForm::Mapping,
            compose_lens::project::ProjectDeployGenericResourceForm::Mapping,
        ]
    ));
    assert_eq!(items[1].provenance().operation(), MergeOperation::Authored);
    assert_eq!(
        items[1].effective_source().map(|span| &source[span.range()]),
        Some("malformed-item")
    );
    assert!(items[2].value().discrete_resource_spec().is_none());
    assert!(items[2].value().unmodeled_fields().iter().any(|field| {
        field.path()
            == [
                "services",
                "mixed",
                "deploy",
                "resources",
                "reservations",
                "generic_resources",
                "discrete_resource_spec",
            ]
    }));
    for item in &items[3..5] {
        assert_eq!(
            item.value()
                .discrete_resource_spec()
                .map(ProjectValue::value)
                .map(compose_lens::project::ProjectDeployDiscreteResourceSpec::unmodeled_fields)
                .map(<[_]>::len),
            Some(1)
        );
    }
    assert!(matches!(
        items[5]
            .value()
            .discrete_resource_spec()
            .and_then(|spec| spec.value().kind())
            .map(ProjectValue::value),
        Some(kind) if kind == "tpu"
    ));
    let outer = effective_deploy_resources(view, "outer")?
        .value()
        .reservations()
        .ok_or("outer reservations")?;
    assert!(outer.value().generic_resources().is_none());
    assert!(outer.value().unmodeled_fields().iter().any(|field| {
        field.path()
            == [
                "services",
                "outer",
                "deploy",
                "resources",
                "reservations",
                "generic_resources",
            ]
    }));
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == PROJECT_EXPECTED_FORM
            && diagnostic.message() == "deploy resource generic-resource entries must be mappings"
            && diagnostic
                .labels()
                .iter()
                .any(|label| &source[label.span().range()] == "malformed-item")
    }));
    Ok(())
}

#[test]
fn retains_effective_reservation_devices_append_reset_override_and_sensitivity()
-> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(3118),
            DocumentOrigin::new("base", "workspace"),
            concat!(
                "services:\n  merged:\n    deploy: {resources: {limits: {cpus: 2}, reservations: {devices: [{capabilities: [gpu], driver: nvidia}]}}}\n",
                "  reset:\n    deploy: {resources: {reservations: {devices: [{capabilities: [old]}]}}}\n",
                "  override:\n    deploy: {resources: {reservations: {devices: [{capabilities: [old]}]}}}\n",
                "  sensitive:\n    deploy: {resources: {reservations: {devices: [{capabilities: [\"${CAP}\"], driver: \"${DRIVER}\"}]}}}\n",
                "  resource-override:\n    deploy: {resources: {reservations: {devices: [{capabilities: [old]}]}}}\n",
                "  reservation-reset:\n    deploy: {resources: {reservations: {devices: [{capabilities: [old]}]}}}\n",
                "  resource-reset:\n    deploy: {resources: {reservations: {devices: [{capabilities: [old]}]}}}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(3119),
            DocumentOrigin::new("override", "workspace"),
            concat!(
                "services:\n  merged:\n    deploy: {resources: {reservations: {devices: [{capabilities: [gpu]}]}}}\n",
                "  reset:\n    deploy: {resources: {reservations: {devices: !reset []}}}\n",
                "  override:\n    deploy: {resources: {reservations: {devices: !override [{capabilities: [custom]}]}}}\n",
                "  resource-override:\n    deploy: {resources: !override {reservations: {devices: [{capabilities: [new]}]}}}\n",
                "  reservation-reset:\n    deploy: {resources: {reservations: !reset {}}}\n",
                "  resource-reset:\n    deploy: {resources: !reset {}}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CAP", "private-capability");
    let _ = environment.insert_sensitive("DRIVER", "private-driver");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let view = result.view().ok_or("view")?;
    let devices = |service| {
        effective_deploy_resources(view, service)
            .ok()
            .and_then(|resources| resources.value().reservations())
            .and_then(|reservations| reservations.value().devices())
    };
    let merged_devices = devices("merged").ok_or("merged devices")?;
    assert_eq!(merged_devices.provenance().operation(), MergeOperation::Appended);
    assert_eq!(merged_devices.value().len(), 2);
    assert!(matches!(
        merged_devices.value()[0].value().form(),
        compose_lens::project::ProjectDeployReservationDeviceForm::Mapping
    ));
    assert_eq!(
        merged_devices.value()[1]
            .value()
            .capabilities()
            .map(ProjectValue::value)
            .map(|items| items
                .iter()
                .filter_map(|item| item.value().value().map(ProjectValue::value).map(String::as_str))
                .collect::<Vec<_>>()),
        Some(vec!["gpu"])
    );
    let reset = devices("reset").ok_or("reset devices")?;
    assert!(reset.value().is_empty() && reset.provenance().operation() == MergeOperation::Reset);
    assert_eq!(
        devices("override").ok_or("override devices")?.provenance().operation(),
        MergeOperation::Override
    );
    let sensitive = devices("sensitive")
        .and_then(|devices| devices.value().first())
        .ok_or("sensitive device")?;
    assert!(sensitive.is_sensitive() && !format!("{sensitive:?}").contains("private-capability"));
    let sensitive_driver = sensitive.value().driver().ok_or("sensitive driver")?;
    assert!(sensitive_driver.is_sensitive() && !format!("{sensitive_driver:?}").contains("private-driver"));
    assert!(matches!(
        effective_deploy_resources(view, "merged")?
            .value()
            .limits()
            .and_then(|limits| limits.value().cpus())
            .map(ProjectValue::value),
        Some(DeployResourceCpus::YamlNumber(value)) if value == "2"
    ));
    assert_eq!(
        effective_deploy_resources(view, "resource-override")?
            .provenance()
            .operation(),
        MergeOperation::Override
    );
    let reservation_reset = effective_deploy_resources(view, "reservation-reset")?
        .value()
        .reservations()
        .ok_or("reservation reset")?;
    assert!(reservation_reset.value().devices().is_none());
    assert_eq!(reservation_reset.provenance().operation(), MergeOperation::Reset);
    let resource_reset = effective_deploy_resources(view, "resource-reset")?;
    assert!(resource_reset.value().reservations().is_none());
    assert_eq!(resource_reset.provenance().operation(), MergeOperation::Reset);
    Ok(())
}

#[test]
fn retains_effective_reservation_device_allocation_selectors_and_conflicts() -> Result<(), Box<dyn std::error::Error>> {
    let base = SourceId::new(3131);
    let override_id = SourceId::new(3132);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base,
            DocumentOrigin::new("base", "workspace"),
            concat!(
                "services:\n  app:\n    deploy: {resources: {reservations: {devices: [{capabilities: [gpu], count: 2, device_ids: [first, first, \"${ID}\", true, !!timestamp 2023-12-25]}]}}}\n",
                "  separate:\n    deploy: {resources: {reservations: {devices: [{capabilities: [gpu], count: \"all\"}, {capabilities: [gpu], device_ids: [second]}]}}}\n",
                "  malformed:\n    deploy: {resources: {reservations: {devices: [{capabilities: [gpu], count: 1.5, device_ids: wrong}, {capabilities: [gpu], count: !!regex 'gpu.*'}]}}}\n",
                "  reset:\n    deploy: {resources: {reservations: {devices: [{capabilities: [gpu], count: 2}]}}}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  reset:\n    deploy: {resources: {reservations: {devices: !reset []}}}\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("ID", "private-id");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let view = result.view().ok_or("view")?;
    let devices = |name| {
        effective_deploy_resources(view, name)
            .ok()
            .and_then(|resources| resources.value().reservations())
            .and_then(|reservations| reservations.value().devices())
            .ok_or("devices")
    };
    let app = &devices("app")?.value()[0];
    assert!(matches!(
        app.value().count().map(ProjectValue::value),
        Some(DeployReservationDeviceCount::YamlInteger(value)) if value == "2"
    ));
    let ids = app.value().device_ids().ok_or("ids")?;
    assert_eq!(ids.value().len(), 5);
    assert!(matches!(
        ids.value()
            .iter()
            .map(|item| item.value().form())
            .collect::<Vec<_>>()
            .as_slice(),
        [
            compose_lens::project::ProjectDeployReservationDeviceIdForm::String,
            compose_lens::project::ProjectDeployReservationDeviceIdForm::String,
            compose_lens::project::ProjectDeployReservationDeviceIdForm::String,
            compose_lens::project::ProjectDeployReservationDeviceIdForm::Unmodeled,
            compose_lens::project::ProjectDeployReservationDeviceIdForm::Unmodeled,
        ]
    ));
    assert!(ids.value()[2].is_sensitive() && !format!("{:?}", ids.value()[2]).contains("private-id"));
    assert!(matches!(
        devices("separate")?.value()[0].value().count().map(ProjectValue::value),
        Some(DeployReservationDeviceCount::String(value)) if value == "all"
    ));
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code()
                == compose_lens::model::DEPLOY_RESERVATION_DEVICE_ALLOCATION_SELECTOR_CONFLICT)
            .count(),
        2
    );
    assert!(
        devices("malformed")?
            .value()
            .iter()
            .all(|item| item.value().count().is_none())
    );
    assert!(devices("reset")?.value().is_empty());
    assert!(result.diagnostics().iter().any(|diagnostic| diagnostic.message()
        == "deploy resource reservation device count must be a YAML integer or string scalar"));
    assert!(
        result.diagnostics().iter().any(
            |diagnostic| diagnostic.message() == "deploy resource reservation device device_ids must be a sequence"
        )
    );
    Ok(())
}

#[test]
fn retains_effective_reservation_device_options_forms_provenance_and_recovery() -> Result<(), Box<dyn std::error::Error>>
{
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(3141),
            DocumentOrigin::new("base", "workspace"),
            "services:\n  map:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              options: {name: \"${VALUE}\", enabled: true, bad: {nested: value}}\n  list:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              options: [KEY=VALUE, \"\", KEY=VALUE, true]\n  reset:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              options: [old]\n",
        ),
        DocumentInput::new(
            SourceId::new(3142),
            DocumentOrigin::new("override", "workspace"),
            "services:\n  reset:\n    deploy: {resources: {reservations: {devices: !reset []}}}\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("VALUE", "private-value");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let view = result.view().ok_or("view")?;
    let device = |name| {
        effective_deploy_resources(view, name)
            .ok()
            .and_then(|resources| resources.value().reservations())
            .and_then(|reservations| reservations.value().devices())
            .and_then(|devices| devices.value().first())
            .ok_or("device")
    };
    let map = device("map")?.value().options().ok_or("options")?;
    assert!(matches!(map.value().as_map(), Some(entries) if entries.len() == 2));
    assert!(map.is_sensitive() && !format!("{map:?}").contains("private-value"));
    assert_eq!(
        map.value().unmodeled_entries().map(<[ProjectFieldReference]>::len),
        Some(1)
    );
    let list = device("list")?.value().options().ok_or("list options")?;
    assert!(matches!(
        list.value()
            .as_list()
            .map(<[ProjectValue<compose_lens::project::ProjectDeployReservationDeviceOptionItem>]>::len),
        Some(4)
    ));
    assert!(matches!(
        list.value()
            .as_list()
            .map(|items| items[0].value().value().map(ProjectValue::value).map(String::as_str)),
        Some(Some("KEY=VALUE"))
    ));
    assert!(matches!(
        list.value().as_list().map(|items| items[3].value().form()),
        Some(compose_lens::project::ProjectDeployReservationDeviceOptionItemForm::Unmodeled)
    ));
    assert!(
        effective_deploy_resources(view, "reset")?
            .value()
            .reservations()
            .and_then(|reservations| reservations.value().devices())
            .is_some_and(|devices| devices.value().is_empty())
    );
    assert!(
        result.diagnostics().iter().any(
            |diagnostic| diagnostic.code() == compose_lens::model::DEPLOY_RESERVATION_DEVICE_OPTIONS_DUPLICATE_ITEM
        )
    );
    Ok(())
}

#[test]
fn retains_effective_malformed_reservation_device_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  malformed:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - scalar-item\n            - capabilities: no-list\n            - driver: retained\n            - capabilities: [valid, true, {bad: value}]\n  duplicate:\n    deploy: {resources: {reservations: {devices: [{capabilities: [same, same]}]}}}\n  outer:\n    deploy: {resources: {reservations: {devices: bad}}}\n  reset-null:\n    deploy: {resources: {reservations: {devices: !reset null}}}\n";
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3120),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let view = result.view().ok_or("view")?;
    let reservations = effective_deploy_resources(view, "malformed")?
        .value()
        .reservations()
        .ok_or("reservations")?;
    let items = reservations
        .value()
        .devices()
        .map(ProjectValue::value)
        .ok_or("devices")?;
    assert_eq!(items.len(), 4);
    assert!(matches!(
        items[0].value().form(),
        compose_lens::project::ProjectDeployReservationDeviceForm::Unmodeled
    ));
    assert_eq!(
        items[0].effective_source().map(|span| &source[span.range()]),
        Some("scalar-item")
    );
    assert!(items[1].value().capabilities().is_none() && items[1].value().unmodeled_fields().len() == 1);
    assert_eq!(
        items[2].value().driver().map(ProjectValue::value).map(String::as_str),
        Some("retained")
    );
    assert!(matches!(
        items[3]
            .value()
            .capabilities()
            .map(ProjectValue::value)
            .map(|capabilities| capabilities.iter().map(|item| item.value().form()).collect::<Vec<_>>())
            .as_deref(),
        Some([
            compose_lens::project::ProjectDeployReservationDeviceCapabilityForm::String,
            compose_lens::project::ProjectDeployReservationDeviceCapabilityForm::Unmodeled,
            compose_lens::project::ProjectDeployReservationDeviceCapabilityForm::Unmodeled,
        ])
    ));
    let outer = effective_deploy_resources(view, "outer")?
        .value()
        .reservations()
        .ok_or("outer")?;
    assert!(outer.value().devices().is_none());
    assert!(
        outer
            .value()
            .unmodeled_fields()
            .iter()
            .any(|field| field.path().last() == Some(&"devices".to_owned()))
    );
    for code in [PROJECT_EXPECTED_FORM, PROJECT_MISSING_FIELD] {
        assert!(result.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    let duplicate = effective_deploy_resources(view, "duplicate")?
        .value()
        .reservations()
        .and_then(|reservations| reservations.value().devices())
        .and_then(|devices| devices.value().first())
        .and_then(|device| device.value().capabilities())
        .map(ProjectValue::value)
        .ok_or("duplicate capabilities")?;
    assert_eq!(duplicate.len(), 2);
    assert!(result.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == compose_lens::model::DEPLOY_RESERVATION_DEVICE_CAPABILITY_DUPLICATE_ITEM
    }));
    let reset_null = effective_deploy_resources(view, "reset-null")?
        .value()
        .reservations()
        .ok_or("reset null")?;
    assert!(reset_null.value().devices().is_none());
    assert!(
        reset_null
            .value()
            .unmodeled_fields()
            .iter()
            .any(|field| field.path().last() == Some(&"devices".to_owned()))
    );
    Ok(())
}

#[test]
fn rejects_non_string_effective_reservation_device_drivers() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  invalid:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              driver: !!timestamp 2023-12-25\n            - capabilities: [gpu]\n              driver: !!regex 'gpu.*'\n  quoted:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              driver: \"2023-12-25\"\n            - capabilities: [gpu]\n              driver: \"gpu.*\"\n";
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3122),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let devices = effective_deploy_resources(result.view().ok_or("view")?, "invalid")?
        .value()
        .reservations()
        .and_then(|reservations| reservations.value().devices())
        .map(ProjectValue::value)
        .ok_or("devices")?;
    assert!(
        devices
            .iter()
            .all(|device| device.value().driver().is_none() && device.value().unmodeled_fields().len() == 1)
    );
    assert!(result.diagnostics().iter().all(|diagnostic| {
        diagnostic.message() == "deploy resource reservation device driver must be a YAML string scalar"
    }));
    let quoted = effective_deploy_resources(result.view().ok_or("view")?, "quoted")?
        .value()
        .reservations()
        .and_then(|reservations| reservations.value().devices())
        .map(ProjectValue::value)
        .ok_or("quoted devices")?;
    assert_eq!(
        quoted
            .iter()
            .filter_map(|device| device.value().driver().map(ProjectValue::value).map(String::as_str))
            .collect::<Vec<_>>(),
        ["2023-12-25", "gpu.*"]
    );
    Ok(())
}

#[test]
fn retains_effective_build_privileged_merge_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let a = SourceId::new(2402);
    let b = SourceId::new(2403);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            a,
            DocumentOrigin::new("a", "w"),
            "services:\n  replaced:\n    build: {privileged: false}\n  reset:\n    build: {privileged: true}\n  override:\n    build: {privileged: false}\n  sensitive:\n    build: {privileged: \"${PRIV}\"}\n  sensitive-boolean:\n    init: \"${INIT}\"\n  invalid:\n    build: {context: kept, privileged: \"yes\"}\n",
        ),
        DocumentInput::new(
            b,
            DocumentOrigin::new("b", "w"),
            "services:\n  replaced:\n    build: {privileged: true}\n  reset:\n    build: !reset {}\n  override:\n    build: !override {privileged: true}\n",
        ),
    ])?;
    let mut env = MapEnvironment::new();
    let _ = env.insert_sensitive("PRIV", "true");
    let _ = env.insert_sensitive("INIT", "true");
    let interpolation = loaded.interpolate(&env);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let view = result.view().ok_or("view")?;
    let definition = |n| {
        view.service(n)
            .and_then(ProjectService::build)
            .and_then(|b| match b.value() {
                ProjectBuild::Definition(d) => Some(d),
                _ => None,
            })
            .ok_or("def")
    };
    let replaced = definition("replaced")?.privileged().ok_or("replaced")?;
    assert!(
        matches!(replaced.value(), BooleanValue::Literal(true))
            && replaced.provenance().operation() == MergeOperation::Replaced
    );
    assert!(definition("reset")?.privileged().is_none());
    let override_build = view
        .service("override")
        .and_then(ProjectService::build)
        .ok_or("override")?;
    assert_eq!(override_build.provenance().operation(), MergeOperation::Override);
    assert!(matches!(
        definition("override")?.privileged().map(ProjectValue::value),
        Some(BooleanValue::Literal(true))
    ));
    assert!(
        definition("sensitive")?
            .privileged()
            .is_some_and(|value| value.is_sensitive() && value.value() == &BooleanValue::Literal(true))
    );
    assert!(matches!(
        view.service("sensitive-boolean")
            .and_then(ProjectService::init)
            .map(ProjectValue::value),
        Some(BooleanValue::Literal(true))
    ));
    assert!(
        view.service("sensitive-boolean")
            .and_then(ProjectService::init)
            .is_some_and(ProjectValue::is_sensitive)
    );
    let invalid = definition("invalid")?;
    assert!(
        invalid.privileged().is_none()
            && invalid.context().is_some()
            && invalid
                .unmodeled_fields()
                .iter()
                .any(|f| f.path() == ["services", "invalid", "privileged"])
    );
    Ok(())
}

#[test]
fn retains_effective_no_cache_filter_merge_forms_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let a = SourceId::new(2302);
    let b = SourceId::new(2303);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            a,
            DocumentOrigin::new("a", "w"),
            "services:\n  append:\n    build: {no_cache_filter: [one, one]}\n  mixed:\n    build: {no_cache_filter: [one]}\n  reset:\n    build: {no_cache_filter: [one]}\n  override:\n    build: {no_cache_filter: [one]}\n  sensitive:\n    build: {no_cache_filter: [\"${FILTER}\"]}\n  bad:\n    build: {context: kept, no_cache_filter: [one, false]}\n",
        ),
        DocumentInput::new(
            b,
            DocumentOrigin::new("b", "w"),
            "services:\n  append:\n    build: {no_cache_filter: [two, one]}\n  mixed:\n    build: {no_cache_filter: scalar}\n  reset:\n    build: !reset {}\n  override:\n    build: !override {no_cache_filter: scalar}\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("FILTER", "private");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged")?, None);
    let view = result.view().ok_or("view")?;
    let definition = |n| {
        view.service(n)
            .and_then(ProjectService::build)
            .and_then(|b| match b.value() {
                ProjectBuild::Definition(d) => Some(d),
                _ => None,
            })
            .ok_or("definition")
    };
    let append = definition("append")?.no_cache_filter().ok_or("append")?;
    assert!(
        matches!(append.value(),ProjectBuildNoCacheFilter::List(v) if v.iter().map(ProjectValue::value).map(String::as_str).collect::<Vec<_>>()==["one","one","two","one"])
    );
    assert_eq!(append.provenance().operation(), MergeOperation::Appended);
    assert!(
        matches!(definition("mixed")?.no_cache_filter().map(ProjectValue::value),Some(ProjectBuildNoCacheFilter::Scalar(v))if v.value()=="scalar")
    );
    assert!(definition("reset")?.no_cache_filter().is_none());
    let override_build = view
        .service("override")
        .and_then(ProjectService::build)
        .ok_or("override")?;
    assert_eq!(override_build.provenance().operation(), MergeOperation::Override);
    let override_filter = definition("override")?.no_cache_filter().ok_or("override filter")?;
    assert!(matches!(override_filter.value(), ProjectBuildNoCacheFilter::Scalar(value) if value.value() == "scalar"));
    assert!(
        definition("sensitive")?
            .no_cache_filter()
            .is_some_and(ProjectValue::is_sensitive)
    );
    let bad = definition("bad")?;
    assert!(
        bad.context().is_some()
            && matches!(bad.no_cache_filter().map(ProjectValue::value),Some(ProjectBuildNoCacheFilter::List(v))if v.len()==1)
    );
    assert!(
        bad.unmodeled_fields()
            .iter()
            .any(|f| f.path() == ["services", "bad", "no_cache_filter"])
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .filter(|d| d.code() == compose_lens::model::BUILD_NO_CACHE_FILTER_DUPLICATE_ITEM)
            .count()
            >= 2
    );
    Ok(())
}

#[test]
fn retains_effective_build_provenance_merge_reset_override_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::BuildProvenance;
    let base = SourceId::new(2203);
    let over = SourceId::new(2204);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base,
            DocumentOrigin::new("compose.yaml", "workspace"),
            "services:\n  replaced:\n    build: {provenance: true}\n  reset:\n    build: {provenance: false}\n  override:\n    build: {provenance: false}\n  sensitive:\n    build: {provenance: \"${MODE}\"}\n  invalid:\n    build: {context: retained, provenance: 1}\n",
        ),
        DocumentInput::new(
            over,
            DocumentOrigin::new("override.yaml", "workspace"),
            "services:\n  replaced:\n    build: {provenance: \"mode=max\"}\n  reset:\n    build: !reset {}\n  override:\n    build: !override {provenance: true}\n",
        ),
    ])?;
    let mut env = MapEnvironment::new();
    let _ = env.insert_sensitive("MODE", "mode=min");
    let interpolation = loaded.interpolate(&env);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged expected")?, None);
    let view = result.view().ok_or("view expected")?;
    let definition = |name| {
        view.service(name)
            .and_then(ProjectService::build)
            .and_then(|b| match b.value() {
                ProjectBuild::Definition(d) => Some(d),
                _ => None,
            })
            .ok_or("definition expected")
    };
    let replaced = definition("replaced")?.provenance().ok_or("replaced expected")?;
    assert!(matches!(replaced.value(), BuildProvenance::String(v) if v == "mode=max"));
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base, over]);
    assert!(definition("reset")?.provenance().is_none());
    let override_build = view
        .service("override")
        .and_then(ProjectService::build)
        .ok_or("override expected")?;
    assert_eq!(override_build.provenance().operation(), MergeOperation::Override);
    assert!(matches!(
        definition("override")?.provenance().map(ProjectValue::value),
        Some(BuildProvenance::Boolean(true))
    ));
    let sensitive = definition("sensitive")?.provenance().ok_or("sensitive expected")?;
    assert!(sensitive.is_sensitive() && matches!(sensitive.value(), BuildProvenance::String(v) if v == "mode=min"));
    let invalid = definition("invalid")?;
    assert!(
        invalid.provenance().is_none()
            && invalid.context().is_some()
            && invalid
                .unmodeled_fields()
                .iter()
                .any(|f| f.path() == ["services", "invalid", "provenance"])
    );
    Ok(())
}

#[test]
fn retains_merged_interpolated_and_reset_network_boolean_definitions() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(820),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "networks:\n",
                "  retained:\n",
                "    driver: \"${NETWORK_DRIVER}\"\n",
                "    internal: true\n",
                "    enable_ipv4: true\n",
                "    enable_ipv6: false\n",
                "  reset:\n",
                "    internal: true\n",
                "    enable_ipv4: true\n",
                "    enable_ipv6: true\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(821),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            concat!(
                "networks:\n",
                "  retained:\n",
                "    driver: \"${NETWORK_DRIVER_OVERRIDE}\"\n",
                "    internal: false\n",
                "    enable_ipv4: false\n",
                "    enable_ipv6: true\n",
                "  reset: !reset {}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("NETWORK_DRIVER", "base-driver");
    let _ = environment.insert("NETWORK_DRIVER_OVERRIDE", "override-driver");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    let retained = view
        .networks()
        .iter()
        .find(|network| network.name().value() == "retained")
        .ok_or("retained network expected")?;
    assert_eq!(retained.definition().provenance().operation(), MergeOperation::Merged);
    let retained = retained.definition().value();
    assert_eq!(
        retained.driver().map(|value| value.value().as_str()),
        Some("override-driver")
    );
    assert_eq!(
        retained.internal().map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(
        retained.enable_ipv4().map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(
        retained.enable_ipv6().map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(true))
    );

    let reset = view
        .networks()
        .iter()
        .find(|network| network.name().value() == "reset")
        .ok_or("reset network expected")?;
    assert_eq!(reset.definition().provenance().operation(), MergeOperation::Reset);
    let reset = reset.definition().value();
    assert!(reset.internal().is_none() && reset.enable_ipv4().is_none() && reset.enable_ipv6().is_none());
    Ok(())
}

#[test]
fn retains_effective_sensitive_build_ssh_forms_merge_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(998);
    let override_id = SourceId::new(999);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  mapping:\n    build:\n      ssh:\n        default: \"${BASE_SSH}\"\n        retries: 2\n        enabled: true\n        empty: null\n",
                "  list:\n    build:\n      ssh: [default, default]\n",
                "  reset:\n    build:\n      ssh: [old]\n",
                "  mixed:\n    build:\n      ssh: [old]\n",
                "  malformed:\n    build:\n      ssh: [before, false, {bad: value}, later]\n",
                "  wrong:\n    build:\n      ssh: default\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  mapping:\n    build:\n      ssh:\n        default: \"${NEXT_SSH}\"\n        added: next\n",
                "  list:\n    build:\n      ssh: [\"id=deploy,src=${NEXT_SSH}\"]\n",
                "  reset:\n    build:\n      ssh: !reset []\n",
                "  mixed:\n    build:\n      ssh:\n        default: new\n",
                "  malformed:\n    build:\n      ssh: [\"id=interpolated,src=${NEXT_SSH}\", []]\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("BASE_SSH", "private-base-ssh");
    let _ = environment.insert("NEXT_SSH", "private-next-ssh");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    let mapping = build_definition(view, "mapping")?.ssh().ok_or("mapping ssh expected")?;
    assert!(mapping.is_sensitive());
    assert_eq!(mapping.provenance().operation(), MergeOperation::Merged);
    let ProjectBuildSsh::Map(entries) = mapping.value() else {
        return Err("mapping ssh form expected".into());
    };
    assert_eq!(
        entries.iter().map(|entry| entry.name().value()).collect::<Vec<_>>(),
        ["default", "retries", "enabled", "empty", "added"]
    );
    assert!(
        entries
            .iter()
            .all(|entry| entry.name().is_sensitive() && entry.value().is_sensitive())
    );
    for rendered in [
        format!("{mapping:?}"),
        format!("{entries:?}"),
        format!("{:?}", result.diagnostics()),
    ] {
        for secret in [
            "default",
            "private-base-ssh",
            "private-next-ssh",
            "id=deploy",
            "id=interpolated",
        ] {
            assert!(!rendered.contains(secret), "sensitive effective value leaked: {secret}");
        }
    }

    let list = build_definition(view, "list")?.ssh().ok_or("list ssh expected")?;
    assert_eq!(list.provenance().operation(), MergeOperation::Appended);
    assert!(matches!(list.value(), ProjectBuildSsh::List(items)
        if items.iter().map(ProjectValue::value).map(String::as_str).collect::<Vec<_>>()
            == ["default", "default", "id=deploy,src=private-next-ssh"]
            && items.iter().all(ProjectValue::is_sensitive)));
    assert!(!format!("{list:?}").contains("private-next-ssh"));

    let reset = build_definition(view, "reset")?.ssh().ok_or("reset ssh expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(matches!(reset.value(), ProjectBuildSsh::List(items) if items.is_empty()));
    let mixed = build_definition(view, "mixed")?.ssh().ok_or("mixed ssh expected")?;
    assert!(matches!(mixed.value(), ProjectBuildSsh::Map(entries) if entries.len() == 1));

    let malformed = build_definition(view, "malformed")?
        .ssh()
        .ok_or("malformed ssh expected")?;
    assert!(matches!(malformed.value(), ProjectBuildSsh::List(items)
        if items.iter().map(ProjectValue::value).map(String::as_str).collect::<Vec<_>>()
            == ["before", "later", "id=interpolated,src=private-next-ssh"]));
    for service in ["malformed", "wrong"] {
        assert!(
            build_definition(view, service)?
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", service, "ssh"])
        );
    }
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

#[test]
fn retains_effective_inline_dockerfiles_with_merge_provenance_and_conflicts() -> Result<(), Box<dyn std::error::Error>>
{
    let base_id = SourceId::new(2006);
    let override_id = SourceId::new(2007);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  inline:\n    build:\n      dockerfile_inline: |-\n        FROM ${BASE_IMAGE}\n        RUN echo inline\n",
                "  sensitive:\n    build: {dockerfile_inline: \"${BASE_IMAGE}\"}\n",
                "  empty:\n    build: {dockerfile_inline: \"\"}\n",
                "  replaced:\n    build: {dockerfile_inline: FROM base}\n",
                "  reset:\n    build: {dockerfile_inline: FROM reset}\n",
                "  overridden:\n    build: {dockerfile_inline: FROM old}\n",
                "  malformed:\n    build: {context: retained, dockerfile_inline: false}\n",
                "  conflicting:\n    build: {dockerfile: Dockerfile, dockerfile_inline: FROM scratch}\n",
                "  cross-conflicting:\n    build: {dockerfile: Dockerfile}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    build: {dockerfile_inline: \"${REPLACED_IMAGE}\"}\n",
                "  reset:\n    build: !reset {}\n",
                "  overridden:\n    build: !override {dockerfile_inline: FROM override}\n",
                "  cross-conflicting:\n    build: {dockerfile_inline: FROM scratch}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("BASE_IMAGE", "private/base");
    let _ = environment.insert_sensitive("REPLACED_IMAGE", "private/replaced");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    let inline = inline_dockerfile(view, "inline")?;
    assert_eq!(inline.value(), "FROM ${BASE_IMAGE}\nRUN echo inline");
    assert!(!inline.is_sensitive());
    let sensitive = inline_dockerfile(view, "sensitive")?;
    assert_eq!(sensitive.value(), "private/base");
    assert!(sensitive.is_sensitive());
    assert_eq!(
        inline_dockerfile(view, "empty")?.value(),
        "",
        "explicit empty inline Dockerfile must remain distinct from omission"
    );
    let replaced = inline_dockerfile(view, "replaced")?;
    assert_eq!(replaced.value(), "private/replaced");
    assert!(replaced.is_sensitive());
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    let reset = build_definition(view, "reset")?;
    assert!(reset.dockerfile_inline().is_none());
    let overridden_build = view
        .service("overridden")
        .and_then(ProjectService::build)
        .ok_or("overridden build expected")?;
    assert_eq!(overridden_build.provenance().operation(), MergeOperation::Override);
    let ProjectBuild::Definition(overridden_definition) = overridden_build.value() else {
        return Err("overridden build definition expected".into());
    };
    let overridden = overridden_definition
        .dockerfile_inline()
        .ok_or("overridden inline Dockerfile expected")?;
    assert_eq!(overridden.value(), "FROM override");

    let malformed = build_definition(view, "malformed")?;
    assert!(malformed.context().is_some() && malformed.dockerfile_inline().is_none());
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "malformed", "dockerfile_inline"])
    );
    for service in ["conflicting", "cross-conflicting"] {
        let definition = build_definition(view, service)?;
        assert!(definition.dockerfile().is_some() && definition.dockerfile_inline().is_some());
    }
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == BUILD_DOCKERFILE_INLINE_CONFLICT)
            .count(),
        2
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_INVALID_VALUE)
    );
    Ok(())
}

fn inline_dockerfile<'a>(
    view: &'a ProjectView,
    service: &str,
) -> Result<&'a ProjectValue<String>, Box<dyn std::error::Error>> {
    build_definition(view, service)?
        .dockerfile_inline()
        .ok_or_else(|| format!("{service} inline Dockerfile expected").into())
}

#[test]
fn retains_merged_volume_driver_options_and_reports_external_driver_configuration()
-> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(824),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n  app:\n    image: example.invalid/app\n",
                "volumes:\n",
                "  data:\n",
                "    driver: \"${VOLUME_DRIVER}\"\n",
                "    driver_opts: {string: \"2\", inherited: base}\n",
                "  reset: {driver: base}\n",
                "  override: {driver: base}\n",
                "  external: {external: true, driver: opaque}\n",
                "  implicit:\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(825),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            concat!(
                "volumes:\n",
                "  data:\n",
                "    driver_opts: {number: 2, inherited: override}\n",
                "  reset: !reset {}\n",
                "  override: !override {driver_opts: {}}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("VOLUME_DRIVER", "opaque-driver");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;
    let data = view
        .volumes()
        .iter()
        .find(|volume| volume.name().value() == "data")
        .ok_or("data volume expected")?;
    assert_eq!(data.definition().provenance().operation(), MergeOperation::Merged);
    assert_eq!(
        data.definition().value().driver().map(|driver| driver.value().as_str()),
        Some("opaque-driver")
    );
    assert!(matches!(
        data.definition().value().driver_opts()[0].value().value(),
        ComposeScalar::String(value) if value == "2"
    ));
    assert!(matches!(
        data.definition().value().driver_opts()[1].value().value(),
        ComposeScalar::String(value) if value == "override"
    ));
    assert!(matches!(
        data.definition().value().driver_opts()[2].value().value(),
        ComposeScalar::Number(value) if value == "2"
    ));
    assert!(
        view.volumes()
            .iter()
            .find(|volume| volume.name().value() == "reset")
            .is_some_and(|volume| volume.definition().provenance().operation() == MergeOperation::Reset)
    );
    assert!(
        view.volumes()
            .iter()
            .find(|volume| volume.name().value() == "override")
            .is_some_and(|volume| volume.definition().provenance().operation() == MergeOperation::Override)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == VOLUME_EXTERNAL_DRIVER_CONFIGURATION)
    );
    Ok(())
}

#[test]
fn retains_network_label_interpolation_and_generic_merge_operations() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(822),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "networks:\n",
                "  mapping:\n    labels: {\"${NETWORK_LABEL_KEY}\": \"${NETWORK_LABEL_VALUE}\", plain: base}\n",
                "  sequence:\n    labels: [\"base=one\"]\n",
                "  map-to-list:\n    labels: {old: old}\n",
                "  list-to-map:\n    labels: [\"old=old\"]\n",
                "  reset:\n    labels: {old: old}\n",
                "  override:\n    labels: {old: old}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(823),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            concat!(
                "networks:\n",
                "  mapping:\n    labels: {plain: override, later: later}\n",
                "  sequence:\n    labels: [\"later=two\"]\n",
                "  map-to-list:\n    labels: [\"new=two\"]\n",
                "  list-to-map:\n    labels: {new: two}\n",
                "  reset:\n    labels: !reset {}\n",
                "  override:\n    labels: !override {replacement: override}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("NETWORK_LABEL_KEY", "must-not-change-a-mapping-key");
    let _ = environment.insert_sensitive("NETWORK_LABEL_VALUE", "effective-sensitive-value");
    let interpolation = loaded.interpolate(&environment);
    let merged_result = merge_project(&loaded, Some(&interpolation));
    let merged = merged_result.project().ok_or("merged project expected")?;
    let result = build_project_view(merged, None);
    let view = result.view().ok_or("project view expected")?;

    let mapping = view
        .networks()
        .iter()
        .find(|network| network.name().value() == "mapping")
        .ok_or("mapped network expected")?;
    assert!(mapping.definition().is_sensitive());
    let Some(Labels::Map { entries, .. }) = mapping.definition().value().labels() else {
        return Err("mapped labels expected".into());
    };
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].key().value(), "${NETWORK_LABEL_KEY}");
    assert_eq!(
        entries[0].value().value(),
        &ComposeScalar::String("effective-sensitive-value".to_owned())
    );
    assert_eq!(
        entries[1].value().value(),
        &ComposeScalar::String("override".to_owned())
    );
    assert_eq!(entries[2].key().value(), "later");

    let sequence = view
        .networks()
        .iter()
        .find(|network| network.name().value() == "sequence")
        .and_then(|network| network.definition().value().labels())
        .ok_or("sequence labels expected")?;
    assert!(
        matches!(sequence, Labels::List { values, .. } if values.iter().map(|value| value.value().as_str()).collect::<Vec<_>>() == ["base=one", "later=two"])
    );

    for (network_name, expected) in [("map-to-list", "new=two"), ("list-to-map", "new")] {
        let labels = view
            .networks()
            .iter()
            .find(|network| network.name().value() == network_name)
            .and_then(|network| network.definition().value().labels())
            .ok_or("cross-form labels expected")?;
        assert!(match (network_name, labels) {
            ("map-to-list", Labels::List { values, .. }) => values[0].value() == expected,
            ("list-to-map", Labels::Map { entries, .. }) => entries[0].key().value() == expected,
            _ => false,
        });
    }

    let reset = merged
        .value(&["networks", "reset", "labels"])
        .ok_or("reset labels expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.as_mapping().is_some_and(<[_]>::is_empty));
    let overridden = merged
        .value(&["networks", "override", "labels"])
        .ok_or("override labels expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(
        overridden
            .as_mapping()
            .is_some_and(|entries| entries.len() == 1 && entries[0].key() == "replacement")
    );
    Ok(())
}

#[test]
fn retains_volume_label_interpolation_and_generic_merge_operations() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(824),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "volumes:\n",
                "  mapping:\n    labels: {plain: base, secret: \"${VOLUME_LABEL_VALUE}\"}\n",
                "  sequence:\n    labels: [base=one]\n",
                "  map-to-list:\n    labels: {old: old}\n",
                "  list-to-map:\n    labels: [old=old]\n",
                "  reset:\n    labels: {old: old}\n",
                "  override:\n    labels: {old: old}\n",
                "  external-empty:\n    external: true\n    labels: {}\n",
                "  external-both:\n    external: true\n    driver: opaque\n    labels: {retained: value}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(825),
            DocumentOrigin::new("compose.override.yaml", "workspace/project"),
            concat!(
                "volumes:\n",
                "  mapping:\n    labels: {plain: override, later: later}\n",
                "  sequence:\n    labels: [later=two]\n",
                "  map-to-list:\n    labels: [new=two]\n",
                "  list-to-map:\n    labels: {new: two}\n",
                "  reset:\n    labels: !reset {}\n",
                "  override:\n    labels: !override {replacement: override}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("VOLUME_LABEL_VALUE", "effective-sensitive-value");
    let interpolation = loaded.interpolate(&environment);
    let merged_result = merge_project(&loaded, Some(&interpolation));
    let merged = merged_result.project().ok_or("merged project expected")?;
    let result = build_project_view(merged, None);
    let view = result.view().ok_or("project view expected")?;

    assert_volume_label_merge_view(view)?;
    assert_volume_label_merge_operations(merged)?;
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == VOLUME_EXTERNAL_LABELS_CONFIGURATION)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == VOLUME_EXTERNAL_DRIVER_CONFIGURATION)
    );
    Ok(())
}

fn assert_volume_label_merge_view(view: &ProjectView) -> Result<(), Box<dyn std::error::Error>> {
    let mapping = view
        .volumes()
        .iter()
        .find(|volume| volume.name().value() == "mapping")
        .ok_or("mapped volume expected")?;
    assert!(mapping.definition().is_sensitive());
    let Some(Labels::Map { entries, .. }) = mapping.definition().value().labels() else {
        return Err("mapped labels expected".into());
    };
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].key().value(), "plain");
    assert_eq!(
        entries[0].value().value(),
        &ComposeScalar::String("override".to_owned())
    );
    assert_eq!(entries[1].key().value(), "secret");
    assert_eq!(
        entries[1].value().value(),
        &ComposeScalar::String("effective-sensitive-value".to_owned())
    );
    assert_eq!(entries[2].key().value(), "later");

    let sequence = view
        .volumes()
        .iter()
        .find(|volume| volume.name().value() == "sequence")
        .and_then(|volume| volume.definition().value().labels())
        .ok_or("sequence labels expected")?;
    assert!(
        matches!(sequence, Labels::List { values, .. } if values.iter().map(|value| value.value().as_str()).collect::<Vec<_>>() == ["base=one", "later=two"])
    );

    for (volume_name, expected) in [("map-to-list", "new=two"), ("list-to-map", "new")] {
        let labels = view
            .volumes()
            .iter()
            .find(|volume| volume.name().value() == volume_name)
            .and_then(|volume| volume.definition().value().labels())
            .ok_or("cross-form labels expected")?;
        assert!(match (volume_name, labels) {
            ("map-to-list", Labels::List { values, .. }) => values[0].value() == expected,
            ("list-to-map", Labels::Map { entries, .. }) => entries[0].key().value() == expected,
            _ => false,
        });
    }
    Ok(())
}

fn assert_volume_label_merge_operations(
    merged: &compose_lens::merge::MergedProject,
) -> Result<(), Box<dyn std::error::Error>> {
    let reset = merged
        .value(&["volumes", "reset", "labels"])
        .ok_or("reset labels expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.as_mapping().is_some_and(<[_]>::is_empty));
    let overridden = merged
        .value(&["volumes", "override", "labels"])
        .ok_or("override labels expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(
        overridden
            .as_mapping()
            .is_some_and(|entries| entries.len() == 1 && entries[0].key() == "replacement")
    );
    Ok(())
}

const IPAM_BASE: &str = concat!(
    "x-aux: &aux\n",
    "  inherited: opaque-inherited-address\n",
    "  shared: opaque-default-address\n",
    "networks:\n",
    "  merged:\n",
    "    ipam:\n",
    "      driver: \"${IPAM_DRIVER}\"\n",
    "      config:\n",
    "        - subnet: opaque-base-subnet\n",
    "          aux_addresses:\n",
    "            <<: *aux\n",
    "            shared: opaque-effective-address\n",
    "      options:\n",
    "        shared: base\n",
    "        inherited: base\n",
    "  reset:\n",
    "    ipam:\n",
    "      config: [{subnet: old}]\n",
    "  overridden:\n",
    "    ipam:\n",
    "      config: [{subnet: old}]\n",
);

const IPAM_OVERRIDE: &str = concat!(
    "networks:\n",
    "  merged:\n",
    "    ipam:\n",
    "      driver: \"${IPAM_DRIVER_OVERRIDE}\"\n",
    "      config:\n",
    "        - subnet: \"${SECOND_SUBNET}\"\n",
    "          ip_range: opaque-second-range\n",
    "          gateway: opaque-second-gateway\n",
    "          aux_addresses: {second: opaque-second-address}\n",
    "      options:\n",
    "        shared: override\n",
    "        secret: \"${IPAM_OPTION_SECRET}\"\n",
    "  reset:\n",
    "    ipam:\n",
    "      config: !reset []\n",
    "  overridden:\n",
    "    ipam:\n",
    "      config: !override [{subnet: opaque-replacement}, {subnet: opaque-replacement}]\n",
);

#[test]
fn exposes_effective_ipam_append_tags_nested_mapping_merges_and_interpolation_sources()
-> Result<(), Box<dyn std::error::Error>> {
    let result = ipam_project_view()?;
    let view = result.view().ok_or("project view expected")?;
    let ipam = assert_effective_ipam_network(&result, view)?;
    assert_ipam_configs_and_aux_addresses(ipam)?;
    assert_ipam_options(ipam);
    assert_ipam_config_tags(view)?;
    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    Ok(())
}

fn ipam_project_view() -> Result<compose_lens::project::ProjectViewResult, Box<dyn std::error::Error>> {
    let base_id = SourceId::new(822);
    let override_id = SourceId::new(823);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            IPAM_BASE,
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            IPAM_OVERRIDE,
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("IPAM_DRIVER", "opaque-base-driver");
    let _ = environment.insert("IPAM_DRIVER_OVERRIDE", "opaque-override-driver");
    let _ = environment.insert("SECOND_SUBNET", "opaque-interpolated-subnet");
    let _ = environment.insert_sensitive("IPAM_OPTION_SECRET", "effective-secret");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = merged.project().ok_or("merged project expected")?;
    assert_eq!(
        project
            .value(&["networks", "merged", "ipam", "config"])
            .map(|value| value.provenance().operation()),
        Some(MergeOperation::Appended)
    );
    assert_eq!(
        project
            .value(&["networks", "merged", "ipam", "options"])
            .map(|value| value.provenance().operation()),
        Some(MergeOperation::Merged)
    );
    for (network, operation) in [
        ("reset", MergeOperation::Reset),
        ("overridden", MergeOperation::Override),
    ] {
        assert_eq!(
            project
                .value(&["networks", network, "ipam", "config"])
                .map(|value| value.provenance().operation()),
            Some(operation)
        );
    }
    Ok(build_project_view(project, None))
}

fn assert_effective_ipam_network<'a>(
    result: &compose_lens::project::ProjectViewResult,
    view: &'a ProjectView,
) -> Result<&'a compose_lens::model::Ipam, Box<dyn std::error::Error>> {
    let base_id = SourceId::new(822);
    let override_id = SourceId::new(823);
    let merged_network = view
        .networks()
        .iter()
        .find(|network| network.name().value() == "merged")
        .ok_or("merged network expected")?;
    assert_eq!(
        merged_network.definition().provenance().operation(),
        MergeOperation::Merged
    );
    assert_source_ids(
        merged_network.definition().provenance().sources(),
        &[base_id, override_id],
    );
    assert!(merged_network.definition().is_sensitive());
    assert!(!format!("{result:?}").contains("effective-secret"));

    let ipam = merged_network
        .definition()
        .value()
        .ipam()
        .ok_or("effective IPAM expected")?;
    let driver = ipam.driver().ok_or("effective driver expected")?;
    assert_eq!(driver.value(), "opaque-override-driver");
    assert_eq!(driver.span().source_id(), override_id);
    Ok(ipam)
}

fn assert_ipam_configs_and_aux_addresses(ipam: &compose_lens::model::Ipam) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        ipam.config().len(),
        2,
        "ordinary IPAM configuration merge appends in source order"
    );
    assert_eq!(
        ipam.config()
            .iter()
            .map(|config| config
                .subnet()
                .map(compose_lens::model::Located::value)
                .map(String::as_str))
            .collect::<Vec<_>>(),
        [Some("opaque-base-subnet"), Some("opaque-interpolated-subnet")]
    );
    assert_eq!(
        ipam.config()[0]
            .subnet()
            .ok_or("base subnet expected")?
            .span()
            .source_id(),
        SourceId::new(822)
    );
    assert_eq!(
        ipam.config()[1]
            .subnet()
            .ok_or("interpolated subnet expected")?
            .span()
            .source_id(),
        SourceId::new(823)
    );
    assert_eq!(
        ipam.config()[0]
            .aux_addresses()
            .iter()
            .map(|entry| entry.key().value().as_str())
            .collect::<Vec<_>>(),
        ["shared", "inherited"]
    );
    assert_eq!(
        ipam.config()[0].aux_addresses()[0].value().value(),
        &ComposeScalar::String("opaque-effective-address".to_owned())
    );
    Ok(())
}

fn assert_ipam_options(ipam: &compose_lens::model::Ipam) {
    assert_eq!(
        ipam.options()
            .iter()
            .map(|entry| entry.key().value().as_str())
            .collect::<Vec<_>>(),
        ["shared", "inherited", "secret"]
    );
    let shared = &ipam.options()[0];
    assert_eq!(shared.value().value(), &ComposeScalar::String("override".to_owned()));
    assert_eq!(shared.value().span().source_id(), SourceId::new(823));
    let secret = &ipam.options()[2];
    assert_eq!(
        secret.value().value(),
        &ComposeScalar::String("effective-secret".to_owned())
    );
    assert_eq!(secret.value().span().source_id(), SourceId::new(823));
    assert_eq!(
        &IPAM_OVERRIDE[secret.value().span().range()],
        "\"${IPAM_OPTION_SECRET}\""
    );
}

fn assert_ipam_config_tags(view: &ProjectView) -> Result<(), Box<dyn std::error::Error>> {
    let reset = view
        .networks()
        .iter()
        .find(|network| network.name().value() == "reset")
        .and_then(|network| network.definition().value().ipam())
        .ok_or("reset IPAM expected")?;
    assert!(reset.config().is_empty());
    let overridden = view
        .networks()
        .iter()
        .find(|network| network.name().value() == "overridden")
        .and_then(|network| network.definition().value().ipam())
        .ok_or("overridden IPAM expected")?;
    assert_eq!(
        overridden
            .config()
            .iter()
            .map(|config| config
                .subnet()
                .map(compose_lens::model::Located::value)
                .map(String::as_str))
            .collect::<Vec<_>>(),
        [Some("opaque-replacement"), Some("opaque-replacement")]
    );
    Ok(())
}

#[test]
fn exposes_keyed_annotations_with_replacement_raw_evidence_sensitivity_and_ambiguity()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{ANNOTATIONS_DUPLICATE_NAME, ANNOTATIONS_EXPECTED_STRING, ANNOTATIONS_KEY_ONLY};

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(698),
            DocumentOrigin::new("compose.yaml", "workspace/base"),
            concat!(
                "services:\n",
                "  app:\n    annotations:\n",
                "      - \"io.example.same=base\"\n",
                "  ambiguous:\n    annotations:\n",
                "      - \"io.example.key-only\"\n",
                "      - 7\n",
                "  malformed:\n    annotations: [null, [nested]]\n",
                "  reset:\n    annotations: {io.example.old: old}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(699),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n",
                "  app:\n    annotations:\n",
                "      io.example.same: \"${ANNOTATION_SECRET}\"\n",
                "  reset:\n    annotations: !reset {}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("ANNOTATION_SECRET", "effective-secret");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("partial project view expected")?;
    let annotations = view
        .service("app")
        .and_then(ProjectService::annotations)
        .ok_or("effective annotations expected")?;
    let same = annotations
        .value()
        .get("io.example.same")
        .ok_or("same annotation expected")?;
    assert_eq!(same.name().sources().len(), 2);
    assert_eq!(same.contributors().len(), 1);
    assert_eq!(same.contributors()[0].sources().len(), 2);
    let value = same.value().ok_or("explicit annotation value expected")?;
    assert!(matches!(value.value().effective(), ComposeScalar::String(value) if value == "effective-secret"));
    assert_eq!(value.value().authored(), "\"${ANNOTATION_SECRET}\"");
    assert!(value.is_sensitive());
    assert!(!format!("{annotations:?}").contains("effective-secret"));

    let ambiguous = view
        .service("ambiguous")
        .and_then(ProjectService::annotations)
        .ok_or("ambiguous annotations expected")?;
    let key_only = ambiguous
        .value()
        .get("io.example.key-only")
        .ok_or("key-only evidence expected")?;
    assert_eq!(key_only.syntax(), EntrySyntax::ListKeyOnly);
    assert!(key_only.value().is_none());
    assert!(key_only.raw_list_item().is_some());
    assert!(
        ambiguous.value().get("7").is_none(),
        "only valid string key=value entries enter the keyed effective view"
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == ANNOTATIONS_EXPECTED_STRING),
        "invalid scalar evidence remains in the merged source and is diagnosed"
    );
    let reset = view
        .service("reset")
        .and_then(ProjectService::annotations)
        .ok_or("reset annotations expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.value().entries().is_empty());
    for code in [ANNOTATIONS_EXPECTED_STRING, ANNOTATIONS_KEY_ONLY] {
        assert!(
            result.diagnostics().iter().any(|diagnostic| diagnostic.code() == code),
            "missing {code}"
        );
    }
    assert!(
        result
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code() != ANNOTATIONS_DUPLICATE_NAME),
        "field-specific merge already replaced the cross-file duplicate"
    );
    Ok(())
}

#[test]
fn exposes_merged_expose_identity_provenance_sensitivity_reset_and_recovery() -> Result<(), Box<dyn std::error::Error>>
{
    use compose_lens::model::{
        EXPOSE_DUPLICATE_ITEM, EXPOSE_EXPECTED_SCALAR, EXPOSE_INVALID_ITEM, EXPOSE_PROVIDER_DEPENDENT, ExposeItemKind,
        ExposeScalarKind,
    };

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(692),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n",
                "  merged:\n    expose: [80, \"80\", \"80/tcp\"]\n",
                "  reset:\n    expose: [90]\n",
                "  malformed:\n    image: example.invalid/recovery:1\n    expose: [91, true, [92], broken, \"93/sctp\"]\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(693),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n",
                "  merged:\n    expose: [80, \"80\", \"${EXPOSE_SECRET}\", \"80/udp\"]\n",
                "  reset:\n    expose: !reset null\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("EXPOSE_SECRET", "443/tcp");
    let interpolation = loaded.interpolate(&environment);
    let merged_result = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged_result.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("partial project view expected")?;
    let expose = view
        .service("merged")
        .and_then(ProjectService::expose)
        .ok_or("expose expected")?;
    assert_eq!(expose.provenance().operation(), MergeOperation::Merged);
    assert_eq!(expose.value().len(), 5);
    assert_eq!(expose.value()[0].value().scalar_kind(), ExposeScalarKind::Number);
    assert_eq!(expose.value()[1].value().scalar_kind(), ExposeScalarKind::String);
    assert_eq!(expose.value()[0].provenance().sources().len(), 2);
    assert_eq!(expose.value()[1].provenance().sources().len(), 2);
    assert_eq!(expose.value()[2].value().value(), "80/tcp");
    assert_eq!(expose.value()[3].value().authored(), "\"${EXPOSE_SECRET}\"");
    assert_eq!(expose.value()[3].value().value(), "443/tcp");
    assert!(expose.value()[3].is_sensitive());
    assert!(!format!("{expose:?}").contains("443/tcp"));
    assert_eq!(expose.value()[4].value().value(), "80/udp");

    let reset = view
        .service("reset")
        .and_then(ProjectService::expose)
        .ok_or("reset expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.value().is_empty());

    let malformed = view.service("malformed").ok_or("malformed service retained")?;
    assert!(malformed.image().is_some());
    let malformed_items = malformed.expose().ok_or("partial expose expected")?.value();
    assert_eq!(malformed_items.len(), 3);
    assert!(matches!(malformed_items[1].value().kind(), ExposeItemKind::Malformed));
    assert!(matches!(malformed_items[2].value().kind(), ExposeItemKind::Sctp { .. }));
    for code in [EXPOSE_EXPECTED_SCALAR, EXPOSE_INVALID_ITEM, EXPOSE_PROVIDER_DEPENDENT] {
        assert!(
            result.diagnostics().iter().any(|diagnostic| diagnostic.code() == code),
            "missing {code}"
        );
    }
    assert!(
        !result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPOSE_DUPLICATE_ITEM)
    );
    Ok(())
}

#[test]
fn exposes_merged_dns_search_forms_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(690),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n",
                "  appended:\n    dns_search: [base.internal, same.internal]\n",
                "  scalar:\n    dns_search: old.internal\n",
                "  cross-form:\n    dns_search: old.internal\n",
                "  reset:\n    dns_search: [old.internal]\n",
                "  malformed:\n    image: example.invalid/recovery:1\n    dns_search: [valid-before.internal, true, [nested], valid-after.internal]\n",
                "  bad-form:\n    image: example.invalid/bad:1\n    dns_search: {domain: invalid.internal}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(691),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n",
                "  appended:\n    dns_search: [same.internal, later.internal]\n",
                "  scalar:\n    dns_search: \"${DNS_SEARCH_SECRET}\"\n",
                "  cross-form:\n    dns_search: [new.internal]\n",
                "  reset:\n    dns_search: !reset null\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("DNS_SEARCH_SECRET", "secret.internal");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("partial project view expected")?;

    let appended = view
        .service("appended")
        .and_then(ProjectService::dns_search)
        .ok_or("appended dns_search expected")?;
    assert_eq!(appended.provenance().operation(), MergeOperation::Appended);
    assert!(matches!(appended.value(), ProjectDnsSearch::List(items)
        if items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>()
            == ["base.internal", "same.internal", "same.internal", "later.internal"]
        && items.iter().all(|item| item.provenance().sources().len() == 1)));

    let scalar = view
        .service("scalar")
        .and_then(ProjectService::dns_search)
        .ok_or("scalar dns_search expected")?;
    assert_eq!(scalar.provenance().operation(), MergeOperation::Replaced);
    assert!(scalar.is_sensitive());
    assert!(matches!(scalar.value(), ProjectDnsSearch::Scalar(item)
        if item.value() == "secret.internal" && item.is_sensitive()));
    assert!(!format!("{scalar:?}").contains("secret.internal"));

    assert!(matches!(
        view.service("cross-form").and_then(ProjectService::dns_search).map(ProjectValue::value),
        Some(ProjectDnsSearch::List(items)) if items.len() == 1 && items[0].value() == "new.internal"
    ));
    let reset = view
        .service("reset")
        .and_then(ProjectService::dns_search)
        .ok_or("reset dns_search expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(matches!(reset.value(), ProjectDnsSearch::List(items) if items.is_empty()));

    let malformed = view.service("malformed").ok_or("malformed service retained")?;
    assert!(malformed.image().is_some());
    assert!(
        matches!(malformed.dns_search().map(ProjectValue::value), Some(ProjectDnsSearch::List(items))
        if items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>()
            == ["valid-before.internal", "valid-after.internal"])
    );
    assert!(
        view.service("bad-form")
            .is_some_and(|service| service.image().is_some())
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == DNS_SEARCH_EXPECTED_STRING)
            .count(),
        2
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == DNS_SEARCH_EXPECTED_FORM)
            .count(),
        1
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == DNS_SEARCH_DUPLICATE_ITEM)
            .count(),
        1
    );
    Ok(())
}

#[test]
fn exposes_replaced_dns_options_with_item_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>>
{
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(687),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n",
                "  omitted:\n    image: example.invalid/omitted:1\n",
                "  replaced:\n    dns_opt: [ndots:2, timeout:1]\n",
                "  reset:\n    dns_opt: [rotate]\n",
                "  malformed:\n    image: example.invalid/recovery:1\n    dns_opt: [old]\n",
                "  bad-form:\n    image: example.invalid/bad:1\n    dns_opt: [old]\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(688),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n",
                "  replaced:\n    dns_opt: [\"${DNS_OPTION}\", attempts:4]\n",
                "  reset:\n    dns_opt: !reset null\n",
                "  malformed:\n    dns_opt: [valid-before, true, [nested], valid-after, valid-before]\n",
                "  bad-form:\n    dns_opt: rotate\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("DNS_OPTION", "timeout:3");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("partial project view expected")?;

    assert!(
        view.service("omitted")
            .is_some_and(|service| service.dns_options().is_none())
    );
    let replaced = view
        .service("replaced")
        .and_then(ProjectService::dns_options)
        .ok_or("replaced dns_opt expected")?;
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_eq!(replaced.provenance().sources().len(), 2);
    assert!(replaced.is_sensitive());
    assert_eq!(
        replaced
            .value()
            .iter()
            .map(|item| item.value().as_str())
            .collect::<Vec<_>>(),
        ["timeout:3", "attempts:4"]
    );
    assert!(replaced.value()[0].is_sensitive());
    assert_eq!(replaced.value()[0].provenance().sources().len(), 1);
    assert!(!format!("{replaced:?}").contains("timeout:3"));

    let reset = view
        .service("reset")
        .and_then(ProjectService::dns_options)
        .ok_or("reset dns_opt expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.value().is_empty());

    let malformed = view.service("malformed").ok_or("malformed service retained")?;
    assert!(malformed.image().is_some());
    assert_eq!(
        malformed
            .dns_options()
            .ok_or("partial dns_opt expected")?
            .value()
            .iter()
            .map(|item| item.value().as_str())
            .collect::<Vec<_>>(),
        ["valid-before", "valid-after", "valid-before"]
    );
    assert!(
        view.service("bad-form")
            .is_some_and(|service| service.image().is_some())
    );
    for code in [
        DNS_OPT_EXPECTED_SEQUENCE,
        DNS_OPT_EXPECTED_STRING,
        DNS_OPT_DUPLICATE_ITEM,
    ] {
        assert!(result.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn exposes_appended_raw_security_options_with_authored_spelling_sensitivity_and_conflicts()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::merge::MergedScalarKind;
    use compose_lens::model::{
        SECURITY_OPT_APPARMOR_CONFLICT, SECURITY_OPT_APPARMOR_NEAR_MISS, SECURITY_OPT_EMPTY_ITEM,
        SECURITY_OPT_EXPECTED_STRING, SecurityOptionKind,
    };

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(697),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n",
                "  omitted:\n    image: example.invalid/omitted:1\n",
                "  merged:\n    security_opt: [\"label=disable\", \"${APPARMOR_OPTION}\", \"\"]\n",
                "  reset:\n    security_opt: [old]\n",
                "  malformed:\n    image: example.invalid/recovery:1\n    security_opt: [before]\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(698),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n",
                "  merged:\n    security_opt: [\"apparmor=next\", \"apparmor=next\", \"AppArmor=near\"]\n",
                "  reset:\n    security_opt: !reset null\n",
                "  malformed:\n    security_opt: [after, true, [nested]]\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("APPARMOR_OPTION", "apparmor=secret-profile");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("partial project view expected")?;

    assert!(
        view.service("omitted")
            .is_some_and(|service| service.security_options().is_none())
    );
    let effective = view
        .service("merged")
        .and_then(ProjectService::security_options)
        .ok_or("effective security_opt expected")?;
    assert_eq!(effective.provenance().operation(), MergeOperation::Appended);
    assert_eq!(effective.provenance().sources().len(), 2);
    assert!(effective.is_sensitive());
    assert_eq!(effective.value().len(), 6);
    let interpolated = effective.value()[1].value();
    assert_eq!(interpolated.authored(), "\"${APPARMOR_OPTION}\"");
    assert_eq!(interpolated.value(), "apparmor=secret-profile");
    assert_eq!(interpolated.scalar_kind(), MergedScalarKind::String);
    assert!(matches!(interpolated.kind(), SecurityOptionKind::AppArmor { profile } if profile == "secret-profile"));
    assert!(effective.value()[1].is_sensitive());
    assert!(!format!("{effective:?}").contains("secret-profile"));
    assert!(matches!(effective.value()[2].value().kind(), SecurityOptionKind::Empty));
    assert!(matches!(
        effective.value()[5].value().kind(),
        SecurityOptionKind::AppArmorNearMiss
    ));
    assert_eq!(
        effective.value()[3].value().value(),
        effective.value()[4].value().value()
    );

    let reset = view
        .service("reset")
        .and_then(ProjectService::security_options)
        .ok_or("reset security_opt expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.value().is_empty());

    let malformed = view.service("malformed").ok_or("malformed service retained")?;
    assert!(malformed.image().is_some());
    assert_eq!(
        malformed
            .security_options()
            .ok_or("partial security_opt expected")?
            .value()
            .iter()
            .map(|item| item.value().value())
            .collect::<Vec<_>>(),
        ["before", "after"]
    );
    for code in [
        SECURITY_OPT_EXPECTED_STRING,
        SECURITY_OPT_EMPTY_ITEM,
        SECURITY_OPT_APPARMOR_NEAR_MISS,
        SECURITY_OPT_APPARMOR_CONFLICT,
    ] {
        assert!(result.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_every_effective_mask_candidate_without_selecting_or_conflicting() -> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{SECURITY_OPT_MASK_NEAR_MISS, SecurityOptionKind};

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(730),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"${MASK_OPTION}\"\n",
                "      - \"mask=/proc/acpi:/proc/kcore\"\n",
                "      - \"mask:/near-miss\"\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(731),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            "services:\n  app:\n    security_opt: [\"mask=/proc/acpi:/proc/kcore\"]\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("MASK_OPTION", "mask=/proc/acpi:/proc/kcore");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let options = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::security_options)
        .ok_or("effective mask security options expected")?;

    assert_eq!(options.provenance().operation(), MergeOperation::Appended);
    assert_eq!(options.value().len(), 4);
    for index in [0, 1, 3] {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::Mask { paths } if paths == "/proc/acpi:/proc/kcore"
        ));
        assert_eq!(options.value()[index].provenance().sources().len(), 1);
    }
    assert!(options.value()[0].is_sensitive());
    assert_eq!(options.value()[0].value().authored(), "\"${MASK_OPTION}\"");
    assert!(matches!(
        options.value()[2].value().kind(),
        SecurityOptionKind::MaskNearMiss
    ));
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_MASK_NEAR_MISS)
            .count(),
        1
    );
    assert_eq!(result.diagnostics().len(), 1, "repeatable exact masks do not conflict");
    assert!(!format!("{options:?}").contains("/proc/acpi:/proc/kcore"));
    Ok(())
}

#[test]
fn retains_every_effective_unmask_candidate_without_selecting_or_conflicting() -> Result<(), Box<dyn std::error::Error>>
{
    use compose_lens::model::{SECURITY_OPT_UNMASK_NEAR_MISS, SecurityOptionKind};

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(732),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"${UNMASK_OPTION}\"\n",
                "      - \"unmask=ALL\"\n",
                "      - \"unmask=/proc/*\"\n",
                "      - \"unmask=all\"\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(733),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            "services:\n  app:\n    security_opt: [\"unmask=ALL\", \"unmask=/proc/acpi\"]\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("UNMASK_OPTION", "unmask=/proc/acpi:/sys/firmware");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let options = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::security_options)
        .ok_or("effective unmask security options expected")?;

    assert_eq!(options.provenance().operation(), MergeOperation::Appended);
    assert_eq!(options.value().len(), 6);
    for (index, expected) in [
        (0, "/proc/acpi:/sys/firmware"),
        (1, "ALL"),
        (2, "/proc/*"),
        (4, "ALL"),
        (5, "/proc/acpi"),
    ] {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::Unmask { paths } if paths == expected
        ));
        assert_eq!(options.value()[index].provenance().sources().len(), 1);
    }
    assert!(options.value()[0].is_sensitive());
    assert_eq!(options.value()[0].value().authored(), "\"${UNMASK_OPTION}\"");
    assert!(matches!(
        options.value()[3].value().kind(),
        SecurityOptionKind::UnmaskNearMiss
    ));
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_UNMASK_NEAR_MISS)
            .count(),
        1
    );
    assert_eq!(
        result.diagnostics().len(),
        1,
        "repeatable exact unmask values do not conflict"
    );
    assert!(!format!("{options:?}").contains("/sys/firmware"));
    Ok(())
}

#[test]
fn classifies_effective_no_new_privileges_candidates_without_selecting_a_conflict()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{
        SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT, SECURITY_OPT_NO_NEW_PRIVILEGES_NEAR_MISS, SecurityOptionKind,
    };

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(706),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            "services:\n  app:\n    security_opt: [\"${NNP_OPTION}\", \"no-new-privileges:true\"]\n",
        ),
        DocumentInput::new(
            SourceId::new(707),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"no-new-privileges:false\"\n",
                "      - \"no-new-privileges:true\"\n",
                "      - \"No-New-Privileges:true\"\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("NNP_OPTION", "no-new-privileges:false");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let options = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::security_options)
        .ok_or("effective security options expected")?;

    assert_eq!(options.provenance().operation(), MergeOperation::Appended);
    assert_eq!(options.value().len(), 5);
    assert_eq!(options.value()[0].value().authored(), "\"${NNP_OPTION}\"");
    assert!(options.value()[0].is_sensitive());
    for (index, enabled) in [(0, false), (1, true), (2, false), (3, true)] {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::NoNewPrivileges { enabled: actual } if *actual == enabled
        ));
    }
    assert!(matches!(
        options.value()[4].value().kind(),
        SecurityOptionKind::NoNewPrivilegesNearMiss
    ));
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT)
            .count(),
        3
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SECURITY_OPT_NO_NEW_PRIVILEGES_NEAR_MISS)
    );
    Ok(())
}

#[test]
fn classifies_effective_seccomp_candidates_without_selecting_a_conflict() -> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{
        SECURITY_OPT_APPARMOR_CONFLICT, SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT, SECURITY_OPT_SECCOMP_CONFLICT,
        SECURITY_OPT_SECCOMP_NEAR_MISS, SecurityOptionKind,
    };

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(708),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"${SECCOMP_OPTION}\"\n",
                "      - \"apparmor=one-profile\"\n",
                "      - \"no-new-privileges:true\"\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(709),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"seccomp=unconfined\"\n",
                "      - \"seccomp=/workspace/seccomp.json\"\n",
                "      - \"Seccomp=unconfined\"\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("SECCOMP_OPTION", "seccomp=unconfined");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let options = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::security_options)
        .ok_or("effective security options expected")?;

    assert_eq!(options.provenance().operation(), MergeOperation::Appended);
    assert_eq!(options.value().len(), 6);
    assert_eq!(options.value()[0].value().authored(), "\"${SECCOMP_OPTION}\"");
    assert!(options.value()[0].is_sensitive());
    for (index, profile) in [(0, "unconfined"), (3, "unconfined"), (4, "/workspace/seccomp.json")] {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::Seccomp { profile: actual } if actual == profile
        ));
    }
    assert!(matches!(
        options.value()[5].value().kind(),
        SecurityOptionKind::SeccompNearMiss
    ));
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_SECCOMP_CONFLICT)
            .count(),
        2
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SECURITY_OPT_SECCOMP_NEAR_MISS)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code() != SECURITY_OPT_APPARMOR_CONFLICT)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code() != SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT)
    );
    Ok(())
}

#[test]
fn classifies_effective_security_label_disable_candidates_without_selecting_a_conflict()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{
        SECURITY_OPT_APPARMOR_CONFLICT, SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT, SECURITY_OPT_SECCOMP_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT, SECURITY_OPT_SECURITY_LABEL_DISABLE_NEAR_MISS,
        SecurityOptionKind,
    };

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(710),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"${LABEL_DISABLE_OPTION}\"\n",
                "      - \"label:user:USER\"\n",
                "      - \"label:role:ROLE\"\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(711),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"label:disable\"\n",
                "      - \"label:disable\"\n",
                "      - \"label=disable\"\n",
                "      - \"label:disable:false\"\n",
                "      - \"Label:disable\"\n",
                "      - \" label:disable\"\n",
                "      - \"label\"\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("LABEL_DISABLE_OPTION", "label:disable");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let options = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::security_options)
        .ok_or("effective security options expected")?;

    assert_eq!(options.provenance().operation(), MergeOperation::Appended);
    assert_eq!(options.value().len(), 10);
    assert_eq!(options.value()[0].value().authored(), "\"${LABEL_DISABLE_OPTION}\"");
    assert!(options.value()[0].is_sensitive());
    for index in [0, 3, 4] {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::SecurityLabelDisable { enabled: true }
        ));
    }
    for index in 5..10 {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::SecurityLabelDisableNearMiss
        ));
    }
    for index in [1, 2] {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::Other
        ));
    }
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT)
            .count(),
        2
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_SECURITY_LABEL_DISABLE_NEAR_MISS)
            .count(),
        5
    );
    for code in [
        SECURITY_OPT_APPARMOR_CONFLICT,
        SECURITY_OPT_SECCOMP_CONFLICT,
        SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT,
    ] {
        assert!(result.diagnostics().iter().all(|diagnostic| diagnostic.code() != code));
    }
    Ok(())
}

#[test]
fn classifies_effective_security_label_filetype_candidates_without_selecting_a_conflict()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::SecurityOptionKind;

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(712),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"${LABEL_FILETYPE_OPTION}\"\n",
                "      - \"apparmor=one-profile\"\n",
                "      - \"seccomp=unconfined\"\n",
                "      - \"no-new-privileges:true\"\n",
                "      - \"label:disable\"\n",
                "      - \"label:type:TYPE\"\n",
                "      - \"label:user:USER\"\n",
                "      - \"label:role:ROLE\"\n",
                "      - \"label:level:LEVEL\"\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(713),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"label:filetype:container_file_t\"\n",
                "      - \"label:filetype:container_file_t\"\n",
                "      - \"label=filetype:container_file_t\"\n",
                "      - \"label:filetype=container_file_t\"\n",
                "      - \"Label:filetype:container_file_t\"\n",
                "      - \"label:FileType:container_file_t\"\n",
                "      - \" label:filetype:container_file_t\"\n",
                "      - \"label:filetype:container file t\"\n",
                "      - \"label:filetype:\"\n",
                "      - \"label:filetype\"\n",
                "      - \"${LABEL_FILETYPE_NEAR_MISS}\"\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("LABEL_FILETYPE_OPTION", "label:filetype:container_file_t");
    let _ = environment.insert_sensitive("LABEL_FILETYPE_NEAR_MISS", "Label:filetype:container_file_t");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let options = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::security_options)
        .ok_or("effective security options expected")?;

    assert_eq!(options.provenance().operation(), MergeOperation::Appended);
    assert_eq!(options.value().len(), 20);
    assert_eq!(options.value()[0].value().authored(), "\"${LABEL_FILETYPE_OPTION}\"");
    assert!(options.value()[0].is_sensitive());
    for index in [0, 9, 10] {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::SecurityLabelFileType { file_type }
                if file_type == "container_file_t"
        ));
    }
    for index in 11..20 {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::SecurityLabelFileTypeNearMiss
        ));
    }
    assert_eq!(
        options.value()[19].value().authored(),
        "\"${LABEL_FILETYPE_NEAR_MISS}\""
    );
    assert!(options.value()[19].is_sensitive());
    assert!(matches!(
        options.value()[5].value().kind(),
        SecurityOptionKind::SecurityLabelType { label_type } if label_type == "TYPE"
    ));
    for index in 6..8 {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::Other
        ));
    }
    assert!(matches!(
        options.value()[8].value().kind(),
        SecurityOptionKind::SecurityLabelLevel { level } if level == "LEVEL"
    ));
    assert_effective_security_label_filetype_diagnostics(&result);
    Ok(())
}

fn assert_effective_security_label_filetype_diagnostics(result: &compose_lens::project::ProjectViewResult) {
    use compose_lens::model::{
        SECURITY_OPT_APPARMOR_CONFLICT, SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT, SECURITY_OPT_SECCOMP_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT, SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_FILETYPE_NEAR_MISS,
    };

    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT)
            .count(),
        2
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_SECURITY_LABEL_FILETYPE_NEAR_MISS)
            .count(),
        9
    );
    for code in [
        SECURITY_OPT_APPARMOR_CONFLICT,
        SECURITY_OPT_SECCOMP_CONFLICT,
        SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT,
    ] {
        assert!(result.diagnostics().iter().all(|diagnostic| diagnostic.code() != code));
    }
}

#[test]
fn classifies_effective_security_label_level_candidates_without_selecting_a_conflict()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::SecurityOptionKind;

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(714),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"${LABEL_LEVEL_OPTION}\"\n",
                "      - \"apparmor=one-profile\"\n",
                "      - \"seccomp=unconfined\"\n",
                "      - \"no-new-privileges:true\"\n",
                "      - \"label:disable\"\n",
                "      - \"label:filetype:container_file_t\"\n",
                "      - \"label:type:TYPE\"\n",
                "      - \"label:user:USER\"\n",
                "      - \"label:role:ROLE\"\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(715),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"label:level:s0:c1,c2\"\n",
                "      - \"label:level:s0:c1,c2\"\n",
                "      - \"label=level:s0:c1,c2\"\n",
                "      - \"label:level=s0:c1,c2\"\n",
                "      - \"Label:level:s0:c1,c2\"\n",
                "      - \"label:Level:s0:c1,c2\"\n",
                "      - \" label:level:s0:c1,c2\"\n",
                "      - \"label:level:s0 c1\"\n",
                "      - \"label:level:\"\n",
                "      - \"label:level\"\n",
                "      - \"label=level\"\n",
                "      - \"${LABEL_LEVEL_NEAR_MISS}\"\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("LABEL_LEVEL_OPTION", "label:level:s0:c1,c2");
    let _ = environment.insert_sensitive("LABEL_LEVEL_NEAR_MISS", "Label:level:s0:c1,c2");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let options = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::security_options)
        .ok_or("effective security options expected")?;

    assert_eq!(options.provenance().operation(), MergeOperation::Appended);
    assert_eq!(options.value().len(), 21);
    assert_eq!(options.value()[0].value().authored(), "\"${LABEL_LEVEL_OPTION}\"");
    assert!(options.value()[0].is_sensitive());
    for index in [0, 9, 10] {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::SecurityLabelLevel { level } if level == "s0:c1,c2"
        ));
    }
    for index in 11..21 {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::SecurityLabelLevelNearMiss
        ));
    }
    assert_eq!(options.value()[20].value().authored(), "\"${LABEL_LEVEL_NEAR_MISS}\"");
    assert!(options.value()[20].is_sensitive());
    assert!(matches!(
        options.value()[6].value().kind(),
        SecurityOptionKind::SecurityLabelType { label_type } if label_type == "TYPE"
    ));
    for index in 7..9 {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::Other
        ));
    }
    assert_effective_security_label_level_diagnostics(&result);
    Ok(())
}

fn assert_effective_security_label_level_diagnostics(result: &compose_lens::project::ProjectViewResult) {
    use compose_lens::model::{
        SECURITY_OPT_APPARMOR_CONFLICT, SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT, SECURITY_OPT_SECCOMP_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT, SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT, SECURITY_OPT_SECURITY_LABEL_LEVEL_NEAR_MISS,
    };

    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT)
            .count(),
        2
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_SECURITY_LABEL_LEVEL_NEAR_MISS)
            .count(),
        10
    );
    for code in [
        SECURITY_OPT_APPARMOR_CONFLICT,
        SECURITY_OPT_SECCOMP_CONFLICT,
        SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT,
    ] {
        assert!(result.diagnostics().iter().all(|diagnostic| diagnostic.code() != code));
    }
}

#[test]
fn classifies_effective_security_label_nested_candidates_without_selecting_a_conflict()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::SecurityOptionKind;

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(716),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"${LABEL_NESTED_OPTION}\"\n",
                "      - \"apparmor=one-profile\"\n",
                "      - \"seccomp=unconfined\"\n",
                "      - \"no-new-privileges:true\"\n",
                "      - \"label:disable\"\n",
                "      - \"label:filetype:container_file_t\"\n",
                "      - \"label:level:s0:c1,c2\"\n",
                "      - \"label:type:TYPE\"\n",
                "      - \"label:user:USER\"\n",
                "      - \"label:role:ROLE\"\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(717),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"label:nested\"\n",
                "      - \"label:nested\"\n",
                "      - \"label=nested\"\n",
                "      - \"Label:nested\"\n",
                "      - \"label:Nested\"\n",
                "      - \" label:nested\"\n",
                "      - \"label : nested\"\n",
                "      - \"label:nested:true\"\n",
                "      - \"label:nested=\"\n",
                "      - \"nested\"\n",
                "      - \"${LABEL_NESTED_NEAR_MISS}\"\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("LABEL_NESTED_OPTION", "label:nested");
    let _ = environment.insert_sensitive("LABEL_NESTED_NEAR_MISS", "Label:nested");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let options = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::security_options)
        .ok_or("effective security options expected")?;

    assert_eq!(options.provenance().operation(), MergeOperation::Appended);
    assert_eq!(options.value().len(), 21);
    assert_eq!(options.value()[0].value().authored(), "\"${LABEL_NESTED_OPTION}\"");
    assert!(options.value()[0].is_sensitive());
    for index in [0, 10, 11] {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::SecurityLabelNested { enabled: true }
        ));
    }
    for index in 12..21 {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::SecurityLabelNestedNearMiss
        ));
    }
    assert_eq!(options.value()[20].value().authored(), "\"${LABEL_NESTED_NEAR_MISS}\"");
    assert!(options.value()[20].is_sensitive());
    assert!(matches!(
        options.value()[7].value().kind(),
        SecurityOptionKind::SecurityLabelType { label_type } if label_type == "TYPE"
    ));
    for index in 8..10 {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::Other
        ));
    }
    assert_effective_security_label_nested_diagnostics(&result);
    Ok(())
}

fn assert_effective_security_label_nested_diagnostics(result: &compose_lens::project::ProjectViewResult) {
    use compose_lens::model::{
        SECURITY_OPT_APPARMOR_CONFLICT, SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT, SECURITY_OPT_SECCOMP_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT, SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT, SECURITY_OPT_SECURITY_LABEL_NESTED_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_NESTED_NEAR_MISS,
    };

    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_SECURITY_LABEL_NESTED_CONFLICT)
            .count(),
        2
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_SECURITY_LABEL_NESTED_NEAR_MISS)
            .count(),
        9
    );
    for code in [
        SECURITY_OPT_APPARMOR_CONFLICT,
        SECURITY_OPT_SECCOMP_CONFLICT,
        SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT,
    ] {
        assert!(result.diagnostics().iter().all(|diagnostic| diagnostic.code() != code));
    }
}

#[test]
fn classifies_effective_security_label_type_candidates_without_selecting_a_conflict()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::SecurityOptionKind;

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(718),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"${LABEL_TYPE_OPTION}\"\n",
                "      - \"apparmor=one-profile\"\n",
                "      - \"seccomp=unconfined\"\n",
                "      - \"no-new-privileges:true\"\n",
                "      - \"label:disable\"\n",
                "      - \"label:filetype:container_file_t\"\n",
                "      - \"label:level:s0:c1,c2\"\n",
                "      - \"label:nested\"\n",
                "      - \"label:user:USER\"\n",
                "      - \"label:role:ROLE\"\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(719),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n  app:\n    security_opt:\n",
                "      - \"label:type:container_t\"\n",
                "      - \"label:type:container_t\"\n",
                "      - \"label=type:container_t\"\n",
                "      - \"label:type=container_t\"\n",
                "      - \"label=type=container_t\"\n",
                "      - \"Label:type:container_t\"\n",
                "      - \"label:Type:container_t\"\n",
                "      - \" label:type:container_t\"\n",
                "      - \"label : type : container_t\"\n",
                "      - \"label:type:container t\"\n",
                "      - \"label:type:\"\n",
                "      - \"label:type\"\n",
                "      - \"label=type\"\n",
                "      - \"type\"\n",
                "      - \"label:type:container_t:extended\"\n",
                "      - \"label:type:container_t=extended\"\n",
                "      - \"${LABEL_TYPE_NEAR_MISS}\"\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("LABEL_TYPE_OPTION", "label:type:container_t");
    let _ = environment.insert_sensitive("LABEL_TYPE_NEAR_MISS", "Label:type:container_t");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let options = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::security_options)
        .ok_or("effective security options expected")?;

    assert_eq!(options.provenance().operation(), MergeOperation::Appended);
    assert_eq!(options.value().len(), 27);
    assert_eq!(options.value()[0].value().authored(), "\"${LABEL_TYPE_OPTION}\"");
    assert!(options.value()[0].is_sensitive());
    for index in [0, 10, 11] {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::SecurityLabelType { label_type } if label_type == "container_t"
        ));
    }
    for index in 12..27 {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::SecurityLabelTypeNearMiss
        ));
    }
    assert_eq!(options.value()[26].value().authored(), "\"${LABEL_TYPE_NEAR_MISS}\"");
    assert!(options.value()[26].is_sensitive());
    for index in 1..8 {
        assert!(!matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::SecurityLabelType { .. } | SecurityOptionKind::SecurityLabelTypeNearMiss
        ));
    }
    assert_effective_security_label_type_diagnostics(&result);
    Ok(())
}

fn assert_effective_security_label_type_diagnostics(result: &compose_lens::project::ProjectViewResult) {
    use compose_lens::model::{
        SECURITY_OPT_APPARMOR_CONFLICT, SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT, SECURITY_OPT_SECCOMP_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT, SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT, SECURITY_OPT_SECURITY_LABEL_NESTED_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_TYPE_CONFLICT, SECURITY_OPT_SECURITY_LABEL_TYPE_NEAR_MISS,
    };

    for (code, expected) in [
        (SECURITY_OPT_SECURITY_LABEL_TYPE_CONFLICT, 2),
        (SECURITY_OPT_SECURITY_LABEL_TYPE_NEAR_MISS, 15),
    ] {
        assert_eq!(
            result
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.code() == code)
                .count(),
            expected
        );
    }
    for code in [
        SECURITY_OPT_APPARMOR_CONFLICT,
        SECURITY_OPT_SECCOMP_CONFLICT,
        SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_NESTED_CONFLICT,
    ] {
        assert!(result.diagnostics().iter().all(|diagnostic| diagnostic.code() != code));
    }
}

#[test]
fn exposes_merged_dns_forms_with_item_and_collection_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let result = merged_dns_project_view()?;
    let view = result.view().ok_or("partial project view expected")?;
    assert_merged_dns(view)?;
    assert_replaced_reset_and_malformed_dns(view, &result)?;
    Ok(())
}

fn merged_dns_project_view() -> Result<compose_lens::project::ProjectViewResult, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(685),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n",
                "  appended:\n    dns: [base.example, same.example]\n",
                "  scalar:\n    dns: old.example\n",
                "  cross-form:\n    dns: old.example\n",
                "  reset:\n    dns: [old.example]\n",
                "  override:\n    dns: [old.example]\n",
                "  malformed:\n    image: example.invalid/recovery:1\n    dns: [valid-before.example, true, [nested], valid-after.example]\n",
                "  bad-form:\n    image: example.invalid/bad:1\n    dns: {server: invalid.example}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(686),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n",
                "  appended:\n    dns: [same.example, later.example]\n",
                "  scalar:\n    dns: \"${DNS_SECRET}\"\n",
                "  cross-form:\n    dns: [new.example]\n",
                "  reset:\n    dns: !reset []\n",
                "  override:\n    dns: !override [same.example, same.example]\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("DNS_SECRET", "secret-resolver.example");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    Ok(build_project_view(
        merged.project().ok_or("merged project expected")?,
        None,
    ))
}

fn assert_merged_dns(view: &ProjectView) -> Result<(), Box<dyn std::error::Error>> {
    let appended = view
        .service("appended")
        .and_then(ProjectService::dns)
        .ok_or("appended DNS expected")?;
    assert_eq!(appended.provenance().operation(), MergeOperation::Appended);
    let ProjectDns::List(items) = appended.value() else {
        return Err("DNS list expected".into());
    };
    assert_eq!(
        items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>(),
        ["base.example", "same.example", "same.example", "later.example"]
    );
    assert_eq!(
        items
            .iter()
            .map(|item| item.provenance().sources().len())
            .collect::<Vec<_>>(),
        [1, 1, 1, 1]
    );

    let scalar = view
        .service("scalar")
        .and_then(ProjectService::dns)
        .ok_or("scalar DNS expected")?;
    assert_eq!(scalar.provenance().operation(), MergeOperation::Replaced);
    assert!(scalar.is_sensitive());
    assert!(matches!(scalar.value(), ProjectDns::Scalar(item)
        if item.value() == "secret-resolver.example" && item.is_sensitive()));
    assert!(!format!("{scalar:?}").contains("secret-resolver.example"));

    Ok(())
}

fn assert_replaced_reset_and_malformed_dns(
    view: &ProjectView,
    result: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let cross_form = view
        .service("cross-form")
        .and_then(ProjectService::dns)
        .ok_or("cross-form DNS expected")?;
    assert_eq!(cross_form.provenance().operation(), MergeOperation::Replaced);
    assert!(matches!(cross_form.value(), ProjectDns::List(items)
        if items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>() == ["new.example"]));

    let reset = view
        .service("reset")
        .and_then(ProjectService::dns)
        .ok_or("reset DNS expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(matches!(reset.value(), ProjectDns::List(items) if items.is_empty()));
    let overridden = view
        .service("override")
        .and_then(ProjectService::dns)
        .ok_or("override DNS expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(overridden.value(), ProjectDns::List(items)
        if items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>()
            == ["same.example", "same.example"]));

    let malformed = view.service("malformed").ok_or("malformed service retained")?;
    assert!(malformed.image().is_some());
    assert!(
        matches!(malformed.dns().map(ProjectValue::value), Some(ProjectDns::List(items))
        if items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>()
            == ["valid-before.example", "valid-after.example"])
    );
    assert!(
        view.service("bad-form")
            .is_some_and(|service| service.image().is_some())
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == DNS_EXPECTED_STRING)
            .count(),
        2
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == DNS_EXPECTED_FORM)
            .count(),
        1
    );
    Ok(())
}

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

#[test]
fn exposes_recursively_merged_logging_with_replacement_provenance_reset_override_and_sensitivity()
-> Result<(), Box<dyn std::error::Error>> {
    let result = logging_project_view()?;
    let view = result.view().ok_or("logging project view expected")?;

    assert_merged_logging(view, &result)?;
    assert_reset_override_and_malformed_logging(view)?;
    Ok(())
}

fn logging_project_view() -> Result<compose_lens::project::ProjectViewResult, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(813),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n",
                "  app:\n    logging:\n",
                "      driver: \"driver-${DRIVER}\"\n",
                "      options:\n",
                "        max-size: \"10m\"\n",
                "        max-file: 3\n",
                "        secret: \"${SECRET}\"\n",
                "        literal-${KEY}: base\n",
                "      x-evidence: base\n",
                "  reset:\n    logging: {driver: old, options: {old: value}}\n",
                "  override:\n    logging: {driver: old, options: {old: value}}\n",
                "  malformed:\n    logging: {driver: kept, options: {valid: yes, invalid: true}}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(814),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n",
                "  app:\n    logging:\n",
                "      driver: local\n",
                "      options:\n",
                "        max-size: \"20m\"\n",
                "        added: null\n",
                "      unknown: retained\n",
                "  reset:\n    logging: !reset {}\n",
                "  override:\n    logging: !override {driver: replacement, options: {only: \"1\"}}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("DRIVER", "json-file");
    let _ = environment.insert_sensitive("SECRET", "logging-secret");
    let _ = environment.insert("KEY", "must-not-change-key");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    Ok(build_project_view(
        merged.project().ok_or("merged logging project expected")?,
        None,
    ))
}

fn assert_merged_logging(
    view: &ProjectView,
    result: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let logging = view
        .service("app")
        .and_then(ProjectService::logging)
        .ok_or("effective logging expected")?;
    assert_eq!(logging.provenance().operation(), MergeOperation::Merged);
    let driver = logging.value().driver().ok_or("effective driver expected")?;
    assert_eq!(driver.value(), "local");
    assert_eq!(driver.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(driver.provenance().sources(), &[SourceId::new(813), SourceId::new(814)]);
    let options = logging.value().options().ok_or("effective options expected")?;
    assert_eq!(options.provenance().operation(), MergeOperation::Merged);
    let entries = options.value().entries();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.value().name().value())
            .collect::<Vec<_>>(),
        ["max-size", "max-file", "secret", "literal-${KEY}", "added"]
    );
    let max_size = entries
        .iter()
        .find(|entry| entry.value().name().value() == "max-size")
        .ok_or("max-size option expected")?;
    assert_eq!(
        max_size.value().value().provenance().operation(),
        MergeOperation::Replaced
    );
    assert_source_ids(
        max_size.value().value().provenance().sources(),
        &[SourceId::new(813), SourceId::new(814)],
    );
    assert!(
        matches!(max_size.value().value().value(), ProjectLoggingOptionValue::String { authored, value }
        if authored == "\"20m\"" && value == "20m")
    );
    let max_file = entries
        .iter()
        .find(|entry| entry.value().name().value() == "max-file")
        .ok_or("max-file option expected")?;
    assert!(matches!(max_file.value().value().value(), ProjectLoggingOptionValue::Number(value) if value == "3"));
    let secret = entries
        .iter()
        .find(|entry| entry.value().name().value() == "secret")
        .ok_or("secret option expected")?;
    assert!(secret.is_sensitive());
    assert!(
        matches!(secret.value().value().value(), ProjectLoggingOptionValue::String { authored, value }
        if authored == "\"${SECRET}\"" && value == "logging-secret")
    );
    assert!(!format!("{result:?}").contains("logging-secret"));
    assert_eq!(logging.value().unmodeled_fields().len(), 2);
    Ok(())
}

fn assert_reset_override_and_malformed_logging(view: &ProjectView) -> Result<(), Box<dyn std::error::Error>> {
    let reset = view
        .service("reset")
        .and_then(ProjectService::logging)
        .ok_or("reset logging expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.value().driver().is_none() && reset.value().options().is_none());
    let overridden = view
        .service("override")
        .and_then(ProjectService::logging)
        .ok_or("override logging expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        overridden.value().driver().map(ProjectValue::value).map(String::as_str),
        Some("replacement")
    );
    let malformed = view
        .service("malformed")
        .and_then(ProjectService::logging)
        .ok_or("malformed logging expected")?;
    assert_eq!(
        malformed.value().driver().map(ProjectValue::value).map(String::as_str),
        Some("kept")
    );
    assert_eq!(
        malformed
            .value()
            .options()
            .map(ProjectValue::value)
            .map(|options| options.entries().len()),
        Some(1)
    );
    assert_eq!(
        malformed
            .value()
            .options()
            .map(ProjectValue::value)
            .map(|options| options.unmodeled_entries().len()),
        Some(1)
    );
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
fn retains_effective_pull_policy_replacement_provenance_sensitivity_and_pull_refresh_after()
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
        .pull_refresh_after()
        .ok_or("effective pull refresh interval expected")?;
    assert_eq!(refresh.value(), "12h");
    assert_eq!(refresh.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(
        refresh.provenance().sources(),
        &[SourceId::new(660), SourceId::new(661)],
    );
    Ok(())
}

#[test]
fn retains_effective_pull_refresh_after_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>>
{
    let base_id = SourceId::new(3202);
    let override_id = SourceId::new(3203);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("base", "workspace"),
            "services:\n  replaced:\n    pull_refresh_after: first\n  sensitive:\n    pull_refresh_after: \"${REFRESH_AFTER}\"\n  reset:\n    pull_refresh_after: first\n  override:\n    pull_refresh_after: first\n  malformed:\n    pull_refresh_after: first\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  replaced:\n    pull_refresh_after: second\n  reset:\n    pull_refresh_after: !reset null\n  override:\n    pull_refresh_after: !override replacement\n  malformed:\n    pull_refresh_after: 1\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("REFRESH_AFTER", "private-refresh");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let pull_refresh_after = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::pull_refresh_after)
            .ok_or("pull refresh interval")
    };
    let replaced = pull_refresh_after("replaced")?;
    assert_eq!(replaced.value(), "second");
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    let sensitive = pull_refresh_after("sensitive")?;
    assert!(sensitive.is_sensitive());
    assert!(!format!("{sensitive:?}").contains("private-refresh"));
    assert!(
        result
            .view()
            .and_then(|view| view.service("reset"))
            .is_some_and(|service| service.pull_refresh_after().is_none())
    );
    let overridden = pull_refresh_after("override")?;
    assert_eq!(overridden.value(), "replacement");
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(
        result
            .view()
            .and_then(|view| view.service("malformed"))
            .is_some_and(|service| service.pull_refresh_after().is_none())
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
fn retains_effective_platform_provenance_sensitivity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3209);
    let override_id = SourceId::new(3210);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("base", "workspace"),
            "services:\n  replaced:\n    platform: first\n  sensitive:\n    platform: \"${PLATFORM}\"\n  reset:\n    platform: first\n  override:\n    platform: first\n  malformed:\n    platform: first\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("override", "workspace"),
            "services:\n  replaced:\n    platform: second\n  reset:\n    platform: !reset null\n  override:\n    platform: !override replacement\n  malformed:\n    platform: 1\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("PLATFORM", "private-platform");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let platform = |name| {
        result
            .view()
            .and_then(|view| view.service(name))
            .and_then(ProjectService::platform)
            .ok_or("platform")
    };
    let replaced = platform("replaced")?;
    assert_eq!(replaced.value(), "second");
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    let sensitive = platform("sensitive")?;
    assert!(sensitive.is_sensitive());
    assert!(!format!("{sensitive:?}").contains("private-platform"));
    assert!(
        result
            .view()
            .and_then(|view| view.service("reset"))
            .is_some_and(|service| service.platform().is_none())
    );
    let overridden = platform("override")?;
    assert_eq!(overridden.value(), "replacement");
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(
        result
            .view()
            .and_then(|view| view.service("malformed"))
            .is_some_and(|service| service.platform().is_none())
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

#[test]
fn retains_merged_stdin_open_replacement_and_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(685);
    let override_id = SourceId::new(686);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            "services:\n  app:\n    stdin_open: false\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            "services:\n  app:\n    stdin_open: true\n",
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let stdin_open = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::stdin_open)
        .ok_or("effective stdin_open expected")?;

    assert_eq!(stdin_open.value(), &BooleanValue::Literal(true));
    assert_eq!(stdin_open.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(stdin_open.provenance().sources(), &[base_id, override_id]);

    Ok(())
}

#[test]
fn retains_merged_tty_replacement_and_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(688);
    let override_id = SourceId::new(689);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            "services:\n  app:\n    tty: false\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            "services:\n  app:\n    tty: true\n",
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let tty = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::tty)
        .ok_or("effective tty expected")?;

    assert_eq!(tty.value(), &BooleanValue::Literal(true));
    assert_eq!(tty.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(tty.provenance().sources(), &[base_id, override_id]);

    Ok(())
}

#[test]
fn retains_merged_privileged_replacement_and_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(692);
    let override_id = SourceId::new(693);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            "services:\n  app:\n    privileged: false\n",
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            "services:\n  app:\n    privileged: true\n",
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let privileged = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::privileged)
        .ok_or("effective privileged expected")?;

    assert_eq!(privileged.value(), &BooleanValue::Literal(true));
    assert_eq!(privileged.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(privileged.provenance().sources(), &[base_id, override_id]);

    Ok(())
}

#[test]
fn retains_effective_attach_replacement_reset_override_sensitivity_and_malformed_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3215);
    let override_id = SourceId::new(3216);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    attach: false\n",
                "  reset:\n    attach: true\n",
                "  override:\n    attach: false\n",
                "  sensitive:\n    attach: \"${ATTACH_SECRET}\"\n",
                "  malformed:\n    attach: [true]\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    attach: true\n",
                "  reset:\n    attach: !reset null\n",
                "  override:\n    attach: !override true\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("ATTACH_SECRET", "false");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("partial project view expected")?;

    let replaced = view
        .service("replaced")
        .and_then(ProjectService::attach)
        .ok_or("replaced attach expected")?;
    assert_eq!(replaced.value(), &BooleanValue::Literal(true));
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);

    let reset = view.service("reset").ok_or("reset service expected")?;
    assert!(reset.attach().is_none());
    assert!(
        reset
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "reset", "attach"])
    );

    let overridden = view
        .service("override")
        .and_then(ProjectService::attach)
        .ok_or("overridden attach expected")?;
    assert_eq!(overridden.value(), &BooleanValue::Literal(true));
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_source_ids(overridden.provenance().sources(), &[base_id, override_id, override_id]);

    let sensitive = view
        .service("sensitive")
        .and_then(ProjectService::attach)
        .ok_or("sensitive attach expected")?;
    assert_eq!(sensitive.value(), &BooleanValue::Literal(false));
    assert!(sensitive.is_sensitive());
    assert!(!format!("{sensitive:?}").contains("false"));

    let malformed = view.service("malformed").ok_or("malformed service expected")?;
    assert!(malformed.attach().is_none());
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "malformed", "attach"])
    );
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
fn retains_effective_build_context_forms_interpolation_provenance_and_unmodeled_siblings()
-> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(694);
    let override_id = SourceId::new(695);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  short:\n    build: \"${SHORT_CONTEXT}\"\n",
                "  long:\n",
                "    build:\n",
                "      context: \"${LONG_CONTEXT}\"\n",
                "      dockerfile: Dockerfile\n",
                "  merged:\n",
                "    build:\n",
                "      context: ./base\n",
                "      dockerfile: Basefile\n",
                "      target: base-stage\n",
                "  reset:\n    build: {context: ./reset}\n",
                "  overridden:\n    build: {context: ./old}\n",
                "  malformed:\n    build: {context: [], dockerfile: Basefile, target: base-stage}\n",
                "  conflicting:\n    build: {dockerfile: Dockerfile, dockerfile_inline: FROM scratch}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  merged:\n    build: {context: \"${MERGED_CONTEXT}\", dockerfile: \"${MERGED_DOCKERFILE}\", target: \"${MERGED_TARGET}\"}\n",
                "  reset:\n    build: !reset {}\n",
                "  overridden:\n    build: !override {context: ./new, dockerfile: Dockerfile.release, target: \"\"}\n",
                "  malformed:\n    build: {dockerfile: [], target: []}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("SHORT_CONTEXT", "private-short");
    let _ = environment.insert_sensitive("LONG_CONTEXT", "private-long");
    let _ = environment.insert_sensitive("MERGED_CONTEXT", "private-merged");
    let _ = environment.insert_sensitive("MERGED_DOCKERFILE", "private-Dockerfile");
    let _ = environment.insert_sensitive("MERGED_TARGET", "private-target");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    assert_effective_build_contexts(&result, base_id, override_id)
}

fn assert_effective_build_contexts(
    result: &compose_lens::project::ProjectViewResult,
    base_id: SourceId,
    override_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let view = result.view().ok_or("project view expected")?;

    let short = view
        .service("short")
        .and_then(ProjectService::build)
        .ok_or("short build expected")?;
    let ProjectBuild::Context(context) = short.value() else {
        return Err("short build context expected".into());
    };
    assert_eq!(context.value(), "private-short");
    assert!(context.is_sensitive() && short.is_sensitive());
    assert_eq!(short.provenance().operation(), MergeOperation::Authored);
    assert_source_ids(short.provenance().sources(), &[base_id]);
    assert!(!format!("{short:?}").contains("private-short"));

    let long = view
        .service("long")
        .and_then(ProjectService::build)
        .ok_or("long build expected")?;
    assert!(long.is_sensitive());
    let ProjectBuild::Definition(long) = long.value() else {
        return Err("long build definition expected".into());
    };
    let long_context = long.context().ok_or("long build context expected")?;
    assert_eq!(long_context.value(), "private-long");
    assert!(long_context.is_sensitive());
    assert_eq!(
        long.dockerfile().map(ProjectValue::value).map(String::as_str),
        Some("Dockerfile")
    );
    assert!(long.unmodeled_fields().is_empty());

    let merged = view
        .service("merged")
        .and_then(ProjectService::build)
        .ok_or("merged build expected")?;
    assert!(merged.is_sensitive());
    assert_eq!(merged.provenance().operation(), MergeOperation::Merged);
    assert_source_ids(merged.provenance().sources(), &[base_id, override_id]);
    let ProjectBuild::Definition(merged) = merged.value() else {
        return Err("merged build definition expected".into());
    };
    let merged_context = merged.context().ok_or("merged context expected")?;
    assert_eq!(merged_context.value(), "private-merged");
    assert!(merged_context.is_sensitive());
    assert_eq!(merged_context.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(merged_context.provenance().sources(), &[base_id, override_id]);
    let merged_dockerfile = merged.dockerfile().ok_or("merged Dockerfile expected")?;
    assert_eq!(merged_dockerfile.value(), "private-Dockerfile");
    assert!(merged_dockerfile.is_sensitive());
    assert_eq!(merged_dockerfile.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(merged_dockerfile.provenance().sources(), &[base_id, override_id]);
    let merged_target = merged.target().ok_or("merged target expected")?;
    assert_eq!(merged_target.value(), "private-target");
    assert!(merged_target.is_sensitive());
    assert_eq!(merged_target.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(merged_target.provenance().sources(), &[base_id, override_id]);

    let reset = view
        .service("reset")
        .and_then(ProjectService::build)
        .ok_or("reset build expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(matches!(reset.value(), ProjectBuild::Definition(definition) if definition.context().is_none()));

    let overridden = view
        .service("overridden")
        .and_then(ProjectService::build)
        .ok_or("overridden build expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    let ProjectBuild::Definition(overridden) = overridden.value() else {
        return Err("overridden build definition expected".into());
    };
    assert_eq!(
        overridden.context().map(ProjectValue::value).map(String::as_str),
        Some("./new")
    );
    assert_eq!(
        overridden.dockerfile().map(ProjectValue::value).map(String::as_str),
        Some("Dockerfile.release")
    );
    assert_eq!(
        overridden.target().map(ProjectValue::value).map(String::as_str),
        Some("")
    );
    assert!(overridden.unmodeled_fields().is_empty());

    assert_malformed_and_conflicting_builds(view, result, base_id, override_id)
}

fn assert_malformed_and_conflicting_builds(
    view: &ProjectView,
    result: &compose_lens::project::ProjectViewResult,
    base_id: SourceId,
    override_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let malformed = view
        .service("malformed")
        .and_then(ProjectService::build)
        .ok_or("partial malformed build expected")?;
    assert!(matches!(malformed.value(), ProjectBuild::Definition(definition)
        if definition.context().is_none() && definition.dockerfile().is_none() && definition.target().is_none() && definition.unmodeled_fields().len() == 3));
    let ProjectBuild::Definition(malformed) = malformed.value() else {
        return Err("malformed build definition expected".into());
    };
    let malformed_dockerfile = malformed
        .unmodeled_fields()
        .iter()
        .find(|field| field.path() == ["services", "malformed", "dockerfile"])
        .ok_or("malformed Dockerfile evidence expected")?;
    assert_eq!(malformed_dockerfile.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(malformed_dockerfile.provenance().sources(), &[base_id, override_id]);
    let malformed_target = malformed
        .unmodeled_fields()
        .iter()
        .find(|field| field.path() == ["services", "malformed", "target"])
        .ok_or("malformed target evidence expected")?;
    assert_eq!(malformed_target.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(malformed_target.provenance().sources(), &[base_id, override_id]);
    let conflicting = view
        .service("conflicting")
        .and_then(ProjectService::build)
        .ok_or("conflicting build expected")?;
    let ProjectBuild::Definition(conflicting) = conflicting.value() else {
        return Err("conflicting build definition expected".into());
    };
    assert_eq!(
        conflicting.dockerfile().map(ProjectValue::value).map(String::as_str),
        Some("Dockerfile")
    );
    assert_eq!(
        conflicting
            .dockerfile_inline()
            .map(ProjectValue::value)
            .map(String::as_str),
        Some("FROM scratch")
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == BUILD_DOCKERFILE_INLINE_CONFLICT && diagnostic.labels().len() == 2)
    );
    Ok(())
}

#[test]
fn retains_effective_build_tags_order_duplicates_and_partial_malformed_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(722);
    let override_id = SourceId::new(723);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  appended:\n    build:\n      tags:\n        - \"example/app:${BASE_TAG}\"\n        - example/app:shared\n        - example/app:shared\n",
                "  reset:\n    build:\n      tags:\n        - example/app:old\n",
                "  overridden:\n    build:\n      tags:\n        - example/app:old\n",
                "  malformed:\n    build:\n      tags:\n        - example/app:base\n        - {}\n",
                "  nonsequence:\n    build:\n      tags: {}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  appended:\n    build:\n      tags:\n        - example/app:shared\n        - \"example/app:${NEXT_TAG}\"\n",
                "  reset:\n    build:\n      tags: !reset []\n",
                "  overridden:\n    build:\n      tags: !override [example/app:new, example/app:new]\n",
                "  malformed:\n    build:\n      tags:\n        - \"example/app:${MALFORMED_TAG}\"\n        - []\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("BASE_TAG", "private-base");
    let _ = environment.insert_sensitive("NEXT_TAG", "private-next");
    let _ = environment.insert_sensitive("MALFORMED_TAG", "private-malformed");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    let appended = build_tags(view, "appended")?;
    assert_appended_build_tags(appended, base_id, override_id);
    let reset = build_tags(view, "reset")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.value().is_empty());
    let overridden = build_tags(view, "overridden")?;
    assert_overridden_build_tags(overridden);
    let malformed = build_definition(view, "malformed")?;
    assert_malformed_build_tags(malformed, base_id, override_id)?;
    let nonsequence = build_definition(view, "nonsequence")?;
    assert!(nonsequence.tags().is_none());
    assert!(
        nonsequence
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "nonsequence", "tags"])
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

fn assert_appended_build_tags(
    tags: &ProjectValue<Vec<ProjectValue<String>>>,
    base_id: SourceId,
    override_id: SourceId,
) {
    assert_eq!(tags.provenance().operation(), MergeOperation::Appended);
    assert_source_ids(tags.provenance().sources(), &[base_id, override_id]);
    assert!(tags.is_sensitive());
    assert_eq!(
        tags.value()
            .iter()
            .map(ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "example/app:private-base",
            "example/app:shared",
            "example/app:shared",
            "example/app:shared",
            "example/app:private-next",
        ]
    );
    assert!(tags.value()[0].is_sensitive());
    assert_eq!(tags.value()[3].provenance().operation(), MergeOperation::Added);
    assert_source_ids(tags.value()[3].provenance().sources(), &[override_id]);
    assert!(!format!("{tags:?}").contains("private-base"));
}

fn assert_overridden_build_tags(tags: &ProjectValue<Vec<ProjectValue<String>>>) {
    assert_eq!(tags.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        tags.value()
            .iter()
            .map(ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["example/app:new", "example/app:new"]
    );
}

fn assert_malformed_build_tags(
    definition: &compose_lens::project::ProjectBuildDefinition,
    base_id: SourceId,
    override_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let tags = definition.tags().ok_or("partial malformed build tags expected")?;
    assert_eq!(tags.provenance().operation(), MergeOperation::Appended);
    assert_eq!(
        tags.value()
            .iter()
            .map(ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["example/app:base", "example/app:private-malformed"]
    );
    assert!(tags.value()[1].is_sensitive());
    let evidence = definition
        .unmodeled_fields()
        .iter()
        .find(|field| field.path() == ["services", "malformed", "tags"])
        .ok_or("malformed build tags evidence expected")?;
    assert_eq!(evidence.provenance().operation(), MergeOperation::Appended);
    assert_source_ids(evidence.provenance().sources(), &[base_id, override_id]);
    assert!(evidence.is_sensitive());
    Ok(())
}

#[test]
fn retains_effective_build_platforms_order_duplicates_and_partial_malformed_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(7230);
    let override_id = SourceId::new(7231);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  appended:\n    build:\n      platforms:\n        - \"${BASE_PLATFORM}\"\n        - linux/amd64\n        - linux/amd64\n",
                "  empty:\n    build:\n      platforms: []\n",
                "  omitted:\n    build: {context: .}\n",
                "  reset:\n    build:\n      platforms: [linux/amd64]\n",
                "  overridden:\n    build:\n      platforms: [linux/amd64]\n",
                "  malformed:\n    build:\n      platforms: [linux/arm64, {}]\n",
                "  nonsequence:\n    build:\n      platforms: {}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  appended:\n    build:\n      platforms: [linux/amd64, \"${NEXT_PLATFORM}\"]\n",
                "  reset:\n    build:\n      platforms: !reset []\n",
                "  overridden:\n    build:\n      platforms: !override [linux/arm64, linux/arm64]\n",
                "  malformed:\n    build:\n      platforms: [\"${MALFORMED_PLATFORM}\", []]\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("BASE_PLATFORM", "private-base");
    let _ = environment.insert_sensitive("NEXT_PLATFORM", "private-next");
    let _ = environment.insert_sensitive("MALFORMED_PLATFORM", "private-malformed");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert_appended_build_platforms(build_platforms(view, "appended")?, base_id, override_id);

    let empty = build_platforms(view, "empty")?;
    assert!(empty.value().is_empty());
    assert_eq!(empty.provenance().operation(), MergeOperation::Authored);
    assert!(build_definition(view, "omitted")?.platforms().is_none());
    let reset = build_platforms(view, "reset")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.value().is_empty());
    let overridden = build_platforms(view, "overridden")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        overridden
            .value()
            .iter()
            .map(ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["linux/arm64", "linux/arm64"]
    );
    let malformed = build_definition(view, "malformed")?;
    let recovered = malformed.platforms().ok_or("recovered build platforms expected")?;
    assert_eq!(recovered.provenance().operation(), MergeOperation::Appended);
    assert_eq!(
        recovered
            .value()
            .iter()
            .map(ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["linux/arm64", "private-malformed"]
    );
    assert!(recovered.value()[1].is_sensitive());
    let evidence = malformed
        .unmodeled_fields()
        .iter()
        .find(|field| field.path() == ["services", "malformed", "platforms"])
        .ok_or("malformed build platforms evidence expected")?;
    assert_eq!(evidence.provenance().operation(), MergeOperation::Appended);
    assert_source_ids(evidence.provenance().sources(), &[base_id, override_id]);
    assert!(evidence.is_sensitive());
    let nonsequence = build_definition(view, "nonsequence")?;
    assert!(nonsequence.platforms().is_none());
    assert!(
        nonsequence
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "nonsequence", "platforms"])
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

fn assert_appended_build_platforms(
    platforms: &ProjectValue<Vec<ProjectValue<String>>>,
    base_id: SourceId,
    override_id: SourceId,
) {
    assert_eq!(platforms.provenance().operation(), MergeOperation::Appended);
    assert_source_ids(platforms.provenance().sources(), &[base_id, override_id]);
    assert!(platforms.is_sensitive());
    assert_eq!(
        platforms
            .value()
            .iter()
            .map(ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "private-base",
            "linux/amd64",
            "linux/amd64",
            "linux/amd64",
            "private-next"
        ]
    );
    assert!(platforms.value()[0].is_sensitive());
    assert_eq!(platforms.value()[0].provenance().operation(), MergeOperation::Authored);
    assert_eq!(
        platforms.value()[0]
            .effective_source()
            .map(compose_lens::source::SourceSpan::source_id),
        Some(base_id)
    );
    assert_eq!(platforms.value()[3].provenance().operation(), MergeOperation::Added);
    assert_eq!(
        platforms.value()[3]
            .effective_source()
            .map(compose_lens::source::SourceSpan::source_id),
        Some(override_id)
    );
    assert!(platforms.value()[4].is_sensitive());
    assert!(!format!("{platforms:?}").contains("private-base"));
}

#[test]
fn retains_effective_build_secrets_with_grant_provenance_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(7240);
    let override_id = SourceId::new(7241);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "secrets:\n  short-secret: {}\n  long-secret: {}\n  override-secret: {}\n",
                "services:\n",
                "  appended:\n    build:\n      secrets:\n",
                "        - \"${BUILD_SECRET}\"\n",
                "        - source: long-secret\n          target: /run/secrets/build\n          uid: \"01000\"\n          gid: 1001\n          mode: \"0440\"\n",
                "        - short-secret\n",
                "  reset:\n    build: {secrets: [short-secret]}\n",
                "  overridden:\n    build: {secrets: [short-secret]}\n",
                "  malformed:\n    build:\n      secrets:\n        - source: []\n          target: /run/secrets/bad\n        - short-secret\n        - []\n",
                "  nonsequence:\n    build: {secrets: {source: short-secret}}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  appended:\n    build:\n      secrets:\n",
                "        - short-secret\n",
                "        - source: override-secret\n          target: /run/secrets/override\n          uid: \"2000\"\n          gid: \"2001\"\n          mode: 0400\n",
                "  reset:\n    build: {secrets: !reset []}\n",
                "  overridden:\n    build: {secrets: !override [override-secret, override-secret]}\n",
                "  malformed:\n    build:\n      secrets:\n        - source: \"${BUILD_LONG_SECRET}\"\n          target: /run/secrets/later\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("BUILD_SECRET", "private-short-secret");
    let _ = environment.insert_sensitive("BUILD_LONG_SECRET", "private-long-secret");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    assert_effective_build_secrets(&result, base_id, override_id)
}

fn assert_effective_build_secrets(
    result: &compose_lens::project::ProjectViewResult,
    base_id: SourceId,
    override_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let view = result.view().ok_or("project view expected")?;

    assert!(
        view.secrets()
            .iter()
            .any(|secret| secret.name().value() == "long-secret")
    );
    let appended = build_definition(view, "appended")?
        .secrets()
        .ok_or("appended build secrets expected")?;
    assert_eq!(appended.provenance().operation(), MergeOperation::Appended);
    assert_source_ids(appended.provenance().sources(), &[base_id, override_id]);
    assert!(appended.is_sensitive());
    assert_eq!(appended.value().len(), 5);
    assert!(matches!(appended.value()[0].value(), ProjectGrant::Short(value) if value == "private-short-secret"));
    assert!(appended.value()[0].is_sensitive());
    let ProjectGrant::Long(long) = appended.value()[1].value() else {
        return Err("long build secret expected".into());
    };
    assert_eq!(
        long.source().map(ProjectValue::value).map(String::as_str),
        Some("long-secret")
    );
    assert_eq!(
        long.target().map(ProjectValue::value).map(String::as_str),
        Some("/run/secrets/build")
    );
    assert_eq!(long.uid().map(ProjectValue::value).map(String::as_str), Some("01000"));
    assert_eq!(long.gid().map(ProjectValue::value).map(String::as_str), Some("1001"));
    assert_eq!(long.mode().map(ProjectValue::value).map(String::as_str), Some("0440"));
    assert_eq!(appended.value()[3].provenance().operation(), MergeOperation::Added);
    assert!(!format!("{appended:?}").contains("private-short-secret"));

    let reset = build_definition(view, "reset")?
        .secrets()
        .ok_or("reset build secrets expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.value().is_empty());
    let overridden = build_definition(view, "overridden")?
        .secrets()
        .ok_or("overridden build secrets expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(overridden.value().as_slice(), [first, second]
        if matches!(first.value(), ProjectGrant::Short(value) if value == "override-secret")
            && matches!(second.value(), ProjectGrant::Short(value) if value == "override-secret")));

    let malformed = build_definition(view, "malformed")?;
    let malformed_secrets = malformed.secrets().ok_or("partial malformed build secrets expected")?;
    assert_eq!(malformed_secrets.value().len(), 3);
    assert!(malformed_secrets.is_sensitive());
    assert!(matches!(malformed_secrets.value()[0].value(), ProjectGrant::Long(long) if long.source().is_none()));
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "malformed", "secrets"])
    );
    let nonsequence = build_definition(view, "nonsequence")?;
    assert!(nonsequence.secrets().is_none());
    assert!(
        nonsequence
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "nonsequence", "secrets"])
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_MISSING_FIELD)
    );
    Ok(())
}

#[test]
fn retains_effective_build_label_mapping_replacement_and_malformed_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let base_id = SourceId::new(726);
    let override_id = SourceId::new(727);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n  app:\n    build:\n      labels:\n",
                "        public: old\n        private: \"${PRIVATE_BASE}\"\n        malformed: []\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            "services:\n  app:\n    build:\n      labels:\n        public: new\n        private: \"${PRIVATE_OVERRIDE}\"\n",
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("PRIVATE_BASE", "private-base");
    let _ = environment.insert_sensitive("PRIVATE_OVERRIDE", "private-override");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let definition = build_definition(result.view().ok_or("project view expected")?, "app")?;
    let labels = definition.labels().ok_or("build labels expected")?;
    let Some(entries) = labels.value().as_map() else {
        return Err("mapping build labels expected".into());
    };
    assert_eq!(labels.provenance().operation(), MergeOperation::Merged);
    assert!(labels.is_sensitive());
    assert_eq!(entries.len(), 2);
    let public = entries
        .iter()
        .find(|entry| entry.name().value() == "public")
        .ok_or("public label expected")?;
    assert_eq!(public.value().value(), &ComposeScalar::String("new".to_owned()));
    assert_eq!(public.value().provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(public.value().provenance().sources(), &[base_id, override_id]);
    let private = entries
        .iter()
        .find(|entry| entry.name().value() == "private")
        .ok_or("private label expected")?;
    assert!(private.value().is_sensitive());
    assert_eq!(
        private.value().value(),
        &ComposeScalar::String("private-override".to_owned())
    );
    assert!(
        definition
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "app", "labels"])
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    assert!(!format!("{labels:?}").contains("private-override"));
    Ok(())
}

#[test]
fn retains_effective_build_label_list_append_reset_override_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(728);
    let override_id = SourceId::new(729);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  appended:\n    build:\n      labels: [base=one, bare, base=one]\n",
                "  reset:\n    build:\n      labels: [old]\n",
                "  overridden:\n    build:\n      labels: [old]\n",
                "  malformed:\n    build:\n      labels:\n        - base\n        - {}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  appended:\n    build:\n      labels: [\"next=${NEXT}\", bare]\n",
                "  reset:\n    build:\n      labels: !reset []\n",
                "  overridden:\n    build:\n      labels: !override [new, new]\n",
                "  malformed:\n    build:\n      labels:\n        - \"later=${LATER}\"\n        - []\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("NEXT", "private-next");
    let _ = environment.insert_sensitive("LATER", "private-later");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;
    assert_build_label_list(
        build_definition(view, "appended")?
            .labels()
            .ok_or("appended labels expected")?,
        MergeOperation::Appended,
        ["base=one", "bare", "base=one", "next=private-next", "bare"],
    )?;
    assert_build_label_list(
        build_definition(view, "reset")?
            .labels()
            .ok_or("reset labels expected")?,
        MergeOperation::Reset,
        [],
    )?;
    assert_build_label_list(
        build_definition(view, "overridden")?
            .labels()
            .ok_or("overridden labels expected")?,
        MergeOperation::Override,
        ["new", "new"],
    )?;
    let malformed = build_definition(view, "malformed")?;
    let labels = malformed.labels().ok_or("partial malformed labels expected")?;
    assert_build_label_list(labels, MergeOperation::Appended, ["base", "later=private-later"])?;
    assert!(labels.is_sensitive() && labels.value().as_list().is_some_and(|values| values[1].is_sensitive()));
    assert_source_ids(labels.provenance().sources(), &[base_id, override_id]);
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "malformed", "labels"])
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

fn assert_build_label_list<const N: usize>(
    labels: &ProjectValue<ProjectBuildLabels>,
    operation: MergeOperation,
    expected: [&str; N],
) -> Result<(), Box<dyn std::error::Error>> {
    let values = labels.value().as_list().ok_or("list build labels expected")?;
    assert_eq!(labels.provenance().operation(), operation);
    assert_eq!(
        values
            .iter()
            .map(ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        expected
    );
    Ok(())
}

#[test]
fn retains_effective_build_network_interpolation_and_merge_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(724);
    let override_id = SourceId::new(725);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    build:\n      network: base-network\n",
                "  reset:\n    build:\n      network: old-network\n",
                "  overridden:\n    build:\n      network: old-network\n",
                "  malformed:\n    build:\n      network: []\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    build:\n      network: \"${BUILD_NETWORK}\"\n",
                "  reset:\n    build:\n      network: !reset null\n",
                "  overridden:\n    build:\n      network: !override \"\"\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("BUILD_NETWORK", "private-network");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    let replaced = build_definition(view, "replaced")?
        .network()
        .ok_or("replaced build network expected")?;
    assert_eq!(replaced.value(), "private-network");
    assert!(replaced.is_sensitive());
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    assert!(!format!("{replaced:?}").contains("private-network"));

    let reset = build_definition(view, "reset")?;
    assert!(reset.network().is_none());
    let reset_evidence = reset
        .unmodeled_fields()
        .iter()
        .find(|field| field.path() == ["services", "reset", "network"])
        .ok_or("reset build network evidence expected")?;
    assert_eq!(reset_evidence.provenance().operation(), MergeOperation::Reset);

    let overridden = build_definition(view, "overridden")?
        .network()
        .ok_or("overridden build network expected")?;
    assert_eq!(overridden.value(), "");
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_source_ids(overridden.provenance().sources(), &[base_id, override_id, override_id]);

    let malformed = build_definition(view, "malformed")?;
    assert!(malformed.network().is_none());
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "malformed", "network"])
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
fn retains_effective_build_pull_replacement_reset_override_and_malformed_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(832);
    let override_id = SourceId::new(833);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    build: {pull: true}\n",
                "  reset:\n    build: {pull: true}\n",
                "  overridden:\n    build: {pull: false}\n",
                "  expression:\n    build: {pull: \"${BUILD_PULL}\"}\n",
                "  malformed:\n    build: {pull: nope}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    build: {pull: false}\n",
                "  reset:\n    build: {pull: !reset null}\n",
                "  overridden:\n    build: {pull: !override true}\n",
            ),
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    let replaced = build_definition(view, "replaced")?
        .pull()
        .ok_or("replaced build pull expected")?;
    assert_eq!(replaced.value(), &BooleanValue::Literal(false));
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);

    let reset = build_definition(view, "reset")?;
    assert!(reset.pull().is_none());
    let reset_evidence = reset
        .unmodeled_fields()
        .iter()
        .find(|field| field.path() == ["services", "reset", "pull"])
        .ok_or("reset build pull evidence expected")?;
    assert_eq!(reset_evidence.provenance().operation(), MergeOperation::Reset);

    let overridden = build_definition(view, "overridden")?
        .pull()
        .ok_or("overridden build pull expected")?;
    assert_eq!(overridden.value(), &BooleanValue::Literal(true));
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_source_ids(overridden.provenance().sources(), &[base_id, override_id, override_id]);

    let expression = build_definition(view, "expression")?
        .pull()
        .ok_or("deferred build pull expected")?;
    assert_eq!(
        expression.value(),
        &BooleanValue::Expression("${BUILD_PULL}".to_owned())
    );

    let malformed = build_definition(view, "malformed")?;
    assert!(malformed.pull().is_none());
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "malformed", "pull"])
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_INVALID_VALUE)
    );
    Ok(())
}

#[test]
fn retains_effective_build_no_cache_type_interpolation_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(843);
    let override_id = SourceId::new(844);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  boolean:\n    build: {no_cache: true}\n",
                "  string:\n    build: {no_cache: \"false\"}\n",
                "  expression:\n    build: {no_cache: \"${NO_CACHE}\"}\n",
                "  empty:\n    build: {no_cache: \"\"}\n",
                "  replaced:\n    build: {no_cache: true}\n",
                "  reset:\n    build: {no_cache: true}\n",
                "  overridden:\n    build: {no_cache: false}\n",
                "  invalid-null:\n    build: {context: retained, no_cache: null}\n",
                "  invalid-number:\n    build: {context: retained, no_cache: 1}\n",
                "  invalid-mapping:\n    build: {context: retained, no_cache: {invalid: value}}\n",
                "  invalid-sequence:\n    build: {context: retained, no_cache: [true]}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    build: {no_cache: \"true\"}\n",
                "  reset:\n    build: {no_cache: !reset null}\n",
                "  overridden:\n    build: {no_cache: !override \"${SECRET}\"}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("NO_CACHE", "false");
    let _ = environment.insert_sensitive("SECRET", "private-cache-choice");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert!(matches!(
        build_definition(view, "boolean")?.no_cache().map(ProjectValue::value),
        Some(BuildNoCache::Boolean(true))
    ));
    assert!(
        matches!(build_definition(view, "string")?.no_cache().map(ProjectValue::value), Some(BuildNoCache::String(value)) if value == "false")
    );
    assert!(
        matches!(build_definition(view, "expression")?.no_cache().map(ProjectValue::value), Some(BuildNoCache::String(value)) if value == "false")
    );
    assert!(
        matches!(build_definition(view, "empty")?.no_cache().map(ProjectValue::value), Some(BuildNoCache::String(value)) if value.is_empty())
    );

    let replaced = build_definition(view, "replaced")?
        .no_cache()
        .ok_or("replaced no_cache expected")?;
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    assert!(matches!(replaced.value(), BuildNoCache::String(value) if value == "true"));

    let reset = build_definition(view, "reset")?;
    assert!(reset.no_cache().is_none());
    let reset_evidence = reset
        .unmodeled_fields()
        .iter()
        .find(|field| field.path() == ["services", "reset", "no_cache"])
        .ok_or("reset no_cache evidence expected")?;
    assert_eq!(reset_evidence.provenance().operation(), MergeOperation::Reset);

    let overridden = build_definition(view, "overridden")?
        .no_cache()
        .ok_or("overridden no_cache expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_source_ids(overridden.provenance().sources(), &[base_id, override_id, override_id]);
    assert!(matches!(overridden.value(), BuildNoCache::String(value) if value == "private-cache-choice"));
    assert!(overridden.is_sensitive());
    assert!(!format!("{overridden:?}").contains("private-cache-choice"));

    assert_invalid_build_no_cache_recovery(view)?;
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
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

#[test]
fn retains_effective_build_sbom_type_interpolation_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(8422);
    let override_id = SourceId::new(8423);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  boolean:\n    build: {sbom: true}\n",
                "  generator:\n    build: {sbom: \"generator=base\"}\n",
                "  expression:\n    build: {sbom: \"${SBOM_GENERATOR}\"}\n",
                "  empty:\n    build: {sbom: \"\"}\n",
                "  replaced:\n    build: {sbom: false}\n",
                "  reset:\n    build: {sbom: true}\n",
                "  overridden:\n    build: {sbom: false}\n",
                "  invalid-null:\n    build: {context: retained, sbom: null}\n",
                "  invalid-number:\n    build: {context: retained, sbom: 1}\n",
                "  invalid-mapping:\n    build: {context: retained, sbom: {invalid: value}}\n",
                "  invalid-sequence:\n    build: {context: retained, sbom: [true]}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    build: {sbom: \"generator=override\"}\n",
                "  reset:\n    build: {sbom: !reset null}\n",
                "  overridden:\n    build: {sbom: !override \"${SBOM_SECRET}\"}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("SBOM_GENERATOR", "generator=interpolated");
    let _ = environment.insert_sensitive("SBOM_SECRET", "generator=private");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert!(matches!(
        build_definition(view, "boolean")?.sbom().map(ProjectValue::value),
        Some(BuildSbom::Boolean(true))
    ));
    assert!(
        matches!(build_definition(view, "generator")?.sbom().map(ProjectValue::value), Some(BuildSbom::String(value)) if value == "generator=base")
    );
    assert!(
        matches!(build_definition(view, "expression")?.sbom().map(ProjectValue::value), Some(BuildSbom::String(value)) if value == "generator=interpolated")
    );
    assert!(
        matches!(build_definition(view, "empty")?.sbom().map(ProjectValue::value), Some(BuildSbom::String(value)) if value.is_empty())
    );

    let replaced = build_definition(view, "replaced")?
        .sbom()
        .ok_or("replaced sbom expected")?;
    assert!(matches!(replaced.value(), BuildSbom::String(value) if value == "generator=override"));
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);

    let reset = build_definition(view, "reset")?;
    assert!(reset.sbom().is_none());
    assert!(
        reset
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "reset", "sbom"]
                && field.provenance().operation() == MergeOperation::Reset)
    );

    let overridden = build_definition(view, "overridden")?
        .sbom()
        .ok_or("overridden sbom expected")?;
    assert!(matches!(overridden.value(), BuildSbom::String(value) if value == "generator=private"));
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(overridden.is_sensitive());
    assert!(!format!("{overridden:?}").contains("generator=private"));

    assert_invalid_build_sbom_recovery(view, &result)?;
    Ok(())
}

fn assert_invalid_build_sbom_recovery(
    view: &ProjectView,
    result: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    for service in ["invalid-null", "invalid-number", "invalid-mapping", "invalid-sequence"] {
        let definition = build_definition(view, service)?;
        assert!(definition.sbom().is_none());
        assert_eq!(
            definition.context().map(ProjectValue::value).map(String::as_str),
            Some("retained")
        );
        assert!(
            definition
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", service, "sbom"])
        );
    }
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
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

#[test]
fn retains_effective_build_shm_size_classification_merge_provenance_and_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{ShmSizeKind, ShmSizeScalarKind, ShmSizeUnit};

    let base_id = SourceId::new(8461);
    let override_id = SourceId::new(8462);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  documented:\n    build: {shm_size: \"64mb\"}\n",
                "  number:\n    build: {shm_size: 64}\n",
                "  zero:\n    build: {shm_size: \"000m\"}\n",
                "  replaced:\n    build: {shm_size: 64m}\n",
                "  reset:\n    build: {shm_size: 64m}\n",
                "  overridden:\n    build: {shm_size: 64m}\n",
                "  invalid-null:\n    build:\n      context: retained\n      shm_size: null\n      x-retained: true\n",
                "  invalid-mapping:\n    build:\n      context: retained\n      shm_size:\n        value: 64mb\n      x-retained: true\n",
                "  invalid-sequence:\n    build:\n      context: retained\n      shm_size:\n        - 64mb\n      x-retained: true\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    build: {shm_size: \"${BUILD_SHM_SIZE}\"}\n",
                "  reset:\n    build: {shm_size: !reset null}\n",
                "  overridden:\n    build: {shm_size: !override \"${OVERRIDE_SHM_SIZE}\"}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("BUILD_SHM_SIZE", "128mb");
    let _ = environment.insert_sensitive("OVERRIDE_SHM_SIZE", "256mb");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    let documented = build_definition(view, "documented")?
        .shm_size()
        .ok_or("documented build shm_size expected")?;
    assert_eq!(documented.value().raw().value(), "64mb");
    assert_eq!(documented.value().scalar_kind(), ShmSizeScalarKind::String);
    assert!(matches!(
        documented.value().kind(),
        ShmSizeKind::Documented { amount_raw, unit: ShmSizeUnit::Mb } if amount_raw == "64"
    ));
    assert!(matches!(
        build_definition(view, "number")?
            .shm_size()
            .map(ProjectValue::value)
            .map(compose_lens::model::ShmSize::kind),
        Some(ShmSizeKind::ProviderDependentNumber)
    ));
    assert!(matches!(
        build_definition(view, "zero")?.shm_size().map(ProjectValue::value).map(compose_lens::model::ShmSize::kind),
        Some(ShmSizeKind::Zero { amount_raw, unit: Some(ShmSizeUnit::M) }) if amount_raw == "000"
    ));

    let replaced = build_definition(view, "replaced")?
        .shm_size()
        .ok_or("replaced build shm_size expected")?;
    assert_eq!(replaced.value().raw().value(), "128mb");
    assert!(replaced.is_sensitive());
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);
    assert!(!format!("{replaced:?}").contains("128mb"));

    let reset = build_definition(view, "reset")?;
    assert!(reset.shm_size().is_none());
    let reset_evidence = reset
        .unmodeled_fields()
        .iter()
        .find(|field| field.path() == ["services", "reset", "shm_size"])
        .ok_or("reset build shm_size evidence expected")?;
    assert_eq!(reset_evidence.provenance().operation(), MergeOperation::Reset);

    let overridden = build_definition(view, "overridden")?
        .shm_size()
        .ok_or("overridden build shm_size expected")?;
    assert_eq!(overridden.value().raw().value(), "256mb");
    assert!(overridden.is_sensitive());
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_source_ids(overridden.provenance().sources(), &[base_id, override_id, override_id]);

    assert_invalid_build_shm_size_recovery(view, &result)?;
    Ok(())
}

fn assert_invalid_build_shm_size_recovery(
    view: &ProjectView,
    result: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    for service in ["invalid-null", "invalid-mapping", "invalid-sequence"] {
        let definition = build_definition(view, service)?;
        assert!(definition.shm_size().is_none());
        assert_eq!(
            definition.context().map(ProjectValue::value).map(String::as_str),
            Some("retained")
        );
        assert_eq!(definition.unmodeled_fields().len(), 2);
        assert!(
            definition
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", service, "shm_size"])
        );
        assert!(
            definition
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", service, "x-retained"])
        );
    }
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == compose_lens::model::SHM_SIZE_EXPECTED_VALUE)
    );
    Ok(())
}

fn assert_invalid_build_no_cache_recovery(view: &ProjectView) -> Result<(), Box<dyn std::error::Error>> {
    for service in ["invalid-null", "invalid-number", "invalid-mapping", "invalid-sequence"] {
        let definition = build_definition(view, service)?;
        assert!(definition.no_cache().is_none());
        assert_eq!(
            definition.context().map(ProjectValue::value).map(String::as_str),
            Some("retained")
        );
        assert!(
            definition
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", service, "no_cache"])
        );
    }
    Ok(())
}

#[test]
fn retains_effective_build_isolation_interpolation_provenance_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(847);
    let override_id = SourceId::new(848);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  interpolated:\n    build: {isolation: \"${ISOLATION}\"}\n",
                "  replaced:\n    build: {isolation: process}\n",
                "  reset:\n    build: {isolation: process}\n",
                "  overridden:\n    build: {isolation: process}\n",
                "  invalid-boolean:\n    build: {context: retained, isolation: true}\n",
                "  invalid-number:\n    build: {context: retained, isolation: 1}\n",
                "  invalid-null:\n    build: {context: retained, isolation: null}\n",
                "  invalid-sequence:\n    build: {context: retained, isolation: [process]}\n",
                "  invalid-mapping:\n    build: {context: retained, isolation: {mode: process}}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  replaced:\n    build: {isolation: hyperv}\n",
                "  reset:\n    build: {isolation: !reset null}\n",
                "  overridden:\n    build: {isolation: !override \"${SECRET}\"}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("ISOLATION", "process");
    let _ = environment.insert_sensitive("SECRET", "private-isolation");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert_eq!(
        build_definition(view, "interpolated")?
            .isolation()
            .map(ProjectValue::value)
            .map(String::as_str),
        Some("process")
    );
    let replaced = build_definition(view, "replaced")?
        .isolation()
        .ok_or("replaced isolation expected")?;
    assert_eq!(replaced.value(), "hyperv");
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.provenance().sources(), &[base_id, override_id]);

    let reset = build_definition(view, "reset")?;
    assert!(reset.isolation().is_none());
    assert!(reset.unmodeled_fields().iter().any(|field| {
        field.path() == ["services", "reset", "isolation"] && field.provenance().operation() == MergeOperation::Reset
    }));

    let overridden = build_definition(view, "overridden")?
        .isolation()
        .ok_or("overridden isolation expected")?;
    assert_eq!(overridden.value(), "private-isolation");
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_source_ids(overridden.provenance().sources(), &[base_id, override_id, override_id]);
    assert!(overridden.is_sensitive());
    assert!(!format!("{overridden:?}").contains("private-isolation"));

    for service in [
        "invalid-boolean",
        "invalid-number",
        "invalid-null",
        "invalid-sequence",
        "invalid-mapping",
    ] {
        let definition = build_definition(view, service)?;
        assert!(definition.isolation().is_none());
        assert_eq!(
            definition.context().map(ProjectValue::value).map(String::as_str),
            Some("retained")
        );
        assert!(
            definition
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", service, "isolation"])
        );
    }
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
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

fn build_definition<'view>(
    view: &'view ProjectView,
    service: &str,
) -> Result<&'view compose_lens::project::ProjectBuildDefinition, Box<dyn std::error::Error>> {
    let build = view
        .service(service)
        .and_then(ProjectService::build)
        .ok_or("build definition expected")?;
    let ProjectBuild::Definition(definition) = build.value() else {
        return Err("long build definition expected".into());
    };
    Ok(definition)
}

fn build_tags<'view>(
    view: &'view ProjectView,
    service: &str,
) -> Result<&'view ProjectValue<Vec<ProjectValue<String>>>, Box<dyn std::error::Error>> {
    build_definition(view, service)?
        .tags()
        .ok_or_else(|| "build tags expected".into())
}

fn build_platforms<'view>(
    view: &'view ProjectView,
    service: &str,
) -> Result<&'view ProjectValue<Vec<ProjectValue<String>>>, Box<dyn std::error::Error>> {
    build_definition(view, service)?
        .platforms()
        .ok_or_else(|| "build platforms expected".into())
}

fn build_cache_from<'view>(
    view: &'view ProjectView,
    service: &str,
) -> Result<&'view ProjectValue<Vec<ProjectValue<String>>>, Box<dyn std::error::Error>> {
    build_definition(view, service)?
        .cache_from()
        .ok_or_else(|| "build cache_from expected".into())
}

fn build_cache_to<'view>(
    view: &'view ProjectView,
    service: &str,
) -> Result<&'view ProjectValue<Vec<ProjectValue<String>>>, Box<dyn std::error::Error>> {
    build_definition(view, service)?
        .cache_to()
        .ok_or_else(|| "build cache_to expected".into())
}

#[test]
fn retains_effective_raw_build_cache_locations_and_malformed_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(911);
    let override_id = SourceId::new(912);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  appended:\n    build:\n",
                "      cache_from: [\"type=registry,ref=${BASE_CACHE}\", \"type=local,src=.cache\", \"type=local,src=.cache\"]\n",
                "      cache_to: [\"type=local,dest=.cache\", \"type=local,dest=.cache\"]\n",
                "  empty:\n    build: {cache_from: [], cache_to: []}\n",
                "  reset:\n    build: {cache_from: [\"type=local,src=old\"], cache_to: [\"type=local,dest=old\"]}\n",
                "  overridden:\n    build: {cache_from: [\"type=local,src=old\"], cache_to: [\"type=local,dest=old\"]}\n",
                "  malformed:\n    build:\n",
                "      cache_from: [\"type=local,src=base\", false, {}, \"type=gha\"]\n",
                "      cache_to: [\"type=local,dest=base\", 7, [], \"type=gha\"]\n",
                "  wrong-outer:\n    build: {cache_from: {type: local}, cache_to: \"type=local,dest=.cache\"}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  appended:\n    build:\n",
                "      cache_from: [\"type=local,src=.cache\", \"type=registry,ref=${NEXT_CACHE}\"]\n",
                "      cache_to: [\"type=local,dest=.cache\", \"type=registry,ref=${NEXT_CACHE}\"]\n",
                "  reset:\n    build: {cache_from: !reset [], cache_to: !reset []}\n",
                "  overridden:\n    build: {cache_from: !override [\"type=gha\", \"type=gha\"], cache_to: !override [\"type=gha\", \"type=gha\"]}\n",
                "  malformed:\n    build:\n",
                "      cache_from: [\"type=registry,ref=${MALFORMED_CACHE}\", []]\n",
                "      cache_to: [\"type=registry,ref=${MALFORMED_CACHE}\", null]\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("BASE_CACHE", "private-base");
    let _ = environment.insert_sensitive("NEXT_CACHE", "private-next");
    let _ = environment.insert_sensitive("MALFORMED_CACHE", "private-malformed");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert_appended_build_cache_locations(view, base_id, override_id)?;
    assert_reset_and_overridden_build_cache_locations(view)?;
    assert_malformed_build_cache_locations(view)?;
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

fn assert_appended_build_cache_locations(
    view: &ProjectView,
    base_id: SourceId,
    override_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let cache_from = build_cache_from(view, "appended")?;
    assert_eq!(cache_from.provenance().operation(), MergeOperation::Appended);
    assert_source_ids(cache_from.provenance().sources(), &[base_id, override_id]);
    assert!(cache_from.is_sensitive());
    assert_eq!(
        cache_values(cache_from),
        [
            "type=registry,ref=private-base",
            "type=local,src=.cache",
            "type=local,src=.cache",
            "type=local,src=.cache",
            "type=registry,ref=private-next"
        ]
    );
    assert!(cache_from.value()[0].is_sensitive());
    assert_eq!(cache_from.value()[3].provenance().operation(), MergeOperation::Added);
    assert_source_ids(cache_from.value()[3].provenance().sources(), &[override_id]);
    assert!(!format!("{cache_from:?}").contains("private-base"));

    let cache_to = build_cache_to(view, "appended")?;
    assert_eq!(cache_to.provenance().operation(), MergeOperation::Appended);
    assert!(cache_to.is_sensitive());
    assert_eq!(
        cache_values(cache_to),
        [
            "type=local,dest=.cache",
            "type=local,dest=.cache",
            "type=local,dest=.cache",
            "type=registry,ref=private-next"
        ]
    );
    assert_eq!(cache_to.value()[2].provenance().operation(), MergeOperation::Added);
    assert_source_ids(cache_to.value()[2].provenance().sources(), &[override_id]);
    assert!(cache_to.value()[3].is_sensitive());
    Ok(())
}

fn assert_reset_and_overridden_build_cache_locations(view: &ProjectView) -> Result<(), Box<dyn std::error::Error>> {
    let empty = build_definition(view, "empty")?;
    for locations in [empty.cache_from(), empty.cache_to()] {
        let locations = locations.ok_or("empty build cache locations expected")?;
        assert!(locations.value().is_empty());
        assert_eq!(locations.provenance().operation(), MergeOperation::Authored);
    }
    let reset = build_definition(view, "reset")?;
    for locations in [reset.cache_from(), reset.cache_to()] {
        let locations = locations.ok_or("reset build cache locations expected")?;
        assert!(locations.value().is_empty());
        assert_eq!(locations.provenance().operation(), MergeOperation::Reset);
    }
    let overridden = build_definition(view, "overridden")?;
    for locations in [overridden.cache_from(), overridden.cache_to()] {
        let locations = locations.ok_or("overridden build cache locations expected")?;
        assert_eq!(cache_values(locations), ["type=gha", "type=gha"]);
        assert_eq!(locations.provenance().operation(), MergeOperation::Override);
    }
    Ok(())
}

fn assert_malformed_build_cache_locations(view: &ProjectView) -> Result<(), Box<dyn std::error::Error>> {
    let malformed = build_definition(view, "malformed")?;
    assert_eq!(
        cache_values(build_cache_from(view, "malformed")?),
        ["type=local,src=base", "type=gha", "type=registry,ref=private-malformed"]
    );
    assert_eq!(
        cache_values(build_cache_to(view, "malformed")?),
        [
            "type=local,dest=base",
            "type=gha",
            "type=registry,ref=private-malformed"
        ]
    );
    for name in ["cache_from", "cache_to"] {
        assert!(
            malformed
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", "malformed", name])
        );
    }
    let wrong_outer = build_definition(view, "wrong-outer")?;
    assert!(wrong_outer.cache_from().is_none() && wrong_outer.cache_to().is_none());
    for name in ["cache_from", "cache_to"] {
        assert!(
            wrong_outer
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", "wrong-outer", name])
        );
    }
    Ok(())
}

fn cache_values(locations: &ProjectValue<Vec<ProjectValue<String>>>) -> Vec<&str> {
    locations
        .value()
        .iter()
        .map(ProjectValue::value)
        .map(String::as_str)
        .collect()
}

#[test]
fn retains_effective_build_args_forms_merge_provenance_and_partial_recovery() -> Result<(), Box<dyn std::error::Error>>
{
    let base_id = SourceId::new(730);
    let override_id = SourceId::new(731);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  mapping:\n    build:\n      args: {retained: base, replaced: old, count: 1, enabled: true, empty: null}\n",
                "  list:\n    build:\n      args: [base=value, BARE, base=value]\n",
                "  reset:\n    build:\n      args: {old: value}\n",
                "  empty-list:\n    build:\n      args: []\n",
                "  overridden:\n    build:\n      args: [old]\n",
                "  malformed:\n    build:\n      args: [before, false, {nested: value}]\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  mapping:\n    build:\n      args: {replaced: \"${REPLACED}\", added: after}\n",
                "  list:\n    build:\n      args: [\"next=${NEXT}\", BARE]\n",
                "  reset:\n    build:\n      args: !reset {}\n",
                "  overridden:\n    build:\n      args: !override [new, new]\n",
                "  malformed:\n    build:\n      args: [later, []]\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("REPLACED", "private-value");
    let _ = environment.insert_sensitive("NEXT", "private-next");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert_effective_mapping_build_args(view, base_id, override_id)?;
    assert_effective_list_build_args(view, &result)?;
    Ok(())
}

fn assert_effective_mapping_build_args(
    view: &ProjectView,
    base_id: SourceId,
    override_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let mapping = build_definition(view, "mapping")?
        .args()
        .ok_or("mapping args expected")?;
    assert_eq!(mapping.provenance().operation(), MergeOperation::Merged);
    assert_source_ids(mapping.provenance().sources(), &[base_id, override_id]);
    let ProjectBuildArgs::Map(entries) = mapping.value() else {
        return Err("mapping build args expected".into());
    };
    assert_eq!(entries.len(), 6);
    let replaced = entries
        .iter()
        .find(|entry| entry.name().value() == "replaced")
        .ok_or("replaced arg expected")?;
    assert_eq!(
        replaced.value().value(),
        &ComposeScalar::String("private-value".to_owned())
    );
    assert!(replaced.value().is_sensitive());
    assert_eq!(replaced.value().provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.value().provenance().sources(), &[base_id, override_id]);
    assert!(
        matches!(entries.iter().find(|entry| entry.name().value() == "count").map(|entry| entry.value().value()), Some(ComposeScalar::Number(value)) if value == "1")
    );
    assert!(matches!(
        entries
            .iter()
            .find(|entry| entry.name().value() == "enabled")
            .map(|entry| entry.value().value()),
        Some(ComposeScalar::Boolean(true))
    ));
    assert!(matches!(
        entries
            .iter()
            .find(|entry| entry.name().value() == "empty")
            .map(|entry| entry.value().value()),
        Some(ComposeScalar::Null)
    ));
    Ok(())
}

fn assert_effective_list_build_args(
    view: &ProjectView,
    result: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let list = build_definition(view, "list")?.args().ok_or("list args expected")?;
    let ProjectBuildArgs::List(items) = list.value() else {
        return Err("list build args expected".into());
    };
    assert_eq!(
        items
            .iter()
            .map(ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["base=value", "BARE", "base=value", "next=private-next", "BARE"]
    );
    assert_eq!(items[3].provenance().operation(), MergeOperation::Added);
    assert!(items[3].is_sensitive());
    assert_eq!(
        build_definition(view, "reset")?
            .args()
            .ok_or("reset args expected")?
            .provenance()
            .operation(),
        MergeOperation::Reset
    );
    assert!(
        matches!(build_definition(view, "reset")?.args().map(ProjectValue::value), Some(ProjectBuildArgs::Map(entries)) if entries.is_empty())
    );
    assert!(
        matches!(build_definition(view, "empty-list")?.args().map(ProjectValue::value), Some(ProjectBuildArgs::List(items)) if items.is_empty())
    );
    let overridden = build_definition(view, "overridden")?
        .args()
        .ok_or("overridden args expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(overridden.value(), ProjectBuildArgs::List(items)
        if items.iter().map(ProjectValue::value).map(String::as_str).collect::<Vec<_>>() == ["new", "new"]));
    let malformed = build_definition(view, "malformed")?;
    assert!(
        matches!(malformed.args().map(ProjectValue::value), Some(ProjectBuildArgs::List(items))
        if items.iter().map(ProjectValue::value).map(String::as_str).collect::<Vec<_>>() == ["before", "later"])
    );
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "malformed", "args"])
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
fn retains_effective_build_additional_context_forms_generic_merges_and_partial_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(901);
    let override_id = SourceId::new(902);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  mapping:\n    build:\n      additional_contexts: {retained: ./base, replaced: ./old, value: \"${VALUE}\"}\n",
                "  list:\n    build:\n      additional_contexts: [assets=./assets, duplicate=./one]\n",
                "  mixed:\n    build:\n      additional_contexts: {old: ./old}\n",
                "  reset:\n    build:\n      additional_contexts: {old: ./old}\n",
                "  overridden:\n    build:\n      additional_contexts: [old=./old]\n",
                "  malformed:\n    build:\n      additional_contexts: {retained: ./retained, invalid: [nested], later: null}\n",
                "  duplicate:\n    build:\n      additional_contexts: {retained: ./retained, duplicate: first, duplicate: second, later: ./later}\n",
                "  wrong-outer:\n    build:\n      additional_contexts: ./not-a-collection\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  mapping:\n    build:\n      additional_contexts: {replaced: \"${REPLACED}\", \"${KEY}\": \"${ADDED}\"}\n",
                "  list:\n    build:\n      additional_contexts: [duplicate=./one, \"next=${NEXT}\"]\n",
                "  mixed:\n    build:\n      additional_contexts: [replacement=./new]\n",
                "  reset:\n    build:\n      additional_contexts: !reset {}\n",
                "  overridden:\n    build:\n      additional_contexts: !override [new=./new, new=./new]\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("VALUE", "private-value");
    let _ = environment.insert_sensitive("REPLACED", "private-replaced");
    let _ = environment.insert_sensitive("ADDED", "private-added");
    let _ = environment.insert_sensitive("KEY", "must-not-interpolate");
    let _ = environment.insert_sensitive("NEXT", "private-next");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert_effective_build_additional_context_merges(view, base_id, override_id)?;
    assert_effective_build_additional_context_recovery(view, &result)
}

fn assert_effective_build_additional_context_merges(
    view: &ProjectView,
    base_id: SourceId,
    override_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let mapping = build_definition(view, "mapping")?
        .additional_contexts()
        .ok_or("mapping additional contexts expected")?;
    assert_eq!(mapping.provenance().operation(), MergeOperation::Merged);
    assert_source_ids(mapping.provenance().sources(), &[base_id, override_id]);
    assert!(mapping.is_sensitive());
    let ProjectBuildAdditionalContexts::Map(entries) = mapping.value() else {
        return Err("mapping additional contexts expected".into());
    };
    assert_eq!(entries.len(), 4);
    let replaced = entries
        .iter()
        .find(|entry| entry.name().value() == "replaced")
        .ok_or("replaced context expected")?;
    assert_eq!(
        replaced.value().value(),
        &ComposeScalar::String("private-replaced".to_owned())
    );
    assert!(replaced.value().is_sensitive());
    assert_eq!(replaced.value().provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(replaced.value().provenance().sources(), &[base_id, override_id]);
    let unexpanded_key = entries
        .iter()
        .find(|entry| entry.name().value() == "${KEY}")
        .ok_or("uninterpolated context key expected")?;
    assert!(!unexpanded_key.name().is_sensitive());
    assert_eq!(
        unexpanded_key.value().value(),
        &ComposeScalar::String("private-added".to_owned())
    );
    assert!(unexpanded_key.value().is_sensitive());

    let list = build_definition(view, "list")?
        .additional_contexts()
        .ok_or("list additional contexts expected")?;
    let ProjectBuildAdditionalContexts::List(items) = list.value() else {
        return Err("list additional contexts expected".into());
    };
    assert_eq!(
        items
            .iter()
            .map(ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "assets=./assets",
            "duplicate=./one",
            "duplicate=./one",
            "next=private-next"
        ]
    );
    assert_eq!(items[3].provenance().operation(), MergeOperation::Added);
    assert!(items[3].is_sensitive());
    Ok(())
}

fn assert_effective_build_additional_context_recovery(
    view: &ProjectView,
    result: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let mixed = build_definition(view, "mixed")?
        .additional_contexts()
        .ok_or("mixed additional contexts expected")?;
    assert_eq!(mixed.provenance().operation(), MergeOperation::Replaced);
    assert!(matches!(mixed.value(), ProjectBuildAdditionalContexts::List(items)
        if items.iter().map(ProjectValue::value).map(String::as_str).collect::<Vec<_>>() == ["replacement=./new"]));
    let reset = build_definition(view, "reset")?
        .additional_contexts()
        .ok_or("reset additional contexts expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(matches!(reset.value(), ProjectBuildAdditionalContexts::Map(entries) if entries.is_empty()));
    let overridden = build_definition(view, "overridden")?
        .additional_contexts()
        .ok_or("overridden additional contexts expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(overridden.value(), ProjectBuildAdditionalContexts::List(items)
        if items.iter().map(ProjectValue::value).map(String::as_str).collect::<Vec<_>>() == ["new=./new", "new=./new"]));

    let malformed = build_definition(view, "malformed")?;
    assert!(matches!(malformed.additional_contexts().map(ProjectValue::value),
        Some(ProjectBuildAdditionalContexts::Map(entries))
            if entries.iter().map(|entry| entry.name().value()).collect::<Vec<_>>() == ["retained", "later"]));
    let duplicate = build_definition(view, "duplicate")?
        .additional_contexts()
        .ok_or("duplicate additional contexts expected")?;
    assert!(matches!(duplicate.value(), ProjectBuildAdditionalContexts::Map(entries)
        if entries.iter().map(|entry| entry.name().value()).collect::<Vec<_>>() == ["retained", "duplicate", "later"]));
    for service in ["malformed", "duplicate", "wrong-outer"] {
        assert!(
            build_definition(view, service)?
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", service, "additional_contexts"])
        );
    }
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_INVALID_VALUE)
    );
    Ok(())
}

#[test]
fn retains_effective_build_ulimits_merge_provenance_and_malformed_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(1818);
    let override_id = SourceId::new(1819);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  merged:\n    build:\n      ulimits:\n        nofile: \"001024\"\n        nproc: {soft: \"${SOFT_LIMIT}\", hard: 4096}\n",
                "  empty:\n    build: {ulimits: {}}\n",
                "  reset:\n    build: {ulimits: {nofile: 1}}\n",
                "  overridden:\n    build: {ulimits: {nofile: 1}}\n",
                "  malformed:\n    build:\n      ulimits:\n        retained: 1\n        Bad: 2\n        broken: []\n        range: {soft: true, hard: 4, future: retained}\n",
                "  wrong-outer:\n    build: {ulimits: []}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  merged:\n    build:\n      ulimits:\n        nofile: \"${NOFILE}\"\n        nproc: {hard: \"8192\"}\n",
                "  reset:\n    build: {ulimits: !reset {}}\n",
                "  overridden:\n    build: {ulimits: !override {core: -1}}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("SOFT_LIMIT", "1024");
    let _ = environment.insert_sensitive("NOFILE", "2048");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert_merged_build_ulimits(view, base_id, override_id)?;
    assert_empty_and_overridden_build_ulimits(view)?;
    assert_malformed_build_ulimit_evidence(view, &result)?;
    Ok(())
}

fn assert_merged_build_ulimits(
    view: &ProjectView,
    base_id: SourceId,
    override_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let limits = build_definition(view, "merged")?
        .ulimits()
        .ok_or("merged build ulimits expected")?;
    assert_eq!(limits.provenance().operation(), MergeOperation::Merged);
    let nofile = limits
        .value()
        .entries()
        .iter()
        .find(|entry| entry.value().name().value() == "nofile")
        .ok_or("effective nofile expected")?;
    let ProjectUlimitValue::Single(nofile) = nofile.value().value() else {
        return Err("effective nofile must remain scalar syntax".into());
    };
    assert_eq!(nofile.value().authored(), "\"${NOFILE}\"");
    assert_eq!(nofile.value().value().raw(), "2048");
    assert!(nofile.is_sensitive());
    assert_eq!(nofile.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(nofile.provenance().sources(), &[base_id, override_id]);

    let nproc = limits
        .value()
        .entries()
        .iter()
        .find(|entry| entry.value().name().value() == "nproc")
        .ok_or("effective nproc expected")?;
    let ProjectUlimitValue::Range(nproc) = nproc.value().value() else {
        return Err("effective nproc must remain range syntax".into());
    };
    let soft = nproc.soft().ok_or("effective soft limit expected")?;
    let hard = nproc.hard().ok_or("effective hard limit expected")?;
    assert_eq!(soft.value().value().raw(), "1024");
    assert!(soft.is_sensitive());
    assert_eq!(hard.value().authored(), "\"8192\"");
    assert_eq!(hard.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(hard.provenance().sources(), &[base_id, override_id]);
    Ok(())
}

fn assert_empty_and_overridden_build_ulimits(view: &ProjectView) -> Result<(), Box<dyn std::error::Error>> {
    let empty = build_definition(view, "empty")?
        .ulimits()
        .ok_or("empty build ulimits expected")?;
    assert!(empty.value().is_empty());
    assert_eq!(empty.provenance().operation(), MergeOperation::Authored);
    let reset = build_definition(view, "reset")?
        .ulimits()
        .ok_or("reset build ulimits expected")?;
    assert!(reset.value().is_empty());
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    let overridden = build_definition(view, "overridden")?
        .ulimits()
        .ok_or("overridden build ulimits expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(overridden.value().entries(), [entry]
        if entry.value().name().value() == "core"
            && matches!(entry.value().value(), ProjectUlimitValue::Single(value) if value.value().value().raw() == "-1")));
    Ok(())
}

fn assert_malformed_build_ulimit_evidence(
    view: &ProjectView,
    result: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let malformed = build_definition(view, "malformed")?;
    let malformed_limits = malformed.ulimits().ok_or("partial malformed build ulimits expected")?;
    assert_eq!(
        malformed_limits
            .value()
            .entries()
            .iter()
            .map(|entry| entry.value().name().value())
            .collect::<Vec<_>>(),
        ["retained", "range"]
    );
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "malformed", "ulimits", "Bad"])
    );
    assert!(
        malformed
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "malformed", "ulimits", "broken"])
    );
    let ProjectUlimitValue::Range(range) = malformed_limits.value().entries()[1].value().value() else {
        return Err("partially malformed range expected".into());
    };
    assert!(range.soft().is_none());
    for path in [
        ["services", "malformed", "ulimits", "range", "soft"],
        ["services", "malformed", "ulimits", "range", "future"],
    ] {
        assert!(range.unmodeled_fields().iter().any(|field| field.path() == path));
    }
    let wrong_outer = build_definition(view, "wrong-outer")?;
    assert!(wrong_outer.ulimits().is_none());
    assert!(
        wrong_outer
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "wrong-outer", "ulimits"])
    );
    for code in [
        PROJECT_EXPECTED_FORM,
        ULIMIT_INVALID_NAME,
        ULIMIT_INVALID_VALUE,
        ULIMIT_MISSING_RANGE_MEMBER,
    ] {
        assert!(result.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_effective_build_entitlements_merge_provenance_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(2002);
    let override_id = SourceId::new(2003);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  appended:\n    build: {entitlements: [network.host, security.insecure, network.host]}\n",
                "  empty:\n    build: {entitlements: []}\n",
                "  reset:\n    build: {entitlements: [network.host]}\n",
                "  overridden:\n    build: {entitlements: [network.host]}\n",
                "  malformed:\n    build: {entitlements: [network.host, false, {}, \"\"]}\n",
                "  outer:\n    build: {entitlements: network.host}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  appended:\n    build: {entitlements: [network.host, \"${NEXT}\"]}\n",
                "  reset:\n    build: {entitlements: !reset []}\n",
                "  overridden:\n    build: {entitlements: !override [security.insecure, security.insecure]}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("NEXT", "private.entitlement");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert_appended_build_entitlements(view, base_id, override_id)?;
    assert_build_entitlement_recovery(view, &result)
}

fn assert_appended_build_entitlements(
    view: &ProjectView,
    base_id: SourceId,
    override_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let entitlements = build_definition(view, "appended")?
        .entitlements()
        .ok_or("appended build entitlements expected")?;
    assert_eq!(entitlements.provenance().operation(), MergeOperation::Appended);
    assert_source_ids(entitlements.provenance().sources(), &[base_id, override_id]);
    assert!(entitlements.is_sensitive());
    assert_eq!(
        entitlements
            .value()
            .iter()
            .map(ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "network.host",
            "security.insecure",
            "network.host",
            "network.host",
            "private.entitlement"
        ]
    );
    assert_eq!(entitlements.value()[3].provenance().operation(), MergeOperation::Added);
    assert!(entitlements.value()[4].is_sensitive());
    Ok(())
}

fn assert_build_entitlement_recovery(
    view: &ProjectView,
    result: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        matches!(build_definition(view, "empty")?.entitlements().map(ProjectValue::value), Some(values) if values.is_empty())
    );
    let reset = build_definition(view, "reset")?
        .entitlements()
        .ok_or("reset build entitlements expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.value().is_empty());
    let overridden = build_definition(view, "overridden")?
        .entitlements()
        .ok_or("overridden build entitlements expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        overridden
            .value()
            .iter()
            .map(ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["security.insecure", "security.insecure"]
    );
    let malformed = build_definition(view, "malformed")?;
    assert!(matches!(malformed.entitlements().map(ProjectValue::value), Some(values)
        if values.iter().map(ProjectValue::value).map(String::as_str).collect::<Vec<_>>() == ["network.host", ""]));
    for service in ["malformed", "outer"] {
        assert!(
            build_definition(view, service)?
                .unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", service, "entitlements"])
        );
    }
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
    );
    Ok(())
}

#[test]
fn retains_effective_build_extra_hosts_forms_generic_merge_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(1931);
    let override_id = SourceId::new(1932);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  map:\n    build:\n      extra_hosts: {retained: 10.0.0.1, replaced: \"${OLD}\", list: [\"[::1]\"]}\n",
                "  list:\n    build:\n      extra_hosts: [\"db:127.0.0.1\", \"gateway=host-gateway\"]\n",
                "  mixed:\n    build: {extra_hosts: {old: 127.0.0.1}}\n",
                "  reset:\n    build: {extra_hosts: {old: 127.0.0.1}}\n",
                "  overridden:\n    build: {extra_hosts: [old:127.0.0.1]}\n",
                "  malformed:\n    build: {extra_hosts: {retained: 127.0.0.1, bad: [ok, 7], wrong: {nested: no}}}\n",
                "  outer:\n    build: {extra_hosts: 127.0.0.1}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  map:\n    build:\n      extra_hosts: {replaced: \"${NEW}\", list: [host-gateway], \"${KEY}\": \"${ADDED}\"}\n",
                "  list:\n    build:\n      extra_hosts: [\"db:127.0.0.1\", \"v6=[::1]\"]\n",
                "  mixed:\n    build: {extra_hosts: [replacement=host-gateway]}\n",
                "  reset:\n    build: {extra_hosts: !reset {}}\n",
                "  overridden:\n    build: {extra_hosts: !override {new: [\"[fd00::1]\"]}}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("OLD", "private-old");
    let _ = environment.insert_sensitive("NEW", "private-new");
    let _ = environment.insert_sensitive("ADDED", "private-added");
    let _ = environment.insert_sensitive("KEY", "must-not-interpolate");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;

    assert_effective_build_extra_hosts_mapping(view, base_id, override_id)?;
    assert_effective_build_extra_hosts_collections_and_recovery(view, &result)
}

fn assert_effective_build_extra_hosts_mapping(
    view: &ProjectView,
    base_id: SourceId,
    override_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let map = build_definition(view, "map")?
        .extra_hosts()
        .ok_or("map extra_hosts expected")?;
    assert_eq!(map.provenance().operation(), MergeOperation::Merged);
    assert_source_ids(map.provenance().sources(), &[base_id, override_id]);
    assert!(map.is_sensitive());
    let ProjectBuildExtraHosts::Map(hosts) = map.value() else {
        return Err("mapping extra_hosts expected".into());
    };
    let replaced = hosts
        .iter()
        .find(|entry| entry.hostname().value() == "replaced")
        .ok_or("replaced host expected")?;
    assert!(
        matches!(replaced.addresses(), ProjectBuildExtraHostAddresses::Scalar(value)
        if value.value() == "private-new" && value.is_sensitive() && value.provenance().operation() == MergeOperation::Replaced)
    );
    let list = hosts
        .iter()
        .find(|entry| entry.hostname().value() == "list")
        .ok_or("address list expected")?;
    assert!(matches!(list.addresses(), ProjectBuildExtraHostAddresses::List(values)
        if values.iter().map(ProjectValue::value).map(String::as_str).collect::<Vec<_>>() == ["[::1]", "host-gateway"]
            && values[1].provenance().operation() == MergeOperation::Added));
    let unexpanded = hosts
        .iter()
        .find(|entry| entry.hostname().value() == "${KEY}")
        .ok_or("raw key expected")?;
    assert!(!unexpanded.hostname().is_sensitive());
    assert!(
        matches!(unexpanded.addresses(), ProjectBuildExtraHostAddresses::Scalar(value) if value.value() == "private-added")
    );
    Ok(())
}

fn assert_effective_build_extra_hosts_collections_and_recovery(
    view: &ProjectView,
    result: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let list = build_definition(view, "list")?
        .extra_hosts()
        .ok_or("list extra_hosts expected")?;
    assert!(matches!(list.value(), ProjectBuildExtraHosts::List(values)
        if values.iter().map(ProjectValue::value).map(String::as_str).collect::<Vec<_>>()
            == ["db:127.0.0.1", "gateway=host-gateway", "db:127.0.0.1", "v6=[::1]"]));
    assert_eq!(list.provenance().operation(), MergeOperation::Appended);
    assert!(
        matches!(build_definition(view, "mixed")?.extra_hosts().map(ProjectValue::value),
        Some(ProjectBuildExtraHosts::List(values)) if values[0].value() == "replacement=host-gateway")
    );
    let reset = build_definition(view, "reset")?
        .extra_hosts()
        .ok_or("reset extra_hosts expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(matches!(reset.value(), ProjectBuildExtraHosts::Map(entries) if entries.is_empty()));
    let overridden = build_definition(view, "overridden")?
        .extra_hosts()
        .ok_or("override extra_hosts expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(matches!(overridden.value(), ProjectBuildExtraHosts::Map(entries)
        if matches!(entries[0].addresses(), ProjectBuildExtraHostAddresses::List(values) if values[0].value() == "[fd00::1]")));
    let malformed = build_definition(view, "malformed")?;
    assert!(
        matches!(malformed.extra_hosts().map(ProjectValue::value), Some(ProjectBuildExtraHosts::Map(entries))
        if entries.iter().map(|entry| entry.hostname().value()).collect::<Vec<_>>() == ["retained", "bad"])
    );
    for path in [
        vec!["services", "malformed", "extra_hosts"],
        vec!["services", "malformed", "extra_hosts", "bad", "1"],
        vec!["services", "malformed", "extra_hosts", "wrong"],
    ] {
        assert!(
            malformed.unmodeled_fields().iter().any(|field| field.path() == path),
            "missing {path:?}; got {:?}",
            malformed
                .unmodeled_fields()
                .iter()
                .map(ProjectFieldReference::path)
                .collect::<Vec<_>>()
        );
    }
    assert!(
        build_definition(view, "outer")?
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "outer", "extra_hosts"])
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
fn retains_effective_blkio_nested_provenance_recovery_sensitivity_and_tags() -> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(3227);
    let override_id = SourceId::new(3228);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  app:\n    blkio_config: {weight: 500, device_read_bps: [{path: /dev/base, rate: 1}]}\n",
                "  sensitive:\n    blkio_config: {weight: \"${BLKIO_WEIGHT}\"}\n",
                "  reset:\n    blkio_config: {device_read_bps: [{path: /dev/old, rate: 1}]}\n",
                "  overridden:\n    blkio_config: {weight: 500, device_read_bps: [{path: /dev/old, rate: 1}]}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  app:\n    blkio_config: {weight: \"600\", device_read_bps: [not-a-map, {path: /dev/next, rate: 2, future: retained}, {path: [bad], rate: false}]}\n",
                "  reset:\n    blkio_config: {device_read_bps: !reset []}\n",
                "  overridden:\n    blkio_config: !override {weight_device: [{path: /dev/override, weight: 700}]}\n",
                "  malformed:\n    blkio_config:\n      weight: [bad]\n      device_read_iops: wrong\n      device_write_bps: {bad: value}\n      weight_device: {bad: value}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("BLKIO_WEIGHT", "900");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project")?, None);
    let view = result.view().ok_or("project view")?;
    let config = |name| {
        view.service(name)
            .and_then(ProjectService::blkio_config)
            .ok_or("effective blkio config")
    };

    let app = config("app")?;
    assert_eq!(app.provenance().operation(), MergeOperation::Merged);
    let weight = app.value().weight().ok_or("effective weight")?;
    assert!(matches!(
        weight.value(),
        compose_lens::model::BlkioScalar::String(value) if value == "600"
    ));
    assert_eq!(weight.provenance().operation(), MergeOperation::Replaced);
    assert_source_ids(weight.provenance().sources(), &[base_id, override_id]);
    let rates = app.value().device_read_bps().ok_or("effective rates")?;
    assert_eq!(rates.provenance().operation(), MergeOperation::Appended);
    assert_eq!(rates.value().len(), 4);
    assert_eq!(
        rates.value()[1].value().form(),
        compose_lens::project::ProjectBlkioDeviceRateForm::Unmodeled
    );
    assert_eq!(rates.value()[2].value().unmodeled_fields().len(), 1);
    assert_eq!(
        rates.value()[2].value().unmodeled_fields()[0].path(),
        ["services", "app", "blkio_config", "device_read_bps", "future"]
    );
    assert_eq!(rates.value()[3].value().unmodeled_fields().len(), 2);
    assert_eq!(
        rates.value()[3].value().unmodeled_fields()[0].path(),
        ["services", "app", "blkio_config", "device_read_bps", "path"]
    );
    assert_eq!(
        rates.value()[3].value().unmodeled_fields()[1].path(),
        ["services", "app", "blkio_config", "device_read_bps", "rate"]
    );
    assert_source_ids(rates.value()[2].provenance().sources(), &[override_id]);

    let sensitive = config("sensitive")?.value().weight().ok_or("sensitive weight")?;
    assert!(sensitive.is_sensitive());
    let debug = format!("{sensitive:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("900"));

    let reset_rates = config("reset")?.value().device_read_bps().ok_or("reset rates")?;
    assert_eq!(reset_rates.provenance().operation(), MergeOperation::Reset);
    assert!(reset_rates.value().is_empty());

    let overridden = config("overridden")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert!(overridden.value().weight().is_none());
    assert_eq!(
        overridden
            .value()
            .weight_device()
            .map(|items| items.provenance().operation()),
        Some(MergeOperation::Authored)
    );

    assert_malformed_blkio_project_recovery(view, &result)?;
    Ok(())
}

fn assert_malformed_blkio_project_recovery(
    view: &ProjectView,
    result: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let malformed = view
        .service("malformed")
        .and_then(ProjectService::blkio_config)
        .ok_or("malformed effective blkio config")?
        .value();
    assert!(malformed.weight().is_none());
    assert!(malformed.device_read_iops().is_none());
    assert!(malformed.device_write_bps().is_none());
    assert!(malformed.weight_device().is_none());
    for path in [
        ["services", "malformed", "blkio_config", "weight"],
        ["services", "malformed", "blkio_config", "device_read_iops"],
        ["services", "malformed", "blkio_config", "device_write_bps"],
        ["services", "malformed", "blkio_config", "weight_device"],
    ] {
        assert!(malformed.unmodeled_fields().iter().any(|field| field.path() == path));
    }
    assert!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == PROJECT_EXPECTED_FORM)
            .count()
            >= 6
    );
    Ok(())
}
