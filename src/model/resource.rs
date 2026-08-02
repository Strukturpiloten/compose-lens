//! Top-level volumes, configs, secrets, and service grants.

use super::{BooleanValue, FieldReference, KeyValueEntry, Labels, Located};
use crate::source::SourceSpan;

/// A typed top-level volume definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeDefinition {
    name: Located<String>,
    span: SourceSpan,
    driver: Option<Located<String>>,
    driver_opts: Vec<KeyValueEntry>,
    external: Option<Located<BooleanValue>>,
    labels: Option<Labels>,
    custom_name: Option<Located<String>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl VolumeDefinition {
    pub(crate) fn new(name: Located<String>, span: SourceSpan) -> Self {
        Self {
            name,
            span,
            driver: None,
            driver_opts: Vec::new(),
            external: None,
            labels: None,
            custom_name: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }
    pub(crate) fn set_driver(&mut self, value: Located<String>) {
        self.driver = Some(value);
    }
    pub(crate) fn set_driver_opts(&mut self, value: Vec<KeyValueEntry>) {
        self.driver_opts = value;
    }
    pub(crate) fn set_external(&mut self, value: Located<BooleanValue>) {
        self.external = Some(value);
    }
    pub(crate) fn set_labels(&mut self, value: Labels) {
        self.labels = Some(value);
    }
    pub(crate) fn set_custom_name(&mut self, value: Located<String>) {
        self.custom_name = Some(value);
    }
    pub(crate) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }
    pub(crate) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the model identifier.
    #[must_use]
    pub const fn name(&self) -> &Located<String> {
        &self.name
    }
    /// Returns the complete definition span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns the volume driver.
    #[must_use]
    pub const fn driver(&self) -> Option<&Located<String>> {
        self.driver.as_ref()
    }
    /// Returns driver options.
    #[must_use]
    pub fn driver_opts(&self) -> &[KeyValueEntry] {
        &self.driver_opts
    }
    /// Returns the external-lifecycle setting.
    #[must_use]
    pub const fn external(&self) -> Option<&Located<BooleanValue>> {
        self.external.as_ref()
    }
    /// Returns labels with their syntax form retained.
    #[must_use]
    pub const fn labels(&self) -> Option<&Labels> {
        self.labels.as_ref()
    }
    /// Returns the platform-level custom name.
    #[must_use]
    pub const fn custom_name(&self) -> Option<&Located<String>> {
        self.custom_name.as_ref()
    }
    /// Returns retained `x-` fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }
    /// Returns unrecognized fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// A typed top-level config definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDefinition {
    name: Located<String>,
    span: SourceSpan,
    file: Option<Located<String>>,
    environment: Option<Located<String>>,
    content: Option<Located<String>>,
    external: Option<Located<BooleanValue>>,
    custom_name: Option<Located<String>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl ConfigDefinition {
    pub(crate) fn new(name: Located<String>, span: SourceSpan) -> Self {
        Self {
            name,
            span,
            file: None,
            environment: None,
            content: None,
            external: None,
            custom_name: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }
    pub(crate) fn set_file(&mut self, value: Located<String>) {
        self.file = Some(value);
    }
    pub(crate) fn set_environment(&mut self, value: Located<String>) {
        self.environment = Some(value);
    }
    pub(crate) fn set_content(&mut self, value: Located<String>) {
        self.content = Some(value);
    }
    pub(crate) fn set_external(&mut self, value: Located<BooleanValue>) {
        self.external = Some(value);
    }
    pub(crate) fn set_custom_name(&mut self, value: Located<String>) {
        self.custom_name = Some(value);
    }
    pub(crate) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }
    pub(crate) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the model identifier.
    #[must_use]
    pub const fn name(&self) -> &Located<String> {
        &self.name
    }
    /// Returns the complete definition span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns the source file path.
    #[must_use]
    pub const fn file(&self) -> Option<&Located<String>> {
        self.file.as_ref()
    }
    /// Returns the host environment-variable source name.
    #[must_use]
    pub const fn environment(&self) -> Option<&Located<String>> {
        self.environment.as_ref()
    }
    /// Returns inline config content.
    #[must_use]
    pub const fn content(&self) -> Option<&Located<String>> {
        self.content.as_ref()
    }
    /// Returns the external-lifecycle setting.
    #[must_use]
    pub const fn external(&self) -> Option<&Located<BooleanValue>> {
        self.external.as_ref()
    }
    /// Returns the platform-level custom name.
    #[must_use]
    pub const fn custom_name(&self) -> Option<&Located<String>> {
        self.custom_name.as_ref()
    }
    /// Returns retained `x-` fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }
    /// Returns unrecognized fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// A typed top-level secret definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretDefinition {
    name: Located<String>,
    span: SourceSpan,
    file: Option<Located<String>>,
    environment: Option<Located<String>>,
    external: Option<Located<BooleanValue>>,
    custom_name: Option<Located<String>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl SecretDefinition {
    pub(crate) fn new(name: Located<String>, span: SourceSpan) -> Self {
        Self {
            name,
            span,
            file: None,
            environment: None,
            external: None,
            custom_name: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }
    pub(crate) fn set_file(&mut self, value: Located<String>) {
        self.file = Some(value);
    }
    pub(crate) fn set_environment(&mut self, value: Located<String>) {
        self.environment = Some(value);
    }
    pub(crate) fn set_external(&mut self, value: Located<BooleanValue>) {
        self.external = Some(value);
    }
    pub(crate) fn set_custom_name(&mut self, value: Located<String>) {
        self.custom_name = Some(value);
    }
    pub(crate) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }
    pub(crate) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the model identifier.
    #[must_use]
    pub const fn name(&self) -> &Located<String> {
        &self.name
    }
    /// Returns the complete definition span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns the source file path.
    #[must_use]
    pub const fn file(&self) -> Option<&Located<String>> {
        self.file.as_ref()
    }
    /// Returns the host environment-variable source name.
    #[must_use]
    pub const fn environment(&self) -> Option<&Located<String>> {
        self.environment.as_ref()
    }
    /// Returns the external-lifecycle setting.
    #[must_use]
    pub const fn external(&self) -> Option<&Located<BooleanValue>> {
        self.external.as_ref()
    }
    /// Returns the platform-level custom name.
    #[must_use]
    pub const fn custom_name(&self) -> Option<&Located<String>> {
        self.custom_name.as_ref()
    }
    /// Returns retained `x-` fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }
    /// Returns unrecognized fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// Long syntax shared by service config and secret grants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongGrant {
    span: SourceSpan,
    source: Option<Located<String>>,
    target: Option<Located<String>>,
    uid: Option<Located<String>>,
    gid: Option<Located<String>>,
    mode: Option<Located<String>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl LongGrant {
    pub(crate) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            source: None,
            target: None,
            uid: None,
            gid: None,
            mode: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }
    pub(crate) fn set_source(&mut self, value: Located<String>) {
        self.source = Some(value);
    }
    pub(crate) fn set_target(&mut self, value: Located<String>) {
        self.target = Some(value);
    }
    pub(crate) fn set_uid(&mut self, value: Located<String>) {
        self.uid = Some(value);
    }
    pub(crate) fn set_gid(&mut self, value: Located<String>) {
        self.gid = Some(value);
    }
    pub(crate) fn set_mode(&mut self, value: Located<String>) {
        self.mode = Some(value);
    }
    pub(crate) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }
    pub(crate) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns the referenced resource name.
    #[must_use]
    pub const fn source(&self) -> Option<&Located<String>> {
        self.source.as_ref()
    }
    /// Returns the container target.
    #[must_use]
    pub const fn target(&self) -> Option<&Located<String>> {
        self.target.as_ref()
    }
    /// Returns the requested user ID spelling.
    #[must_use]
    pub const fn uid(&self) -> Option<&Located<String>> {
        self.uid.as_ref()
    }
    /// Returns the requested group ID spelling.
    #[must_use]
    pub const fn gid(&self) -> Option<&Located<String>> {
        self.gid.as_ref()
    }
    /// Returns the requested permission-mode spelling.
    #[must_use]
    pub const fn mode(&self) -> Option<&Located<String>> {
        self.mode.as_ref()
    }
    /// Returns retained `x-` fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }
    /// Returns unrecognized fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// A service config grant with its syntax form retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigGrant {
    /// Config-name short syntax.
    Short(Located<String>),
    /// Mapping-based long syntax.
    Long(Box<LongGrant>),
}

/// A service secret grant with its syntax form retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretGrant {
    /// Secret-name short syntax.
    Short(Located<String>),
    /// Mapping-based long syntax.
    Long(Box<LongGrant>),
}
