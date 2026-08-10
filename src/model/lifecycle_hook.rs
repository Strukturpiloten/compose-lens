//! Source-aware service lifecycle-hook descriptors.

use crate::source::SourceSpan;

use super::{BooleanValue, Command, Environment, FieldReference, Located};

/// One authored service lifecycle-hook mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceHook {
    span: SourceSpan,
    command: Option<Command>,
    environment: Option<Environment>,
    privileged: Option<Located<BooleanValue>>,
    user: Option<Located<String>>,
    working_dir: Option<Located<String>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl ServiceHook {
    pub(crate) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            command: None,
            environment: None,
            privileged: None,
            user: None,
            working_dir: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(crate) fn set_command(&mut self, value: Command) {
        self.command = Some(value);
    }

    pub(crate) fn set_environment(&mut self, value: Environment) {
        self.environment = Some(value);
    }

    pub(crate) fn set_privileged(&mut self, value: Located<BooleanValue>) {
        self.privileged = Some(value);
    }

    pub(crate) fn set_user(&mut self, value: Located<String>) {
        self.user = Some(value);
    }

    pub(crate) fn set_working_dir(&mut self, value: Located<String>) {
        self.working_dir = Some(value);
    }

    pub(crate) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(crate) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the complete hook mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the required command without shell or argv interpretation.
    #[must_use]
    pub const fn command(&self) -> Option<&Command> {
        self.command.as_ref()
    }

    /// Returns the hook-local environment without combining it with service environment values.
    #[must_use]
    pub const fn environment(&self) -> Option<&Environment> {
        self.environment.as_ref()
    }

    /// Returns the explicit hook privilege choice.
    #[must_use]
    pub const fn privileged(&self) -> Option<&Located<BooleanValue>> {
        self.privileged.as_ref()
    }

    /// Returns the strict YAML-string hook user without default resolution.
    #[must_use]
    pub const fn user(&self) -> Option<&Located<String>> {
        self.user.as_ref()
    }

    /// Returns the strict YAML-string hook working directory without path resolution.
    #[must_use]
    pub const fn working_dir(&self) -> Option<&Located<String>> {
        self.working_dir.as_ref()
    }

    /// Returns retained hook `x-*` members.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns malformed, duplicate, and unknown hook members.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// One authored service `pre_start` hook mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreStartServiceHook {
    span: SourceSpan,
    command: Option<Command>,
    image: Option<Located<String>>,
    environment: Option<Environment>,
    privileged: Option<Located<BooleanValue>>,
    per_replica: Option<Located<BooleanValue>>,
    user: Option<Located<String>>,
    working_dir: Option<Located<String>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl PreStartServiceHook {
    pub(crate) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            command: None,
            image: None,
            environment: None,
            privileged: None,
            per_replica: None,
            user: None,
            working_dir: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(crate) fn set_command(&mut self, value: Command) {
        self.command = Some(value);
    }

    pub(crate) fn set_image(&mut self, value: Located<String>) {
        self.image = Some(value);
    }

    pub(crate) fn set_environment(&mut self, value: Environment) {
        self.environment = Some(value);
    }

    pub(crate) fn set_privileged(&mut self, value: Located<BooleanValue>) {
        self.privileged = Some(value);
    }

    pub(crate) fn set_per_replica(&mut self, value: Located<BooleanValue>) {
        self.per_replica = Some(value);
    }

    pub(crate) fn set_user(&mut self, value: Located<String>) {
        self.user = Some(value);
    }

    pub(crate) fn set_working_dir(&mut self, value: Located<String>) {
        self.working_dir = Some(value);
    }

    pub(crate) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(crate) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the complete hook mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the optional command without shell or argv interpretation.
    #[must_use]
    pub const fn command(&self) -> Option<&Command> {
        self.command.as_ref()
    }

    /// Returns the strict raw hook image without image-reference interpretation.
    #[must_use]
    pub const fn image(&self) -> Option<&Located<String>> {
        self.image.as_ref()
    }

    /// Returns the hook-local environment without combining it with service environment values.
    #[must_use]
    pub const fn environment(&self) -> Option<&Environment> {
        self.environment.as_ref()
    }

    /// Returns the explicit hook privilege choice.
    #[must_use]
    pub const fn privileged(&self) -> Option<&Located<BooleanValue>> {
        self.privileged.as_ref()
    }

    /// Returns the explicit per-replica choice without injecting a default.
    #[must_use]
    pub const fn per_replica(&self) -> Option<&Located<BooleanValue>> {
        self.per_replica.as_ref()
    }

    /// Returns the strict YAML-string hook user without default resolution.
    #[must_use]
    pub const fn user(&self) -> Option<&Located<String>> {
        self.user.as_ref()
    }

    /// Returns the strict YAML-string hook working directory without path resolution.
    #[must_use]
    pub const fn working_dir(&self) -> Option<&Located<String>> {
        self.working_dir.as_ref()
    }

    /// Returns retained hook `x-*` members.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns malformed, duplicate, and unknown hook members.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// One authored `post_start` sequence item.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PostStartHook {
    /// A valid or partially recovered hook mapping.
    Hook(Box<ServiceHook>),
    /// A non-mapping item retained by source span.
    Unmodeled {
        /// Complete malformed-item span.
        span: SourceSpan,
    },
}

impl PostStartHook {
    /// Returns the complete sequence-item span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Hook(hook) => hook.span(),
            Self::Unmodeled { span } => *span,
        }
    }
}

