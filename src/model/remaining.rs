//! Structured values for the final closed-schema Compose keys.

use super::{BooleanValue, Command, ComposeScalar, Environment, FieldReference, Located};
use crate::source::SourceSpan;
use std::fmt;

/// An authored service `label_file` value with its scalar or list form retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelFiles {
    span: SourceSpan,
    form: LabelFilesForm,
    unmodeled_items: Vec<SourceSpan>,
}

impl LabelFiles {
    pub(crate) const fn new(span: SourceSpan, form: LabelFilesForm, unmodeled_items: Vec<SourceSpan>) -> Self {
        Self {
            span,
            form,
            unmodeled_items,
        }
    }

    /// Returns the complete authored field-value span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the exact scalar or ordered-list form without reading label files.
    #[must_use]
    pub const fn form(&self) -> &LabelFilesForm {
        &self.form
    }

    /// Returns spans for malformed list items retained in the syntax document.
    #[must_use]
    pub fn unmodeled_items(&self) -> &[SourceSpan] {
        &self.unmodeled_items
    }
}

/// The exact authored syntax form of service `label_file`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LabelFilesForm {
    /// One label-file path scalar.
    Scalar(Located<String>),
    /// An ordered list of label-file paths, including an explicitly empty list.
    List(Vec<Located<String>>),
}

/// Top-level Compose includes in their short and long forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Includes {
    span: SourceSpan,
    items: Vec<IncludeItem>,
    unmodeled_fields: Vec<FieldReference>,
}

#[allow(
    missing_docs,
    reason = "the documented type contract covers its conventional accessors"
)]
impl Includes {
    pub(crate) fn new(span: SourceSpan, items: Vec<IncludeItem>, unmodeled_fields: Vec<FieldReference>) -> Self {
        Self {
            span,
            items,
            unmodeled_fields,
        }
    }
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    #[must_use]
    pub fn items(&self) -> &[IncludeItem] {
        &self.items
    }
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[FieldReference] {
        &self.unmodeled_fields
    }
}

/// One authored include declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum IncludeItem {
    /// A short scalar path.
    Short(Located<String>),
    /// A long include mapping.
    Long(IncludeLong),
    /// An invalid form retained in the syntax document and reported diagnostically.
    Unmodeled,
}

/// Long-form include values. Paths remain inert source data: no documents or env files are read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeLong {
    span: SourceSpan,
    paths: Vec<Located<String>>,
    env_files: Vec<Located<String>>,
    project_directory: Option<Located<String>>,
    unmodeled_fields: Vec<FieldReference>,
}

#[allow(
    missing_docs,
    reason = "the documented type contract covers its conventional accessors"
)]
impl IncludeLong {
    pub(crate) fn new(
        span: SourceSpan,
        paths: Vec<Located<String>>,
        env_files: Vec<Located<String>>,
        project_directory: Option<Located<String>>,
        unmodeled_fields: Vec<FieldReference>,
    ) -> Self {
        Self {
            span,
            paths,
            env_files,
            project_directory,
            unmodeled_fields,
        }
    }
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    #[must_use]
    pub fn paths(&self) -> &[Located<String>] {
        &self.paths
    }
    #[must_use]
    pub fn env_files(&self) -> &[Located<String>] {
        &self.env_files
    }
    #[must_use]
    pub const fn project_directory(&self) -> Option<&Located<String>> {
        self.project_directory.as_ref()
    }
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[FieldReference] {
        &self.unmodeled_fields
    }
}

/// Top-level model definitions keyed by their Compose model name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDefinitions {
    span: SourceSpan,
    definitions: Vec<ModelDefinition>,
    unmodeled_fields: Vec<FieldReference>,
}

#[allow(
    missing_docs,
    reason = "the documented type contract covers its conventional accessors"
)]
impl ModelDefinitions {
    pub(crate) fn new(
        span: SourceSpan,
        definitions: Vec<ModelDefinition>,
        unmodeled_fields: Vec<FieldReference>,
    ) -> Self {
        Self {
            span,
            definitions,
            unmodeled_fields,
        }
    }
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    #[must_use]
    pub fn definitions(&self) -> &[ModelDefinition] {
        &self.definitions
    }
    #[must_use]
    pub fn definition(&self, name: &str) -> Option<&ModelDefinition> {
        self.definitions
            .iter()
            .find(|definition| definition.key.value() == name)
    }
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[FieldReference] {
        &self.unmodeled_fields
    }
}

/// One source-aware top-level model definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDefinition {
    key: Located<String>,
    span: SourceSpan,
    name: Option<Located<String>>,
    model: Option<Located<String>>,
    context_size: Option<Located<ComposeScalar>>,
    runtime_flags: Vec<Located<String>>,
    unmodeled_fields: Vec<FieldReference>,
}

