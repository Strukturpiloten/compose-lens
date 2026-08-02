//! Public Phase 3 processing behavior.

use compose_lens::diagnostic::{Diagnostic, Severity};
use compose_lens::interpolation::{
    EmptyEnvironment, EnvironmentProvider, EnvironmentValue, INVALID_EXPRESSION, InterpolationInput,
    InterpolationOptions, MapEnvironment, MissingVariablePolicy, NESTING_LIMIT, REQUIRED_VARIABLE, SubstitutionOutcome,
    UNSET_VARIABLE, interpolate, interpolate_document, interpolate_with_options,
};
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject, ProjectLoadError};
use compose_lens::merge::{
    EntrySyntax, INTERPOLATION_PROJECT_MISMATCH, MergeOperation, MergedEntry, MergedProject, MergedScalar, MergedValue,
    merge_project,
};
use compose_lens::source::{SourceId, SourceSpan};
use std::path::Path;

const OPERATOR_CASES: &str = include_str!("../fixtures/processing/interpolation-operators/cases.txt");
const DOCUMENT_INTERPOLATION: &str = include_str!("../fixtures/processing/document-interpolation/compose.yaml");
const MULTI_FILE_BASE: &str = include_str!("../fixtures/processing/multi-file-loading/compose.yaml");
const MULTI_FILE_OVERRIDE: &str = include_str!("../fixtures/processing/multi-file-loading/compose.override.yaml");
const MERGE_BASE: &str = include_str!("../fixtures/processing/multi-file-merge/compose.yaml");
const MERGE_OVERRIDE: &str = include_str!("../fixtures/processing/multi-file-merge/compose.override.yaml");
const TAG_BASE: &str = include_str!("../fixtures/processing/merge-tags/compose.yaml");
const TAG_OVERRIDE: &str = include_str!("../fixtures/processing/merge-tags/compose.override.yaml");

fn span(text: &str) -> Result<SourceSpan, Box<dyn std::error::Error>> {
    SourceSpan::new(SourceId::new(61), 0, text.len()).ok_or_else(|| "valid test span expected".into())
}

fn environment() -> MapEnvironment {
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("SET", "value");
    let _ = environment.insert("EMPTY", "");
    let _ = environment.insert("FALLBACK", "fallback");
    environment
}

#[test]
fn evaluates_the_authored_operator_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let environment = environment();
    for (line_index, line) in OPERATOR_CASES.lines().enumerate() {
        let (input, expected) = line
            .split_once(" => ")
            .ok_or_else(|| format!("fixture line {} has no separator", line_index + 1))?;
        let expected = if expected == "<empty>" { "" } else { expected };
        let result = interpolate(InterpolationInput::new(input, span(input)?), &environment);

        assert_eq!(result.resolved(), expected, "fixture line {}", line_index + 1);
        assert!(
            result.is_valid(),
            "fixture line {}: {:#?}",
            line_index + 1,
            result.diagnostics()
        );
        assert!(result.diagnostics().is_empty(), "fixture line {}", line_index + 1);
    }
    Ok(())
}

#[test]
fn applies_explicit_missing_and_required_variable_policies() -> Result<(), Box<dyn std::error::Error>> {
    let environment = EmptyEnvironment;
    let direct = "image:${MISSING}";
    let direct_result = interpolate(InterpolationInput::new(direct, span(direct)?), &environment);
    assert_eq!(direct_result.resolved(), "image:");
    assert!(direct_result.is_valid());
    assert_eq!(direct_result.diagnostics()[0].code(), UNSET_VARIABLE);
    assert_eq!(direct_result.diagnostics()[0].severity(), Severity::Warning);

    let required = "${REQUIRED:?caller supplied secret detail}";
    let required_result = interpolate(InterpolationInput::new(required, span(required)?), &environment);
    assert_eq!(required_result.resolved(), required);
    assert!(!required_result.is_valid());
    assert_eq!(required_result.diagnostics()[0].code(), REQUIRED_VARIABLE);
    assert!(!diagnostic_text(&required_result.diagnostics()[0]).contains("caller supplied secret detail"));

    let options = InterpolationOptions::new(MissingVariablePolicy::Error);
    let strict = interpolate_with_options(InterpolationInput::new(direct, span(direct)?), &environment, options);
    assert_eq!(strict.resolved(), direct);
    assert!(!strict.is_valid());
    assert_eq!(strict.diagnostics()[0].severity(), Severity::Error);

    let preserve_options = InterpolationOptions::new(MissingVariablePolicy::PreserveWithWarning);
    let preserved = interpolate_with_options(
        InterpolationInput::new(direct, span(direct)?),
        &environment,
        preserve_options,
    );
    assert_eq!(preserved.resolved(), direct);
    assert!(preserved.is_valid());
    assert_eq!(preserved.diagnostics()[0].severity(), Severity::Warning);
    Ok(())
}

