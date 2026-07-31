//! Ordered, caller-supplied Compose project loading.

use crate::diagnostic::{Diagnostic, Severity};
use crate::interpolation::{
    DocumentInterpolation, EnvironmentProvider, InterpolationOptions, interpolate_document_with_options,
};
use crate::model::{ComposeDocument, ModelParse};
use crate::source::SourceId;
use crate::syntax::{SyntaxDocument, SyntaxParseError};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
