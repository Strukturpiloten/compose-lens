//! Regression coverage for the remaining long-syntax service-volume option blocks.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::{MergeOperation, merge_project};
use compose_lens::model::{
    BooleanValue, ComposeDocument, ComposeScalar, DUPLICATE_FIELD, EXPECTED_FIELD_FORM, EXPECTED_SCALAR, Labels,
    Located, VOLUME_OPTION_EXPECTED_MAPPING, VolumeMount,
};
use compose_lens::project::{ProjectService, ProjectValue, build_project_view};
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;

#[test]
fn authored_long_mount_options_retain_every_supported_shape_and_source_form() -> Result<(), Box<dyn std::error::Error>>
{
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    volumes:\n",
        "      - type: image\n",
        "        source: example/app\n",
        "        target: /image\n",
        "        consistency: delegated\n",
        "        image:\n          subpath: \"${IMAGE_SUBPATH:-assets}\"\n          x-extra: retained\n          future: retained\n",
        "      - type: tmpfs\n",
        "        target: /run/cache\n",
        "        tmpfs:\n          size: 64m\n          mode: \"01777\"\n          x-extra: retained\n          future: retained\n",
        "      - type: volume\n",
        "        source: data\n",
        "        target: /data\n",
        "        volume:\n",
        "          nocopy: \"${NOCOPY:-true}\"\n",
        "          subpath: data\n",
        "          labels: [com.example.role=data]\n",
        "          x-extra: retained\n",
        "          future: retained\n",
        "      - type: bind\n",
        "        source: ./host\n",
        "        target: /host\n",
        "        bind: {recursive: readonly}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(9_401), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let service = parsed
        .document()
        .and_then(|document| document.service("app"))
        .ok_or("service missing")?;
    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());

    let VolumeMount::Long(image) = &service.volumes()[0] else {
        return Err("image mount missing".into());
    };
    assert_eq!(
        image.consistency().map(Located::value).map(String::as_str),
        Some("delegated")
    );
    let image_options = image.image().ok_or("image options missing")?;
    assert_eq!(
        image_options.subpath().map(Located::value).map(String::as_str),
        Some("${IMAGE_SUBPATH:-assets}")
    );
    assert_eq!(image_options.extension_fields().len(), 1);
    assert_eq!(image_options.unknown_fields().len(), 1);

    let VolumeMount::Long(tmpfs) = &service.volumes()[1] else {
        return Err("tmpfs mount missing".into());
    };
    let tmpfs_options = tmpfs.tmpfs().ok_or("tmpfs options missing")?;
    assert!(matches!(tmpfs_options.size().map(Located::value), Some(ComposeScalar::String(value)) if value == "64m"));
    assert!(matches!(tmpfs_options.mode().map(Located::value), Some(ComposeScalar::String(value)) if value == "01777"));
    assert_eq!(tmpfs_options.extension_fields().len(), 1);
    assert_eq!(tmpfs_options.unknown_fields().len(), 1);

    let VolumeMount::Long(volume) = &service.volumes()[2] else {
        return Err("volume mount missing".into());
    };
    let volume_options = volume.volume().ok_or("named-volume options missing")?;
    assert!(
        matches!(volume_options.nocopy().map(Located::value), Some(BooleanValue::Expression(value)) if value == "${NOCOPY:-true}")
    );
    assert_eq!(
        volume_options.subpath().map(Located::value).map(String::as_str),
        Some("data")
    );
    assert!(
        matches!(volume_options.labels(), Some(Labels::List { values, .. }) if values[0].value() == "com.example.role=data")
    );
    assert_eq!(volume_options.extension_fields().len(), 1);
    assert_eq!(volume_options.unknown_fields().len(), 1);

    let VolumeMount::Long(bind) = &service.volumes()[3] else {
        return Err("bind mount missing".into());
    };
    assert_eq!(
        bind.bind()
            .and_then(|options| options.recursive())
            .map(Located::value)
            .map(String::as_str),
        Some("readonly")
    );
    Ok(())
}

