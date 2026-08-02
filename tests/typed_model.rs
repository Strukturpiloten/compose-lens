//! Public typed-model behavior and representation fidelity.

use compose_lens::model::{
    BooleanValue, Build, BuildFieldKind, Command, ComposeDocument, ComposeScalar, ConfigGrant, ContainerPathKind,
    DEPENDENCY_HEALTHCHECK_UNVERIFIED, DEPENDENCY_INVALID_CONDITION, DEPENDENCY_MISSING_HEALTHCHECK,
    DEPENDENCY_MISSING_SERVICE, DependencyCondition, DeployFieldKind, EXPECTED_BOOLEAN, EXPECTED_FIELD_FORM,
    EXPECTED_MAPPING, EXPECTED_SEQUENCE, EXTRA_HOST_INVALID_ENTRY, Environment, ExtraHostSeparator, ExtraHosts,
    GRANT_EXPECTED_FORM, GRANT_MISSING_SOURCE, HEALTHCHECK_INVALID_DURATION, HEALTHCHECK_INVALID_RETRIES,
    HEALTHCHECK_INVALID_TEST, HealthcheckTestKind, HostAddressKind, IdentityComponent, Labels, LimitValue, Located,
    MountType, PORT_EXPECTED_FORM, PORT_MISSING_TARGET, Port, RESOURCE_EXPECTED_FORM, SecretGrant, SelinuxRelabel,
    ServiceNetworks, ULIMIT_INVALID_VALUE, UlimitValue, UserNamespaceModeKind, VOLUME_EXPECTED_FORM,
    VOLUME_INVALID_SELINUX, VOLUME_MISSING_TARGET, VOLUME_MISSING_TYPE, VolumeMount, VolumeSyntax,
};
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;

const VOLUME_FORMS: &str = include_str!("../fixtures/typed-model/volume-syntax-fidelity/compose.yaml");
const INVALID_VOLUME_FORMS: &str = include_str!("../fixtures/typed-model/invalid-volume-forms/compose.yaml");
const PHASE_TWO_FORMS: &str = include_str!("../fixtures/typed-model/phase-two-field-forms/compose.yaml");
const INVALID_PHASE_TWO_FORMS: &str = include_str!("../fixtures/typed-model/invalid-phase-two-forms/compose.yaml");
const POST_01_FORMS: &str = include_str!("../fixtures/typed-model/post-01-issue-backlog/compose.yaml");
const POST_01_INVALID: &str = include_str!("../fixtures/typed-model/post-01-invalid/compose.yaml");
const TRAILING_EMPTY_VALUE: &str = include_str!("../fixtures/roundtrip/canonical-merged/compose.yaml");

