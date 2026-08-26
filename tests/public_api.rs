//! Consumer-facing contract for the supported 0.2.x processing path.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{
    DocumentInput, DocumentOrigin, IncludeIdentity, IncludeLoadError, IncludeLoader, IncludeProjectDirectoryRequest,
    IncludeProjectDirectoryResolution, IncludeProjectDirectoryResolveError, IncludeProjectDirectoryResolver,
    IncludeProjectDirectoryStatus, IncludeRequest, IncludeResolution, IncludedProjectInput, LoadedProject,
};
use compose_lens::merge::merge_project;
use compose_lens::model::{
    BooleanValue, Build, BuildAdditionalContexts, BuildArgs, BuildExtraHostAddresses, BuildExtraHosts, BuildFieldKind,
    BuildNoCache, BuildSbom, BuildSshForm, ComposeDocument, DependencyCondition, DeployDiscreteResourceValue,
    DeployEndpointMode, DeployMode, DeployPlacementMaxReplicasPerNode, DeployReplicas, DeployReservationDeviceCount,
    DeployResourceCpus, DeployResourceMemoryKind, DeployResourceMemoryUnit, DeployResourcePids, DeployRestartCondition,
    Entrypoint, EnvironmentFileFormatKind, HealthcheckDuration, HostAddressKind, HostnameKind, IdentityComponent,
    MemLimitKind, MemLimitScalarKind, MemLimitUnit, PidsLimitKind, PullPolicyKind, RestartPolicyKind, ShmSizeKind,
    ShmSizeScalarKind, ShmSizeUnit, StopGracePeriod, UserNamespaceModeKind,
};
use compose_lens::profiles::{ProfileRequest, select_profiles};
use compose_lens::project::{
    ProjectBuild, ProjectBuildAdditionalContexts, ProjectBuildArgs, ProjectBuildDefinition,
    ProjectBuildExtraHostAddresses, ProjectBuildExtraHosts, ProjectBuildLabels, ProjectBuildSsh, ProjectDevice,
    ProjectDns, ProjectDnsSearch, ProjectEnvironmentFile, ProjectGrant, ProjectLogging, ProjectLoggingOption,
    ProjectLoggingOptionValue, ProjectLoggingOptions, ProjectSysctls, ProjectTmpfs, ProjectUlimit, ProjectUlimitRange,
    ProjectUlimitScalar, ProjectUlimitValue, ProjectUlimits, build_project_view,
};
use compose_lens::render::{
    ComposeDocumentBuilder, GeneratedAnnotation, GeneratedConfigFileDefinition, GeneratedDevice, GeneratedDns,
    GeneratedDnsSearch, GeneratedEntrypoint, GeneratedEnvironmentFile, GeneratedHostname, GeneratedLabel,
    GeneratedLogging, GeneratedLoggingOption, GeneratedLoggingOptionValue, GeneratedLongDevice, GeneratedMemLimit,
    GeneratedNetworkAttachment, GeneratedNetworkDefinition, GeneratedNetworkDriverOption,
    GeneratedNetworkDriverOptionValue, GeneratedPidsLimit, GeneratedPullPolicy, GeneratedRestartPolicy,
    GeneratedSecretFileDefinition, GeneratedService, GeneratedShmSize, GeneratedString, GeneratedSysctl,
    GeneratedSysctls, GeneratedTmpfs, GeneratedUlimit, GeneratedUlimitValue, GeneratedUlimits,
    GeneratedVolumeDefinition, GeneratedVolumeDriverOption, GeneratedVolumeDriverOptionValue, ReplacementScalar,
    ScalarEdit, apply_preservation_edits, render_canonical,
};
use std::path::PathBuf;

struct SyntheticIncludeLoader;

impl IncludeLoader for SyntheticIncludeLoader {
    fn load_include(&self, request: &IncludeRequest) -> Result<IncludedProjectInput, IncludeLoadError> {
        match request.paths().first().map(compose_lens::model::Located::value) {
            Some(path) if path == "child.yaml" => Ok(IncludedProjectInput::new(
                IncludeIdentity::new("synthetic-child"),
                [DocumentInput::new(
                    SourceId::new(15_001),
                    DocumentOrigin::new("synthetic-child.yaml", "synthetic"),
                    "services: {child: {image: example/child}}\nnetworks: {child-network: {}}\n",
                )],
            )),
            _ => Err(IncludeLoadError::denied("synthetic loader only authorizes child.yaml")),
        }
    }
}

struct SyntheticDirectoryResolver;

impl IncludeProjectDirectoryResolver for SyntheticDirectoryResolver {
    fn resolve_project_directory(
        &self,
        request: &IncludeProjectDirectoryRequest<'_>,
    ) -> Result<IncludeProjectDirectoryResolution, IncludeProjectDirectoryResolveError> {
        if request.parent_node_index() != 0
            || request.child_node_index() != 1
            || request.declaration().value() != "synthetic-directory"
        {
            return Err(IncludeProjectDirectoryResolveError::Unresolved);
        }
        Ok(IncludeProjectDirectoryResolution::Resolved(PathBuf::from(
            "authorized/synthetic-directory",
        )))
    }
}

#[test]
fn preserves_build_field_kind_discriminants() {
    let published = [
        (BuildFieldKind::AdditionalContexts, 0),
        (BuildFieldKind::Args, 1),
        (BuildFieldKind::CacheFrom, 2),
        (BuildFieldKind::CacheTo, 3),
        (BuildFieldKind::Context, 4),
        (BuildFieldKind::Dockerfile, 5),
        (BuildFieldKind::DockerfileInline, 6),
        (BuildFieldKind::Entitlements, 7),
        (BuildFieldKind::ExtraHosts, 8),
        (BuildFieldKind::Isolation, 9),
        (BuildFieldKind::Labels, 10),
        (BuildFieldKind::Network, 11),
        (BuildFieldKind::NoCache, 12),
        (BuildFieldKind::Platforms, 13),
        (BuildFieldKind::Privileged, 14),
        (BuildFieldKind::Provenance, 15),
        (BuildFieldKind::Pull, 16),
        (BuildFieldKind::Sbom, 17),
        (BuildFieldKind::Secrets, 18),
        (BuildFieldKind::Ssh, 19),
        (BuildFieldKind::ShmSize, 20),
        (BuildFieldKind::Tags, 21),
        (BuildFieldKind::Target, 22),
        (BuildFieldKind::Ulimits, 23),
        (BuildFieldKind::NoCacheFilter, 24),
    ];

    for (kind, discriminant) in published {
        assert_eq!(kind as usize, discriminant);
    }
}

#[test]
fn exposes_authored_effective_and_generated_annotations_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n    annotations: [\"io.example.owner=platform\", \"io.example.key-only\"]\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(700), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::annotations)
            .map(compose_lens::model::Annotations::form),
        Some(compose_lens::model::AnnotationsForm::List(items)) if items.len() == 2
    ));

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(701),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let annotations = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::annotations)
        .ok_or("effective annotations expected")?;
    let owner: &compose_lens::project::ProjectAnnotationEntry = annotations
        .value()
        .get("io.example.owner")
        .ok_or("owner annotation expected")?;
    assert!(owner.value().is_some());
    assert!(
        annotations
            .value()
            .get("io.example.key-only")
            .is_some_and(|entry| entry.value().is_none())
    );

    let mut service = GeneratedService::new("app")?;
    service.set_annotations(vec![GeneratedAnnotation::new(
        "io.example.owner",
        GeneratedString::plain("platform")?,
    )?])?;
    assert_eq!(service.annotations().map(<[GeneratedAnnotation]>::len), Some(1));
    Ok(())
}

#[test]
fn exposes_service_runtime_key_contracts_at_authored_and_generated_boundaries() -> Result<(), Box<dyn std::error::Error>>
{
    let syntax = SyntaxDocument::parse(
        SourceId::new(9110),
        "services:\n  app:\n    cpu_rt_runtime: 500us\n    device_cgroup_rules: [rule, true]\n    volumes_from: [db, false]\n    scale: 2\n",
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let service = parsed
        .document()
        .and_then(|document| document.service("app"))
        .ok_or("service expected")?;
    assert!(service.cpu_rt_runtime().is_some());
    assert_eq!(service.invalid_device_cgroup_rules().len(), 1);
    assert_eq!(service.invalid_volumes_from().len(), 1);
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9111),
        DocumentOrigin::new("compose.yaml", "workspace"),
        "services:\n  app:\n    cpu_rt_runtime: 500us\n    device_cgroup_rules: [rule, true]\n    volumes_from: [db, false]\n    scale: 2\n",
    )])?;
    let merged = merge_project(&loaded, None);
    let project = build_project_view(merged.project().ok_or("project expected")?, None);
    let effective = project
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("effective service expected")?;
    assert!(effective.cpu_rt_runtime().is_some() && effective.scale().is_some());
    assert_eq!(effective.invalid_device_cgroup_rules().len(), 1);
    assert_eq!(effective.invalid_volumes_from().len(), 1);
    let mut generated = GeneratedService::new("generated")?;
    generated.add_runtime_field(compose_lens::render::GeneratedServiceRuntimeField::Scale(
        GeneratedString::plain("2")?,
    ))?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(generated)?;
    assert!(
        builder
            .build(SourceId::new(9112))?
            .document()
            .service("generated")
            .is_some()
    );
    Ok(())
}

#[test]
fn exposes_sensitive_build_ssh_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source =
        "services:\n  app:\n    build:\n      ssh:\n        default: /private/agent.sock\n        retries: 2\n";
    let syntax = SyntaxDocument::parse(SourceId::new(997), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => definition.ssh(),
            Build::Context(_) => None,
        })
        .ok_or("authored build ssh expected")?;
    assert_eq!(authored.form(), BuildSshForm::Map);
    assert_eq!(
        authored.as_map().map(<[compose_lens::model::KeyValueEntry]>::len),
        Some(2)
    );
    assert!(!format!("{authored:?}").contains("/private/agent.sock"));

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(998),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project_view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let ssh = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|build| match build.value() {
            ProjectBuild::Definition(definition) => definition.ssh(),
            _ => None,
        })
        .ok_or("effective build ssh expected")?;
    assert!(ssh.is_sensitive());
    assert!(matches!(ssh.value(), ProjectBuildSsh::Map(entries) if entries.len() == 2));
    assert!(!format!("{ssh:?}").contains("/private/agent.sock"));
    Ok(())
}

#[test]
fn exposes_authored_and_effective_build_context_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "secrets:\n  build-secret: {}\n",
        "services:\n",
        "  short:\n    build: ./short\n",
        "  long:\n    build:\n      context: ./long\n      dockerfile_inline: \"FROM scratch\\nRUN echo public\"\n      target: \"\"\n      cache_from: [\"type=local,src=.cache\", \"type=local,src=.cache\"]\n      cache_to: [\"type=local,dest=.cache\", \"type=local,dest=.cache\"]\n      platforms: [linux/amd64, linux/amd64]\n      tags: [example/app:one, example/app:one]\n      secrets: [build-secret, {source: build-secret, target: /run/secrets/build, uid: \"1000\", gid: \"1001\", mode: \"0440\"}]\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(705), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|document| document.service("short"))
            .and_then(compose_lens::model::Service::build),
        Some(Build::Context(context)) if context.value() == "./short"
    ));
    assert!(matches!(
        parsed
            .document()
            .and_then(|document| document.service("long"))
            .and_then(compose_lens::model::Service::build),
        Some(Build::Definition(definition)) if definition.context().is_some() && definition.dockerfile_inline().is_some() && definition.target().is_some() && definition.cache_from().is_some() && definition.cache_to().is_some() && definition.platforms().is_some() && definition.tags().is_some() && definition.secrets().is_some()
    ));
    let Some(Build::Definition(authored_long)) = parsed
        .document()
        .and_then(|document| document.service("long"))
        .and_then(compose_lens::model::Service::build)
    else {
        return Err("authored long build definition expected".into());
    };
    assert_eq!(
        authored_long
            .target()
            .map(compose_lens::model::Located::value)
            .map(String::as_str),
        Some("")
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(706),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    assert!(matches!(
        view.view()
            .and_then(|view| view.service("short"))
            .and_then(compose_lens::project::ProjectService::build)
            .map(compose_lens::project::ProjectValue::value),
        Some(ProjectBuild::Context(context)) if context.value() == "./short"
    ));
    let long = view
        .view()
        .and_then(|view| view.service("long"))
        .and_then(compose_lens::project::ProjectService::build)
        .ok_or("long build expected")?;
    let ProjectBuild::Definition(definition) = long.value() else {
        return Err("long build definition expected".into());
    };
    let _: &ProjectBuildDefinition = definition;
    assert_eq!(
        definition
            .context()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str),
        Some("./long")
    );
    assert_eq!(
        definition
            .dockerfile_inline()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str),
        Some("FROM scratch\nRUN echo public")
    );
    assert_eq!(
        definition
            .target()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str),
        Some("")
    );
    assert_public_build_sequences(authored_long, definition)?;
    assert_public_build_secrets(definition)?;
    assert!(definition.unmodeled_fields().is_empty());
    Ok(())
}

#[test]
fn exposes_distinct_build_provenance_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    build: {provenance: \"${MODE}\"}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(2201), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|d| d.service("app"))
        .and_then(compose_lens::model::Service::build)
        .and_then(|b| match b {
            Build::Definition(d) => d.provenance(),
            Build::Context(_) => None,
        })
        .ok_or("authored provenance expected")?;
    assert!(matches!(authored.value(), compose_lens::model::BuildProvenance::String(value) if value == "${MODE}"));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(2201),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("MODE", "mode=max");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let view = build_project_view(merged.project().ok_or("merged expected")?, None);
    let effective = view
        .view()
        .and_then(|v| v.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|b| match b.value() {
            ProjectBuild::Definition(d) => d.provenance(),
            _ => None,
        })
        .ok_or("effective provenance expected")?;
    assert!(matches!(effective.value(), compose_lens::model::BuildProvenance::String(value) if value == "mode=max"));
    assert!(effective.is_sensitive());
    Ok(())
}

