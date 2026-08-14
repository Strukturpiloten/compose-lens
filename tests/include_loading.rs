//! Public, caller-authorized include traversal contracts.

use compose_lens::loader::{
    DocumentInput, DocumentOrigin, INCLUDE_CYCLE, INCLUDE_DUPLICATE_SOURCE_ID, INCLUDE_EMPTY_RESULT,
    INCLUDE_LOADER_DENIED, INCLUDE_LOADER_FAILED, INCLUDE_UNMODELED, IncludeComposition, IncludeIdentity,
    IncludeLoadError, IncludeLoader, IncludeRequest, IncludeResolution, IncludedProjectInput,
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

fn assert_root_cross_references(root: &IncludeComposition) {
    assert!(
        root.service("root")
            .is_some_and(|definition| definition.definition().networks().is_some())
    );
    assert!(
        root.service("root")
            .is_some_and(|definition| definition.definition().volumes().is_some())
    );
    assert!(
        root.service("root")
            .is_some_and(|definition| definition.definition().configs().is_some())
    );
    assert!(
        root.service("root")
            .is_some_and(|definition| definition.definition().secrets().is_some())
    );
    assert!(
        root.service("root")
            .is_some_and(|definition| definition.definition().models().is_some())
    );
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

#[test]
fn composes_nested_definitions_after_each_nodes_multifile_merge() {
    let root = input(
        "root",
        100,
        "root.yaml",
        "root-directory",
        concat!(
            "services:\n",
            "  root:\n",
            "    image: root\n",
            "    networks: [child-network]\n",
            "    volumes: [child-volume:/data]\n",
            "    configs: [child-config]\n",
            "    secrets: [child-secret]\n",
            "    models: [child-model]\n",
            "networks: {root-network: {}}\n",
            "volumes: {root-volume: {}}\n",
            "configs: {root-config: {}}\n",
            "secrets: {root-secret: {}}\n",
            "models: {root-model: {model: example/root}}\n",
            "include: [child.yaml]\n",
        ),
    );
    let child = IncludedProjectInput::new(
        IncludeIdentity::new("child"),
        [
            DocumentInput::new(
                SourceId::new(101),
                DocumentOrigin::new("child-base.yaml", "child-directory"),
                concat!(
                    "services: {child: {image: before-merge}}\n",
                    "networks: {child-network: {}}\n",
                    "volumes: {child-volume: {}}\n",
                    "configs: {child-config: {}}\n",
                    "secrets: {child-secret: {}}\n",
                    "models: {child-model: {model: example/child}}\n",
                    "include: [grandchild.yaml]\n",
                ),
            ),
            DocumentInput::new(
                SourceId::new(102),
                DocumentOrigin::new("child-override.yaml", "child-directory"),
                "services: {child: {image: after-merge}}\n",
            ),
        ],
    );
    let grandchild = input(
        "grandchild",
        103,
        "grandchild.yaml",
        "grandchild-directory",
        concat!(
            "services: {grandchild: {image: grandchild}}\n",
            "networks: {grandchild-network: {}}\n",
            "volumes: {grandchild-volume: {}}\n",
            "configs: {grandchild-config: {}}\n",
            "secrets: {grandchild-secret: {}}\n",
            "models: {grandchild-model: {model: example/grandchild}}\n",
        ),
    );
    let loader = FixtureLoader::default()
        .with_result("child.yaml", Ok(child))
        .with_result("grandchild.yaml", Ok(grandchild));

    let composed = IncludeResolution::load(root, &loader).compose();
    assert!(!composed.compositions().is_empty());
    let root = &composed.compositions()[0];

    assert!(composed.is_complete());
    assert_eq!(root.services().len(), 3);
    assert_eq!(root.networks().len(), 3);
    assert_eq!(root.volumes().len(), 3);
    assert_eq!(root.configs().len(), 3);
    assert_eq!(root.secrets().len(), 3);
    assert_eq!(root.models().len(), 3);
    assert_eq!(
        root.service("child")
            .and_then(|definition| definition.definition().image())
            .map(|image| image.value().raw().to_owned()),
        Some("after-merge".to_owned())
    );
    assert_eq!(
        root.service("child")
            .map(|definition| definition.evidence().identity().as_str()),
        Some("child")
    );
    assert!(root.network("child-network").is_some());
    assert!(root.volume("child-volume").is_some());
    assert!(root.config("child-config").is_some());
    assert!(root.secret("child-secret").is_some());
    assert!(root.model("child-model").is_some());
    assert_root_cross_references(root);
}

#[test]
fn keeps_parent_and_first_sibling_definitions_on_every_namespace_conflict() {
    let definitions = concat!(
        "services: {shared: {image: child}}\n",
        "networks: {shared: {}}\n",
        "volumes: {shared: {}}\n",
        "configs: {shared: {}}\n",
        "secrets: {shared: {}}\n",
        "models: {shared: {model: example/child}}\n",
    );
    let root = input(
        "root",
        110,
        "root.yaml",
        "root-directory",
        &format!("{}include: [a.yaml, b.yaml]\n", definitions.replace("child", "root")),
    );
    let loader = FixtureLoader::default()
        .with_result("a.yaml", Ok(input("a", 111, "a.yaml", "a-directory", definitions)))
        .with_result("b.yaml", Ok(input("b", 112, "b.yaml", "b-directory", definitions)));

    let composed = IncludeResolution::load(root, &loader).compose();
    assert!(!composed.compositions().is_empty());
    let root = &composed.compositions()[0];

    assert!(!composed.is_complete());
    assert!(composed.is_valid());
    assert_eq!(composed.conflicts().len(), 12);
    for conflict in composed.conflicts() {
        assert_eq!(conflict.name(), "shared");
        assert_eq!(conflict.incumbent().identity().as_str(), "root");
        assert!(matches!(conflict.incoming().identity().as_str(), "a" | "b"));
        assert!(conflict.incoming().source_label().is_some());
        assert!(conflict.incumbent().source_label().is_some());
    }
    assert_eq!(root.services().len(), 1);
    assert_eq!(root.networks().len(), 1);
    assert_eq!(root.volumes().len(), 1);
    assert_eq!(root.configs().len(), 1);
    assert_eq!(root.secrets().len(), 1);
    assert_eq!(root.models().len(), 1);
    assert_eq!(
        root.service("shared")
            .and_then(|definition| definition.definition().image())
            .map(|image| image.value().raw().to_owned()),
        Some("root".to_owned())
    );
    let warnings = composed
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code() == compose_lens::loader::INCLUDE_RESOURCE_CONFLICT)
        .collect::<Vec<_>>();
    assert_eq!(warnings.len(), 12);
    assert!(warnings.iter().all(|warning| warning.labels().len() == 2));
}

#[test]
fn keeps_the_first_child_definition_when_later_siblings_collide() {
    let definitions = concat!(
        "services: {shared: {image: child}}\n",
        "networks: {shared: {}}\n",
        "volumes: {shared: {}}\n",
        "configs: {shared: {}}\n",
        "secrets: {shared: {}}\n",
        "models: {shared: {model: example/child}}\n",
    );
    let root = input(
        "root",
        115,
        "root.yaml",
        "root-directory",
        "include: [a.yaml, b.yaml]\n",
    );
    let loader = FixtureLoader::default()
        .with_result("a.yaml", Ok(input("a", 116, "a.yaml", "a-directory", definitions)))
        .with_result("b.yaml", Ok(input("b", 117, "b.yaml", "b-directory", definitions)));

    let composed = IncludeResolution::load(root, &loader).compose();
    assert!(!composed.compositions().is_empty());
    let root = &composed.compositions()[0];

    assert_eq!(composed.conflicts().len(), 6);
    for conflict in composed.conflicts() {
        assert_eq!(conflict.incumbent().identity().as_str(), "a");
        assert_eq!(conflict.incoming().identity().as_str(), "b");
        assert_ne!(
            conflict.incumbent().occurrence_index(),
            conflict.incoming().occurrence_index()
        );
    }
    assert_eq!(
        root.service("shared")
            .map(|definition| definition.evidence().identity().as_str()),
        Some("a")
    );
}

#[test]
fn recovers_other_imports_when_an_include_is_malformed_empty_or_cyclic() {
    let root = input(
        "root",
        120,
        "root.yaml",
        "root-directory",
        "include: [good.yaml, empty.yaml, cycle.yaml, 42]\n",
    );
    let cycle = input(
        "cycle",
        121,
        "cycle.yaml",
        "cycle-directory",
        "services: {cycle: {image: cycle}}\ninclude: [root.yaml]\n",
    );
    let loader = FixtureLoader::default()
        .with_result(
            "good.yaml",
            Ok(input(
                "good",
                122,
                "good.yaml",
                "good-directory",
                "services: {good: {image: good}}\n",
            )),
        )
        .with_result(
            "empty.yaml",
            Ok(IncludedProjectInput::new(IncludeIdentity::new("empty"), [])),
        )
        .with_result("cycle.yaml", Ok(cycle))
        .with_result(
            "root.yaml",
            Ok(input(
                "root",
                123,
                "shadow-root.yaml",
                "shadow-directory",
                "services: {shadow: {image: shadow}}\n",
            )),
        );

    let resolution = IncludeResolution::load(root, &loader);
    assert!(resolution.edges()[3].is_cycle());
    assert_eq!(resolution.edges()[3].child_node_index(), 0);
    let composed = resolution.compose();
    assert!(!composed.compositions().is_empty());
    let root = &composed.compositions()[0];

    assert!(!composed.is_complete());
    assert!(root.service("good").is_some());
    assert!(root.service("cycle").is_some());
    assert!(root.service("shadow").is_none());
}
