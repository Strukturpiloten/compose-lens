//! Source-aware native Compose document types.

mod command;
mod environment;
mod image;
mod network;
mod port;
mod resource;
mod value;
mod volume;

pub use command::Command;
pub use environment::{Environment, EnvironmentListEntry, EnvironmentMapEntry};
pub use image::{ImageDigest, ImageReference};
pub use network::{Ipam, IpamConfig, NetworkDefinition, ServiceNetwork, ServiceNetworks};
pub use port::{LongPort, Port, ShortPort};
pub use resource::{ConfigDefinition, ConfigGrant, LongGrant, SecretDefinition, SecretGrant, VolumeDefinition};
pub use value::{BooleanValue, ComposeScalar, KeyValueEntry, Labels};
pub use volume::{
    BindOptions, LongVolumeMount, MountType, SelinuxRelabel, ShortVolumeMount, VolumeMount, VolumeSyntax,
};

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::source::{SourceId, SourceSpan};
use crate::syntax::SyntaxDocument;
use std::collections::BTreeMap;
use yaml_edit::{AsYaml, Mapping, ScalarType, ScalarValue, YamlNode};

/// A Compose document root must be a mapping.
pub const DOCUMENT_ROOT_TYPE: DiagnosticCode = DiagnosticCode::new("compose.document.expected-mapping");

/// `ComposeLens` currently types the first document in a multi-document YAML stream.
pub const MULTIPLE_DOCUMENTS: DiagnosticCode = DiagnosticCode::new("compose.document.multiple-documents");

/// A mapping contains a duplicate field.
pub const DUPLICATE_FIELD: DiagnosticCode = DiagnosticCode::new("compose.model.duplicate-field");

/// A Compose value has to be a mapping at this location.
pub const EXPECTED_MAPPING: DiagnosticCode = DiagnosticCode::new("compose.model.expected-mapping");

/// A Compose value has to be a sequence at this location.
pub const EXPECTED_SEQUENCE: DiagnosticCode = DiagnosticCode::new("compose.model.expected-sequence");

/// A Compose value has to be a scalar at this location.
pub const EXPECTED_SCALAR: DiagnosticCode = DiagnosticCode::new("compose.model.expected-scalar");

/// A Compose value has to be a boolean at this location.
pub const EXPECTED_BOOLEAN: DiagnosticCode = DiagnosticCode::new("compose.model.expected-boolean");

/// A field supports multiple Compose syntax forms, but the authored form is invalid here.
pub const EXPECTED_FIELD_FORM: DiagnosticCode = DiagnosticCode::new("compose.model.expected-field-form");

/// A service port is neither scalar short syntax nor mapping long syntax.
pub const PORT_EXPECTED_FORM: DiagnosticCode = DiagnosticCode::new("compose.port.expected-short-or-long");

/// A long-syntax service port is missing `target`.
pub const PORT_MISSING_TARGET: DiagnosticCode = DiagnosticCode::new("compose.port.long.missing-target");

/// A service config or secret grant is neither scalar short syntax nor mapping long syntax.
pub const GRANT_EXPECTED_FORM: DiagnosticCode = DiagnosticCode::new("compose.grant.expected-short-or-long");

/// A long-syntax service config or secret grant is missing `source`.
pub const GRANT_MISSING_SOURCE: DiagnosticCode = DiagnosticCode::new("compose.grant.long.missing-source");

/// A top-level resource definition must be a mapping or an explicit null.
pub const RESOURCE_EXPECTED_FORM: DiagnosticCode = DiagnosticCode::new("compose.resource.expected-mapping-or-null");

/// A service-volume item is neither short nor long syntax.
pub const VOLUME_EXPECTED_FORM: DiagnosticCode = DiagnosticCode::new("compose.volume.expected-short-or-long");

/// A long-syntax service volume is missing `type`.
pub const VOLUME_MISSING_TYPE: DiagnosticCode = DiagnosticCode::new("compose.volume.long.missing-type");

/// A long-syntax service volume is missing `target`.
pub const VOLUME_MISSING_TARGET: DiagnosticCode = DiagnosticCode::new("compose.volume.long.missing-target");

/// A long-syntax bind mount has an invalid `SELinux` value.
pub const VOLUME_INVALID_SELINUX: DiagnosticCode = DiagnosticCode::new("compose.volume.bind.invalid-selinux");

/// A typed value and the exact source span from which it was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located<T> {
    value: T,
    span: SourceSpan,
}

impl<T> Located<T> {
    pub(crate) const fn new(value: T, span: SourceSpan) -> Self {
        Self { value, span }
    }

    /// Returns the typed value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Returns the value's source span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Removes the source wrapper and returns the typed value.
    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }
}

/// Source provenance for an extension or not-yet-typed field.
///
/// The loss-aware [`SyntaxDocument`] retains the actual value and spelling. This reference lets
/// typed callers locate it without exposing the private YAML implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldReference {
    name: Located<String>,
    span: SourceSpan,
    value_span: Option<SourceSpan>,
}

impl FieldReference {
    /// Returns the semantic field name and its source span.
    #[must_use]
    pub const fn name(&self) -> &Located<String> {
        &self.name
    }

    /// Returns the span covering the key and value when both are available.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the value span when the YAML node exposes one.
    #[must_use]
    pub const fn value_span(&self) -> Option<SourceSpan> {
        self.value_span
    }
}