#[allow(
    missing_docs,
    reason = "the documented type contract covers its conventional accessors"
)]
impl ModelDefinition {
    pub(crate) fn new(key: Located<String>, span: SourceSpan) -> Self {
        Self {
            key,
            span,
            name: None,
            model: None,
            context_size: None,
            runtime_flags: Vec::new(),
            unmodeled_fields: Vec::new(),
        }
    }
    pub(crate) fn set_name(&mut self, value: Located<String>) {
        self.name = Some(value);
    }
    pub(crate) fn set_model(&mut self, value: Located<String>) {
        self.model = Some(value);
    }
    pub(crate) fn set_context_size(&mut self, value: Located<ComposeScalar>) {
        self.context_size = Some(value);
    }
    pub(crate) fn set_runtime_flags(&mut self, values: Vec<Located<String>>) {
        self.runtime_flags = values;
    }
    pub(crate) fn push_unmodeled(&mut self, field: FieldReference) {
        self.unmodeled_fields.push(field);
    }
    #[must_use]
    pub const fn key(&self) -> &Located<String> {
        &self.key
    }
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    #[must_use]
    pub const fn name(&self) -> Option<&Located<String>> {
        self.name.as_ref()
    }
    #[must_use]
    pub const fn model(&self) -> Option<&Located<String>> {
        self.model.as_ref()
    }
    #[must_use]
    pub const fn context_size(&self) -> Option<&Located<ComposeScalar>> {
        self.context_size.as_ref()
    }
    #[must_use]
    pub fn runtime_flags(&self) -> &[Located<String>] {
        &self.runtime_flags
    }
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[FieldReference] {
        &self.unmodeled_fields
    }
}

/// Per-service bindings to top-level model definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceModels {
    span: SourceSpan,
    bindings: Vec<ServiceModelBinding>,
    unmodeled_fields: Vec<FieldReference>,
}

#[allow(
    missing_docs,
    reason = "the documented type contract covers its conventional accessors"
)]
impl ServiceModels {
    pub(crate) fn new(
        span: SourceSpan,
        bindings: Vec<ServiceModelBinding>,
        unmodeled_fields: Vec<FieldReference>,
    ) -> Self {
        Self {
            span,
            bindings,
            unmodeled_fields,
        }
    }
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    #[must_use]
    pub fn bindings(&self) -> &[ServiceModelBinding] {
        &self.bindings
    }
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[FieldReference] {
        &self.unmodeled_fields
    }
}

/// One service model binding from a scalar list or mapping form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceModelBinding {
    model: Located<String>,
    span: SourceSpan,
    endpoint_var: Option<Located<String>>,
    model_var: Option<Located<String>>,
    unmodeled_fields: Vec<FieldReference>,
}

#[allow(
    missing_docs,
    reason = "the documented type contract covers its conventional accessors"
)]
impl ServiceModelBinding {
    pub(crate) fn new(model: Located<String>, span: SourceSpan) -> Self {
        Self {
            model,
            span,
            endpoint_var: None,
            model_var: None,
            unmodeled_fields: Vec::new(),
        }
    }
    pub(crate) fn set_endpoint_var(&mut self, value: Located<String>) {
        self.endpoint_var = Some(value);
    }
    pub(crate) fn set_model_var(&mut self, value: Located<String>) {
        self.model_var = Some(value);
    }
    pub(crate) fn push_unmodeled(&mut self, field: FieldReference) {
        self.unmodeled_fields.push(field);
    }
    #[must_use]
    pub const fn model(&self) -> &Located<String> {
        &self.model
    }
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    #[must_use]
    pub const fn endpoint_var(&self) -> Option<&Located<String>> {
        self.endpoint_var.as_ref()
    }
    #[must_use]
    pub const fn model_var(&self) -> Option<&Located<String>> {
        self.model_var.as_ref()
    }
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[FieldReference] {
        &self.unmodeled_fields
    }
}

/// Service GPU declarations in scalar `all` or detailed list form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gpus {
    /// The portable scalar selector, normally `all`.
    All(Located<String>),
    /// Ordered long-form selectors.
    Devices {
        /// Span of the complete selector sequence.
        span: SourceSpan,
        /// Selectors in authored order.
        devices: Vec<GpuDevice>,
        /// Exact spans of malformed selector items retained in the syntax document.
        unmodeled_items: Vec<SourceSpan>,
    },
}

#[allow(missing_docs, reason = "the enum contract covers the shared span accessor")]
impl Gpus {
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::All(value) => value.span(),
            Self::Devices { span, .. } => *span,
        }
    }

    /// Returns malformed selector-item spans retained without device allocation interpretation.
    #[must_use]
    pub fn unmodeled_items(&self) -> &[SourceSpan] {
        match self {
            Self::All(_) => &[],
            Self::Devices { unmodeled_items, .. } => unmodeled_items,
        }
    }
}

