//! Deterministic generated-document construction and parse-back validation.

use compose_lens::{
    model::{
        AnnotationsForm, BooleanValue, Command, ComposeScalar, DnsForm, DnsSearchForm, Entrypoint, Environment,
        EnvironmentFile, EnvironmentFileFormatKind, ExtraHosts, HostnameKind, Labels, MemLimitKind, MemLimitScalarKind,
        MemLimitUnit, Port, ServiceNetworks, ShmSizeKind, ShmSizeScalarKind, ShmSizeUnit, StopGracePeriod, SysctlsForm,
        UlimitValue, VolumeMount,
    },
    render::{
        ComposeDocumentBuilder, GeneratedAnnotation, GeneratedCommand, GeneratedComposeDocument, GeneratedDevice,
        GeneratedDns, GeneratedDnsSearch, GeneratedEntrypoint, GeneratedEnvironment, GeneratedEnvironmentFile,
        GeneratedEnvironmentFileFormat, GeneratedExtraHost, GeneratedHostname, GeneratedLabel, GeneratedLogging,
        GeneratedLoggingOption, GeneratedLoggingOptionValue, GeneratedLongDevice, GeneratedMemLimit, GeneratedMount,
        GeneratedNetworkAttachment, GeneratedNetworkDefinition, GeneratedNetworkDriverOption,
        GeneratedNetworkDriverOptionValue, GeneratedPidsLimit, GeneratedPort, GeneratedProtocol, GeneratedPullPolicy,
        GeneratedResource, GeneratedRestartPolicy, GeneratedSelinux, GeneratedService, GeneratedShmSize,
        GeneratedString, GeneratedSysctl, GeneratedSysctls, GeneratedTmpfs, GeneratedUlimit, GeneratedUlimitValue,
        GeneratedUlimits, GeneratedVolumeDefinition, GeneratedVolumeDriverOption, GeneratedVolumeDriverOptionValue,
        GenerationError,
    },
    source::SourceId,
};

#[test]
fn generates_network_definition_boolean_omission_combinations_with_drivers_and_parse_back()
-> Result<(), Box<dyn std::error::Error>> {
    let mut configured = GeneratedNetworkDefinition::application("configured")?;
    configured.set_custom_name("observed-configured")?;
    configured.set_driver(GeneratedString::sensitive("vendor-driver")?)?;
    configured.set_driver_opts(vec![
        GeneratedNetworkDriverOption::new(
            "string-shaped-number",
            GeneratedNetworkDriverOptionValue::String(plain("2")?),
        )?,
        GeneratedNetworkDriverOption::new("numeric", GeneratedNetworkDriverOptionValue::Number(plain("2")?))?,
    ])?;
    configured.set_enable_ipv6(false)?;
    configured.set_internal(true)?;
    configured.set_labels(vec![
        GeneratedLabel::new("com.example.equals", plain("left=right")?)?,
        GeneratedLabel::new("com.example.secret", GeneratedString::sensitive("private-label")?)?,
    ])?;
    assert_eq!(configured.driver().map(GeneratedString::expose), Some("vendor-driver"));
    assert_eq!(
        configured.driver_opts().map(<[GeneratedNetworkDriverOption]>::len),
        Some(2)
    );
    assert_eq!(configured.enable_ipv6(), Some(false));
    assert_eq!(configured.internal(), Some(true));
    assert_eq!(configured.labels().map(<[GeneratedLabel]>::len), Some(2));

    let mut empty_options = GeneratedNetworkDefinition::application("empty-options")?;
    empty_options.set_driver_opts(Vec::new())?;
    empty_options.set_enable_ipv6(true)?;
    empty_options.set_internal(false)?;
    empty_options.set_labels(Vec::new())?;

    let mut service = GeneratedService::new("app")?;
    service.add_network(GeneratedNetworkAttachment::new("configured")?)?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(service)?;
    builder.add_network(GeneratedResource::application("basic")?)?;
    builder.add_network_definition(configured)?;
    builder.add_network_definition(empty_options)?;
    let generated = builder.build(SourceId::new(819))?;

    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"app\":\n",
            "    networks:\n",
            "      \"configured\": {}\n",
            "networks:\n",
            "  \"basic\": {}\n",
            "  \"configured\":\n",
            "    name: \"observed-configured\"\n",
            "    driver: \"vendor-driver\"\n",
            "    driver_opts:\n",
            "      \"string-shaped-number\": \"2\"\n",
            "      \"numeric\": 2\n",
            "    enable_ipv6: false\n",
            "    internal: true\n",
            "    labels:\n",
            "      \"com.example.equals\": \"left=right\"\n",
            "      \"com.example.secret\": \"private-label\"\n",
            "  \"empty-options\":\n",
            "    driver_opts: {}\n",
            "    enable_ipv6: true\n",
            "    internal: false\n",
            "    labels: {}\n",
        )
    );
    assert!(!generated.text().contains("enable_ipv4"));
    assert!(generated.is_sensitive());
    assert!(format!("{generated:?}").contains("<redacted>"));
    assert!(!format!("{generated:?}").contains("vendor-driver"));
    assert!(!format!("{generated:?}").contains("private-label"));

    assert_generated_network_definition_parse_back(generated.document())
}

#[test]
fn generates_application_owned_volume_driver_options_with_scalar_fidelity_and_parse_back()
-> Result<(), Box<dyn std::error::Error>> {
    let mut configured = GeneratedVolumeDefinition::application("configured")?;
    configured.set_custom_name("observed-configured")?;
    configured.set_driver(plain("opaque-driver")?)?;
    configured.set_driver_opts(vec![
        GeneratedVolumeDriverOption::new(
            "string-shaped-number",
            GeneratedVolumeDriverOptionValue::String(plain("2")?),
        )?,
        GeneratedVolumeDriverOption::new("numeric", GeneratedVolumeDriverOptionValue::Number(plain("2")?))?,
    ])?;
    configured.set_labels(vec![
        GeneratedLabel::new("com.example.equals", plain("left=right")?)?,
        GeneratedLabel::new("com.example.secret", GeneratedString::sensitive("private-label")?)?,
    ])?;
    assert_eq!(configured.driver().map(GeneratedString::expose), Some("opaque-driver"));
    assert_eq!(
        configured.driver_opts().map(<[GeneratedVolumeDriverOption]>::len),
        Some(2)
    );
    assert_eq!(configured.labels().map(<[GeneratedLabel]>::len), Some(2));

    let mut empty_options = GeneratedVolumeDefinition::application("empty-options")?;
    empty_options.set_driver_opts(Vec::new())?;
    empty_options.set_labels(Vec::new())?;

    let mut builder = ComposeDocumentBuilder::new();
    let mut service = GeneratedService::new("app")?;
    service.set_image(plain("example/app")?)?;
    builder.add_service(service)?;
    builder.add_volume_definition(configured)?;
    builder.add_volume_definition(empty_options)?;
    let generated = builder.build(SourceId::new(820))?;

    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"app\":\n",
            "    image: \"example/app\"\n",
            "volumes:\n",
            "  \"configured\":\n",
            "    name: \"observed-configured\"\n",
            "    driver: \"opaque-driver\"\n",
            "    driver_opts:\n",
            "      \"string-shaped-number\": \"2\"\n",
            "      \"numeric\": 2\n",
            "    labels:\n",
            "      \"com.example.equals\": \"left=right\"\n",
            "      \"com.example.secret\": \"private-label\"\n",
            "  \"empty-options\":\n",
            "    driver_opts: {}\n",
            "    labels: {}\n",
        )
    );
    assert!(generated.is_sensitive());
    assert!(format!("{generated:?}").contains("<redacted>"));
    assert!(!format!("{generated:?}").contains("opaque-driver"));
    assert!(!format!("{generated:?}").contains("private-label"));

    let configured = generated
        .document()
        .volumes()
        .iter()
        .find(|volume| volume.name().value() == "configured")
        .ok_or("configured volume expected")?;
    assert_eq!(
        configured.driver().map(|driver| driver.value().as_str()),
        Some("opaque-driver")
    );
    assert!(matches!(
        configured.driver_opts()[0].value().value(),
        ComposeScalar::String(value) if value == "2"
    ));
    assert!(matches!(
        configured.driver_opts()[1].value().value(),
        ComposeScalar::Number(value) if value == "2"
    ));
    assert!(matches!(
        configured.labels(),
        Some(Labels::Map { entries, .. })
            if entries.len() == 2
                && entries[0].key().value() == "com.example.equals"
                && matches!(entries[0].value().value(), ComposeScalar::String(value) if value == "left=right")
                && entries[1].key().value() == "com.example.secret"
                && matches!(entries[1].value().value(), ComposeScalar::String(value) if value == "private-label")
    ));
    assert!(
        generated
            .document()
            .volumes()
            .iter()
            .find(|volume| volume.name().value() == "empty-options")
            .is_some_and(|volume| {
                volume.driver_opts().is_empty()
                    && matches!(volume.labels(), Some(Labels::Map { entries, .. }) if entries.is_empty())
            })
    );
    Ok(())
}

#[test]
fn volume_definition_rejects_invalid_or_duplicate_driver_configuration() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        GeneratedVolumeDriverOption::new(
            "count",
            GeneratedVolumeDriverOptionValue::Number(plain("not-a-number")?),
        ),
        Err(GenerationError::InvalidVolumeDriverOptionNumber)
    );

    let first = GeneratedVolumeDriverOption::new("same", GeneratedVolumeDriverOptionValue::String(plain("first")?))?;
    let replacement = GeneratedVolumeDriverOption::new("same", GeneratedVolumeDriverOptionValue::Number(plain("2")?))?;
    let mut volume = GeneratedVolumeDefinition::application("configured")?;
    volume.set_driver(plain("unrecognized-driver")?)?;
    assert_eq!(
        volume.set_driver(plain("replacement")?),
        Err(GenerationError::DuplicateField("volume driver"))
    );
    assert_eq!(
        volume.set_driver_opts(vec![first, replacement]),
        Err(GenerationError::DuplicateName {
            kind: "volume driver option",
            name: "same".to_owned(),
        })
    );
    volume.set_driver_opts(Vec::new())?;
    assert_eq!(
        volume.set_driver_opts(Vec::new()),
        Err(GenerationError::DuplicateField("volume driver_opts"))
    );
    volume.set_labels(Vec::new())?;
    assert_eq!(
        volume.set_labels(Vec::new()),
        Err(GenerationError::DuplicateField("volume labels"))
    );

    let duplicate = GeneratedLabel::new("com.example.same", plain("first")?)?;
    let replacement = GeneratedLabel::new("com.example.same", plain("second")?)?;
    let mut duplicate_labels = GeneratedVolumeDefinition::application("duplicate-labels")?;
    assert_eq!(
        duplicate_labels.set_labels(vec![duplicate, replacement]),
        Err(GenerationError::DuplicateName {
            kind: "volume label",
            name: "com.example.same".to_owned(),
        })
    );

    let mut builder = ComposeDocumentBuilder::new();
    let mut service = GeneratedService::new("app")?;
    service.set_image(plain("example/app")?)?;
    builder.add_service(service)?;
    builder.add_volume(GeneratedResource::external("configured")?)?;
    assert_eq!(
        builder.add_volume_definition(volume),
        Err(GenerationError::DuplicateName {
            kind: "volume",
            name: "configured".to_owned(),
        })
    );
    Ok(())
}

fn assert_generated_network_definition_parse_back(
    document: &compose_lens::model::ComposeDocument,
) -> Result<(), Box<dyn std::error::Error>> {
    let configured = document
        .networks()
        .iter()
        .find(|network| network.name().value() == "configured")
        .ok_or("configured network expected")?;
    assert_eq!(
        configured.driver().map(|driver| driver.value().as_str()),
        Some("vendor-driver")
    );
    assert!(matches!(
        configured.driver_opts()[0].value().value(),
        ComposeScalar::String(value) if value == "2"
    ));
    assert!(matches!(
        configured.driver_opts()[1].value().value(),
        ComposeScalar::Number(value) if value == "2"
    ));
    assert_eq!(
        configured.enable_ipv6().map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(
        configured.internal().map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(true))
    );
    let labels = configured.labels().ok_or("configured network labels expected")?;
    let Labels::Map { entries, .. } = labels else {
        return Err("generated network labels should parse as a mapping".into());
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].key().value(), "com.example.equals");
    assert_eq!(
        entries[0].value().value(),
        &ComposeScalar::String("left=right".to_owned())
    );
    assert_eq!(
        entries[1].value().value(),
        &ComposeScalar::String("private-label".to_owned())
    );
    assert!(
        document
            .networks()
            .iter()
            .find(|network| network.name().value() == "empty-options")
            .is_some_and(|network| {
                network.driver_opts().is_empty()
                    && network
                        .enable_ipv6()
                        .is_some_and(|value| value.value() == &BooleanValue::Literal(true))
                    && network
                        .internal()
                        .is_some_and(|value| value.value() == &BooleanValue::Literal(false))
                    && matches!(network.labels(), Some(Labels::Map { entries, .. }) if entries.is_empty())
            })
    );
    Ok(())
}

#[test]
fn network_definition_driver_options_reject_duplicates_and_invalid_numbers_without_plugin_validation()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        GeneratedNetworkDriverOption::new(
            "count",
            GeneratedNetworkDriverOptionValue::Number(plain("not-a-number")?),
        ),
        Err(GenerationError::InvalidNetworkDriverOptionNumber)
    );

    let option = GeneratedNetworkDriverOption::new("same", GeneratedNetworkDriverOptionValue::String(plain("first")?))?;
    let replacement =
        GeneratedNetworkDriverOption::new("same", GeneratedNetworkDriverOptionValue::Number(plain("2")?))?;
    let mut network = GeneratedNetworkDefinition::application("custom")?;
    network.set_driver(plain("unrecognized-plugin")?)?;
    assert_eq!(
        network.set_driver(plain("replacement")?),
        Err(GenerationError::DuplicateField("network driver"))
    );
    assert_eq!(
        network.set_driver_opts(vec![option, replacement]),
        Err(GenerationError::DuplicateName {
            kind: "network driver option",
            name: "same".to_owned(),
        })
    );
    network.set_driver_opts(Vec::new())?;
    assert_eq!(
        network.set_driver_opts(Vec::new()),
        Err(GenerationError::DuplicateField("network driver_opts"))
    );
    network.set_enable_ipv6(false)?;
    assert_eq!(
        network.set_enable_ipv6(true),
        Err(GenerationError::DuplicateField("network enable_ipv6"))
    );
    network.set_internal(false)?;
    assert_eq!(
        network.set_internal(true),
        Err(GenerationError::DuplicateField("network internal"))
    );
    network.set_labels(Vec::new())?;
    assert_eq!(
        network.set_labels(Vec::new()),
        Err(GenerationError::DuplicateField("network labels"))
    );

    let duplicate = GeneratedLabel::new("com.example.same", plain("first")?)?;
    let replacement = GeneratedLabel::new("com.example.same", plain("second")?)?;
    let mut duplicate_labels = GeneratedNetworkDefinition::application("duplicate-labels")?;
    assert_eq!(
        duplicate_labels.set_labels(vec![duplicate, replacement]),
        Err(GenerationError::DuplicateName {
            kind: "network label",
            name: "com.example.same".to_owned(),
        })
    );

    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(GeneratedService::new("app")?)?;
    builder.add_network(GeneratedResource::application("custom")?)?;
    assert_eq!(
        builder.add_network_definition(network),
        Err(GenerationError::DuplicateName {
            kind: "network",
            name: "custom".to_owned(),
        })
    );
    Ok(())
}

