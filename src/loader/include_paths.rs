//! Caller-owned planning for effective include project directories.

use super::{IncludeIdentity, IncludeResolution};
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::model::Located;
use crate::source::SourceSpan;
use std::fmt;
use std::path::{Path, PathBuf};

/// An explicit include project directory could not be resolved by caller policy.
pub const INCLUDE_PROJECT_DIRECTORY_UNRESOLVED: DiagnosticCode =
    DiagnosticCode::new("compose.include.project-directory-unresolved");

/// The successful result of resolving an explicit include project directory.
#[derive(Clone, PartialEq, Eq)]
pub enum IncludeProjectDirectoryResolution {
    /// The caller authorized this effective directory.
    Resolved(PathBuf),
    /// The caller intentionally deferred this declaration without producing an error.
    Deferred,
}

impl fmt::Debug for IncludeProjectDirectoryResolution {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resolved(_) => formatter.write_str("Resolved(<authorized-directory>)"),
            Self::Deferred => formatter.write_str("Deferred"),
        }
    }
}

/// A typed resolver failure that does not carry caller-controlled message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IncludeProjectDirectoryResolveError {
    /// Caller policy could not resolve the explicit declaration.
    Unresolved,
}

/// Context supplied to a caller-owned explicit project-directory resolver.
pub struct IncludeProjectDirectoryRequest<'a> {
    edge_index: usize,
    request_index: usize,
    parent_node_index: usize,
    child_node_index: usize,
    parent_identity: &'a IncludeIdentity,
    child_identity: &'a IncludeIdentity,
    parent_effective_directory: Option<&'a Path>,
    declaration: &'a Located<String>,
    child_first_document_directory: Option<&'a Path>,
}

impl IncludeProjectDirectoryRequest<'_> {
    /// Returns the index of the include edge that authorized this planning request.
    #[must_use]
    pub const fn edge_index(&self) -> usize {
        self.edge_index
    }

    /// Returns the index of the original include declaration request.
    #[must_use]
    pub const fn request_index(&self) -> usize {
        self.request_index
    }

    /// Returns the occurrence index that declared this include.
    #[must_use]
    pub const fn parent_node_index(&self) -> usize {
        self.parent_node_index
    }

    /// Returns the occurrence index of the included project.
    #[must_use]
    pub const fn child_node_index(&self) -> usize {
        self.child_node_index
    }

    /// Returns the caller-defined identity of the including occurrence.
    #[must_use]
    pub const fn parent_identity(&self) -> &IncludeIdentity {
        self.parent_identity
    }

    /// Returns the caller-defined identity of the included occurrence.
    #[must_use]
    pub const fn child_identity(&self) -> &IncludeIdentity {
        self.child_identity
    }

    /// Returns the recursively effective parent directory, when planning has one.
    #[must_use]
    pub const fn parent_effective_directory(&self) -> Option<&Path> {
        self.parent_effective_directory
    }

    /// Returns the raw, un-interpolated explicit declaration and its source span.
    #[must_use]
    pub const fn declaration(&self) -> &Located<String> {
        self.declaration
    }

    /// Returns the explicit declaration source span.
    #[must_use]
    pub const fn declaration_span(&self) -> SourceSpan {
        self.declaration.span()
    }

    /// Returns the child first-document directory retained by traversal, when available.
    #[must_use]
    pub const fn child_first_document_directory(&self) -> Option<&Path> {
        self.child_first_document_directory
    }
}

/// The only caller-owned policy boundary for explicit include project directories.
pub trait IncludeProjectDirectoryResolver {
    /// Resolves or defers one explicit include `project_directory` declaration.
    ///
    /// Implementations decide whether raw declarations are relative, absolute, opaque, URI-like,
    /// or otherwise valid. `ComposeLens` does not join, normalize, canonicalize, open, or inspect
    /// any path.
    ///
    /// # Errors
    ///
    /// Returns [`IncludeProjectDirectoryResolveError::Unresolved`] when caller policy cannot
    /// authorize an effective directory for the explicit declaration.
    fn resolve_project_directory(
        &self,
        request: &IncludeProjectDirectoryRequest<'_>,
    ) -> Result<IncludeProjectDirectoryResolution, IncludeProjectDirectoryResolveError>;
}

