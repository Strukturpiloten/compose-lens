//! Independent native contracts for reviewed real-application definitions.

#[path = "support/application.rs"]
mod application;

#[path = "support/application_policy.rs"]
mod application_policy;

use application::{CopiedFile, read_fixture, verify_application_fixture};
use compose_lens::interpolation::{InterpolationInput, MapEnvironment, interpolate};
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::{MergeOperation, MergedProject, merge_project};
use compose_lens::profiles::{ProfileRequest, select_profiles};
use compose_lens::project::build_project_view;
use compose_lens::render::{
    ComposeDocumentBuilder, GeneratedEnvironment, GeneratedMount, GeneratedNetworkAttachment, GeneratedPort,
    GeneratedProtocol, GeneratedResource, GeneratedService, GeneratedString,
};
use compose_lens::resolution::{
    EnvironmentFileContent, EnvironmentFileLoadError, EnvironmentFileProvider, EnvironmentFileRequest,
    ResolvedEnvironmentOrigin, ResolvedEnvironmentValue, resolve_service_environment, validate_references,
};
use compose_lens::source::{SourceId, SourceSpan};
use std::path::Path;

const NEXTCLOUD_REVISION: &str = "18716257362932e7520d5aed68412ac7c8492e6d";
const FORGEJO_REVISION: &str = "68e08b9a86bcb9ced34e8d831563e732176275a1";
const FORGEJO_IMAGE: &str =
    "codeberg.org/forgejo/forgejo@sha256:214f4ae63ee78be1e445e58573c88dc7215e72091210852e0df94eaac1a25685";
const GENERATED_SECRET: &str = "compose-lens-public-generated-canary";

#[test]
fn retained_application_evidence_is_bounded_and_linked() -> Result<(), String> {
    application_policy::validate_application_evidence()
}

#[test]
fn application_fixtures_are_exact_and_independent() -> Result<(), String> {
    application_policy::validate_application_fixtures()
}

#[test]
fn application_gate_retains_independence_and_complete_gate_coverage() -> Result<(), String> {
    application_policy::validate_application_independence_and_gates()
}

#[test]
fn provider_empty_required_message_is_supported() -> Result<(), Box<dyn std::error::Error>> {
    let source = "${BF_POSTGRES_IMAGE:?}";
    let span =
        SourceSpan::new(SourceId::new(110_000), 0, source.len()).ok_or("valid provider expression span expected")?;
    let mut environment = MapEnvironment::new();
    environment.insert("BF_POSTGRES_IMAGE", "example.invalid/postgres@sha256:reviewed");
    let result = interpolate(InterpolationInput::new(source, span), &environment);
    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert_eq!(result.resolved(), "example.invalid/postgres@sha256:reviewed");
    Ok(())
}

