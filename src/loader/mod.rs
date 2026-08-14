//! Ordered, caller-supplied Compose project loading.

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::interpolation::{
    DocumentInterpolation, EnvironmentProvider, InterpolationOptions, interpolate_document_with_options,
};
use crate::merge::{MergedProject, merge_project};
use crate::model::{
    ComposeDocument, ConfigDefinition, IncludeItem, Located, ModelDefinition, ModelParse, NetworkDefinition,
    SecretDefinition, VolumeDefinition,
};
use crate::project::{ProjectResource, ProjectService, ProjectView, build_project_view};
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
/// An included definition collides with an already selected parent definition.
pub const INCLUDE_RESOURCE_CONFLICT: DiagnosticCode = DiagnosticCode::new("compose.include.resource-conflict");

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
    index: usize,
    identity: IncludeIdentity,
    inputs: IncludedProjectInput,
    origins: Vec<DocumentOrigin>,
    loaded_project: Option<LoadedProject>,
    merged_project: Option<MergedProject>,
    project_view: Option<ProjectView>,
}

impl IncludeNode {
    /// Returns this occurrence's stable index within [`IncludeResolution::nodes`].
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

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
    parent_node_index: usize,
    child_node_index: usize,
    request_index: usize,
    cycle: bool,
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

    /// Returns the occurrence index that declared this edge.
    #[must_use]
    pub const fn parent_node_index(&self) -> usize {
        self.parent_node_index
    }

    /// Returns the retained target occurrence index.
    ///
    /// A cycle targets its existing active occurrence; every other successful edge targets the
    /// retained occurrence loaded for that request.
    #[must_use]
    pub const fn child_node_index(&self) -> usize {
        self.child_node_index
    }

    /// Returns the matching [`IncludeResolution::requests`] index.
    #[must_use]
    pub const fn request_index(&self) -> usize {
        self.request_index
    }

    /// Reports whether this edge targets an already active occurrence.
    #[must_use]
    pub const fn is_cycle(&self) -> bool {
        self.cycle
    }
}

/// One of the six top-level definition namespaces composed from includes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum IncludeResourceNamespace {
    /// Compose services.
    Services,
    /// Compose networks.
    Networks,
    /// Compose volumes.
    Volumes,
    /// Compose configs.
    Configs,
    /// Compose secrets.
    Secrets,
    /// Individual Compose model definitions.
    Models,
}

impl IncludeResourceNamespace {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Services => "service",
            Self::Networks => "network",
            Self::Volumes => "volume",
            Self::Configs => "config",
            Self::Secrets => "secret",
            Self::Models => "model",
        }
    }
}

/// Source and occurrence evidence for one selected or conflicting definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeDefinitionEvidence {
    occurrence_index: usize,
    identity: IncludeIdentity,
    source: Option<SourceSpan>,
    source_label: Option<String>,
}

impl IncludeDefinitionEvidence {
    /// Returns the retained project occurrence index.
    #[must_use]
    pub const fn occurrence_index(&self) -> usize {
        self.occurrence_index
    }

    /// Returns the caller-defined identity of the retained project occurrence.
    #[must_use]
    pub const fn identity(&self) -> &IncludeIdentity {
        &self.identity
    }

    /// Returns the effective definition source, when the definition had one.
    #[must_use]
    pub const fn source(&self) -> Option<SourceSpan> {
        self.source
    }

    /// Returns the caller-defined label for [`Self::source`], when available.
    #[must_use]
    pub fn source_label(&self) -> Option<&str> {
        self.source_label.as_deref()
    }
}

/// One typed definition selected during include composition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeDefinition<T> {
    name: String,
    definition: T,
    evidence: IncludeDefinitionEvidence,
}

impl<T> IncludeDefinition<T> {
    /// Returns the Compose definition name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the typed effective definition.
    #[must_use]
    pub const fn definition(&self) -> &T {
        &self.definition
    }

