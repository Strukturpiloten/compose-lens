//! Raw-preserving service `sysctls` declarations.

use crate::source::SourceSpan;

use super::{KeyValueEntry, Located};

/// The exact authored mapping or list form of service `sysctls`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SysctlsForm {
    /// Ordered mapping syntax with exact scalar value kinds and spelling retained.
    Map(Vec<KeyValueEntry>),
    /// Ordered list syntax with exact string items retained.
    List(Vec<Located<String>>),
}

/// An explicitly authored service-level Compose `sysctls` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sysctls {
    span: SourceSpan,
    form: SysctlsForm,
}

impl Sysctls {
    pub(crate) const fn new(span: SourceSpan, form: SysctlsForm) -> Self {
        Self { span, form }
    }

    /// Returns the exact span of the complete authored collection.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the authored mapping or list form without coercion or normalization.
    #[must_use]
    pub const fn form(&self) -> &SysctlsForm {
        &self.form
    }
}
