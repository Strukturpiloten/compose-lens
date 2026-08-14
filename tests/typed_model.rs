//! Public typed-model behavior and representation fidelity.

use compose_lens::model::{
    BUILD_DOCKERFILE_EXPECTED_NON_EMPTY, BUILD_DOCKERFILE_INLINE_CONFLICT, BUILD_EXTRA_HOSTS_DUPLICATE_ITEM,
    BUILD_EXTRA_HOSTS_EXPECTED_FORM, BUILD_EXTRA_HOSTS_EXPECTED_STRING, BUILD_ISOLATION_EXPECTED_STRING,
    BUILD_NO_CACHE_EXPECTED_BOOLEAN_OR_STRING, BUILD_SBOM_EXPECTED_BOOLEAN_OR_STRING, BUILD_SSH_DUPLICATE_ITEM,
    BUILD_SSH_EXPECTED_FORM, BooleanValue, Build, BuildAdditionalContexts, BuildArgs, BuildExtraHostAddresses,
    BuildExtraHosts, BuildFieldKind, BuildNoCache, BuildSbom, BuildSshForm, CAP_ADD_DUPLICATE_ITEM,
    CAP_ADD_EXPECTED_SEQUENCE, CAP_ADD_EXPECTED_STRING, CAP_DROP_DUPLICATE_ITEM, CAP_DROP_EXPECTED_SEQUENCE,
    CAP_DROP_EXPECTED_STRING, Command, ComposeDocument, ComposeScalar, ConfigGrant, ContainerPathKind,
    DEPENDENCY_HEALTHCHECK_UNVERIFIED, DEPENDENCY_INVALID_CONDITION, DEPENDENCY_MISSING_HEALTHCHECK,
    DEPENDENCY_MISSING_SERVICE, DEPLOY_ENDPOINT_MODE_PORTABILITY, DEPLOY_MODE_PORTABILITY, DEVICE_EXPECTED_FORM,
    DEVICE_EXPECTED_STRING, DEVICE_MISSING_SOURCE, DEVICES_EXPECTED_SEQUENCE, DNS_EXPECTED_FORM, DNS_EXPECTED_STRING,
    DNS_SEARCH_DUPLICATE_ITEM, DNS_SEARCH_EXPECTED_FORM, DNS_SEARCH_EXPECTED_STRING, DUPLICATE_FIELD,
    DependencyCondition, DeployEndpointMode, DeployFieldKind, DeployMode, DeployPlacementMaxReplicasPerNode,
    DeployReplicas, DeployResourceCpus, DeployResourceMemoryKind, DeployResourceMemoryUnit, DeployResourcePids,
    DeployRestartCondition, DeployRestartMaxAttempts, Device, DnsForm, DnsSearchForm, ENVIRONMENT_FILE_EXPECTED_FORM,
    ENVIRONMENT_FILE_INVALID_FORMAT, ENVIRONMENT_FILE_MISSING_PATH, EXPECTED_BOOLEAN, EXPECTED_FIELD_FORM,
    EXPECTED_MAPPING, EXPECTED_SCALAR, EXPECTED_SEQUENCE, EXTENDS_MISSING_SERVICE, EXTRA_HOST_INVALID_ENTRY,
    Entrypoint, Environment, EnvironmentFile, EnvironmentFileFormatKind, ExtraHostSeparator, ExtraHosts,
    GRANT_EXPECTED_FORM, GRANT_MISSING_SOURCE, HEALTHCHECK_INVALID_DURATION, HEALTHCHECK_INVALID_RETRIES,
    HEALTHCHECK_INVALID_TEST, HealthcheckTestKind, HostAddressKind, HostnameKind, IdentityComponent, KeyValueEntry,
    LOGGING_DRIVER_EXPECTED_STRING, LOGGING_EXPECTED_MAPPING, LOGGING_OPTION_EMPTY_KEY, LOGGING_OPTION_EXPECTED_SCALAR,
    LOGGING_OPTIONS_EXPECTED_MAPPING, Labels, LimitValue, Located, LoggingOptionValue, MountType, PORT_EXPECTED_FORM,
    PORT_MISSING_TARGET, POST_START_MISSING_COMMAND, PRE_STOP_MISSING_COMMAND, PROVIDER_MISSING_TYPE, Port,
    RESOURCE_EXPECTED_FORM, RESTART_INVALID_POLICY, RestartPolicyKind, STOP_GRACE_PERIOD_INVALID,
    SYSCTLS_DUPLICATE_ITEM, SYSCTLS_EMPTY_KEY, SYSCTLS_EXPECTED_FORM, SYSCTLS_EXPECTED_SCALAR, SYSCTLS_EXPECTED_STRING,
    SecretGrant, SelinuxRelabel, ServiceNetworks, StopGracePeriod, SysctlsForm, ULIMIT_INVALID_NAME,
    ULIMIT_INVALID_VALUE, ULIMIT_MISSING_RANGE_MEMBER, UlimitValue, UserNamespaceModeKind, VOLUME_EXPECTED_FORM,
    VOLUME_EXTERNAL_DRIVER_CONFIGURATION, VOLUME_EXTERNAL_LABELS_CONFIGURATION, VOLUME_INVALID_SELINUX,
    VOLUME_MISSING_TARGET, VOLUME_MISSING_TYPE, VolumeMount, VolumeSyntax,
};

#[test]
fn validates_ulimit_names_and_required_range_members_without_losing_valid_siblings()
-> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(680),
        concat!(
            "services:\n",
            "  empty:\n",
            "    ulimits: {}\n",
            "  app:\n",
            "    ulimits:\n",
            "      nofile: -1\n",
            "      nproc: \"001024\"\n",
            "      core:\n",
            "        soft: \"0\"\n",
            "        hard: 8\n",
            "      Bad: 1\n",
            "      missingsoft: {hard: 2}\n",
            "      missinghard: {soft: 3}\n",
            "      boolean: true\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;
    assert!(
        document
            .service("empty")
            .and_then(compose_lens::model::Service::ulimits)
            .is_some_and(|limits| limits.entries().is_empty())
    );
    let limits = document
        .service("app")
        .and_then(compose_lens::model::Service::ulimits)
        .ok_or("partial ulimits expected")?;
    assert_eq!(limits.entries().len(), 7);
    assert!(matches!(limits.entries()[0].value(), UlimitValue::Single(value) if value.value().raw() == "-1"));
    assert!(matches!(limits.entries()[1].value(), UlimitValue::Single(value) if value.value().raw() == "001024"));
    assert!(matches!(limits.entries()[2].value(), UlimitValue::Range(range)
        if range.soft().is_some() && range.hard().is_some()));
    for code in [ULIMIT_INVALID_NAME, ULIMIT_MISSING_RANGE_MEMBER, ULIMIT_INVALID_VALUE] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == ULIMIT_MISSING_RANGE_MEMBER)
            .count(),
        2
    );
    Ok(())
}

#[test]
fn retains_authored_build_entitlements_as_opaque_ordered_strings() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(2001),
        concat!(
            "services:\n",
            "  app:\n    build:\n      entitlements: [network.host, \"security.insecure\", network.host, \"\"]\n",
            "  empty:\n    build: {entitlements: []}\n",
            "  malformed:\n    build: {entitlements: [network.host, false, {}, \"security.insecure\"]}\n",
            "  outer:\n    build: {entitlements: network.host}\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    assert!(
        matches!(authored_build_definition(document, "app")?.entitlements(), Some(values)
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["network.host", "security.insecure", "network.host", ""])
    );
    assert!(matches!(authored_build_definition(document, "empty")?.entitlements(), Some(values) if values.is_empty()));
    assert!(
        matches!(authored_build_definition(document, "malformed")?.entitlements(), Some(values)
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["network.host", "security.insecure"])
    );
    let outer = authored_build_definition(document, "outer")?;
    assert!(outer.entitlements().is_none());
    assert!(outer.field(BuildFieldKind::Entitlements).is_some());
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SEQUENCE)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    Ok(())
}

#[test]
fn retains_authored_build_provenance_categories_recovery_and_duplicates() -> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::BuildProvenance;
    let syntax = SyntaxDocument::parse(
        SourceId::new(2202),
        concat!(
            "services:\n  truthy:\n    build: {provenance: true}\n  falsey:\n    build: {provenance: false}\n",
            "  string:\n    build: {provenance: \"mode=max\"}\n  empty:\n    build: {provenance: \"\"}\n",
            "  invalid-number:\n    build: {context: retained, provenance: 1}\n  invalid-sequence:\n    build: {context: retained, provenance: []}\n  invalid-map:\n    build: {context: retained, provenance: {mode: max}}\n  invalid-null:\n    build: {context: retained, provenance: null}\n",
            "  duplicate:\n    build:\n      provenance: true\n      provenance: \"mode=max\"\n"
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;
    let value = |name| {
        authored_build_definition(document, name)
            .ok()
            .and_then(|d| d.provenance().map(Located::value))
    };
    assert!(matches!(value("truthy"), Some(BuildProvenance::Boolean(true))));
    assert!(matches!(value("falsey"), Some(BuildProvenance::Boolean(false))));
    assert!(matches!(value("string"), Some(BuildProvenance::String(v)) if v == "mode=max"));
    assert!(matches!(value("empty"), Some(BuildProvenance::String(v)) if v.is_empty()));
    for name in ["invalid-number", "invalid-sequence", "invalid-map", "invalid-null"] {
        let d = authored_build_definition(document, name)?;
        assert!(d.provenance().is_none() && d.context().is_some());
    }
    assert!(matches!(value("duplicate"), Some(BuildProvenance::Boolean(true))));
    assert!(parsed.diagnostics().iter().any(|d| d.code() == DUPLICATE_FIELD));
    Ok(())
}

#[test]
fn retains_authored_no_cache_filter_scalar_list_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::BuildNoCacheFilter;
    let syntax = SyntaxDocument::parse(
        SourceId::new(2301),
        "services:\n  scalar:\n    build: {no_cache_filter: \"\"}\n  list:\n    build: {no_cache_filter: [one, one, false, two]}\n  invalid:\n    build: {context: kept, no_cache_filter: {bad: value}}\n  number:\n    build: {context: kept, no_cache_filter: 1}\n  null:\n    build: {context: kept, no_cache_filter: null}\n",
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let doc = parsed.document().ok_or("doc")?;
    assert!(
        matches!(authored_build_definition(doc,"scalar")?.no_cache_filter(),Some(BuildNoCacheFilter::Scalar(v)) if v.value().is_empty())
    );
    assert!(
        matches!(authored_build_definition(doc,"list")?.no_cache_filter(),Some(BuildNoCacheFilter::List(v)) if v.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>()==["one","one","two"])
    );
    for name in ["invalid", "number", "null"] {
        let invalid = authored_build_definition(doc, name)?;
        assert!(invalid.no_cache_filter().is_none() && invalid.context().is_some());
    }
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|d| d.code() == compose_lens::model::BUILD_NO_CACHE_FILTER_DUPLICATE_ITEM)
    );
    Ok(())
}

#[test]
fn retains_authored_deploy_endpoint_modes_and_recovers_invalid_fields() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(2501),
        concat!(
            "services:\n",
            "  vip:\n    deploy: {endpoint_mode: vip}\n",
            "  dnsrr:\n    deploy: {endpoint_mode: dnsrr}\n",
            "  provider:\n    deploy: {endpoint_mode: mesh}\n",
            "  expression:\n    deploy: {endpoint_mode: \"${ENDPOINT_MODE}\"}\n",
            "  invalid:\n    deploy:\n      endpoint_mode: true\n      replicas: 2\n",
            "  duplicate:\n    deploy:\n      endpoint_mode: vip\n      endpoint_mode: dnsrr\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let endpoint_mode = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.endpoint_mode())
            .map(Located::value)
    };
    assert!(matches!(endpoint_mode("vip"), Some(DeployEndpointMode::Vip)));
    assert!(matches!(endpoint_mode("dnsrr"), Some(DeployEndpointMode::Dnsrr)));
    assert!(matches!(endpoint_mode("provider"), Some(DeployEndpointMode::Other(value)) if value == "mesh"));
    assert!(
        matches!(endpoint_mode("expression"), Some(DeployEndpointMode::Other(value)) if value == "${ENDPOINT_MODE}")
    );
    assert!(matches!(endpoint_mode("duplicate"), Some(DeployEndpointMode::Vip)));
    let invalid = document
        .service("invalid")
        .and_then(compose_lens::model::Service::deploy)
        .ok_or("invalid deploy definition expected")?;
    assert!(invalid.endpoint_mode().is_none() && invalid.field(DeployFieldKind::Replicas).is_some());
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == DEPLOY_ENDPOINT_MODE_PORTABILITY)
            .count(),
        2
    );
    for code in [EXPECTED_SCALAR, DUPLICATE_FIELD] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_authored_deploy_modes_and_recovers_invalid_fields() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(2601),
        concat!(
            "services:\n",
            "  global:\n    deploy: {mode: global}\n",
            "  replicated:\n    deploy: {mode: replicated}\n",
            "  provider:\n    deploy: {mode: job}\n",
            "  empty:\n    deploy: {mode: \"\"}\n",
            "  expression:\n    deploy: {mode: \"${DEPLOY_MODE}\"}\n",
            "  invalid:\n    deploy:\n      mode: false\n      replicas: 2\n",
            "  duplicate:\n    deploy:\n      mode: global\n      mode: replicated\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let mode = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.mode())
            .map(Located::value)
    };
    assert!(matches!(mode("global"), Some(DeployMode::Global)));
    assert!(matches!(mode("replicated"), Some(DeployMode::Replicated)));
    for service in ["provider", "empty", "expression"] {
        assert!(matches!(mode(service), Some(DeployMode::Other(_))));
    }
    assert!(matches!(mode("duplicate"), Some(DeployMode::Global)));
    let invalid = document
        .service("invalid")
        .and_then(compose_lens::model::Service::deploy)
        .ok_or("invalid deploy definition expected")?;
    assert!(invalid.mode().is_none() && invalid.field(DeployFieldKind::Replicas).is_some());
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == DEPLOY_MODE_PORTABILITY)
            .count(),
        3
    );
    for code in [EXPECTED_SCALAR, DUPLICATE_FIELD] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_authored_deploy_replicas_spelling_and_scalar_category() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(2701),
        concat!(
            "services:\n",
            "  number:\n    deploy: {replicas: 2}\n",
            "  zero:\n    deploy: {replicas: 0}\n",
            "  decimal:\n    deploy: {replicas: 1.50}\n",
            "  string:\n    deploy: {replicas: \"2\"}\n",
            "  empty:\n    deploy: {replicas: \"\"}\n",
            "  expression:\n    deploy: {replicas: \"${REPLICAS}\"}\n",
            "  invalid:\n    deploy:\n      replicas: false\n      mode: global\n",
            "  null:\n    deploy: {replicas: null}\n",
            "  mapping:\n    deploy: {replicas: {count: 2}}\n",
            "  sequence:\n    deploy: {replicas: [2]}\n",
            "  duplicate:\n    deploy:\n      replicas: 2\n      replicas: 3\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let replicas = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.replicas())
            .map(Located::value)
    };
    for (service, expected) in [("number", "2"), ("zero", "0"), ("decimal", "1.50"), ("duplicate", "2")] {
        assert!(matches!(replicas(service), Some(DeployReplicas::YamlNumber(value)) if value == expected));
    }
    for (service, expected) in [("string", "2"), ("empty", ""), ("expression", "${REPLICAS}")] {
        assert!(matches!(replicas(service), Some(DeployReplicas::String(value)) if value == expected));
    }
    let invalid = document
        .service("invalid")
        .and_then(compose_lens::model::Service::deploy)
        .ok_or("invalid deploy definition expected")?;
    assert!(invalid.replicas().is_none() && invalid.mode().is_some());
    for service in ["null", "mapping", "sequence"] {
        assert!(replicas(service).is_none());
    }
    for code in [EXPECTED_SCALAR, DUPLICATE_FIELD] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_authored_deploy_labels_forms_scalars_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(2801),
        concat!(
            "services:\n  map:\n    deploy:\n      labels:\n        text: value\n        number: 2\n        boolean: true\n        null: null\n",
            "  list:\n    deploy:\n      labels: [bare, pair=value, pair=value, 2]\n",
            "  malformed:\n    deploy:\n      labels: true\n      mode: global\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let labels = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.labels())
    };
    let Some(Labels::Map { entries, .. }) = labels("map") else {
        return Err("mapping labels expected".into());
    };
    assert!(matches!(entries[0].value().value(), ComposeScalar::String(value) if value == "value"));
    assert!(matches!(entries[1].value().value(), ComposeScalar::Number(value) if value == "2"));
    assert!(matches!(entries[2].value().value(), ComposeScalar::Boolean(true)));
    assert!(matches!(entries[3].value().value(), ComposeScalar::Null));
    let Some(Labels::List { values, .. }) = labels("list") else {
        return Err("list labels expected".into());
    };
    assert_eq!(
        values.iter().map(Located::value).collect::<Vec<_>>(),
        ["bare", "pair=value", "pair=value"]
    );
    let malformed = document
        .service("malformed")
        .and_then(compose_lens::model::Service::deploy)
        .ok_or("malformed deploy expected")?;
    assert!(malformed.labels().is_none() && malformed.mode().is_some());
    for code in [EXPECTED_FIELD_FORM, EXPECTED_SCALAR] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_authored_deploy_restart_policy_members() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(2901),
        "services:\n  app:\n    deploy:\n      restart_policy:\n        condition: any\n        delay: 1.5s\n        max_attempts: 003\n        window: \"${WINDOW}\"\n        x-note: kept\n",
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let policy = parsed
        .document()
        .and_then(|doc| doc.service("app"))
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.restart_policy())
        .ok_or("restart policy expected")?;
    assert!(matches!(
        policy.condition().map(Located::value),
        Some(DeployRestartCondition::Any)
    ));
    assert_eq!(policy.delay().map(|value| value.value().raw()), Some("1.5s"));
    assert!(
        matches!(policy.max_attempts().map(Located::value), Some(DeployRestartMaxAttempts::YamlNumber(value)) if value == "003")
    );
    assert_eq!(policy.window().map(|value| value.value().raw()), Some("${WINDOW}"));
    assert_eq!(policy.extension_fields().len(), 1);
    Ok(())
}

#[test]
fn retains_authored_deploy_update_config_members_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  valid:\n    deploy:\n      update_config:\n        parallelism: 003\n        delay: later\n        monitor: observe\n        failure_action: continue\n        max_failure_ratio: 0.25\n        order: start-first\n        x-note: kept\n        future: kept\n  empty:\n    deploy: {update_config: {}}\n  malformed:\n    deploy:\n      update_config:\n        parallelism: 1.5\n        delay: true\n        monitor: !!timestamp 2023-12-25\n        failure_action: pause\n        max_failure_ratio: true\n        order: vendor\n        parallelism: 2\n  outer:\n    deploy: {update_config: [bad]}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3150), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let config = |name| {
        parsed
            .document()
            .and_then(|doc| doc.service(name))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.update_config())
    };
    let valid = config("valid").ok_or("valid update config")?;
    assert!(
        matches!(valid.parallelism().map(Located::value), Some(compose_lens::model::DeployUpdateParallelism::YamlInteger(value)) if value == "003")
    );
    assert_eq!(valid.delay().map(Located::value).map(String::as_str), Some("later"));
    assert_eq!(
        valid.failure_action().map(Located::value).map(String::as_str),
        Some("continue")
    );
    assert!(matches!(
        valid.order().map(Located::value),
        Some(compose_lens::model::DeployUpdateOrder::StartFirst)
    ));
    assert_eq!(valid.extension_fields().len(), 1);
    assert_eq!(valid.unknown_fields().len(), 1);
    assert!(config("empty").is_some_and(|config| config.parallelism().is_none() && config.unknown_fields().is_empty()));
    let malformed = config("malformed").ok_or("malformed update config")?;
    assert_eq!(
        malformed.failure_action().map(Located::value).map(String::as_str),
        Some("pause")
    );
    assert!(
        matches!(malformed.order().map(Located::value), Some(compose_lens::model::DeployUpdateOrder::Other(value)) if value == "vendor")
    );
    assert_eq!(malformed.unknown_fields().len(), 4);
    assert!(config("outer").is_none());
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == compose_lens::model::DEPLOY_UPDATE_CONFIG_ORDER_PORTABILITY)
    );
    Ok(())
}

#[test]
fn retains_authored_deploy_rollback_config_members_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  valid:\n    deploy:\n      rollback_config:\n        parallelism: 003\n        delay: later\n        monitor: observe\n        failure_action: continue\n        max_failure_ratio: 0.25\n        order: stop-first\n        x-note: kept\n        future: kept\n  empty:\n    deploy: {rollback_config: {}}\n  malformed:\n    deploy:\n      rollback_config:\n        parallelism: 1.5\n        delay: true\n        monitor: !!timestamp 2023-12-25\n        failure_action: pause\n        max_failure_ratio: true\n        order: vendor\n        parallelism: 2\n  outer:\n    deploy: {rollback_config: [bad]}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3154), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let config = |name| {
        parsed
            .document()
            .and_then(|doc| doc.service(name))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.rollback_config())
    };
    let valid = config("valid").ok_or("valid rollback config")?;
    assert!(matches!(
        valid.parallelism().map(Located::value),
        Some(compose_lens::model::DeployRollbackParallelism::YamlInteger(value)) if value == "003"
    ));
    assert_eq!(valid.delay().map(Located::value).map(String::as_str), Some("later"));
    assert_eq!(
        valid.failure_action().map(Located::value).map(String::as_str),
        Some("continue")
    );
    assert!(matches!(
        valid.order().map(Located::value),
        Some(compose_lens::model::DeployRollbackOrder::StopFirst)
    ));
    assert_eq!(valid.extension_fields().len(), 1);
    assert_eq!(valid.unknown_fields().len(), 1);
    assert!(config("empty").is_some_and(|config| config.parallelism().is_none() && config.unknown_fields().is_empty()));
    let malformed = config("malformed").ok_or("malformed rollback config")?;
    assert_eq!(
        malformed.failure_action().map(Located::value).map(String::as_str),
        Some("pause")
    );
    assert!(matches!(
        malformed.order().map(Located::value),
        Some(compose_lens::model::DeployRollbackOrder::Other(value)) if value == "vendor"
    ));
    assert_eq!(malformed.unknown_fields().len(), 4);
    assert!(config("outer").is_none());
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.code() == compose_lens::model::DEPLOY_ROLLBACK_CONFIG_ORDER_PORTABILITY })
    );
    Ok(())
}

#[test]
fn retains_authored_credential_spec_members_spans_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  valid:\n    credential_spec:\n      config: \"\"\n      file: ' C:\\\\gmsa.json '\n      registry: \"${REGISTRY:-registry://account}\"\n      x-note: retained\n      future: retained\n  uri:\n    credential_spec: {config: 'config://credential'}\n  empty:\n    credential_spec: {}\n  malformed:\n    credential_spec:\n      config: valid\n      config: duplicate\n      file: !!timestamp 2024-01-01\n      registry: [bad]\n  outer:\n    credential_spec: [bad]\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3158), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let credential_spec = |name| {
        parsed
            .document()
            .and_then(|document| document.service(name))
            .and_then(compose_lens::model::Service::credential_spec)
    };
    let valid = credential_spec("valid").ok_or("valid credential spec")?;
    assert_eq!(valid.config().map(Located::value).map(String::as_str), Some(""));
    assert_eq!(
        valid.file().map(Located::value).map(String::as_str),
        Some(" C:\\\\gmsa.json ")
    );
    assert_eq!(
        valid.registry().map(Located::value).map(String::as_str),
        Some("${REGISTRY:-registry://account}")
    );
    assert_eq!(
        &source[valid.file().ok_or("file")?.span().range()],
        "' C:\\\\gmsa.json '"
    );
    assert_eq!(valid.extension_fields().len(), 1);
    assert_eq!(valid.unknown_fields().len(), 1);
    assert_eq!(
        credential_spec("uri")
            .and_then(compose_lens::model::CredentialSpec::config)
            .map(Located::value)
            .map(String::as_str),
        Some("config://credential")
    );
    assert!(credential_spec("empty").is_some_and(|value| value.config().is_none()));
    let malformed = credential_spec("malformed").ok_or("malformed credential spec")?;
    assert_eq!(
        malformed.config().map(Located::value).map(String::as_str),
        Some("valid")
    );
    assert_eq!(malformed.unknown_fields().len(), 2);
    assert!(credential_spec("outer").is_none());
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_MAPPING)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    Ok(())
}

#[test]
fn retains_authored_provider_options_spans_categories_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  valid:\n    provider:\n      type: \"\"\n      options:\n        text: ' raw '\n        count: 01\n        enabled: true\n        values: [first, 2, false, null, {bad: value}]\n        x-option: retained\n        \"\": empty\n        text: duplicate\n      x-parent: retained\n      future: retained\n  missing:\n    provider: {options: {kept: value}}\n  malformed:\n    provider:\n      type: true\n      options: [bad]\n  outer:\n    provider: [bad]\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3166), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let provider = |name| {
        parsed
            .document()
            .and_then(|document| document.service(name))
            .and_then(compose_lens::model::Service::provider)
    };
    let valid = provider("valid").ok_or("valid provider")?;
    assert_eq!(valid.type_().map(Located::value).map(String::as_str), Some(""));
    assert_eq!(valid.extension_fields().len(), 1);
    assert_eq!(valid.unknown_fields().len(), 1);
    let options = valid.options().ok_or("provider options")?;
    assert_eq!(
        &source[options.span().range()],
        "text: ' raw '\n        count: 01\n        enabled: true\n        values: [first, 2, false, null, {bad: value}]\n        x-option: retained\n        \"\": empty\n        text: duplicate\n"
    );
    assert_eq!(options.entries().len(), 5);
    assert_eq!(options.unmodeled_entries().len(), 2);
    assert!(
        matches!(options.entries()[0].value(), compose_lens::model::ProviderOptionValue::Scalar(value) if matches!(value.value(), ComposeScalar::String(value) if value == " raw "))
    );
    assert!(
        matches!(options.entries()[1].value(), compose_lens::model::ProviderOptionValue::Scalar(value) if matches!(value.value(), ComposeScalar::Number(value) if value == "01"))
    );
    assert!(
        matches!(options.entries()[2].value(), compose_lens::model::ProviderOptionValue::Scalar(value) if matches!(value.value(), ComposeScalar::Boolean(true)))
    );
    let compose_lens::model::ProviderOptionValue::Sequence { items, .. } = options.entries()[3].value() else {
        return Err("provider sequence expected".into());
    };
    assert_eq!(items.len(), 5);
    assert!(matches!(items[0], compose_lens::model::ProviderOptionItem::Scalar(_)));
    assert!(matches!(
        items[3],
        compose_lens::model::ProviderOptionItem::Unmodeled { .. }
    ));
    assert!(matches!(
        items[4],
        compose_lens::model::ProviderOptionItem::Unmodeled { .. }
    ));
    assert!(provider("missing").is_some_and(|value| value.type_().is_none()));
    let malformed = provider("malformed").ok_or("malformed provider")?;
    assert!(malformed.type_().is_none() && malformed.options().is_none());
    assert!(provider("outer").is_none());
    for code in [
        DUPLICATE_FIELD,
        EXPECTED_MAPPING,
        EXPECTED_SCALAR,
        EXPECTED_FIELD_FORM,
        PROVIDER_MISSING_TYPE,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_authored_post_start_hooks_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    post_start:\n      - command: null\n        environment: [ONE=1]\n        privileged: false\n        user: ' 1000 '\n        working_dir: /work\n        x-note: retained\n        future: retained\n      - command: [echo, second]\n      - malformed\n      - command: {bad: form}\n        command: duplicate\n        environment: {ONE: one}\n        privileged: true\n        user: 1000\n        working_dir: false\n  outer:\n    post_start: {command: nope}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3173), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let hooks = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::post_start)
        .ok_or("post-start hooks")?;
    assert_eq!(
        &source[hooks.span().range()],
        "- command: null\n        environment: [ONE=1]\n        privileged: false\n        user: ' 1000 '\n        working_dir: /work\n        x-note: retained\n        future: retained\n      - command: [echo, second]\n      - malformed\n      - command: {bad: form}\n        command: duplicate\n        environment: {ONE: one}\n        privileged: true\n        user: 1000\n        working_dir: false\n"
    );
    assert_eq!(hooks.entries().len(), 4);
    let compose_lens::model::PostStartHook::Hook(first) = &hooks.entries()[0] else {
        return Err("first post-start hook expected".into());
    };
    assert!(matches!(first.command(), Some(Command::Null(_))));
    assert!(matches!(first.environment(), Some(Environment::List { entries, .. }) if entries.len() == 1));
    assert_eq!(first.user().map(Located::value).map(String::as_str), Some(" 1000 "));
    assert_eq!(first.extension_fields().len(), 1);
    assert_eq!(first.unknown_fields().len(), 1);
    assert!(matches!(
        hooks.entries()[1],
        compose_lens::model::PostStartHook::Hook(ref hook)
            if matches!(hook.command(), Some(Command::List { values, .. }) if values.len() == 2)
    ));
    assert!(matches!(
        hooks.entries()[2],
        compose_lens::model::PostStartHook::Unmodeled { .. }
    ));
    let compose_lens::model::PostStartHook::Hook(malformed) = &hooks.entries()[3] else {
        return Err("recovered post-start hook expected".into());
    };
    assert!(malformed.command().is_none());
    assert!(matches!(malformed.environment(), Some(Environment::Map { entries, .. }) if entries.len() == 1));
    assert_eq!(malformed.unknown_fields().len(), 4);
    assert!(
        parsed
            .document()
            .and_then(|document| document.service("outer"))
            .and_then(compose_lens::model::Service::post_start)
            .is_none()
    );
    for code in [
        DUPLICATE_FIELD,
        EXPECTED_FIELD_FORM,
        EXPECTED_MAPPING,
        EXPECTED_SCALAR,
        EXPECTED_SEQUENCE,
        POST_START_MISSING_COMMAND,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_authored_pre_stop_hooks_with_distinct_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    pre_stop:\n      - command: null\n        environment: {LOCAL: value}\n        x-note: retained\n        future: retained\n      - command: [echo, stop]\n      - malformed\n      - command: first\n        command: duplicate\n  missing:\n    pre_stop: [{environment: [LOCAL=value]}]\n  outer:\n    pre_stop: {command: nope}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3180), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let hooks = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::pre_stop)
        .ok_or("pre-stop hooks")?;
    assert_eq!(hooks.entries().len(), 4);
    let compose_lens::model::PreStopHook::Hook(first) = &hooks.entries()[0] else {
        return Err("first pre-stop hook expected".into());
    };
    assert!(matches!(first.command(), Some(Command::Null(_))));
    assert!(matches!(first.environment(), Some(Environment::Map { entries, .. }) if entries.len() == 1));
    assert_eq!(first.extension_fields().len(), 1);
    assert_eq!(first.unknown_fields().len(), 1);
    assert!(matches!(
        hooks.entries()[2],
        compose_lens::model::PreStopHook::Unmodeled { .. }
    ));
    assert!(parsed
        .document()
        .and_then(|document| document.service("missing"))
        .and_then(compose_lens::model::Service::pre_stop)
        .is_some_and(|hooks| matches!(hooks.entries()[0], compose_lens::model::PreStopHook::Hook(ref hook) if hook.command().is_none())));
    assert!(
        parsed
            .document()
            .and_then(|document| document.service("outer"))
            .and_then(compose_lens::model::Service::pre_stop)
            .is_none()
    );
    for code in [
        DUPLICATE_FIELD,
        EXPECTED_MAPPING,
        EXPECTED_SEQUENCE,
        PRE_STOP_MISSING_COMMAND,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_authored_pre_start_hooks_without_requiring_commands() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    pre_start:\n      - {}\n      - command: null\n      - command: echo start\n      - command: [echo, start]\n        image: 'not an image reference @ all'\n        environment: [LOCAL=value]\n        privileged: true\n        per_replica: \"${REPLICA}\"\n        user: hook-user\n        working_dir: /hook\n        x-note: retained\n        future: retained\n      - command: first\n        command: duplicate\n        image: 1\n        privileged: sometimes\n        per_replica: maybe\n        user: 1000\n        working_dir: false\n      - malformed\n  outer:\n    pre_start: {command: nope}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3187), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let hooks = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::pre_start)
        .ok_or("pre-start hooks")?;
    assert_eq!(hooks.entries().len(), 6);
    assert!(matches!(
        hooks.entries()[0],
        compose_lens::model::PreStartHook::Hook(ref hook) if hook.command().is_none()
    ));
    assert!(matches!(
        hooks.entries()[1],
        compose_lens::model::PreStartHook::Hook(ref hook) if matches!(hook.command(), Some(Command::Null(_)))
    ));
    assert!(matches!(
        hooks.entries()[2],
        compose_lens::model::PreStartHook::Hook(ref hook) if matches!(hook.command(), Some(Command::String(_)))
    ));
    let compose_lens::model::PreStartHook::Hook(full) = &hooks.entries()[3] else {
        return Err("complete pre-start hook expected".into());
    };
    assert!(matches!(full.command(), Some(Command::List { values, .. }) if values.len() == 2));
    assert_eq!(
        full.image().map(Located::value).map(String::as_str),
        Some("not an image reference @ all")
    );
    assert!(matches!(full.environment(), Some(Environment::List { entries, .. }) if entries.len() == 1));
    assert!(matches!(
        full.privileged().map(Located::value),
        Some(BooleanValue::Literal(true))
    ));
    assert!(
        matches!(full.per_replica().map(Located::value), Some(BooleanValue::Expression(value)) if value == "${REPLICA}")
    );
    assert_eq!(full.extension_fields().len(), 1);
    assert_eq!(full.unknown_fields().len(), 1);
    let compose_lens::model::PreStartHook::Hook(malformed) = &hooks.entries()[4] else {
        return Err("recovered pre-start hook expected".into());
    };
    assert!(malformed.image().is_none() && malformed.privileged().is_none() && malformed.per_replica().is_none());
    assert_eq!(malformed.unknown_fields().len(), 6);
    assert!(matches!(
        hooks.entries()[5],
        compose_lens::model::PreStartHook::Unmodeled { .. }
    ));
    assert!(
        parsed
            .document()
            .and_then(|document| document.service("outer"))
            .and_then(compose_lens::model::Service::pre_start)
            .is_none()
    );
    for code in [
        DUPLICATE_FIELD,
        EXPECTED_BOOLEAN,
        EXPECTED_MAPPING,
        EXPECTED_SCALAR,
        EXPECTED_SEQUENCE,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    assert!(
        !parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| { matches!(diagnostic.code(), POST_START_MISSING_COMMAND | PRE_STOP_MISSING_COMMAND) })
    );
    Ok(())
}