/// How one occurrence received its planned project directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IncludeProjectDirectoryStatus {
    /// The root reused its first caller-supplied document directory.
    Root,
    /// A child had no explicit declaration and reused its first document directory.
    Defaulted,
    /// Caller policy authorized an explicit declaration.
    Resolved,
    /// Caller policy deliberately deferred an explicit declaration.
    Deferred,
    /// Caller policy could not resolve an explicit declaration.
    Unresolved,
}

/// One deterministic occurrence entry in an [`IncludeProjectDirectoryPlan`].
#[derive(Clone, PartialEq, Eq)]
pub struct IncludeProjectDirectoryEntry {
    node_index: usize,
    identity: IncludeIdentity,
    status: IncludeProjectDirectoryStatus,
    effective_directory: Option<PathBuf>,
    edge_index: Option<usize>,
    request_index: Option<usize>,
    declaration: Option<Located<String>>,
}

impl fmt::Debug for IncludeProjectDirectoryEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IncludeProjectDirectoryEntry")
            .field("node_index", &self.node_index)
            .field("identity", &"<redacted-identity>")
            .field("status", &self.status)
            .field(
                "effective_directory",
                &self.effective_directory.as_ref().map(|_| "<authorized-directory>"),
            )
            .field("edge_index", &self.edge_index)
            .field("request_index", &self.request_index)
            .field(
                "declaration",
                &self.declaration.as_ref().map(|_| "<redacted-declaration>"),
            )
            .finish()
    }
}

impl IncludeProjectDirectoryEntry {
    /// Returns the retained include occurrence index represented by this entry.
    #[must_use]
    pub const fn node_index(&self) -> usize {
        self.node_index
    }

    /// Returns the caller-defined identity of this retained occurrence.
    #[must_use]
    pub const fn identity(&self) -> &IncludeIdentity {
        &self.identity
    }

    /// Returns how this occurrence received its planned directory.
    #[must_use]
    pub const fn status(&self) -> IncludeProjectDirectoryStatus {
        self.status
    }

    /// Returns the caller-authorized effective directory, when one is available.
    #[must_use]
    pub fn effective_directory(&self) -> Option<&Path> {
        self.effective_directory.as_deref()
    }

    /// Returns the edge through which this child occurred; root has no incoming edge.
    #[must_use]
    pub const fn edge_index(&self) -> Option<usize> {
        self.edge_index
    }

    /// Returns the originating include request index; root has no incoming request.
    #[must_use]
    pub const fn request_index(&self) -> Option<usize> {
        self.request_index
    }

    /// Returns the raw explicit declaration and its provenance, when one was authored.
    #[must_use]
    pub const fn declaration(&self) -> Option<&Located<String>> {
        self.declaration.as_ref()
    }
}

/// The I/O-free, caller-authorized project-directory plan for an include traversal.
#[derive(Clone, PartialEq, Eq)]
pub struct IncludeProjectDirectoryPlan {
    entries: Vec<IncludeProjectDirectoryEntry>,
    diagnostics: Vec<Diagnostic>,
}

impl fmt::Debug for IncludeProjectDirectoryPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IncludeProjectDirectoryPlan")
            .field("entries", &self.entries)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}

impl IncludeProjectDirectoryPlan {
    /// Returns entries in retained node-index order.
    #[must_use]
    pub fn entries(&self) -> &[IncludeProjectDirectoryEntry] {
        &self.entries
    }

    /// Finds an occurrence entry by its retained node index.
    #[must_use]
    pub fn entry(&self, node_index: usize) -> Option<&IncludeProjectDirectoryEntry> {
        self.entries.get(node_index)
    }

    /// Returns unchanged traversal diagnostics followed by planning diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether traversal and explicit directory resolution emitted no errors.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }

    /// Reports whether every retained occurrence has an effective directory and no errors.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.is_valid() && self.entries.iter().all(|entry| entry.effective_directory.is_some())
    }
}

