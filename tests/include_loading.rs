//! Public, caller-authorized include traversal contracts.

use compose_lens::loader::{
    DocumentInput, DocumentOrigin, INCLUDE_CYCLE, INCLUDE_DUPLICATE_SOURCE_ID, INCLUDE_EMPTY_RESULT,
    INCLUDE_LOADER_DENIED, INCLUDE_LOADER_FAILED, INCLUDE_UNMODELED, IncludeIdentity, IncludeLoadError, IncludeLoader,
    IncludeRequest, IncludeResolution, IncludedProjectInput,
};
use compose_lens::source::SourceId;
use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Default)]
struct FixtureLoader {
    results: BTreeMap<String, Result<IncludedProjectInput, IncludeLoadError>>,
    calls: RefCell<Vec<String>>,
}

impl FixtureLoader {
    fn with_result(mut self, path: &str, result: Result<IncludedProjectInput, IncludeLoadError>) -> Self {
        self.results.insert(path.to_owned(), result);
        self
    }
}

impl IncludeLoader for FixtureLoader {
    fn load_include(&self, request: &IncludeRequest) -> Result<IncludedProjectInput, IncludeLoadError> {
        let path = request
            .paths()
            .first()
            .map(|path| path.value().clone())
            .unwrap_or_default();
        self.calls.borrow_mut().push(path.clone());
        self.results
            .get(&path)
            .cloned()
            .unwrap_or_else(|| Err(IncludeLoadError::failed("fixture path is not authorized")))
    }
}

fn input(identity: &str, source_id: u32, label: &str, directory: &str, source: &str) -> IncludedProjectInput {
    IncludedProjectInput::new(
        IncludeIdentity::new(identity),
        [DocumentInput::new(
            SourceId::new(source_id),
            DocumentOrigin::new(label, directory),
            source,
        )],
    )
}

fn codes(resolution: &IncludeResolution) -> Vec<&'static str> {
    resolution
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code().as_str())
        .collect()
}

#[test]
fn public_consumer_traverses_effective_includes_depth_first_with_raw_context() {
    let root = input(
        "root",
        1,
        "root.compose.yaml",
        "root-directory",
        concat!(
            "include:\n",
            "  - child-a.yaml\n",
            "  - path: child-b.yaml\n",
            "    env_file: [child.env, second.env]\n",
            "    project_directory: child-project\n",
        ),
    );
    let child_a = input(
        "child-a",
        2,
        "child-a.yaml",
        "a-directory",
        "include: [grandchild.yaml]\n",
    );
    let grandchild = input("grandchild", 3, "grandchild.yaml", "grand-directory", "services: {}\n");
    let child_b = input("child-b", 4, "child-b.yaml", "b-directory", "services: {}\n");
    let loader = FixtureLoader::default()
        .with_result("child-a.yaml", Ok(child_a))
        .with_result("grandchild.yaml", Ok(grandchild))
        .with_result("child-b.yaml", Ok(child_b));

    let resolution = IncludeResolution::load(root, &loader);

    assert_eq!(
        loader.calls.into_inner(),
        ["child-a.yaml", "grandchild.yaml", "child-b.yaml"]
    );
    assert_eq!(
        resolution
            .nodes()
            .iter()
            .map(|node| node.identity().as_str())
            .collect::<Vec<_>>(),
        ["root", "child-a", "grandchild", "child-b"]
    );
    assert_eq!(resolution.nodes()[0].origins()[0].label(), "root.compose.yaml");
    assert_eq!(
        resolution.requests()[1].parent_base_directory().to_string_lossy(),
        "a-directory"
    );
    assert_eq!(resolution.requests()[2].env_files()[0].value(), "child.env");
    assert_eq!(resolution.requests()[2].env_files()[1].value(), "second.env");
    assert_eq!(
        resolution.requests()[2]
            .project_directory()
            .map(compose_lens::model::Located::value),
        Some(&"child-project".to_owned())
    );
    assert_eq!(
        resolution.requests()[2].declaration_span().source_id(),
        SourceId::new(1)
    );
    assert_eq!(
        resolution.requests()[2].declaration_origin().map(DocumentOrigin::label),
        Some("root.compose.yaml")
    );
    assert!(resolution.diagnostics().is_empty());
}