#[test]
fn retains_authored_service_runtime_strings_and_malformed_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  empty:\n    runtime: \"\"\n  expression:\n    runtime: \"${RUNTIME}\"\n  duplicate:\n    runtime: first\n    runtime: second\n  null:\n    runtime: null\n  number:\n    runtime: 1\n  boolean:\n    runtime: true\n  sequence:\n    runtime: [runc]\n  mapping:\n    runtime: {name: runc}\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3194), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document")?;
    assert_eq!(
        document
            .service("empty")
            .and_then(compose_lens::model::Service::runtime)
            .map(Located::value)
            .map(String::as_str),
        Some("")
    );
    assert_eq!(
        document
            .service("expression")
            .and_then(compose_lens::model::Service::runtime)
            .map(Located::value)
            .map(String::as_str),
        Some("${RUNTIME}")
    );
    assert_eq!(
        document
            .service("duplicate")
            .and_then(compose_lens::model::Service::runtime)
            .map(Located::value)
            .map(String::as_str),
        Some("first")
    );
    for service in ["null", "number", "boolean", "sequence", "mapping"] {
        let service = document.service(service).ok_or("service")?;
        assert!(service.runtime().is_none());
        assert!(
            service
                .unknown_fields()
                .iter()
                .any(|field| field.name().value() == "runtime")
        );
    }
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    Ok(())
}

#[test]
fn retains_authored_cgroup_namespaces_and_malformed_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  host: {cgroup: host}\n",
        "  private: {cgroup: private}\n",
        "  expression: {cgroup: \"${CGROUP}\"}\n",
        "  invalid: {cgroup: \"\"}\n",
        "  case: {cgroup: Host}\n",
        "  none: {cgroup: none}\n",
        "  duplicate:\n    cgroup: host\n    cgroup: private\n",
        "  omitted: {image: example/app}\n",
        "  null: {cgroup: null}\n",
        "  number: {cgroup: 1}\n",
        "  boolean: {cgroup: true}\n",
        "  sequence: {cgroup: [host]}\n",
        "  mapping: {cgroup: {mode: host}}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3231), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document")?;
    let cgroup = |name| document.service(name).and_then(compose_lens::model::Service::cgroup);
    assert!(matches!(
        cgroup("host").map(compose_lens::model::CgroupNamespace::kind),
        Some(compose_lens::model::CgroupNamespaceKind::Host)
    ));
    assert!(matches!(
        cgroup("private").map(compose_lens::model::CgroupNamespace::kind),
        Some(compose_lens::model::CgroupNamespaceKind::Private)
    ));
    assert!(matches!(
        cgroup("expression").map(compose_lens::model::CgroupNamespace::kind),
        Some(compose_lens::model::CgroupNamespaceKind::Expression(value)) if value == "${CGROUP}"
    ));
    for (service, expected) in [("invalid", ""), ("case", "Host"), ("none", "none")] {
        let cgroup = cgroup(service).ok_or("retained cgroup")?;
        assert!(!cgroup.is_valid());
        assert!(matches!(cgroup.kind(), compose_lens::model::CgroupNamespaceKind::Other(value) if value == expected));
    }
    let duplicate = cgroup("duplicate").ok_or("duplicate cgroup")?;
    assert!(matches!(
        duplicate.kind(),
        compose_lens::model::CgroupNamespaceKind::Host
    ));
    assert_eq!(duplicate.raw().value(), "host");
    assert!(cgroup("omitted").is_none());
    for service in ["null", "number", "boolean", "sequence", "mapping"] {
        let service = document.service(service).ok_or("malformed service")?;
        assert!(service.cgroup().is_none());
        assert!(
            service
                .unknown_fields()
                .iter()
                .any(|field| field.name().value() == "cgroup")
        );
    }
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == compose_lens::model::CGROUP_NAMESPACE_INVALID)
            .count(),
        3
    );
    for code in [DUPLICATE_FIELD, EXPECTED_SCALAR] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_authored_cgroup_parent_strings_and_malformed_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  ordinary: {cgroup_parent: parent.slice}\n",
        "  empty: {cgroup_parent: \"\"}\n",
        "  whitespace: {cgroup_parent: \" parent \"}\n",
        "  expression: {cgroup_parent: \"${PARENT}\"}\n",
        "  independent:\n    cgroup: private\n    cgroup_parent: unrelated\n",
        "  duplicate:\n    cgroup_parent: first\n    cgroup_parent: second\n",
        "  omitted: {image: example/app}\n",
        "  null: {cgroup_parent: null}\n",
        "  number: {cgroup_parent: 1}\n",
        "  sequence: {cgroup_parent: [parent]}\n",
        "  mapping: {cgroup_parent: {path: parent}}\n",
    );
    let source_id = SourceId::new(3241);
    let syntax = SyntaxDocument::parse(source_id, source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document")?;
    let parent = |name| {
        document
            .service(name)
            .and_then(compose_lens::model::Service::cgroup_parent)
    };
    for (service, expected) in [
        ("ordinary", "parent.slice"),
        ("empty", ""),
        ("whitespace", " parent "),
        ("expression", "${PARENT}"),
        ("independent", "unrelated"),
        ("duplicate", "first"),
    ] {
        let parent = parent(service).ok_or("cgroup parent")?;
        assert_eq!(parent.value(), expected);
        assert_eq!(parent.span().source_id(), source_id);
    }
    assert!(matches!(
        document
            .service("independent")
            .and_then(compose_lens::model::Service::cgroup)
            .map(compose_lens::model::CgroupNamespace::kind),
        Some(compose_lens::model::CgroupNamespaceKind::Private)
    ));
    assert!(parent("omitted").is_none());
    for service in ["null", "number", "sequence", "mapping"] {
        let service = document.service(service).ok_or("malformed service")?;
        assert!(service.cgroup_parent().is_none());
        assert!(
            service
                .unknown_fields()
                .iter()
                .any(|field| field.name().value() == "cgroup_parent")
        );
    }
    for code in [DUPLICATE_FIELD, EXPECTED_SCALAR] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_authored_cpu_count_categories_invalid_evidence_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  zero: {cpu_count: 0}\n",
        "  huge: {cpu_count: 999999999999999999999999999999999999}\n",
        "  binary: {cpu_count: 0b1_0}\n",
        "  octal: {cpu_count: 0o7_7}\n",
        "  hexadecimal: {cpu_count: 0xCA_FE}\n",
        "  negative-zero: {cpu_count: -0}\n",
        "  negative: {cpu_count: -1}\n",
        "  quoted: {cpu_count: \"007\"}\n",
        "  quoted-negative: {cpu_count: \"-1\"}\n",
        "  expression: {cpu_count: \"${CPU_COUNT}\"}\n",
        "  empty: {cpu_count: \"\"}\n",
        "  literal:\n    cpu_count: |-\n      101\n",
        "  folded:\n    cpu_count: >-\n      101\n",
        "  duplicate:\n    cpu_count: 1\n    cpu_count: 2\n",
        "  omitted: {image: example/app}\n",
        "  float: {cpu_count: 0.5}\n",
        "  boolean: {cpu_count: true}\n",
        "  null: {cpu_count: null}\n",
        "  timestamp: {cpu_count: !!timestamp 2024-01-01}\n",
        "  regex: {cpu_count: !!regex '007'}\n",
        "  mapping: {cpu_count: {value: 1}}\n",
        "  sequence: {cpu_count: [1]}\n",
    );
    let source_id = SourceId::new(3251);
    let syntax = SyntaxDocument::parse(source_id, source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document")?;
    let count = |name| {
        document
            .service(name)
            .and_then(compose_lens::model::Service::cpu_count)
            .map(Located::value)
    };
    for (service, expected) in [
        ("zero", "0"),
        ("huge", "999999999999999999999999999999999999"),
        ("binary", "0b1_0"),
        ("octal", "0o7_7"),
        ("hexadecimal", "0xCA_FE"),
        ("negative-zero", "-0"),
        ("duplicate", "1"),
    ] {
        assert!(
            matches!(count(service), Some(compose_lens::model::CpuCount::YamlInteger(value)) if value == expected),
            "{service}: {:?}",
            count(service)
        );
    }
    for service in ["literal", "folded"] {
        assert!(matches!(count(service), Some(compose_lens::model::CpuCount::String(_))));
    }
    for (service, expected) in [
        ("quoted", "007"),
        ("quoted-negative", "-1"),
        ("expression", "${CPU_COUNT}"),
        ("empty", ""),
    ] {
        assert!(matches!(count(service), Some(compose_lens::model::CpuCount::String(value)) if value == expected));
    }
    assert!(
        matches!(count("negative"), Some(compose_lens::model::CpuCount::NegativeYamlInteger(value)) if value == "-1")
    );
    assert!(
        document
            .service("zero")
            .and_then(compose_lens::model::Service::cpu_count)
            .is_some_and(|value| value.span().source_id() == source_id)
    );
    assert!(count("omitted").is_none());
    for service in ["float", "boolean", "null", "timestamp", "regex", "mapping", "sequence"] {
        let service = document.service(service).ok_or("malformed service")?;
        assert!(service.cpu_count().is_none());
        assert!(
            service
                .unknown_fields()
                .iter()
                .any(|field| field.name().value() == "cpu_count")
        );
    }
    for code in [
        compose_lens::model::CPU_COUNT_EXPECTED_VALUE,
        compose_lens::model::CPU_COUNT_NEGATIVE,
        DUPLICATE_FIELD,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_authored_cpu_percent_categories_range_evidence_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  zero: {cpu_percent: 0}\n",
        "  negative-zero: {cpu_percent: -0}\n",
        "  maximum: {cpu_percent: 0x6_4}\n",
        "  binary: {cpu_percent: 0b110_0100}\n",
        "  negative: {cpu_percent: -1}\n",
        "  over: {cpu_percent: 101}\n",
        "  hexadecimal-over: {cpu_percent: 0x65}\n",
        "  huge: {cpu_percent: 999999999999999999999999999999999999}\n",
        "  quoted-negative: {cpu_percent: \"-1\"}\n",
        "  quoted-over: {cpu_percent: \"101\"}\n",
        "  quoted-float: {cpu_percent: \"0.5\"}\n",
        "  expression: {cpu_percent: \"${CPU_PERCENT}\"}\n",
        "  empty: {cpu_percent: \"\"}\n",
        "  block:\n    cpu_percent: |-\n      101\n",
        "  folded:\n    cpu_percent: >-\n      101\n",
        "  duplicate:\n    cpu_percent: 1\n    cpu_percent: 2\n",
        "  float: {cpu_percent: 0.5}\n",
        "  boolean: {cpu_percent: true}\n",
        "  null: {cpu_percent: null}\n",
        "  timestamp: {cpu_percent: !!timestamp 2024-01-01}\n",
        "  regex: {cpu_percent: !!regex '101'}\n",
        "  tagged: {cpu_percent: !opaque 101}\n",
        "  mapping: {cpu_percent: {value: 1}}\n",
        "  sequence: {cpu_percent: [1]}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3261), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document")?;
    let percent = |name| {
        document
            .service(name)
            .and_then(compose_lens::model::Service::cpu_percent)
            .map(Located::value)
    };
    for (service, expected) in [
        ("zero", "0"),
        ("negative-zero", "-0"),
        ("maximum", "0x6_4"),
        ("binary", "0b110_0100"),
        ("duplicate", "1"),
    ] {
        assert!(
            matches!(percent(service), Some(compose_lens::model::CpuPercent::YamlInteger(value)) if value == expected),
            "{service}: {:?}",
            percent(service)
        );
    }
    for (service, expected) in [
        ("negative", "-1"),
        ("over", "101"),
        ("hexadecimal-over", "0x65"),
        ("huge", "999999999999999999999999999999999999"),
    ] {
        assert!(
            matches!(percent(service), Some(compose_lens::model::CpuPercent::OutOfRangeYamlInteger(value)) if value == expected)
        );
    }
    for (service, expected) in [
        ("quoted-negative", "-1"),
        ("quoted-over", "101"),
        ("quoted-float", "0.5"),
        ("expression", "${CPU_PERCENT}"),
        ("empty", ""),
    ] {
        assert!(matches!(percent(service), Some(compose_lens::model::CpuPercent::String(value)) if value == expected));
    }
    for service in ["block", "folded"] {
        assert!(matches!(
            percent(service),
            Some(compose_lens::model::CpuPercent::String(_))
        ));
    }
    for service in [
        "float",
        "boolean",
        "null",
        "timestamp",
        "regex",
        "tagged",
        "mapping",
        "sequence",
    ] {
        let service = document.service(service).ok_or("malformed service")?;
        assert!(service.cpu_percent().is_none());
        assert!(
            service
                .unknown_fields()
                .iter()
                .any(|field| field.name().value() == "cpu_percent")
        );
    }
    for code in [
        compose_lens::model::CPU_PERCENT_EXPECTED_VALUE,
        compose_lens::model::CPU_PERCENT_OUT_OF_RANGE,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_authored_cpu_period_number_and_string_categories() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  integer: {cpu_period: -0xF_F}\n  float: {cpu_period: +1.5}\n  exponent: {cpu_period: 1e+6}\n  quoted: {cpu_period: \"1e6\"}\n  plain: {cpu_period: opaque}\n  literal:\n    cpu_period: |- # comment\n      1000\n  folded:\n    cpu_period: >+ # comment\n      1000\n  duplicate:\n    cpu_period: 1\n    cpu_period: 2\n  boolean: {cpu_period: true}\n  null: {cpu_period: null}\n  tagged: {cpu_period: !opaque 1}\n  timestamp: {cpu_period: !!timestamp 2024-01-01}\n  regex: {cpu_period: !!regex '1'}\n  mapping: {cpu_period: {value: 1}}\n  sequence: {cpu_period: [1]}\n";
    let parsed = ComposeDocument::parse(SyntaxDocument::parse(SourceId::new(3269), source)?.document());
    let document = parsed.document().ok_or("typed document")?;
    let period = |name| {
        document
            .service(name)
            .and_then(compose_lens::model::Service::cpu_period)
            .map(Located::value)
    };
    for (service, expected) in [
        ("integer", "-0xF_F"),
        ("float", "+1.5"),
        ("exponent", "1e+6"),
        ("duplicate", "1"),
    ] {
        assert!(
            matches!(period(service), Some(compose_lens::model::CpuPeriod::YamlNumber(value)) if value == expected),
            "{service}: {:?}",
            period(service)
        );
    }
    for service in ["quoted", "plain", "literal", "folded"] {
        assert!(matches!(
            period(service),
            Some(compose_lens::model::CpuPeriod::String(_))
        ));
    }
    for service in ["boolean", "null", "tagged", "timestamp", "regex", "mapping", "sequence"] {
        let service = document.service(service).ok_or("malformed service")?;
        assert!(service.cpu_period().is_none());
        assert!(
            service
                .unknown_fields()
                .iter()
                .any(|field| field.name().value() == "cpu_period")
        );
    }
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == compose_lens::model::CPU_PERIOD_EXPECTED_VALUE)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    Ok(())
}

#[test]
fn retains_authored_cpu_quota_number_and_string_categories() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  integer: {cpu_quota: -0xF_F}\n  float: {cpu_quota: +1.5}\n  exponent: {cpu_quota: 1e+6}\n  quoted: {cpu_quota: \"1e6\"}\n  plain: {cpu_quota: opaque}\n  literal:\n    cpu_quota: |- # comment\n      1000\n  folded:\n    cpu_quota: >+ # comment\n      1000\n  duplicate:\n    cpu_quota: 1\n    cpu_quota: 2\n  boolean: {cpu_quota: true}\n  null: {cpu_quota: null}\n  tagged: {cpu_quota: !opaque 1}\n  timestamp: {cpu_quota: !!timestamp 2024-01-01}\n  regex: {cpu_quota: !!regex '1'}\n  mapping: {cpu_quota: {value: 1}}\n  sequence: {cpu_quota: [1]}\n";
    let parsed = ComposeDocument::parse(SyntaxDocument::parse(SourceId::new(3277), source)?.document());
    let document = parsed.document().ok_or("typed document")?;
    let quota = |name| {
        document
            .service(name)
            .and_then(compose_lens::model::Service::cpu_quota)
            .map(Located::value)
    };
    for (service, expected) in [
        ("integer", "-0xF_F"),
        ("float", "+1.5"),
        ("exponent", "1e+6"),
        ("duplicate", "1"),
    ] {
        assert!(
            matches!(quota(service), Some(compose_lens::model::CpuQuota::YamlNumber(value)) if value == expected),
            "{service}: {:?}",
            quota(service)
        );
    }
    for service in ["quoted", "plain", "literal", "folded"] {
        assert!(matches!(quota(service), Some(compose_lens::model::CpuQuota::String(_))));
    }
    for service in ["boolean", "null", "tagged", "timestamp", "regex", "mapping", "sequence"] {
        let service = document.service(service).ok_or("malformed service")?;
        assert!(service.cpu_quota().is_none());
        assert!(
            service
                .unknown_fields()
                .iter()
                .any(|field| field.name().value() == "cpu_quota")
        );
    }
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == compose_lens::model::CPU_QUOTA_EXPECTED_VALUE)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    Ok(())
}

#[test]
fn retains_authored_cpu_rt_period_categories_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  integer: {cpu_rt_period: -0xF_F}\n  float: {cpu_rt_period: +1.5}\n  exponent: {cpu_rt_period: 1e+6}\n  duration: {cpu_rt_period: 1m30s}\n  fraction: {cpu_rt_period: 1.5s}\n  quoted: {cpu_rt_period: \"250ms\"}\n  literal:\n    cpu_rt_period: |- # comment\n      1us\n  folded:\n    cpu_rt_period: >+ # comment\n      1h\n  expression: {cpu_rt_period: \"${CPU_RT_PERIOD}\"}\n  other: {cpu_rt_period: \"1000\"}\n  empty: {cpu_rt_period: \"\"}\n  invalid-unit: {cpu_rt_period: 1ns}\n  malformed-duration: {cpu_rt_period: 1.s}\n  duplicate:\n    cpu_rt_period: 1s\n    cpu_rt_period: 2s\n  boolean: {cpu_rt_period: true}\n  null: {cpu_rt_period: null}\n  tagged: {cpu_rt_period: !opaque 1s}\n  timestamp: {cpu_rt_period: !!timestamp 2024-01-01}\n  regex: {cpu_rt_period: !!regex '1s'}\n  mapping: {cpu_rt_period: {value: 1}}\n  sequence: {cpu_rt_period: [1]}\n";
    let parsed = ComposeDocument::parse(SyntaxDocument::parse(SourceId::new(3285), source)?.document());
    let document = parsed.document().ok_or("typed document")?;
    let period = |name| {
        document
            .service(name)
            .and_then(compose_lens::model::Service::cpu_rt_period)
            .map(Located::value)
    };
    for (service, expected) in [("integer", "-0xF_F"), ("float", "+1.5"), ("exponent", "1e+6")] {
        assert!(matches!(
            period(service),
            Some(compose_lens::model::CpuRtPeriod::YamlNumber(value)) if value == expected
        ));
    }
    for (service, expected) in [
        ("duration", "1m30s"),
        ("fraction", "1.5s"),
        ("quoted", "250ms"),
        ("literal", "1us"),
        ("folded", "1h\n"),
        ("duplicate", "1s"),
    ] {
        assert!(
            matches!(
                period(service),
                Some(compose_lens::model::CpuRtPeriod::Duration(value)) if value == expected
            ),
            "{service}: {:?}",
            period(service)
        );
    }
    assert!(matches!(
        period("expression"),
        Some(compose_lens::model::CpuRtPeriod::Expression(value)) if value == "${CPU_RT_PERIOD}"
    ));
    for service in ["other", "empty", "invalid-unit", "malformed-duration"] {
        assert!(matches!(
            period(service),
            Some(compose_lens::model::CpuRtPeriod::Other(_))
        ));
    }
    for service in ["boolean", "null", "tagged", "timestamp", "regex", "mapping", "sequence"] {
        let service = document.service(service).ok_or("malformed service")?;
        assert!(service.cpu_rt_period().is_none());
        assert!(
            service
                .unknown_fields()
                .iter()
                .any(|field| field.name().value() == "cpu_rt_period")
        );
    }
    for code in [
        compose_lens::model::CPU_RT_PERIOD_EXPECTED_VALUE,
        compose_lens::model::CPU_RT_PERIOD_INVALID,
        DUPLICATE_FIELD,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_authored_service_pull_refresh_after_strings_and_malformed_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let source = "services:\n  empty:\n    pull_refresh_after: \"\"\n  expression:\n    pull_refresh_after: \"${REFRESH_AFTER}\"\n  duplicate:\n    pull_refresh_after: first\n    pull_refresh_after: second\n  null:\n    pull_refresh_after: null\n  number:\n    pull_refresh_after: 1\n  boolean:\n    pull_refresh_after: true\n  sequence:\n    pull_refresh_after: [1h]\n  mapping:\n    pull_refresh_after: {duration: 1h}\n";
    let source_id = SourceId::new(3201);
    let syntax = SyntaxDocument::parse(source_id, source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document")?;
    assert_eq!(
        document
            .service("empty")
            .and_then(compose_lens::model::Service::pull_refresh_after)
            .map(Located::value)
            .map(String::as_str),
        Some("")
    );
    let expression = document
        .service("expression")
        .and_then(compose_lens::model::Service::pull_refresh_after)
        .ok_or("expression pull refresh interval")?;
    assert_eq!(expression.value(), "${REFRESH_AFTER}");
    assert_eq!(expression.span().source_id(), source_id);
    assert_eq!(
        document
            .service("duplicate")
            .and_then(compose_lens::model::Service::pull_refresh_after)
            .map(Located::value)
            .map(String::as_str),
        Some("first")
    );
    for service in ["null", "number", "boolean", "sequence", "mapping"] {
        let service = document.service(service).ok_or("service")?;
        assert!(service.pull_refresh_after().is_none());
        assert!(
            service
                .unknown_fields()
                .iter()
                .any(|field| field.name().value() == "pull_refresh_after")
        );
    }
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    Ok(())
}

#[test]
fn retains_authored_service_platform_strings_and_malformed_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  normal:\n    platform: linux/amd64\n  empty:\n    platform: \"\"\n  expression:\n    platform: \"${PLATFORM}\"\n  duplicate:\n    platform: first\n    platform: second\n  null:\n    platform: null\n  number:\n    platform: 1\n  boolean:\n    platform: true\n  sequence:\n    platform: [linux, amd64]\n  mapping:\n    platform: {os: linux}\n";
    let source_id = SourceId::new(3208);
    let syntax = SyntaxDocument::parse(source_id, source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document")?;
    assert_eq!(
        document
            .service("normal")
            .and_then(compose_lens::model::Service::platform)
            .map(Located::value)
            .map(String::as_str),
        Some("linux/amd64")
    );
    assert_eq!(
        document
            .service("empty")
            .and_then(compose_lens::model::Service::platform)
            .map(Located::value)
            .map(String::as_str),
        Some("")
    );
    let expression = document
        .service("expression")
        .and_then(compose_lens::model::Service::platform)
        .ok_or("expression platform")?;
    assert_eq!(expression.value(), "${PLATFORM}");
    assert_eq!(expression.span().source_id(), source_id);
    assert_eq!(
        document
            .service("duplicate")
            .and_then(compose_lens::model::Service::platform)
            .map(Located::value)
            .map(String::as_str),
        Some("first")
    );
    for service in ["null", "number", "boolean", "sequence", "mapping"] {
        let service = document.service(service).ok_or("service")?;
        assert!(service.platform().is_none());
        assert!(
            service
                .unknown_fields()
                .iter()
                .any(|field| field.name().value() == "platform")
        );
    }
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    Ok(())
}

#[test]
fn retains_authored_extends_forms_spans_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  long:\n    extends:\n      service: ' web '\n      file: './base.yml'\n      x-note: retained\n      future: retained\n  short:\n    extends: \"${PARENT}\"\n  empty:\n    extends: {service: \"\"}\n  missing:\n    extends: {}\n  malformed:\n    extends:\n      service: app\n      service: duplicate\n      file: !!timestamp 2024-01-01\n      future: retained\n  outer:\n    extends: [bad]\n";
    let syntax = SyntaxDocument::parse(SourceId::new(3162), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let extends = |name| {
        parsed
            .document()
            .and_then(|document| document.service(name))
            .and_then(compose_lens::model::Service::extends)
    };
    let compose_lens::model::Extends::Long(long) = extends("long").ok_or("long extends")? else {
        return Err("long form expected".into());
    };
    assert_eq!(long.service().map(Located::value).map(String::as_str), Some(" web "));
    assert_eq!(long.file().map(Located::value).map(String::as_str), Some("./base.yml"));
    assert_eq!(
        &source[long.span().range()],
        "service: ' web '\n      file: './base.yml'\n      x-note: retained\n      future: retained\n"
    );
    assert_eq!(long.extension_fields().len(), 1);
    assert_eq!(long.unknown_fields().len(), 1);
    assert!(
        matches!(extends("short"), Some(compose_lens::model::Extends::Short(value)) if value.value() == "${PARENT}")
    );
    assert!(
        matches!(extends("empty"), Some(compose_lens::model::Extends::Long(value)) if value.service().is_some_and(|service| service.value().is_empty()))
    );
    assert!(matches!(extends("missing"), Some(compose_lens::model::Extends::Long(value)) if value.service().is_none()));
    let compose_lens::model::Extends::Long(malformed) = extends("malformed").ok_or("malformed extends")? else {
        return Err("malformed long form expected".into());
    };
    assert_eq!(malformed.service().map(Located::value).map(String::as_str), Some("app"));
    assert_eq!(malformed.unknown_fields().len(), 2);
    assert!(extends("outer").is_none());
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_FIELD_FORM)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXTENDS_MISSING_SERVICE)
    );
    Ok(())
}

#[test]
fn rejects_float_restart_attempts_and_retains_non_scalar_unmodeled_members() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(2905),
        concat!(
            "services:\n",
            "  app:\n",
            "    deploy:\n",
            "      restart_policy:\n",
            "        condition: any\n",
            "        max_attempts: 1.5\n",
            "        x-map: {retained: value}\n",
            "        future: [retained]\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let policy = parsed
        .document()
        .and_then(|doc| doc.service("app"))
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.restart_policy())
        .ok_or("restart policy expected")?;
    assert!(matches!(
        policy.condition().map(Located::value),
        Some(DeployRestartCondition::Any)
    ));
    assert!(policy.max_attempts().is_none());
    assert_eq!(policy.extension_fields().len(), 1);
    assert_eq!(policy.unknown_fields().len(), 1);
    assert_eq!(parsed.diagnostics().len(), 1);
    assert_eq!(parsed.diagnostics()[0].code(), EXPECTED_SCALAR);
    Ok(())
}

