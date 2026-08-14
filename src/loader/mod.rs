//! Ordered, caller-supplied Compose project loading.

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::interpolation::{
    DocumentInterpolation, EnvironmentProvider, InterpolationOptions, interpolate_document_with_options,
};
use crate::merge::{MergedProject, merge_project};
use crate::model::{ComposeDocument, IncludeItem, Located, ModelParse};
use crate::project::{ProjectView, build_project_view};
use crate::source::{SourceId, SourceSpan};
use crate::syntax::{SyntaxDocument, SyntaxParseError};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// An effective include declaration could not be modeled safely enough to authorize loading.
pub const INCLUDE_UNMODELED: DiagnosticCode = DiagnosticCode::new("compose.include.unmodeled");
/// The caller denied an include request.
pub const INCLUDE_LOADER_DENIED: DiagnosticCode = DiagnosticCode::new("compose.include.loader-denied");
/// The caller's include loader failed to supply a project.
pub const INCLUDE_LOADER_FAILED: DiagnosticCode = DiagnosticCode::new("compose.include.loader-failed");
/// The caller supplied no documents for an included project.
pub const INCLUDE_EMPTY_RESULT: DiagnosticCode = DiagnosticCode::new("compose.include.empty-result");
/// An included identity appears again on the active traversal stack.
pub const INCLUDE_CYCLE: DiagnosticCode = DiagnosticCode::new("compose.include.cycle");
/// A source identifier was reused anywhere in an include traversal.
pub const INCLUDE_DUPLICATE_SOURCE_ID: DiagnosticCode = DiagnosticCode::new("compose.include.duplicate-source-id");
/// A caller-supplied include project could not enter the existing ordered loader.
pub const INCLUDE_PROJECT_LOAD_FAILED: DiagnosticCode = DiagnosticCode::new("compose.include.project-load-failed");
/// The root input contains no documents.
pub const INCLUDE_EMPTY_ROOT: DiagnosticCode = DiagnosticCode::new("compose.include.empty-root");

/// The caller-defined location of one Compose document.
///
/// The label is for display and may be a path, URI, or synthetic name. The document directory is
/// retained verbatim for later path-resolution decisions; `ComposeLens` does not canonicalize it or
/// access the file system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentOrigin {
    label: String,
    directory: PathBuf,
}

impl DocumentOrigin {
    /// Creates an explicit document origin.
    #[must_use]
    pub fn new(label: impl Into<String>, directory: impl Into<PathBuf>) -> Self {
        Self {
            label: label.into(),
            directory: directory.into(),
        }
    }

    /// Returns the caller-defined display label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the caller-supplied directory associated with this document.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

/// One source document supplied to the project loader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentInput {
    source_id: SourceId,
    origin: DocumentOrigin,
    source: Arc<str>,
}

impl DocumentInput {
    /// Creates an input without reading the file system or process environment.
    #[must_use]
    pub fn new(source_id: SourceId, origin: DocumentOrigin, source: impl Into<Arc<str>>) -> Self {
        Self {
            source_id,
            origin,
            source: source.into(),
        }
    }

    /// Returns the caller-managed source identifier.
    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// Returns the caller-defined document origin.
    #[must_use]
    pub const fn origin(&self) -> &DocumentOrigin {
        &self.origin
    }

    /// Returns the supplied source text.
    #[must_use]
    pub fn source_text(&self) -> &str {
        &self.source
    }
}

/// A caller-defined canonical identity for one includable Compose project.
///
/// The caller, not `ComposeLens`, establishes identity equivalence. In particular, this type does
/// not open, join, normalize, or canonicalize paths.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IncludeIdentity(String);

impl IncludeIdentity {
    /// Creates an opaque identity whose canonical form is owned by the caller.
    #[must_use]
    pub fn new(canonical: impl Into<String>) -> Self {
        Self(canonical.into())
    }