#[test]
fn nextcloud_application_has_independent_project_expectations() -> Result<(), Box<dyn std::error::Error>> {
    let root = nextcloud_fixture()?;
    let scenario = read_fixture(&root, "scenario.compose.yaml")?;
    let acceptance = read_fixture(&root, "scenario.acceptance.yaml")?;
    let mut environment = MapEnvironment::new();
    environment.insert_sensitive("BF_TEST_DB_PASSWORD", "public-db-canary");
    environment.insert_sensitive("BF_TEST_CACHE_PASSWORD", "public-cache-canary");
    environment.insert_sensitive("BF_TEST_ADMIN_PASSWORD", "public-admin-canary");
    environment.insert_sensitive("BF_TEST_EDGE_TOKEN", "public-edge-canary");

    let project = load_merged(
        &root,
        &[
            (SourceId::new(110_001), "scenario.compose.yaml", scenario),
            (SourceId::new(110_002), "scenario.acceptance.yaml", acceptance),
        ],
        &environment,
    )?;
    assert_eq!(project.source_ids(), &[SourceId::new(110_001), SourceId::new(110_002)]);
    assert_eq!(project.base_directory(), root);
    assert_eq!(scalar(&project, &["name"])?, "nextcloud-application");
    assert_eq!(mapping_len(&project, &["services"])?, 8);
    assert_eq!(mapping_len(&project, &["volumes"])?, 3);
    assert_eq!(mapping_len(&project, &["networks"])?, 2);
    assert_eq!(
        scalar(&project, &["services", "edge-proxy", "ports", "0"])?,
        "127.0.0.1:18443:8443"
    );
    assert_eq!(
        scalar(&project, &["services", "frontend", "depends_on", "app", "condition"])?,
        "service_started"
    );
    assert_eq!(
        scalar(&project, &["services", "init", "command", "1"])?,
        "--non-interactive"
    );
    assert_eq!(
        scalar(
            &project,
            &["services", "edge-proxy", "environment", "BOXFERRY_EDGE_TEST_TOKEN"]
        )?,
        "public-edge-canary"
    );
    assert_eq!(
        scalar(&project, &["services", "frontend", "volumes", "1", "source"],)?,
        "/srv/boxferry-fixture/nextcloud-application/frontend.conf"
    );
    assert_eq!(
        scalar(&project, &["services", "frontend", "volumes", "1", "target"],)?,
        "/etc/nginx/conf.d/nextcloud.conf"
    );

    let view = build_project_view(&project, None);
    assert!(view.is_valid(), "{:#?}", view.diagnostics());
    assert_eq!(view.view().ok_or("Nextcloud project view missing")?.services().len(), 8);
    let references = validate_references(&project, None);
    assert!(references.is_valid(), "{:#?}", references.diagnostics());
    assert!(project.root().is_sensitive());
    let debug = format!("{project:#?}");
    for secret in [
        "public-db-canary",
        "public-cache-canary",
        "public-admin-canary",
        "public-edge-canary",
    ] {
        assert!(!debug.contains(secret), "merged Debug leaked {secret}");
    }
    Ok(())
}

#[test]
fn forgejo_imported_scenario_has_independent_project_expectations() -> Result<(), Box<dyn std::error::Error>> {
    let root = forgejo_fixture()?;
    let scenario = read_fixture(&root, "scenario.compose.yaml")?;
    let mut environment = MapEnvironment::new();
    environment.insert_sensitive("BF_FORGEJO_DB_PASSWORD", "public-db-canary");
    environment.insert_sensitive("BF_FORGEJO_SECRET_KEY", "public-secret-canary");

    let project = load_merged(
        &root,
        &[(SourceId::new(110_100), "scenario.compose.yaml", scenario)],
        &environment,
    )?;
    assert_eq!(project.source_ids(), &[SourceId::new(110_100)]);
    assert_eq!(project.base_directory(), root);
    assert_eq!(scalar(&project, &["name"])?, "forgejo-application");
    assert_eq!(mapping_len(&project, &["services"])?, 2);
    assert_eq!(mapping_len(&project, &["volumes"])?, 2);
    assert_eq!(mapping_len(&project, &["networks"])?, 2);

    assert_eq!(scalar(&project, &["services", "forgejo", "image"])?, FORGEJO_IMAGE);
    assert_eq!(scalar(&project, &["services", "forgejo", "user"])?, "1000:1000");
    assert_eq!(
        scalar(
            &project,
            &["services", "forgejo", "environment", "FORGEJO__database__PASSWD"]
        )?,
        "public-db-canary"
    );
    assert_eq!(
        scalar(
            &project,
            &["services", "forgejo", "environment", "FORGEJO__security__SECRET_KEY"]
        )?,
        "public-secret-canary"
    );
    assert_eq!(
        scalar(&project, &["services", "forgejo", "ports", "0"])?,
        "127.0.0.1:13000:3000"
    );
    assert_eq!(
        scalar(&project, &["services", "forgejo", "ports", "1"])?,
        "127.0.0.1:12222:2222"
    );
    assert_eq!(
        scalar(&project, &["services", "forgejo", "volumes", "0"])?,
        "forgejo-data:/var/lib/gitea"
    );
    assert_eq!(
        scalar(&project, &["services", "db", "volumes", "0"])?,
        "db-data:/var/lib/postgresql/data"
    );
    assert_eq!(scalar(&project, &["services", "forgejo", "networks", "0"])?, "backend");
    assert_eq!(scalar(&project, &["services", "forgejo", "networks", "1"])?, "edge");
    assert_eq!(scalar(&project, &["services", "db", "networks", "0"])?, "backend");
    assert_eq!(scalar(&project, &["networks", "backend", "internal"])?, "true");
    assert_eq!(scalar(&project, &["networks", "edge", "external"])?, "true");
    assert!(project.value(&["services", "forgejo", "command"]).is_none());
    assert!(project.value(&["services", "db", "command"]).is_none());

    let view = build_project_view(&project, None);
    assert!(view.is_valid(), "{:#?}", view.diagnostics());
    assert_eq!(view.view().ok_or("Forgejo project view missing")?.services().len(), 2);
    let references = validate_references(&project, None);
    assert!(references.is_valid(), "{:#?}", references.diagnostics());
    assert!(project.root().is_sensitive());
    let debug = format!("{project:#?}");
    assert!(!debug.contains("public-db-canary"));
    assert!(!debug.contains("public-secret-canary"));
    Ok(())
}