#[test]
fn retains_authored_deploy_placement_forms_and_recovers_malformed_members() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(3001),
        concat!(
            "services:\n",
            "  valid:\n",
            "    deploy:\n",
            "      placement:\n",
            "        constraints: [\"node.labels.zone == east\", \"\", \"node.labels.zone == east\"]\n",
            "        preferences:\n",
            "          - spread: node.labels.rack\n",
            "            x-retained: {nested: value}\n",
            "            future: [value]\n",
            "          - {}\n",
            "        max_replicas_per_node: 003\n",
            "        x-placement: [retained]\n",
            "        later: {retained: true}\n",
            "  string:\n    deploy: {placement: {max_replicas_per_node: \"003\"}}\n",
            "  malformed:\n",
            "    deploy:\n",
            "      placement:\n",
            "        constraints: [valid, 1, {bad: value}, later]\n",
            "        preferences: [{spread: 1, x-retained: []}, [], {future: value}]\n",
            "        max_replicas_per_node: 1.5\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let placement = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.placement())
            .ok_or("deploy placement expected")
    };
    let valid = placement("valid")?;
    assert_eq!(
        valid
            .constraints()
            .ok_or("constraints expected")?
            .iter()
            .map(Located::value)
            .collect::<Vec<_>>(),
        ["node.labels.zone == east", "", "node.labels.zone == east"]
    );
    let preferences = valid.preferences().ok_or("preferences expected")?;
    assert_eq!(preferences.len(), 2);
    assert_eq!(
        preferences[0].spread().map(Located::value).map(String::as_str),
        Some("node.labels.rack")
    );
    assert_eq!(preferences[0].extension_fields().len(), 1);
    assert_eq!(preferences[0].unknown_fields().len(), 1);
    assert!(preferences[1].spread().is_none());
    assert!(matches!(
        valid.max_replicas_per_node().map(Located::value),
        Some(DeployPlacementMaxReplicasPerNode::YamlInteger(value)) if value == "003"
    ));
    assert_eq!(valid.extension_fields().len(), 1);
    assert_eq!(valid.unknown_fields().len(), 1);
    assert!(matches!(
        placement("string")?.max_replicas_per_node().map(Located::value),
        Some(DeployPlacementMaxReplicasPerNode::String(value)) if value == "003"
    ));

    let malformed = placement("malformed")?;
    assert_eq!(
        malformed
            .constraints()
            .ok_or("partially recovered constraints expected")?
            .iter()
            .map(Located::value)
            .collect::<Vec<_>>(),
        ["valid", "later"]
    );
    let preferences = malformed
        .preferences()
        .ok_or("partially recovered preferences expected")?;
    assert_eq!(preferences.len(), 2);
    assert!(malformed.max_replicas_per_node().is_none());
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_MAPPING)
    );
    Ok(())
}

#[test]
fn retains_authored_deploy_resource_limit_pids_categories_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(3101),
        concat!(
            "services:\n",
            "  integer:\n    deploy:\n      resources:\n        limits:\n          pids: 003\n          x-limit: kept\n          later: kept\n        x-resources: kept\n        future: kept\n",
            "  string:\n    deploy: {resources: {limits: {pids: \"003\"}}}\n",
            "  deferred:\n    deploy: {resources: {limits: {pids: \"${PIDS}\"}}}\n",
            "  empty:\n    deploy: {resources: {limits: {pids: \"\"}}}\n",
            "  float:\n    deploy: {resources: {limits: {pids: 1.5}}}\n",
            "  boolean:\n    deploy: {resources: {limits: {pids: true}}}\n",
            "  null:\n    deploy: {resources: {limits: {pids: null}}}\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let pids = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.limits())
            .and_then(|limits| limits.pids())
            .map(Located::value)
    };
    assert!(matches!(pids("integer"), Some(DeployResourcePids::YamlInteger(value)) if value == "003"));
    for (service, expected) in [("string", "003"), ("deferred", "${PIDS}"), ("empty", "")] {
        assert!(matches!(pids(service), Some(DeployResourcePids::String(value)) if value == expected));
    }
    let resources = document
        .service("integer")
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .ok_or("resources expected")?;
    assert_eq!(resources.extension_fields().len(), 1);
    assert_eq!(resources.unknown_fields().len(), 1);
    let limits = resources.limits().ok_or("limits expected")?;
    assert_eq!(limits.extension_fields().len(), 1);
    assert_eq!(limits.unknown_fields().len(), 1);
    for service in ["float", "boolean", "null"] {
        assert!(pids(service).is_none());
    }
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    Ok(())
}

#[test]
fn retains_authored_deploy_resource_limit_cpu_categories_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(3105),
        concat!(
            "services:\n",
            "  integer:\n    deploy:\n      resources:\n        limits:\n          cpus: 2\n          x-limit: kept\n          future: kept\n        x-resources: kept\n        later: kept\n",
            "  float:\n    deploy: {resources: {limits: {cpus: 0.50}}}\n",
            "  exponent:\n    deploy: {resources: {limits: {cpus: 1e-3}}}\n",
            "  string:\n    deploy: {resources: {limits: {cpus: \"0.500\"}}}\n",
            "  deferred:\n    deploy: {resources: {limits: {cpus: \"${CPUS}\"}}}\n",
            "  boolean:\n    deploy: {resources: {limits: {cpus: true}}}\n",
            "  null:\n    deploy: {resources: {limits: {cpus: null}}}\n",
            "  mapping:\n    deploy: {resources: {limits: {cpus: {bad: value}}}}\n",
            "  sequence:\n    deploy: {resources: {limits: {cpus: [1]}}}\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let cpus = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.limits())
            .and_then(|limits| limits.cpus())
            .map(Located::value)
    };
    for (service, expected) in [("integer", "2"), ("float", "0.50"), ("exponent", "1e-3")] {
        assert!(matches!(cpus(service), Some(DeployResourceCpus::YamlNumber(value)) if value == expected));
    }
    for (service, expected) in [("string", "0.500"), ("deferred", "${CPUS}")] {
        assert!(matches!(cpus(service), Some(DeployResourceCpus::String(value)) if value == expected));
    }
    let resources = document
        .service("integer")
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .ok_or("resources expected")?;
    assert_eq!(resources.extension_fields().len(), 1);
    assert_eq!(resources.unknown_fields().len(), 1);
    let limits = resources.limits().ok_or("limits expected")?;
    assert_eq!(limits.extension_fields().len(), 1);
    assert_eq!(limits.unknown_fields().len(), 1);
    for service in ["boolean", "null", "mapping", "sequence"] {
        assert!(cpus(service).is_none());
    }
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    Ok(())
}

#[test]
fn retains_authored_deploy_resource_limit_memory_categories_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  megabytes:\n    deploy:\n      resources:\n        limits:\n          memory: \"50m\"\n          x-limit: kept\n          future: kept\n        x-resources: kept\n        later: kept\n",
        "  bytes:\n    deploy: {resources: {limits: {memory: \"1b\"}}}\n",
        "  kilo-short:\n    deploy: {resources: {limits: {memory: \"2k\"}}}\n",
        "  kilobytes:\n    deploy: {resources: {limits: {memory: \"001kb\"}}}\n",
        "  giga-short:\n    deploy: {resources: {limits: {memory: \"3g\"}}}\n",
        "  giga-long:\n    deploy: {resources: {limits: {memory: \"4gb\"}}}\n",
        "  zero:\n    deploy: {resources: {limits: {memory: \"000mb\"}}}\n",
        "  deferred:\n    deploy: {resources: {limits: {memory: \"${MEMORY}\"}}}\n",
        "  uppercase:\n    deploy: {resources: {limits: {memory: \"64MB\"}}}\n",
        "  bare:\n    deploy: {resources: {limits: {memory: \"64\"}}}\n",
        "  number:\n    deploy: {resources: {limits: {memory: 64}}}\n",
        "  boolean:\n    deploy: {resources: {limits: {memory: true}}}\n",
        "  null:\n    deploy: {resources: {limits: {memory: null}}}\n",
        "  mapping:\n    deploy: {resources: {limits: {memory: {bad: value}}}}\n",
        "  sequence:\n    deploy: {resources: {limits: {memory: [1]}}}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3106), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let memory = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.limits())
            .and_then(|limits| limits.memory())
    };
    let documented = memory("megabytes").ok_or("documented memory expected")?;
    assert_eq!(&source[documented.span().range()], "\"50m\"");
    let documented = documented.value();
    assert_eq!(documented.raw(), "50m");
    assert!(matches!(
        documented.kind(),
        DeployResourceMemoryKind::Documented { amount_raw, unit: DeployResourceMemoryUnit::M } if amount_raw == "50"
    ));
    assert!(matches!(
        memory("kilobytes").map(Located::value).map(compose_lens::model::DeployResourceMemory::kind),
        Some(DeployResourceMemoryKind::Documented { amount_raw, unit: DeployResourceMemoryUnit::Kb }) if amount_raw == "001"
    ));
    for (service, amount_raw, unit) in [
        ("bytes", "1", DeployResourceMemoryUnit::B),
        ("kilo-short", "2", DeployResourceMemoryUnit::K),
        ("giga-short", "3", DeployResourceMemoryUnit::G),
        ("giga-long", "4", DeployResourceMemoryUnit::Gb),
    ] {
        assert!(matches!(
            memory(service).map(Located::value).map(compose_lens::model::DeployResourceMemory::kind),
            Some(DeployResourceMemoryKind::Documented { amount_raw: actual_amount, unit: actual_unit })
                if actual_amount == amount_raw && *actual_unit == unit
        ));
    }
    assert!(matches!(
        memory("zero").map(Located::value).map(compose_lens::model::DeployResourceMemory::kind),
        Some(DeployResourceMemoryKind::Zero { amount_raw, unit: Some(DeployResourceMemoryUnit::Mb) }) if amount_raw == "000"
    ));
    assert!(matches!(
        memory("deferred")
            .map(Located::value)
            .map(compose_lens::model::DeployResourceMemory::kind),
        Some(DeployResourceMemoryKind::Expression)
    ));
    for service in ["uppercase", "bare"] {
        assert!(matches!(
            memory(service)
                .map(Located::value)
                .map(compose_lens::model::DeployResourceMemory::kind),
            Some(DeployResourceMemoryKind::ProviderDependentString)
        ));
    }
    let limits = document
        .service("megabytes")
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .and_then(|resources| resources.limits())
        .ok_or("limits expected")?;
    assert_eq!(limits.extension_fields().len(), 1);
    assert_eq!(limits.unknown_fields().len(), 1);
    for service in ["number", "boolean", "null", "mapping", "sequence"] {
        assert!(memory(service).is_none());
    }
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    Ok(())
}

#[test]
fn retains_authored_deploy_resource_reservation_cpu_categories_and_recovery() -> Result<(), Box<dyn std::error::Error>>
{
    let syntax = SyntaxDocument::parse(
        SourceId::new(3110),
        concat!(
            "services:\n",
            "  integer:\n    deploy:\n      resources:\n        reservations:\n          cpus: 2\n          x-reservation: kept\n          future: kept\n        x-resources: kept\n        later: kept\n",
            "  decimal:\n    deploy: {resources: {reservations: {cpus: 0.50}}}\n",
            "  exponent:\n    deploy: {resources: {reservations: {cpus: 1e-3}}}\n",
            "  string:\n    deploy: {resources: {reservations: {cpus: \"0.500\"}}}\n",
            "  deferred:\n    deploy: {resources: {reservations: {cpus: \"${CPUS}\"}}}\n",
            "  boolean:\n    deploy: {resources: {reservations: {cpus: true}}}\n",
            "  null:\n    deploy: {resources: {reservations: {cpus: null}}}\n",
            "  mapping:\n    deploy: {resources: {reservations: {cpus: {bad: value}}}}\n",
            "  sequence:\n    deploy: {resources: {reservations: {cpus: [1]}}}\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let cpus = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.reservations())
            .and_then(|reservations| reservations.cpus())
            .map(Located::value)
    };
    for (service, expected) in [("integer", "2"), ("decimal", "0.50"), ("exponent", "1e-3")] {
        assert!(matches!(cpus(service), Some(DeployResourceCpus::YamlNumber(value)) if value == expected));
    }
    for (service, expected) in [("string", "0.500"), ("deferred", "${CPUS}")] {
        assert!(matches!(cpus(service), Some(DeployResourceCpus::String(value)) if value == expected));
    }
    let resources = document
        .service("integer")
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .ok_or("resources expected")?;
    assert_eq!(resources.extension_fields().len(), 1);
    assert_eq!(resources.unknown_fields().len(), 1);
    let reservations = resources.reservations().ok_or("reservations expected")?;
    assert_eq!(reservations.extension_fields().len(), 1);
    assert_eq!(reservations.unknown_fields().len(), 1);
    for service in ["boolean", "null", "mapping", "sequence"] {
        assert!(cpus(service).is_none());
    }
    assert!(parsed.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == EXPECTED_SCALAR
            && diagnostic.message() == "deploy resource reservations cpus must be a YAML number or string scalar"
    }));
    assert!(!parsed.diagnostics().iter().any(|diagnostic| {
        diagnostic.message() == "deploy resource limits cpus must be a YAML number or string scalar"
    }));
    Ok(())
}

#[test]
fn retains_authored_deploy_resource_reservation_memory_categories_and_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  primary:\n    mem_limit: \"100m\"\n    deploy:\n      resources:\n        limits: {memory: \"99m\"}\n        reservations:\n          memory: \"50m\"\n          x-reservation: kept\n          future: kept\n        x-resources: kept\n        later: kept\n",
        "  bytes:\n    deploy: {resources: {reservations: {memory: \"1b\"}}}\n",
        "  kilo-short:\n    deploy: {resources: {reservations: {memory: \"2k\"}}}\n",
        "  kilobytes:\n    deploy: {resources: {reservations: {memory: \"001kb\"}}}\n",
        "  giga-short:\n    deploy: {resources: {reservations: {memory: \"3g\"}}}\n",
        "  giga-long:\n    deploy: {resources: {reservations: {memory: \"4gb\"}}}\n",
        "  zero:\n    deploy: {resources: {reservations: {memory: \"000mb\"}}}\n",
        "  bare-zero:\n    deploy: {resources: {reservations: {memory: \"000\"}}}\n",
        "  deferred:\n    deploy: {resources: {reservations: {memory: \"${MEMORY}\"}}}\n",
        "  uppercase:\n    deploy: {resources: {reservations: {memory: \"64MB\"}}}\n",
        "  bare:\n    deploy: {resources: {reservations: {memory: \"64\"}}}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3112), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let memory = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.reservations())
            .and_then(|reservations| reservations.memory())
    };
    let primary = memory("primary").ok_or("reservation memory expected")?;
    assert_eq!(&source[primary.span().range()], "\"50m\"");
    assert_eq!(primary.value().raw(), "50m");
    assert!(matches!(
        primary.value().kind(),
        DeployResourceMemoryKind::Documented { amount_raw, unit: DeployResourceMemoryUnit::M } if amount_raw == "50"
    ));
    for (service, amount_raw, unit) in [
        ("bytes", "1", DeployResourceMemoryUnit::B),
        ("kilo-short", "2", DeployResourceMemoryUnit::K),
        ("kilobytes", "001", DeployResourceMemoryUnit::Kb),
        ("giga-short", "3", DeployResourceMemoryUnit::G),
        ("giga-long", "4", DeployResourceMemoryUnit::Gb),
    ] {
        assert!(matches!(
            memory(service).map(Located::value).map(compose_lens::model::DeployResourceMemory::kind),
            Some(DeployResourceMemoryKind::Documented { amount_raw: actual_amount, unit: actual_unit })
                if actual_amount == amount_raw && *actual_unit == unit
        ));
    }
    assert!(matches!(
        memory("zero").map(Located::value).map(compose_lens::model::DeployResourceMemory::kind),
        Some(DeployResourceMemoryKind::Zero { amount_raw, unit: Some(DeployResourceMemoryUnit::Mb) }) if amount_raw == "000"
    ));
    assert!(matches!(
        memory("bare-zero").map(Located::value).map(compose_lens::model::DeployResourceMemory::kind),
        Some(DeployResourceMemoryKind::Zero { amount_raw, unit: None }) if amount_raw == "000"
    ));
    assert!(matches!(
        memory("deferred")
            .map(Located::value)
            .map(compose_lens::model::DeployResourceMemory::kind),
        Some(DeployResourceMemoryKind::Expression)
    ));
    for service in ["uppercase", "bare"] {
        assert!(matches!(
            memory(service)
                .map(Located::value)
                .map(compose_lens::model::DeployResourceMemory::kind),
            Some(DeployResourceMemoryKind::ProviderDependentString)
        ));
    }
    let primary_resources = document
        .service("primary")
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .ok_or("resources expected")?;
    assert_eq!(primary_resources.extension_fields().len(), 1);
    assert_eq!(primary_resources.unknown_fields().len(), 1);
    let reservations = primary_resources.reservations().ok_or("reservations expected")?;
    assert_eq!(reservations.extension_fields().len(), 1);
    assert_eq!(reservations.unknown_fields().len(), 1);
    assert_eq!(
        primary_resources
            .limits()
            .and_then(|limits| limits.memory())
            .map(Located::value)
            .map(compose_lens::model::DeployResourceMemory::raw),
        Some("99m")
    );
    Ok(())
}

#[test]
fn recovers_authored_deploy_resource_reservation_memory_malformed_and_duplicate_values()
-> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(3113),
        concat!(
            "services:\n",
            "  number:\n    deploy: {resources: {reservations: {memory: 64}}}\n",
            "  boolean:\n    deploy: {resources: {reservations: {memory: true}}}\n",
            "  null:\n    deploy: {resources: {reservations: {memory: null}}}\n",
            "  mapping:\n    deploy: {resources: {reservations: {memory: {bad: value}}}}\n",
            "  sequence:\n    deploy: {resources: {reservations: {memory: [1]}}}\n",
            "  duplicate:\n    deploy:\n      resources:\n        reservations:\n          memory: \"2m\"\n          memory: \"3m\"\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let memory = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.reservations())
            .and_then(|reservations| reservations.memory())
    };
    for service in ["number", "boolean", "null", "mapping", "sequence"] {
        assert!(memory(service).is_none());
    }
    assert_eq!(
        memory("duplicate")
            .map(Located::value)
            .map(compose_lens::model::DeployResourceMemory::raw),
        Some("2m")
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    assert!(parsed.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == EXPECTED_SCALAR
            && diagnostic.message() == "deploy resource reservations memory must be a YAML string scalar"
    }));
    assert!(
        !parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.message() == "deploy resource limits memory must be a YAML string scalar" })
    );
    Ok(())
}

#[test]
fn retains_authored_reservation_generic_resources_with_schema_only_members() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n  app:\n    deploy:\n      resources:\n        reservations:\n          cpus: 2\n          memory: \"10m\"\n          generic_resources:\n            - discrete_resource_spec: {kind: gpu, value: 001}\n              x-item: kept\n            - discrete_resource_spec: {kind: \"\", value: \"${COUNT}\", later: kept}\n            - {}\n          x-reservation: kept\n",
        "  empty:\n    deploy: {resources: {reservations: {generic_resources: []}}}\n",
        "  invalid:\n    deploy: {resources: {reservations: {generic_resources: [{discrete_resource_spec: {kind: 1, value: false}}, scalar]}}}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3114), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let reservations = document
        .service("app")
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .and_then(|resources| resources.reservations())
        .ok_or("reservations")?;
    let resources = reservations.generic_resources().ok_or("generic resources")?;
    assert_eq!(resources.items().len(), 3);
    assert!(source[resources.span().range()].starts_with("- discrete_resource_spec: {kind: gpu, value: 001}"));
    let first = resources
        .items()
        .first()
        .and_then(|item| item.discrete_resource_spec())
        .ok_or("first spec")?;
    assert!(matches!(first.kind().map(Located::value), Some(value) if value == "gpu"));
    assert_eq!(reservations.extension_fields().len(), 1);
    assert!(
        document
            .service("empty")
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|d| d.resources())
            .and_then(|r| r.reservations())
            .and_then(|r| r.generic_resources())
            .is_some_and(|items| items.items().is_empty())
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_MAPPING)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    Ok(())
}

#[test]
fn retains_authored_reservation_generic_resource_block_scalar_categories() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n  app:\n    deploy:\n      resources:\n        reservations:\n          generic_resources:\n",
        "            - discrete_resource_spec:\n                kind: gpu\n                value: 001\n",
        "            - discrete_resource_spec:\n                value: 1e-3\n",
        "            - discrete_resource_spec:\n                value: \"exact\"\n",
        "            - discrete_resource_spec:\n                value: \"${COUNT}\"\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3116), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let items = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .and_then(|resources| resources.reservations())
        .and_then(|reservations| reservations.generic_resources())
        .map(compose_lens::model::DeployGenericResources::items)
        .ok_or("generic resources")?;
    assert_eq!(items.len(), 4);
    let value = |index: usize| {
        items
            .get(index)
            .and_then(|item| item.discrete_resource_spec())
            .and_then(|spec| spec.value())
            .map(Located::value)
    };
    assert!(
        matches!(value(0), Some(compose_lens::model::DeployDiscreteResourceValue::YamlNumber(raw)) if raw == "001")
    );
    assert!(
        matches!(value(1), Some(compose_lens::model::DeployDiscreteResourceValue::YamlNumber(raw)) if raw == "1e-3")
    );
    assert!(matches!(value(2), Some(compose_lens::model::DeployDiscreteResourceValue::String(raw)) if raw == "exact"));
    assert!(
        matches!(value(3), Some(compose_lens::model::DeployDiscreteResourceValue::String(raw)) if raw == "${COUNT}")
    );
    assert!(source[items[0].span().range()].contains("value: 001"));
    Ok(())
}

#[test]
fn retains_authored_malformed_reservation_generic_resource_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n  app:\n    deploy:\n      resources:\n        reservations:\n          generic_resources:\n",
        "            - discrete_resource_spec: {kind: gpu, value: 1}\n",
        "            - malformed-item\n",
        "            - discrete_resource_spec: malformed-spec\n",
        "            - discrete_resource_spec:\n                kind: {broken: mapping}\n                value: 1\n",
        "            - discrete_resource_spec:\n                kind: fpga\n                value: {broken: mapping}\n",
        "            - discrete_resource_spec: {kind: tpu, value: \"ready\"}\n",
        "  outer:\n    deploy: {resources: {reservations: {generic_resources: malformed-collection}}}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3117), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let reservations = document
        .service("app")
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .and_then(|resources| resources.reservations())
        .ok_or("reservations")?;
    let items = reservations
        .generic_resources()
        .map(compose_lens::model::DeployGenericResources::items)
        .ok_or("generic resources")?;
    assert_eq!(items.len(), 6);
    assert!(matches!(
        items
            .iter()
            .map(compose_lens::model::DeployGenericResource::form)
            .collect::<Vec<_>>()
            .as_slice(),
        [
            compose_lens::model::DeployGenericResourceForm::Mapping,
            compose_lens::model::DeployGenericResourceForm::Unmodeled,
            compose_lens::model::DeployGenericResourceForm::Mapping,
            compose_lens::model::DeployGenericResourceForm::Mapping,
            compose_lens::model::DeployGenericResourceForm::Mapping,
            compose_lens::model::DeployGenericResourceForm::Mapping,
        ]
    ));
    assert_eq!(&source[items[1].span().range()], "malformed-item");
    assert!(items[2].discrete_resource_spec().is_none() && items[2].unknown_fields().len() == 1);
    let invalid_kind = items[3]
        .discrete_resource_spec()
        .map(compose_lens::model::DeployDiscreteResourceSpec::unknown_fields)
        .ok_or("partial discrete specification")?;
    assert_eq!(invalid_kind.len(), 1);
    let invalid_value = items[4]
        .discrete_resource_spec()
        .map(compose_lens::model::DeployDiscreteResourceSpec::unknown_fields)
        .ok_or("partial discrete specification")?;
    assert_eq!(invalid_value.len(), 1);
    assert!(matches!(
        items[5]
            .discrete_resource_spec()
            .and_then(|spec| spec.kind())
            .map(Located::value),
        Some(kind) if kind == "tpu"
    ));
    assert!(parsed.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == EXPECTED_MAPPING
            && diagnostic.message() == "deploy resource generic-resource entries must be mappings"
            && diagnostic
                .labels()
                .iter()
                .any(|label| &source[label.span().range()] == "malformed-item")
    }));
    let outer = document
        .service("outer")
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .and_then(|resources| resources.reservations())
        .ok_or("outer reservations")?;
    assert!(outer.generic_resources().is_none());
    assert!(
        outer
            .unknown_fields()
            .iter()
            .any(|field| field.name().value() == "generic_resources")
    );
    Ok(())
}

#[test]
fn retains_authored_reservation_device_capabilities_and_nested_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n  app:\n    deploy:\n      resources:\n        reservations:\n          devices:\n",
        "            - capabilities: [gpu, nvidia.com/gpu, GPU, \"\", \"${CAP}\", gpu]\n",
        "              driver: nvidia\n              count: 2\n              device_ids: [first]\n              options: {mode: shared}\n              x-device: kept\n              future: kept\n",
        "            - capabilities: []\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3118), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let devices = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::deploy)
        .and_then(|deploy| deploy.resources())
        .and_then(|resources| resources.reservations())
        .and_then(|reservations| reservations.devices())
        .ok_or("reservation devices")?;
    assert_eq!(devices.items().len(), 2);
    assert!(source[devices.span().range()].starts_with("- capabilities"));
    let first = &devices.items()[0];
    assert!(matches!(
        first.form(),
        compose_lens::model::DeployReservationDeviceForm::Mapping
    ));
    assert_eq!(first.extension_fields().len(), 1);
    assert_eq!(first.unknown_fields().len(), 1);
    let driver = first.driver().ok_or("driver")?;
    assert_eq!(driver.value(), "nvidia");
    assert_eq!(&source[driver.span().range()], "nvidia");
    assert!(matches!(
        first.count().map(Located::value),
        Some(compose_lens::model::DeployReservationDeviceCount::YamlInteger(value)) if value == "2"
    ));
    assert!(matches!(
        first.device_ids().map(compose_lens::model::DeployReservationDeviceIds::items),
        Some([id]) if matches!(id.value().map(Located::value), Some(value) if value == "first")
    ));
    assert!(matches!(
        first.options().and_then(compose_lens::model::DeployReservationDeviceOptions::as_map),
        Some([entry]) if entry.key().value() == "mode"
    ));
    let capabilities = first.capabilities().ok_or("capabilities")?;
    assert_eq!(capabilities.items().len(), 6);
    assert_eq!(&source[capabilities.items()[0].span().range()], "gpu");
    assert_eq!(
        capabilities
            .items()
            .iter()
            .filter_map(|item| item.value().map(Located::value))
            .collect::<Vec<_>>(),
        ["gpu", "nvidia.com/gpu", "GPU", "", "${CAP}", "gpu"]
    );
    assert!(capabilities.items().iter().all(|item| matches!(
        item.form(),
        compose_lens::model::DeployReservationDeviceCapabilityForm::String
    )));
    assert!(
        devices.items()[1]
            .capabilities()
            .is_some_and(|capabilities| capabilities.items().is_empty())
    );
    assert!(parsed.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == compose_lens::model::DEPLOY_RESERVATION_DEVICE_CAPABILITY_DUPLICATE_ITEM
            && diagnostic
                .labels()
                .iter()
                .any(|label| &source[label.span().range()] == "gpu")
    }));
    Ok(())
}

#[test]
fn retains_authored_reservation_device_options_forms_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n  map:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              options: {name: text, number: 2, enabled: true, empty: null, bad: {nested: value}, \"\": blank, true: invalid}\n",
        "  list:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              options: [KEY=VALUE, \" spaced \", \"\", KEY=VALUE, true, !!timestamp 2023-12-25, {bad: value}]\n",
        "  complex:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              options: {? [complex, key]: {nested: value}}\n",
        "  outer:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              options: invalid\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3140), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let option = |service| {
        parsed
            .document()
            .and_then(|doc| doc.service(service))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.reservations())
            .and_then(|reservations| reservations.devices())
            .and_then(|devices| devices.items().first())
            .and_then(compose_lens::model::DeployReservationDevice::options)
    };
    let map = option("map").ok_or("map")?;
    assert!(matches!(map.as_map(), Some(entries) if entries.len() == 4));
    assert!(matches!(
        map.as_map().map(|entries| entries[2].value().value()),
        Some(ComposeScalar::Boolean(true))
    ));
    assert_eq!(
        map.unmodeled_entries()
            .map(<[compose_lens::model::FieldReference]>::len),
        Some(3)
    );
    let complex = option("complex").ok_or("complex")?;
    let complex_reference = complex
        .unmodeled_entries()
        .and_then(|entries| entries.first())
        .ok_or("complex key evidence")?;
    assert_eq!(complex_reference.name().value(), "<unmodeled-key>");
    assert!(source[complex_reference.span().range()].contains("[complex, key]"));
    let value_span = complex_reference.value_span().ok_or("complex value span")?;
    assert!(source[complex_reference.span().range()].contains("{nested: value}"));
    assert_eq!(&source[value_span.range()], "{nested: value}");
    let list = option("list").ok_or("list")?.as_list().ok_or("list form")?;
    assert_eq!(list.len(), 7);
    assert!(matches!(list[0].value().map(Located::value), Some(value) if value == "KEY=VALUE"));
    assert!(matches!(
        list[4].form(),
        compose_lens::model::DeployReservationDeviceOptionItemForm::Unmodeled
    ));
    assert!(option("outer").is_none());
    assert!(
        parsed.diagnostics().iter().any(
            |diagnostic| diagnostic.code() == compose_lens::model::DEPLOY_RESERVATION_DEVICE_OPTIONS_DUPLICATE_ITEM
        )
    );
    Ok(())
}