#[test]
fn required_operators_distinguish_unset_from_empty() -> Result<(), Box<dyn std::error::Error>> {
    let environment = environment();
    let unset_only = "${EMPTY?redacted-message}";
    let unset_only_result = interpolate(InterpolationInput::new(unset_only, span(unset_only)?), &environment);
    assert_eq!(unset_only_result.resolved(), "");
    assert!(unset_only_result.is_valid());

    let unset_or_empty = "${EMPTY:?redacted-message}";
    let unset_or_empty_result = interpolate(
        InterpolationInput::new(unset_or_empty, span(unset_or_empty)?),
        &environment,
    );
    assert_eq!(unset_or_empty_result.resolved(), unset_or_empty);
    assert!(!unset_or_empty_result.is_valid());
    assert_eq!(unset_or_empty_result.diagnostics()[0].code(), REQUIRED_VARIABLE);
    assert!(!diagnostic_text(&unset_or_empty_result.diagnostics()[0]).contains("redacted-message"));
    Ok(())
}

#[test]
fn propagates_sensitivity_only_when_sensitive_content_is_inserted() -> Result<(), Box<dyn std::error::Error>> {
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("TOKEN", "top-secret");

    let direct = "token=${TOKEN}";
    let direct_result = interpolate(InterpolationInput::new(direct, span(direct)?), &environment);
    assert_eq!(direct_result.original(), direct);
    assert_eq!(direct_result.span(), span(direct)?);
    assert_eq!(direct_result.resolved(), "token=top-secret");
    assert!(direct_result.is_sensitive());
    assert!(direct_result.substitutions()[0].is_sensitive());
    assert_eq!(
        direct_result.substitutions()[0].outcome(),
        SubstitutionOutcome::Environment
    );

    let alternative = "${TOKEN:+configured}";
    let alternative_result = interpolate(InterpolationInput::new(alternative, span(alternative)?), &environment);
    assert_eq!(alternative_result.resolved(), "configured");
    assert!(!alternative_result.is_sensitive());

    let sensitive_literal = "authored-secret";
    let literal_result = interpolate(
        InterpolationInput::new(sensitive_literal, span(sensitive_literal)?).sensitive(),
        &EmptyEnvironment,
    );
    assert!(literal_result.is_sensitive());
    Ok(())
}

#[test]
fn recovers_unsupported_and_overly_nested_expressions() -> Result<(), Box<dyn std::error::Error>> {
    let environment = environment();
    let unsupported = "${SET/foo/bar}";
    let unsupported_result = interpolate(InterpolationInput::new(unsupported, span(unsupported)?), &environment);
    assert_eq!(unsupported_result.resolved(), unsupported);
    assert_eq!(unsupported_result.diagnostics()[0].code(), INVALID_EXPRESSION);

    let unclosed = "${SET";
    let unclosed_result = interpolate(InterpolationInput::new(unclosed, span(unclosed)?), &environment);
    assert_eq!(unclosed_result.resolved(), unclosed);
    assert_eq!(unclosed_result.diagnostics()[0].code(), INVALID_EXPRESSION);

    let nested = "${MISSING:-${SECOND:-${SET}}}";
    let options = InterpolationOptions::default().with_max_nesting(1);
    let nested_result = interpolate_with_options(InterpolationInput::new(nested, span(nested)?), &environment, options);
    assert!(!nested_result.is_valid());
    assert!(
        nested_result
            .diagnostics()
            .iter()
            .any(|value| value.code() == NESTING_LIMIT)
    );
    Ok(())
}