#[test]
fn malformed_duplicate_and_wrong_shape_mount_options_keep_source_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n  app:\n    volumes:\n",
        "      - type: bind\n        target: /data\n        consistency: [wrong]\n        bind:\n          recursive: [wrong]\n          recursive: enabled\n",
        "      - type: image\n        target: /image\n        image: wrong\n",
        "      - type: tmpfs\n        target: /tmp\n        tmpfs:\n          size: false\n          mode: {wrong: shape}\n",
        "      - type: volume\n        target: /volume\n        volume:\n          nocopy: falseish\n          subpath: [wrong]\n          labels: false\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(9_402), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let service = parsed
        .document()
        .and_then(|document| document.service("app"))
        .ok_or("service missing")?;
    assert!(!parsed.is_valid());
    for code in [
        DUPLICATE_FIELD,
        EXPECTED_FIELD_FORM,
        EXPECTED_SCALAR,
        VOLUME_OPTION_EXPECTED_MAPPING,
    ] {
        assert!(
            parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code),
            "missing {code}"
        );
    }
    let VolumeMount::Long(bind) = &service.volumes()[0] else {
        return Err("bind mount missing".into());
    };
    assert_eq!(bind.bind().map(|value| value.unknown_fields().len()), Some(1));
    let VolumeMount::Long(image) = &service.volumes()[1] else {
        return Err("image mount missing".into());
    };
    assert!(image.image().is_none());
    assert_eq!(image.unknown_fields().len(), 1);
    let VolumeMount::Long(tmpfs) = &service.volumes()[2] else {
        return Err("tmpfs mount missing".into());
    };
    let options = tmpfs.tmpfs().ok_or("tmpfs options missing")?;
    assert_eq!(options.unknown_fields().len(), 2);
    let VolumeMount::Long(volume) = &service.volumes()[3] else {
        return Err("volume mount missing".into());
    };
    let options = volume.volume().ok_or("volume options missing")?;
    assert_eq!(options.unknown_fields().len(), 3);

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9_407),
        DocumentOrigin::new("compose.yaml", "workspace"),
        "services:\n  app:\n    volumes:\n      - type: bind\n        target: /data\n        consistency: [wrong]\n",
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("merged project missing")?, None);
    let options = result
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::volume_mount_options)
        .ok_or("effective mount option evidence missing")?;
    assert!(options.value()[0].value().unmodeled_fields().iter().any(|field| {
        field.key().value() == "consistency"
            && field
                .path()
                .ends_with(&["volumes".to_owned(), "0".to_owned(), "consistency".to_owned()])
    }));
    Ok(())
}

#[test]
fn effective_mount_options_preserve_merge_reset_override_and_interpolation_provenance()
-> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(9_403);
    let override_id = SourceId::new(9_404);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n",
                "  retained:\n    volumes:\n      - type: volume\n        source: data\n        target: /data\n        consistency: \"${CONSISTENCY:-delegated}\"\n        volume: {nocopy: \"${NOCOPY:-true}\", subpath: base, labels: {com.example.enabled: true}}\n",
                "  reset:\n    volumes:\n      - type: tmpfs\n        target: /tmp\n        tmpfs: {size: 64m, mode: \"01777\"}\n",
                "  overridden:\n    volumes:\n      - type: image\n        source: image\n        target: /image\n        consistency: delegated\n        image: {subpath: base}\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n",
                "  reset:\n    volumes: !reset []\n",
                "  overridden:\n    volumes: !override [{type: bind, source: ./host, target: /host, consistency: cached, bind: {recursive: enabled}}]\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("CONSISTENCY", "cached");
    let _ = environment.insert_sensitive("NOCOPY", "false");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("merged project missing")?, None);
    let view = result.view().ok_or("project view missing")?;

    let retained = view
        .service("retained")
        .ok_or("retained service missing")?
        .volumes()
        .ok_or("volumes missing")?;
    assert_eq!(retained.provenance().operation(), MergeOperation::Authored);
    let mount = retained.value()[0].value();
    let VolumeMount::Long(mount) = mount else {
        return Err("retained mount missing".into());
    };
    assert_eq!(
        mount.consistency().map(Located::value).map(String::as_str),
        Some("cached")
    );
    let options = mount.volume().ok_or("volume options missing")?;
    assert!(matches!(
        options.nocopy().map(Located::value),
        Some(BooleanValue::Literal(false))
    ));
    assert!(
        matches!(options.labels(), Some(Labels::Map { entries, .. }) if matches!(entries[0].value().value(), ComposeScalar::Boolean(true)))
    );

    let reset = view.service("reset").ok_or("reset service missing")?;
    assert!(matches!(reset.volumes().map(ProjectValue::value), Some(values) if values.is_empty()));
    assert_eq!(
        reset
            .volumes()
            .map(ProjectValue::provenance)
            .map(compose_lens::merge::MergeProvenance::operation),
        Some(MergeOperation::Reset)
    );

    let overridden = view
        .service("overridden")
        .ok_or("overridden service missing")?
        .volumes()
        .ok_or("volumes missing")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    let VolumeMount::Long(mount) = overridden.value()[0].value() else {
        return Err("override mount missing".into());
    };
    assert_eq!(
        mount
            .bind()
            .and_then(|value| value.recursive())
            .map(Located::value)
            .map(String::as_str),
        Some("enabled")
    );
    assert_effective_option_companion_views(view)?;
    Ok(())
}