#[test]
fn retains_authored_reservation_device_allocation_selectors_and_conflicts() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n  valid:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              count: 2\n              device_ids: [first, first, \"${ID}\", true, !!timestamp 2023-12-25, !!regex 'gpu.*', {bad: value}]\n",
        "  strings:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              count: \"all\"\n              device_ids: []\n",
        "  malformed:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              count: 1.5\n              device_ids: wrong\n            - capabilities: [gpu]\n              count: true\n            - capabilities: [gpu]\n              count: null\n            - capabilities: [gpu]\n              count: {bad: value}\n            - capabilities: [gpu]\n              count: [bad]\n            - capabilities: [gpu]\n              count: !!timestamp 2023-12-25\n            - capabilities: [gpu]\n              count: !!regex 'gpu.*'\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3130), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let devices = |service| {
        parsed
            .document()
            .and_then(|document| document.service(service))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.reservations())
            .and_then(|reservations| reservations.devices())
            .ok_or("devices")
    };
    let valid = &devices("valid")?.items()[0];
    assert!(matches!(
        valid.count().map(Located::value),
        Some(compose_lens::model::DeployReservationDeviceCount::YamlInteger(value)) if value == "2"
    ));
    assert!(matches!(
        valid
            .device_ids()
            .map(|ids| {
                ids.items()
                    .iter()
                    .map(compose_lens::model::DeployReservationDeviceId::form)
                    .collect::<Vec<_>>()
            })
            .as_deref(),
        Some([
            compose_lens::model::DeployReservationDeviceIdForm::String,
            compose_lens::model::DeployReservationDeviceIdForm::String,
            compose_lens::model::DeployReservationDeviceIdForm::String,
            compose_lens::model::DeployReservationDeviceIdForm::Unmodeled,
            compose_lens::model::DeployReservationDeviceIdForm::Unmodeled,
            compose_lens::model::DeployReservationDeviceIdForm::Unmodeled,
            compose_lens::model::DeployReservationDeviceIdForm::Unmodeled,
        ])
    ));
    assert!(matches!(
        devices("strings")?.items()[0].count().map(Located::value),
        Some(compose_lens::model::DeployReservationDeviceCount::String(value)) if value == "all"
    ));
    assert!(
        devices("strings")?.items()[0]
            .device_ids()
            .is_some_and(|ids| ids.items().is_empty())
    );
    let malformed = devices("malformed")?;
    assert!(malformed.items().iter().all(|item| item.count().is_none()));
    assert_eq!(malformed.items()[0].unknown_fields().len(), 2);
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code()
                == compose_lens::model::DEPLOY_RESERVATION_DEVICE_ALLOCATION_SELECTOR_CONFLICT)
            .count(),
        3
    );
    assert!(parsed.diagnostics().iter().any(|diagnostic| {
        diagnostic.message() == "deploy resource reservation device count must be a YAML integer or string scalar"
    }));
    assert!(parsed.diagnostics().iter().any(|diagnostic| {
        diagnostic.message() == "deploy resource reservation device device_ids must be a sequence"
    }));
    Ok(())
}

#[test]
fn recovers_authored_malformed_reservation_devices_without_dropping_siblings() -> Result<(), Box<dyn std::error::Error>>
{
    let source = concat!(
        "services:\n  malformed:\n    deploy:\n      resources:\n        reservations:\n          devices:\n",
        "            - scalar-item\n            - capabilities: not-a-list\n            - driver: retained\n",
        "            - capabilities: [valid, true, {bad: value}]\n",
        "  duplicate:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [first]\n              capabilities: [second]\n",
        "  outer:\n    deploy: {resources: {reservations: {devices: wrong}}}\n",
        "  reset-null:\n    deploy: {resources: {reservations: {devices: !reset null}}}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3119), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document")?;
    let devices = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.reservations())
    };
    let malformed = devices("malformed").ok_or("malformed reservations")?;
    let items = malformed.devices().ok_or("malformed devices")?.items();
    assert_eq!(items.len(), 4);
    assert!(matches!(
        items[0].form(),
        compose_lens::model::DeployReservationDeviceForm::Unmodeled
    ));
    assert_eq!(&source[items[0].span().range()], "scalar-item");
    assert!(items[1].capabilities().is_none() && items[1].unknown_fields().len() == 1);
    assert_eq!(
        items[2].driver().map(Located::value).map(String::as_str),
        Some("retained")
    );
    let malformed_capabilities = items[3].capabilities().ok_or("partial capabilities")?.items();
    assert!(matches!(
        malformed_capabilities
            .iter()
            .map(compose_lens::model::DeployReservationDeviceCapability::form)
            .collect::<Vec<_>>()
            .as_slice(),
        [
            compose_lens::model::DeployReservationDeviceCapabilityForm::String,
            compose_lens::model::DeployReservationDeviceCapabilityForm::Unmodeled,
            compose_lens::model::DeployReservationDeviceCapabilityForm::Unmodeled,
        ]
    ));
    assert_eq!(&source[malformed_capabilities[1].span().range()], "true");
    let duplicate = devices("duplicate").ok_or("duplicate reservations")?;
    assert_eq!(
        duplicate
            .devices()
            .and_then(|devices| devices.items().first())
            .and_then(|item| item.capabilities())
            .map(|capabilities| capabilities.items()[0].value().map(Located::value).map(String::as_str)),
        Some(Some("first"))
    );
    for service in ["outer", "reset-null"] {
        let reservations = devices(service).ok_or("reservations")?;
        assert!(reservations.devices().is_none());
        assert!(
            reservations
                .unknown_fields()
                .iter()
                .any(|field| field.name().value() == "devices")
        );
    }
    for code in [
        EXPECTED_MAPPING,
        EXPECTED_SEQUENCE,
        EXPECTED_SCALAR,
        compose_lens::model::DEPLOY_RESERVATION_DEVICE_MISSING_CAPABILITIES,
        DUPLICATE_FIELD,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_only_yaml_string_reservation_device_drivers() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n  valid:\n    deploy:\n      resources:\n        reservations:\n          devices:\n",
        "            - capabilities: [gpu]\n              driver: nvidia\n            - capabilities: [gpu]\n              driver: \"\"\n",
        "            - capabilities: [gpu]\n              driver: '${DRIVER}'\n            - capabilities: [gpu]\n              driver: ' Driver '\n",
        "            - capabilities: [gpu]\n              driver: \"2023-12-25\"\n            - capabilities: [gpu]\n              driver: \"gpu.*\"\n",
        "  invalid:\n    deploy:\n      resources:\n        reservations:\n          devices:\n",
        "            - capabilities: [gpu]\n              driver: 1\n            - capabilities: [gpu]\n              driver: true\n",
        "            - capabilities: [gpu]\n              driver: null\n            - capabilities: [gpu]\n              driver: [bad]\n",
        "            - capabilities: [gpu]\n              driver: {bad: value}\n            - capabilities: [gpu]\n              driver: !!timestamp 2023-12-25\n",
        "            - capabilities: [gpu]\n              driver: !!regex 'gpu.*'\n",
        "  duplicate:\n    deploy:\n      resources:\n        reservations:\n          devices:\n            - capabilities: [gpu]\n              driver: first\n              driver: second\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3121), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let device = |service: &str, index| {
        parsed
            .document()
            .and_then(|document| document.service(service))
            .and_then(compose_lens::model::Service::deploy)
            .and_then(|deploy| deploy.resources())
            .and_then(|resources| resources.reservations())
            .and_then(|reservations| reservations.devices())
            .and_then(|devices| devices.items().get(index))
    };
    for (index, expected) in ["nvidia", "", "${DRIVER}", " Driver ", "2023-12-25", "gpu.*"]
        .iter()
        .enumerate()
    {
        let driver = device("valid", index).and_then(compose_lens::model::DeployReservationDevice::driver);
        assert_eq!(driver.map(Located::value).map(String::as_str), Some(*expected));
    }
    for index in 0..7 {
        let invalid = device("invalid", index).ok_or("invalid device")?;
        assert!(invalid.driver().is_none() && invalid.unknown_fields().len() == 1);
    }
    assert_eq!(
        device("duplicate", 0)
            .and_then(compose_lens::model::DeployReservationDevice::driver)
            .map(Located::value)
            .map(String::as_str),
        Some("first")
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
            .all(|diagnostic| diagnostic.message()
                == "deploy resource reservation device driver must be a YAML string scalar")
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    Ok(())
}

#[test]
fn retains_authored_build_privileged_boolean_expression_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(2401),
        "services:\n  yes:\n    build: {privileged: true}\n  no:\n    build: {privileged: false}\n  expression:\n    build: {privileged: \"${PRIVILEGED}\"}\n  duplicate:\n    build:\n      privileged: true\n      privileged: false\n  string:\n    build: {context: kept, privileged: \"yes\"}\n  invalid:\n    build: {context: kept, privileged: 1}\n",
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let doc = parsed.document().ok_or("doc")?;
    let value = |name| {
        authored_build_definition(doc, name)
            .ok()
            .and_then(|d| d.privileged().map(Located::value))
    };
    assert!(
        matches!(value("yes"), Some(BooleanValue::Literal(true)))
            && matches!(value("no"), Some(BooleanValue::Literal(false)))
            && matches!(value("expression"), Some(BooleanValue::Expression(_)))
            && matches!(value("duplicate"), Some(BooleanValue::Literal(true)))
    );
    for name in ["string", "invalid"] {
        let d = authored_build_definition(doc, name)?;
        assert!(d.privileged().is_none() && d.context().is_some());
    }
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    Ok(())
}

#[test]
fn retains_authored_inline_dockerfiles_as_exact_string_scalars() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(2005),
        concat!(
            "services:\n",
            "  inline:\n    build:\n      dockerfile_inline: |-\n        FROM scratch\n        RUN echo inline\n",
            "  empty:\n    build: {dockerfile_inline: \"\"}\n",
            "  malformed:\n    build: {context: retained, dockerfile_inline: false}\n",
            "  duplicate:\n    build:\n      dockerfile_inline: first\n      dockerfile_inline: second\n",
            "  conflicting:\n    build:\n      dockerfile: Dockerfile\n      dockerfile_inline: \"FROM scratch\"\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    let inline = authored_build_definition(document, "inline")?
        .dockerfile_inline()
        .ok_or("inline Dockerfile expected")?;
    assert_eq!(inline.value(), "FROM scratch\nRUN echo inline");
    assert!(!inline.span().range().is_empty());
    assert_eq!(
        authored_build_definition(document, "empty")?
            .dockerfile_inline()
            .map(Located::value)
            .map(String::as_str),
        Some("")
    );
    let malformed = authored_build_definition(document, "malformed")?;
    assert!(malformed.dockerfile_inline().is_none() && malformed.context().is_some());
    assert_eq!(
        authored_build_definition(document, "duplicate")?
            .dockerfile_inline()
            .map(Located::value)
            .map(String::as_str),
        Some("first")
    );
    let conflicting = authored_build_definition(document, "conflicting")?;
    assert!(conflicting.dockerfile().is_some() && conflicting.dockerfile_inline().is_some());
    for code in [DUPLICATE_FIELD, BUILD_DOCKERFILE_INLINE_CONFLICT] {
        assert!(
            parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code),
            "missing {code}"
        );
    }
    Ok(())
}

#[test]
fn retains_authored_build_ulimits_with_service_equivalent_forms() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(1817),
        concat!(
            "services:\n",
            "  app:\n",
            "    build:\n",
            "      ulimits:\n",
            "        nofile: \"001024\"\n",
            "        nproc:\n          soft: \"${SOFT_LIMIT}\"\n          hard: -1\n          x-retained: value\n",
            "        Bad: 7\n",
            "        invalid: nope\n",
            "        malformed: []\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let definition = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => Some(definition),
            Build::Context(_) => None,
        })
        .ok_or("long build definition expected")?;
    let limits = definition.ulimits().ok_or("authored build ulimits expected")?;
    assert_eq!(limits.entries().len(), 4);
    assert!(matches!(limits.entries()[0].value(), UlimitValue::Single(value)
        if value.value().raw() == "001024"));
    assert!(matches!(limits.entries()[1].value(), UlimitValue::Range(range)
        if range.soft().is_some() && range.hard().is_some() && range.extension_fields().len() == 1));
    assert_eq!(limits.entries()[2].name().value(), "Bad");
    assert!(matches!(limits.entries()[3].value(), UlimitValue::Single(value) if !value.value().is_valid()));
    assert!(
        definition
            .fields()
            .iter()
            .any(|field| field.kind() == BuildFieldKind::Ulimits)
    );
    for code in [ULIMIT_INVALID_NAME, ULIMIT_INVALID_VALUE, EXPECTED_FIELD_FORM] {
        assert!(
            parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code),
            "missing {code}; diagnostics: {:?}",
            parsed.diagnostics()
        );
    }
    Ok(())
}
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;

const VOLUME_FORMS: &str = include_str!("../fixtures/typed-model/volume-syntax-fidelity/compose.yaml");
const INVALID_VOLUME_FORMS: &str = include_str!("../fixtures/typed-model/invalid-volume-forms/compose.yaml");
const PHASE_TWO_FORMS: &str = include_str!("../fixtures/typed-model/phase-two-field-forms/compose.yaml");
const INVALID_PHASE_TWO_FORMS: &str = include_str!("../fixtures/typed-model/invalid-phase-two-forms/compose.yaml");
const POST_01_FORMS: &str = include_str!("../fixtures/typed-model/post-01-issue-backlog/compose.yaml");
const POST_01_INVALID: &str = include_str!("../fixtures/typed-model/post-01-invalid/compose.yaml");
const SERVICE_LABEL_FORMS: &str = include_str!("../fixtures/typed-model/service-label-forms/compose.yaml");
const INVALID_SERVICE_LABEL_FORMS: &str =
    include_str!("../fixtures/typed-model/invalid-service-label-forms/compose.yaml");
const NETWORK_LABEL_FORMS: &str = include_str!("../fixtures/typed-model/network-label-forms/compose.yaml");
const INVALID_NETWORK_LABEL_FORMS: &str =
    include_str!("../fixtures/typed-model/invalid-network-label-forms/compose.yaml");
const TRAILING_EMPTY_VALUE: &str = include_str!("../fixtures/roundtrip/canonical-merged/compose.yaml");
const COMMA_PLAIN_SCALAR: &str = include_str!("../fixtures/syntax/comma-plain-scalar/compose.yaml");
const SERVICE_HOSTNAME: &str = include_str!("../fixtures/typed-model/service-hostname-model/compose.yaml");
const SERVICE_PULL_POLICY: &str = include_str!("../fixtures/typed-model/service-pull-policy-model/compose.yaml");
const SERVICE_PIDS_LIMIT: &str = include_str!("../fixtures/typed-model/service-pids-limit-model/compose.yaml");
const SERVICE_SHM_SIZE: &str = include_str!("../fixtures/typed-model/service-shm-size-model/compose.yaml");
const SERVICE_MEM_LIMIT: &str = include_str!("../fixtures/typed-model/service-mem-limit-model/compose.yaml");
const SERVICE_CAP_ADD: &str = include_str!("../fixtures/typed-model/service-cap-add-model/compose.yaml");
const SERVICE_CAP_DROP: &str = include_str!("../fixtures/typed-model/service-cap-drop-model/compose.yaml");
const SERVICE_TMPFS: &str = include_str!("../fixtures/typed-model/service-tmpfs-model/compose.yaml");
const SERVICE_SYSCTLS: &str = include_str!("../fixtures/typed-model/service-sysctls-model/compose.yaml");
const SERVICE_LOGGING: &str = include_str!("../fixtures/typed-model/service-logging-model/compose.yaml");
const SERVICE_DEVICES: &str = include_str!("../fixtures/typed-model/service-devices-model/compose.yaml");
const SERVICE_DNS: &str = include_str!("../fixtures/typed-model/service-dns-model/compose.yaml");
const SERVICE_DNS_OPTIONS: &str = include_str!("../fixtures/typed-model/service-dns-options-model/compose.yaml");
const SERVICE_DNS_SEARCH: &str = include_str!("../fixtures/typed-model/service-dns-search-model/compose.yaml");
const SERVICE_EXPOSE: &str = include_str!("../fixtures/typed-model/service-expose-model/compose.yaml");
const SERVICE_ANNOTATIONS: &str = include_str!("../fixtures/typed-model/service-annotations-model/compose.yaml");
const SERVICE_SECURITY_OPTIONS: &str =
    include_str!("../fixtures/typed-model/service-security-options-model/compose.yaml");

fn assert_security_option_candidate_kinds(exact: &compose_lens::model::SecurityOptions) {
    use compose_lens::model::SecurityOptionKind;

    assert_eq!(exact.items().len(), 128);
    assert!(matches!(
        exact.items()[0].kind(),
        SecurityOptionKind::AppArmor { profile } if profile == "profile-a"
    ));
    for (index, enabled) in [(1, true), (2, false), (3, true)] {
        assert!(matches!(
            exact.items()[index].kind(),
            SecurityOptionKind::NoNewPrivileges { enabled: actual } if *actual == enabled
        ));
    }
    assert!(matches!(
        exact.items()[4].kind(),
        SecurityOptionKind::SecurityLabelDisableNearMiss
    ));
    assert!(matches!(exact.items()[7].kind(), SecurityOptionKind::Expression));
    assert!(matches!(exact.items()[8].kind(), SecurityOptionKind::Empty));
    assert!(
        exact.items()[9..13]
            .iter()
            .all(|item| matches!(item.kind(), SecurityOptionKind::AppArmorNearMiss))
    );
    assert!(
        exact.items()[13..18]
            .iter()
            .all(|item| matches!(item.kind(), SecurityOptionKind::NoNewPrivilegesNearMiss))
    );
    for (index, profile) in [(18, "unconfined"), (19, "/workspace/seccomp.json"), (20, "unconfined")] {
        assert!(matches!(
            exact.items()[index].kind(),
            SecurityOptionKind::Seccomp { profile: actual } if actual == profile
        ));
    }
    assert!(matches!(exact.items()[21].kind(), SecurityOptionKind::Expression));
    assert!(
        exact.items()[22..28]
            .iter()
            .all(|item| matches!(item.kind(), SecurityOptionKind::SeccompNearMiss))
    );
    for index in [28, 29] {
        assert!(matches!(
            exact.items()[index].kind(),
            SecurityOptionKind::SecurityLabelDisable { enabled: true }
        ));
    }
    assert!(
        exact.items()[30..35]
            .iter()
            .all(|item| matches!(item.kind(), SecurityOptionKind::SecurityLabelDisableNearMiss))
    );
    assert!(
        exact.items()[35..37]
            .iter()
            .all(|item| matches!(item.kind(), SecurityOptionKind::Other))
    );
    assert!(matches!(exact.items()[37].kind(), SecurityOptionKind::Expression));
    for index in [38, 39] {
        assert!(matches!(
            exact.items()[index].kind(),
            SecurityOptionKind::SecurityLabelFileType { file_type }
                if file_type == "container_file_t"
        ));
    }
    assert!(
        exact.items()[40..48]
            .iter()
            .all(|item| matches!(item.kind(), SecurityOptionKind::SecurityLabelFileTypeNearMiss))
    );
    assert!(matches!(
        exact.items()[48].kind(),
        SecurityOptionKind::SecurityLabelType { label_type } if label_type == "TYPE"
    ));
    assert!(
        exact.items()[49..51]
            .iter()
            .all(|item| matches!(item.kind(), SecurityOptionKind::Other))
    );
    assert!(matches!(
        exact.items()[51].kind(),
        SecurityOptionKind::SecurityLabelLevel { level } if level == "LEVEL"
    ));
    assert!(matches!(exact.items()[52].kind(), SecurityOptionKind::Expression));
    for index in [53, 54] {
        assert!(matches!(
            exact.items()[index].kind(),
            SecurityOptionKind::SecurityLabelLevel { level } if level == "s0:c1,c2"
        ));
    }
    assert!(
        exact.items()[55..64]
            .iter()
            .all(|item| matches!(item.kind(), SecurityOptionKind::SecurityLabelLevelNearMiss))
    );
    assert!(matches!(exact.items()[64].kind(), SecurityOptionKind::Expression));
    assert_security_label_nested_candidate_kinds(exact);
}

fn assert_security_label_nested_candidate_kinds(exact: &compose_lens::model::SecurityOptions) {
    use compose_lens::model::SecurityOptionKind;

    for index in [65, 66] {
        assert!(matches!(
            exact.items()[index].kind(),
            SecurityOptionKind::SecurityLabelNested { enabled: true }
        ));
    }
    assert!(
        exact.items()[67..75]
            .iter()
            .all(|item| matches!(item.kind(), SecurityOptionKind::SecurityLabelNestedNearMiss))
    );
    assert!(matches!(exact.items()[75].kind(), SecurityOptionKind::Expression));
    assert_security_label_type_candidate_kinds(exact);
}

fn assert_security_label_type_candidate_kinds(exact: &compose_lens::model::SecurityOptions) {
    use compose_lens::model::SecurityOptionKind;

    for index in [76, 77] {
        assert!(matches!(
            exact.items()[index].kind(),
            SecurityOptionKind::SecurityLabelType { label_type } if label_type == "container_t"
        ));
    }
    assert!(
        exact.items()[78..92]
            .iter()
            .all(|item| matches!(item.kind(), SecurityOptionKind::SecurityLabelTypeNearMiss))
    );
    assert!(matches!(exact.items()[92].kind(), SecurityOptionKind::Expression));
    assert_mask_candidate_kinds(exact);
}

fn assert_mask_candidate_kinds(exact: &compose_lens::model::SecurityOptions) {
    use compose_lens::model::SecurityOptionKind;

    for index in [93, 94] {
        assert!(matches!(
            exact.items()[index].kind(),
            SecurityOptionKind::Mask { paths } if paths == "/proc/acpi:/proc/kcore"
        ));
    }
    assert!(matches!(
        exact.items()[95].kind(),
        SecurityOptionKind::Mask { paths } if paths == "relative:opaque=value"
    ));
    assert!(matches!(exact.items()[96].kind(), SecurityOptionKind::Expression));
    assert!(
        exact.items()[97..105]
            .iter()
            .all(|item| matches!(item.kind(), SecurityOptionKind::MaskNearMiss))
    );
    assert_unmask_candidate_kinds(exact);
}

fn assert_unmask_candidate_kinds(exact: &compose_lens::model::SecurityOptions) {
    use compose_lens::model::SecurityOptionKind;

    for index in [105, 106] {
        assert!(matches!(
            exact.items()[index].kind(),
            SecurityOptionKind::Unmask { paths } if paths == "ALL"
        ));
    }
    for (index, expected) in [(107, "/proc/acpi"), (108, "/proc/acpi:/sys/firmware"), (109, "/proc/*")] {
        assert!(matches!(
            exact.items()[index].kind(),
            SecurityOptionKind::Unmask { paths } if paths == expected
        ));
    }
    assert!(matches!(exact.items()[110].kind(), SecurityOptionKind::Expression));
    assert!(
        exact.items()[111..128]
            .iter()
            .all(|item| matches!(item.kind(), SecurityOptionKind::UnmaskNearMiss))
    );
}

fn assert_unmask_near_miss_diagnostics(parsed: &compose_lens::model::ModelParse) {
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == compose_lens::model::SECURITY_OPT_UNMASK_NEAR_MISS)
            .count(),
        17
    );
}

#[test]
fn types_raw_security_options_and_narrow_candidates_with_recovery() -> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{
        SECURITY_OPT_APPARMOR_CONFLICT, SECURITY_OPT_APPARMOR_NEAR_MISS, SECURITY_OPT_EMPTY_ITEM,
        SECURITY_OPT_EXPECTED_SEQUENCE, SECURITY_OPT_EXPECTED_STRING, SECURITY_OPT_MASK_NEAR_MISS,
        SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT, SECURITY_OPT_NO_NEW_PRIVILEGES_NEAR_MISS,
        SECURITY_OPT_SECCOMP_CONFLICT, SECURITY_OPT_SECCOMP_NEAR_MISS, SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_DISABLE_NEAR_MISS, SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_FILETYPE_NEAR_MISS, SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_LEVEL_NEAR_MISS, SECURITY_OPT_SECURITY_LABEL_NESTED_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_NESTED_NEAR_MISS, SECURITY_OPT_SECURITY_LABEL_TYPE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_TYPE_NEAR_MISS, SECURITY_OPT_UNMASK_NEAR_MISS,
    };

    let syntax = SyntaxDocument::parse(SourceId::new(697), SERVICE_SECURITY_OPTIONS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial security_opt document expected")?;

    assert!(
        document
            .service("omitted")
            .is_some_and(|service| service.security_options().is_none())
    );
    let empty = document
        .service("empty-sequence")
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("explicit empty security_opt expected")?;
    assert!(empty.items().is_empty());
    assert!(!empty.span().range().is_empty());

    let exact = document
        .service("exact")
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("typed security_opt expected")?;
    assert_security_option_candidate_kinds(exact);
    assert!(exact.items().iter().all(|item| !item.span().range().is_empty()));

    let recovered = document
        .service("malformed-siblings")
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("partially recovered security_opt expected")?;
    assert_eq!(
        recovered
            .items()
            .iter()
            .map(compose_lens::model::SecurityOptionItem::value)
            .collect::<Vec<_>>(),
        ["valid-before", "valid-after"]
    );
    assert!(
        document
            .service("malformed-siblings")
            .is_some_and(|service| service.image().is_some())
    );
    for code in [
        SECURITY_OPT_EXPECTED_SEQUENCE,
        SECURITY_OPT_EXPECTED_STRING,
        SECURITY_OPT_EMPTY_ITEM,
        SECURITY_OPT_APPARMOR_NEAR_MISS,
        SECURITY_OPT_APPARMOR_CONFLICT,
        SECURITY_OPT_NO_NEW_PRIVILEGES_NEAR_MISS,
        SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT,
        SECURITY_OPT_SECCOMP_NEAR_MISS,
        SECURITY_OPT_SECCOMP_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_DISABLE_NEAR_MISS,
        SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_FILETYPE_NEAR_MISS,
        SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_LEVEL_NEAR_MISS,
        SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_NESTED_NEAR_MISS,
        SECURITY_OPT_SECURITY_LABEL_NESTED_CONFLICT,
        SECURITY_OPT_SECURITY_LABEL_TYPE_NEAR_MISS,
        SECURITY_OPT_SECURITY_LABEL_TYPE_CONFLICT,
        SECURITY_OPT_MASK_NEAR_MISS,
        SECURITY_OPT_UNMASK_NEAR_MISS,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_EXPECTED_STRING)
            .count(),
        5
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_SECCOMP_CONFLICT)
            .count(),
        2
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_MASK_NEAR_MISS)
            .count(),
        8
    );
    assert_unmask_near_miss_diagnostics(&parsed);
    Ok(())
}

#[test]
fn types_annotation_forms_scalar_kinds_duplicates_ambiguity_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{
        ANNOTATIONS_DUPLICATE_NAME, ANNOTATIONS_EMPTY_NAME, ANNOTATIONS_EXPECTED_FORM, ANNOTATIONS_EXPECTED_STRING,
        ANNOTATIONS_KEY_ONLY, AnnotationsForm,
    };

    let syntax = SyntaxDocument::parse(SourceId::new(696), SERVICE_ANNOTATIONS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial annotations document expected")?;
    assert!(
        document
            .service("omitted")
            .is_some_and(|service| service.annotations().is_none())
    );
    assert!(matches!(
        document.service("empty-map").and_then(compose_lens::model::Service::annotations).map(compose_lens::model::Annotations::form),
        Some(AnnotationsForm::Map(entries)) if entries.is_empty()
    ));
    assert!(matches!(
        document.service("empty-list").and_then(compose_lens::model::Service::annotations).map(compose_lens::model::Annotations::form),
        Some(AnnotationsForm::List(items)) if items.is_empty()
    ));

    let mapping = document
        .service("mapping")
        .and_then(compose_lens::model::Service::annotations)
        .ok_or("mapping annotations expected")?;
    let AnnotationsForm::Map(entries) = mapping.form() else {
        return Err("mapping form expected".into());
    };
    assert_eq!(entries.len(), 7, "authored duplicate mapping entries stay visible");
    assert!(matches!(entries[0].value().value(), ComposeScalar::String(value) if value.contains("STRING_VALUE")));
    assert!(matches!(entries[1].value().value(), ComposeScalar::Number(value) if value == "001"));
    assert!(matches!(entries[2].value().value(), ComposeScalar::Boolean(true)));
    assert!(matches!(entries[3].value().value(), ComposeScalar::Null));
    assert_eq!(entries[4].key().value(), entries[5].key().value());
    assert!(entries.iter().all(|entry| !entry.span().range().is_empty()));

    let list = document
        .service("list")
        .and_then(compose_lens::model::Service::annotations)
        .ok_or("list annotations expected")?;
    let AnnotationsForm::List(items) = list.form() else {
        return Err("list form expected".into());
    };
    assert_eq!(items.len(), 9, "scalar invalid items remain authored evidence");
    assert!(matches!(&items[1].value(), ComposeScalar::String(value) if value == "io.example.equals=left=right"));
    assert!(matches!(items[6].value(), ComposeScalar::Number(value) if value == "7"));
    assert!(matches!(items[7].value(), ComposeScalar::Boolean(true)));
    assert!(matches!(items[8].value(), ComposeScalar::Null));
    assert!(
        document
            .service("wrong-form")
            .is_some_and(|service| service.annotations().is_none())
    );
    for code in [
        ANNOTATIONS_DUPLICATE_NAME,
        ANNOTATIONS_EMPTY_NAME,
        ANNOTATIONS_EXPECTED_FORM,
        ANNOTATIONS_EXPECTED_STRING,
        ANNOTATIONS_KEY_ONLY,
    ] {
        assert!(
            parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code),
            "missing {code}"
        );
    }
    Ok(())
}

