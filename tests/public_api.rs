//! Consumer-facing contract for the supported 0.1.x processing path.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::merge_project;
use compose_lens::model::{
    BooleanValue, ComposeDocument, DependencyCondition, HealthcheckDuration, HostAddressKind, IdentityComponent,
    RestartPolicyKind, UserNamespaceModeKind,
};
use compose_lens::profiles::{ProfileRequest, select_profiles};
use compose_lens::project::{ProjectGrant, build_project_view};
use compose_lens::render::{
    ComposeDocumentBuilder, GeneratedLabel, GeneratedRestartPolicy, GeneratedService, GeneratedString,
    ReplacementScalar, ScalarEdit, apply_preservation_edits, render_canonical,
};
use compose_lens::resolution::validate_references;
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;
use compose_lens::validation::{
    CompatibilityFeature, CompatibilityProfile, ImplementationVersion, validate_compatibility,
};

#[test]
fn supported_public_pipeline_compiles_and_preserves_explicit_stages() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    image: example.invalid/app:old\n",
        "    user: 1000:1001\n",
        "    userns_mode: keep-id\n",
        "    group_add: [audio, '44']\n",
        "    working_dir: /srv/app\n",
        "    read_only: true\n",
        "    labels:\n",
        "      com.example.owner: strukturpiloten\n",
        "    extra_hosts:\n",
        "      - host.docker.internal=host-gateway\n",
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
    let syntax = SyntaxDocument::parse(SourceId::new(501), source)?;
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
    assert_eq!(
        project_view
            .view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::image)
            .map(|image| image.value().raw()),
        Some("example.invalid/app:1.2.3@sha256:abcdef")
    );
    assert_host_and_health(&project_view)?;
    assert_dependency(&project_view)?;
    assert_execution_identity(&project_view)?;
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
    service.set_container_name(GeneratedString::plain("example-app")?)?;
    service.set_image(GeneratedString::plain("example.invalid/app:1")?)?;
    service.set_restart(GeneratedRestartPolicy::UnlessStopped)?;
    service.add_label(GeneratedLabel::new(
        "com.example.owner",
        GeneratedString::plain("strukturpiloten")?,
    )?)?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.set_name("example")?;
    builder.add_service(service)?;

    let generated = builder.build(SourceId::new(503))?;
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
    assert!(
        generated
            .document()
            .service("app")
            .and_then(compose_lens::model::Service::labels)
            .is_some()
    );
    assert!(matches!(
        generated
            .document()
            .service("app")
            .and_then(compose_lens::model::Service::restart)
            .map(compose_lens::model::RestartPolicy::kind),
        Some(RestartPolicyKind::UnlessStopped)
    ));
    assert_eq!(
        generated.text(),
        concat!(
            "name: \"example\"\n",
            "services:\n",
            "  \"app\":\n",
            "    container_name: \"example-app\"\n",
            "    image: \"example.invalid/app:1\"\n",
            "    labels:\n",
            "      \"com.example.owner\": \"strukturpiloten\"\n",
            "    restart: \"unless-stopped\"\n",
        )
    );
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
        service.working_dir().map(compose_lens::project::ProjectValue::value),
        Some(&"/srv/app".to_owned())
    );
    assert_eq!(
        service.read_only().map(compose_lens::project::ProjectValue::value),
        Some(&BooleanValue::Literal(true))
    );
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