#[test]
fn generates_network_attachment_address_omission_combinations_with_aliases_and_parse_back()
-> Result<(), Box<dyn std::error::Error>> {
    let mut omitted = GeneratedService::new("omitted")?;
    omitted.add_network(GeneratedNetworkAttachment::new("frontend")?)?;

    let mut ipv4_only = GeneratedService::new("ipv4-only")?;
    let mut ipv4 = GeneratedNetworkAttachment::new("frontend")?;
    ipv4.set_ipv4_address(plain("192.0.2.10")?)?;
    assert_eq!(ipv4.ipv4_address().map(GeneratedString::expose), Some("192.0.2.10"));
    assert!(ipv4.ipv6_address().is_none());
    ipv4_only.add_network(ipv4)?;

    let mut ipv6_only = GeneratedService::new("ipv6-only")?;
    let mut ipv6 = GeneratedNetworkAttachment::new("frontend")?;
    ipv6.set_ipv6_address(plain("2001:db8::10")?)?;
    assert!(ipv6.ipv4_address().is_none());
    assert_eq!(ipv6.ipv6_address().map(GeneratedString::expose), Some("2001:db8::10"));
    ipv6_only.add_network(ipv6)?;

    let mut both = GeneratedService::new("both")?;
    let mut addresses = GeneratedNetworkAttachment::new("frontend")?;
    addresses.add_alias("web")?;
    addresses.add_alias("api")?;
    addresses.set_ipv4_address(plain("198.51.100.20")?)?;
    addresses.set_ipv6_address(plain("2001:db8::20")?)?;
    both.add_network(addresses)?;

    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(omitted)?;
    builder.add_service(ipv4_only)?;
    builder.add_service(ipv6_only)?;
    builder.add_service(both)?;
    let generated = builder.build(SourceId::new(816))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    networks:\n",
            "      \"frontend\": {}\n",
            "  \"ipv4-only\":\n",
            "    networks:\n",
            "      \"frontend\":\n",
            "        ipv4_address: \"192.0.2.10\"\n",
            "  \"ipv6-only\":\n",
            "    networks:\n",
            "      \"frontend\":\n",
            "        ipv6_address: \"2001:db8::10\"\n",
            "  \"both\":\n",
            "    networks:\n",
            "      \"frontend\":\n",
            "        aliases:\n",
            "          - \"web\"\n",
            "          - \"api\"\n",
            "        ipv4_address: \"198.51.100.20\"\n",
            "        ipv6_address: \"2001:db8::20\"\n",
        )
    );

    let omitted = generated_network(&generated, "omitted")?;
    assert!(omitted.aliases().is_empty());
    assert!(omitted.ipv4_address().is_none());
    assert!(omitted.ipv6_address().is_none());
    let ipv4 = generated_network(&generated, "ipv4-only")?;
    assert_eq!(
        ipv4.ipv4_address().map(|value| value.value().as_str()),
        Some("192.0.2.10")
    );
    assert!(ipv4.ipv6_address().is_none());
    let ipv6 = generated_network(&generated, "ipv6-only")?;
    assert!(ipv6.ipv4_address().is_none());
    assert_eq!(
        ipv6.ipv6_address().map(|value| value.value().as_str()),
        Some("2001:db8::10")
    );
    let both = generated_network(&generated, "both")?;
    assert_eq!(
        both.aliases()
            .iter()
            .map(|alias| alias.value().as_str())
            .collect::<Vec<_>>(),
        ["web", "api"]
    );
    assert_eq!(
        both.ipv4_address().map(|value| value.value().as_str()),
        Some("198.51.100.20")
    );
    assert_eq!(
        both.ipv6_address().map(|value| value.value().as_str()),
        Some("2001:db8::20")
    );
    Ok(())
}

#[test]
fn network_attachment_addresses_keep_generated_string_safety_redaction_and_duplicate_rules()
-> Result<(), Box<dyn std::error::Error>> {
    let mut attachment = GeneratedNetworkAttachment::new("frontend")?;
    attachment.set_ipv4_address(GeneratedString::sensitive("not-an-ip\n${STATIC_V4}")?)?;
    attachment.set_ipv6_address(plain("")?)?;
    assert_eq!(
        attachment.set_ipv4_address(plain("192.0.2.1")?),
        Err(GenerationError::DuplicateField("ipv4_address"))
    );
    assert_eq!(
        attachment.set_ipv6_address(plain("2001:db8::1")?),
        Err(GenerationError::DuplicateField("ipv6_address"))
    );
    assert_eq!(
        GeneratedString::plain("192.0.2.1\0hidden"),
        Err(GenerationError::ContainsNul("string"))
    );

    let mut service = GeneratedService::new("app")?;
    service.add_network(attachment)?;
    assert_eq!(
        service.add_network(GeneratedNetworkAttachment::new("frontend")?),
        Err(GenerationError::DuplicateName {
            kind: "service network",
            name: "frontend".to_owned(),
        })
    );
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(service)?;
    let generated = builder.build(SourceId::new(817))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"app\":\n",
            "    networks:\n",
            "      \"frontend\":\n",
            "        ipv4_address: \"not-an-ip\\n${STATIC_V4}\"\n",
            "        ipv6_address: \"\"\n",
        )
    );
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("not-an-ip"));
    let parsed = generated_network(&generated, "app")?;
    assert_eq!(
        parsed.ipv4_address().map(|value| value.value().as_str()),
        Some("not-an-ip\n${STATIC_V4}")
    );
    assert_eq!(parsed.ipv6_address().map(|value| value.value().as_str()), Some(""));
    Ok(())
}

#[test]
fn generates_explicit_logging_with_ordered_scalar_kinds_empty_options_parse_back_and_redaction()
-> Result<(), Box<dyn std::error::Error>> {
    let mut empty = GeneratedService::new("empty")?;
    empty.set_logging(GeneratedLogging::new(plain("custom-driver")?, Vec::new())?)?;

    let mut configured = GeneratedService::new("configured")?;
    configured.set_logging(GeneratedLogging::new(
        plain("vendor-driver")?,
        vec![
            GeneratedLoggingOption::new(
                "string-option",
                GeneratedLoggingOptionValue::String(GeneratedString::sensitive("01")?),
            )?,
            GeneratedLoggingOption::new("number-option", GeneratedLoggingOptionValue::Number(plain("0001")?))?,
            GeneratedLoggingOption::new("null-option", GeneratedLoggingOptionValue::Null)?,
        ],
    )?)?;
    assert_eq!(
        configured
            .logging()
            .map(GeneratedLogging::driver)
            .map(GeneratedString::expose),
        Some("vendor-driver")
    );
    assert_eq!(
        configured.logging().map(GeneratedLogging::options).map(<[_]>::len),
        Some(3)
    );

    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(empty)?;
    builder.add_service(configured)?;
    let generated = builder.build(SourceId::new(815))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"empty\":\n",
            "    logging:\n",
            "      driver: \"custom-driver\"\n",
            "      options: {}\n",
            "  \"configured\":\n",
            "    logging:\n",
            "      driver: \"vendor-driver\"\n",
            "      options:\n",
            "        \"string-option\": \"01\"\n",
            "        \"number-option\": 0001\n",
            "        \"null-option\": null\n",
        )
    );
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("01"));
    let logging = generated
        .document()
        .service("configured")
        .and_then(compose_lens::model::Service::logging)
        .ok_or("generated logging parse-back expected")?;
    assert_eq!(
        logging
            .driver()
            .map(compose_lens::model::Located::value)
            .map(String::as_str),
        Some("vendor-driver")
    );
    let options = logging.options().ok_or("generated logging options expected")?;
    assert!(
        matches!(options.entries()[0].value().value(), compose_lens::model::LoggingOptionValue::String(value) if value == "01")
    );
    assert!(
        matches!(options.entries()[1].value().value(), compose_lens::model::LoggingOptionValue::Number(value) if value == "0001")
    );
    assert!(matches!(
        options.entries()[2].value().value(),
        compose_lens::model::LoggingOptionValue::Null
    ));
    Ok(())
}

#[test]
fn rejects_invalid_or_duplicate_generated_logging_without_driver_semantics() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        GeneratedLoggingOption::new("", GeneratedLoggingOptionValue::Null),
        Err(GenerationError::EmptyValue("logging option key"))
    );
    assert_eq!(
        GeneratedLoggingOption::new("bad-number", GeneratedLoggingOptionValue::Number(plain("1 trailing")?),),
        Err(GenerationError::InvalidLoggingOptionNumber)
    );
    let first = GeneratedLoggingOption::new("same", GeneratedLoggingOptionValue::Null)?;
    let second = GeneratedLoggingOption::new("same", GeneratedLoggingOptionValue::String(plain("value")?))?;
    assert_eq!(
        GeneratedLogging::new(plain("")?, vec![first, second]),
        Err(GenerationError::DuplicateName {
            kind: "logging option",
            name: "same".to_owned(),
        })
    );
    let mut service = GeneratedService::new("app")?;
    service.set_logging(GeneratedLogging::new(plain("uninterpreted:${DRIVER}")?, Vec::new())?)?;
    assert_eq!(
        service.set_logging(GeneratedLogging::new(plain("second")?, Vec::new())?),
        Err(GenerationError::DuplicateField("logging"))
    );
    Ok(())
}

#[test]
fn generates_safe_unique_annotations_empty_state_sensitivity_and_parse_back() -> Result<(), Box<dyn std::error::Error>>
{
    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;
    let mut empty = GeneratedService::new("empty")?;
    empty.set_annotations(Vec::new())?;
    let mut configured = GeneratedService::new("configured")?;
    configured.set_annotations(vec![
        GeneratedAnnotation::new("io.example.empty", plain("")?)?,
        GeneratedAnnotation::new("io.example.equals", plain("left=right")?)?,
        GeneratedAnnotation::new("io.example.secret", GeneratedString::sensitive("secret-value")?)?,
    ])?;
    assert_eq!(configured.annotations().map(<[GeneratedAnnotation]>::len), Some(3));
    assert_eq!(
        configured.set_annotations(Vec::new()),
        Err(GenerationError::DuplicateField("annotations"))
    );

    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(omitted)?;
    builder.add_service(empty)?;
    builder.add_service(configured)?;
    let generated = builder.build(SourceId::new(700))?;
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("secret-value"));
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"empty\":\n",
            "    annotations: {}\n",
            "  \"configured\":\n",
            "    annotations:\n",
            "      \"io.example.empty\": \"\"\n",
            "      \"io.example.equals\": \"left=right\"\n",
            "      \"io.example.secret\": \"secret-value\"\n",
        )
    );
    assert!(
        generated
            .document()
            .service("omitted")
            .is_some_and(|service| service.annotations().is_none())
    );
    assert!(matches!(
        generated.document().service("empty").and_then(compose_lens::model::Service::annotations).map(compose_lens::model::Annotations::form),
        Some(AnnotationsForm::Map(entries)) if entries.is_empty()
    ));
    assert!(matches!(
        generated.document().service("configured").and_then(compose_lens::model::Service::annotations).map(compose_lens::model::Annotations::form),
        Some(AnnotationsForm::Map(entries)) if entries.len() == 3
    ));
    Ok(())
}

#[test]
fn generated_annotations_reject_duplicates_deferred_and_malformed_values() -> Result<(), Box<dyn std::error::Error>> {
    for name in ["", "$NAME", "line\nbreak"] {
        assert_eq!(
            GeneratedAnnotation::new(name, plain("value")?),
            Err(GenerationError::InvalidAnnotationName)
        );
    }
    for value in ["$VALUE", "line\rbreak", "line\nbreak"] {
        assert_eq!(
            GeneratedAnnotation::new("io.example.name", plain(value)?),
            Err(GenerationError::InvalidAnnotationValue)
        );
    }
    let mut duplicate = GeneratedService::new("app")?;
    assert!(matches!(
        duplicate.set_annotations(vec![
            GeneratedAnnotation::new("io.example.same", plain("one")?)?,
            GeneratedAnnotation::new("io.example.same", plain("two")?)?,
        ]),
        Err(GenerationError::DuplicateName { kind: "service annotation", name }) if name == "io.example.same"
    ));
    assert!(duplicate.annotations().is_none());
    Ok(())
}

#[test]
fn generates_quoted_unique_expose_items_empty_state_sensitivity_and_parse_back()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{ExposeItemKind, ExposeScalarKind};

    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;
    let mut empty = GeneratedService::new("empty")?;
    empty.set_expose(Vec::new())?;
    let mut configured = GeneratedService::new("configured")?;
    configured.set_expose(vec![
        plain("80")?,
        plain("80/tcp")?,
        GeneratedString::sensitive("53/udp")?,
        plain("080-090")?,
    ])?;
    assert_eq!(configured.expose().map(<[GeneratedString]>::len), Some(4));
    assert_eq!(
        configured.set_expose(Vec::new()),
        Err(GenerationError::DuplicateField("expose"))
    );

    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(omitted)?;
    builder.add_service(empty)?;
    builder.add_service(configured)?;
    let generated = builder.build(SourceId::new(692))?;
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("53/udp"));
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"empty\":\n",
            "    expose: []\n",
            "  \"configured\":\n",
            "    expose:\n",
            "      - \"80\"\n",
            "      - \"80/tcp\"\n",
            "      - \"53/udp\"\n",
            "      - \"080-090\"\n",
        )
    );
    assert!(
        generated
            .document()
            .service("omitted")
            .is_some_and(|service| service.expose().is_none())
    );
    assert!(
        generated
            .document()
            .service("empty")
            .and_then(compose_lens::model::Service::expose)
            .is_some_and(|expose| expose.items().is_empty())
    );
    let items = generated
        .document()
        .service("configured")
        .and_then(compose_lens::model::Service::expose)
        .ok_or("parse-back expose expected")?
        .items();
    assert_eq!(
        items
            .iter()
            .map(compose_lens::model::ExposeItem::value)
            .collect::<Vec<_>>(),
        ["80", "80/tcp", "53/udp", "080-090"]
    );
    assert!(items.iter().all(|item| item.scalar_kind() == ExposeScalarKind::String));
    assert!(
        items
            .iter()
            .all(|item| matches!(item.kind(), ExposeItemKind::Documented { .. }))
    );
    Ok(())
}

