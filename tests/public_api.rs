//! Consumer-facing contract for the supported 0.1.x processing path.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::merge_project;
use compose_lens::model::{ComposeDocument, DependencyCondition, HealthcheckDuration, HostAddressKind};
use compose_lens::profiles::{ProfileRequest, select_profiles};
use compose_lens::project::build_project_view;
use compose_lens::render::{ReplacementScalar, ScalarEdit, apply_preservation_edits, render_canonical};
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
        "  database:\n",
        "    image: example.invalid/database:1\n",
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
    let gateway = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::extra_hosts)
        .and_then(|hosts| hosts.value().entries().first())
        .ok_or("native project extra host expected")?;
    assert_eq!(gateway.hostname().value(), "host.docker.internal");
    assert_eq!(gateway.address().value().raw(), "host-gateway");
    assert_eq!(gateway.address().value().kind(), HostAddressKind::HostGateway);
    let healthcheck = project_view
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::healthcheck)
        .ok_or("native project healthcheck expected")?;
    assert!(matches!(
        healthcheck
            .value()
            .interval()
            .map(compose_lens::project::ProjectValue::value),
        Some(HealthcheckDuration::Value(value)) if value == "30s"
    ));
    assert_dependency(&project_view)?;
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