#[test]
fn retains_short_and_long_volume_forms_as_distinct_variants() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(31), VOLUME_FORMS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document was not recovered")?;
    let service = document.service("app").ok_or("app service is missing")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(document.name().map(|name| name.value().as_str()), Some("volume-forms"));
    assert_eq!(document.networks().len(), 1);
    assert_eq!(document.volumes().len(), 1);
    assert_eq!(document.configs().len(), 1);
    assert_eq!(document.secrets().len(), 1);
    assert_eq!(document.extension_fields().len(), 1);
    assert_eq!(service.extension_fields().len(), 1);
    assert_eq!(service.unknown_fields().len(), 0);
    assert_eq!(
        service.image().map(|image| image.value().raw()),
        Some("example/app:1.0@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
    assert_eq!(service.volumes().len(), 4);

    let syntaxes: Vec<_> = service.volumes().iter().map(VolumeMount::syntax).collect();
    assert_eq!(
        syntaxes,
        vec![
            VolumeSyntax::Short,
            VolumeSyntax::Short,
            VolumeSyntax::Short,
            VolumeSyntax::Long,
        ]
    );
    assert_eq!(service.volumes()[0].selinux_relabel(), Some(SelinuxRelabel::Shared));
    assert_eq!(service.volumes()[1].selinux_relabel(), Some(SelinuxRelabel::Private));
    assert_eq!(service.volumes()[2].selinux_relabel(), None);
    assert_eq!(
        syntax.document().text(service.volumes()[0].span()),
        Some("./shared:/srv/shared:z")
    );

    let VolumeMount::Short(private) = &service.volumes()[1] else {
        return Err("second mount did not retain short syntax".into());
    };
    assert_eq!(private.source(), Some("./private"));
    assert_eq!(private.target(), Some("/srv/private"));
    assert_eq!(private.options(), &["Z".to_owned(), "ro".to_owned()]);

    let VolumeMount::Long(long) = &service.volumes()[3] else {
        return Err("fourth mount did not retain long syntax".into());
    };
    assert_eq!(long.mount_type().map(Located::value), Some(&MountType::Bind));
    assert_eq!(long.source().map(|value| value.value().as_str()), Some("./strict"));
    assert_eq!(long.target().map(|value| value.value().as_str()), Some("/srv/strict"));
    assert_eq!(long.read_only().map(Located::value), Some(&BooleanValue::Literal(true)));
    assert_eq!(long.extension_fields().len(), 1);
    assert_eq!(long.unknown_fields().len(), 1);

    let bind = long.bind().ok_or("long mount bind options are missing")?;
    assert_eq!(
        bind.create_host_path().map(Located::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(bind.propagation().map(|value| value.value().as_str()), Some("rshared"));
    assert_eq!(
        bind.selinux().map(|value| *value.value()),
        Some(SelinuxRelabel::Private)
    );
    assert_eq!(bind.extension_fields().len(), 1);
    assert_eq!(bind.unknown_fields().len(), 1);
    Ok(())
}

#[test]
fn invalid_volume_forms_return_partial_data_and_stable_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(37), INVALID_VOLUME_FORMS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document was not recovered")?;
    let service = document.service("app").ok_or("app service is missing")?;
    let codes: Vec<_> = parsed
        .diagnostics()
        .iter()
        .map(compose_lens::diagnostic::Diagnostic::code)
        .collect();

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(!parsed.is_valid());
    assert_eq!(service.volumes().len(), 2);
    for expected in [
        VOLUME_EXPECTED_FORM,
        VOLUME_MISSING_TARGET,
        VOLUME_INVALID_SELINUX,
        EXPECTED_BOOLEAN,
        VOLUME_MISSING_TYPE,
        EXPECTED_MAPPING,
    ] {
        assert!(codes.contains(&expected), "missing diagnostic {expected}");
    }
    assert!(parsed.diagnostics().iter().all(|diagnostic| {
        diagnostic.labels().iter().all(|label| {
            label.span().source_id() == SourceId::new(37) && label.span().end() <= INVALID_VOLUME_FORMS.len()
        })
    }));
    Ok(())
}

#[test]
fn retains_image_command_and_environment_forms() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(41), PHASE_TWO_FORMS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document was not recovered")?;
    let app = document.service("app").ok_or("app service is missing")?;
    let worker = document.service("worker").ok_or("worker service is missing")?;
    let shell = document.service("shell").ok_or("shell service is missing")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    let image = app.image().ok_or("app image is missing")?;
    assert_eq!(image.value().raw(), "registry.example:5000/team/app:1.2@sha256:abcdef");
    assert_eq!(image.value().name(), "registry.example:5000/team/app");
    assert_eq!(image.value().tag(), Some("1.2"));
    assert_eq!(
        image
            .value()
            .digest()
            .and_then(compose_lens::model::ImageDigest::algorithm),
        Some("sha256")
    );
    assert!(matches!(app.command(), Some(Command::Null(_))));

    let Some(Environment::Map { entries, .. }) = app.environment() else {
        return Err("app environment did not retain mapping syntax".into());
    };
    assert_eq!(entries.len(), 5);
    assert_eq!(*entries[0].value().value(), ComposeScalar::Null);
    assert_eq!(*entries[1].value().value(), ComposeScalar::String(String::new()));
    assert_eq!(*entries[2].value().value(), ComposeScalar::Boolean(true));
    assert_eq!(*entries[3].value().value(), ComposeScalar::Number("3".to_owned()));
    assert_eq!(
        *entries[4].value().value(),
        ComposeScalar::String("${APP_VALUE:-fallback}".to_owned())
    );

    let Some(Environment::List { entries, .. }) = worker.environment() else {
        return Err("worker environment did not retain list syntax".into());
    };
    assert_eq!(entries[0].value(), None);
    assert_eq!(entries[1].value(), Some(""));
    assert_eq!(entries[2].value(), Some("a=b"));
    assert!(matches!(worker.command(), Some(Command::List { values, .. }) if values.is_empty()));
    assert!(matches!(shell.command(), Some(Command::String(value)) if value.value().is_empty()));
    Ok(())
}

#[test]
fn trailing_empty_value_does_not_absorb_parent_fields() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(42), TRAILING_EMPTY_VALUE)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document was not recovered")?;
    let app = document.service("app").ok_or("app service is missing")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    let Some(Environment::Map { entries, .. }) = app.environment() else {
        return Err("app environment did not retain mapping syntax".into());
    };
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[2].name().value(), "EMPTY");
    assert_eq!(*entries[2].value().value(), ComposeScalar::Null);
    assert_eq!(app.ports().len(), 1);
    assert_eq!(app.volumes().len(), 1);
    assert_eq!(app.extension_fields().len(), 1);
    Ok(())
}