#[test]
fn generated_expose_rejects_unsafe_malformed_provider_dependent_and_exact_duplicates()
-> Result<(), Box<dyn std::error::Error>> {
    for invalid in ["", "$PORT", "80\r90", "80\n90", "port", "80-", "80/sctp", "80/HTTP"] {
        let mut service = GeneratedService::new("app")?;
        assert_eq!(
            service.set_expose(vec![plain(invalid)?]),
            Err(GenerationError::InvalidExposeValue),
            "invalid {invalid:?}",
        );
        assert!(service.expose().is_none());
    }
    let mut duplicate = GeneratedService::new("app")?;
    assert_eq!(
        duplicate.set_expose(vec![plain("80")?, plain("80")?]),
        Err(GenerationError::DuplicateItem("expose"))
    );
    assert!(duplicate.expose().is_none());
    let mut distinct = GeneratedService::new("app")?;
    distinct.set_expose(vec![plain("80")?, plain("80/tcp")?])?;
    Ok(())
}

fn assert_generated_security_option_parse_back(parsed: &compose_lens::model::SecurityOptions) {
    use compose_lens::model::SecurityOptionKind;

    assert_eq!(
        parsed
            .items()
            .iter()
            .map(compose_lens::model::SecurityOptionItem::value)
            .collect::<Vec<_>>(),
        [
            "no-new-privileges:true",
            "no-new-privileges:false",
            "no-new-privileges:true",
            "apparmor=profile-a",
            "label:disable",
            "label:disable",
            "label=disable",
            "label:filetype:container_file_t",
            "label:filetype:container_file_t",
            "label:level:s0:c1,c2",
            "label:level:s0:c1,c2",
            "label:nested",
            "label:nested",
            "label:type:container_t",
            "label:type:container_t",
            "label:type:container_t:extended",
            "mask=/proc/acpi:/proc/kcore",
            "mask=/proc/acpi:/proc/kcore",
            "mask=relative:opaque=value",
            "apparmor=profile-a",
            "seccomp=unconfined",
            "seccomp=/workspace/seccomp.json",
            "seccomp=unconfined",
        ]
    );
    for (index, enabled) in [(0, true), (1, false), (2, true)] {
        assert!(matches!(
            parsed.items()[index].kind(),
            SecurityOptionKind::NoNewPrivileges { enabled: actual } if *actual == enabled
        ));
    }
    assert!(matches!(parsed.items()[3].kind(), SecurityOptionKind::AppArmor { .. }));
    for index in [4, 5] {
        assert!(matches!(
            parsed.items()[index].kind(),
            SecurityOptionKind::SecurityLabelDisable { enabled: true }
        ));
    }
    assert!(matches!(
        parsed.items()[6].kind(),
        SecurityOptionKind::SecurityLabelDisableNearMiss
    ));
    for index in [7, 8] {
        assert!(matches!(
            parsed.items()[index].kind(),
            SecurityOptionKind::SecurityLabelFileType { file_type }
                if file_type == "container_file_t"
        ));
    }
    for index in [9, 10] {
        assert!(matches!(
            parsed.items()[index].kind(),
            SecurityOptionKind::SecurityLabelLevel { level } if level == "s0:c1,c2"
        ));
    }
    for index in [11, 12] {
        assert!(matches!(
            parsed.items()[index].kind(),
            SecurityOptionKind::SecurityLabelNested { enabled: true }
        ));
    }
    for index in [13, 14] {
        assert!(matches!(
            parsed.items()[index].kind(),
            SecurityOptionKind::SecurityLabelType { label_type } if label_type == "container_t"
        ));
    }
    assert!(matches!(
        parsed.items()[15].kind(),
        SecurityOptionKind::SecurityLabelTypeNearMiss
    ));
    for index in [16, 17] {
        assert!(matches!(
            parsed.items()[index].kind(),
            SecurityOptionKind::Mask { paths } if paths == "/proc/acpi:/proc/kcore"
        ));
    }
    assert!(matches!(
        parsed.items()[18].kind(),
        SecurityOptionKind::Mask { paths } if paths == "relative:opaque=value"
    ));
    for (index, profile) in [(20, "unconfined"), (21, "/workspace/seccomp.json"), (22, "unconfined")] {
        assert!(matches!(
            parsed.items()[index].kind(),
            SecurityOptionKind::Seccomp { profile: actual } if actual == profile
        ));
    }
}

#[test]
fn generates_raw_ordered_security_options_with_duplicates_empty_state_and_parse_back()
-> Result<(), Box<dyn std::error::Error>> {
    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;
    let mut empty = GeneratedService::new("empty")?;
    empty.set_security_options(Vec::new())?;
    let mut configured = GeneratedService::new("configured")?;
    configured.set_security_options(vec![
        plain("no-new-privileges:true")?,
        plain("no-new-privileges:false")?,
        plain("no-new-privileges:true")?,
        GeneratedString::sensitive("apparmor=profile-a")?,
        plain("label:disable")?,
        plain("label:disable")?,
        plain("label=disable")?,
        plain("label:filetype:container_file_t")?,
        plain("label:filetype:container_file_t")?,
        plain("label:level:s0:c1,c2")?,
        plain("label:level:s0:c1,c2")?,
        plain("label:nested")?,
        plain("label:nested")?,
        plain("label:type:container_t")?,
        plain("label:type:container_t")?,
        plain("label:type:container_t:extended")?,
        plain("mask=/proc/acpi:/proc/kcore")?,
        plain("mask=/proc/acpi:/proc/kcore")?,
        plain("mask=relative:opaque=value")?,
        plain("apparmor=profile-a")?,
        plain("seccomp=unconfined")?,
        plain("seccomp=/workspace/seccomp.json")?,
        plain("seccomp=unconfined")?,
    ])?;
    assert_eq!(configured.security_options().map(<[GeneratedString]>::len), Some(23));
    assert_eq!(
        configured.set_security_options(Vec::new()),
        Err(GenerationError::DuplicateField("security_opt"))
    );

    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(omitted)?;
    builder.add_service(empty)?;
    builder.add_service(configured)?;
    let generated = builder.build(SourceId::new(697))?;
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("profile-a"));
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"empty\":\n",
            "    security_opt: []\n",
            "  \"configured\":\n",
            "    security_opt:\n",
            "      - \"no-new-privileges:true\"\n",
            "      - \"no-new-privileges:false\"\n",
            "      - \"no-new-privileges:true\"\n",
            "      - \"apparmor=profile-a\"\n",
            "      - \"label:disable\"\n",
            "      - \"label:disable\"\n",
            "      - \"label=disable\"\n",
            "      - \"label:filetype:container_file_t\"\n",
            "      - \"label:filetype:container_file_t\"\n",
            "      - \"label:level:s0:c1,c2\"\n",
            "      - \"label:level:s0:c1,c2\"\n",
            "      - \"label:nested\"\n",
            "      - \"label:nested\"\n",
            "      - \"label:type:container_t\"\n",
            "      - \"label:type:container_t\"\n",
            "      - \"label:type:container_t:extended\"\n",
            "      - \"mask=/proc/acpi:/proc/kcore\"\n",
            "      - \"mask=/proc/acpi:/proc/kcore\"\n",
            "      - \"mask=relative:opaque=value\"\n",
            "      - \"apparmor=profile-a\"\n",
            "      - \"seccomp=unconfined\"\n",
            "      - \"seccomp=/workspace/seccomp.json\"\n",
            "      - \"seccomp=unconfined\"\n",
        )
    );
    assert!(
        generated
            .document()
            .service("omitted")
            .is_some_and(|service| service.security_options().is_none())
    );
    assert!(
        generated
            .document()
            .service("empty")
            .and_then(compose_lens::model::Service::security_options)
            .is_some_and(|options| options.items().is_empty())
    );
    let parsed = generated
        .document()
        .service("configured")
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("parse-back security_opt expected")?;
    assert_generated_security_option_parse_back(parsed);
    Ok(())
}

#[test]
fn generates_repeatable_unmask_candidates_without_normalizing_payloads() -> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::SecurityOptionKind;

    let mut service = GeneratedService::new("app")?;
    service.set_security_options(vec![
        plain("unmask=ALL")?,
        plain("unmask=ALL")?,
        plain("unmask=/proc/acpi")?,
        plain("unmask=/proc/acpi:/sys/firmware")?,
        plain("unmask=/proc/*")?,
    ])?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(service)?;
    let generated = builder.build(SourceId::new(735))?;
    let parsed = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("parse-back unmask options expected")?;

    assert_eq!(
        parsed
            .items()
            .iter()
            .map(compose_lens::model::SecurityOptionItem::value)
            .collect::<Vec<_>>(),
        [
            "unmask=ALL",
            "unmask=ALL",
            "unmask=/proc/acpi",
            "unmask=/proc/acpi:/sys/firmware",
            "unmask=/proc/*",
        ]
    );
    for (item, expected) in
        parsed
            .items()
            .iter()
            .zip(["ALL", "ALL", "/proc/acpi", "/proc/acpi:/sys/firmware", "/proc/*"])
    {
        assert!(matches!(
            item.kind(),
            SecurityOptionKind::Unmask { paths } if paths == expected
        ));
    }
    assert!(generated.text().contains("      - \"unmask=/proc/*\"\n"));
    Ok(())
}

#[test]
fn generated_security_options_reject_unsafe_values_without_rejecting_duplicates()
-> Result<(), Box<dyn std::error::Error>> {
    for invalid in ["", "$SECURITY_OPT", "line\rbreak", "line\nbreak"] {
        let mut service = GeneratedService::new("app")?;
        assert_eq!(
            service.set_security_options(vec![plain(invalid)?]),
            Err(GenerationError::InvalidSecurityOptionValue)
        );
        assert!(service.security_options().is_none());
    }
    let mut duplicate = GeneratedService::new("app")?;
    duplicate.set_security_options(vec![plain("same")?, plain("same")?])?;
    assert_eq!(duplicate.security_options().map(<[GeneratedString]>::len), Some(2));
    assert!(matches!(
        GeneratedString::plain("nul\0option"),
        Err(GenerationError::ContainsNul("string"))
    ));
    Ok(())
}

#[test]
fn generates_raw_dns_scalar_list_empty_and_sensitive_forms_with_parse_back() -> Result<(), Box<dyn std::error::Error>> {
    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;
    let mut scalar = GeneratedService::new("scalar")?;
    scalar.set_dns(GeneratedDns::Scalar(plain("resolver.internal")?))?;
    let mut empty = GeneratedService::new("empty")?;
    empty.set_dns(GeneratedDns::List(Vec::new()))?;
    let mut listed = GeneratedService::new("listed")?;
    listed.set_dns(GeneratedDns::List(vec![
        plain("1.1.1.1")?,
        plain("1.1.1.1")?,
        GeneratedString::sensitive("2001:db8::53")?,
        plain("resolver.internal")?,
    ]))?;
    assert!(matches!(listed.dns(), Some(GeneratedDns::List(items)) if items.len() == 4));
    assert_eq!(
        listed.set_dns(GeneratedDns::List(Vec::new())),
        Err(GenerationError::DuplicateField("dns"))
    );

    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(omitted)?;
    builder.add_service(scalar)?;
    builder.add_service(empty)?;
    builder.add_service(listed)?;
    let generated = builder.build(SourceId::new(687))?;
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("2001:db8::53"));
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"scalar\":\n",
            "    dns: \"resolver.internal\"\n",
            "  \"empty\":\n",
            "    dns: []\n",
            "  \"listed\":\n",
            "    dns:\n",
            "      - \"1.1.1.1\"\n",
            "      - \"1.1.1.1\"\n",
            "      - \"2001:db8::53\"\n",
            "      - \"resolver.internal\"\n",
        )
    );
    assert!(
        generated
            .document()
            .service("omitted")
            .is_some_and(|service| service.dns().is_none())
    );
    assert!(matches!(
        generated.document().service("scalar").and_then(compose_lens::model::Service::dns).map(compose_lens::model::Dns::form),
        Some(DnsForm::Scalar(value)) if value.value() == "resolver.internal"
    ));
    assert!(matches!(
        generated.document().service("empty").and_then(compose_lens::model::Service::dns).map(compose_lens::model::Dns::form),
        Some(DnsForm::List(values)) if values.is_empty()
    ));
    assert!(matches!(
        generated.document().service("listed").and_then(compose_lens::model::Service::dns).map(compose_lens::model::Dns::form),
        Some(DnsForm::List(values))
            if values.iter().map(|value| value.value().as_str()).collect::<Vec<_>>()
                == ["1.1.1.1", "1.1.1.1", "2001:db8::53", "resolver.internal"]
    ));
    Ok(())
}

#[test]
fn generated_dns_rejects_unresolved_or_physical_line_unsafe_values() -> Result<(), Box<dyn std::error::Error>> {
    for invalid in ["", "${DNS_SERVER}", "line\nbreak", "line\rbreak"] {
        let mut service = GeneratedService::new("app")?;
        assert_eq!(
            service.set_dns(GeneratedDns::Scalar(plain(invalid)?)),
            Err(GenerationError::InvalidDnsValue)
        );
    }
    assert!(matches!(
        GeneratedString::plain("nul\0server"),
        Err(GenerationError::ContainsNul("string"))
    ));
    Ok(())
}