/// A typed Compose service in the initial Phase 2 subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    name: Located<String>,
    span: SourceSpan,
    image: Option<Located<ImageReference>>,
    command: Option<Command>,
    environment: Option<Environment>,
    ports: Vec<Port>,
    volumes: Vec<VolumeMount>,
    networks: Option<ServiceNetworks>,
    profiles: Vec<Located<String>>,
    configs: Vec<ConfigGrant>,
    secrets: Vec<SecretGrant>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl Service {
    /// Returns the service name.
    #[must_use]
    pub const fn name(&self) -> &Located<String> {
        &self.name
    }

    /// Returns the complete service definition span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the explicitly authored image reference.
    #[must_use]
    pub const fn image(&self) -> Option<&Located<ImageReference>> {
        self.image.as_ref()
    }

    /// Returns the command without normalizing its authored form.
    #[must_use]
    pub const fn command(&self) -> Option<&Command> {
        self.command.as_ref()
    }

    /// Returns environment variables with list and mapping forms kept distinct.
    #[must_use]
    pub const fn environment(&self) -> Option<&Environment> {
        self.environment.as_ref()
    }

    /// Returns published ports in authored order.
    #[must_use]
    pub fn ports(&self) -> &[Port] {
        &self.ports
    }

    /// Returns service-volume mounts in authored order.
    #[must_use]
    pub fn volumes(&self) -> &[VolumeMount] {
        &self.volumes
    }

    /// Returns service network attachments with short and long forms kept distinct.
    #[must_use]
    pub const fn networks(&self) -> Option<&ServiceNetworks> {
        self.networks.as_ref()
    }

    /// Returns explicitly authored profile names.
    #[must_use]
    pub fn profiles(&self) -> &[Located<String>] {
        &self.profiles
    }

    /// Returns service config grants in authored order.
    #[must_use]
    pub fn configs(&self) -> &[ConfigGrant] {
        &self.configs
    }

    /// Returns service secret grants in authored order.
    #[must_use]
    pub fn secrets(&self) -> &[SecretGrant] {
        &self.secrets
    }

    /// Returns retained service `x-` extension fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns service fields not yet represented by the typed subset.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// The initial source-aware native Compose document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeDocument {
    source_id: SourceId,
    span: SourceSpan,
    name: Option<Located<String>>,
    services: Vec<Service>,
    networks: Vec<NetworkDefinition>,
    volumes: Vec<VolumeDefinition>,
    configs: Vec<ConfigDefinition>,
    secrets: Vec<SecretDefinition>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl ComposeDocument {
    /// Extracts the initial typed Compose subset from a loss-aware syntax document.
    ///
    /// Parsing does not interpolate values, apply defaults, normalize short and long forms, or
    /// access the environment. Structural problems produce diagnostics and as much typed data as
    /// can be recovered.
    #[must_use]
    pub fn parse(syntax: &SyntaxDocument) -> ModelParse {
        Parser::new(syntax).parse()
    }

    /// Returns the source identifier.
    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// Returns the typed root mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the explicitly authored project name.
    #[must_use]
    pub const fn name(&self) -> Option<&Located<String>> {
        self.name.as_ref()
    }

    /// Returns services in authored order.
    #[must_use]
    pub fn services(&self) -> &[Service] {
        &self.services
    }

    /// Finds the first service with the requested name.
    #[must_use]
    pub fn service(&self, name: &str) -> Option<&Service> {
        self.services.iter().find(|service| service.name.value == name)
    }

    /// Returns top-level network definitions in authored order.
    #[must_use]
    pub fn networks(&self) -> &[NetworkDefinition] {
        &self.networks
    }

    /// Returns top-level volume definitions in authored order.
    #[must_use]
    pub fn volumes(&self) -> &[VolumeDefinition] {
        &self.volumes
    }

    /// Returns top-level config definitions in authored order.
    #[must_use]
    pub fn configs(&self) -> &[ConfigDefinition] {
        &self.configs
    }

    /// Returns top-level secret definitions in authored order.
    #[must_use]
    pub fn secrets(&self) -> &[SecretDefinition] {
        &self.secrets
    }

    /// Returns retained top-level `x-` extension fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns top-level fields not yet represented by the typed subset.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// A recoverable typed-model parse result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelParse {
    document: Option<ComposeDocument>,
    diagnostics: Vec<Diagnostic>,
}

impl ModelParse {
    /// Returns the typed document when the root could be interpreted.
    #[must_use]
    pub const fn document(&self) -> Option<&ComposeDocument> {
        self.document.as_ref()
    }

    /// Returns structural typed-model diagnostics in source order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether no error diagnostics were emitted.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == Severity::Error)
    }

    /// Separates the recovered document and diagnostics.
    #[must_use]
    pub fn into_parts(self) -> (Option<ComposeDocument>, Vec<Diagnostic>) {
        (self.document, self.diagnostics)
    }
}

#[derive(Debug)]
struct Parser {
    source_id: SourceId,
    source_span: SourceSpan,
    source: String,
    tree: yaml_edit::YamlFile,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    fn new(syntax: &SyntaxDocument) -> Self {
        Self {
            source_id: syntax.source_id(),
            source_span: syntax.source_span(),
            source: syntax.source_text().to_owned(),
            tree: syntax.yaml_file(),
            diagnostics: Vec::new(),
        }
    }

    fn parse(mut self) -> ModelParse {
        if self.tree.documents().count() > 1 {
            self.diagnostics.push(
                Diagnostic::new(
                    MULTIPLE_DOCUMENTS,
                    Severity::Error,
                    "Compose input must contain one YAML document",
                )
                .with_label(DiagnosticLabel::primary(self.source_span, "multiple YAML documents")),
            );
        }

        let Some(root) = self.tree.document() else {
            self.diagnostics.push(
                Diagnostic::new(
                    DOCUMENT_ROOT_TYPE,
                    Severity::Error,
                    "Compose document root must be a mapping",
                )
                .with_label(DiagnosticLabel::primary(self.source_span, "empty document")),
            );
            return ModelParse {
                document: None,
                diagnostics: self.diagnostics,
            };
        };
        let root_span = span_from_position(self.source_id, root.byte_range());
        let Some(mapping) = root.as_mapping() else {
            self.diagnostics.push(
                Diagnostic::new(
                    DOCUMENT_ROOT_TYPE,
                    Severity::Error,
                    "Compose document root must be a mapping",
                )
                .with_label(DiagnosticLabel::primary(root_span, "not a mapping")),
            );
            return ModelParse {
                document: None,
                diagnostics: self.diagnostics,
            };
        };

        let document = self.parse_root(&mapping, root_span);
        ModelParse {
            document: Some(document),
            diagnostics: self.diagnostics,
        }
    }

