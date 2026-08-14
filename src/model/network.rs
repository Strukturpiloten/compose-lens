//! Service-network attachments and top-level network definitions.

use super::{BooleanValue, FieldReference, KeyValueEntry, Labels, Located, ResourceExternal};
use crate::source::SourceSpan;

/// Options for one service attachment to a network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceNetwork {
    name: Located<String>,
    span: SourceSpan,
    aliases: Vec<Located<String>>,
    interface_name: Option<Located<String>>,
    ipv4_address: Option<Located<String>>,
    ipv6_address: Option<Located<String>>,
    link_local_ips: Vec<Located<String>>,
    mac_address: Option<Located<String>>,
    driver_opts: Vec<KeyValueEntry>,
    gw_priority: Option<Located<String>>,
    priority: Option<Located<String>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl ServiceNetwork {
    pub(crate) fn new(name: Located<String>, span: SourceSpan) -> Self {
        Self {
            name,
            span,
            aliases: Vec::new(),
            interface_name: None,
            ipv4_address: None,
            ipv6_address: None,
            link_local_ips: Vec::new(),
            mac_address: None,
            driver_opts: Vec::new(),
            gw_priority: None,
            priority: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(crate) fn set_aliases(&mut self, value: Vec<Located<String>>) {
        self.aliases = value;
    }
    pub(crate) fn set_interface_name(&mut self, value: Located<String>) {
        self.interface_name = Some(value);
    }
    pub(crate) fn set_ipv4_address(&mut self, value: Located<String>) {
        self.ipv4_address = Some(value);
    }
    pub(crate) fn set_ipv6_address(&mut self, value: Located<String>) {
        self.ipv6_address = Some(value);
    }
    pub(crate) fn set_link_local_ips(&mut self, value: Vec<Located<String>>) {
        self.link_local_ips = value;
    }
    pub(crate) fn set_mac_address(&mut self, value: Located<String>) {
        self.mac_address = Some(value);
    }
    pub(crate) fn set_driver_opts(&mut self, value: Vec<KeyValueEntry>) {
        self.driver_opts = value;
    }
    pub(crate) fn set_gw_priority(&mut self, value: Located<String>) {
        self.gw_priority = Some(value);
    }
    pub(crate) fn set_priority(&mut self, value: Located<String>) {
        self.priority = Some(value);
    }
    pub(crate) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }
    pub(crate) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the referenced top-level network name.
    #[must_use]
    pub const fn name(&self) -> &Located<String> {
        &self.name
    }
    /// Returns the complete attachment span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns network-scoped aliases.
    #[must_use]
    pub fn aliases(&self) -> &[Located<String>] {
        &self.aliases
    }
    /// Returns the requested interface name.
    #[must_use]
    pub const fn interface_name(&self) -> Option<&Located<String>> {
        self.interface_name.as_ref()
    }
    /// Returns the requested IPv4 address.
    #[must_use]
    pub const fn ipv4_address(&self) -> Option<&Located<String>> {
        self.ipv4_address.as_ref()
    }
    /// Returns the requested IPv6 address.
    #[must_use]
    pub const fn ipv6_address(&self) -> Option<&Located<String>> {
        self.ipv6_address.as_ref()
    }
    /// Returns link-local addresses.
    #[must_use]
    pub fn link_local_ips(&self) -> &[Located<String>] {
        &self.link_local_ips
    }
    /// Returns the network-scoped MAC address.
    #[must_use]
    pub const fn mac_address(&self) -> Option<&Located<String>> {
        self.mac_address.as_ref()
    }
    /// Returns driver options in authored order.
    #[must_use]
    pub fn driver_opts(&self) -> &[KeyValueEntry] {
        &self.driver_opts
    }
    /// Returns the default-gateway priority without applying interpolation or defaults.
    #[must_use]
    pub const fn gw_priority(&self) -> Option<&Located<String>> {
        self.gw_priority.as_ref()
    }
    /// Returns connection priority without applying interpolation or defaults.
    #[must_use]
    pub const fn priority(&self) -> Option<&Located<String>> {
        self.priority.as_ref()
    }
    /// Returns retained `x-` fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }
    /// Returns unrecognized attachment fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// Service network syntax, retained as a sequence or mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceNetworks {
    /// Sequence syntax containing network names only.
    Short {
        /// The complete sequence span.
        span: SourceSpan,
        /// Network names in authored order.
        names: Vec<Located<String>>,
    },
    /// Mapping syntax containing per-network options.
    Long {
        /// The complete mapping span.
        span: SourceSpan,
        /// Attachments in authored order.
        networks: Vec<ServiceNetwork>,
    },
}

impl ServiceNetworks {
    /// Returns the complete service-networks span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Short { span, .. } | Self::Long { span, .. } => *span,
        }
    }
}

