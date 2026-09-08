//! Strict helpers for independently asserted application fixtures.

use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use toml::{Table, Value};

pub(crate) struct CopiedFile<'a> {
    pub(crate) path: &'a str,
    pub(crate) hash_key: &'a str,
    pub(crate) sha256: &'a str,
}

pub(crate) fn verify_application_fixture(
    id: &str,
    revision: &str,
    copied: &[CopiedFile<'_>],
    authored: &[&str],
) -> Result<PathBuf, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/real-world")
        .join(id);
    let manifest_path = root.join("fixture.toml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
    let manifest = manifest_text
        .parse::<Table>()
        .map_err(|error| format!("invalid {}: {error}", manifest_path.display()))?;

    require_integer(&manifest, "schema", 1)?;
    require_string(&manifest, "id", id)?;
    require_string(&manifest, "suite", "real-world")?;
    if manifest.get("secrets_reviewed").and_then(Value::as_bool) != Some(true) {
        return Err(format!("{id}: secrets_reviewed must be true"));
    }

    let provenance = require_table(&manifest, "provenance")?;
    require_string(provenance, "source", "external")?;
    require_string(provenance, "revision", revision)?;
    require_string(provenance, "license", "MPL-2.0")?;
    require_string(provenance, "redistribution", "allowed")?;
    let url = string(provenance, "url")?;
    if !url.starts_with("https://github.com/Strukturpiloten/boxferry/tree/") || !url.contains(revision) {
        return Err(format!("{id}: provenance URL must pin {revision}"));
    }

    let listed = string_array(&manifest, "files")?;
    let declared = copied
        .iter()
        .map(|file| file.path)
        .chain(authored.iter().copied())
        .collect::<BTreeSet<_>>();
    if declared.len() != copied.len() + authored.len() {
        return Err(format!("{id}: copied/authored file declarations overlap"));
    }
    if listed != declared {
        return Err(format!(
            "{id}: manifest files differ from independently declared fixture files"
        ));
    }
    verify_directory_members(&root, &listed)?;

    let application = require_table(require_table(&manifest, "extensions")?, "application")?;
    require_string(
        application,
        "source-repository",
        "https://github.com/Strukturpiloten/boxferry",
    )?;
    let evidence = string(application, "evidence")?;
    if !evidence.contains("not inferred") {
        return Err(format!(
            "{id}: fixture must deny inference of provider/runtime evidence"
        ));
    }
    for file in copied {
        require_string(application, file.hash_key, file.sha256)?;
        let actual = sha256(&root.join(file.path))?;
        if actual != file.sha256 {
            return Err(format!(
                "{id}: {} hash mismatch: expected {}, got {actual}",
                file.path, file.sha256
            ));
        }
    }

    let license = fs::read_to_string(root.join("UPSTREAM_LICENSE"))
        .map_err(|error| format!("{id}: failed to read upstream license: {error}"))?;
    if !license.starts_with("Mozilla Public License Version 2.0") || license.len() < 15_000 {
        return Err(format!("{id}: incomplete MPL-2.0 license copy"));
    }
    Ok(root)
}

pub(crate) fn read_fixture(root: &Path, path: &str) -> Result<String, String> {
    if !is_safe_relative(path) {
        return Err(format!("unsafe fixture path: {path}"));
    }
    fs::read_to_string(root.join(path)).map_err(|error| format!("failed to read {path}: {error}"))
}

fn verify_directory_members(root: &Path, listed: &BTreeSet<&str>) -> Result<(), String> {
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(root).map_err(|error| format!("{}: {error}", root.display()))? {
        let entry = entry.map_err(|error| format!("{}: {error}", root.display()))?;
        if !entry
            .file_type()
            .map_err(|error| format!("{}: {error}", entry.path().display()))?
            .is_file()
        {
            return Err(format!("{}: nested entries are forbidden", entry.path().display()));
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("{}: non-UTF-8 filename", entry.path().display()))?;
        if name != "fixture.toml" {
            actual.insert(name);
        }
    }
    let expected = listed.iter().copied().map(str::to_owned).collect();
    if actual != expected {
        return Err(format!(
            "{}: directory members do not match fixture.toml",
            root.display()
        ));
    }
    Ok(())
}

fn sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn require_table<'a>(table: &'a Table, field: &str) -> Result<&'a Table, String> {
    table
        .get(field)
        .and_then(Value::as_table)
        .ok_or_else(|| format!("{field} must be a table"))
}

fn string<'a>(table: &'a Table, field: &str) -> Result<&'a str, String> {
    table
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{field} must be a non-empty string"))
}

fn require_string(table: &Table, field: &str, expected: &str) -> Result<(), String> {
    let actual = string(table, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{field}: expected {expected:?}, got {actual:?}"))
    }
}

fn require_integer(table: &Table, field: &str, expected: i64) -> Result<(), String> {
    let actual = table.get(field).and_then(Value::as_integer);
    if actual == Some(expected) {
        Ok(())
    } else {
        Err(format!("{field}: expected integer {expected}, got {actual:?}"))
    }
}

fn string_array<'a>(table: &'a Table, field: &str) -> Result<BTreeSet<&'a str>, String> {
    let values = table
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{field} must be an array"))?;
    let mut result = BTreeSet::new();
    for value in values {
        let value = value
            .as_str()
            .filter(|value| is_safe_relative(value))
            .ok_or_else(|| format!("{field} contains an unsafe non-string path"))?;
        if !result.insert(value) {
            return Err(format!("{field} contains duplicate {value}"));
        }
    }
    Ok(result)
}

fn is_safe_relative(value: &str) -> bool {
    !value.is_empty()
        && !Path::new(value).is_absolute()
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}
