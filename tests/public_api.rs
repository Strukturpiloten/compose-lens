//! Consumer-facing contract for the supported 0.1.x processing path.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::merge_project;
use compose_lens::model::{
    BooleanValue, ComposeDocument, DependencyCondition, Entrypoint, EnvironmentFileFormatKind, HealthcheckDuration,
    HostAddressKind, HostnameKind, IdentityComponent, MemLimitKind, MemLimitScalarKind, MemLimitUnit, PidsLimitKind,
    PullPolicyKind, RestartPolicyKind, ShmSizeKind, ShmSizeScalarKind, ShmSizeUnit, StopGracePeriod,
    UserNamespaceModeKind,
};
use compose_lens::profiles::{ProfileRequest, select_profiles};
use compose_lens::project::{
    ProjectDevice, ProjectEnvironmentFile, ProjectGrant, ProjectSysctls, ProjectTmpfs, ProjectUlimit,
    ProjectUlimitRange, ProjectUlimitScalar, ProjectUlimitValue, ProjectUlimits, build_project_view,
};
use compose_lens::render::{
    ComposeDocumentBuilder, GeneratedDevice, GeneratedEntrypoint, GeneratedEnvironmentFile, GeneratedHostname,
    GeneratedLabel, GeneratedLongDevice, GeneratedMemLimit, GeneratedPidsLimit, GeneratedPullPolicy,
    GeneratedRestartPolicy, GeneratedService, GeneratedShmSize, GeneratedString, GeneratedSysctl, GeneratedSysctls,
    GeneratedTmpfs, GeneratedUlimit, GeneratedUlimitValue, GeneratedUlimits, ReplacementScalar, ScalarEdit,
    apply_preservation_edits, render_canonical,
};
use compose_lens::resolution::validate_references;
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;
use compose_lens::validation::{
    CompatibilityFeature, CompatibilityProfile, ImplementationVersion, validate_compatibility,
};

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
    let mut builder = ComposeDocumentBuilder::new();
    builder.set_name("example")?;
    builder.add_service(service)?;

    let generated = builder.build(SourceId::new(503))?;
    assert_generated_hostname(&generated)?;
    assert_generated_capabilities(&generated)?;
    assert_generated_devices(&generated)?;
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
            "name: \"example\"\n",
            "services:\n",
            "  \"app\":\n",
            "    hostname: \"API.Example.COM\"\n",
            "    container_name: \"example-app\"\n",
            "    image: \"example.invalid/app:1\"\n",
            "    entrypoint:\n",
            "      - \"/usr/bin/env\"\n",
            "    init: true\n",
            "    env_file:\n",
            "      - \"./app.env\"\n",
            "    labels:\n",
            "      \"com.example.owner\": \"strukturpiloten\"\n",
            "    cap_add:\n",
            "      - \"SYS_TIME\"\n",
            "      - \"sys_time\"\n",
            "    cap_drop:\n",
            "      - \"NET_ADMIN\"\n",
            "      - \"net_admin\"\n",
            "    pids_limit: 64\n",
            "    shm_size: \"64mb\"\n",
            "    mem_limit: \"128b\"\n",
            "    devices:\n",
            "      - \"vendor.example/device=gpu\"\n",
            "      - source: \"/dev/video0\"\n",
            "        target: \"/dev/camera\"\n",
            "        permissions: \"rw\"\n",
            "    tmpfs:\n",
            "      - \"/run\"\n",
            "      - \"/cache:mode=0700\"\n",
            "    sysctls:\n",
            "      \"net.ipv4.ip_forward\": \"1\"\n",
            "    ulimits:\n",
            "      \"nofile\":\n",
            "        soft: \"1024\"\n",
            "        hard: \"-1\"\n",
            "    pull_policy: \"every_12h\"\n",
            "    restart: \"unless-stopped\"\n",
            "    stop_signal: \"15\"\n",
            "    stop_grace_period: \"0s\"\n",
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
    assert!(service.unmodeled_fields().iter().any(|field| {
        field
            .path()
            .last()
            .is_some_and(|segment| segment == "pull_refresh_after")
    }));
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