#[test]
fn forgejo_application_distinguishes_interpolation_env_files_and_merge_tags() -> Result<(), Box<dyn std::error::Error>>
{
    let root = forgejo_fixture()?;
    let base = read_fixture(&root, "processing.base.yaml")?;
    let override_source = read_fixture(&root, "processing.override.yaml")?;
    let mut interpolation = MapEnvironment::new();
    interpolation.insert_sensitive("BF_FORGEJO_CONFIG_ROOT", "/reviewed/private/config");
    let project = load_merged(
        &root,
        &[
            (SourceId::new(110_101), "processing.base.yaml", base),
            (SourceId::new(110_102), "processing.override.yaml", override_source),
        ],
        &interpolation,
    )?;

    assert_eq!(
        scalar(&project, &["services", "forgejo", "environment", "INLINE_PRECEDENCE"])?,
        "final-inline"
    );
    assert_eq!(
        scalar(&project, &["services", "forgejo", "environment", "LITERAL_DOLLARS"])?,
        "$not-expanded"
    );
    assert_eq!(
        scalar(&project, &["services", "forgejo", "volumes", "0"])?,
        "/reviewed/private/config/app.ini:/etc/gitea/app.ini:ro"
    );
    assert_eq!(
        operation(&project, &["services", "forgejo", "ports"])?,
        MergeOperation::Override
    );
    assert_eq!(
        operation(&project, &["services", "maintenance", "environment"])?,
        MergeOperation::Reset
    );
    assert_eq!(
        operation(&project, &["services", "maintenance", "command"])?,
        MergeOperation::Override
    );
    assert_eq!(mapping_len(&project, &["services", "maintenance", "environment"])?, 0);
    assert_eq!(
        scalar(&project, &["services", "maintenance", "command", "2"])?,
        "printf '$final'"
    );

    let default_selection = select_profiles(&project, &ProfileRequest::new());
    assert!(default_selection.is_active("forgejo"));
    assert!(!default_selection.is_active("maintenance"));
    let all_selection = select_profiles(&project, &ProfileRequest::all());
    assert!(all_selection.is_active("forgejo"));
    assert!(all_selection.is_active("maintenance"));
    let selected = build_project_view(&project, Some(&all_selection));
    assert!(selected.is_valid(), "{:#?}", selected.diagnostics());
    assert_eq!(
        selected.view().ok_or("Forgejo project view missing")?.services().len(),
        2
    );
    let references = validate_references(&project, Some(&all_selection));
    assert!(references.is_valid(), "{:#?}", references.diagnostics());

    let service = selected
        .view()
        .and_then(|view| view.service("forgejo"))
        .ok_or("Forgejo service missing")?;
    let files = FixtureEnvironmentFiles {
        base: read_fixture(&root, "service.base.env")?,
        raw: read_fixture(&root, "service.raw.env")?,
    };
    let mut caller = MapEnvironment::new();
    caller.insert("FROM_CALLER", "from-caller");
    caller.insert_sensitive("BF_FORGEJO_DB_PASSWORD", "public-db-canary");
    let resolution = resolve_service_environment(service, &caller, &files);
    assert!(resolution.is_valid(), "{:#?}", resolution.diagnostics());
    assert_resolved(&resolution, "FROM_FILE", Some("from-base"), false)?;
    assert_resolved(&resolution, "INLINE_PRECEDENCE", Some("final-inline"), false)?;
    assert_resolved(&resolution, "INLINE_EMPTY", Some(""), false)?;
    assert_resolved(&resolution, "FROM_CALLER", Some("from-caller"), false)?;
    assert_resolved(&resolution, "EMPTY_FILE", Some(""), false)?;
    assert_resolved(&resolution, "UNSET_FILE", None, false)?;
    assert_resolved(&resolution, "QUOTED", Some("quoted # value"), false)?;
    assert_resolved(&resolution, "ESCAPED_DOLLAR", Some("${BF_FORGEJO_DB_PASSWORD}"), false)?;
    assert_resolved(
        &resolution,
        "RAW_VALUE",
        Some("${BF_FORGEJO_DB_PASSWORD} # literal"),
        false,
    )?;
    let inline = resolved_entry(&resolution, "INLINE_PRECEDENCE")?;
    assert!(matches!(inline.origin(), ResolvedEnvironmentOrigin::Service { .. }));
    let from_file = resolved_entry(&resolution, "FROM_FILE")?;
    assert!(matches!(from_file.origin(), ResolvedEnvironmentOrigin::File { path, .. } if path == "./service.base.env"));
    let debug = format!("{project:#?}\n{resolution:#?}");
    assert!(!debug.contains("/reviewed/private/config"));
    assert!(!debug.contains("public-db-canary"));
    Ok(())
}

