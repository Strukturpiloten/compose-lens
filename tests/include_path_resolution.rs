//! Public include-aware config and secret file-path resolution contracts.

use compose_lens::loader::{
    DocumentInput, DocumentOrigin, IncludeIdentity, IncludeLoadError, IncludeLoader, IncludeProjectDirectoryRequest,
    IncludeProjectDirectoryResolution, IncludeProjectDirectoryResolveError, IncludeProjectDirectoryResolver,
    IncludeRequest, IncludeResolution, IncludedProjectInput,
};
use compose_lens::resolution::{
    HOME_DIRECTORY_REQUIRED, HostPathKind, INCLUDE_RESOURCE_PATH_BASE_UNAVAILABLE, INCLUDE_RESOURCE_PATH_PLAN_MISMATCH,
    IncludedResourcePath, IncludedResourcePathResolution, PathContext, PathPurpose, resolve_included_resource_paths,
};
use compose_lens::source::SourceId;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct FixtureLoader {
    results: BTreeMap<String, IncludedProjectInput>,
}

impl FixtureLoader {
    fn with(mut self, path: &str, input: IncludedProjectInput) -> Self {
        self.results.insert(path.to_owned(), input);
        self
    }
}

impl IncludeLoader for FixtureLoader {
    fn load_include(&self, request: &IncludeRequest) -> Result<IncludedProjectInput, IncludeLoadError> {
        let path = request
            .paths()
            .first()
            .map(|path| path.value().as_str())
            .unwrap_or_default();
        self.results
            .get(path)
            .cloned()
            .ok_or_else(|| IncludeLoadError::failed("fixture include unavailable"))
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

#[derive(Default)]
struct LexicalResolver {
    calls: RefCell<Vec<(usize, Option<PathBuf>, String)>>,
}

impl IncludeProjectDirectoryResolver for LexicalResolver {
    fn resolve_project_directory(
        &self,
        request: &IncludeProjectDirectoryRequest<'_>,
    ) -> Result<IncludeProjectDirectoryResolution, IncludeProjectDirectoryResolveError> {
        let raw = request.declaration().value().clone();
        self.calls.borrow_mut().push((
            request.child_node_index(),
            request.parent_effective_directory().map(Path::to_path_buf),
            raw.clone(),
        ));
        let path = PathBuf::from(&raw);
        let resolved = if path.is_absolute() {
            path
        } else {
            request
                .parent_effective_directory()
                .map(|parent| parent.join(path))
                .ok_or(IncludeProjectDirectoryResolveError::Unresolved)?
        };
        Ok(IncludeProjectDirectoryResolution::Resolved(resolved))
    }
}

#[test]
fn resolves_selected_config_and_secret_files_from_their_occurrence_bases() {
    let root = input(
        "root",
        20_000,
        "root.yaml",
        "/workspace/root",
        concat!(
            "configs:\n",
            "  root-config: {file: root.txt}\n",
            "  shared: {file: root-shared.txt}\n",
            "include:\n",
            "  - path: child.yaml\n",
            "    project_directory: child-base\n",
        ),
    );
    let child = input(
        "child",
        20_001,
        "child.yaml",
        "/loaded/child",
        concat!(
            "configs:\n",
            "  child-relative: {file: child.txt}\n",
            "  absolute: {file: /opt/application/config}\n",
            "  drive: {file: 'C:\\application\\config'}\n",
            "  unc: {file: '\\\\server\\share\\config'}\n",
            "  home: {file: ~/application/config}\n",
            "  shared: {file: child-shared.txt}\n",
            "secrets:\n",
            "  child-secret: {file: secret.txt}\n",
            "include:\n",
            "  - path: grand.yaml\n",
            "    project_directory: grand-base\n",
        ),
    );
    let grand = input(
        "grand",
        20_002,
        "grand.yaml",
        "/loaded/grand",
        "configs: {grand-config: {file: grand.txt}}\n",
    );
    let loader = FixtureLoader::default()
        .with("child.yaml", child)
        .with("grand.yaml", grand);
    let resolution = IncludeResolution::load(root, &loader);
    let composition = resolution.compose();
    let resolver = LexicalResolver::default();
    let directories = resolution.plan_project_directories(&resolver);
    let paths = resolve_included_resource_paths(
        &composition,
        &directories,
        &PathContext::new().with_home_directory("/home/fixture"),
    );

    assert_selected_resource_paths(&paths);
    assert_eq!(resolver.calls.borrow().len(), 2);
}

fn assert_selected_resource_paths(paths: &IncludedResourcePathResolution) {
    assert!(paths.is_valid());
    assert!(!paths.is_complete(), "the selected composition retains the conflict");
    assert_eq!(paths.paths().len(), 9);
    let find = |purpose: &PathPurpose| paths.paths().iter().find(|path| path.purpose() == purpose);
    assert_eq!(
        find(&PathPurpose::ConfigFile {
            config: "root-config".to_owned(),
        })
        .and_then(|path| path.resolved()),
        Some(Path::new("/workspace/root/root.txt"))
    );
    assert_eq!(
        find(&PathPurpose::ConfigFile {
            config: "child-relative".to_owned(),
        })
        .and_then(|path| path.resolved()),
        Some(Path::new("/workspace/root/child-base/child.txt"))
    );
    assert_eq!(
        find(&PathPurpose::ConfigFile {
            config: "grand-config".to_owned(),
        })
        .and_then(|path| path.resolved()),
        Some(Path::new("/workspace/root/child-base/grand-base/grand.txt"))
    );
    assert_eq!(
        find(&PathPurpose::ConfigFile {
            config: "absolute".to_owned(),
        })
        .map(IncludedResourcePath::kind),
        Some(HostPathKind::UnixAbsolute)
    );
    assert_eq!(
        find(&PathPurpose::ConfigFile {
            config: "drive".to_owned(),
        })
        .map(IncludedResourcePath::kind),
        Some(HostPathKind::WindowsDriveAbsolute)
    );
    assert_eq!(
        find(&PathPurpose::ConfigFile {
            config: "unc".to_owned(),
        })
        .map(IncludedResourcePath::kind),
        Some(HostPathKind::WindowsUnc)
    );
    assert_eq!(
        find(&PathPurpose::ConfigFile {
            config: "home".to_owned(),
        })
        .and_then(|path| path.resolved()),
        Some(Path::new("/home/fixture/application/config"))
    );
    assert_eq!(
        find(&PathPurpose::SecretFile {
            secret: "child-secret".to_owned(),
        })
        .map(|path| path.identity().as_str()),
        Some("child")
    );
    assert!(paths.paths().iter().all(|path| {
        let debug = format!("{path:?}");
        !debug.contains(path.raw()) && !debug.contains("/workspace") && !debug.contains("/loaded")
    }));
    assert!(paths.paths().iter().all(|path| path.raw() != "child-shared.txt"));
}

struct DeferredResolver;

impl IncludeProjectDirectoryResolver for DeferredResolver {
    fn resolve_project_directory(
        &self,
        _: &IncludeProjectDirectoryRequest<'_>,
    ) -> Result<IncludeProjectDirectoryResolution, IncludeProjectDirectoryResolveError> {
        Ok(IncludeProjectDirectoryResolution::Deferred)
    }
}

struct UnresolvedResolver;

impl IncludeProjectDirectoryResolver for UnresolvedResolver {
    fn resolve_project_directory(
        &self,
        _: &IncludeProjectDirectoryRequest<'_>,
    ) -> Result<IncludeProjectDirectoryResolution, IncludeProjectDirectoryResolveError> {
        Err(IncludeProjectDirectoryResolveError::Unresolved)
    }
}

#[test]
fn never_guesses_an_unavailable_or_mismatched_occurrence_base() {
    let root = input(
        "root",
        20_010,
        "root.yaml",
        "/private/root",
        "include:\n  - path: child.yaml\n    project_directory: opaque://child\n",
    );
    let child = input(
        "child",
        20_011,
        "child.yaml",
        "/private/child",
        "configs: {application: {file: private/config.txt}}\n",
    );
    let resolution = IncludeResolution::load(root, &FixtureLoader::default().with("child.yaml", child));
    let composition = resolution.compose();
    let directories = resolution.plan_project_directories(&DeferredResolver);
    let unavailable = resolve_included_resource_paths(&composition, &directories, &PathContext::new());
    assert!(!unavailable.is_valid());
    assert!(!unavailable.is_complete());
    assert_eq!(unavailable.paths()[0].base_directory(), None);
    assert_eq!(unavailable.paths()[0].resolved(), None);
    assert!(unavailable.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == INCLUDE_RESOURCE_PATH_BASE_UNAVAILABLE
            && !format!("{diagnostic:?}").contains("private/config.txt")
    }));

