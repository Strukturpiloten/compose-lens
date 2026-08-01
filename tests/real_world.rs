//! Licensed real-world Compose project regressions.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::{MergedScalar, MergedValue, merge_project};
use compose_lens::model::{
    BooleanValue, ComposeDocument, Located, MountType, SelinuxRelabel, VolumeMount, VolumeSyntax,
};
use compose_lens::profiles::{ProfileRequest, select_profiles};
use compose_lens::render::render_canonical;
use compose_lens::resolution::{ReferenceStatus, validate_references};
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;
use compose_lens::validation::{CompatibilityFeature, CompatibilityProfile, validate_compatibility};

const TYPO3_COMPOSE: &str = include_str!("../fixtures/real-world/strukturpiloten-typo3-postgresql/compose.yaml");
const TYPO3_ENVIRONMENT: &str = include_str!("../fixtures/real-world/strukturpiloten-typo3-postgresql/environment.env");
const FIXTURE_DIRECTORY: &str = "fixtures/real-world/strukturpiloten-typo3-postgresql";
const DATABASE_PASSWORD: &str = "fixture-database-password";
const ADMIN_PASSWORD: &str = "fixture-admin-password";
const AWESOME_COMPOSE: &str =
    include_str!("../fixtures/real-world/docker-awesome-compose-nginx-golang-mysql/compose.yaml");
const AWESOME_FIXTURE_DIRECTORY: &str = "fixtures/real-world/docker-awesome-compose-nginx-golang-mysql";

#[test]
fn preserves_and_processes_docker_awesome_compose() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(405), AWESOME_COMPOSE)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed Awesome Compose document expected")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert_eq!(syntax.document().render_preserved(), AWESOME_COMPOSE);
    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(document.services().len(), 3);
    assert_eq!(document.volumes().len(), 1);
    assert_eq!(document.secrets().len(), 1);

    let backend = document.service("backend").ok_or("backend service expected")?;
    assert_eq!(backend.secrets().len(), 1);
    assert!(
        backend
            .unknown_fields()
            .iter()
            .any(|field| field.name().value() == "build")
    );
    let database = document.service("db").ok_or("database service expected")?;
    assert_eq!(database.secrets().len(), 1);
    assert_eq!(database.volumes().len(), 1);
    let proxy = document.service("proxy").ok_or("proxy service expected")?;
    let Some(VolumeMount::Long(bind)) = proxy.volumes().first() else {
        return Err("proxy long bind mount expected".into());
    };
    assert_eq!(bind.mount_type().map(Located::value), Some(&MountType::Bind));
    assert_eq!(bind.read_only().map(Located::value), Some(&BooleanValue::Literal(true)));

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(406),
        DocumentOrigin::new("compose.yaml", AWESOME_FIXTURE_DIRECTORY),
        AWESOME_COMPOSE,
    )])?;
    let merge = merge_project(&loaded, None);
    let project = merge.project().ok_or("merged Awesome Compose project expected")?;
    assert!(loaded.is_valid(), "{:#?}", loaded.diagnostics());
    assert!(merge.is_valid(), "{:#?}", merge.diagnostics());

    let selection = select_profiles(project, &ProfileRequest::new());
    let references = validate_references(project, Some(&selection));
    assert!(references.is_valid(), "{:#?}", references.diagnostics());
    assert_eq!(references.references().len(), 5);
    assert!(
        references
            .references()
            .iter()
            .all(|reference| reference.status() == ReferenceStatus::Found)
    );

    let rendered = render_canonical(project, Some(&selection));
    assert!(rendered.is_valid(), "{:#?}", rendered.diagnostics());
    let reparsed = SyntaxDocument::parse(SourceId::new(407), rendered.output())?;
    assert!(reparsed.is_valid(), "{:#?}", reparsed.diagnostics());
    Ok(())
}

#[test]
fn preserves_and_types_the_generated_typo3_project() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(401), TYPO3_COMPOSE)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed TYPO3 document expected")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert_eq!(syntax.document().render_preserved(), TYPO3_COMPOSE);
    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(
        document.name().map(|name| name.value().as_str()),
        Some("${PODMAN_NAMESPACE}-${PODMAN_SERVICE}-${PODMAN_STAGE}")
    );
    assert_eq!(document.services().len(), 5);
    assert_eq!(document.networks().len(), 2);
    assert_eq!(
        document
            .services()
            .iter()
            .map(|service| service.name().value().as_str())
            .collect::<Vec<_>>(),
        [
            "typo3-phpfpm",
            "typo3-installer",
            "typo3-manager",
            "nginx",
            "postgresql"
        ]
    );

    let mounts = document
        .services()
        .iter()
        .flat_map(compose_lens::model::Service::volumes)
        .collect::<Vec<_>>();
    assert_eq!(mounts.len(), 15);
    assert!(mounts.iter().all(|mount| mount.syntax() == VolumeSyntax::Short));
    assert_eq!(
        mounts
            .iter()
            .filter(|mount| mount.selinux_relabel() == Some(SelinuxRelabel::Shared))
            .count(),
        10
    );
    assert_eq!(
        mounts
            .iter()
            .filter(|mount| mount.selinux_relabel() == Some(SelinuxRelabel::Private))
            .count(),
        5
    );
    assert!(document.services().iter().all(|service| {
        service
            .unknown_fields()
            .iter()
            .any(|field| field.name().value() == "userns_mode")
    }));
    Ok(())
}

