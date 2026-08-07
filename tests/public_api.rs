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
    ProjectDevice, ProjectDns, ProjectDnsSearch, ProjectEnvironmentFile, ProjectGrant, ProjectLogging,
    ProjectLoggingOption, ProjectLoggingOptionValue, ProjectLoggingOptions, ProjectSysctls, ProjectTmpfs,
    ProjectUlimit, ProjectUlimitRange, ProjectUlimitScalar, ProjectUlimitValue, ProjectUlimits, build_project_view,
};
use compose_lens::render::{
    ComposeDocumentBuilder, GeneratedAnnotation, GeneratedDevice, GeneratedDns, GeneratedDnsSearch,
    GeneratedEntrypoint, GeneratedEnvironmentFile, GeneratedHostname, GeneratedLabel, GeneratedLogging,
    GeneratedLoggingOption, GeneratedLoggingOptionValue, GeneratedLongDevice, GeneratedMemLimit,
    GeneratedNetworkAttachment, GeneratedNetworkDefinition, GeneratedNetworkDriverOption,
    GeneratedNetworkDriverOptionValue, GeneratedPidsLimit, GeneratedPullPolicy, GeneratedRestartPolicy,
    GeneratedService, GeneratedShmSize, GeneratedString, GeneratedSysctl, GeneratedSysctls, GeneratedTmpfs,
    GeneratedUlimit, GeneratedUlimitValue, GeneratedUlimits, GeneratedVolumeDefinition, GeneratedVolumeDriverOption,
    GeneratedVolumeDriverOptionValue, ReplacementScalar, ScalarEdit, apply_preservation_edits, render_canonical,
};

#[test]
fn exposes_authored_effective_and_generated_annotations_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n    annotations: [\"io.example.owner=platform\", \"io.example.key-only\"]\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(700), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        parsed
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::annotations)
            .map(compose_lens::model::Annotations::form),
        Some(compose_lens::model::AnnotationsForm::List(items)) if items.len() == 2
    ));

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(701),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let annotations = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::annotations)
        .ok_or("effective annotations expected")?;
    let owner: &compose_lens::project::ProjectAnnotationEntry = annotations
        .value()
        .get("io.example.owner")
        .ok_or("owner annotation expected")?;
    assert!(owner.value().is_some());
    assert!(
        annotations
            .value()
            .get("io.example.key-only")
            .is_some_and(|entry| entry.value().is_none())
    );

    let mut service = GeneratedService::new("app")?;
    service.set_annotations(vec![GeneratedAnnotation::new(
        "io.example.owner",
        GeneratedString::plain("platform")?,
    )?])?;
    assert_eq!(service.annotations().map(<[GeneratedAnnotation]>::len), Some(1));
    Ok(())
}

#[test]
fn exposes_generated_volume_definition_driver_options_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let mut volume = GeneratedVolumeDefinition::application("data")?;
    volume.set_driver(GeneratedString::plain("opaque-driver")?)?;
    volume.set_driver_opts(vec![
        GeneratedVolumeDriverOption::new(
            "quoted",
            GeneratedVolumeDriverOptionValue::String(GeneratedString::plain("2")?),
        )?,
        GeneratedVolumeDriverOption::new(
            "number",
            GeneratedVolumeDriverOptionValue::Number(GeneratedString::plain("2")?),
        )?,
    ])?;
    volume.set_labels(vec![GeneratedLabel::new(
        "com.example.owner",
        GeneratedString::plain("strukturpiloten")?,
    )?])?;
    assert_eq!(volume.driver().map(GeneratedString::expose), Some("opaque-driver"));
    assert_eq!(volume.driver_opts().map(<[GeneratedVolumeDriverOption]>::len), Some(2));
    assert_eq!(volume.labels().map(<[GeneratedLabel]>::len), Some(1));

    let mut builder = ComposeDocumentBuilder::new();
    let mut service = GeneratedService::new("app")?;
    service.set_image(GeneratedString::plain("example/app")?)?;
    builder.add_service(service)?;
    builder.add_volume_definition(volume)?;
    let generated = builder.build(SourceId::new(820))?;
    let volume = generated
        .document()
        .volumes()
        .first()
        .ok_or("generated volume expected")?;
    assert!(matches!(
        volume.driver_opts()[0].value().value(),
        compose_lens::model::ComposeScalar::String(value) if value == "2"
    ));
    assert!(matches!(
        volume.driver_opts()[1].value().value(),
        compose_lens::model::ComposeScalar::Number(value) if value == "2"
    ));
    assert!(matches!(
        volume.labels(),
        Some(compose_lens::model::Labels::Map { entries, .. })
            if entries[0].key().value() == "com.example.owner"
                && matches!(entries[0].value().value(), compose_lens::model::ComposeScalar::String(value) if value == "strukturpiloten")
    ));
    Ok(())
}