#[test]
fn types_expose_string_and_number_identity_classification_and_sibling_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{
        EXPOSE_DUPLICATE_ITEM, EXPOSE_EXPECTED_SCALAR, EXPOSE_EXPECTED_SEQUENCE, EXPOSE_INVALID_ITEM,
        EXPOSE_PROVIDER_DEPENDENT, ExposeItemKind, ExposeScalarKind,
    };

    let syntax = SyntaxDocument::parse(SourceId::new(690), SERVICE_EXPOSE)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;
    assert!(
        document
            .service("omitted")
            .is_some_and(|service| service.expose().is_none())
    );
    assert!(
        document
            .service("empty")
            .and_then(compose_lens::model::Service::expose)
            .is_some_and(|expose| expose.items().is_empty())
    );

    let items = document
        .service("exact")
        .and_then(compose_lens::model::Service::expose)
        .ok_or("typed expose expected")?
        .items();
    assert_eq!(items.len(), 11);
    assert_eq!(items[0].value(), "80");
    assert_eq!(items[0].scalar_kind(), ExposeScalarKind::Number);
    assert_eq!(items[1].scalar_kind(), ExposeScalarKind::String);
    assert!(matches!(items[2].kind(), ExposeItemKind::Documented { .. }));
    assert!(matches!(items[5].kind(), ExposeItemKind::Expression));
    assert!(matches!(items[6].kind(), ExposeItemKind::Sctp { .. }));
    assert!(matches!(items[7].kind(), ExposeItemKind::UnknownProtocol { protocol, .. } if protocol == "HTTP"));
    assert!(matches!(items[8].kind(), ExposeItemKind::Malformed));
    assert!(items.iter().all(|item| !item.span().range().is_empty()));

    let recovered = document
        .service("malformed-siblings")
        .and_then(compose_lens::model::Service::expose)
        .ok_or("partial expose expected")?;
    assert_eq!(
        recovered
            .items()
            .iter()
            .map(compose_lens::model::ExposeItem::value)
            .collect::<Vec<_>>(),
        ["90", "93/udp"]
    );
    assert!(
        document
            .service("wrong-form")
            .is_some_and(|service| service.expose().is_none())
    );
    for code in [
        EXPOSE_EXPECTED_SCALAR,
        EXPOSE_EXPECTED_SEQUENCE,
        EXPOSE_INVALID_ITEM,
        EXPOSE_PROVIDER_DEPENDENT,
        EXPOSE_DUPLICATE_ITEM,
    ] {
        assert!(
            parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code),
            "missing {code}"
        );
    }
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == EXPOSE_DUPLICATE_ITEM)
            .count(),
        2
    );
    Ok(())
}

#[test]
fn types_raw_dns_search_scalar_and_list_forms_and_recovers_valid_siblings() -> Result<(), Box<dyn std::error::Error>> {
    let source_id = SourceId::new(689);
    let syntax = SyntaxDocument::parse(source_id, SERVICE_DNS_SEARCH)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;

    assert!(
        document
            .service("omitted")
            .is_some_and(|service| service.dns_search().is_none())
    );
    let scalar = document
        .service("scalar")
        .and_then(compose_lens::model::Service::dns_search)
        .ok_or("scalar dns_search expected")?;
    assert!(matches!(scalar.form(), DnsSearchForm::Scalar(value) if value.value() == "."));
    assert!(!scalar.span().range().is_empty());

    let empty = document
        .service("empty-list")
        .and_then(compose_lens::model::Service::dns_search)
        .ok_or("empty dns_search expected")?;
    assert!(matches!(empty.form(), DnsSearchForm::List(values) if values.is_empty()));

    let exact = document
        .service("exact-list")
        .and_then(compose_lens::model::Service::dns_search)
        .ok_or("dns_search list expected")?;
    let DnsSearchForm::List(values) = exact.form() else {
        return Err("dns_search list form expected".into());
    };
    assert_eq!(
        values.iter().map(|value| value.value().as_str()).collect::<Vec<_>>(),
        [
            "example.internal",
            "example.internal",
            ".",
            "${DNS_SEARCH:-corp.internal}"
        ]
    );
    assert!(values.iter().all(|value| !value.span().range().is_empty()));

    let recovered = document
        .service("malformed-items")
        .and_then(compose_lens::model::Service::dns_search)
        .ok_or("partial dns_search list expected")?;
    assert!(matches!(recovered.form(), DnsSearchForm::List(values)
        if values.iter().map(|value| value.value().as_str()).collect::<Vec<_>>()
            == ["valid-before.internal", "valid-after.internal"]));
    assert!(
        document
            .service("malformed-items")
            .is_some_and(|service| service.image().is_some())
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|item| item.code() == DNS_SEARCH_EXPECTED_STRING)
            .count(),
        5
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|item| item.code() == DNS_SEARCH_EXPECTED_FORM)
            .count(),
        4
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|item| item.code() == DNS_SEARCH_DUPLICATE_ITEM)
            .count(),
        1
    );
    Ok(())
}

#[test]
fn types_dns_options_as_an_ordered_unique_string_sequence_with_loss_aware_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{DNS_OPT_DUPLICATE_ITEM, DNS_OPT_EXPECTED_SEQUENCE, DNS_OPT_EXPECTED_STRING};

    let syntax = SyntaxDocument::parse(SourceId::new(687), SERVICE_DNS_OPTIONS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;

    assert!(
        document
            .service("omitted")
            .is_some_and(|service| service.dns_options().is_none())
    );
    let empty = document
        .service("empty")
        .and_then(compose_lens::model::Service::dns_options)
        .ok_or("explicit empty dns_opt expected")?;
    assert!(empty.items().is_empty());
    assert!(!empty.span().range().is_empty());

    let exact = document
        .service("exact")
        .and_then(compose_lens::model::Service::dns_options)
        .ok_or("typed dns_opt expected")?;
    assert_eq!(
        exact
            .items()
            .iter()
            .map(|item| item.value().as_str())
            .collect::<Vec<_>>(),
        ["ndots:5", "timeout:2", "ndots:5", "${DNS_OPTION:-attempts:3}"]
    );
    assert!(exact.items().iter().all(|item| !item.span().range().is_empty()));

    let recovered = document
        .service("malformed-items")
        .and_then(compose_lens::model::Service::dns_options)
        .ok_or("partially recovered dns_opt expected")?;
    assert_eq!(
        recovered
            .items()
            .iter()
            .map(|item| item.value().as_str())
            .collect::<Vec<_>>(),
        ["valid-before", "valid-after"]
    );
    assert!(
        document
            .service("malformed-items")
            .is_some_and(|service| service.image().is_some())
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|item| item.code() == DNS_OPT_DUPLICATE_ITEM)
            .count(),
        1
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|item| item.code() == DNS_OPT_EXPECTED_STRING)
            .count(),
        5
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|item| item.code() == DNS_OPT_EXPECTED_SEQUENCE)
            .count(),
        3
    );
    Ok(())
}

#[test]
fn types_raw_dns_scalar_and_list_forms_and_recovers_valid_siblings() -> Result<(), Box<dyn std::error::Error>> {
    let source_id = SourceId::new(684);
    let syntax = SyntaxDocument::parse(source_id, SERVICE_DNS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;

    assert!(
        document
            .service("omitted")
            .is_some_and(|service| service.dns().is_none())
    );
    let scalar = document
        .service("scalar")
        .and_then(compose_lens::model::Service::dns)
        .ok_or("scalar dns expected")?;
    assert!(matches!(scalar.form(), DnsForm::Scalar(value) if value.value() == "local-resolver.example"));
    assert!(!scalar.span().range().is_empty());

    let empty = document
        .service("empty-list")
        .and_then(compose_lens::model::Service::dns)
        .ok_or("empty dns expected")?;
    assert!(matches!(empty.form(), DnsForm::List(values) if values.is_empty()));

    let exact = document
        .service("exact-list")
        .and_then(compose_lens::model::Service::dns)
        .ok_or("dns list expected")?;
    let DnsForm::List(values) = exact.form() else {
        return Err("dns list form expected".into());
    };
    assert_eq!(
        values.iter().map(|value| value.value().as_str()).collect::<Vec<_>>(),
        [
            "1.1.1.1",
            "1.1.1.1",
            "local-resolver.example",
            "${DNS_SERVER:-resolver.internal}",
            "2001:db8::53",
        ]
    );
    assert!(values.iter().all(|value| !value.span().range().is_empty()));

    let recovered = document
        .service("malformed-items")
        .and_then(compose_lens::model::Service::dns)
        .ok_or("partial DNS list expected")?;
    assert!(matches!(recovered.form(), DnsForm::List(values)
        if values.iter().map(|value| value.value().as_str()).collect::<Vec<_>>()
            == ["valid-before.example", "valid-after.example"]));
    assert!(
        document
            .service("malformed-items")
            .is_some_and(|service| service.image().is_some())
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == DNS_EXPECTED_STRING)
            .count(),
        5
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == DNS_EXPECTED_FORM)
            .count(),
        4
    );
    Ok(())
}

#[test]
fn types_ordered_mixed_devices_and_recovers_invalid_forms_without_runtime_validation()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::ShortDeviceKind;

    let source_id = SourceId::new(82);
    let syntax = SyntaxDocument::parse(source_id, SERVICE_DEVICES)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;
    assert!(
        document
            .service("omitted")
            .is_some_and(|service| service.devices().is_none())
    );
    assert!(
        document
            .service("empty")
            .and_then(compose_lens::model::Service::devices)
            .is_some_and(|devices| devices.items().is_empty())
    );

    let devices = document
        .service("mixed")
        .and_then(compose_lens::model::Service::devices)
        .ok_or("mixed devices expected")?;
    assert_eq!(devices.items().len(), 7);
    let short = devices.items()[..5]
        .iter()
        .map(|device| match device {
            Device::Short(device) => Ok((device.raw().value().as_str(), device.kind())),
            Device::Long(_) => Err("short device expected"),
            _ => Err("unknown device form"),
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        short,
        [
            ("/dev/dri:/dev/dri:rwm", ShortDeviceKind::Path),
            ("/dev/dri:/dev/dri:rwm", ShortDeviceKind::Path),
            ("vendor.example/device=gpu", ShortDeviceKind::Cdi),
            (
                "${DEVICE_SELECTOR:-vendor.example/device=gpu}",
                ShortDeviceKind::Deferred
            ),
            ("provider-token", ShortDeviceKind::Opaque),
        ]
    );
    assert!(
        devices
            .items()
            .iter()
            .all(|device| device.span().source_id() == source_id)
    );
    let Device::Long(long) = &devices.items()[5] else {
        return Err("long device expected".into());
    };
    assert_eq!(
        long.source().map(Located::value).map(String::as_str),
        Some("/dev/video0")
    );
    assert_eq!(
        long.target().map(Located::value).map(String::as_str),
        Some("/dev/camera")
    );
    assert_eq!(
        long.permissions().map(Located::value).map(String::as_str),
        Some("weird-permissions")
    );
    assert_eq!(long.extension_fields().len(), 1);
    assert_eq!(long.unknown_fields().len(), 1);

    let recovered = document
        .service("malformed-items")
        .and_then(compose_lens::model::Service::devices)
        .ok_or("partially recovered devices expected")?;
    assert_eq!(recovered.items().len(), 4);
    assert!(
        document
            .service("malformed-items")
            .is_some_and(|service| service.image().is_some())
    );
    for service in ["scalar-field", "mapping-field", "null-field"] {
        assert!(
            document
                .service(service)
                .is_some_and(|service| service.devices().is_none() && service.image().is_some())
        );
    }
    for (code, count) in [
        (DEVICES_EXPECTED_SEQUENCE, 3),
        (DEVICE_EXPECTED_FORM, 4),
        (DEVICE_EXPECTED_STRING, 3),
        (DEVICE_MISSING_SOURCE, 2),
    ] {
        assert_eq!(
            parsed
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.code() == code)
                .count(),
            count,
            "unexpected diagnostic count for {code}"
        );
    }
    Ok(())
}

#[test]
fn types_service_sysctls_without_normalizing_form_scalars_or_invalid_siblings() -> Result<(), Box<dyn std::error::Error>>
{
    let source_id = SourceId::new(81);
    let syntax = SyntaxDocument::parse(source_id, SERVICE_SYSCTLS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;

    assert!(
        document
            .service("omitted")
            .is_some_and(|service| service.sysctls().is_none())
    );
    assert_typed_sysctls_mapping(document, source_id)?;
    assert_typed_sysctls_collections(document, source_id)?;
    assert_typed_sysctls_recovery(document, &parsed)?;
    Ok(())
}

#[test]
fn types_service_logging_without_interpreting_drivers_or_losing_malformed_siblings()
-> Result<(), Box<dyn std::error::Error>> {
    let source_id = SourceId::new(812);
    let syntax = SyntaxDocument::parse(source_id, SERVICE_LOGGING)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial logging document expected")?;

    assert!(
        document
            .service("omitted")
            .is_some_and(|service| service.logging().is_none())
    );
    assert!(
        document
            .service("empty")
            .and_then(compose_lens::model::Service::logging)
            .is_some_and(|logging| logging.driver().is_none() && logging.options().is_none())
    );
    let logging = document
        .service("configured")
        .and_then(compose_lens::model::Service::logging)
        .ok_or("configured logging expected")?;
    assert_eq!(
        logging.driver().map(Located::value).map(String::as_str),
        Some("vendor-driver:${DRIVER_SUFFIX}")
    );
    assert_eq!(logging.extension_fields().len(), 1);
    assert_eq!(logging.unknown_fields().len(), 1);
    let options = logging.options().ok_or("logging options expected")?;
    assert_eq!(
        options
            .entries()
            .iter()
            .map(|entry| entry.name().value().as_str())
            .collect::<Vec<_>>(),
        ["string-option", "number-option", "null-option", "expression-option"]
    );
    assert!(matches!(options.entries()[0].value().value(), LoggingOptionValue::String(value) if value == "01"));
    assert!(matches!(options.entries()[1].value().value(), LoggingOptionValue::Number(value) if value == "0001"));
    assert!(matches!(options.entries()[2].value().value(), LoggingOptionValue::Null));
    assert!(
        options
            .entries()
            .iter()
            .all(|entry| entry.span().source_id() == source_id)
    );
    assert!(
        document
            .service("empty-options")
            .and_then(compose_lens::model::Service::logging)
            .and_then(compose_lens::model::Logging::options)
            .is_some_and(|options| options.entries().is_empty())
    );

    assert_typed_logging_recovery(document)?;

    for code in [
        LOGGING_DRIVER_EXPECTED_STRING,
        LOGGING_OPTIONS_EXPECTED_MAPPING,
        LOGGING_OPTION_EMPTY_KEY,
        LOGGING_OPTION_EXPECTED_SCALAR,
        LOGGING_EXPECTED_MAPPING,
    ] {
        assert!(
            parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code),
            "missing {code}"
        );
    }
    Ok(())
}

fn assert_typed_logging_recovery(document: &ComposeDocument) -> Result<(), Box<dyn std::error::Error>> {
    let malformed_driver = document
        .service("malformed-driver")
        .ok_or("malformed driver service expected")?;
    let logging = malformed_driver
        .logging()
        .ok_or("partially recovered logging expected")?;
    assert!(logging.driver().is_none());
    assert!(logging.options().is_some_and(|options| options.entries().len() == 1));
    assert!(malformed_driver.image().is_some());
    let malformed_options = document
        .service("malformed-options")
        .and_then(compose_lens::model::Service::logging)
        .ok_or("malformed options logging expected")?;
    assert_eq!(
        malformed_options.driver().map(Located::value).map(String::as_str),
        Some("retained-driver")
    );
    let recovered = malformed_options.options().ok_or("recovered options expected")?;
    assert_eq!(
        recovered
            .entries()
            .iter()
            .map(|entry| entry.name().value().as_str())
            .collect::<Vec<_>>(),
        ["valid-before", "valid-after"]
    );
    assert_eq!(recovered.unmodeled_entries().len(), 3);
    assert!(
        document
            .service("malformed-options-field")
            .and_then(compose_lens::model::Service::logging)
            .is_some_and(|logging| logging.driver().is_some() && logging.options().is_none())
    );
    assert!(
        document
            .service("malformed-logging")
            .is_some_and(|service| service.logging().is_none() && service.image().is_some())
    );
    Ok(())
}

fn assert_typed_sysctls_mapping(
    document: &ComposeDocument,
    source_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let mapping = document
        .service("mapping")
        .and_then(compose_lens::model::Service::sysctls)
        .ok_or("mapping sysctls expected")?;
    assert_eq!(mapping.span().source_id(), source_id);
    let SysctlsForm::Map(entries) = mapping.form() else {
        return Err("mapping sysctls form expected".into());
    };
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.key().value().as_str())
            .collect::<Vec<_>>(),
        [
            "net.ipv4.ip_forward",
            "kernel.shm_rmid_forced",
            "fs.protected_hardlinks",
            "net.ipv6.conf.all.disable_ipv6",
            "user.${LITERAL_KEY}",
        ]
    );
    assert_eq!(entries[0].value().value(), &ComposeScalar::String("01".to_owned()));
    assert_eq!(entries[1].value().value(), &ComposeScalar::Number("0001".to_owned()));
    assert_eq!(entries[2].value().value(), &ComposeScalar::Boolean(true));
    assert_eq!(entries[3].value().value(), &ComposeScalar::Null);
    assert_eq!(
        entries[4].value().value(),
        &ComposeScalar::String("${SENSITIVE_VALUE}".to_owned())
    );
    assert!(entries.iter().all(|entry| entry.span().source_id() == source_id));
    Ok(())
}

fn assert_typed_sysctls_collections(
    document: &ComposeDocument,
    source_id: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    for (service_name, list_form) in [("empty-map", false), ("empty-list", true)] {
        let sysctls = document
            .service(service_name)
            .and_then(compose_lens::model::Service::sysctls)
            .ok_or("explicit empty sysctls expected")?;
        assert!(match sysctls.form() {
            SysctlsForm::Map(entries) => !list_form && entries.is_empty(),
            SysctlsForm::List(items) => list_form && items.is_empty(),
            _ => false,
        });
    }

    let list = document
        .service("list")
        .and_then(compose_lens::model::Service::sysctls)
        .ok_or("list sysctls expected")?;
    let SysctlsForm::List(items) = list.form() else {
        return Err("list sysctls form expected".into());
    };
    assert_eq!(
        items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>(),
        [
            "net.core.somaxconn=1024",
            "${SYSCTL_ASSIGNMENT}",
            "net.core.somaxconn=1024",
        ]
    );
    assert!(items.iter().all(|item| item.span().source_id() == source_id));
    Ok(())
}

fn assert_typed_sysctls_recovery(
    document: &ComposeDocument,
    parsed: &compose_lens::model::ModelParse,
) -> Result<(), Box<dyn std::error::Error>> {
    let recovered_map = document
        .service("malformed-map")
        .and_then(compose_lens::model::Service::sysctls)
        .ok_or("partially recovered sysctls map expected")?;
    assert!(matches!(
        recovered_map.form(),
        SysctlsForm::Map(entries)
            if entries.iter().map(|entry| entry.key().value().as_str()).collect::<Vec<_>>()
                == ["valid.before", "valid.after"]
    ));
    let recovered_list = document
        .service("malformed-list")
        .and_then(compose_lens::model::Service::sysctls)
        .ok_or("partially recovered sysctls list expected")?;
    assert!(matches!(
        recovered_list.form(),
        SysctlsForm::List(items)
            if items.iter().map(|item| item.value().as_str()).collect::<Vec<_>>()
                == ["valid.before=value", "valid.after=value"]
    ));
    assert!(
        document
            .service("malformed-map")
            .is_some_and(|service| service.image().is_some())
    );
    assert!(
        document
            .service("malformed-list")
            .is_some_and(|service| service.image().is_some())
    );
    assert!(
        document
            .service("scalar")
            .is_some_and(|service| service.sysctls().is_none())
    );
    assert!(
        document
            .service("deploy-only")
            .is_some_and(|service| { service.sysctls().is_none() && service.deploy().is_some() })
    );

    for (code, count) in [
        (SYSCTLS_DUPLICATE_ITEM, 1),
        (SYSCTLS_EMPTY_KEY, 1),
        (SYSCTLS_EXPECTED_SCALAR, 1),
        (SYSCTLS_EXPECTED_STRING, 5),
        (SYSCTLS_EXPECTED_FORM, 1),
    ] {
        assert_eq!(
            parsed
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.code() == code)
                .count(),
            count,
            "unexpected diagnostic count for {code}"
        );
    }
    Ok(())
}

#[test]
fn types_service_tmpfs_without_normalizing_form_items_or_recovery() -> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{TmpfsForm, TmpfsItemKind};

    let source_id = SourceId::new(79);
    let syntax = SyntaxDocument::parse(source_id, SERVICE_TMPFS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;
    assert!(
        document
            .service("omitted")
            .is_some_and(|service| service.tmpfs().is_none())
    );

    let scalar = document
        .service("scalar")
        .and_then(compose_lens::model::Service::tmpfs)
        .ok_or("scalar tmpfs expected")?;
    let TmpfsForm::Scalar(item) = scalar.form() else {
        return Err("scalar tmpfs form expected".into());
    };
    assert_eq!(item.value(), "/run:mode=1777,uid=1000,gid=1000");
    assert_eq!(item.kind(), TmpfsItemKind::Documented);
    assert_eq!(item.span().source_id(), source_id);

    let empty = document
        .service("empty-list")
        .and_then(compose_lens::model::Service::tmpfs)
        .ok_or("explicit empty tmpfs list expected")?;
    assert!(matches!(empty.form(), TmpfsForm::List(items) if items.is_empty()));

    let exact = document
        .service("exact-list")
        .and_then(compose_lens::model::Service::tmpfs)
        .ok_or("tmpfs list expected")?;
    let TmpfsForm::List(items) = exact.form() else {
        return Err("tmpfs list form expected".into());
    };
    assert_eq!(
        items
            .iter()
            .map(compose_lens::model::TmpfsItem::value)
            .collect::<Vec<_>>(),
        [
            "/cache",
            "/cache",
            "/path,with,commas",
            "/state:uid=User,gid=Group,mode=0700",
            "${TMPFS_PATH:-/expression}",
            "/provider:size=64m",
            "/provider:exec,nosuid",
            "/case:Mode=1777",
            ""
        ]
    );
    assert_eq!(items[2].kind(), TmpfsItemKind::Documented);
    assert_eq!(items[3].kind(), TmpfsItemKind::Documented);
    assert_eq!(items[4].kind(), TmpfsItemKind::Expression);
    assert!(
        items[5..]
            .iter()
            .all(|item| item.kind() == TmpfsItemKind::ProviderDependent)
    );

    let recovered = document
        .service("malformed-items")
        .and_then(compose_lens::model::Service::tmpfs)
        .ok_or("partially recovered tmpfs list expected")?;
    assert!(matches!(
        recovered.form(),
        TmpfsForm::List(items)
            if items.iter().map(compose_lens::model::TmpfsItem::value).collect::<Vec<_>>()
                == ["/valid", "/later:mode=0755"]
    ));
    assert!(
        document
            .service("malformed-items")
            .is_some_and(|service| service.image().is_some())
    );
    for service_name in ["null", "boolean", "number", "mapping"] {
        let service = document.service(service_name).ok_or("malformed service retained")?;
        assert!(service.tmpfs().is_none());
        assert!(service.image().is_some());
    }
    assert!(matches!(
        document
            .service("nested-list")
            .and_then(compose_lens::model::Service::tmpfs)
            .map(compose_lens::model::Tmpfs::form),
        Some(TmpfsForm::List(items)) if items.is_empty()
    ));
    assert_tmpfs_diagnostics(&parsed);
    Ok(())
}

fn assert_tmpfs_diagnostics(parsed: &compose_lens::model::ModelParse) {
    use compose_lens::model::{TMPFS_EXPECTED_FORM, TMPFS_EXPECTED_STRING, TMPFS_PROVIDER_DEPENDENT};

    for (code, count) in [
        (TMPFS_EXPECTED_FORM, 4),
        (TMPFS_EXPECTED_STRING, 6),
        (TMPFS_PROVIDER_DEPENDENT, 5),
    ] {
        assert_eq!(
            parsed
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.code() == code)
                .count(),
            count,
            "unexpected diagnostic count for {code}"
        );
    }
}

#[test]
fn keeps_service_tmpfs_distinct_from_long_volume_tmpfs() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(80), SERVICE_TMPFS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let service = parsed
        .document()
        .and_then(|document| document.service("volume-tmpfs-only"))
        .ok_or("volume tmpfs service expected")?;
    assert!(service.tmpfs().is_none());
    assert!(matches!(
        service.volumes(),
        [VolumeMount::Long(mount)] if mount.mount_type().is_some_and(|kind| *kind.value() == MountType::Tmpfs)
    ));
    Ok(())
}

#[test]
fn types_cap_add_as_an_exact_string_sequence_with_loss_aware_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(78), SERVICE_CAP_ADD)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert_eq!(document.services().len(), 10);
    assert!(
        document
            .service("omitted")
            .ok_or("service expected")?
            .cap_add()
            .is_none()
    );

    let empty = document
        .service("empty")
        .and_then(compose_lens::model::Service::cap_add)
        .ok_or("explicit empty cap_add expected")?;
    assert!(empty.items().is_empty());
    assert_eq!(empty.span().source_id(), SourceId::new(78));

    let exact = document
        .service("exact-values")
        .and_then(compose_lens::model::Service::cap_add)
        .ok_or("typed cap_add expected")?;
    assert_eq!(
        exact
            .items()
            .iter()
            .map(compose_lens::model::CapabilityAddItem::value)
            .collect::<Vec<_>>(),
        [
            "NET_ADMIN",
            "NET_ADMIN",
            "net_admin",
            "",
            "CAP WITH SPACE",
            "${ADD_CAP}"
        ]
    );
    assert!(exact.items()[0].is_exact_candidate());
    assert!(exact.items()[2].is_exact_candidate());
    assert!(!exact.items()[3].is_exact_candidate());
    assert!(!exact.items()[4].is_exact_candidate());
    assert!(exact.items()[5].is_exact_candidate());
    assert!(
        exact
            .items()
            .iter()
            .all(|item| item.span().source_id() == SourceId::new(78))
    );
    assert_eq!(
        document
            .service("exact-values")
            .and_then(compose_lens::model::Service::cap_drop)
            .ok_or("coexisting cap_drop expected")?
            .items()[0]
            .value(),
        "MKNOD"
    );

    let recovered = document
        .service("malformed-items")
        .and_then(compose_lens::model::Service::cap_add)
        .ok_or("partially recovered cap_add expected")?;
    assert_eq!(
        recovered
            .items()
            .iter()
            .map(compose_lens::model::CapabilityAddItem::value)
            .collect::<Vec<_>>(),
        ["CHOWN", "SYS_NICE"]
    );
    assert!(
        document
            .service("malformed-items")
            .and_then(compose_lens::model::Service::image)
            .is_some()
    );

    for service in ["scalar", "null", "boolean", "number", "mapping"] {
        let service = document.service(service).ok_or("malformed service expected")?;
        assert!(service.cap_add().is_none());
        assert!(service.image().is_some(), "valid sibling image was lost");
    }
    assert!(
        document
            .service("nested-sequence")
            .and_then(compose_lens::model::Service::cap_add)
            .is_some_and(|capabilities| capabilities.items().is_empty())
    );
    assert_cap_add_diagnostics(parsed.diagnostics());
    Ok(())
}

fn assert_cap_add_diagnostics(diagnostics: &[compose_lens::diagnostic::Diagnostic]) {
    for (code, expected) in [
        (CAP_ADD_DUPLICATE_ITEM, 1),
        (CAP_ADD_EXPECTED_SEQUENCE, 5),
        (CAP_ADD_EXPECTED_STRING, 6),
    ] {
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code() == code)
                .count(),
            expected,
            "unexpected diagnostic count for {code}"
        );
    }
}

#[test]
fn types_cap_drop_as_an_exact_string_sequence_with_loss_aware_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(77), SERVICE_CAP_DROP)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert_eq!(document.services().len(), 10);
    assert!(
        document
            .service("omitted")
            .ok_or("service expected")?
            .cap_drop()
            .is_none()
    );

    let empty = document
        .service("empty")
        .and_then(compose_lens::model::Service::cap_drop)
        .ok_or("explicit empty cap_drop expected")?;
    assert!(empty.items().is_empty());
    assert_eq!(empty.span().source_id(), SourceId::new(77));

    let exact = document
        .service("exact-values")
        .and_then(compose_lens::model::Service::cap_drop)
        .ok_or("typed cap_drop expected")?;
    assert_eq!(
        exact
            .items()
            .iter()
            .map(compose_lens::model::CapabilityDropItem::value)
            .collect::<Vec<_>>(),
        [
            "NET_ADMIN",
            "NET_ADMIN",
            "net_admin",
            "",
            "CAP WITH SPACE",
            "${DROP_CAP}"
        ]
    );
    assert!(exact.items()[0].is_exact_candidate());
    assert!(exact.items()[2].is_exact_candidate());
    assert!(!exact.items()[3].is_exact_candidate());
    assert!(!exact.items()[4].is_exact_candidate());
    assert!(exact.items()[5].is_exact_candidate());
    assert!(
        exact
            .items()
            .iter()
            .all(|item| item.span().source_id() == SourceId::new(77))
    );

    let recovered = document
        .service("malformed-items")
        .and_then(compose_lens::model::Service::cap_drop)
        .ok_or("partially recovered cap_drop expected")?;
    assert_eq!(
        recovered
            .items()
            .iter()
            .map(compose_lens::model::CapabilityDropItem::value)
            .collect::<Vec<_>>(),
        ["CHOWN", "SYS_NICE"]
    );
    assert_eq!(
        document
            .service("malformed-items")
            .and_then(compose_lens::model::Service::image)
            .map(|image| image.value().raw()),
        Some("example.invalid/recovered:1")
    );

    for service in ["scalar", "null", "boolean", "number", "mapping"] {
        let service = document.service(service).ok_or("malformed service expected")?;
        assert!(service.cap_drop().is_none());
        assert!(service.image().is_some(), "valid sibling image was lost");
    }
    assert!(
        document
            .service("nested-sequence")
            .and_then(compose_lens::model::Service::cap_drop)
            .is_some_and(|capabilities| capabilities.items().is_empty())
    );

    assert_cap_drop_diagnostics(parsed.diagnostics());
    Ok(())
}

fn assert_cap_drop_diagnostics(diagnostics: &[compose_lens::diagnostic::Diagnostic]) {
    for (code, expected) in [
        (CAP_DROP_DUPLICATE_ITEM, 1),
        (CAP_DROP_EXPECTED_SEQUENCE, 5),
        (CAP_DROP_EXPECTED_STRING, 6),
    ] {
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code() == code)
                .count(),
            expected,
            "unexpected diagnostic count for {code}"
        );
    }
}