#[test]
fn imported_forgejo_projects_keep_external_peer_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let root = forgejo_fixture()?;
    let mut environment = MapEnvironment::new();
    environment.insert_sensitive("BF_DB_PASSWORD", "public-db-canary");
    environment.insert_sensitive("BF_FORGEJO_SECRET_KEY", "public-secret-canary");
    environment.insert("BF_PREFIX", "reviewed");
    environment.insert("BF_RUN", "one");
    environment.insert("BF_FORGEJO_IMAGE", FORGEJO_IMAGE);
    environment.insert(
        "BF_POSTGRES_IMAGE",
        "docker.io/library/postgres@sha256:ef257d85f76e48da1c64832459b59fcaba1a4dac97bf5d7450c77753542eee94",
    );
    environment.insert(
        "BF_GIT_CLIENT_IMAGE",
        "docker.io/alpine/git@sha256:6f3b5029566da8e90b24945933dcd806be866b64b1e706f51828bf84faccf21b",
    );
    let app_source = read_fixture(&root, "provider.compose.yaml")?;
    let app = load_merged(
        &root,
        &[(SourceId::new(110_201), "provider.compose.yaml", app_source)],
        &environment,
    )?;
    let peer_source = read_fixture(&root, "provider-peer.compose.yaml")?;
    let peer = load_merged(
        &root,
        &[(SourceId::new(110_202), "provider-peer.compose.yaml", peer_source)],
        &environment,
    )?;
    assert_eq!(mapping_len(&app, &["services"])?, 2);
    assert_eq!(mapping_len(&peer, &["services"])?, 1);
    assert_eq!(mapping_len(&app, &["networks"])?, 2);
    assert_eq!(mapping_len(&app, &["volumes"])?, 2);
    assert!(app.value(&["services", "peer"]).is_none());
    assert!(peer.value(&["services", "forgejo"]).is_none());
    assert_eq!(
        scalar(&app, &["services", "db", "labels", "io.boxferry.live-run"])?,
        "one"
    );
    assert_eq!(
        scalar(&app, &["services", "forgejo", "labels", "io.boxferry.application"])?,
        "reviewed-forgejo"
    );
    assert_eq!(
        scalar(&app, &["networks", "backend", "labels", "io.boxferry.live-run"])?,
        "one"
    );
    assert_eq!(
        scalar(&app, &["volumes", "data", "labels", "io.boxferry.application"])?,
        "reviewed-forgejo"
    );
    assert_eq!(
        scalar(&app, &["services", "db", "healthcheck", "test", "0"])?,
        "CMD-SHELL"
    );
    assert_eq!(
        scalar(&app, &["services", "db", "healthcheck", "test", "1"])?,
        "pg_isready -U forgejo -d forgejo"
    );
    assert_eq!(
        scalar(&app, &["services", "forgejo", "depends_on", "db", "condition"])?,
        "service_healthy"
    );
    assert_eq!(
        scalar(&app, &["services", "forgejo", "ports", "0"])?,
        "127.0.0.1:13000:3000"
    );
    assert_eq!(
        scalar(&app, &["services", "forgejo", "ports", "1"])?,
        "127.0.0.1:12222:2222"
    );
    assert_eq!(
        scalar(&app, &["services", "db", "volumes", "0"])?,
        "db:/var/lib/postgresql/data"
    );
    assert_eq!(
        scalar(&app, &["services", "forgejo", "volumes", "0"])?,
        "data:/var/lib/gitea"
    );
    assert_eq!(scalar(&app, &["networks", "backend", "internal"])?, "true");
    assert_eq!(scalar(&app, &["networks", "edge", "external"])?, "true");
    assert_eq!(scalar(&peer, &["networks", "edge", "external"])?, "true");
    assert_eq!(scalar(&app, &["networks", "edge", "name"])?, "reviewed-shared-edge");
    assert_eq!(scalar(&peer, &["networks", "edge", "name"])?, "reviewed-shared-edge");
    assert_eq!(scalar(&peer, &["services", "peer", "entrypoint", "0"])?, "/bin/sh");
    assert_eq!(scalar(&peer, &["services", "peer", "entrypoint", "1"])?, "-ceu");
    assert_eq!(scalar(&peer, &["services", "peer", "command", "0"])?, "sleep 3600");
    let debug = format!("{app:#?}\n{peer:#?}");
    assert!(!debug.contains("public-db-canary"));
    assert!(!debug.contains("public-secret-canary"));
    Ok(())
}