#[test]
fn exposes_authored_effective_and_generated_logging_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    logging:\n",
        "      driver: custom\n",
        "      options:\n",
        "        text: \"01\"\n",
        "        count: 2\n",
        "        empty: null\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(816), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored: &compose_lens::model::Logging = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::logging)
        .ok_or("authored logging expected")?;
    let authored_options: &compose_lens::model::LoggingOptions =
        authored.options().ok_or("authored options expected")?;
    let _: &compose_lens::model::LoggingOption = &authored_options.entries()[0];
    assert!(
        matches!(authored_options.entries()[0].value().value(), compose_lens::model::LoggingOptionValue::String(value) if value == "01")
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(817),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let logging: &ProjectLogging = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::logging)
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective logging expected")?;
    let options: &ProjectLoggingOptions = logging
        .options()
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective options expected")?;
    let option: &ProjectLoggingOption = options.entries()[0].value();
    assert!(matches!(option.value().value(), ProjectLoggingOptionValue::String { value, .. } if value == "01"));

    let generated_option = GeneratedLoggingOption::new("empty", GeneratedLoggingOptionValue::Null)?;
    let generated = GeneratedLogging::new(GeneratedString::plain("custom")?, vec![generated_option])?;
    let mut service = GeneratedService::new("app")?;
    service.set_logging(generated)?;
    assert_eq!(
        service.logging().map(GeneratedLogging::options).map(<[_]>::len),
        Some(1)
    );
    Ok(())
}

#[test]
fn exposes_generated_network_definition_driver_options_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let mut network = GeneratedNetworkDefinition::application("frontend")?;
    network.set_driver(GeneratedString::plain("opaque-driver")?)?;
    network.set_driver_opts(vec![
        GeneratedNetworkDriverOption::new(
            "quoted",
            GeneratedNetworkDriverOptionValue::String(GeneratedString::plain("2")?),
        )?,
        GeneratedNetworkDriverOption::new(
            "number",
            GeneratedNetworkDriverOptionValue::Number(GeneratedString::plain("2")?),
        )?,
    ])?;
    network.set_enable_ipv6(false)?;
    network.set_internal(true)?;
    network.set_labels(vec![GeneratedLabel::new(
        "com.example.owner",
        GeneratedString::plain("strukturpiloten")?,
    )?])?;
    assert_eq!(network.driver().map(GeneratedString::expose), Some("opaque-driver"));
    assert_eq!(
        network.driver_opts().map(<[GeneratedNetworkDriverOption]>::len),
        Some(2)
    );
    assert_eq!(network.enable_ipv6(), Some(false));
    assert_eq!(network.internal(), Some(true));
    assert_eq!(network.labels().map(<[GeneratedLabel]>::len), Some(1));

    let mut service = GeneratedService::new("app")?;
    service.add_network(GeneratedNetworkAttachment::new("frontend")?)?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(service)?;
    builder.add_network_definition(network)?;
    let generated = builder.build(SourceId::new(819))?;
    let network = generated
        .document()
        .networks()
        .first()
        .ok_or("generated network expected")?;
    assert!(matches!(
        network.driver_opts()[0].value().value(),
        compose_lens::model::ComposeScalar::String(value) if value == "2"
    ));
    assert!(matches!(
        network.driver_opts()[1].value().value(),
        compose_lens::model::ComposeScalar::Number(value) if value == "2"
    ));
    assert!(matches!(
        network.labels(),
        Some(compose_lens::model::Labels::Map { entries, .. })
            if entries[0].key().value() == "com.example.owner"
                && matches!(entries[0].value().value(), compose_lens::model::ComposeScalar::String(value) if value == "strukturpiloten")
    ));
    assert_eq!(
        network.enable_ipv6().map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(
        network.internal().map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(true))
    );
    Ok(())
}

