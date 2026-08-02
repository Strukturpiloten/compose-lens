//! Public implementation compatibility-profile behavior.

use compose_lens::diagnostic::Severity;
use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::{MergedProject, merge_project};
use compose_lens::profiles::{ProfileRequest, select_profiles};
use compose_lens::source::SourceId;
use compose_lens::validation::{
    CompatibilityClassification, CompatibilityFeature, CompatibilityProfile, CompatibilityReport, ComposeProvider,
    ContainerRuntime, EvidenceKind, IMPLEMENTATION_SPECIFIC_FEATURE, ImplementationVersion, UNKNOWN_FEATURE_SUPPORT,
    UNSUPPORTED_FEATURE, VersionRange, validate_compatibility,
};

const BASE: &str = include_str!("../fixtures/processing/compatibility-profiles/compose.yaml");
const OVERRIDE: &str = include_str!("../fixtures/processing/compatibility-profiles/compose.override.yaml");
const ISSUE_BACKLOG: &str = include_str!("../fixtures/typed-model/post-01-issue-backlog/compose.yaml");

#[test]
fn parses_exact_versions_and_checks_inclusive_evidence_ranges() -> Result<(), Box<dyn std::error::Error>> {
    let version: ImplementationVersion = "v2.24.4".parse()?;
    assert_eq!(version, ImplementationVersion::new(2, 24, 4));
    assert_eq!(version.to_string(), "2.24.4");
    assert!("2.24".parse::<ImplementationVersion>().is_err());
    assert!("2.24.4-beta".parse::<ImplementationVersion>().is_err());

    let range = VersionRange::new(
        Some(ImplementationVersion::new(2, 24, 4)),
        Some(ImplementationVersion::new(2, 40, 3)),
    )?;
    assert!(range.contains(ImplementationVersion::new(2, 24, 4)));
    assert!(range.contains(ImplementationVersion::new(2, 40, 3)));
    assert!(!range.contains(ImplementationVersion::new(2, 24, 3)));
    assert!(
        VersionRange::new(
            Some(ImplementationVersion::new(5, 0, 0)),
            Some(ImplementationVersion::new(4, 0, 0)),
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn discovers_selected_compatibility_features_without_normalizing_source() -> Result<(), Box<dyn std::error::Error>> {
    let project = compatibility_project()?;
    let selection = select_profiles(&project, &ProfileRequest::new());
    let report = validate_compatibility(&project, Some(&selection), CompatibilityProfile::specification());

    assert_eq!(report.findings().len(), 7, "{:#?}", report.findings());
    assert_eq!(feature_count(&report, CompatibilityFeature::ImageTagAndDigest), 1);
    assert_eq!(feature_count(&report, CompatibilityFeature::ShortBindSelinuxRelabel), 1);
    assert_eq!(feature_count(&report, CompatibilityFeature::LongBindSelinuxRelabel), 1);
    assert_eq!(feature_count(&report, CompatibilityFeature::ResetTag), 1);
    assert_eq!(feature_count(&report, CompatibilityFeature::OverrideTag), 1);
    assert_eq!(feature_count(&report, CompatibilityFeature::ExtensionField), 2);
    assert!(
        report
            .findings()
            .iter()
            .all(|finding| !finding.occurrence().path().iter().any(|segment| segment == "debug"))
    );
    assert_eq!(
        classification(&report, CompatibilityFeature::ImageTagAndDigest),
        Some(CompatibilityClassification::ImplementationSpecific)
    );
    assert_eq!(
        classification(&report, CompatibilityFeature::LongBindSelinuxRelabel),
        Some(CompatibilityClassification::Supported)
    );
    assert_eq!(
        project
            .value(&["services", "app", "image"])
            .and_then(compose_lens::merge::MergedValue::as_scalar)
            .map(compose_lens::merge::MergedScalar::value),
        Some("registry.example/app:1.2@sha256:abcdef")
    );
    Ok(())
}

#[test]
fn applies_the_documented_docker_override_version_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let project = compatibility_project()?;
    let old = validate_compatibility(
        &project,
        None,
        CompatibilityProfile::docker_compose(ImplementationVersion::new(2, 24, 3)),
    );
    let supported = validate_compatibility(
        &project,
        None,
        CompatibilityProfile::docker_compose(ImplementationVersion::new(2, 24, 4)),
    );

    assert!(!old.is_valid());
    assert_eq!(
        classification(&old, CompatibilityFeature::OverrideTag),
        Some(CompatibilityClassification::Unsupported)
    );
    assert!(
        old.diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == UNSUPPORTED_FEATURE)
    );
    assert_eq!(
        classification(&supported, CompatibilityFeature::OverrideTag),
        Some(CompatibilityClassification::Supported)
    );
    let evidence = supported
        .findings()
        .iter()
        .find(|finding| finding.occurrence().feature() == CompatibilityFeature::OverrideTag)
        .ok_or("override finding expected")?
        .rule()
        .evidence();
    assert_eq!(
        evidence[0].provider_versions().and_then(VersionRange::minimum),
        Some(ImplementationVersion::new(2, 24, 4))
    );
    assert_eq!(evidence[0].kind(), EvidenceKind::OfficialDocumentation);
    let old_evidence = old
        .findings()
        .iter()
        .find(|finding| finding.occurrence().feature() == CompatibilityFeature::OverrideTag)
        .ok_or("old override finding expected")?
        .rule()
        .evidence();
    assert_eq!(old_evidence[0].kind(), EvidenceKind::ProviderConformance);
    assert_eq!(
        old_evidence[0].provider_versions(),
        Some(VersionRange::exact(ImplementationVersion::new(2, 24, 3)))
    );
    Ok(())
}

#[test]
fn applies_reviewed_provider_outcomes_only_to_their_exact_versions() -> Result<(), Box<dyn std::error::Error>> {
    let project = compatibility_project()?;
    let observed = validate_compatibility(
        &project,
        None,
        CompatibilityProfile::docker_compose(ImplementationVersion::new(5, 3, 1)),
    );
    let unobserved = validate_compatibility(
        &project,
        None,
        CompatibilityProfile::docker_compose(ImplementationVersion::new(5, 3, 0)),
    );

    assert_eq!(
        classification(&observed, CompatibilityFeature::ImageTagAndDigest),
        Some(CompatibilityClassification::Supported)
    );
    assert_eq!(
        classification(&observed, CompatibilityFeature::ResetTag),
        Some(CompatibilityClassification::Supported)
    );
    assert_eq!(
        classification(&unobserved, CompatibilityFeature::ImageTagAndDigest),
        Some(CompatibilityClassification::ImplementationSpecific)
    );
    assert_eq!(
        classification(&unobserved, CompatibilityFeature::ResetTag),
        Some(CompatibilityClassification::ImplementationSpecific)
    );
    let evidence = observed
        .findings()
        .iter()
        .find(|finding| finding.occurrence().feature() == CompatibilityFeature::ImageTagAndDigest)
        .ok_or("image evidence expected")?
        .rule()
        .evidence();
    assert_eq!(evidence[0].kind(), EvidenceKind::ProviderConformance);
    assert_eq!(
        evidence[0].provider_versions(),
        Some(VersionRange::exact(ImplementationVersion::new(5, 3, 1)))
    );
    Ok(())
}

#[test]
fn separates_compose_provider_from_the_podman_runtime() -> Result<(), Box<dyn std::error::Error>> {
    let project = compatibility_project()?;
    let profile = CompatibilityProfile::docker_compose(ImplementationVersion::new(2, 40, 3))
        .with_runtime(ContainerRuntime::Podman(ImplementationVersion::new(5, 6, 2)));
    let report = validate_compatibility(&project, None, profile);

    assert_eq!(
        report.profile().provider(),
        ComposeProvider::DockerCompose(ImplementationVersion::new(2, 40, 3))
    );
    assert_eq!(
        report.profile().runtime(),
        Some(ContainerRuntime::Podman(ImplementationVersion::new(5, 6, 2)))
    );
    assert_eq!(
        classification(&report, CompatibilityFeature::ShortBindSelinuxRelabel),
        Some(CompatibilityClassification::Supported)
    );
    assert_eq!(
        classification(&report, CompatibilityFeature::LongBindSelinuxRelabel),
        Some(CompatibilityClassification::Unsupported)
    );
    let evidence = report
        .findings()
        .iter()
        .find(|finding| finding.occurrence().feature() == CompatibilityFeature::LongBindSelinuxRelabel)
        .ok_or("long SELinux finding expected")?
        .rule()
        .evidence();
    assert_eq!(
        evidence[0].provider_versions().and_then(VersionRange::minimum),
        Some(ImplementationVersion::new(2, 40, 3))
    );
    assert_eq!(
        evidence[0].runtime_versions().and_then(VersionRange::minimum),
        Some(ImplementationVersion::new(5, 6, 2))
    );
    assert!(!report.is_valid());
    Ok(())
}

#[test]
fn applies_exact_podman_compose_provider_evidence_without_claiming_runtime_effects()
-> Result<(), Box<dyn std::error::Error>> {
    let project = compatibility_project()?;
    let profile = CompatibilityProfile::podman_compose(ImplementationVersion::new(1, 5, 0))
        .with_runtime(ContainerRuntime::Podman(ImplementationVersion::new(5, 8, 2)));
    let report = validate_compatibility(&project, None, profile);

    assert!(!report.is_valid());
    assert_eq!(
        classification(&report, CompatibilityFeature::ResetTag),
        Some(CompatibilityClassification::Unsupported)
    );
    assert_eq!(
        classification(&report, CompatibilityFeature::OverrideTag),
        Some(CompatibilityClassification::Supported)
    );
    assert_eq!(
        classification(&report, CompatibilityFeature::ImageTagAndDigest),
        Some(CompatibilityClassification::Supported)
    );
    assert_eq!(
        classification(&report, CompatibilityFeature::LongBindSelinuxRelabel),
        Some(CompatibilityClassification::Unknown)
    );
    assert_eq!(
        classification(&report, CompatibilityFeature::ExtensionField),
        Some(CompatibilityClassification::Extension)
    );
    assert!(
        report
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == UNSUPPORTED_FEATURE)
    );
    assert!(report.diagnostics().iter().any(|diagnostic| {
        diagnostic.severity() == Severity::Warning && diagnostic.code() == UNKNOWN_FEATURE_SUPPORT
    }));
    Ok(())
}

