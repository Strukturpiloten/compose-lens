//! Regression coverage for the native service runtime/resource-key boundary.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::merge_project;
use compose_lens::model::{
    BooleanValue, CPU_RT_RUNTIME_INVALID, ComposeDocument, CpuRtRuntime, Cpus, IpcMode, MEMSWAP_LIMIT_EXPECTED_VALUE,
    MEMSWAP_LIMIT_INVALID, MemLimitUnit, MemswapLimitKind, MemswapLimitScalarKind, NetworkMode, PidMode,
    ServiceInteger,
};
use compose_lens::project::{
    ProjectService, ProjectValue, SERVICE_CPUS_DEPLOY_LIMIT_CONFLICT, SERVICE_MEMORY_RESERVATION_DEPLOY_CONFLICT,
    SERVICE_MEMSWAP_BELOW_MEMORY_LIMIT, SERVICE_MEMSWAP_REQUIRES_MEMORY_LIMIT, SERVICE_NETWORK_MODE_NETWORKS_CONFLICT,
    SERVICE_SCALE_DEPLOY_REPLICAS_CONFLICT, build_project_view,
};
use compose_lens::resolution::{ReferenceKind, ReferenceStatus, validate_references};
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;

const SOURCE: &str = concat!(
    "services:\n",
    "  app:\n",
    "    cpu_rt_runtime: 500us\n",
    "    cpu_shares: 1024\n",
    "    cpus: 0.000\n",
    "    cpuset: \"0-3\"\n",
    "    device_cgroup_rules: [\"c 1:3 mr\", \"c 1:3 mr\"]\n",
    "    ipc: service:db\n",
    "    mem_reservation: 64m\n",
    "    mem_swappiness: 100\n",
    "    memswap_limit: -1\n",
    "    network_mode: service:db\n",
    "    oom_kill_disable: \"${OOM}\"\n",
    "    oom_score_adj: -1000\n",
    "    pid: container:external\n",
    "    scale: 2\n",
    "    volumes_from: [db:ro, cache, cache]\n",
    "  db: {image: postgres}\n",
    "  bad: {mem_swappiness: 101, oom_score_adj: 1001, cpus: \"nope\"}\n",
);

#[test]
fn retains_authored_service_runtime_keys_and_invalid_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(9101), SOURCE)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let app = parsed
        .document()
        .and_then(|document| document.service("app"))
        .ok_or("app expected")?;
    assert!(
        matches!(app.cpus().map(compose_lens::model::Located::value), Some(Cpus::Decimal(value)) if value == "0.000")
    );
    assert!(
        matches!(app.ipc().map(compose_lens::model::Located::value), Some(IpcMode::Service(value)) if value == "db")
    );
    assert!(
        matches!(app.network_mode().map(compose_lens::model::Located::value), Some(NetworkMode::Service(value)) if value == "db")
    );
    assert!(
        matches!(app.pid().map(compose_lens::model::Located::value), Some(PidMode::Container(value)) if value == "external")
    );
    assert!(
        matches!(app.oom_kill_disable().map(compose_lens::model::Located::value), Some(BooleanValue::Expression(value)) if value == "${OOM}")
    );
    assert_eq!(app.device_cgroup_rules().len(), 2);
    assert_eq!(app.volumes_from().len(), 3);
    assert!(app.volumes_from()[0].read_only());
    let bad = parsed
        .document()
        .and_then(|document| document.service("bad"))
        .ok_or("bad expected")?;
    assert!(
        matches!(bad.mem_swappiness().map(compose_lens::model::Located::value), Some(ServiceInteger::OutOfRange(value)) if value == "101")
    );
    assert!(
        bad.cpus().is_none()
            || matches!(
                bad.cpus().map(compose_lens::model::Located::value),
                Some(Cpus::Other(_))
            )
    );
    for key in ["mem_swappiness", "oom_score_adj", "cpus"] {
        assert!(
            !bad.unknown_fields().iter().any(|field| field.name().value() == key),
            "typed malformed value should not be duplicated as unknown evidence: {key}"
        );
    }
    assert!(!parsed.diagnostics().is_empty());
    Ok(())
}