#[test]
fn retains_service_ports_networks_profiles_and_grants() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(43), PHASE_TWO_FORMS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document was not recovered")?;
    let app = document.service("app").ok_or("app service is missing")?;

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    let Port::Short(short_port) = &app.ports()[0] else {
        return Err("first port did not retain short syntax".into());
    };
    assert_eq!(short_port.host_ip(), Some("127.0.0.1"));
    assert_eq!(short_port.published(), Some("8080"));
    assert_eq!(short_port.target(), "80");
    assert_eq!(short_port.protocol(), Some("tcp"));
    let Port::Long(long_port) = &app.ports()[1] else {
        return Err("second port did not retain long syntax".into());
    };
    assert_eq!(long_port.target().map(|value| value.value().as_str()), Some("443"));
    assert_eq!(long_port.host_ip().map(|value| value.value().as_str()), Some("::1"));
    assert_eq!(long_port.extension_fields().len(), 1);
    assert_eq!(long_port.unknown_fields().len(), 1);

    let VolumeMount::Long(long_mount) = &app.volumes()[1] else {
        return Err("second mount did not retain long syntax".into());
    };
    assert_eq!(
        long_mount.read_only().map(Located::value),
        Some(&BooleanValue::Expression("${READ_ONLY:-false}".to_owned()))
    );
    let bind = long_mount.bind().ok_or("bind options are missing")?;
    assert_eq!(
        bind.create_host_path().map(Located::value),
        Some(&BooleanValue::Expression("${CREATE_PATH:-true}".to_owned()))
    );

    let Some(ServiceNetworks::Long { networks, .. }) = app.networks() else {
        return Err("app networks did not retain long syntax".into());
    };
    let frontend = networks
        .iter()
        .find(|network| network.name().value() == "frontend")
        .ok_or("frontend service network is missing")?;
    assert_eq!(networks.len(), 2);
    assert_eq!(networks[0].name().value(), "implicit");
    assert_eq!(frontend.aliases().len(), 2);
    assert_eq!(
        frontend.interface_name().map(|value| value.value().as_str()),
        Some("eth1")
    );
    assert_eq!(frontend.driver_opts().len(), 1);
    assert_eq!(frontend.extension_fields().len(), 1);
    assert_eq!(frontend.unknown_fields().len(), 1);
    assert_eq!(app.profiles().len(), 2);
    assert!(matches!(app.configs()[0], ConfigGrant::Short(_)));
    assert!(matches!(app.configs()[1], ConfigGrant::Long(_)));
    assert!(matches!(app.secrets()[0], SecretGrant::Short(_)));
    assert!(matches!(app.secrets()[1], SecretGrant::Long(_)));
    Ok(())
}

