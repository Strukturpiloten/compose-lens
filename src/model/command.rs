//! Service command forms.

use super::Located;
use crate::source::SourceSpan;

/// A Compose service command with null, scalar, and list forms retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Explicit null: use the command declared by the image.
    Null(SourceSpan),
    /// Scalar syntax, including an explicitly empty string.
    String(Located<String>),
    /// List syntax, including an explicitly empty list.
    List {
        /// The complete sequence span.
        span: SourceSpan,
        /// Command arguments in authored order.
        values: Vec<Located<String>>,
    },
}

impl Command {
    /// Returns the complete command value span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Null(span) | Self::List { span, .. } => *span,
            Self::String(value) => value.span(),
        }
    }
}
