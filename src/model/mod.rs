//! Source-aware native Compose document types.

mod annotation;
mod build_extra_host;
mod capability;
mod command;
mod dependency;
mod device;
mod dns;
mod dns_option;
mod dns_search;
mod entrypoint;
mod environment;
mod expose;
mod host;
mod hostname;
mod identity;
mod image;
mod lifecycle;
mod logging;
mod memory;
mod network;
mod pids;
mod port;
mod pull;
mod resource;
mod restart;
mod sections;
mod security_option;
mod shm;
mod sysctl;
mod tmpfs;
mod ulimit;
mod value;
mod volume;

pub use annotation::{Annotations, AnnotationsForm};
pub use build_extra_host::{BuildExtraHostAddresses, BuildExtraHostEntry, BuildExtraHosts};
pub use capability::{CapabilityAdd, CapabilityAddItem, CapabilityDrop, CapabilityDropItem};
pub use command::Command;
pub use dependency::{
    DependencyCondition, DependsOn, Healthcheck, HealthcheckDuration, HealthcheckRetries, HealthcheckTest,
    HealthcheckTestKind, ServiceDependency,
};
pub(crate) use device::valid_generated_device_string;
pub use device::{Device, Devices, LongDevice, ShortDevice, ShortDeviceKind};
pub use dns::{Dns, DnsForm};
pub use dns_option::DnsOptions;
pub use dns_search::{DnsSearch, DnsSearchForm};
pub use entrypoint::Entrypoint;
pub use environment::{
    Environment, EnvironmentFile, EnvironmentFileFormat, EnvironmentFileFormatKind, EnvironmentListEntry,
    EnvironmentMapEntry, LongEnvironmentFile,
};
pub use expose::{Expose, ExposeItem, ExposeItemKind, ExposePort, ExposeProtocol, ExposeScalarKind};
pub(crate) use expose::{classify_expose_item, valid_generated_expose_item};
pub use host::{ExtraHostSeparator, ExtraHosts, HostAddress, HostAddressKind, LongExtraHost, ShortExtraHost};
pub(crate) use hostname::valid_hostname;
pub use hostname::{Hostname, HostnameKind};
pub use identity::{IdentityComponent, UserNamespaceMode, UserNamespaceModeKind, UserSpec};
pub use image::{ImageDigest, ImageReference};
pub use lifecycle::StopGracePeriod;
pub use logging::{Logging, LoggingOption, LoggingOptionValue, LoggingOptions};
pub(crate) use memory::valid_generated_mem_amount;
pub use memory::{MemLimit, MemLimitKind, MemLimitScalarKind, MemLimitUnit};
pub use network::{Ipam, IpamConfig, NetworkDefinition, ServiceNetwork, ServiceNetworks};
pub(crate) use pids::valid_positive_pids_decimal;
pub use pids::{PidsLimit, PidsLimitKind};
pub use port::{LongPort, Port, ShortPort};
pub(crate) use pull::valid_pull_policy_duration;
pub use pull::{PullPolicy, PullPolicyKind};
pub use resource::{ConfigDefinition, ConfigGrant, LongGrant, SecretDefinition, SecretGrant, VolumeDefinition};
pub use restart::{RestartPolicy, RestartPolicyKind};
pub use sections::{
    Build, BuildAdditionalContexts, BuildArgs, BuildDefinition, BuildField, BuildFieldKind, BuildNoCacheFilter,
    BuildSsh, BuildSshForm, DeployDefinition, DeployEndpointMode, DeployField, DeployFieldKind, DeployMode,
    DeployPlacement, DeployPlacementMaxReplicasPerNode, DeployPlacementPreference, DeployReplicas,
    DeployRestartCondition, DeployRestartDuration, DeployRestartMaxAttempts, DeployRestartPolicy,
};
pub(crate) use security_option::{SecurityOptionCandidateCounts, classify_security_option};
pub use security_option::{SecurityOptionItem, SecurityOptionKind, SecurityOptions};
pub(crate) use shm::valid_generated_shm_amount;
pub use shm::{ShmSize, ShmSizeKind, ShmSizeScalarKind, ShmSizeUnit};
pub use sysctl::{Sysctls, SysctlsForm};
pub(crate) use tmpfs::valid_generated_tmpfs_item;
pub use tmpfs::{Tmpfs, TmpfsForm, TmpfsItem, TmpfsItemKind};
pub(crate) use ulimit::valid_ulimit_name;
pub use ulimit::{LimitValue, Ulimit, UlimitRange, UlimitValue, Ulimits};
pub use value::{BooleanValue, BuildNoCache, BuildProvenance, BuildSbom, ComposeScalar, KeyValueEntry, Labels};
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
/// A deploy endpoint mode is retained but is outside Compose's documented portable values.
pub const DEPLOY_ENDPOINT_MODE_PORTABILITY: DiagnosticCode =
    DiagnosticCode::new("compose.deploy.endpoint-mode.portability");
/// A deploy mode is retained but is outside Compose's documented portable values.
pub const DEPLOY_MODE_PORTABILITY: DiagnosticCode = DiagnosticCode::new("compose.deploy.mode.portability");
/// A Build no-cache filter list repeats an exact stage name.
pub const BUILD_NO_CACHE_FILTER_DUPLICATE_ITEM: DiagnosticCode =
    DiagnosticCode::new("compose.build.no-cache-filter.duplicate-item");

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

/// A build `dockerfile` must be a non-empty scalar.
pub const BUILD_DOCKERFILE_EXPECTED_NON_EMPTY: DiagnosticCode =
    DiagnosticCode::new("compose.build.dockerfile.expected-non-empty-scalar");

/// A build definition declares both `dockerfile` and `dockerfile_inline`.
pub const BUILD_DOCKERFILE_INLINE_CONFLICT: DiagnosticCode =
    DiagnosticCode::new("compose.build.dockerfile-inline-conflict");

/// A build `no_cache` value is neither a YAML boolean nor a YAML string scalar.
pub const BUILD_NO_CACHE_EXPECTED_BOOLEAN_OR_STRING: DiagnosticCode =
    DiagnosticCode::new("compose.build.no-cache.expected-boolean-or-string");

/// A build `sbom` value is neither a YAML boolean nor a YAML string scalar.
pub const BUILD_SBOM_EXPECTED_BOOLEAN_OR_STRING: DiagnosticCode =
    DiagnosticCode::new("compose.build.sbom.expected-boolean-or-string");

/// A build `isolation` value is not a YAML string scalar.
pub const BUILD_ISOLATION_EXPECTED_STRING: DiagnosticCode =
    DiagnosticCode::new("compose.build.isolation.expected-string");

/// Build `extra_hosts` has neither list nor mapping syntax.
pub const BUILD_EXTRA_HOSTS_EXPECTED_FORM: DiagnosticCode =
    DiagnosticCode::new("compose.build.extra-hosts.expected-form");

/// A build `extra_hosts` list item or address is not a YAML string scalar.
pub const BUILD_EXTRA_HOSTS_EXPECTED_STRING: DiagnosticCode =
    DiagnosticCode::new("compose.build.extra-hosts.expected-string");

/// A build `extra_hosts` list repeats a schema-unique raw entry.
pub const BUILD_EXTRA_HOSTS_DUPLICATE_ITEM: DiagnosticCode =
    DiagnosticCode::new("compose.build.extra-hosts.duplicate-item");

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

/// An external volume also configures a local driver or driver options.
///
/// Both authored values remain available for diagnosis; `ComposeLens` does not silently select one.
pub const VOLUME_EXTERNAL_DRIVER_CONFIGURATION: DiagnosticCode =
    DiagnosticCode::new("compose.volume.external-driver-configuration");

/// An external volume also configures labels.
///
/// The authored labels remain available for diagnosis; `ComposeLens` does not silently discard
/// them or repurpose the driver-configuration diagnostic.
pub const VOLUME_EXTERNAL_LABELS_CONFIGURATION: DiagnosticCode =
    DiagnosticCode::new("compose.volume.external-labels-configuration");

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

/// A service limit name is outside Compose's portable lowercase-name grammar.
pub const ULIMIT_INVALID_NAME: DiagnosticCode = DiagnosticCode::new("compose.ulimits.invalid-name");

/// A service limit range is missing its required `soft` or `hard` member.
pub const ULIMIT_MISSING_RANGE_MEMBER: DiagnosticCode = DiagnosticCode::new("compose.ulimits.missing-range-member");

/// A health-check list has no valid command-mode token.
pub const HEALTHCHECK_INVALID_TEST: DiagnosticCode = DiagnosticCode::new("compose.healthcheck.invalid-test");

/// A health-check duration does not follow Compose duration syntax.
pub const HEALTHCHECK_INVALID_DURATION: DiagnosticCode = DiagnosticCode::new("compose.healthcheck.invalid-duration");

/// A health-check retry count is not a non-negative integer or deferred expression.
pub const HEALTHCHECK_INVALID_RETRIES: DiagnosticCode = DiagnosticCode::new("compose.healthcheck.invalid-retries");

/// A service-level restart policy is not one of the Compose-defined forms or an expression.
pub const RESTART_INVALID_POLICY: DiagnosticCode = DiagnosticCode::new("compose.restart.invalid-policy");

/// A service hostname is not authored as a YAML string scalar.
pub const HOSTNAME_EXPECTED_STRING: DiagnosticCode = DiagnosticCode::new("compose.hostname.expected-string");

/// A resolved service hostname does not satisfy the conservative RFC-1123 grammar.
pub const HOSTNAME_INVALID: DiagnosticCode = DiagnosticCode::new("compose.hostname.invalid-value");

/// A service PID limit is not a number or string scalar.
pub const PIDS_LIMIT_EXPECTED_VALUE: DiagnosticCode =
    DiagnosticCode::new("compose.pids-limit.expected-number-or-string");

/// A service PID limit is neither unlimited, positive integral decimal, nor deferred.
pub const PIDS_LIMIT_INVALID: DiagnosticCode = DiagnosticCode::new("compose.pids-limit.invalid-value");

/// A zero service PID limit has ambiguous and unportable native semantics.
pub const PIDS_LIMIT_AMBIGUOUS_ZERO: DiagnosticCode = DiagnosticCode::new("compose.pids-limit.ambiguous-zero");

/// A service shared-memory size is not a number or string scalar.
pub const SHM_SIZE_EXPECTED_VALUE: DiagnosticCode = DiagnosticCode::new("compose.shm-size.expected-number-or-string");

/// A zero service shared-memory size has no defined Compose semantics.
pub const SHM_SIZE_AMBIGUOUS_ZERO: DiagnosticCode = DiagnosticCode::new("compose.shm-size.ambiguous-zero");

/// A schema-accepted numeric shared-memory size lacks a documented explicit unit.
pub const SHM_SIZE_PROVIDER_DEPENDENT_NUMBER: DiagnosticCode =
    DiagnosticCode::new("compose.shm-size.provider-dependent-number");

/// A schema-accepted string shared-memory size is outside the documented lowercase suffix family.
pub const SHM_SIZE_PROVIDER_DEPENDENT_STRING: DiagnosticCode =
    DiagnosticCode::new("compose.shm-size.provider-dependent-string");

/// A service memory limit is not a number or string scalar.
pub const MEM_LIMIT_EXPECTED_VALUE: DiagnosticCode = DiagnosticCode::new("compose.mem-limit.expected-number-or-string");

/// A zero service memory limit has no portable cross-provider meaning inferred by `ComposeLens`.
pub const MEM_LIMIT_AMBIGUOUS_ZERO: DiagnosticCode = DiagnosticCode::new("compose.mem-limit.ambiguous-zero");

/// A schema-accepted numeric memory limit lacks a documented explicit unit.
pub const MEM_LIMIT_SCHEMA_NUMBER: DiagnosticCode = DiagnosticCode::new("compose.mem-limit.schema-number");

/// A schema-accepted string memory limit is outside the documented lowercase suffix family.
pub const MEM_LIMIT_PROVIDER_DEPENDENT_STRING: DiagnosticCode =
    DiagnosticCode::new("compose.mem-limit.provider-dependent-string");

/// A service image pull policy is not documented, schema-recognized, or deferred.
pub const PULL_POLICY_INVALID: DiagnosticCode = DiagnosticCode::new("compose.pull-policy.invalid-policy");

/// A service stop grace period does not match the raw-preserving policy based on documented Compose units.
pub const STOP_GRACE_PERIOD_INVALID: DiagnosticCode =
    DiagnosticCode::new("compose.lifecycle.invalid-stop-grace-period");

/// A service `cap_drop` value is not a YAML sequence.
pub const CAP_DROP_EXPECTED_SEQUENCE: DiagnosticCode = DiagnosticCode::new("compose.cap-drop.expected-sequence");

/// A service `cap_drop` item is not a YAML string scalar.
pub const CAP_DROP_EXPECTED_STRING: DiagnosticCode = DiagnosticCode::new("compose.cap-drop.expected-string");

/// A service `cap_drop` sequence contains an exact duplicate string.
pub const CAP_DROP_DUPLICATE_ITEM: DiagnosticCode = DiagnosticCode::new("compose.cap-drop.duplicate-item");

/// A service `cap_add` value is not a YAML sequence.
pub const CAP_ADD_EXPECTED_SEQUENCE: DiagnosticCode = DiagnosticCode::new("compose.cap-add.expected-sequence");

/// A service `cap_add` item is not a YAML string scalar.
pub const CAP_ADD_EXPECTED_STRING: DiagnosticCode = DiagnosticCode::new("compose.cap-add.expected-string");

/// A service `cap_add` sequence contains an exact duplicate string.
pub const CAP_ADD_DUPLICATE_ITEM: DiagnosticCode = DiagnosticCode::new("compose.cap-add.duplicate-item");

/// A service `devices` value is not a YAML sequence.
pub const DEVICES_EXPECTED_SEQUENCE: DiagnosticCode = DiagnosticCode::new("compose.devices.expected-sequence");

/// A service device item is neither a string scalar nor a mapping.
pub const DEVICE_EXPECTED_FORM: DiagnosticCode = DiagnosticCode::new("compose.devices.expected-short-or-long");

/// A short device or long-device member is not a YAML string scalar.
pub const DEVICE_EXPECTED_STRING: DiagnosticCode = DiagnosticCode::new("compose.devices.expected-string");

/// A long-syntax service device is missing its required `source` string.
pub const DEVICE_MISSING_SOURCE: DiagnosticCode = DiagnosticCode::new("compose.devices.long.missing-source");

/// A service `dns` value is neither a YAML string scalar nor a sequence.
pub const DNS_EXPECTED_FORM: DiagnosticCode = DiagnosticCode::new("compose.dns.expected-string-or-list");

/// A service `dns` list item is not a YAML string scalar.
pub const DNS_EXPECTED_STRING: DiagnosticCode = DiagnosticCode::new("compose.dns.expected-string");

/// A service `dns_opt` value is not a YAML sequence.
pub const DNS_OPT_EXPECTED_SEQUENCE: DiagnosticCode = DiagnosticCode::new("compose.dns-opt.expected-sequence");

/// A service `dns_opt` item is not a YAML string scalar.
pub const DNS_OPT_EXPECTED_STRING: DiagnosticCode = DiagnosticCode::new("compose.dns-opt.expected-string");

/// A service `dns_opt` sequence contains an exact duplicate string.
pub const DNS_OPT_DUPLICATE_ITEM: DiagnosticCode = DiagnosticCode::new("compose.dns-opt.duplicate-item");

/// A service `dns_search` value is neither a YAML string scalar nor a sequence.
pub const DNS_SEARCH_EXPECTED_FORM: DiagnosticCode = DiagnosticCode::new("compose.dns-search.expected-string-or-list");

/// A service `dns_search` list item is not a YAML string scalar.
pub const DNS_SEARCH_EXPECTED_STRING: DiagnosticCode = DiagnosticCode::new("compose.dns-search.expected-string");

/// A service `dns_search` list contains an exact duplicate string.
pub const DNS_SEARCH_DUPLICATE_ITEM: DiagnosticCode = DiagnosticCode::new("compose.dns-search.duplicate-item");

/// A service `expose` value is not a YAML sequence.
pub const EXPOSE_EXPECTED_SEQUENCE: DiagnosticCode = DiagnosticCode::new("compose.expose.expected-sequence");

/// A service `expose` item is not a YAML string or number scalar.
pub const EXPOSE_EXPECTED_SCALAR: DiagnosticCode = DiagnosticCode::new("compose.expose.expected-string-or-number");

/// A service `expose` item does not match the documented decimal port/range grammar.
pub const EXPOSE_INVALID_ITEM: DiagnosticCode = DiagnosticCode::new("compose.expose.invalid-item");