    /// Returns occurrence and source evidence for this selected definition.
    #[must_use]
    pub const fn evidence(&self) -> &IncludeDefinitionEvidence {
        &self.evidence
    }
}

/// A same-name conflict that prevented an included definition from being imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeResourceConflict {
    namespace: IncludeResourceNamespace,
    name: String,
    edge_index: usize,
    incoming: IncludeDefinitionEvidence,
    incumbent: IncludeDefinitionEvidence,
}

impl IncludeResourceConflict {
    /// Returns the colliding Compose namespace.
    #[must_use]
    pub const fn namespace(&self) -> IncludeResourceNamespace {
        self.namespace
    }

    /// Returns the shared definition name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the include-edge index through which the incoming candidate was considered.
    #[must_use]
    pub const fn edge_index(&self) -> usize {
        self.edge_index
    }

    /// Returns the child-side candidate that was not imported.
    #[must_use]
    pub const fn incoming(&self) -> &IncludeDefinitionEvidence {
        &self.incoming
    }

    /// Returns the already selected parent-side candidate.
    #[must_use]
    pub const fn incumbent(&self) -> &IncludeDefinitionEvidence {
        &self.incumbent
    }
}

/// The typed definitions selected for one retained include occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeComposition {
    node_index: usize,
    services: Vec<IncludeDefinition<ProjectService>>,
    networks: Vec<IncludeDefinition<NetworkDefinition>>,
    volumes: Vec<IncludeDefinition<VolumeDefinition>>,
    configs: Vec<IncludeDefinition<ConfigDefinition>>,
    secrets: Vec<IncludeDefinition<SecretDefinition>>,
    models: Vec<IncludeDefinition<ModelDefinition>>,
}

impl IncludeComposition {
    /// Returns the retained occurrence represented by this composition.
    #[must_use]
    pub const fn node_index(&self) -> usize {
        self.node_index
    }

    /// Returns selected services in local-then-depth-first import order.
    #[must_use]
    pub fn services(&self) -> &[IncludeDefinition<ProjectService>] {
        &self.services
    }

    /// Finds a selected service by name.
    #[must_use]
    pub fn service(&self, name: &str) -> Option<&IncludeDefinition<ProjectService>> {
        self.services.iter().find(|definition| definition.name == name)
    }

    /// Returns selected networks in local-then-depth-first import order.
    #[must_use]
    pub fn networks(&self) -> &[IncludeDefinition<NetworkDefinition>] {
        &self.networks
    }

    /// Finds a selected network by name.
    #[must_use]
    pub fn network(&self, name: &str) -> Option<&IncludeDefinition<NetworkDefinition>> {
        self.networks.iter().find(|definition| definition.name == name)
    }

    /// Returns selected volumes in local-then-depth-first import order.
    #[must_use]
    pub fn volumes(&self) -> &[IncludeDefinition<VolumeDefinition>] {
        &self.volumes
    }

    /// Finds a selected volume by name.
    #[must_use]
    pub fn volume(&self, name: &str) -> Option<&IncludeDefinition<VolumeDefinition>> {
        self.volumes.iter().find(|definition| definition.name == name)
    }

    /// Returns selected configs in local-then-depth-first import order.
    #[must_use]
    pub fn configs(&self) -> &[IncludeDefinition<ConfigDefinition>] {
        &self.configs
    }

    /// Finds a selected config by name.
    #[must_use]
    pub fn config(&self, name: &str) -> Option<&IncludeDefinition<ConfigDefinition>> {
        self.configs.iter().find(|definition| definition.name == name)
    }

    /// Returns selected secrets in local-then-depth-first import order.
    #[must_use]
    pub fn secrets(&self) -> &[IncludeDefinition<SecretDefinition>] {
        &self.secrets
    }