#[test]
fn generates_raw_dns_search_forms_duplicates_dot_and_sensitive_values_with_parse_back()
-> Result<(), Box<dyn std::error::Error>> {
    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;
    let mut scalar = GeneratedService::new("scalar")?;
    scalar.set_dns_search(GeneratedDnsSearch::Scalar(plain(".")?))?;
    let mut empty = GeneratedService::new("empty")?;
    empty.set_dns_search(GeneratedDnsSearch::List(Vec::new()))?;
    let mut listed = GeneratedService::new("listed")?;
    listed.set_dns_search(GeneratedDnsSearch::List(vec![
        plain("example.internal")?,
        plain("example.internal")?,
        GeneratedString::sensitive("secret.internal")?,
        plain(".")?,
    ]))?;
    assert!(matches!(listed.dns_search(), Some(GeneratedDnsSearch::List(items)) if items.len() == 4));
    assert_eq!(
        listed.set_dns_search(GeneratedDnsSearch::List(Vec::new())),
        Err(GenerationError::DuplicateField("dns_search"))
    );

    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(omitted)?;
    builder.add_service(scalar)?;
    builder.add_service(empty)?;
    builder.add_service(listed)?;
    let generated = builder.build(SourceId::new(690))?;
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("secret.internal"));
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"scalar\":\n",
            "    dns_search: \".\"\n",
            "  \"empty\":\n",
            "    dns_search: []\n",
            "  \"listed\":\n",
            "    dns_search:\n",
            "      - \"example.internal\"\n",
            "      - \"example.internal\"\n",
            "      - \"secret.internal\"\n",
            "      - \".\"\n",
        )
    );
    assert!(
        generated
            .document()
            .service("omitted")
            .is_some_and(|service| service.dns_search().is_none())
    );
    assert!(matches!(
        generated
            .document()
            .service("scalar")
            .and_then(compose_lens::model::Service::dns_search)
            .map(compose_lens::model::DnsSearch::form),
        Some(DnsSearchForm::Scalar(value)) if value.value() == "."
    ));
    assert!(matches!(
        generated
            .document()
            .service("empty")
            .and_then(compose_lens::model::Service::dns_search)
            .map(compose_lens::model::DnsSearch::form),
        Some(DnsSearchForm::List(values)) if values.is_empty()
    ));
    assert!(matches!(
        generated
            .document()
            .service("listed")
            .and_then(compose_lens::model::Service::dns_search)
            .map(compose_lens::model::DnsSearch::form),
        Some(DnsSearchForm::List(values))
            if values.iter().map(|value| value.value().as_str()).collect::<Vec<_>>()
                == ["example.internal", "example.internal", "secret.internal", "."]
    ));
    Ok(())
}

#[test]
fn generated_dns_search_rejects_unresolved_or_physical_line_unsafe_values() -> Result<(), Box<dyn std::error::Error>> {
    for invalid in ["", "${DNS_SEARCH}", "line\nbreak", "line\rbreak"] {
        let mut service = GeneratedService::new("app")?;
        assert_eq!(
            service.set_dns_search(GeneratedDnsSearch::Scalar(plain(invalid)?)),
            Err(GenerationError::InvalidDnsSearchValue)
        );
        assert!(service.dns_search().is_none());
    }
    assert!(matches!(
        GeneratedString::plain("nul\0domain"),
        Err(GenerationError::ContainsNul("string"))
    ));
    Ok(())
}

#[test]
fn generates_ordered_dns_options_with_explicit_empty_and_safe_parse_back() -> Result<(), Box<dyn std::error::Error>> {
    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;
    let mut empty = GeneratedService::new("empty")?;
    empty.set_dns_options(Vec::new())?;
    let mut configured = GeneratedService::new("configured")?;
    configured.set_dns_options(vec![
        plain("ndots:5")?,
        GeneratedString::sensitive("timeout:2")?,
        plain("attempts:3")?,
    ])?;
    assert_eq!(
        configured
            .dns_options()
            .ok_or("configured dns_opt expected")?
            .iter()
            .map(GeneratedString::expose)
            .collect::<Vec<_>>(),
        ["ndots:5", "timeout:2", "attempts:3"]
    );
    assert!(!format!("{configured:?}").contains("timeout:2"));
    assert_eq!(
        configured.set_dns_options(Vec::new()),
        Err(GenerationError::DuplicateField("dns_opt"))
    );

    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(omitted)?;
    builder.add_service(empty)?;
    builder.add_service(configured)?;
    let generated = builder.build(SourceId::new(688))?;
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("timeout:2"));
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"empty\":\n",
            "    dns_opt: []\n",
            "  \"configured\":\n",
            "    dns_opt:\n",
            "      - \"ndots:5\"\n",
            "      - \"timeout:2\"\n",
            "      - \"attempts:3\"\n",
        )
    );
    let parsed = generated
        .document()
        .service("configured")
        .and_then(compose_lens::model::Service::dns_options)
        .ok_or("parse-back dns_opt expected")?;
    assert_eq!(
        parsed
            .items()
            .iter()
            .map(|item| item.value().as_str())
            .collect::<Vec<_>>(),
        ["ndots:5", "timeout:2", "attempts:3"]
    );
    assert!(
        generated
            .document()
            .service("empty")
            .and_then(compose_lens::model::Service::dns_options)
            .is_some_and(|options| options.items().is_empty())
    );
    assert!(
        generated
            .document()
            .service("omitted")
            .is_some_and(|service| service.dns_options().is_none())
    );
    Ok(())
}

#[test]
fn rejects_unsafe_or_duplicate_generated_dns_options() -> Result<(), Box<dyn std::error::Error>> {
    for invalid in ["", "$DNS_OPTION", "line\rbreak", "line\nbreak"] {
        let mut service = GeneratedService::new("app")?;
        assert_eq!(
            service.set_dns_options(vec![plain(invalid)?]),
            Err(GenerationError::InvalidDnsOptionValue)
        );
        assert!(service.dns_options().is_none());
    }
    let mut duplicate = GeneratedService::new("app")?;
    assert_eq!(
        duplicate.set_dns_options(vec![plain("rotate")?, plain("rotate")?]),
        Err(GenerationError::DuplicateItem("dns_opt"))
    );
    assert!(duplicate.dns_options().is_none());
    assert!(matches!(
        GeneratedString::plain("nul\0option"),
        Err(GenerationError::ContainsNul("string"))
    ));
    Ok(())
}

#[test]
fn generates_ordered_ulimits_as_quoted_single_range_and_empty_forms_with_parse_back()
-> Result<(), Box<dyn std::error::Error>> {
    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;
    let mut empty = GeneratedService::new("empty")?;
    empty.set_ulimits(GeneratedUlimits::new(Vec::new())?)?;
    let mut configured = GeneratedService::new("configured")?;
    configured.set_ulimits(GeneratedUlimits::new(vec![
        GeneratedUlimit::single("nproc", plain("0")?)?,
        GeneratedUlimit::range("nofile", GeneratedString::sensitive("1024")?, plain("-1")?)?,
        GeneratedUlimit::single("core", plain("0008")?)?,
    ])?)?;
    assert_eq!(
        configured.ulimits().map(GeneratedUlimits::entries).map(<[_]>::len),
        Some(3)
    );
    assert_eq!(
        configured.set_ulimits(GeneratedUlimits::new(Vec::new())?),
        Err(GenerationError::DuplicateField("ulimits"))
    );

    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(omitted)?;
    builder.add_service(empty)?;
    builder.add_service(configured)?;
    let generated = builder.build(SourceId::new(680))?;
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("1024"));
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"empty\":\n",
            "    ulimits: {}\n",
            "  \"configured\":\n",
            "    ulimits:\n",
            "      \"nproc\": \"0\"\n",
            "      \"nofile\":\n",
            "        soft: \"1024\"\n",
            "        hard: \"-1\"\n",
            "      \"core\": \"0008\"\n",
        )
    );
    assert!(
        generated
            .document()
            .service("omitted")
            .and_then(compose_lens::model::Service::ulimits)
            .is_none()
    );
    assert!(
        generated
            .document()
            .service("empty")
            .and_then(compose_lens::model::Service::ulimits)
            .is_some_and(|limits| limits.entries().is_empty())
    );
    let limits = generated
        .document()
        .service("configured")
        .and_then(compose_lens::model::Service::ulimits)
        .ok_or("parse-back ulimits expected")?;
    assert_eq!(
        limits
            .entries()
            .iter()
            .map(|limit| limit.name().value())
            .collect::<Vec<_>>(),
        ["nproc", "nofile", "core"]
    );
    assert!(matches!(limits.entries()[0].value(), UlimitValue::Single(value) if value.value().raw() == "0"));
    assert!(matches!(limits.entries()[1].value(), UlimitValue::Range(range)
        if range.soft().is_some_and(|value| value.value().raw() == "1024")
            && range.hard().is_some_and(|value| value.value().raw() == "-1")));
    Ok(())
}

#[test]
fn rejects_unsafe_duplicate_or_incomplete_generated_ulimits() -> Result<(), Box<dyn std::error::Error>> {
    for name in ["", "NoFile", "no_file", "nofile1", "ä"] {
        assert_eq!(
            GeneratedUlimit::single(name, plain("1")?),
            Err(GenerationError::InvalidUlimitName),
            "expected invalid name {name:?}"
        );
    }
    for value in ["", "+1", "-2", "1.0", "1e3", "host", "${LIMIT}", "1\n2", "1\r2"] {
        assert_eq!(
            GeneratedUlimit::single("nofile", plain(value)?),
            Err(GenerationError::InvalidUlimitValue),
            "expected invalid value {value:?}"
        );
    }
    assert_eq!(
        GeneratedString::plain("1\0limit"),
        Err(GenerationError::ContainsNul("string"))
    );
    assert_eq!(
        GeneratedUlimit::new(
            "nofile",
            GeneratedUlimitValue::Range {
                soft: None,
                hard: Some(plain("1")?),
            },
        ),
        Err(GenerationError::MissingUlimitRangeMember("soft"))
    );
    assert_eq!(
        GeneratedUlimit::new(
            "nofile",
            GeneratedUlimitValue::Range {
                soft: Some(plain("1")?),
                hard: None,
            },
        ),
        Err(GenerationError::MissingUlimitRangeMember("hard"))
    );
    assert_eq!(
        GeneratedUlimits::new(vec![
            GeneratedUlimit::single("nofile", plain("1")?)?,
            GeneratedUlimit::single("nofile", plain("2")?)?,
        ]),
        Err(GenerationError::DuplicateName {
            kind: "ulimit",
            name: "nofile".to_owned(),
        })
    );
    Ok(())
}

#[test]
fn generates_sysctls_forms_as_safe_quoted_strings_with_parse_back_and_redaction()
-> Result<(), Box<dyn std::error::Error>> {
    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;

    let mut empty_map = GeneratedService::new("empty-map")?;
    empty_map.set_sysctls(GeneratedSysctls::Map(Vec::new()))?;
    let mut empty_list = GeneratedService::new("empty-list")?;
    empty_list.set_sysctls(GeneratedSysctls::List(Vec::new()))?;

    let mut mapping = GeneratedService::new("mapping")?;
    mapping.set_sysctls(GeneratedSysctls::Map(vec![
        GeneratedSysctl::new("net.ipv4.ip_forward", plain("1")?)?,
        GeneratedSysctl::new("fs.protected_hardlinks", plain("true")?)?,
        GeneratedSysctl::new("kernel.shm_rmid_forced", GeneratedString::sensitive("0")?)?,
        GeneratedSysctl::new("net.ipv6.conf.all.disable_ipv6", plain("null")?)?,
    ]))?;
    assert!(matches!(mapping.sysctls(), Some(GeneratedSysctls::Map(entries)) if entries.len() == 4));
    assert_eq!(
        mapping.set_sysctls(GeneratedSysctls::List(Vec::new())),
        Err(GenerationError::DuplicateField("sysctls"))
    );

    let mut list = GeneratedService::new("list")?;
    list.set_sysctls(GeneratedSysctls::List(vec![
        plain("net.core.somaxconn=1024")?,
        plain("")?,
    ]))?;

    let mut project = ComposeDocumentBuilder::new();
    project.add_service(omitted)?;
    project.add_service(empty_map)?;
    project.add_service(empty_list)?;
    project.add_service(mapping)?;
    project.add_service(list)?;
    let generated = project.build(SourceId::new(712))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"empty-map\":\n",
            "    sysctls: {}\n",
            "  \"empty-list\":\n",
            "    sysctls: []\n",
            "  \"mapping\":\n",
            "    sysctls:\n",
            "      \"net.ipv4.ip_forward\": \"1\"\n",
            "      \"fs.protected_hardlinks\": \"true\"\n",
            "      \"kernel.shm_rmid_forced\": \"0\"\n",
            "      \"net.ipv6.conf.all.disable_ipv6\": \"null\"\n",
            "  \"list\":\n",
            "    sysctls:\n",
            "      - \"net.core.somaxconn=1024\"\n",
            "      - \"\"\n",
        )
    );
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("kernel.shm_rmid_forced"));

    let service = generated
        .document()
        .service("mapping")
        .ok_or("mapping service expected")?;
    let SysctlsForm::Map(entries) = service
        .sysctls()
        .map(compose_lens::model::Sysctls::form)
        .ok_or("mapping sysctls expected")?
    else {
        return Err("mapping form expected".into());
    };
    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry.value().value(), ComposeScalar::String(_)))
    );
    assert!(matches!(
        generated
            .document()
            .service("list")
            .and_then(compose_lens::model::Service::sysctls)
            .map(compose_lens::model::Sysctls::form),
        Some(SysctlsForm::List(items)) if items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>()
            == ["net.core.somaxconn=1024", ""]
    ));
    Ok(())
}

#[test]
fn rejects_unsafe_or_duplicate_generated_sysctls_without_runtime_validation() -> Result<(), Box<dyn std::error::Error>>
{
    assert_eq!(
        GeneratedSysctl::new("", plain("value")?),
        Err(GenerationError::InvalidSysctlName)
    );
    assert_eq!(
        GeneratedSysctl::new("name\nnext", plain("value")?),
        Err(GenerationError::InvalidSysctlName)
    );
    assert_eq!(
        GeneratedSysctl::new("${NAME}", plain("value")?),
        Err(GenerationError::InvalidSysctlName)
    );
    assert_eq!(
        GeneratedSysctl::new("name", plain("value\nnext")?),
        Err(GenerationError::InvalidSysctlValue)
    );
    assert_eq!(
        GeneratedSysctl::new("name", plain("${VALUE}")?),
        Err(GenerationError::InvalidSysctlValue)
    );
    assert_eq!(
        GeneratedString::plain("value\0suffix"),
        Err(GenerationError::ContainsNul("string"))
    );

    let mut duplicate_map = GeneratedService::new("map")?;
    assert_eq!(
        duplicate_map.set_sysctls(GeneratedSysctls::Map(vec![
            GeneratedSysctl::new("unrecognized.namespace", plain("opaque")?)?,
            GeneratedSysctl::new("unrecognized.namespace", plain("different")?)?,
        ])),
        Err(GenerationError::DuplicateName {
            kind: "sysctl",
            name: "unrecognized.namespace".to_owned(),
        })
    );

    let mut duplicate_list = GeneratedService::new("list")?;
    assert_eq!(
        duplicate_list.set_sysctls(GeneratedSysctls::List(vec![plain("same=value")?, plain("same=value")?])),
        Err(GenerationError::DuplicateItem("sysctls"))
    );
    assert_eq!(
        duplicate_list.set_sysctls(GeneratedSysctls::List(vec![plain("${DEFERRED}")?])),
        Err(GenerationError::InvalidSysctlValue)
    );
    assert_eq!(
        duplicate_list.set_sysctls(GeneratedSysctls::List(vec![plain("line\rbreak")?])),
        Err(GenerationError::InvalidSysctlValue)
    );
    Ok(())
}

