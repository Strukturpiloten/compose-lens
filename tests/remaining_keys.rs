//! Regression coverage for the final closed-schema Compose keys.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::merge_project;
use compose_lens::model::{
    BooleanValue, ComposeDocument, DEVELOP_MISSING_WATCH, DEVELOP_WATCH_EXEC_MISSING_COMMAND,
    DEVELOP_WATCH_INVALID_ACTION, DEVELOP_WATCH_MISSING_ACTION, DEVELOP_WATCH_MISSING_EXEC, DEVELOP_WATCH_MISSING_PATH,
    DEVELOP_WATCH_MISSING_TARGET, GPU_COUNT_DEVICE_IDS_CONFLICT, GpuOptions, Gpus, IncludeItem, LabelFilesForm,
    Located, VERSION_OBSOLETE,
};
use compose_lens::project::{
    ProjectLabelFiles, ProjectService, ProjectValue, SERVICE_GPUS_DEPLOY_DEVICES_CONFLICT, build_project_view,
};
use compose_lens::resolution::{MISSING_REFERENCE, validate_references};
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;

const SOURCE: &str = concat!(
    "version: '3.9'\n",
    "include: [base.yaml, prod.yaml]\n",
    "models: {embedder: {model: local-model}}\n",
    "services:\n",
    "  app:\n",
    "    domainname: example.test\n",
    "    isolation: process\n",
    "    mac_address: 02:42:ac:11:00:02\n",
    "    uts: host\n",
    "    use_api_socket: '${API_SOCKET}'\n",
    "    label_file: [labels.common, labels.prod]\n",
    "    external_links: [external:alias]\n",
    "    links: [db:database]\n",
    "    storage_opt: {size: 1G}\n",
    "    models: {embedder: MODEL_URL}\n",
    "    gpus: all\n",
    "    develop: {watch: [{action: sync, path: ., target: /src}]}\n",
    "  invalid:\n",
    "    develop: {watch: [{path: .}]}\n",
);

#[test]
fn retains_authored_final_keys_without_io() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(9601), SOURCE)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;
    assert_eq!(document.version().map(Located::value), Some(&"3.9".to_owned()));
    assert_eq!(document.include().ok_or("includes expected")?.items().len(), 2);
    assert!(document.models().is_some());
    let app = document.service("app").ok_or("app expected")?;
    assert_eq!(app.domainname().map(Located::value), Some(&"example.test".to_owned()));
    assert!(matches!(
        app.label_files().map(compose_lens::model::LabelFiles::form),
        Some(LabelFilesForm::List(values)) if values.len() == 2
    ));
    assert_eq!(app.external_links().len(), 1);
    assert_eq!(app.links().len(), 1);
    assert!(app.storage_opt().is_some());
    assert!(app.models().is_some());
    assert!(matches!(app.gpus(), Some(Gpus::All(value)) if value.value() == "all"));
    assert!(
        matches!(app.use_api_socket().map(Located::value), Some(BooleanValue::Expression(value)) if value == "${API_SOCKET}")
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == VERSION_OBSOLETE)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DEVELOP_WATCH_MISSING_ACTION)
    );
    Ok(())
}

#[test]
fn project_view_keeps_effective_key_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(9602),
            DocumentOrigin::new("compose.yaml", "workspace"),
            SOURCE,
        ),
        DocumentInput::new(
            SourceId::new(9603),
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            "services:\n  app:\n    domainname: overridden.test\n    label_file: [labels.override]\n",
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("view expected")?;
    assert_eq!(view.include().ok_or("include expected")?.value().items().len(), 2);
    let app = view.service("app").ok_or("app expected")?;
    assert_eq!(
        ProjectService::domainname(app).map(ProjectValue::value),
        Some(&"overridden.test".to_owned())
    );
    assert!(matches!(
        ProjectService::label_files(app)
            .ok_or("label files expected")?
            .value()
            .form(),
        ProjectLabelFiles::List(values) if values.len() == 3
    ));
    assert!(ProjectService::develop(app).is_some());
    Ok(())
}

