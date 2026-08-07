//! Licensed real-world Compose project regressions.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::{MergedScalar, MergedValue, merge_project};
use compose_lens::model::{
    BooleanValue, ComposeDocument, Located, MountType, SecurityOptionKind, SelinuxRelabel, VolumeMount, VolumeSyntax,
};
use compose_lens::profiles::{ProfileRequest, select_profiles};
use compose_lens::project::{ProjectSecurityOptionItem, ProjectService, ProjectValue, build_project_view};
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
const TYPO3_SECURITY_OVERRIDE: &str = concat!(
    "services:\n",
    "  nginx:\n    security_opt:\n",
    "      - \"no-new-privileges:true\"\n",
    "      - \"apparmor=fixture-profile\"\n",
    "      - \"seccomp=/workspace/seccomp.json\"\n",
    "      - \"seccomp=/workspace/seccomp.json\"\n",
    "      - \"label:disable\"\n",
    "      - \"label:disable\"\n",
    "      - \"label=disable\"\n",
    "      - \"label:filetype:container_file_t\"\n",
    "      - \"label:filetype:container_file_t\"\n",
    "      - \"label:filetype=container_file_t\"\n",
    "      - \"label:level:s0:c1,c2\"\n",
    "      - \"label:level:s0:c1,c2\"\n",
    "      - \"label:level=s0:c1,c2\"\n",
    "      - \"label:nested\"\n",
    "      - \"label:nested\"\n",
    "      - \"label=nested\"\n",
    "      - \"label:type:container_t\"\n",
    "      - \"label:type:container_t\"\n",
    "      - \"label:type=container_t\"\n",
    "      - \"mask=/proc/acpi:/proc/kcore\"\n",
    "      - \"mask=/proc/acpi:/proc/kcore\"\n",
    "      - \"mask=relative:opaque=value\"\n",
    "      - \"mask:/proc/acpi\"\n",
    "      - \"unmask=ALL\"\n",
    "      - \"unmask=ALL\"\n",
    "      - \"unmask=/proc/acpi:/sys/firmware\"\n",
    "      - \"unmask=/proc/*\"\n",
    "      - \"unmask=all\"\n",
);

#[test]
fn appends_raw_security_options_to_the_licensed_typo3_project_without_runtime_interpretation()
-> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(410),
            DocumentOrigin::new("compose.yaml", FIXTURE_DIRECTORY),
            TYPO3_COMPOSE,
        ),
        DocumentInput::new(
            SourceId::new(411),
            DocumentOrigin::new("compose.security.yaml", FIXTURE_DIRECTORY),
            TYPO3_SECURITY_OVERRIDE,
        ),
    ])?;
    let interpolation = loaded.interpolate(&MapEnvironment::new());
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let options = result
        .view()
        .and_then(|view| view.service("nginx"))
        .and_then(ProjectService::security_options)
        .ok_or("nginx security options expected")?;
    assert_eq!(
        options
            .value()
            .iter()
            .map(|item| item.value().value())
            .collect::<Vec<_>>(),
        [
            "no-new-privileges:true",
            "apparmor=fixture-profile",
            "seccomp=/workspace/seccomp.json",
            "seccomp=/workspace/seccomp.json",
            "label:disable",
            "label:disable",
            "label=disable",
            "label:filetype:container_file_t",
            "label:filetype:container_file_t",
            "label:filetype=container_file_t",
            "label:level:s0:c1,c2",
            "label:level:s0:c1,c2",
            "label:level=s0:c1,c2",
            "label:nested",
            "label:nested",
            "label=nested",
            "label:type:container_t",
            "label:type:container_t",
            "label:type=container_t",
            "mask=/proc/acpi:/proc/kcore",
            "mask=/proc/acpi:/proc/kcore",
            "mask=relative:opaque=value",
            "mask:/proc/acpi",
            "unmask=ALL",
            "unmask=ALL",
            "unmask=/proc/acpi:/sys/firmware",
            "unmask=/proc/*",
            "unmask=all",
        ]
    );
    assert!(matches!(
        options.value()[0].value().kind(),
        SecurityOptionKind::NoNewPrivileges { enabled: true }
    ));
    assert!(matches!(
        options.value()[2].value().kind(),
        SecurityOptionKind::Seccomp { profile } if profile == "/workspace/seccomp.json"
    ));
    assert!(matches!(
        options.value()[3].value().kind(),
        SecurityOptionKind::Seccomp { profile } if profile == "/workspace/seccomp.json"
    ));
    for index in [4, 5] {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::SecurityLabelDisable { enabled: true }
        ));
    }
    assert!(matches!(
        options.value()[6].value().kind(),
        SecurityOptionKind::SecurityLabelDisableNearMiss
    ));
    for index in [7, 8] {
        assert!(matches!(
            options.value()[index].value().kind(),
            SecurityOptionKind::SecurityLabelFileType { file_type }
                if file_type == "container_file_t"
        ));
    }
    assert!(matches!(
        options.value()[9].value().kind(),
        SecurityOptionKind::SecurityLabelFileTypeNearMiss
    ));
    assert_real_world_security_label_level(options.value());
    assert_real_world_mask_candidates(options.value());
    assert_real_world_unmask_candidates(options.value());
    assert_real_world_security_label_conflicts(&result);
    Ok(())
}