#[test]
fn exposes_repeatable_unmask_security_option_contract() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n    security_opt: [\"unmask=ALL\", \"${UNMASK}\", \"unmask=all\"]\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(736), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("authored unmask security options expected")?;
    assert!(matches!(
        authored.items()[0].kind(),
        compose_lens::model::SecurityOptionKind::Unmask { paths } if paths == "ALL"
    ));
    assert!(matches!(
        authored.items()[1].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert_eq!(
        parsed.diagnostics()[0].code(),
        compose_lens::model::SECURITY_OPT_UNMASK_NEAR_MISS
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(737),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("UNMASK", "unmask=/proc/acpi:/sys/firmware");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let effective: &compose_lens::project::ProjectSecurityOptionItem = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(1))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective unmask option expected")?;
    assert!(matches!(
        effective.kind(),
        compose_lens::model::SecurityOptionKind::Unmask { paths }
            if paths == "/proc/acpi:/sys/firmware"
    ));

    let mut service = GeneratedService::new("app")?;
    service.set_security_options(vec![
        GeneratedString::plain("unmask=ALL")?,
        GeneratedString::plain("unmask=ALL")?,
        GeneratedString::plain("unmask=/proc/*")?,
    ])?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(service)?;
    let generated = builder.build(SourceId::new(738))?;
    let generated_options = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("generated unmask options expected")?;
    assert_eq!(generated_options.items().len(), 3);
    assert!(matches!(
        generated_options.items()[2].kind(),
        compose_lens::model::SecurityOptionKind::Unmask { paths } if paths == "/proc/*"
    ));
    Ok(())
}
use compose_lens::resolution::validate_references;
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;
use compose_lens::validation::{
    CompatibilityFeature, CompatibilityProfile, ImplementationVersion, validate_compatibility,
};

fn assert_generated_seccomp_public_contract(options: &compose_lens::model::SecurityOptions) {
    assert_eq!(options.items().len(), 21);
    for index in [1, 2] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::SecurityLabelDisable { enabled: true }
        ));
    }
    for index in [3, 4] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::SecurityLabelFileType { file_type }
                if file_type == "container_file_t"
        ));
    }
    for index in [5, 6] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::SecurityLabelLevel { level }
                if level == "s0:c1,c2"
        ));
    }
    for index in [7, 8] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::SecurityLabelNested { enabled: true }
        ));
    }
    for index in [9, 10] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::SecurityLabelType { label_type }
                if label_type == "container_t"
        ));
    }
    for index in [11, 12] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::Mask { paths }
                if paths == "/proc/acpi:/proc/kcore"
        ));
    }
    for (index, profile) in [(14, "unconfined"), (15, "/workspace/seccomp.json"), (16, "unconfined")] {
        assert!(matches!(
            options.items()[index].kind(),
            compose_lens::model::SecurityOptionKind::Seccomp { profile: actual } if actual == profile
        ));
    }
}