    /// Returns the caller-defined canonical identity text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IncludeIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A caller-created ordered project input returned by an [`IncludeLoader`].
///
/// This is equally suitable for the traversal root. `ComposeLens` keeps every origin and source
/// text supplied here, but it never discovers documents from the identity or include paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludedProjectInput {
    identity: IncludeIdentity,
    documents: Vec<DocumentInput>,
}

impl IncludedProjectInput {
    /// Creates one project input from caller-created documents in merge order.
    #[must_use]
    pub fn new(identity: IncludeIdentity, documents: impl IntoIterator<Item = DocumentInput>) -> Self {
        Self {
            identity,
            documents: documents.into_iter().collect(),
        }
    }

    /// Returns the caller-defined canonical project identity.
    #[must_use]
    pub const fn identity(&self) -> &IncludeIdentity {
        &self.identity
    }

    /// Returns documents in caller-selected merge order.
    #[must_use]
    pub fn documents(&self) -> &[DocumentInput] {
        &self.documents
    }
}

/// One effective, caller-authorized include request.
///
/// Every path-like value remains raw source data in declared order. The loader receives this
/// complete context and alone decides whether and how any document may be obtained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeRequest {
    parent_identity: IncludeIdentity,
    parent_base_directory: PathBuf,
    declaration_span: SourceSpan,
    declaration_origin: Option<DocumentOrigin>,
    item: IncludeItem,
    paths: Vec<Located<String>>,
    env_files: Vec<Located<String>>,
    project_directory: Option<Located<String>>,
}

impl IncludeRequest {
    fn from_item(
        parent_identity: IncludeIdentity,
        parent_base_directory: PathBuf,
        declaration_origin: Option<DocumentOrigin>,
        item: IncludeItem,
    ) -> Option<Self> {
        let declaration_span = include_item_span(&item)?;
        let (paths, env_files, project_directory) = match &item {
            IncludeItem::Short(path) => (vec![path.clone()], Vec::new(), None),
            IncludeItem::Long(include) if include.unmodeled_fields().is_empty() && !include.paths().is_empty() => (
                include.paths().to_vec(),
                include.env_files().to_vec(),
                include.project_directory().cloned(),
            ),
            IncludeItem::Long(_) | IncludeItem::Unmodeled => return None,
        };
        Some(Self {
            parent_identity,
            parent_base_directory,
            declaration_span,
            declaration_origin,
            item,
            paths,
            env_files,
            project_directory,
        })
    }

    /// Returns the identity of the project that declared this request.
    #[must_use]
    pub const fn parent_identity(&self) -> &IncludeIdentity {
        &self.parent_identity
    }

    /// Returns the parent project's first-document directory exactly as supplied by the caller.
    #[must_use]
    pub fn parent_base_directory(&self) -> &Path {
        &self.parent_base_directory
    }

    /// Returns the complete span of the effective declaration.
    #[must_use]
    pub const fn declaration_span(&self) -> SourceSpan {
        self.declaration_span
    }

    /// Returns the source identifier containing the effective declaration.
    #[must_use]
    pub const fn declaration_source_id(&self) -> SourceId {
        self.declaration_span.source_id()
    }

    /// Returns the origin of the document containing the effective declaration, when available.
    #[must_use]
    pub const fn declaration_origin(&self) -> Option<&DocumentOrigin> {
        self.declaration_origin.as_ref()
    }

    /// Returns the complete effective typed include item without interpolation.
    #[must_use]
    pub const fn item(&self) -> &IncludeItem {
        &self.item
    }

    /// Returns raw include paths in declared order.
    #[must_use]
    pub fn paths(&self) -> &[Located<String>] {
        &self.paths
    }

    /// Returns raw include environment-file declarations in declared order.
    #[must_use]
    pub fn env_files(&self) -> &[Located<String>] {
        &self.env_files
    }

