//! Typed service-volume mounts that preserve their authored syntax form.

use super::{BooleanValue, FieldReference, Located};
use crate::source::SourceSpan;

/// The authored Compose syntax form of a service-volume mount.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VolumeSyntax {
    /// A colon-delimited scalar such as `./data:/var/lib/data:Z`.
    Short,
    /// A mapping with fields such as `type`, `source`, and `target`.
    Long,
}

/// A requested `SELinux` relabel mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelinuxRelabel {
    /// Shared relabeling, spelled `z`.
    Shared,
    /// Private relabeling, spelled `Z`.
    Private,
}

/// A long-syntax service-volume mount type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MountType {
    /// A named or anonymous container volume.
    Volume,
    /// A host bind mount.
    Bind,
    /// An in-memory temporary filesystem.
    Tmpfs,
    /// A Windows named pipe.
    NamedPipe,
    /// An image-backed mount.
    Image,
    /// A cluster-managed mount.
    Cluster,
    /// A value not recognized by this `ComposeLens` release.
    Other(String),
}

impl MountType {
    pub(super) fn from_text(value: String) -> Self {
        match value.as_str() {
            "volume" => Self::Volume,
            "bind" => Self::Bind,
            "tmpfs" => Self::Tmpfs,
            "npipe" => Self::NamedPipe,
            "image" => Self::Image,
            "cluster" => Self::Cluster,
            _ => Self::Other(value),
        }
    }
}

/// A short-syntax service-volume mount.
///
/// `source`, `target`, and `options` are a conservative decomposition for common Compose
/// strings. [`Self::raw`] remains authoritative because platform-specific path grammars and
/// implementation extensions can make a short string ambiguous.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortVolumeMount {
    raw: Located<String>,
    source: Option<String>,
    target: Option<String>,
    options: Vec<String>,
}

impl ShortVolumeMount {
    pub(crate) fn new(raw: Located<String>) -> Self {
        let (source, target, options) = split_short_volume(raw.value());
        Self {
            raw,
            source,
            target,
            options,
        }
    }

    /// Returns the unquoted semantic scalar and its source span.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns the conservatively parsed source, if one was present.
    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Returns the conservatively parsed container target.
    #[must_use]
    pub fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }

    /// Returns access-mode tokens in authored order.
    #[must_use]
    pub fn options(&self) -> &[String] {
        &self.options
    }

    /// Returns the requested `SELinux` relabel mode, if present.
    #[must_use]
    pub fn selinux_relabel(&self) -> Option<SelinuxRelabel> {
        self.options.iter().find_map(|option| match option.as_str() {
            "z" => Some(SelinuxRelabel::Shared),
            "Z" => Some(SelinuxRelabel::Private),
            _ => None,
        })
    }
}