#[test]
fn generates_the_runtime_migration_subset_deterministically() -> Result<(), Box<dyn std::error::Error>> {
    let project = complete_project()?;
    let builder_debug = format!("{project:?}");
    assert!(!builder_debug.contains("production-secret"));
    assert!(!builder_debug.contains("1001:1002"));
    assert!(!builder_debug.contains("/usr/bin/env"));
    let generated = project.clone().build(SourceId::new(701))?;
    let repeated = project.build(SourceId::new(702))?;

    assert_eq!(generated.text(), expected_document());
    assert_eq!(generated.text(), repeated.text());
    assert!(generated.is_sensitive());
    let service = generated
        .document()
        .service("web")
        .ok_or("generated service expected")?;
    assert_eq!(
        service.container_name().map(|name| name.value().as_str()),
        Some("application-web")
    );
    assert!(matches!(
        service.restart().map(compose_lens::model::RestartPolicy::kind),
        Some(compose_lens::model::RestartPolicyKind::OnFailure {
            maximum_retries: Some(value),
        }) if value == "3"
    ));
    assert_eq!(
        service.image().map(|image| image.value().raw()),
        Some("example.invalid/web:1@sha256:abcd")
    );
    assert!(matches!(service.entrypoint(), Some(Entrypoint::List { values, .. }) if values.len() == 2));
    assert!(matches!(service.command(), Some(Command::List { values, .. }) if values.len() == 2));
    assert_eq!(
        service.init().map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(true))
    );
    assert!(matches!(service.environment(), Some(Environment::List { entries, .. }) if entries.len() == 3));
    assert!(matches!(service.labels(), Some(Labels::Map { entries, .. }) if entries.len() == 3));
    assert!(matches!(service.extra_hosts(), Some(ExtraHosts::Short { entries, .. }) if entries.len() == 2));
    assert_eq!(service.ports().len(), 1);
    assert_eq!(service.volumes().len(), 4);
    assert!(matches!(service.volumes()[1], VolumeMount::Short(_)));
    assert!(
        matches!(service.networks(), Some(ServiceNetworks::Long { networks, .. }) if networks[0].aliases().len() == 2)
    );
    assert_eq!(
        generated.document().networks()[0]
            .custom_name()
            .map(|name| name.value().as_str()),
        Some("observed-frontend")
    );
    assert_eq!(
        generated.document().volumes()[0]
            .custom_name()
            .map(|name| name.value().as_str()),
        Some("observed-data")
    );

    let debug = format!("{generated:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("production-secret"));
    assert!(!debug.contains("private-label-value"));
    assert!(!debug.contains("1001:1002"));
    Ok(())
}

#[test]
fn generates_cap_drop_with_explicit_empty_order_validation_and_redaction() -> Result<(), Box<dyn std::error::Error>> {
    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;

    let mut empty = GeneratedService::new("empty")?;
    empty.set_cap_drop(Vec::new())?;
    assert!(empty.cap_drop().is_some_and(<[GeneratedString]>::is_empty));

    let mut configured = GeneratedService::new("configured")?;
    configured.set_cap_drop(vec![
        plain("NET_ADMIN")?,
        GeneratedString::sensitive("net_admin")?,
        plain("CAP WITH SPACE")?,
    ])?;
    assert_eq!(
        configured
            .cap_drop()
            .ok_or("configured vector expected")?
            .iter()
            .map(GeneratedString::expose)
            .collect::<Vec<_>>(),
        ["NET_ADMIN", "net_admin", "CAP WITH SPACE"]
    );
    assert_eq!(
        configured.set_cap_drop(vec![plain("CHOWN")?]),
        Err(GenerationError::DuplicateField("cap_drop"))
    );

    let mut project = ComposeDocumentBuilder::new();
    project.add_service(omitted)?;
    project.add_service(empty)?;
    project.add_service(configured)?;
    let generated = project.build(SourceId::new(706))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"empty\":\n",
            "    cap_drop: []\n",
            "  \"configured\":\n",
            "    cap_drop:\n",
            "      - \"NET_ADMIN\"\n",
            "      - \"net_admin\"\n",
            "      - \"CAP WITH SPACE\"\n",
        )
    );
    let generated_empty = generated
        .document()
        .service("empty")
        .and_then(compose_lens::model::Service::cap_drop)
        .ok_or("parse-back explicit empty cap_drop expected")?;
    assert!(generated_empty.items().is_empty());
    assert!(
        generated
            .document()
            .service("omitted")
            .ok_or("parse-back omitted service expected")?
            .cap_drop()
            .is_none()
    );
    assert_eq!(
        generated
            .document()
            .service("configured")
            .and_then(compose_lens::model::Service::cap_drop)
            .ok_or("parse-back configured cap_drop expected")?
            .items()
            .iter()
            .map(compose_lens::model::CapabilityDropItem::value)
            .collect::<Vec<_>>(),
        ["NET_ADMIN", "net_admin", "CAP WITH SPACE"]
    );
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("NET_ADMIN"));

    assert_invalid_generated_cap_drop()?;
    Ok(())
}

fn assert_invalid_generated_cap_drop() -> Result<(), Box<dyn std::error::Error>> {
    let mut invalid = GeneratedService::new("invalid")?;
    assert_eq!(
        invalid.set_cap_drop(vec![plain("CHOWN")?, plain("CHOWN")?]),
        Err(GenerationError::DuplicateItem("cap_drop"))
    );
    assert!(invalid.cap_drop().is_none());
    assert_eq!(
        invalid.set_cap_drop(vec![plain("")?]),
        Err(GenerationError::EmptyValue("cap_drop item"))
    );
    for line_break in ["NET\nADMIN", "NET\rADMIN"] {
        assert_eq!(
            invalid.set_cap_drop(vec![plain(line_break)?]),
            Err(GenerationError::ContainsLineBreak("cap_drop item"))
        );
    }
    assert_eq!(
        GeneratedString::plain("NET\0ADMIN"),
        Err(GenerationError::ContainsNul("string"))
    );
    invalid.set_cap_drop(vec![plain("CHOWN")?, plain("chown")?])?;
    Ok(())
}

#[test]
fn generates_cap_add_without_rewriting_independent_cap_drop() -> Result<(), Box<dyn std::error::Error>> {
    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;

    let mut empty = GeneratedService::new("empty")?;
    empty.set_cap_add(Vec::new())?;
    assert!(empty.cap_add().is_some_and(<[GeneratedString]>::is_empty));

    let mut configured = GeneratedService::new("configured")?;
    configured.set_cap_add(vec![
        plain("NET_ADMIN")?,
        GeneratedString::sensitive("net_admin")?,
        plain("CAP WITH SPACE")?,
        plain("vendor.capability/token")?,
    ])?;
    configured.set_cap_drop(vec![plain("MKNOD")?])?;
    assert_eq!(
        configured
            .cap_add()
            .ok_or("configured vector expected")?
            .iter()
            .map(GeneratedString::expose)
            .collect::<Vec<_>>(),
        ["NET_ADMIN", "net_admin", "CAP WITH SPACE", "vendor.capability/token"]
    );
    assert_eq!(
        configured.set_cap_add(vec![plain("CHOWN")?]),
        Err(GenerationError::DuplicateField("cap_add"))
    );

    let mut project = ComposeDocumentBuilder::new();
    project.add_service(omitted)?;
    project.add_service(empty)?;
    project.add_service(configured)?;
    let generated = project.build(SourceId::new(707))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"empty\":\n",
            "    cap_add: []\n",
            "  \"configured\":\n",
            "    cap_add:\n",
            "      - \"NET_ADMIN\"\n",
            "      - \"net_admin\"\n",
            "      - \"CAP WITH SPACE\"\n",
            "      - \"vendor.capability/token\"\n",
            "    cap_drop:\n",
            "      - \"MKNOD\"\n",
        )
    );
    assert!(
        generated
            .document()
            .service("empty")
            .and_then(compose_lens::model::Service::cap_add)
            .is_some_and(|capabilities| capabilities.items().is_empty())
    );
    assert!(
        generated
            .document()
            .service("omitted")
            .ok_or("omitted service expected")?
            .cap_add()
            .is_none()
    );
    let parsed = generated
        .document()
        .service("configured")
        .ok_or("configured service expected")?;
    assert_eq!(
        parsed
            .cap_add()
            .ok_or("parse-back configured cap_add expected")?
            .items()
            .iter()
            .map(compose_lens::model::CapabilityAddItem::value)
            .collect::<Vec<_>>(),
        ["NET_ADMIN", "net_admin", "CAP WITH SPACE", "vendor.capability/token"]
    );
    assert_eq!(
        parsed
            .cap_drop()
            .ok_or("parse-back independent cap_drop expected")?
            .items()[0]
            .value(),
        "MKNOD"
    );
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("NET_ADMIN"));

    assert_invalid_generated_cap_add()?;
    Ok(())
}

fn assert_invalid_generated_cap_add() -> Result<(), Box<dyn std::error::Error>> {
    let mut invalid = GeneratedService::new("invalid")?;
    assert_eq!(
        invalid.set_cap_add(vec![plain("CHOWN")?, plain("CHOWN")?]),
        Err(GenerationError::DuplicateItem("cap_add"))
    );
    assert!(invalid.cap_add().is_none());
    assert_eq!(
        invalid.set_cap_add(vec![plain("")?]),
        Err(GenerationError::EmptyValue("cap_add item"))
    );
    for line_break in ["NET\nADMIN", "NET\rADMIN"] {
        assert_eq!(
            invalid.set_cap_add(vec![plain(line_break)?]),
            Err(GenerationError::ContainsLineBreak("cap_add item"))
        );
    }
    assert_eq!(
        GeneratedString::plain("NET\0ADMIN"),
        Err(GenerationError::ContainsNul("string"))
    );
    invalid.set_cap_add(vec![plain("CHOWN")?, plain("chown")?])?;
    Ok(())
}

#[test]
fn retains_empty_shell_protocol_and_shared_selinux_variants() -> Result<(), Box<dyn std::error::Error>> {
    let mut shell = GeneratedService::new("shell")?;
    shell.set_image(plain("example.invalid/shell:1")?)?;
    shell.set_entrypoint(GeneratedEntrypoint::String(plain("")?))?;
    shell.set_command(GeneratedCommand::Shell(plain("echo hello")?))?;
    shell.set_init(false)?;
    shell.set_read_only(false)?;
    shell.add_port(GeneratedPort::new(53, Some(1053), None, GeneratedProtocol::Udp)?);
    shell.add_mount(GeneratedMount::bind(
        "/srv/shared",
        "/shared",
        false,
        Some(GeneratedSelinux::Shared),
    )?);
    shell.add_network(GeneratedNetworkAttachment::new("frontend")?)?;

    let mut empty = GeneratedService::new("empty")?;
    empty.set_image(plain("example.invalid/empty:1")?)?;
    empty.set_entrypoint(GeneratedEntrypoint::Empty)?;
    empty.set_command(GeneratedCommand::Empty)?;
    empty.add_port(GeneratedPort::new(5000, None, None, GeneratedProtocol::Sctp)?);
    empty.add_port(GeneratedPort::new(
        5001,
        Some(15001),
        Some("::1".to_owned()),
        GeneratedProtocol::Sctp,
    )?);

    let mut project = ComposeDocumentBuilder::new();
    project.add_service(shell)?;
    project.add_service(empty)?;
    let generated = project.build(SourceId::new(705))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"shell\":\n",
            "    image: \"example.invalid/shell:1\"\n",
            "    entrypoint: \"\"\n",
            "    command: \"echo hello\"\n",
            "    init: false\n",
            "    read_only: false\n",
            "    ports:\n",
            "      - target: 53\n",
            "        published: \"1053\"\n",
            "        protocol: \"udp\"\n",
            "    volumes:\n",
            "      - \"/srv/shared:/shared:z\"\n",
            "    networks:\n",
            "      \"frontend\": {}\n",
            "  \"empty\":\n",
            "    image: \"example.invalid/empty:1\"\n",
            "    entrypoint: []\n",
            "    command: []\n",
            "    ports:\n",
            "      - \"5000/sctp\"\n",
            "      - \"[::1]:15001:5001/sctp\"\n",
        )
    );
    assert!(
        matches!(generated.document().service("empty").and_then(|service| service.command()), Some(Command::List { values, .. }) if values.is_empty())
    );
    assert!(
        matches!(generated.document().service("empty").and_then(|service| service.entrypoint()), Some(Entrypoint::List { values, .. }) if values.is_empty())
    );
    assert!(matches!(
        generated.document().service("shell").and_then(|service| service.entrypoint()),
        Some(Entrypoint::String(value)) if value.value().is_empty()
    ));
    assert_eq!(
        generated
            .document()
            .service("shell")
            .and_then(compose_lens::model::Service::init)
            .map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(false))
    );
    assert!(
        generated
            .document()
            .service("empty")
            .and_then(compose_lens::model::Service::init)
            .is_none()
    );
    assert!(matches!(
        generated.document().service("empty").and_then(|service| service.ports().first()),
        Some(Port::Short(port)) if port.target() == "5000" && port.protocol() == Some("sctp")
    ));
    assert!(matches!(
        generated.document().service("empty").and_then(|service| service.ports().get(1)),
        Some(Port::Short(port))
            if port.host_ip() == Some("[::1]")
                && port.published() == Some("15001")
                && port.target() == "5001"
                && port.protocol() == Some("sctp")
    ));
    Ok(())
}

#[test]
fn generates_only_resolved_valid_hostnames_and_parses_back_the_exact_value() -> Result<(), Box<dyn std::error::Error>> {
    let mut service = GeneratedService::new("app")?;
    service.set_hostname(GeneratedHostname::Resolved(GeneratedString::sensitive(
        "3API.Example-Corp.COM",
    )?))?;
    service.set_image(plain("example.invalid/app:1")?)?;
    let mut project = ComposeDocumentBuilder::new();
    project.add_service(service)?;

    let generated = project.build(SourceId::new(723))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"app\":\n",
            "    hostname: \"3API.Example-Corp.COM\"\n",
            "    image: \"example.invalid/app:1\"\n",
        )
    );
    let hostname = generated
        .document()
        .service("app")
        .and_then(compose_lens::model::Service::hostname)
        .ok_or("parse-back hostname expected")?;
    assert_eq!(hostname.raw().value(), "3API.Example-Corp.COM");
    assert_eq!(hostname.kind(), &HostnameKind::Resolved);
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("3API.Example-Corp.COM"));
    Ok(())
}