fn assert_generated_security_option_contract() -> Result<(), Box<dyn std::error::Error>> {
    let mut service = GeneratedService::new("app")?;
    service.set_security_options(vec![
        GeneratedString::plain("apparmor=public-api")?,
        GeneratedString::plain("label:disable")?,
        GeneratedString::plain("label:disable")?,
        GeneratedString::plain("label:filetype:container_file_t")?,
        GeneratedString::plain("label:filetype:container_file_t")?,
        GeneratedString::plain("label:level:s0:c1,c2")?,
        GeneratedString::plain("label:level:s0:c1,c2")?,
        GeneratedString::plain("label:nested")?,
        GeneratedString::plain("label:nested")?,
        GeneratedString::plain("label:type:container_t")?,
        GeneratedString::plain("label:type:container_t")?,
        GeneratedString::plain("mask=/proc/acpi:/proc/kcore")?,
        GeneratedString::plain("mask=/proc/acpi:/proc/kcore")?,
        GeneratedString::plain("no-new-privileges:true")?,
        GeneratedString::plain("seccomp=unconfined")?,
        GeneratedString::plain("seccomp=/workspace/seccomp.json")?,
        GeneratedString::plain("seccomp=unconfined")?,
        GeneratedString::plain("no-new-privileges:false")?,
        GeneratedString::plain("no-new-privileges:true")?,
        GeneratedString::plain("label:user:USER")?,
        GeneratedString::plain("label=disable")?,
    ])?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(service)?;
    let generated = builder.build(SourceId::new(704))?;
    let generated_options = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("generated security options expected")?;
    assert_generated_seccomp_public_contract(generated_options);
    Ok(())
}

#[test]
fn exposes_authored_effective_and_generated_security_option_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n    security_opt: [\"apparmor=${PROFILE}\", \"label:disable\", \"label:disable\", \"${NNP}\", \"${SECCOMP}\", \"${LABEL_DISABLE}\", \"${LABEL_FILETYPE}\", \"${LABEL_LEVEL}\", \"${LABEL_NESTED}\", \"${LABEL_TYPE}\", \"${MASK}\"]\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(702), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored: &compose_lens::model::SecurityOptions = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("authored security options expected")?;
    assert_authored_security_option_contract(authored);

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(703),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("PROFILE", "public-api");
    let _ = environment.insert("NNP", "no-new-privileges:false");
    let _ = environment.insert("SECCOMP", "seccomp=/workspace/seccomp.json");
    let _ = environment.insert("LABEL_DISABLE", "label:disable");
    let _ = environment.insert("LABEL_FILETYPE", "label:filetype:container_file_t");
    let _ = environment.insert("LABEL_LEVEL", "label:level:s0:c1,c2");
    let _ = environment.insert("LABEL_NESTED", "label:nested");
    let _ = environment.insert("LABEL_TYPE", "label:type:container_t");
    let _ = environment.insert("MASK", "mask=/proc/acpi:/proc/kcore");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let effective = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let item: &compose_lens::project::ProjectSecurityOptionItem = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().first())
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective security option expected")?;
    assert_eq!(item.authored(), "\"apparmor=${PROFILE}\"");
    assert_eq!(item.value(), "apparmor=public-api");
    let no_new_privileges = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(3))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective no-new-privileges option expected")?;
    assert!(matches!(
        no_new_privileges.kind(),
        compose_lens::model::SecurityOptionKind::NoNewPrivileges { enabled: false }
    ));
    let seccomp = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(4))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective seccomp option expected")?;
    assert!(matches!(
        seccomp.kind(),
        compose_lens::model::SecurityOptionKind::Seccomp { profile } if profile == "/workspace/seccomp.json"
    ));
    let label_disable = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(5))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective SELinux label-disable option expected")?;
    assert!(matches!(
        label_disable.kind(),
        compose_lens::model::SecurityOptionKind::SecurityLabelDisable { enabled: true }
    ));
    assert_effective_label_filetype_contract(&effective)?;
    assert_effective_label_level_contract(&effective)?;
    assert_effective_label_nested_contract(&effective)?;
    assert_effective_label_type_contract(&effective)?;
    assert_effective_mask_contract(&effective)?;

    assert_generated_security_option_contract()
}

