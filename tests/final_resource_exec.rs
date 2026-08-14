//! Regression coverage for the final closed Compose resource and develop-exec values.

use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::merge_project;
use compose_lens::model::{
    BooleanValue, Command, ComposeDocument, Environment, GPU_MISSING_CAPABILITIES, Labels, Located,
    RESOURCE_EXTERNAL_CREATION_CONFIGURATION, RESOURCE_EXTERNAL_LEGACY_DEPRECATED, RESOURCE_EXTERNAL_NAME_CONFLICT,
    ResourceExternal,
};
use compose_lens::project::build_project_view;
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;

#[test]
fn retains_resource_metadata_legacy_external_names_and_typed_develop_exec() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "configs:\n",
        "  app:\n",
        "    external: {name: legacy-config, x-note: retained}\n",
        "    name: modern-config\n",
        "    labels: [team=platform]\n",
        "    template_driver: golang\n",
        "secrets:\n",
        "  token:\n",
        "    external: true\n",
        "    driver: vault\n",
        "    driver_opts: {retries: 2, namespace: platform}\n",
        "    labels: {owner: platform}\n",
        "    template_driver: json\n",
        "services:\n",
        "  app:\n",
        "    develop:\n",
        "      watch:\n",
        "        - action: sync+exec\n",
        "          path: .\n",
        "          target: /src\n",
        "          exec:\n",
        "            command: [cargo, test]\n",
        "            user: app\n",
        "            privileged: ${DEV_PRIVILEGED:-false}\n",
        "            working_dir: /src\n",
        "            environment:\n              TOKEN: seeded-canary\n              MODE: dev\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(9901), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;

    let config = document.configs().first().ok_or("config expected")?;
    assert!(matches!(config.labels(), Some(Labels::List { values, .. }) if values.len() == 1));
    assert_eq!(
        config.template_driver().map(|value| value.value().as_str()),
        Some("golang")
    );
    assert!(matches!(config.external_syntax(), Some(ResourceExternal::Legacy(legacy)) if legacy.name().is_some()));
    let secret = document.secrets().first().ok_or("secret expected")?;
    assert_eq!(secret.driver().map(|value| value.value().as_str()), Some("vault"));
    assert!(!secret.driver_opts().is_empty());
    assert!(matches!(secret.labels(), Some(Labels::Map { entries, .. }) if entries.len() == 1));
    assert_eq!(
        secret.template_driver().map(|value| value.value().as_str()),
        Some("json")
    );
    assert!(matches!(
        secret.external().map(Located::value),
        Some(BooleanValue::Literal(true))
    ));

    let exec = document
        .service("app")
        .and_then(|service| service.develop())
        .and_then(|develop| develop.watch().first())
        .and_then(|watch| watch.exec())
        .ok_or("typed exec expected")?;
    assert!(matches!(exec.command(), Some(Command::List { values, .. }) if values.len() == 2));
    assert_eq!(exec.user().map(|value| value.value().as_str()), Some("app"));
    assert!(matches!(
        exec.privileged().map(Located::value),
        Some(BooleanValue::Expression(_))
    ));
    assert_eq!(exec.working_dir().map(|value| value.value().as_str()), Some("/src"));
    assert!(matches!(exec.environment(), Some(Environment::Map { entries, .. }) if entries.len() == 2));

    for code in [
        RESOURCE_EXTERNAL_LEGACY_DEPRECATED,
        RESOURCE_EXTERNAL_NAME_CONFLICT,
        RESOURCE_EXTERNAL_CREATION_CONFIGURATION,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    assert!(!format!("{document:?}").contains("seeded-canary"));
    Ok(())
}

#[test]
fn retains_invalid_legacy_and_exec_members_as_source_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "networks:\n  n:\n    external:\n      name: 4\n      future: true\n",
        "services:\n  app:\n    develop:\n      watch:\n        - action: sync+exec\n          path: .\n          target: /src\n          exec:\n            command: []\n            user: 4\n            environment: false\n            x-note: kept\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(9902), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;
    let network = document.networks().first().ok_or("network expected")?;
    assert!(matches!(network.external_syntax(), Some(ResourceExternal::Legacy(legacy)) if legacy.name().is_none()));
    let exec = document
        .service("app")
        .and_then(|service| service.develop())
        .and_then(|develop| develop.watch().first())
        .and_then(|watch| watch.exec())
        .ok_or("typed exec expected")?;
    assert!(matches!(exec.command(), Some(Command::List { values, .. }) if values.is_empty()));
    assert_eq!(exec.unmodeled_fields().len(), 3);
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code().as_str() == "compose.develop.watch.exec.missing-command")
    );
    Ok(())
}