fn assert_real_world_mask_candidates(options: &[ProjectValue<ProjectSecurityOptionItem>]) {
    for index in [19, 20] {
        assert!(matches!(
            options[index].value().kind(),
            SecurityOptionKind::Mask { paths } if paths == "/proc/acpi:/proc/kcore"
        ));
    }
    assert!(matches!(
        options[21].value().kind(),
        SecurityOptionKind::Mask { paths } if paths == "relative:opaque=value"
    ));
    assert!(matches!(options[22].value().kind(), SecurityOptionKind::MaskNearMiss));
}

fn assert_real_world_unmask_candidates(options: &[ProjectValue<ProjectSecurityOptionItem>]) {
    for index in [23, 24] {
        assert!(matches!(
            options[index].value().kind(),
            SecurityOptionKind::Unmask { paths } if paths == "ALL"
        ));
    }
    for (index, expected) in [(25, "/proc/acpi:/sys/firmware"), (26, "/proc/*")] {
        assert!(matches!(
            options[index].value().kind(),
            SecurityOptionKind::Unmask { paths } if paths == expected
        ));
    }
    assert!(matches!(options[27].value().kind(), SecurityOptionKind::UnmaskNearMiss));
}

fn assert_real_world_security_label_level(options: &[ProjectValue<ProjectSecurityOptionItem>]) {
    for index in [10, 11] {
        assert!(matches!(
            options[index].value().kind(),
            SecurityOptionKind::SecurityLabelLevel { level } if level == "s0:c1,c2"
        ));
    }
    assert!(matches!(
        options[12].value().kind(),
        SecurityOptionKind::SecurityLabelLevelNearMiss
    ));
    for index in [13, 14] {
        assert!(matches!(
            options[index].value().kind(),
            SecurityOptionKind::SecurityLabelNested { enabled: true }
        ));
    }
    assert!(matches!(
        options[15].value().kind(),
        SecurityOptionKind::SecurityLabelNestedNearMiss
    ));
    for index in [16, 17] {
        assert!(matches!(
            options[index].value().kind(),
            SecurityOptionKind::SecurityLabelType { label_type } if label_type == "container_t"
        ));
    }
    assert!(matches!(
        options[18].value().kind(),
        SecurityOptionKind::SecurityLabelTypeNearMiss
    ));
}

fn assert_real_world_security_label_conflicts(result: &compose_lens::project::ProjectViewResult) {
    use compose_lens::model::{
        SECURITY_OPT_MASK_NEAR_MISS, SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT, SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_NESTED_CONFLICT, SECURITY_OPT_SECURITY_LABEL_TYPE_CONFLICT,
        SECURITY_OPT_UNMASK_NEAR_MISS,
    };

    for code in [
        SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_NESTED_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_TYPE_CONFLICT,
    ] {
        assert_eq!(
            result
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.code() == code)
                .count(),
            1
        );
    }
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_MASK_NEAR_MISS)
            .count(),
        1
    );
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_UNMASK_NEAR_MISS)
            .count(),
        1
    );
}

#[test]
fn applies_an_annotation_override_to_the_licensed_typo3_project_without_reparsing_generated_yaml()
-> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(408),
            DocumentOrigin::new("compose.yaml", FIXTURE_DIRECTORY),
            TYPO3_COMPOSE,
        ),
        DocumentInput::new(
            SourceId::new(409),
            DocumentOrigin::new("compose.annotations.yaml", FIXTURE_DIRECTORY),
            concat!(
                "services:\n",
                "  nginx:\n    annotations:\n",
                "      io.example.source: \"licensed-regression\"\n",
                "      io.example.stage: \"${ANNOTATION_STAGE:-test}\"\n",
            ),
        ),
    ])?;
    let interpolation = loaded.interpolate(&MapEnvironment::new());
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let annotations = result
        .view()
        .and_then(|view| view.service("nginx"))
        .and_then(ProjectService::annotations)
        .ok_or("nginx annotations expected")?;
    assert_eq!(annotations.value().entries().len(), 2);
    assert!(annotations.value().get("io.example.source").is_some());
    assert!(annotations.value().get("io.example.stage").is_some());
    Ok(())
}

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
    assert!(backend.build().is_some());
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
            .userns_mode()
            .is_some_and(|mode| mode.kind() == compose_lens::model::UserNamespaceModeKind::PodmanKeepId)
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
    assert_eq!(
        compatibility
            .findings()
            .iter()
            .filter(|finding| finding.occurrence().feature() == CompatibilityFeature::PodmanUserNamespaceMode)
            .count(),
        5
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