#[test]
fn classifies_service_hostnames_and_rejects_non_string_yaml_shapes_without_erasing_services()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{DUPLICATE_FIELD, HOSTNAME_EXPECTED_STRING, HOSTNAME_INVALID};

    let syntax = SyntaxDocument::parse(SourceId::new(73), SERVICE_HOSTNAME)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(
        document
            .service("omitted")
            .ok_or("omitted service expected")?
            .hostname()
            .is_none()
    );
    for (service, expected) in [
        ("lowercase", "api-1.example"),
        ("uppercase", "API.Example-Corp.COM"),
        ("digit-leading", "3api.example"),
        ("quoted-number", "123"),
    ] {
        let hostname = document
            .service(service)
            .and_then(compose_lens::model::Service::hostname)
            .ok_or("resolved hostname expected")?;
        assert_eq!(hostname.raw().value(), expected);
        assert_eq!(hostname.kind(), &HostnameKind::Resolved);
        assert!(hostname.is_resolved());
        assert_eq!(
            SERVICE_HOSTNAME[hostname.raw().span().range()].trim_matches('"'),
            expected
        );
    }
    for service in ["deferred", "dollar-literal"] {
        assert_eq!(
            document
                .service(service)
                .and_then(compose_lens::model::Service::hostname)
                .map(compose_lens::model::Hostname::kind),
            Some(&HostnameKind::Expression)
        );
    }
    for service in [
        "empty",
        "trailing-dot",
        "empty-label",
        "underscore",
        "leading-hyphen",
        "trailing-hyphen",
        "non-ascii",
        "overlong-label",
    ] {
        assert_eq!(
            document
                .service(service)
                .and_then(compose_lens::model::Service::hostname)
                .map(compose_lens::model::Hostname::kind),
            Some(&HostnameKind::Invalid),
            "invalid hostname should remain inspectable for {service}"
        );
    }
    for service in ["null", "boolean", "numeric", "mapping", "flow-list", "sequence"] {
        assert!(
            document
                .service(service)
                .ok_or("malformed service expected")?
                .hostname()
                .is_none()
        );
    }
    assert_eq!(document.services().len(), 22);
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == HOSTNAME_INVALID)
            .count(),
        8
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == HOSTNAME_EXPECTED_STRING)
            .count(),
        6
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    assert_eq!(
        document
            .service("duplicate")
            .and_then(compose_lens::model::Service::hostname)
            .map(|hostname| hostname.raw().value().as_str()),
        Some("first.example")
    );
    Ok(())
}

#[test]
fn classifies_service_pids_limits_without_overflow_or_erasing_malformed_services()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{
        DUPLICATE_FIELD, PIDS_LIMIT_AMBIGUOUS_ZERO, PIDS_LIMIT_EXPECTED_VALUE, PIDS_LIMIT_INVALID,
    };

    let syntax = SyntaxDocument::parse(SourceId::new(72), SERVICE_PIDS_LIMIT)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;
    assert_pids_limit_classifications(document)?;
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == PIDS_LIMIT_INVALID)
            .count(),
        5
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == PIDS_LIMIT_AMBIGUOUS_ZERO)
            .count(),
        2
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == PIDS_LIMIT_EXPECTED_VALUE)
            .count(),
        5
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    assert_eq!(document.services().len(), 20);
    Ok(())
}

fn assert_pids_limit_classifications(document: &ComposeDocument) -> Result<(), &'static str> {
    use compose_lens::model::PidsLimitKind;

    assert!(
        document
            .service("omitted")
            .ok_or("omitted service expected")?
            .pids_limit()
            .is_none()
    );
    for service in ["unlimited-number", "unlimited-string"] {
        let limit = document
            .service(service)
            .and_then(compose_lens::model::Service::pids_limit)
            .ok_or("unlimited PID limit expected")?;
        assert_eq!(limit.raw().value(), "-1");
        assert_eq!(limit.kind(), &PidsLimitKind::Unlimited);
    }
    for (service, decimal) in [
        ("finite-number", "64"),
        ("finite-string", "00064"),
        ("overflow", "18446744073709551616000000000000000000000000000000"),
    ] {
        let limit = document
            .service(service)
            .and_then(compose_lens::model::Service::pids_limit)
            .ok_or("finite PID limit expected")?;
        assert_eq!(limit.raw().value(), decimal);
        assert_eq!(
            limit.kind(),
            &PidsLimitKind::Finite {
                decimal: decimal.to_owned()
            }
        );
    }
    let quoted = document
        .service("finite-string")
        .and_then(compose_lens::model::Service::pids_limit)
        .ok_or("quoted finite PID limit expected")?;
    assert_eq!(&SERVICE_PIDS_LIMIT[quoted.raw().span().range()], "\"00064\"");
    for service in ["zero-number", "zero-string"] {
        assert!(matches!(
            document
                .service(service)
                .and_then(compose_lens::model::Service::pids_limit)
                .map(compose_lens::model::PidsLimit::kind),
            Some(PidsLimitKind::Zero)
        ));
    }
    assert!(matches!(
        document
            .service("deferred")
            .and_then(compose_lens::model::Service::pids_limit)
            .map(compose_lens::model::PidsLimit::kind),
        Some(PidsLimitKind::Expression)
    ));
    for service in [
        "fraction-number",
        "fraction-string",
        "exponent",
        "negative",
        "arbitrary",
    ] {
        assert!(matches!(
            document
                .service(service)
                .and_then(compose_lens::model::Service::pids_limit)
                .map(compose_lens::model::PidsLimit::kind),
            Some(PidsLimitKind::Other)
        ));
    }
    for service in ["boolean", "null", "map", "flow-list", "sequence"] {
        assert!(
            document
                .service(service)
                .ok_or("malformed service expected")?
                .pids_limit()
                .is_none()
        );
    }
    assert_eq!(
        document
            .service("duplicate")
            .and_then(compose_lens::model::Service::pids_limit)
            .map(|limit| limit.raw().value().as_str()),
        Some("64")
    );
    Ok(())
}

#[test]
fn classifies_service_shm_sizes_without_normalization_or_erasing_malformed_services()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{ShmSizeKind, ShmSizeScalarKind, ShmSizeUnit};

    let syntax = SyntaxDocument::parse(SourceId::new(73), SERVICE_SHM_SIZE)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;
    assert!(
        document
            .service("omitted")
            .ok_or("omitted expected")?
            .shm_size()
            .is_none()
    );

    for (service, amount, unit) in [
        ("bytes", "1", ShmSizeUnit::B),
        ("kilo-short", "2", ShmSizeUnit::K),
        ("kilo-long", "3", ShmSizeUnit::Kb),
        ("mega-short", "4", ShmSizeUnit::M),
        ("mega-long", "5", ShmSizeUnit::Mb),
        ("giga-short", "6", ShmSizeUnit::G),
        ("giga-long", "7", ShmSizeUnit::Gb),
        ("leading-zero-amount", "00064", ShmSizeUnit::Mb),
        ("signed-amount", "+64", ShmSizeUnit::Mb),
        ("fractional-amount", "1.5", ShmSizeUnit::Mb),
        ("arbitrary-amount", "huge", ShmSizeUnit::Mb),
    ] {
        let size = document
            .service(service)
            .and_then(compose_lens::model::Service::shm_size)
            .ok_or("documented shared-memory size expected")?;
        assert_eq!(size.scalar_kind(), ShmSizeScalarKind::String);
        assert!(matches!(
            size.kind(),
            ShmSizeKind::Documented { amount_raw, unit: actual_unit }
                if amount_raw == amount && *actual_unit == unit
        ));
        assert!(SERVICE_SHM_SIZE[size.raw().span().range()].contains(size.raw().value()));
    }

    for service in ["zero-number", "zero-string", "zero-unit"] {
        assert!(matches!(
            document
                .service(service)
                .and_then(compose_lens::model::Service::shm_size)
                .map(compose_lens::model::ShmSize::kind),
            Some(ShmSizeKind::Zero { .. })
        ));
    }
    assert_eq!(
        document
            .service("deferred")
            .and_then(compose_lens::model::Service::shm_size)
            .map(compose_lens::model::ShmSize::kind),
        Some(&ShmSizeKind::Expression)
    );
    for service in ["numeric-integer", "numeric-fraction", "numeric-exponent"] {
        let size = document
            .service(service)
            .and_then(compose_lens::model::Service::shm_size)
            .ok_or("numeric shared-memory size expected")?;
        assert_eq!(size.scalar_kind(), ShmSizeScalarKind::Number);
        assert_eq!(size.kind(), &ShmSizeKind::ProviderDependentNumber);
    }
    for service in ["string-bare", "uppercase-unit", "iec-unit", "whitespace"] {
        let size = document
            .service(service)
            .and_then(compose_lens::model::Service::shm_size)
            .ok_or("string shared-memory size expected")?;
        assert_eq!(size.scalar_kind(), ShmSizeScalarKind::String);
        assert_eq!(size.kind(), &ShmSizeKind::ProviderDependentString);
    }
    for service in ["boolean", "null", "map", "flow-list", "sequence"] {
        assert!(
            document
                .service(service)
                .ok_or("malformed service expected")?
                .shm_size()
                .is_none()
        );
    }
    assert_eq!(document.services().len(), 29);
    assert_shm_size_diagnostics(parsed.diagnostics());
    assert_eq!(
        document
            .service("duplicate")
            .and_then(compose_lens::model::Service::shm_size)
            .map(|size| size.raw().value().as_str()),
        Some("64m")
    );
    Ok(())
}

fn assert_shm_size_diagnostics(diagnostics: &[compose_lens::diagnostic::Diagnostic]) {
    use compose_lens::model::{
        DUPLICATE_FIELD, SHM_SIZE_AMBIGUOUS_ZERO, SHM_SIZE_EXPECTED_VALUE, SHM_SIZE_PROVIDER_DEPENDENT_NUMBER,
        SHM_SIZE_PROVIDER_DEPENDENT_STRING,
    };

    for (code, expected) in [
        (SHM_SIZE_AMBIGUOUS_ZERO, 3),
        (SHM_SIZE_PROVIDER_DEPENDENT_NUMBER, 3),
        (SHM_SIZE_PROVIDER_DEPENDENT_STRING, 4),
        (SHM_SIZE_EXPECTED_VALUE, 5),
        (DUPLICATE_FIELD, 1),
    ] {
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code() == code)
                .count(),
            expected,
            "unexpected diagnostic count for {code}"
        );
    }
}

#[test]
fn classifies_service_mem_limits_without_normalization_or_erasing_malformed_services()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{MemLimitKind, MemLimitScalarKind, MemLimitUnit};

    let syntax = SyntaxDocument::parse(SourceId::new(76), SERVICE_MEM_LIMIT)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;
    assert!(
        document
            .service("omitted")
            .ok_or("omitted expected")?
            .mem_limit()
            .is_none()
    );

    for (service, amount, unit) in [
        ("bytes", "1", MemLimitUnit::B),
        ("kilo-short", "2", MemLimitUnit::K),
        ("kilo-long", "3", MemLimitUnit::Kb),
        ("mega-short", "4", MemLimitUnit::M),
        ("mega-long", "5", MemLimitUnit::Mb),
        ("giga-short", "6", MemLimitUnit::G),
        ("giga-long", "7", MemLimitUnit::Gb),
        ("leading-zero-amount", "00064", MemLimitUnit::Mb),
        ("signed-amount", "+64", MemLimitUnit::Mb),
        ("fractional-amount", "1.5", MemLimitUnit::Mb),
        ("arbitrary-amount", "huge", MemLimitUnit::Mb),
    ] {
        let limit = document
            .service(service)
            .and_then(compose_lens::model::Service::mem_limit)
            .ok_or("documented memory limit expected")?;
        assert_eq!(limit.scalar_kind(), MemLimitScalarKind::String);
        assert!(matches!(
            limit.kind(),
            MemLimitKind::Documented { amount_raw, unit: actual_unit }
                if amount_raw == amount && *actual_unit == unit
        ));
        assert!(SERVICE_MEM_LIMIT[limit.raw().span().range()].contains(limit.raw().value()));
    }
    for service in ["zero-number", "zero-string", "zero-unit"] {
        assert!(matches!(
            document
                .service(service)
                .and_then(compose_lens::model::Service::mem_limit)
                .map(compose_lens::model::MemLimit::kind),
            Some(MemLimitKind::Zero { .. })
        ));
    }
    assert_eq!(
        document
            .service("deferred")
            .and_then(compose_lens::model::Service::mem_limit)
            .map(compose_lens::model::MemLimit::kind),
        Some(&MemLimitKind::Expression)
    );
    for service in ["numeric-integer", "numeric-fraction", "numeric-exponent"] {
        let limit = document
            .service(service)
            .and_then(compose_lens::model::Service::mem_limit)
            .ok_or("numeric memory limit expected")?;
        assert_eq!(limit.scalar_kind(), MemLimitScalarKind::Number);
        assert_eq!(limit.kind(), &MemLimitKind::SchemaNumber);
    }
    for service in ["string-bare", "uppercase-unit", "iec-unit", "whitespace"] {
        let limit = document
            .service(service)
            .and_then(compose_lens::model::Service::mem_limit)
            .ok_or("provider-dependent memory limit expected")?;
        assert_eq!(limit.scalar_kind(), MemLimitScalarKind::String);
        assert_eq!(limit.kind(), &MemLimitKind::ProviderDependentString);
    }
    for service in ["boolean", "null", "map", "flow-list", "sequence"] {
        assert!(
            document
                .service(service)
                .ok_or("malformed service expected")?
                .mem_limit()
                .is_none()
        );
    }
    assert_eq!(document.services().len(), 29);
    assert_mem_limit_diagnostics(parsed.diagnostics());
    assert_eq!(
        document
            .service("duplicate")
            .and_then(compose_lens::model::Service::mem_limit)
            .map(|limit| limit.raw().value().as_str()),
        Some("64m")
    );
    Ok(())
}

fn assert_mem_limit_diagnostics(diagnostics: &[compose_lens::diagnostic::Diagnostic]) {
    use compose_lens::model::{
        DUPLICATE_FIELD, MEM_LIMIT_AMBIGUOUS_ZERO, MEM_LIMIT_EXPECTED_VALUE, MEM_LIMIT_PROVIDER_DEPENDENT_STRING,
        MEM_LIMIT_SCHEMA_NUMBER,
    };

    for (code, expected) in [
        (MEM_LIMIT_AMBIGUOUS_ZERO, 3),
        (MEM_LIMIT_SCHEMA_NUMBER, 3),
        (MEM_LIMIT_PROVIDER_DEPENDENT_STRING, 4),
        (MEM_LIMIT_EXPECTED_VALUE, 5),
        (DUPLICATE_FIELD, 1),
    ] {
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code() == code)
                .count(),
            expected,
            "unexpected diagnostic count for {code}"
        );
    }
}

#[test]
fn classifies_service_pull_policies_without_erasing_malformed_services() -> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{DUPLICATE_FIELD, PULL_POLICY_INVALID, PullPolicyKind};

    let syntax = SyntaxDocument::parse(SourceId::new(71), SERVICE_PULL_POLICY)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;

    assert!(
        document
            .service("omitted")
            .ok_or("omitted service expected")?
            .pull_policy()
            .is_none()
    );
    for (service, expected) in [
        ("always", PullPolicyKind::Always),
        ("never", PullPolicyKind::Never),
        ("missing", PullPolicyKind::Missing),
        ("alias", PullPolicyKind::IfNotPresentAlias),
        ("build", PullPolicyKind::Build),
        ("daily", PullPolicyKind::Daily),
        ("weekly", PullPolicyKind::Weekly),
        ("deferred", PullPolicyKind::Expression),
        ("schema-refresh", PullPolicyKind::RefreshSchemaOnly),
        ("invalid-every", PullPolicyKind::Other),
        ("invalid-microseconds", PullPolicyKind::Other),
        ("invalid-milliseconds", PullPolicyKind::Other),
        ("invalid-fraction", PullPolicyKind::Other),
        ("invalid-dangling", PullPolicyKind::Other),
        ("provider-specific", PullPolicyKind::Other),
    ] {
        let policy = document
            .service(service)
            .and_then(compose_lens::model::Service::pull_policy)
            .ok_or("retained pull policy expected")?;
        assert_eq!(policy.kind(), &expected, "unexpected classification for {service}");
    }
    let every = document
        .service("every")
        .and_then(compose_lens::model::Service::pull_policy)
        .ok_or("custom interval expected")?;
    assert_eq!(every.raw().value(), "every_1h30m");
    assert_eq!(
        every.kind(),
        &PullPolicyKind::Every {
            duration: "1h30m".to_owned(),
        }
    );
    for (service, duration) in [("every-week", "1w"), ("every-day", "2d"), ("every-zero", "0s")] {
        let policy = document
            .service(service)
            .and_then(compose_lens::model::Service::pull_policy)
            .ok_or("additional custom interval expected")?;
        assert_eq!(policy.raw().value(), &format!("every_{duration}"));
        assert_eq!(
            policy.kind(),
            &PullPolicyKind::Every {
                duration: duration.to_owned(),
            }
        );
    }
    let refresh_after = document
        .service("schema-refresh")
        .and_then(compose_lens::model::Service::pull_refresh_after)
        .ok_or("pull refresh interval expected")?;
    assert_eq!(refresh_after.value(), "12h");
    assert_eq!(refresh_after.span().source_id(), SourceId::new(71));
    for service in ["null-value", "map-value", "list-value"] {
        let service = document.service(service).ok_or("malformed service expected")?;
        assert!(service.pull_policy().is_none());
    }
    let duplicate = document.service("duplicate").ok_or("duplicate service expected")?;
    assert_eq!(
        duplicate.pull_policy().map(|policy| policy.raw().value().as_str()),
        Some("always")
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == PULL_POLICY_INVALID)
            .count(),
        6
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
            .count()
            >= 3
    );
    assert_eq!(document.services().len(), 24);
    Ok(())
}

#[test]
fn retains_an_explicit_container_name_as_a_source_aware_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  web:\n    container_name: application-web\n    image: example.invalid/web:1\n";
    let syntax = SyntaxDocument::parse(SourceId::new(31), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let service = parsed
        .document()
        .and_then(|document| document.service("web"))
        .ok_or("web service expected")?;

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(
        service.container_name().map(|name| name.value().as_str()),
        Some("application-web")
    );
    assert!(service.unknown_fields().is_empty());
    Ok(())
}

#[test]
fn reports_a_non_scalar_container_name_without_losing_the_service() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  web:\n    container_name:\n      invalid: mapping\n    image: example.invalid/web:1\n";
    let syntax = SyntaxDocument::parse(SourceId::new(32), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let service = parsed
        .document()
        .and_then(|document| document.service("web"))
        .ok_or("partial web service expected")?;

    assert!(!parsed.is_valid());
    assert!(service.container_name().is_none());
    assert!(parsed.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == EXPECTED_SCALAR
            && diagnostic
                .labels()
                .iter()
                .all(|label| label.span().source_id() == SourceId::new(32))
    }));
    Ok(())
}

#[test]
fn retains_short_and_long_environment_files_with_source_options() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  scalar:\n",
        "    env_file: scalar.env\n",
        "  list:\n",
        "    env_file:\n",
        "      - base.env\n",
        "      - path: optional.env\n",
        "        required: false\n",
        "        x-owner: application\n",
        "      - path: raw.env\n",
        "        format: raw\n",
        "        vendor-option: retained\n",
        "      - path: expression.env\n",
        "        required: ${ENV_FILE_REQUIRED:-true}\n",
        "        format: ${ENV_FILE_FORMAT:-raw}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(34), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let scalar = document.service("scalar").ok_or("scalar service expected")?;
    let list = document.service("list").ok_or("list service expected")?;

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert!(matches!(
        scalar.environment_files(),
        [EnvironmentFile::Short(path)] if path.value() == "scalar.env"
    ));
    assert_eq!(list.environment_files().len(), 4);
    assert!(matches!(
        &list.environment_files()[0],
        EnvironmentFile::Short(path) if path.value() == "base.env"
    ));
    let EnvironmentFile::Long(optional) = &list.environment_files()[1] else {
        return Err("optional long environment file expected".into());
    };
    assert_eq!(
        optional.path().map(Located::value).map(String::as_str),
        Some("optional.env")
    );
    assert_eq!(
        optional.required().map(Located::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(optional.extension_fields().len(), 1);
    let EnvironmentFile::Long(raw) = &list.environment_files()[2] else {
        return Err("raw long environment file expected".into());
    };
    assert_eq!(
        raw.format().map(compose_lens::model::EnvironmentFileFormat::kind),
        Some(EnvironmentFileFormatKind::Raw)
    );
    assert_eq!(raw.unknown_fields().len(), 1);
    let EnvironmentFile::Long(expression) = &list.environment_files()[3] else {
        return Err("expression long environment file expected".into());
    };
    assert!(matches!(
        expression.required().map(Located::value),
        Some(BooleanValue::Expression(value)) if value == "${ENV_FILE_REQUIRED:-true}"
    ));
    assert_eq!(
        expression
            .format()
            .map(compose_lens::model::EnvironmentFileFormat::kind),
        Some(EnvironmentFileFormatKind::Expression)
    );
    Ok(())
}

#[test]
fn reports_malformed_environment_files_without_erasing_valid_items() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    env_file:\n",
        "      - good.env\n",
        "      - required: true\n",
        "      - path: unsupported.env\n",
        "        format: dotenv\n",
        "      - [invalid]\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(35), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let service = parsed
        .document()
        .and_then(|document| document.service("app"))
        .ok_or("partial app service expected")?;

    assert!(!parsed.is_valid());
    assert_eq!(service.environment_files().len(), 3);
    for code in [
        ENVIRONMENT_FILE_MISSING_PATH,
        ENVIRONMENT_FILE_INVALID_FORMAT,
        ENVIRONMENT_FILE_EXPECTED_FORM,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn classifies_restart_policies_without_losing_authored_spelling() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  disabled:\n",
        "    restart: \"no\"\n",
        "  always:\n",
        "    restart: always\n",
        "  failure:\n",
        "    restart: on-failure\n",
        "  limited:\n",
        "    restart: on-failure:003\n",
        "  stopped:\n",
        "    restart: unless-stopped\n",
        "  deferred:\n",
        "    restart: ${RESTART_POLICY:-always}\n",
        "  invalid:\n",
        "    restart: sometimes\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(33), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(!parsed.is_valid());
    assert!(matches!(
        document
            .service("disabled")
            .and_then(compose_lens::model::Service::restart)
            .map(compose_lens::model::RestartPolicy::kind),
        Some(RestartPolicyKind::No)
    ));
    assert!(matches!(
        document
            .service("always")
            .and_then(compose_lens::model::Service::restart)
            .map(compose_lens::model::RestartPolicy::kind),
        Some(RestartPolicyKind::Always)
    ));
    assert!(matches!(
        document
            .service("failure")
            .and_then(compose_lens::model::Service::restart)
            .map(compose_lens::model::RestartPolicy::kind),
        Some(RestartPolicyKind::OnFailure { maximum_retries: None })
    ));
    let limited = document
        .service("limited")
        .and_then(compose_lens::model::Service::restart)
        .ok_or("limited restart policy expected")?;
    assert_eq!(limited.raw().value(), "on-failure:003");
    assert!(matches!(
        limited.kind(),
        RestartPolicyKind::OnFailure { maximum_retries: Some(value) } if value == "003"
    ));
    assert!(matches!(
        document
            .service("stopped")
            .and_then(compose_lens::model::Service::restart)
            .map(compose_lens::model::RestartPolicy::kind),
        Some(RestartPolicyKind::UnlessStopped)
    ));
    assert!(matches!(
        document
            .service("deferred")
            .and_then(compose_lens::model::Service::restart)
            .map(compose_lens::model::RestartPolicy::kind),
        Some(RestartPolicyKind::Expression)
    ));
    assert!(matches!(
        document
            .service("invalid")
            .and_then(compose_lens::model::Service::restart)
            .map(compose_lens::model::RestartPolicy::kind),
        Some(RestartPolicyKind::Other)
    ));
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == RESTART_INVALID_POLICY)
    );
    assert!(
        document
            .services()
            .iter()
            .all(|service| service.unknown_fields().is_empty())
    );
    Ok(())
}

#[test]
fn types_independent_stop_signal_and_grace_period_values_without_normalizing_them()
-> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  omitted: {}\n",
        "  empty:\n",
        "    stop_signal: \"\"\n",
        "  named:\n",
        "    stop_signal: SIGUSR1\n",
        "    stop_grace_period: 1s\n",
        "  numeric:\n",
        "    stop_signal: 15\n",
        "    stop_grace_period: 1m30s\n",
        "  zero:\n",
        "    stop_grace_period: 0s\n",
        "  fractional:\n",
        "    stop_grace_period: 1.5s\n",
        "  deferred:\n",
        "    stop_grace_period: ${STOP_GRACE_PERIOD:-1s}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(36), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    let omitted = document.service("omitted").ok_or("omitted service expected")?;
    assert!(omitted.stop_signal().is_none());
    assert!(omitted.stop_grace_period().is_none());
    assert_eq!(
        document
            .service("empty")
            .and_then(compose_lens::model::Service::stop_signal)
            .map(Located::value)
            .map(String::as_str),
        Some("")
    );
    let named = document.service("named").ok_or("named service expected")?;
    assert_eq!(
        named.stop_signal().map(Located::value).map(String::as_str),
        Some("SIGUSR1")
    );
    assert!(matches!(
        named.stop_grace_period().map(Located::value),
        Some(StopGracePeriod::Value(value)) if value == "1s"
    ));
    let numeric = document.service("numeric").ok_or("numeric service expected")?;
    assert_eq!(
        numeric.stop_signal().map(Located::value).map(String::as_str),
        Some("15")
    );
    assert!(matches!(
        numeric.stop_grace_period().map(Located::value),
        Some(StopGracePeriod::Value(value)) if value == "1m30s"
    ));
    assert!(matches!(
        document
            .service("zero")
            .and_then(compose_lens::model::Service::stop_grace_period)
            .map(Located::value),
        Some(StopGracePeriod::Value(value)) if value == "0s"
    ));
    assert!(matches!(
        document
            .service("fractional")
            .and_then(compose_lens::model::Service::stop_grace_period)
            .map(Located::value),
        Some(StopGracePeriod::Value(value)) if value == "1.5s"
    ));
    assert!(matches!(
        document
            .service("deferred")
            .and_then(compose_lens::model::Service::stop_grace_period)
            .map(Located::value),
        Some(StopGracePeriod::Expression(value)) if value == "${STOP_GRACE_PERIOD:-1s}"
    ));
    assert!(
        document
            .services()
            .iter()
            .all(|service| service.unknown_fields().is_empty())
    );
    Ok(())
}

#[test]
fn reports_malformed_stop_lifecycle_values_without_dropping_invalid_scalars() -> Result<(), Box<dyn std::error::Error>>
{
    let source = concat!(
        "services:\n",
        "  nanoseconds:\n",
        "    stop_grace_period: 1ns\n",
        "  unicode-microseconds:\n",
        "    stop_grace_period: 1µs\n",
        "  greek-microseconds:\n",
        "    stop_grace_period: 1μs\n",
        "  incomplete-fraction:\n",
        "    stop_grace_period: 1.s\n",
        "  malformed:\n",
        "    stop_grace_period: 1fortnight\n",
        "  duplicate:\n",
        "    stop_signal: SIGTERM\n",
        "    stop_signal: SIGKILL\n",
        "  wrong-shape:\n",
        "    stop_signal: [SIGTERM]\n",
        "    stop_grace_period: [1s]\n",
        "  explicit-null:\n",
        "    stop_signal: null\n",
        "    stop_grace_period:\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(37), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(!parsed.is_valid());
    for (service, expected) in [
        ("nanoseconds", "1ns"),
        ("unicode-microseconds", "1µs"),
        ("greek-microseconds", "1μs"),
        ("incomplete-fraction", "1.s"),
        ("malformed", "1fortnight"),
    ] {
        assert!(matches!(
            document
                .service(service)
                .and_then(compose_lens::model::Service::stop_grace_period)
                .map(Located::value),
            Some(StopGracePeriod::Other(value)) if value == expected
        ));
    }
    assert_eq!(
        document
            .service("duplicate")
            .and_then(compose_lens::model::Service::stop_signal)
            .map(Located::value)
            .map(String::as_str),
        Some("SIGTERM")
    );
    for service in ["wrong-shape", "explicit-null"] {
        let service = document.service(service).ok_or("malformed service expected")?;
        assert!(service.stop_signal().is_none());
        assert!(service.stop_grace_period().is_none());
    }
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == STOP_GRACE_PERIOD_INVALID)
            .count(),
        5
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DUPLICATE_FIELD)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
            .count()
            >= 4
    );
    Ok(())
}

#[test]
fn retains_service_label_mapping_and_sequence_forms() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(29), SERVICE_LABEL_FORMS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let mapping = document.service("mapping").ok_or("mapping service expected")?;
    let sequence = document.service("sequence").ok_or("sequence service expected")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    let Some(Labels::Map { entries, .. }) = mapping.labels() else {
        return Err("mapping service labels expected".into());
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].key().value(), "com.example.description");
    assert_eq!(
        entries[0].value().value(),
        &ComposeScalar::String("mapping form".to_owned())
    );
    assert_eq!(entries[1].value().value(), &ComposeScalar::String(String::new()));

    let Some(Labels::List { values, .. }) = sequence.labels() else {
        return Err("sequence service labels expected".into());
    };
    assert_eq!(
        values.iter().map(|value| value.value().as_str()).collect::<Vec<_>>(),
        [
            "com.example.description=sequence form",
            "com.example.empty",
            "com.example.equals=left=right",
        ]
    );
    assert!(mapping.unknown_fields().is_empty());
    assert!(sequence.unknown_fields().is_empty());
    Ok(())
}

#[test]
fn reports_invalid_service_label_forms_without_losing_services() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(28), INVALID_SERVICE_LABEL_FORMS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(!parsed.is_valid());
    assert_eq!(document.services().len(), 3);
    assert!(parsed.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == EXPECTED_FIELD_FORM
            && diagnostic
                .labels()
                .iter()
                .all(|label| label.span().source_id() == SourceId::new(28))
    }));
    assert!(parsed.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == EXPECTED_SCALAR
            && diagnostic
                .labels()
                .iter()
                .all(|label| label.span().source_id() == SourceId::new(28))
    }));
    Ok(())
}

#[test]
fn retains_network_label_mapping_and_sequence_forms() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(31), NETWORK_LABEL_FORMS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let mapping = document
        .networks()
        .iter()
        .find(|network| network.name().value() == "mapping")
        .ok_or("mapping network expected")?;
    let sequence = document
        .networks()
        .iter()
        .find(|network| network.name().value() == "sequence")
        .ok_or("sequence network expected")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    let Some(Labels::Map { entries, .. }) = mapping.labels() else {
        return Err("mapping network labels expected".into());
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].key().value(), "com.example.empty");
    assert_eq!(entries[0].value().value(), &ComposeScalar::String(String::new()));
    assert_eq!(
        entries[1].value().value(),
        &ComposeScalar::String("left=right".to_owned())
    );

    let Some(Labels::List { values, .. }) = sequence.labels() else {
        return Err("sequence network labels expected".into());
    };
    assert_eq!(
        values.iter().map(|value| value.value().as_str()).collect::<Vec<_>>(),
        [
            "com.example.value=sequence",
            "com.example.empty",
            "com.example.equals=left=right",
        ]
    );
    Ok(())
}

