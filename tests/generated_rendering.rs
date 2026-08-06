//! Deterministic generated-document construction and parse-back validation.

use compose_lens::{
    model::{
        BooleanValue, Command, ComposeScalar, Entrypoint, Environment, EnvironmentFile, EnvironmentFileFormatKind,
        ExtraHosts, HostnameKind, Labels, MemLimitKind, MemLimitScalarKind, MemLimitUnit, Port, ServiceNetworks,
        ShmSizeKind, ShmSizeScalarKind, ShmSizeUnit, StopGracePeriod, SysctlsForm, UlimitValue, VolumeMount,
    },
    render::{
        ComposeDocumentBuilder, GeneratedCommand, GeneratedComposeDocument, GeneratedDevice, GeneratedEntrypoint,
        GeneratedEnvironment, GeneratedEnvironmentFile, GeneratedEnvironmentFileFormat, GeneratedExtraHost,
        GeneratedHostname, GeneratedLabel, GeneratedLongDevice, GeneratedMemLimit, GeneratedMount,
        GeneratedNetworkAttachment, GeneratedPidsLimit, GeneratedPort, GeneratedProtocol, GeneratedPullPolicy,
        GeneratedResource, GeneratedRestartPolicy, GeneratedSelinux, GeneratedService, GeneratedShmSize,
        GeneratedString, GeneratedSysctl, GeneratedSysctls, GeneratedTmpfs, GeneratedUlimit, GeneratedUlimitValue,
        GeneratedUlimits, GenerationError,
    },
    source::SourceId,
};

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