    let unresolved_directories = resolution.plan_project_directories(&UnresolvedResolver);
    let unresolved = resolve_included_resource_paths(&composition, &unresolved_directories, &PathContext::new());
    assert!(!unresolved.is_valid());
    assert_eq!(unresolved.paths()[0].base_directory(), None);
    assert_eq!(unresolved.paths()[0].resolved(), None);
    assert!(
        unresolved
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == INCLUDE_RESOURCE_PATH_BASE_UNAVAILABLE)
    );

    let other_root = input(
        "other-root",
        20_012,
        "other.yaml",
        "/other/root",
        "include: [other-child.yaml]\n",
    );
    let other_child = input(
        "different-child",
        20_013,
        "other-child.yaml",
        "/other/child",
        "services: {}\n",
    );
    let other_resolution = IncludeResolution::load(
        other_root,
        &FixtureLoader::default().with("other-child.yaml", other_child),
    );
    let mismatched_plan = other_resolution.plan_project_directories(&DeferredResolver);
    let mismatched = resolve_included_resource_paths(&composition, &mismatched_plan, &PathContext::new());
    assert!(!mismatched.is_valid());
    assert!(
        mismatched
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == INCLUDE_RESOURCE_PATH_PLAN_MISMATCH)
    );
    assert_eq!(mismatched.paths()[0].resolved(), None);
}

