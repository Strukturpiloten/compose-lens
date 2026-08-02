//! Parse-render-parse stability for the initial loss-aware syntax document.

use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;

const STABLE_REPARSE: &str = include_str!("../fixtures/roundtrip/stable-reparse/compose.yaml");

#[test]
fn preservation_rendering_is_stable_across_reparse() -> Result<(), Box<dyn std::error::Error>> {
    let first = SyntaxDocument::parse(SourceId::new(1), STABLE_REPARSE)?;
    let first_render = first.document().render_preserved();
    let second = SyntaxDocument::parse(SourceId::new(2), first_render.as_str())?;
    let second_render = second.document().render_preserved();

    assert!(first.is_valid(), "{:#?}", first.diagnostics());
    assert!(second.is_valid(), "{:#?}", second.diagnostics());
    assert_eq!(first_render, STABLE_REPARSE);
    assert_eq!(second_render, STABLE_REPARSE);
    Ok(())
}