    /// Returns the raw optional project-directory declaration.
    #[must_use]
    pub const fn project_directory(&self) -> Option<&Located<String>> {
        self.project_directory.as_ref()
    }
}

/// The only authorization and I/O boundary used by recursive include traversal.
pub trait IncludeLoader {
    /// Authorizes and loads one requested included project.
    ///
    /// Implementations may use files, editor buffers, archives, URIs, or another caller policy.
    /// `ComposeLens` itself performs none of those operations.
    ///
    /// # Errors
    ///
    /// Returns an [`IncludeLoadError`] when caller policy denies the request or the selected
    /// external source cannot supply a project input.
    fn load_include(&self, request: &IncludeRequest) -> Result<IncludedProjectInput, IncludeLoadError>;
}

/// A caller-controlled include loading outcome that prevents traversal of one requested edge.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum IncludeLoadError {
    /// The caller's policy denied the request.
    Denied(String),
    /// The caller could not load an authorized request.
    Failed(String),
}

impl IncludeLoadError {
    /// Creates a policy-denial result without exposing it through diagnostics by default.
    #[must_use]
    pub fn denied(message: impl Into<String>) -> Self {
        Self::Denied(message.into())
    }

    /// Creates a loader-failure result without exposing it through diagnostics by default.
    #[must_use]
    pub fn failed(message: impl Into<String>) -> Self {
        Self::Failed(message.into())
    }
}

impl fmt::Display for IncludeLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Denied(message) | Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl Error for IncludeLoadError {}

/// One parsed document in an ordered Compose project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedDocument {
    origin: DocumentOrigin,
    syntax: SyntaxDocument,
    syntax_diagnostics: Vec<Diagnostic>,
    model: ModelParse,
}

impl LoadedDocument {
    /// Returns the document's source identifier.
    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        self.syntax.source_id()
    }

    /// Returns the explicit origin retained for this document.
    #[must_use]
    pub const fn origin(&self) -> &DocumentOrigin {
        &self.origin
    }

    /// Returns the loss-aware syntax document.
    #[must_use]
    pub const fn syntax(&self) -> &SyntaxDocument {
        &self.syntax
    }

    /// Returns recoverable YAML syntax diagnostics.
    #[must_use]
    pub fn syntax_diagnostics(&self) -> &[Diagnostic] {
        &self.syntax_diagnostics
    }

    /// Returns the recoverable typed-model parse result.
    #[must_use]
    pub const fn model(&self) -> &ModelParse {
        &self.model
    }

    /// Reports whether syntax and typed-model parsing emitted no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.syntax_diagnostics
            .iter()
            .chain(self.model.diagnostics())
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }
}

/// An ordered set of parsed Compose documents and their path origins.
///
/// File order is semantically significant. The first document supplies the project directory used
/// by Compose's multi-file relative-path rules, while every document retains its own origin for
/// provenance and future `include` support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProject {
    documents: Vec<LoadedDocument>,
    base_directory: PathBuf,
    diagnostics: Vec<Diagnostic>,
}