fn assert_effective_option_companion_views(
    view: &compose_lens::project::ProjectView,
) -> Result<(), Box<dyn std::error::Error>> {
    let retained = view
        .service("retained")
        .and_then(ProjectService::volume_mount_options)
        .ok_or("effective volume option evidence missing")?;
    let named = retained.value()[0]
        .value()
        .volume()
        .ok_or("named-volume evidence missing")?;
    assert!(matches!(
        named.nocopy().map(ProjectValue::value),
        Some(BooleanValue::Literal(false))
    ));
    assert!(named.nocopy().is_some_and(ProjectValue::is_sensitive));

    let reset = view.service("reset").ok_or("reset service missing")?;
    assert!(matches!(reset.volume_mount_options().map(ProjectValue::value), Some(values) if values.is_empty()));

    let overridden = view
        .service("overridden")
        .and_then(ProjectService::volume_mount_options)
        .ok_or("override option evidence missing")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        overridden.value()[0]
            .value()
            .bind()
            .and_then(|value| value.recursive())
            .map(ProjectValue::value)
            .map(String::as_str),
        Some("enabled")
    );
    let consistency = overridden.value()[0].value().consistency();
    assert_eq!(consistency.map(ProjectValue::value).map(String::as_str), Some("cached"));
    assert_eq!(
        consistency
            .map(ProjectValue::provenance)
            .map(compose_lens::merge::MergeProvenance::operation),
        Some(MergeOperation::Authored)
    );
    assert_eq!(provenance_source_ids(consistency), Some(vec![SourceId::new(9_404)]));
    Ok(())
}