/// A service `expose` item uses a protocol outside the documented portable set.
pub const EXPOSE_PROVIDER_DEPENDENT: DiagnosticCode = DiagnosticCode::new("compose.expose.provider-dependent-protocol");

/// A service `expose` sequence contains an exact duplicate scalar identity.
pub const EXPOSE_DUPLICATE_ITEM: DiagnosticCode = DiagnosticCode::new("compose.expose.duplicate-item");

/// A service `security_opt` value is not a YAML sequence.
pub const SECURITY_OPT_EXPECTED_SEQUENCE: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.expected-sequence");

/// A service `security_opt` item is not a YAML string scalar.
pub const SECURITY_OPT_EXPECTED_STRING: DiagnosticCode = DiagnosticCode::new("compose.security-opt.expected-string");

/// A service `security_opt` item is an explicitly empty string.
pub const SECURITY_OPT_EMPTY_ITEM: DiagnosticCode = DiagnosticCode::new("compose.security-opt.empty-item");

/// An AppArmor-shaped service `security_opt` item is not the exact narrow candidate form.
pub const SECURITY_OPT_APPARMOR_NEAR_MISS: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.apparmor-near-miss");

/// More than one exact `AppArmor` candidate remains in a service `security_opt` sequence.
pub const SECURITY_OPT_APPARMOR_CONFLICT: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.apparmor-conflict");

/// A seccomp-shaped service `security_opt` item is not the exact narrow candidate form.
pub const SECURITY_OPT_SECCOMP_NEAR_MISS: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.seccomp-near-miss");

/// More than one exact seccomp candidate remains in a service `security_opt` sequence.
pub const SECURITY_OPT_SECCOMP_CONFLICT: DiagnosticCode = DiagnosticCode::new("compose.security-opt.seccomp-conflict");

/// A no-new-privileges-shaped item is not an exact lowercase boolean candidate.
pub const SECURITY_OPT_NO_NEW_PRIVILEGES_NEAR_MISS: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.no-new-privileges-near-miss");

/// More than one exact no-new-privileges candidate remains in one effective sequence.
pub const SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.no-new-privileges-conflict");

/// A mask-shaped service `security_opt` item is not the exact narrow candidate form.
pub const SECURITY_OPT_MASK_NEAR_MISS: DiagnosticCode = DiagnosticCode::new("compose.security-opt.mask-near-miss");

/// An unmask-shaped service `security_opt` item is not the exact narrow candidate form.
pub const SECURITY_OPT_UNMASK_NEAR_MISS: DiagnosticCode = DiagnosticCode::new("compose.security-opt.unmask-near-miss");

pub(crate) fn security_path_option_diagnostic(kind: &SecurityOptionKind, span: SourceSpan) -> Option<Diagnostic> {
    let (code, message) = match kind {
        SecurityOptionKind::MaskNearMiss => (
            SECURITY_OPT_MASK_NEAR_MISS,
            "mask candidates require exact lowercase `mask=<paths>` spelling with a non-empty whitespace-free payload",
        ),
        SecurityOptionKind::UnmaskNearMiss => (
            SECURITY_OPT_UNMASK_NEAR_MISS,
            "unmask candidates require exact lowercase `unmask=ALL` or colon-separated slash-prefixed paths without whitespace",
        ),
        _ => return None,
    };
    Some(
        Diagnostic::new(code, Severity::Warning, message)
            .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
    )
}

/// A `SELinux` label-disable-shaped item is not the exact lowercase candidate.
pub const SECURITY_OPT_SECURITY_LABEL_DISABLE_NEAR_MISS: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.security-label-disable-near-miss");

/// More than one exact `SELinux` label-disable candidate remains in one effective sequence.
pub const SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.security-label-disable-conflict");

/// A `SELinux` label-filetype-shaped item is not the exact lowercase candidate.
pub const SECURITY_OPT_SECURITY_LABEL_FILETYPE_NEAR_MISS: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.security-label-filetype-near-miss");

/// More than one exact `SELinux` label-filetype candidate remains in one effective sequence.
pub const SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.security-label-filetype-conflict");

/// A `SELinux` label-level-shaped item is not the exact lowercase candidate.
pub const SECURITY_OPT_SECURITY_LABEL_LEVEL_NEAR_MISS: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.security-label-level-near-miss");

/// More than one exact `SELinux` label-level candidate remains in one effective sequence.
pub const SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.security-label-level-conflict");

/// A `SELinux` label-nested-shaped item is not the exact lowercase candidate.
pub const SECURITY_OPT_SECURITY_LABEL_NESTED_NEAR_MISS: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.security-label-nested-near-miss");

/// More than one exact `SELinux` label-nested candidate remains in one effective sequence.
pub const SECURITY_OPT_SECURITY_LABEL_NESTED_CONFLICT: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.security-label-nested-conflict");

/// A `SELinux` label-type-shaped item is not the exact lowercase candidate.
pub const SECURITY_OPT_SECURITY_LABEL_TYPE_NEAR_MISS: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.security-label-type-near-miss");

/// More than one exact `SELinux` label-type candidate remains in one effective sequence.
pub const SECURITY_OPT_SECURITY_LABEL_TYPE_CONFLICT: DiagnosticCode =
    DiagnosticCode::new("compose.security-opt.security-label-type-conflict");

fn authored_security_label_diagnostic(
    kind: &SecurityOptionKind,
    span: SourceSpan,
    candidates: &mut SecurityOptionCandidateCounts,
) -> Option<Diagnostic> {
    match kind {
        SecurityOptionKind::SecurityLabelDisable { .. } => {
            candidates.security_label_disable += 1;
            (candidates.security_label_disable > 1).then(|| {
                Diagnostic::new(
                    SECURITY_OPT_SECURITY_LABEL_DISABLE_CONFLICT,
                    Severity::Warning,
                    "multiple SELinux label-disable candidates are retained; a consumer must resolve the conflict explicitly",
                )
                .with_label(DiagnosticLabel::primary(
                    span,
                    "additional SELinux label-disable candidate retained",
                ))
            })
        }
        SecurityOptionKind::SecurityLabelDisableNearMiss => Some(
            Diagnostic::new(
                SECURITY_OPT_SECURITY_LABEL_DISABLE_NEAR_MISS,
                Severity::Warning,
                "SELinux label-disable candidates require exact lowercase `label:disable` spelling without whitespace",
            )
            .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
        ),
        SecurityOptionKind::SecurityLabelFileType { .. } => {
            candidates.security_label_filetype += 1;
            (candidates.security_label_filetype > 1).then(|| {
                Diagnostic::new(
                    SECURITY_OPT_SECURITY_LABEL_FILETYPE_CONFLICT,
                    Severity::Warning,
                    "multiple SELinux label-filetype candidates are retained; a consumer must resolve the conflict explicitly",
                )
                .with_label(DiagnosticLabel::primary(
                    span,
                    "additional SELinux label-filetype candidate retained",
                ))
            })
        }
        SecurityOptionKind::SecurityLabelFileTypeNearMiss => Some(
            Diagnostic::new(
                SECURITY_OPT_SECURITY_LABEL_FILETYPE_NEAR_MISS,
                Severity::Warning,
                "SELinux label-filetype candidates require exact lowercase `label:filetype:<type>` spelling without whitespace",
            )
            .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
        ),
        SecurityOptionKind::SecurityLabelLevel { .. } => {
            candidates.security_label_level += 1;
            (candidates.security_label_level > 1).then(|| {
                Diagnostic::new(
                    SECURITY_OPT_SECURITY_LABEL_LEVEL_CONFLICT,
                    Severity::Warning,
                    "multiple SELinux label-level candidates are retained; a consumer must resolve the conflict explicitly",
                )
                .with_label(DiagnosticLabel::primary(
                    span,
                    "additional SELinux label-level candidate retained",
                ))
            })
        }
        SecurityOptionKind::SecurityLabelLevelNearMiss => Some(
            Diagnostic::new(
                SECURITY_OPT_SECURITY_LABEL_LEVEL_NEAR_MISS,
                Severity::Warning,
                "SELinux label-level candidates require exact lowercase `label:level:<level>` spelling without whitespace",
            )
            .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
        ),
        SecurityOptionKind::SecurityLabelNested { .. } => {
            candidates.security_label_nested += 1;
            (candidates.security_label_nested > 1).then(|| {
                Diagnostic::new(
                    SECURITY_OPT_SECURITY_LABEL_NESTED_CONFLICT,
                    Severity::Warning,
                    "multiple SELinux label-nested candidates are retained; a consumer must resolve the conflict explicitly",
                )
                .with_label(DiagnosticLabel::primary(
                    span,
                    "additional SELinux label-nested candidate retained",
                ))
            })
        }
        SecurityOptionKind::SecurityLabelNestedNearMiss => Some(
            Diagnostic::new(
                SECURITY_OPT_SECURITY_LABEL_NESTED_NEAR_MISS,
                Severity::Warning,
                "SELinux label-nested candidates require exact lowercase `label:nested` spelling without whitespace",
            )
            .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
        ),
        SecurityOptionKind::SecurityLabelType { .. } | SecurityOptionKind::SecurityLabelTypeNearMiss => {
            authored_security_label_type_diagnostic(kind, span, &mut candidates.security_label_type)
        }
        _ => None,
    }
}

fn authored_security_label_type_diagnostic(
    kind: &SecurityOptionKind,
    span: SourceSpan,
    candidates: &mut usize,
) -> Option<Diagnostic> {
    match kind {
        SecurityOptionKind::SecurityLabelType { .. } => {
            *candidates += 1;
            (*candidates > 1).then(|| {
                Diagnostic::new(
                    SECURITY_OPT_SECURITY_LABEL_TYPE_CONFLICT,
                    Severity::Warning,
                    "multiple SELinux label-type candidates are retained; a consumer must resolve the conflict explicitly",
                )
                .with_label(DiagnosticLabel::primary(
                    span,
                    "additional SELinux label-type candidate retained",
                ))
            })
        }
        SecurityOptionKind::SecurityLabelTypeNearMiss => Some(
            Diagnostic::new(
                SECURITY_OPT_SECURITY_LABEL_TYPE_NEAR_MISS,
                Severity::Warning,
                "SELinux label-type candidates require exact lowercase `label:type:<type>` spelling with one non-empty whitespace-free type",
            )
            .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
        ),
        _ => None,
    }
}

/// A service `annotations` value is neither mapping nor list syntax.
pub const ANNOTATIONS_EXPECTED_FORM: DiagnosticCode = DiagnosticCode::new("compose.annotations.expected-map-or-list");

/// A service annotation list item is not a YAML string scalar.
pub const ANNOTATIONS_EXPECTED_STRING: DiagnosticCode = DiagnosticCode::new("compose.annotations.expected-string");

/// A service annotation has an empty semantic name.
pub const ANNOTATIONS_EMPTY_NAME: DiagnosticCode = DiagnosticCode::new("compose.annotations.empty-name");

/// A key-only service annotation list item has no defined explicit value.
pub const ANNOTATIONS_KEY_ONLY: DiagnosticCode = DiagnosticCode::new("compose.annotations.key-only");

/// More than one authored service annotation resolves to the same semantic name.
pub const ANNOTATIONS_DUPLICATE_NAME: DiagnosticCode = DiagnosticCode::new("compose.annotations.duplicate-name");

/// A service-level `tmpfs` value is neither a string scalar nor a sequence.
pub const TMPFS_EXPECTED_FORM: DiagnosticCode = DiagnosticCode::new("compose.tmpfs.expected-string-or-list");

/// A service-level `tmpfs` sequence item is not a YAML string scalar.
pub const TMPFS_EXPECTED_STRING: DiagnosticCode = DiagnosticCode::new("compose.tmpfs.expected-string");

/// A service-level `tmpfs` item is malformed or depends on provider- or target-specific behavior.
pub const TMPFS_PROVIDER_DEPENDENT: DiagnosticCode = DiagnosticCode::new("compose.tmpfs.provider-dependent-item");

/// A service `sysctls` value is neither a mapping nor a sequence.
pub const SYSCTLS_EXPECTED_FORM: DiagnosticCode = DiagnosticCode::new("compose.sysctls.expected-map-or-list");

/// A service `sysctls` mapping contains an empty key.
pub const SYSCTLS_EMPTY_KEY: DiagnosticCode = DiagnosticCode::new("compose.sysctls.empty-key");

/// A service `sysctls` mapping value is not a scalar or null.
pub const SYSCTLS_EXPECTED_SCALAR: DiagnosticCode = DiagnosticCode::new("compose.sysctls.expected-scalar");

/// A service `sysctls` list item is not a YAML string scalar.
pub const SYSCTLS_EXPECTED_STRING: DiagnosticCode = DiagnosticCode::new("compose.sysctls.expected-string");

/// A service `sysctls` list contains an exact duplicate string.
pub const SYSCTLS_DUPLICATE_ITEM: DiagnosticCode = DiagnosticCode::new("compose.sysctls.duplicate-item");

/// A service `logging` value is not a mapping.
pub const LOGGING_EXPECTED_MAPPING: DiagnosticCode = DiagnosticCode::new("compose.logging.expected-mapping");

/// A service logging driver is not a YAML string scalar.
pub const LOGGING_DRIVER_EXPECTED_STRING: DiagnosticCode =
    DiagnosticCode::new("compose.logging.driver.expected-string");

/// A service logging options value is not a mapping.
pub const LOGGING_OPTIONS_EXPECTED_MAPPING: DiagnosticCode =
    DiagnosticCode::new("compose.logging.options.expected-mapping");

/// A service logging option has an empty key.
pub const LOGGING_OPTION_EMPTY_KEY: DiagnosticCode = DiagnosticCode::new("compose.logging.option.empty-key");

/// A service logging option is not a YAML string, number, or null scalar.
pub const LOGGING_OPTION_EXPECTED_SCALAR: DiagnosticCode =
    DiagnosticCode::new("compose.logging.option.expected-scalar");

/// A service environment-file item is neither scalar short syntax nor mapping long syntax.
pub const ENVIRONMENT_FILE_EXPECTED_FORM: DiagnosticCode =
    DiagnosticCode::new("compose.environment-file.expected-short-or-long");

/// A long-syntax service environment-file entry is missing `path`.
pub const ENVIRONMENT_FILE_MISSING_PATH: DiagnosticCode =
    DiagnosticCode::new("compose.environment-file.long.missing-path");

/// A long-syntax service environment-file format is not defined by Compose.
pub const ENVIRONMENT_FILE_INVALID_FORMAT: DiagnosticCode =
    DiagnosticCode::new("compose.environment-file.invalid-format");

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

/// A `BuildKit` SSH declaration has an unsupported outer or item form.
pub const BUILD_SSH_EXPECTED_FORM: DiagnosticCode = DiagnosticCode::new("compose.build.ssh-expected-form");