#[test]
fn rejects_empty_deferred_and_invalid_generated_hostnames_before_rendering() -> Result<(), Box<dyn std::error::Error>> {
    let label_64 = "a".repeat(64);
    let label_63 = "a".repeat(63);
    let too_long = format!("{label_63}.{label_63}.{label_63}.{}", "a".repeat(62));
    let mut service = GeneratedService::new("app")?;
    for invalid in [
        "",
        "${SERVICE_HOSTNAME}",
        "literal$marker",
        "invalid_host",
        "example.",
        "example..com",
        "-example",
        "example-",
        "café.example",
        label_64.as_str(),
        too_long.as_str(),
    ] {
        assert_eq!(
            service.set_hostname(GeneratedHostname::Resolved(plain(invalid)?)),
            Err(GenerationError::InvalidHostname),
            "unexpected generation result for {invalid}"
        );
    }
    service.set_hostname(GeneratedHostname::Resolved(plain("valid.example")?))?;
    assert_eq!(
        service.set_hostname(GeneratedHostname::Resolved(plain("replacement.example")?)),
        Err(GenerationError::DuplicateField("hostname"))
    );
    Ok(())
}

#[test]
fn generates_ordered_environment_file_short_and_long_forms() -> Result<(), Box<dyn std::error::Error>> {
    let mut service = GeneratedService::new("web")?;
    service.set_image(plain("example.invalid/web:1")?)?;
    service.add_environment_file(GeneratedEnvironmentFile::short(plain("./default.env")?)?);
    service.add_environment_file(GeneratedEnvironmentFile::long(
        GeneratedString::sensitive("/run/credentials/private.env")?,
        Some(false),
        Some(GeneratedEnvironmentFileFormat::Raw),
    )?);
    let mut project = ComposeDocumentBuilder::new();
    project.add_service(service)?;

    let generated = project.build(SourceId::new(704))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"web\":\n",
            "    image: \"example.invalid/web:1\"\n",
            "    env_file:\n",
            "      - \"./default.env\"\n",
            "      - path: \"/run/credentials/private.env\"\n",
            "        required: false\n",
            "        format: \"raw\"\n",
        )
    );
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("private.env"));

    let environment_files = generated
        .document()
        .service("web")
        .ok_or("generated service expected")?
        .environment_files();
    assert!(matches!(
        &environment_files[0],
        EnvironmentFile::Short(path) if path.value() == "./default.env"
    ));
    let EnvironmentFile::Long(long) = &environment_files[1] else {
        return Err("generated long environment file expected".into());
    };
    assert_eq!(
        long.required().map(compose_lens::model::Located::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(
        long.format().map(compose_lens::model::EnvironmentFileFormat::kind),
        Some(EnvironmentFileFormatKind::Raw)
    );
    Ok(())
}

#[test]
fn generates_every_service_restart_policy_form() -> Result<(), Box<dyn std::error::Error>> {
    let policies = [
        ("disabled", GeneratedRestartPolicy::No, "no"),
        ("always", GeneratedRestartPolicy::Always, "always"),
        (
            "failure",
            GeneratedRestartPolicy::OnFailure { maximum_retries: None },
            "on-failure",
        ),
        (
            "limited",
            GeneratedRestartPolicy::OnFailure {
                maximum_retries: Some(3),
            },
            "on-failure:3",
        ),
        ("stopped", GeneratedRestartPolicy::UnlessStopped, "unless-stopped"),
    ];
    let mut project = ComposeDocumentBuilder::new();
    for (name, policy, _) in policies {
        let mut service = GeneratedService::new(name)?;
        service.set_restart(policy)?;
        project.add_service(service)?;
    }

    let generated = project.build(SourceId::new(703))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"disabled\":\n",
            "    restart: \"no\"\n",
            "  \"always\":\n",
            "    restart: \"always\"\n",
            "  \"failure\":\n",
            "    restart: \"on-failure\"\n",
            "  \"limited\":\n",
            "    restart: \"on-failure:3\"\n",
            "  \"stopped\":\n",
            "    restart: \"unless-stopped\"\n",
        )
    );
    for (name, _, expected) in policies {
        assert_eq!(
            generated
                .document()
                .service(name)
                .and_then(compose_lens::model::Service::restart)
                .map(compose_lens::model::RestartPolicy::raw)
                .map(compose_lens::model::Located::value)
                .map(String::as_str),
            Some(expected)
        );
    }
    Ok(())
}

#[test]
fn generates_lifecycle_fields_with_authored_duration_spelling_and_parse_back_validation()
-> Result<(), Box<dyn std::error::Error>> {
    let services = [
        ("omitted", None, None),
        ("empty", Some(""), None),
        ("named", Some("SIGUSR1"), Some("1s")),
        ("numeric", Some("15"), Some("1m30s")),
        ("zero", None, Some("0s")),
        ("fractional", None, Some("1.5s")),
        ("deferred", None, Some("${STOP_GRACE_PERIOD:-1s}")),
    ];
    let mut project = ComposeDocumentBuilder::new();
    for (name, signal, period) in services {
        let mut service = GeneratedService::new(name)?;
        if name == "omitted" {
            service.set_image(plain("example.invalid/omitted:1")?)?;
        }
        if let Some(signal) = signal {
            service.set_stop_signal(plain(signal)?)?;
        }
        if let Some(period) = period {
            let period = if name == "numeric" {
                GeneratedString::sensitive(period)?
            } else {
                plain(period)?
            };
            service.set_stop_grace_period(period)?;
        }
        project.add_service(service)?;
    }

    let generated = project.build(SourceId::new(704))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"empty\":\n",
            "    stop_signal: \"\"\n",
            "  \"named\":\n",
            "    stop_signal: \"SIGUSR1\"\n",
            "    stop_grace_period: \"1s\"\n",
            "  \"numeric\":\n",
            "    stop_signal: \"15\"\n",
            "    stop_grace_period: \"1m30s\"\n",
            "  \"zero\":\n",
            "    stop_grace_period: \"0s\"\n",
            "  \"fractional\":\n",
            "    stop_grace_period: \"1.5s\"\n",
            "  \"deferred\":\n",
            "    stop_grace_period: \"${STOP_GRACE_PERIOD:-1s}\"\n",
        )
    );
    let omitted = generated
        .document()
        .service("omitted")
        .ok_or("omitted service expected")?;
    assert!(omitted.stop_signal().is_none());
    assert!(omitted.stop_grace_period().is_none());
    assert_generated_stop_signal(&generated, "empty", "");
    assert_generated_stop_signal(&generated, "named", "SIGUSR1");
    for (name, expected) in [
        ("named", "1s"),
        ("numeric", "1m30s"),
        ("zero", "0s"),
        ("fractional", "1.5s"),
    ] {
        assert!(matches!(
            generated
                .document()
                .service(name)
                .and_then(compose_lens::model::Service::stop_grace_period)
                .map(compose_lens::model::Located::value),
            Some(StopGracePeriod::Value(value)) if value == expected
        ));
    }
    assert!(matches!(
        generated
            .document()
            .service("deferred")
            .and_then(compose_lens::model::Service::stop_grace_period)
            .map(compose_lens::model::Located::value),
        Some(StopGracePeriod::Expression(value)) if value == "${STOP_GRACE_PERIOD:-1s}"
    ));
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("1m30s"));
    Ok(())
}

fn assert_generated_stop_signal(generated: &GeneratedComposeDocument, service: &str, expected: &str) {
    assert_eq!(
        generated
            .document()
            .service(service)
            .and_then(compose_lens::model::Service::stop_signal)
            .map(compose_lens::model::Located::value)
            .map(String::as_str),
        Some(expected)
    );
}

#[test]
fn rejects_ambiguous_or_duplicate_generation_requests() -> Result<(), Box<dyn std::error::Error>> {
    let empty = ComposeDocumentBuilder::new();
    assert_eq!(empty.build(SourceId::new(710)), Err(GenerationError::MissingService));

    let mut service = GeneratedService::new("web")?;
    service.set_container_name(plain("application-web")?)?;
    service.set_image(plain("example.invalid/web:1")?)?;
    assert_eq!(
        service.set_image(plain("example.invalid/web:2")?),
        Err(GenerationError::DuplicateField("image"))
    );
    assert_eq!(
        service.set_container_name(plain("replacement-web")?),
        Err(GenerationError::DuplicateField("container_name"))
    );
    service.set_restart(GeneratedRestartPolicy::Always)?;
    assert_eq!(
        service.set_restart(GeneratedRestartPolicy::No),
        Err(GenerationError::DuplicateField("restart"))
    );
    assert_invalid_lifecycle_generation(&mut service)?;
    service.set_init(true)?;
    assert_eq!(service.set_init(false), Err(GenerationError::DuplicateField("init")));
    assert_eq!(
        GeneratedEnvironment::literal("INVALID=NAME", plain("value")?),
        Err(GenerationError::InvalidEnvironmentName)
    );
    assert_eq!(
        GeneratedEnvironmentFile::short(plain("")?),
        Err(GenerationError::EmptyValue("environment-file path"))
    );
    assert_eq!(
        GeneratedLabel::new("", plain("value")?),
        Err(GenerationError::EmptyValue("label name"))
    );
    let mut invalid_service = GeneratedService::new("invalid")?;
    assert_eq!(
        invalid_service.set_container_name(plain("invalid name")?),
        Err(GenerationError::InvalidContainerName)
    );
    assert_eq!(
        invalid_service.set_container_name(plain("a")?),
        Err(GenerationError::InvalidContainerName)
    );
    assert_eq!(
        invalid_service.set_image(plain("")?),
        Err(GenerationError::EmptyValue("service image"))
    );
    assert_eq!(
        invalid_service.add_supplementary_group(plain("")?),
        Err(GenerationError::EmptyValue("supplementary group"))
    );
    invalid_service.add_label(GeneratedLabel::new("com.example.duplicate", plain("first")?)?)?;
    assert_eq!(
        invalid_service.add_label(GeneratedLabel::new("com.example.duplicate", plain("second")?)?),
        Err(GenerationError::DuplicateName {
            kind: "service label",
            name: "com.example.duplicate".to_owned(),
        })
    );
    assert_eq!(
        GeneratedExtraHost::new("invalid=name", "127.0.0.1"),
        Err(GenerationError::InvalidShortComponent("extra-host hostname"))
    );
    assert_eq!(
        GeneratedMount::bind("C:\\data", "/data", false, Some(GeneratedSelinux::Private)),
        Err(GenerationError::InvalidSelinuxBind)
    );
    assert_eq!(
        GeneratedPort::new(5000, None, Some("127.0.0.1".to_owned()), GeneratedProtocol::Sctp,),
        Err(GenerationError::UnrepresentableSctpHostIp)
    );

    let mut resource = GeneratedResource::application("data")?;
    resource.set_custom_name("observed-data")?;
    assert_eq!(
        resource.set_custom_name("replacement-data"),
        Err(GenerationError::DuplicateField("resource name"))
    );

    let mut project = ComposeDocumentBuilder::new();
    project.add_service(service.clone())?;
    assert_eq!(
        project.add_service(service),
        Err(GenerationError::DuplicateName {
            kind: "service",
            name: "web".to_owned(),
        })
    );
    Ok(())
}

fn assert_invalid_lifecycle_generation(service: &mut GeneratedService) -> Result<(), Box<dyn std::error::Error>> {
    service.set_stop_signal(plain("SIGTERM")?)?;
    assert_eq!(
        service.set_stop_signal(plain("SIGKILL")?),
        Err(GenerationError::DuplicateField("stop_signal"))
    );
    for invalid in ["0", "1ns", "1µs", "1μs", "1d", "1.s", "malformed"] {
        assert_eq!(
            service.set_stop_grace_period(plain(invalid)?),
            Err(GenerationError::InvalidStopGracePeriod)
        );
    }
    service.set_stop_grace_period(plain("0s")?)?;
    assert_eq!(
        service.set_stop_grace_period(plain("1s")?),
        Err(GenerationError::DuplicateField("stop_grace_period"))
    );
    Ok(())
}

#[test]
fn non_sensitive_output_remains_reviewable_in_debug() -> Result<(), Box<dyn std::error::Error>> {
    let mut service = GeneratedService::new("web")?;
    service.set_image(plain("example.invalid/web:1")?)?;
    let mut project = ComposeDocumentBuilder::new();
    project.add_service(service)?;
    let generated = project.build(SourceId::new(720))?;

    assert!(!generated.is_sensitive());
    assert!(format!("{generated:?}").contains("example.invalid/web:1"));
    Ok(())
}

fn complete_project() -> Result<ComposeDocumentBuilder, Box<dyn std::error::Error>> {
    let mut service = GeneratedService::new("web")?;
    service.set_container_name(plain("application-web")?)?;
    service.set_image(plain("example.invalid/web:1@sha256:abcd")?)?;
    service.set_entrypoint(GeneratedEntrypoint::List(vec![
        GeneratedString::sensitive("/usr/bin/env")?,
        plain("php")?,
    ]))?;
    service.set_command(GeneratedCommand::Exec(vec![plain("server")?, plain("--foreground")?]))?;
    service.set_init(true)?;
    service.add_environment(GeneratedEnvironment::literal(
        "MODE",
        GeneratedString::sensitive("production-secret")?,
    )?);
    service.add_environment(GeneratedEnvironment::host("FROM_HOST")?);
    service.add_environment(GeneratedEnvironment::literal("MODE", plain("last-wins")?)?);
    service.add_label(GeneratedLabel::new("com.example.purpose", plain("runtime=migration")?)?)?;
    service.add_label(GeneratedLabel::new("com.example.empty", plain("")?)?)?;
    service.add_label(GeneratedLabel::new(
        "com.example.secret",
        GeneratedString::sensitive("private-label-value")?,
    )?)?;
    service.set_user(GeneratedString::sensitive("1001:1002")?)?;
    service.set_userns_mode(plain("keep-id")?)?;
    service.add_supplementary_group(plain("audio")?)?;
    service.add_supplementary_group(plain("44")?)?;
    service.set_working_dir(plain("/srv/app")?)?;
    service.set_read_only(true)?;
    service.set_restart(GeneratedRestartPolicy::OnFailure {
        maximum_retries: Some(3),
    })?;
    service.add_extra_host(GeneratedExtraHost::new("host.docker.internal", "host-gateway")?);
    service.add_extra_host(GeneratedExtraHost::new("ipv6", "::1")?);
    service.add_port(GeneratedPort::new(
        8080,
        Some(18080),
        Some("127.0.0.1".to_owned()),
        GeneratedProtocol::Tcp,
    )?);
    service.add_mount(GeneratedMount::volume("data", "/var/lib/data", true)?);
    service.add_mount(GeneratedMount::bind(
        "/srv/data",
        "/data",
        true,
        Some(GeneratedSelinux::Private),
    )?);
    service.add_mount(GeneratedMount::bind("/srv/config", "/etc/config", false, None)?);
    service.add_mount(GeneratedMount::anonymous("/cache", false)?);
    let mut network = GeneratedNetworkAttachment::new("frontend")?;
    network.add_alias("web")?;
    network.add_alias("public-api")?;
    service.add_network(network)?;

    let mut project = ComposeDocumentBuilder::new();
    project.set_name("example")?;
    project.add_service(service)?;
    let mut network = GeneratedResource::application("frontend")?;
    network.set_custom_name("observed-frontend")?;
    project.add_network(network)?;
    let mut volume = GeneratedResource::external("data")?;
    volume.set_custom_name("observed-data")?;
    project.add_volume(volume)?;
    Ok(project)
}

