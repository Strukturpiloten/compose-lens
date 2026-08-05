//! Source-aware native Compose document types.

mod command;
mod dependency;
mod environment;
mod host;
mod identity;
mod image;
mod network;
mod port;
mod resource;
mod sections;
mod ulimit;
mod value;
mod volume;

pub use command::Command;
pub use dependency::{
    DependencyCondition, DependsOn, Healthcheck, HealthcheckDuration, HealthcheckRetries, HealthcheckTest,
    HealthcheckTestKind, ServiceDependency,
};
pub use environment::{Environment, EnvironmentListEntry, EnvironmentMapEntry};
pub use host::{ExtraHostSeparator, ExtraHosts, HostAddress, HostAddressKind, LongExtraHost, ShortExtraHost};
pub use identity::{IdentityComponent, UserNamespaceMode, UserNamespaceModeKind, UserSpec};
pub use image::{ImageDigest, ImageReference};
pub use network::{Ipam, IpamConfig, NetworkDefinition, ServiceNetwork, ServiceNetworks};
pub use port::{LongPort, Port, ShortPort};
pub use resource::{ConfigDefinition, ConfigGrant, LongGrant, SecretDefinition, SecretGrant, VolumeDefinition};
pub use sections::{
    Build, BuildDefinition, BuildField, BuildFieldKind, DeployDefinition, DeployField, DeployFieldKind,
};
pub use ulimit::{LimitValue, Ulimit, UlimitRange, UlimitValue, Ulimits};
pub use value::{BooleanValue, ComposeScalar, KeyValueEntry, Labels};
pub use volume::{
    BindOptions, ContainerPath, ContainerPathKind, LongVolumeMount, MountType, SelinuxRelabel, ShortVolumeMount,
    VolumeMount, VolumeSyntax,
};

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::source::{SourceId, SourceSpan};
use crate::syntax::{SyntaxDocument, scalar_string_from_source};
use std::collections::{BTreeMap, BTreeSet};
use yaml_edit::{AnchorRegistry, AsYaml, Mapping, ScalarType, ScalarValue, YamlNode};

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

/// A short `extra_hosts` entry does not contain a hostname/address separator.
pub const EXTRA_HOST_INVALID_ENTRY: DiagnosticCode = DiagnosticCode::new("compose.extra-hosts.invalid-entry");

/// A service limit is neither unlimited, a non-negative integer, nor deferred.
pub const ULIMIT_INVALID_VALUE: DiagnosticCode = DiagnosticCode::new("compose.ulimits.invalid-value");

/// A health-check list has no valid command-mode token.
pub const HEALTHCHECK_INVALID_TEST: DiagnosticCode = DiagnosticCode::new("compose.healthcheck.invalid-test");

/// A health-check duration does not follow Compose duration syntax.
pub const HEALTHCHECK_INVALID_DURATION: DiagnosticCode = DiagnosticCode::new("compose.healthcheck.invalid-duration");

/// A health-check retry count is not a non-negative integer or deferred expression.
pub const HEALTHCHECK_INVALID_RETRIES: DiagnosticCode = DiagnosticCode::new("compose.healthcheck.invalid-retries");

/// A long dependency uses an unrecognized condition.
pub const DEPENDENCY_INVALID_CONDITION: DiagnosticCode = DiagnosticCode::new("compose.dependencies.invalid-condition");

/// A typed dependency names a service missing from the same document.
pub const DEPENDENCY_MISSING_SERVICE: DiagnosticCode = DiagnosticCode::new("compose.dependencies.missing-service");

/// A `service_healthy` dependency has no enabled health check.
pub const DEPENDENCY_MISSING_HEALTHCHECK: DiagnosticCode =
    DiagnosticCode::new("compose.dependencies.missing-healthcheck");

/// A `service_healthy` dependency may rely on health metadata from its image.
pub const DEPENDENCY_HEALTHCHECK_UNVERIFIED: DiagnosticCode =
    DiagnosticCode::new("compose.dependencies.healthcheck-unverified");

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

