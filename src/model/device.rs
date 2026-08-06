//! Raw-preserving service device declarations.

use crate::source::SourceSpan;

use super::{FieldReference, Located};

/// One scalar short-syntax service device declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortDevice {
    raw: Located<String>,
    kind: ShortDeviceKind,
}

impl ShortDevice {
    pub(crate) fn new(raw: Located<String>) -> Self {
        let kind = classify_short_device(raw.value());
        Self { raw, kind }
    }

    /// Returns the complete scalar without parsing or normalizing colon-delimited components.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns a conservative lexical family that makes no runtime-support claim.
    #[must_use]
    pub const fn kind(&self) -> ShortDeviceKind {
        self.kind
    }
}

/// A conservative lexical family for one short device declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShortDeviceKind {
    /// A dollar-bearing value whose effective spelling depends on interpolation.
    Deferred,
    /// A selector containing a non-empty CDI-like `vendor/device=name` split.
    Cdi,
    /// A slash-prefixed or colon-delimited path-like spelling.
    Path,
    /// Any other raw provider-dependent spelling.
    Opaque,
}

/// One mapping-form service device declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongDevice {
    span: SourceSpan,
    source: Option<Located<String>>,
    target: Option<Located<String>>,
    permissions: Option<Located<String>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl LongDevice {
    pub(crate) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            source: None,
            target: None,
            permissions: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(crate) fn set_source(&mut self, source: Located<String>) {
        self.source = Some(source);
    }

    pub(crate) fn set_target(&mut self, target: Located<String>) {
        self.target = Some(target);
    }

    pub(crate) fn set_permissions(&mut self, permissions: Located<String>) {
        self.permissions = Some(permissions);
    }

    pub(crate) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(crate) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the complete mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the required source when it was valid and present.
    #[must_use]
    pub const fn source(&self) -> Option<&Located<String>> {
        self.source.as_ref()
    }

    /// Returns the optional target without host/container path interpretation.
    #[must_use]
    pub const fn target(&self) -> Option<&Located<String>> {
        self.target.as_ref()
    }

    /// Returns the optional raw permissions string without validating its letters or meaning.
    #[must_use]
    pub const fn permissions(&self) -> Option<&Located<String>> {
        self.permissions.as_ref()
    }

    /// Returns retained `x-` options in authored order.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns unrecognized mapping options in authored order.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// One ordered service device item with its authored syntax form retained.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Device {
    /// A raw scalar short form.
    Short(ShortDevice),
    /// A mapping long form.
    Long(LongDevice),
}

impl Device {
    /// Returns the complete item span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Short(device) => device.raw().span(),
            Self::Long(device) => device.span(),
        }
    }
}

/// An explicitly authored ordered service `devices` sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Devices {
    span: SourceSpan,
    items: Vec<Device>,
}

impl Devices {
    pub(crate) const fn new(span: SourceSpan, items: Vec<Device>) -> Self {
        Self { span, items }
    }

    /// Returns the complete sequence span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns items in authored order, including exact duplicates.
    #[must_use]
    pub fn items(&self) -> &[Device] {
        &self.items
    }
}

fn classify_short_device(value: &str) -> ShortDeviceKind {
    if value.contains('$') {
        return ShortDeviceKind::Deferred;
    }
    if value
        .split_once('=')
        .is_some_and(|(selector, name)| !selector.is_empty() && !name.is_empty() && selector.contains('/'))
    {
        return ShortDeviceKind::Cdi;
    }
    if value.starts_with('/') || value.starts_with('.') || value.contains(':') || value.starts_with(r"\\") {
        return ShortDeviceKind::Path;
    }
    ShortDeviceKind::Opaque
}

pub(crate) fn valid_generated_device_string(value: &str, require_non_empty: bool) -> bool {
    (!require_non_empty || !value.is_empty()) && !value.contains(['\0', '\r', '\n', '$'])
}

#[cfg(test)]
mod tests {
    use super::{ShortDeviceKind, classify_short_device, valid_generated_device_string};

    #[test]
    fn classification_is_lexical_and_raw_preserving() {
        assert_eq!(classify_short_device("/dev/dri:/dev/dri:rwm"), ShortDeviceKind::Path);
        assert_eq!(classify_short_device("vendor.example/device=gpu"), ShortDeviceKind::Cdi);
        assert_eq!(classify_short_device("${DEVICE}"), ShortDeviceKind::Deferred);
        assert_eq!(classify_short_device("provider-token"), ShortDeviceKind::Opaque);
    }

    #[test]
    fn generated_device_strings_only_enforce_safe_resolved_output() {
        assert!(valid_generated_device_string("not-a-host-device", true));
        assert!(valid_generated_device_string("not-permissions", false));
        assert!(!valid_generated_device_string("", true));
        assert!(!valid_generated_device_string("${DEVICE}", true));
        assert!(!valid_generated_device_string("line\nbreak", false));
    }
}