/// One source-aware long-form GPU selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuDevice {
    span: SourceSpan,
    capabilities: Vec<Located<String>>,
    count: Option<Located<ComposeScalar>>,
    device_ids: Vec<Located<String>>,
    driver: Option<Located<String>>,
    options: Option<GpuOptions>,
    unmodeled_fields: Vec<FieldReference>,
}

#[allow(
    missing_docs,
    reason = "the documented type contract covers its conventional accessors"
)]
impl GpuDevice {
    pub(crate) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            capabilities: Vec::new(),
            count: None,
            device_ids: Vec::new(),
            driver: None,
            options: None,
            unmodeled_fields: Vec::new(),
        }
    }
    pub(crate) fn set_capabilities(&mut self, values: Vec<Located<String>>) {
        self.capabilities = values;
    }
    pub(crate) fn set_count(&mut self, value: Located<ComposeScalar>) {
        self.count = Some(value);
    }
    pub(crate) fn set_device_ids(&mut self, values: Vec<Located<String>>) {
        self.device_ids = values;
    }
    pub(crate) fn set_driver(&mut self, value: Located<String>) {
        self.driver = Some(value);
    }
    pub(crate) fn set_options(&mut self, value: GpuOptions) {
        self.options = Some(value);
    }
    pub(crate) fn push_unmodeled(&mut self, field: FieldReference) {
        self.unmodeled_fields.push(field);
    }
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    #[must_use]
    pub fn capabilities(&self) -> &[Located<String>] {
        &self.capabilities
    }
    #[must_use]
    pub const fn count(&self) -> Option<&Located<ComposeScalar>> {
        self.count.as_ref()
    }
    #[must_use]
    pub fn device_ids(&self) -> &[Located<String>] {
        &self.device_ids
    }
    #[must_use]
    pub const fn driver(&self) -> Option<&Located<String>> {
        self.driver.as_ref()
    }
    #[must_use]
    pub const fn options(&self) -> Option<&GpuOptions> {
        self.options.as_ref()
    }
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[FieldReference] {
        &self.unmodeled_fields
    }
}

/// GPU selector options in their authored mapping or list form.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GpuOptions {
    /// Ordered mapping entries with scalar values.
    Mapping(Vec<super::KeyValueEntry>),
    /// Ordered raw string option entries.
    List(Vec<Located<String>>),
}

impl GpuOptions {
    /// Returns mapping entries when the selector used mapping syntax.
    #[must_use]
    pub fn as_mapping(&self) -> Option<&[super::KeyValueEntry]> {
        let Self::Mapping(entries) = self else {
            return None;
        };
        Some(entries)
    }

    /// Returns list entries when the selector used list syntax.
    #[must_use]
    pub fn as_list(&self) -> Option<&[Located<String>]> {
        let Self::List(items) = self else {
            return None;
        };
        Some(items)
    }
}

/// Side-effect-free `develop` watch configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Develop {
    span: SourceSpan,
    watch: Vec<DevelopWatch>,
    unmodeled_fields: Vec<FieldReference>,
    unmodeled_items: Vec<SourceSpan>,
}

#[allow(
    missing_docs,
    reason = "the documented type contract covers its conventional accessors"
)]
impl Develop {
    pub(crate) fn new(
        span: SourceSpan,
        watch: Vec<DevelopWatch>,
        unmodeled_fields: Vec<FieldReference>,
        unmodeled_items: Vec<SourceSpan>,
    ) -> Self {
        Self {
            span,
            watch,
            unmodeled_fields,
            unmodeled_items,
        }
    }
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    #[must_use]
    pub fn watch(&self) -> &[DevelopWatch] {
        &self.watch
    }
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[FieldReference] {
        &self.unmodeled_fields
    }

    /// Returns exact spans of malformed watch items retained in the syntax document.
    #[must_use]
    pub fn unmodeled_items(&self) -> &[SourceSpan] {
        &self.unmodeled_items
    }
}

/// One declared watch action. The type deliberately never watches a path or executes a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopWatch {
    span: SourceSpan,
    action: Option<Located<String>>,
    path: Option<Located<String>>,
    target: Option<Located<String>>,
    ignore: Vec<Located<String>>,
    include: Vec<Located<String>>,
    initial_sync: Option<Located<BooleanValue>>,
    exec: Option<DevelopWatchExec>,
    unmodeled_fields: Vec<FieldReference>,
}