#[test]
fn types_top_level_network_and_volume_definitions() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(47), PHASE_TWO_FORMS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document was not recovered")?;
    let network = document
        .networks()
        .iter()
        .find(|value| value.name().value() == "frontend")
        .ok_or("frontend network is missing")?;
    let volume = document
        .volumes()
        .iter()
        .find(|value| value.name().value() == "cache")
        .ok_or("cache volume is missing")?;
    let implicit_network = document
        .networks()
        .iter()
        .find(|value| value.name().value() == "implicit")
        .ok_or("implicit network is missing")?;
    let implicit_volume = document
        .volumes()
        .iter()
        .find(|value| value.name().value() == "implicit")
        .ok_or("implicit volume is missing")?;

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(network.driver().map(|value| value.value().as_str()), Some("bridge"));
    assert_eq!(network.driver_opts().len(), 1);
    assert_eq!(
        network.attachable().map(Located::value),
        Some(&BooleanValue::Expression("${ATTACHABLE:-true}".to_owned()))
    );
    assert_eq!(
        network.enable_ipv4().map(Located::value),
        Some(&BooleanValue::Literal(true))
    );
    assert_eq!(
        network.enable_ipv6().map(Located::value),
        Some(&BooleanValue::Literal(false))
    );
    let ipam = network.ipam().ok_or("network IPAM is missing")?;
    assert_eq!(ipam.config().len(), 1);
    assert_eq!(ipam.config()[0].aux_addresses().len(), 1);
    assert_eq!(ipam.options().len(), 1);
    assert!(matches!(network.labels(), Some(Labels::Map { entries, .. }) if entries.len() == 1));
    assert_eq!(network.extension_fields().len(), 1);
    assert_eq!(network.unknown_fields().len(), 1);
    assert_eq!(implicit_network.driver(), None);

    assert_eq!(volume.driver().map(|value| value.value().as_str()), Some("local"));
    assert_eq!(volume.driver_opts().len(), 3);
    assert_eq!(
        volume.external().map(Located::value),
        Some(&BooleanValue::Expression("${CACHE_EXTERNAL:-false}".to_owned()))
    );
    assert!(matches!(volume.labels(), Some(Labels::List { values, .. }) if values.len() == 1));
    assert_eq!(volume.extension_fields().len(), 1);
    assert_eq!(volume.unknown_fields().len(), 1);
    assert_eq!(implicit_volume.driver(), None);
    Ok(())
}

#[test]
fn types_top_level_configs_and_secrets() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(49), PHASE_TWO_FORMS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document was not recovered")?;
    let app_config = document
        .configs()
        .iter()
        .find(|value| value.name().value() == "app-config")
        .ok_or("app config is missing")?;
    let generated_config = document
        .configs()
        .iter()
        .find(|value| value.name().value() == "generated-config")
        .ok_or("generated config is missing")?;
    let environment_config = document
        .configs()
        .iter()
        .find(|value| value.name().value() == "environment-config")
        .ok_or("environment config is missing")?;
    let external_config = document
        .configs()
        .iter()
        .find(|value| value.name().value() == "external-config")
        .ok_or("external config is missing")?;
    let app_secret = document
        .secrets()
        .iter()
        .find(|value| value.name().value() == "app-secret")
        .ok_or("app secret is missing")?;
    let token = document
        .secrets()
        .iter()
        .find(|value| value.name().value() == "token")
        .ok_or("token secret is missing")?;
    let external_secret = document
        .secrets()
        .iter()
        .find(|value| value.name().value() == "external-secret")
        .ok_or("external secret is missing")?;

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(document.configs().len(), 5);
    assert_eq!(
        app_config.file().map(|value| value.value().as_str()),
        Some("./app.conf")
    );
    assert_eq!(
        generated_config.content().map(|value| value.value().as_str()),
        Some("mode=generated\n")
    );
    assert_eq!(
        environment_config.environment().map(|value| value.value().as_str()),
        Some("APP_CONFIG")
    );
    assert_eq!(
        external_config.external().map(Located::value),
        Some(&BooleanValue::Expression("${CONFIG_EXTERNAL:-true}".to_owned()))
    );
    assert_eq!(document.configs()[0].name().value(), "implicit");
    assert_eq!(document.configs()[0].file(), None);
    assert_eq!(document.secrets().len(), 4);
    assert_eq!(
        app_secret.file().map(|value| value.value().as_str()),
        Some("./app.secret")
    );
    assert_eq!(
        token.environment().map(|value| value.value().as_str()),
        Some("APP_TOKEN")
    );
    assert_eq!(
        external_secret.external().map(Located::value),
        Some(&BooleanValue::Literal(true))
    );
    assert_eq!(document.secrets()[0].name().value(), "implicit");
    assert_eq!(document.secrets()[0].file(), None);
    Ok(())
}