#[test]
fn records_podman_compose_1_3_override_as_unsupported() -> Result<(), Box<dyn std::error::Error>> {
    let project = compatibility_project()?;
    let report = validate_compatibility(
        &project,
        None,
        CompatibilityProfile::podman_compose(ImplementationVersion::new(1, 3, 0)),
    );
    assert_eq!(
        classification(&report, CompatibilityFeature::OverrideTag),
        Some(CompatibilityClassification::Unsupported)
    );
    assert_eq!(
        classification(&report, CompatibilityFeature::ResetTag),
        Some(CompatibilityClassification::Unsupported)
    );
    Ok(())
}

#[test]
fn tolerant_profile_preserves_unknown_support_as_notes() -> Result<(), Box<dyn std::error::Error>> {
    let project = compatibility_project()?;
    let report = validate_compatibility(&project, None, CompatibilityProfile::tolerant());

    assert!(report.is_valid());
    assert!(
        report
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.severity() == Severity::Note)
    );
    Ok(())
}

#[test]
fn redacts_sensitive_feature_values_from_reports() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    image: example/app:${PRIVATE}@sha256:abcdef\n";
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(251),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("PRIVATE", "private-compatibility-value");
    let interpolation = loaded.interpolate(&environment);
    let merge = merge_project(&loaded, Some(&interpolation));
    let project = merge.project().ok_or("merged project expected")?;
    let report = validate_compatibility(project, None, CompatibilityProfile::specification());

    assert!(report.findings()[0].occurrence().is_sensitive());
    assert!(!format!("{report:?}").contains("private-compatibility-value"));
    assert!(report.diagnostics().iter().all(|diagnostic| {
        !diagnostic.message().contains("private-compatibility-value")
            && diagnostic
                .labels()
                .iter()
                .all(|label| !label.message().contains("private-compatibility-value"))
            && diagnostic
                .notes()
                .iter()
                .all(|note| !note.contains("private-compatibility-value"))
    }));
    Ok(())
}