    /// Finds a selected secret by name.
    #[must_use]
    pub fn secret(&self, name: &str) -> Option<&IncludeDefinition<SecretDefinition>> {
        self.secrets.iter().find(|definition| definition.name == name)
    }

    /// Returns selected individual model definitions in local-then-depth-first import order.
    #[must_use]
    pub fn models(&self) -> &[IncludeDefinition<ModelDefinition>] {
        &self.models
    }

    /// Finds a selected model definition by name.
    #[must_use]
    pub fn model(&self, name: &str) -> Option<&IncludeDefinition<ModelDefinition>> {
        self.models.iter().find(|definition| definition.name == name)
    }
}

/// The I/O-free outcome of composing an [`IncludeResolution`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeCompositionResult {
    compositions: Vec<IncludeComposition>,
    diagnostics: Vec<Diagnostic>,
    conflicts: Vec<IncludeResourceConflict>,
}

impl IncludeCompositionResult {
    /// Returns the root composition, when the root occurrence was retained.
    #[must_use]
    pub fn root(&self) -> Option<&IncludeComposition> {
        self.compositions.first()
    }

    /// Returns compositions indexed by [`IncludeNode::index`].
    #[must_use]
    pub fn compositions(&self) -> &[IncludeComposition] {
        &self.compositions
    }

    /// Finds one retained occurrence's composition by node index.
    #[must_use]
    pub fn composition(&self, node_index: usize) -> Option<&IncludeComposition> {
        self.compositions.get(node_index)
    }

    /// Returns traversal diagnostics followed by composition conflict warnings.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns every non-imported same-name candidate in deterministic import order.
    #[must_use]
    pub fn conflicts(&self) -> &[IncludeResourceConflict] {
        &self.conflicts
    }

    /// Reports whether traversal and composition emitted no error diagnostic.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }

    /// Reports whether traversal completed without errors and no include resource conflicted.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.is_valid() && self.conflicts.is_empty()
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

    /// Composes already loaded child occurrences without authorizing further loading or performing I/O.
    ///
    /// The pass first composes every non-cycle child recursively, then imports absent definitions
    /// into each parent after that parent's ordinary multi-file merge. Same-namespace same-name
    /// candidates remain separate conflict records and never enter ordinary Compose merge rules.
    #[must_use]
    pub fn compose(&self) -> IncludeCompositionResult {
        let mut diagnostics = self.diagnostics.clone();
        let mut conflicts = Vec::new();
        let mut compositions = vec![None; self.nodes.len()];
        for node_index in 0..self.nodes.len() {
            let _ = compose_node(node_index, self, &mut compositions, &mut diagnostics, &mut conflicts);
        }
        IncludeCompositionResult {
            compositions: compositions.into_iter().flatten().collect(),
            diagnostics,
            conflicts,
        }
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
    active: &mut Vec<(IncludeIdentity, usize)>,
    source_origins: &mut BTreeMap<SourceId, DocumentOrigin>,
    resolution: &mut IncludeResolution,
    request_span: Option<SourceSpan>,
) -> usize {
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
        return resolution.nodes.len() - 1;
    }

    if !register_source_ids(input.documents(), source_origins, resolution, request_span) {
        retain_unloaded_node(input, resolution);
        return resolution.nodes.len() - 1;
    }

    let Some(prepared) = load_include_node(input, resolution, request_span) else {
        return resolution.nodes.len() - 1;
    };
    let PreparedIncludeNode {
        identity,
        base_directory,
        items,
        node_index,
    } = prepared;
    active.push((identity.clone(), node_index));

    visit_includes(
        items,
        &identity,
        &base_directory,
        node_index,
        include_loader,
        (active, source_origins, resolution),
    );

    let popped = active.pop();
    debug_assert_eq!(popped.as_ref().map(|(identity, _)| identity), Some(&identity));
    node_index
}