#[test]
fn malformed_develop_is_diagnosed_and_retained() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(9604),
        "services:\n  app:\n    develop: {}\n    gpus: [{driver: nvidia}]\n    label_file: labels.txt\n",
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let app = parsed
        .document()
        .and_then(|document| document.service("app"))
        .ok_or("app expected")?;
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DEVELOP_MISSING_WATCH)
    );
    assert!(matches!(app.gpus(), Some(Gpus::Devices { .. })));
    assert!(matches!(
        app.label_files().map(compose_lens::model::LabelFiles::form),
        Some(LabelFilesForm::Scalar(value)) if value.value() == "labels.txt"
    ));
    Ok(())
}

#[test]
fn reports_gpu_and_deploy_device_selectors_together_without_discarding_either() -> Result<(), Box<dyn std::error::Error>>
{
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9605),
        DocumentOrigin::new("compose.yaml", "workspace"),
        "services:\n  app:\n    gpus: all\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n",
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SERVICE_GPUS_DEPLOY_DEVICES_CONFLICT)
    );
    let service = result
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("service expected")?;
    assert!(service.gpus().is_some());
    assert!(
        service
            .deploy()
            .and_then(|deploy| deploy.value().resources())
            .and_then(|resources| resources.value().reservations())
            .and_then(|reservations| reservations.value().devices())
            .is_some()
    );
    Ok(())
}

#[test]
fn validates_structured_gpu_and_develop_forms_without_runtime_access() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(9606),
        concat!(
            "services:\n  app:\n",
            "    gpus:\n      - capabilities: gpu\n        count: 1\n        device_ids: [gpu0]\n        options: [mig]\n",
            "    develop:\n      watch:\n        - action: sync+exec\n          target: /src\n        - action: sync\n          path: .\n        - action: unknown\n          path: .\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let app = parsed
        .document()
        .and_then(|document| document.service("app"))
        .ok_or("app expected")?;
    let device = match app.gpus() {
        Some(Gpus::Devices { devices, .. }) => devices.first().ok_or("GPU selector expected")?,
        _ => return Err("GPU sequence expected".into()),
    };
    assert!(matches!(device.options(), Some(GpuOptions::List(items)) if items.len() == 1));
    for code in [
        GPU_COUNT_DEVICE_IDS_CONFLICT,
        DEVELOP_WATCH_MISSING_PATH,
        DEVELOP_WATCH_MISSING_EXEC,
        DEVELOP_WATCH_INVALID_ACTION,
        DEVELOP_WATCH_MISSING_TARGET,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn rejects_non_string_structured_members_and_requires_an_exec_command() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "include:\n  - true\n  - path: [base.yaml, false]\n    env_file: false\n    project_directory: false\n",
        "models:\n  valid:\n    model: local\n    name: false\n    context_size: false\n    runtime_flags: [fast, false]\n",
        "services:\n  app:\n",
        "    models: [false]\n",
        "    gpus:\n      - false\n      - capabilities: [gpu, false]\n        device_ids: [gpu0, 1]\n        count: true\n",
        "    develop:\n      watch:\n        - false\n        - action: sync+exec\n          path: .\n          target: /work\n          exec:\n            environment: {A: B}\n          ignore: [target, false]\n",
        "  bad_label: {label_file: false}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(9611), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;
    let includes = document.include().ok_or("includes expected")?;
    assert_eq!(includes.unmodeled_fields().len(), 1);
    let IncludeItem::Long(long) = &includes.items()[1] else {
        return Err("long malformed include expected".into());
    };
    assert_eq!(long.unmodeled_fields().len(), 3);
    let definition = document
        .models()
        .and_then(|models| models.definition("valid"))
        .ok_or("model definition expected")?;
    assert_eq!(definition.unmodeled_fields().len(), 3);
    let app = document.service("app").ok_or("service expected")?;
    assert!(!app.models().ok_or("models expected")?.unmodeled_fields().is_empty());
    let gpus = app.gpus().ok_or("GPU selectors expected")?;
    assert_eq!(gpus.unmodeled_items().len(), 1);
    let Gpus::Devices { devices, .. } = gpus else {
        return Err("GPU selector list expected".into());
    };
    assert_eq!(devices[0].unmodeled_fields().len(), 3);
    let develop = app.develop().ok_or("develop expected")?;
    assert_eq!(develop.unmodeled_items().len(), 1);
    assert_eq!(develop.watch()[0].unmodeled_fields().len(), 1);
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DEVELOP_WATCH_EXEC_MISSING_COMMAND)
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9612),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DEVELOP_WATCH_EXEC_MISSING_COMMAND)
    );
    let view = result.view().ok_or("view expected")?;
    assert!(!view.unmodeled_fields().is_empty());
    assert!(
        view.unmodeled_fields()
            .iter()
            .any(|field| field.path().first().is_some_and(|component| component == "include"))
    );
    assert!(
        view.unmodeled_fields()
            .iter()
            .any(|field| field.path().first().is_some_and(|component| component == "models"))
    );
    let effective_app = view.service("app").ok_or("service expected")?;
    for field_name in ["models", "gpus", "develop"] {
        assert!(
            effective_app
                .unmodeled_fields()
                .iter()
                .any(|field| field.path().iter().any(|component| component == field_name))
        );
    }
    assert!(
        view.service("bad_label")
            .ok_or("bad label service expected")?
            .unmodeled_fields()
            .iter()
            .any(|field| field.path().ends_with(&["label_file".to_owned()]))
    );
    Ok(())
}