fn compatibility_project() -> Result<MergedProject, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(241),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            BASE,
        ),
        DocumentInput::new(
            SourceId::new(242),
            DocumentOrigin::new("compose.override.yaml", "workspace/overrides"),
            OVERRIDE,
        ),
    ])?;
    let merge = merge_project(&loaded, None);
    merge.project().cloned().ok_or_else(|| "merged project expected".into())
}

fn feature_count(report: &CompatibilityReport, feature: CompatibilityFeature) -> usize {
    report
        .findings()
        .iter()
        .filter(|finding| finding.occurrence().feature() == feature)
        .count()
}

fn classification(report: &CompatibilityReport, feature: CompatibilityFeature) -> Option<CompatibilityClassification> {
    report
        .findings()
        .iter()
        .find(|finding| finding.occurrence().feature() == feature)
        .map(|finding| finding.rule().classification())
}

#[test]
fn implementation_specific_diagnostics_use_a_stable_code() -> Result<(), Box<dyn std::error::Error>> {
    let project = compatibility_project()?;
    let report = validate_compatibility(&project, None, CompatibilityProfile::specification());
    assert!(
        report
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == IMPLEMENTATION_SPECIFIC_FEATURE)
    );
    Ok(())
}

#[test]
fn classifies_evidence_backed_runtime_tokens_separately() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(251),
        DocumentOrigin::new("compose.yaml", "fixtures/typed-model/post-01-issue-backlog"),
        ISSUE_BACKLOG,
    )])?;
    let merge = merge_project(&loaded, None);
    let project = merge.project().ok_or("merged issue-backlog project expected")?;
    let report = validate_compatibility(project, None, CompatibilityProfile::specification());

    assert_eq!(feature_count(&report, CompatibilityFeature::HostGatewayToken), 3);
    assert_eq!(feature_count(&report, CompatibilityFeature::PodmanUserNamespaceMode), 1);
    for feature in [
        CompatibilityFeature::HostGatewayToken,
        CompatibilityFeature::PodmanUserNamespaceMode,
    ] {
        assert_eq!(
            classification(&report, feature),
            Some(CompatibilityClassification::ImplementationSpecific)
        );
        let finding = report
            .findings()
            .iter()
            .find(|finding| finding.occurrence().feature() == feature)
            .ok_or("runtime-token finding expected")?;
        assert!(
            finding
                .rule()
                .evidence()
                .iter()
                .any(|evidence| evidence.kind() == EvidenceKind::OfficialDocumentation)
        );
    }
    let userns = report
        .findings()
        .iter()
        .find(|finding| finding.occurrence().feature() == CompatibilityFeature::PodmanUserNamespaceMode)
        .ok_or("Podman userns finding expected")?;
    assert_eq!(
        userns.rule().evidence()[0]
            .runtime_versions()
            .and_then(VersionRange::minimum),
        Some(ImplementationVersion::new(5, 4, 0))
    );
    Ok(())
}