#[test]
fn exposes_merged_values_provenance_and_namespace_references() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9102),
        DocumentOrigin::new("compose.yaml", "workspace"),
        SOURCE,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("OOM", "false");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let project = merged.project().ok_or("merged project expected")?;
    let view_result = build_project_view(project, None);
    let view = view_result.view().ok_or("project view expected")?;
    let app = view.service("app").ok_or("app view expected")?;
    assert!(matches!(app.cpus().map(ProjectValue::value), Some(Cpus::Decimal(value)) if value == "0.000"));
    assert_eq!(app.device_cgroup_rules().ok_or("rules expected")?.value().len(), 2);
    assert_eq!(app.volumes_from().ok_or("volumes_from expected")?.value().len(), 3);
    assert!(ProjectService::mem_reservation(app).is_some());
    let bad = view.service("bad").ok_or("bad view expected")?;
    for key in ["mem_swappiness", "oom_score_adj", "cpus"] {
        assert!(
            !bad.unmodeled_fields()
                .iter()
                .any(|field| field.path() == ["services", "bad", key]),
            "typed malformed value should not be duplicated as unmodeled evidence: {key}"
        );
    }
    let references = validate_references(project, None);
    assert!(
        references
            .references()
            .iter()
            .any(|reference| reference.kind() == ReferenceKind::ServiceNamespace
                && reference.target() == "db"
                && reference.status() == ReferenceStatus::Found)
    );
    Ok(())
}

#[test]
fn reports_cross_field_conflicts_without_discarding_native_values() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9103),
        DocumentOrigin::new("compose.yaml", "workspace"),
        concat!(
            "services:\n",
            "  app:\n",
            "    cpus: \"0.5\"\n",
            "    mem_reservation: 64m\n",
            "    memswap_limit: 128m\n",
            "    network_mode: host\n",
            "    networks: [default]\n",
            "    scale: 2\n",
            "    deploy:\n",
            "      replicas: 3\n",
            "      resources:\n",
            "        limits: {cpus: \"1.0\"}\n",
            "        reservations: {memory: 32m}\n",
        ),
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let app = result
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("app expected")?;
    assert!(app.cpus().is_some() && app.memswap_limit().is_some() && app.network_mode().is_some());
    for code in [
        SERVICE_CPUS_DEPLOY_LIMIT_CONFLICT,
        SERVICE_MEMORY_RESERVATION_DEPLOY_CONFLICT,
        SERVICE_MEMSWAP_REQUIRES_MEMORY_LIMIT,
        SERVICE_NETWORK_MODE_NETWORKS_CONFLICT,
        SERVICE_SCALE_DEPLOY_REPLICAS_CONFLICT,
    ] {
        assert!(result.diagnostics().iter().any(|diagnostic| diagnostic.code() == code));
    }
    Ok(())
}

#[test]
fn compares_only_proven_equivalent_semantics_and_labels_both_conflicting_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9105),
        DocumentOrigin::new("compose.yaml", "workspace"),
        concat!(
            "services:\n",
            "  equal:\n    cpus: \"000.500\"\n    scale: 002\n    mem_reservation: 064m\n    deploy: {replicas: 2, resources: {limits: {cpus: 0.5}, reservations: {memory: 64m}}}\n",
            "  unproven: {cpus: \".5\", deploy: {resources: {limits: {cpus: 1}}}}\n",
            "  conflict:\n    cpus: \"0.5\"\n    scale: 2\n    mem_reservation: 64m\n    deploy: {replicas: 3, resources: {limits: {cpus: 1}, reservations: {memory: 32m}}}\n",
            "  malformed: {cpu_rt_runtime: 1.5}\n",
        ),
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let equal_diagnostics: Vec<_> = result
        .diagnostics()
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.code(),
                SERVICE_CPUS_DEPLOY_LIMIT_CONFLICT
                    | SERVICE_SCALE_DEPLOY_REPLICAS_CONFLICT
                    | SERVICE_MEMORY_RESERVATION_DEPLOY_CONFLICT
            )
        })
        .collect();
    assert_eq!(equal_diagnostics.len(), 3);
    assert!(
        equal_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.labels().len() == 2)
    );
    let syntax = SyntaxDocument::parse(SourceId::new(9106), "services: {app: {cpu_rt_runtime: 1.5}}")?;
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == CPU_RT_RUNTIME_INVALID)
    );
    assert!(
        !parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code().as_str().contains("cpu-rt-period"))
    );
    Ok(())
}

#[test]
fn retains_negative_cpu_rt_runtime_as_invalid_authored_and_effective_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let source = "services: {app: {cpu_rt_runtime: -500}}";
    let syntax = SyntaxDocument::parse(SourceId::new(9116), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::cpu_rt_runtime)
        .map(compose_lens::model::Located::value);
    assert!(matches!(authored, Some(CpuRtRuntime::Other(value)) if value == "-500"));
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == CPU_RT_RUNTIME_INVALID)
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9117),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let effective = build_project_view(merged.project().ok_or("project expected")?, None);
    let runtime = effective
        .view()
        .and_then(|view| view.service("app"))
        .and_then(ProjectService::cpu_rt_runtime)
        .map(ProjectValue::value);
    assert!(matches!(runtime, Some(CpuRtRuntime::Other(value)) if value == "-500"));
    assert!(
        effective
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == CPU_RT_RUNTIME_INVALID)
    );
    Ok(())
}