#[test]
fn processes_and_renders_the_sanitized_typo3_project() -> Result<(), Box<dyn std::error::Error>> {
    let environment = fixture_environment()?;
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(402),
        DocumentOrigin::new("compose.yaml", FIXTURE_DIRECTORY),
        TYPO3_COMPOSE,
    )])?;
    let interpolation = loaded.interpolate(&environment);
    let merge = merge_project(&loaded, Some(&interpolation));
    let project = merge.project().ok_or("merged TYPO3 project expected")?;

    assert!(loaded.is_valid(), "{:#?}", loaded.diagnostics());
    assert!(interpolation.is_valid(), "{:#?}", interpolation.diagnostics());
    assert!(merge.is_valid(), "{:#?}", merge.diagnostics());
    assert_eq!(scalar(project.value(&["name"])), Some("strukturpiloten-typo3-test"));
    assert_eq!(
        scalar(project.value(&["services", "postgresql", "image"])),
        Some(
            "docker.io/postgres:18.4-alpine3.22@sha256:774521500f4c22761b25a6bdb772a0a3c2e8dd32468210bdad9231c5752ea398"
        )
    );
    assert_eq!(
        scalar(project.value(&["services", "nginx", "userns_mode"])),
        Some("keep-id")
    );

    let selection = select_profiles(project, &ProfileRequest::new());
    assert!(selection.is_valid(), "{:#?}", selection.diagnostics());
    assert_eq!(selection.services().len(), 5);
    assert!(
        selection
            .services()
            .iter()
            .all(compose_lens::profiles::ServiceSelection::is_active)
    );

    let references = validate_references(project, Some(&selection));
    assert!(references.is_valid(), "{:#?}", references.diagnostics());
    assert_eq!(references.references().len(), 12);
    assert!(
        references
            .references()
            .iter()
            .all(|reference| reference.status() == ReferenceStatus::Found)
    );

    let compatibility = validate_compatibility(project, Some(&selection), CompatibilityProfile::tolerant());
    assert!(compatibility.is_valid(), "{:#?}", compatibility.diagnostics());
    assert_eq!(
        compatibility
            .findings()
            .iter()
            .filter(|finding| finding.occurrence().feature() == CompatibilityFeature::ImageTagAndDigest)
            .count(),
        2
    );
    assert_eq!(
        compatibility
            .findings()
            .iter()
            .filter(|finding| finding.occurrence().feature() == CompatibilityFeature::ShortBindSelinuxRelabel)
            .count(),
        15
    );

    let rendered = render_canonical(project, Some(&selection));
    assert!(rendered.is_valid(), "{:#?}", rendered.diagnostics());
    assert!(rendered.is_sensitive());
    assert!(rendered.output().contains(DATABASE_PASSWORD));
    assert!(rendered.output().contains(ADMIN_PASSWORD));
    assert!(!format!("{rendered:?}").contains(DATABASE_PASSWORD));
    assert!(!format!("{rendered:?}").contains(ADMIN_PASSWORD));

    let reparsed = SyntaxDocument::parse(SourceId::new(403), rendered.output())?;
    assert!(reparsed.is_valid(), "{:#?}", reparsed.diagnostics());
    let reloaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(404),
        DocumentOrigin::new("canonical.yaml", FIXTURE_DIRECTORY),
        rendered.output(),
    )])?;
    let remerge = merge_project(&reloaded, None);
    let reproject = remerge.project().ok_or("remerged TYPO3 project expected")?;
    assert_eq!(render_canonical(reproject, None).output(), rendered.output());
    Ok(())
}

fn fixture_environment() -> Result<MapEnvironment, Box<dyn std::error::Error>> {
    let mut environment = MapEnvironment::new();
    for (index, line) in TYPO3_ENVIRONMENT.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line
            .split_once('=')
            .ok_or_else(|| format!("environment fixture line {} has no equals sign", index + 1))?;
        if matches!(name, "DATABASE_SQL_PASSWORD" | "TYPO3_SETUP_ADMIN_PASSWORD") {
            let _ = environment.insert_sensitive(name, value);
        } else {
            let _ = environment.insert(name, value);
        }
    }
    Ok(environment)
}

fn scalar(value: Option<&MergedValue>) -> Option<&str> {
    value.and_then(MergedValue::as_scalar).map(MergedScalar::value)
}