impl LoadedProject {
    /// Parses ordered, caller-supplied documents into a loaded project.
    ///
    /// Recoverable YAML and typed-model problems remain available in [`Self::diagnostics`]. No
    /// interpolation or merge is performed by this operation.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectLoadError`] when no document is supplied, a source identifier is reused, or
    /// one source exceeds the syntax tree's byte-offset capacity.
    pub fn load(inputs: impl IntoIterator<Item = DocumentInput>) -> Result<Self, ProjectLoadError> {
        let inputs: Vec<_> = inputs.into_iter().collect();
        let Some(first) = inputs.first() else {
            return Err(ProjectLoadError::EmptyProject);
        };

        let mut source_origins = BTreeMap::new();
        for input in &inputs {
            if let Some(first_origin) = source_origins.insert(input.source_id, input.origin.label.clone()) {
                return Err(ProjectLoadError::DuplicateSourceId {
                    source_id: input.source_id,
                    first_origin,
                    duplicate_origin: input.origin.label.clone(),
                });
            }
        }

        let base_directory = first.origin.directory.clone();
        let mut documents = Vec::with_capacity(inputs.len());
        let mut diagnostics = Vec::new();
        for input in inputs {
            let syntax = SyntaxDocument::parse(input.source_id, input.source).map_err(|error| {
                ProjectLoadError::SyntaxCapacity {
                    origin: input.origin.clone(),
                    error,
                }
            })?;
            let (syntax, syntax_diagnostics) = syntax.into_parts();
            let model = ComposeDocument::parse(&syntax);
            diagnostics.extend(syntax_diagnostics.iter().cloned());
            diagnostics.extend(model.diagnostics().iter().cloned());
            documents.push(LoadedDocument {
                origin: input.origin,
                syntax,
                syntax_diagnostics,
                model,
            });
        }

        Ok(Self {
            documents,
            base_directory,
            diagnostics,
        })
    }

    /// Returns documents in caller-supplied merge order.
    #[must_use]
    pub fn documents(&self) -> &[LoadedDocument] {
        &self.documents
    }

    /// Finds a document by its unique source identifier.
    #[must_use]
    pub fn document(&self, source_id: SourceId) -> Option<&LoadedDocument> {
        self.documents.iter().find(|document| document.source_id() == source_id)
    }

    /// Returns the project directory inherited from the first document.
    #[must_use]
    pub fn base_directory(&self) -> &Path {
        &self.base_directory
    }

    /// Returns aggregated syntax and typed-model diagnostics in document order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether loading emitted no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }

    /// Interpolates each document independently, in file order, without modifying the project.
    #[must_use]
    pub fn interpolate(&self, environment: &dyn EnvironmentProvider) -> ProjectInterpolation {
        self.interpolate_with_options(environment, InterpolationOptions::default())
    }

    /// Interpolates each document independently with explicit options.
    #[must_use]
    pub fn interpolate_with_options(
        &self,
        environment: &dyn EnvironmentProvider,
        options: InterpolationOptions,
    ) -> ProjectInterpolation {
        let documents: Vec<_> = self
            .documents
            .iter()
            .map(|document| interpolate_document_with_options(document.syntax(), environment, options))
            .collect();
        let diagnostics = documents
            .iter()
            .flat_map(|document| document.diagnostics().iter().cloned())
            .collect();
        ProjectInterpolation { documents, diagnostics }
    }
}

/// One visited project occurrence in an [`IncludeResolution`].
///
/// A repeated identity is retained as a separate occurrence when it is reached by distinct
/// non-cyclic edges. Traversal intentionally does not cache those diamonds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeNode {
    identity: IncludeIdentity,
    inputs: IncludedProjectInput,
    origins: Vec<DocumentOrigin>,
    loaded_project: Option<LoadedProject>,
    merged_project: Option<MergedProject>,
    project_view: Option<ProjectView>,
}

impl IncludeNode {
    /// Returns this occurrence's caller-defined identity.
    #[must_use]
    pub const fn identity(&self) -> &IncludeIdentity {
        &self.identity
    }

    /// Returns the complete caller-created input retained for this occurrence.
    #[must_use]
    pub const fn inputs(&self) -> &IncludedProjectInput {
        &self.inputs
    }

    /// Returns source origins in the input's merge order.
    #[must_use]
    pub fn origins(&self) -> &[DocumentOrigin] {
        &self.origins
    }

    /// Returns the ordered loaded project when no fatal load boundary failed.
    #[must_use]
    pub const fn loaded_project(&self) -> Option<&LoadedProject> {
        self.loaded_project.as_ref()
    }

    /// Returns the authored, no-interpolation merge when a mapping-root project was available.
    #[must_use]
    pub const fn merged_project(&self) -> Option<&MergedProject> {
        self.merged_project.as_ref()
    }

