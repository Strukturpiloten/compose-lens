//! Deterministic canonical rendering and recovery behavior.

use compose_lens::interpolation::MapEnvironment;
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::{MergedProject, merge_project};
use compose_lens::profiles::{ProfileRequest, select_profiles};
use compose_lens::render::{
    CanonicalFormatting, IndentWidth, LineEnding, UNRENDERABLE_ALIAS, render_canonical,
    render_canonical_with_formatting,
};
use compose_lens::resolution::SELECTION_PROJECT_MISMATCH;
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;

const BASE: &str = include_str!("../fixtures/roundtrip/canonical-merged/compose.yaml");
const OVERRIDE: &str = include_str!("../fixtures/roundtrip/canonical-merged/compose.override.yaml");
const EXPECTED: &str = include_str!("../fixtures/roundtrip/canonical-merged/expected.yaml");

#[test]
fn renders_the_merged_project_in_one_deterministic_form() -> Result<(), Box<dyn std::error::Error>> {
    let project = canonical_project()?;
    let first = render_canonical(&project, None);
    let second = render_canonical(&project, None);

    assert!(first.is_valid(), "{:#?}", first.diagnostics());
    assert_eq!(first.output(), EXPECTED);
    assert_eq!(first, second);
    assert!(!first.is_sensitive());
    Ok(())
}

#[test]
fn canonical_output_is_stable_across_parse_merge_render() -> Result<(), Box<dyn std::error::Error>> {
    let project = canonical_project()?;
    let first = render_canonical(&project, None);
    let syntax = SyntaxDocument::parse(SourceId::new(263), first.output())?;
    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());

    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(264),
        DocumentOrigin::new("canonical.yaml", "workspace/project"),
        first.output(),
    )])?;
    let merge = merge_project(&loaded, None);
    let reparsed = merge.project().ok_or("reparsed merged project expected")?;
    let second = render_canonical(reparsed, None);

    assert!(second.is_valid(), "{:#?}", second.diagnostics());
    assert_eq!(second.output(), first.output());
    Ok(())
}

#[test]
fn default_formatting_is_byte_identical_to_canonical_v2() -> Result<(), Box<dyn std::error::Error>> {
    let project = canonical_project()?;
    let canonical = render_canonical(&project, None);
    let formatted = render_canonical_with_formatting(&project, None, &CanonicalFormatting::default());

    assert!(canonical.output().starts_with("---\n"));
    assert_eq!(formatted, canonical);
    Ok(())
}

#[test]
fn canonical_rendering_uses_minimal_safe_string_quoting_and_preserves_every_string_on_parse_back()
-> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "services:\n",
        "  web:\n",
        "    image: example.invalid/web:1\n",
        "    environment:\n",
        "      plain: plain-value\n",
        "      image: example.invalid/web:1\n",
        "      \"yes\": \"yes\"\n",
        "      \"no\": \"no\"\n",
        "      \"on\": \"on\"\n",
        "      \"off\": \"off\"\n",
        "      \"true\": \"true\"\n",
        "      \"false\": \"false\"\n",
        "      \"null\": \"null\"\n",
        "      \"~\": \"~\"\n",
        "      number: \"007\"\n",
        "      indicator: \"-not-a-list-item\"\n",
        "      comment: \"value # remains text\"\n",
        "      sexagesimal: \"1:20\"\n",
        "      date: \"2026-08-19\"\n",
        "      timestamp: \"2026-08-19T12:34:56Z\"\n",
        "      infinity: \".inf\"\n",
        "      nan: \".NaN\"\n",
    );
    let project = one_file_project(source, 289)?;
    let rendered = render_canonical(&project, None);

    assert!(rendered.is_valid(), "{:#?}", rendered.diagnostics());
    assert!(rendered.output().starts_with("---\nservices:\n  web:\n"));
    assert!(rendered.output().contains("image: example.invalid/web:1\n"));
    assert!(rendered.output().contains("plain: plain-value\n"));
    for value in [
        "yes",
        "no",
        "on",
        "off",
        "true",
        "false",
        "null",
        "~",
        "007",
        "value # remains text",
        "1:20",
        "2026-08-19",
        "2026-08-19T12:34:56Z",
        ".inf",
        ".NaN",
    ] {
        assert!(
            rendered.output().contains(&format!("\"{value}\"")),
            "unsafe YAML string was not quoted: {value:?}\n{}",
            rendered.output()
        );
    }

    let parsed = SyntaxDocument::parse(SourceId::new(290), rendered.output())?;
    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    let reloaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(291),
        DocumentOrigin::new("rendered.yaml", "workspace/project"),
        rendered.output(),
    )])?;
    let remerged = merge_project(&reloaded, None);
    let reproject = remerged.project().ok_or("remerged project expected")?;
    assert_eq!(render_canonical(reproject, None).output(), rendered.output());
    Ok(())
}