#[test]
fn reports_invalid_network_label_forms_without_losing_networks() -> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::DUPLICATE_FIELD;

    let syntax = SyntaxDocument::parse(SourceId::new(32), INVALID_NETWORK_LABEL_FORMS)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(!parsed.is_valid());
    assert_eq!(document.networks().len(), 5);
    for code in [EXPECTED_FIELD_FORM, EXPECTED_SCALAR, DUPLICATE_FIELD] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
            .count(),
        4
    );
    assert!(
        document
            .networks()
            .iter()
            .find(|network| network.name().value() == "invalid-list-item")
            .is_some_and(|network| matches!(network.labels(), Some(Labels::List { values, .. }) if values.is_empty()))
    );
    let empty_key = document
        .networks()
        .iter()
        .find(|network| network.name().value() == "empty-map-key")
        .and_then(compose_lens::model::NetworkDefinition::labels)
        .ok_or("empty-key network labels expected")?;
    assert!(
        matches!(empty_key, Labels::Map { entries, .. } if entries.len() == 1 && entries[0].key().value().is_empty())
    );
    Ok(())
}

#[test]
fn types_a_complete_unquoted_short_volume_with_comma_options() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(30), COMMA_PLAIN_SCALAR)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let service = parsed
        .document()
        .and_then(|document| document.service("app"))
        .ok_or("app service is missing")?;
    let VolumeMount::Short(volume) = &service.volumes()[0] else {
        return Err("short volume expected".into());
    };

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(volume.raw().value(), "./data:/data:Z,ro");
    assert_eq!(volume.source(), Some("./data"));
    assert_eq!(volume.target(), Some("/data"));
    assert_eq!(volume.options(), &["Z".to_owned(), "ro".to_owned()]);
    assert_eq!(volume.selinux_relabel(), Some(SelinuxRelabel::Private));
    Ok(())
}

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
    assert_eq!(
        long.consistency().map(Located::value).map(String::as_str),
        Some("delegated")
    );
    assert_eq!(long.extension_fields().len(), 1);
    assert_eq!(long.unknown_fields().len(), 0);

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
    let exec_entrypoint = document
        .service("exec-entrypoint")
        .ok_or("exec-entrypoint service is missing")?;

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
    assert!(matches!(app.entrypoint(), Some(Entrypoint::Null(_))));
    assert_eq!(app.init().map(Located::value), Some(&BooleanValue::Literal(true)));

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
    assert!(matches!(worker.entrypoint(), Some(Entrypoint::List { values, .. }) if values.is_empty()));
    assert!(matches!(
        worker.init().map(Located::value),
        Some(BooleanValue::Expression(value)) if value == "${USE_INIT:-false}"
    ));
    assert!(matches!(shell.command(), Some(Command::String(value)) if value.value().is_empty()));
    assert!(matches!(shell.entrypoint(), Some(Entrypoint::String(value)) if value.value().is_empty()));
    assert_eq!(shell.init().map(Located::value), Some(&BooleanValue::Literal(false)));
    assert!(matches!(
        exec_entrypoint.entrypoint(),
        Some(Entrypoint::List { values, .. })
            if values.iter().map(Located::value).map(String::as_str).eq(["/usr/bin/env", "php"])
    ));
    assert!(exec_entrypoint.init().is_none());
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
    assert_eq!(
        network.internal().map(Located::value),
        Some(&BooleanValue::Expression("${INTERNAL:-false}".to_owned()))
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
fn retains_volume_driver_option_scalar_kinds_and_external_driver_conflicts() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    image: example.invalid/app\n",
        "volumes:\n",
        "  implicit:\n",
        "  scalar-kinds:\n",
        "    driver: opaque\n",
        "    driver_opts:\n",
        "      string: \"2\"\n",
        "      number: 2\n",
        "      boolean: true\n",
        "      null:\n",
        "  external-driver:\n",
        "    external: true\n",
        "    driver: opaque\n",
        "    driver_opts: {o: bind}\n",
        "  malformed-external:\n",
        "    external:\n",
        "    driver_opts: {boolean: false, null: null}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(821), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let scalar_kinds = document
        .volumes()
        .iter()
        .find(|volume| volume.name().value() == "scalar-kinds")
        .ok_or("scalar kinds volume expected")?;
    assert!(matches!(scalar_kinds.driver_opts()[0].value().value(), ComposeScalar::String(value) if value == "2"));
    assert!(matches!(scalar_kinds.driver_opts()[1].value().value(), ComposeScalar::Number(value) if value == "2"));
    assert!(matches!(
        scalar_kinds.driver_opts()[2].value().value(),
        ComposeScalar::Boolean(true)
    ));
    assert!(matches!(
        scalar_kinds.driver_opts()[3].value().value(),
        ComposeScalar::Null
    ));
    assert!(
        document
            .volumes()
            .iter()
            .any(|volume| volume.name().value() == "implicit")
    );
    let external = document
        .volumes()
        .iter()
        .find(|volume| volume.name().value() == "external-driver")
        .ok_or("external driver volume expected")?;
    assert_eq!(external.driver().map(|driver| driver.value().as_str()), Some("opaque"));
    assert_eq!(external.driver_opts().len(), 1);
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == VOLUME_EXTERNAL_DRIVER_CONFIGURATION)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_FIELD_FORM)
    );
    Ok(())
}

#[test]
fn retains_volume_label_forms_and_diagnoses_literal_external_labels_without_discarding_them()
-> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "volumes:\n",
        "  mapping:\n",
        "    labels:\n",
        "      com.example.empty: \"\"\n",
        "      com.example.equals: left=right\n",
        "  sequence:\n",
        "    labels: [com.example.value=sequence, com.example.key-only]\n",
        "  external-empty-map:\n",
        "    external: true\n",
        "    labels: {}\n",
        "  external-empty-list:\n",
        "    external: true\n",
        "    labels: []\n",
        "  external-both:\n",
        "    external: true\n",
        "    driver: opaque\n",
        "    labels: {retained: value}\n",
        "  deferred-external:\n",
        "    external: \"${EXTERNAL}\"\n",
        "    labels: {retained: value}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(824), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    let mapping = document
        .volumes()
        .iter()
        .find(|volume| volume.name().value() == "mapping")
        .and_then(compose_lens::model::VolumeDefinition::labels)
        .ok_or("mapping volume labels expected")?;
    assert!(matches!(
        mapping,
        Labels::Map { entries, .. }
            if entries.len() == 2
                && entries[0].key().value() == "com.example.empty"
                && matches!(entries[0].value().value(), ComposeScalar::String(value) if value.is_empty())
                && entries[1].key().value() == "com.example.equals"
                && matches!(entries[1].value().value(), ComposeScalar::String(value) if value == "left=right")
    ));
    let sequence = document
        .volumes()
        .iter()
        .find(|volume| volume.name().value() == "sequence")
        .and_then(compose_lens::model::VolumeDefinition::labels)
        .ok_or("sequence volume labels expected")?;
    assert!(matches!(
        sequence,
        Labels::List { values, .. }
            if values.iter().map(|value| value.value().as_str()).collect::<Vec<_>>()
                == ["com.example.value=sequence", "com.example.key-only"]
    ));
    for name in [
        "external-empty-map",
        "external-empty-list",
        "external-both",
        "deferred-external",
    ] {
        assert!(
            document
                .volumes()
                .iter()
                .find(|volume| volume.name().value() == name)
                .and_then(compose_lens::model::VolumeDefinition::labels)
                .is_some(),
            "{name} labels should remain available"
        );
    }
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == VOLUME_EXTERNAL_LABELS_CONFIGURATION)
            .count(),
        3
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == VOLUME_EXTERNAL_DRIVER_CONFIGURATION)
            .count(),
        1
    );
    Ok(())
}

#[test]
fn preserves_opaque_ipam_strings_ordered_configs_and_scalar_mappings_without_defaults()
-> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "networks:\n",
        "  opaque:\n",
        "    ipam:\n",
        "      driver: opaque-driver\n",
        "      config:\n",
        "        - subnet: not-a-cidr\n",
        "          ip_range: not-a-range\n",
        "          gateway: not-an-address\n",
        "          aux_addresses:\n",
        "            number: 7\n",
        "            boolean: false\n",
        "            null:\n",
        "            string: opaque\n",
        "        - subnet: second-opaque-subnet\n",
        "      options:\n",
        "        number: 9\n",
        "        boolean: true\n",
        "        null:\n",
        "        string: opaque\n",
        "  empty:\n",
        "    ipam: {}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(771), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let opaque = document
        .networks()
        .iter()
        .find(|network| network.name().value() == "opaque")
        .and_then(compose_lens::model::NetworkDefinition::ipam)
        .ok_or("opaque IPAM expected")?;

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(
        opaque.driver().map(Located::value).map(String::as_str),
        Some("opaque-driver")
    );
    assert_eq!(
        &source[opaque.driver().ok_or("driver expected")?.span().range()],
        "opaque-driver"
    );
    assert_eq!(opaque.config().len(), 2);
    assert_eq!(
        opaque
            .config()
            .iter()
            .map(|config| config.subnet().map(Located::value).map(String::as_str))
            .collect::<Vec<_>>(),
        [Some("not-a-cidr"), Some("second-opaque-subnet")]
    );
    let first = &opaque.config()[0];
    assert_eq!(
        first.ip_range().map(Located::value).map(String::as_str),
        Some("not-a-range")
    );
    assert_eq!(
        first.gateway().map(Located::value).map(String::as_str),
        Some("not-an-address")
    );
    assert_eq!(
        first
            .aux_addresses()
            .iter()
            .map(|entry| entry.key().value().as_str())
            .collect::<Vec<_>>(),
        ["number", "boolean", "null", "string"]
    );
    assert!(matches!(first.aux_addresses()[0].value().value(), ComposeScalar::Number(value) if value == "7"));
    assert!(matches!(
        first.aux_addresses()[1].value().value(),
        ComposeScalar::Boolean(false)
    ));
    assert!(matches!(first.aux_addresses()[2].value().value(), ComposeScalar::Null));
    assert!(matches!(first.aux_addresses()[3].value().value(), ComposeScalar::String(value) if value == "opaque"));
    assert_eq!(
        opaque
            .options()
            .iter()
            .map(|entry| entry.key().value().as_str())
            .collect::<Vec<_>>(),
        ["number", "boolean", "null", "string"]
    );
    assert!(matches!(opaque.options()[0].value().value(), ComposeScalar::Number(value) if value == "9"));
    assert!(matches!(
        opaque.options()[1].value().value(),
        ComposeScalar::Boolean(true)
    ));
    assert!(matches!(opaque.options()[2].value().value(), ComposeScalar::Null));
    assert!(matches!(opaque.options()[3].value().value(), ComposeScalar::String(value) if value == "opaque"));

    let empty = document
        .networks()
        .iter()
        .find(|network| network.name().value() == "empty")
        .and_then(compose_lens::model::NetworkDefinition::ipam)
        .ok_or("empty IPAM expected")?;
    assert!(empty.driver().is_none() && empty.config().is_empty() && empty.options().is_empty());
    Ok(())
}

#[test]
fn diagnoses_invalid_ipam_shapes_while_retaining_valid_siblings_extensions_and_unknown_fields()
-> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{DUPLICATE_FIELD, EXPECTED_MAPPING, EXPECTED_SCALAR, EXPECTED_SEQUENCE};

    let syntax = SyntaxDocument::parse(
        SourceId::new(772),
        concat!(
            "networks:\n",
            "  mixed:\n",
            "    ipam:\n",
            "      driver: first\n",
            "      driver: second\n",
            "      config:\n",
            "        - subnet: retained\n",
            "          x-config-evidence: retained\n",
            "          unknown-config: retained\n",
            "        - subnet: 7\n",
            "          ip_range: false\n",
            "          gateway:\n",
            "          aux_addresses: scalar\n",
            "      x-ipam-evidence: retained\n",
            "      unknown-ipam: retained\n",
            "  bad-driver:\n",
            "    ipam: {driver: 7}\n",
            "  bad-config:\n",
            "    ipam: {config: {not: a-sequence}}\n",
            "  bad-entries:\n",
            "    ipam: {config: [scalar, true, null]}\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("partial typed document expected")?;
    let mixed = document
        .networks()
        .iter()
        .find(|network| network.name().value() == "mixed")
        .and_then(compose_lens::model::NetworkDefinition::ipam)
        .ok_or("mixed IPAM expected")?;

    assert!(!parsed.is_valid());
    assert_eq!(mixed.driver().map(Located::value).map(String::as_str), Some("first"));
    assert_eq!(
        mixed.config().len(),
        2,
        "invalid members retain their configuration entry"
    );
    assert_eq!(mixed.config()[0].extension_fields().len(), 1);
    assert_eq!(mixed.config()[0].unknown_fields().len(), 1);
    assert_eq!(
        mixed.config()[1].subnet().map(Located::value).map(String::as_str),
        Some("7")
    );
    assert_eq!(
        mixed.config()[1].ip_range().map(Located::value).map(String::as_str),
        Some("false")
    );
    assert!(mixed.config()[1].gateway().is_none());
    assert!(mixed.config()[1].aux_addresses().is_empty());
    assert_eq!(mixed.extension_fields().len(), 1);
    assert_eq!(mixed.unknown_fields().len(), 1);
    assert!(
        document
            .networks()
            .iter()
            .find(|network| network.name().value() == "bad-driver")
            .and_then(compose_lens::model::NetworkDefinition::ipam)
            .is_some_and(|ipam| ipam.driver().is_some_and(|driver| driver.value() == "7"))
    );
    assert!(
        document
            .networks()
            .iter()
            .find(|network| network.name().value() == "bad-config")
            .and_then(compose_lens::model::NetworkDefinition::ipam)
            .is_some_and(|ipam| ipam.config().is_empty())
    );
    assert!(
        document
            .networks()
            .iter()
            .find(|network| network.name().value() == "bad-entries")
            .and_then(compose_lens::model::NetworkDefinition::ipam)
            .is_some_and(|ipam| ipam.config().is_empty())
    );
    for code in [DUPLICATE_FIELD, EXPECTED_SCALAR, EXPECTED_SEQUENCE, EXPECTED_MAPPING] {
        assert!(
            parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code),
            "expected IPAM diagnostic {code}; got {:#?}",
            parsed.diagnostics()
        );
    }
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
    let init_shape = document
        .service("init-shape")
        .ok_or("service with malformed init shape is missing")?;
    assert!(init_shape.init().is_none());
    assert!(init_shape.image().is_some());
    for expected in [
        EXPECTED_FIELD_FORM,
        EXPECTED_BOOLEAN,
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

    assert_execution_identity(app)?;
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

fn assert_execution_identity(service: &compose_lens::model::Service) -> Result<(), &'static str> {
    let user = service.user().ok_or("typed user expected")?;
    assert_eq!(user.raw().value(), "${UID:-1000}:${GID:-1000}");
    assert!(matches!(user.user(), IdentityComponent::Expression(value) if value == "${UID:-1000}"));
    assert!(matches!(user.group(), Some(IdentityComponent::Expression(value)) if value == "${GID:-1000}"));
    assert_eq!(
        service.userns_mode().map(compose_lens::model::UserNamespaceMode::kind),
        Some(UserNamespaceModeKind::PodmanKeepId)
    );
    assert_eq!(
        service
            .group_add()
            .iter()
            .map(|group| group.value().as_str())
            .collect::<Vec<_>>(),
        ["audio", "44"]
    );
    assert_eq!(
        service.working_dir().map(Located::value).map(String::as_str),
        Some("/srv/app")
    );
    assert_eq!(
        service.read_only().map(Located::value),
        Some(&BooleanValue::Literal(true))
    );
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
    assert_eq!(build.context().map(Located::value).map(String::as_str), Some("."));
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
fn retains_short_and_long_build_contexts_and_reports_malformed_forms() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(696),
        concat!(
            "services:\n",
            "  short:\n    build: ./short\n",
            "  long:\n    build:\n      context: ./long\n      dockerfile: Dockerfile\n      target: \"\"\n",
            "  malformed-context:\n    build:\n      context: []\n",
            "  malformed-dockerfile:\n    build:\n      dockerfile: []\n",
            "  malformed-target:\n    build:\n      target: []\n",
            "  empty-dockerfile:\n    build:\n      dockerfile: \"\"\n",
            "  conflicting-dockerfile:\n    build:\n      dockerfile: Dockerfile\n      dockerfile_inline: FROM scratch\n",
            "  malformed-build:\n    build: []\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    assert!(
        matches!(document.service("short").and_then(compose_lens::model::Service::build),
        Some(Build::Context(context)) if context.value() == "./short")
    );
    let Some(Build::Definition(long)) = document.service("long").and_then(compose_lens::model::Service::build) else {
        return Err("long build definition expected".into());
    };
    assert_eq!(long.context().map(Located::value).map(String::as_str), Some("./long"));
    assert_eq!(
        long.dockerfile().map(Located::value).map(String::as_str),
        Some("Dockerfile")
    );
    assert_eq!(long.target().map(Located::value).map(String::as_str), Some(""));
    assert!(long.field(BuildFieldKind::Dockerfile).is_some());
    let Some(Build::Definition(malformed_context)) = document
        .service("malformed-context")
        .and_then(compose_lens::model::Service::build)
    else {
        return Err("partial malformed build definition expected".into());
    };
    assert!(malformed_context.context().is_none());
    assert!(malformed_context.field(BuildFieldKind::Context).is_some());
    let Some(Build::Definition(malformed_dockerfile)) = document
        .service("malformed-dockerfile")
        .and_then(compose_lens::model::Service::build)
    else {
        return Err("partial malformed Dockerfile definition expected".into());
    };
    assert!(malformed_dockerfile.dockerfile().is_none());
    assert!(malformed_dockerfile.field(BuildFieldKind::Dockerfile).is_some());
    let Some(Build::Definition(malformed_target)) = document
        .service("malformed-target")
        .and_then(compose_lens::model::Service::build)
    else {
        return Err("partial malformed target definition expected".into());
    };
    assert!(malformed_target.target().is_none());
    assert!(malformed_target.field(BuildFieldKind::Target).is_some());
    let Some(Build::Definition(empty_dockerfile)) = document
        .service("empty-dockerfile")
        .and_then(compose_lens::model::Service::build)
    else {
        return Err("partial empty Dockerfile definition expected".into());
    };
    assert!(empty_dockerfile.dockerfile().is_none());
    let Some(Build::Definition(conflicting_dockerfile)) = document
        .service("conflicting-dockerfile")
        .and_then(compose_lens::model::Service::build)
    else {
        return Err("conflicting Dockerfile definition expected".into());
    };
    assert_eq!(
        conflicting_dockerfile
            .dockerfile()
            .map(Located::value)
            .map(String::as_str),
        Some("Dockerfile")
    );
    assert!(conflicting_dockerfile.field(BuildFieldKind::DockerfileInline).is_some());
    assert!(
        document
            .service("malformed-build")
            .and_then(compose_lens::model::Service::build)
            .is_none()
    );
    for code in [
        BUILD_DOCKERFILE_EXPECTED_NON_EMPTY,
        BUILD_DOCKERFILE_INLINE_CONFLICT,
        EXPECTED_FIELD_FORM,
        EXPECTED_SCALAR,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == BUILD_DOCKERFILE_INLINE_CONFLICT && diagnostic.labels().len() == 2)
    );
    Ok(())
}

#[test]
fn retains_build_no_cache_yaml_type_and_invalid_field_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source_id = SourceId::new(842);
    let syntax = SyntaxDocument::parse(
        source_id,
        concat!(
            "services:\n",
            "  boolean:\n    build: {no_cache: true}\n",
            "  string:\n    build: {no_cache: \"true\"}\n",
            "  expression:\n    build: {no_cache: \"${NO_CACHE:-false}\"}\n",
            "  empty:\n    build: {no_cache: \"\"}\n",
            "  null:\n    build:\n      context: retained\n      no_cache: null\n",
            "  number:\n    build:\n      context: retained\n      no_cache: 1\n",
            "  mapping:\n    build:\n      context: retained\n      no_cache: {invalid: value}\n",
            "  sequence:\n    build:\n      context: retained\n      no_cache: [true]\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    let no_cache = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::build)
            .and_then(|build| match build {
                Build::Definition(definition) => definition.no_cache(),
                Build::Context(_) => None,
            })
    };
    assert!(matches!(
        no_cache("boolean").map(Located::value),
        Some(BuildNoCache::Boolean(true))
    ));
    assert!(matches!(no_cache("string").map(Located::value), Some(BuildNoCache::String(value)) if value == "true"));
    assert!(
        matches!(no_cache("expression").map(Located::value), Some(BuildNoCache::String(value)) if value == "${NO_CACHE:-false}")
    );
    assert!(matches!(no_cache("empty").map(Located::value), Some(BuildNoCache::String(value)) if value.is_empty()));
    assert_eq!(
        no_cache("string")
            .map(Located::span)
            .map(compose_lens::source::SourceSpan::source_id),
        Some(source_id)
    );

    for service in ["null", "number", "mapping", "sequence"] {
        let build = document
            .service(service)
            .and_then(compose_lens::model::Service::build)
            .ok_or("partial invalid build definition expected")?;
        let Build::Definition(definition) = build else {
            return Err("long build definition expected".into());
        };
        assert!(definition.no_cache().is_none());
        assert!(
            definition.field(BuildFieldKind::NoCache).is_some(),
            "missing no_cache field for {service}: {:?}",
            definition.fields()
        );
        assert_eq!(
            definition.context().map(Located::value).map(String::as_str),
            Some("retained")
        );
    }
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == BUILD_NO_CACHE_EXPECTED_BOOLEAN_OR_STRING)
            .count(),
        4
    );
    Ok(())
}

#[test]
fn retains_build_sbom_yaml_type_spelling_and_invalid_field_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source_id = SourceId::new(8421);
    let syntax = SyntaxDocument::parse(
        source_id,
        concat!(
            "services:\n",
            "  boolean:\n    build: {sbom: true}\n",
            "  generator:\n    build: {sbom: \"generator=example.com/sbom\"}\n",
            "  expression:\n    build: {sbom: \"${SBOM_GENERATOR}\"}\n",
            "  empty:\n    build: {sbom: \"\"}\n",
            "  null:\n    build:\n      context: retained\n      sbom: null\n",
            "  number:\n    build:\n      context: retained\n      sbom: 1\n",
            "  mapping:\n    build:\n      context: retained\n      sbom: {invalid: value}\n",
            "  sequence:\n    build:\n      context: retained\n      sbom: [true]\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let sbom = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::build)
            .and_then(|build| match build {
                Build::Definition(definition) => definition.sbom(),
                Build::Context(_) => None,
            })
    };

    assert!(matches!(
        sbom("boolean").map(Located::value),
        Some(BuildSbom::Boolean(true))
    ));
    assert!(
        matches!(sbom("generator").map(Located::value), Some(BuildSbom::String(value)) if value == "generator=example.com/sbom")
    );
    assert!(
        matches!(sbom("expression").map(Located::value), Some(BuildSbom::String(value)) if value == "${SBOM_GENERATOR}")
    );
    assert!(matches!(sbom("empty").map(Located::value), Some(BuildSbom::String(value)) if value.is_empty()));
    assert_eq!(
        sbom("generator")
            .map(Located::span)
            .map(compose_lens::source::SourceSpan::source_id),
        Some(source_id)
    );

    for service in ["null", "number", "mapping", "sequence"] {
        let Some(Build::Definition(definition)) =
            document.service(service).and_then(compose_lens::model::Service::build)
        else {
            return Err("partial invalid build definition expected".into());
        };
        assert!(definition.sbom().is_none());
        assert!(
            definition.field(BuildFieldKind::Sbom).is_some(),
            "missing sbom for {service}"
        );
        assert_eq!(
            definition.context().map(Located::value).map(String::as_str),
            Some("retained")
        );
    }
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == BUILD_SBOM_EXPECTED_BOOLEAN_OR_STRING)
            .count(),
        4
    );
    Ok(())
}

#[test]
fn retains_build_shm_size_classification_and_invalid_field_evidence() -> Result<(), Box<dyn std::error::Error>> {
    use compose_lens::model::{SHM_SIZE_EXPECTED_VALUE, ShmSizeKind, ShmSizeScalarKind, ShmSizeUnit};

    let source_id = SourceId::new(8460);
    let syntax = SyntaxDocument::parse(
        source_id,
        concat!(
            "services:\n",
            "  documented:\n    build: {shm_size: \"64mb\"}\n",
            "  number:\n    build: {shm_size: 64}\n",
            "  zero:\n    build: {shm_size: \"000m\"}\n",
            "  expression:\n    build: {shm_size: \"${BUILD_SHM_SIZE:-64mb}\"}\n",
            "  null:\n    build:\n      context: retained\n      shm_size: null\n      x-retained: true\n",
            "  mapping:\n    build:\n      context: retained\n      shm_size:\n        value: 64mb\n      x-retained: true\n",
            "  sequence:\n    build:\n      context: retained\n      shm_size:\n        - 64mb\n      x-retained: true\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let shm_size = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::build)
            .and_then(|build| match build {
                Build::Definition(definition) => definition.shm_size(),
                Build::Context(_) => None,
            })
    };

    let documented = shm_size("documented").ok_or("documented build shm_size expected")?;
    assert_eq!(documented.raw().value(), "64mb");
    assert_eq!(documented.raw().span().source_id(), source_id);
    assert_eq!(documented.scalar_kind(), ShmSizeScalarKind::String);
    assert!(matches!(
        documented.kind(),
        ShmSizeKind::Documented { amount_raw, unit: ShmSizeUnit::Mb } if amount_raw == "64"
    ));
    assert!(matches!(
        shm_size("number").map(compose_lens::model::ShmSize::kind),
        Some(ShmSizeKind::ProviderDependentNumber)
    ));
    assert!(matches!(
        shm_size("zero").map(compose_lens::model::ShmSize::kind),
        Some(ShmSizeKind::Zero { amount_raw, unit: Some(ShmSizeUnit::M) }) if amount_raw == "000"
    ));
    assert!(matches!(
        shm_size("expression").map(compose_lens::model::ShmSize::kind),
        Some(ShmSizeKind::Expression)
    ));

    for service in ["null", "mapping", "sequence"] {
        let Build::Definition(definition) = document
            .service(service)
            .and_then(compose_lens::model::Service::build)
            .ok_or("partial invalid build definition expected")?
        else {
            return Err("long build definition expected".into());
        };
        assert!(definition.shm_size().is_none());
        assert!(
            definition.field(BuildFieldKind::ShmSize).is_some(),
            "missing build shm_size field for {service}: {:?}",
            definition.fields()
        );
        assert_eq!(
            definition.context().map(Located::value).map(String::as_str),
            Some("retained")
        );
        assert_eq!(definition.extension_fields().len(), 1);
    }
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SHM_SIZE_EXPECTED_VALUE)
            .count(),
        3
    );
    Ok(())
}

#[test]
fn retains_opaque_build_isolation_strings_and_invalid_field_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source_id = SourceId::new(847);
    let syntax = SyntaxDocument::parse(
        source_id,
        concat!(
            "services:\n",
            "  quoted:\n    build: {isolation: \"process\"}\n",
            "  plain:\n    build: {isolation: hyperv}\n",
            "  empty:\n    build: {isolation: \"\"}\n",
            "  boolean:\n    build:\n      context: retained\n      isolation: true\n",
            "  number:\n    build:\n      context: retained\n      isolation: 1\n",
            "  null:\n    build:\n      context: retained\n      isolation: null\n",
            "  sequence:\n    build:\n      context: retained\n      isolation: [process]\n",
            "  mapping:\n    build:\n      context: retained\n      isolation: {mode: process}\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    let isolation = |service| {
        document
            .service(service)
            .and_then(compose_lens::model::Service::build)
            .and_then(|build| match build {
                Build::Definition(definition) => definition.isolation(),
                Build::Context(_) => None,
            })
    };
    assert_eq!(
        isolation("quoted").map(Located::value).map(String::as_str),
        Some("process")
    );
    assert_eq!(
        isolation("plain").map(Located::value).map(String::as_str),
        Some("hyperv")
    );
    assert_eq!(isolation("empty").map(Located::value).map(String::as_str), Some(""));
    assert_eq!(
        isolation("quoted")
            .map(Located::span)
            .map(compose_lens::source::SourceSpan::source_id),
        Some(source_id)
    );

    for service in ["boolean", "number", "null", "sequence", "mapping"] {
        let Build::Definition(definition) = document
            .service(service)
            .and_then(compose_lens::model::Service::build)
            .ok_or("partial invalid build definition expected")?
        else {
            return Err("long build definition expected".into());
        };
        assert!(definition.isolation().is_none());
        assert!(
            definition.field(BuildFieldKind::Isolation).is_some(),
            "missing isolation field for {service}: {:?}",
            definition.fields()
        );
        assert_eq!(
            definition.context().map(Located::value).map(String::as_str),
            Some("retained")
        );
    }
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == BUILD_ISOLATION_EXPECTED_STRING)
            .count(),
        5
    );
    Ok(())
}