    /// Returns the effective typed project view used to discover child include declarations.
    #[must_use]
    pub const fn project_view(&self) -> Option<&ProjectView> {
        self.project_view.as_ref()
    }
}

/// One requested include edge in traversal order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeEdge {
    parent: IncludeIdentity,
    child: IncludeIdentity,
    request_index: usize,
}

impl IncludeEdge {
    /// Returns the identity that declared the include.
    #[must_use]
    pub const fn parent(&self) -> &IncludeIdentity {
        &self.parent
    }

    /// Returns the identity supplied by the caller's loader for the child.
    #[must_use]
    pub const fn child(&self) -> &IncludeIdentity {
        &self.child
    }

    /// Returns the matching [`IncludeResolution::requests`] index.
    #[must_use]
    pub const fn request_index(&self) -> usize {
        self.request_index
    }
}

/// A partial or complete depth-first traversal of caller-authorized Compose includes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeResolution {
    root: IncludeIdentity,
    nodes: Vec<IncludeNode>,
    edges: Vec<IncludeEdge>,
    requests: Vec<IncludeRequest>,
    diagnostics: Vec<Diagnostic>,
}

impl IncludeResolution {
    /// Loads and traverses one root project in depth-first effective-include order.
    ///
    /// Each node uses the established ordered loader, then an authored no-interpolation merge and
    /// native project view before child declarations are considered. The returned graph remains
    /// inspectable after loader denials, loader failures, malformed declarations, duplicate source
    /// identifiers, cycles, or malformed project inputs. Resources from children are never merged
    /// or imported into their parents.
    #[must_use]
    pub fn load(root: IncludedProjectInput, loader: &dyn IncludeLoader) -> Self {
        let root_identity = root.identity().clone();
        let mut resolution = Self {
            root: root_identity.clone(),
            nodes: Vec::new(),
            edges: Vec::new(),
            requests: Vec::new(),
            diagnostics: Vec::new(),
        };
        let mut active = Vec::new();
        let mut source_origins = BTreeMap::new();
        visit_project(root, loader, &mut active, &mut source_origins, &mut resolution, None);
        resolution
    }

    /// Returns the root identity selected by the caller.
    #[must_use]
    pub const fn root(&self) -> &IncludeIdentity {
        &self.root
    }

    /// Returns visited project occurrences in depth-first traversal order.
    #[must_use]
    pub fn nodes(&self) -> &[IncludeNode] {
        &self.nodes
    }

    /// Returns successful loader edges in request order.
    #[must_use]
    pub fn edges(&self) -> &[IncludeEdge] {
        &self.edges
    }

    /// Returns every effective declaration submitted to the caller's loader in order.
    #[must_use]
    pub fn requests(&self) -> &[IncludeRequest] {
        &self.requests
    }

    /// Returns diagnostics from every reached loading, merge, view, and traversal boundary.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether traversal reached no error diagnostic.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }
}