#[test]
fn canonical_document_marker_can_be_explicitly_disabled() -> Result<(), Box<dyn std::error::Error>> {
    let project = one_file_project("services:\n  app:\n    image: example/app:1\n", 292)?;
    let formatting = CanonicalFormatting::default().with_document_marker(false);
    let rendered = render_canonical_with_formatting(&project, None, &formatting);

    assert!(rendered.is_valid(), "{:#?}", rendered.diagnostics());
    assert!(!rendered.output().starts_with("---"));
    assert_eq!(rendered.output(), "services:\n  app:\n    image: example/app:1\n");
    Ok(())
}

#[test]
fn formatting_options_change_presentation_without_changing_semantics() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    image: example/app:1\n    command: [serve, \"--port=80\"]\n";
    let project = one_file_project(source, 291)?;
    let indent = IndentWidth::new(4).ok_or("four-space indentation expected")?;
    let formatting = CanonicalFormatting::default()
        .with_indent_width(indent)
        .with_line_ending(LineEnding::CrLf)
        .with_document_marker(true)
        .with_final_newline(false);
    let formatted = render_canonical_with_formatting(&project, None, &formatting);
    let expected = "---\r\nservices:\r\n    app:\r\n        image: example/app:1\r\n        command:\r\n            - serve\r\n            - \"--port=80\"";

    assert!(formatted.is_valid(), "{:#?}", formatted.diagnostics());
    assert_eq!(formatting.indent_width(), indent);
    assert_eq!(formatting.line_ending(), LineEnding::CrLf);
    assert!(formatting.document_marker());
    assert!(!formatting.final_newline());
    assert_eq!(formatted.output(), expected);
    assert!(!formatted.output().ends_with(['\r', '\n']));

    let reparsed = SyntaxDocument::parse(SourceId::new(292), formatted.output())?;
    assert!(reparsed.is_valid(), "{:#?}", reparsed.diagnostics());
    let reloaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(293),
        DocumentOrigin::new("formatted.yaml", "workspace/project"),
        formatted.output(),
    )])?;
    let remerged = merge_project(&reloaded, None);
    let reproject = remerged.project().ok_or("remerged project expected")?;
    assert_eq!(
        render_canonical(reproject, None).output(),
        render_canonical(&project, None).output()
    );
    Ok(())
}

#[test]
fn indentation_width_rejects_values_that_cannot_form_yaml_nesting() {
    assert_eq!(IndentWidth::new(0), None);
    assert_eq!(IndentWidth::new(1).map(IndentWidth::spaces), Some(1));
    assert_eq!(IndentWidth::new(u8::MAX).map(IndentWidth::spaces), Some(u8::MAX));
}

#[test]
fn renders_only_services_in_an_explicit_profile_selection() -> Result<(), Box<dyn std::error::Error>> {
    let project = canonical_project()?;
    let selection = select_profiles(&project, &ProfileRequest::new());
    let selected = render_canonical(&project, Some(&selection));

    assert!(selected.is_valid(), "{:#?}", selected.diagnostics());
    assert!(selected.output().contains("  app:"));
    assert!(!selected.output().contains("  debug:"));
    assert!(selected.output().contains("networks:"));
    Ok(())
}

#[test]
fn rejects_a_profile_selection_from_another_project() -> Result<(), Box<dyn std::error::Error>> {
    let selected_project = one_file_project("services:\n  selected:\n    image: example/selected\n", 266)?;
    let rendered_project = one_file_project("services:\n  rendered:\n    image: example/rendered\n", 267)?;
    let selection = select_profiles(&selected_project, &ProfileRequest::new());
    let rendered = render_canonical(&rendered_project, Some(&selection));

    assert!(!rendered.is_valid());
    assert!(rendered.output().is_empty());
    assert!(
        rendered
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SELECTION_PROJECT_MISMATCH)
    );
    Ok(())
}

#[test]
fn unresolved_aliases_produce_valid_recovery_yaml_and_an_error() -> Result<(), Box<dyn std::error::Error>> {
    let project = one_file_project("services:\n  app:\n    image: *missing\n", 271)?;
    let rendered = render_canonical(&project, None);

    assert!(!rendered.is_valid());
    assert_eq!(rendered.output(), "---\nservices:\n  app:\n    image: null\n");
    assert!(
        rendered
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == UNRENDERABLE_ALIAS)
    );
    let recovered = SyntaxDocument::parse(SourceId::new(272), rendered.output())?;
    assert!(recovered.is_valid(), "{:#?}", recovered.diagnostics());
    Ok(())
}