#[test]
fn invalid_phase_two_forms_return_partial_data_and_stable_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(53), INVALID_PHASE_TWO_FORMS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document was not recovered")?;
    let service = document.service("app").ok_or("app service is missing")?;
    let codes: Vec<_> = parsed
        .diagnostics()
        .iter()
        .map(compose_lens::diagnostic::Diagnostic::code)
        .collect();

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(!parsed.is_valid());
    assert_eq!(service.ports().len(), 1);
    assert_eq!(service.configs().len(), 1);
    assert_eq!(service.secrets().len(), 1);
    for expected in [
        EXPECTED_FIELD_FORM,
        PORT_EXPECTED_FORM,
        PORT_MISSING_TARGET,
        EXPECTED_SEQUENCE,
        GRANT_EXPECTED_FORM,
        GRANT_MISSING_SOURCE,
        RESOURCE_EXPECTED_FORM,
        EXPECTED_MAPPING,
    ] {
        assert!(codes.contains(&expected), "missing diagnostic {expected}");
    }
    assert!(
        parsed
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.labels().iter().all(|label| {
                label.span().source_id() == SourceId::new(53) && label.span().end() <= INVALID_PHASE_TWO_FORMS.len()
            }))
    );
    Ok(())
}

#[test]
fn types_issue_derived_runtime_values_without_erasing_authored_forms() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(59), POST_01_FORMS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let app = document.service("app").ok_or("app service expected")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());

    let Some(ExtraHosts::Short { entries, .. }) = app.extra_hosts() else {
        return Err("short extra_hosts expected".into());
    };
    assert_eq!(entries.len(), 5);
    assert_eq!(entries[1].separator(), Some(ExtraHostSeparator::Colon));
    assert_eq!(
        entries[2].address().map(compose_lens::model::HostAddress::raw),
        Some("::1")
    );
    assert_eq!(
        entries[3].address().map(compose_lens::model::HostAddress::kind),
        Some(HostAddressKind::Ipv6 { bracketed: true })
    );
    assert!(app.extra_hosts().is_some_and(ExtraHosts::contains_host_gateway));

    let user = app.user().ok_or("typed user expected")?;
    assert_eq!(user.raw().value(), "${UID:-1000}:${GID:-1000}");
    assert!(matches!(user.user(), IdentityComponent::Expression(value) if value == "${UID:-1000}"));
    assert!(matches!(user.group(), Some(IdentityComponent::Expression(value)) if value == "${GID:-1000}"));
    assert_eq!(
        app.userns_mode().map(compose_lens::model::UserNamespaceMode::kind),
        Some(UserNamespaceModeKind::PodmanKeepId)
    );
    let numeric = document
        .service("numeric-user")
        .and_then(compose_lens::model::Service::user)
        .ok_or("numeric user expected")?;
    assert!(matches!(numeric.user(), IdentityComponent::Numeric(value) if value == "1000"));
    assert!(matches!(numeric.group(), Some(IdentityComponent::Numeric(value)) if value == "1001"));
    let named = document
        .service("named-user")
        .and_then(compose_lens::model::Service::user)
        .ok_or("named user expected")?;
    assert!(matches!(named.user(), IdentityComponent::Name(value) if value == "www-data"));
    assert!(matches!(named.group(), Some(IdentityComponent::Name(value)) if value == "staff"));

    let ulimits = app.ulimits().ok_or("typed ulimits expected")?;
    assert_eq!(ulimits.entries().len(), 3);
    let nofile = ulimits
        .entries()
        .iter()
        .find(|limit| limit.name().value() == "nofile")
        .ok_or("nofile expected")?;
    let UlimitValue::Range(range) = nofile.value() else {
        return Err("nofile range expected".into());
    };
    assert_eq!(range.soft().map(Located::value), Some(&LimitValue::Unlimited));
    assert_eq!(
        range.hard().map(Located::value),
        Some(&LimitValue::Number("1048576".to_owned()))
    );

    let depends_on = app.depends_on().ok_or("typed dependencies expected")?;
    let compose_lens::model::DependsOn::Long { services, .. } = depends_on else {
        return Err("long dependencies expected".into());
    };
    assert!(services.iter().any(|dependency| {
        dependency.service().value() == "database"
            && matches!(
                dependency.condition().map(Located::value),
                Some(DependencyCondition::ServiceHealthy)
            )
    }));
    assert_eq!(
        app.healthcheck()
            .and_then(compose_lens::model::Healthcheck::test)
            .and_then(compose_lens::model::HealthcheckTest::kind),
        Some(HealthcheckTestKind::CmdShell)
    );

    let VolumeMount::Short(anonymous) = &app.volumes()[0] else {
        return Err("anonymous short volume expected".into());
    };
    assert_eq!(anonymous.source(), None);
    assert_eq!(
        anonymous.target_path().map(compose_lens::model::ContainerPath::kind),
        Some(ContainerPathKind::UnixAbsolute)
    );

    let diagnostics = document.validate_dependencies();
    for code in [
        DEPENDENCY_MISSING_SERVICE,
        DEPENDENCY_MISSING_HEALTHCHECK,
        DEPENDENCY_HEALTHCHECK_UNVERIFIED,
    ] {
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == code));
    }
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == DEPENDENCY_MISSING_SERVICE
            && diagnostic.severity() == compose_lens::diagnostic::Severity::Warning
    }));
    Ok(())
}