/// One top-level IPAM subnet configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpamConfig {
    span: SourceSpan,
    subnet: Option<Located<String>>,
    ip_range: Option<Located<String>>,
    gateway: Option<Located<String>>,
    aux_addresses: Vec<KeyValueEntry>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl IpamConfig {
    pub(crate) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            subnet: None,
            ip_range: None,
            gateway: None,
            aux_addresses: Vec::new(),
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }
    pub(crate) fn set_subnet(&mut self, value: Located<String>) {
        self.subnet = Some(value);
    }
    pub(crate) fn set_ip_range(&mut self, value: Located<String>) {
        self.ip_range = Some(value);
    }
    pub(crate) fn set_gateway(&mut self, value: Located<String>) {
        self.gateway = Some(value);
    }
    pub(crate) fn set_aux_addresses(&mut self, value: Vec<KeyValueEntry>) {
        self.aux_addresses = value;
    }
    pub(crate) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }
    pub(crate) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete configuration span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns the subnet.
    #[must_use]
    pub const fn subnet(&self) -> Option<&Located<String>> {
        self.subnet.as_ref()
    }
    /// Returns the allocation range.
    #[must_use]
    pub const fn ip_range(&self) -> Option<&Located<String>> {
        self.ip_range.as_ref()
    }
    /// Returns the gateway.
    #[must_use]
    pub const fn gateway(&self) -> Option<&Located<String>> {
        self.gateway.as_ref()
    }
    /// Returns auxiliary addresses.
    #[must_use]
    pub fn aux_addresses(&self) -> &[KeyValueEntry] {
        &self.aux_addresses
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

/// Top-level network IPAM configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipam {
    span: SourceSpan,
    driver: Option<Located<String>>,
    config: Vec<IpamConfig>,
    options: Vec<KeyValueEntry>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl Ipam {
    pub(crate) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            driver: None,
            config: Vec::new(),
            options: Vec::new(),
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }
    pub(crate) fn set_driver(&mut self, value: Located<String>) {
        self.driver = Some(value);
    }
    pub(crate) fn set_config(&mut self, value: Vec<IpamConfig>) {
        self.config = value;
    }
    pub(crate) fn set_options(&mut self, value: Vec<KeyValueEntry>) {
        self.options = value;
    }
    pub(crate) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }
    pub(crate) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete IPAM mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns the IPAM driver.
    #[must_use]
    pub const fn driver(&self) -> Option<&Located<String>> {
        self.driver.as_ref()
    }
    /// Returns subnet configurations.
    #[must_use]
    pub fn config(&self) -> &[IpamConfig] {
        &self.config
    }
    /// Returns driver-specific options.
    #[must_use]
    pub fn options(&self) -> &[KeyValueEntry] {
        &self.options
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

/// A typed top-level Compose network definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDefinition {
    name: Located<String>,
    span: SourceSpan,
    driver: Option<Located<String>>,
    driver_opts: Vec<KeyValueEntry>,
    attachable: Option<Located<BooleanValue>>,
    enable_ipv4: Option<Located<BooleanValue>>,
    enable_ipv6: Option<Located<BooleanValue>>,
    external: Option<ResourceExternal>,
    internal: Option<Located<BooleanValue>>,
    ipam: Option<Ipam>,
    labels: Option<Labels>,
    custom_name: Option<Located<String>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl NetworkDefinition {
    pub(crate) fn new(name: Located<String>, span: SourceSpan) -> Self {
        Self {
            name,
            span,
            driver: None,
            driver_opts: Vec::new(),
            attachable: None,
            enable_ipv4: None,
            enable_ipv6: None,
            external: None,
            internal: None,
            ipam: None,
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
    pub(crate) fn set_attachable(&mut self, value: Located<BooleanValue>) {
        self.attachable = Some(value);
    }
    pub(crate) fn set_enable_ipv4(&mut self, value: Located<BooleanValue>) {
        self.enable_ipv4 = Some(value);
    }
    pub(crate) fn set_enable_ipv6(&mut self, value: Located<BooleanValue>) {
        self.enable_ipv6 = Some(value);
    }
    pub(crate) fn set_external(&mut self, value: ResourceExternal) {
        self.external = Some(value);
    }
    pub(crate) fn set_internal(&mut self, value: Located<BooleanValue>) {
        self.internal = Some(value);
    }
    pub(crate) fn set_ipam(&mut self, value: Ipam) {
        self.ipam = Some(value);
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

    /// Returns the model identifier used by services.
    #[must_use]
    pub const fn name(&self) -> &Located<String> {
        &self.name
    }
    /// Returns the complete definition span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns the network driver.
    #[must_use]
    pub const fn driver(&self) -> Option<&Located<String>> {
        self.driver.as_ref()
    }
    /// Returns driver options.
    #[must_use]
    pub fn driver_opts(&self) -> &[KeyValueEntry] {
        &self.driver_opts
    }
    /// Returns the attachable setting.
    #[must_use]
    pub const fn attachable(&self) -> Option<&Located<BooleanValue>> {
        self.attachable.as_ref()
    }
    /// Returns the IPv4 setting.
    #[must_use]
    pub const fn enable_ipv4(&self) -> Option<&Located<BooleanValue>> {
        self.enable_ipv4.as_ref()
    }
    /// Returns the IPv6 setting.
    #[must_use]
    pub const fn enable_ipv6(&self) -> Option<&Located<BooleanValue>> {
        self.enable_ipv6.as_ref()
    }
    /// Returns the complete authored external-lifecycle setting.
    #[must_use]
    pub const fn external(&self) -> Option<&ResourceExternal> {
        self.external.as_ref()
    }
    /// Returns the internal-network setting.
    #[must_use]
    pub const fn internal(&self) -> Option<&Located<BooleanValue>> {
        self.internal.as_ref()
    }
    /// Returns IPAM configuration.
    #[must_use]
    pub const fn ipam(&self) -> Option<&Ipam> {
        self.ipam.as_ref()
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