fn plain(value: &str) -> Result<GeneratedString, GenerationError> {
    GeneratedString::plain(value)
}

fn generated_network<'a>(
    generated: &'a GeneratedComposeDocument,
    service_name: &str,
) -> Result<&'a compose_lens::model::ServiceNetwork, &'static str> {
    match generated
        .document()
        .service(service_name)
        .and_then(compose_lens::model::Service::networks)
    {
        Some(ServiceNetworks::Long { networks, .. }) => networks.first().ok_or("generated network expected"),
        _ => Err("generated long-form service networks expected"),
    }
}

fn expected_document() -> &'static str {
    concat!(
        "name: \"example\"\n",
        "services:\n",
        "  \"web\":\n",
        "    container_name: \"application-web\"\n",
        "    image: \"example.invalid/web:1@sha256:abcd\"\n",
        "    entrypoint:\n",
        "      - \"/usr/bin/env\"\n",
        "      - \"php\"\n",
        "    command:\n",
        "      - \"server\"\n",
        "      - \"--foreground\"\n",
        "    init: true\n",
        "    environment:\n",
        "      - \"MODE=production-secret\"\n",
        "      - \"FROM_HOST\"\n",
        "      - \"MODE=last-wins\"\n",
        "    labels:\n",
        "      \"com.example.purpose\": \"runtime=migration\"\n",
        "      \"com.example.empty\": \"\"\n",
        "      \"com.example.secret\": \"private-label-value\"\n",
        "    user: \"1001:1002\"\n",
        "    userns_mode: \"keep-id\"\n",
        "    group_add:\n",
        "      - \"audio\"\n",
        "      - \"44\"\n",
        "    working_dir: \"/srv/app\"\n",
        "    read_only: true\n",
        "    restart: \"on-failure:3\"\n",
        "    extra_hosts:\n",
        "      - \"host.docker.internal=host-gateway\"\n",
        "      - \"ipv6=::1\"\n",
        "    ports:\n",
        "      - target: 8080\n",
        "        published: \"18080\"\n",
        "        host_ip: \"127.0.0.1\"\n",
        "        protocol: \"tcp\"\n",
        "    volumes:\n",
        "      - type: \"volume\"\n",
        "        source: \"data\"\n",
        "        target: \"/var/lib/data\"\n",
        "        read_only: true\n",
        "      - \"/srv/data:/data:Z,ro\"\n",
        "      - type: \"bind\"\n",
        "        source: \"/srv/config\"\n",
        "        target: \"/etc/config\"\n",
        "      - type: \"volume\"\n",
        "        target: \"/cache\"\n",
        "    networks:\n",
        "      \"frontend\":\n",
        "        aliases:\n",
        "          - \"web\"\n",
        "          - \"public-api\"\n",
        "networks:\n",
        "  \"frontend\":\n",
        "    name: \"observed-frontend\"\n",
        "volumes:\n",
        "  \"data\":\n",
        "    name: \"observed-data\"\n",
        "    external: true\n",
    )
}

#[test]
fn generates_documented_pull_policies_with_exact_interval_spelling_and_redaction()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::PullPolicyKind;

    let mut project = ComposeDocumentBuilder::new();
    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;
    project.add_service(omitted)?;
    for (name, policy) in [
        ("always", GeneratedPullPolicy::Always),
        ("never", GeneratedPullPolicy::Never),
        ("missing", GeneratedPullPolicy::Missing),
        ("alias", GeneratedPullPolicy::IfNotPresentAlias),
        ("build", GeneratedPullPolicy::Build),
        ("daily", GeneratedPullPolicy::Daily),
        ("weekly", GeneratedPullPolicy::Weekly),
    ] {
        let mut service = GeneratedService::new(name)?;
        service.set_pull_policy(policy)?;
        project.add_service(service)?;
    }
    let mut interval = GeneratedService::new("interval")?;
    interval.set_pull_policy(GeneratedPullPolicy::Every(GeneratedString::sensitive("1w2d3h4m5s")?))?;
    project.add_service(interval)?;

    let debug = format!("{project:?}");
    assert!(!debug.contains("01h30m"));
    let generated = project.build(SourceId::new(710))?;
    assert!(generated.is_sensitive());
    assert!(generated.text().contains("    pull_policy: \"if_not_present\"\n"));
    assert!(generated.text().contains("    pull_policy: \"every_1w2d3h4m5s\"\n"));
    assert!(
        generated
            .document()
            .service("omitted")
            .ok_or("omitted service expected")?
            .pull_policy()
            .is_none()
    );
    let parsed = generated
        .document()
        .service("interval")
        .and_then(compose_lens::model::Service::pull_policy)
        .ok_or("parsed interval policy expected")?;
    assert_eq!(parsed.raw().value(), "every_1w2d3h4m5s");
    assert_eq!(
        parsed.kind(),
        &PullPolicyKind::Every {
            duration: "1w2d3h4m5s".to_owned(),
        }
    );
    assert!(!format!("{generated:?}").contains("every_1w2d3h4m5s"));
    Ok(())
}

#[test]
fn rejects_invalid_or_duplicate_generated_pull_policies() -> Result<(), Box<dyn std::error::Error>> {
    for duration in ["1w", "2d", "1w2d3h4m5s", "0s"] {
        let mut service = GeneratedService::new(format!("valid-{duration}"))?;
        service.set_pull_policy(GeneratedPullPolicy::Every(plain(duration)?))?;
    }
    for duration in ["", "1us", "1ms", "1.5h", "1h30", "1x"] {
        let mut service = GeneratedService::new("invalid")?;
        assert_eq!(
            service.set_pull_policy(GeneratedPullPolicy::Every(plain(duration)?)),
            Err(GenerationError::InvalidPullPolicyDuration),
            "expected invalid duration {duration}"
        );
    }
    let mut service = GeneratedService::new("app")?;
    service.set_pull_policy(GeneratedPullPolicy::Missing)?;
    assert_eq!(
        service.set_pull_policy(GeneratedPullPolicy::Always),
        Err(GenerationError::DuplicateField("pull_policy"))
    );
    Ok(())
}

#[test]
fn generates_only_unlimited_or_positive_integral_pids_limits_and_parses_them_back()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::PidsLimitKind;

    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;
    let mut unlimited = GeneratedService::new("unlimited")?;
    unlimited.set_pids_limit(GeneratedPidsLimit::Unlimited)?;
    let mut finite = GeneratedService::new("finite")?;
    finite.set_pids_limit(GeneratedPidsLimit::Finite("00064".to_owned()))?;
    let arbitrary_precision = "18446744073709551616000000000000000000000000000000";
    let mut large = GeneratedService::new("large")?;
    large.set_pids_limit(GeneratedPidsLimit::Finite(arbitrary_precision.to_owned()))?;

    let mut project = ComposeDocumentBuilder::new();
    project.add_service(omitted)?;
    project.add_service(unlimited)?;
    project.add_service(finite)?;
    project.add_service(large)?;
    let generated = project.build(SourceId::new(711))?;

    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"unlimited\":\n",
            "    pids_limit: -1\n",
            "  \"finite\":\n",
            "    pids_limit: 00064\n",
            "  \"large\":\n",
            "    pids_limit: 18446744073709551616000000000000000000000000000000\n",
        )
    );
    assert!(
        generated
            .document()
            .service("omitted")
            .ok_or("omitted service expected")?
            .pids_limit()
            .is_none()
    );
    assert!(matches!(
        generated
            .document()
            .service("unlimited")
            .and_then(compose_lens::model::Service::pids_limit)
            .map(compose_lens::model::PidsLimit::kind),
        Some(PidsLimitKind::Unlimited)
    ));
    for (service, decimal) in [("finite", "00064"), ("large", arbitrary_precision)] {
        let limit = generated
            .document()
            .service(service)
            .and_then(compose_lens::model::Service::pids_limit)
            .ok_or("finite PID limit expected")?;
        assert_eq!(limit.raw().value(), decimal);
        assert_eq!(
            limit.kind(),
            &PidsLimitKind::Finite {
                decimal: decimal.to_owned(),
            }
        );
    }
    Ok(())
}

#[test]
fn rejects_zero_or_non_integral_generated_pids_limits_and_duplicates() -> Result<(), Box<dyn std::error::Error>> {
    for invalid in ["", "0", "000", "-1", "+1", "1.0", "1e3", "many"] {
        let mut service = GeneratedService::new("invalid")?;
        assert_eq!(
            service.set_pids_limit(GeneratedPidsLimit::Finite(invalid.to_owned())),
            Err(GenerationError::InvalidPidsLimit),
            "expected invalid finite PID limit {invalid}"
        );
    }
    let mut service = GeneratedService::new("app")?;
    service.set_pids_limit(GeneratedPidsLimit::Unlimited)?;
    assert_eq!(
        service.set_pids_limit(GeneratedPidsLimit::Finite("64".to_owned())),
        Err(GenerationError::DuplicateField("pids_limit"))
    );
    Ok(())
}

#[test]
fn generates_only_quoted_canonical_positive_explicit_shm_sizes_and_parses_back_exact_parts()
-> Result<(), Box<dyn std::error::Error>> {
    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;
    let mut shared = GeneratedService::new("shared")?;
    shared.set_shm_size(GeneratedShmSize::Explicit {
        amount: GeneratedString::sensitive("64")?,
        unit: ShmSizeUnit::Mb,
    })?;
    let mut project = ComposeDocumentBuilder::new();
    project.add_service(omitted)?;
    project.add_service(shared)?;
    let generated = project.build(SourceId::new(712))?;

    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"shared\":\n",
            "    shm_size: \"64mb\"\n",
        )
    );
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("64mb"));
    assert!(
        generated
            .document()
            .service("omitted")
            .ok_or("omitted service expected")?
            .shm_size()
            .is_none()
    );
    let size = generated
        .document()
        .service("shared")
        .and_then(compose_lens::model::Service::shm_size)
        .ok_or("parse-back shared-memory size expected")?;
    assert_eq!(size.raw().value(), "64mb");
    assert_eq!(size.scalar_kind(), ShmSizeScalarKind::String);
    assert!(matches!(
        size.kind(),
        ShmSizeKind::Documented { amount_raw, unit: ShmSizeUnit::Mb } if amount_raw == "64"
    ));
    Ok(())
}

#[test]
fn supports_every_documented_shm_unit_and_rejects_unsafe_amount_spellings() -> Result<(), Box<dyn std::error::Error>> {
    for (unit, suffix) in [
        (ShmSizeUnit::B, "b"),
        (ShmSizeUnit::K, "k"),
        (ShmSizeUnit::Kb, "kb"),
        (ShmSizeUnit::M, "m"),
        (ShmSizeUnit::Mb, "mb"),
        (ShmSizeUnit::G, "g"),
        (ShmSizeUnit::Gb, "gb"),
    ] {
        let mut service = GeneratedService::new(format!("unit-{suffix}"))?;
        service.set_shm_size(GeneratedShmSize::Explicit {
            amount: plain("18446744073709551616000000000000000000000000000000")?,
            unit,
        })?;
        let mut project = ComposeDocumentBuilder::new();
        project.add_service(service)?;
        let generated = project.build(SourceId::new(713))?;
        assert!(generated.text().contains(&format!(
            "    shm_size: \"18446744073709551616000000000000000000000000000000{suffix}\"\n"
        )));
    }

    for invalid in [
        "", "0", "00", "01", "-1", "+1", "1.0", "1e3", "1M", "1MiB", "1mb ", " 1", "${SIZE}",
    ] {
        let mut service = GeneratedService::new("invalid")?;
        assert_eq!(
            service.set_shm_size(GeneratedShmSize::Explicit {
                amount: plain(invalid)?,
                unit: ShmSizeUnit::Mb,
            }),
            Err(GenerationError::InvalidShmSize),
            "expected unsafe amount to be rejected: {invalid:?}"
        );
    }
    let mut service = GeneratedService::new("duplicate")?;
    service.set_shm_size(GeneratedShmSize::Explicit {
        amount: plain("64")?,
        unit: ShmSizeUnit::Mb,
    })?;
    assert_eq!(
        service.set_shm_size(GeneratedShmSize::Explicit {
            amount: plain("128")?,
            unit: ShmSizeUnit::M,
        }),
        Err(GenerationError::DuplicateField("shm_size"))
    );
    Ok(())
}

#[test]
fn generates_only_quoted_positive_mem_limits_and_parses_back_exact_parts() -> Result<(), Box<dyn std::error::Error>> {
    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;
    let mut limited = GeneratedService::new("limited")?;
    limited.set_mem_limit(GeneratedMemLimit::Explicit {
        amount: GeneratedString::sensitive("18446744073709551616000000000000000000000000000000")?,
        unit: MemLimitUnit::B,
    })?;
    let mut project = ComposeDocumentBuilder::new();
    project.add_service(omitted)?;
    project.add_service(limited)?;
    let generated = project.build(SourceId::new(716))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"limited\":\n",
            "    mem_limit: \"18446744073709551616000000000000000000000000000000b\"\n",
        )
    );
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("18446744073709551616000000000000000000000000000000b"));
    assert!(
        generated
            .document()
            .service("omitted")
            .ok_or("omitted expected")?
            .mem_limit()
            .is_none()
    );
    let limit = generated
        .document()
        .service("limited")
        .and_then(compose_lens::model::Service::mem_limit)
        .ok_or("parse-back memory limit expected")?;
    assert_eq!(limit.scalar_kind(), MemLimitScalarKind::String);
    assert!(matches!(
        limit.kind(),
        MemLimitKind::Documented { amount_raw, unit: MemLimitUnit::B }
            if amount_raw == "18446744073709551616000000000000000000000000000000"
    ));
    Ok(())
}

