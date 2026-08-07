//! Public typed-model behavior and representation fidelity.

use compose_lens::model::{
    BooleanValue, Build, BuildFieldKind, CAP_ADD_DUPLICATE_ITEM, CAP_ADD_EXPECTED_SEQUENCE, CAP_ADD_EXPECTED_STRING,
    CAP_DROP_DUPLICATE_ITEM, CAP_DROP_EXPECTED_SEQUENCE, CAP_DROP_EXPECTED_STRING, Command, ComposeDocument,
    ComposeScalar, ConfigGrant, ContainerPathKind, DEPENDENCY_HEALTHCHECK_UNVERIFIED, DEPENDENCY_INVALID_CONDITION,
    DEPENDENCY_MISSING_HEALTHCHECK, DEPENDENCY_MISSING_SERVICE, DEVICE_EXPECTED_FORM, DEVICE_EXPECTED_STRING,
    DEVICE_MISSING_SOURCE, DEVICES_EXPECTED_SEQUENCE, DNS_EXPECTED_FORM, DNS_EXPECTED_STRING,
    DNS_SEARCH_DUPLICATE_ITEM, DNS_SEARCH_EXPECTED_FORM, DNS_SEARCH_EXPECTED_STRING, DependencyCondition,
    DeployFieldKind, Device, DnsForm, DnsSearchForm, ENVIRONMENT_FILE_EXPECTED_FORM, ENVIRONMENT_FILE_INVALID_FORMAT,
    ENVIRONMENT_FILE_MISSING_PATH, EXPECTED_BOOLEAN, EXPECTED_FIELD_FORM, EXPECTED_MAPPING, EXPECTED_SCALAR,
    EXPECTED_SEQUENCE, EXTRA_HOST_INVALID_ENTRY, Entrypoint, Environment, EnvironmentFile, EnvironmentFileFormatKind,
    ExtraHostSeparator, ExtraHosts, GRANT_EXPECTED_FORM, GRANT_MISSING_SOURCE, HEALTHCHECK_INVALID_DURATION,
    HEALTHCHECK_INVALID_RETRIES, HEALTHCHECK_INVALID_TEST, HealthcheckTestKind, HostAddressKind, HostnameKind,
    IdentityComponent, Labels, LimitValue, Located, MountType, PORT_EXPECTED_FORM, PORT_MISSING_TARGET, Port,
    RESOURCE_EXPECTED_FORM, RESTART_INVALID_POLICY, RestartPolicyKind, STOP_GRACE_PERIOD_INVALID,
    SYSCTLS_DUPLICATE_ITEM, SYSCTLS_EMPTY_KEY, SYSCTLS_EXPECTED_FORM, SYSCTLS_EXPECTED_SCALAR, SYSCTLS_EXPECTED_STRING,
    SecretGrant, SelinuxRelabel, ServiceNetworks, StopGracePeriod, SysctlsForm, ULIMIT_INVALID_NAME,
    ULIMIT_INVALID_VALUE, ULIMIT_MISSING_RANGE_MEMBER, UlimitValue, UserNamespaceModeKind, VOLUME_EXPECTED_FORM,
    VOLUME_INVALID_SELINUX, VOLUME_MISSING_TARGET, VOLUME_MISSING_TYPE, VolumeMount, VolumeSyntax,
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
    assert!(
        document
            .service("schema-refresh")
            .ok_or("schema refresh service expected")?
            .unknown_fields()
            .iter()
            .any(|field| field.name().value() == "pull_refresh_after")
    );
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
            .any(|diagnostic| diagnostic.code() == compose_lens::model::DUPLICATE_FIELD)
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