#[allow(
    missing_docs,
    reason = "the documented type contract covers its conventional accessors"
)]
impl DevelopWatch {
    pub(crate) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            action: None,
            path: None,
            target: None,
            ignore: Vec::new(),
            include: Vec::new(),
            initial_sync: None,
            exec: None,
            unmodeled_fields: Vec::new(),
        }
    }
    pub(crate) fn set_action(&mut self, value: Located<String>) {
        self.action = Some(value);
    }
    pub(crate) fn set_path(&mut self, value: Located<String>) {
        self.path = Some(value);
    }
    pub(crate) fn set_target(&mut self, value: Located<String>) {
        self.target = Some(value);
    }
    pub(crate) fn set_ignore(&mut self, values: Vec<Located<String>>) {
        self.ignore = values;
    }
    pub(crate) fn set_include(&mut self, values: Vec<Located<String>>) {
        self.include = values;
    }
    pub(crate) fn set_initial_sync(&mut self, value: Located<BooleanValue>) {
        self.initial_sync = Some(value);
    }
    pub(crate) fn set_exec(&mut self, value: DevelopWatchExec) {
        self.exec = Some(value);
    }
    pub(crate) fn push_unmodeled(&mut self, field: FieldReference) {
        self.unmodeled_fields.push(field);
    }
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    #[must_use]
    pub const fn action(&self) -> Option<&Located<String>> {
        self.action.as_ref()
    }
    #[must_use]
    pub const fn path(&self) -> Option<&Located<String>> {
        self.path.as_ref()
    }
    #[must_use]
    pub const fn target(&self) -> Option<&Located<String>> {
        self.target.as_ref()
    }
    #[must_use]
    pub fn ignore(&self) -> &[Located<String>] {
        &self.ignore
    }
    #[must_use]
    pub fn include(&self) -> &[Located<String>] {
        &self.include
    }
    #[must_use]
    pub const fn initial_sync(&self) -> Option<&Located<BooleanValue>> {
        self.initial_sync.as_ref()
    }
    #[must_use]
    pub const fn exec(&self) -> Option<&DevelopWatchExec> {
        self.exec.as_ref()
    }
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[FieldReference] {
        &self.unmodeled_fields
    }
}

/// Side-effect-free `develop.watch.exec` configuration.
///
/// This only retains the hook declaration. It never starts a watcher, executes
/// a command, resolves a user, or reads an environment file.
#[derive(Clone, PartialEq, Eq)]
pub struct DevelopWatchExec {
    span: SourceSpan,
    command: Option<Command>,
    user: Option<Located<String>>,
    privileged: Option<Located<BooleanValue>>,
    working_dir: Option<Located<String>>,
    environment: Option<Environment>,
    unmodeled_fields: Vec<FieldReference>,
}

impl fmt::Debug for DevelopWatchExec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("DevelopWatchExec");
        debug
            .field("span", &self.span)
            .field("command", &self.command)
            .field("user", &self.user)
            .field("privileged", &self.privileged)
            .field("working_dir", &self.working_dir)
            .field("environment", &self.environment.as_ref().map(|_| "<redacted>"))
            .field("unmodeled_fields", &self.unmodeled_fields)
            .finish()
    }
}

#[allow(
    missing_docs,
    reason = "the documented type contract covers its conventional accessors"
)]
impl DevelopWatchExec {
    pub(crate) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            command: None,
            user: None,
            privileged: None,
            working_dir: None,
            environment: None,
            unmodeled_fields: Vec::new(),
        }
    }
    pub(crate) fn set_command(&mut self, value: Command) {
        self.command = Some(value);
    }
    pub(crate) fn set_user(&mut self, value: Located<String>) {
        self.user = Some(value);
    }
    pub(crate) fn set_privileged(&mut self, value: Located<BooleanValue>) {
        self.privileged = Some(value);
    }
    pub(crate) fn set_working_dir(&mut self, value: Located<String>) {
        self.working_dir = Some(value);
    }
    pub(crate) fn set_environment(&mut self, value: Environment) {
        self.environment = Some(value);
    }
    pub(crate) fn push_unmodeled(&mut self, value: FieldReference) {
        self.unmodeled_fields.push(value);
    }
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    #[must_use]
    pub const fn command(&self) -> Option<&Command> {
        self.command.as_ref()
    }
    #[must_use]
    pub const fn user(&self) -> Option<&Located<String>> {
        self.user.as_ref()
    }
    #[must_use]
    pub const fn privileged(&self) -> Option<&Located<BooleanValue>> {
        self.privileged.as_ref()
    }
    #[must_use]
    pub const fn working_dir(&self) -> Option<&Located<String>> {
        self.working_dir.as_ref()
    }
    #[must_use]
    pub const fn environment(&self) -> Option<&Environment> {
        self.environment.as_ref()
    }
    #[must_use]
    pub fn unmodeled_fields(&self) -> &[FieldReference] {
        &self.unmodeled_fields
    }
}