#[test]
fn retains_long_include_models_and_model_references_without_loading_them() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "include:\n",
        "  - path: [base.yaml, optional.yaml]\n",
        "    env_file: local.env\n",
        "    project_directory: included\n",
        "  - path: prod.yaml\n",
        "    env_file: [common.env, prod.env]\n",
        "models:\n",
        "  embedder:\n",
        "    name: local-name\n",
        "    model: \"org/embedder\"\n",
        "    context_size: 4096\n",
        "    runtime_flags: [\"--fast\"]\n",
        "  flow: {model: \"org/flow\"}\n",
        "services:\n",
        "  found: {models: [embedder]}\n",
        "  missing: {models: {absent: {endpoint_var: MODEL_ENDPOINT, model_var: MODEL_NAME}}}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(9607), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;
    let includes = document.include().ok_or("includes expected")?;
    let IncludeItem::Long(long) = &includes.items()[0] else {
        return Err("long include expected".into());
    };
    assert_eq!(long.paths().len(), 2);
    assert_eq!(long.env_files()[0].value(), "local.env");
    assert_eq!(
        long.project_directory().map(Located::value),
        Some(&"included".to_owned())
    );
    let definition = document
        .models()
        .and_then(|models| models.definition("embedder"))
        .ok_or("model expected")?;
    assert_eq!(
        definition.model().map(Located::value),
        Some(&"org/embedder".to_owned()),
        "diagnostics: {:?}",
        parsed.diagnostics()
    );
    assert_eq!(definition.runtime_flags()[0].value(), "--fast");
    assert_eq!(
        document
            .models()
            .and_then(|models| models.definition("flow"))
            .and_then(compose_lens::model::ModelDefinition::model)
            .map(Located::value),
        Some(&"org/flow".to_owned())
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9608),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let references = validate_references(merged.project().ok_or("project expected")?, None);
    assert!(
        references
            .references()
            .iter()
            .all(|reference| reference.target() != "embedder")
    );
    assert!(
        references
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == MISSING_REFERENCE)
    );
    Ok(())
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one regression proves label-file authored forms plus merged reset and override provenance"
)]
fn retains_label_file_forms_and_effective_invalid_item_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  scalar: {label_file: labels.txt}\n",
        "  list: {label_file: [base.labels, false, later.labels]}\n",
        "  retained: {label_file: [base.labels, false, later.labels]}\n",
        "  bad: {label_file: false}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(9613), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;
    assert!(matches!(
        document
            .service("scalar")
            .and_then(compose_lens::model::Service::label_files)
            .map(compose_lens::model::LabelFiles::form),
        Some(LabelFilesForm::Scalar(value)) if value.value() == "labels.txt"
    ));
    let list = document
        .service("list")
        .and_then(compose_lens::model::Service::label_files)
        .ok_or("authored label-file list expected")?;
    assert!(matches!(list.form(), LabelFilesForm::List(values) if values.len() == 2));
    assert_eq!(list.unmodeled_items().len(), 1);
    assert!(
        document
            .service("bad")
            .ok_or("bad service expected")?
            .unknown_fields()
            .iter()
            .any(|field| field.name().value() == "label_file")
    );

    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(9614),
            DocumentOrigin::new("compose.yaml", "workspace"),
            source,
        ),
        DocumentInput::new(
            SourceId::new(9615),
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  scalar: {label_file: [override.labels]}\n",
                "  list: {label_file: !reset []}\n",
                "  reset: {label_file: [old.labels]}\n",
                "  override: {label_file: [old.labels]}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(9616),
            DocumentOrigin::new("compose.second-override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  reset: {label_file: !reset []}\n",
                "  override: {label_file: !override override.labels}\n",
            ),
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let view = result.view().ok_or("project view expected")?;
    assert!(matches!(
        view.service("scalar")
            .and_then(ProjectService::label_files)
            .map(ProjectValue::value)
            .map(compose_lens::project::ProjectServiceLabelFiles::form),
        Some(ProjectLabelFiles::List(values)) if values.len() == 1
    ));
    let list = view
        .service("list")
        .and_then(ProjectService::label_files)
        .ok_or("effective list expected")?;
    assert!(matches!(list.value().form(), ProjectLabelFiles::List(values) if values.is_empty()));
    assert_eq!(
        list.provenance().operation(),
        compose_lens::merge::MergeOperation::Reset
    );
    let reset = view
        .service("reset")
        .and_then(ProjectService::label_files)
        .ok_or("effective reset expected")?;
    assert!(matches!(reset.value().form(), ProjectLabelFiles::List(values) if values.is_empty()));
    assert_eq!(
        reset.provenance().operation(),
        compose_lens::merge::MergeOperation::Reset
    );
    let override_value = view
        .service("override")
        .and_then(ProjectService::label_files)
        .ok_or("effective override expected")?;
    assert!(matches!(
        override_value.value().form(),
        ProjectLabelFiles::Scalar(value) if value.value() == "override.labels"
    ));
    assert_eq!(
        override_value.provenance().operation(),
        compose_lens::merge::MergeOperation::Override
    );
    let invalid = view
        .service("retained")
        .and_then(ProjectService::label_files)
        .ok_or("effective invalid list expected")?;
    assert!(matches!(invalid.value().form(), ProjectLabelFiles::List(values) if values.len() == 2));
    assert_eq!(invalid.value().unmodeled_items().len(), 1);
    Ok(())
}