#[test]
fn providers_are_caller_owned_and_composable_by_contract() {
    struct FixedProvider;

    impl EnvironmentProvider for FixedProvider {
        fn get(&self, name: &str) -> Option<EnvironmentValue> {
            (name == "FIXED").then(|| EnvironmentValue::plain("provided"))
        }
    }

    let value = FixedProvider.get("FIXED");
    assert_eq!(value.as_ref().map(EnvironmentValue::value), Some("provided"));
    assert_eq!(EmptyEnvironment.get("FIXED"), None);

    let mut mapped = MapEnvironment::new();
    assert!(mapped.is_empty());
    let _ = mapped.insert_value("FIXED", EnvironmentValue::plain("mapped"));
    assert_eq!(mapped.len(), 1);
}

#[test]
fn applies_a_non_destructive_overlay_to_yaml_values_only() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = compose_lens::syntax::SyntaxDocument::parse(SourceId::new(67), DOCUMENT_INTERPOLATION)?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("TAG", "1.2");
    let _ = environment.insert_sensitive("LABEL_VALUE", "confidential");
    let overlay = interpolate_document(syntax.document(), &environment);
    let originals: Vec<_> = overlay
        .values()
        .iter()
        .map(compose_lens::interpolation::InterpolationResult::original)
        .collect();

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert_eq!(overlay.source_id(), SourceId::new(67));
    assert!(overlay.is_valid());
    assert!(originals.contains(&"example/app:${TAG}"));
    assert!(originals.contains(&"future-${FUTURE:-enabled}"));
    assert!(originals.contains(&"${LABEL_VALUE}"));
    assert!(!originals.contains(&"echo ${SINGLE_QUOTED}"));
    assert!(!originals.contains(&"${LABEL_KEY}"));
    assert_eq!(resolved(&overlay, "example/app:${TAG}"), Some("example/app:1.2"));
    assert_eq!(resolved(&overlay, "RUNTIME=$$RUNTIME"), Some("RUNTIME=$RUNTIME"));
    assert_eq!(resolved(&overlay, "BLOCKED=${MISSING}"), Some("BLOCKED="));
    let sensitive = overlay
        .values()
        .iter()
        .find(|value| value.original() == "${LABEL_VALUE}")
        .ok_or("sensitive label value is missing")?;
    assert!(sensitive.is_sensitive());
    assert_eq!(overlay.value(sensitive.span()), Some(sensitive));
    assert_eq!(syntax.document().render_preserved(), DOCUMENT_INTERPOLATION);
    Ok(())
}

#[test]
fn loads_documents_in_order_and_retains_all_origins() -> Result<(), Box<dyn std::error::Error>> {
    let project = loaded_project()?;

    assert_eq!(project.documents().len(), 2);
    assert_eq!(project.base_directory(), Path::new("workspace/project"));
    assert_eq!(project.documents()[0].source_id(), SourceId::new(71));
    assert_eq!(project.documents()[0].origin().label(), "compose.yaml");
    assert_eq!(project.documents()[1].source_id(), SourceId::new(72));
    assert_eq!(
        project.documents()[1].origin().directory(),
        Path::new("workspace/overrides")
    );
    assert_eq!(
        project
            .document(SourceId::new(72))
            .map(|document| document.syntax().source_text()),
        Some(MULTI_FILE_OVERRIDE)
    );
    assert!(project.is_valid(), "{:#?}", project.diagnostics());
    assert!(project.diagnostics().is_empty());
    Ok(())
}

#[test]
fn interpolates_each_loaded_file_before_any_merge() -> Result<(), Box<dyn std::error::Error>> {
    let project = loaded_project()?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("TAG", "3.2");
    let overlays = project.interpolate(&environment);

    assert_eq!(overlays.documents().len(), 2);
    assert_eq!(
        overlays
            .document(SourceId::new(71))
            .and_then(|document| resolved(document, "example/base:${TAG}")),
        Some("example/base:3.2")
    );
    assert_eq!(
        overlays
            .document(SourceId::new(72))
            .and_then(|document| resolved(document, "example/override:${TAG:-latest}")),
        Some("example/override:3.2")
    );
    assert!(overlays.is_valid(), "{:#?}", overlays.diagnostics());
    assert_eq!(project.documents()[0].syntax().render_preserved(), MULTI_FILE_BASE);
    assert_eq!(project.documents()[1].syntax().render_preserved(), MULTI_FILE_OVERRIDE);
    Ok(())
}

