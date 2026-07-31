use super::{effective_span, entry_span, selection_matches, service_entries, service_in_scope};
use crate::diagnostic::{Diagnostic, Severity};
use crate::merge::{MergedProject, MergedValue};
use crate::model::{Located, ShortPort, ShortVolumeMount};
use crate::profiles::ProfileSelection;
use crate::source::SourceSpan;
use std::fmt;

/// A configurable omission for which a semantic default can be requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefaultKind {
    /// The implicit project network definition.
    ImplicitNetwork,
    /// A service attachment to the implicit `default` network.
    ServiceNetwork,
    /// Port protocol, documented as `tcp`.
    PortProtocol,
    /// Port publication mode, documented as `ingress`.
    PortMode,
    /// Volume access mode, documented as read-write.
    VolumeReadOnly,
    /// Config target path.
    ConfigTarget,
    /// Config file mode, documented as `0444`.
    ConfigMode,
    /// Secret target name.
    SecretTarget,
    /// Secret file mode, documented as `0444`.
    SecretMode,
    /// Service restart policy, documented as `no`.
    RestartPolicy,
}

/// The project location where a default would apply.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DefaultLocation {
    /// A project-level implicit object.
    Project,
    /// An omitted service field.
    Service {
        /// Service name.
        service: String,
    },
    /// One item in a service sequence.
    ServiceItem {
        /// Service name.
        service: String,
        /// Field name.
        field: String,
        /// Zero-based merged item index.
        index: usize,
    },
}

/// A typed default value supplied by a policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefaultValue {
    /// A string-valued default.
    String(String),
    /// A boolean-valued default.
    Boolean(bool),
}

/// One explicit request made to a caller-owned default policy.
#[derive(Clone, PartialEq, Eq)]
pub struct DefaultRequest {
    kind: DefaultKind,
    location: DefaultLocation,
    source_name: Option<String>,
    anchor: SourceSpan,
    sensitive: bool,
}

impl DefaultRequest {
    /// Returns the omitted semantic field.
    #[must_use]
    pub const fn kind(&self) -> DefaultKind {
        self.kind
    }

    /// Returns where the default would apply.
    #[must_use]
    pub const fn location(&self) -> &DefaultLocation {
        &self.location
    }

    /// Returns the source resource name needed by target-path defaults.
    #[must_use]
    pub fn source_name(&self) -> Option<&str> {
        self.source_name.as_deref()
    }

    /// Returns a nearby source span for diagnostics and provenance.
    #[must_use]
    pub const fn anchor(&self) -> SourceSpan {
        self.anchor
    }

    /// Reports whether interpolation inserted sensitive content into the request.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

impl fmt::Debug for DefaultRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DefaultRequest")
            .field("kind", &self.kind)
            .field("location", &self.location)
            .field(
                "source_name",
                &if self.sensitive {
                    Some("<redacted>")
                } else {
                    self.source_name.as_deref()
                },
            )
            .field("anchor", &self.anchor)
            .field("sensitive", &self.sensitive)
            .finish()
    }
}

/// Supplies semantic defaults without granting access to ambient state.
pub trait DefaultProvider {
    /// Returns a value for one omission, or `None` to leave it unresolved.
    fn resolve(&self, request: &DefaultRequest) -> Option<DefaultValue>;
}

/// A policy that deliberately leaves every omission unresolved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoDefaults;

impl DefaultProvider for NoDefaults {
    fn resolve(&self, _request: &DefaultRequest) -> Option<DefaultValue> {
        None
    }
}

/// Container path platform used by the specification-oriented defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContainerPlatform {
    /// Linux container paths.
    Linux,
    /// Windows container paths.
    Windows,
}

/// The documented Compose Specification defaults covered by this release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComposeDefaults {
    platform: ContainerPlatform,
}

impl ComposeDefaults {
    /// Creates a specification-oriented provider for the target container platform.
    #[must_use]
    pub const fn new(platform: ContainerPlatform) -> Self {
        Self { platform }
    }

    /// Returns the target container platform.
    #[must_use]
    pub const fn platform(self) -> ContainerPlatform {
        self.platform
    }
}