#[test]
fn effective_nested_mount_options_keep_contributors_extensions_and_malformed_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let base_id = SourceId::new(9_405);
    let override_id = SourceId::new(9_406);
    let loaded = LoadedProject::load([
        DocumentInput::new(
            base_id,
            DocumentOrigin::new("compose.yaml", "workspace"),
            concat!(
                "services:\n  app:\n    volumes:\n",
                "      - type: image\n        source: image\n        target: /image\n        consistency: delegated\n        image:\n          subpath: base\n          x-origin: retained\n          future: retained\n",
                "      - type: tmpfs\n        target: /tmp\n        tmpfs:\n          size: false\n          x-origin: retained\n          future: retained\n",
                "      - type: volume\n        source: data\n        target: /data\n        volume:\n          subpath: base\n          labels: false\n          x-origin: retained\n          future: retained\n",
            ),
        ),
        DocumentInput::new(
            override_id,
            DocumentOrigin::new("compose.override.yaml", "workspace"),
            concat!(
                "services:\n  app:\n    volumes:\n",
                "      - target: /image\n        consistency: cached\n        image:\n          subpath: override\n          x-override: retained\n",
                "      - target: /tmp\n        tmpfs:\n          mode: \"01777\"\n          x-override: retained\n",
                "      - target: /data\n        volume:\n          nocopy: true\n          x-override: retained\n",
            ),
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("merged project missing")?, None);
    let view = result.view().ok_or("project view missing")?;
    let options = view
        .service("app")
        .and_then(ProjectService::volume_mount_options)
        .ok_or("effective mount option evidence missing")?;
    assert_eq!(options.value().len(), 3);

    assert_effective_image_mount_options(options.value()[0].value(), base_id, override_id)?;

    let tmpfs = options.value()[1].value().tmpfs().ok_or("tmpfs evidence missing")?;
    assert!(matches!(tmpfs.mode().map(ProjectValue::value), Some(ComposeScalar::String(value)) if value == "01777"));
    assert_eq!(provenance_source_ids(tmpfs.mode()), Some(vec![override_id]));
    assert_eq!(tmpfs.extension_fields().len(), 2);
    assert_eq!(tmpfs.unknown_fields().len(), 2);
    assert!(tmpfs.unknown_fields().iter().any(|field| field.key().value() == "size"));
    assert!(
        tmpfs
            .unknown_fields()
            .iter()
            .any(|field| field.key().value() == "future")
    );

    let volume = options.value()[2]
        .value()
        .volume()
        .ok_or("named-volume evidence missing")?;
    assert!(matches!(
        volume.nocopy().map(ProjectValue::value),
        Some(BooleanValue::Literal(true))
    ));
    assert_eq!(provenance_source_ids(volume.nocopy()), Some(vec![override_id]));
    assert_eq!(
        volume.subpath().map(ProjectValue::value).map(String::as_str),
        Some("base")
    );
    assert_eq!(volume.extension_fields().len(), 2);
    assert_eq!(volume.unknown_fields().len(), 2);
    assert!(
        volume
            .unknown_fields()
            .iter()
            .any(|field| field.key().value() == "labels")
    );
    assert!(
        volume
            .unknown_fields()
            .iter()
            .any(|field| field.key().value() == "future")
    );
    Ok(())
}

fn assert_effective_image_mount_options(
    mount: &compose_lens::project::ProjectVolumeMountOptions,
    base_id: SourceId,
    override_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        mount.consistency().map(ProjectValue::value).map(String::as_str),
        Some("cached")
    );
    assert_eq!(
        provenance_source_ids(mount.consistency()),
        Some(vec![base_id, override_id])
    );
    let image = mount.image().ok_or("image evidence missing")?;
    assert_eq!(
        image.subpath().map(ProjectValue::value).map(String::as_str),
        Some("override")
    );
    assert_eq!(provenance_source_ids(image.subpath()), Some(vec![base_id, override_id]));
    assert_eq!(image.extension_fields().len(), 2);
    assert_eq!(image.unknown_fields().len(), 1);
    assert!(
        image
            .extension_fields()
            .iter()
            .all(compose_lens::project::ProjectFieldReference::is_extension)
    );
    assert!(image.unknown_fields().iter().all(|field| !field.is_extension()));
    assert!(
        image.unknown_fields()[0]
            .path()
            .ends_with(&["image".to_owned(), "future".to_owned()])
    );
    Ok(())
}

fn provenance_source_ids<T>(value: Option<&ProjectValue<T>>) -> Option<Vec<SourceId>> {
    value.map(|value| {
        value
            .provenance()
            .sources()
            .iter()
            .map(|span| span.source_id())
            .collect()
    })
}

#[test]
fn mount_option_public_types_are_usable_without_generation_or_runtime_access() {
    fn inspect(mount: &VolumeMount) -> bool {
        match mount {
            VolumeMount::Short(_) => false,
            VolumeMount::Long(value) => {
                let _ = value.consistency();
                let _ = value.bind().and_then(|value| value.recursive());
                let _ = value.image().and_then(|value| value.subpath());
                let _ = value.tmpfs().and_then(|value| value.size());
                let _ = value.tmpfs().and_then(|value| value.mode());
                let _ = value.volume().and_then(|value| value.nocopy());
                let _ = value.volume().and_then(|value| value.subpath());
                let _ = value.volume().and_then(|value| value.labels());
                true
            }
        }
    }
    let _ = inspect;
}