/// One authored `pre_stop` sequence item.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PreStopHook {
    /// A valid or partially recovered hook mapping.
    Hook(Box<ServiceHook>),
    /// A non-mapping item retained by source span.
    Unmodeled {
        /// Complete malformed-item span.
        span: SourceSpan,
    },
}

impl PreStopHook {
    /// Returns the complete sequence-item span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Hook(hook) => hook.span(),
            Self::Unmodeled { span } => *span,
        }
    }
}

/// One authored `pre_start` sequence item.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PreStartHook {
    /// A valid or partially recovered hook mapping.
    Hook(Box<PreStartServiceHook>),
    /// A non-mapping item retained by source span.
    Unmodeled {
        /// Complete malformed-item span.
        span: SourceSpan,
    },
}

impl PreStartHook {
    /// Returns the complete sequence-item span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Hook(hook) => hook.span(),
            Self::Unmodeled { span } => *span,
        }
    }
}

/// An authored `post_start` hook sequence, including an explicit empty sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostStartHooks {
    span: SourceSpan,
    entries: Vec<PostStartHook>,
}

impl PostStartHooks {
    pub(crate) const fn new(span: SourceSpan, entries: Vec<PostStartHook>) -> Self {
        Self { span, entries }
    }

    /// Returns the complete hook sequence span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns all hook and malformed entries in physical authored order.
    #[must_use]
    pub fn entries(&self) -> &[PostStartHook] {
        &self.entries
    }
}

/// An authored `pre_stop` hook sequence, including an explicit empty sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreStopHooks {
    span: SourceSpan,
    entries: Vec<PreStopHook>,
}

impl PreStopHooks {
    pub(crate) const fn new(span: SourceSpan, entries: Vec<PreStopHook>) -> Self {
        Self { span, entries }
    }

    /// Returns the complete hook sequence span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns all hook and malformed entries in physical authored order.
    #[must_use]
    pub fn entries(&self) -> &[PreStopHook] {
        &self.entries
    }
}

/// An authored `pre_start` hook sequence, including an explicit empty sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreStartHooks {
    span: SourceSpan,
    entries: Vec<PreStartHook>,
}

impl PreStartHooks {
    pub(crate) const fn new(span: SourceSpan, entries: Vec<PreStartHook>) -> Self {
        Self { span, entries }
    }

    /// Returns the complete hook sequence span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns all hook and malformed entries in physical authored order.
    #[must_use]
    pub fn entries(&self) -> &[PreStartHook] {
        &self.entries
    }
}