#[test]
fn exposes_build_no_cache_filter_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    build: {no_cache_filter: [one, two]}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(2304), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|d| d.service("app"))
        .and_then(compose_lens::model::Service::build)
        .and_then(|b| match b {
            Build::Definition(d) => d.no_cache_filter(),
            Build::Context(_) => None,
        })
        .ok_or("authored")?;
    assert!(matches!(authored,compose_lens::model::BuildNoCacheFilter::List(v)if v.len()==2));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(2304),
        DocumentOrigin::new("a", "w"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged")?, None);
    let effective = view
        .view()
        .and_then(|v| v.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|b| match b.value() {
            ProjectBuild::Definition(d) => d.no_cache_filter(),
            _ => None,
        })
        .ok_or("effective")?;
    assert!(matches!(effective.value(),compose_lens::project::ProjectBuildNoCacheFilter::List(v)if v.len()==2));
    Ok(())
}

#[test]
fn exposes_deploy_endpoint_mode_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    deploy: {endpoint_mode: dnsrr, replicas: 2}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(2504), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.endpoint_mode())
        .ok_or("authored endpoint mode")?;
    assert!(matches!(authored.value(), DeployEndpointMode::Dnsrr));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(2504),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let deploy = view
        .view()
        .and_then(|project| project.service("app"))
        .and_then(compose_lens::project::ProjectService::deploy)
        .ok_or("effective deploy")?;
    assert!(matches!(
        deploy
            .value()
            .endpoint_mode()
            .map(compose_lens::project::ProjectValue::value),
        Some(DeployEndpointMode::Dnsrr)
    ));
    assert!(matches!(
        deploy
            .value()
            .replicas()
            .map(compose_lens::project::ProjectValue::value),
        Some(DeployReplicas::YamlNumber(value)) if value == "2"
    ));
    Ok(())
}

#[test]
fn exposes_deploy_mode_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    deploy: {mode: global, replicas: 2}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(2604), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.mode())
            .map(compose_lens::model::Located::value),
        Some(DeployMode::Global)
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(2604),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let deploy = view
        .view()
        .and_then(|project| project.service("app"))
        .and_then(compose_lens::project::ProjectService::deploy)
        .ok_or("effective deploy")?;
    assert!(matches!(
        deploy.value().mode().map(compose_lens::project::ProjectValue::value),
        Some(DeployMode::Global)
    ));
    Ok(())
}

#[test]
fn exposes_deploy_replicas_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    deploy: {replicas: 1.50}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(2704), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed.document().and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.replicas())
            .map(compose_lens::model::Located::value),
        Some(DeployReplicas::YamlNumber(value)) if value == "1.50"
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(2704),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let deploy = view
        .view()
        .and_then(|project| project.service("app"))
        .and_then(compose_lens::project::ProjectService::deploy)
        .ok_or("effective deploy")?;
    assert!(matches!(
        deploy.value().replicas().map(compose_lens::project::ProjectValue::value),
        Some(DeployReplicas::YamlNumber(value)) if value == "1.50"
    ));
    Ok(())
}

#[test]
fn exposes_deploy_labels_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  map:\n    deploy: {labels: {owner: platform}}\n  list:\n    deploy: {labels: [bare, owner=platform]}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(2804), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed.document().and_then(|document| document.service("map"))
            .and_then(compose_lens::model::Service::deploy).and_then(|deploy| deploy.labels()),
        Some(compose_lens::model::Labels::Map { entries, .. }) if entries.len() == 1
    ));
    assert!(matches!(
        parsed.document().and_then(|document| document.service("list"))
            .and_then(compose_lens::model::Service::deploy).and_then(|deploy| deploy.labels()),
        Some(compose_lens::model::Labels::List { values, .. }) if values.len() == 2
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(2804),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let map = view
        .view()
        .and_then(|project| project.service("map"))
        .and_then(compose_lens::project::ProjectService::deploy)
        .and_then(|deploy| deploy.value().labels())
        .ok_or("effective mapping labels")?;
    assert!(map.value().get("owner").is_some());
    assert!(matches!(
        map.value().form(),
        compose_lens::project::ProjectLabelsForm::Map
    ));
    let list = view
        .view()
        .and_then(|project| project.service("list"))
        .and_then(compose_lens::project::ProjectService::deploy)
        .and_then(|deploy| deploy.value().labels())
        .ok_or("effective list labels")?;
    assert_eq!(list.value().entries().len(), 2);
    assert!(matches!(
        list.value().form(),
        compose_lens::project::ProjectLabelsForm::List
    ));
    Ok(())
}

#[test]
fn exposes_deploy_update_config_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source =
        "services:\n  app:\n    deploy:\n      update_config:\n        parallelism: 2\n        order: start-first\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3153), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|doc| doc.service("app"))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.update_config())
            .and_then(compose_lens::model::DeployUpdateConfig::order)
            .map(compose_lens::model::Located::value),
        Some(compose_lens::model::DeployUpdateOrder::StartFirst)
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3153),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    assert!(
        build_project_view(merged.project().ok_or("merged")?, None)
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::deploy)
            .and_then(|deploy| deploy.value().update_config())
            .is_some()
    );
    Ok(())
}

#[test]
fn exposes_deploy_rollback_config_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source =
        "services:\n  app:\n    deploy:\n      rollback_config:\n        parallelism: 2\n        order: stop-first\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3157), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|doc| doc.service("app"))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.rollback_config())
            .and_then(compose_lens::model::DeployRollbackConfig::order)
            .map(compose_lens::model::Located::value),
        Some(compose_lens::model::DeployRollbackOrder::StopFirst)
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3157),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    assert!(
        build_project_view(merged.project().ok_or("merged")?, None)
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::deploy)
            .and_then(|deploy| deploy.value().rollback_config())
            .is_some()
    );
    Ok(())
}

#[test]
fn exposes_credential_spec_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    credential_spec:\n      config: credential\n      file: C:\\\\gmsa.json\n      registry: registry://account\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3161), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert_eq!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::credential_spec)
            .and_then(compose_lens::model::CredentialSpec::registry)
            .map(compose_lens::model::Located::value)
            .map(String::as_str),
        Some("registry://account")
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3161),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    assert!(
        build_project_view(merged.project().ok_or("merged")?, None)
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::credential_spec)
            .is_some()
    );
    Ok(())
}

#[test]
fn exposes_extends_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    extends:\n      service: parent\n      file: ./base.yml\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3165), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::extends),
        Some(compose_lens::model::Extends::Long(reference))
            if reference.service().map(compose_lens::model::Located::value).map(String::as_str) == Some("parent")
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3165),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    assert!(matches!(
        build_project_view(merged.project().ok_or("merged")?, None)
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::extends)
            .map(compose_lens::project::ProjectValue::value),
        Some(compose_lens::project::ProjectExtends::Long(_))
    ));
    Ok(())
}

#[test]
fn exposes_provider_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    provider:\n      type: example\n      options:\n        enabled: true\n        values: [one, 2]\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3167), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::provider)
            .and_then(compose_lens::model::Provider::options)
            .and_then(|options| options.entries().get(1))
            .map(compose_lens::model::ProviderOption::value),
        Some(compose_lens::model::ProviderOptionValue::Sequence { items, .. }) if items.len() == 2
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3168),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let provider = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::provider)
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective provider")?;
    assert!(matches!(
        provider.options().map(compose_lens::project::ProjectValue::value),
        Some(options) if matches!(
            options.entries()[0].value().value().value(),
            compose_lens::project::ProjectProviderOptionValue::Scalar(compose_lens::model::ComposeScalar::Boolean(true))
        )
    ));
    Ok(())
}

#[test]
fn exposes_post_start_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    post_start:\n      - command: null\n        environment: [HOOK=true]\n        privileged: false\n        user: hook-user\n        working_dir: /hook\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3178), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::post_start)
            .and_then(|hooks| hooks.entries().first()),
        Some(compose_lens::model::PostStartHook::Hook(hook))
            if matches!(hook.command(), Some(compose_lens::model::Command::Null(_)))
                && hook.environment().is_some()
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3179),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let hooks = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::post_start)
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective post-start hooks")?;
    assert!(matches!(
        hooks.first().map(compose_lens::project::ProjectValue::value),
        Some(compose_lens::project::ProjectPostStartHook::Hook(hook))
            if hook.command().is_some() && hook.environment().is_some() && hook.privileged().is_some()
    ));
    Ok(())
}

#[test]
fn exposes_pre_stop_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    pre_stop:\n      - command: null\n        environment: [HOOK=true]\n        privileged: false\n        user: hook-user\n        working_dir: /hook\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3185), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::pre_stop)
            .and_then(|hooks| hooks.entries().first()),
        Some(compose_lens::model::PreStopHook::Hook(hook))
            if matches!(hook.command(), Some(compose_lens::model::Command::Null(_)))
                && hook.environment().is_some()
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3186),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let hooks = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::pre_stop)
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective pre-stop hooks")?;
    assert!(matches!(
        hooks.first().map(compose_lens::project::ProjectValue::value),
        Some(compose_lens::project::ProjectPreStopHook::Hook(hook))
            if hook.command().is_some() && hook.environment().is_some() && hook.privileged().is_some()
    ));
    Ok(())
}

#[test]
fn exposes_pre_start_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    pre_start:\n      - command: null\n        image: hook-image\n        environment: [HOOK=true]\n        privileged: false\n        per_replica: true\n        user: hook-user\n        working_dir: /hook\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3190), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::pre_start)
            .and_then(|hooks| hooks.entries().first()),
        Some(compose_lens::model::PreStartHook::Hook(hook))
            if matches!(hook.command(), Some(compose_lens::model::Command::Null(_)))
                && hook.image().is_some()
                && hook.per_replica().is_some()
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3191),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let hooks = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::pre_start)
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective pre-start hooks")?;
    assert!(matches!(
        hooks.first().map(compose_lens::project::ProjectValue::value),
        Some(compose_lens::project::ProjectPreStartHook::Hook(hook))
            if hook.command().is_some() && hook.image().is_some() && hook.environment().is_some()
                && hook.privileged().is_some() && hook.per_replica().is_some()
    ));
    Ok(())
}

#[test]
fn exposes_service_runtime_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    runtime: \"${RUNTIME}\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3197), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert_eq!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::runtime)
            .map(compose_lens::model::Located::value)
            .map(String::as_str),
        Some("${RUNTIME}")
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3198),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("RUNTIME", "private-runtime");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let runtime = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::runtime)
        .ok_or("effective runtime")?;
    assert_eq!(runtime.value(), "private-runtime");
    assert!(runtime.is_sensitive());
    assert!(!format!("{runtime:?}").contains("private-runtime"));
    Ok(())
}

#[test]
fn exposes_service_cgroup_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    cgroup: \"${CGROUP}\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3236), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::cgroup)
        .ok_or("authored cgroup")?;
    let _: &compose_lens::model::CgroupNamespace = authored;
    assert!(matches!(
        authored.kind(),
        compose_lens::model::CgroupNamespaceKind::Expression(value) if value == "${CGROUP}"
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3237),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CGROUP", "private");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let cgroup = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::cgroup)
        .ok_or("effective cgroup")?;
    let _: &compose_lens::model::CgroupNamespace = cgroup.value();
    assert!(cgroup.is_sensitive());
    assert!(matches!(
        cgroup.value().kind(),
        compose_lens::model::CgroupNamespaceKind::Private
    ));
    Ok(())
}

#[test]
fn exposes_service_cgroup_parent_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    cgroup_parent: \"${PARENT}\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3246), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::cgroup_parent)
        .ok_or("authored cgroup parent")?;
    let _: &compose_lens::model::Located<String> = authored;
    assert_eq!(authored.value(), "${PARENT}");
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3247),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("PARENT", "private-parent");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let parent = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::cgroup_parent)
        .ok_or("effective cgroup parent")?;
    let _: &String = parent.value();
    assert_eq!(parent.value(), "private-parent");
    assert!(parent.is_sensitive());
    assert!(!format!("{parent:?}").contains("private-parent"));
    Ok(())
}

#[test]
fn exposes_service_cpu_count_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    cpu_count: \"${CPU_COUNT}\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3256), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::cpu_count)
        .ok_or("authored cpu count")?;
    let _: &compose_lens::model::Located<compose_lens::model::CpuCount> = authored;
    assert!(matches!(authored.value(), compose_lens::model::CpuCount::String(value) if value == "${CPU_COUNT}"));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3257),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CPU_COUNT", "private-cpu-count");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let count = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::cpu_count)
        .ok_or("effective cpu count")?;
    let _: &compose_lens::model::CpuCount = count.value();
    assert!(matches!(count.value(), compose_lens::model::CpuCount::String(value) if value == "private-cpu-count"));
    assert!(count.is_sensitive());
    assert!(!format!("{count:?}").contains("private-cpu-count"));
    Ok(())
}

#[test]
fn exposes_service_cpu_percent_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    cpu_percent: \"${CPU_PERCENT}\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3264), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::cpu_percent)
        .ok_or("authored cpu percent")?;
    let _: &compose_lens::model::Located<compose_lens::model::CpuPercent> = authored;
    assert!(matches!(
        authored.value(),
        compose_lens::model::CpuPercent::String(value) if value == "${CPU_PERCENT}"
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3265),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CPU_PERCENT", "private-cpu-percent");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let percent = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::cpu_percent)
        .ok_or("effective cpu percent")?;
    let _: &compose_lens::model::CpuPercent = percent.value();
    assert!(matches!(
        percent.value(),
        compose_lens::model::CpuPercent::String(value) if value == "private-cpu-percent"
    ));
    assert!(percent.is_sensitive());
    assert!(!format!("{percent:?}").contains("private-cpu-percent"));
    Ok(())
}

#[test]
fn exposes_service_cpu_period_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    cpu_period: \"${CPU_PERIOD}\"\n";
    let parsed = ComposeDocument::parse(SyntaxDocument::parse(SourceId::new(3272), source)?.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::cpu_period)
        .ok_or("authored cpu period")?;
    let _: &compose_lens::model::Located<compose_lens::model::CpuPeriod> = authored;
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3273),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CPU_PERIOD", "private-cpu-period");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let period = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::cpu_period)
        .ok_or("effective cpu period")?;
    let _: &compose_lens::model::CpuPeriod = period.value();
    assert!(period.is_sensitive());
    assert!(!format!("{period:?}").contains("private-cpu-period"));
    Ok(())
}