#[test]
fn retains_ordered_build_tags_and_recovers_malformed_forms() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(697),
        concat!(
            "services:\n",
            "  valid:\n    build:\n      tags:\n        - example/app:one\n        - \"7\"\n        - example/app:one\n",
            "  malformed-form:\n    build:\n      tags: {}\n",
            "  malformed-items:\n    build:\n      tags:\n        - good\n        - []\n        - later\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let tags = authored_build_tags(document, "valid")?;
    assert_eq!(
        tags.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>(),
        ["example/app:one", "7", "example/app:one"]
    );
    let malformed_form = document
        .service("malformed-form")
        .and_then(compose_lens::model::Service::build)
        .ok_or("partial malformed build tags definition expected")?;
    assert!(
        matches!(malformed_form, Build::Definition(definition) if definition.tags().is_none() && definition.field(BuildFieldKind::Tags).is_some())
    );
    let malformed_items = authored_build_tags(document, "malformed-items")?;
    assert_eq!(
        malformed_items
            .iter()
            .map(Located::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["good", "later"]
    );
    for code in [EXPECTED_SCALAR, EXPECTED_SEQUENCE] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_raw_build_platforms_with_empty_and_partial_malformed_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(698),
        concat!(
            "services:\n",
            "  valid:\n    build:\n      platforms: [linux/amd64, 7, linux/amd64]\n",
            "  empty:\n    build:\n      platforms: []\n",
            "  malformed-form:\n    build:\n      platforms: {}\n",
            "  malformed-items:\n    build:\n      platforms: [linux/amd64, {}, linux/arm64]\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let platforms = authored_build_platforms(document, "valid")?;
    assert_eq!(
        platforms
            .iter()
            .map(Located::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["linux/amd64", "7", "linux/amd64"]
    );
    assert!(authored_build_platforms(document, "empty")?.is_empty());
    let malformed_form = document
        .service("malformed-form")
        .and_then(compose_lens::model::Service::build)
        .ok_or("partial malformed build platforms definition expected")?;
    assert!(matches!(malformed_form, Build::Definition(definition)
            if definition.platforms().is_none() && definition.field(BuildFieldKind::Platforms).is_some()));
    assert_eq!(
        authored_build_platforms(document, "malformed-items")?
            .iter()
            .map(Located::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["linux/amd64", "linux/arm64"]
    );
    for code in [EXPECTED_SCALAR, EXPECTED_SEQUENCE] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_build_label_forms_empties_and_partial_malformed_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(699),
        concat!(
            "services:\n",
            "  map:\n    build:\n      labels: {}\n",
            "  list:\n    build:\n      labels:\n        - io.example.role=database\n        - io.example.bare\n        - io.example.role=database\n",
            "  malformed-map:\n    build:\n      labels:\n        valid: value\n        invalid: []\n        later: null\n",
            "  malformed-list:\n    build:\n      labels:\n        - valid=value\n        - []\n        - later\n",
            "  malformed-form:\n    build:\n      labels: label\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    let map = authored_build_definition(document, "map")?
        .labels()
        .ok_or("empty map labels expected")?;
    assert!(matches!(map, Labels::Map { entries, .. } if entries.is_empty()));
    let list = authored_build_definition(document, "list")?
        .labels()
        .ok_or("list labels expected")?;
    assert!(matches!(list, Labels::List { values, .. }
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["io.example.role=database", "io.example.bare", "io.example.role=database"]));
    let malformed_map = authored_build_definition(document, "malformed-map")?
        .labels()
        .ok_or("partial map labels expected")?;
    assert!(matches!(malformed_map, Labels::Map { entries, .. }
        if entries.iter().map(KeyValueEntry::key).map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["valid", "later"]));
    let malformed_list = authored_build_definition(document, "malformed-list")?
        .labels()
        .ok_or("partial list labels expected")?;
    assert!(matches!(malformed_list, Labels::List { values, .. }
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["valid=value", "later"]));
    let malformed_form = authored_build_definition(document, "malformed-form")?;
    assert!(malformed_form.labels().is_none());
    assert!(malformed_form.field(BuildFieldKind::Labels).is_some());
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_FIELD_FORM)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    Ok(())
}

#[test]
fn retains_build_additional_context_forms_duplicates_and_malformed_siblings() -> Result<(), Box<dyn std::error::Error>>
{
    let syntax = SyntaxDocument::parse(
        SourceId::new(900),
        concat!(
            "services:\n",
            "  list:\n    build:\n      additional_contexts: [assets=./assets, service=service:base, assets=./assets]\n",
            "  map:\n    build:\n      additional_contexts:\n        assets: ./assets\n        image: example/context:latest\n        empty: null\n",
            "  malformed-list:\n    build:\n      additional_contexts: [before=./before, 7, {bad: value}, later=https://example.invalid/context]\n",
            "  malformed-map:\n    build:\n      additional_contexts:\n        retained: ./retained\n        invalid: [nested]\n        duplicate: first\n        duplicate: second\n        later: service:base\n",
            "  malformed-form:\n    build:\n      additional_contexts: ./not-a-collection\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    let list = authored_build_definition(document, "list")?
        .additional_contexts()
        .ok_or("list additional contexts expected")?;
    assert!(matches!(list, BuildAdditionalContexts::List { values, .. }
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["assets=./assets", "service=service:base", "assets=./assets"]));
    let map = authored_build_definition(document, "map")?
        .additional_contexts()
        .ok_or("map additional contexts expected")?;
    assert!(matches!(map, BuildAdditionalContexts::Map { entries, .. }
        if entries.iter().map(KeyValueEntry::key).map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["assets", "image", "empty"]));
    assert!(matches!(
        map,
        BuildAdditionalContexts::Map { entries, .. }
            if matches!(entries[2].value().value(), ComposeScalar::Null)
    ));
    let malformed_list = authored_build_definition(document, "malformed-list")?
        .additional_contexts()
        .ok_or("partial list additional contexts expected")?;
    assert!(matches!(malformed_list, BuildAdditionalContexts::List { values, .. }
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["before=./before", "later=https://example.invalid/context"]));
    let malformed_map = authored_build_definition(document, "malformed-map")?
        .additional_contexts()
        .ok_or("partial map additional contexts expected")?;
    assert!(matches!(malformed_map, BuildAdditionalContexts::Map { entries, .. }
        if entries.iter().map(KeyValueEntry::key).map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["retained", "duplicate", "later"]));
    assert!(
        authored_build_definition(document, "malformed-form")?
            .additional_contexts()
            .is_none()
    );
    for code in [DUPLICATE_FIELD, EXPECTED_FIELD_FORM, EXPECTED_SCALAR] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_build_extra_hosts_as_a_distinct_raw_list_or_nested_address_mapping() -> Result<(), Box<dyn std::error::Error>>
{
    let syntax = SyntaxDocument::parse(
        SourceId::new(1930),
        concat!(
            "services:\n",
            "  list:\n    build:\n      extra_hosts: [\"db:127.0.0.1\", \"v6=[::1]\", \"gateway=host-gateway\", \"db:127.0.0.1\"]\n",
            "  map:\n    build:\n      extra_hosts:\n        db: 127.0.0.1\n        v6: [\"[::1]\", host-gateway]\n",
            "  malformed:\n    build:\n      extra_hosts:\n        retained: 127.0.0.1\n        broken: 7\n        later: [\"host-gateway\"]\n",
            "  outer:\n    build: {extra_hosts: not-a-collection}\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    let list = authored_build_definition(document, "list")?
        .extra_hosts()
        .ok_or("build list extra_hosts expected")?;
    assert!(matches!(list, BuildExtraHosts::List { values, .. }
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["db:127.0.0.1", "v6=[::1]", "gateway=host-gateway", "db:127.0.0.1"]));
    let map = authored_build_definition(document, "map")?
        .extra_hosts()
        .ok_or("build map extra_hosts expected")?;
    let BuildExtraHosts::Map { entries, .. } = map else {
        return Err("build mapping extra_hosts expected".into());
    };
    assert_eq!(entries[0].hostname().value(), "db");
    assert!(matches!(entries[0].addresses(), BuildExtraHostAddresses::Scalar(value) if value.value() == "127.0.0.1"));
    assert!(
        matches!(entries[1].addresses(), BuildExtraHostAddresses::List { values, .. }
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>() == ["[::1]", "host-gateway"])
    );
    let malformed = authored_build_definition(document, "malformed")?
        .extra_hosts()
        .ok_or("partial build extra_hosts expected")?;
    let BuildExtraHosts::Map { entries, .. } = malformed else {
        return Err("malformed mapping expected".into());
    };
    assert_eq!(
        entries.iter().map(|entry| entry.hostname().value()).collect::<Vec<_>>(),
        ["retained", "later"]
    );
    assert!(authored_build_definition(document, "outer")?.extra_hosts().is_none());
    for code in [
        BUILD_EXTRA_HOSTS_DUPLICATE_ITEM,
        BUILD_EXTRA_HOSTS_EXPECTED_STRING,
        BUILD_EXTRA_HOSTS_EXPECTED_FORM,
    ] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_opaque_build_network_and_reports_malformed_form() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(698),
        concat!(
            "services:\n",
            "  valid:\n    build:\n      network: \"\"\n",
            "  malformed:\n    build:\n      network: []\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let valid = authored_build_definition(document, "valid")?;
    assert_eq!(valid.network().map(Located::value).map(String::as_str), Some(""));
    assert!(valid.field(BuildFieldKind::Network).is_some());
    let malformed = authored_build_definition(document, "malformed")?;
    assert!(malformed.network().is_none());
    assert!(malformed.field(BuildFieldKind::Network).is_some());
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    Ok(())
}

#[test]
fn retains_build_secret_grant_forms_and_recovers_malformed_items() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(700),
        concat!(
            "secrets:\n  short-secret: {}\n  long-secret: {}\n",
            "services:\n",
            "  valid:\n    build:\n      secrets:\n",
            "        - short-secret\n",
            "        - source: long-secret\n          target: /run/secrets/build\n          uid: \"01000\"\n          gid: 1001\n          mode: \"0440\"\n          x-retained: yes\n          future: retained\n",
            "        - short-secret\n",
            "  malformed:\n    build:\n      secrets:\n",
            "        - source: []\n          target: /run/secrets/bad\n",
            "        - valid-after\n",
            "        - []\n",
            "  empty:\n    build: {secrets: []}\n",
            "  malformed-form:\n    build: {secrets: {source: short-secret}}\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    assert!(
        document
            .secrets()
            .iter()
            .any(|secret| secret.name().value() == "short-secret")
    );
    let valid = authored_build_definition(document, "valid")?
        .secrets()
        .ok_or("build secrets expected")?;
    assert_eq!(valid.len(), 3);
    assert!(matches!(&valid[0], SecretGrant::Short(value) if value.value() == "short-secret"));
    let SecretGrant::Long(long) = &valid[1] else {
        return Err("long build secret expected".into());
    };
    assert_eq!(
        long.source().map(Located::value).map(String::as_str),
        Some("long-secret")
    );
    assert_eq!(
        long.target().map(Located::value).map(String::as_str),
        Some("/run/secrets/build")
    );
    assert_eq!(long.uid().map(Located::value).map(String::as_str), Some("01000"));
    assert_eq!(long.gid().map(Located::value).map(String::as_str), Some("1001"));
    assert_eq!(long.mode().map(Located::value).map(String::as_str), Some("0440"));
    assert_eq!(long.extension_fields().len(), 1);
    assert_eq!(long.unknown_fields().len(), 1);

    let malformed = authored_build_definition(document, "malformed")?
        .secrets()
        .ok_or("partial build secrets expected")?;
    assert_eq!(malformed.len(), 2);
    assert!(matches!(&malformed[1], SecretGrant::Short(value) if value.value() == "valid-after"));
    assert!(
        authored_build_definition(document, "empty")?
            .secrets()
            .is_some_and(<[SecretGrant]>::is_empty)
    );
    let malformed_form = authored_build_definition(document, "malformed-form")?;
    assert!(malformed_form.secrets().is_none());
    assert!(malformed_form.field(BuildFieldKind::Secrets).is_some());
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == GRANT_EXPECTED_FORM)
    );
    Ok(())
}

#[test]
fn retains_raw_build_cache_locations_and_recovers_malformed_siblings() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(908),
        concat!(
            "services:\n",
            "  valid:\n    build:\n",
            "      cache_from: [\"type=registry,ref=example.invalid/cache\", \"\", \"type=registry,ref=example.invalid/cache\"]\n",
            "      cache_to: [\"type=local,dest=.cache\", \"\", \"type=local,dest=.cache\"]\n",
            "  malformed:\n    build:\n",
            "      cache_from: [\"type=local,src=.cache\", 7, null, {nested: value}, \"type=gha\"]\n",
            "      cache_to: [\"type=local,dest=.cache\", false, [], \"type=gha\"]\n",
            "  malformed-outer:\n    build:\n",
            "      cache_from: {type: local}\n",
            "      cache_to: type=local,dest=.cache\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    let valid = authored_build_definition(document, "valid")?;
    assert!(matches!(valid.cache_from(), Some(values)
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["type=registry,ref=example.invalid/cache", "", "type=registry,ref=example.invalid/cache"]));
    assert!(matches!(valid.cache_to(), Some(values)
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["type=local,dest=.cache", "", "type=local,dest=.cache"]));
    assert!(
        valid
            .cache_from()
            .is_some_and(|values| values.iter().all(|value| !value.span().is_empty()))
    );
    assert!(
        valid
            .cache_to()
            .is_some_and(|values| values.iter().all(|value| !value.span().is_empty()))
    );

    let malformed = authored_build_definition(document, "malformed")?;
    assert!(matches!(malformed.cache_from(), Some(values)
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["type=local,src=.cache", "type=gha"]));
    assert!(matches!(malformed.cache_to(), Some(values)
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["type=local,dest=.cache", "type=gha"]));

    let outer = authored_build_definition(document, "malformed-outer")?;
    assert!(outer.cache_from().is_none());
    assert!(outer.cache_to().is_none());
    assert!(outer.field(BuildFieldKind::CacheFrom).is_some());
    assert!(outer.field(BuildFieldKind::CacheTo).is_some());
    for code in [EXPECTED_SEQUENCE, EXPECTED_SCALAR] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

fn authored_build_tags<'document>(
    document: &'document ComposeDocument,
    service: &str,
) -> Result<&'document [Located<String>], Box<dyn std::error::Error>> {
    let build = document
        .service(service)
        .and_then(compose_lens::model::Service::build)
        .ok_or("long build definition expected")?;
    let Build::Definition(definition) = build else {
        return Err("long build definition expected".into());
    };
    definition.tags().ok_or_else(|| "build tags expected".into())
}

fn authored_build_platforms<'document>(
    document: &'document ComposeDocument,
    service: &str,
) -> Result<&'document [Located<String>], Box<dyn std::error::Error>> {
    let definition = authored_build_definition(document, service)?;
    definition.platforms().ok_or_else(|| "build platforms expected".into())
}

fn authored_build_definition<'document>(
    document: &'document ComposeDocument,
    service: &str,
) -> Result<&'document compose_lens::model::BuildDefinition, Box<dyn std::error::Error>> {
    let build = document
        .service(service)
        .and_then(compose_lens::model::Service::build)
        .ok_or("long build definition expected")?;
    let Build::Definition(definition) = build else {
        return Err("long build definition expected".into());
    };
    Ok(definition)
}

#[test]
fn retains_build_pull_literals_expressions_and_malformed_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(831),
        concat!(
            "services:\n",
            "  true-value:\n    build: {pull: true}\n",
            "  false-value:\n    build: {pull: false}\n",
            "  deferred:\n    build: {pull: \"${BUILD_PULL:-true}\"}\n",
            "  malformed:\n    build: {pull: nope}\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    assert_eq!(
        authored_build_definition(document, "true-value")?
            .pull()
            .map(Located::value),
        Some(&BooleanValue::Literal(true))
    );
    assert_eq!(
        authored_build_definition(document, "false-value")?
            .pull()
            .map(Located::value),
        Some(&BooleanValue::Literal(false))
    );
    let deferred = authored_build_definition(document, "deferred")?
        .pull()
        .ok_or("deferred build pull expected")?;
    assert_eq!(
        deferred.value(),
        &BooleanValue::Expression("${BUILD_PULL:-true}".to_owned())
    );
    assert!(!deferred.span().is_empty());

    let malformed = authored_build_definition(document, "malformed")?;
    assert!(malformed.pull().is_none());
    assert!(malformed.field(BuildFieldKind::Pull).is_some());
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_BOOLEAN)
    );
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

#[test]
fn retains_service_stdin_open_literals_expressions_and_duplicate_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(684),
        concat!(
            "services:\n",
            "  literal-true:\n    stdin_open: true\n",
            "  literal-false:\n    stdin_open: false\n",
            "  deferred:\n    stdin_open: ${KEEP_STDIN:-true}\n",
            "  invalid:\n    stdin_open: [true]\n",
            "  duplicate:\n    stdin_open: true\n    stdin_open: false\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    assert_eq!(
        document
            .service("literal-true")
            .and_then(compose_lens::model::Service::stdin_open)
            .map(Located::value),
        Some(&BooleanValue::Literal(true))
    );
    assert_eq!(
        document
            .service("literal-false")
            .and_then(compose_lens::model::Service::stdin_open)
            .map(Located::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(
        document
            .service("deferred")
            .and_then(compose_lens::model::Service::stdin_open)
            .map(Located::value),
        Some(&BooleanValue::Expression("${KEEP_STDIN:-true}".to_owned()))
    );
    assert!(
        document
            .service("invalid")
            .and_then(compose_lens::model::Service::stdin_open)
            .is_none()
    );
    assert_eq!(
        document
            .service("duplicate")
            .and_then(compose_lens::model::Service::stdin_open)
            .map(Located::value),
        Some(&BooleanValue::Literal(true))
    );
    for code in [EXPECTED_BOOLEAN, DUPLICATE_FIELD] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_service_tty_literals_expressions_and_duplicate_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(687),
        concat!(
            "services:\n",
            "  literal-true:\n    tty: true\n",
            "  literal-false:\n    tty: false\n",
            "  deferred:\n    tty: ${KEEP_TTY:-true}\n",
            "  invalid:\n    tty: [true]\n",
            "  duplicate:\n    tty: true\n    tty: false\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    assert_eq!(
        document
            .service("literal-true")
            .and_then(compose_lens::model::Service::tty)
            .map(Located::value),
        Some(&BooleanValue::Literal(true))
    );
    assert_eq!(
        document
            .service("literal-false")
            .and_then(compose_lens::model::Service::tty)
            .map(Located::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(
        document
            .service("deferred")
            .and_then(compose_lens::model::Service::tty)
            .map(Located::value),
        Some(&BooleanValue::Expression("${KEEP_TTY:-true}".to_owned()))
    );
    assert!(
        document
            .service("invalid")
            .and_then(compose_lens::model::Service::tty)
            .is_none()
    );
    assert_eq!(
        document
            .service("duplicate")
            .and_then(compose_lens::model::Service::tty)
            .map(Located::value),
        Some(&BooleanValue::Literal(true))
    );
    for code in [EXPECTED_BOOLEAN, DUPLICATE_FIELD] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_service_privileged_literals_expressions_spans_and_duplicate_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(691),
        concat!(
            "services:\n",
            "  omitted: {}\n",
            "  literal-true:\n    privileged: true\n",
            "  literal-false:\n    privileged: false\n",
            "  deferred:\n    privileged: ${KEEP_PRIVILEGED:-true}\n",
            "  invalid:\n    privileged: [true]\n",
            "  duplicate:\n    privileged: true\n    privileged: false\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    assert!(
        document
            .service("omitted")
            .and_then(compose_lens::model::Service::privileged)
            .is_none()
    );
    let literal_true = document
        .service("literal-true")
        .and_then(compose_lens::model::Service::privileged)
        .ok_or("literal privileged true expected")?;
    assert_eq!(literal_true.value(), &BooleanValue::Literal(true));
    assert!(!literal_true.span().is_empty());
    assert_eq!(
        document
            .service("literal-false")
            .and_then(compose_lens::model::Service::privileged)
            .map(Located::value),
        Some(&BooleanValue::Literal(false))
    );
    assert_eq!(
        document
            .service("deferred")
            .and_then(compose_lens::model::Service::privileged)
            .map(Located::value),
        Some(&BooleanValue::Expression("${KEEP_PRIVILEGED:-true}".to_owned()))
    );
    assert!(
        document
            .service("invalid")
            .and_then(compose_lens::model::Service::privileged)
            .is_none()
    );
    assert_eq!(
        document
            .service("duplicate")
            .and_then(compose_lens::model::Service::privileged)
            .map(Located::value),
        Some(&BooleanValue::Literal(true))
    );
    for code in [EXPECTED_BOOLEAN, DUPLICATE_FIELD] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_service_attach_literals_expression_source_evidence_and_invalid_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(3214),
        concat!(
            "services:\n",
            "  literal-true:\n    attach: true\n",
            "  literal-false:\n    attach: false\n",
            "  deferred:\n    attach: \"${ATTACH:-true}\"\n",
            "  duplicate:\n    attach: true\n    attach: false\n",
            "  quoted:\n    attach: \"true\"\n",
            "  null:\n    attach: null\n",
            "  number:\n    attach: 1\n",
            "  mapping:\n    attach: {enabled: true}\n",
            "  list:\n    attach: [true]\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    assert_eq!(
        document
            .service("literal-true")
            .and_then(compose_lens::model::Service::attach)
            .map(Located::value),
        Some(&BooleanValue::Literal(true))
    );
    assert_eq!(
        document
            .service("literal-false")
            .and_then(compose_lens::model::Service::attach)
            .map(Located::value),
        Some(&BooleanValue::Literal(false))
    );
    let deferred = document
        .service("deferred")
        .and_then(compose_lens::model::Service::attach)
        .ok_or("deferred attach expected")?;
    assert_eq!(
        deferred.value(),
        &BooleanValue::Expression("${ATTACH:-true}".to_owned())
    );
    assert!(!deferred.span().is_empty());
    assert_eq!(
        document
            .service("duplicate")
            .and_then(compose_lens::model::Service::attach)
            .map(Located::value),
        Some(&BooleanValue::Literal(true))
    );
    for service in ["quoted", "null", "number", "mapping", "list"] {
        let service = document.service(service).ok_or("service expected")?;
        assert!(service.attach().is_none());
        assert!(
            service
                .unknown_fields()
                .iter()
                .any(|field| field.name().value() == "attach")
        );
    }
    for code in [EXPECTED_BOOLEAN, DUPLICATE_FIELD] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn retains_build_args_mapping_and_raw_list_forms_with_partial_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(712),
        concat!(
            "services:\n",
            "  omitted:\n    build: {}\n",
            "  empty-map:\n    build:\n      args: {}\n",
            "  empty-list:\n    build:\n      args: []\n",
            "  mapping:\n    build:\n      args:\n        string: value\n        number: 42\n        boolean: true\n        empty: null\n",
            "  list:\n    build:\n      args: [KEY=value, BARE, KEY=value]\n",
            "  malformed:\n    build:\n      args: [valid=value, false, {nested: value}, later]\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;

    assert!(
        document
            .service("omitted")
            .and_then(compose_lens::model::Service::build)
            .is_some_and(|build| matches!(build, Build::Definition(definition) if definition.args().is_none()))
    );
    assert!(matches!(
        document
            .service("empty-map")
            .and_then(compose_lens::model::Service::build)
            .and_then(|build| match build {
                Build::Definition(definition) => definition.args(),
                Build::Context(_) => None,
            }),
        Some(BuildArgs::Map { entries, .. }) if entries.is_empty()
    ));
    assert!(matches!(
        document
            .service("empty-list")
            .and_then(compose_lens::model::Service::build)
            .and_then(|build| match build {
                Build::Definition(definition) => definition.args(),
                Build::Context(_) => None,
            }),
        Some(BuildArgs::List { values, .. }) if values.is_empty()
    ));

    let mapping = document
        .service("mapping")
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => definition.args(),
            Build::Context(_) => None,
        })
        .ok_or("mapping build args expected")?;
    let BuildArgs::Map { entries, .. } = mapping else {
        return Err("mapping build args form expected".into());
    };
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.key().value().as_str())
            .collect::<Vec<_>>(),
        ["string", "number", "boolean", "empty"]
    );
    assert_eq!(entries[0].value().value(), &ComposeScalar::String("value".to_owned()));
    assert_eq!(entries[1].value().value(), &ComposeScalar::Number("42".to_owned()));
    assert_eq!(entries[2].value().value(), &ComposeScalar::Boolean(true));
    assert_eq!(entries[3].value().value(), &ComposeScalar::Null);
    let list = document
        .service("list")
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => definition.args(),
            Build::Context(_) => None,
        })
        .ok_or("list build args expected")?;
    assert!(matches!(list, BuildArgs::List { values, .. }
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>()
            == ["KEY=value", "BARE", "KEY=value"]));
    let malformed = document
        .service("malformed")
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => definition.args(),
            Build::Context(_) => None,
        })
        .ok_or("partially recovered build args expected")?;
    assert!(matches!(malformed, BuildArgs::List { values, .. }
        if values.iter().map(Located::value).map(String::as_str).collect::<Vec<_>>() == ["valid=value", "later"]));
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == EXPECTED_SCALAR)
    );
    Ok(())
}

#[test]
fn retains_sensitive_build_ssh_forms_duplicates_and_valid_siblings() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(997),
        concat!(
            "services:\n",
            "  list:\n    build:\n      ssh: [default, \"id=deploy,src=/private/key\", default, false, {invalid: item}, later]\n",
            "  map:\n    build:\n      ssh:\n        default: \"/private/socket\"\n        retries: 2\n        enabled: true\n        empty: null\n        nested: {bad: value}\n",
            "  wrong:\n    build:\n      ssh: default\n",
        ),
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document expected")?;
    let list = document
        .service("list")
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => definition.ssh(),
            Build::Context(_) => None,
        })
        .ok_or("list build ssh expected")?;
    assert_eq!(list.form(), BuildSshForm::List);
    assert_eq!(
        list.as_list()
            .ok_or("list accessor expected")?
            .iter()
            .map(Located::value)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["default", "id=deploy,src=/private/key", "default", "later"]
    );
    assert!(!format!("{list:?}").contains("/private/key"));

    let mapping = document
        .service("map")
        .and_then(compose_lens::model::Service::build)
        .and_then(|build| match build {
            Build::Definition(definition) => definition.ssh(),
            Build::Context(_) => None,
        })
        .ok_or("mapping build ssh expected")?;
    assert_eq!(mapping.form(), BuildSshForm::Map);
    let entries = mapping.as_map().ok_or("mapping accessor expected")?;
    assert!(entries.len() >= 4);
    assert_eq!(
        entries
            .iter()
            .take(4)
            .map(|entry| entry.key().value().as_str())
            .collect::<Vec<_>>(),
        ["default", "retries", "enabled", "empty"]
    );
    assert!(!format!("{mapping:?}").contains("/private/socket"));
    for rendered in [format!("{document:?}"), format!("{:?}", parsed.diagnostics())] {
        for secret in ["default", "id=deploy", "/private/key", "/private/socket"] {
            assert!(!rendered.contains(secret), "sensitive authored input leaked: {secret}");
        }
    }
    assert!(
        document
            .service("wrong")
            .and_then(compose_lens::model::Service::build)
            .is_some_and(|build| matches!(build, Build::Definition(definition) if definition.ssh().is_none()))
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == BUILD_SSH_DUPLICATE_ITEM)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == BUILD_SSH_EXPECTED_FORM)
    );
    Ok(())
}

#[test]
fn retains_authored_blkio_config_members_and_partial_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(
        SourceId::new(3221),
        "services:\n  app:\n    blkio_config:\n      weight: 500\n      device_read_bps: [{path: /dev/a, rate: 1mb}, {path: /dev/a, rate: 2}]\n      device_read_iops: [{path: /dev/b, rate: 3}]\n      device_write_bps: [{path: /dev/c, rate: 4}]\n      device_write_iops: [{path: /dev/d, rate: 5}]\n      weight_device: [{path: /dev/e, weight: 600}]\n      x-retained: yes\n      future: kept\n",
    )?;
    let parsed = ComposeDocument::parse(syntax.document());
    let blkio = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::blkio_config)
        .ok_or("blkio config")?;
    assert!(
        matches!(blkio.weight().map(Located::value), Some(compose_lens::model::BlkioScalar::YamlInteger(value)) if value == "500")
    );
    assert_eq!(blkio.device_read_bps().len(), 2);
    assert_eq!(blkio.device_read_iops().len(), 1);
    assert_eq!(blkio.device_write_bps().len(), 1);
    assert_eq!(blkio.device_write_iops().len(), 1);
    assert_eq!(blkio.weight_device().len(), 1);
    assert_eq!(blkio.extension_fields().len(), 1);
    assert_eq!(blkio.unknown_fields().len(), 1);
    Ok(())
}

#[test]
fn retains_blkio_forms_duplicates_and_malformed_sequence_items() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  app:\n",
        "    blkio_config:\n",
        "      weight: 500\n",
        "      weight: \"600\"\n",
        "      device_read_bps:\n        - path: /dev/read\n          rate: \"100\"\n        - scalar-rate\n        - path: 1\n          rate: false\n          x-retained: yes\n          future: kept\n",
        "      device_read_iops: [{path: /dev/read-iops, rate: 1}]\n",
        "      device_write_bps: [{path: /dev/write, rate: \"2mb\"}]\n",
        "      device_write_iops: [{path: /dev/write-iops, rate: 3}]\n",
        "      weight_device:\n        - path: /dev/weight\n          weight: \"600\"\n        - scalar-weight\n        - path: /dev/duplicate\n          path: /dev/later\n          weight: 4\n",
        "  deferred:\n    blkio_config:\n      weight: \"${WEIGHT}\"\n",
        "  scalar: {blkio_config: 500}\n",
        "  list: {blkio_config: []}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(3226), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("typed document")?;
    let config = document
        .service("app")
        .and_then(compose_lens::model::Service::blkio_config)
        .ok_or("blkio config")?;

    assert!(matches!(
        config.weight().map(Located::value),
        Some(compose_lens::model::BlkioScalar::YamlInteger(value)) if value == "500"
    ));
    assert_eq!(config.device_read_iops().len(), 1);
    assert_eq!(config.device_write_bps().len(), 1);
    assert_eq!(config.device_write_iops().len(), 1);
    let read_rates = config.device_read_bps();
    assert_eq!(read_rates.len(), 3);
    assert!(matches!(
        read_rates[0].rate().map(Located::value),
        Some(compose_lens::model::BlkioScalar::String(value)) if value == "100"
    ));
    assert_eq!(
        read_rates[1].form(),
        compose_lens::model::BlkioDeviceRateForm::Unmodeled
    );
    assert!(!read_rates[1].span().range().is_empty());
    assert!(read_rates[2].rate().is_none());
    assert_eq!(read_rates[2].extension_fields().len(), 1);
    assert_eq!(read_rates[2].unknown_fields().len(), 2);

    let weights = config.weight_device();
    assert_eq!(weights.len(), 3);
    assert!(matches!(
        weights[0].weight().map(Located::value),
        Some(compose_lens::model::BlkioScalar::String(value)) if value == "600"
    ));
    assert_eq!(weights[1].form(), compose_lens::model::BlkioWeightDeviceForm::Unmodeled);
    assert_eq!(
        weights[2].path().map(Located::value).map(String::as_str),
        Some("/dev/duplicate")
    );
    let deferred = document
        .service("deferred")
        .and_then(compose_lens::model::Service::blkio_config)
        .and_then(compose_lens::model::BlkioConfig::weight)
        .ok_or("deferred weight")?;
    assert!(matches!(
        deferred.value(),
        compose_lens::model::BlkioScalar::String(value) if value == "${WEIGHT}"
    ));
    for service in ["scalar", "list"] {
        let service = document.service(service).ok_or("malformed service")?;
        assert!(service.blkio_config().is_none());
        assert!(
            service
                .unknown_fields()
                .iter()
                .any(|field| field.name().value() == "blkio_config")
        );
    }
    for code in [DUPLICATE_FIELD, EXPECTED_MAPPING, EXPECTED_SCALAR] {
        assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}