impl DefaultProvider for ComposeDefaults {
    fn resolve(&self, request: &DefaultRequest) -> Option<DefaultValue> {
        match request.kind {
            DefaultKind::ImplicitNetwork | DefaultKind::ServiceNetwork => {
                Some(DefaultValue::String("default".to_owned()))
            }
            DefaultKind::PortProtocol => Some(DefaultValue::String("tcp".to_owned())),
            DefaultKind::PortMode => Some(DefaultValue::String("ingress".to_owned())),
            DefaultKind::VolumeReadOnly => Some(DefaultValue::Boolean(false)),
            DefaultKind::ConfigTarget => request.source_name.as_ref().map(|source| {
                DefaultValue::String(match self.platform {
                    ContainerPlatform::Linux => format!("/{source}"),
                    ContainerPlatform::Windows => format!(r"C:\{source}"),
                })
            }),
            DefaultKind::ConfigMode | DefaultKind::SecretMode => Some(DefaultValue::String("0444".to_owned())),
            DefaultKind::SecretTarget => request
                .source_name
                .as_ref()
                .map(|source| DefaultValue::String(source.clone())),
            DefaultKind::RestartPolicy => Some(DefaultValue::String("no".to_owned())),
        }
    }
}

/// One policy-approved default decision; the merged source remains unchanged.
#[derive(Clone, PartialEq, Eq)]
pub struct AppliedDefault {
    request: DefaultRequest,
    value: DefaultValue,
}

impl AppliedDefault {
    /// Returns the request that caused the decision.
    #[must_use]
    pub const fn request(&self) -> &DefaultRequest {
        &self.request
    }

    /// Returns the supplied default value.
    #[must_use]
    pub const fn value(&self) -> &DefaultValue {
        &self.value
    }
}

impl fmt::Debug for AppliedDefault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AppliedDefault")
            .field("request", &self.request)
            .field(
                "value",
                &if self.request.sensitive {
                    "<redacted>".to_owned()
                } else {
                    format!("{:?}", self.value)
                },
            )
            .finish()
    }
}

/// Non-destructive default decisions for one selected project view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultResolution {
    defaults: Vec<AppliedDefault>,
    diagnostics: Vec<Diagnostic>,
}

impl DefaultResolution {
    /// Returns policy-approved defaults in deterministic traversal order.
    #[must_use]
    pub fn defaults(&self) -> &[AppliedDefault] {
        &self.defaults
    }

    /// Returns resolution diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether default resolution emitted no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }
}

/// Requests defaults for omissions in active services without modifying the merged project.
#[must_use]
pub fn resolve_defaults(
    project: &MergedProject,
    selection: Option<&ProfileSelection>,
    provider: &dyn DefaultProvider,
) -> DefaultResolution {
    let mut diagnostics = Vec::new();
    if !selection_matches(project, selection, &mut diagnostics) {
        return DefaultResolution {
            defaults: Vec::new(),
            diagnostics,
        };
    }

    let mut defaults = Vec::new();
    let mut needs_implicit_network = false;
    for service in service_entries(project) {
        if !service_in_scope(selection, service.key()) {
            continue;
        }
        let anchor = entry_span(service);
        if service.value().get("restart").is_none() {
            request(
                provider,
                &mut defaults,
                DefaultKind::RestartPolicy,
                service_location(service.key()),
                None,
                anchor,
                false,
            );
        }
        if service.value().get("network_mode").is_none() && networks_empty(service.value().get("networks")) {
            needs_implicit_network = true;
            request(
                provider,
                &mut defaults,
                DefaultKind::ServiceNetwork,
                service_location(service.key()),
                None,
                anchor,
                false,
            );
        }
        collect_port_defaults(service.key(), service.value(), provider, &mut defaults);
        collect_volume_defaults(service.key(), service.value(), provider, &mut defaults);
        collect_grant_defaults(service.key(), service.value(), "configs", true, provider, &mut defaults);
        collect_grant_defaults(
            service.key(),
            service.value(),
            "secrets",
            false,
            provider,
            &mut defaults,
        );
    }

    let has_default_network = project
        .root()
        .get("networks")
        .and_then(MergedValue::as_mapping)
        .is_some_and(|entries| entries.iter().any(|entry| entry.key() == "default"));
    if needs_implicit_network && !has_default_network {
        request(
            provider,
            &mut defaults,
            DefaultKind::ImplicitNetwork,
            DefaultLocation::Project,
            None,
            effective_span(project.root()),
            false,
        );
    }

    DefaultResolution { defaults, diagnostics }
}

fn service_location(service: &str) -> DefaultLocation {
    DefaultLocation::Service {
        service: service.to_owned(),
    }
}