#[test]
fn validates_only_proven_memswap_branches_and_comparable_limits() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9109),
        DocumentOrigin::new("compose.yaml", "workspace"),
        concat!(
            "services:\n",
            "  requires-memory: {memswap_limit: 128m}\n",
            "  below-memory: {mem_limit: 64m, memswap_limit: 32m}\n",
            "  unlimited: {memswap_limit: -1}\n",
            "  zero: {memswap_limit: 0}\n",
            "  incomparable-units: {mem_limit: 1g, memswap_limit: 512m}\n",
        ),
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let diagnostics = result.diagnostics();
    let requires = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == SERVICE_MEMSWAP_REQUIRES_MEMORY_LIMIT)
        .ok_or("requires-memory diagnostic expected")?;
    assert_eq!(requires.labels().len(), 1);
    let below = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == SERVICE_MEMSWAP_BELOW_MEMORY_LIMIT)
        .ok_or("below-memory diagnostic expected")?;
    assert_eq!(below.labels().len(), 2);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code() == SERVICE_MEMSWAP_REQUIRES_MEMORY_LIMIT)
            .count(),
        1
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code() == SERVICE_MEMSWAP_BELOW_MEMORY_LIMIT)
            .count(),
        1
    );
    Ok(())
}

#[test]
fn retains_malformed_strict_sequence_items_alongside_valid_siblings() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n  app:\n",
        "    device_cgroup_rules: [\"c 1:3 mr\", true, 2, {}, null]\n",
        "    volumes_from: [db, false, 1, {}, null]\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(9107), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let app = parsed
        .document()
        .and_then(|document| document.service("app"))
        .ok_or("app expected")?;
    assert_eq!(app.device_cgroup_rules().len(), 1);
    assert_eq!(app.invalid_device_cgroup_rules().len(), 4);
    assert_eq!(app.volumes_from().len(), 1);
    assert_eq!(app.invalid_volumes_from().len(), 4);
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9108),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let app = result
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("view app expected")?;
    assert_eq!(app.invalid_device_cgroup_rules().len(), 4);
    assert_eq!(app.invalid_volumes_from().len(), 4);
    Ok(())
}

#[test]
fn retains_non_sequence_runtime_fields_as_field_level_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n  app:\n",
        "    device_cgroup_rules: c 1:3 mr\n",
        "    volumes_from: {source: db}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(9113), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("app"))
        .ok_or("authored service expected")?;
    assert!(authored.device_cgroup_rules().is_empty() && authored.volumes_from().is_empty());
    assert!(
        authored
            .unknown_fields()
            .iter()
            .any(|field| field.name().value() == "device_cgroup_rules")
    );
    assert!(
        authored
            .unknown_fields()
            .iter()
            .any(|field| field.name().value() == "volumes_from")
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9114),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let effective = result
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("effective service expected")?;
    assert!(effective.device_cgroup_rules().is_none() && effective.volumes_from().is_none());
    assert!(
        effective
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "app", "device_cgroup_rules"])
    );
    assert!(
        effective
            .unmodeled_fields()
            .iter()
            .any(|field| field.path() == ["services", "app", "volumes_from"])
    );
    Ok(())
}