/// A source-aware typed Compose service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    name: Located<String>,
    span: SourceSpan,
    image: Option<Located<ImageReference>>,
    command: Option<Command>,
    environment: Option<Environment>,
    labels: Option<Labels>,
    extra_hosts: Option<ExtraHosts>,
    user: Option<UserSpec>,
    userns_mode: Option<UserNamespaceMode>,
    group_add: Vec<Located<String>>,
    working_dir: Option<Located<String>>,
    read_only: Option<Located<BooleanValue>>,
    ulimits: Option<Ulimits>,
    depends_on: Option<DependsOn>,
    healthcheck: Option<Healthcheck>,
    build: Option<Build>,
    deploy: Option<DeployDefinition>,
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
    fn new(name: Located<String>, span: SourceSpan) -> Self {
        Self {
            name,
            span,
            image: None,
            command: None,
            environment: None,
            labels: None,
            extra_hosts: None,
            user: None,
            userns_mode: None,
            group_add: Vec::new(),
            working_dir: None,
            read_only: None,
            ulimits: None,
            depends_on: None,
            healthcheck: None,
            build: None,
            deploy: None,
            ports: Vec::new(),
            volumes: Vec::new(),
            networks: None,
            profiles: Vec::new(),
            configs: Vec::new(),
            secrets: Vec::new(),
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

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

    /// Returns service metadata labels with list and mapping forms kept distinct.
    #[must_use]
    pub const fn labels(&self) -> Option<&Labels> {
        self.labels.as_ref()
    }

    /// Returns additional host mappings with short and long forms retained.
    #[must_use]
    pub const fn extra_hosts(&self) -> Option<&ExtraHosts> {
        self.extra_hosts.as_ref()
    }

    /// Returns the raw-preserving container user/group value.
    #[must_use]
    pub const fn user(&self) -> Option<&UserSpec> {
        self.user.as_ref()
    }

    /// Returns the raw-preserving user-namespace mode.
    #[must_use]
    pub const fn userns_mode(&self) -> Option<&UserNamespaceMode> {
        self.userns_mode.as_ref()
    }

    /// Returns supplementary groups in authored order without resolving names or IDs.
    #[must_use]
    pub fn group_add(&self) -> &[Located<String>] {
        &self.group_add
    }

    /// Returns the container working-directory override.
    #[must_use]
    pub const fn working_dir(&self) -> Option<&Located<String>> {
        self.working_dir.as_ref()
    }

    /// Returns the explicit read-only root-filesystem choice.
    #[must_use]
    pub const fn read_only(&self) -> Option<&Located<BooleanValue>> {
        self.read_only.as_ref()
    }

    /// Returns explicitly authored service resource limits.
    #[must_use]
    pub const fn ulimits(&self) -> Option<&Ulimits> {
        self.ulimits.as_ref()
    }

    /// Returns service dependencies with short and long forms retained.
    #[must_use]
    pub const fn depends_on(&self) -> Option<&DependsOn> {
        self.depends_on.as_ref()
    }

    /// Returns the service health-check definition.
    #[must_use]
    pub const fn healthcheck(&self) -> Option<&Healthcheck> {
        self.healthcheck.as_ref()
    }

    /// Returns the build declaration with short and long forms retained.
    #[must_use]
    pub const fn build(&self) -> Option<&Build> {
        self.build.as_ref()
    }

    /// Returns independently classified deploy subfields.
    #[must_use]
    pub const fn deploy(&self) -> Option<&DeployDefinition> {
        self.deploy.as_ref()
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

/// A source-aware native Compose document.
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

    /// Validates dependency targets and `service_healthy` health-check requirements in this document.
    ///
    /// Multi-file callers should validate the merged project view through
    /// [`crate::resolution::validate_references`] instead.
    #[must_use]
    pub fn validate_dependencies(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for service in &self.services {
            let Some(depends_on) = service.depends_on() else {
                continue;
            };
            match depends_on {
                DependsOn::Short { services, .. } => {
                    for target in services {
                        if self.service(target.value()).is_none() {
                            diagnostics.push(missing_dependency_diagnostic(target.span(), false, true));
                        }
                    }
                }
                DependsOn::Long { services, .. } => {
                    for dependency in services {
                        let required = !matches!(
                            dependency.required().map(Located::value),
                            Some(BooleanValue::Literal(false))
                        );
                        let Some(target) = self.service(dependency.service().value()) else {
                            diagnostics.push(missing_dependency_diagnostic(
                                dependency.service().span(),
                                false,
                                required,
                            ));
                            continue;
                        };
                        let needs_healthcheck = matches!(
                            dependency.condition().map(Located::value),
                            Some(DependencyCondition::ServiceHealthy)
                        );
                        if needs_healthcheck && target.healthcheck().is_none() {
                            let span = dependency
                                .condition()
                                .map_or_else(|| dependency.service().span(), Located::span);
                            diagnostics.push(unverified_healthcheck_diagnostic(span));
                        } else if needs_healthcheck && target.healthcheck().is_some_and(Healthcheck::is_disabled) {
                            let span = dependency
                                .condition()
                                .map_or_else(|| dependency.service().span(), Located::span);
                            diagnostics.push(missing_dependency_diagnostic(span, true, required));
                        }
                    }
                }
            }
        }
        diagnostics
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

fn missing_dependency_diagnostic(span: SourceSpan, healthcheck: bool, required: bool) -> Diagnostic {
    let severity = if required { Severity::Error } else { Severity::Warning };
    if healthcheck {
        Diagnostic::new(
            DEPENDENCY_MISSING_HEALTHCHECK,
            severity,
            if required {
                "service_healthy dependency requires an enabled health check"
            } else {
                "optional service_healthy dependency has no enabled health check"
            },
        )
        .with_label(DiagnosticLabel::primary(span, "dependency cannot become healthy"))
    } else {
        Diagnostic::new(
            DEPENDENCY_MISSING_SERVICE,
            severity,
            if required {
                "service dependency is not declared in this Compose document"
            } else {
                "optional service dependency is not declared in this Compose document"
            },
        )
        .with_label(DiagnosticLabel::primary(span, "missing dependency service"))
    }
}

fn unverified_healthcheck_diagnostic(span: SourceSpan) -> Diagnostic {
    Diagnostic::new(
        DEPENDENCY_HEALTHCHECK_UNVERIFIED,
        Severity::Warning,
        "service_healthy dependency has no Compose healthcheck to validate",
    )
    .with_label(DiagnosticLabel::primary(span, "image health metadata is not available"))
    .with_note("the dependency image may still define a health check; verify it at build or runtime")
}

#[derive(Debug)]
struct Parser {
    source_id: SourceId,
    source_span: SourceSpan,
    source: String,
    tree: yaml_edit::YamlFile,
    anchors: AnchorRegistry,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    fn new(syntax: &SyntaxDocument) -> Self {
        let tree = syntax.yaml_file();
        let anchors = tree
            .document()
            .map_or_else(AnchorRegistry::new, |document| AnchorRegistry::from_document(&document));
        Self {
            source_id: syntax.source_id(),
            source_span: syntax.source_span(),
            source: syntax.source_text().to_owned(),
            tree,
            anchors,
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
        let mut service = Service::new(field.name.clone(), field.span);
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
                "labels" if !duplicate => {
                    service.labels = self.parse_labels(&service_field);
                }
                "extra_hosts" if !duplicate => {
                    service.extra_hosts = self.parse_extra_hosts(&service_field);
                }
                "user" if !duplicate => {
                    service.user = self.parse_string(&service_field, "service user").map(UserSpec::parse);
                }
                "userns_mode" if !duplicate => {
                    service.userns_mode = self
                        .parse_string(&service_field, "service user namespace mode")
                        .map(UserNamespaceMode::parse);
                }
                "group_add" if !duplicate => {
                    service.group_add = self.parse_string_sequence(&service_field, "service supplementary groups");
                }
                "working_dir" if !duplicate => {
                    service.working_dir = self.parse_string(&service_field, "service working directory");
                }
                "read_only" if !duplicate => {
                    service.read_only = self.parse_boolean(&service_field, "service read_only");
                }
                "ulimits" if !duplicate => {
                    service.ulimits = self.parse_ulimits(&service_field);
                }
                "depends_on" if !duplicate => {
                    service.depends_on = self.parse_depends_on(&service_field);
                }
                "healthcheck" if !duplicate => {
                    service.healthcheck = self.parse_healthcheck(&service_field);
                }
                "build" if !duplicate => {
                    service.build = self.parse_build(&service_field);
                }
                "deploy" if !duplicate => {
                    service.deploy = self.parse_deploy(&service_field);
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
                    Some(Command::String(Located::new(
                        scalar_string_from_source(&self.source, scalar),
                        span,
                    )))
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

    fn parse_extra_hosts(&mut self, field: &ParsedField) -> Option<ExtraHosts> {
        match field.value.as_ref() {
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let entries = self
                    .parse_scalar_nodes(sequence.values(), field.span, "extra_hosts entries must be scalars")
                    .into_iter()
                    .map(|raw| {
                        let entry = ShortExtraHost::parse(raw);
                        if !entry.is_complete() {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    EXTRA_HOST_INVALID_ENTRY,
                                    Severity::Error,
                                    "short extra_hosts entry must contain a hostname and address",
                                )
                                .with_label(DiagnosticLabel::primary(
                                    entry.raw().span(),
                                    "missing separator or value",
                                )),
                            );
                        }
                        entry
                    })
                    .collect();
                Some(ExtraHosts::Short { span, entries })
            }
            Some(YamlNode::Mapping(mapping)) => {
                let span = span_from_position(self.source_id, mapping.byte_range());
                let mut entries = Vec::new();
                let mut seen = BTreeMap::new();
                for host in self.fields(mapping) {
                    if self.record_duplicate(&mut seen, &host) {
                        continue;
                    }
                    if let Some(address) = self.parse_string(&host, "extra host address") {
                        let address = Located::new(HostAddress::parse(address.value), address.span);
                        entries.push(LongExtraHost::new(host.name, address, host.span));
                    }
                }
                Some(ExtraHosts::Long { span, entries })
            }
            _ => {
                self.expected(EXPECTED_FIELD_FORM, field, "extra_hosts must be a sequence or mapping");
                None
            }
        }
    }

    fn parse_ulimits(&mut self, field: &ParsedField) -> Option<Ulimits> {
        let Some(mapping) = field.value.as_ref().and_then(YamlNode::as_mapping) else {
            self.expected(EXPECTED_MAPPING, field, "ulimits must be a mapping");
            return None;
        };
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut entries = Vec::new();
        let mut seen = BTreeMap::new();
        for limit in self.fields(mapping) {
            if self.record_duplicate(&mut seen, &limit) {
                continue;
            }
            let value = match limit.value.as_ref() {
                Some(YamlNode::Scalar(_)) => self.parse_limit_value(&limit, "ulimit value").map(UlimitValue::Single),
                Some(YamlNode::Mapping(range)) => Some(UlimitValue::Range(self.parse_ulimit_range(range))),
                _ => {
                    self.expected(
                        EXPECTED_FIELD_FORM,
                        &limit,
                        "ulimit must be a scalar or soft/hard mapping",
                    );
                    None
                }
            };
            if let Some(value) = value {
                entries.push(Ulimit::new(limit.name, limit.span, value));
            }
        }
        Some(Ulimits::new(span, entries))
    }

    fn parse_ulimit_range(&mut self, mapping: &Mapping) -> UlimitRange {
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut range = UlimitRange::new(span);
        let mut seen = BTreeMap::new();
        for field in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &field);
            match field.name.value.as_str() {
                "soft" if !duplicate => self
                    .parse_limit_value(&field, "ulimit soft value")
                    .into_iter()
                    .for_each(|value| range.set_soft(value)),
                "hard" if !duplicate => self
                    .parse_limit_value(&field, "ulimit hard value")
                    .into_iter()
                    .for_each(|value| range.set_hard(value)),
                name if name.starts_with("x-") => range.push_extension(field.reference()),
                _ if duplicate => {}
                _ => range.push_unknown(field.reference()),
            }
        }
        range
    }

    fn parse_limit_value(&mut self, field: &ParsedField, description: &str) -> Option<Located<LimitValue>> {
        let value = self.parse_string(field, description)?;
        let parsed = LimitValue::parse(value.value);
        if !parsed.is_valid() {
            self.diagnostics.push(
                Diagnostic::new(
                    ULIMIT_INVALID_VALUE,
                    Severity::Error,
                    "ulimit must be -1, a non-negative integer, or an interpolation expression",
                )
                .with_label(DiagnosticLabel::primary(value.span, "invalid ulimit value")),
            );
        }
        Some(Located::new(parsed, value.span))
    }

    fn parse_depends_on(&mut self, field: &ParsedField) -> Option<DependsOn> {
        match field.value.as_ref() {
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let services = self.parse_scalar_nodes(
                    sequence.values(),
                    field.span,
                    "dependency service names must be scalars",
                );
                Some(DependsOn::Short { span, services })
            }
            Some(YamlNode::Mapping(mapping)) => {
                let span = span_from_position(self.source_id, mapping.byte_range());
                let mut services = Vec::new();
                let mut seen = BTreeMap::new();
                for dependency in self.fields(mapping) {
                    if self.record_duplicate(&mut seen, &dependency) {
                        continue;
                    }
                    let mut parsed = ServiceDependency::new(dependency.name.clone(), dependency.span);
                    if Self::field_is_null(&dependency) {
                        services.push(parsed);
                        continue;
                    }
                    let Some(options) = dependency.value.as_ref().and_then(YamlNode::as_mapping) else {
                        self.expected(
                            EXPECTED_MAPPING,
                            &dependency,
                            "long dependency options must be a mapping or null",
                        );
                        continue;
                    };
                    let mut option_seen = BTreeMap::new();
                    for option in self.fields(options) {
                        let duplicate = self.record_duplicate(&mut option_seen, &option);
                        match option.name.value.as_str() {
                            "condition" if !duplicate => {
                                if let Some(value) = self.parse_string(&option, "dependency condition") {
                                    let condition = DependencyCondition::parse(value.value);
                                    if !condition.is_known() {
                                        self.diagnostics.push(
                                            Diagnostic::new(
                                                DEPENDENCY_INVALID_CONDITION,
                                                Severity::Error,
                                                "dependency condition is not defined by Compose",
                                            )
                                            .with_label(
                                                DiagnosticLabel::primary(value.span, "unknown dependency condition"),
                                            ),
                                        );
                                    }
                                    parsed.set_condition(Located::new(condition, value.span));
                                }
                            }
                            "restart" if !duplicate => self
                                .parse_boolean(&option, "dependency restart")
                                .into_iter()
                                .for_each(|value| parsed.set_restart(value)),
                            "required" if !duplicate => self
                                .parse_boolean(&option, "dependency required")
                                .into_iter()
                                .for_each(|value| parsed.set_required(value)),
                            name if name.starts_with("x-") => parsed.push_extension(option.reference()),
                            _ if duplicate => {}
                            _ => parsed.push_unknown(option.reference()),
                        }
                    }
                    services.push(parsed);
                }
                Some(DependsOn::Long { span, services })
            }
            _ => {
                self.expected(EXPECTED_FIELD_FORM, field, "depends_on must be a sequence or mapping");
                None
            }
        }
    }

    fn parse_healthcheck(&mut self, field: &ParsedField) -> Option<Healthcheck> {
        let Some(mapping) = field.value.as_ref().and_then(YamlNode::as_mapping) else {
            self.expected(EXPECTED_MAPPING, field, "healthcheck must be a mapping");
            return None;
        };
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut healthcheck = Healthcheck::new(span);
        let mut seen = BTreeMap::new();
        for option in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &option);
            match option.name.value.as_str() {
                "test" if !duplicate => self
                    .parse_healthcheck_test(&option)
                    .into_iter()
                    .for_each(|value| healthcheck.set_test(value)),
                "interval" if !duplicate => self
                    .parse_healthcheck_duration(&option, "healthcheck interval")
                    .into_iter()
                    .for_each(|value| healthcheck.set_interval(value)),
                "timeout" if !duplicate => self
                    .parse_healthcheck_duration(&option, "healthcheck timeout")
                    .into_iter()
                    .for_each(|value| healthcheck.set_timeout(value)),
                "retries" if !duplicate => self
                    .parse_healthcheck_retries(&option)
                    .into_iter()
                    .for_each(|value| healthcheck.set_retries(value)),
                "start_period" if !duplicate => self
                    .parse_healthcheck_duration(&option, "healthcheck start period")
                    .into_iter()
                    .for_each(|value| healthcheck.set_start_period(value)),
                "start_interval" if !duplicate => self
                    .parse_healthcheck_duration(&option, "healthcheck start interval")
                    .into_iter()
                    .for_each(|value| healthcheck.set_start_interval(value)),
                "disable" if !duplicate => self
                    .parse_boolean(&option, "healthcheck disable")
                    .into_iter()
                    .for_each(|value| healthcheck.set_disable(value)),
                name if name.starts_with("x-") => healthcheck.push_extension(option.reference()),
                _ if duplicate => {}
                _ => healthcheck.push_unknown(option.reference()),
            }
        }
        Some(healthcheck)
    }

    fn parse_healthcheck_duration(
        &mut self,
        field: &ParsedField,
        description: &str,
    ) -> Option<Located<HealthcheckDuration>> {
        let value = self.parse_string(field, description)?;
        let duration = HealthcheckDuration::parse(value.value);
        if !duration.is_valid() {
            self.diagnostics.push(
                Diagnostic::new(
                    HEALTHCHECK_INVALID_DURATION,
                    Severity::Error,
                    "healthcheck duration must use Compose duration syntax or interpolation",
                )
                .with_label(DiagnosticLabel::primary(value.span, "invalid healthcheck duration")),
            );
        }
        Some(Located::new(duration, value.span))
    }

    fn parse_healthcheck_retries(&mut self, field: &ParsedField) -> Option<Located<HealthcheckRetries>> {
        let value = self.parse_string(field, "healthcheck retries")?;
        let retries = HealthcheckRetries::parse(value.value);
        if !retries.is_valid() {
            self.diagnostics.push(
                Diagnostic::new(
                    HEALTHCHECK_INVALID_RETRIES,
                    Severity::Error,
                    "healthcheck retries must be a non-negative integer or interpolation expression",
                )
                .with_label(DiagnosticLabel::primary(value.span, "invalid healthcheck retry count")),
            );
        }
        Some(Located::new(retries, value.span))
    }

    fn parse_healthcheck_test(&mut self, field: &ParsedField) -> Option<HealthcheckTest> {
        match field.value.as_ref() {
            Some(YamlNode::Scalar(_)) => self
                .parse_string(field, "healthcheck test")
                .map(HealthcheckTest::String),
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let values =
                    self.parse_scalar_nodes(sequence.values(), field.span, "healthcheck test items must be scalars");
                let kind = values.first().map(|value| HealthcheckTestKind::parse(value.value()));
                if kind.is_none()
                    || kind == Some(HealthcheckTestKind::Other)
                    || (kind == Some(HealthcheckTestKind::None) && values.len() != 1)
                {
                    self.diagnostics.push(
                        Diagnostic::new(
                            HEALTHCHECK_INVALID_TEST,
                            Severity::Error,
                            "healthcheck list must begin with NONE, CMD, or CMD-SHELL",
                        )
                        .with_label(DiagnosticLabel::primary(span, "invalid healthcheck command mode")),
                    );
                }
                Some(HealthcheckTest::List { span, kind, values })
            }
            _ => {
                self.expected(
                    EXPECTED_FIELD_FORM,
                    field,
                    "healthcheck test must be a scalar or sequence",
                );
                None
            }
        }
    }

    fn parse_build(&mut self, field: &ParsedField) -> Option<Build> {
        match field.value.as_ref() {
            Some(YamlNode::Scalar(_)) => self.parse_string(field, "build context").map(Build::Context),
            Some(YamlNode::Mapping(mapping)) => {
                let span = span_from_position(self.source_id, mapping.byte_range());
                let mut definition = BuildDefinition::new(span);
                let mut seen = BTreeMap::new();
                for option in self.fields(mapping) {
                    let duplicate = self.record_duplicate(&mut seen, &option);
                    if duplicate {
                        continue;
                    }
                    if let Some(kind) = BuildFieldKind::from_name(option.name.value()) {
                        definition.push_field(BuildField::new(kind, option.reference()));
                    } else if option.name.value().starts_with("x-") {
                        definition.push_extension(option.reference());
                    } else {
                        definition.push_unknown(option.reference());
                    }
                }
                Some(Build::Definition(definition))
            }
            _ => {
                self.expected(EXPECTED_FIELD_FORM, field, "build must be a scalar context or mapping");
                None
            }
        }
    }

    fn parse_deploy(&mut self, field: &ParsedField) -> Option<DeployDefinition> {
        let Some(mapping) = field.value.as_ref().and_then(YamlNode::as_mapping) else {
            self.expected(EXPECTED_MAPPING, field, "deploy must be a mapping");
            return None;
        };
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut definition = DeployDefinition::new(span);
        let mut seen = BTreeMap::new();
        for option in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &option);
            if duplicate {
                continue;
            }
            if let Some(kind) = DeployFieldKind::from_name(option.name.value()) {
                definition.push_field(DeployField::new(kind, option.reference()));
            } else if option.name.value().starts_with("x-") {
                definition.push_extension(option.reference());
            } else {
                definition.push_unknown(option.reference());
            }
        }
        Some(definition)
    }

    fn source_column(&self, offset: usize) -> usize {
        let prefix = self.source.get(..offset).unwrap_or_default();
        let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
        self.source[line_start..offset].chars().count()
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
                    ports.push(Port::Short(ShortPort::parse(Located::new(
                        scalar_string_from_source(&self.source, &scalar),
                        span,
                    ))));
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
                    grants.push(ParsedGrant::Short(Located::new(
                        scalar_string_from_source(&self.source, &scalar),
                        span,
                    )));
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
                    let raw = Located::new(scalar_string_from_source(&self.source, &scalar), span);
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
            scalar_string_from_source(&self.source, scalar),
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
        let value = scalar_string_from_source(&self.source, scalar);
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
            values.push(Located::new(scalar_string_from_source(&self.source, &scalar), span));
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
            ScalarType::Integer | ScalarType::Float => {
                ComposeScalar::Number(scalar_string_from_source(&self.source, scalar))
            }
            ScalarType::String | ScalarType::Timestamp | ScalarType::Regex => {
                ComposeScalar::String(scalar_string_from_source(&self.source, scalar))
            }
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
        let mut fields = self.flatten_empty_value_continuations(fields);
        for field in &mut fields {
            field.value = field.value.take().map(|value| self.resolve_alias(value));
        }
        fields
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
                    name: Located::new(scalar_string_from_source(&self.source, scalar), name_span),
                    value,
                    value_span,
                    span,
                })
            })
            .collect()
    }

    fn resolve_alias(&self, node: YamlNode) -> YamlNode {
        let mut node = node;
        let mut visited = BTreeSet::new();
        for _ in 0..64 {
            let YamlNode::Alias(alias) = &node else {
                return node;
            };
            if !visited.insert(alias.name()) {
                return node;
            }
            let Some(target) = self.anchors.resolve(&alias.name()).and_then(|target| {
                YamlNode::from_syntax(target.clone()).or_else(|| target.children().find_map(YamlNode::from_syntax))
            }) else {
                return node;
            };
            node = target;
        }
        node
    }

    fn flatten_empty_value_continuations(&mut self, fields: Vec<ParsedField>) -> Vec<ParsedField> {
        let Some(target_column) = fields.first().map(|field| self.source_column(field.name.span.start())) else {
            return fields;
        };
        self.recover_fields(fields, target_column)
    }

    fn recover_fields(&mut self, fields: Vec<ParsedField>, target_column: usize) -> Vec<ParsedField> {
        let mut flattened = Vec::new();
        for mut field in fields {
            let field_column = self.source_column(field.name.span.start());
            let nested_mapping = field.value.as_ref().and_then(YamlNode::as_mapping).cloned();
            let continuation = nested_mapping.as_ref().is_some_and(|mapping| {
                !self.is_flow_mapping(mapping)
                    && mapping
                        .entries()
                        .find_map(|entry| {
                            let key = entry.key_node()?;
                            let scalar = key.as_scalar()?;
                            Some(scalar.byte_range().start as usize)
                        })
                        .is_some_and(|key_start| self.source_column(key_start) <= field_column)
            });

            if continuation {
                field.value = None;
                field.value_span = None;
                field.span = field.name.span;
            }
            if field_column == target_column {
                flattened.push(field);
            }
            if let Some(mapping) = nested_mapping.filter(|mapping| !self.is_flow_mapping(mapping)) {
                let nested = self.raw_fields(&mapping);
                flattened.extend(self.recover_fields(nested, target_column));
            }
        }
        flattened
    }

    fn is_flow_mapping(&self, mapping: &Mapping) -> bool {
        let position = mapping.byte_range();
        self.source
            .get(position.start as usize..position.end as usize)
            .is_some_and(|text| text.trim_start().starts_with('{'))
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