#[test]
fn applies_no_interpolation_multifile_reset_and_override_before_requesting() {
    let override_root = IncludedProjectInput::new(
        IncludeIdentity::new("override-root"),
        [
            DocumentInput::new(
                SourceId::new(10),
                DocumentOrigin::new("base.yaml", "base-directory"),
                "include: [old.yaml]\n",
            ),
            DocumentInput::new(
                SourceId::new(11),
                DocumentOrigin::new("override.yaml", "override-directory"),
                "include: !override [new.yaml]\n",
            ),
        ],
    );
    let reset_root = IncludedProjectInput::new(
        IncludeIdentity::new("reset-root"),
        [
            DocumentInput::new(
                SourceId::new(12),
                DocumentOrigin::new("base.yaml", "base-directory"),
                "include: [old.yaml]\n",
            ),
            DocumentInput::new(
                SourceId::new(13),
                DocumentOrigin::new("reset.yaml", "reset-directory"),
                "include: !reset []\n",
            ),
        ],
    );
    let loader = FixtureLoader::default().with_result(
        "new.yaml",
        Ok(input("new", 14, "new.yaml", "new-directory", "services: {}\n")),
    );

    let overridden = IncludeResolution::load(override_root, &loader);
    assert_eq!(loader.calls.borrow().as_slice(), ["new.yaml"]);
    assert_eq!(overridden.requests()[0].paths()[0].value(), "new.yaml");
    assert_eq!(
        overridden.requests()[0].parent_base_directory().to_string_lossy(),
        "base-directory"
    );

    let reset = IncludeResolution::load(reset_root, &loader);
    assert_eq!(loader.calls.borrow().as_slice(), ["new.yaml"]);
    assert!(reset.requests().is_empty());
    assert!(reset.edges().is_empty());
}

#[test]
fn preserves_partial_graph_for_malformed_denied_failed_and_empty_requests() {
    let root = input(
        "root",
        20,
        "root.yaml",
        "root-directory",
        concat!(
            "include:\n",
            "  - 42\n",
            "  - denied.yaml\n",
            "  - failed.yaml\n",
            "  - empty.yaml\n",
            "  - path: malformed.yaml\n",
            "    env_file: [42]\n",
        ),
    );
    let loader = FixtureLoader::default()
        .with_result("denied.yaml", Err(IncludeLoadError::denied("policy")))
        .with_result("failed.yaml", Err(IncludeLoadError::failed("offline")))
        .with_result(
            "empty.yaml",
            Ok(IncludedProjectInput::new(IncludeIdentity::new("empty"), [])),
        );

    let resolution = IncludeResolution::load(root, &loader);

    assert_eq!(loader.calls.into_inner(), ["denied.yaml", "failed.yaml", "empty.yaml"]);
    assert_eq!(resolution.nodes().len(), 2);
    assert_eq!(resolution.requests().len(), 3);
    assert_eq!(resolution.edges().len(), 1);
    let diagnostic_codes = codes(&resolution);
    assert!(diagnostic_codes.contains(&INCLUDE_UNMODELED.as_str()));
    assert!(diagnostic_codes.contains(&INCLUDE_LOADER_DENIED.as_str()));
    assert!(diagnostic_codes.contains(&INCLUDE_LOADER_FAILED.as_str()));
    assert!(diagnostic_codes.contains(&INCLUDE_EMPTY_RESULT.as_str()));
}