    fn parse_root(&mut self, mapping: &Mapping, span: SourceSpan) -> ComposeDocument {
        let mut document = ComposeDocument {
            source_id: self.source_id,
            span,
            name: None,
            services: Vec::new(),
            networks: Vec::new(),
            volumes: Vec::new(),
            configs: Vec::new(),
            secrets: Vec::new(),
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        };
        let mut seen = BTreeMap::new();

        for field in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &field);
            match field.name.value.as_str() {
                "name" if !duplicate => {
                    document.name = self.parse_string(&field, "project name");
                }
                "services" if !duplicate => {
                    document.services = self.parse_services(&field);
                }
                "networks" if !duplicate => {
                    document.networks = self.parse_network_definitions(&field);
                }
                "volumes" if !duplicate => {
                    document.volumes = self.parse_volume_definitions(&field);
                }
                "configs" if !duplicate => {
                    document.configs = self.parse_config_definitions(&field);
                }
                "secrets" if !duplicate => {
                    document.secrets = self.parse_secret_definitions(&field);
                }
                name if name.starts_with("x-") => {
                    document.extension_fields.push(field.reference());
                }
                _ if duplicate => {}
                _ => document.unknown_fields.push(field.reference()),
            }
        }
        document
    }

    fn parse_services(&mut self, field: &ParsedField) -> Vec<Service> {
        let Some(mapping) = field.value.as_ref().and_then(YamlNode::as_mapping) else {
            self.expected(EXPECTED_MAPPING, field, "services must be a mapping");
            return Vec::new();
        };
        let mut services = Vec::new();
        let mut seen = BTreeMap::new();
        for service_field in self.fields(mapping) {
            self.record_duplicate(&mut seen, &service_field);
            let Some(service_mapping) = service_field.value.as_ref().and_then(YamlNode::as_mapping) else {
                self.expected(EXPECTED_MAPPING, &service_field, "service definition must be a mapping");
                continue;
            };
            services.push(self.parse_service(&service_field, service_mapping));
        }
        services
    }

    fn parse_service(&mut self, field: &ParsedField, mapping: &Mapping) -> Service {
        let mut service = Service {
            name: field.name.clone(),
            span: field.span,
            image: None,
            command: None,
            environment: None,
            ports: Vec::new(),
            volumes: Vec::new(),
            networks: None,
            profiles: Vec::new(),
            configs: Vec::new(),
            secrets: Vec::new(),
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        };
        let mut seen = BTreeMap::new();
        for service_field in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &service_field);
            match service_field.name.value.as_str() {
                "image" if !duplicate => {
                    service.image = self
                        .parse_string(&service_field, "service image")
                        .map(|value| Located::new(ImageReference::parse(value.value), value.span));
                }
                "command" if !duplicate => {
                    service.command = self.parse_command(&service_field);
                }
                "environment" if !duplicate => {
                    service.environment = self.parse_environment(&service_field);
                }
                "ports" if !duplicate => {
                    service.ports = self.parse_service_ports(&service_field);
                }
                "volumes" if !duplicate => {
                    service.volumes = self.parse_service_volumes(&service_field);
                }
                "networks" if !duplicate => {
                    service.networks = self.parse_service_networks(&service_field);
                }
                "profiles" if !duplicate => {
                    service.profiles = self.parse_string_sequence(&service_field, "service profiles");
                }
                "configs" if !duplicate => {
                    service.configs = self.parse_config_grants(&service_field);
                }
                "secrets" if !duplicate => {
                    service.secrets = self.parse_secret_grants(&service_field);
                }
                name if name.starts_with("x-") => {
                    service.extension_fields.push(service_field.reference());
                }
                _ if duplicate => {}
                _ => service.unknown_fields.push(service_field.reference()),
            }
        }
        service
    }

    fn parse_command(&mut self, field: &ParsedField) -> Option<Command> {
        match field.value.as_ref() {
            Some(YamlNode::Scalar(scalar)) => {
                let span = span_from_position(self.source_id, scalar.byte_range());
                if ScalarValue::from_scalar(scalar).scalar_type() == ScalarType::Null {
                    Some(Command::Null(span))
                } else {
                    Some(Command::String(Located::new(scalar.as_string(), span)))
                }
            }
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let values =
                    self.parse_scalar_nodes(sequence.values(), field.span, "command list items must be scalars");
                Some(Command::List { span, values })
            }
            _ => {
                self.expected(
                    EXPECTED_FIELD_FORM,
                    field,
                    "command must be null, a scalar, or a sequence",
                );
                None
            }
        }
    }

    fn parse_environment(&mut self, field: &ParsedField) -> Option<Environment> {
        match field.value.as_ref() {
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let entries = self
                    .parse_scalar_nodes(sequence.values(), field.span, "environment list items must be scalars")
                    .into_iter()
                    .map(EnvironmentListEntry::parse)
                    .collect();
                Some(Environment::List { span, entries })
            }
            Some(YamlNode::Mapping(mapping)) => {
                let span = span_from_position(self.source_id, mapping.byte_range());
                let entries = self.parse_environment_map(mapping);
                Some(Environment::Map { span, entries })
            }
            _ => {
                self.expected(EXPECTED_FIELD_FORM, field, "environment must be a sequence or mapping");
                None
            }
        }
    }

    fn parse_environment_map(&mut self, mapping: &Mapping) -> Vec<EnvironmentMapEntry> {
        let mut entries = Vec::new();
        let mut seen = BTreeMap::new();
        for field in self.fields(mapping) {
            if self.record_duplicate(&mut seen, &field) {
                continue;
            }
            let value = self.parse_compose_scalar(&field, "environment values must be scalars");
            if let Some(value) = value {
                entries.push(EnvironmentMapEntry::new(field.name, value, field.span));
            }
        }
        entries
    }

    fn indentation(&self, offset: usize) -> usize {
        let prefix = self.source.get(..offset).unwrap_or_default();
        let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
        self.source[line_start..offset]
            .chars()
            .take_while(|character| character.is_whitespace())
            .count()
    }

    fn parse_service_ports(&mut self, field: &ParsedField) -> Vec<Port> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(EXPECTED_SEQUENCE, field, "service ports must be a sequence");
            return Vec::new();
        };

        let mut ports = Vec::new();
        for value in sequence.values() {
            match value {
                YamlNode::Scalar(scalar) => {
                    let span = span_from_position(self.source_id, scalar.byte_range());
                    ports.push(Port::Short(ShortPort::parse(Located::new(scalar.as_string(), span))));
                }
                YamlNode::Mapping(mapping) => {
                    ports.push(Port::Long(Box::new(self.parse_long_port(&mapping))));
                }
                other => self.unsupported_sequence_item(
                    PORT_EXPECTED_FORM,
                    &other,
                    field.span,
                    "service port must use scalar short syntax or mapping long syntax",
                ),
            }
        }
        ports
    }

    fn parse_long_port(&mut self, mapping: &Mapping) -> LongPort {
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut port = LongPort::new(span);
        let mut seen = BTreeMap::new();
        for field in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &field);
            match field.name.value.as_str() {
                "target" if !duplicate => self
                    .parse_string(&field, "port target")
                    .into_iter()
                    .for_each(|value| port.set_target(value)),
                "published" if !duplicate => self
                    .parse_string(&field, "published port")
                    .into_iter()
                    .for_each(|value| port.set_published(value)),
                "host_ip" if !duplicate => self
                    .parse_string(&field, "port host IP")
                    .into_iter()
                    .for_each(|value| port.set_host_ip(value)),
                "protocol" if !duplicate => self
                    .parse_string(&field, "port protocol")
                    .into_iter()
                    .for_each(|value| port.set_protocol(value)),
                "app_protocol" if !duplicate => self
                    .parse_string(&field, "port application protocol")
                    .into_iter()
                    .for_each(|value| port.set_app_protocol(value)),
                "mode" if !duplicate => self
                    .parse_string(&field, "port mode")
                    .into_iter()
                    .for_each(|value| port.set_mode(value)),
                "name" if !duplicate => self
                    .parse_string(&field, "port name")
                    .into_iter()
                    .for_each(|value| port.set_name(value)),
                name if name.starts_with("x-") => port.push_extension(field.reference()),
                _ if duplicate => {}
                _ => port.push_unknown(field.reference()),
            }
        }
        if port.target().is_none() {
            self.missing(PORT_MISSING_TARGET, span, "long port is missing `target`");
        }
        port
    }

    fn parse_service_networks(&mut self, field: &ParsedField) -> Option<ServiceNetworks> {
        match field.value.as_ref() {
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let names =
                    self.parse_scalar_nodes(sequence.values(), field.span, "service network names must be scalars");
                Some(ServiceNetworks::Short { span, names })
            }
            Some(YamlNode::Mapping(mapping)) => {
                let span = span_from_position(self.source_id, mapping.byte_range());
                let networks = self.parse_service_network_map(mapping);
                Some(ServiceNetworks::Long { span, networks })
            }
            _ => {
                self.expected(
                    EXPECTED_FIELD_FORM,
                    field,
                    "service networks must be a sequence or mapping",
                );
                None
            }
        }
    }

    fn parse_service_network_map(&mut self, mapping: &Mapping) -> Vec<ServiceNetwork> {
        let mut networks = Vec::new();
        let mut seen = BTreeMap::new();
        for field in self.fields(mapping) {
            if self.record_duplicate(&mut seen, &field) {
                continue;
            }
            if Self::field_is_null(&field) {
                networks.push(ServiceNetwork::new(field.name, field.span));
                continue;
            }
            let Some(options) = field.value.as_ref().and_then(YamlNode::as_mapping) else {
                self.expected(
                    EXPECTED_MAPPING,
                    &field,
                    "service network options must be a mapping or null",
                );
                continue;
            };
            networks.push(self.parse_service_network(&field, options));
        }
        networks
    }

    fn parse_service_network(&mut self, field: &ParsedField, mapping: &Mapping) -> ServiceNetwork {
        let mut network = ServiceNetwork::new(field.name.clone(), field.span);
        let mut seen = BTreeMap::new();
        for option in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &option);
            match option.name.value.as_str() {
                "aliases" if !duplicate => network.set_aliases(self.parse_string_sequence(&option, "network aliases")),
                "interface_name" if !duplicate => self
                    .parse_string(&option, "network interface name")
                    .into_iter()
                    .for_each(|value| network.set_interface_name(value)),
                "ipv4_address" if !duplicate => self
                    .parse_string(&option, "network IPv4 address")
                    .into_iter()
                    .for_each(|value| network.set_ipv4_address(value)),
                "ipv6_address" if !duplicate => self
                    .parse_string(&option, "network IPv6 address")
                    .into_iter()
                    .for_each(|value| network.set_ipv6_address(value)),
                "link_local_ips" if !duplicate => {
                    network.set_link_local_ips(self.parse_string_sequence(&option, "link-local IP addresses"));
                }
                "mac_address" if !duplicate => self
                    .parse_string(&option, "network MAC address")
                    .into_iter()
                    .for_each(|value| network.set_mac_address(value)),
                "driver_opts" if !duplicate => {
                    network.set_driver_opts(self.parse_scalar_mapping(&option, "network driver options"));
                }
                "gw_priority" if !duplicate => self
                    .parse_string(&option, "network gateway priority")
                    .into_iter()
                    .for_each(|value| network.set_gw_priority(value)),
                "priority" if !duplicate => self
                    .parse_string(&option, "network priority")
                    .into_iter()
                    .for_each(|value| network.set_priority(value)),
                name if name.starts_with("x-") => network.push_extension(option.reference()),
                _ if duplicate => {}
                _ => network.push_unknown(option.reference()),
            }
        }
        network
    }

    fn parse_config_grants(&mut self, field: &ParsedField) -> Vec<ConfigGrant> {
        self.parse_grants(field)
            .into_iter()
            .map(|grant| match grant {
                ParsedGrant::Short(value) => ConfigGrant::Short(value),
                ParsedGrant::Long(value) => ConfigGrant::Long(value),
            })
            .collect()
    }

    fn parse_secret_grants(&mut self, field: &ParsedField) -> Vec<SecretGrant> {
        self.parse_grants(field)
            .into_iter()
            .map(|grant| match grant {
                ParsedGrant::Short(value) => SecretGrant::Short(value),
                ParsedGrant::Long(value) => SecretGrant::Long(value),
            })
            .collect()
    }

    fn parse_grants(&mut self, field: &ParsedField) -> Vec<ParsedGrant> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(EXPECTED_SEQUENCE, field, "service grants must be a sequence");
            return Vec::new();
        };
        let mut grants = Vec::new();
        for value in sequence.values() {
            match value {
                YamlNode::Scalar(scalar) => {
                    let span = span_from_position(self.source_id, scalar.byte_range());
                    grants.push(ParsedGrant::Short(Located::new(scalar.as_string(), span)));
                }
                YamlNode::Mapping(mapping) => {
                    grants.push(ParsedGrant::Long(Box::new(self.parse_long_grant(&mapping))));
                }
                other => self.unsupported_sequence_item(
                    GRANT_EXPECTED_FORM,
                    &other,
                    field.span,
                    "grant must use scalar short syntax or mapping long syntax",
                ),
            }
        }
        grants
    }

    fn parse_long_grant(&mut self, mapping: &Mapping) -> LongGrant {
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut grant = LongGrant::new(span);
        let mut seen = BTreeMap::new();
        for field in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &field);
            match field.name.value.as_str() {
                "source" if !duplicate => self
                    .parse_string(&field, "grant source")
                    .into_iter()
                    .for_each(|value| grant.set_source(value)),
                "target" if !duplicate => self
                    .parse_string(&field, "grant target")
                    .into_iter()
                    .for_each(|value| grant.set_target(value)),
                "uid" if !duplicate => self
                    .parse_string(&field, "grant user ID")
                    .into_iter()
                    .for_each(|value| grant.set_uid(value)),
                "gid" if !duplicate => self
                    .parse_string(&field, "grant group ID")
                    .into_iter()
                    .for_each(|value| grant.set_gid(value)),
                "mode" if !duplicate => self
                    .parse_string(&field, "grant mode")
                    .into_iter()
                    .for_each(|value| grant.set_mode(value)),
                name if name.starts_with("x-") => grant.push_extension(field.reference()),
                _ if duplicate => {}
                _ => grant.push_unknown(field.reference()),
            }
        }
        if grant.source().is_none() {
            self.missing(GRANT_MISSING_SOURCE, span, "long grant is missing `source`");
        }
        grant
    }

    fn parse_service_volumes(&mut self, field: &ParsedField) -> Vec<VolumeMount> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(EXPECTED_SEQUENCE, field, "service volumes must be a sequence");
            return Vec::new();
        };

        sequence
            .values()
            .filter_map(|value| match value {
                YamlNode::Scalar(scalar) => {
                    let span = span_from_position(self.source_id, scalar.byte_range());
                    let raw = Located::new(scalar.as_string(), span);
                    Some(VolumeMount::Short(ShortVolumeMount::new(raw)))
                }
                YamlNode::Mapping(mapping) => Some(VolumeMount::Long(Box::new(self.parse_long_volume(&mapping)))),
                other => {
                    let span = node_span(self.source_id, &other).unwrap_or(field.span);
                    self.diagnostics.push(
                        Diagnostic::new(
                            VOLUME_EXPECTED_FORM,
                            Severity::Error,
                            "service volume must use scalar short syntax or mapping long syntax",
                        )
                        .with_label(DiagnosticLabel::primary(span, "unsupported volume form")),
                    );
                    None
                }
            })
            .collect()
    }

    fn parse_long_volume(&mut self, mapping: &Mapping) -> LongVolumeMount {
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut mount = LongVolumeMount::new(span);
        let mut seen = BTreeMap::new();
        for field in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &field);
            match field.name.value.as_str() {
                "type" if !duplicate => {
                    if let Some(value) = self.parse_string(&field, "volume type") {
                        mount.set_mount_type(Located::new(MountType::from_text(value.value), value.span));
                    }
                }
                "source" if !duplicate => {
                    if let Some(value) = self.parse_string(&field, "volume source") {
                        mount.set_source(value);
                    }
                }
                "target" if !duplicate => {
                    if let Some(value) = self.parse_string(&field, "volume target") {
                        mount.set_target(value);
                    }
                }
                "read_only" if !duplicate => {
                    if let Some(value) = self.parse_boolean(&field, "read_only") {
                        mount.set_read_only(value);
                    }
                }
                "bind" if !duplicate => {
                    if let Some(value) = self.parse_bind_options(&field) {
                        mount.set_bind(value);
                    }
                }
                name if name.starts_with("x-") => mount.push_extension(field.reference()),
                _ if duplicate => {}
                _ => mount.push_unknown(field.reference()),
            }
        }

        if mount.mount_type().is_none() {
            self.missing(VOLUME_MISSING_TYPE, span, "long volume is missing `type`");
        }
        if mount.target().is_none() {
            self.missing(VOLUME_MISSING_TARGET, span, "long volume is missing `target`");
        }
        mount
    }

    fn parse_bind_options(&mut self, field: &ParsedField) -> Option<BindOptions> {
        let Some(mapping) = field.value.as_ref().and_then(YamlNode::as_mapping) else {
            self.expected(EXPECTED_MAPPING, field, "bind options must be a mapping");
            return None;
        };
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut bind = BindOptions::new(span);
        let mut seen = BTreeMap::new();
        for bind_field in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &bind_field);
            match bind_field.name.value.as_str() {
                "propagation" if !duplicate => {
                    if let Some(value) = self.parse_string(&bind_field, "bind propagation") {
                        bind.set_propagation(value);
                    }
                }
                "create_host_path" if !duplicate => {
                    if let Some(value) = self.parse_boolean(&bind_field, "create_host_path") {
                        bind.set_create_host_path(value);
                    }
                }
                "selinux" if !duplicate => {
                    if let Some(value) = self.parse_string(&bind_field, "SELinux relabel mode") {
                        let mode = match value.value.as_str() {
                            "z" => Some(SelinuxRelabel::Shared),
                            "Z" => Some(SelinuxRelabel::Private),
                            _ => None,
                        };
                        if let Some(mode) = mode {
                            bind.set_selinux(Located::new(mode, value.span));
                        } else {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    VOLUME_INVALID_SELINUX,
                                    Severity::Error,
                                    "SELinux relabel mode must be `z` or `Z`",
                                )
                                .with_label(DiagnosticLabel::primary(value.span, "invalid SELinux mode")),
                            );
                        }
                    }
                }
                name if name.starts_with("x-") => bind.push_extension(bind_field.reference()),
                _ if duplicate => {}
                _ => bind.push_unknown(bind_field.reference()),
            }
        }
        Some(bind)
    }

    fn parse_network_definitions(&mut self, field: &ParsedField) -> Vec<NetworkDefinition> {
        let Some(mapping) = self.resource_collection(field, "networks") else {
            return Vec::new();
        };
        let mut definitions = Vec::new();
        let mut seen = BTreeMap::new();
        for resource in self.fields(&mapping) {
            if self.record_duplicate(&mut seen, &resource) {
                continue;
            }
            if Self::field_is_null(&resource) {
                definitions.push(NetworkDefinition::new(resource.name, resource.span));
                continue;
            }
            let Some(definition) = resource.value.as_ref().and_then(YamlNode::as_mapping) else {
                self.expected(
                    RESOURCE_EXPECTED_FORM,
                    &resource,
                    "network definition must be a mapping or null",
                );
                continue;
            };
            definitions.push(self.parse_network_definition(&resource, definition));
        }
        definitions
    }

    fn parse_network_definition(&mut self, field: &ParsedField, mapping: &Mapping) -> NetworkDefinition {
        let mut network = NetworkDefinition::new(field.name.clone(), field.span);
        let mut seen = BTreeMap::new();
        for option in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &option);
            match option.name.value.as_str() {
                "driver" if !duplicate => self
                    .parse_string(&option, "network driver")
                    .into_iter()
                    .for_each(|value| network.set_driver(value)),
                "driver_opts" if !duplicate => {
                    network.set_driver_opts(self.parse_scalar_mapping(&option, "network driver options"));
                }
                "attachable" if !duplicate => self
                    .parse_boolean(&option, "network attachable")
                    .into_iter()
                    .for_each(|value| network.set_attachable(value)),
                "enable_ipv4" if !duplicate => self
                    .parse_boolean(&option, "network enable_ipv4")
                    .into_iter()
                    .for_each(|value| network.set_enable_ipv4(value)),
                "enable_ipv6" if !duplicate => self
                    .parse_boolean(&option, "network enable_ipv6")
                    .into_iter()
                    .for_each(|value| network.set_enable_ipv6(value)),
                "external" if !duplicate => self
                    .parse_boolean(&option, "network external")
                    .into_iter()
                    .for_each(|value| network.set_external(value)),
                "internal" if !duplicate => self
                    .parse_boolean(&option, "network internal")
                    .into_iter()
                    .for_each(|value| network.set_internal(value)),
                "ipam" if !duplicate => self
                    .parse_ipam(&option)
                    .into_iter()
                    .for_each(|value| network.set_ipam(value)),
                "labels" if !duplicate => self
                    .parse_labels(&option)
                    .into_iter()
                    .for_each(|value| network.set_labels(value)),
                "name" if !duplicate => self
                    .parse_string(&option, "network custom name")
                    .into_iter()
                    .for_each(|value| network.set_custom_name(value)),
                name if name.starts_with("x-") => network.push_extension(option.reference()),
                _ if duplicate => {}
                _ => network.push_unknown(option.reference()),
            }
        }
        network
    }

    fn parse_ipam(&mut self, field: &ParsedField) -> Option<Ipam> {
        let Some(mapping) = field.value.as_ref().and_then(YamlNode::as_mapping) else {
            self.expected(EXPECTED_MAPPING, field, "network IPAM must be a mapping");
            return None;
        };
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut ipam = Ipam::new(span);
        let mut seen = BTreeMap::new();
        for option in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &option);
            match option.name.value.as_str() {
                "driver" if !duplicate => self
                    .parse_string(&option, "IPAM driver")
                    .into_iter()
                    .for_each(|value| ipam.set_driver(value)),
                "config" if !duplicate => ipam.set_config(self.parse_ipam_configs(&option)),
                "options" if !duplicate => {
                    ipam.set_options(self.parse_scalar_mapping(&option, "IPAM options"));
                }
                name if name.starts_with("x-") => ipam.push_extension(option.reference()),
                _ if duplicate => {}
                _ => ipam.push_unknown(option.reference()),
            }
        }
        Some(ipam)
    }

    fn parse_ipam_configs(&mut self, field: &ParsedField) -> Vec<IpamConfig> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(EXPECTED_SEQUENCE, field, "IPAM config must be a sequence");
            return Vec::new();
        };
        let mut configs = Vec::new();
        for value in sequence.values() {
            let YamlNode::Mapping(mapping) = value else {
                self.unsupported_sequence_item(
                    EXPECTED_MAPPING,
                    &value,
                    field.span,
                    "IPAM config entries must be mappings",
                );
                continue;
            };
            configs.push(self.parse_ipam_config(&mapping));
        }
        configs
    }

    fn parse_ipam_config(&mut self, mapping: &Mapping) -> IpamConfig {
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut config = IpamConfig::new(span);
        let mut seen = BTreeMap::new();
        for field in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &field);
            match field.name.value.as_str() {
                "subnet" if !duplicate => self
                    .parse_string(&field, "IPAM subnet")
                    .into_iter()
                    .for_each(|value| config.set_subnet(value)),
                "ip_range" if !duplicate => self
                    .parse_string(&field, "IPAM allocation range")
                    .into_iter()
                    .for_each(|value| config.set_ip_range(value)),
                "gateway" if !duplicate => self
                    .parse_string(&field, "IPAM gateway")
                    .into_iter()
                    .for_each(|value| config.set_gateway(value)),
                "aux_addresses" if !duplicate => {
                    config.set_aux_addresses(self.parse_scalar_mapping(&field, "IPAM auxiliary addresses"));
                }
                name if name.starts_with("x-") => config.push_extension(field.reference()),
                _ if duplicate => {}
                _ => config.push_unknown(field.reference()),
            }
        }
        config
    }

    fn parse_volume_definitions(&mut self, field: &ParsedField) -> Vec<VolumeDefinition> {
        let Some(mapping) = self.resource_collection(field, "volumes") else {
            return Vec::new();
        };
        let mut definitions = Vec::new();
        let mut seen = BTreeMap::new();
        for resource in self.fields(&mapping) {
            if self.record_duplicate(&mut seen, &resource) {
                continue;
            }
            let mut volume = VolumeDefinition::new(resource.name.clone(), resource.span);
            if Self::field_is_null(&resource) {
                definitions.push(volume);
                continue;
            }
            let Some(definition) = resource.value.as_ref().and_then(YamlNode::as_mapping) else {
                self.expected(
                    RESOURCE_EXPECTED_FORM,
                    &resource,
                    "volume definition must be a mapping or null",
                );
                continue;
            };
            let mut nested_seen = BTreeMap::new();
            for option in self.fields(definition) {
                let duplicate = self.record_duplicate(&mut nested_seen, &option);
                match option.name.value.as_str() {
                    "driver" if !duplicate => self
                        .parse_string(&option, "volume driver")
                        .into_iter()
                        .for_each(|value| volume.set_driver(value)),
                    "driver_opts" if !duplicate => {
                        volume.set_driver_opts(self.parse_scalar_mapping(&option, "volume driver options"));
                    }
                    "external" if !duplicate => self
                        .parse_boolean(&option, "volume external")
                        .into_iter()
                        .for_each(|value| volume.set_external(value)),
                    "labels" if !duplicate => self
                        .parse_labels(&option)
                        .into_iter()
                        .for_each(|value| volume.set_labels(value)),
                    "name" if !duplicate => self
                        .parse_string(&option, "volume custom name")
                        .into_iter()
                        .for_each(|value| volume.set_custom_name(value)),
                    name if name.starts_with("x-") => volume.push_extension(option.reference()),
                    _ if duplicate => {}
                    _ => volume.push_unknown(option.reference()),
                }
            }
            definitions.push(volume);
        }
        definitions
    }

    fn parse_config_definitions(&mut self, field: &ParsedField) -> Vec<ConfigDefinition> {
        let Some(mapping) = self.resource_collection(field, "configs") else {
            return Vec::new();
        };
        let mut definitions = Vec::new();
        let mut seen = BTreeMap::new();
        for resource in self.fields(&mapping) {
            if self.record_duplicate(&mut seen, &resource) {
                continue;
            }
            let mut config = ConfigDefinition::new(resource.name.clone(), resource.span);
            if Self::field_is_null(&resource) {
                definitions.push(config);
                continue;
            }
            let Some(definition) = resource.value.as_ref().and_then(YamlNode::as_mapping) else {
                self.expected(
                    RESOURCE_EXPECTED_FORM,
                    &resource,
                    "config definition must be a mapping or null",
                );
                continue;
            };
            let mut nested_seen = BTreeMap::new();
            for option in self.fields(definition) {
                let duplicate = self.record_duplicate(&mut nested_seen, &option);
                match option.name.value.as_str() {
                    "file" if !duplicate => self
                        .parse_string(&option, "config file")
                        .into_iter()
                        .for_each(|value| config.set_file(value)),
                    "environment" if !duplicate => self
                        .parse_string(&option, "config environment source")
                        .into_iter()
                        .for_each(|value| config.set_environment(value)),
                    "content" if !duplicate => self
                        .parse_string(&option, "config content")
                        .into_iter()
                        .for_each(|value| config.set_content(value)),
                    "external" if !duplicate => self
                        .parse_boolean(&option, "config external")
                        .into_iter()
                        .for_each(|value| config.set_external(value)),
                    "name" if !duplicate => self
                        .parse_string(&option, "config custom name")
                        .into_iter()
                        .for_each(|value| config.set_custom_name(value)),
                    name if name.starts_with("x-") => config.push_extension(option.reference()),
                    _ if duplicate => {}
                    _ => config.push_unknown(option.reference()),
                }
            }
            definitions.push(config);
        }
        definitions
    }

    fn parse_secret_definitions(&mut self, field: &ParsedField) -> Vec<SecretDefinition> {
        let Some(mapping) = self.resource_collection(field, "secrets") else {
            return Vec::new();
        };
        let mut definitions = Vec::new();
        let mut seen = BTreeMap::new();
        for resource in self.fields(&mapping) {
            if self.record_duplicate(&mut seen, &resource) {
                continue;
            }
            let mut secret = SecretDefinition::new(resource.name.clone(), resource.span);
            if Self::field_is_null(&resource) {
                definitions.push(secret);
                continue;
            }
            let Some(definition) = resource.value.as_ref().and_then(YamlNode::as_mapping) else {
                self.expected(
                    RESOURCE_EXPECTED_FORM,
                    &resource,
                    "secret definition must be a mapping or null",
                );
                continue;
            };
            let mut nested_seen = BTreeMap::new();
            for option in self.fields(definition) {
                let duplicate = self.record_duplicate(&mut nested_seen, &option);
                match option.name.value.as_str() {
                    "file" if !duplicate => self
                        .parse_string(&option, "secret file")
                        .into_iter()
                        .for_each(|value| secret.set_file(value)),
                    "environment" if !duplicate => self
                        .parse_string(&option, "secret environment source")
                        .into_iter()
                        .for_each(|value| secret.set_environment(value)),
                    "external" if !duplicate => self
                        .parse_boolean(&option, "secret external")
                        .into_iter()
                        .for_each(|value| secret.set_external(value)),
                    "name" if !duplicate => self
                        .parse_string(&option, "secret custom name")
                        .into_iter()
                        .for_each(|value| secret.set_custom_name(value)),
                    name if name.starts_with("x-") => secret.push_extension(option.reference()),
                    _ if duplicate => {}
                    _ => secret.push_unknown(option.reference()),
                }
            }
            definitions.push(secret);
        }
        definitions
    }

    fn resource_collection(&mut self, field: &ParsedField, kind: &str) -> Option<Mapping> {
        let Some(mapping) = field.value.as_ref().and_then(YamlNode::as_mapping) else {
            self.expected(EXPECTED_MAPPING, field, format!("top-level {kind} must be a mapping"));
            return None;
        };
        Some(mapping.clone())
    }

    fn parse_string(&mut self, field: &ParsedField, description: &str) -> Option<Located<String>> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(EXPECTED_SCALAR, field, format!("{description} must be a scalar"));
            return None;
        };
        if ScalarValue::from_scalar(scalar).scalar_type() == ScalarType::Null {
            self.expected(
                EXPECTED_SCALAR,
                field,
                format!("{description} must be a non-null scalar"),
            );
            return None;
        }
        Some(Located::new(
            scalar.as_string(),
            span_from_position(self.source_id, scalar.byte_range()),
        ))
    }

    fn parse_boolean(&mut self, field: &ParsedField, description: &str) -> Option<Located<BooleanValue>> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(EXPECTED_BOOLEAN, field, format!("{description} must be a boolean"));
            return None;
        };
        let span = span_from_position(self.source_id, scalar.byte_range());
        let scalar_value = ScalarValue::from_scalar(scalar);
        if let Some(value) = scalar_value.to_bool() {
            return Some(Located::new(BooleanValue::Literal(value), span));
        }
        let value = scalar.as_string();
        if value.contains('$') {
            return Some(Located::new(BooleanValue::Expression(value), span));
        }
        self.diagnostics.push(
            Diagnostic::new(
                EXPECTED_BOOLEAN,
                Severity::Error,
                format!("{description} must be a boolean or interpolation expression"),
            )
            .with_label(DiagnosticLabel::primary(span, "not a boolean expression")),
        );
        None
    }

    fn parse_string_sequence(&mut self, field: &ParsedField, description: &str) -> Vec<Located<String>> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(EXPECTED_SEQUENCE, field, format!("{description} must be a sequence"));
            return Vec::new();
        };
        self.parse_scalar_nodes(
            sequence.values(),
            field.span,
            format!("{description} entries must be scalars"),
        )
    }

    fn parse_scalar_nodes(
        &mut self,
        nodes: impl Iterator<Item = YamlNode>,
        fallback_span: SourceSpan,
        message: impl Into<String>,
    ) -> Vec<Located<String>> {
        let message = message.into();
        let mut values = Vec::new();
        for node in nodes {
            let YamlNode::Scalar(scalar) = node else {
                self.unsupported_sequence_item(EXPECTED_SCALAR, &node, fallback_span, &message);
                continue;
            };
            let scalar_value = ScalarValue::from_scalar(&scalar);
            if scalar_value.scalar_type() == ScalarType::Null {
                self.unsupported_sequence_item(EXPECTED_SCALAR, &YamlNode::Scalar(scalar), fallback_span, &message);
                continue;
            }
            let span = span_from_position(self.source_id, scalar.byte_range());
            values.push(Located::new(scalar.as_string(), span));
        }
        values
    }

    fn parse_scalar_mapping(&mut self, field: &ParsedField, description: &str) -> Vec<KeyValueEntry> {
        let Some(mapping) = field.value.as_ref().and_then(YamlNode::as_mapping) else {
            self.expected(EXPECTED_MAPPING, field, format!("{description} must be a mapping"));
            return Vec::new();
        };
        let mut entries = Vec::new();
        let mut seen = BTreeMap::new();
        for entry in self.fields(mapping) {
            if self.record_duplicate(&mut seen, &entry) {
                continue;
            }
            if let Some(value) = self.parse_compose_scalar(&entry, format!("{description} values must be scalars")) {
                entries.push(KeyValueEntry::new(entry.name, value, entry.span));
            }
        }
        entries
    }

    fn parse_compose_scalar(
        &mut self,
        field: &ParsedField,
        message: impl Into<String>,
    ) -> Option<Located<ComposeScalar>> {
        let Some(node) = field.value.as_ref() else {
            return Some(Located::new(ComposeScalar::Null, field.name.span));
        };
        let Some(scalar) = node.as_scalar() else {
            self.expected(EXPECTED_SCALAR, field, message);
            return None;
        };
        let span = span_from_position(self.source_id, scalar.byte_range());
        let value = ScalarValue::from_scalar(scalar);
        let typed = match value.scalar_type() {
            ScalarType::Null => ComposeScalar::Null,
            ScalarType::Boolean => ComposeScalar::Boolean(value.to_bool().unwrap_or(false)),
            ScalarType::Integer | ScalarType::Float => ComposeScalar::Number(scalar.as_string()),
            ScalarType::String | ScalarType::Timestamp | ScalarType::Regex => ComposeScalar::String(scalar.as_string()),
        };
        Some(Located::new(typed, span))
    }

    fn parse_labels(&mut self, field: &ParsedField) -> Option<Labels> {
        match field.value.as_ref() {
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let values =
                    self.parse_scalar_nodes(sequence.values(), field.span, "label list entries must be scalars");
                Some(Labels::List { span, values })
            }
            Some(YamlNode::Mapping(mapping)) => {
                let span = span_from_position(self.source_id, mapping.byte_range());
                let entries = self.parse_scalar_mapping(field, "labels");
                Some(Labels::Map { span, entries })
            }
            _ => {
                self.expected(EXPECTED_FIELD_FORM, field, "labels must be a sequence or mapping");
                None
            }
        }
    }

    fn field_is_null(field: &ParsedField) -> bool {
        field.value.as_ref().is_none_or(|node| {
            node.as_scalar()
                .is_some_and(|scalar| ScalarValue::from_scalar(scalar).scalar_type() == ScalarType::Null)
        })
    }

    fn unsupported_sequence_item(
        &mut self,
        code: DiagnosticCode,
        node: &YamlNode,
        fallback_span: SourceSpan,
        message: impl Into<String>,
    ) {
        let span = node_span(self.source_id, node).unwrap_or(fallback_span);
        self.diagnostics.push(
            Diagnostic::new(code, Severity::Error, message)
                .with_label(DiagnosticLabel::primary(span, "unsupported value form")),
        );
    }

    fn fields(&mut self, mapping: &Mapping) -> Vec<ParsedField> {
        let fields = self.raw_fields(mapping);
        self.flatten_empty_value_continuations(fields)
    }

    fn raw_fields(&mut self, mapping: &Mapping) -> Vec<ParsedField> {
        mapping
            .entries()
            .filter_map(|entry| {
                let key = entry.key_node()?;
                let Some(scalar) = key.as_scalar() else {
                    let span = node_span(self.source_id, &key)
                        .unwrap_or_else(|| span_from_position(self.source_id, mapping.byte_range()));
                    self.diagnostics.push(
                        Diagnostic::new(EXPECTED_SCALAR, Severity::Error, "Compose mapping keys must be scalars")
                            .with_label(DiagnosticLabel::primary(span, "non-scalar key")),
                    );
                    return None;
                };
                let name_span = span_from_position(self.source_id, scalar.byte_range());
                let authored_value = entry.value_node();
                let value_span = authored_value
                    .as_ref()
                    .and_then(|value| node_span(self.source_id, value));
                let value = authored_value.map(unwrap_processing_tag);
                let span = value_span.map_or(name_span, |value_span| union(name_span, value_span));
                Some(ParsedField {
                    name: Located::new(scalar.as_string(), name_span),
                    value,
                    value_span,
                    span,
                })
            })
            .collect()
    }

    fn flatten_empty_value_continuations(&mut self, fields: Vec<ParsedField>) -> Vec<ParsedField> {
        let mut flattened = Vec::new();
        for mut field in fields {
            let continuation = field
                .value
                .as_ref()
                .and_then(YamlNode::as_mapping)
                .filter(|mapping| {
                    mapping
                        .entries()
                        .find_map(|entry| {
                            let key = entry.key_node()?;
                            let scalar = key.as_scalar()?;
                            Some(scalar.byte_range().start as usize)
                        })
                        .is_some_and(|key_start| {
                            self.indentation(key_start) <= self.indentation(field.name.span.start())
                        })
                })
                .cloned();

            if let Some(mapping) = continuation {
                field.value = None;
                field.value_span = None;
                field.span = field.name.span;
                flattened.push(field);
                let nested = self.raw_fields(&mapping);
                flattened.extend(self.flatten_empty_value_continuations(nested));
            } else {
                flattened.push(field);
            }
        }
        flattened
    }

    fn record_duplicate(&mut self, seen: &mut BTreeMap<String, SourceSpan>, field: &ParsedField) -> bool {
        if let Some(first) = seen.get(field.name.value()) {
            self.diagnostics.push(
                Diagnostic::new(
                    DUPLICATE_FIELD,
                    Severity::Error,
                    "Compose mapping fields must be unique",
                )
                .with_label(DiagnosticLabel::primary(field.name.span, "duplicate field"))
                .with_label(DiagnosticLabel::secondary(*first, "first field")),
            );
            true
        } else {
            seen.insert(field.name.value.clone(), field.name.span);
            false
        }
    }

    fn expected(&mut self, code: DiagnosticCode, field: &ParsedField, message: impl Into<String>) {
        self.diagnostics.push(
            Diagnostic::new(code, Severity::Error, message)
                .with_label(DiagnosticLabel::primary(field.span, "unexpected value form")),
        );
    }

    fn missing(&mut self, code: DiagnosticCode, span: SourceSpan, message: &'static str) {
        self.diagnostics.push(
            Diagnostic::new(code, Severity::Error, message)
                .with_label(DiagnosticLabel::primary(span, "incomplete long syntax")),
        );
    }
}