#[test]
fn exposes_service_cpu_quota_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    cpu_quota: \"${CPU_QUOTA}\"\n";
    let parsed = ComposeDocument::parse(SyntaxDocument::parse(SourceId::new(3283), source)?.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::cpu_quota)
        .ok_or("authored cpu quota")?;
    let _: &compose_lens::model::Located<compose_lens::model::CpuQuota> = authored;
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3284),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CPU_QUOTA", "private-cpu-quota");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let quota = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::cpu_quota)
        .ok_or("effective cpu quota")?;
    let _: &compose_lens::model::CpuQuota = quota.value();
    assert!(quota.is_sensitive());
    assert!(!format!("{quota:?}").contains("private-cpu-quota"));
    Ok(())
}

#[test]
fn exposes_service_cpu_rt_period_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    cpu_rt_period: \"${CPU_RT_PERIOD}\"\n";
    let parsed = ComposeDocument::parse(SyntaxDocument::parse(SourceId::new(3291), source)?.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::cpu_rt_period)
        .ok_or("authored cpu rt period")?;
    let _: &compose_lens::model::Located<compose_lens::model::CpuRtPeriod> = authored;
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3292),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CPU_RT_PERIOD", "1m30s");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let period = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::cpu_rt_period)
        .ok_or("effective cpu rt period")?;
    let _: &compose_lens::model::CpuRtPeriod = period.value();
    assert!(period.is_sensitive());
    assert!(!format!("{period:?}").contains("1m30s"));
    Ok(())
}

#[test]
fn exposes_service_pull_refresh_after_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    pull_refresh_after: \"${REFRESH_AFTER}\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3206), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert_eq!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::pull_refresh_after)
            .map(compose_lens::model::Located::value)
            .map(String::as_str),
        Some("${REFRESH_AFTER}")
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3207),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("REFRESH_AFTER", "private-refresh");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let pull_refresh_after = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::pull_refresh_after)
        .ok_or("effective pull refresh interval")?;
    assert_eq!(pull_refresh_after.value(), "private-refresh");
    assert!(pull_refresh_after.is_sensitive());
    assert!(!format!("{pull_refresh_after:?}").contains("private-refresh"));
    Ok(())
}

#[test]
fn exposes_service_platform_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    platform: \"${PLATFORM}\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3213), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert_eq!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::platform)
            .map(compose_lens::model::Located::value)
            .map(String::as_str),
        Some("${PLATFORM}")
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3214),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("PLATFORM", "private-platform");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = build_project_view(merged.project().ok_or("merged")?, None);
    let platform = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::platform)
        .ok_or("effective platform")?;
    assert_eq!(platform.value(), "private-platform");
    assert!(platform.is_sensitive());
    assert!(!format!("{platform:?}").contains("private-platform"));
    Ok(())
}

#[test]
fn exposes_deploy_restart_policy_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    deploy: {restart_policy: {condition: on-failure, max_attempts: 3}}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(2904), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|doc| doc.service("app"))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.restart_policy())
            .and_then(|policy| policy.condition())
            .map(compose_lens::model::Located::value),
        Some(DeployRestartCondition::OnFailure)
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(2904),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged")?, None);
    assert!(
        view.view()
            .and_then(|project| project.service("app"))
            .and_then(compose_lens::project::ProjectService::deploy)
            .and_then(|deploy| deploy.value().restart_policy())
            .is_some()
    );
    Ok(())
}

#[test]
fn exposes_deploy_placement_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    deploy:\n",
        "      placement:\n",
        "        constraints: [node.labels.zone == east]\n",
        "        preferences: [{spread: node.labels.rack}]\n",
        "        max_replicas_per_node: 003\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3004), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|doc| doc.service("app"))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.placement())
            .and_then(|placement| placement.max_replicas_per_node())
            .map(compose_lens::model::Located::value),
        Some(DeployPlacementMaxReplicasPerNode::YamlInteger(value)) if value == "003"
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3004),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    assert!(
        view.view()
            .and_then(|project| project.service("app"))
            .and_then(compose_lens::project::ProjectService::deploy)
            .and_then(|deploy| deploy.value().placement())
            .is_some()
    );
    Ok(())
}

#[test]
fn exposes_deploy_resource_pids_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    deploy:\n      resources:\n        limits:\n          pids: 003\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3104), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|doc| doc.service("app"))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.limits())
            .and_then(|limits| limits.pids())
            .map(compose_lens::model::Located::value),
        Some(DeployResourcePids::YamlInteger(value)) if value == "003"
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3104),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    assert!(
        build_project_view(merged.project().ok_or("merged project expected")?, None)
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::deploy)
            .and_then(|deploy| deploy.value().resources())
            .and_then(|resources| resources.value().limits())
            .and_then(|limits| limits.value().pids())
            .is_some()
    );
    Ok(())
}

#[test]
fn exposes_deploy_resource_cpus_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    deploy:\n      resources:\n        limits:\n          cpus: 0.5\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3105), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|doc| doc.service("app"))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.limits())
            .and_then(|limits| limits.cpus())
            .map(compose_lens::model::Located::value),
        Some(DeployResourceCpus::YamlNumber(value)) if value == "0.5"
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3105),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    assert!(matches!(
        build_project_view(merged.project().ok_or("merged project expected")?, None)
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::deploy)
            .and_then(|deploy| deploy.value().resources())
            .and_then(|resources| resources.value().limits())
            .and_then(|limits| limits.value().cpus())
            .map(compose_lens::project::ProjectValue::value),
        Some(DeployResourceCpus::YamlNumber(value)) if value == "0.5"
    ));
    Ok(())
}

#[test]
fn exposes_deploy_resource_reservation_cpus_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    deploy:\n      resources:\n        reservations:\n          cpus: 0.5\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3110), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed.document().and_then(|doc| doc.service("app"))
            .and_then(compose_lens::model::Service::deploy).and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.reservations()).and_then(|reservations| reservations.cpus())
            .map(compose_lens::model::Located::value),
        Some(DeployResourceCpus::YamlNumber(value)) if value == "0.5"
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3110),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    assert!(matches!(
        build_project_view(merged.project().ok_or("merged project expected")?, None).view()
            .and_then(|view| view.service("app")).and_then(compose_lens::project::ProjectService::deploy)
            .and_then(|deploy| deploy.value().resources()).and_then(|resources| resources.value().reservations())
            .and_then(|reservations| reservations.value().cpus()).map(compose_lens::project::ProjectValue::value),
        Some(DeployResourceCpus::YamlNumber(value)) if value == "0.5"
    ));
    Ok(())
}

#[test]
fn exposes_deploy_resource_reservation_memory_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    deploy:\n      resources:\n        reservations:\n          memory: \"50m\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3112), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|doc| doc.service("app"))
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .and_then(|resources| resources.reservations())
        .and_then(|reservations| reservations.memory())
        .ok_or("authored reservation memory expected")?;
    assert_eq!(authored.value().raw(), "50m");
    assert!(matches!(
        authored.value().kind(),
        DeployResourceMemoryKind::Documented { amount_raw, unit: DeployResourceMemoryUnit::M } if amount_raw == "50"
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3112),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let effective = view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::deploy)
        .and_then(|deploy| deploy.value().resources())
        .and_then(|resources| resources.value().reservations())
        .and_then(|reservations| reservations.value().memory())
        .ok_or("effective reservation memory expected")?;
    assert_eq!(effective.value().raw(), "50m");
    assert!(matches!(
        effective.value().kind(),
        DeployResourceMemoryKind::Documented { amount_raw, unit: DeployResourceMemoryUnit::M } if amount_raw == "50"
    ));
    Ok(())
}

#[test]
fn exposes_reservation_generic_resources_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    deploy:\n      resources:\n        reservations:\n          generic_resources:\n            - discrete_resource_spec:\n                kind: gpu\n                value: 1.5\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3114), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|doc| doc.service("app"))
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .and_then(|resources| resources.reservations())
        .and_then(|reservations| reservations.generic_resources())
        .ok_or("authored generic resources")?;
    assert!(
        matches!(authored.items().first().and_then(|item| item.discrete_resource_spec()).and_then(|spec| spec.value()).map(compose_lens::model::Located::value), Some(DeployDiscreteResourceValue::YamlNumber(value)) if value == "1.5")
    );
    assert!(matches!(
        authored
            .items()
            .first()
            .map(compose_lens::model::DeployGenericResource::form),
        Some(compose_lens::model::DeployGenericResourceForm::Mapping)
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3114),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    assert_eq!(
        build_project_view(merged.project().ok_or("merged")?, None)
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::deploy)
            .and_then(|deploy| deploy.value().resources())
            .and_then(|resources| resources.value().reservations())
            .and_then(|reservations| reservations.value().generic_resources())
            .and_then(|values| values.value().first())
            .map(|item| item.value().form()),
        Some(compose_lens::project::ProjectDeployGenericResourceForm::Mapping)
    );
    Ok(())
}

#[test]
fn exposes_reservation_devices_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu, custom]\n              driver: nvidia\n              count: 2\n              device_ids: [gpu-0]\n              options: {mode: shared}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3118), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|doc| doc.service("app"))
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .and_then(|resources| resources.reservations())
        .and_then(|reservations| reservations.devices())
        .ok_or("authored reservation devices")?;
    assert!(matches!(
        authored
            .items()
            .first()
            .map(compose_lens::model::DeployReservationDevice::form),
        Some(compose_lens::model::DeployReservationDeviceForm::Mapping)
    ));
    assert_eq!(
        authored
            .items()
            .first()
            .and_then(|device| device.capabilities())
            .map(|capabilities| capabilities.items().len()),
        Some(2)
    );
    assert_eq!(
        authored
            .items()
            .first()
            .and_then(compose_lens::model::DeployReservationDevice::driver)
            .map(compose_lens::model::Located::value)
            .map(String::as_str),
        Some("nvidia")
    );
    assert!(matches!(
        authored.items().first().and_then(compose_lens::model::DeployReservationDevice::count).map(compose_lens::model::Located::value),
        Some(DeployReservationDeviceCount::YamlInteger(value)) if value == "2"
    ));
    assert_eq!(
        authored
            .items()
            .first()
            .and_then(compose_lens::model::DeployReservationDevice::device_ids)
            .map(|ids| ids.items().len()),
        Some(1)
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3118),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    assert!(matches!(
        build_project_view(merged.project().ok_or("merged")?, None).view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::deploy)
            .and_then(|deploy| deploy.value().resources())
            .and_then(|resources| resources.value().reservations())
            .and_then(|reservations| reservations.value().devices())
            .and_then(|devices| devices.value().first())
            .and_then(|device| device.value().count())
            .map(compose_lens::project::ProjectValue::value),
        Some(DeployReservationDeviceCount::YamlInteger(value)) if value == "2"
    ));
    assert!(matches!(
        build_project_view(merged.project().ok_or("merged")?, None)
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::deploy)
            .and_then(|deploy| deploy.value().resources())
            .and_then(|resources| resources.value().reservations())
            .and_then(|reservations| reservations.value().devices())
            .and_then(|devices| devices.value().first())
            .map(|device| device.value().form()),
        Some(compose_lens::project::ProjectDeployReservationDeviceForm::Mapping)
    ));
    assert_eq!(
        build_project_view(merged.project().ok_or("merged")?, None)
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::deploy)
            .and_then(|deploy| deploy.value().resources())
            .and_then(|resources| resources.value().reservations())
            .and_then(|reservations| reservations.value().devices())
            .and_then(|devices| devices.value().first())
            .and_then(|device| device.value().driver())
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str),
        Some("nvidia")
    );
    Ok(())
}

#[test]
fn exposes_reservation_device_options_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              options: {mode: shared}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3119), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed.document().and_then(|doc| doc.service("app"))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.reservations())
            .and_then(|reservations| reservations.devices())
            .and_then(|devices| devices.items().first())
            .and_then(compose_lens::model::DeployReservationDevice::options)
            .and_then(compose_lens::model::DeployReservationDeviceOptions::as_map),
        Some([entry]) if entry.key().value() == "mode"
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3119),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    assert!(matches!(
        build_project_view(merged.project().ok_or("merged")?, None).view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::deploy)
            .and_then(|deploy| deploy.value().resources())
            .and_then(|resources| resources.value().reservations())
            .and_then(|reservations| reservations.value().devices())
            .and_then(|devices| devices.value().first())
            .and_then(|device| device.value().options())
            .and_then(|options| options.value().as_map()),
        Some([entry]) if entry.value().value().value()
            == &compose_lens::model::ComposeScalar::String("shared".to_owned())
    ));
    Ok(())
}

#[test]
fn exposes_deploy_resource_memory_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    deploy:\n      resources:\n        limits:\n          memory: \"50m\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3106), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|doc| doc.service("app"))
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .and_then(|resources| resources.limits())
        .and_then(|limits| limits.memory())
        .ok_or("authored deploy memory expected")?;
    assert_eq!(authored.value().raw(), "50m");
    assert!(matches!(
        authored.value().kind(),
        DeployResourceMemoryKind::Documented { amount_raw, unit: DeployResourceMemoryUnit::M } if amount_raw == "50"
    ));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3106),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let effective = view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::deploy)
        .and_then(|deploy| deploy.value().resources())
        .and_then(|resources| resources.value().limits())
        .and_then(|limits| limits.value().memory())
        .ok_or("effective deploy memory expected")?;
    assert_eq!(effective.value().raw(), "50m");
    assert!(matches!(
        effective.value().kind(),
        DeployResourceMemoryKind::Documented { amount_raw, unit: DeployResourceMemoryUnit::M } if amount_raw == "50"
    ));
    Ok(())
}