#[test]
fn generated_forgejo_subset_matches_independent_bytes_and_mutation_oracle() -> Result<(), Box<dyn std::error::Error>> {
    let root = forgejo_fixture()?;
    let expected = read_fixture(&root, "expected.generated.yaml")?;
    let generated = generated_forgejo()?;
    assert_eq!(generated.text(), expected);
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:#?}").contains(GENERATED_SECRET));
    validate_generated_forgejo(generated.text())?;

    let missing_port = generated
        .text()
        .replace("      published: \"12222\"\n", "      published: \"12223\"\n");
    let error = match validate_generated_forgejo(&missing_port) {
        Ok(()) => return Err("wrong SSH port was accepted".into()),
        Err(error) => error,
    };
    assert!(error.contains("SSH publication"));
    let wrong_environment = generated
        .text()
        .replace("FORGEJO__database__HOST=db:5432", "FORGEJO__database__HOST=db:5433");
    let error = match validate_generated_forgejo(&wrong_environment) {
        Ok(()) => return Err("wrong database endpoint was accepted".into()),
        Err(error) => error,
    };
    assert!(error.contains("database endpoint"));
    Ok(())
}

fn generated_forgejo() -> Result<compose_lens::render::GeneratedComposeDocument, Box<dyn std::error::Error>> {
    let mut service = GeneratedService::new("forgejo")?;
    service.set_image(GeneratedString::plain(FORGEJO_IMAGE)?)?;
    service.add_environment(GeneratedEnvironment::literal(
        "FORGEJO__database__HOST",
        GeneratedString::plain("db:5432")?,
    )?);
    service.add_environment(GeneratedEnvironment::literal(
        "FORGEJO__database__PASSWD",
        GeneratedString::sensitive(GENERATED_SECRET)?,
    )?);
    service.add_port(GeneratedPort::new(
        3000,
        Some(13_000),
        Some("127.0.0.1".to_owned()),
        GeneratedProtocol::Tcp,
    )?);
    service.add_port(GeneratedPort::new(
        2222,
        Some(12_222),
        Some("127.0.0.1".to_owned()),
        GeneratedProtocol::Tcp,
    )?);
    service.add_mount(GeneratedMount::volume("forgejo-data", "/var/lib/gitea", false)?);
    service.add_network(GeneratedNetworkAttachment::new("backend")?)?;
    service.add_network(GeneratedNetworkAttachment::new("edge")?)?;

    let mut builder = ComposeDocumentBuilder::new();
    builder.set_name("forgejo-generated")?;
    builder.add_service(service)?;
    builder.add_network(GeneratedResource::application("backend")?)?;
    builder.add_network(GeneratedResource::external("edge")?)?;
    builder.add_volume(GeneratedResource::application("forgejo-data")?)?;
    Ok(builder.build(SourceId::new(110_301))?)
}