fn assert_authored_security_option_contract(authored: &compose_lens::model::SecurityOptions) {
    assert!(matches!(
        authored.items()[0].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[3].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[4].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[1].kind(),
        compose_lens::model::SecurityOptionKind::SecurityLabelDisable { enabled: true }
    ));
    assert!(matches!(
        authored.items()[5].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[6].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[7].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[8].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[9].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
    assert!(matches!(
        authored.items()[10].kind(),
        compose_lens::model::SecurityOptionKind::Expression
    ));
}

fn assert_effective_mask_contract(
    effective: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let mask = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(10))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective mask option expected")?;
    assert!(matches!(
        mask.kind(),
        compose_lens::model::SecurityOptionKind::Mask { paths }
            if paths == "/proc/acpi:/proc/kcore"
    ));
    Ok(())
}

fn assert_effective_label_type_contract(
    effective: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let label_type = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(9))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective SELinux label-type option expected")?;
    assert!(matches!(
        label_type.kind(),
        compose_lens::model::SecurityOptionKind::SecurityLabelType { label_type }
            if label_type == "container_t"
    ));
    Ok(())
}

fn assert_effective_label_nested_contract(
    effective: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let label_nested = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(8))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective SELinux label-nested option expected")?;
    assert!(matches!(
        label_nested.kind(),
        compose_lens::model::SecurityOptionKind::SecurityLabelNested { enabled: true }
    ));
    Ok(())
}

fn assert_effective_label_level_contract(
    effective: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let label_level = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(7))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective SELinux label-level option expected")?;
    assert!(matches!(
        label_level.kind(),
        compose_lens::model::SecurityOptionKind::SecurityLabelLevel { level }
            if level == "s0:c1,c2"
    ));
    Ok(())
}

fn assert_effective_label_filetype_contract(
    effective: &compose_lens::project::ProjectViewResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let label_filetype = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::security_options)
        .and_then(|options| options.value().get(6))
        .map(compose_lens::project::ProjectValue::value)
        .ok_or("effective SELinux label-filetype option expected")?;
    assert!(matches!(
        label_filetype.kind(),
        compose_lens::model::SecurityOptionKind::SecurityLabelFileType { file_type }
            if file_type == "container_file_t"
    ));
    Ok(())
}

#[test]
fn exposes_authored_effective_and_generated_expose_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    expose: [80, \"80\", \"80/tcp\"]\n";
    let syntax = SyntaxDocument::parse(SourceId::new(694), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::expose)
        .ok_or("authored expose expected")?;
    assert_eq!(
        authored.items()[0].scalar_kind(),
        compose_lens::model::ExposeScalarKind::Number
    );
    assert_eq!(
        authored.items()[1].scalar_kind(),
        compose_lens::model::ExposeScalarKind::String
    );
    assert!(matches!(
        authored.items()[2].kind(),
        compose_lens::model::ExposeItemKind::Documented { .. }
    ));

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(695),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = build_project_view(merged.project().ok_or("merged project expected")?, None);
    let effective = project
        .view()
        .and_then(|view| view.service("app"))
        .and_then(compose_lens::project::ProjectService::expose)
        .ok_or("effective expose expected")?;
    assert_eq!(effective.value().len(), 3);
    let _: &compose_lens::project::ProjectExposeItem = effective.value()[0].value();

    let mut service = GeneratedService::new("app")?;
    service.set_expose(vec![GeneratedString::plain("80")?, GeneratedString::plain("80/tcp")?])?;
    assert_eq!(service.expose().map(<[GeneratedString]>::len), Some(2));
    Ok(())
}

#[test]
fn exposes_dns_search_authored_project_and_generated_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    dns_search: [example.internal, example.internal, .]\n";
    let syntax = SyntaxDocument::parse(SourceId::new(692), source)?;
    let document = ComposeDocument::parse(syntax.document());
    assert!(matches!(
        document
            .document()
            .and_then(|document| document.service("app"))
            .and_then(compose_lens::model::Service::dns_search)
            .map(compose_lens::model::DnsSearch::form),
        Some(compose_lens::model::DnsSearchForm::List(items)) if items.len() == 3
    ));

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(693),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let view = build_project_view(merged.project().ok_or("merged project expected")?, None);
    assert!(matches!(
        view.view()
            .and_then(|view| view.service("app"))
            .and_then(compose_lens::project::ProjectService::dns_search)
            .map(compose_lens::project::ProjectValue::value),
        Some(ProjectDnsSearch::List(items)) if items.len() == 3
    ));

    let mut service = GeneratedService::new("app")?;
    service.set_dns_search(GeneratedDnsSearch::Scalar(GeneratedString::plain(".")?))?;
    assert!(matches!(service.dns_search(), Some(GeneratedDnsSearch::Scalar(value)) if value.expose() == "."));
    Ok(())
}

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
    "    dns: [1.1.1.1, 1.1.1.1, resolver.internal]\n",
    "    dns_opt: [ndots:5, timeout:2]\n",
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
    set_generated_dns(&mut service)?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.set_name("example")?;
    builder.add_service(service)?;

    let generated = builder.build(SourceId::new(503))?;
    assert_generated_hostname(&generated)?;
    assert_generated_capabilities(&generated)?;
    assert_generated_devices(&generated)?;
    assert_generated_dns(&generated)?;
    assert_generated_dns_options(&generated)?;
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