/// A `BuildKit` SSH list repeats an item despite the schema uniqueness rule.
pub const BUILD_SSH_DUPLICATE_ITEM: DiagnosticCode = DiagnosticCode::new("compose.build.ssh-duplicate-item");

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
    hostname: Option<Hostname>,
    container_name: Option<Located<String>>,
    image: Option<Located<ImageReference>>,
    entrypoint: Option<Entrypoint>,
    command: Option<Command>,
    init: Option<Located<BooleanValue>>,
    stdin_open: Option<Located<BooleanValue>>,
    tty: Option<Located<BooleanValue>>,
    privileged: Option<Located<BooleanValue>>,
    environment: Option<Environment>,
    environment_files: Vec<EnvironmentFile>,
    labels: Option<Labels>,
    annotations: Option<Annotations>,
    extra_hosts: Option<ExtraHosts>,
    user: Option<UserSpec>,
    userns_mode: Option<UserNamespaceMode>,
    group_add: Vec<Located<String>>,
    cap_add: Option<CapabilityAdd>,
    cap_drop: Option<CapabilityDrop>,
    devices: Option<Devices>,
    dns: Option<Dns>,
    dns_options: Option<DnsOptions>,
    dns_search: Option<DnsSearch>,
    expose: Option<Expose>,
    security_options: Option<SecurityOptions>,
    working_dir: Option<Located<String>>,
    read_only: Option<Located<BooleanValue>>,
    pids_limit: Option<PidsLimit>,
    shm_size: Option<ShmSize>,
    mem_limit: Option<MemLimit>,
    tmpfs: Option<Tmpfs>,
    sysctls: Option<Sysctls>,
    logging: Option<Logging>,
    pull_policy: Option<PullPolicy>,
    restart: Option<RestartPolicy>,
    stop_signal: Option<Located<String>>,
    stop_grace_period: Option<Located<StopGracePeriod>>,
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
            hostname: None,
            container_name: None,
            image: None,
            entrypoint: None,
            command: None,
            init: None,
            stdin_open: None,
            tty: None,
            privileged: None,
            environment: None,
            environment_files: Vec::new(),
            labels: None,
            annotations: None,
            extra_hosts: None,
            user: None,
            userns_mode: None,
            group_add: Vec::new(),
            cap_add: None,
            cap_drop: None,
            devices: None,
            dns: None,
            dns_options: None,
            dns_search: None,
            expose: None,
            security_options: None,
            working_dir: None,
            read_only: None,
            pids_limit: None,
            shm_size: None,
            mem_limit: None,
            tmpfs: None,
            sysctls: None,
            logging: None,
            pull_policy: None,
            restart: None,
            stop_signal: None,
            stop_grace_period: None,
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

    /// Returns the explicitly authored raw-preserving service hostname.
    #[must_use]
    pub const fn hostname(&self) -> Option<&Hostname> {
        self.hostname.as_ref()
    }

    /// Returns the explicitly authored runtime container name.
    #[must_use]
    pub const fn container_name(&self) -> Option<&Located<String>> {
        self.container_name.as_ref()
    }

    /// Returns the explicitly authored image reference.
    #[must_use]
    pub const fn image(&self) -> Option<&Located<ImageReference>> {
        self.image.as_ref()
    }

    /// Returns the entrypoint without normalizing its authored form.
    #[must_use]
    pub const fn entrypoint(&self) -> Option<&Entrypoint> {
        self.entrypoint.as_ref()
    }

    /// Returns the command without normalizing its authored form.
    #[must_use]
    pub const fn command(&self) -> Option<&Command> {
        self.command.as_ref()
    }

    /// Returns whether Compose should run its platform-specific init process.
    #[must_use]
    pub const fn init(&self) -> Option<&Located<BooleanValue>> {
        self.init.as_ref()
    }

    /// Returns whether Compose should keep standard input open for the service.
    #[must_use]
    pub const fn stdin_open(&self) -> Option<&Located<BooleanValue>> {
        self.stdin_open.as_ref()
    }

    /// Returns whether Compose should allocate a terminal for the service.
    #[must_use]
    pub const fn tty(&self) -> Option<&Located<BooleanValue>> {
        self.tty.as_ref()
    }

    /// Returns whether Compose should run the service with its privileged choice.
    #[must_use]
    pub const fn privileged(&self) -> Option<&Located<BooleanValue>> {
        self.privileged.as_ref()
    }

    /// Returns environment variables with list and mapping forms kept distinct.
    #[must_use]
    pub const fn environment(&self) -> Option<&Environment> {
        self.environment.as_ref()
    }

    /// Returns service environment files in authored order with syntax retained.
    #[must_use]
    pub fn environment_files(&self) -> &[EnvironmentFile] {
        &self.environment_files
    }

    /// Returns service metadata labels with list and mapping forms kept distinct.
    #[must_use]
    pub const fn labels(&self) -> Option<&Labels> {
        self.labels.as_ref()
    }

    /// Returns service annotations with list and mapping forms kept distinct.
    #[must_use]
    pub const fn annotations(&self) -> Option<&Annotations> {
        self.annotations.as_ref()
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

    /// Returns the explicitly authored capability-add sequence, including an explicit empty one.
    #[must_use]
    pub const fn cap_add(&self) -> Option<&CapabilityAdd> {
        self.cap_add.as_ref()
    }

    /// Returns the explicitly authored capability-drop sequence, including an explicit empty one.
    #[must_use]
    pub const fn cap_drop(&self) -> Option<&CapabilityDrop> {
        self.cap_drop.as_ref()
    }

    /// Returns the explicitly authored ordered device sequence, including an explicit empty one.
    #[must_use]
    pub const fn devices(&self) -> Option<&Devices> {
        self.devices.as_ref()
    }

    /// Returns raw service DNS servers with scalar and ordered-list forms retained.
    #[must_use]
    pub const fn dns(&self) -> Option<&Dns> {
        self.dns.as_ref()
    }

    /// Returns the explicitly authored ordered DNS resolver-option sequence.
    #[must_use]
    pub const fn dns_options(&self) -> Option<&DnsOptions> {
        self.dns_options.as_ref()
    }

    /// Returns raw DNS search domains with scalar and ordered-list forms retained.
    #[must_use]
    pub const fn dns_search(&self) -> Option<&DnsSearch> {
        self.dns_search.as_ref()
    }

    /// Returns the explicitly authored ordered exposed-port sequence.
    #[must_use]
    pub const fn expose(&self) -> Option<&Expose> {
        self.expose.as_ref()
    }

    /// Returns the explicitly authored ordered raw service security options.
    #[must_use]
    pub const fn security_options(&self) -> Option<&SecurityOptions> {
        self.security_options.as_ref()
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

    /// Returns the raw-preserving service PID limit.
    #[must_use]
    pub const fn pids_limit(&self) -> Option<&PidsLimit> {
        self.pids_limit.as_ref()
    }

    /// Returns the raw-preserving service shared-memory size.
    #[must_use]
    pub const fn shm_size(&self) -> Option<&ShmSize> {
        self.shm_size.as_ref()
    }

    /// Returns the raw-preserving service memory limit.
    #[must_use]
    pub const fn mem_limit(&self) -> Option<&MemLimit> {
        self.mem_limit.as_ref()
    }

    /// Returns service-level temporary filesystems with scalar and list forms retained.
    #[must_use]
    pub const fn tmpfs(&self) -> Option<&Tmpfs> {
        self.tmpfs.as_ref()
    }

    /// Returns service sysctls with mapping/list form and scalar spelling retained.
    #[must_use]
    pub const fn sysctls(&self) -> Option<&Sysctls> {
        self.sysctls.as_ref()
    }

    /// Returns service logging configuration with an uninterpreted driver and ordered options.
    #[must_use]
    pub const fn logging(&self) -> Option<&Logging> {
        self.logging.as_ref()
    }

    /// Returns the raw-preserving service image pull policy.
    #[must_use]
    pub const fn pull_policy(&self) -> Option<&PullPolicy> {
        self.pull_policy.as_ref()
    }

    /// Returns the service-level container restart policy.
    #[must_use]
    pub const fn restart(&self) -> Option<&RestartPolicy> {
        self.restart.as_ref()
    }

    /// Returns the explicitly authored signal used to stop the service.
    #[must_use]
    pub const fn stop_signal(&self) -> Option<&Located<String>> {
        self.stop_signal.as_ref()
    }

    /// Returns the raw-preserving service stop grace period.
    #[must_use]
    pub const fn stop_grace_period(&self) -> Option<&Located<StopGracePeriod>> {
        self.stop_grace_period.as_ref()
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

fn annotation_diagnostic(
    code: DiagnosticCode,
    severity: Severity,
    span: SourceSpan,
    message: &'static str,
    label: &'static str,
) -> Diagnostic {
    Diagnostic::new(code, severity, message).with_label(DiagnosticLabel::primary(span, label))
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
                "hostname" if !duplicate => service.hostname = self.parse_hostname(&service_field),
                "container_name" if !duplicate => {
                    service.container_name = self.parse_string(&service_field, "container name");
                }
                "image" if !duplicate => service.image = self.parse_image(&service_field),
                "entrypoint" if !duplicate => service.entrypoint = self.parse_entrypoint(&service_field),
                "command" if !duplicate => service.command = self.parse_command(&service_field),
                "init" if !duplicate => service.init = self.parse_boolean(&service_field, "service init"),
                "stdin_open" if !duplicate => service.stdin_open = self.parse_boolean(&service_field, "stdin_open"),
                "tty" if !duplicate => service.tty = self.parse_boolean(&service_field, "tty"),
                "privileged" if !duplicate => service.privileged = self.parse_boolean(&service_field, "privileged"),
                "environment" if !duplicate => service.environment = self.parse_environment(&service_field),
                "env_file" if !duplicate => service.environment_files = self.parse_environment_files(&service_field),
                "labels" if !duplicate => service.labels = self.parse_labels(&service_field),
                "annotations" if !duplicate => service.annotations = self.parse_annotations(&service_field),
                "extra_hosts" if !duplicate => service.extra_hosts = self.parse_extra_hosts(&service_field),
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
                "cap_add" if !duplicate => service.cap_add = self.parse_cap_add(&service_field),
                "cap_drop" if !duplicate => service.cap_drop = self.parse_cap_drop(&service_field),
                "devices" if !duplicate => service.devices = self.parse_devices(&service_field),
                "dns" if !duplicate => service.dns = self.parse_dns(&service_field),
                "dns_opt" if !duplicate => service.dns_options = self.parse_dns_options(&service_field),
                "dns_search" if !duplicate => service.dns_search = self.parse_dns_search(&service_field),
                "expose" if !duplicate => service.expose = self.parse_expose(&service_field),
                "security_opt" if !duplicate => service.security_options = self.parse_security_options(&service_field),
                "working_dir" if !duplicate => {
                    service.working_dir = self.parse_string(&service_field, "service working directory");
                }
                "read_only" if !duplicate => {
                    service.read_only = self.parse_boolean(&service_field, "service read_only");
                }
                "pids_limit" if !duplicate => service.pids_limit = self.parse_pids_limit(&service_field),
                "shm_size" if !duplicate => service.shm_size = self.parse_shm_size(&service_field),
                "mem_limit" if !duplicate => service.mem_limit = self.parse_mem_limit(&service_field),
                "tmpfs" if !duplicate => service.tmpfs = self.parse_tmpfs(&service_field),
                "sysctls" if !duplicate => service.sysctls = self.parse_sysctls(&service_field),
                "logging" if !duplicate => service.logging = self.parse_logging(&service_field),
                "pull_policy" if !duplicate => service.pull_policy = self.parse_pull_policy(&service_field),
                "restart" if !duplicate => service.restart = self.parse_restart_policy(&service_field),
                "stop_signal" if !duplicate => {
                    service.stop_signal = self.parse_string(&service_field, "service stop signal");
                }
                "stop_grace_period" if !duplicate => {
                    service.stop_grace_period = self.parse_stop_grace_period(&service_field);
                }
                "ulimits" if !duplicate => service.ulimits = self.parse_ulimits(&service_field),
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
                    service.secrets = self.parse_secret_grants(&service_field).unwrap_or_default();
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

    fn parse_hostname(&mut self, field: &ParsedField) -> Option<Hostname> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(HOSTNAME_EXPECTED_STRING, field, "hostname must be a YAML string scalar");
            return None;
        };
        if ScalarValue::from_scalar(scalar).scalar_type() != ScalarType::String {
            self.expected(HOSTNAME_EXPECTED_STRING, field, "hostname must be a YAML string scalar");
            return None;
        }
        let span = span_from_position(self.source_id, scalar.byte_range());
        let hostname = Hostname::parse(Located::new(scalar_string_from_source(&self.source, scalar), span));
        if hostname.kind() == &HostnameKind::Invalid {
            self.diagnostics.push(
                Diagnostic::new(
                    HOSTNAME_INVALID,
                    Severity::Error,
                    "hostname must be an ASCII RFC-1123 name of 1 to 253 characters with dot-separated labels of 1 to 63 alphanumeric or hyphen characters",
                )
                .with_label(DiagnosticLabel::primary(span, "invalid service hostname"))
                .with_note("each label must start and end with an ASCII letter or digit"),
            );
        }
        Some(hostname)
    }

    fn parse_image(&mut self, field: &ParsedField) -> Option<Located<ImageReference>> {
        self.parse_string(field, "service image")
            .map(|value| Located::new(ImageReference::parse(value.value), value.span))
    }

    fn parse_cap_drop(&mut self, field: &ParsedField) -> Option<CapabilityDrop> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(
                CAP_DROP_EXPECTED_SEQUENCE,
                field,
                "cap_drop must be a sequence of string scalars",
            );
            return None;
        };
        let span = span_from_position(self.source_id, sequence.byte_range());
        let mut items = Vec::new();
        let mut seen = BTreeMap::new();
        for node in sequence.values() {
            let YamlNode::Scalar(scalar) = node else {
                self.unsupported_sequence_item(
                    CAP_DROP_EXPECTED_STRING,
                    &node,
                    field.span,
                    "cap_drop entries must be string scalars",
                );
                continue;
            };
            let scalar_type = ScalarValue::from_scalar(&scalar).scalar_type();
            if !matches!(
                scalar_type,
                ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
            ) {
                self.unsupported_sequence_item(
                    CAP_DROP_EXPECTED_STRING,
                    &YamlNode::Scalar(scalar),
                    field.span,
                    "cap_drop entries must be string scalars",
                );
                continue;
            }
            let item_span = span_from_position(self.source_id, scalar.byte_range());
            let value = scalar_string_from_source(&self.source, &scalar);
            if let Some(first) = seen.get(&value) {
                self.diagnostics.push(
                    Diagnostic::new(
                        CAP_DROP_DUPLICATE_ITEM,
                        Severity::Error,
                        "cap_drop entries must be unique exact strings",
                    )
                    .with_label(DiagnosticLabel::primary(item_span, "duplicate capability string"))
                    .with_label(DiagnosticLabel::secondary(*first, "first identical string")),
                );
            } else {
                seen.insert(value.clone(), item_span);
            }
            items.push(CapabilityDropItem::new(Located::new(value, item_span)));
        }
        Some(CapabilityDrop::new(span, items))
    }

    fn parse_cap_add(&mut self, field: &ParsedField) -> Option<CapabilityAdd> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(
                CAP_ADD_EXPECTED_SEQUENCE,
                field,
                "cap_add must be a sequence of string scalars",
            );
            return None;
        };
        let span = span_from_position(self.source_id, sequence.byte_range());
        let mut items = Vec::new();
        let mut seen = BTreeMap::new();
        for node in sequence.values() {
            let YamlNode::Scalar(scalar) = node else {
                self.unsupported_sequence_item(
                    CAP_ADD_EXPECTED_STRING,
                    &node,
                    field.span,
                    "cap_add entries must be string scalars",
                );
                continue;
            };
            let scalar_type = ScalarValue::from_scalar(&scalar).scalar_type();
            if !matches!(
                scalar_type,
                ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
            ) {
                self.unsupported_sequence_item(
                    CAP_ADD_EXPECTED_STRING,
                    &YamlNode::Scalar(scalar),
                    field.span,
                    "cap_add entries must be string scalars",
                );
                continue;
            }
            let item_span = span_from_position(self.source_id, scalar.byte_range());
            let value = scalar_string_from_source(&self.source, &scalar);
            if let Some(first) = seen.get(&value) {
                self.diagnostics.push(
                    Diagnostic::new(
                        CAP_ADD_DUPLICATE_ITEM,
                        Severity::Error,
                        "cap_add entries must be unique exact strings",
                    )
                    .with_label(DiagnosticLabel::primary(item_span, "duplicate capability string"))
                    .with_label(DiagnosticLabel::secondary(*first, "first identical string")),
                );
            } else {
                seen.insert(value.clone(), item_span);
            }
            items.push(CapabilityAddItem::new(Located::new(value, item_span)));
        }
        Some(CapabilityAdd::new(span, items))
    }

    fn parse_devices(&mut self, field: &ParsedField) -> Option<Devices> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(
                DEVICES_EXPECTED_SEQUENCE,
                field,
                "service devices must be a sequence of string scalars or mappings",
            );
            return None;
        };
        let span = span_from_position(self.source_id, sequence.byte_range());
        let mut devices = Vec::new();
        for node in sequence.values() {
            match node {
                YamlNode::Scalar(scalar)
                    if matches!(
                        ScalarValue::from_scalar(&scalar).scalar_type(),
                        ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
                    ) =>
                {
                    let item_span = span_from_position(self.source_id, scalar.byte_range());
                    let raw = Located::new(scalar_string_from_source(&self.source, &scalar), item_span);
                    devices.push(Device::Short(ShortDevice::new(raw)));
                }
                YamlNode::Mapping(mapping) => devices.push(Device::Long(self.parse_long_device(&mapping))),
                other => self.unsupported_sequence_item(
                    DEVICE_EXPECTED_FORM,
                    &other,
                    field.span,
                    "service device must use string short syntax or mapping long syntax",
                ),
            }
        }
        Some(Devices::new(span, devices))
    }

    fn parse_long_device(&mut self, mapping: &Mapping) -> LongDevice {
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut device = LongDevice::new(span);
        let mut seen = BTreeMap::new();
        for field in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &field);
            match field.name.value.as_str() {
                "source" if !duplicate => self
                    .parse_device_string(&field, "device source")
                    .into_iter()
                    .for_each(|value| device.set_source(value)),
                "target" if !duplicate => self
                    .parse_device_string(&field, "device target")
                    .into_iter()
                    .for_each(|value| device.set_target(value)),
                "permissions" if !duplicate => self
                    .parse_device_string(&field, "device permissions")
                    .into_iter()
                    .for_each(|value| device.set_permissions(value)),
                name if name.starts_with("x-") => device.push_extension(field.reference()),
                _ if duplicate => {}
                _ => device.push_unknown(field.reference()),
            }
        }
        if device.source().is_none() {
            self.missing(
                DEVICE_MISSING_SOURCE,
                span,
                "long service device is missing required string `source`",
            );
        }
        device
    }

    fn parse_device_string(&mut self, field: &ParsedField, description: &str) -> Option<Located<String>> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(
                DEVICE_EXPECTED_STRING,
                field,
                format!("{description} must be a string scalar"),
            );
            return None;
        };
        if !matches!(
            ScalarValue::from_scalar(scalar).scalar_type(),
            ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
        ) {
            self.expected(
                DEVICE_EXPECTED_STRING,
                field,
                format!("{description} must be a string scalar"),
            );
            return None;
        }
        Some(Located::new(
            scalar_string_from_source(&self.source, scalar),
            span_from_position(self.source_id, scalar.byte_range()),
        ))
    }

    fn parse_dns(&mut self, field: &ParsedField) -> Option<Dns> {
        let value = field.value.as_ref()?;
        if let Some(scalar) = value.as_scalar() {
            if !matches!(
                ScalarValue::from_scalar(scalar).scalar_type(),
                ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
            ) {
                self.expected(
                    DNS_EXPECTED_FORM,
                    field,
                    "dns must be a string scalar or a sequence of string scalars",
                );
                return None;
            }
            let span = span_from_position(self.source_id, scalar.byte_range());
            return Some(Dns::new(
                span,
                DnsForm::Scalar(Located::new(scalar_string_from_source(&self.source, scalar), span)),
            ));
        }

        let Some(sequence) = value.as_sequence() else {
            self.expected(
                DNS_EXPECTED_FORM,
                field,
                "dns must be a string scalar or a sequence of string scalars",
            );
            return None;
        };
        let span = span_from_position(self.source_id, sequence.byte_range());
        let mut items = Vec::new();
        for node in sequence.values() {
            let YamlNode::Scalar(scalar) = node else {
                self.unsupported_sequence_item(
                    DNS_EXPECTED_STRING,
                    &node,
                    field.span,
                    "dns entries must be string scalars",
                );
                continue;
            };
            if !matches!(
                ScalarValue::from_scalar(&scalar).scalar_type(),
                ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
            ) {
                self.unsupported_sequence_item(
                    DNS_EXPECTED_STRING,
                    &YamlNode::Scalar(scalar),
                    field.span,
                    "dns entries must be string scalars",
                );
                continue;
            }
            let item_span = span_from_position(self.source_id, scalar.byte_range());
            items.push(Located::new(
                scalar_string_from_source(&self.source, &scalar),
                item_span,
            ));
        }
        Some(Dns::new(span, DnsForm::List(items)))
    }

    fn parse_dns_options(&mut self, field: &ParsedField) -> Option<DnsOptions> {
        let value = field.value.as_ref()?;
        let Some(sequence) = value.as_sequence() else {
            self.expected(
                DNS_OPT_EXPECTED_SEQUENCE,
                field,
                "dns_opt must be a sequence of string scalars",
            );
            return None;
        };
        let span = span_from_position(self.source_id, sequence.byte_range());
        let mut items = Vec::new();
        let mut seen = BTreeSet::new();
        for node in sequence.values() {
            let YamlNode::Scalar(scalar) = node else {
                self.unsupported_sequence_item(
                    DNS_OPT_EXPECTED_STRING,
                    &node,
                    field.span,
                    "dns_opt entries must be string scalars",
                );
                continue;
            };
            if !matches!(
                ScalarValue::from_scalar(&scalar).scalar_type(),
                ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
            ) {
                self.unsupported_sequence_item(
                    DNS_OPT_EXPECTED_STRING,
                    &YamlNode::Scalar(scalar),
                    field.span,
                    "dns_opt entries must be string scalars",
                );
                continue;
            }
            let item_span = span_from_position(self.source_id, scalar.byte_range());
            let option = scalar_string_from_source(&self.source, &scalar);
            if !seen.insert(option.clone()) {
                self.diagnostics.push(
                    Diagnostic::new(
                        DNS_OPT_DUPLICATE_ITEM,
                        Severity::Warning,
                        "dns_opt entries must be unique exact strings",
                    )
                    .with_label(DiagnosticLabel::primary(item_span, "duplicate DNS option retained")),
                );
            }
            items.push(Located::new(option, item_span));
        }
        Some(DnsOptions::new(span, items))
    }

    fn parse_dns_search(&mut self, field: &ParsedField) -> Option<DnsSearch> {
        let value = field.value.as_ref()?;
        if let Some(scalar) = value.as_scalar() {
            if !matches!(
                ScalarValue::from_scalar(scalar).scalar_type(),
                ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
            ) {
                self.expected(
                    DNS_SEARCH_EXPECTED_FORM,
                    field,
                    "dns_search must be a string scalar or a sequence of string scalars",
                );
                return None;
            }
            let span = span_from_position(self.source_id, scalar.byte_range());
            return Some(DnsSearch::new(
                span,
                DnsSearchForm::Scalar(Located::new(scalar_string_from_source(&self.source, scalar), span)),
            ));
        }

        let Some(sequence) = value.as_sequence() else {
            self.expected(
                DNS_SEARCH_EXPECTED_FORM,
                field,
                "dns_search must be a string scalar or a sequence of string scalars",
            );
            return None;
        };
        let span = span_from_position(self.source_id, sequence.byte_range());
        let mut items = Vec::new();
        let mut seen = BTreeSet::new();
        for node in sequence.values() {
            let YamlNode::Scalar(scalar) = node else {
                self.unsupported_sequence_item(
                    DNS_SEARCH_EXPECTED_STRING,
                    &node,
                    field.span,
                    "dns_search entries must be string scalars",
                );
                continue;
            };
            if !matches!(
                ScalarValue::from_scalar(&scalar).scalar_type(),
                ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
            ) {
                self.unsupported_sequence_item(
                    DNS_SEARCH_EXPECTED_STRING,
                    &YamlNode::Scalar(scalar),
                    field.span,
                    "dns_search entries must be string scalars",
                );
                continue;
            }
            let item_span = span_from_position(self.source_id, scalar.byte_range());
            let search = scalar_string_from_source(&self.source, &scalar);
            if !seen.insert(search.clone()) {
                self.diagnostics.push(
                    Diagnostic::new(
                        DNS_SEARCH_DUPLICATE_ITEM,
                        Severity::Warning,
                        "dns_search schema entries are unique, but duplicate merge behavior is ambiguous",
                    )
                    .with_label(DiagnosticLabel::primary(
                        item_span,
                        "duplicate DNS search domain retained",
                    )),
                );
            }
            items.push(Located::new(search, item_span));
        }
        Some(DnsSearch::new(span, DnsSearchForm::List(items)))
    }

    fn parse_expose(&mut self, field: &ParsedField) -> Option<Expose> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(
                EXPOSE_EXPECTED_SEQUENCE,
                field,
                "expose must be a sequence of string or number scalars",
            );
            return None;
        };
        let span = span_from_position(self.source_id, sequence.byte_range());
        let mut items = Vec::new();
        let mut seen = Vec::new();
        for node in sequence.values() {
            let YamlNode::Scalar(scalar) = node else {
                self.unsupported_sequence_item(
                    EXPOSE_EXPECTED_SCALAR,
                    &node,
                    field.span,
                    "expose entries must be string or number scalars",
                );
                continue;
            };
            let scalar_kind = match ScalarValue::from_scalar(&scalar).scalar_type() {
                ScalarType::Integer | ScalarType::Float => ExposeScalarKind::Number,
                ScalarType::String | ScalarType::Timestamp | ScalarType::Regex => ExposeScalarKind::String,
                ScalarType::Null | ScalarType::Boolean => {
                    self.unsupported_sequence_item(
                        EXPOSE_EXPECTED_SCALAR,
                        &YamlNode::Scalar(scalar),
                        field.span,
                        "expose entries must be string or number scalars",
                    );
                    continue;
                }
            };
            let item_span = span_from_position(self.source_id, scalar.byte_range());
            let raw = scalar_string_from_source(&self.source, &scalar);
            if seen.contains(&(scalar_kind, raw.clone())) {
                self.diagnostics.push(
                    Diagnostic::new(
                        EXPOSE_DUPLICATE_ITEM,
                        Severity::Warning,
                        "expose entries must be unique by exact scalar identity",
                    )
                    .with_label(DiagnosticLabel::primary(
                        item_span,
                        "duplicate exposed-port item retained",
                    )),
                );
            } else {
                seen.push((scalar_kind, raw.clone()));
            }
            let item = ExposeItem::parse(Located::new(raw, item_span), scalar_kind);
            self.diagnose_expose_item(&item);
            items.push(item);
        }
        Some(Expose::new(span, items))
    }

    fn diagnose_expose_item(&mut self, item: &ExposeItem) {
        match item.kind() {
            ExposeItemKind::Documented { .. } | ExposeItemKind::Expression => {}
            ExposeItemKind::Sctp { .. } | ExposeItemKind::UnknownProtocol { .. } => {
                self.diagnostics.push(
                    Diagnostic::new(
                        EXPOSE_PROVIDER_DEPENDENT,
                        Severity::Warning,
                        "expose protocol is outside the documented portable `tcp` and `udp` set",
                    )
                    .with_label(DiagnosticLabel::primary(
                        item.span(),
                        "provider-dependent exposed-port protocol retained",
                    ))
                    .with_note("ComposeLens does not normalize or reject the raw protocol spelling"),
                );
            }
            ExposeItemKind::Malformed => {
                self.diagnostics.push(
                    Diagnostic::new(
                        EXPOSE_INVALID_ITEM,
                        Severity::Error,
                        "expose item must be a decimal port or range with an optional protocol",
                    )
                    .with_label(DiagnosticLabel::primary(
                        item.span(),
                        "malformed exposed-port item retained",
                    ))
                    .with_note("use `PORT`, `START-END`, `PORT/tcp`, or `PORT/udp` for documented portable syntax"),
                );
            }
        }
    }

    fn parse_security_options(&mut self, field: &ParsedField) -> Option<SecurityOptions> {
        let value = field.value.as_ref()?;
        let Some(sequence) = value.as_sequence() else {
            self.expected(
                SECURITY_OPT_EXPECTED_SEQUENCE,
                field,
                "security_opt must be a sequence of string scalars",
            );
            return None;
        };
        let span = span_from_position(self.source_id, sequence.byte_range());
        let mut items = Vec::new();
        let mut candidates = SecurityOptionCandidateCounts::default();
        for node in sequence.values() {
            let YamlNode::Scalar(scalar) = node else {
                self.unsupported_sequence_item(
                    SECURITY_OPT_EXPECTED_STRING,
                    &node,
                    field.span,
                    "security_opt entries must be string scalars",
                );
                continue;
            };
            if !matches!(
                ScalarValue::from_scalar(&scalar).scalar_type(),
                ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
            ) {
                self.unsupported_sequence_item(
                    SECURITY_OPT_EXPECTED_STRING,
                    &YamlNode::Scalar(scalar),
                    field.span,
                    "security_opt entries must be string scalars",
                );
                continue;
            }
            let item_span = span_from_position(self.source_id, scalar.byte_range());
            let raw = scalar_string_from_source(&self.source, &scalar);
            let item = SecurityOptionItem::parse(Located::new(raw, item_span));
            self.diagnose_security_option_item(item.kind(), item_span, &mut candidates);
            items.push(item);
        }
        Some(SecurityOptions::new(span, items))
    }

    fn diagnose_security_option_item(
        &mut self,
        kind: &SecurityOptionKind,
        span: SourceSpan,
        candidates: &mut SecurityOptionCandidateCounts,
    ) {
        let diagnostic = match kind {
            SecurityOptionKind::AppArmor { .. } => {
                candidates.apparmor += 1;
                (candidates.apparmor > 1).then(|| {
                    Diagnostic::new(
                        SECURITY_OPT_APPARMOR_CONFLICT,
                        Severity::Warning,
                        "multiple AppArmor candidates are retained; a consumer must resolve the conflict explicitly",
                    )
                    .with_label(DiagnosticLabel::primary(span, "additional AppArmor candidate retained"))
                })
            }
            SecurityOptionKind::AppArmorNearMiss => Some(
                Diagnostic::new(
                    SECURITY_OPT_APPARMOR_NEAR_MISS,
                    Severity::Warning,
                    "AppArmor candidates require exact lowercase `apparmor=<profile>` spelling without whitespace",
                )
                .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
            ),
            SecurityOptionKind::Seccomp { .. } => {
                candidates.seccomp += 1;
                (candidates.seccomp > 1).then(|| {
                    Diagnostic::new(
                        SECURITY_OPT_SECCOMP_CONFLICT,
                        Severity::Warning,
                        "multiple seccomp candidates are retained; a consumer must resolve the conflict explicitly",
                    )
                    .with_label(DiagnosticLabel::primary(span, "additional seccomp candidate retained"))
                })
            }
            SecurityOptionKind::SeccompNearMiss => Some(
                Diagnostic::new(
                    SECURITY_OPT_SECCOMP_NEAR_MISS,
                    Severity::Warning,
                    "seccomp candidates require exact lowercase `seccomp=<profile>` spelling without whitespace",
                )
                .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
            ),
            SecurityOptionKind::NoNewPrivileges { .. } => {
                candidates.no_new_privileges += 1;
                (candidates.no_new_privileges > 1).then(|| {
                    Diagnostic::new(
                        SECURITY_OPT_NO_NEW_PRIVILEGES_CONFLICT,
                        Severity::Warning,
                        "multiple no-new-privileges candidates are retained; a consumer must resolve the conflict explicitly",
                    )
                    .with_label(DiagnosticLabel::primary(
                        span,
                        "additional no-new-privileges candidate retained",
                    ))
                })
            }
            SecurityOptionKind::NoNewPrivilegesNearMiss => Some(
                Diagnostic::new(
                    SECURITY_OPT_NO_NEW_PRIVILEGES_NEAR_MISS,
                    Severity::Warning,
                    "no-new-privileges candidates require exact lowercase `no-new-privileges:true` or `no-new-privileges:false` spelling without whitespace",
                )
                .with_label(DiagnosticLabel::primary(span, "raw near-miss security option retained")),
            ),
            SecurityOptionKind::Mask { .. }
            | SecurityOptionKind::MaskNearMiss
            | SecurityOptionKind::Unmask { .. }
            | SecurityOptionKind::UnmaskNearMiss => security_path_option_diagnostic(kind, span),
            SecurityOptionKind::SecurityLabelDisable { .. }
            | SecurityOptionKind::SecurityLabelDisableNearMiss
            | SecurityOptionKind::SecurityLabelFileType { .. }
            | SecurityOptionKind::SecurityLabelFileTypeNearMiss
            | SecurityOptionKind::SecurityLabelLevel { .. }
            | SecurityOptionKind::SecurityLabelLevelNearMiss
            | SecurityOptionKind::SecurityLabelNested { .. }
            | SecurityOptionKind::SecurityLabelNestedNearMiss
            | SecurityOptionKind::SecurityLabelType { .. }
            | SecurityOptionKind::SecurityLabelTypeNearMiss => {
                authored_security_label_diagnostic(kind, span, candidates)
            }
            SecurityOptionKind::Empty => Some(
                Diagnostic::new(
                    SECURITY_OPT_EMPTY_ITEM,
                    Severity::Error,
                    "security_opt entries must not be empty strings",
                )
                .with_label(DiagnosticLabel::primary(span, "empty security option retained")),
            ),
            SecurityOptionKind::Expression | SecurityOptionKind::Other => None,
        };
        if let Some(diagnostic) = diagnostic {
            self.diagnostics.push(diagnostic);
        }
    }

    fn parse_tmpfs(&mut self, field: &ParsedField) -> Option<Tmpfs> {
        let value = field.value.as_ref()?;
        if let Some(scalar) = value.as_scalar() {
            if !matches!(
                ScalarValue::from_scalar(scalar).scalar_type(),
                ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
            ) {
                self.expected(
                    TMPFS_EXPECTED_FORM,
                    field,
                    "tmpfs must be a string scalar or a sequence of string scalars",
                );
                return None;
            }
            let span = span_from_position(self.source_id, scalar.byte_range());
            let item = TmpfsItem::parse(Located::new(scalar_string_from_source(&self.source, scalar), span));
            self.diagnose_tmpfs_item(&item);
            return Some(Tmpfs::new(span, TmpfsForm::Scalar(item)));
        }

        let Some(sequence) = value.as_sequence() else {
            self.expected(
                TMPFS_EXPECTED_FORM,
                field,
                "tmpfs must be a string scalar or a sequence of string scalars",
            );
            return None;
        };
        let span = span_from_position(self.source_id, sequence.byte_range());
        let mut items = Vec::new();
        for node in sequence.values() {
            let YamlNode::Scalar(scalar) = node else {
                self.unsupported_sequence_item(
                    TMPFS_EXPECTED_STRING,
                    &node,
                    field.span,
                    "tmpfs entries must be string scalars",
                );
                continue;
            };
            if !matches!(
                ScalarValue::from_scalar(&scalar).scalar_type(),
                ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
            ) {
                self.unsupported_sequence_item(
                    TMPFS_EXPECTED_STRING,
                    &YamlNode::Scalar(scalar),
                    field.span,
                    "tmpfs entries must be string scalars",
                );
                continue;
            }
            let item_span = span_from_position(self.source_id, scalar.byte_range());
            let raw = scalar_string_from_source(&self.source, &scalar);
            let item = TmpfsItem::parse(Located::new(raw, item_span));
            self.diagnose_tmpfs_item(&item);
            items.push(item);
        }
        Some(Tmpfs::new(span, TmpfsForm::List(items)))
    }

    fn diagnose_tmpfs_item(&mut self, item: &TmpfsItem) {
        if item.kind() != TmpfsItemKind::ProviderDependent {
            return;
        }
        self.diagnostics.push(
            Diagnostic::new(
                TMPFS_PROVIDER_DEPENDENT,
                Severity::Warning,
                "tmpfs item is malformed or uses provider- or target-specific options",
            )
            .with_label(DiagnosticLabel::primary(
                item.span(),
                "provider-dependent temporary-filesystem item",
            ))
            .with_note("use a non-empty path with only non-empty `mode`, `uid`, or `gid` assignments for documented portable syntax"),
        );
    }

    fn parse_sysctls(&mut self, field: &ParsedField) -> Option<Sysctls> {
        match field.value.as_ref() {
            Some(YamlNode::Mapping(mapping)) => {
                let span = span_from_position(self.source_id, mapping.byte_range());
                let mut entries = Vec::new();
                let mut seen = BTreeMap::new();
                for entry in self.fields(mapping) {
                    if self.record_duplicate(&mut seen, &entry) {
                        continue;
                    }
                    if entry.name.value.is_empty() {
                        self.diagnostics.push(
                            Diagnostic::new(
                                SYSCTLS_EMPTY_KEY,
                                Severity::Error,
                                "sysctls mapping keys must not be empty",
                            )
                            .with_label(DiagnosticLabel::primary(entry.name.span, "empty sysctl name")),
                        );
                        continue;
                    }
                    if entry.value.as_ref().is_some_and(|value| value.as_scalar().is_none()) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                SYSCTLS_EXPECTED_SCALAR,
                                Severity::Error,
                                "sysctls mapping values must be scalar strings, numbers, booleans, or null",
                            )
                            .with_label(DiagnosticLabel::primary(
                                entry.value_span.unwrap_or(entry.span),
                                "non-scalar sysctl value",
                            )),
                        );
                        continue;
                    }
                    let Some(value) = self.parse_compose_scalar(&entry, "sysctls mapping values must be scalars")
                    else {
                        continue;
                    };
                    entries.push(KeyValueEntry::new(entry.name, value, entry.span));
                }
                Some(Sysctls::new(span, SysctlsForm::Map(entries)))
            }
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let mut items = Vec::new();
                let mut seen = BTreeMap::new();
                for node in sequence.values() {
                    let YamlNode::Scalar(scalar) = node else {
                        self.unsupported_sequence_item(
                            SYSCTLS_EXPECTED_STRING,
                            &node,
                            field.span,
                            "sysctls list entries must be YAML string scalars",
                        );
                        continue;
                    };
                    if !matches!(
                        ScalarValue::from_scalar(&scalar).scalar_type(),
                        ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
                    ) {
                        self.unsupported_sequence_item(
                            SYSCTLS_EXPECTED_STRING,
                            &YamlNode::Scalar(scalar),
                            field.span,
                            "sysctls list entries must be YAML string scalars",
                        );
                        continue;
                    }
                    let item_span = span_from_position(self.source_id, scalar.byte_range());
                    let value = scalar_string_from_source(&self.source, &scalar);
                    if let Some(first) = seen.get(&value) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                SYSCTLS_DUPLICATE_ITEM,
                                Severity::Error,
                                "sysctls list entries must be unique exact strings",
                            )
                            .with_label(DiagnosticLabel::primary(item_span, "duplicate sysctl string"))
                            .with_label(DiagnosticLabel::secondary(*first, "first identical string")),
                        );
                    } else {
                        seen.insert(value.clone(), item_span);
                    }
                    items.push(Located::new(value, item_span));
                }
                Some(Sysctls::new(span, SysctlsForm::List(items)))
            }
            _ => {
                self.expected(
                    SYSCTLS_EXPECTED_FORM,
                    field,
                    "sysctls must be a mapping or a sequence of string scalars",
                );
                None
            }
        }
    }

    fn parse_logging(&mut self, field: &ParsedField) -> Option<Logging> {
        let Some(mapping) = field.value.as_ref().and_then(YamlNode::as_mapping) else {
            self.expected(
                LOGGING_EXPECTED_MAPPING,
                field,
                "logging must be a mapping with optional driver and options fields",
            );
            return None;
        };
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut logging = Logging::new(span);
        let mut seen = BTreeMap::new();
        for member in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &member);
            match member.name.value.as_str() {
                "driver" if !duplicate => {
                    if let Some(driver) = self.parse_logging_driver(&member) {
                        logging.set_driver(driver);
                    }
                }
                "options" if !duplicate => {
                    if let Some(options) = self.parse_logging_options(&member) {
                        logging.set_options(options);
                    }
                }
                name if name.starts_with("x-") => logging.push_extension(member.reference()),
                _ if duplicate => {}
                _ => logging.push_unknown(member.reference()),
            }
        }
        Some(logging)
    }

    fn parse_logging_driver(&mut self, field: &ParsedField) -> Option<Located<String>> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(
                LOGGING_DRIVER_EXPECTED_STRING,
                field,
                "logging driver must be a YAML string scalar",
            );
            return None;
        };
        if !matches!(
            ScalarValue::from_scalar(scalar).scalar_type(),
            ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
        ) {
            self.expected(
                LOGGING_DRIVER_EXPECTED_STRING,
                field,
                "logging driver must be a YAML string scalar",
            );
            return None;
        }
        let span = span_from_position(self.source_id, scalar.byte_range());
        Some(Located::new(scalar_string_from_source(&self.source, scalar), span))
    }

    fn parse_logging_options(&mut self, field: &ParsedField) -> Option<LoggingOptions> {
        let Some(mapping) = field.value.as_ref().and_then(YamlNode::as_mapping) else {
            self.expected(
                LOGGING_OPTIONS_EXPECTED_MAPPING,
                field,
                "logging options must be a mapping",
            );
            return None;
        };
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut entries = Vec::new();
        let mut unmodeled_entries = Vec::new();
        let mut seen = BTreeMap::new();
        for option in self.fields(mapping) {
            if self.record_duplicate(&mut seen, &option) {
                continue;
            }
            if option.name.value.is_empty() {
                self.diagnostics.push(
                    Diagnostic::new(
                        LOGGING_OPTION_EMPTY_KEY,
                        Severity::Error,
                        "logging option keys must not be empty",
                    )
                    .with_label(DiagnosticLabel::primary(option.name.span, "empty logging option key")),
                );
                unmodeled_entries.push(option.reference());
                continue;
            }
            let Some(value) = self.parse_logging_option_scalar(&option) else {
                unmodeled_entries.push(option.reference());
                continue;
            };
            entries.push(LoggingOption::new(option.name, value, option.span));
        }
        Some(LoggingOptions::new(span, entries, unmodeled_entries))
    }

    fn parse_logging_option_scalar(&mut self, field: &ParsedField) -> Option<Located<LoggingOptionValue>> {
        let Some(node) = field.value.as_ref() else {
            return Some(Located::new(LoggingOptionValue::Null, field.name.span));
        };
        let Some(scalar) = node.as_scalar() else {
            self.expected(
                LOGGING_OPTION_EXPECTED_SCALAR,
                field,
                "logging option values must be YAML string, number, or null scalars",
            );
            return None;
        };
        let span = span_from_position(self.source_id, scalar.byte_range());
        let scalar_value = ScalarValue::from_scalar(scalar);
        let value = match scalar_value.scalar_type() {
            ScalarType::Null => LoggingOptionValue::Null,
            ScalarType::Integer | ScalarType::Float => {
                LoggingOptionValue::Number(scalar_string_from_source(&self.source, scalar))
            }
            ScalarType::String | ScalarType::Timestamp | ScalarType::Regex => {
                LoggingOptionValue::String(scalar_string_from_source(&self.source, scalar))
            }
            ScalarType::Boolean => {
                self.diagnostics.push(
                    Diagnostic::new(
                        LOGGING_OPTION_EXPECTED_SCALAR,
                        Severity::Error,
                        "logging option values must be YAML string, number, or null scalars",
                    )
                    .with_label(DiagnosticLabel::primary(
                        span,
                        "boolean logging option retained as malformed",
                    )),
                );
                return None;
            }
        };
        Some(Located::new(value, span))
    }

    fn parse_restart_policy(&mut self, field: &ParsedField) -> Option<RestartPolicy> {
        let value = self.parse_string(field, "service restart policy")?;
        let policy = RestartPolicy::parse(value);
        if !policy.is_valid() {
            self.diagnostics.push(
                Diagnostic::new(
                    RESTART_INVALID_POLICY,
                    Severity::Error,
                    "restart must be `no`, `always`, `on-failure[:max-retries]`, `unless-stopped`, or interpolation",
                )
                .with_label(DiagnosticLabel::primary(
                    policy.raw().span(),
                    "invalid service restart policy",
                )),
            );
        }
        Some(policy)
    }

    fn parse_pids_limit(&mut self, field: &ParsedField) -> Option<PidsLimit> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(
                PIDS_LIMIT_EXPECTED_VALUE,
                field,
                "pids_limit must be a number or string scalar",
            );
            return None;
        };
        if matches!(
            ScalarValue::from_scalar(scalar).scalar_type(),
            ScalarType::Boolean | ScalarType::Null
        ) {
            self.expected(
                PIDS_LIMIT_EXPECTED_VALUE,
                field,
                "pids_limit must be a number or string scalar",
            );
            return None;
        }
        let span = span_from_position(self.source_id, scalar.byte_range());
        let limit = PidsLimit::parse(Located::new(scalar_string_from_source(&self.source, scalar), span));
        match limit.kind() {
            PidsLimitKind::Zero => self.diagnostics.push(
                Diagnostic::new(
                    PIDS_LIMIT_AMBIGUOUS_ZERO,
                    Severity::Warning,
                    "pids_limit zero is preserved as an ambiguous and unportable native state",
                )
                .with_label(DiagnosticLabel::primary(span, "ambiguous zero PID limit")),
            ),
            PidsLimitKind::Other => self.diagnostics.push(
                Diagnostic::new(
                    PIDS_LIMIT_INVALID,
                    Severity::Error,
                    "pids_limit must be `-1`, a positive integral decimal, or interpolation",
                )
                .with_label(DiagnosticLabel::primary(span, "unsupported service PID limit")),
            ),
            _ => {}
        }
        Some(limit)
    }

    fn parse_shm_size(&mut self, field: &ParsedField) -> Option<ShmSize> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(
                SHM_SIZE_EXPECTED_VALUE,
                field,
                "shm_size must be a YAML number or string scalar",
            );
            return None;
        };
        let scalar_kind = match ScalarValue::from_scalar(scalar).scalar_type() {
            ScalarType::Integer | ScalarType::Float => ShmSizeScalarKind::Number,
            ScalarType::String | ScalarType::Timestamp | ScalarType::Regex => ShmSizeScalarKind::String,
            ScalarType::Boolean | ScalarType::Null => {
                self.expected(
                    SHM_SIZE_EXPECTED_VALUE,
                    field,
                    "shm_size must be a YAML number or string scalar",
                );
                return None;
            }
        };
        let span = span_from_position(self.source_id, scalar.byte_range());
        let size = ShmSize::parse(
            Located::new(scalar_string_from_source(&self.source, scalar), span),
            scalar_kind,
        );
        self.diagnose_shm_size(&size);
        Some(size)
    }

    fn diagnose_shm_size(&mut self, size: &ShmSize) {
        let (code, message, label, note) = match size.kind() {
            ShmSizeKind::Zero { .. } => (
                SHM_SIZE_AMBIGUOUS_ZERO,
                "shm_size zero is preserved because Compose does not define its semantics",
                "ambiguous zero shared-memory size",
                "choose a positive size with an explicit documented lowercase unit",
            ),
            ShmSizeKind::ProviderDependentNumber => (
                SHM_SIZE_PROVIDER_DEPENDENT_NUMBER,
                "numeric shm_size is schema-accepted but lacks a documented explicit unit",
                "provider-dependent numeric shared-memory size",
                "use a positive quoted value with `b`, `k`, `kb`, `m`, `mb`, `g`, or `gb` for portable intent",
            ),
            ShmSizeKind::ProviderDependentString => (
                SHM_SIZE_PROVIDER_DEPENDENT_STRING,
                "string shm_size is schema-accepted but falls outside the documented lowercase suffix family",
                "provider-dependent string shared-memory size",
                "use an explicit lowercase `b`, `k`, `kb`, `m`, `mb`, `g`, or `gb` suffix when that is the intended unit",
            ),
            ShmSizeKind::Documented { .. } | ShmSizeKind::Expression => return,
        };
        self.diagnostics.push(
            Diagnostic::new(code, Severity::Warning, message)
                .with_label(DiagnosticLabel::primary(size.raw().span(), label))
                .with_note(note),
        );
    }

    fn parse_mem_limit(&mut self, field: &ParsedField) -> Option<MemLimit> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(
                MEM_LIMIT_EXPECTED_VALUE,
                field,
                "mem_limit must be a YAML number or string scalar",
            );
            return None;
        };
        let scalar_kind = match ScalarValue::from_scalar(scalar).scalar_type() {
            ScalarType::Integer | ScalarType::Float => MemLimitScalarKind::Number,
            ScalarType::String | ScalarType::Timestamp | ScalarType::Regex => MemLimitScalarKind::String,
            ScalarType::Boolean | ScalarType::Null => {
                self.expected(
                    MEM_LIMIT_EXPECTED_VALUE,
                    field,
                    "mem_limit must be a YAML number or string scalar",
                );
                return None;
            }
        };
        let span = span_from_position(self.source_id, scalar.byte_range());
        let limit = MemLimit::parse(
            Located::new(scalar_string_from_source(&self.source, scalar), span),
            scalar_kind,
        );
        self.diagnose_mem_limit(&limit);
        Some(limit)
    }

    fn diagnose_mem_limit(&mut self, limit: &MemLimit) {
        let (code, message, label, note) = match limit.kind() {
            MemLimitKind::Zero { .. } => (
                MEM_LIMIT_AMBIGUOUS_ZERO,
                "mem_limit zero is preserved without inferring portable runtime behavior",
                "ambiguous zero memory limit",
                "choose a positive size with an explicit documented lowercase unit",
            ),
            MemLimitKind::SchemaNumber => (
                MEM_LIMIT_SCHEMA_NUMBER,
                "numeric mem_limit is schema-accepted but lacks a documented explicit unit",
                "schema-only numeric memory limit",
                "use a positive quoted value with `b`, `k`, `kb`, `m`, `mb`, `g`, or `gb` for explicit intent",
            ),
            MemLimitKind::ProviderDependentString => (
                MEM_LIMIT_PROVIDER_DEPENDENT_STRING,
                "string mem_limit is schema-accepted but falls outside the documented lowercase suffix family",
                "provider-dependent string memory limit",
                "use an explicit lowercase `b`, `k`, `kb`, `m`, `mb`, `g`, or `gb` suffix when that is the intended unit",
            ),
            MemLimitKind::Documented { .. } | MemLimitKind::Expression => return,
        };
        self.diagnostics.push(
            Diagnostic::new(code, Severity::Warning, message)
                .with_label(DiagnosticLabel::primary(limit.raw().span(), label))
                .with_note(note),
        );
    }

    fn parse_pull_policy(&mut self, field: &ParsedField) -> Option<PullPolicy> {
        let value = self.parse_string(field, "service pull policy")?;
        let policy = PullPolicy::parse(value);
        if !policy.is_recognized() {
            self.diagnostics.push(
                Diagnostic::new(
                    PULL_POLICY_INVALID,
                    Severity::Error,
                    "pull_policy must be a documented Compose policy, the retained `if_not_present` alias, schema-only `refresh`, an `every_` interval matching integer `w`, `d`, `h`, `m`, and `s` components, or interpolation",
                )
                .with_label(DiagnosticLabel::primary(
                    policy.raw().span(),
                    "invalid or provider-specific service pull policy",
                )),
            );
        }
        Some(policy)
    }

    fn parse_stop_grace_period(&mut self, field: &ParsedField) -> Option<Located<StopGracePeriod>> {
        let value = self.parse_string(field, "service stop grace period")?;
        let period = StopGracePeriod::parse(value.value);
        if !period.is_valid() {
            self.diagnostics.push(
                Diagnostic::new(
                    STOP_GRACE_PERIOD_INVALID,
                    Severity::Error,
                    "stop_grace_period must match the ComposeLens duration policy using `us`, `ms`, `s`, `m`, or `h`, or contain an interpolation marker",
                )
                .with_label(DiagnosticLabel::primary(
                    value.span,
                    "invalid service stop grace period",
                )),
            );
        }
        Some(Located::new(period, value.span))
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

    fn parse_entrypoint(&mut self, field: &ParsedField) -> Option<Entrypoint> {
        match field.value.as_ref() {
            Some(YamlNode::Scalar(scalar)) => {
                let span = span_from_position(self.source_id, scalar.byte_range());
                if ScalarValue::from_scalar(scalar).scalar_type() == ScalarType::Null {
                    Some(Entrypoint::Null(span))
                } else {
                    Some(Entrypoint::String(Located::new(
                        scalar_string_from_source(&self.source, scalar),
                        span,
                    )))
                }
            }
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let values =
                    self.parse_scalar_nodes(sequence.values(), field.span, "entrypoint list items must be scalars");
                Some(Entrypoint::List { span, values })
            }
            _ => {
                self.expected(
                    EXPECTED_FIELD_FORM,
                    field,
                    "entrypoint must be null, a scalar, or a sequence",
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

    fn parse_environment_files(&mut self, field: &ParsedField) -> Vec<EnvironmentFile> {
        match field.value.as_ref() {
            Some(YamlNode::Scalar(_)) => self
                .parse_string(field, "service environment-file path")
                .map(EnvironmentFile::Short)
                .into_iter()
                .collect(),
            Some(YamlNode::Sequence(sequence)) => sequence
                .values()
                .filter_map(|value| match value {
                    YamlNode::Scalar(scalar) => {
                        let span = span_from_position(self.source_id, scalar.byte_range());
                        Some(EnvironmentFile::Short(Located::new(
                            scalar_string_from_source(&self.source, &scalar),
                            span,
                        )))
                    }
                    YamlNode::Mapping(mapping) => Some(EnvironmentFile::Long(Box::new(
                        self.parse_long_environment_file(&mapping),
                    ))),
                    _ => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                ENVIRONMENT_FILE_EXPECTED_FORM,
                                Severity::Error,
                                "env_file item must use scalar short syntax or mapping long syntax",
                            )
                            .with_label(DiagnosticLabel::primary(
                                node_span(self.source_id, &value).unwrap_or(field.span),
                                "invalid environment-file item",
                            )),
                        );
                        None
                    }
                })
                .collect(),
            _ => {
                self.expected(
                    EXPECTED_FIELD_FORM,
                    field,
                    "env_file must be a scalar path or a sequence of short/long entries",
                );
                Vec::new()
            }
        }
    }

    fn parse_long_environment_file(&mut self, mapping: &Mapping) -> LongEnvironmentFile {
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut environment_file = LongEnvironmentFile::new(span);
        let mut seen = BTreeMap::new();
        for field in self.fields(mapping) {
            let duplicate = self.record_duplicate(&mut seen, &field);
            match field.name.value.as_str() {
                "path" if !duplicate => self
                    .parse_string(&field, "environment-file path")
                    .into_iter()
                    .for_each(|value| environment_file.set_path(value)),
                "required" if !duplicate => self
                    .parse_boolean(&field, "environment-file required option")
                    .into_iter()
                    .for_each(|value| environment_file.set_required(value)),
                "format" if !duplicate => {
                    if let Some(raw) = self.parse_string(&field, "environment-file format") {
                        let format = EnvironmentFileFormat::parse(raw);
                        if !format.is_valid() {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    ENVIRONMENT_FILE_INVALID_FORMAT,
                                    Severity::Error,
                                    "environment-file format must be `raw` or interpolation",
                                )
                                .with_label(DiagnosticLabel::primary(format.raw().span(), "invalid format")),
                            );
                        }
                        environment_file.set_format(format);
                    }
                }
                name if name.starts_with("x-") => environment_file.push_extension(field.reference()),
                _ if duplicate => {}
                _ => environment_file.push_unknown(field.reference()),
            }
        }
        if environment_file.path().is_none() {
            self.missing(
                ENVIRONMENT_FILE_MISSING_PATH,
                span,
                "long environment-file entry is missing `path`",
            );
        }
        environment_file
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
            if !valid_ulimit_name(limit.name.value()) {
                self.diagnostics.push(
                    Diagnostic::new(
                        ULIMIT_INVALID_NAME,
                        Severity::Error,
                        "ulimit names must contain only lowercase ASCII letters",
                    )
                    .with_label(DiagnosticLabel::primary(limit.name.span, "invalid ulimit name")),
                );
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
        if range.soft().is_none() {
            self.missing(
                ULIMIT_MISSING_RANGE_MEMBER,
                span,
                "ulimit range is missing required `soft`",
            );
        }
        if range.hard().is_none() {
            self.missing(
                ULIMIT_MISSING_RANGE_MEMBER,
                span,
                "ulimit range is missing required `hard`",
            );
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
                let mut definition = BuildDefinition::new(span_from_position(self.source_id, mapping.byte_range()));
                let mut seen = BTreeMap::new();
                let (mut dockerfile, mut dockerfile_inline) = (None, None);
                for option in self.fields(mapping) {
                    if self.record_duplicate(&mut seen, &option) {
                        continue;
                    }
                    if let Some(kind) = BuildFieldKind::from_name(option.name.value()) {
                        definition.push_field(BuildField::new(kind, option.reference()));
                        match kind {
                            BuildFieldKind::AdditionalContexts => {
                                definition.set_additional_contexts(self.parse_build_additional_contexts(&option));
                            }
                            BuildFieldKind::Args => {
                                if let Some(args) = self.parse_build_args(&option) {
                                    definition.set_args(args);
                                }
                            }
                            BuildFieldKind::CacheFrom | BuildFieldKind::CacheTo => {
                                self.set_build_cache_locations(&mut definition, &option, kind);
                            }
                            BuildFieldKind::Entitlements => {
                                if let Some(entitlements) = self.parse_build_entitlements(&option) {
                                    definition.set_entitlements(entitlements);
                                }
                            }
                            BuildFieldKind::ExtraHosts => {
                                if let Some(extra_hosts) = self.parse_build_extra_hosts(&option) {
                                    definition.set_extra_hosts(extra_hosts);
                                }
                            }
                            BuildFieldKind::Context => {
                                if let Some(context) = self.parse_string(&option, "build context") {
                                    definition.set_context(context);
                                }
                            }
                            BuildFieldKind::Dockerfile => {
                                dockerfile = Some(self.parse_build_dockerfile(&mut definition, &option));
                            }
                            BuildFieldKind::DockerfileInline => {
                                self.set_build_dockerfile_inline(&mut definition, &option, &mut dockerfile_inline);
                            }
                            BuildFieldKind::Target => {
                                if let Some(target) = self.parse_string(&option, "build target") {
                                    definition.set_target(target);
                                }
                            }
                            BuildFieldKind::Network => {
                                if let Some(network) = self.parse_string(&option, "build network") {
                                    definition.set_network(network);
                                }
                            }
                            BuildFieldKind::Isolation => self.set_build_isolation(&mut definition, &option),
                            BuildFieldKind::Platforms => {
                                if let Some(platforms) = self.parse_build_platforms(&option) {
                                    definition.set_platforms(platforms);
                                }
                            }
                            BuildFieldKind::NoCache => self.set_build_no_cache(&mut definition, &option),
                            BuildFieldKind::NoCacheFilter => self.set_build_no_cache_filter(&mut definition, &option),
                            BuildFieldKind::Privileged => self.set_build_privileged(&mut definition, &option),
                            BuildFieldKind::Sbom => self.set_build_sbom(&mut definition, &option),
                            BuildFieldKind::Provenance => self.set_build_provenance(&mut definition, &option),
                            BuildFieldKind::Pull => {
                                if let Some(pull) = self.parse_boolean(&option, "build pull") {
                                    definition.set_pull(pull);
                                }
                            }
                            BuildFieldKind::ShmSize => self.set_build_shm_size(&mut definition, &option),
                            BuildFieldKind::Tags => {
                                if let Some(tags) = self.parse_build_tags(&option) {
                                    definition.set_tags(tags);
                                }
                            }
                            BuildFieldKind::Labels => {
                                if let Some(labels) = self.parse_labels(&option) {
                                    definition.set_labels(labels);
                                }
                            }
                            BuildFieldKind::Secrets => self
                                .parse_secret_grants(&option)
                                .into_iter()
                                .for_each(|secrets| definition.set_secrets(secrets)),
                            BuildFieldKind::Ssh => self.set_build_ssh(&mut definition, &option),
                            BuildFieldKind::Ulimits => self.set_build_ulimits(&mut definition, &option),
                        }
                    } else if option.name.value().starts_with("x-") {
                        definition.push_extension(option.reference());
                    } else {
                        definition.push_unknown(option.reference());
                    }
                }
                self.report_build_dockerfile_conflict(dockerfile, dockerfile_inline);
                Some(Build::Definition(definition))
            }
            _ => self.invalid_build_form(field),
        }
    }

    fn invalid_build_form(&mut self, field: &ParsedField) -> Option<Build> {
        self.expected(EXPECTED_FIELD_FORM, field, "build must be a scalar context or mapping");
        None
    }

    fn report_build_dockerfile_conflict(
        &mut self,
        dockerfile: Option<FieldReference>,
        dockerfile_inline: Option<FieldReference>,
    ) {
        let (Some(dockerfile), Some(dockerfile_inline)) = (dockerfile, dockerfile_inline) else {
            return;
        };
        self.diagnostics.push(
            Diagnostic::new(
                BUILD_DOCKERFILE_INLINE_CONFLICT,
                Severity::Error,
                "build `dockerfile` and `dockerfile_inline` are mutually exclusive",
            )
            .with_label(DiagnosticLabel::primary(dockerfile.span(), "dockerfile retained"))
            .with_label(DiagnosticLabel::secondary(
                dockerfile_inline.span(),
                "dockerfile_inline retained",
            )),
        );
    }

    fn set_build_shm_size(&mut self, definition: &mut BuildDefinition, field: &ParsedField) {
        if let Some(shm_size) = self.parse_shm_size(field) {
            definition.set_shm_size(shm_size);
        }
    }

    fn set_build_ulimits(&mut self, definition: &mut BuildDefinition, field: &ParsedField) {
        if let Some(ulimits) = self.parse_ulimits(field) {
            definition.set_ulimits(ulimits);
        }
    }

    fn parse_build_tags(&mut self, field: &ParsedField) -> Option<Vec<Located<String>>> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(EXPECTED_SEQUENCE, field, "build tags must be a sequence of scalars");
            return None;
        };
        Some(self.parse_scalar_nodes(
            sequence.values(),
            field.span,
            "build tag entries must be non-null scalars",
        ))
    }

    fn parse_build_entitlements(&mut self, field: &ParsedField) -> Option<Vec<Located<String>>> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(
                EXPECTED_SEQUENCE,
                field,
                "build entitlements must be a sequence of string scalars",
            );
            return None;
        };
        Some(self.parse_string_scalar_nodes(
            sequence.values(),
            field.span,
            "build entitlement entries must be string scalars",
        ))
    }

    fn parse_build_cache_locations(&mut self, field: &ParsedField, name: &str) -> Option<Vec<Located<String>>> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(
                EXPECTED_SEQUENCE,
                field,
                format!("build {name} must be a sequence of string scalars"),
            );
            return None;
        };
        Some(self.parse_string_scalar_nodes(
            sequence.values(),
            field.span,
            format!("build {name} entries must be string scalars"),
        ))
    }

    fn set_build_cache_locations(
        &mut self,
        definition: &mut BuildDefinition,
        field: &ParsedField,
        kind: BuildFieldKind,
    ) {
        let name = if kind == BuildFieldKind::CacheFrom {
            "cache_from"
        } else {
            "cache_to"
        };
        if let Some(locations) = self.parse_build_cache_locations(field, name) {
            if kind == BuildFieldKind::CacheFrom {
                definition.set_cache_from(locations);
            } else {
                definition.set_cache_to(locations);
            }
        }
    }

    fn parse_build_extra_hosts(&mut self, field: &ParsedField) -> Option<BuildExtraHosts> {
        match field.value.as_ref() {
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let mut values = Vec::new();
                let mut seen = BTreeSet::new();
                for node in sequence.values() {
                    let YamlNode::Scalar(scalar) = node else {
                        self.unsupported_sequence_item(
                            BUILD_EXTRA_HOSTS_EXPECTED_STRING,
                            &node,
                            field.span,
                            "build extra_hosts list entries must be string scalars",
                        );
                        continue;
                    };
                    if !matches!(
                        ScalarValue::from_scalar(&scalar).scalar_type(),
                        ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
                    ) {
                        self.unsupported_sequence_item(
                            BUILD_EXTRA_HOSTS_EXPECTED_STRING,
                            &YamlNode::Scalar(scalar),
                            field.span,
                            "build extra_hosts list entries must be string scalars",
                        );
                        continue;
                    }
                    let raw = scalar_string_from_source(&self.source, &scalar);
                    let item_span = span_from_position(self.source_id, scalar.byte_range());
                    if !seen.insert(raw.clone()) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                BUILD_EXTRA_HOSTS_DUPLICATE_ITEM,
                                Severity::Error,
                                "build extra_hosts list entries must be unique raw strings",
                            )
                            .with_label(DiagnosticLabel::primary(item_span, "duplicate entry retained")),
                        );
                    }
                    values.push(Located::new(raw, item_span));
                }
                Some(BuildExtraHosts::List { span, values })
            }
            Some(YamlNode::Mapping(mapping)) => {
                let span = span_from_position(self.source_id, mapping.byte_range());
                let mut entries = Vec::new();
                let mut seen = BTreeMap::new();
                for entry in self.fields(mapping) {
                    if self.record_duplicate(&mut seen, &entry) {
                        continue;
                    }
                    let Some(addresses) = self.parse_build_extra_host_addresses(&entry) else {
                        continue;
                    };
                    entries.push(BuildExtraHostEntry::new(entry.name, addresses, entry.span));
                }
                Some(BuildExtraHosts::Map { span, entries })
            }
            _ => {
                self.expected(
                    BUILD_EXTRA_HOSTS_EXPECTED_FORM,
                    field,
                    "build extra_hosts must be a sequence or mapping",
                );
                None
            }
        }
    }

    fn parse_build_extra_host_addresses(&mut self, field: &ParsedField) -> Option<BuildExtraHostAddresses> {
        match field.value.as_ref() {
            Some(YamlNode::Scalar(scalar)) => {
                if !matches!(
                    ScalarValue::from_scalar(scalar).scalar_type(),
                    ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
                ) {
                    self.expected(
                        BUILD_EXTRA_HOSTS_EXPECTED_STRING,
                        field,
                        "build extra_hosts mapping addresses must be string scalars or sequences of string scalars",
                    );
                    return None;
                }
                let span = span_from_position(self.source_id, scalar.byte_range());
                Some(BuildExtraHostAddresses::Scalar(Located::new(
                    scalar_string_from_source(&self.source, scalar),
                    span,
                )))
            }
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let values = self.parse_string_scalar_nodes(
                    sequence.values(),
                    field.span,
                    "build extra_hosts mapping address lists must contain string scalars",
                );
                Some(BuildExtraHostAddresses::List { span, values })
            }
            _ => {
                self.expected(
                    BUILD_EXTRA_HOSTS_EXPECTED_STRING,
                    field,
                    "build extra_hosts mapping addresses must be string scalars or sequences of string scalars",
                );
                None
            }
        }
    }

    fn parse_build_additional_contexts(&mut self, field: &ParsedField) -> Option<BuildAdditionalContexts> {
        match field.value.as_ref() {
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let values = self.parse_string_scalar_nodes(
                    sequence.values(),
                    field.span,
                    "build additional context list entries must be string scalars",
                );
                Some(BuildAdditionalContexts::List { span, values })
            }
            Some(YamlNode::Mapping(mapping)) => {
                let span = span_from_position(self.source_id, mapping.byte_range());
                let entries = self.parse_scalar_mapping(field, "build additional contexts");
                Some(BuildAdditionalContexts::Map { span, entries })
            }
            _ => {
                self.expected(
                    EXPECTED_FIELD_FORM,
                    field,
                    "build additional_contexts must be a sequence or mapping",
                );
                None
            }
        }
    }

    fn parse_build_platforms(&mut self, field: &ParsedField) -> Option<Vec<Located<String>>> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(
                EXPECTED_SEQUENCE,
                field,
                "build platforms must be a sequence of scalars",
            );
            return None;
        };
        Some(self.parse_scalar_nodes(
            sequence.values(),
            field.span,
            "build platform entries must be non-null scalars",
        ))
    }

    fn parse_build_args(&mut self, field: &ParsedField) -> Option<BuildArgs> {
        match field.value.as_ref() {
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let values = self.parse_string_scalar_nodes(
                    sequence.values(),
                    field.span,
                    "build argument list entries must be string scalars",
                );
                Some(BuildArgs::List { span, values })
            }
            Some(YamlNode::Mapping(mapping)) => {
                let span = span_from_position(self.source_id, mapping.byte_range());
                let entries = self.parse_scalar_mapping(field, "build arguments");
                Some(BuildArgs::Map { span, entries })
            }
            _ => {
                self.expected(EXPECTED_FIELD_FORM, field, "build args must be a sequence or mapping");
                None
            }
        }
    }

    fn parse_build_ssh(&mut self, field: &ParsedField) -> Option<BuildSsh> {
        match field.value.as_ref() {
            Some(YamlNode::Sequence(sequence)) => {
                let span = span_from_position(self.source_id, sequence.byte_range());
                let mut values = Vec::new();
                let mut seen = BTreeSet::new();
                for node in sequence.values() {
                    let YamlNode::Scalar(scalar) = node else {
                        self.unsupported_sequence_item(
                            BUILD_SSH_EXPECTED_FORM,
                            &node,
                            field.span,
                            "build ssh list entries must be string scalars",
                        );
                        continue;
                    };
                    if !matches!(
                        ScalarValue::from_scalar(&scalar).scalar_type(),
                        ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
                    ) {
                        self.unsupported_sequence_item(
                            BUILD_SSH_EXPECTED_FORM,
                            &YamlNode::Scalar(scalar),
                            field.span,
                            "build ssh list entries must be string scalars",
                        );
                        continue;
                    }
                    let value = scalar_string_from_source(&self.source, &scalar);
                    let span = span_from_position(self.source_id, scalar.byte_range());
                    if !seen.insert(value.clone()) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                BUILD_SSH_DUPLICATE_ITEM,
                                Severity::Error,
                                "build ssh list entries must be unique",
                            )
                            .with_label(DiagnosticLabel::primary(span, "duplicate SSH entry retained")),
                        );
                    }
                    values.push(Located::new(value, span));
                }
                Some(BuildSsh::list(span, values))
            }
            Some(YamlNode::Mapping(mapping)) => {
                let span = span_from_position(self.source_id, mapping.byte_range());
                let mut entries = Vec::new();
                let mut seen = BTreeMap::new();
                for entry in self.fields(mapping) {
                    if self.record_duplicate(&mut seen, &entry) {
                        continue;
                    }
                    let key_span = entry.name.span;
                    let Some(value) = entry.value.as_ref() else {
                        entries.push(KeyValueEntry::new(
                            entry.name,
                            Located::new(ComposeScalar::Null, key_span),
                            entry.span,
                        ));
                        continue;
                    };
                    let Some(scalar) = value.as_scalar() else {
                        self.expected(
                            BUILD_SSH_EXPECTED_FORM,
                            &entry,
                            "build ssh mapping values must be scalars or null",
                        );
                        continue;
                    };
                    let scalar_span = span_from_position(self.source_id, scalar.byte_range());
                    let scalar_value = ScalarValue::from_scalar(scalar);
                    let value = match scalar_value.scalar_type() {
                        ScalarType::Null => ComposeScalar::Null,
                        ScalarType::Boolean => ComposeScalar::Boolean(scalar_value.to_bool().unwrap_or(false)),
                        ScalarType::Integer | ScalarType::Float => {
                            ComposeScalar::Number(scalar_string_from_source(&self.source, scalar))
                        }
                        ScalarType::String | ScalarType::Timestamp | ScalarType::Regex => {
                            ComposeScalar::String(scalar_string_from_source(&self.source, scalar))
                        }
                    };
                    entries.push(KeyValueEntry::new(
                        entry.name,
                        Located::new(value, scalar_span),
                        entry.span,
                    ));
                }
                Some(BuildSsh::map(span, entries))
            }
            _ => {
                self.expected(
                    BUILD_SSH_EXPECTED_FORM,
                    field,
                    "build ssh must be a sequence or mapping",
                );
                None
            }
        }
    }

    fn set_build_ssh(&mut self, definition: &mut BuildDefinition, field: &ParsedField) {
        if let Some(ssh) = self.parse_build_ssh(field) {
            definition.set_ssh(ssh);
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
                match kind {
                    DeployFieldKind::EndpointMode => self.set_deploy_endpoint_mode(&mut definition, &option),
                    DeployFieldKind::Labels => self
                        .parse_labels(&option)
                        .into_iter()
                        .for_each(|labels| definition.set_labels(labels)),
                    DeployFieldKind::Mode => self.set_deploy_mode(&mut definition, &option),
                    DeployFieldKind::Placement => self
                        .parse_deploy_placement(&option)
                        .into_iter()
                        .for_each(|value| definition.set_placement(value)),
                    DeployFieldKind::Replicas => self.set_deploy_replicas(&mut definition, &option),
                    DeployFieldKind::RestartPolicy => self
                        .parse_deploy_restart_policy(&option)
                        .into_iter()
                        .for_each(|value| definition.set_restart_policy(value)),
                    _ => {}
                }
            } else if option.name.value().starts_with("x-") {
                definition.push_extension(option.reference());
            } else {
                definition.push_unknown(option.reference());
            }
        }
        Some(definition)
    }

    fn set_deploy_endpoint_mode(&mut self, definition: &mut DeployDefinition, field: &ParsedField) {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(
                EXPECTED_SCALAR,
                field,
                "deploy endpoint_mode must be a YAML string scalar",
            );
            return;
        };
        if ScalarValue::from_scalar(scalar).scalar_type() != ScalarType::String {
            self.expected(
                EXPECTED_SCALAR,
                field,
                "deploy endpoint_mode must be a YAML string scalar",
            );
            return;
        }
        let endpoint_mode = Located::new(
            DeployEndpointMode::parse(scalar_string_from_source(&self.source, scalar)),
            span_from_position(self.source_id, scalar.byte_range()),
        );
        if !endpoint_mode.value().is_documented() {
            self.diagnostics.push(
                Diagnostic::new(
                    DEPLOY_ENDPOINT_MODE_PORTABILITY,
                    Severity::Warning,
                    "deploy endpoint_mode is outside Compose's documented portable values",
                )
                .with_label(DiagnosticLabel::primary(
                    endpoint_mode.span(),
                    "retained provider-specific endpoint mode",
                )),
            );
        }
        definition.set_endpoint_mode(endpoint_mode);
    }

    fn set_deploy_mode(&mut self, definition: &mut DeployDefinition, field: &ParsedField) {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(EXPECTED_SCALAR, field, "deploy mode must be a YAML string scalar");
            return;
        };
        if ScalarValue::from_scalar(scalar).scalar_type() != ScalarType::String {
            self.expected(EXPECTED_SCALAR, field, "deploy mode must be a YAML string scalar");
            return;
        }
        let mode = Located::new(
            DeployMode::parse(scalar_string_from_source(&self.source, scalar)),
            span_from_position(self.source_id, scalar.byte_range()),
        );
        if !mode.value().is_documented() {
            self.diagnostics.push(
                Diagnostic::new(
                    DEPLOY_MODE_PORTABILITY,
                    Severity::Warning,
                    "deploy mode is outside Compose's documented portable values",
                )
                .with_label(DiagnosticLabel::primary(
                    mode.span(),
                    "retained provider-specific deploy mode",
                )),
            );
        }
        definition.set_mode(mode);
    }

    fn set_deploy_replicas(&mut self, definition: &mut DeployDefinition, field: &ParsedField) {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(
                EXPECTED_SCALAR,
                field,
                "deploy replicas must be a YAML number or string scalar",
            );
            return;
        };
        let value = match ScalarValue::from_scalar(scalar).scalar_type() {
            ScalarType::Integer | ScalarType::Float => {
                DeployReplicas::YamlNumber(scalar_string_from_source(&self.source, scalar))
            }
            ScalarType::String => DeployReplicas::String(scalar_string_from_source(&self.source, scalar)),
            ScalarType::Boolean | ScalarType::Null | ScalarType::Timestamp | ScalarType::Regex => {
                self.expected(
                    EXPECTED_SCALAR,
                    field,
                    "deploy replicas must be a YAML number or string scalar",
                );
                return;
            }
        };
        definition.set_replicas(Located::new(
            value,
            span_from_position(self.source_id, scalar.byte_range()),
        ));
    }

    fn parse_deploy_restart_policy(&mut self, field: &ParsedField) -> Option<DeployRestartPolicy> {
        let Some(mapping) = field.value.as_ref().and_then(YamlNode::as_mapping) else {
            self.expected(EXPECTED_MAPPING, field, "deploy restart_policy must be a mapping");
            return None;
        };
        let mut policy = DeployRestartPolicy::new(span_from_position(self.source_id, mapping.byte_range()));
        let mut seen = BTreeMap::new();
        for option in self.fields(mapping) {
            if self.record_duplicate(&mut seen, &option) {
                continue;
            }
            match option.name.value().as_str() {
                name if name.starts_with("x-") => {
                    policy.push_extension(option.reference());
                    continue;
                }
                "condition" | "delay" | "max_attempts" | "window" => {}
                _ => {
                    policy.push_unknown(option.reference());
                    continue;
                }
            }
            let Some(scalar) = option.value.as_ref().and_then(YamlNode::as_scalar) else {
                self.expected(EXPECTED_SCALAR, &option, "deploy restart-policy members must be scalar");
                continue;
            };
            let span = span_from_position(self.source_id, scalar.byte_range());
            match option.name.value().as_str() {
                "condition" if ScalarValue::from_scalar(scalar).scalar_type() == ScalarType::String => policy
                    .set_condition(Located::new(
                        DeployRestartCondition::parse(scalar_string_from_source(&self.source, scalar)),
                        span,
                    )),
                "delay" | "window" if ScalarValue::from_scalar(scalar).scalar_type() == ScalarType::String => {
                    let value = Located::new(
                        DeployRestartDuration::new(scalar_string_from_source(&self.source, scalar)),
                        span,
                    );
                    if option.name.value() == "delay" {
                        policy.set_delay(value);
                    } else {
                        policy.set_window(value);
                    }
                }
                "max_attempts" => match ScalarValue::from_scalar(scalar).scalar_type() {
                    ScalarType::Integer => policy.set_max_attempts(Located::new(
                        DeployRestartMaxAttempts::YamlNumber(scalar_string_from_source(&self.source, scalar)),
                        span,
                    )),
                    ScalarType::String => policy.set_max_attempts(Located::new(
                        DeployRestartMaxAttempts::String(scalar_string_from_source(&self.source, scalar)),
                        span,
                    )),
                    _ => self.expected(
                        EXPECTED_SCALAR,
                        &option,
                        "deploy restart-policy max_attempts must be a YAML integer or string scalar",
                    ),
                },
                "condition" | "delay" | "window" => self.expected(
                    EXPECTED_SCALAR,
                    &option,
                    "deploy restart-policy condition, delay, and window must be YAML string scalars",
                ),
                _ => unreachable!("recognized deploy restart-policy field already matched"),
            }
        }
        Some(policy)
    }

    fn parse_deploy_placement(&mut self, field: &ParsedField) -> Option<DeployPlacement> {
        let Some(mapping) = field.value.as_ref().and_then(YamlNode::as_mapping) else {
            self.expected(EXPECTED_MAPPING, field, "deploy placement must be a mapping");
            return None;
        };
        let mut placement = DeployPlacement::new(span_from_position(self.source_id, mapping.byte_range()));
        let mut seen = BTreeMap::new();
        for option in self.fields(mapping) {
            if self.record_duplicate(&mut seen, &option) {
                continue;
            }
            match option.name.value().as_str() {
                name if name.starts_with("x-") => placement.push_extension(option.reference()),
                "constraints" => self
                    .parse_deploy_placement_constraints(&option)
                    .into_iter()
                    .for_each(|value| placement.set_constraints(value)),
                "preferences" => self
                    .parse_deploy_placement_preferences(&option)
                    .into_iter()
                    .for_each(|value| placement.set_preferences(value)),
                "max_replicas_per_node" => self
                    .parse_deploy_placement_max_replicas_per_node(&option)
                    .into_iter()
                    .for_each(|value| placement.set_max_replicas_per_node(value)),
                _ => placement.push_unknown(option.reference()),
            }
        }
        Some(placement)
    }

    fn parse_deploy_placement_constraints(&mut self, field: &ParsedField) -> Option<Vec<Located<String>>> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(
                EXPECTED_SEQUENCE,
                field,
                "deploy placement constraints must be a sequence",
            );
            return None;
        };
        let mut constraints = Vec::new();
        for value in sequence.values() {
            let YamlNode::Scalar(scalar) = value else {
                self.unsupported_sequence_item(
                    EXPECTED_SCALAR,
                    &value,
                    field.span,
                    "deploy placement constraints must contain YAML string scalars",
                );
                continue;
            };
            if ScalarValue::from_scalar(&scalar).scalar_type() != ScalarType::String {
                self.unsupported_sequence_item(
                    EXPECTED_SCALAR,
                    &YamlNode::Scalar(scalar),
                    field.span,
                    "deploy placement constraints must contain YAML string scalars",
                );
                continue;
            }
            constraints.push(Located::new(
                scalar_string_from_source(&self.source, &scalar),
                span_from_position(self.source_id, scalar.byte_range()),
            ));
        }
        Some(constraints)
    }

    fn parse_deploy_placement_preferences(&mut self, field: &ParsedField) -> Option<Vec<DeployPlacementPreference>> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(
                EXPECTED_SEQUENCE,
                field,
                "deploy placement preferences must be a sequence",
            );
            return None;
        };
        let mut preferences = Vec::new();
        for value in sequence.values() {
            let YamlNode::Mapping(mapping) = value else {
                self.unsupported_sequence_item(
                    EXPECTED_MAPPING,
                    &value,
                    field.span,
                    "deploy placement preferences must contain mappings",
                );
                continue;
            };
            let mut preference =
                DeployPlacementPreference::new(span_from_position(self.source_id, mapping.byte_range()));
            let mut seen = BTreeMap::new();
            for member in self.fields(&mapping) {
                if self.record_duplicate(&mut seen, &member) {
                    continue;
                }
                match member.name.value().as_str() {
                    name if name.starts_with("x-") => preference.push_extension(member.reference()),
                    "spread" => self
                        .parse_deploy_placement_string(&member, "deploy placement preference spread")
                        .into_iter()
                        .for_each(|value| preference.set_spread(value)),
                    _ => preference.push_unknown(member.reference()),
                }
            }
            preferences.push(preference);
        }
        Some(preferences)
    }

    fn parse_deploy_placement_max_replicas_per_node(
        &mut self,
        field: &ParsedField,
    ) -> Option<Located<DeployPlacementMaxReplicasPerNode>> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(
                EXPECTED_SCALAR,
                field,
                "deploy placement max_replicas_per_node must be a YAML integer or string scalar",
            );
            return None;
        };
        let value = match ScalarValue::from_scalar(scalar).scalar_type() {
            ScalarType::Integer => {
                DeployPlacementMaxReplicasPerNode::YamlInteger(scalar_string_from_source(&self.source, scalar))
            }
            ScalarType::String => {
                DeployPlacementMaxReplicasPerNode::String(scalar_string_from_source(&self.source, scalar))
            }
            _ => {
                self.expected(
                    EXPECTED_SCALAR,
                    field,
                    "deploy placement max_replicas_per_node must be a YAML integer or string scalar",
                );
                return None;
            }
        };
        Some(Located::new(
            value,
            span_from_position(self.source_id, scalar.byte_range()),
        ))
    }

    fn parse_deploy_placement_string(&mut self, field: &ParsedField, description: &str) -> Option<Located<String>> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(
                EXPECTED_SCALAR,
                field,
                format!("{description} must be a YAML string scalar"),
            );
            return None;
        };
        if ScalarValue::from_scalar(scalar).scalar_type() != ScalarType::String {
            self.expected(
                EXPECTED_SCALAR,
                field,
                format!("{description} must be a YAML string scalar"),
            );
            return None;
        }
        Some(Located::new(
            scalar_string_from_source(&self.source, scalar),
            span_from_position(self.source_id, scalar.byte_range()),
        ))
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
            .unwrap_or_default()
            .into_iter()
            .map(|grant| match grant {
                ParsedGrant::Short(value) => ConfigGrant::Short(value),
                ParsedGrant::Long(value) => ConfigGrant::Long(value),
            })
            .collect()
    }

    fn parse_secret_grants(&mut self, field: &ParsedField) -> Option<Vec<SecretGrant>> {
        Some(
            self.parse_grants(field)?
                .into_iter()
                .map(|grant| match grant {
                    ParsedGrant::Short(value) => SecretGrant::Short(value),
                    ParsedGrant::Long(value) => SecretGrant::Long(value),
                })
                .collect(),
        )
    }

    fn parse_grants(&mut self, field: &ParsedField) -> Option<Vec<ParsedGrant>> {
        let Some(sequence) = field.value.as_ref().and_then(YamlNode::as_sequence) else {
            self.expected(EXPECTED_SEQUENCE, field, "service grants must be a sequence");
            return None;
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
        Some(grants)
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
            self.validate_external_volume_driver_configuration(&volume);
            self.validate_external_volume_labels_configuration(&volume);
            definitions.push(volume);
        }
        definitions
    }

    fn validate_external_volume_driver_configuration(&mut self, volume: &VolumeDefinition) {
        if !matches!(volume.external().map(Located::value), Some(BooleanValue::Literal(true)))
            || (volume.driver().is_none() && volume.driver_opts().is_empty())
        {
            return;
        }
        let span = volume
            .driver()
            .map(Located::span)
            .or_else(|| volume.driver_opts().first().map(KeyValueEntry::span))
            .unwrap_or_else(|| volume.span());
        self.diagnostics.push(
            Diagnostic::new(
                VOLUME_EXTERNAL_DRIVER_CONFIGURATION,
                Severity::Error,
                "external volume cannot also configure `driver` or `driver_opts`",
            )
            .with_label(DiagnosticLabel::primary(
                span,
                "driver configuration remains retained for review",
            )),
        );
    }

    fn validate_external_volume_labels_configuration(&mut self, volume: &VolumeDefinition) {
        if !matches!(volume.external().map(Located::value), Some(BooleanValue::Literal(true)))
            || volume.labels().is_none()
        {
            return;
        }
        let span = volume.labels().map_or_else(|| volume.span(), Labels::span);
        self.diagnostics.push(
            Diagnostic::new(
                VOLUME_EXTERNAL_LABELS_CONFIGURATION,
                Severity::Error,
                "external volume cannot also configure `labels`",
            )
            .with_label(DiagnosticLabel::primary(span, "labels remain retained for review")),
        );
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

    fn parse_non_empty_string(&mut self, field: &ParsedField, description: &str) -> Option<Located<String>> {
        let value = self.parse_string(field, description)?;
        if value.value().is_empty() {
            self.diagnostics.push(
                Diagnostic::new(
                    BUILD_DOCKERFILE_EXPECTED_NON_EMPTY,
                    Severity::Error,
                    format!("{description} must be a non-empty scalar"),
                )
                .with_label(DiagnosticLabel::primary(value.span(), "empty scalar retained")),
            );
            return None;
        }
        Some(value)
    }

    fn parse_build_dockerfile(&mut self, definition: &mut BuildDefinition, field: &ParsedField) -> FieldReference {
        if let Some(dockerfile) = self.parse_non_empty_string(field, "build dockerfile") {
            definition.set_dockerfile(dockerfile);
        }
        field.reference()
    }

    fn set_build_dockerfile_inline(
        &mut self,
        definition: &mut BuildDefinition,
        field: &ParsedField,
        dockerfile_inline: &mut Option<FieldReference>,
    ) {
        *dockerfile_inline = Some(field.reference());
        if let Some(value) = self.parse_build_dockerfile_inline(field) {
            definition.set_dockerfile_inline(value);
        }
    }

    fn parse_build_dockerfile_inline(&mut self, field: &ParsedField) -> Option<Located<String>> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(
                EXPECTED_SCALAR,
                field,
                "build dockerfile_inline must be a YAML string scalar",
            );
            return None;
        };
        if ScalarValue::from_scalar(scalar).scalar_type() != ScalarType::String {
            self.expected(
                EXPECTED_SCALAR,
                field,
                "build dockerfile_inline must be a YAML string scalar",
            );
            return None;
        }
        Some(Located::new(
            scalar_string_from_source(&self.source, scalar),
            span_from_position(self.source_id, scalar.byte_range()),
        ))
    }

    fn set_build_no_cache(&mut self, definition: &mut BuildDefinition, field: &ParsedField) {
        if let Some(no_cache) = self.parse_build_no_cache(field) {
            definition.set_no_cache(no_cache);
        }
    }

    fn set_build_sbom(&mut self, definition: &mut BuildDefinition, field: &ParsedField) {
        if let Some(sbom) = self.parse_build_sbom(field) {
            definition.set_sbom(sbom);
        }
    }

    fn set_build_provenance(&mut self, definition: &mut BuildDefinition, field: &ParsedField) {
        if let Some(value) = self.parse_build_provenance(field) {
            definition.set_provenance(value);
        }
    }

    fn set_build_isolation(&mut self, definition: &mut BuildDefinition, field: &ParsedField) {
        if let Some(isolation) = self.parse_build_isolation(field) {
            definition.set_isolation(isolation);
        }
    }

    fn parse_build_isolation(&mut self, field: &ParsedField) -> Option<Located<String>> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(
                BUILD_ISOLATION_EXPECTED_STRING,
                field,
                "build isolation must be a YAML string scalar",
            );
            return None;
        };
        if ScalarValue::from_scalar(scalar).scalar_type() != ScalarType::String {
            self.expected(
                BUILD_ISOLATION_EXPECTED_STRING,
                field,
                "build isolation must be a YAML string scalar",
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

    fn parse_build_no_cache(&mut self, field: &ParsedField) -> Option<Located<BuildNoCache>> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(
                BUILD_NO_CACHE_EXPECTED_BOOLEAN_OR_STRING,
                field,
                "build no_cache must be a YAML boolean or string scalar",
            );
            return None;
        };
        let span = span_from_position(self.source_id, scalar.byte_range());
        let scalar_value = ScalarValue::from_scalar(scalar);
        let value = match scalar_value.scalar_type() {
            ScalarType::Boolean => BuildNoCache::Boolean(scalar_value.to_bool().unwrap_or(false)),
            ScalarType::String | ScalarType::Timestamp | ScalarType::Regex => {
                BuildNoCache::String(scalar_string_from_source(&self.source, scalar))
            }
            ScalarType::Null | ScalarType::Integer | ScalarType::Float => {
                self.expected(
                    BUILD_NO_CACHE_EXPECTED_BOOLEAN_OR_STRING,
                    field,
                    "build no_cache must be a YAML boolean or string scalar",
                );
                return None;
            }
        };
        Some(Located::new(value, span))
    }
    fn parse_build_no_cache_filter(&mut self, field: &ParsedField) -> Option<BuildNoCacheFilter> {
        match field.value.as_ref() {
            Some(YamlNode::Scalar(s)) if ScalarValue::from_scalar(s).scalar_type() == ScalarType::String => {
                Some(BuildNoCacheFilter::Scalar(Located::new(
                    scalar_string_from_source(&self.source, s),
                    span_from_position(self.source_id, s.byte_range()),
                )))
            }
            Some(YamlNode::Sequence(seq)) => {
                let values = self.parse_string_scalar_nodes(
                    seq.values(),
                    field.span,
                    "build no_cache_filter entries must be string scalars",
                );
                let mut seen = BTreeSet::new();
                for value in &values {
                    if !seen.insert(value.value().clone()) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                BUILD_NO_CACHE_FILTER_DUPLICATE_ITEM,
                                Severity::Warning,
                                "build no_cache_filter retains duplicate stage",
                            )
                            .with_label(DiagnosticLabel::primary(value.span(), "duplicate retained")),
                        );
                    }
                }
                Some(BuildNoCacheFilter::List(values))
            }
            _ => {
                self.expected(
                    EXPECTED_FIELD_FORM,
                    field,
                    "build no_cache_filter must be a string scalar or sequence",
                );
                None
            }
        }
    }
    fn set_build_no_cache_filter(&mut self, definition: &mut BuildDefinition, field: &ParsedField) {
        if let Some(value) = self.parse_build_no_cache_filter(field) {
            definition.set_no_cache_filter(value);
        }
    }
    fn set_build_privileged(&mut self, definition: &mut BuildDefinition, field: &ParsedField) {
        if let Some(value) = self.parse_boolean(field, "build privileged") {
            definition.set_privileged(value);
        }
    }

    fn parse_build_sbom(&mut self, field: &ParsedField) -> Option<Located<BuildSbom>> {
        let Some(scalar) = field.value.as_ref().and_then(YamlNode::as_scalar) else {
            self.expected(
                BUILD_SBOM_EXPECTED_BOOLEAN_OR_STRING,
                field,
                "build sbom must be a YAML boolean or string scalar",
            );
            return None;
        };
        let span = span_from_position(self.source_id, scalar.byte_range());
        let scalar_value = ScalarValue::from_scalar(scalar);
        let value = match scalar_value.scalar_type() {
            ScalarType::Boolean => BuildSbom::Boolean(scalar_value.to_bool().unwrap_or(false)),
            ScalarType::String | ScalarType::Timestamp | ScalarType::Regex => {
                BuildSbom::String(scalar_string_from_source(&self.source, scalar))
            }
            ScalarType::Null | ScalarType::Integer | ScalarType::Float => {
                self.expected(
                    BUILD_SBOM_EXPECTED_BOOLEAN_OR_STRING,
                    field,
                    "build sbom must be a YAML boolean or string scalar",
                );
                return None;
            }
        };
        Some(Located::new(value, span))
    }

    fn parse_build_provenance(&mut self, field: &ParsedField) -> Option<Located<BuildProvenance>> {
        let scalar = field.value.as_ref().and_then(YamlNode::as_scalar)?;
        let span = span_from_position(self.source_id, scalar.byte_range());
        let value = match ScalarValue::from_scalar(scalar).scalar_type() {
            ScalarType::Boolean => {
                BuildProvenance::Boolean(ScalarValue::from_scalar(scalar).to_bool().unwrap_or(false))
            }
            ScalarType::String | ScalarType::Timestamp | ScalarType::Regex => {
                BuildProvenance::String(scalar_string_from_source(&self.source, scalar))
            }
            _ => {
                self.expected(
                    EXPECTED_SCALAR,
                    field,
                    "build provenance must be a YAML boolean or string scalar",
                );
                return None;
            }
        };
        Some(Located::new(value, span))
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
                let values = self.parse_string_scalar_nodes(
                    sequence.values(),
                    field.span,
                    "label list entries must be string scalars",
                );
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

    fn parse_string_scalar_nodes(
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
            if !matches!(
                ScalarValue::from_scalar(&scalar).scalar_type(),
                ScalarType::String | ScalarType::Timestamp | ScalarType::Regex
            ) {
                self.unsupported_sequence_item(EXPECTED_SCALAR, &YamlNode::Scalar(scalar), fallback_span, &message);
                continue;
            }
            let span = span_from_position(self.source_id, scalar.byte_range());
            values.push(Located::new(scalar_string_from_source(&self.source, &scalar), span));
        }
        values
    }

    fn parse_annotations(&mut self, field: &ParsedField) -> Option<Annotations> {
        match field.value.as_ref() {
            Some(YamlNode::Sequence(sequence)) => Some(self.parse_annotation_list(sequence, field.span)),
            Some(YamlNode::Mapping(mapping)) => Some(self.parse_annotation_map(mapping)),
            _ => {
                self.expected(
                    ANNOTATIONS_EXPECTED_FORM,
                    field,
                    "annotations must be a sequence or mapping",
                );
                None
            }
        }
    }

    fn parse_annotation_list(&mut self, sequence: &yaml_edit::Sequence, fallback: SourceSpan) -> Annotations {
        let span = span_from_position(self.source_id, sequence.byte_range());
        let mut values = Vec::new();
        let mut seen = BTreeSet::new();
        for node in sequence.values() {
            let YamlNode::Scalar(scalar) = node else {
                self.unsupported_sequence_item(
                    ANNOTATIONS_EXPECTED_STRING,
                    &node,
                    fallback,
                    "annotation list entries must be string scalars",
                );
                continue;
            };
            let item_span = span_from_position(self.source_id, scalar.byte_range());
            let scalar_value = ScalarValue::from_scalar(&scalar);
            let value = match scalar_value.scalar_type() {
                ScalarType::Null => ComposeScalar::Null,
                ScalarType::Boolean => ComposeScalar::Boolean(scalar_value.to_bool().unwrap_or(false)),
                ScalarType::Integer | ScalarType::Float => {
                    ComposeScalar::Number(scalar_string_from_source(&self.source, &scalar))
                }
                ScalarType::String | ScalarType::Timestamp | ScalarType::Regex => {
                    ComposeScalar::String(scalar_string_from_source(&self.source, &scalar))
                }
            };
            self.validate_annotation_list_scalar(&value, item_span, &mut seen);
            values.push(Located::new(value, item_span));
        }
        Annotations::new(span, AnnotationsForm::List(values))
    }

    fn validate_annotation_list_scalar(
        &mut self,
        value: &ComposeScalar,
        span: SourceSpan,
        seen: &mut BTreeSet<String>,
    ) {
        let ComposeScalar::String(raw) = value else {
            self.diagnostics.push(annotation_diagnostic(
                ANNOTATIONS_EXPECTED_STRING,
                Severity::Error,
                span,
                "annotation list entries must be string scalars",
                "non-string annotation item retained",
            ));
            return;
        };
        let name = raw.split_once('=').map_or(raw.as_str(), |(name, _)| name);
        if name.is_empty() {
            self.diagnostics.push(annotation_diagnostic(
                ANNOTATIONS_EMPTY_NAME,
                Severity::Error,
                span,
                "service annotation name must not be empty",
                "empty annotation name",
            ));
        } else if !seen.insert(name.to_owned()) {
            self.diagnostics.push(annotation_diagnostic(
                ANNOTATIONS_DUPLICATE_NAME,
                Severity::Error,
                span,
                "service annotation names must be unique",
                "duplicate annotation name",
            ));
        }
        if !raw.contains('=') {
            self.diagnostics.push(annotation_diagnostic(
                ANNOTATIONS_KEY_ONLY,
                Severity::Warning,
                span,
                "key-only service annotation has no explicit value",
                "ambiguous key-only annotation",
            ));
        }
    }

    fn parse_annotation_map(&mut self, mapping: &Mapping) -> Annotations {
        let span = span_from_position(self.source_id, mapping.byte_range());
        let mut entries = Vec::new();
        let mut seen = BTreeMap::new();
        for entry in self.fields(mapping) {
            let _duplicate = self.record_duplicate(&mut seen, &entry);
            if entry.name.value.is_empty() {
                self.diagnostics.push(annotation_diagnostic(
                    ANNOTATIONS_EMPTY_NAME,
                    Severity::Error,
                    entry.name.span,
                    "service annotation name must not be empty",
                    "empty annotation name",
                ));
            }
            if let Some(value) = self.parse_compose_scalar(
                &entry,
                "annotation mapping values must be scalar strings, numbers, booleans, or null",
            ) {
                entries.push(KeyValueEntry::new(entry.name, value, entry.span));
            }
        }
        Annotations::new(span, AnnotationsForm::Map(entries))
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