#[test]
fn exposes_build_privileged_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    build: {privileged: true}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(2404), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|d| d.service("app"))
        .and_then(compose_lens::model::Service::build)
        .and_then(|b| match b {
            Build::Definition(d) => d.privileged(),
            Build::Context(_) => None,
        })
        .ok_or("authored")?;
    assert_eq!(authored.value(), &BooleanValue::Literal(true));
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(2404),
        DocumentOrigin::new("a", "w"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged")?, None);
    let effective = view
        .view()
        .and_then(|v| v.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|b| match b.value() {
            ProjectBuild::Definition(d) => d.privileged(),
            _ => None,
        })
        .ok_or("effective")?;
    assert_eq!(effective.value(), &BooleanValue::Literal(true));
    Ok(())
}

fn assert_public_build_secrets(definition: &ProjectBuildDefinition) -> Result<(), Box<dyn std::error::Error>> {
    let secrets = definition.secrets().ok_or("effective build secrets expected")?;
    assert!(matches!(secrets.value().as_slice(), [short, long]
        if matches!(short.value(), ProjectGrant::Short(value) if value == "build-secret")
            && matches!(long.value(), ProjectGrant::Long(long) if long.source().is_some()
                && long.target().is_some() && long.uid().is_some() && long.gid().is_some() && long.mode().is_some())));
    Ok(())
}

fn assert_public_build_sequences(
    authored: &compose_lens::model::BuildDefinition,
    definition: &ProjectBuildDefinition,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        authored.tags().map(|tags| tags
            .iter()
            .map(compose_lens::model::Located::value)
            .map(String::as_str)
            .collect::<Vec<_>>()),
        Some(vec!["example/app:one", "example/app:one"])
    );
    assert_eq!(
        authored.platforms().map(|platforms| platforms
            .iter()
            .map(compose_lens::model::Located::value)
            .map(String::as_str)
            .collect::<Vec<_>>()),
        Some(vec!["linux/amd64", "linux/amd64"])
    );
    assert_eq!(
        authored.cache_from().map(|locations| locations
            .iter()
            .map(compose_lens::model::Located::value)
            .map(String::as_str)
            .collect::<Vec<_>>()),
        Some(vec!["type=local,src=.cache", "type=local,src=.cache"])
    );
    assert_eq!(
        authored.cache_to().map(|locations| locations
            .iter()
            .map(compose_lens::model::Located::value)
            .map(String::as_str)
            .collect::<Vec<_>>()),
        Some(vec!["type=local,dest=.cache", "type=local,dest=.cache"])
    );
    assert_eq!(
        definition
            .tags()
            .ok_or("effective build tags expected")?
            .value()
            .iter()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["example/app:one", "example/app:one"]
    );
    assert_eq!(
        definition
            .platforms()
            .ok_or("effective build platforms expected")?
            .value()
            .iter()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["linux/amd64", "linux/amd64"]
    );
    assert_eq!(
        definition
            .cache_from()
            .ok_or("effective build cache_from expected")?
            .value()
            .iter()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["type=local,src=.cache", "type=local,src=.cache"]
    );
    assert_eq!(
        definition
            .cache_to()
            .ok_or("effective build cache_to expected")?
            .value()
            .iter()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["type=local,dest=.cache", "type=local,dest=.cache"]
    );
    Ok(())
}

#[test]
fn exposes_authored_and_effective_build_network_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    build:\n      network: \"\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(708), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .ok_or("authored build expected")?;
    let Build::Definition(authored) = authored else {
        return Err("authored build definition expected".into());
    };
    assert_eq!(
        authored
            .network()
            .map(compose_lens::model::Located::value)
            .map(String::as_str),
        Some("")
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(709),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let definition = view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|build| match build.value() {
            ProjectBuild::Definition(definition) => Some(definition),
            _ => None,
        })
        .ok_or("effective build definition expected")?;
    assert_eq!(
        definition
            .network()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str),
        Some("")
    );
    Ok(())
}

#[test]
fn exposes_authored_and_effective_build_isolation_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    build:\n      isolation: \"${BUILD_ISOLATION}\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(849), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let Build::Definition(authored) = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .ok_or("authored build expected")?
    else {
        return Err("authored build definition expected".into());
    };
    assert_eq!(
        authored
            .isolation()
            .map(compose_lens::model::Located::value)
            .map(String::as_str),
        Some("${BUILD_ISOLATION}")
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(850),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("BUILD_ISOLATION", "process");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let definition = view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|build| match build.value() {
            ProjectBuild::Definition(definition) => Some(definition),
            _ => None,
        })
        .ok_or("effective build definition expected")?;
    let _: &ProjectBuildDefinition = definition;
    assert_eq!(
        definition
            .isolation()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str),
        Some("process")
    );
    Ok(())
}

#[test]
fn exposes_authored_and_effective_build_pull_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    build:\n      pull: \"${BUILD_PULL:-false}\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(834), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .ok_or("authored build expected")?;
    let Build::Definition(authored) = authored else {
        return Err("authored build definition expected".into());
    };
    assert_eq!(
        authored.pull().map(compose_lens::model::Located::value),
        Some(&BooleanValue::Expression("${BUILD_PULL:-false}".to_owned()))
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(835),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let definition = view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|build| match build.value() {
            ProjectBuild::Definition(definition) => Some(definition),
            _ => None,
        })
        .ok_or("effective build definition expected")?;
    assert_eq!(
        definition.pull().map(compose_lens::project::ProjectValue::value),
        Some(&BooleanValue::Expression("${BUILD_PULL:-false}".to_owned()))
    );
    Ok(())
}

#[test]
fn exposes_authored_and_effective_build_no_cache_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    build:\n      no_cache: \"${BUILD_NO_CACHE:-false}\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(845), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .ok_or("authored build expected")?;
    let Build::Definition(authored) = authored else {
        return Err("authored build definition expected".into());
    };
    assert!(matches!(
        authored.no_cache().map(compose_lens::model::Located::value),
        Some(BuildNoCache::String(value)) if value == "${BUILD_NO_CACHE:-false}"
    ));

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(846),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let definition = view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|build| match build.value() {
            ProjectBuild::Definition(definition) => Some(definition),
            _ => None,
        })
        .ok_or("effective build definition expected")?;
    assert!(matches!(
        definition.no_cache().map(compose_lens::project::ProjectValue::value),
        Some(BuildNoCache::String(value)) if value == "${BUILD_NO_CACHE:-false}"
    ));
    Ok(())
}

#[test]
fn exposes_authored_and_effective_build_sbom_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    build:\n      sbom: \"${SBOM_GENERATOR:-generator=default}\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(8451), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => definition.sbom(),
            Build::Context(_) => None,
        })
        .ok_or("authored build sbom expected")?;
    assert!(matches!(authored.value(), BuildSbom::String(value) if value == "${SBOM_GENERATOR:-generator=default}"));

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(8452),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let sbom = view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|build| match build.value() {
            ProjectBuild::Definition(definition) => definition.sbom(),
            _ => None,
        })
        .ok_or("effective build sbom expected")?;
    assert!(matches!(sbom.value(), BuildSbom::String(value) if value == "${SBOM_GENERATOR:-generator=default}"));
    Ok(())
}

#[test]
fn exposes_authored_and_effective_build_shm_size_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    build:\n      shm_size: \"64mb\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(8463), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => definition.shm_size(),
            Build::Context(_) => None,
        })
        .ok_or("authored build shm_size expected")?;
    assert_eq!(authored.raw().value(), "64mb");
    assert_eq!(authored.scalar_kind(), ShmSizeScalarKind::String);
    assert!(matches!(
        authored.kind(),
        ShmSizeKind::Documented { amount_raw, unit: ShmSizeUnit::Mb } if amount_raw == "64"
    ));

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(8464),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let effective = view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|build| match build.value() {
            ProjectBuild::Definition(definition) => definition.shm_size(),
            _ => None,
        })
        .ok_or("effective build shm_size expected")?;
    assert_eq!(effective.value().raw().value(), "64mb");
    assert_eq!(effective.value().scalar_kind(), ShmSizeScalarKind::String);
    assert!(matches!(
        effective.value().kind(),
        ShmSizeKind::Documented { amount_raw, unit: ShmSizeUnit::Mb } if amount_raw == "64"
    ));
    Ok(())
}

#[test]
fn exposes_authored_and_effective_build_labels_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source =
        "services:\n  app:\n    build:\n      labels: [io.example.value=one, io.example.bare, io.example.value=one]\n";
    let syntax = SyntaxDocument::parse(SourceId::new(710), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .ok_or("authored build expected")?;
    let Build::Definition(authored) = authored else {
        return Err("authored build definition expected".into());
    };
    assert!(
        matches!(authored.labels(), Some(compose_lens::model::Labels::List { values, .. })
        if values.iter().map(compose_lens::model::Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["io.example.value=one", "io.example.bare", "io.example.value=one"])
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(711),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project_view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let definition = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|build| match build.value() {
            ProjectBuild::Definition(definition) => Some(definition),
            _ => None,
        })
        .ok_or("effective build definition expected")?;
    let labels = definition.labels().ok_or("effective build labels expected")?;
    let _: &ProjectBuildLabels = labels.value();
    assert_eq!(
        labels
            .value()
            .as_list()
            .ok_or("list build labels expected")?
            .iter()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["io.example.value=one", "io.example.bare", "io.example.value=one"]
    );
    Ok(())
}

#[test]
fn exposes_authored_and_effective_build_args_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n  app:\n    build:\n      args:\n",
        "        string: value\n        number: 1\n        boolean: true\n        empty: null\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(713), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => definition.args(),
            Build::Context(_) => None,
        })
        .ok_or("authored build args expected")?;
    let _: &BuildArgs = authored;

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(714),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project_view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let definition = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|build| match build.value() {
            ProjectBuild::Definition(definition) => Some(definition),
            _ => None,
        })
        .ok_or("effective build definition expected")?;
    let args = definition.args().ok_or("effective build args expected")?;
    let _: &ProjectBuildArgs = args.value();
    assert!(matches!(args.value(), ProjectBuildArgs::Map(entries)
        if entries.len() == 4 && entries.iter().any(|entry| entry.name().value() == "empty")));
    Ok(())
}

#[test]
fn exposes_authored_and_effective_build_additional_contexts_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n  app:\n    build:\n      additional_contexts:\n",
        "        assets: ./assets\n        image: example/context:latest\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(915), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => definition.additional_contexts(),
            Build::Context(_) => None,
        })
        .ok_or("authored additional contexts expected")?;
    let _: &BuildAdditionalContexts = authored;

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(916),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project_view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let definition = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|build| match build.value() {
            ProjectBuild::Definition(definition) => Some(definition),
            _ => None,
        })
        .ok_or("effective build definition expected")?;
    let contexts = definition
        .additional_contexts()
        .ok_or("effective additional contexts expected")?;
    let _: &ProjectBuildAdditionalContexts = contexts.value();
    assert!(matches!(contexts.value(), ProjectBuildAdditionalContexts::Map(entries)
        if entries.len() == 2 && entries.iter().any(|entry| entry.name().value() == "assets")));
    Ok(())
}

#[test]
fn exposes_stdin_open_tty_and_privileged_at_authored_effective_and_generated_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    stdin_open: true\n    tty: false\n    privileged: true\n";
    let syntax = SyntaxDocument::parse(SourceId::new(707), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert_eq!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::stdin_open)
            .map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(true))
    );
    assert_eq!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::tty)
            .map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::privileged)
            .map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(true))
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(708),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    assert_eq!(
        build_project_view(merged.project().ok_or("merged project expected")?, None)
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::stdin_open)
            .map(compose_lens::project::ProjectValue::value),
        Some(&BooleanValue::Literal(true))
    );
    assert_eq!(
        build_project_view(merged.project().ok_or("merged project expected")?, None)
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::tty)
            .map(compose_lens::project::ProjectValue::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(
        build_project_view(merged.project().ok_or("merged project expected")?, None)
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::privileged)
            .map(compose_lens::project::ProjectValue::value),
        Some(&BooleanValue::Literal(true))
    );

    let mut service = GeneratedService::new("generated")?;
    service.set_stdin_open(false)?;
    service.set_tty(true)?;
    service.set_privileged(false)?;
    Ok(())
}

#[test]
fn exposes_attach_at_authored_and_effective_boundaries_without_a_generated_setter()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    attach: \"${ATTACH}\"\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3219), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert_eq!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::attach)
            .map(compose_lens::model::Located::value),
        Some(&BooleanValue::Expression("${ATTACH}".to_owned()))
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3220),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("ATTACH", "true");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project_view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let attach = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::attach)
        .ok_or("effective attach expected")?;
    assert_eq!(attach.value(), &BooleanValue::Literal(true));
    assert!(attach.is_sensitive());
    let debug = format!("{attach:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("Literal(true)"));
    Ok(())
}

#[test]
fn exposes_authored_and_effective_blkio_config_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    blkio_config:\n      weight: 500\n      device_read_bps: [{path: /dev/a, rate: 1mb}]\n      device_read_iops: [{path: /dev/b, rate: 2}]\n      device_write_bps: [{path: /dev/c, rate: 3}]\n      device_write_iops: [{path: /dev/d, rate: 4}]\n      weight_device: [{path: /dev/e, weight: 600}]\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3222), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::blkio_config)
        .ok_or("authored blkio")?;
    let _: &compose_lens::model::BlkioConfig = authored;
    let _: compose_lens::model::BlkioDeviceRateForm = authored.device_read_bps()[0].form();
    let _: compose_lens::model::BlkioWeightDeviceForm = authored.weight_device()[0].form();
    assert_eq!(authored.device_write_iops().len(), 1);
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(3223),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project_view = build_project_view(merged.project().ok_or("project")?, None);
    let effective = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::blkio_config)
        .ok_or("effective blkio")?;
    let _: &compose_lens::project::ProjectBlkioConfig = effective.value();
    let _: compose_lens::project::ProjectBlkioDeviceRateForm =
        effective.value().device_read_bps().ok_or("effective rate")?.value()[0]
            .value()
            .form();
    let _: compose_lens::project::ProjectBlkioWeightDeviceForm = effective
        .value()
        .weight_device()
        .ok_or("effective weight device")?
        .value()[0]
        .value()
        .form();
    assert_eq!(
        effective.value().weight_device().map(|items| items.value().len()),
        Some(1)
    );
    Ok(())
}

