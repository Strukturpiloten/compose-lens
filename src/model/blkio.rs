use super::{FieldReference, Located};
use crate::source::SourceSpan;

/// Authored service block-I/O configuration without controller or runtime interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlkioConfig {
    span: SourceSpan,
    weight: Option<Located<BlkioScalar>>,
    device_read_bps: Vec<BlkioDeviceRate>,
    device_read_iops: Vec<BlkioDeviceRate>,
    device_write_bps: Vec<BlkioDeviceRate>,
    device_write_iops: Vec<BlkioDeviceRate>,
    weight_device: Vec<BlkioWeightDevice>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl BlkioConfig {
    pub(super) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            weight: None,
            device_read_bps: Vec::new(),
            device_read_iops: Vec::new(),
            device_write_bps: Vec::new(),
            device_write_iops: Vec::new(),
            weight_device: Vec::new(),
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_weight(&mut self, value: Located<BlkioScalar>) {
        self.weight = Some(value);
    }

    pub(super) fn device_rates_mut(&mut self, name: &str) -> Option<&mut Vec<BlkioDeviceRate>> {
        match name {
            "device_read_bps" => Some(&mut self.device_read_bps),
            "device_read_iops" => Some(&mut self.device_read_iops),
            "device_write_bps" => Some(&mut self.device_write_bps),
            "device_write_iops" => Some(&mut self.device_write_iops),
            _ => None,
        }
    }

    pub(super) fn weight_devices_mut(&mut self) -> &mut Vec<BlkioWeightDevice> {
        &mut self.weight_device
    }

    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }

    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the optional raw overall weight.
    #[must_use]
    pub const fn weight(&self) -> Option<&Located<BlkioScalar>> {
        self.weight.as_ref()
    }

    /// Returns ordered read-byte-rate items.
    #[must_use]
    pub fn device_read_bps(&self) -> &[BlkioDeviceRate] {
        &self.device_read_bps
    }

    /// Returns ordered read-I/O-rate items.
    #[must_use]
    pub fn device_read_iops(&self) -> &[BlkioDeviceRate] {
        &self.device_read_iops
    }

    /// Returns ordered write-byte-rate items.
    #[must_use]
    pub fn device_write_bps(&self) -> &[BlkioDeviceRate] {
        &self.device_write_bps
    }

    /// Returns ordered write-I/O-rate items.
    #[must_use]
    pub fn device_write_iops(&self) -> &[BlkioDeviceRate] {
        &self.device_write_iops
    }

    /// Returns ordered device-weight items.
    #[must_use]
    pub fn weight_device(&self) -> &[BlkioWeightDevice] {
        &self.weight_device
    }

    /// Returns retained extensions.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns retained unknown or malformed members.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// Exact YAML integer-or-string spelling for a block-I/O weight or rate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlkioScalar {
    /// An exact YAML integer scalar spelling.
    YamlInteger(String),
    /// An exact YAML string scalar spelling.
    String(String),
}

/// One ordered block-I/O device rate item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlkioDeviceRate {
    span: SourceSpan,
    form: BlkioDeviceRateForm,
    path: Option<Located<String>>,
    rate: Option<Located<BlkioScalar>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl BlkioDeviceRate {
    pub(super) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            form: BlkioDeviceRateForm::Mapping,
            path: None,
            rate: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn unmodeled(span: SourceSpan) -> Self {
        Self {
            span,
            form: BlkioDeviceRateForm::Unmodeled,
            path: None,
            rate: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_path(&mut self, value: Located<String>) {
        self.path = Some(value);
    }

    pub(super) fn set_rate(&mut self, value: Located<BlkioScalar>) {
        self.rate = Some(value);
    }

    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }

    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete item span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns whether this entry was a mapping or retained unmodeled sequence item.
    #[must_use]
    pub const fn form(&self) -> BlkioDeviceRateForm {
        self.form
    }

    /// Returns the raw device path when its YAML-string form was valid.
    #[must_use]
    pub const fn path(&self) -> Option<&Located<String>> {
        self.path.as_ref()
    }

    /// Returns the raw integer-or-string rate when its scalar form was valid.
    #[must_use]
    pub const fn rate(&self) -> Option<&Located<BlkioScalar>> {
        self.rate.as_ref()
    }

    /// Returns retained extensions on this entry.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns retained unknown or malformed members on this entry.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// Authored block-I/O device-rate entry shape retained without coercion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlkioDeviceRateForm {
    /// A mapping-form entry.
    Mapping,
    /// A non-mapping sequence entry retained as evidence.
    Unmodeled,
}

/// One ordered block-I/O device weight item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlkioWeightDevice {
    span: SourceSpan,
    form: BlkioWeightDeviceForm,
    path: Option<Located<String>>,
    weight: Option<Located<BlkioScalar>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl BlkioWeightDevice {
    pub(super) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            form: BlkioWeightDeviceForm::Mapping,
            path: None,
            weight: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn unmodeled(span: SourceSpan) -> Self {
        Self {
            span,
            form: BlkioWeightDeviceForm::Unmodeled,
            path: None,
            weight: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_path(&mut self, value: Located<String>) {
        self.path = Some(value);
    }

    pub(super) fn set_weight(&mut self, value: Located<BlkioScalar>) {
        self.weight = Some(value);
    }

    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }

    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete item span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns whether this entry was a mapping or retained unmodeled sequence item.
    #[must_use]
    pub const fn form(&self) -> BlkioWeightDeviceForm {
        self.form
    }

    /// Returns the raw device path when its YAML-string form was valid.
    #[must_use]
    pub const fn path(&self) -> Option<&Located<String>> {
        self.path.as_ref()
    }

    /// Returns the raw integer-or-string weight when its scalar form was valid.
    #[must_use]
    pub const fn weight(&self) -> Option<&Located<BlkioScalar>> {
        self.weight.as_ref()
    }

    /// Returns retained extensions on this entry.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns retained unknown or malformed members on this entry.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// Authored block-I/O device-weight entry shape retained without coercion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlkioWeightDeviceForm {
    /// A mapping-form entry.
    Mapping,
    /// A non-mapping sequence entry retained as evidence.
    Unmodeled,
}