fn validate_generated_forgejo(source: &str) -> Result<(), String> {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(110_302),
        DocumentOrigin::new("generated.yaml", "independent-application-oracle"),
        source,
    )])
    .map_err(|error| error.to_string())?;
    let merge = merge_project(&loaded, None);
    let project = merge.project().ok_or_else(|| "generated project missing".to_owned())?;
    if scalar(project, &["services", "forgejo", "environment", "0"])? != "FORGEJO__database__HOST=db:5432" {
        return Err("generated database endpoint differs from independent oracle".to_owned());
    }
    let ports = project
        .value(&["services", "forgejo", "ports"])
        .and_then(compose_lens::merge::MergedValue::as_sequence)
        .ok_or_else(|| "generated ports must be a sequence".to_owned())?;
    if ports.len() != 2
        || scalar(project, &["services", "forgejo", "ports", "0", "target"])? != "3000"
        || scalar(project, &["services", "forgejo", "ports", "0", "published"])? != "13000"
        || scalar(project, &["services", "forgejo", "ports", "1", "target"])? != "2222"
        || scalar(project, &["services", "forgejo", "ports", "1", "published"])? != "12222"
    {
        return Err("generated HTTP/SSH publication differs from independent oracle".to_owned());
    }
    if scalar(project, &["networks", "edge", "external"])? != "true" {
        return Err("generated edge network must remain external".to_owned());
    }
    Ok(())
}

fn nextcloud_fixture() -> Result<std::path::PathBuf, String> {
    verify_application_fixture(
        "boxferry-nextcloud-application",
        NEXTCLOUD_REVISION,
        &[
            CopiedFile {
                path: "scenario.compose.yaml",
                hash_key: "scenario-compose-sha256",
                sha256: "9ed20c06a3b2b340833e5f660e47725593b84540f3b473009bffe7f69df6536f",
            },
            CopiedFile {
                path: "scenario.acceptance.yaml",
                hash_key: "scenario-acceptance-sha256",
                sha256: "79dc3dc478b710a9545a010a7e0f1338628c2c352e446dcfaef039238ea6447d",
            },
            CopiedFile {
                path: "interpolation.env",
                hash_key: "interpolation-sha256",
                sha256: "b9e1182df0d7aee94a3b3758b27501649d0aa6a1ee2b52683f81baf3ddc95eac",
            },
            CopiedFile {
                path: "provider.compose.yaml",
                hash_key: "provider-compose-sha256",
                sha256: "ca714ddfe9b64620faf47c714db6be2907f7ec6428529c3e041bfd123bdd4761",
            },
        ],
        &["UPSTREAM_LICENSE"],
    )
}

fn forgejo_fixture() -> Result<std::path::PathBuf, String> {
    verify_application_fixture(
        "boxferry-forgejo-application",
        FORGEJO_REVISION,
        &[
            CopiedFile {
                path: "scenario.compose.yaml",
                hash_key: "scenario-compose-sha256",
                sha256: "20f199e9d46d163bd4f3b8b6c3022e816b7f1542b7c59329ef5628fd1eb188f6",
            },
            CopiedFile {
                path: "interpolation.env",
                hash_key: "interpolation-sha256",
                sha256: "5de63b3ad5bafb6535ff49806a1ef737e25d3b32ee81257dcfb236db035cb4ce",
            },
            CopiedFile {
                path: "provider.compose.yaml",
                hash_key: "provider-compose-sha256",
                sha256: "2d7e81eeefc7060812900791db0a3a9fef08b748779f8697fef12f0cced4d5be",
            },
            CopiedFile {
                path: "provider-peer.compose.yaml",
                hash_key: "provider-peer-compose-sha256",
                sha256: "14a22ea3ab8f4941b2611c2a983a349cd69f8a141cce239cc4dbaa8469bca6c8",
            },
        ],
        &[
            "UPSTREAM_LICENSE",
            "processing.base.yaml",
            "processing.override.yaml",
            "service.base.env",
            "service.raw.env",
            "expected.generated.yaml",
        ],
    )
}