#[test]
fn exposes_generated_volume_definition_driver_options_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let mut volume = GeneratedVolumeDefinition::application("data")?;
    volume.set_driver(GeneratedString::plain("opaque-driver")?)?;
    volume.set_driver_opts(vec![
        GeneratedVolumeDriverOption::new(
            "quoted",
            GeneratedVolumeDriverOptionValue::String(GeneratedString::plain("2")?),
        )?,
        GeneratedVolumeDriverOption::new(
            "number",
            GeneratedVolumeDriverOptionValue::Number(GeneratedString::plain("2")?),
        )?,
    ])?;
    volume.set_labels(vec![GeneratedLabel::new(
        "com.example.owner",
        GeneratedString::plain("strukturpiloten")?,
    )?])?;
    assert_eq!(volume.driver().map(GeneratedString::expose), Some("opaque-driver"));
    assert_eq!(volume.driver_opts().map(<[GeneratedVolumeDriverOption]>::len), Some(2));
    assert_eq!(volume.labels().map(<[GeneratedLabel]>::len), Some(1));

    let mut builder = ComposeDocumentBuilder::new();
    let mut service = GeneratedService::new("app")?;
    service.set_image(GeneratedString::plain("example/app")?)?;
    builder.add_service(service)?;
    builder.add_volume_definition(volume)?;
    let generated = builder.build(SourceId::new(820))?;
    let volume = generated
        .document()
        .volumes()
        .first()
        .ok_or("generated volume expected")?;
    assert!(matches!(
        volume.driver_opts()[0].value().value(),
        compose_lens::model::ComposeScalar::String(value) if value == "2"
    ));
    assert!(matches!(
        volume.driver_opts()[1].value().value(),
        compose_lens::model::ComposeScalar::Number(value) if value == "2"
    ));
    assert!(matches!(
        volume.labels(),
        Some(compose_lens::model::Labels::Map { entries, .. })
            if entries[0].key().value() == "com.example.owner"
                && matches!(entries[0].value().value(), compose_lens::model::ComposeScalar::String(value) if value == "strukturpiloten")
    ));
    Ok(())
}

#[test]
fn exposes_authored_effective_and_generated_logging_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    logging:\n",
        "      driver: custom\n",
        "      options:\n",
        "        text: \"01\"\n",
        "        count: 2\n",
        "        empty: null\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(816), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored: &compose_lens::model::Logging = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::logging)
        .ok_or("authored logging expected")?;
    let authored_options: &compose_lens::model::LoggingOptions =
        authored.options().ok_or("authored options expected")?;
    let _: &compose_lens::model::LoggingOption = &authored_options.entries()[0];
    assert!(
        matches!(authored_options.entries()[0].value().value(), compose_lens::model::LoggingOptionValue::String(value) if value == "01")
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(817),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let logging: &ProjectLogging = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::logging)
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective logging expected")?;
    let options: &ProjectLoggingOptions = logging
        .options()
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective options expected")?;
    let option: &ProjectLoggingOption = options.entries()[0].value();
    assert!(matches!(option.value().value(), ProjectLoggingOptionValue::String { value, .. } if value == "01"));

    let generated_option = GeneratedLoggingOption::new("empty", GeneratedLoggingOptionValue::Null)?;
    let generated = GeneratedLogging::new(GeneratedString::plain("custom")?, vec![generated_option])?;
    let mut service = GeneratedService::new("app")?;
    service.set_logging(generated)?;
    assert_eq!(
        service.logging().map(GeneratedLogging::options).map(<[_]>::len),
        Some(1)
    );
    Ok(())
}

#[test]
fn exposes_generated_network_definition_driver_options_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let mut network = GeneratedNetworkDefinition::application("frontend")?;
    network.set_driver(GeneratedString::plain("opaque-driver")?)?;
    network.set_driver_opts(vec![
        GeneratedNetworkDriverOption::new(
            "quoted",
            GeneratedNetworkDriverOptionValue::String(GeneratedString::plain("2")?),
        )?,
        GeneratedNetworkDriverOption::new(
            "number",
            GeneratedNetworkDriverOptionValue::Number(GeneratedString::plain("2")?),
        )?,
    ])?;
    network.set_enable_ipv6(false)?;
    network.set_internal(true)?;
    network.set_labels(vec![GeneratedLabel::new(
        "com.example.owner",
        GeneratedString::plain("strukturpiloten")?,
    )?])?;
    assert_eq!(network.driver().map(GeneratedString::expose), Some("opaque-driver"));
    assert_eq!(
        network.driver_opts().map(<[GeneratedNetworkDriverOption]>::len),
        Some(2)
    );
    assert_eq!(network.enable_ipv6(), Some(false));
    assert_eq!(network.internal(), Some(true));
    assert_eq!(network.labels().map(<[GeneratedLabel]>::len), Some(1));

    let mut service = GeneratedService::new("app")?;
    service.add_network(GeneratedNetworkAttachment::new("frontend")?)?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(service)?;
    builder.add_network_definition(network)?;
    let generated = builder.build(SourceId::new(819))?;
    let network = generated
        .document()
        .networks()
        .first()
        .ok_or("generated network expected")?;
    assert!(matches!(
        network.driver_opts()[0].value().value(),
        compose_lens::model::ComposeScalar::String(value) if value == "2"
    ));
    assert!(matches!(
        network.driver_opts()[1].value().value(),
        compose_lens::model::ComposeScalar::Number(value) if value == "2"
    ));
    assert!(matches!(
        network.labels(),
        Some(compose_lens::model::Labels::Map { entries, .. })
            if entries[0].key().value() == "com.example.owner"
                && matches!(entries[0].value().value(), compose_lens::model::ComposeScalar::String(value) if value == "strukturpiloten")
    ));
    assert_eq!(
        network.enable_ipv6().map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(
        network.internal().map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(true))
    );
    Ok(())
}

#[test]
fn exposes_repeatable_unmask_security_option_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n    security_opt: [\"unmask=ALL\", \"${UNMASK}\", \"unmask=all\"]\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(736), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("authored unmask security options expected")?;
    assert!(matches!(
        authored.items()[0].kind(),
        compose_lens::model::SecurityOptionKind::Unmask { paths } if paths == "ALL"
    ));
    assert!(matches!(
        authored.items()[1].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert_eq!(
        parsed.diagnostics()[0].code(),
        compose_lens::model::SECURITY_OPT_UNMASK_NEAR_MISS
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(737),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("UNMASK", "unmask=/proc/acpi:/sys/firmware");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let effective: &compose_lens::project::ProjectSecurityOptionItem = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(1))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective unmask option expected")?;
    assert!(matches!(
        effective.kind(),
        compose_lens::model::SecurityOptionKind::Unmask { paths }
            if paths == "/proc/acpi:/sys/firmware"
    ));

    let mut service = GeneratedService::new("app")?;
    service.set_security_options(vec![
        GeneratedString::plain("unmask=ALL")?,
        GeneratedString::plain("unmask=ALL")?,
        GeneratedString::plain("unmask=/proc/*")?,
    ])?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(service)?;
    let generated = builder.build(SourceId::new(738))?;
    let generated_options = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("generated unmask options expected")?;
    assert_eq!(generated_options.items().len(), 3);
    assert!(matches!(
        generated_options.items()[2].kind(),
        compose_lens::model::SecurityOptionKind::Unmask { paths } if paths == "/proc/*"
    ));
    Ok(())
}
use compose_lens::resolution::validate_references;
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;
use compose_lens::validation::{
    CompatibilityFeature, CompatibilityProfile, ImplementationVersion, validate_compatibility,
};

fn assert_generated_seccomp_public_contract(options: &compose_lens::model::SecurityOptions) {
    assert_eq!(options.items().len(), 21);
    for index in [1, 2] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::SecurityLabelDisable { enabled: true }
        ));
    }
    for index in [3, 4] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::SecurityLabelFileType { file_type }
                if file_type == "container_file_t"
        ));
    }
    for index in [5, 6] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::SecurityLabelLevel { level }
                if level == "s0:c1,c2"
        ));
    }
    for index in [7, 8] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::SecurityLabelNested { enabled: true }
        ));
    }
    for index in [9, 10] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::SecurityLabelType { label_type }
                if label_type == "container_t"
        ));
    }
    for index in [11, 12] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::Mask { paths }
                if paths == "/proc/acpi:/proc/kcore"
        ));
    }
    for (index, profile) in [(14, "unconfined"), (15, "/workspace/seccomp.json"), (16, "unconfined")] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::Seccomp { profile: actual } if actual == profile
        ));
    }
}

fn assert_generated_security_option_contract() -> Result<(), Box<dyn std::error::Error>> {
    let mut service = GeneratedService::new("app")?;
    service.set_security_options(vec![
        GeneratedString::plain("apparmor=public-api")?,
        GeneratedString::plain("label:disable")?,
        GeneratedString::plain("label:disable")?,
        GeneratedString::plain("label:filetype:container_file_t")?,
        GeneratedString::plain("label:filetype:container_file_t")?,
        GeneratedString::plain("label:level:s0:c1,c2")?,
        GeneratedString::plain("label:level:s0:c1,c2")?,
        GeneratedString::plain("label:nested")?,
        GeneratedString::plain("label:nested")?,
        GeneratedString::plain("label:type:container_t")?,
        GeneratedString::plain("label:type:container_t")?,
        GeneratedString::plain("mask=/proc/acpi:/proc/kcore")?,
        GeneratedString::plain("mask=/proc/acpi:/proc/kcore")?,
        GeneratedString::plain("no-new-privileges:true")?,
        GeneratedString::plain("seccomp=unconfined")?,
        GeneratedString::plain("seccomp=/workspace/seccomp.json")?,
        GeneratedString::plain("seccomp=unconfined")?,
        GeneratedString::plain("no-new-privileges:false")?,
        GeneratedString::plain("no-new-privileges:true")?,
        GeneratedString::plain("label:user:USER")?,
        GeneratedString::plain("label=disable")?,
    ])?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(service)?;
    let generated = builder.build(SourceId::new(704))?;
    let generated_options = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("generated security options expected")?;
    assert_generated_seccomp_public_contract(generated_options);
    Ok(())
}

#[test]
fn exposes_authored_effective_and_generated_security_option_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n    security_opt: [\"apparmor=${PROFILE}\", \"label:disable\", \"label:disable\", \"${NNP}\", \"${SECCOMP}\", \"${LABEL_DISABLE}\", \"${LABEL_FILETYPE}\", \"${LABEL_LEVEL}\", \"${LABEL_NESTED}\", \"${LABEL_TYPE}\", \"${MASK}\"]\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(702), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored: &compose_lens::model::SecurityOptions = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("authored security options expected")?;
    assert_authored_security_option_contract(authored);

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(703),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("PROFILE", "public-api");
    let _ = environment.insert("NNP", "no-new-privileges:false");
    let _ = environment.insert("SECCOMP", "seccomp=/workspace/seccomp.json");
    let _ = environment.insert("LABEL_DISABLE", "label:disable");
    let _ = environment.insert("LABEL_FILETYPE", "label:filetype:container_file_t");
    let _ = environment.insert("LABEL_LEVEL", "label:level:s0:c1,c2");
    let _ = environment.insert("LABEL_NESTED", "label:nested");
    let _ = environment.insert("LABEL_TYPE", "label:type:container_t");
    let _ = environment.insert("MASK", "mask=/proc/acpi:/proc/kcore");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let effective = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let item: &compose_lens::project::ProjectSecurityOptionItem = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().first())
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective security option expected")?;
    assert_eq!(item.authored(), "\"apparmor=${PROFILE}\"");
    assert_eq!(item.value(), "apparmor=public-api");
    let no_new_privileges = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(3))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective no-new-privileges option expected")?;
    assert!(matches!(
        no_new_privileges.kind(),
        compose_lens::model::SecurityOptionKind::NoNewPrivileges { enabled: false }
    ));
    let seccomp = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(4))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective seccomp option expected")?;
    assert!(matches!(
        seccomp.kind(),
        compose_lens::model::SecurityOptionKind::Seccomp { profile } if profile == "/workspace/seccomp.json"
    ));
    let label_disable = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(5))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective SELinux label-disable option expected")?;
    assert!(matches!(
        label_disable.kind(),
        compose_lens::model::SecurityOptionKind::SecurityLabelDisable { enabled: true }
    ));
    assert_effective_label_filetype_contract(&effective)?;
    assert_effective_label_level_contract(&effective)?;
    assert_effective_label_nested_contract(&effective)?;
    assert_effective_label_type_contract(&effective)?;
    assert_effective_mask_contract(&effective)?;

    assert_generated_security_option_contract()
}

fn assert_authored_security_option_contract(authored: &compose_lens::model::SecurityOptions) {
    assert!(matches!(
        authored.items()[0].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[3].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[4].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[1].kind(),
        compose_lens::model::SecurityOptionKind::SecurityLabelDisable { enabled: true }
    ));
    assert!(matches!(
        authored.items()[5].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[6].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[7].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[8].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[9].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[10].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
}

fn assert_effective_mask_contract(
    effective: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let mask = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(10))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective mask option expected")?;
    assert!(matches!(
        mask.kind(),
        compose_lens::model::SecurityOptionKind::Mask { paths }
            if paths == "/proc/acpi:/proc/kcore"
    ));
    Ok(())
}

fn assert_effective_label_type_contract(
    effective: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let label_type = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(9))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective SELinux label-type option expected")?;
    assert!(matches!(
        label_type.kind(),
        compose_lens::model::SecurityOptionKind::SecurityLabelType { label_type }
            if label_type == "container_t"
    ));
    Ok(())
}

fn assert_effective_label_nested_contract(
    effective: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let label_nested = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(8))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective SELinux label-nested option expected")?;
    assert!(matches!(
        label_nested.kind(),
        compose_lens::model::SecurityOptionKind::SecurityLabelNested { enabled: true }
    ));
    Ok(())
}

fn assert_effective_label_level_contract(
    effective: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let label_level = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(7))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective SELinux label-level option expected")?;
    assert!(matches!(
        label_level.kind(),
        compose_lens::model::SecurityOptionKind::SecurityLabelLevel { level }
            if level == "s0:c1,c2"
    ));
    Ok(())
}