/// Bind-specific options in a long-syntax mount.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindOptions {
    span: SourceSpan,
    propagation: Option<Located<String>>,
    create_host_path: Option<Located<BooleanValue>>,
    selinux: Option<Located<SelinuxRelabel>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl BindOptions {
    pub(super) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            propagation: None,
            create_host_path: None,
            selinux: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_propagation(&mut self, value: Located<String>) {
        self.propagation = Some(value);
    }

    pub(super) fn set_create_host_path(&mut self, value: Located<BooleanValue>) {
        self.create_host_path = Some(value);
    }

    pub(super) fn set_selinux(&mut self, value: Located<SelinuxRelabel>) {
        self.selinux = Some(value);
    }

    pub(super) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(super) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the complete `bind` mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the requested bind-propagation mode.
    #[must_use]
    pub const fn propagation(&self) -> Option<&Located<String>> {
        self.propagation.as_ref()
    }

    /// Returns the explicitly authored host-path creation setting.
    ///
    /// `None` means the field was omitted; it is deliberately not replaced by an
    /// implementation default during typed parsing.
    #[must_use]
    pub const fn create_host_path(&self) -> Option<&Located<BooleanValue>> {
        self.create_host_path.as_ref()
    }

    /// Returns the requested `SELinux` relabel mode.
    #[must_use]
    pub const fn selinux(&self) -> Option<&Located<SelinuxRelabel>> {
        self.selinux.as_ref()
    }

    /// Returns `x-` extension fields retained from the bind mapping.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns unrecognized bind fields retained from the source.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// A long-syntax service-volume mount.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongVolumeMount {
    span: SourceSpan,
    mount_type: Option<Located<MountType>>,
    source: Option<Located<String>>,
    target: Option<Located<String>>,
    read_only: Option<Located<BooleanValue>>,
    bind: Option<BindOptions>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl LongVolumeMount {
    pub(super) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            mount_type: None,
            source: None,
            target: None,
            read_only: None,
            bind: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_mount_type(&mut self, value: Located<MountType>) {
        self.mount_type = Some(value);
    }

    pub(super) fn set_source(&mut self, value: Located<String>) {
        self.source = Some(value);
    }

    pub(super) fn set_target(&mut self, value: Located<String>) {
        self.target = Some(value);
    }

    pub(super) fn set_read_only(&mut self, value: Located<BooleanValue>) {
        self.read_only = Some(value);
    }

    pub(super) fn set_bind(&mut self, value: BindOptions) {
        self.bind = Some(value);
    }

    pub(super) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(super) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the complete mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the explicitly authored mount type.
    #[must_use]
    pub const fn mount_type(&self) -> Option<&Located<MountType>> {
        self.mount_type.as_ref()
    }

    /// Returns the mount source.
    #[must_use]
    pub const fn source(&self) -> Option<&Located<String>> {
        self.source.as_ref()
    }

    /// Returns the container target.
    #[must_use]
    pub const fn target(&self) -> Option<&Located<String>> {
        self.target.as_ref()
    }

    /// Returns the explicitly authored read-only setting.
    #[must_use]
    pub const fn read_only(&self) -> Option<&Located<BooleanValue>> {
        self.read_only.as_ref()
    }

    /// Returns long-syntax bind options.
    #[must_use]
    pub const fn bind(&self) -> Option<&BindOptions> {
        self.bind.as_ref()
    }

    /// Returns `x-` extension fields retained from the mount mapping.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns recognized-by-Compose but not-yet-typed and unrecognized fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// A service-volume mount with its authored syntax form retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VolumeMount {
    /// Colon-delimited short syntax.
    Short(ShortVolumeMount),
    /// Mapping-based long syntax.
    Long(Box<LongVolumeMount>),
}

impl VolumeMount {
    /// Returns the authored syntax form.
    #[must_use]
    pub const fn syntax(&self) -> VolumeSyntax {
        match self {
            Self::Short(_) => VolumeSyntax::Short,
            Self::Long(_) => VolumeSyntax::Long,
        }
    }

    /// Returns the complete source span of the mount value.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Short(value) => value.raw().span(),
            Self::Long(value) => value.span(),
        }
    }

    /// Returns the requested `SELinux` relabel mode without erasing syntax provenance.
    #[must_use]
    pub fn selinux_relabel(&self) -> Option<SelinuxRelabel> {
        match self {
            Self::Short(value) => value.selinux_relabel(),
            Self::Long(value) => value.bind()?.selinux().map(|mode| *mode.value()),
        }
    }
}

fn split_short_volume(value: &str) -> (Option<String>, Option<String>, Vec<String>) {
    let fields = split_colon_fields(value);
    match fields.as_slice() {
        [] => (None, None, Vec::new()),
        [target] => (None, Some((*target).to_owned()), Vec::new()),
        [source, target] => (Some((*source).to_owned()), Some((*target).to_owned()), Vec::new()),
        [source, middle @ .., options] => (
            Some((*source).to_owned()),
            Some(middle.join(":")),
            options.split(',').map(str::to_owned).collect(),
        ),
    }
}

fn split_colon_fields(value: &str) -> Vec<&str> {
    let mut fields = Vec::new();
    let mut start = 0;
    for (index, character) in value.char_indices() {
        if character != ':' || is_drive_separator(value, start, index) {
            continue;
        }
        fields.push(&value[start..index]);
        start = index + character.len_utf8();
    }
    fields.push(&value[start..]);
    fields
}

fn is_drive_separator(value: &str, field_start: usize, colon_index: usize) -> bool {
    let field = &value[field_start..colon_index];
    let next = value[colon_index + 1..].chars().next();
    field.len() == 1 && field.as_bytes()[0].is_ascii_alphabetic() && matches!(next, Some('/' | '\\'))
}

#[cfg(test)]
mod tests {
    use super::split_short_volume;

    #[test]
    fn conservatively_splits_linux_and_windows_short_mounts() {
        assert_eq!(
            split_short_volume("./data:/var/lib/data:Z,ro"),
            (
                Some("./data".to_owned()),
                Some("/var/lib/data".to_owned()),
                vec!["Z".to_owned(), "ro".to_owned()]
            )
        );
        assert_eq!(
            split_short_volume(r"C:\data:/var/lib/data:z"),
            (
                Some(r"C:\data".to_owned()),
                Some("/var/lib/data".to_owned()),
                vec!["z".to_owned()]
            )
        );
        assert_eq!(
            split_short_volume("cache:/cache"),
            (Some("cache".to_owned()), Some("/cache".to_owned()), Vec::new())
        );
    }
}