#[test]
fn supports_every_documented_mem_unit_and_rejects_unsafe_amount_spellings() -> Result<(), Box<dyn std::error::Error>> {
    for (unit, suffix) in [
        (MemLimitUnit::B, "b"),
        (MemLimitUnit::K, "k"),
        (MemLimitUnit::Kb, "kb"),
        (MemLimitUnit::M, "m"),
        (MemLimitUnit::Mb, "mb"),
        (MemLimitUnit::G, "g"),
        (MemLimitUnit::Gb, "gb"),
    ] {
        let mut service = GeneratedService::new(format!("unit-{suffix}"))?;
        service.set_mem_limit(GeneratedMemLimit::Explicit {
            amount: plain("64")?,
            unit,
        })?;
        let mut project = ComposeDocumentBuilder::new();
        project.add_service(service)?;
        assert!(
            project
                .build(SourceId::new(717))?
                .text()
                .contains(&format!("    mem_limit: \"64{suffix}\"\n"))
        );
    }
    for invalid in ["", "0", "00", "01", "-1", "+1", "1.0", "1e3", " 1", "1 ", "${LIMIT}"] {
        let mut service = GeneratedService::new("invalid")?;
        assert_eq!(
            service.set_mem_limit(GeneratedMemLimit::Explicit {
                amount: plain(invalid)?,
                unit: MemLimitUnit::Mb,
            }),
            Err(GenerationError::InvalidMemLimit),
            "expected unsafe amount to be rejected: {invalid:?}"
        );
    }
    let mut service = GeneratedService::new("duplicate")?;
    service.set_mem_limit(GeneratedMemLimit::Explicit {
        amount: plain("64")?,
        unit: MemLimitUnit::Mb,
    })?;
    assert_eq!(
        service.set_mem_limit(GeneratedMemLimit::Explicit {
            amount: plain("128")?,
            unit: MemLimitUnit::B,
        }),
        Err(GenerationError::DuplicateField("mem_limit"))
    );
    Ok(())
}

#[test]
fn generates_explicit_scalar_list_and_empty_tmpfs_forms_with_exact_parse_back() -> Result<(), Box<dyn std::error::Error>>
{
    use compose_lens::model::{TmpfsForm, TmpfsItemKind};

    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;
    let mut scalar = GeneratedService::new("scalar")?;
    scalar.set_tmpfs(GeneratedTmpfs::Scalar(plain("/run:mode=1777")?))?;
    let mut empty = GeneratedService::new("empty")?;
    empty.set_tmpfs(GeneratedTmpfs::List(Vec::new()))?;
    let mut listed = GeneratedService::new("listed")?;
    listed.set_tmpfs(GeneratedTmpfs::List(vec![
        plain("/cache:uid=1000")?,
        plain("/cache:uid=1000")?,
        GeneratedString::sensitive("/state:size=64m,nosuid")?,
    ]))?;

    let mut project = ComposeDocumentBuilder::new();
    project.add_service(omitted)?;
    project.add_service(scalar)?;
    project.add_service(empty)?;
    project.add_service(listed)?;
    let generated = project.build(SourceId::new(714))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"scalar\":\n",
            "    tmpfs: \"/run:mode=1777\"\n",
            "  \"empty\":\n",
            "    tmpfs: []\n",
            "  \"listed\":\n",
            "    tmpfs:\n",
            "      - \"/cache:uid=1000\"\n",
            "      - \"/cache:uid=1000\"\n",
            "      - \"/state:size=64m,nosuid\"\n",
        )
    );
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("/state"));
    assert!(
        generated
            .document()
            .service("omitted")
            .is_some_and(|service| service.tmpfs().is_none())
    );

    let scalar = generated
        .document()
        .service("scalar")
        .and_then(compose_lens::model::Service::tmpfs)
        .ok_or("parse-back scalar tmpfs expected")?;
    assert!(matches!(
        scalar.form(),
        TmpfsForm::Scalar(item)
            if item.value() == "/run:mode=1777" && item.kind() == TmpfsItemKind::Documented
    ));
    let empty = generated
        .document()
        .service("empty")
        .and_then(compose_lens::model::Service::tmpfs)
        .ok_or("parse-back empty tmpfs expected")?;
    assert!(matches!(empty.form(), TmpfsForm::List(items) if items.is_empty()));
    let listed = generated
        .document()
        .service("listed")
        .and_then(compose_lens::model::Service::tmpfs)
        .ok_or("parse-back tmpfs list expected")?;
    assert!(matches!(
        listed.form(),
        TmpfsForm::List(items)
            if items.iter().map(compose_lens::model::TmpfsItem::value).collect::<Vec<_>>()
                == ["/cache:uid=1000", "/cache:uid=1000", "/state:size=64m,nosuid"]
    ));
    Ok(())
}

#[test]
fn rejects_malformed_generated_tmpfs_items_and_reconfiguration() -> Result<(), Box<dyn std::error::Error>> {
    let mut service = GeneratedService::new("invalid")?;
    for invalid in ["${TMPFS}", ":mode=1777", "/run:", "/run:,mode=1777", "/run:mode="] {
        assert_eq!(
            service.set_tmpfs(GeneratedTmpfs::Scalar(plain(invalid)?)),
            Err(GenerationError::InvalidTmpfsItem),
            "expected unsafe tmpfs item {invalid:?} to be rejected"
        );
        assert!(service.tmpfs().is_none());
    }
    assert_eq!(
        service.set_tmpfs(GeneratedTmpfs::Scalar(plain("")?)),
        Err(GenerationError::EmptyValue("tmpfs item"))
    );
    for multiline in ["/run\nnext", "/run\rnext"] {
        assert_eq!(
            service.set_tmpfs(GeneratedTmpfs::Scalar(plain(multiline)?)),
            Err(GenerationError::ContainsLineBreak("tmpfs item"))
        );
    }
    service.set_tmpfs(GeneratedTmpfs::List(vec![
        plain("/run:size=64m")?,
        plain("/run:size=64m")?,
    ]))?;
    assert_eq!(
        GeneratedString::plain("/run\0tmpfs"),
        Err(GenerationError::ContainsNul("string"))
    );
    assert_eq!(
        service.tmpfs(),
        Some(&GeneratedTmpfs::List(vec![
            plain("/run:size=64m")?,
            plain("/run:size=64m")?,
        ]))
    );
    assert_eq!(
        service.set_tmpfs(GeneratedTmpfs::List(Vec::new())),
        Err(GenerationError::DuplicateField("tmpfs"))
    );
    Ok(())
}

#[test]
fn generates_ordered_mixed_and_empty_devices_with_exact_parse_back_and_redaction()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{Device, ShortDeviceKind};

    let mut omitted = GeneratedService::new("omitted")?;
    omitted.set_image(plain("example.invalid/omitted:1")?)?;
    let mut empty = GeneratedService::new("empty")?;
    empty.set_devices(Vec::new())?;
    let mut mixed = GeneratedService::new("mixed")?;
    mixed.set_devices(vec![
        GeneratedDevice::Short(plain("/dev/dri:/dev/dri:rwm")?),
        GeneratedDevice::Short(plain("/dev/dri:/dev/dri:rwm")?),
        GeneratedDevice::Short(plain("vendor.example/device=gpu")?),
        GeneratedDevice::Long(GeneratedLongDevice::new(
            GeneratedString::sensitive("/dev/video0")?,
            Some(plain("/dev/camera")?),
            Some(plain("provider-permissions")?),
        )?),
        GeneratedDevice::Long(GeneratedLongDevice::new(
            plain("vendor.example/device=accelerator")?,
            None,
            None,
        )?),
    ])?;
    assert_eq!(mixed.devices().map(<[_]>::len), Some(5));
    assert_eq!(
        mixed.set_devices(Vec::new()),
        Err(GenerationError::DuplicateField("devices"))
    );

    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(omitted)?;
    builder.add_service(empty)?;
    builder.add_service(mixed)?;
    let generated = builder.build(SourceId::new(715))?;
    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"omitted\":\n",
            "    image: \"example.invalid/omitted:1\"\n",
            "  \"empty\":\n",
            "    devices: []\n",
            "  \"mixed\":\n",
            "    devices:\n",
            "      - \"/dev/dri:/dev/dri:rwm\"\n",
            "      - \"/dev/dri:/dev/dri:rwm\"\n",
            "      - \"vendor.example/device=gpu\"\n",
            "      - source: \"/dev/video0\"\n",
            "        target: \"/dev/camera\"\n",
            "        permissions: \"provider-permissions\"\n",
            "      - source: \"vendor.example/device=accelerator\"\n",
        )
    );
    assert!(generated.is_sensitive());
    assert!(!format!("{generated:?}").contains("/dev/video0"));
    assert!(
        generated
            .document()
            .service("omitted")
            .is_some_and(|service| service.devices().is_none())
    );
    assert!(
        generated
            .document()
            .service("empty")
            .and_then(compose_lens::model::Service::devices)
            .is_some_and(|devices| devices.items().is_empty())
    );
    let devices = generated
        .document()
        .service("mixed")
        .and_then(compose_lens::model::Service::devices)
        .ok_or("parse-back devices expected")?;
    assert_eq!(devices.items().len(), 5);
    assert!(matches!(
        &devices.items()[2],
        Device::Short(device)
            if device.raw().value() == "vendor.example/device=gpu" && device.kind() == ShortDeviceKind::Cdi
    ));
    assert!(matches!(
        &devices.items()[3],
        Device::Long(device)
            if device.source().is_some_and(|value| value.value() == "/dev/video0")
                && device.target().is_some_and(|value| value.value() == "/dev/camera")
                && device.permissions().is_some_and(|value| value.value() == "provider-permissions")
    ));
    Ok(())
}

#[test]
fn rejects_unsafe_or_malformed_generated_devices_without_validating_runtime_semantics()
-> Result<(), Box<dyn std::error::Error>> {
    for source in ["", "${DEVICE}", "line\nbreak", "line\rbreak"] {
        assert_eq!(
            GeneratedLongDevice::new(plain(source)?, None, None),
            Err(GenerationError::InvalidDeviceValue("source"))
        );
    }
    assert_eq!(
        GeneratedLongDevice::new(plain("/dev/source")?, Some(plain("${TARGET}")?), None),
        Err(GenerationError::InvalidDeviceValue("target"))
    );
    assert_eq!(
        GeneratedLongDevice::new(plain("/dev/source")?, None, Some(plain("raw\npermissions")?),),
        Err(GenerationError::InvalidDeviceValue("permissions"))
    );
    assert_eq!(
        GeneratedString::plain("/dev/source\0bad"),
        Err(GenerationError::ContainsNul("string"))
    );

    let mut service = GeneratedService::new("invalid")?;
    for value in ["", "${DEVICE}", "line\nbreak", "line\rbreak"] {
        assert_eq!(
            service.set_devices(vec![GeneratedDevice::Short(plain(value)?)]),
            Err(GenerationError::InvalidDeviceValue("short item"))
        );
        assert!(service.devices().is_none());
    }
    service.set_devices(vec![
        GeneratedDevice::Short(plain("opaque-provider-token")?),
        GeneratedDevice::Long(GeneratedLongDevice::new(
            plain("not-a-host-device")?,
            Some(plain("")?),
            Some(plain("not-rwm")?),
        )?),
    ])?;
    Ok(())
}

#[test]
fn generates_literal_stdin_open_choices_and_rejects_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let mut opened = GeneratedService::new("opened")?;
    opened.set_stdin_open(true)?;
    assert_eq!(
        opened.set_stdin_open(false),
        Err(GenerationError::DuplicateField("stdin_open"))
    );

    let mut closed = GeneratedService::new("closed")?;
    closed.set_stdin_open(false)?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(opened)?;
    builder.add_service(closed)?;
    let generated = builder.build(SourceId::new(685))?;

    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"opened\":\n",
            "    stdin_open: true\n",
            "  \"closed\":\n",
            "    stdin_open: false\n",
        )
    );
    for (name, expected) in [("opened", true), ("closed", false)] {
        assert_eq!(
            generated
                .document()
                .service(name)
                .and_then(compose_lens::model::Service::stdin_open)
                .map(compose_lens::model::Located::value),
            Some(&BooleanValue::Literal(expected))
        );
    }
    Ok(())
}

#[test]
fn generates_literal_tty_choices_and_rejects_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let mut allocated = GeneratedService::new("allocated")?;
    allocated.set_tty(true)?;
    assert_eq!(allocated.set_tty(false), Err(GenerationError::DuplicateField("tty")));

    let mut disabled = GeneratedService::new("disabled")?;
    disabled.set_tty(false)?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(allocated)?;
    builder.add_service(disabled)?;
    let generated = builder.build(SourceId::new(690))?;

    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"allocated\":\n",
            "    tty: true\n",
            "  \"disabled\":\n",
            "    tty: false\n",
        )
    );
    for (name, expected) in [("allocated", true), ("disabled", false)] {
        assert_eq!(
            generated
                .document()
                .service(name)
                .and_then(compose_lens::model::Service::tty)
                .map(compose_lens::model::Located::value),
            Some(&BooleanValue::Literal(expected))
        );
    }
    Ok(())
}

#[test]
fn generates_literal_privileged_choices_and_rejects_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let mut enabled = GeneratedService::new("enabled")?;
    enabled.set_privileged(true)?;
    assert_eq!(
        enabled.set_privileged(false),
        Err(GenerationError::DuplicateField("privileged"))
    );

    let mut disabled = GeneratedService::new("disabled")?;
    disabled.set_privileged(false)?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(enabled)?;
    builder.add_service(disabled)?;
    let generated = builder.build(SourceId::new(694))?;

    assert_eq!(
        generated.text(),
        concat!(
            "services:\n",
            "  \"enabled\":\n",
            "    privileged: true\n",
            "  \"disabled\":\n",
            "    privileged: false\n",
        )
    );
    for (name, expected) in [("enabled", true), ("disabled", false)] {
        assert_eq!(
            generated
                .document()
                .service(name)
                .and_then(compose_lens::model::Service::privileged)
                .map(compose_lens::model::Located::value),
            Some(&BooleanValue::Literal(expected))
        );
    }
    Ok(())
}