fn assert_effective_label_filetype_contract(
    effective: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let label_filetype = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(6))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective SELinux label-filetype option expected")?;
    assert!(matches!(
        label_filetype.kind(),
        compose_lens::model::SecurityOptionKind::SecurityLabelFileType { file_type }
            if file_type == "container_file_t"
    ));
    Ok(())
}

#[test]
fn exposes_authored_effective_and_generated_expose_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    expose: [80, \"80\", \"80/tcp\"]\n";
    let syntax = SyntaxDocument::parse(SourceId::new(694), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::expose)
        .ok_or("authored expose expected")?;
    assert_eq!(
        authored.items()[0].scalar_kind(),
        compose_lens::model::ExposeScalarKind::Number
    );
    assert_eq!(
        authored.items()[1].scalar_kind(),
        compose_lens::model::ExposeScalarKind::String
    );
    assert!(matches!(
        authored.items()[2].kind(),
        compose_lens::model::ExposeItemKind::Documented { .. }
    ));

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(695),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let effective = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::expose)
        .ok_or("effective expose expected")?;
    assert_eq!(effective.value().len(), 3);
    let _: &compose_lens::project::ProjectExposeItem = effective.value()[0].value();

    let mut service = GeneratedService::new("app")?;
    service.set_expose(vec![GeneratedString::plain("80")?, GeneratedString::plain("80/tcp")?])?;
    assert_eq!(service.expose().map(<[GeneratedString]>::len), Some(2));
    Ok(())
}

#[test]
fn exposes_dns_search_authored_project_and_generated_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    dns_search: [example.internal, example.internal, .]\n";
    let syntax = SyntaxDocument::parse(SourceId::new(692), source)?;
    let document = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        document
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::dns_search)
            .map(compose_lens::model::DnsSearch::form),
        Some(compose_lens::model::DnsSearchForm::List(items)) if items.len() == 3
    ));

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(693),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    assert!(matches!(
        view.view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::dns_search)
            .map(compose_lens::project::ProjectValue::value),
        Some(ProjectDnsSearch::List(items)) if items.len() == 3
    ));

    let mut service = GeneratedService::new("app")?;
    service.set_dns_search(GeneratedDnsSearch::Scalar(GeneratedString::plain(".")?))?;
    assert!(matches!(service.dns_search(), Some(GeneratedDnsSearch::Scalar(value)) if value.expose() == "."));
    Ok(())
}

const PUBLIC_SOURCE: &str = concat!(
    "services:\n",
    "  app:\n",
    "    hostname: API.Example.COM\n    image: example.invalid/app:old\n",
    "    entrypoint: [/usr/bin/env, php]\n",
    "    init: true\n",
    "    user: 1000:1001\n",
    "    userns_mode: keep-id\n",
    "    group_add: [audio, '44']\n",
    "    cap_add: [SYS_TIME, sys_time]\n",
    "    cap_drop: [NET_ADMIN, net_admin]\n",
    "    devices: [/dev/dri:/dev/dri:rwm, {source: /dev/video0, target: /dev/camera, permissions: rw}]\n",
    "    dns: [1.1.1.1, 1.1.1.1, resolver.internal]\n",
    "    dns_opt: [ndots:5, timeout:2]\n",
    "    working_dir: /srv/app\n",
    "    read_only: true\n    pids_limit: 00064\n    shm_size: 64mb\n    mem_limit: 128b\n",
    "    tmpfs: [/run, /cache:mode=0700]\n",
    "    sysctls: {net.ipv4.ip_forward: \"1\", kernel.shm_rmid_forced: 0001}\n",
    "    ulimits:\n",
    "      nofile:\n",
    "        soft: \"1024\"\n",
    "        hard: -1\n",
    "      nproc: 2048\n",
    "    pull_policy: every_12h\n",
    "    pull_refresh_after: 6h\n",
    "    stop_signal: SIGUSR1\n",
    "    stop_grace_period: 1m30s\n",
    "    labels:\n",
    "      com.example.owner: strukturpiloten\n",
    "    extra_hosts:\n",
    "      - host.docker.internal=host-gateway\n",
    "    env_file: [{ path: ./app.env, required: false, format: raw }]\n",
    "    healthcheck:\n",
    "      test: [CMD, /usr/bin/true]\n",
    "      interval: 30s\n",
    "    depends_on:\n",
    "      database:\n",
    "        condition: service_healthy\n",
    "    volumes:\n",
    "      - ./data:/data:z\n",
    "    configs: [app-config]\n",
    "    secrets:\n",
    "      - source: app-secret\n",
    "        target: password\n",
    "        mode: \"0440\"\n",
    "  database:\n",
    "    image: example.invalid/database:1\n",
    "configs:\n",
    "  app-config:\n",
    "    file: ./app.conf\n",
    "secrets:\n",
    "  app-secret:\n",
    "    external: true\n",
);

#[test]
fn supported_public_pipeline_compiles_and_preserves_explicit_stages() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(501), PUBLIC_SOURCE)?;
    let typed = ComposeDocument::parse(syntax.document());
    let image_span = typed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::image)
        .map(compose_lens::model::Located::span)
        .ok_or("typed image span expected")?;
    let edit = ScalarEdit::new(
        image_span,
        ReplacementScalar::string("example.invalid/app:${TAG}@sha256:abcdef"),
    );
    let edited = apply_preservation_edits(syntax.document(), &[edit]);
    assert!(edited.is_valid(), "{:#?}", edited.diagnostics());

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(502),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        edited.output(),
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("TAG", "1.2.3");
    let interpolation = loaded.interpolate(&environment);
    let merge = merge_project(&loaded, Some(&interpolation));
    let project = merge.project().ok_or("merged public API project expected")?;
    let selection = select_profiles(project, &ProfileRequest::new());
    let project_view = build_project_view(project, Some(&selection));
    let references = validate_references(project, Some(&selection));
    let compatibility = validate_compatibility(
        project,
        Some(&selection),
        CompatibilityProfile::docker_compose(ImplementationVersion::new(5, 3, 1)),
    );
    let rendered = render_canonical(project, Some(&selection));

    assert!(loaded.is_valid(), "{:#?}", loaded.diagnostics());
    assert!(interpolation.is_valid(), "{:#?}", interpolation.diagnostics());
    assert!(merge.is_valid(), "{:#?}", merge.diagnostics());
    assert!(selection.is_valid(), "{:#?}", selection.diagnostics());
    assert!(project_view.is_valid(), "{:#?}", project_view.diagnostics());
    assert_image_and_entrypoint(&project_view)?;
    assert_host_and_health(&project_view)?;
    assert_environment_file(&project_view)?;
    assert_dependency(&project_view)?;
    assert_execution_identity(&project_view)?;
    assert_pull_policy(&project_view)?;
    assert_lifecycle(&project_view)?;
    assert_sysctls(&project_view)?;
    assert_ulimits(&project_view)?;
    assert_resource_grants(&project_view)?;
    assert_labels(&project_view);
    assert!(references.is_valid(), "{:#?}", references.diagnostics());
    assert!(compatibility.is_valid(), "{:#?}", compatibility.diagnostics());
    assert!(
        compatibility
            .findings()
            .iter()
            .any(|finding| { finding.occurrence().feature() == CompatibilityFeature::ImageTagAndDigest })
    );
    assert!(rendered.is_valid(), "{:#?}", rendered.diagnostics());
    assert!(rendered.output().contains("example.invalid/app:1.2.3@sha256:abcdef"));
    Ok(())
}

fn assert_image_and_entrypoint(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let service = project_view
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("native project service expected")?;
    assert_eq!(
        service.image().map(|image| image.value().raw()),
        Some("example.invalid/app:1.2.3@sha256:abcdef")
    );
    let entrypoint = service.entrypoint().ok_or("native project entrypoint expected")?;
    assert!(matches!(entrypoint.value(), Entrypoint::List { values, .. } if values.len() == 2));
    assert_eq!(
        service.init().map(compose_lens::project::ProjectValue::value),
        Some(&BooleanValue::Literal(true))
    );
    let hostname = service.hostname().ok_or("native project hostname expected")?;
    assert_eq!(hostname.value().raw().value(), "API.Example.COM");
    assert_eq!(hostname.value().kind(), &HostnameKind::Resolved);
    Ok(())
}

fn assert_environment_file(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let environment_file = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::environment_files)
        .and_then(|files| files.value().first())
        .ok_or("native project environment file expected")?;
    let ProjectEnvironmentFile::Long(environment_file) = environment_file.value() else {
        return Err("long environment-file syntax expected");
    };
    assert_eq!(
        environment_file
            .path()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str),
        Some("./app.env")
    );
    assert_eq!(
        environment_file
            .required()
            .map(compose_lens::project::ProjectValue::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(
        environment_file
            .format()
            .map(compose_lens::project::ProjectValue::value)
            .map(compose_lens::model::EnvironmentFileFormat::kind),
        Some(EnvironmentFileFormatKind::Raw)
    );
    Ok(())
}

fn assert_labels(project_view: &compose_lens::project::ProjectViewResult) {
    assert_eq!(
        project_view
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::labels)
            .and_then(|labels| labels.value().get("com.example.owner"))
            .and_then(|label| match label.value().value() {
                compose_lens::model::ComposeScalar::String(value) => Some(value.as_str()),
                _ => None,
            }),
        Some("strukturpiloten")
    );
}

#[test]
fn supported_generated_document_boundary_is_parse_back_validated() -> Result<(), Box<dyn std::error::Error>> {
    let mut service = GeneratedService::new("app")?;
    service.set_hostname(GeneratedHostname::Resolved(GeneratedString::plain("API.Example.COM")?))?;
    service.set_container_name(GeneratedString::plain("example-app")?)?;
    service.set_image(GeneratedString::plain("example.invalid/app:1")?)?;
    service.set_entrypoint(GeneratedEntrypoint::List(vec![GeneratedString::plain("/usr/bin/env")?]))?;
    service.set_init(true)?;
    service.add_environment_file(GeneratedEnvironmentFile::short(GeneratedString::plain("./app.env")?)?);
    set_generated_limits(&mut service)?;
    service.set_restart(GeneratedRestartPolicy::UnlessStopped)?;
    service.set_stop_signal(GeneratedString::plain("15")?)?;
    service.set_stop_grace_period(GeneratedString::plain("0s")?)?;
    service.add_label(GeneratedLabel::new(
        "com.example.owner",
        GeneratedString::plain("strukturpiloten")?,
    )?)?;
    service.set_cap_add(vec![
        GeneratedString::plain("SYS_TIME")?,
        GeneratedString::plain("sys_time")?,
    ])?;
    service.set_cap_drop(vec![
        GeneratedString::plain("NET_ADMIN")?,
        GeneratedString::plain("net_admin")?,
    ])?;
    service.set_devices(vec![
        GeneratedDevice::Short(GeneratedString::plain("vendor.example/device=gpu")?),
        GeneratedDevice::Long(GeneratedLongDevice::new(
            GeneratedString::plain("/dev/video0")?,
            Some(GeneratedString::plain("/dev/camera")?),
            Some(GeneratedString::plain("rw")?),
        )?),
    ])?;
    set_generated_dns(&mut service)?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.set_name("example")?;
    builder.add_service(service)?;

    let generated = builder.build(SourceId::new(503))?;
    assert_generated_hostname(&generated)?;
    assert_generated_capabilities(&generated)?;
    assert_generated_devices(&generated)?;
    assert_generated_dns(&generated)?;
    assert_generated_dns_options(&generated)?;
    assert_generated_tmpfs(&generated)?;
    assert_generated_kernel_controls(&generated)?;
    assert_eq!(
        generated
            .document()
            .service("app")
            .and_then(compose_lens::model::Service::image)
            .map(|image| image.value().raw()),
        Some("example.invalid/app:1")
    );
    assert_eq!(
        generated
            .document()
            .service("app")
            .and_then(compose_lens::model::Service::container_name)
            .map(|name| name.value().as_str()),
        Some("example-app")
    );
    assert!(matches!(
        generated
            .document()
            .service("app")
            .and_then(compose_lens::model::Service::entrypoint),
        Some(Entrypoint::List { values, .. }) if values.len() == 1
    ));
    assert_eq!(
        generated
            .document()
            .service("app")
            .and_then(compose_lens::model::Service::init)
            .map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(true))
    );
    assert!(
        generated
            .document()
            .service("app")
            .and_then(compose_lens::model::Service::labels)
            .is_some()
    );
    assert_eq!(
        generated
            .document()
            .service("app")
            .map(compose_lens::model::Service::environment_files)
            .map(<[_]>::len),
        Some(1)
    );
    assert!(matches!(
        generated
            .document()
            .service("app")
            .and_then(compose_lens::model::Service::restart)
            .map(compose_lens::model::RestartPolicy::kind),
        Some(RestartPolicyKind::UnlessStopped)
    ));
    assert_generated_controls(&generated)?;
    assert_generated_document_text(&generated);
    Ok(())
}

#[test]
fn generated_network_attachment_addresses_are_additive_public_contract() -> Result<(), Box<dyn std::error::Error>> {
    let mut attachment = GeneratedNetworkAttachment::new("frontend")?;
    attachment.add_alias("app")?;
    attachment.set_ipv4_address(GeneratedString::plain("192.0.2.40")?)?;
    attachment.set_ipv6_address(GeneratedString::plain("2001:db8::40")?)?;
    assert_eq!(
        attachment.ipv4_address().map(GeneratedString::expose),
        Some("192.0.2.40")
    );
    assert_eq!(
        attachment.ipv6_address().map(GeneratedString::expose),
        Some("2001:db8::40")
    );

    let mut service = GeneratedService::new("app")?;
    service.add_network(attachment)?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(service)?;
    let generated = builder.build(SourceId::new(818))?;
    assert!(matches!(
        generated.document().service("app").and_then(compose_lens::model::Service::networks),
        Some(compose_lens::model::ServiceNetworks::Long { networks, .. })
            if networks.first().is_some_and(|network| {
                network.ipv4_address().is_some_and(|value| value.value() == "192.0.2.40")
                    && network.ipv6_address().is_some_and(|value| value.value() == "2001:db8::40")
            })
    ));
    Ok(())
}