#[test]
fn retains_effective_merges_resets_overrides_and_explicit_interpolation() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(9609),
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "include:\n  - path: ${BASE_INCLUDE}\n    env_file: ${BASE_ENV}\n",
                "models:\n  model:\n    model: ${MODEL}\n    runtime_flags: [\"--base\"]\n",
                "services:\n  merged:\n    gpus:\n      - capabilities: [gpu]\n",
                "    develop:\n      watch:\n        - action: sync\n          path: ${WATCH_PATH}\n          target: /src\n",
                "  reset: {gpus: all, develop: {watch: [{action: restart, path: .}]}}\n",
                "  override: {gpus: all, develop: {watch: [{action: restart, path: .}]}}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(9610),
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "include: !reset []\nmodels: !reset {}\nservices:\n",
                "  merged:\n    gpus:\n      - driver: nvidia\n    develop:\n      watch:\n        - action: restart\n          path: override\n",
                "  reset:\n    gpus: !reset []\n    develop: !reset {}\n",
                "  override:\n    gpus: !override\n      - capabilities: [custom]\n    develop: !override\n      watch:\n        - action: rebuild\n          path: .\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    environment.insert("BASE_INCLUDE", "base.yaml");
    environment.insert("BASE_ENV", "base.env");
    environment.insert("MODEL", "org/model");
    environment.insert("WATCH_PATH", "src");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let view = result.view().ok_or("view expected")?;
    assert_eq!(view.include().map(|value| value.value().items().len()), Some(0));
    assert_eq!(view.models().map(|value| value.value().definitions().len()), Some(0));
    assert!(
        matches!(view.service("merged").and_then(ProjectService::gpus).map(ProjectValue::value), Some(Gpus::Devices { devices, .. }) if devices.len() == 2)
    );
    assert!(
        matches!(view.service("reset").and_then(ProjectService::gpus).map(ProjectValue::value), Some(Gpus::Devices { devices, .. }) if devices.is_empty())
    );
    assert!(
        matches!(view.service("override").and_then(ProjectService::gpus).map(ProjectValue::value), Some(Gpus::Devices { devices, .. }) if devices.len() == 1)
    );
    Ok(())
}