#[test]
fn retains_unloaded_empty_root_and_child_nodes_for_graph_integrity() {
    let root = input("root", 25, "root.yaml", "root-directory", "include: [empty.yaml]\n");
    let loader = FixtureLoader::default().with_result(
        "empty.yaml",
        Ok(IncludedProjectInput::new(IncludeIdentity::new("empty-child"), [])),
    );

    let child_resolution = IncludeResolution::load(root, &loader);
    assert_eq!(child_resolution.nodes().len(), 2);
    assert_eq!(child_resolution.edges().len(), 1);
    assert_eq!(child_resolution.edges()[0].child().as_str(), "empty-child");
    assert_eq!(child_resolution.nodes()[1].identity().as_str(), "empty-child");
    assert!(child_resolution.nodes()[1].origins().is_empty());
    assert!(child_resolution.nodes()[1].loaded_project().is_none());
    assert!(codes(&child_resolution).contains(&INCLUDE_EMPTY_RESULT.as_str()));

    let root_resolution = IncludeResolution::load(
        IncludedProjectInput::new(IncludeIdentity::new("empty-root"), []),
        &FixtureLoader::default(),
    );
    assert_eq!(root_resolution.nodes().len(), 1);
    assert_eq!(root_resolution.nodes()[0].identity().as_str(), "empty-root");
    assert!(root_resolution.nodes()[0].origins().is_empty());
    assert!(root_resolution.nodes()[0].loaded_project().is_none());
    assert!(root_resolution.edges().is_empty());
}

#[test]
fn detects_active_identity_cycles_without_recursing_into_them() {
    let root = input("root", 30, "root.yaml", "root-directory", "include: [child.yaml]\n");
    let child = input("child", 31, "child.yaml", "child-directory", "include: [root.yaml]\n");
    let loader = FixtureLoader::default()
        .with_result("child.yaml", Ok(child))
        .with_result(
            "root.yaml",
            Ok(input(
                "root",
                32,
                "shadow-root.yaml",
                "shadow-directory",
                "services: {}\n",
            )),
        );

    let resolution = IncludeResolution::load(root, &loader);

    assert_eq!(loader.calls.into_inner(), ["child.yaml", "root.yaml"]);
    assert_eq!(resolution.nodes().len(), 2);
    assert_eq!(resolution.edges().len(), 2);
    assert!(codes(&resolution).contains(&INCLUDE_CYCLE.as_str()));
}

#[test]
fn does_not_cache_diamonds_and_rejects_source_ids_across_the_whole_graph() {
    let root = input("root", 40, "root.yaml", "root-directory", "include: [a.yaml, b.yaml]\n");
    let a = input("a", 41, "a.yaml", "a-directory", "include: [common.yaml]\n");
    let b = input("b", 42, "b.yaml", "b-directory", "include: [common.yaml]\n");
    let common = input("common", 43, "common.yaml", "common-directory", "services: {}\n");
    let loader = FixtureLoader::default()
        .with_result("a.yaml", Ok(a))
        .with_result("b.yaml", Ok(b))
        .with_result("common.yaml", Ok(common));

    let resolution = IncludeResolution::load(root, &loader);
    assert_eq!(
        loader.calls.into_inner(),
        ["a.yaml", "common.yaml", "b.yaml", "common.yaml"]
    );
    assert_eq!(resolution.nodes().len(), 5);
    assert!(codes(&resolution).contains(&INCLUDE_DUPLICATE_SOURCE_ID.as_str()));

    let duplicate_root = input(
        "duplicate-root",
        50,
        "root.yaml",
        "root-directory",
        "include: [x.yaml, y.yaml]\n",
    );
    let duplicate_loader = FixtureLoader::default()
        .with_result("x.yaml", Ok(input("x", 51, "x.yaml", "x-directory", "services: {}\n")))
        .with_result("y.yaml", Ok(input("y", 51, "y.yaml", "y-directory", "services: {}\n")));
    let duplicate = IncludeResolution::load(duplicate_root, &duplicate_loader);
    assert_eq!(duplicate_loader.calls.into_inner(), ["x.yaml", "y.yaml"]);
    assert_eq!(duplicate.nodes().len(), 3);
    assert!(codes(&duplicate).contains(&INCLUDE_DUPLICATE_SOURCE_ID.as_str()));
}