pub(super) fn plan_project_directories(
    resolution: &IncludeResolution,
    resolver: &dyn IncludeProjectDirectoryResolver,
) -> IncludeProjectDirectoryPlan {
    let mut entries = vec![None; resolution.nodes.len()];
    let mut diagnostics = resolution.diagnostics.clone();
    if !resolution.nodes.is_empty() {
        let root = &resolution.nodes[0];
        entries[0] = Some(IncludeProjectDirectoryEntry {
            node_index: 0,
            identity: root.identity().clone(),
            status: IncludeProjectDirectoryStatus::Root,
            effective_directory: first_document_directory(root),
            edge_index: None,
            request_index: None,
            declaration: None,
        });
        plan_children(0, resolution, resolver, &mut entries, &mut diagnostics);
    }
    IncludeProjectDirectoryPlan {
        entries: entries.into_iter().flatten().collect(),
        diagnostics,
    }
}

fn plan_children(
    parent_node_index: usize,
    resolution: &IncludeResolution,
    resolver: &dyn IncludeProjectDirectoryResolver,
    entries: &mut [Option<IncludeProjectDirectoryEntry>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let parent_effective_directory = entries[parent_node_index]
        .as_ref()
        .and_then(IncludeProjectDirectoryEntry::effective_directory)
        .map(Path::to_path_buf);
    for (edge_index, edge) in resolution.edges.iter().enumerate() {
        if edge.parent_node_index != parent_node_index || edge.cycle {
            continue;
        }
        let child_node_index = edge.child_node_index;
        let request = &resolution.requests[edge.request_index];
        let declaration = request.project_directory();
        let child = &resolution.nodes[child_node_index];
        let child_first_document_directory = first_document_directory(child);
        let entry = match declaration {
            None => IncludeProjectDirectoryEntry {
                node_index: child_node_index,
                identity: child.identity().clone(),
                status: IncludeProjectDirectoryStatus::Defaulted,
                effective_directory: child_first_document_directory,
                edge_index: Some(edge_index),
                request_index: Some(edge.request_index),
                declaration: None,
            },
            Some(declaration) => {
                let context = IncludeProjectDirectoryRequest {
                    edge_index,
                    request_index: edge.request_index,
                    parent_node_index,
                    child_node_index,
                    parent_identity: &edge.parent,
                    child_identity: &edge.child,
                    parent_effective_directory: parent_effective_directory.as_deref(),
                    declaration,
                    child_first_document_directory: child_first_document_directory.as_deref(),
                };
                match resolver.resolve_project_directory(&context) {
                    Ok(IncludeProjectDirectoryResolution::Resolved(directory)) => IncludeProjectDirectoryEntry {
                        node_index: child_node_index,
                        identity: child.identity().clone(),
                        status: IncludeProjectDirectoryStatus::Resolved,
                        effective_directory: Some(directory),
                        edge_index: Some(edge_index),
                        request_index: Some(edge.request_index),
                        declaration: Some(declaration.clone()),
                    },
                    Ok(IncludeProjectDirectoryResolution::Deferred) => IncludeProjectDirectoryEntry {
                        node_index: child_node_index,
                        identity: child.identity().clone(),
                        status: IncludeProjectDirectoryStatus::Deferred,
                        effective_directory: None,
                        edge_index: Some(edge_index),
                        request_index: Some(edge.request_index),
                        declaration: Some(declaration.clone()),
                    },
                    Err(IncludeProjectDirectoryResolveError::Unresolved) => {
                        diagnostics.push(project_directory_unresolved_diagnostic(declaration.span()));
                        IncludeProjectDirectoryEntry {
                            node_index: child_node_index,
                            identity: child.identity().clone(),
                            status: IncludeProjectDirectoryStatus::Unresolved,
                            effective_directory: None,
                            edge_index: Some(edge_index),
                            request_index: Some(edge.request_index),
                            declaration: Some(declaration.clone()),
                        }
                    }
                }
            }
        };
        entries[child_node_index] = Some(entry);
        plan_children(child_node_index, resolution, resolver, entries, diagnostics);
    }
}

fn first_document_directory(node: &super::IncludeNode) -> Option<PathBuf> {
    node.inputs()
        .documents()
        .first()
        .map(|document| document.origin().directory().to_path_buf())
}

fn project_directory_unresolved_diagnostic(span: SourceSpan) -> Diagnostic {
    Diagnostic::new(
        INCLUDE_PROJECT_DIRECTORY_UNRESOLVED,
        Severity::Error,
        "included project directory could not be resolved",
    )
    .with_label(DiagnosticLabel::primary(span, "project directory declaration"))
}