fn assert_generated_kernel_controls(
    generated: &compose_lens::render::GeneratedComposeDocument,
) -> Result<(), &'static str> {
    let service = generated
        .document()
        .service("app")
        .ok_or("generated service expected")?;
    assert!(service.sysctls().is_some());
    assert!(service.ulimits().is_some());
    Ok(())
}

fn assert_generated_capabilities(
    generated: &compose_lens::render::GeneratedComposeDocument,
) -> Result<(), &'static str> {
    let service = generated
        .document()
        .service("app")
        .ok_or("generated service expected")?;
    assert_eq!(
        service.cap_add().map(|values| values
            .items()
            .iter()
            .map(compose_lens::model::CapabilityAddItem::value)
            .collect()),
        Some(vec!["SYS_TIME", "sys_time"])
    );
    assert_eq!(
        service.cap_drop().map(|values| values
            .items()
            .iter()
            .map(compose_lens::model::CapabilityDropItem::value)
            .collect()),
        Some(vec!["NET_ADMIN", "net_admin"])
    );
    Ok(())
}

fn assert_generated_document_text(generated: &compose_lens::render::GeneratedComposeDocument) {
    assert_eq!(
        generated.text(),
        concat!(
            "---\n",
            "name: example\n",
            "services:\n",
            "  app:\n",
            "    hostname: API.Example.COM\n",
            "    container_name: example-app\n",
            "    image: example.invalid/app:1\n",
            "    entrypoint:\n",
            "      - /usr/bin/env\n",
            "    init: true\n",
            "    env_file:\n",
            "      - ./app.env\n",
            "    labels:\n",
            "      com.example.owner: strukturpiloten\n",
            "    cap_add:\n",
            "      - SYS_TIME\n",
            "      - sys_time\n",
            "    cap_drop:\n",
            "      - NET_ADMIN\n",
            "      - net_admin\n",
            "    pids_limit: 64\n",
            "    shm_size: 64mb\n",
            "    mem_limit: 128b\n",
            "    devices:\n",
            "      - vendor.example/device=gpu\n",
            "      - source: /dev/video0\n",
            "        target: /dev/camera\n",
            "        permissions: rw\n",
            "    dns:\n",
            "      - 1.1.1.1\n",
            "      - 1.1.1.1\n",
            "      - resolver.internal\n",
            "    dns_opt:\n",
            "      - ndots:5\n",
            "      - timeout:2\n",
            "    tmpfs:\n",
            "      - /run\n",
            "      - /cache:mode=0700\n",
            "    sysctls:\n",
            "      net.ipv4.ip_forward: \"1\"\n",
            "    ulimits:\n",
            "      nofile:\n",
            "        soft: \"1024\"\n",
            "        hard: \"-1\"\n",
            "    pull_policy: every_12h\n",
            "    restart: unless-stopped\n",
            "    stop_signal: \"15\"\n",
            "    stop_grace_period: 0s\n",
        )
    );
}

fn set_generated_limits(service: &mut GeneratedService) -> Result<(), compose_lens::render::GenerationError> {
    service.set_pids_limit(GeneratedPidsLimit::Finite("64".to_owned()))?;
    service.set_shm_size(GeneratedShmSize::Explicit {
        amount: GeneratedString::plain("64")?,
        unit: ShmSizeUnit::Mb,
    })?;
    service.set_mem_limit(GeneratedMemLimit::Explicit {
        amount: GeneratedString::plain("128")?,
        unit: MemLimitUnit::B,
    })?;
    service.set_tmpfs(GeneratedTmpfs::List(vec![
        GeneratedString::plain("/run")?,
        GeneratedString::plain("/cache:mode=0700")?,
    ]))?;
    service.set_sysctls(GeneratedSysctls::Map(vec![GeneratedSysctl::new(
        "net.ipv4.ip_forward",
        GeneratedString::plain("1")?,
    )?]))?;
    service.set_ulimits(GeneratedUlimits::new(vec![GeneratedUlimit::new(
        "nofile",
        GeneratedUlimitValue::Range {
            soft: Some(GeneratedString::plain("1024")?),
            hard: Some(GeneratedString::plain("-1")?),
        },
    )?])?)?;
    service.set_pull_policy(GeneratedPullPolicy::Every(GeneratedString::plain("12h")?))
}

fn set_generated_dns(service: &mut GeneratedService) -> Result<(), compose_lens::render::GenerationError> {
    service.set_dns(GeneratedDns::List(vec![
        GeneratedString::plain("1.1.1.1")?,
        GeneratedString::plain("1.1.1.1")?,
        GeneratedString::plain("resolver.internal")?,
    ]))?;
    service.set_dns_options(vec![
        GeneratedString::plain("ndots:5")?,
        GeneratedString::plain("timeout:2")?,
    ])
}

fn assert_sysctls(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let sysctls = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::sysctls)
        .ok_or("native project sysctls expected")?;
    let ProjectSysctls::Map(entries) = sysctls.value() else {
        return Err("native project sysctls mapping expected");
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].value().name().value(), "net.ipv4.ip_forward");
    assert_eq!(
        entries[0].value().value().value(),
        &compose_lens::model::ComposeScalar::String("1".to_owned())
    );
    assert_eq!(
        entries[1].value().value().value(),
        &compose_lens::model::ComposeScalar::Number("0001".to_owned())
    );
    Ok(())
}

fn assert_ulimits(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let limits = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::ulimits)
        .ok_or("native project ulimits expected")?;
    let project_limits: &ProjectUlimits = limits.value();
    assert_eq!(project_limits.entries().len(), 2);
    let entry: &ProjectUlimit = project_limits.entries()[0].value();
    assert_eq!(entry.name().value(), "nofile");
    let ProjectUlimitValue::Range(range) = entry.value() else {
        return Err("native project ulimit range expected");
    };
    let range: &ProjectUlimitRange = range;
    let soft: &ProjectUlimitScalar = range.soft().ok_or("native soft ulimit expected")?.value();
    assert_eq!(soft.authored(), "\"1024\"");
    Ok(())
}

fn assert_generated_hostname(generated: &compose_lens::render::GeneratedComposeDocument) -> Result<(), &'static str> {
    let hostname = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::hostname)
        .ok_or("generated hostname expected")?;
    assert_eq!(hostname.raw().value(), "API.Example.COM");
    assert_eq!(hostname.kind(), &HostnameKind::Resolved);
    Ok(())
}

fn assert_generated_controls(generated: &compose_lens::render::GeneratedComposeDocument) -> Result<(), &'static str> {
    assert_generated_pull_policy(generated)?;
    assert_generated_pids_limit(generated)?;
    assert_generated_shm_size(generated)?;
    assert_generated_mem_limit(generated)?;
    assert_generated_lifecycle(generated)
}

fn assert_pids_limit(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let limit = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::pids_limit)
        .ok_or("native project PID limit expected")?;
    assert_eq!(limit.value().raw().value(), "00064");
    assert!(matches!(
        limit.value().kind(),
        PidsLimitKind::Finite { decimal } if decimal == "00064"
    ));
    Ok(())
}

fn assert_shm_size(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let size = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::shm_size)
        .ok_or("native project shared-memory size expected")?;
    assert_eq!(size.value().raw().value(), "64mb");
    assert_eq!(size.value().scalar_kind(), ShmSizeScalarKind::String);
    assert!(matches!(
        size.value().kind(),
        ShmSizeKind::Documented { amount_raw, unit: ShmSizeUnit::Mb } if amount_raw == "64"
    ));
    Ok(())
}

fn assert_mem_limit(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let limit = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::mem_limit)
        .ok_or("native project memory limit expected")?;
    assert_eq!(limit.value().raw().value(), "128b");
    assert_eq!(limit.value().scalar_kind(), MemLimitScalarKind::String);
    assert!(matches!(
        limit.value().kind(),
        MemLimitKind::Documented { amount_raw, unit: MemLimitUnit::B } if amount_raw == "128"
    ));
    Ok(())
}

fn assert_tmpfs(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let tmpfs = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::tmpfs)
        .ok_or("native project tmpfs expected")?;
    match tmpfs.value() {
        ProjectTmpfs::List(items) => assert_eq!(
            items.iter().map(|item| item.value().value()).collect::<Vec<_>>(),
            ["/run", "/cache:mode=0700"]
        ),
        _ => return Err("native project tmpfs list expected"),
    }
    Ok(())
}

fn assert_generated_tmpfs(generated: &compose_lens::render::GeneratedComposeDocument) -> Result<(), &'static str> {
    let tmpfs = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::tmpfs)
        .ok_or("generated tmpfs expected")?;
    match tmpfs.form() {
        compose_lens::model::TmpfsForm::List(items) => assert_eq!(
            items
                .iter()
                .map(compose_lens::model::TmpfsItem::value)
                .collect::<Vec<_>>(),
            ["/run", "/cache:mode=0700"]
        ),
        _ => return Err("generated tmpfs list expected"),
    }
    Ok(())
}

fn assert_pull_policy(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let service = project_view
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("native project service expected")?;
    assert!(matches!(
        service.pull_policy().map(compose_lens::project::ProjectValue::value).map(
            compose_lens::model::PullPolicy::kind
        ),
        Some(PullPolicyKind::Every { duration }) if duration == "12h"
    ));
    assert_eq!(
        service
            .pull_refresh_after()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str),
        Some("6h")
    );
    Ok(())
}

fn assert_generated_lifecycle(generated: &compose_lens::render::GeneratedComposeDocument) -> Result<(), &'static str> {
    let service = generated.document().service("app").ok_or("generated app expected")?;
    assert_eq!(
        service
            .stop_signal()
            .map(compose_lens::model::Located::value)
            .map(String::as_str),
        Some("15")
    );
    assert!(matches!(
        service
            .stop_grace_period()
            .map(compose_lens::model::Located::value),
        Some(StopGracePeriod::Value(value)) if value == "0s"
    ));
    Ok(())
}

fn assert_generated_pull_policy(
    generated: &compose_lens::render::GeneratedComposeDocument,
) -> Result<(), &'static str> {
    let policy = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::pull_policy)
        .ok_or("generated pull policy expected")?;
    assert!(matches!(policy.kind(), PullPolicyKind::Every { duration } if duration == "12h"));
    Ok(())
}

fn assert_generated_pids_limit(generated: &compose_lens::render::GeneratedComposeDocument) -> Result<(), &'static str> {
    let limit = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::pids_limit)
        .ok_or("generated PID limit expected")?;
    assert!(matches!(
        limit.kind(),
        PidsLimitKind::Finite { decimal } if decimal == "64"
    ));
    Ok(())
}

fn assert_generated_shm_size(generated: &compose_lens::render::GeneratedComposeDocument) -> Result<(), &'static str> {
    let size = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::shm_size)
        .ok_or("generated shared-memory size expected")?;
    assert_eq!(size.raw().value(), "64mb");
    assert_eq!(size.scalar_kind(), ShmSizeScalarKind::String);
    assert!(matches!(
        size.kind(),
        ShmSizeKind::Documented { amount_raw, unit: ShmSizeUnit::Mb } if amount_raw == "64"
    ));
    Ok(())
}

fn assert_generated_mem_limit(generated: &compose_lens::render::GeneratedComposeDocument) -> Result<(), &'static str> {
    let limit = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::mem_limit)
        .ok_or("generated memory limit expected")?;
    assert_eq!(limit.raw().value(), "128b");
    assert_eq!(limit.scalar_kind(), MemLimitScalarKind::String);
    assert!(matches!(
        limit.kind(),
        MemLimitKind::Documented { amount_raw, unit: MemLimitUnit::B } if amount_raw == "128"
    ));
    Ok(())
}

fn assert_resource_grants(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let service = project_view
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("native project service expected")?;
    assert!(matches!(
        service
            .configs()
            .and_then(|grants| grants.value().first())
            .map(compose_lens::project::ProjectValue::value),
        Some(ProjectGrant::Short(source)) if source == "app-config"
    ));
    let Some(ProjectGrant::Long(secret)) = service
        .secrets()
        .and_then(|grants| grants.value().first())
        .map(compose_lens::project::ProjectValue::value)
    else {
        return Err("long secret grant expected");
    };
    assert_eq!(
        secret
            .source()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str),
        Some("app-secret")
    );
    assert_eq!(
        secret
            .target()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str),
        Some("password")
    );
    assert_eq!(
        secret
            .mode()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str),
        Some("0440")
    );
    Ok(())
}

fn assert_host_and_health(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let service = project_view
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("native project service expected")?;
    let gateway = service
        .extra_hosts()
        .and_then(|hosts| hosts.value().entries().first())
        .ok_or("native project extra host expected")?;
    assert_eq!(gateway.hostname().value(), "host.docker.internal");
    assert_eq!(gateway.address().value().raw(), "host-gateway");
    assert_eq!(gateway.address().value().kind(), HostAddressKind::HostGateway);
    let healthcheck = service.healthcheck().ok_or("native project healthcheck expected")?;
    assert!(matches!(
        healthcheck
            .value()
            .interval()
            .map(compose_lens::project::ProjectValue::value),
        Some(HealthcheckDuration::Value(value)) if value == "30s"
    ));
    Ok(())
}