fn visit_includes(
    items: Vec<IncludeItem>,
    identity: &IncludeIdentity,
    base_directory: &Path,
    node_index: usize,
    include_loader: &dyn IncludeLoader,
    state: (
        &mut Vec<(IncludeIdentity, usize)>,
        &mut BTreeMap<SourceId, DocumentOrigin>,
        &mut IncludeResolution,
    ),
) {
    let (active, source_origins, resolution) = state;
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
            IncludeRequest::from_item(identity.clone(), base_directory.to_path_buf(), declaration_origin, item)
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
        let edge_index = resolution.edges.len();
        if let Some((_, active_node_index)) = active
            .iter()
            .find(|(active_identity, _)| active_identity == &child_identity)
        {
            resolution.edges.push(IncludeEdge {
                parent: identity.clone(),
                child: child_identity,
                parent_node_index: node_index,
                child_node_index: *active_node_index,
                request_index,
                cycle: true,
            });
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
        let child_node_index = visit_project(
            child,
            include_loader,
            active,
            source_origins,
            resolution,
            Some(request_span),
        );
        resolution.edges.insert(
            edge_index,
            IncludeEdge {
                parent: identity.clone(),
                child: child_identity,
                parent_node_index: node_index,
                child_node_index,
                request_index,
                cycle: false,
            },
        );
    }
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
            index: resolution.nodes.len(),
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
        index: node_index,
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
        index: resolution.nodes.len(),
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

fn compose_node(
    node_index: usize,
    resolution: &IncludeResolution,
    compositions: &mut [Option<IncludeComposition>],
    diagnostics: &mut Vec<Diagnostic>,
    conflicts: &mut Vec<IncludeResourceConflict>,
) -> IncludeComposition {
    if let Some(composition) = &compositions[node_index] {
        return composition.clone();
    }

    let mut composition = local_composition(node_index, resolution);
    for (edge_index, edge) in resolution.edges.iter().enumerate() {
        if edge.parent_node_index != node_index || edge.cycle {
            continue;
        }
        let child = compose_node(edge.child_node_index, resolution, compositions, diagnostics, conflicts);
        import_definitions(
            IncludeResourceNamespace::Services,
            edge_index,
            &mut composition.services,
            &child.services,
            edge,
            diagnostics,
            conflicts,
        );
        import_definitions(
            IncludeResourceNamespace::Networks,
            edge_index,
            &mut composition.networks,
            &child.networks,
            edge,
            diagnostics,
            conflicts,
        );
        import_definitions(
            IncludeResourceNamespace::Volumes,
            edge_index,
            &mut composition.volumes,
            &child.volumes,
            edge,
            diagnostics,
            conflicts,
        );
        import_definitions(
            IncludeResourceNamespace::Configs,
            edge_index,
            &mut composition.configs,
            &child.configs,
            edge,
            diagnostics,
            conflicts,
        );
        import_definitions(
            IncludeResourceNamespace::Secrets,
            edge_index,
            &mut composition.secrets,
            &child.secrets,
            edge,
            diagnostics,
            conflicts,
        );
        import_definitions(
            IncludeResourceNamespace::Models,
            edge_index,
            &mut composition.models,
            &child.models,
            edge,
            diagnostics,
            conflicts,
        );
    }
    compositions[node_index] = Some(composition.clone());
    composition
}

fn local_composition(node_index: usize, resolution: &IncludeResolution) -> IncludeComposition {
    let node = &resolution.nodes[node_index];
    let Some(view) = node.project_view() else {
        return IncludeComposition {
            node_index,
            services: Vec::new(),
            networks: Vec::new(),
            volumes: Vec::new(),
            configs: Vec::new(),
            secrets: Vec::new(),
            models: Vec::new(),
        };
    };

    IncludeComposition {
        node_index,
        services: view
            .services()
            .iter()
            .map(|service| {
                local_definition(
                    service.name().value(),
                    service.clone(),
                    node_index,
                    node,
                    service
                        .provenance()
                        .effective_source()
                        .or_else(|| service.name().effective_source()),
                )
            })
            .collect(),
        networks: local_resources(view.networks(), node_index, node),
        volumes: local_resources(view.volumes(), node_index, node),
        configs: local_resources(view.configs(), node_index, node),
        secrets: local_resources(view.secrets(), node_index, node),
        models: view
            .models()
            .into_iter()
            .flat_map(|models| models.value().definitions())
            .map(|model| local_definition(model.key().value(), model.clone(), node_index, node, Some(model.span())))
            .collect(),
    }
}

fn local_resources<T: Clone>(
    resources: &[ProjectResource<T>],
    node_index: usize,
    node: &IncludeNode,
) -> Vec<IncludeDefinition<T>> {
    resources
        .iter()
        .map(|resource| {
            local_definition(
                resource.name().value(),
                resource.definition().value().clone(),
                node_index,
                node,
                resource
                    .definition()
                    .effective_source()
                    .or_else(|| resource.name().effective_source()),
            )
        })
        .collect()
}

fn local_definition<T>(
    name: &str,
    definition: T,
    node_index: usize,
    node: &IncludeNode,
    source: Option<SourceSpan>,
) -> IncludeDefinition<T> {
    let source_label = source.and_then(|source| {
        node.origins()
            .iter()
            .zip(node.inputs().documents())
            .find(|(_, input)| input.source_id() == source.source_id())
            .map(|(origin, _)| origin.label().to_owned())
    });
    IncludeDefinition {
        name: name.to_owned(),
        definition,
        evidence: IncludeDefinitionEvidence {
            occurrence_index: node_index,
            identity: node.identity().clone(),
            source,
            source_label,
        },
    }
}

fn import_definitions<T: Clone>(
    namespace: IncludeResourceNamespace,
    edge_index: usize,
    parent: &mut Vec<IncludeDefinition<T>>,
    child: &[IncludeDefinition<T>],
    edge: &IncludeEdge,
    diagnostics: &mut Vec<Diagnostic>,
    conflicts: &mut Vec<IncludeResourceConflict>,
) {
    for incoming in child {
        if let Some(incumbent) = parent.iter().find(|candidate| candidate.name == incoming.name) {
            let conflict = IncludeResourceConflict {
                namespace,
                name: incoming.name.clone(),
                edge_index,
                incoming: incoming.evidence.clone(),
                incumbent: incumbent.evidence.clone(),
            };
            diagnostics.push(include_resource_conflict_diagnostic(&conflict, edge));
            conflicts.push(conflict);
        } else {
            parent.push(incoming.clone());
        }
    }
}

fn include_resource_conflict_diagnostic(conflict: &IncludeResourceConflict, edge: &IncludeEdge) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        INCLUDE_RESOURCE_CONFLICT,
        Severity::Warning,
        format!(
            "included {} `{}` conflicts with an already selected definition",
            conflict.namespace.as_str(),
            conflict.name
        ),
    );
    if let Some(source) = conflict.incoming.source {
        diagnostic = diagnostic.with_label(DiagnosticLabel::primary(
            source,
            format!(
                "incoming {} `{}` from {}",
                conflict.namespace.as_str(),
                conflict.name,
                conflict.incoming.source_label().unwrap_or("included occurrence")
            ),
        ));
    }
    if let Some(source) = conflict.incumbent.source {
        diagnostic = diagnostic.with_label(DiagnosticLabel::secondary(
            source,
            format!(
                "incumbent {} `{}` from {}",
                conflict.namespace.as_str(),
                conflict.name,
                conflict.incumbent.source_label().unwrap_or("parent occurrence")
            ),
        ));
    }
    diagnostic.with_note(format!(
        "declaring include edge #{}, occurrence #{} ({}) -> #{} ({})",
        conflict.edge_index, edge.parent_node_index, edge.parent, edge.child_node_index, edge.child
    ))
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
