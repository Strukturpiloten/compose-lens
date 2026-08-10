//! Pure policy checks for the isolated runtime-effect matrix.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use compose_lens::validation::ImplementationVersion;
use sha2::{Digest, Sha256};
use toml::{Table, Value};

const MATRIX: &str = include_str!("../conformance/runtime-effect-matrix.toml");
const PROVIDER_MATRIX: &str = include_str!("../conformance/provider-config-matrix.toml");

#[test]
fn runtime_effect_matrix_is_exact_isolated_and_complete() -> Result<(), String> {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = MATRIX
        .parse::<Table>()
        .map_err(|error| format!("invalid runtime matrix TOML: {error}"))?;
    assert_eq!(root.get("schema").and_then(Value::as_integer), Some(1));
    assert_eq!(root.get("reviewed").and_then(Value::as_str), Some("2026-07-31"));
    validate_isolation(&root)?;

    let provider_targets = provider_targets()?;
    let contexts = table_array(&root, "contexts")?;
    assert_eq!(contexts.len(), 18);
    let mut context_ids = BTreeSet::new();
    for context in contexts {
        let id = string(context, "id")?;
        if !context_ids.insert(id.to_owned()) {
            return Err(format!("duplicate runtime context `{id}`"));
        }
        let provider_target = string(context, "provider_target")?;
        let (matrix_provider, matrix_version) = provider_targets
            .get(provider_target)
            .ok_or_else(|| format!("context `{id}` references unknown provider target"))?;
        if string(context, "provider")? != matrix_provider || string(context, "provider_version")? != matrix_version {
            return Err(format!(
                "context `{id}` provider identity differs from the provider matrix"
            ));
        }
        let _: ImplementationVersion = string(context, "provider_version")?
            .parse()
            .map_err(|_| format!("context `{id}` provider version is not exact"))?;
        let runtime = string(context, "runtime")?;
        let runtime_version = string(context, "runtime_version")?;
        let _: ImplementationVersion = runtime_version
            .parse()
            .map_err(|_| format!("context `{id}` runtime version is not exact"))?;
        let expected_release = match runtime {
            "podman" => format!("https://github.com/podman-container-tools/podman/releases/tag/v{runtime_version}"),
            "docker-engine" if runtime_version == "29.6.2" => {
                "https://docs.docker.com/engine/release-notes/29/#2962".to_owned()
            }
            _ => return Err(format!("context `{id}` has unsupported runtime identity")),
        };
        if string(context, "runtime_release_url")? != expected_release {
            return Err(format!("context `{id}` runtime release URL is not exact"));
        }
        if !matches!(string(context, "privilege")?, "rootless" | "rootful")
            || string(context, "selinux")? != "enforcing"
            || string(context, "status")? != "planned"
        {
            return Err(format!("context `{id}` has an invalid execution dimension"));
        }
    }
    assert!(context_ids.iter().any(|id| id.contains("podman-540-rootless")));
    assert!(context_ids.iter().any(|id| id.contains("podman-602-rootful")));
    assert!(context_ids.iter().any(|id| id.contains("dockerengine-2962-rootless")));

    let probes = table_array(&root, "probes")?;
    assert_eq!(probes.len(), 2);
    let mut probe_ids = BTreeSet::new();
    for probe in probes {
        let id = string(probe, "id")?;
        if !probe_ids.insert(id.to_owned()) {
            return Err(format!("duplicate runtime probe `{id}`"));
        }
        let fixture = string(probe, "fixture")?;
        if !safe_relative(fixture) {
            return Err(format!("runtime probe `{id}` fixture path is unsafe"));
        }
        let files = strings(probe, "files")?;
        let expected = string(probe, "fixture_sha256")?;
        let actual = fixture_sha256(&repository.join(fixture), &files)
            .map_err(|error| format!("runtime probe `{id}` hash failed: {error}"))?;
        if actual != expected {
            return Err(format!("runtime probe `{id}` fixture SHA-256 differs"));
        }
        let _observation = string(probe, "observation")?;
    }

    let runs = table_array(&root, "runs")?;
    assert_eq!(runs.len(), context_ids.len() * probe_ids.len());
    let mut run_ids = BTreeSet::new();
    for run in runs {
        let context = string(run, "context")?;
        let probe = string(run, "probe")?;
        if !context_ids.contains(context) || !probe_ids.contains(probe) {
            return Err("runtime run references an unknown context or probe".to_owned());
        }
        if string(run, "status")? != "planned" {
            return Err("unreviewed runtime effects must remain planned".to_owned());
        }
        if !run_ids.insert((context.to_owned(), probe.to_owned())) {
            return Err("duplicate runtime matrix run".to_owned());
        }
    }
    for context in &context_ids {
        for probe in &probe_ids {
            assert!(run_ids.contains(&(context.clone(), probe.clone())));
        }
    }
    Ok(())
}

fn validate_isolation(root: &Table) -> Result<(), String> {
    let isolation = root
        .get("isolation")
        .and_then(Value::as_table)
        .ok_or("runtime matrix isolation table is missing")?;
    for (field, expected) in [
        ("result_directory", "new-and-empty"),
        ("workspace", "inside-result-directory"),
        ("host_source", "inside-workspace"),
        ("network", "none"),
        ("image", "caller-supplied-preloaded-digest-pinned"),
        ("selinux", "enforcing"),
        (
            "cleanup",
            "always-run-provider-down-then-audit-runtime-project-resources",
        ),
        (
            "failure_policy",
            "retain-observation-and-cleanup-results-without-promoting-support",
        ),
    ] {
        if string(isolation, field)? != expected {
            return Err(format!("runtime isolation `{field}` is not fail-closed"));
        }
    }
    if isolation.get("registry_access").and_then(Value::as_bool) != Some(false) {
        return Err("runtime isolation must forbid registry access".to_owned());
    }
    Ok(())
}

fn provider_targets() -> Result<BTreeMap<String, (String, String)>, String> {
    let root = PROVIDER_MATRIX
        .parse::<Table>()
        .map_err(|error| format!("invalid provider matrix TOML: {error}"))?;
    table_array(&root, "targets")?
        .into_iter()
        .map(|target| {
            Ok((
                string(target, "id")?.to_owned(),
                (
                    string(target, "provider")?.to_owned(),
                    string(target, "version")?.to_owned(),
                ),
            ))
        })
        .collect()
}

fn fixture_sha256(directory: &Path, files: &[String]) -> Result<String, std::io::Error> {
    let mut digest = Sha256::new();
    for file in files {
        digest.update(file.as_bytes());
        digest.update([0]);
        digest.update(fs::read(directory.join(file))?);
        digest.update([0]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn table_array<'a>(root: &'a Table, field: &str) -> Result<Vec<&'a Table>, String> {
    root.get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("runtime matrix `{field}` must be an array of tables"))?
        .iter()
        .map(|value| {
            value
                .as_table()
                .ok_or_else(|| format!("runtime matrix `{field}` entry is not a table"))
        })
        .collect()
}

fn string<'a>(table: &'a Table, field: &str) -> Result<&'a str, String> {
    table
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("runtime matrix `{field}` must be a non-empty string"))
}

fn strings(table: &Table, field: &str) -> Result<Vec<String>, String> {
    table
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("runtime matrix `{field}` must be an array"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| format!("runtime matrix `{field}` must contain strings"))
        })
        .collect()
}

fn safe_relative(value: &str) -> bool {
    let path = Path::new(value);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}