#[test]
fn canonical_render_debug_output_redacts_interpolated_values() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    image: example/app:${PRIVATE}\n";
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(281),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("PRIVATE", "private-render-value");
    let interpolation = loaded.interpolate(&environment);
    let merge = merge_project(&loaded, Some(&interpolation));
    let project = merge.project().ok_or("merged project expected")?;
    let rendered = render_canonical(project, None);

    assert!(rendered.is_sensitive());
    assert!(rendered.output().contains("private-render-value"));
    assert!(!format!("{rendered:?}").contains("private-render-value"));
    assert!(rendered.diagnostics().iter().all(|diagnostic| {
        !diagnostic.message().contains("private-render-value")
            && diagnostic
                .labels()
                .iter()
                .all(|label| !label.message().contains("private-render-value"))
            && diagnostic
                .notes()
                .iter()
                .all(|note| !note.contains("private-render-value"))
    }));
    Ok(())
}

#[test]
fn canonical_rendering_preserves_cpu_count_scalar_spelling() -> Result<(), Box<dyn std::error::Error>> {
    let project = one_file_project(
        "services:\n  app:\n    cpu_count: 0xCA_FE\n  quoted:\n    cpu_count: \"007\"\n",
        3258,
    )?;
    let rendered = render_canonical(&project, None);
    assert!(rendered.is_valid(), "{:#?}", rendered.diagnostics());
    assert!(rendered.output().contains("cpu_count: 0xCA_FE"));
    assert!(rendered.output().contains("cpu_count: \"007\""));
    Ok(())
}

#[test]
fn canonical_rendering_preserves_cpu_percent_scalar_spelling() -> Result<(), Box<dyn std::error::Error>> {
    let project = one_file_project(
        "services:\n  app:\n    cpu_percent: 0x64\n  quoted:\n    cpu_percent: \"101\"\n",
        3266,
    )?;
    let rendered = render_canonical(&project, None);
    assert!(rendered.is_valid(), "{:#?}", rendered.diagnostics());
    assert!(rendered.output().contains("cpu_percent: 0x64"), "{}", rendered.output());
    assert!(
        rendered.output().contains("cpu_percent: \"101\""),
        "{}",
        rendered.output()
    );
    Ok(())
}

#[test]
fn canonical_rendering_preserves_cpu_period_scalar_spelling() -> Result<(), Box<dyn std::error::Error>> {
    let project = one_file_project(
        "services:\n  app:\n    cpu_period: 1e6\n  quoted:\n    cpu_period: \"1000\"\n",
        3274,
    )?;
    let rendered = render_canonical(&project, None);
    assert!(rendered.is_valid(), "{:#?}", rendered.diagnostics());
    assert!(rendered.output().contains("cpu_period: 1e6"));
    assert!(rendered.output().contains("cpu_period: \"1000\""));
    Ok(())
}

#[test]
fn canonical_rendering_preserves_cpu_quota_scalar_spelling() -> Result<(), Box<dyn std::error::Error>> {
    let project = one_file_project(
        "services:\n  app:\n    cpu_quota: 1e6\n  quoted:\n    cpu_quota: \"1000\"\n",
        3280,
    )?;
    let rendered = render_canonical(&project, None);
    assert!(rendered.is_valid(), "{:#?}", rendered.diagnostics());
    assert!(rendered.output().contains("cpu_quota: 1e6"));
    assert!(rendered.output().contains("cpu_quota: \"1000\""));
    Ok(())
}

#[test]
fn canonical_rendering_preserves_cpu_rt_period_scalar_spelling() -> Result<(), Box<dyn std::error::Error>> {
    let project = one_file_project(
        "services:\n  numeric:\n    cpu_rt_period: 1e6\n  duration:\n    cpu_rt_period: \"1m30s\"\n",
        3288,
    )?;
    let rendered = render_canonical(&project, None);
    assert!(rendered.is_valid(), "{:#?}", rendered.diagnostics());
    assert!(rendered.output().contains("cpu_rt_period: 1e6"));
    assert!(rendered.output().contains("cpu_rt_period: 1m30s"));
    Ok(())
}

fn canonical_project() -> Result<MergedProject, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(261),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            BASE,
        ),
        DocumentInput::new(
            SourceId::new(262),
            DocumentOrigin::new("compose.override.yaml", "workspace/overrides"),
            OVERRIDE,
        ),
    ])?;
    let merge = merge_project(&loaded, None);
    merge.project().cloned().ok_or_else(|| "merged project expected".into())
}

fn one_file_project(source: &'static str, source_id: u32) -> Result<MergedProject, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(source_id),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let merge = merge_project(&loaded, None);
    merge.project().cloned().ok_or_else(|| "merged project expected".into())
}