#[test]
fn requires_non_empty_gpu_capabilities_at_authored_and_effective_boundaries() -> Result<(), Box<dyn std::error::Error>>
{
    let source = concat!(
        "services:\n",
        "  valid:\n    gpus: [{capabilities: [gpu]}]\n",
        "  absent:\n    gpus: [{driver: nvidia}]\n",
        "  empty:\n    gpus: [{capabilities: []}]\n",
        "  malformed:\n    gpus: [{capabilities: [false]}]\n",
        "  scalar:\n    gpus: [{capabilities: gpu}]\n",
        "  mapping:\n    gpus: [{capabilities: {kind: gpu}}]\n",
        "  reset:\n    gpus: [{capabilities: [gpu]}]\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(9903), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == GPU_MISSING_CAPABILITIES)
            .count(),
        5
    );
    let scalar = parsed
        .document()
        .and_then(|document| document.service("scalar"))
        .and_then(|service| service.gpus())
        .and_then(|gpus| match gpus {
            compose_lens::model::Gpus::Devices { devices, .. } => devices.first(),
            compose_lens::model::Gpus::All(_) => None,
        })
        .ok_or("scalar GPU selector expected")?;
    assert!(scalar.capabilities().is_empty());
    assert_eq!(scalar.unmodeled_fields().len(), 1);

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(9904),
            DocumentOrigin::new("base.yaml", "workspace"),
            source,
        ),
        DocumentInput::new(
            SourceId::new(9905),
            DocumentOrigin::new("override.yaml", "workspace"),
            "services:\n  reset:\n    gpus: !reset []\n  valid:\n    gpus: !override [{capabilities: [compute]}]\n",
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let effective = build_project_view(merged.project().ok_or("project expected")?, None);
    assert_eq!(
        effective
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == GPU_MISSING_CAPABILITIES)
            .count(),
        5
    );
    assert!(
        effective
            .view()
            .ok_or("view expected")?
            .service("scalar")
            .ok_or("scalar service expected")?
            .unmodeled_fields()
            .iter()
            .any(|field| field.path().iter().any(|segment| segment == "capabilities"))
    );
    assert!(
        effective
            .view()
            .ok_or("view expected")?
            .service("reset")
            .and_then(|service| service.gpus())
            .is_some_and(
                |gpus| matches!(gpus.value(), compose_lens::model::Gpus::Devices { devices, .. } if devices.is_empty())
            )
    );
    Ok(())
}

#[test]
fn keeps_invalid_legacy_external_name_evidence_in_the_effective_resource() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(9906),
            DocumentOrigin::new("base.yaml", "workspace"),
            "networks:\n  app:\n    external: {name: \"\", x-base: retained}\n",
        ),
        DocumentInput::new(
            SourceId::new(9907),
            DocumentOrigin::new("override.yaml", "workspace"),
            "networks:\n  app:\n    external: {name: 4, future: retained}\n",
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let network = result
        .view()
        .ok_or("view expected")?
        .networks()
        .first()
        .ok_or("network expected")?
        .definition()
        .value();
    let ResourceExternal::Legacy(legacy) = network.external_syntax().ok_or("legacy syntax expected")? else {
        return Err("legacy syntax expected".into());
    };
    assert!(legacy.name().is_none());
    assert!(
        legacy
            .unknown_fields()
            .iter()
            .any(|field| field.name().value() == "name")
    );
    assert!(
        legacy
            .unknown_fields()
            .iter()
            .any(|field| field.name().value() == "future")
    );
    assert!(
        legacy
            .extension_fields()
            .iter()
            .any(|field| field.name().value() == "x-base")
    );
    Ok(())
}