#[test]
fn identifies_build_and_deploy_subfields_independently() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(60), POST_01_FORMS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let app = parsed
        .document()
        .and_then(|document| document.service("app"))
        .ok_or("app service expected")?;

    let Some(Build::Definition(build)) = app.build() else {
        return Err("build definition expected".into());
    };
    for kind in [
        BuildFieldKind::Context,
        BuildFieldKind::Dockerfile,
        BuildFieldKind::Args,
        BuildFieldKind::ExtraHosts,
        BuildFieldKind::Entitlements,
        BuildFieldKind::Target,
    ] {
        assert!(build.field(kind).is_some(), "missing build field {kind:?}");
    }
    assert_eq!(build.extension_fields().len(), 1);
    assert_eq!(build.unknown_fields().len(), 1);

    let deploy = app.deploy().ok_or("deploy definition expected")?;
    for kind in [
        DeployFieldKind::Mode,
        DeployFieldKind::Replicas,
        DeployFieldKind::EndpointMode,
        DeployFieldKind::Labels,
        DeployFieldKind::Placement,
        DeployFieldKind::Resources,
        DeployFieldKind::RestartPolicy,
        DeployFieldKind::UpdateConfig,
        DeployFieldKind::RollbackConfig,
    ] {
        assert!(deploy.field(kind).is_some(), "missing deploy field {kind:?}");
    }
    assert_eq!(deploy.extension_fields().len(), 1);
    assert_eq!(deploy.unknown_fields().len(), 1);
    Ok(())
}

#[test]
fn malformed_issue_derived_fields_return_partial_data() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(61), POST_01_INVALID)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let app = parsed
        .document()
        .and_then(|document| document.service("app"))
        .ok_or("app service expected")?;

    assert!(!parsed.is_valid());
    for code in [
        EXTRA_HOST_INVALID_ENTRY,
        ULIMIT_INVALID_VALUE,
        HEALTHCHECK_INVALID_TEST,
        HEALTHCHECK_INVALID_DURATION,
        HEALTHCHECK_INVALID_RETRIES,
        DEPENDENCY_INVALID_CONDITION,
        EXPECTED_FIELD_FORM,
        EXPECTED_MAPPING,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    assert!(matches!(app.extra_hosts(), Some(ExtraHosts::Short { entries, .. }) if entries.len() == 2));
    assert_eq!(app.ulimits().map(|limits| limits.entries().len()), Some(2));
    Ok(())
}