fn load_merged(
    root: &Path,
    inputs: &[(SourceId, &str, String)],
    environment: &MapEnvironment,
) -> Result<MergedProject, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load(inputs.iter().map(|(source_id, name, source)| {
        DocumentInput::new(*source_id, DocumentOrigin::new(*name, root), source.clone())
    }))?;
    let interpolation = loaded.interpolate(environment);
    if !interpolation.is_valid() {
        let invalid = interpolation
            .documents()
            .iter()
            .flat_map(compose_lens::interpolation::DocumentInterpolation::values)
            .filter(|value| !value.is_valid())
            .map(compose_lens::interpolation::InterpolationResult::original)
            .collect::<Vec<_>>();
        return Err(format!(
            "application interpolation failed for {invalid:?}: {:#?}",
            interpolation.diagnostics()
        )
        .into());
    }
    let merge = merge_project(&loaded, Some(&interpolation));
    if !merge.is_valid() {
        return Err(format!("application merge failed: {:#?}", merge.diagnostics()).into());
    }
    merge.project().cloned().ok_or_else(|| "merged project missing".into())
}

fn scalar<'a>(project: &'a MergedProject, path: &[&str]) -> Result<&'a str, String> {
    let mut current = project.root();
    for segment in path {
        current = if let Ok(index) = segment.parse::<usize>() {
            current
                .as_sequence()
                .and_then(|values| values.get(index))
                .ok_or_else(|| format!("missing sequence path segment {segment} in {path:?}"))?
        } else {
            current
                .get(segment)
                .ok_or_else(|| format!("missing mapping path segment {segment} in {path:?}"))?
        };
    }
    current
        .as_scalar()
        .map(compose_lens::merge::MergedScalar::value)
        .ok_or_else(|| format!("expected scalar at {path:?}"))
}

fn mapping_len(project: &MergedProject, path: &[&str]) -> Result<usize, String> {
    project
        .value(path)
        .and_then(compose_lens::merge::MergedValue::as_mapping)
        .map(<[compose_lens::merge::MergedEntry]>::len)
        .ok_or_else(|| format!("expected mapping at {path:?}"))
}

fn operation(project: &MergedProject, path: &[&str]) -> Result<MergeOperation, String> {
    project
        .value(path)
        .map(|value| value.provenance().operation())
        .ok_or_else(|| format!("expected value at {path:?}"))
}

struct FixtureEnvironmentFiles {
    base: String,
    raw: String,
}

impl EnvironmentFileProvider for FixtureEnvironmentFiles {
    fn load(
        &self,
        request: &EnvironmentFileRequest<'_>,
    ) -> Result<Option<EnvironmentFileContent>, EnvironmentFileLoadError> {
        Ok(match request.path() {
            "./service.base.env" => Some(EnvironmentFileContent::plain(self.base.clone())),
            "./service.raw.env" => Some(EnvironmentFileContent::plain(self.raw.clone())),
            _ => None,
        })
    }
}

fn resolved_entry<'a>(
    resolution: &'a compose_lens::resolution::ServiceEnvironmentResolution,
    name: &str,
) -> Result<&'a compose_lens::resolution::ResolvedEnvironmentEntry, String> {
    resolution
        .entries()
        .iter()
        .find(|entry| entry.name() == name)
        .ok_or_else(|| format!("missing resolved environment {name}"))
}

fn assert_resolved(
    resolution: &compose_lens::resolution::ServiceEnvironmentResolution,
    name: &str,
    expected: Option<&str>,
    sensitive: bool,
) -> Result<(), String> {
    let entry = resolved_entry(resolution, name)?;
    match (entry.value(), expected) {
        (ResolvedEnvironmentValue::Value(value), Some(expected)) => {
            if value.value() != expected || value.is_sensitive() != sensitive {
                return Err(format!("unexpected resolved environment {name}: {value:?}"));
            }
        }
        (ResolvedEnvironmentValue::Unset, None) => {}
        (actual, expected) => {
            return Err(format!(
                "unexpected resolved environment state for {name}: {actual:?}, expected {expected:?}"
            ));
        }
    }
    Ok(())
}