#[test]
fn generated_network_attachment_addresses_are_additive_public_contract() -> Result<(), Box<dyn std::error::Error>> {
    let mut attachment = GeneratedNetworkAttachment::new("frontend")?;
    attachment.add_alias("app")?;
    attachment.set_ipv4_address(GeneratedString::plain("192.0.2.40")?)?;
    attachment.set_ipv6_address(GeneratedString::plain("2001:db8::40")?)?;
    assert_eq!(
        attachment.ipv4_address().map(GeneratedString::expose),
        Some("192.0.2.40")
    );
    assert_eq!(
        attachment.ipv6_address().map(GeneratedString::expose),
        Some("2001:db8::40")
    );

    let mut service = GeneratedService::new("app")?;
    service.add_network(attachment)?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(service)?;
    let generated = builder.build(SourceId::new(818))?;
    assert!(matches!(
        generated.document().service("app").and_then(compose_lens::model::Service::networks),
        Some(compose_lens::model::ServiceNetworks::Long { networks, .. })
            if networks.first().is_some_and(|network| {
                network.ipv4_address().is_some_and(|value| value.value() == "192.0.2.40")
                    && network.ipv6_address().is_some_and(|value| value.value() == "2001:db8::40")
            })
    ));
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
            "    dns:\n",
            "      - \"1.1.1.1\"\n",
            "      - \"1.1.1.1\"\n",
            "      - \"resolver.internal\"\n",
            "    dns_opt:\n",
            "      - \"ndots:5\"\n",
            "      - \"timeout:2\"\n",
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

fn set_generated_dns(service: &mut GeneratedService) -> Result<(), compose_lens::render::GenerationError> {
    service.set_dns(GeneratedDns::List(vec![
        GeneratedString::plain("1.1.1.1")?,
        GeneratedString::plain("1.1.1.1")?,
        GeneratedString::plain("resolver.internal")?,
    ]))?;
    service.set_dns_options(vec![
        GeneratedString::plain("ndots:5")?,
        GeneratedString::plain("timeout:2")?,
    ])
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
    let dns = service.dns().ok_or("native project DNS expected")?;
    assert!(matches!(dns.value(), ProjectDns::List(items)
        if items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>()
            == ["1.1.1.1", "1.1.1.1", "resolver.internal"]));
    assert_eq!(
        service.dns_options().map(|options| options
            .value()
            .iter()
            .map(|item| item.value().as_str())
            .collect::<Vec<_>>()),
        Some(vec!["ndots:5", "timeout:2"])
    );
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

fn assert_generated_dns(generated: &compose_lens::render::GeneratedComposeDocument) -> Result<(), &'static str> {
    let dns = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::dns)
        .ok_or("generated DNS expected")?;
    assert!(matches!(dns.form(), compose_lens::model::DnsForm::List(items)
        if items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>()
            == ["1.1.1.1", "1.1.1.1", "resolver.internal"]));
    Ok(())
}

fn assert_generated_dns_options(
    generated: &compose_lens::render::GeneratedComposeDocument,
) -> Result<(), &'static str> {
    let options = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::dns_options)
        .ok_or("generated DNS options expected")?;
    assert_eq!(
        options
            .items()
            .iter()
            .map(|item| item.value().as_str())
            .collect::<Vec<_>>(),
        ["ndots:5", "timeout:2"]
    );
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