#[test]
fn retains_all_unrepresentable_runtime_field_shapes_as_field_level_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    const KEYS: [&str; 15] = [
        "cpu_rt_runtime",
        "cpu_shares",
        "cpus",
        "cpuset",
        "device_cgroup_rules",
        "ipc",
        "mem_reservation",
        "mem_swappiness",
        "memswap_limit",
        "network_mode",
        "oom_kill_disable",
        "oom_score_adj",
        "pid",
        "scale",
        "volumes_from",
    ];
    let source = format!(
        "{SOURCE}{}",
        concat!(
            "  malformed-shapes:\n",
            "    cpu_rt_runtime: [500us]\n",
            "    cpu_shares: [1024]\n",
            "    cpus: [0.5]\n",
            "    cpuset: [\"${CPUSET}\"]\n",
            "    device_cgroup_rules: c 1:3 mr\n",
            "    ipc: [service:db]\n",
            "    mem_reservation: [64m]\n",
            "    mem_swappiness: [60]\n",
            "    memswap_limit: [64m]\n",
            "    network_mode: [host]\n",
            "    oom_kill_disable: [true]\n",
            "    oom_score_adj: [-5]\n",
            "    pid: [host]\n",
            "    scale: [2]\n",
            "    volumes_from: db\n",
        )
    );
    let syntax = SyntaxDocument::parse(SourceId::new(9117), source.as_str())?;
    let parsed = ComposeDocument::parse(syntax.document());
    let authored = parsed
        .document()
        .and_then(|document| document.service("malformed-shapes"))
        .ok_or("authored malformed service expected")?;
    for key in KEYS {
        let field = authored
            .unknown_fields()
            .iter()
            .find(|field| field.name().value() == key)
            .ok_or("authored field evidence expected")?;
        assert!(field.value_span().is_some(), "missing value span for {key}");
    }
    assert!(parsed.diagnostics().len() >= KEYS.len());
    let valid = parsed
        .document()
        .and_then(|document| document.service("app"))
        .ok_or("valid sibling service expected")?;
    assert!(valid.cpu_rt_runtime().is_some() && valid.memswap_limit().is_some());
    assert_eq!(valid.device_cgroup_rules().len(), 2);
    assert_eq!(valid.volumes_from().len(), 3);

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9118),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source.as_str(),
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("CPUSET", "0-3");
    let interpolation = loaded.interpolate(&environment);
    let merged = merge_project(&loaded, Some(&interpolation));
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    let effective = result
        .view()
        .and_then(|view| view.service("malformed-shapes"))
        .ok_or("effective malformed service expected")?;
    for key in KEYS {
        let field = effective
            .unmodeled_fields()
            .iter()
            .find(|field| field.path() == ["services", "malformed-shapes", key])
            .ok_or("effective field evidence expected")?;
        assert!(
            field.provenance().effective_source().is_some(),
            "missing provenance span for {key}"
        );
        if key == "cpuset" {
            assert!(field.is_sensitive());
        }
    }
    assert!(result.diagnostics().len() >= KEYS.len());
    let valid = result
        .view()
        .and_then(|view| view.service("app"))
        .ok_or("effective valid sibling service expected")?;
    assert!(valid.cpu_rt_runtime().is_some() && valid.memswap_limit().is_some());
    assert!(valid.device_cgroup_rules().is_some() && valid.volumes_from().is_some());
    Ok(())
}

#[test]
fn classifies_memswap_limits_without_mem_limit_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  unlimited: {memswap_limit: -1}\n",
        "  zero: {memswap_limit: 000m}\n",
        "  positive: {memswap_limit: 64m}\n",
        "  number: {memswap_limit: 64}\n",
        "  expression: {memswap_limit: \"${MEMSWAP}\"}\n",
        "  invalid: {memswap_limit: nope}\n",
        "  malformed: {memswap_limit: [64m]}\n",
    );
    let syntax = SyntaxDocument::parse(SourceId::new(9115), source)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let document = parsed.document().ok_or("document expected")?;
    assert!(matches!(
        document
            .service("unlimited")
            .and_then(compose_lens::model::Service::memswap_limit)
            .map(compose_lens::model::MemswapLimit::kind),
        Some(MemswapLimitKind::Unlimited)
    ));
    assert!(matches!(
        document.service("zero").and_then(compose_lens::model::Service::memswap_limit).map(compose_lens::model::MemswapLimit::kind),
        Some(MemswapLimitKind::Zero { amount_raw, unit: Some(MemLimitUnit::M) }) if amount_raw == "000"
    ));
    assert!(matches!(
        document.service("positive").and_then(compose_lens::model::Service::memswap_limit).map(compose_lens::model::MemswapLimit::kind),
        Some(MemswapLimitKind::Positive { amount_raw, unit: Some(MemLimitUnit::M) }) if amount_raw == "64"
    ));
    assert!(matches!(
        document.service("number").and_then(compose_lens::model::Service::memswap_limit),
        Some(value) if value.scalar_kind() == MemswapLimitScalarKind::Number
    ));
    assert!(matches!(
        document
            .service("expression")
            .and_then(compose_lens::model::Service::memswap_limit)
            .map(compose_lens::model::MemswapLimit::kind),
        Some(MemswapLimitKind::Expression)
    ));
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == MEMSWAP_LIMIT_INVALID)
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == MEMSWAP_LIMIT_EXPECTED_VALUE)
    );
    assert!(
        !parsed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code().as_str().starts_with("compose.mem-limit."))
    );

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(9116),
        DocumentOrigin::new("compose.yaml", "workspace"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let result = build_project_view(merged.project().ok_or("project expected")?, None);
    assert!(matches!(
        result
            .view()
            .and_then(|view| view.service("unlimited"))
            .and_then(ProjectService::memswap_limit)
            .map(|value| value.value().kind()),
        Some(MemswapLimitKind::Unlimited)
    ));
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == MEMSWAP_LIMIT_INVALID)
    );
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == MEMSWAP_LIMIT_EXPECTED_VALUE)
    );
    assert!(
        !result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code().as_str().starts_with("compose.mem-limit."))
    );
    Ok(())
}
