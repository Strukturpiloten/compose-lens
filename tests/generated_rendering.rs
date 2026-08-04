//! Deterministic generated-document construction and parse-back validation.

use compose_lens::{
    model::{Command, Environment, ExtraHosts, Port, ServiceNetworks, VolumeMount},
    render::{
        ComposeDocumentBuilder, GeneratedCommand, GeneratedEnvironment, GeneratedExtraHost, GeneratedMount,
        GeneratedNetworkAttachment, GeneratedPort, GeneratedProtocol, GeneratedResource, GeneratedSelinux,
        GeneratedService, GeneratedString, GenerationError,
    },
    source::SourceId,
};

#[test]
fn generates_the_runtime_migration_subset_deterministically() -> Result<(), Box<dyn std::error::Error>> {
    let project = complete_project()?;
    let builder_debug = format!("{project:?}");
    assert!(!builder_debug.contains("production-secret"));
    assert!(!builder_debug.contains("1001:1002"));
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
        service.image().map(|image| image.value().raw()),
        Some("example.invalid/web:1@sha256:abcd")
    );
    assert!(matches!(service.command(), Some(Command::List { values, .. }) if values.len() == 2));
    assert!(matches!(service.environment(), Some(Environment::List { entries, .. }) if entries.len() == 3));
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
    assert!(!debug.contains("1001:1002"));
    Ok(())
}

#[test]
fn retains_empty_shell_protocol_and_shared_selinux_variants() -> Result<(), Box<dyn std::error::Error>> {
    let mut shell = GeneratedService::new("shell")?;
    shell.set_image(plain("example.invalid/shell:1")?)?;
    shell.set_command(GeneratedCommand::Shell(plain("echo hello")?))?;
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
            "    command: \"echo hello\"\n",
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
            "    command: []\n",
            "    ports:\n",
            "      - \"5000/sctp\"\n",
            "      - \"[::1]:15001:5001/sctp\"\n",
        )
    );
    assert!(
        matches!(generated.document().service("empty").and_then(|service| service.command()), Some(Command::List { values, .. }) if values.is_empty())
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
fn rejects_ambiguous_or_duplicate_generation_requests() -> Result<(), Box<dyn std::error::Error>> {
    let empty = ComposeDocumentBuilder::new();
    assert_eq!(empty.build(SourceId::new(710)), Err(GenerationError::MissingService));

    let mut service = GeneratedService::new("web")?;
    service.set_image(plain("example.invalid/web:1")?)?;
    assert_eq!(
        service.set_image(plain("example.invalid/web:2")?),
        Err(GenerationError::DuplicateField("image"))
    );
    assert_eq!(
        GeneratedEnvironment::literal("INVALID=NAME", plain("value")?),
        Err(GenerationError::InvalidEnvironmentName)
    );
    let mut invalid_service = GeneratedService::new("invalid")?;
    assert_eq!(
        invalid_service.set_image(plain("")?),
        Err(GenerationError::EmptyValue("service image"))
    );
    assert_eq!(
        invalid_service.add_supplementary_group(plain("")?),
        Err(GenerationError::EmptyValue("supplementary group"))
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
    service.set_image(plain("example.invalid/web:1@sha256:abcd")?)?;
    service.set_command(GeneratedCommand::Exec(vec![plain("server")?, plain("--foreground")?]))?;
    service.add_environment(GeneratedEnvironment::literal(
        "MODE",
        GeneratedString::sensitive("production-secret")?,
    )?);
    service.add_environment(GeneratedEnvironment::host("FROM_HOST")?);
    service.add_environment(GeneratedEnvironment::literal("MODE", plain("last-wins")?)?);
    service.set_user(GeneratedString::sensitive("1001:1002")?)?;
    service.set_userns_mode(plain("keep-id")?)?;
    service.add_supplementary_group(plain("audio")?)?;
    service.add_supplementary_group(plain("44")?)?;
    service.set_working_dir(plain("/srv/app")?)?;
    service.set_read_only(true)?;
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
        "    image: \"example.invalid/web:1@sha256:abcd\"\n",
        "    command:\n",
        "      - \"server\"\n",
        "      - \"--foreground\"\n",
        "    environment:\n",
        "      - \"MODE=production-secret\"\n",
        "      - \"FROM_HOST\"\n",
        "      - \"MODE=last-wins\"\n",
        "    user: \"1001:1002\"\n",
        "    userns_mode: \"keep-id\"\n",
        "    group_add:\n",
        "      - \"audio\"\n",
        "      - \"44\"\n",
        "    working_dir: \"/srv/app\"\n",
        "    read_only: true\n",
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