fn visit_project(
    input: IncludedProjectInput,
    include_loader: &dyn IncludeLoader,
    active: &mut Vec<IncludeIdentity>,
    source_origins: &mut BTreeMap<SourceId, DocumentOrigin>,
    resolution: &mut IncludeResolution,
    request_span: Option<SourceSpan>,
) {
    if input.documents().is_empty() {
        resolution.diagnostics.push(include_diagnostic(
            if request_span.is_some() {
                INCLUDE_EMPTY_RESULT
            } else {
                INCLUDE_EMPTY_ROOT
            },
            "included Compose project contains no documents",
            request_span,
        ));
        retain_unloaded_node(input, resolution);
        return;
    }

    if !register_source_ids(input.documents(), source_origins, resolution, request_span) {
        retain_unloaded_node(input, resolution);
        return;
    }

    let Some(prepared) = load_include_node(input, resolution, request_span) else {
        return;
    };
    let PreparedIncludeNode {
        identity,
        base_directory,
        items,
        node_index,
    } = prepared;
    active.push(identity.clone());

    for item in items {
        let declaration_span = include_item_span(&item);
        let declaration_origin = declaration_span.and_then(|span| {
            resolution.nodes[node_index]
                .inputs()
                .documents()
                .iter()
                .find(|document| document.source_id() == span.source_id())
                .map(|document| document.origin().clone())
        });
        let Some(request) =
            IncludeRequest::from_item(identity.clone(), base_directory.clone(), declaration_origin, item)
        else {
            resolution.diagnostics.push(include_diagnostic(
                INCLUDE_UNMODELED,
                "effective include declaration is malformed or contains unmodeled members",
                declaration_span,
            ));
            continue;
        };
        let request_index = resolution.requests.len();
        let request_span = request.declaration_span();
        let child = match include_loader.load_include(&request) {
            Ok(child) => child,
            Err(IncludeLoadError::Denied(_)) => {
                resolution.diagnostics.push(include_diagnostic(
                    INCLUDE_LOADER_DENIED,
                    "include loader denied this declaration",
                    Some(request_span),
                ));
                resolution.requests.push(request);
                continue;
            }
            Err(IncludeLoadError::Failed(_)) => {
                resolution.diagnostics.push(include_diagnostic(
                    INCLUDE_LOADER_FAILED,
                    "include loader failed to supply this declaration",
                    Some(request_span),
                ));
                resolution.requests.push(request);
                continue;
            }
        };
        let child_identity = child.identity().clone();
        resolution.requests.push(request);
        resolution.edges.push(IncludeEdge {
            parent: identity.clone(),
            child: child_identity.clone(),
            request_index,
        });
        if active.contains(&child_identity) {
            resolution.diagnostics.push(
                include_diagnostic(
                    INCLUDE_CYCLE,
                    "include identity is already active in this traversal",
                    Some(request_span),
                )
                .with_note("the caller controls identity canonicalization"),
            );
            continue;
        }
        visit_project(
            child,
            include_loader,
            active,
            source_origins,
            resolution,
            Some(request_span),
        );
    }

    let popped = active.pop();
    debug_assert_eq!(popped.as_ref(), Some(&identity));
}

struct PreparedIncludeNode {
    identity: IncludeIdentity,
    base_directory: PathBuf,
    items: Vec<IncludeItem>,
    node_index: usize,
}

fn load_include_node(
    input: IncludedProjectInput,
    resolution: &mut IncludeResolution,
    request_span: Option<SourceSpan>,
) -> Option<PreparedIncludeNode> {
    let identity = input.identity().clone();
    let origins = input
        .documents()
        .iter()
        .map(|document| document.origin().clone())
        .collect();
    let Ok(project) = LoadedProject::load(input.documents().iter().cloned()) else {
        resolution.diagnostics.push(include_diagnostic(
            INCLUDE_PROJECT_LOAD_FAILED,
            "included Compose project could not enter the ordered loader",
            request_span,
        ));
        resolution.nodes.push(IncludeNode {
            identity,
            inputs: input,
            origins,
            loaded_project: None,
            merged_project: None,
            project_view: None,
        });
        return None;
    };

    let merge = merge_project(&project, None);
    resolution.diagnostics.extend(merge.diagnostics().iter().cloned());
    let merged_project = merge.project().cloned();
    let (project_view, view_diagnostics) = match merged_project.as_ref() {
        Some(merged) => build_project_view(merged, None).into_parts(),
        None => (None, Vec::new()),
    };
    resolution.diagnostics.extend(view_diagnostics);
    let base_directory = project.base_directory().to_path_buf();
    let items = project_view
        .as_ref()
        .and_then(ProjectView::include)
        .map(|includes| includes.value().items().to_vec())
        .unwrap_or_default();
    let node_index = resolution.nodes.len();
    resolution.nodes.push(IncludeNode {
        identity: identity.clone(),
        inputs: input,
        origins,
        loaded_project: Some(project),
        merged_project,
        project_view,
    });
    Some(PreparedIncludeNode {
        identity,
        base_directory,
        items,
        node_index,
    })
}