fn item_location(service: &str, field: &str, index: usize) -> DefaultLocation {
    DefaultLocation::ServiceItem {
        service: service.to_owned(),
        field: field.to_owned(),
        index,
    }
}

fn networks_empty(value: Option<&MergedValue>) -> bool {
    value.is_none_or(|value| {
        value.as_sequence().is_some_and(<[MergedValue]>::is_empty)
            || value.as_mapping().is_some_and(<[crate::merge::MergedEntry]>::is_empty)
    })
}

fn collect_port_defaults(
    service: &str,
    value: &MergedValue,
    provider: &dyn DefaultProvider,
    defaults: &mut Vec<AppliedDefault>,
) {
    let Some(ports) = value.get("ports").and_then(MergedValue::as_sequence) else {
        return;
    };
    for (index, port) in ports.iter().enumerate() {
        let anchor = effective_span(port);
        let protocol_missing = port.as_scalar().is_some_and(|scalar| {
            ShortPort::parse(Located::new(scalar.value().to_owned(), anchor))
                .protocol()
                .is_none()
        }) || port.as_mapping().is_some_and(|_| port.get("protocol").is_none());
        if protocol_missing {
            request(
                provider,
                defaults,
                DefaultKind::PortProtocol,
                item_location(service, "ports", index),
                None,
                anchor,
                false,
            );
        }
        if port.as_scalar().is_some() || port.as_mapping().is_some_and(|_| port.get("mode").is_none()) {
            request(
                provider,
                defaults,
                DefaultKind::PortMode,
                item_location(service, "ports", index),
                None,
                anchor,
                false,
            );
        }
    }
}

fn collect_volume_defaults(
    service: &str,
    value: &MergedValue,
    provider: &dyn DefaultProvider,
    defaults: &mut Vec<AppliedDefault>,
) {
    let Some(volumes) = value.get("volumes").and_then(MergedValue::as_sequence) else {
        return;
    };
    for (index, volume) in volumes.iter().enumerate() {
        let anchor = effective_span(volume);
        let missing = volume.as_scalar().is_some_and(|scalar| {
            let mount = ShortVolumeMount::new(Located::new(scalar.value().to_owned(), anchor));
            !mount
                .options()
                .iter()
                .any(|option| matches!(option.as_str(), "ro" | "rw"))
        }) || volume.as_mapping().is_some_and(|_| volume.get("read_only").is_none());
        if missing {
            request(
                provider,
                defaults,
                DefaultKind::VolumeReadOnly,
                item_location(service, "volumes", index),
                None,
                anchor,
                false,
            );
        }
    }
}

fn collect_grant_defaults(
    service: &str,
    value: &MergedValue,
    field: &str,
    config: bool,
    provider: &dyn DefaultProvider,
    defaults: &mut Vec<AppliedDefault>,
) {
    let Some(grants) = value.get(field).and_then(MergedValue::as_sequence) else {
        return;
    };
    for (index, grant) in grants.iter().enumerate() {
        let source = grant.as_scalar().map(|scalar| (scalar, true)).or_else(|| {
            grant
                .get("source")
                .and_then(MergedValue::as_scalar)
                .map(|scalar| (scalar, grant.get("target").is_none()))
        });
        let Some((source, target_missing)) = source else {
            continue;
        };
        let location = item_location(service, field, index);
        let anchor = effective_span(grant);
        if target_missing {
            request(
                provider,
                defaults,
                if config {
                    DefaultKind::ConfigTarget
                } else {
                    DefaultKind::SecretTarget
                },
                location.clone(),
                Some(source.value().to_owned()),
                anchor,
                source.is_sensitive(),
            );
        }
        if grant.as_scalar().is_some() || grant.get("mode").is_none() {
            request(
                provider,
                defaults,
                if config {
                    DefaultKind::ConfigMode
                } else {
                    DefaultKind::SecretMode
                },
                location,
                None,
                anchor,
                false,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn request(
    provider: &dyn DefaultProvider,
    defaults: &mut Vec<AppliedDefault>,
    kind: DefaultKind,
    location: DefaultLocation,
    source_name: Option<String>,
    anchor: SourceSpan,
    sensitive: bool,
) {
    let request = DefaultRequest {
        kind,
        location,
        source_name,
        anchor,
        sensitive,
    };
    if let Some(value) = provider.resolve(&request) {
        defaults.push(AppliedDefault { request, value });
    }
}