#[test]
fn rejects_empty_projects_and_duplicate_source_ids() {
    assert_eq!(LoadedProject::load([]), Err(ProjectLoadError::EmptyProject));

    let duplicate = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(73),
            DocumentOrigin::new("first.yaml", "project"),
            "services: {}\n",
        ),
        DocumentInput::new(
            SourceId::new(73),
            DocumentOrigin::new("second.yaml", "override"),
            "services: {}\n",
        ),
    ]);
    assert!(matches!(
        duplicate,
        Err(ProjectLoadError::DuplicateSourceId {
            source_id,
            first_origin,
            duplicate_origin,
        }) if source_id == SourceId::new(73)
            && first_origin == "first.yaml"
            && duplicate_origin == "second.yaml"
    ));
}

#[test]
fn keeps_recoverable_parse_diagnostics_attached_to_the_loaded_document() -> Result<(), Box<dyn std::error::Error>> {
    let project = LoadedProject::load([DocumentInput::new(
        SourceId::new(74),
        DocumentOrigin::new("broken.yaml", "project"),
        "services: [app\n",
    )])?;

    assert_eq!(project.documents().len(), 1);
    assert!(!project.documents()[0].syntax_diagnostics().is_empty());
    assert!(!project.documents()[0].is_valid());
    assert!(!project.is_valid());
    assert!(!project.diagnostics().is_empty());
    Ok(())
}