fn assert_execution_identity(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let service = project_view
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("native project service expected")?;
    let user = service.user().ok_or("native project user expected")?;
    assert!(matches!(user.value().user(), IdentityComponent::Numeric(value) if value == "1000"));
    assert!(matches!(user.value().group(), Some(IdentityComponent::Numeric(value)) if value == "1001"));
    assert_eq!(
        service.userns_mode().map(|value| value.value().kind()),
        Some(UserNamespaceModeKind::PodmanKeepId)
    );
    assert_eq!(
        service.group_add().map(|groups| groups
            .value()
            .iter()
            .map(|group| group.value().as_str())
            .collect::<Vec<_>>()),
        Some(vec!["audio", "44"])
    );
    assert_eq!(
        service.cap_add().map(|capabilities| capabilities
            .value()
            .iter()
            .map(|item| item.value().value())
            .collect::<Vec<_>>()),
        Some(vec!["SYS_TIME", "sys_time"])
    );
    assert_eq!(
        service.cap_drop().map(|capabilities| capabilities
            .value()
            .iter()
            .map(|item| item.value().value())
            .collect::<Vec<_>>()),
        Some(vec!["NET_ADMIN", "net_admin"])
    );
    assert_eq!(
        service.working_dir().map(compose_lens::project::ProjectValue::value),
        Some(&"/srv/app".to_owned())
    );
    assert_eq!(
        service.read_only().map(compose_lens::project::ProjectValue::value),
        Some(&BooleanValue::Literal(true))
    );
    assert_pids_limit(project_view)?;
    assert_shm_size(project_view)?;
    assert_mem_limit(project_view)?;
    assert_tmpfs(project_view)?;
    let dns = service.dns().ok_or("native project DNS expected")?;
    assert!(matches!(dns.value(), ProjectDns::List(items)
        if items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>()
            == ["1.1.1.1", "1.1.1.1", "resolver.internal"]));
    assert_eq!(
        service.dns_options().map(|options| options
            .value()
            .iter()
            .map(|item| item.value().as_str())
            .collect::<Vec<_>>()),
        Some(vec!["ndots:5", "timeout:2"])
    );
    let devices = service.devices().ok_or("native project devices expected")?;
    assert_eq!(devices.value().len(), 2);
    assert!(
        matches!(devices.value()[0].value(), ProjectDevice::Short(device) if device.raw().value() == "/dev/dri:/dev/dri:rwm")
    );
    Ok(())
}

fn assert_generated_devices(generated: &compose_lens::render::GeneratedComposeDocument) -> Result<(), &'static str> {
    let devices = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::devices)
        .ok_or("generated devices expected")?;
    assert_eq!(devices.items().len(), 2);
    assert!(matches!(
        &devices.items()[1],
        compose_lens::model::Device::Long(device)
            if device.source().is_some_and(|source| source.value() == "/dev/video0")
    ));
    Ok(())
}

fn assert_generated_dns(generated: &compose_lens::render::GeneratedComposeDocument) -> Result<(), &'static str> {
    let dns = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::dns)
        .ok_or("generated DNS expected")?;
    assert!(matches!(dns.form(), compose_lens::model::DnsForm::List(items)
        if items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>()
            == ["1.1.1.1", "1.1.1.1", "resolver.internal"]));
    Ok(())
}

fn assert_generated_dns_options(
    generated: &compose_lens::render::GeneratedComposeDocument,
) -> Result<(), &'static str> {
    let options = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::dns_options)
        .ok_or("generated DNS options expected")?;
    assert_eq!(
        options
            .items()
            .iter()
            .map(|item| item.value().as_str())
            .collect::<Vec<_>>(),
        ["ndots:5", "timeout:2"]
    );
    Ok(())
}

fn assert_lifecycle(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let service = project_view
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("native project service expected")?;
    assert_eq!(
        service
            .stop_signal()
            .map(compose_lens::project::ProjectValue::value)
            .map(String::as_str),
        Some("SIGUSR1")
    );
    assert!(matches!(
        service
            .stop_grace_period()
            .map(compose_lens::project::ProjectValue::value),
        Some(StopGracePeriod::Value(value)) if value == "1m30s"
    ));
    Ok(())
}

fn assert_dependency(project_view: &compose_lens::project::ProjectViewResult) -> Result<(), &'static str> {
    let dependency = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::depends_on)
        .and_then(|dependencies| dependencies.value().services().first())
        .ok_or("native project dependency expected")?;
    assert_eq!(dependency.value().service().value(), "database");
    assert!(matches!(
        dependency
            .value()
            .condition()
            .map(compose_lens::project::ProjectValue::value),
        Some(DependencyCondition::ServiceHealthy)
    ));
    Ok(())
}

#[test]
fn exposes_authored_and_effective_build_ulimits_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    build:\n",
        "      ulimits:\n",
        "        nofile: \"001024\"\n",
        "        nproc: {soft: \"1024\", hard: -1}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(1820), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => definition.ulimits(),
            Build::Context(_) => None,
        })
        .ok_or("authored build ulimits expected")?;
    assert_eq!(authored.entries().len(), 2);
    assert_eq!(authored.entries()[0].name().value(), "nofile");

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(1821),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project_view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let effective = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|build| match build.value() {
            ProjectBuild::Definition(definition) => definition.ulimits(),
            _ => None,
        })
        .ok_or("effective build ulimits expected")?;
    let _: &ProjectUlimits = effective.value();
    let entry: &ProjectUlimit = effective.value().entries()[1].value();
    let ProjectUlimitValue::Range(range) = entry.value() else {
        return Err("public build ulimit range expected".into());
    };
    let _: &ProjectUlimitRange = range;
    let hard: &ProjectUlimitScalar = range.hard().ok_or("public hard build ulimit expected")?.value();
    assert_eq!(hard.authored(), "-1");
    Ok(())
}

#[test]
fn exposes_authored_and_effective_build_entitlements_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    build: {entitlements: [network.host, security.insecure]}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(2004), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => definition.entitlements(),
            Build::Context(_) => None,
        })
        .ok_or("authored build entitlements expected")?;
    assert_eq!(authored.len(), 2);

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(2005),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project_view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let effective = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|build| match build.value() {
            ProjectBuild::Definition(definition) => definition.entitlements(),
            _ => None,
        })
        .ok_or("effective build entitlements expected")?;
    let _: &Vec<compose_lens::project::ProjectValue<String>> = effective.value();
    assert_eq!(effective.value()[1].value(), "security.insecure");
    Ok(())
}

#[test]
fn exposes_authored_and_effective_build_extra_hosts_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n    build:\n      extra_hosts:\n        gateway: host-gateway\n        v6: [\"[::1]\", \"${SECOND}\"]\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(1933), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => definition.extra_hosts(),
            Build::Context(_) => None,
        })
        .ok_or("authored build extra_hosts expected")?;
    let BuildExtraHosts::Map { entries, .. } = authored else {
        return Err("public authored mapping expected".into());
    };
    let _: &BuildExtraHostAddresses = entries[1].addresses();

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(1934),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("SECOND", "host-gateway");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project_view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let effective = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::build)
        .and_then(|build| match build.value() {
            ProjectBuild::Definition(definition) => definition.extra_hosts(),
            _ => None,
        })
        .ok_or("effective build extra_hosts expected")?;
    let _: &ProjectBuildExtraHosts = effective.value();
    let ProjectBuildExtraHosts::Map(entries) = effective.value() else {
        return Err("public effective mapping expected".into());
    };
    let _: &ProjectBuildExtraHostAddresses = entries[1].addresses();
    assert!(
        entries[1]
            .addresses()
            .as_list()
            .is_some_and(|values| values[1].is_sensitive())
    );
    Ok(())
}

#[test]
fn exposes_final_compose_key_boundary_without_implicit_io() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "version: '3.9'\ninclude: [base.yaml]\nmodels: {embedder: {model: local}}\nservices:\n",
        "  app:\n    domainname: example.test\n    gpus: all\n    use_api_socket: true\n",
        "    label_file: [labels.txt]\n    external_links: [external]\n    links: [db]\n",
        "    storage_opt: {size: 1G}\n    models: {embedder: MODEL_URL}\n",
        "    isolation: process\n    mac_address: 02:42:ac:11:00:02\n    uts: host\n",
        "    develop: {watch: [{action: sync, path: ., target: /src}]}\n",
        "networks:\n  app: {external: true}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(1999), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;
    let _: Option<&compose_lens::model::Located<String>> = document.version();
    let _: Option<&compose_lens::model::Includes> = document.include();
    let _: Option<&compose_lens::model::ModelDefinitions> = document.models();
    let service = document.service("app").ok_or("service expected")?;
    let _: Option<&compose_lens::model::Located<String>> = service.domainname();
    let _: Option<&compose_lens::model::Located<String>> = service.isolation();
    let _: Option<&compose_lens::model::Located<String>> = service.mac_address();
    let _: Option<&compose_lens::model::Located<String>> = service.uts();
    let _: Option<&compose_lens::model::Located<BooleanValue>> = service.use_api_socket();
    let _: Option<&compose_lens::model::LabelFiles> = service.label_files();
    let _: &[compose_lens::model::Located<String>] = service.external_links();
    let _: &[compose_lens::model::Located<String>] = service.links();
    let _: Option<&compose_lens::model::Labels> = service.storage_opt();
    let _: Option<&compose_lens::model::ServiceModels> = service.models();
    let _: Option<&compose_lens::model::Gpus> = service.gpus();
    let _: Option<&compose_lens::model::Develop> = service.develop();
    let network = document.networks().first().ok_or("network expected")?;
    let _: Option<&compose_lens::model::ResourceExternal> = network.external();

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(2000),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let view = result.view().ok_or("view expected")?;
    let _: Option<&compose_lens::project::ProjectValue<String>> = view.version();
    let _: Option<&compose_lens::project::ProjectValue<compose_lens::model::Includes>> = view.include();
    let service = view.service("app").ok_or("project service expected")?;
    let _: Option<&compose_lens::project::ProjectValue<String>> = service.domainname();
    let _: Option<&compose_lens::project::ProjectValue<compose_lens::model::Develop>> = service.develop();
    Ok(())
}

#[test]
fn composes_synthetic_document_inputs_without_filesystem_or_environment_access()
-> Result<(), Box<dyn std::error::Error>> {
    let root = IncludedProjectInput::new(
        IncludeIdentity::new("synthetic-root"),
        [DocumentInput::new(
            SourceId::new(15_000),
            DocumentOrigin::new("synthetic-root.yaml", "synthetic"),
            "services: {root: {image: example/root}}\ninclude: [child.yaml]\n",
        )],
    );
    let resolution = IncludeResolution::load(root, &SyntheticIncludeLoader);
    let composition = resolution.compose();
    let root = composition.root().ok_or("root composition expected")?;

    let _: &compose_lens::loader::IncludeComposition = root;
    let _: &compose_lens::loader::IncludeDefinition<compose_lens::project::ProjectService> =
        root.service("child").ok_or("child service expected")?;
    assert!(root.network("child-network").is_some());
    assert!(composition.is_complete());
    Ok(())
}

#[test]
fn plans_synthetic_project_directories_through_the_public_resolver_contract() -> Result<(), Box<dyn std::error::Error>>
{
    let root = IncludedProjectInput::new(
        IncludeIdentity::new("synthetic-root"),
        [DocumentInput::new(
            SourceId::new(15_010),
            DocumentOrigin::new("synthetic-root.yaml", "synthetic-root"),
            "include: [{path: child.yaml, project_directory: synthetic-directory}]\n",
        )],
    );
    let resolution = IncludeResolution::load(root, &SyntheticIncludeLoader);
    let plan = resolution.plan_project_directories(&SyntheticDirectoryResolver);
    let child = plan.entry(1).ok_or("child plan expected")?;

    let _: &compose_lens::loader::IncludeProjectDirectoryPlan = &plan;
    let _: &compose_lens::loader::IncludeProjectDirectoryEntry = child;
    assert_eq!(child.status(), IncludeProjectDirectoryStatus::Resolved);
    assert_eq!(
        child.effective_directory().map(|directory| directory.to_string_lossy()),
        Some("authorized/synthetic-directory".into())
    );
    assert!(plan.is_valid() && plan.is_complete());
    Ok(())
}

#[test]
fn exposes_caller_authorized_environment_and_secret_resolution_contracts() -> Result<(), Box<dyn std::error::Error>> {
    struct Files;
    impl compose_lens::resolution::EnvironmentFileProvider for Files {
        fn load(
            &self,
            request: &compose_lens::resolution::EnvironmentFileRequest<'_>,
        ) -> Result<
            Option<compose_lens::resolution::EnvironmentFileContent>,
            compose_lens::resolution::EnvironmentFileLoadError,
        > {
            assert_eq!(request.path(), "app.env");
            Ok(Some(compose_lens::resolution::EnvironmentFileContent::plain(
                "FROM_FILE=value\n",
            )))
        }
    }
    struct Secrets;
    impl compose_lens::resolution::SecretProvider for Secrets {
        fn resolve(
            &self,
            request: &compose_lens::resolution::SecretRequest,
        ) -> Result<Option<compose_lens::resolution::SecretValue>, compose_lens::resolution::SecretResolveError>
        {
            assert!(matches!(
                request.source(),
                compose_lens::resolution::SecretSource::File(_)
            ));
            Ok(Some(compose_lens::resolution::SecretValue::new("protected")))
        }
    }

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(15_019),
        DocumentOrigin::new("compose.yaml", "synthetic"),
        "---\nservices:\n  app:\n    env_file: [app.env]\nsecrets:\n  token:\n    file: token.txt\n",
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;
    let service = view.service("app").ok_or("service expected")?;
    let environment = compose_lens::resolution::resolve_service_environment(
        service,
        &compose_lens::interpolation::EmptyEnvironment,
        &Files,
    );
    assert_eq!(environment.entries()[0].name(), "FROM_FILE");
    let secrets = compose_lens::resolution::resolve_project_secrets(view, &Secrets);
    assert_eq!(secrets.secrets()[0].request().name(), "token");
    assert!(!format!("{secrets:?}").contains("protected"));
    Ok(())
}

#[test]
fn exposes_generated_top_level_file_definition_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let config = GeneratedConfigFileDefinition::new("configuration", GeneratedString::plain("app.yaml")?)?;
    let secret = GeneratedSecretFileDefinition::new("credential", GeneratedString::sensitive("secret.txt")?)?;
    let _: &str = config.name();
    let _: &GeneratedString = config.file();
    let _: &str = secret.name();
    let _: &GeneratedString = secret.file();

    let mut builder = ComposeDocumentBuilder::new();
    let mut service = GeneratedService::new("app")?;
    service.set_image(GeneratedString::plain("example/app")?)?;
    builder.add_service(service)?;
    builder.add_config_file(config)?;
    builder.add_secret_file(secret)?;
    let generated = builder.build(SourceId::new(15_020))?;
    assert!(
        generated
            .document()
            .configs()
            .iter()
            .any(|config| config.name().value() == "configuration")
    );
    assert!(
        generated
            .document()
            .secrets()
            .iter()
            .any(|secret| secret.name().value() == "credential")
    );
    Ok(())
}
