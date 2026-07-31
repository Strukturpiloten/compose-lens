//! Source identities and byte-accurate locations.

use std::fmt;
use std::ops::Range;

/// Identifies one source document within a caller-managed source collection.
///
/// `ComposeLens` deliberately does not infer paths or allocate global identifiers. A caller assigns
/// an identifier when it supplies source text and can maintain any path or URI mapping separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId(u32);

impl SourceId {
    /// Creates an identifier from a caller-managed numeric value.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the caller-managed numeric value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "source#{}", self.0)
    }
}

/// A half-open byte range associated with one source document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    source_id: SourceId,
    start: usize,
    end: usize,
}

impl SourceSpan {
    /// Creates a span when `start` is not after `end`.
    #[must_use]
    pub const fn new(source_id: SourceId, start: usize, end: usize) -> Option<Self> {
        if start <= end {
            Some(Self {
                source_id,
                start,
                end,
            })
        } else {
            None
        }
    }

    pub(crate) const fn from_valid_offsets(source_id: SourceId, start: usize, end: usize) -> Self {
        Self {
            source_id,
            start,
            end,
        }
    }

    /// Returns the source document identifier.
    #[must_use]
    pub const fn source_id(self) -> SourceId {
        self.source_id
    }

    /// Returns the inclusive start byte offset.
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    /// Returns the exclusive end byte offset.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }

    /// Returns the half-open byte range.
    #[must_use]
    pub fn range(self) -> Range<usize> {
        self.start..self.end
    }

    /// Returns the span length in bytes.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    /// Reports whether this is an empty span.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Reports whether the byte offset is inside the half-open span.
    #[must_use]
    pub const fn contains(self, byte_offset: usize) -> bool {
        self.start <= byte_offset && byte_offset < self.end
    }
}

/// A one-based line and Unicode-scalar column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineColumn {
    line: usize,
    column: usize,
}

impl LineColumn {
    /// Returns the one-based line number.
    #[must_use]
    pub const fn line(self) -> usize {
        self.line
    }

    /// Returns the one-based column number.
    #[must_use]
    pub const fn column(self) -> usize {
        self.column
    }
}

/// Converts a byte offset into a one-based line and Unicode-scalar column.
///
/// Returns `None` when the offset is outside the text or is not a UTF-8 character boundary.
#[must_use]
pub fn line_column(text: &str, byte_offset: usize) -> Option<LineColumn> {
    if byte_offset > text.len() || !text.is_char_boundary(byte_offset) {
        return None;
    }

    let prefix = &text[..byte_offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let column = text[line_start..byte_offset].chars().count() + 1;

    Some(LineColumn { line, column })
}

#[cfg(test)]
mod tests {
    use super::{SourceId, SourceSpan, line_column};

    #[test]
    fn rejects_a_reversed_span() {
        assert_eq!(SourceSpan::new(SourceId::new(1), 4, 3), None);
    }

    #[test]
    fn calculates_unicode_scalar_columns() {
        let text = "name: Käfer\nimage: demo\n";
        let position = line_column(text, "name: Kä".len());

        assert_eq!(position.map(super::LineColumn::line), Some(1));
        assert_eq!(position.map(super::LineColumn::column), Some(9));
    }
}