fn retain_unloaded_node(input: IncludedProjectInput, resolution: &mut IncludeResolution) {
    let origins = input
        .documents()
        .iter()
        .map(|document| document.origin().clone())
        .collect();
    resolution.nodes.push(IncludeNode {
        identity: input.identity().clone(),
        inputs: input,
        origins,
        loaded_project: None,
        merged_project: None,
        project_view: None,
    });
}

fn register_source_ids(
    documents: &[DocumentInput],
    source_origins: &mut BTreeMap<SourceId, DocumentOrigin>,
    resolution: &mut IncludeResolution,
    request_span: Option<SourceSpan>,
) -> bool {
    let mut accepted = true;
    for document in documents {
        match source_origins.entry(document.source_id()) {
            std::collections::btree_map::Entry::Occupied(_) => {
                resolution.diagnostics.push(include_diagnostic(
                    INCLUDE_DUPLICATE_SOURCE_ID,
                    "include traversal reused a caller-managed source identifier",
                    request_span,
                ));
                accepted = false;
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(document.origin().clone());
            }
        }
    }
    accepted
}

fn include_item_span(item: &IncludeItem) -> Option<SourceSpan> {
    match item {
        IncludeItem::Short(path) => Some(path.span()),
        IncludeItem::Long(include) => Some(include.span()),
        IncludeItem::Unmodeled => None,
    }
}

fn include_diagnostic(code: DiagnosticCode, message: &'static str, span: Option<SourceSpan>) -> Diagnostic {
    let diagnostic = Diagnostic::new(code, Severity::Error, message);
    match span {
        Some(span) => diagnostic.with_label(DiagnosticLabel::primary(span, "include declaration")),
        None => diagnostic,
    }
}

/// Per-file interpolation overlays for one loaded project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInterpolation {
    documents: Vec<DocumentInterpolation>,
    diagnostics: Vec<Diagnostic>,
}

impl ProjectInterpolation {
    /// Returns document overlays in file order.
    #[must_use]
    pub fn documents(&self) -> &[DocumentInterpolation] {
        &self.documents
    }

    /// Finds a document overlay by source identifier.
    #[must_use]
    pub fn document(&self, source_id: SourceId) -> Option<&DocumentInterpolation> {
        self.documents.iter().find(|document| document.source_id() == source_id)
    }

    /// Returns aggregated interpolation diagnostics in file order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether interpolation emitted no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }
}

/// A fatal project-loading failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectLoadError {
    /// At least one Compose document is required to establish ordering and a base directory.
    EmptyProject,
    /// Two inputs reused a caller-managed source identifier.
    DuplicateSourceId {
        /// The reused identifier.
        source_id: SourceId,
        /// The first document's display label.
        first_origin: String,
        /// The later document's display label.
        duplicate_origin: String,
    },
    /// A document exceeded the syntax tree's byte-offset capacity.
    SyntaxCapacity {
        /// The rejected document's origin.
        origin: DocumentOrigin,
        /// The underlying parser capacity error.
        error: SyntaxParseError,
    },
}

impl fmt::Display for ProjectLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProject => formatter.write_str("a Compose project requires at least one document"),
            Self::DuplicateSourceId {
                source_id,
                first_origin,
                duplicate_origin,
            } => write!(
                formatter,
                "{source_id} is assigned to both `{first_origin}` and `{duplicate_origin}`"
            ),
            Self::SyntaxCapacity { origin, error } => write!(formatter, "{}: {error}", origin.label),
        }
    }
}

impl Error for ProjectLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SyntaxCapacity { error, .. } => Some(error),
            Self::EmptyProject | Self::DuplicateSourceId { .. } => None,
        }
    }
}