#[test]
fn merges_compose_fields_with_order_fidelity_and_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let project = merge_fixture_project(MERGE_BASE, MERGE_OVERRIDE, 81)?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("TAG", "3.2");
    let interpolation = project.interpolate(&environment);
    let result = merge_project(&project, Some(&interpolation));
    let merged = result.project().ok_or("merged project expected")?;

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert_eq!(merged.base_directory(), Path::new("workspace/project"));
    assert_eq!(merged.source_ids(), &[SourceId::new(81), SourceId::new(82)]);
    assert_eq!(
        merged_scalar(merged.value(&["services", "app", "image"])),
        Some("example/override:3.2")
    );
    assert_eq!(
        merged
            .value(&["services", "app", "image"])
            .map(|value| value.provenance().operation()),
        Some(MergeOperation::Replaced)
    );

    let command = merged
        .value(&["services", "app", "command"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged command expected")?;
    assert_eq!(command.len(), 2);
    assert_eq!(command[1].as_scalar().map(MergedScalar::value), Some("override"));
    assert_eq!(
        merged
            .value(&["services", "app", "command"])
            .map(|value| value.provenance().operation()),
        Some(MergeOperation::Replaced)
    );
    assert_eq!(
        merged_scalar(merged.value(&["services", "app", "restart"])),
        Some("unless-stopped")
    );
    assert_shell_command_merge(merged)?;

    let capabilities = merged
        .value(&["services", "app", "cap_add"])
        .and_then(MergedValue::as_sequence)
        .ok_or("appended capabilities expected")?;
    assert_eq!(capabilities.len(), 2);
    assert_eq!(
        merged
            .value(&["services", "app", "cap_add"])
            .map(|value| value.provenance().operation()),
        Some(MergeOperation::Appended)
    );

    assert_keyed_and_unique_merge(merged)?;

    assert_eq!(
        merged_scalar(merged.value(&["services", "app", "x-unknown", "nested"])),
        Some("override")
    );
    assert_eq!(
        merged_scalar(merged.value(&["services", "app", "x-unknown", "added"])),
        Some("true")
    );
    let image_sources: Vec<_> = merged
        .value(&["services", "app", "image"])
        .ok_or("image expected")?
        .provenance()
        .sources()
        .iter()
        .map(|span| span.source_id())
        .collect();
    assert_eq!(image_sources, vec![SourceId::new(81), SourceId::new(82)]);
    Ok(())
}

#[test]
fn applies_reset_and_override_tags_without_normal_merge() -> Result<(), Box<dyn std::error::Error>> {
    let project = merge_fixture_project(TAG_BASE, TAG_OVERRIDE, 91)?;
    let result = merge_project(&project, None);
    let merged = result.project().ok_or("merged project expected")?;

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    let command = merged
        .value(&["services", "app", "command"])
        .and_then(MergedValue::as_sequence)
        .ok_or("overridden command expected")?;
    assert_eq!(command.len(), 2);
    assert_eq!(command[1].as_scalar().map(MergedScalar::value), Some("override"));
    assert_eq!(
        merged
            .value(&["services", "app", "command"])
            .map(|value| value.provenance().operation()),
        Some(MergeOperation::Override)
    );

    let environment = merged
        .value(&["services", "app", "environment"])
        .ok_or("reset environment expected")?;
    assert_eq!(environment.as_mapping().map(<[MergedEntry]>::len), Some(0));
    assert_eq!(environment.provenance().operation(), MergeOperation::Reset);

    let ports = merged
        .value(&["services", "app", "ports"])
        .ok_or("reset ports expected")?;
    assert_eq!(ports.as_sequence().map(<[MergedValue]>::len), Some(0));
    assert_eq!(ports.provenance().operation(), MergeOperation::Reset);

    let volumes = merged
        .value(&["services", "app", "volumes"])
        .ok_or("overridden volumes expected")?;
    assert_eq!(volumes.as_sequence().map(<[MergedValue]>::len), Some(1));
    assert_eq!(volumes.provenance().operation(), MergeOperation::Override);
    Ok(())
}

#[test]
fn merge_results_redact_interpolated_sensitive_values_from_debug_output() -> Result<(), Box<dyn std::error::Error>> {
    let project = LoadedProject::load([DocumentInput::new(
        SourceId::new(101),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        "services:\n  app:\n    image: example/app:${TOKEN}\n",
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("TOKEN", "debug-secret-value");
    let interpolation = project.interpolate(&environment);
    let result = merge_project(&project, Some(&interpolation));
    let image = result
        .project()
        .and_then(|project| project.value(&["services", "app", "image"]))
        .ok_or("merged sensitive image expected")?;

    assert!(image.is_sensitive());
    assert_eq!(
        image.as_scalar().map(MergedScalar::value),
        Some("example/app:debug-secret-value")
    );
    assert!(!format!("{result:?}").contains("debug-secret-value"));
    Ok(())
}

#[test]
fn rejects_interpolation_overlays_from_another_project() -> Result<(), Box<dyn std::error::Error>> {
    let project = LoadedProject::load([DocumentInput::new(
        SourceId::new(111),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        "services: {}\n",
    )])?;
    let other = LoadedProject::load([DocumentInput::new(
        SourceId::new(112),
        DocumentOrigin::new("other.yaml", "workspace/other"),
        "services: {}\n",
    )])?;
    let interpolation = other.interpolate(&EmptyEnvironment);
    let result = merge_project(&project, Some(&interpolation));

    assert!(!result.is_valid());
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == INTERPOLATION_PROJECT_MISMATCH)
            .count(),
        2
    );
    Ok(())
}

fn loaded_project() -> Result<LoadedProject, ProjectLoadError> {
    LoadedProject::load([
        DocumentInput::new(
            SourceId::new(71),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            MULTI_FILE_BASE,
        ),
        DocumentInput::new(
            SourceId::new(72),
            DocumentOrigin::new("compose.override.yaml", "workspace/overrides"),
            MULTI_FILE_OVERRIDE,
        ),
    ])
}

fn merge_fixture_project(
    base: &'static str,
    override_source: &'static str,
    first_id: u32,
) -> Result<LoadedProject, ProjectLoadError> {
    LoadedProject::load([
        DocumentInput::new(
            SourceId::new(first_id),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(first_id + 1),
            DocumentOrigin::new("compose.override.yaml", "workspace/overrides"),
            override_source,
        ),
    ])
}

fn assert_keyed_and_unique_merge(merged: &MergedProject) -> Result<(), Box<dyn std::error::Error>> {
    let environment = merged
        .value(&["services", "app", "environment"])
        .ok_or("merged environment expected")?;
    assert_eq!(merged_scalar(environment.get("MAP_ONLY")), Some("base"));
    assert_eq!(merged_scalar(environment.get("SHARED")), Some("override"));
    assert_eq!(merged_scalar(environment.get("LIST_ONLY")), Some("added"));
    assert_eq!(entry_syntax(environment, "MAP_ONLY"), Some(EntrySyntax::Mapping));
    assert_eq!(entry_syntax(environment, "SHARED"), Some(EntrySyntax::ListKeyValue));

    let labels = merged
        .value(&["services", "app", "labels"])
        .ok_or("merged labels expected")?;
    assert_eq!(merged_scalar(labels.get("list.only")), Some("base"));
    assert_eq!(merged_scalar(labels.get("shared")), Some("override"));
    assert_eq!(merged_scalar(labels.get("map.only")), Some("added"));
    assert_eq!(entry_syntax(labels, "list.only"), Some(EntrySyntax::ListKeyValue));
    assert_eq!(entry_syntax(labels, "shared"), Some(EntrySyntax::Mapping));

    let ports = merged
        .value(&["services", "app", "ports"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged ports expected")?;
    assert_eq!(ports.len(), 3);
    assert_eq!(merged_scalar(ports[0].get("mode")), Some("host"));
    assert_eq!(ports[0].provenance().operation(), MergeOperation::Replaced);

    let volumes = merged
        .value(&["services", "app", "volumes"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged volumes expected")?;
    assert_eq!(volumes.len(), 3);
    assert_eq!(merged_scalar(volumes[0].get("target")), Some("/data"));
    assert_eq!(
        merged_scalar(volumes[0].get("bind").and_then(|bind| bind.get("selinux"))),
        Some("Z")
    );
    assert_eq!(volumes[0].provenance().operation(), MergeOperation::Replaced);

    let devices = merged
        .value(&["services", "app", "devices"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged devices expected")?;
    assert_eq!(devices.len(), 2);
    assert_eq!(
        devices[0].as_scalar().map(MergedScalar::value),
        Some("/dev/override:/dev/device:rw")
    );
    assert_eq!(devices[0].provenance().operation(), MergeOperation::Replaced);

    let configs = merged
        .value(&["services", "app", "configs"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged configs expected")?;
    assert_eq!(configs.len(), 1);
    assert_eq!(merged_scalar(configs[0].get("source")), Some("override-config"));
    assert_eq!(merged_scalar(configs[0].get("mode")), Some("0444"));
    assert_eq!(merged_scalar(configs[0].get("uid")), Some("1000"));
    assert_eq!(configs[0].provenance().operation(), MergeOperation::Merged);

    let secrets = merged
        .value(&["services", "app", "secrets"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged secrets expected")?;
    assert_eq!(secrets.len(), 1);
    assert_eq!(merged_scalar(secrets[0].get("source")), Some("replacement-secret"));
    assert_eq!(secrets[0].provenance().operation(), MergeOperation::Replaced);
    Ok(())
}

fn assert_shell_command_merge(merged: &MergedProject) -> Result<(), Box<dyn std::error::Error>> {
    let entrypoint = merged
        .value(&["services", "app", "entrypoint"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged entrypoint expected")?;
    assert_eq!(entrypoint.len(), 1);
    assert_eq!(
        entrypoint[0].as_scalar().map(MergedScalar::value),
        Some("/override-entrypoint")
    );
    assert_eq!(
        merged
            .value(&["services", "app", "entrypoint"])
            .map(|value| value.provenance().operation()),
        Some(MergeOperation::Replaced)
    );

    let healthcheck = merged
        .value(&["services", "app", "healthcheck"])
        .ok_or("merged healthcheck expected")?;
    let test = healthcheck
        .get("test")
        .and_then(MergedValue::as_sequence)
        .ok_or("merged healthcheck test expected")?;
    assert_eq!(test.len(), 2);
    assert_eq!(test[1].as_scalar().map(MergedScalar::value), Some("override-health"));
    assert_eq!(merged_scalar(healthcheck.get("interval")), Some("30s"));
    assert_eq!(merged_scalar(healthcheck.get("timeout")), Some("5s"));
    assert_eq!(
        healthcheck.get("test").map(|value| value.provenance().operation()),
        Some(MergeOperation::Replaced)
    );
    Ok(())
}

fn merged_scalar(value: Option<&MergedValue>) -> Option<&str> {
    value.and_then(MergedValue::as_scalar).map(MergedScalar::value)
}

fn entry_syntax(value: &MergedValue, key: &str) -> Option<EntrySyntax> {
    value
        .as_mapping()?
        .iter()
        .find(|entry| entry.key() == key)
        .map(MergedEntry::syntax)
}

fn resolved<'a>(overlay: &'a compose_lens::interpolation::DocumentInterpolation, original: &str) -> Option<&'a str> {
    overlay
        .values()
        .iter()
        .find(|value| value.original() == original)
        .map(compose_lens::interpolation::InterpolationResult::resolved)
}

fn diagnostic_text(diagnostic: &Diagnostic) -> String {
    let mut text = diagnostic.message().to_owned();
    for label in diagnostic.labels() {
        text.push_str(label.message());
    }
    for note in diagnostic.notes() {
        text.push_str(note);
    }
    text
}