fn unwrap_processing_tag(node: YamlNode) -> YamlNode {
    let YamlNode::TaggedNode(tagged) = &node else {
        return node;
    };
    if !matches!(tagged.tag().as_deref(), Some("!reset" | "!override")) {
        return node;
    }
    tagged
        .as_node()
        .and_then(|syntax| syntax.children().find_map(YamlNode::from_syntax))
        .unwrap_or(node)
}

#[derive(Debug, Clone)]
enum ParsedGrant {
    Short(Located<String>),
    Long(Box<LongGrant>),
}

#[derive(Debug, Clone)]
struct ParsedField {
    name: Located<String>,
    value: Option<YamlNode>,
    value_span: Option<SourceSpan>,
    span: SourceSpan,
}

impl ParsedField {
    fn reference(&self) -> FieldReference {
        FieldReference {
            name: self.name.clone(),
            span: self.span,
            value_span: self.value_span,
        }
    }
}

fn node_span(source_id: SourceId, node: &YamlNode) -> Option<SourceSpan> {
    let position = match node {
        YamlNode::Scalar(value) => value.byte_range(),
        YamlNode::Mapping(value) => value.byte_range(),
        YamlNode::Sequence(value) => value.byte_range(),
        YamlNode::Alias(_) | YamlNode::TaggedNode(_) => {
            let range = node.as_node()?.text_range();
            return Some(SourceSpan::from_valid_offsets(
                source_id,
                u32::from(range.start()) as usize,
                u32::from(range.end()) as usize,
            ));
        }
    };
    Some(span_from_position(source_id, position))
}

fn span_from_position(source_id: SourceId, position: yaml_edit::TextPosition) -> SourceSpan {
    SourceSpan::from_valid_offsets(source_id, position.start as usize, position.end as usize)
}

fn union(left: SourceSpan, right: SourceSpan) -> SourceSpan {
    SourceSpan::from_valid_offsets(
        left.source_id(),
        left.start().min(right.start()),
        left.end().max(right.end()),
    )
}