#[test]
fn missing_home_context_is_warning_only_but_keeps_the_result_incomplete() {
    let root = input(
        "root",
        20_020,
        "root.yaml",
        "/workspace/root",
        "configs: {home: {file: ~/application/config}}\n",
    );
    let resolution = IncludeResolution::load(root, &FixtureLoader::default());
    let composition = resolution.compose();
    let directories = resolution.plan_project_directories(&DeferredResolver);
    let paths = resolve_included_resource_paths(&composition, &directories, &PathContext::new());
    assert!(paths.is_valid());
    assert!(!paths.is_complete());
    assert_eq!(paths.paths()[0].resolved(), None);
    assert!(
        paths
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == HOME_DIRECTORY_REQUIRED)
    );
}

#[test]
fn repeated_identities_keep_distinct_occurrence_bases() {
    let root = input(
        "root",
        20_030,
        "root.yaml",
        "/workspace/root",
        "include: [first.yaml, second.yaml]\n",
    );
    let first = input(
        "repeated",
        20_031,
        "first.yaml",
        "/workspace/first",
        "configs: {first: {file: first.txt}}\n",
    );
    let second = input(
        "repeated",
        20_032,
        "second.yaml",
        "/workspace/second",
        "secrets: {second: {file: second.txt}}\n",
    );
    let resolution = IncludeResolution::load(
        root,
        &FixtureLoader::default()
            .with("first.yaml", first)
            .with("second.yaml", second),
    );
    let composition = resolution.compose();
    let directories = resolution.plan_project_directories(&DeferredResolver);
    let paths = resolve_included_resource_paths(&composition, &directories, &PathContext::new());

    assert!(paths.is_complete());
    assert_eq!(paths.paths().len(), 2);
    assert_eq!(paths.paths()[0].identity().as_str(), "repeated");
    assert_eq!(paths.paths()[1].identity().as_str(), "repeated");
    assert_ne!(paths.paths()[0].occurrence_index(), paths.paths()[1].occurrence_index());
    assert_eq!(
        paths.paths()[0].resolved(),
        Some(Path::new("/workspace/first/first.txt"))
    );
    assert_eq!(
        paths.paths()[1].resolved(),
        Some(Path::new("/workspace/second/second.txt"))
    );
}
