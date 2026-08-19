//! Deterministic construction of new Compose documents from reviewed native values.

use std::{collections::BTreeSet, error::Error, fmt};

use crate::{
    model::{
        ComposeDocument, CpuRtRuntime, MemLimitUnit, ShmSizeUnit, StopGracePeriod, valid_generated_device_string,
        valid_generated_expose_item, valid_generated_mem_amount, valid_generated_shm_amount,
        valid_generated_tmpfs_item, valid_hostname, valid_positive_pids_decimal, valid_pull_policy_duration,
        valid_ulimit_name,
    },
    source::SourceId,
    syntax::SyntaxDocument,
};
use yaml_edit::{ScalarType, ScalarValue, YamlFile};

use super::write_quoted;

/// A generated Compose construction request is invalid or cannot be represented safely.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GenerationError {
    /// A required value is empty.
    EmptyValue(&'static str),
    /// A value contains a NUL byte and cannot represent native container intent safely.
    ContainsNul(&'static str),
    /// A value contains a carriage return or line feed where one YAML string item is required.
    ContainsLineBreak(&'static str),
    /// An environment name contains Compose list-form's `=` separator.
    InvalidEnvironmentName,
    /// A custom container name does not satisfy Compose's portable name grammar.
    InvalidContainerName,
    /// A service hostname is empty, deferred, or outside the conservative RFC-1123 grammar.
    InvalidHostname,
    /// A custom pull interval does not match the documented Compose duration grammar.
    InvalidPullPolicyDuration,
    /// A finite PID limit is not a positive integral decimal.
    InvalidPidsLimit,
    /// A service shared-memory amount is not a canonical positive ASCII decimal.
    InvalidShmSize,
    /// A service memory-limit amount is not a canonical positive ASCII decimal.
    InvalidMemLimit,
    /// A generated DNS server is empty, multiline, NUL-bearing, or expression-shaped.
    InvalidDnsValue,
    /// A generated DNS resolver option is empty, multiline, NUL-bearing, or expression-shaped.
    InvalidDnsOptionValue,
    /// A generated DNS search domain is empty, multiline, NUL-bearing, or expression-shaped.
    InvalidDnsSearchValue,
    /// A generated exposed-port item is unsafe or outside the documented decimal grammar.
    InvalidExposeValue,
    /// A generated security option is empty, deferred, multiline, or NUL-bearing.
    InvalidSecurityOptionValue,
    /// A generated annotation name is empty, deferred, multiline, or NUL-bearing.
    InvalidAnnotationName,
    /// A generated annotation value is deferred, multiline, or NUL-bearing.
    InvalidAnnotationValue,
    /// A generated top-level config or secret name is not a resolved single-line identifier.
    InvalidFileResourceName,
    /// A generated top-level config or secret `file` value is not a resolved single-line value.
    InvalidFileResourcePath,
    /// A service-level temporary-filesystem item is deferred, malformed, or provider-dependent.
    InvalidTmpfsItem,
    /// A generated short device or long-device member is empty where required, multiline, or deferred.
    InvalidDeviceValue(&'static str),
    /// A generated sysctl mapping name is empty, multiline, NUL-bearing, or expression-shaped.
    InvalidSysctlName,
    /// A generated sysctl value or list item is multiline, NUL-bearing, or expression-shaped.
    InvalidSysctlValue,
    /// A generated logging option number is not one complete YAML number scalar.
    InvalidLoggingOptionNumber,
    /// A generated network driver option number is not one complete YAML number scalar.
    InvalidNetworkDriverOptionNumber,
    /// A generated volume driver option number is not one complete YAML number scalar.
    InvalidVolumeDriverOptionNumber,
    /// A generated ulimit name is outside the portable lowercase ASCII grammar.
    InvalidUlimitName,
    /// A generated ulimit value is outside the supported portable decimal or unlimited set.
    InvalidUlimitValue,
    /// A generated ulimit range omitted its required soft or hard member.
    MissingUlimitRangeMember(&'static str),
    /// A stop grace period does not match the raw-preserving policy based on documented Compose units.
    InvalidStopGracePeriod,
    /// A short-form component contains its reserved separator.
    InvalidShortComponent(&'static str),
    /// A short bind spelling needed for `SELinux` cannot be encoded unambiguously.
    InvalidSelinuxBind,
    /// A raw generated service-runtime field is empty, deferred, multiline, or outside its safe syntax subset.
    InvalidServiceRuntimeField(&'static str),
    /// A singleton field was configured more than once.
    DuplicateField(&'static str),
    /// A named generated collection contains the same name more than once.
    DuplicateName {
        /// Collection whose name collided.
        kind: &'static str,
        /// Duplicate non-sensitive name.
        name: String,
    },
    /// A generated sequence contains an exact duplicate item.
    DuplicateItem(&'static str),
    /// A generated port used target port zero.
    InvalidPort,
    /// An `SCTP` port selected a host address without a published port.
    UnrepresentableSctpHostIp,
    /// A generated project contains no services.
    MissingService,
    /// `ComposeLens` could not parse its own deterministic generated bytes.
    InternalInvariant(&'static str),
}

impl fmt::Display for GenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue(kind) => write!(formatter, "generated {kind} must not be empty"),
            Self::ContainsNul(kind) => write!(formatter, "generated {kind} must not contain a NUL byte"),
            Self::ContainsLineBreak(kind) => {
                write!(formatter, "generated {kind} must not contain a carriage return or line feed")
            }
            Self::InvalidEnvironmentName => formatter.write_str("generated environment name must not contain `=`"),
            Self::InvalidContainerName => {
                formatter.write_str("generated container name must match `[a-zA-Z0-9][a-zA-Z0-9_.-]+`")
            }
            Self::InvalidHostname => formatter.write_str(
                "generated hostname must be a resolved ASCII RFC-1123 name with labels of 1 to 63 characters and total length at most 253",
            ),
            Self::InvalidPullPolicyDuration => formatter.write_str(
                "generated pull policy duration must match integer `w`, `d`, `h`, `m`, and `s` components",
            ),
            Self::InvalidPidsLimit => {
                formatter.write_str("generated finite PID limit must be a positive integral decimal")
            }
            Self::InvalidShmSize => formatter.write_str(
                "generated shared-memory size must use a canonical positive ASCII-integer amount and an explicit documented lowercase unit",
            ),
            Self::InvalidMemLimit => formatter.write_str(
                "generated memory limit must use a canonical positive ASCII-integer amount and an explicit documented lowercase unit",
            ),
            Self::InvalidDnsValue => {
                formatter.write_str("generated DNS server must be a non-empty resolved single-line string")
            }
            Self::InvalidDnsOptionValue => {
                formatter.write_str("generated DNS option must be a non-empty resolved single-line string")
            }
            Self::InvalidDnsSearchValue => {
                formatter.write_str("generated DNS search domain must be a non-empty resolved single-line string")
            }
            Self::InvalidExposeValue => formatter.write_str(
                "generated expose item must be a resolved decimal port or range with an optional `tcp` or `udp` suffix",
            ),
            Self::InvalidSecurityOptionValue => {
                formatter.write_str("generated security option must be a non-empty resolved single-line string")
            }
            Self::InvalidAnnotationName => formatter
                .write_str("generated annotation name must be a non-empty resolved single-line string"),
            Self::InvalidAnnotationValue => formatter
                .write_str("generated annotation value must be a resolved single-line string"),
            Self::InvalidFileResourceName => formatter.write_str(
                "generated top-level config or secret name must be a non-empty resolved single-line string",
            ),
            Self::InvalidFileResourcePath => formatter.write_str(
                "generated top-level config or secret file must be a non-empty resolved single-line string",
            ),
            Self::InvalidTmpfsItem => formatter.write_str(
                "generated tmpfs item must be a non-empty path optionally followed by a colon and non-empty comma-separated raw options",
            ),
            Self::InvalidDeviceValue(member) => write!(
                formatter,
                "generated device {member} must be a safe resolved single-line string{}",
                if matches!(*member, "short item" | "source") {
                    " and must not be empty"
                } else {
                    ""
                }
            ),
            Self::InvalidSysctlName => formatter
                .write_str("generated sysctl name must be a non-empty resolved single-line string"),
            Self::InvalidSysctlValue => formatter
                .write_str("generated sysctl value must be a resolved single-line string"),
            Self::InvalidLoggingOptionNumber => formatter
                .write_str("generated logging option number must be one complete YAML number scalar"),
            Self::InvalidNetworkDriverOptionNumber => formatter
                .write_str("generated network driver option number must be one complete YAML number scalar"),
            Self::InvalidVolumeDriverOptionNumber => formatter
                .write_str("generated volume driver option number must be one complete YAML number scalar"),
            Self::InvalidUlimitName => formatter
                .write_str("generated ulimit name must match lowercase ASCII `[a-z]+`"),
            Self::InvalidUlimitValue => formatter
                .write_str("generated ulimit value must be `-1` or a non-negative ASCII decimal"),
            Self::MissingUlimitRangeMember(member) => {
                write!(formatter, "generated ulimit range is missing required `{member}`")
            }
            Self::InvalidStopGracePeriod => formatter.write_str(
                "generated stop grace period must match the ComposeLens duration policy using `us`, `ms`, `s`, `m`, or `h`, or contain an interpolation marker",
            ),
            Self::InvalidShortComponent(kind) => {
                write!(formatter, "generated {kind} contains its reserved short-form separator")
            }
            Self::InvalidSelinuxBind => formatter
                .write_str("generated SELinux bind source and target must not contain the short-syntax `:` separator"),
            Self::InvalidServiceRuntimeField(field) => write!(formatter, "generated {field} must use a resolved, non-empty field-valid spelling"),
            Self::DuplicateField(field) => write!(formatter, "generated field `{field}` was configured more than once"),
            Self::DuplicateName { kind, name } => {
                write!(formatter, "generated {kind} `{name}` was added more than once")
            }
            Self::DuplicateItem(kind) => write!(formatter, "generated {kind} contains an exact duplicate item"),
            Self::InvalidPort => formatter.write_str("generated container target port must be greater than zero"),
            Self::UnrepresentableSctpHostIp => formatter.write_str(
                "generated SCTP port with a host address also requires a published port for Compose short syntax",
            ),
            Self::MissingService => formatter.write_str("generated Compose project requires at least one service"),
            Self::InternalInvariant(stage) => write!(formatter, "generated Compose document failed {stage} validation"),
        }
    }
}

impl Error for GenerationError {}

/// A plain or sensitive string used by generated Compose fields.
#[derive(Clone, Eq, PartialEq)]
pub struct GeneratedString {
    value: String,
    sensitive: bool,
}

impl GeneratedString {
    /// Creates a non-sensitive generated string.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::ContainsNul`] when the value contains a NUL byte.
    pub fn plain(value: impl Into<String>) -> Result<Self, GenerationError> {
        Self::new(value.into(), false)
    }

    /// Creates a sensitive generated string whose debug representation is redacted.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::ContainsNul`] when the value contains a NUL byte.
    pub fn sensitive(value: impl Into<String>) -> Result<Self, GenerationError> {
        Self::new(value.into(), true)
    }

    fn new(value: String, sensitive: bool) -> Result<Self, GenerationError> {
        if value.contains('\0') {
            return Err(GenerationError::ContainsNul("string"));
        }
        Ok(Self { value, sensitive })
    }

    /// Returns the generated value through an explicit access boundary.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.value
    }

    /// Reports whether debug output must redact this value.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

impl fmt::Debug for GeneratedString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeneratedString")
            .field("value", &if self.sensitive { "<redacted>" } else { &self.value })
            .field("sensitive", &self.sensitive)
            .finish()
    }
}

/// Compose command form selected for a generated service.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedCommand {
    /// Execute an exact argument vector without Compose shell parsing.
    Exec(Vec<GeneratedString>),
    /// Execute one Compose shell-form command.
    Shell(GeneratedString),
    /// Explicitly clear the image command.
    Empty,
}

/// Compose entrypoint form selected for a generated service.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedEntrypoint {
    /// Emit an exact entrypoint list in authored argument order.
    List(Vec<GeneratedString>),
    /// Emit the short scalar string form.
    String(GeneratedString),
    /// Explicitly clear the entrypoint declared by the image.
    Empty,
}

/// A valid service-level Compose restart policy selected for generated output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedRestartPolicy {
    /// Never restart the container automatically.
    No,
    /// Always restart the container until it is removed.
    Always,
    /// Restart after an error, optionally with a maximum retry count.
    OnFailure {
        /// Maximum retries, or `None` for no explicit limit.
        maximum_retries: Option<u64>,
    },
    /// Restart except after an explicit stop or removal.
    UnlessStopped,
}

/// A documented service-level Compose image pull policy selected for generated output.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedPullPolicy {
    /// Pull before every service start.
    Always,
    /// Never pull and rely on a cached image.
    Never,
    /// Pull only when the image is missing.
    Missing,
    /// Emit the retained `if_not_present` alias.
    IfNotPresentAlias,
    /// Build the image before starting the service.
    Build,
    /// Check once per day.
    Daily,
    /// Check once per week.
    Weekly,
    /// Check after an exact caller-supplied duration spelling.
    Every(GeneratedString),
}

/// A service-level Compose PID limit selected for generated output.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedPidsLimit {
    /// Emit the documented unlimited spelling `-1`.
    Unlimited,
    /// Emit an exact positive integral decimal without fixed-width integer parsing.
    Finite(String),
}

/// A safe explicit service shared-memory size selected for generated Compose output.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedShmSize {
    /// Emit one string amount and documented lowercase unit.
    Explicit {
        /// Canonical positive ASCII-integer amount without leading zeros.
        amount: GeneratedString,
        /// Explicit documented lowercase unit.
        unit: ShmSizeUnit,
    },
}

/// A safe explicit service memory limit selected for generated Compose output.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedMemLimit {
    /// Emit one string amount and documented lowercase unit.
    Explicit {
        /// Canonical positive ASCII-integer amount without leading zeros.
        amount: GeneratedString,
        /// Explicit documented lowercase unit.
        unit: MemLimitUnit,
    },
}

/// The exact service-level `tmpfs` form selected for generated Compose output.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedTmpfs {
    /// Emit one YAML string scalar item.
    Scalar(GeneratedString),
    /// Emit one ordered YAML string list, including an explicit empty list.
    List(Vec<GeneratedString>),
}

/// The exact service `dns` form selected for generated Compose output.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedDns {
    /// Emit one raw DNS server string.
    Scalar(GeneratedString),
    /// Emit one ordered YAML string list, including an explicit empty list.
    List(Vec<GeneratedString>),
}

/// The exact service `dns_search` form selected for generated Compose output.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedDnsSearch {
    /// Emit one raw DNS search-domain string.
    Scalar(GeneratedString),
    /// Emit one ordered YAML string list, including an explicit empty list.
    List(Vec<GeneratedString>),
}

/// One generated long-syntax service device.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedLongDevice {
    source: GeneratedString,
    target: Option<GeneratedString>,
    permissions: Option<GeneratedString>,
}

impl GeneratedLongDevice {
    /// Creates a long device from safe resolved strings without interpreting device paths or permissions.
    ///
    /// # Errors
    ///
    /// Rejects an empty source and any NUL-bearing, multiline, or dollar-bearing member. NUL bytes
    /// are normally rejected while constructing [`GeneratedString`]. Empty optional target and
    /// permissions strings remain raw schema strings and are not assigned runtime meaning.
    pub fn new(
        source: GeneratedString,
        target: Option<GeneratedString>,
        permissions: Option<GeneratedString>,
    ) -> Result<Self, GenerationError> {
        validate_generated_device_member("source", &source, true)?;
        if let Some(target) = &target {
            validate_generated_device_member("target", target, false)?;
        }
        if let Some(permissions) = &permissions {
            validate_generated_device_member("permissions", permissions, false)?;
        }
        Ok(Self {
            source,
            target,
            permissions,
        })
    }

    /// Returns the exact generated source through its sensitivity boundary.
    #[must_use]
    pub const fn source(&self) -> &GeneratedString {
        &self.source
    }

    /// Returns the optional exact generated target.
    #[must_use]
    pub const fn target(&self) -> Option<&GeneratedString> {
        self.target.as_ref()
    }

    /// Returns the optional exact raw generated permissions string.
    #[must_use]
    pub const fn permissions(&self) -> Option<&GeneratedString> {
        self.permissions.as_ref()
    }

    fn is_sensitive(&self) -> bool {
        self.source.is_sensitive()
            || self.target.as_ref().is_some_and(GeneratedString::is_sensitive)
            || self.permissions.as_ref().is_some_and(GeneratedString::is_sensitive)
    }
}

/// One generated service device with explicit short or long syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedDevice {
    /// Emit one exact raw YAML string short item.
    Short(GeneratedString),
    /// Emit one ordered long mapping.
    Long(GeneratedLongDevice),
}

/// A generated logging-option value with an explicit YAML scalar kind.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedLoggingOptionValue {
    /// Emit one YAML string with minimal safe quoting.
    String(GeneratedString),
    /// Emit one validated unquoted YAML number with exact spelling retained.
    Number(GeneratedString),
    /// Emit an explicit YAML null.
    Null,
}

impl GeneratedLoggingOptionValue {
    fn is_sensitive(&self) -> bool {
        match self {
            Self::String(value) | Self::Number(value) => value.is_sensitive(),
            Self::Null => false,
        }
    }
}

/// One ordered generated logging option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedLoggingOption {
    name: String,
    value: GeneratedLoggingOptionValue,
}

impl GeneratedLoggingOption {
    /// Creates one option with a non-empty key and a string, number, or null value.
    ///
    /// # Errors
    ///
    /// Rejects an empty or NUL-bearing key and number spellings that are not exactly one YAML
    /// number scalar. No driver-specific option semantics are applied.
    pub fn new(name: impl Into<String>, value: GeneratedLoggingOptionValue) -> Result<Self, GenerationError> {
        let name = required("logging option key", name.into())?;
        if let GeneratedLoggingOptionValue::Number(number) = &value {
            if !valid_yaml_number(number.expose()) {
                return Err(GenerationError::InvalidLoggingOptionNumber);
            }
        }
        Ok(Self { name, value })
    }

    /// Returns the exact non-empty option key.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the selected string, number, or null value.
    #[must_use]
    pub const fn value(&self) -> &GeneratedLoggingOptionValue {
        &self.value
    }
}

/// Explicit service logging configuration for generated output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedLogging {
    driver: GeneratedString,
    options: Vec<GeneratedLoggingOption>,
}

impl GeneratedLogging {
    /// Creates an explicit uninterpreted string driver and ordered unique-key options mapping.
    ///
    /// An empty options vector is retained as `options: {}`. Driver and option values are not
    /// normalized, defaulted, or interpreted for any provider.
    ///
    /// # Errors
    ///
    /// Rejects duplicate option keys.
    pub fn new(driver: GeneratedString, options: Vec<GeneratedLoggingOption>) -> Result<Self, GenerationError> {
        let mut seen = BTreeSet::new();
        for option in &options {
            if !seen.insert(option.name()) {
                return Err(GenerationError::DuplicateName {
                    kind: "logging option",
                    name: option.name().to_owned(),
                });
            }
        }
        Ok(Self { driver, options })
    }

    /// Returns the exact uninterpreted string driver.
    #[must_use]
    pub const fn driver(&self) -> &GeneratedString {
        &self.driver
    }

    /// Returns options in generated mapping order.
    #[must_use]
    pub fn options(&self) -> &[GeneratedLoggingOption] {
        &self.options
    }

    fn is_sensitive(&self) -> bool {
        self.driver.is_sensitive() || self.options.iter().any(|option| option.value.is_sensitive())
    }
}

/// A generated network driver-option value with an explicit YAML scalar kind.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedNetworkDriverOptionValue {
    /// Emit one YAML string with minimal safe quoting.
    String(GeneratedString),
    /// Emit one validated unquoted YAML number with exact spelling retained.
    Number(GeneratedString),
}

impl GeneratedNetworkDriverOptionValue {
    fn is_sensitive(&self) -> bool {
        match self {
            Self::String(value) | Self::Number(value) => value.is_sensitive(),
        }
    }
}

/// One ordered generated network driver option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedNetworkDriverOption {
    name: String,
    value: GeneratedNetworkDriverOptionValue,
}

impl GeneratedNetworkDriverOption {
    /// Creates one driver option with a non-empty key and a string or number value.
    ///
    /// # Errors
    ///
    /// Rejects an empty or NUL-bearing key and number spellings that are not exactly one YAML
    /// number scalar. Driver-option semantics remain uninterpreted.
    pub fn new(name: impl Into<String>, value: GeneratedNetworkDriverOptionValue) -> Result<Self, GenerationError> {
        let name = required("network driver option key", name.into())?;
        if let GeneratedNetworkDriverOptionValue::Number(number) = &value {
            if !valid_yaml_number(number.expose()) {
                return Err(GenerationError::InvalidNetworkDriverOptionNumber);
            }
        }
        Ok(Self { name, value })
    }

    /// Returns the exact non-empty option key.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the selected string or number value.
    #[must_use]
    pub const fn value(&self) -> &GeneratedNetworkDriverOptionValue {
        &self.value
    }
}

/// A generated volume driver-option value with an explicit YAML scalar kind.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedVolumeDriverOptionValue {
    /// Emit one YAML string with minimal safe quoting.
    String(GeneratedString),
    /// Emit one validated unquoted YAML number with exact spelling retained.
    Number(GeneratedString),
}

impl GeneratedVolumeDriverOptionValue {
    fn is_sensitive(&self) -> bool {
        match self {
            Self::String(value) | Self::Number(value) => value.is_sensitive(),
        }
    }
}

/// One ordered generated volume driver option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedVolumeDriverOption {
    name: String,
    value: GeneratedVolumeDriverOptionValue,
}

impl GeneratedVolumeDriverOption {
    /// Creates one driver option with a non-empty key and a string or number value.
    ///
    /// # Errors
    ///
    /// Rejects an empty or NUL-bearing key and number spellings that are not exactly one YAML
    /// number scalar. Driver-option semantics remain uninterpreted.
    pub fn new(name: impl Into<String>, value: GeneratedVolumeDriverOptionValue) -> Result<Self, GenerationError> {
        let name = required("volume driver option key", name.into())?;
        if let GeneratedVolumeDriverOptionValue::Number(number) = &value {
            if !valid_yaml_number(number.expose()) {
                return Err(GenerationError::InvalidVolumeDriverOptionNumber);
            }
        }
        Ok(Self { name, value })
    }

    /// Returns the exact non-empty option key.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the selected string or number value.
    #[must_use]
    pub const fn value(&self) -> &GeneratedVolumeDriverOptionValue {
        &self.value
    }
}

/// A top-level application-owned volume definition with optional driver configuration.
///
/// This type is intentionally distinct from [`GeneratedResource`], which remains the compatible
/// basic/external lifecycle API shared by top-level networks and volumes. External volumes cannot
/// use this driver-configured API.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedVolumeDefinition {
    name: String,
    custom_name: Option<String>,
    driver: Option<GeneratedString>,
    driver_opts: Option<Vec<GeneratedVolumeDriverOption>>,
    labels: Option<Vec<GeneratedLabel>>,
}

impl GeneratedVolumeDefinition {
    /// Creates an application-owned volume definition.
    ///
    /// # Errors
    ///
    /// Rejects an empty or NUL-bearing name.
    pub fn application(name: impl Into<String>) -> Result<Self, GenerationError> {
        Ok(Self {
            name: required("volume name", name.into())?,
            custom_name: None,
            driver: None,
            driver_opts: None,
            labels: None,
        })
    }

    /// Sets the exact platform-level volume name once.
    ///
    /// # Errors
    ///
    /// Rejects an empty/NUL-bearing name and duplicate configuration.
    pub fn set_custom_name(&mut self, name: impl Into<String>) -> Result<(), GenerationError> {
        let name = required("custom volume name", name.into())?;
        set_once(&mut self.custom_name, name, "volume name")
    }

    /// Sets one opaque volume driver exactly once.
    ///
    /// No driver, plugin, provider, runtime, default, or image semantics validation is applied.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_driver(&mut self, driver: GeneratedString) -> Result<(), GenerationError> {
        set_once(&mut self.driver, driver, "volume driver")
    }

    /// Sets the complete ordered unique volume driver-options mapping exactly once.
    ///
    /// An empty mapping remains explicit. String and number YAML scalar identities are selected
    /// by [`GeneratedVolumeDriverOptionValue`] and are never inferred from their text.
    ///
    /// # Errors
    ///
    /// Rejects duplicate option names and duplicate field configuration. No driver-specific
    /// option, plugin, provider, runtime, default, or image semantics are validated.
    pub fn set_driver_opts(&mut self, driver_opts: Vec<GeneratedVolumeDriverOption>) -> Result<(), GenerationError> {
        let mut seen = BTreeSet::new();
        for option in &driver_opts {
            if !seen.insert(option.name()) {
                return Err(GenerationError::DuplicateName {
                    kind: "volume driver option",
                    name: option.name().to_owned(),
                });
            }
        }
        set_once(&mut self.driver_opts, driver_opts, "volume driver_opts")
    }

    /// Sets the complete ordered unique volume-label mapping exactly once.
    ///
    /// An empty mapping remains explicit. Labels use the same explicit string-value contract as
    /// generated service labels, so neither key-only nor non-string label forms are generated.
    ///
    /// # Errors
    ///
    /// Rejects duplicate label names and duplicate field configuration. No provider, runtime, or
    /// injected-label equivalence is inferred.
    pub fn set_labels(&mut self, labels: Vec<GeneratedLabel>) -> Result<(), GenerationError> {
        let mut seen = BTreeSet::new();
        for label in &labels {
            if !seen.insert(label.name()) {
                return Err(GenerationError::DuplicateName {
                    kind: "volume label",
                    name: label.name().to_owned(),
                });
            }
        }
        set_once(&mut self.labels, labels, "volume labels")
    }

    /// Returns the generated volume name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the optional exact platform-level volume name.
    #[must_use]
    pub fn custom_name(&self) -> Option<&str> {
        self.custom_name.as_deref()
    }

    /// Returns the optional opaque volume driver.
    #[must_use]
    pub const fn driver(&self) -> Option<&GeneratedString> {
        self.driver.as_ref()
    }

    /// Returns the optional ordered driver-options mapping, including an explicit empty map.
    #[must_use]
    pub fn driver_opts(&self) -> Option<&[GeneratedVolumeDriverOption]> {
        self.driver_opts.as_deref()
    }

    /// Returns the optional ordered volume-label mapping, including an explicit empty map.
    #[must_use]
    pub fn labels(&self) -> Option<&[GeneratedLabel]> {
        self.labels.as_deref()
    }

    fn is_sensitive(&self) -> bool {
        self.driver.as_ref().is_some_and(GeneratedString::is_sensitive)
            || self
                .driver_opts
                .as_ref()
                .is_some_and(|options| options.iter().any(|option| option.value.is_sensitive()))
            || self
                .labels
                .as_ref()
                .is_some_and(|labels| labels.iter().any(|label| label.value.is_sensitive()))
    }
}

/// A top-level network definition with optional driver configuration.
///
/// This type is intentionally distinct from [`GeneratedResource`], which remains the compatible
/// basic/external lifecycle API shared by top-level networks and volumes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedNetworkDefinition {
    name: String,
    custom_name: Option<String>,
    driver: Option<GeneratedString>,
    driver_opts: Option<Vec<GeneratedNetworkDriverOption>>,
    enable_ipv6: Option<bool>,
    internal: Option<bool>,
    labels: Option<Vec<GeneratedLabel>>,
}

impl GeneratedNetworkDefinition {
    /// Creates an application-owned network definition.
    ///
    /// # Errors
    ///
    /// Rejects an empty or NUL-bearing name.
    pub fn application(name: impl Into<String>) -> Result<Self, GenerationError> {
        Ok(Self {
            name: required("network name", name.into())?,
            custom_name: None,
            driver: None,
            driver_opts: None,
            enable_ipv6: None,
            internal: None,
            labels: None,
        })
    }

    /// Sets the exact platform-level network name once.
    ///
    /// # Errors
    ///
    /// Rejects an empty/NUL-bearing name and duplicate configuration.
    pub fn set_custom_name(&mut self, name: impl Into<String>) -> Result<(), GenerationError> {
        let name = required("custom network name", name.into())?;
        set_once(&mut self.custom_name, name, "network name")
    }

    /// Sets one opaque network driver exactly once.
    ///
    /// No driver, plugin, provider, or runtime availability validation is applied.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_driver(&mut self, driver: GeneratedString) -> Result<(), GenerationError> {
        set_once(&mut self.driver, driver, "network driver")
    }

    /// Sets the complete ordered unique network driver-options mapping exactly once.
    ///
    /// An empty mapping remains explicit. String and number YAML scalar identities are selected
    /// by [`GeneratedNetworkDriverOptionValue`] and are never inferred from their text.
    ///
    /// # Errors
    ///
    /// Rejects duplicate option names and duplicate field configuration. No driver-specific
    /// option, plugin, provider, or runtime semantics are validated.
    pub fn set_driver_opts(&mut self, driver_opts: Vec<GeneratedNetworkDriverOption>) -> Result<(), GenerationError> {
        let mut seen = BTreeSet::new();
        for option in &driver_opts {
            if !seen.insert(option.name()) {
                return Err(GenerationError::DuplicateName {
                    kind: "network driver option",
                    name: option.name().to_owned(),
                });
            }
        }
        set_once(&mut self.driver_opts, driver_opts, "network driver_opts")
    }

    /// Sets the literal IPv6-enable choice exactly once.
    ///
    /// Omission remains distinct from an explicit `false` or `true`. Generation does not infer
    /// defaults or validate driver, IPAM, provider, or runtime behavior.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_enable_ipv6(&mut self, enable_ipv6: bool) -> Result<(), GenerationError> {
        set_once(&mut self.enable_ipv6, enable_ipv6, "network enable_ipv6")
    }

    /// Sets the literal internal-network choice exactly once.
    ///
    /// Omission remains distinct from an explicit `false` or `true`. Generation does not infer
    /// defaults or validate driver, IPAM, provider, or runtime behavior.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_internal(&mut self, internal: bool) -> Result<(), GenerationError> {
        set_once(&mut self.internal, internal, "network internal")
    }

    /// Sets the complete ordered unique network-label mapping exactly once.
    ///
    /// An empty mapping remains explicit. Labels use the same explicit string-value contract as
    /// generated service labels, so neither key-only nor non-string label forms are generated.
    ///
    /// # Errors
    ///
    /// Rejects duplicate label names and duplicate field configuration. No provider, runtime, or
    /// injected-label equivalence is inferred.
    pub fn set_labels(&mut self, labels: Vec<GeneratedLabel>) -> Result<(), GenerationError> {
        let mut seen = BTreeSet::new();
        for label in &labels {
            if !seen.insert(label.name()) {
                return Err(GenerationError::DuplicateName {
                    kind: "network label",
                    name: label.name().to_owned(),
                });
            }
        }
        set_once(&mut self.labels, labels, "network labels")
    }

    /// Returns the generated network name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the optional exact platform-level network name.
    #[must_use]
    pub fn custom_name(&self) -> Option<&str> {
        self.custom_name.as_deref()
    }

    /// Returns the optional opaque network driver.
    #[must_use]
    pub const fn driver(&self) -> Option<&GeneratedString> {
        self.driver.as_ref()
    }

    /// Returns the optional ordered driver-options mapping, including an explicit empty map.
    #[must_use]
    pub fn driver_opts(&self) -> Option<&[GeneratedNetworkDriverOption]> {
        self.driver_opts.as_deref()
    }

    /// Returns the explicitly selected IPv6-enable choice.
    #[must_use]
    pub const fn enable_ipv6(&self) -> Option<bool> {
        self.enable_ipv6
    }

    /// Returns the explicitly selected internal-network choice.
    #[must_use]
    pub const fn internal(&self) -> Option<bool> {
        self.internal
    }

    /// Returns the optional ordered network-label mapping, including an explicit empty map.
    #[must_use]
    pub fn labels(&self) -> Option<&[GeneratedLabel]> {
        self.labels.as_deref()
    }

    fn is_sensitive(&self) -> bool {
        self.driver.as_ref().is_some_and(GeneratedString::is_sensitive)
            || self
                .driver_opts
                .as_ref()
                .is_some_and(|options| options.iter().any(|option| option.value.is_sensitive()))
            || self
                .labels
                .as_ref()
                .is_some_and(|labels| labels.iter().any(|label| label.value.is_sensitive()))
    }
}

impl GeneratedDevice {
    fn is_sensitive(&self) -> bool {
        match self {
            Self::Short(value) => value.is_sensitive(),
            Self::Long(value) => value.is_sensitive(),
        }
    }
}

/// One ordered mapping-form generated sysctl assignment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedSysctl {
    name: String,
    value: GeneratedString,
}

impl GeneratedSysctl {
    /// Creates one resolved string-valued sysctl assignment.
    ///
    /// # Errors
    ///
    /// Rejects empty, multiline, NUL-bearing, or dollar-bearing names and multiline or
    /// dollar-bearing values. Values may be empty. NUL-bearing values are rejected while
    /// constructing [`GeneratedString`].
    pub fn new(name: impl Into<String>, value: GeneratedString) -> Result<Self, GenerationError> {
        let name = name.into();
        if name.is_empty()
            || name.contains(['\0', '\r', '\n'])
            || name.contains('$')
            || value.expose().contains(['\r', '\n', '$'])
        {
            return Err(if name.is_empty() || name.contains(['\0', '\r', '\n', '$']) {
                GenerationError::InvalidSysctlName
            } else {
                GenerationError::InvalidSysctlValue
            });
        }
        Ok(Self { name, value })
    }

    /// Returns the exact generated sysctl name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the exact string value through its sensitivity boundary.
    #[must_use]
    pub const fn value(&self) -> &GeneratedString {
        &self.value
    }
}

/// The mapping or list form selected for generated service `sysctls`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedSysctls {
    /// Ordered unique-name mapping assignments, including an explicit empty mapping.
    Map(Vec<GeneratedSysctl>),
    /// Ordered unique exact strings, including an explicit empty list.
    List(Vec<GeneratedString>),
}

/// The single or soft/hard form selected for one generated service limit.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedUlimitValue {
    /// One value applies to both the soft and hard limit.
    Single(GeneratedString),
    /// Separate required soft and hard values.
    Range {
        /// Required soft limit; omission is rejected during construction.
        soft: Option<GeneratedString>,
        /// Required hard limit; omission is rejected during construction.
        hard: Option<GeneratedString>,
    },
}

/// One ordered generated service limit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedUlimit {
    name: String,
    value: GeneratedUlimitValue,
}

impl GeneratedUlimit {
    /// Creates one validated named generated limit.
    ///
    /// # Errors
    ///
    /// Rejects non-lowercase names, missing range members, deferred/multiline/NUL-bearing values,
    /// and values other than `-1` or non-negative ASCII decimals.
    pub fn new(name: impl Into<String>, value: GeneratedUlimitValue) -> Result<Self, GenerationError> {
        let name = name.into();
        if !valid_ulimit_name(&name) {
            return Err(GenerationError::InvalidUlimitName);
        }
        match &value {
            GeneratedUlimitValue::Single(value) => validate_generated_ulimit_value(value)?,
            GeneratedUlimitValue::Range { soft, hard } => {
                let soft = soft.as_ref().ok_or(GenerationError::MissingUlimitRangeMember("soft"))?;
                let hard = hard.as_ref().ok_or(GenerationError::MissingUlimitRangeMember("hard"))?;
                validate_generated_ulimit_value(soft)?;
                validate_generated_ulimit_value(hard)?;
            }
        }
        Ok(Self { name, value })
    }

    /// Creates one validated single-form generated limit.
    ///
    /// # Errors
    ///
    /// Returns the same name and value validation errors as [`Self::new`].
    pub fn single(name: impl Into<String>, value: GeneratedString) -> Result<Self, GenerationError> {
        Self::new(name, GeneratedUlimitValue::Single(value))
    }

    /// Creates one validated soft/hard generated limit.
    ///
    /// # Errors
    ///
    /// Returns the same name and value validation errors as [`Self::new`].
    pub fn range(
        name: impl Into<String>,
        soft: GeneratedString,
        hard: GeneratedString,
    ) -> Result<Self, GenerationError> {
        Self::new(
            name,
            GeneratedUlimitValue::Range {
                soft: Some(soft),
                hard: Some(hard),
            },
        )
    }

    /// Returns the lowercase limit name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the selected single or soft/hard form.
    #[must_use]
    pub const fn value(&self) -> &GeneratedUlimitValue {
        &self.value
    }

    fn is_sensitive(&self) -> bool {
        match &self.value {
            GeneratedUlimitValue::Single(value) => value.is_sensitive(),
            GeneratedUlimitValue::Range { soft, hard } => {
                soft.iter().chain(hard.iter()).any(GeneratedString::is_sensitive)
            }
        }
    }
}

/// Ordered generated service limits, including an explicit empty mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedUlimits {
    entries: Vec<GeneratedUlimit>,
}

impl GeneratedUlimits {
    /// Creates an ordered unique-name limit mapping.
    ///
    /// # Errors
    ///
    /// Rejects duplicate names without reordering the retained entries.
    pub fn new(entries: Vec<GeneratedUlimit>) -> Result<Self, GenerationError> {
        let mut seen = BTreeSet::new();
        for entry in &entries {
            if !seen.insert(entry.name()) {
                return Err(GenerationError::DuplicateName {
                    kind: "ulimit",
                    name: entry.name().to_owned(),
                });
            }
        }
        Ok(Self { entries })
    }

    /// Returns limits in generated output order.
    #[must_use]
    pub fn entries(&self) -> &[GeneratedUlimit] {
        &self.entries
    }

    /// Reports whether generation will emit an explicit empty mapping.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A resolved service hostname selected for generated Compose output.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedHostname {
    /// Emit one exact resolved hostname after conservative RFC-1123 validation.
    Resolved(GeneratedString),
}

/// One ordered Compose environment entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedEnvironment {
    name: String,
    value: Option<GeneratedString>,
}

/// Explicit parser mode for one generated long-syntax `env_file` entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedEnvironmentFileFormat {
    /// Preserve raw environment-file values without Compose interpolation or quote processing.
    Raw,
}

/// One ordered generated Compose `env_file` declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedEnvironmentFile {
    /// Scalar path syntax with Compose defaults.
    Short(GeneratedString),
    /// Mapping syntax with independently selected options.
    Long {
        /// Environment-file path.
        path: GeneratedString,
        /// Explicit required/optional behavior, or source-format default when omitted.
        required: Option<bool>,
        /// Explicit parser mode, or source-format default when omitted.
        format: Option<GeneratedEnvironmentFileFormat>,
    },
}

/// One generated Compose metadata label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedLabel {
    name: String,
    value: GeneratedString,
}

/// One generated service annotation with an explicit string value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedAnnotation {
    name: String,
    value: GeneratedString,
}

impl GeneratedAnnotation {
    /// Creates one resolved mapping-form annotation.
    ///
    /// # Errors
    ///
    /// Rejects empty/deferred/multiline/NUL-bearing names and deferred/multiline/NUL-bearing
    /// values. Empty explicit values remain representable.
    pub fn new(name: impl Into<String>, value: GeneratedString) -> Result<Self, GenerationError> {
        let name = name.into();
        if name.is_empty() || name.contains(['$', '\r', '\n', '\0']) {
            return Err(GenerationError::InvalidAnnotationName);
        }
        if value.expose().contains(['$', '\r', '\n', '\0']) {
            return Err(GenerationError::InvalidAnnotationValue);
        }
        Ok(Self { name, value })
    }

    /// Returns the exact resolved annotation name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the explicit annotation value through its sensitivity boundary.
    #[must_use]
    pub const fn value(&self) -> &GeneratedString {
        &self.value
    }
}

impl GeneratedLabel {
    /// Creates a label with an explicit string value, including an empty value.
    ///
    /// # Errors
    ///
    /// Rejects an empty or NUL-bearing label name. Values are already validated by
    /// [`GeneratedString`].
    pub fn new(name: impl Into<String>, value: GeneratedString) -> Result<Self, GenerationError> {
        Ok(Self {
            name: required("label name", name.into())?,
            value,
        })
    }

    /// Returns the label name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the label value through its explicit sensitivity boundary.
    #[must_use]
    pub const fn value(&self) -> &GeneratedString {
        &self.value
    }
}

impl GeneratedEnvironment {
    /// Creates a literal `NAME=value` entry.
    ///
    /// # Errors
    ///
    /// Rejects an empty/NUL-bearing name or a name containing `=`.
    pub fn literal(name: impl Into<String>, value: GeneratedString) -> Result<Self, GenerationError> {
        Ok(Self {
            name: environment_name(name.into())?,
            value: Some(value),
        })
    }

    /// Creates a host-resolved key-only environment entry.
    ///
    /// # Errors
    ///
    /// Rejects an empty/NUL-bearing name or a name containing `=`.
    pub fn host(name: impl Into<String>) -> Result<Self, GenerationError> {
        Ok(Self {
            name: environment_name(name.into())?,
            value: None,
        })
    }

    /// Returns the environment name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the optional literal value.
    #[must_use]
    pub const fn value(&self) -> Option<&GeneratedString> {
        self.value.as_ref()
    }
}

impl GeneratedEnvironmentFile {
    /// Creates one scalar short-syntax declaration.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::EmptyValue`] for an empty path. NUL-bearing paths are rejected
    /// while constructing [`GeneratedString`].
    pub fn short(path: GeneratedString) -> Result<Self, GenerationError> {
        require_generated_string("environment-file path", &path)?;
        Ok(Self::Short(path))
    }

    /// Creates one mapping long-syntax declaration.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::EmptyValue`] for an empty path. NUL-bearing paths are rejected
    /// while constructing [`GeneratedString`].
    pub fn long(
        path: GeneratedString,
        required: Option<bool>,
        format: Option<GeneratedEnvironmentFileFormat>,
    ) -> Result<Self, GenerationError> {
        require_generated_string("environment-file path", &path)?;
        Ok(Self::Long { path, required, format })
    }

    /// Returns the environment-file path through its explicit sensitivity boundary.
    #[must_use]
    pub const fn path(&self) -> &GeneratedString {
        match self {
            Self::Short(path) | Self::Long { path, .. } => path,
        }
    }

    /// Returns the explicitly selected required/optional behavior for long syntax.
    #[must_use]
    pub const fn required(&self) -> Option<bool> {
        match self {
            Self::Short(_) => None,
            Self::Long { required, .. } => *required,
        }
    }

    /// Returns the explicitly selected parser mode for long syntax.
    #[must_use]
    pub const fn format(&self) -> Option<GeneratedEnvironmentFileFormat> {
        match self {
            Self::Short(_) => None,
            Self::Long { format, .. } => *format,
        }
    }

    /// Reports whether debug output must redact this declaration's path.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.path().is_sensitive()
    }
}

/// One ordered Compose `extra_hosts` relationship.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedExtraHost {
    hostname: String,
    address: String,
}

impl GeneratedExtraHost {
    /// Creates a short-form `hostname=address` relationship.
    ///
    /// # Errors
    ///
    /// Rejects empty/NUL-bearing values and the unambiguous short-form separator `=`.
    pub fn new(hostname: impl Into<String>, address: impl Into<String>) -> Result<Self, GenerationError> {
        let hostname = short_component("extra-host hostname", hostname.into(), '=')?;
        let address = short_component("extra-host address", address.into(), '=')?;
        Ok(Self { hostname, address })
    }

    /// Returns the hostname.
    #[must_use]
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Returns the address or implementation token.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }
}

/// Transport protocol for one generated published port.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedProtocol {
    /// Transmission Control Protocol.
    Tcp,
    /// User Datagram Protocol.
    Udp,
    /// Stream Control Transmission Protocol.
    Sctp,
}

impl GeneratedProtocol {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
            Self::Sctp => "sctp",
        }
    }
}

/// One generated Compose port entry with protocol-aware syntax selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedPort {
    target: u16,
    published: Option<u16>,
    host_ip: Option<String>,
    protocol: GeneratedProtocol,
}

impl GeneratedPort {
    /// Creates a generated port without normalizing its declared transport.
    ///
    /// # Errors
    ///
    /// Rejects target port zero, an empty/NUL-bearing host address, and an `SCTP` host address
    /// without a published port. `SCTP` uses Compose short syntax because the specification's
    /// long form only defines `tcp` and `udp` protocols.
    pub fn new(
        target: u16,
        published: Option<u16>,
        host_ip: Option<String>,
        protocol: GeneratedProtocol,
    ) -> Result<Self, GenerationError> {
        if target == 0 {
            return Err(GenerationError::InvalidPort);
        }
        if let Some(host_ip) = host_ip.as_deref() {
            required("port host address", host_ip.to_owned())?;
            if protocol == GeneratedProtocol::Sctp && published.is_none() {
                return Err(GenerationError::UnrepresentableSctpHostIp);
            }
        }
        Ok(Self {
            target,
            published,
            host_ip,
            protocol,
        })
    }

    /// Returns the container port.
    #[must_use]
    pub const fn target(&self) -> u16 {
        self.target
    }

    /// Returns the optional host port.
    #[must_use]
    pub const fn published(&self) -> Option<u16> {
        self.published
    }

    /// Returns the optional host-address spelling.
    #[must_use]
    pub fn host_ip(&self) -> Option<&str> {
        self.host_ip.as_deref()
    }

    /// Returns the transport protocol.
    #[must_use]
    pub const fn protocol(&self) -> GeneratedProtocol {
        self.protocol
    }
}

/// `SELinux` relabel option that requires Compose short bind syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedSelinux {
    /// Private unshared relabel (`Z`).
    Private,
    /// Shared relabel (`z`).
    Shared,
}

impl GeneratedSelinux {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Private => "Z",
            Self::Shared => "z",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum GeneratedMountKind {
    Volume {
        source: String,
    },
    Bind {
        source: String,
        selinux: Option<GeneratedSelinux>,
    },
    Anonymous,
}

/// One generated service mount with deliberate short/long syntax selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedMount {
    kind: GeneratedMountKind,
    target: String,
    read_only: bool,
}

impl GeneratedMount {
    /// Creates a long-form named-volume mount.
    ///
    /// # Errors
    ///
    /// Rejects empty or NUL-bearing source and target values.
    pub fn volume(
        source: impl Into<String>,
        target: impl Into<String>,
        read_only: bool,
    ) -> Result<Self, GenerationError> {
        Ok(Self {
            kind: GeneratedMountKind::Volume {
                source: required("volume source", source.into())?,
            },
            target: required("mount target", target.into())?,
            read_only,
        })
    }

    /// Creates a bind mount. `SELinux` relabel intent selects short syntax deliberately.
    ///
    /// # Errors
    ///
    /// Rejects empty/NUL-bearing values. When `selinux` is present, also rejects `:` in source or
    /// target because Compose only honors the relabel option in the short form used here.
    pub fn bind(
        source: impl Into<String>,
        target: impl Into<String>,
        read_only: bool,
        selinux: Option<GeneratedSelinux>,
    ) -> Result<Self, GenerationError> {
        let source = required("bind source", source.into())?;
        let target = required("mount target", target.into())?;
        if selinux.is_some() && (source.contains(':') || target.contains(':')) {
            return Err(GenerationError::InvalidSelinuxBind);
        }
        Ok(Self {
            kind: GeneratedMountKind::Bind { source, selinux },
            target,
            read_only,
        })
    }

    /// Creates a long-form anonymous-volume mount.
    ///
    /// # Errors
    ///
    /// Rejects an empty or NUL-bearing target.
    pub fn anonymous(target: impl Into<String>, read_only: bool) -> Result<Self, GenerationError> {
        Ok(Self {
            kind: GeneratedMountKind::Anonymous,
            target: required("mount target", target.into())?,
            read_only,
        })
    }

    /// Returns the container target path.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Reports whether the mount is read-only.
    #[must_use]
    pub const fn read_only(&self) -> bool {
        self.read_only
    }
}

/// One generated service network attachment and its ordered aliases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedNetworkAttachment {
    name: String,
    aliases: Vec<String>,
    ipv4_address: Option<GeneratedString>,
    ipv6_address: Option<GeneratedString>,
}

impl GeneratedNetworkAttachment {
    /// Creates an attachment without aliases or per-network addresses.
    ///
    /// # Errors
    ///
    /// Rejects an empty or NUL-bearing network name.
    pub fn new(name: impl Into<String>) -> Result<Self, GenerationError> {
        Ok(Self {
            name: required("network name", name.into())?,
            aliases: Vec::new(),
            ipv4_address: None,
            ipv6_address: None,
        })
    }

    /// Adds one ordered alias.
    ///
    /// # Errors
    ///
    /// Rejects an empty or NUL-bearing alias.
    pub fn add_alias(&mut self, alias: impl Into<String>) -> Result<(), GenerationError> {
        self.aliases.push(required("network alias", alias.into())?);
        Ok(())
    }

    /// Sets one raw per-attachment IPv4 address exactly once.
    ///
    /// No IP grammar, top-level IPAM pool, provider, or runtime validation is applied.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_ipv4_address(&mut self, address: GeneratedString) -> Result<(), GenerationError> {
        set_once(&mut self.ipv4_address, address, "ipv4_address")
    }

    /// Sets one raw per-attachment IPv6 address exactly once.
    ///
    /// No IP grammar, top-level IPAM pool, provider, or runtime validation is applied.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_ipv6_address(&mut self, address: GeneratedString) -> Result<(), GenerationError> {
        set_once(&mut self.ipv6_address, address, "ipv6_address")
    }

    /// Returns the network name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns aliases in insertion order.
    #[must_use]
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    /// Returns the optional raw per-attachment IPv4 address.
    #[must_use]
    pub const fn ipv4_address(&self) -> Option<&GeneratedString> {
        self.ipv4_address.as_ref()
    }

    /// Returns the optional raw per-attachment IPv6 address.
    #[must_use]
    pub const fn ipv6_address(&self) -> Option<&GeneratedString> {
        self.ipv6_address.as_ref()
    }

    fn is_sensitive(&self) -> bool {
        self.ipv4_address.as_ref().is_some_and(GeneratedString::is_sensitive)
            || self.ipv6_address.as_ref().is_some_and(GeneratedString::is_sensitive)
    }
}

/// One top-level network or volume lifecycle definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedResource {
    name: String,
    external: bool,
    custom_name: Option<String>,
}

impl GeneratedResource {
    /// Creates an application-owned resource definition.
    ///
    /// # Errors
    ///
    /// Rejects an empty or NUL-bearing name.
    pub fn application(name: impl Into<String>) -> Result<Self, GenerationError> {
        Ok(Self {
            name: required("resource name", name.into())?,
            external: false,
            custom_name: None,
        })
    }

    /// Creates an externally managed resource definition.
    ///
    /// # Errors
    ///
    /// Rejects an empty or NUL-bearing name.
    pub fn external(name: impl Into<String>) -> Result<Self, GenerationError> {
        Ok(Self {
            name: required("resource name", name.into())?,
            external: true,
            custom_name: None,
        })
    }

    /// Sets the exact platform-level resource name once.
    ///
    /// This prevents Compose project scoping from changing a reviewed runtime resource name.
    ///
    /// # Errors
    ///
    /// Rejects an empty/NUL-bearing name and duplicate configuration.
    pub fn set_custom_name(&mut self, name: impl Into<String>) -> Result<(), GenerationError> {
        let name = required("custom resource name", name.into())?;
        set_once(&mut self.custom_name, name, "resource name")
    }

    /// Returns the resource name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Reports whether Compose should reuse an external resource.
    #[must_use]
    pub const fn is_external(&self) -> bool {
        self.external
    }

    /// Returns the optional exact platform-level resource name.
    #[must_use]
    pub fn custom_name(&self) -> Option<&str> {
        self.custom_name.as_deref()
    }
}

/// A generated `cpu_rt_runtime` spelling with an explicit YAML scalar category.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedCpuRtRuntime {
    /// An unquoted integer microsecond scalar.
    Microseconds(GeneratedString),
    /// A Compose duration string.
    Duration(GeneratedString),
}

impl GeneratedCpuRtRuntime {
    fn is_sensitive(&self) -> bool {
        match self {
            Self::Microseconds(value) | Self::Duration(value) => value.is_sensitive(),
        }
    }
}

/// Raw service resource and namespace fields selected for deterministic generated output.
/// String-bearing variants use minimal safe quoting so caller-selected spelling remains a YAML string;
/// `cpu_rt_runtime` explicitly selects either an integer microsecond scalar or a duration string.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GeneratedServiceRuntimeField {
    /// Raw resolved service domain name.
    Domainname(GeneratedString),
    /// Raw resolved service isolation spelling.
    Isolation(GeneratedString),
    /// Raw resolved service MAC-address spelling.
    MacAddress(GeneratedString),
    /// Raw resolved service UTS spelling.
    Uts(GeneratedString),
    /// Literal API-socket mount choice.
    UseApiSocket(bool),
    /// Safe scalar GPU selector.
    GpusAll(GeneratedString),
    /// `cpu_rt_runtime` with an explicit integer or duration scalar category.
    CpuRtRuntime(GeneratedCpuRtRuntime),
    /// `cpu_shares` raw integer spelling.
    CpuShares(GeneratedString),
    /// `cpus` raw decimal spelling.
    Cpus(GeneratedString),
    /// `cpuset` raw string spelling.
    Cpuset(GeneratedString),
    /// Ordered raw `device_cgroup_rules` strings.
    DeviceCgroupRules(Vec<GeneratedString>),
    /// `ipc` raw mode spelling.
    Ipc(GeneratedString),
    /// `mem_reservation` raw byte-value spelling.
    MemReservation(GeneratedString),
    /// `mem_swappiness` raw integer spelling.
    MemSwappiness(GeneratedString),
    /// `memswap_limit` raw unlimited, zero, or positive byte-quantity spelling.
    MemswapLimit(GeneratedString),
    /// `network_mode` raw mode spelling.
    NetworkMode(GeneratedString),
    /// Literal `oom_kill_disable` choice.
    OomKillDisable(bool),
    /// `oom_score_adj` raw integer spelling.
    OomScoreAdj(GeneratedString),
    /// `pid` raw mode spelling.
    Pid(GeneratedString),
    /// `scale` raw integer spelling.
    Scale(GeneratedString),
    /// Ordered raw `volumes_from` strings.
    VolumesFrom(Vec<GeneratedString>),
}

impl GeneratedServiceRuntimeField {
    fn field_name(&self) -> &'static str {
        match self {
            Self::Domainname(_) => "domainname",
            Self::Isolation(_) => "isolation",
            Self::MacAddress(_) => "mac_address",
            Self::Uts(_) => "uts",
            Self::UseApiSocket(_) => "use_api_socket",
            Self::GpusAll(_) => "gpus",
            Self::CpuRtRuntime(_) => "cpu_rt_runtime",
            Self::CpuShares(_) => "cpu_shares",
            Self::Cpus(_) => "cpus",
            Self::Cpuset(_) => "cpuset",
            Self::DeviceCgroupRules(_) => "device_cgroup_rules",
            Self::Ipc(_) => "ipc",
            Self::MemReservation(_) => "mem_reservation",
            Self::MemSwappiness(_) => "mem_swappiness",
            Self::MemswapLimit(_) => "memswap_limit",
            Self::NetworkMode(_) => "network_mode",
            Self::OomKillDisable(_) => "oom_kill_disable",
            Self::OomScoreAdj(_) => "oom_score_adj",
            Self::Pid(_) => "pid",
            Self::Scale(_) => "scale",
            Self::VolumesFrom(_) => "volumes_from",
        }
    }

    fn is_sensitive(&self) -> bool {
        match self {
            Self::Domainname(value)
            | Self::Isolation(value)
            | Self::MacAddress(value)
            | Self::Uts(value)
            | Self::GpusAll(value)
            | Self::CpuShares(value)
            | Self::Cpus(value)
            | Self::Cpuset(value)
            | Self::Ipc(value)
            | Self::MemReservation(value)
            | Self::MemSwappiness(value)
            | Self::MemswapLimit(value)
            | Self::NetworkMode(value)
            | Self::OomScoreAdj(value)
            | Self::Pid(value)
            | Self::Scale(value) => value.is_sensitive(),
            Self::DeviceCgroupRules(values) | Self::VolumesFrom(values) => {
                values.iter().any(GeneratedString::is_sensitive)
            }
            Self::UseApiSocket(_) | Self::OomKillDisable(_) => false,
            Self::CpuRtRuntime(value) => value.is_sensitive(),
        }
    }
}

/// A typed generated Compose service definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedService {
    name: String,
    hostname: Option<GeneratedHostname>,
    container_name: Option<GeneratedString>,
    image: Option<GeneratedString>,
    entrypoint: Option<GeneratedEntrypoint>,
    command: Option<GeneratedCommand>,
    init: Option<bool>,
    stdin_open: Option<bool>,
    tty: Option<bool>,
    privileged: Option<bool>,
    environment_files: Vec<GeneratedEnvironmentFile>,
    environment: Vec<GeneratedEnvironment>,
    labels: Vec<GeneratedLabel>,
    annotations: Option<Vec<GeneratedAnnotation>>,
    user: Option<GeneratedString>,
    userns_mode: Option<GeneratedString>,
    group_add: Vec<GeneratedString>,
    cap_add: Option<Vec<GeneratedString>>,
    cap_drop: Option<Vec<GeneratedString>>,
    devices: Option<Vec<GeneratedDevice>>,
    dns: Option<GeneratedDns>,
    dns_options: Option<Vec<GeneratedString>>,
    dns_search: Option<GeneratedDnsSearch>,
    expose: Option<Vec<GeneratedString>>,
    security_options: Option<Vec<GeneratedString>>,
    working_dir: Option<GeneratedString>,
    read_only: Option<bool>,
    pids_limit: Option<GeneratedPidsLimit>,
    shm_size: Option<GeneratedShmSize>,
    mem_limit: Option<GeneratedMemLimit>,
    tmpfs: Option<GeneratedTmpfs>,
    sysctls: Option<GeneratedSysctls>,
    logging: Option<GeneratedLogging>,
    ulimits: Option<GeneratedUlimits>,
    pull_policy: Option<GeneratedPullPolicy>,
    restart: Option<GeneratedRestartPolicy>,
    stop_signal: Option<GeneratedString>,
    stop_grace_period: Option<GeneratedString>,
    extra_hosts: Vec<GeneratedExtraHost>,
    ports: Vec<GeneratedPort>,
    mounts: Vec<GeneratedMount>,
    networks: Vec<GeneratedNetworkAttachment>,
    runtime_fields: Vec<GeneratedServiceRuntimeField>,
}

impl GeneratedService {
    /// Creates an empty service with a validated name.
    ///
    /// # Errors
    ///
    /// Rejects an empty or NUL-bearing name.
    pub fn new(name: impl Into<String>) -> Result<Self, GenerationError> {
        Ok(Self {
            name: required("service name", name.into())?,
            hostname: None,
            container_name: None,
            image: None,
            entrypoint: None,
            command: None,
            init: None,
            stdin_open: None,
            tty: None,
            privileged: None,
            environment_files: Vec::new(),
            environment: Vec::new(),
            labels: Vec::new(),
            annotations: None,
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
            ulimits: None,
            pull_policy: None,
            restart: None,
            stop_signal: None,
            stop_grace_period: None,
            extra_hosts: Vec::new(),
            ports: Vec::new(),
            mounts: Vec::new(),
            networks: Vec::new(),
            runtime_fields: Vec::new(),
        })
    }

    /// Returns the service name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Adds one generated raw-preserving resource or namespace field exactly once.
    ///
    /// All string values must be resolved single-line strings. The generated YAML is parse-back
    /// validated with the rest of the document; this method deliberately makes no provider or
    /// runtime support claim.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when the same runtime field was already
    /// selected, or [`GenerationError::InvalidServiceRuntimeField`] when its value is not safe
    /// for generated Compose YAML.
    pub fn add_runtime_field(&mut self, field: GeneratedServiceRuntimeField) -> Result<(), GenerationError> {
        if self
            .runtime_fields
            .iter()
            .any(|existing| existing.field_name() == field.field_name())
        {
            return Err(GenerationError::DuplicateField(field.field_name()));
        }
        if !generated_runtime_field_safe(&field) {
            return Err(GenerationError::InvalidServiceRuntimeField(field.field_name()));
        }
        self.runtime_fields.push(field);
        Ok(())
    }

    /// Returns the selected raw-preserving generated runtime fields in insertion order.
    #[must_use]
    pub fn runtime_fields(&self) -> &[GeneratedServiceRuntimeField] {
        &self.runtime_fields
    }

    /// Sets one resolved RFC-1123 service hostname exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidHostname`] for an empty, expression-shaped, non-ASCII,
    /// overlong, or otherwise invalid hostname, or [`GenerationError::DuplicateField`] when
    /// already configured.
    pub fn set_hostname(&mut self, hostname: GeneratedHostname) -> Result<(), GenerationError> {
        let GeneratedHostname::Resolved(value) = &hostname;
        if !valid_hostname(value.expose()) {
            return Err(GenerationError::InvalidHostname);
        }
        set_once(&mut self.hostname, hostname, "hostname")
    }

    /// Sets the custom runtime container name exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidContainerName`] when the value does not match Compose's
    /// portable container-name grammar or [`GenerationError::DuplicateField`] when already
    /// configured.
    pub fn set_container_name(&mut self, name: GeneratedString) -> Result<(), GenerationError> {
        if !valid_container_name(name.expose()) {
            return Err(GenerationError::InvalidContainerName);
        }
        set_once(&mut self.container_name, name, "container_name")
    }

    /// Sets the service image exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::EmptyValue`] for an empty image or
    /// [`GenerationError::DuplicateField`] when already configured.
    pub fn set_image(&mut self, image: GeneratedString) -> Result<(), GenerationError> {
        require_generated_string("service image", &image)?;
        set_once(&mut self.image, image, "image")
    }

    /// Sets the Compose entrypoint form exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_entrypoint(&mut self, entrypoint: GeneratedEntrypoint) -> Result<(), GenerationError> {
        set_once(&mut self.entrypoint, entrypoint, "entrypoint")
    }

    /// Sets the Compose command form exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_command(&mut self, command: GeneratedCommand) -> Result<(), GenerationError> {
        set_once(&mut self.command, command, "command")
    }

    /// Sets the Compose init-process choice exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_init(&mut self, init: bool) -> Result<(), GenerationError> {
        set_once(&mut self.init, init, "init")
    }

    /// Sets the Compose standard-input-open choice exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_stdin_open(&mut self, stdin_open: bool) -> Result<(), GenerationError> {
        set_once(&mut self.stdin_open, stdin_open, "stdin_open")
    }

    /// Sets the Compose terminal-allocation choice exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_tty(&mut self, tty: bool) -> Result<(), GenerationError> {
        set_once(&mut self.tty, tty, "tty")
    }

    /// Sets the Compose privileged choice exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_privileged(&mut self, privileged: bool) -> Result<(), GenerationError> {
        set_once(&mut self.privileged, privileged, "privileged")
    }

    /// Adds one ordered environment-file declaration.
    pub fn add_environment_file(&mut self, environment_file: GeneratedEnvironmentFile) {
        self.environment_files.push(environment_file);
    }

    /// Adds one ordered environment entry.
    pub fn add_environment(&mut self, environment: GeneratedEnvironment) {
        self.environment.push(environment);
    }

    /// Adds one uniquely named service metadata label.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateName`] when the service already defines the label.
    pub fn add_label(&mut self, label: GeneratedLabel) -> Result<(), GenerationError> {
        if self.labels.iter().any(|candidate| candidate.name == label.name) {
            return Err(GenerationError::DuplicateName {
                kind: "service label",
                name: label.name,
            });
        }
        self.labels.push(label);
        Ok(())
    }

    /// Sets the complete ordered mapping-form annotation collection exactly once.
    ///
    /// Omission remains distinct from an explicit empty mapping. Names must be unique and all
    /// entries carry explicit resolved string values; key-only and null forms cannot enter this API.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateName`] for duplicate names,
    /// [`GenerationError::InvalidAnnotationName`] or [`GenerationError::InvalidAnnotationValue`]
    /// for unsafe values, or [`GenerationError::DuplicateField`] when already configured.
    pub fn set_annotations(&mut self, annotations: Vec<GeneratedAnnotation>) -> Result<(), GenerationError> {
        let mut seen = BTreeSet::new();
        for annotation in &annotations {
            if annotation.name.is_empty() || annotation.name.contains(['$', '\r', '\n', '\0']) {
                return Err(GenerationError::InvalidAnnotationName);
            }
            if annotation.value.expose().contains(['$', '\r', '\n', '\0']) {
                return Err(GenerationError::InvalidAnnotationValue);
            }
            if !seen.insert(annotation.name.as_str()) {
                return Err(GenerationError::DuplicateName {
                    kind: "service annotation",
                    name: annotation.name.clone(),
                });
            }
        }
        set_once(&mut self.annotations, annotations, "annotations")
    }

    /// Returns configured annotations, distinguishing omission from an explicit empty mapping.
    #[must_use]
    pub fn annotations(&self) -> Option<&[GeneratedAnnotation]> {
        self.annotations.as_deref()
    }

    /// Sets the combined Compose `user[:group]` value exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_user(&mut self, user: GeneratedString) -> Result<(), GenerationError> {
        set_once(&mut self.user, user, "user")
    }

    /// Sets the user-namespace mode exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::EmptyValue`] for an empty mode or
    /// [`GenerationError::DuplicateField`] when already configured.
    pub fn set_userns_mode(&mut self, mode: GeneratedString) -> Result<(), GenerationError> {
        require_generated_string("user namespace mode", &mode)?;
        set_once(&mut self.userns_mode, mode, "userns_mode")
    }

    /// Adds one ordered supplementary group.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::EmptyValue`] for an empty group.
    pub fn add_supplementary_group(&mut self, group: GeneratedString) -> Result<(), GenerationError> {
        require_generated_string("supplementary group", &group)?;
        self.group_add.push(group);
        Ok(())
    }

    /// Sets the complete ordered `cap_add` sequence exactly once.
    ///
    /// An empty vector is retained as explicit `cap_add: []`; never calling this method omits the
    /// field. Values preserve exact case and ordering. No capability whitelist is applied.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::EmptyValue`] for an empty item,
    /// [`GenerationError::ContainsLineBreak`] for a carriage return or line feed,
    /// [`GenerationError::DuplicateItem`] for an exact case-sensitive duplicate, or
    /// [`GenerationError::DuplicateField`] when already configured. NUL bytes are rejected while
    /// constructing [`GeneratedString`].
    pub fn set_cap_add(&mut self, capabilities: Vec<GeneratedString>) -> Result<(), GenerationError> {
        let mut seen = BTreeSet::new();
        for capability in &capabilities {
            require_generated_string("cap_add item", capability)?;
            if capability.expose().contains('\r') || capability.expose().contains('\n') {
                return Err(GenerationError::ContainsLineBreak("cap_add item"));
            }
            if !seen.insert(capability.expose()) {
                return Err(GenerationError::DuplicateItem("cap_add"));
            }
        }
        set_once(&mut self.cap_add, capabilities, "cap_add")
    }

    /// Returns the configured `cap_add` sequence, distinguishing omission from an empty vector.
    #[must_use]
    pub fn cap_add(&self) -> Option<&[GeneratedString]> {
        self.cap_add.as_deref()
    }

    /// Sets the complete ordered `cap_drop` sequence exactly once.
    ///
    /// An empty vector is retained as explicit `cap_drop: []`; never calling this method omits the
    /// field. Values preserve exact case and ordering. No capability whitelist is applied.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::EmptyValue`] for an empty item,
    /// [`GenerationError::ContainsLineBreak`] for a carriage return or line feed,
    /// [`GenerationError::DuplicateItem`] for an exact case-sensitive duplicate, or
    /// [`GenerationError::DuplicateField`] when already configured. NUL bytes are rejected while
    /// constructing [`GeneratedString`].
    pub fn set_cap_drop(&mut self, capabilities: Vec<GeneratedString>) -> Result<(), GenerationError> {
        let mut seen = BTreeSet::new();
        for capability in &capabilities {
            require_generated_string("cap_drop item", capability)?;
            if capability.expose().contains('\r') || capability.expose().contains('\n') {
                return Err(GenerationError::ContainsLineBreak("cap_drop item"));
            }
            if !seen.insert(capability.expose()) {
                return Err(GenerationError::DuplicateItem("cap_drop"));
            }
        }
        set_once(&mut self.cap_drop, capabilities, "cap_drop")
    }

    /// Returns the configured `cap_drop` sequence, distinguishing omission from an empty vector.
    #[must_use]
    pub fn cap_drop(&self) -> Option<&[GeneratedString]> {
        self.cap_drop.as_deref()
    }

    /// Sets the complete ordered mixed short/long `devices` sequence exactly once.
    ///
    /// An empty vector is emitted as `devices: []`; omission remains distinct. Exact duplicate
    /// items and caller order are preserved. This validates only safe resolved YAML output and
    /// does not inspect host devices, split colon triples, validate CDI, normalize permissions,
    /// or claim runtime access.
    ///
    /// # Errors
    ///
    /// Rejects empty short items and empty long sources, plus NUL-bearing, multiline, or
    /// dollar-bearing values. NUL bytes are normally rejected while constructing
    /// [`GeneratedString`]. Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_devices(&mut self, devices: Vec<GeneratedDevice>) -> Result<(), GenerationError> {
        for device in &devices {
            match device {
                GeneratedDevice::Short(value) => {
                    validate_generated_device_member("short item", value, true)?;
                }
                GeneratedDevice::Long(value) => {
                    validate_generated_device_member("source", value.source(), true)?;
                    if let Some(target) = value.target() {
                        validate_generated_device_member("target", target, false)?;
                    }
                    if let Some(permissions) = value.permissions() {
                        validate_generated_device_member("permissions", permissions, false)?;
                    }
                }
            }
        }
        set_once(&mut self.devices, devices, "devices")
    }

    /// Sets the complete scalar or ordered-list service `dns` form exactly once.
    ///
    /// An empty list remains explicit. Values are retained as raw server strings: this API does
    /// not require an IP address, parse a resolver grammar, or perform network access.
    ///
    /// # Errors
    ///
    /// Rejects empty, multiline, NUL-bearing, or dollar-bearing values and duplicate field
    /// configuration. NUL bytes are normally rejected while constructing [`GeneratedString`].
    pub fn set_dns(&mut self, dns: GeneratedDns) -> Result<(), GenerationError> {
        let values = match &dns {
            GeneratedDns::Scalar(value) => std::slice::from_ref(value),
            GeneratedDns::List(values) => values.as_slice(),
        };
        for value in values {
            if value.expose().is_empty()
                || value.expose().contains('$')
                || value.expose().contains('\r')
                || value.expose().contains('\n')
            {
                return Err(GenerationError::InvalidDnsValue);
            }
        }
        set_once(&mut self.dns, dns, "dns")
    }

    /// Returns the configured scalar or ordered-list DNS form.
    #[must_use]
    pub const fn dns(&self) -> Option<&GeneratedDns> {
        self.dns.as_ref()
    }

    /// Sets the complete ordered service `dns_opt` sequence exactly once.
    ///
    /// An empty vector remains explicit while leaving this setter unused omits the field. Values
    /// are treated as raw resolver-option strings; no option grammar or runtime behavior is
    /// inferred.
    ///
    /// # Errors
    ///
    /// Rejects empty, multiline, NUL-bearing, dollar-bearing, or exact-duplicate values and
    /// duplicate field configuration. NUL bytes are normally rejected while constructing
    /// [`GeneratedString`].
    pub fn set_dns_options(&mut self, options: Vec<GeneratedString>) -> Result<(), GenerationError> {
        let mut seen = BTreeSet::new();
        for option in &options {
            if option.expose().is_empty()
                || option.expose().contains('$')
                || option.expose().contains('\r')
                || option.expose().contains('\n')
                || option.expose().contains('\0')
            {
                return Err(GenerationError::InvalidDnsOptionValue);
            }
            if !seen.insert(option.expose()) {
                return Err(GenerationError::DuplicateItem("dns_opt"));
            }
        }
        set_once(&mut self.dns_options, options, "dns_opt")
    }

    /// Returns configured DNS resolver options, distinguishing omission from an empty sequence.
    #[must_use]
    pub fn dns_options(&self) -> Option<&[GeneratedString]> {
        self.dns_options.as_deref()
    }

    /// Sets the complete scalar or ordered-list service `dns_search` form exactly once.
    ///
    /// An empty list remains explicit, exact duplicates and `.` are retained, and no domain,
    /// resolver, provider, or runtime validation is performed.
    ///
    /// # Errors
    ///
    /// Rejects empty, multiline, NUL-bearing, or dollar-bearing values and duplicate field
    /// configuration. NUL bytes are normally rejected while constructing [`GeneratedString`].
    pub fn set_dns_search(&mut self, search: GeneratedDnsSearch) -> Result<(), GenerationError> {
        let values = match &search {
            GeneratedDnsSearch::Scalar(value) => std::slice::from_ref(value),
            GeneratedDnsSearch::List(values) => values.as_slice(),
        };
        for value in values {
            if value.expose().is_empty()
                || value.expose().contains('$')
                || value.expose().contains('\r')
                || value.expose().contains('\n')
                || value.expose().contains('\0')
            {
                return Err(GenerationError::InvalidDnsSearchValue);
            }
        }
        set_once(&mut self.dns_search, search, "dns_search")
    }

    /// Returns the configured scalar or ordered-list DNS search-domain form.
    #[must_use]
    pub const fn dns_search(&self) -> Option<&GeneratedDnsSearch> {
        self.dns_search.as_ref()
    }

    /// Sets the complete ordered service `expose` sequence exactly once.
    ///
    /// An empty vector remains explicit. Every output item remains a YAML string, so number and
    /// string identities are never silently equated. Omitted protocol and explicit `/tcp` remain
    /// distinct.
    ///
    /// # Errors
    ///
    /// Rejects empty, deferred, multiline, NUL-bearing, malformed, SCTP, unknown-protocol, and
    /// exact-duplicate values, or duplicate field configuration.
    pub fn set_expose(&mut self, expose: Vec<GeneratedString>) -> Result<(), GenerationError> {
        let mut seen = BTreeSet::new();
        for item in &expose {
            if !valid_generated_expose_item(item.expose()) {
                return Err(GenerationError::InvalidExposeValue);
            }
            if !seen.insert(item.expose()) {
                return Err(GenerationError::DuplicateItem("expose"));
            }
        }
        set_once(&mut self.expose, expose, "expose")
    }

    /// Returns the configured exposed-port sequence, including an explicit empty sequence.
    #[must_use]
    pub fn expose(&self) -> Option<&[GeneratedString]> {
        self.expose.as_deref()
    }

    /// Sets the complete ordered raw service `security_opt` sequence exactly once.
    ///
    /// An empty vector remains explicit, exact duplicates retain their order, and no option,
    /// profile, provider, or target-runtime normalization is performed.
    ///
    /// # Errors
    ///
    /// Rejects empty, deferred, multiline, or NUL-bearing values and duplicate field
    /// configuration. NUL bytes are normally rejected while constructing [`GeneratedString`].
    pub fn set_security_options(&mut self, options: Vec<GeneratedString>) -> Result<(), GenerationError> {
        for option in &options {
            if option.expose().is_empty()
                || option.expose().contains('$')
                || option.expose().contains('\r')
                || option.expose().contains('\n')
                || option.expose().contains('\0')
            {
                return Err(GenerationError::InvalidSecurityOptionValue);
            }
        }
        set_once(&mut self.security_options, options, "security_opt")
    }

    /// Returns configured raw security options, distinguishing omission from an empty sequence.
    #[must_use]
    pub fn security_options(&self) -> Option<&[GeneratedString]> {
        self.security_options.as_deref()
    }

    /// Returns configured devices, distinguishing omission from an explicit empty sequence.
    #[must_use]
    pub fn devices(&self) -> Option<&[GeneratedDevice]> {
        self.devices.as_deref()
    }

    /// Sets the container working directory exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::EmptyValue`] for an empty directory or
    /// [`GenerationError::DuplicateField`] when already configured.
    pub fn set_working_dir(&mut self, directory: GeneratedString) -> Result<(), GenerationError> {
        require_generated_string("working directory", &directory)?;
        set_once(&mut self.working_dir, directory, "working_dir")
    }

    /// Sets the read-only-root choice exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_read_only(&mut self, read_only: bool) -> Result<(), GenerationError> {
        set_once(&mut self.read_only, read_only, "read_only")
    }

    /// Sets an unlimited or positive finite service PID limit exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidPidsLimit`] when a finite spelling is empty, zero,
    /// signed, fractional, exponent-shaped, or otherwise not ASCII decimal, or
    /// [`GenerationError::DuplicateField`] when already configured.
    pub fn set_pids_limit(&mut self, limit: GeneratedPidsLimit) -> Result<(), GenerationError> {
        if let GeneratedPidsLimit::Finite(decimal) = &limit {
            if !valid_positive_pids_decimal(decimal) {
                return Err(GenerationError::InvalidPidsLimit);
            }
        }
        set_once(&mut self.pids_limit, limit, "pids_limit")
    }

    /// Sets one explicit positive service shared-memory size exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidShmSize`] when the amount is empty, zero, has leading
    /// zeros, a sign, fraction, exponent, whitespace, or non-ASCII digits, or
    /// [`GenerationError::DuplicateField`] when already configured.
    pub fn set_shm_size(&mut self, size: GeneratedShmSize) -> Result<(), GenerationError> {
        let GeneratedShmSize::Explicit { amount, .. } = &size;
        if !valid_generated_shm_amount(amount.expose()) {
            return Err(GenerationError::InvalidShmSize);
        }
        set_once(&mut self.shm_size, size, "shm_size")
    }

    /// Sets one explicit positive service memory limit exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidMemLimit`] when the amount is empty, zero, has leading
    /// zeros, a sign, fraction, exponent, whitespace, or non-ASCII digits, or
    /// [`GenerationError::DuplicateField`] when already configured.
    pub fn set_mem_limit(&mut self, limit: GeneratedMemLimit) -> Result<(), GenerationError> {
        let GeneratedMemLimit::Explicit { amount, .. } = &limit;
        if !valid_generated_mem_amount(amount.expose()) {
            return Err(GenerationError::InvalidMemLimit);
        }
        set_once(&mut self.mem_limit, limit, "mem_limit")
    }

    /// Sets the complete scalar or list service-level `tmpfs` form exactly once.
    ///
    /// An empty list is retained explicitly. Item spelling, ordering, and case remain unchanged.
    ///
    /// # Errors
    ///
    /// Rejects empty, multiline, deferred, or structurally malformed items. Documented `mode`,
    /// `uid`, and `gid` assignments and other well-shaped raw target options remain exact, including
    /// duplicate list entries. NUL bytes are rejected while constructing [`GeneratedString`]. Returns
    /// [`GenerationError::DuplicateField`] when already configured.
    pub fn set_tmpfs(&mut self, tmpfs: GeneratedTmpfs) -> Result<(), GenerationError> {
        let items = match &tmpfs {
            GeneratedTmpfs::Scalar(item) => std::slice::from_ref(item),
            GeneratedTmpfs::List(items) => items.as_slice(),
        };
        for item in items {
            require_generated_string("tmpfs item", item)?;
            if item.expose().contains('\r') || item.expose().contains('\n') {
                return Err(GenerationError::ContainsLineBreak("tmpfs item"));
            }
            if !valid_generated_tmpfs_item(item.expose()) {
                return Err(GenerationError::InvalidTmpfsItem);
            }
        }
        set_once(&mut self.tmpfs, tmpfs, "tmpfs")
    }

    /// Returns the configured scalar or list form, distinguishing omission from an empty list.
    #[must_use]
    pub const fn tmpfs(&self) -> Option<&GeneratedTmpfs> {
        self.tmpfs.as_ref()
    }

    /// Sets the complete mapping or list `sysctls` form exactly once.
    ///
    /// Empty collections remain explicit. Mapping names and list strings must be exact-unique;
    /// neither form applies namespace validation or runtime coercion.
    ///
    /// # Errors
    ///
    /// Rejects duplicate map names, duplicate exact list items, multiline or dollar-bearing list
    /// items, and duplicate field configuration. NUL-bearing list items are rejected while
    /// constructing [`GeneratedString`].
    pub fn set_sysctls(&mut self, sysctls: GeneratedSysctls) -> Result<(), GenerationError> {
        let mut seen = BTreeSet::new();
        match &sysctls {
            GeneratedSysctls::Map(entries) => {
                for entry in entries {
                    if !seen.insert(entry.name()) {
                        return Err(GenerationError::DuplicateName {
                            kind: "sysctl",
                            name: entry.name().to_owned(),
                        });
                    }
                }
            }
            GeneratedSysctls::List(items) => {
                for item in items {
                    if item.expose().contains(['\r', '\n', '$']) {
                        return Err(GenerationError::InvalidSysctlValue);
                    }
                    if !seen.insert(item.expose()) {
                        return Err(GenerationError::DuplicateItem("sysctls"));
                    }
                }
            }
        }
        set_once(&mut self.sysctls, sysctls, "sysctls")
    }

    /// Returns the configured form, distinguishing omission from explicit empty collections.
    #[must_use]
    pub const fn sysctls(&self) -> Option<&GeneratedSysctls> {
        self.sysctls.as_ref()
    }

    /// Sets explicit logging configuration exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured. Driver spelling and
    /// option semantics are otherwise left uninterpreted.
    pub fn set_logging(&mut self, logging: GeneratedLogging) -> Result<(), GenerationError> {
        set_once(&mut self.logging, logging, "logging")
    }

    /// Returns configured logging, distinguishing omission from explicit empty options.
    #[must_use]
    pub const fn logging(&self) -> Option<&GeneratedLogging> {
        self.logging.as_ref()
    }

    /// Sets the complete ordered service `ulimits` mapping exactly once.
    ///
    /// An empty mapping remains explicit. Values are already validated while constructing
    /// [`GeneratedUlimit`] and names are unique by construction in [`GeneratedUlimits`].
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_ulimits(&mut self, ulimits: GeneratedUlimits) -> Result<(), GenerationError> {
        set_once(&mut self.ulimits, ulimits, "ulimits")
    }

    /// Returns configured ordered limits, distinguishing omission from an explicit empty mapping.
    #[must_use]
    pub const fn ulimits(&self) -> Option<&GeneratedUlimits> {
        self.ulimits.as_ref()
    }

    /// Sets a documented service image pull policy exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidPullPolicyDuration`] for an invalid custom interval or
    /// [`GenerationError::DuplicateField`] when already configured.
    pub fn set_pull_policy(&mut self, policy: GeneratedPullPolicy) -> Result<(), GenerationError> {
        if let GeneratedPullPolicy::Every(duration) = &policy {
            if !valid_pull_policy_duration(duration.expose()) {
                return Err(GenerationError::InvalidPullPolicyDuration);
            }
        }
        set_once(&mut self.pull_policy, policy, "pull_policy")
    }

    /// Sets the service-level restart policy exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured.
    pub fn set_restart(&mut self, restart: GeneratedRestartPolicy) -> Result<(), GenerationError> {
        set_once(&mut self.restart, restart, "restart")
    }

    /// Sets the service stop signal exactly once without imposing a signal-token grammar.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateField`] when already configured. Quoted empty values
    /// are preserved; NUL-bearing values are rejected while constructing [`GeneratedString`].
    pub fn set_stop_signal(&mut self, signal: GeneratedString) -> Result<(), GenerationError> {
        set_once(&mut self.stop_signal, signal, "stop_signal")
    }

    /// Sets the raw-preserving service stop grace period exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidStopGracePeriod`] when the value does not match the
    /// `ComposeLens` raw-preserving duration policy or dollar-marker convention, or
    /// [`GenerationError::DuplicateField`] when already configured.
    pub fn set_stop_grace_period(&mut self, period: GeneratedString) -> Result<(), GenerationError> {
        if !StopGracePeriod::parse(period.expose().to_owned()).is_valid() {
            return Err(GenerationError::InvalidStopGracePeriod);
        }
        set_once(&mut self.stop_grace_period, period, "stop_grace_period")
    }

    /// Adds one ordered host mapping.
    pub fn add_extra_host(&mut self, host: GeneratedExtraHost) {
        self.extra_hosts.push(host);
    }

    /// Adds one ordered published-port declaration.
    pub fn add_port(&mut self, port: GeneratedPort) {
        self.ports.push(port);
    }

    /// Adds one ordered mount.
    pub fn add_mount(&mut self, mount: GeneratedMount) {
        self.mounts.push(mount);
    }

    /// Adds one uniquely named network attachment.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateName`] when the service already uses the network.
    pub fn add_network(&mut self, network: GeneratedNetworkAttachment) -> Result<(), GenerationError> {
        if self.networks.iter().any(|candidate| candidate.name == network.name) {
            return Err(GenerationError::DuplicateName {
                kind: "service network",
                name: network.name,
            });
        }
        self.networks.push(network);
        Ok(())
    }

    fn is_sensitive(&self) -> bool {
        matches!(
            self.hostname.as_ref(),
            Some(GeneratedHostname::Resolved(hostname)) if hostname.is_sensitive()
        ) || self.image.as_ref().is_some_and(GeneratedString::is_sensitive)
            || self.entrypoint.as_ref().is_some_and(entrypoint_is_sensitive)
            || self.command.as_ref().is_some_and(command_is_sensitive)
            || self
                .environment_files
                .iter()
                .any(GeneratedEnvironmentFile::is_sensitive)
            || self
                .environment
                .iter()
                .filter_map(GeneratedEnvironment::value)
                .any(GeneratedString::is_sensitive)
            || self.labels.iter().any(|label| label.value.is_sensitive())
            || self
                .annotations
                .as_ref()
                .is_some_and(|items| items.iter().any(|annotation| annotation.value.is_sensitive()))
            || matches!(
                self.pull_policy.as_ref(),
                Some(GeneratedPullPolicy::Every(duration)) if duration.is_sensitive()
            )
            || matches!(
                self.shm_size.as_ref(),
                Some(GeneratedShmSize::Explicit { amount, .. }) if amount.is_sensitive()
            )
            || matches!(
                self.mem_limit.as_ref(),
                Some(GeneratedMemLimit::Explicit { amount, .. }) if amount.is_sensitive()
            )
            || match self.tmpfs.as_ref() {
                Some(GeneratedTmpfs::Scalar(item)) => item.is_sensitive(),
                Some(GeneratedTmpfs::List(items)) => items.iter().any(GeneratedString::is_sensitive),
                None => false,
            }
            || match self.dns.as_ref() {
                Some(GeneratedDns::Scalar(value)) => value.is_sensitive(),
                Some(GeneratedDns::List(values)) => values.iter().any(GeneratedString::is_sensitive),
                None => false,
            }
            || self
                .dns_options
                .as_ref()
                .is_some_and(|items| items.iter().any(GeneratedString::is_sensitive))
            || self
                .runtime_fields
                .iter()
                .any(GeneratedServiceRuntimeField::is_sensitive)
            || match self.dns_search.as_ref() {
                Some(GeneratedDnsSearch::Scalar(value)) => value.is_sensitive(),
                Some(GeneratedDnsSearch::List(values)) => values.iter().any(GeneratedString::is_sensitive),
                None => false,
            }
            || self
                .expose
                .as_ref()
                .is_some_and(|items| items.iter().any(GeneratedString::is_sensitive))
            || self
                .security_options
                .as_ref()
                .is_some_and(|items| items.iter().any(GeneratedString::is_sensitive))
            || match self.sysctls.as_ref() {
                Some(GeneratedSysctls::Map(entries)) => entries.iter().any(|entry| entry.value.is_sensitive()),
                Some(GeneratedSysctls::List(items)) => items.iter().any(GeneratedString::is_sensitive),
                None => false,
            }
            || self.logging.as_ref().is_some_and(GeneratedLogging::is_sensitive)
            || self
                .ulimits
                .as_ref()
                .is_some_and(|limits| limits.entries.iter().any(GeneratedUlimit::is_sensitive))
            || [
                self.user.as_ref(),
                self.userns_mode.as_ref(),
                self.working_dir.as_ref(),
                self.stop_signal.as_ref(),
                self.stop_grace_period.as_ref(),
            ]
            .into_iter()
            .flatten()
            .any(GeneratedString::is_sensitive)
            || self.group_add.iter().any(GeneratedString::is_sensitive)
            || self
                .cap_add
                .as_ref()
                .is_some_and(|items| items.iter().any(GeneratedString::is_sensitive))
            || self
                .cap_drop
                .as_ref()
                .is_some_and(|items| items.iter().any(GeneratedString::is_sensitive))
            || self
                .devices
                .as_ref()
                .is_some_and(|items| items.iter().any(GeneratedDevice::is_sensitive))
            || self.networks.iter().any(GeneratedNetworkAttachment::is_sensitive)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum GeneratedNetwork {
    Basic(GeneratedResource),
    Definition(GeneratedNetworkDefinition),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum GeneratedVolume {
    Basic(GeneratedResource),
    Definition(GeneratedVolumeDefinition),
}

impl GeneratedVolume {
    fn name(&self) -> &str {
        match self {
            Self::Basic(volume) => volume.name(),
            Self::Definition(volume) => volume.name(),
        }
    }

    fn is_sensitive(&self) -> bool {
        match self {
            Self::Basic(_) => false,
            Self::Definition(volume) => volume.is_sensitive(),
        }
    }
}

impl GeneratedNetwork {
    fn name(&self) -> &str {
        match self {
            Self::Basic(network) => network.name(),
            Self::Definition(network) => network.name(),
        }
    }

    fn is_sensitive(&self) -> bool {
        match self {
            Self::Basic(_) => false,
            Self::Definition(network) => network.is_sensitive(),
        }
    }
}

/// One generated top-level config definition backed by a caller-supplied file spelling.
///
/// The builder deliberately supports no inline content, environment, external lifecycle,
/// labels, template driver, or file access through this type.
#[derive(Clone, Eq, PartialEq)]
pub struct GeneratedConfigFileDefinition {
    name: String,
    file: GeneratedString,
}

impl GeneratedConfigFileDefinition {
    /// Creates a config definition with one required resolved single-line `file` value.
    ///
    /// # Errors
    ///
    /// Rejects empty, deferred, multiline, or NUL-bearing names and file values.
    pub fn new(name: impl Into<String>, file: GeneratedString) -> Result<Self, GenerationError> {
        Ok(Self {
            name: generated_file_resource_name(name.into())?,
            file: generated_file_resource_path(file)?,
        })
    }

    /// Returns the exact generated config name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the explicit generated file value through its sensitivity boundary.
    #[must_use]
    pub const fn file(&self) -> &GeneratedString {
        &self.file
    }

    fn is_sensitive(&self) -> bool {
        self.file.is_sensitive()
    }
}

impl fmt::Debug for GeneratedConfigFileDefinition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeneratedConfigFileDefinition")
            .field("name", &self.name)
            .field("file", &self.file)
            .finish()
    }
}

/// One generated top-level secret definition backed by a caller-supplied file spelling.
///
/// The builder deliberately supports no environment, driver, labels, template driver, external
/// lifecycle, or file access through this type.
#[derive(Clone, Eq, PartialEq)]
pub struct GeneratedSecretFileDefinition {
    name: String,
    file: GeneratedString,
}

impl GeneratedSecretFileDefinition {
    /// Creates a secret definition with one required resolved single-line `file` value.
    ///
    /// # Errors
    ///
    /// Rejects empty, deferred, multiline, or NUL-bearing names and file values.
    pub fn new(name: impl Into<String>, file: GeneratedString) -> Result<Self, GenerationError> {
        Ok(Self {
            name: generated_file_resource_name(name.into())?,
            file: generated_file_resource_path(file)?,
        })
    }

    /// Returns the exact generated secret name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the explicit generated file value through its sensitivity boundary.
    #[must_use]
    pub const fn file(&self) -> &GeneratedString {
        &self.file
    }

    fn is_sensitive(&self) -> bool {
        self.file.is_sensitive()
    }
}

impl fmt::Debug for GeneratedSecretFileDefinition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeneratedSecretFileDefinition")
            .field("name", &self.name)
            .field("file", &self.file)
            .finish()
    }
}

/// Builder for one new deterministic Compose document.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComposeDocumentBuilder {
    name: Option<String>,
    services: Vec<GeneratedService>,
    networks: Vec<GeneratedNetwork>,
    volumes: Vec<GeneratedVolume>,
    configs: Vec<GeneratedConfigFileDefinition>,
    secrets: Vec<GeneratedSecretFileDefinition>,
}

impl ComposeDocumentBuilder {
    /// Creates an empty generated project.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            name: None,
            services: Vec::new(),
            networks: Vec::new(),
            volumes: Vec::new(),
            configs: Vec::new(),
            secrets: Vec::new(),
        }
    }

    /// Sets the optional top-level Compose project name exactly once.
    ///
    /// # Errors
    ///
    /// Rejects empty/NUL-bearing names and duplicate configuration.
    pub fn set_name(&mut self, name: impl Into<String>) -> Result<(), GenerationError> {
        let name = required("project name", name.into())?;
        set_once(&mut self.name, name, "name")
    }

    /// Adds one uniquely named service in output order.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateName`] for a duplicate service name.
    pub fn add_service(&mut self, service: GeneratedService) -> Result<(), GenerationError> {
        insert_named(&mut self.services, service, "service", GeneratedService::name)
    }

    /// Adds one uniquely named top-level network in output order.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateName`] for a duplicate network name.
    pub fn add_network(&mut self, network: GeneratedResource) -> Result<(), GenerationError> {
        insert_named(
            &mut self.networks,
            GeneratedNetwork::Basic(network),
            "network",
            GeneratedNetwork::name,
        )
    }

    /// Adds one uniquely named top-level network definition in output order.
    ///
    /// This is additive to [`Self::add_network`], which retains the existing basic/external
    /// [`GeneratedResource`] API for compatibility.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateName`] for a duplicate network name across basic and
    /// driver-configured network definitions.
    pub fn add_network_definition(&mut self, network: GeneratedNetworkDefinition) -> Result<(), GenerationError> {
        insert_named(
            &mut self.networks,
            GeneratedNetwork::Definition(network),
            "network",
            GeneratedNetwork::name,
        )
    }

    /// Adds one uniquely named top-level volume in output order.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateName`] for a duplicate volume name.
    pub fn add_volume(&mut self, volume: GeneratedResource) -> Result<(), GenerationError> {
        insert_named(
            &mut self.volumes,
            GeneratedVolume::Basic(volume),
            "volume",
            GeneratedVolume::name,
        )
    }

    /// Adds one uniquely named top-level application-owned volume definition in output order.
    ///
    /// This is additive to [`Self::add_volume`], which retains the existing basic/external
    /// [`GeneratedResource`] API for compatibility. Driver-configured external volumes are not
    /// representable: use `GeneratedResource::external` for that lifecycle.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateName`] for a duplicate volume name across basic and
    /// driver-configured volume definitions.
    pub fn add_volume_definition(&mut self, volume: GeneratedVolumeDefinition) -> Result<(), GenerationError> {
        insert_named(
            &mut self.volumes,
            GeneratedVolume::Definition(volume),
            "volume",
            GeneratedVolume::name,
        )
    }

    /// Adds one uniquely named top-level config file definition in output order.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateName`] for a duplicate config name.
    pub fn add_config_file(&mut self, config: GeneratedConfigFileDefinition) -> Result<(), GenerationError> {
        insert_named(&mut self.configs, config, "config", GeneratedConfigFileDefinition::name)
    }

    /// Adds one uniquely named top-level secret file definition in output order.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateName`] for a duplicate secret name.
    pub fn add_secret_file(&mut self, secret: GeneratedSecretFileDefinition) -> Result<(), GenerationError> {
        insert_named(&mut self.secrets, secret, "secret", GeneratedSecretFileDefinition::name)
    }

    /// Generates YAML and parses it back through `ComposeLens`'s syntax and typed-model boundaries.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::MissingService`] for an empty project or
    /// [`GenerationError::InternalInvariant`] if `ComposeLens` cannot parse its own output.
    pub fn build(self, source_id: SourceId) -> Result<GeneratedComposeDocument, GenerationError> {
        if self.services.is_empty() {
            return Err(GenerationError::MissingService);
        }
        let sensitive = self.services.iter().any(GeneratedService::is_sensitive)
            || self.networks.iter().any(GeneratedNetwork::is_sensitive)
            || self.volumes.iter().any(GeneratedVolume::is_sensitive)
            || self.configs.iter().any(GeneratedConfigFileDefinition::is_sensitive)
            || self.secrets.iter().any(GeneratedSecretFileDefinition::is_sensitive);
        let text = render_document(&self);
        let syntax = SyntaxDocument::parse(source_id, text.clone())
            .map_err(|_| GenerationError::InternalInvariant("syntax-tree"))?;
        if !syntax.is_valid() {
            return Err(GenerationError::InternalInvariant("syntax"));
        }
        let model = ComposeDocument::parse(syntax.document());
        if !model.is_valid() {
            return Err(GenerationError::InternalInvariant("typed-model"));
        }
        let document = model
            .document()
            .cloned()
            .ok_or(GenerationError::InternalInvariant("document-root"))?;
        Ok(GeneratedComposeDocument {
            text,
            sensitive,
            document,
        })
    }
}

/// Parse-back-validated deterministic generated Compose document.
#[derive(Clone, Eq, PartialEq)]
pub struct GeneratedComposeDocument {
    text: String,
    sensitive: bool,
    document: ComposeDocument,
}

impl GeneratedComposeDocument {
    /// Returns the deployable generated YAML through an explicit access boundary.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the parse-back-validated native Compose model.
    #[must_use]
    pub const fn document(&self) -> &ComposeDocument {
        &self.document
    }

    /// Reports whether generated output contains a caller-marked sensitive value.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

impl fmt::Debug for GeneratedComposeDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeneratedComposeDocument")
            .field("text", &if self.sensitive { "<redacted>" } else { &self.text })
            .field("sensitive", &self.sensitive)
            .field("document", &if self.sensitive { "<redacted>" } else { "validated" })
            .finish()
    }
}

fn render_document(project: &ComposeDocumentBuilder) -> String {
    let mut output = String::from("---\n");
    if let Some(name) = &project.name {
        output.push_str("name: ");
        write_quoted(&mut output, name);
        output.push('\n');
    }
    output.push_str("services:\n");
    for service in &project.services {
        write_indent(&mut output, 1);
        write_quoted(&mut output, &service.name);
        output.push_str(":\n");
        render_service(&mut output, service);
    }
    render_network_definitions(&mut output, &project.networks);
    render_volume_definitions(&mut output, &project.volumes);
    render_file_definitions(
        &mut output,
        "configs",
        &project.configs,
        GeneratedConfigFileDefinition::name,
        GeneratedConfigFileDefinition::file,
    );
    render_file_definitions(
        &mut output,
        "secrets",
        &project.secrets,
        GeneratedSecretFileDefinition::name,
        GeneratedSecretFileDefinition::file,
    );
    output
}

fn render_service(output: &mut String, service: &GeneratedService) {
    if let Some(GeneratedHostname::Resolved(hostname)) = &service.hostname {
        render_optional_string(output, "hostname", Some(hostname));
    }
    render_optional_string(output, "container_name", service.container_name.as_ref());
    render_optional_string(output, "image", service.image.as_ref());
    if let Some(entrypoint) = &service.entrypoint {
        render_entrypoint(output, entrypoint);
    }
    if let Some(command) = &service.command {
        render_command(output, command);
    }
    if let Some(init) = service.init {
        write_field(output, 2, "init");
        output.push_str(if init { "true\n" } else { "false\n" });
    }
    if let Some(stdin_open) = service.stdin_open {
        write_field(output, 2, "stdin_open");
        output.push_str(if stdin_open { "true\n" } else { "false\n" });
    }
    if let Some(tty) = service.tty {
        write_field(output, 2, "tty");
        output.push_str(if tty { "true\n" } else { "false\n" });
    }
    if let Some(privileged) = service.privileged {
        write_field(output, 2, "privileged");
        output.push_str(if privileged { "true\n" } else { "false\n" });
    }
    render_environment_files(output, &service.environment_files);
    render_environment(output, &service.environment);
    render_labels(output, &service.labels);
    if let Some(annotations) = &service.annotations {
        render_annotations(output, annotations);
    }
    render_optional_string(output, "user", service.user.as_ref());
    render_optional_string(output, "userns_mode", service.userns_mode.as_ref());
    render_string_sequence(output, "group_add", &service.group_add);
    if let Some(capabilities) = &service.cap_add {
        render_configured_string_sequence(output, "cap_add", capabilities);
    }
    if let Some(capabilities) = &service.cap_drop {
        render_configured_string_sequence(output, "cap_drop", capabilities);
    }
    render_optional_string(output, "working_dir", service.working_dir.as_ref());
    if let Some(read_only) = service.read_only {
        write_field(output, 2, "read_only");
        output.push_str(if read_only { "true\n" } else { "false\n" });
    }
    if let Some(pids_limit) = &service.pids_limit {
        render_pids_limit(output, pids_limit);
    }
    if let Some(shm_size) = &service.shm_size {
        render_shm_size(output, shm_size);
    }
    if let Some(mem_limit) = &service.mem_limit {
        render_mem_limit(output, mem_limit);
    }
    render_runtime_fields(output, &service.runtime_fields);
    if let Some(devices) = &service.devices {
        render_devices(output, devices);
    }
    if let Some(dns) = &service.dns {
        render_dns(output, dns);
    }
    if let Some(options) = &service.dns_options {
        render_configured_string_sequence(output, "dns_opt", options);
    }
    if let Some(search) = &service.dns_search {
        render_dns_search(output, search);
    }
    if let Some(expose) = &service.expose {
        render_configured_string_sequence(output, "expose", expose);
    }
    if let Some(options) = &service.security_options {
        render_configured_string_sequence(output, "security_opt", options);
    }
    if let Some(tmpfs) = &service.tmpfs {
        render_tmpfs(output, tmpfs);
    }
    if let Some(sysctls) = &service.sysctls {
        render_sysctls(output, sysctls);
    }
    if let Some(logging) = &service.logging {
        render_logging(output, logging);
    }
    if let Some(ulimits) = &service.ulimits {
        render_ulimits(output, ulimits);
    }
    if let Some(pull_policy) = &service.pull_policy {
        render_pull_policy(output, pull_policy);
    }
    if let Some(restart) = service.restart {
        render_restart(output, restart);
    }
    render_optional_string(output, "stop_signal", service.stop_signal.as_ref());
    render_optional_string(output, "stop_grace_period", service.stop_grace_period.as_ref());
    render_extra_hosts(output, &service.extra_hosts);
    render_ports(output, &service.ports);
    render_mounts(output, &service.mounts);
    render_networks(output, &service.networks);
}

fn render_runtime_fields(output: &mut String, fields: &[GeneratedServiceRuntimeField]) {
    for field in fields {
        match field {
            GeneratedServiceRuntimeField::Domainname(value) => {
                render_optional_string(output, "domainname", Some(value));
            }
            GeneratedServiceRuntimeField::Isolation(value) => {
                render_optional_string(output, "isolation", Some(value));
            }
            GeneratedServiceRuntimeField::MacAddress(value) => {
                render_optional_string(output, "mac_address", Some(value));
            }
            GeneratedServiceRuntimeField::Uts(value) => {
                render_optional_string(output, "uts", Some(value));
            }
            GeneratedServiceRuntimeField::UseApiSocket(value) => {
                write_field(output, 2, "use_api_socket");
                output.push_str(if *value { "true\n" } else { "false\n" });
            }
            GeneratedServiceRuntimeField::GpusAll(value) => {
                render_optional_string(output, "gpus", Some(value));
            }
            GeneratedServiceRuntimeField::CpuRtRuntime(GeneratedCpuRtRuntime::Microseconds(value)) => {
                write_field(output, 2, "cpu_rt_runtime");
                output.push_str(value.expose());
                output.push('\n');
            }
            GeneratedServiceRuntimeField::CpuRtRuntime(GeneratedCpuRtRuntime::Duration(value)) => {
                render_optional_string(output, "cpu_rt_runtime", Some(value));
            }
            GeneratedServiceRuntimeField::CpuShares(value) => {
                render_optional_string(output, "cpu_shares", Some(value));
            }
            GeneratedServiceRuntimeField::Cpus(value) => {
                render_optional_string(output, "cpus", Some(value));
            }
            GeneratedServiceRuntimeField::Cpuset(value) => {
                render_optional_string(output, "cpuset", Some(value));
            }
            GeneratedServiceRuntimeField::DeviceCgroupRules(values) => {
                render_configured_string_sequence(output, "device_cgroup_rules", values);
            }
            GeneratedServiceRuntimeField::Ipc(value) => {
                render_optional_string(output, "ipc", Some(value));
            }
            GeneratedServiceRuntimeField::MemReservation(value) => {
                render_optional_string(output, "mem_reservation", Some(value));
            }
            GeneratedServiceRuntimeField::MemSwappiness(value) => {
                render_optional_string(output, "mem_swappiness", Some(value));
            }
            GeneratedServiceRuntimeField::MemswapLimit(value) => {
                render_optional_string(output, "memswap_limit", Some(value));
            }
            GeneratedServiceRuntimeField::NetworkMode(value) => {
                render_optional_string(output, "network_mode", Some(value));
            }
            GeneratedServiceRuntimeField::OomKillDisable(value) => {
                write_field(output, 2, "oom_kill_disable");
                output.push_str(if *value { "true\n" } else { "false\n" });
            }
            GeneratedServiceRuntimeField::OomScoreAdj(value) => {
                render_optional_string(output, "oom_score_adj", Some(value));
            }
            GeneratedServiceRuntimeField::Pid(value) => {
                render_optional_string(output, "pid", Some(value));
            }
            GeneratedServiceRuntimeField::Scale(value) => {
                render_optional_string(output, "scale", Some(value));
            }
            GeneratedServiceRuntimeField::VolumesFrom(values) => {
                render_configured_string_sequence(output, "volumes_from", values);
            }
        }
    }
}

fn generated_runtime_field_safe(field: &GeneratedServiceRuntimeField) -> bool {
    let safe = |value: &GeneratedString| !value.expose().is_empty() && !value.expose().contains(['\n', '\r', '$']);
    let unsigned = |value: &GeneratedString| safe(value) && value.expose().bytes().all(|byte| byte.is_ascii_digit());
    let bounded_unsigned = |value: &GeneratedString| unsigned(value) && value.expose().parse::<i128>().is_ok();
    let signed_range = |value: &GeneratedString, min: i32, max: i32| {
        safe(value)
            && value
                .expose()
                .parse::<i32>()
                .is_ok_and(|number| (min..=max).contains(&number))
    };
    let decimal = |value: &GeneratedString| safe(value) && normalize_generated_decimal(value.expose()).is_some();
    let reference = |value: &GeneratedString| {
        safe(value)
            && (!value.expose().contains(':')
                || value
                    .expose()
                    .split_once(':')
                    .is_some_and(|(_, target)| !target.is_empty()))
    };
    match field {
        GeneratedServiceRuntimeField::Domainname(value)
        | GeneratedServiceRuntimeField::Isolation(value)
        | GeneratedServiceRuntimeField::MacAddress(value)
        | GeneratedServiceRuntimeField::Uts(value)
        | GeneratedServiceRuntimeField::Cpuset(value) => safe(value),
        GeneratedServiceRuntimeField::UseApiSocket(_) | GeneratedServiceRuntimeField::OomKillDisable(_) => true,
        GeneratedServiceRuntimeField::GpusAll(value) => safe(value) && value.expose() == "all",
        GeneratedServiceRuntimeField::CpuRtRuntime(GeneratedCpuRtRuntime::Microseconds(value)) => unsigned(value),
        GeneratedServiceRuntimeField::CpuShares(value) | GeneratedServiceRuntimeField::Scale(value) => {
            bounded_unsigned(value)
        }
        GeneratedServiceRuntimeField::CpuRtRuntime(GeneratedCpuRtRuntime::Duration(value)) => {
            safe(value)
                && matches!(
                    CpuRtRuntime::parse_string(value.expose().to_owned()),
                    CpuRtRuntime::Duration(_)
                )
        }
        GeneratedServiceRuntimeField::Cpus(value) => decimal(value),
        GeneratedServiceRuntimeField::DeviceCgroupRules(values) => values.iter().all(safe),
        GeneratedServiceRuntimeField::Ipc(value)
        | GeneratedServiceRuntimeField::NetworkMode(value)
        | GeneratedServiceRuntimeField::Pid(value) => reference(value),
        GeneratedServiceRuntimeField::MemReservation(value) => {
            safe(value) && valid_generated_runtime_memory(value.expose(), false)
        }
        GeneratedServiceRuntimeField::MemswapLimit(value) => {
            safe(value) && valid_generated_runtime_memory(value.expose(), true)
        }
        GeneratedServiceRuntimeField::MemSwappiness(value) => signed_range(value, 0, 100),
        GeneratedServiceRuntimeField::OomScoreAdj(value) => signed_range(value, -1000, 1000),
        GeneratedServiceRuntimeField::VolumesFrom(values) => values.iter().all(reference),
    }
}

fn normalize_generated_decimal(value: &str) -> Option<()> {
    let (whole, fraction) = value.split_once('.').map_or((value, ""), |parts| parts);
    let valid_shape = if value.contains('.') {
        !whole.is_empty() && !fraction.is_empty()
    } else {
        !whole.is_empty()
    };
    (valid_shape
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
        && value.bytes().filter(|byte| *byte == b'.').count() <= 1)
        .then_some(())
}

/// Validates resolved byte-value spellings without applying a host-size conversion.
///
/// `memswap_limit` additionally permits Compose's explicit `-1` unlimited branch. The
/// relationship between a positive swap value and `mem_limit` remains a project diagnostic,
/// because it cannot be decided while fields are added independently.
fn valid_generated_runtime_memory(value: &str, allow_unlimited: bool) -> bool {
    if allow_unlimited && value == "-1" {
        return true;
    }
    if !value.is_empty() && value.bytes().all(|byte| byte == b'0') {
        return true;
    }
    let Some(amount) = ["kb", "mb", "gb", "b", "k", "m", "g"]
        .into_iter()
        .find_map(|unit| value.strip_suffix(unit))
    else {
        return false;
    };
    !amount.is_empty() && amount.bytes().all(|byte| byte.is_ascii_digit())
}

fn render_pids_limit(output: &mut String, limit: &GeneratedPidsLimit) {
    write_field(output, 2, "pids_limit");
    match limit {
        GeneratedPidsLimit::Unlimited => output.push_str("-1\n"),
        GeneratedPidsLimit::Finite(decimal) => {
            output.push_str(decimal);
            output.push('\n');
        }
    }
}

fn render_shm_size(output: &mut String, size: &GeneratedShmSize) {
    let GeneratedShmSize::Explicit { amount, unit } = size;
    write_field(output, 2, "shm_size");
    write_quoted(output, &format!("{}{}", amount.expose(), unit.as_str()));
    output.push('\n');
}

fn render_mem_limit(output: &mut String, limit: &GeneratedMemLimit) {
    let GeneratedMemLimit::Explicit { amount, unit } = limit;
    write_field(output, 2, "mem_limit");
    write_quoted(output, &format!("{}{}", amount.expose(), unit.as_str()));
    output.push('\n');
}

fn render_devices(output: &mut String, devices: &[GeneratedDevice]) {
    if devices.is_empty() {
        output.push_str("    devices: []\n");
        return;
    }
    output.push_str("    devices:\n");
    for device in devices {
        match device {
            GeneratedDevice::Short(value) => {
                output.push_str("      - ");
                write_quoted(output, value.expose());
                output.push('\n');
            }
            GeneratedDevice::Long(value) => {
                output.push_str("      - source: ");
                write_quoted(output, value.source().expose());
                output.push('\n');
                if let Some(target) = value.target() {
                    output.push_str("        target: ");
                    write_quoted(output, target.expose());
                    output.push('\n');
                }
                if let Some(permissions) = value.permissions() {
                    output.push_str("        permissions: ");
                    write_quoted(output, permissions.expose());
                    output.push('\n');
                }
            }
        }
    }
}

fn render_dns(output: &mut String, dns: &GeneratedDns) {
    match dns {
        GeneratedDns::Scalar(value) => render_optional_string(output, "dns", Some(value)),
        GeneratedDns::List(values) => render_configured_string_sequence(output, "dns", values),
    }
}

fn render_dns_search(output: &mut String, search: &GeneratedDnsSearch) {
    match search {
        GeneratedDnsSearch::Scalar(value) => render_optional_string(output, "dns_search", Some(value)),
        GeneratedDnsSearch::List(values) => render_configured_string_sequence(output, "dns_search", values),
    }
}

fn render_tmpfs(output: &mut String, tmpfs: &GeneratedTmpfs) {
    match tmpfs {
        GeneratedTmpfs::Scalar(item) => render_optional_string(output, "tmpfs", Some(item)),
        GeneratedTmpfs::List(items) => render_configured_string_sequence(output, "tmpfs", items),
    }
}

fn render_sysctls(output: &mut String, sysctls: &GeneratedSysctls) {
    match sysctls {
        GeneratedSysctls::Map(entries) if entries.is_empty() => output.push_str("    sysctls: {}\n"),
        GeneratedSysctls::Map(entries) => {
            output.push_str("    sysctls:\n");
            for entry in entries {
                write_indent(output, 3);
                write_quoted(output, entry.name());
                output.push_str(": ");
                write_quoted(output, entry.value().expose());
                output.push('\n');
            }
        }
        GeneratedSysctls::List(items) => render_configured_string_sequence(output, "sysctls", items),
    }
}

fn render_logging(output: &mut String, logging: &GeneratedLogging) {
    output.push_str("    logging:\n      driver: ");
    write_quoted(output, logging.driver.expose());
    output.push('\n');
    if logging.options.is_empty() {
        output.push_str("      options: {}\n");
        return;
    }
    output.push_str("      options:\n");
    for option in &logging.options {
        write_indent(output, 4);
        write_quoted(output, option.name());
        output.push_str(": ");
        match option.value() {
            GeneratedLoggingOptionValue::String(value) => write_quoted(output, value.expose()),
            GeneratedLoggingOptionValue::Number(value) => output.push_str(value.expose()),
            GeneratedLoggingOptionValue::Null => output.push_str("null"),
        }
        output.push('\n');
    }
}

fn render_ulimits(output: &mut String, ulimits: &GeneratedUlimits) {
    if ulimits.entries.is_empty() {
        output.push_str("    ulimits: {}\n");
        return;
    }
    output.push_str("    ulimits:\n");
    for limit in &ulimits.entries {
        write_indent(output, 3);
        write_quoted(output, limit.name());
        match limit.value() {
            GeneratedUlimitValue::Single(value) => {
                output.push_str(": ");
                write_quoted(output, value.expose());
                output.push('\n');
            }
            GeneratedUlimitValue::Range {
                soft: Some(soft),
                hard: Some(hard),
            } => {
                output.push_str(":\n");
                write_indent(output, 4);
                output.push_str("soft: ");
                write_quoted(output, soft.expose());
                output.push('\n');
                write_indent(output, 4);
                output.push_str("hard: ");
                write_quoted(output, hard.expose());
                output.push('\n');
            }
            GeneratedUlimitValue::Range { .. } => {
                unreachable!("generated ulimit ranges are validated during construction")
            }
        }
    }
}

fn render_pull_policy(output: &mut String, policy: &GeneratedPullPolicy) {
    write_field(output, 2, "pull_policy");
    let value = match policy {
        GeneratedPullPolicy::Always => "always".to_owned(),
        GeneratedPullPolicy::Never => "never".to_owned(),
        GeneratedPullPolicy::Missing => "missing".to_owned(),
        GeneratedPullPolicy::IfNotPresentAlias => "if_not_present".to_owned(),
        GeneratedPullPolicy::Build => "build".to_owned(),
        GeneratedPullPolicy::Daily => "daily".to_owned(),
        GeneratedPullPolicy::Weekly => "weekly".to_owned(),
        GeneratedPullPolicy::Every(duration) => format!("every_{}", duration.expose()),
    };
    write_quoted(output, &value);
    output.push('\n');
}

fn render_entrypoint(output: &mut String, entrypoint: &GeneratedEntrypoint) {
    match entrypoint {
        GeneratedEntrypoint::List(arguments) if arguments.is_empty() => output.push_str("    entrypoint: []\n"),
        GeneratedEntrypoint::List(arguments) => render_string_sequence(output, "entrypoint", arguments),
        GeneratedEntrypoint::String(entrypoint) => render_optional_string(output, "entrypoint", Some(entrypoint)),
        GeneratedEntrypoint::Empty => output.push_str("    entrypoint: []\n"),
    }
}

fn render_restart(output: &mut String, restart: GeneratedRestartPolicy) {
    write_field(output, 2, "restart");
    let value = match restart {
        GeneratedRestartPolicy::No => "no".to_owned(),
        GeneratedRestartPolicy::Always => "always".to_owned(),
        GeneratedRestartPolicy::OnFailure { maximum_retries: None } => "on-failure".to_owned(),
        GeneratedRestartPolicy::OnFailure {
            maximum_retries: Some(maximum_retries),
        } => format!("on-failure:{maximum_retries}"),
        GeneratedRestartPolicy::UnlessStopped => "unless-stopped".to_owned(),
    };
    write_quoted(output, &value);
    output.push('\n');
}

fn render_optional_string(output: &mut String, key: &str, value: Option<&GeneratedString>) {
    if let Some(value) = value {
        write_field(output, 2, key);
        write_quoted(output, value.expose());
        output.push('\n');
    }
}

fn render_command(output: &mut String, command: &GeneratedCommand) {
    match command {
        GeneratedCommand::Exec(arguments) if arguments.is_empty() => output.push_str("    command: []\n"),
        GeneratedCommand::Exec(arguments) => render_string_sequence(output, "command", arguments),
        GeneratedCommand::Shell(command) => render_optional_string(output, "command", Some(command)),
        GeneratedCommand::Empty => output.push_str("    command: []\n"),
    }
}

fn render_environment(output: &mut String, environment: &[GeneratedEnvironment]) {
    if environment.is_empty() {
        return;
    }
    output.push_str("    environment:\n");
    for variable in environment {
        output.push_str("      - ");
        let value = variable.value.as_ref().map_or_else(
            || variable.name.clone(),
            |value| format!("{}={}", variable.name, value.expose()),
        );
        write_quoted(output, &value);
        output.push('\n');
    }
}

fn render_environment_files(output: &mut String, environment_files: &[GeneratedEnvironmentFile]) {
    if environment_files.is_empty() {
        return;
    }
    output.push_str("    env_file:\n");
    for environment_file in environment_files {
        match environment_file {
            GeneratedEnvironmentFile::Short(path) => {
                output.push_str("      - ");
                write_quoted(output, path.expose());
                output.push('\n');
            }
            GeneratedEnvironmentFile::Long { path, required, format } => {
                output.push_str("      - path: ");
                write_quoted(output, path.expose());
                output.push('\n');
                if let Some(required) = required {
                    output.push_str("        required: ");
                    output.push_str(if *required { "true\n" } else { "false\n" });
                }
                if let Some(format) = format {
                    output.push_str("        format: ");
                    write_quoted(
                        output,
                        match format {
                            GeneratedEnvironmentFileFormat::Raw => "raw",
                        },
                    );
                    output.push('\n');
                }
            }
        }
    }
}

fn render_labels(output: &mut String, labels: &[GeneratedLabel]) {
    if labels.is_empty() {
        return;
    }
    output.push_str("    labels:\n");
    for label in labels {
        output.push_str("      ");
        write_quoted(output, &label.name);
        output.push_str(": ");
        write_quoted(output, label.value.expose());
        output.push('\n');
    }
}

fn render_annotations(output: &mut String, annotations: &[GeneratedAnnotation]) {
    if annotations.is_empty() {
        output.push_str("    annotations: {}\n");
        return;
    }
    output.push_str("    annotations:\n");
    for annotation in annotations {
        output.push_str("      ");
        write_quoted(output, &annotation.name);
        output.push_str(": ");
        write_quoted(output, annotation.value.expose());
        output.push('\n');
    }
}

fn render_string_sequence(output: &mut String, key: &str, values: &[GeneratedString]) {
    if values.is_empty() {
        return;
    }
    write_indent(output, 2);
    output.push_str(key);
    output.push_str(":\n");
    for value in values {
        output.push_str("      - ");
        write_quoted(output, value.expose());
        output.push('\n');
    }
}

fn render_configured_string_sequence(output: &mut String, key: &str, values: &[GeneratedString]) {
    if values.is_empty() {
        write_indent(output, 2);
        output.push_str(key);
        output.push_str(": []\n");
    } else {
        render_string_sequence(output, key, values);
    }
}

fn render_extra_hosts(output: &mut String, hosts: &[GeneratedExtraHost]) {
    if hosts.is_empty() {
        return;
    }
    output.push_str("    extra_hosts:\n");
    for host in hosts {
        output.push_str("      - ");
        write_quoted(output, &format!("{}={}", host.hostname, host.address));
        output.push('\n');
    }
}

fn render_ports(output: &mut String, ports: &[GeneratedPort]) {
    if ports.is_empty() {
        return;
    }
    output.push_str("    ports:\n");
    for port in ports {
        if port.protocol == GeneratedProtocol::Sctp {
            render_short_sctp_port(output, port);
            continue;
        }
        output.push_str("      - target: ");
        output.push_str(&port.target.to_string());
        output.push('\n');
        if let Some(published) = port.published {
            output.push_str("        published: ");
            write_quoted(output, &published.to_string());
            output.push('\n');
        }
        if let Some(host_ip) = &port.host_ip {
            output.push_str("        host_ip: ");
            write_quoted(output, host_ip);
            output.push('\n');
        }
        output.push_str("        protocol: ");
        write_quoted(output, port.protocol.as_str());
        output.push('\n');
    }
}

fn render_short_sctp_port(output: &mut String, port: &GeneratedPort) {
    let mut value = String::new();
    if let Some(host_ip) = &port.host_ip {
        if host_ip.contains(':') && !(host_ip.starts_with('[') && host_ip.ends_with(']')) {
            value.push('[');
            value.push_str(host_ip);
            value.push(']');
        } else {
            value.push_str(host_ip);
        }
        value.push(':');
    }
    if let Some(published) = port.published {
        value.push_str(&published.to_string());
        value.push(':');
    }
    value.push_str(&port.target.to_string());
    value.push_str("/sctp");

    output.push_str("      - ");
    write_quoted(output, &value);
    output.push('\n');
}

fn render_mounts(output: &mut String, mounts: &[GeneratedMount]) {
    if mounts.is_empty() {
        return;
    }
    output.push_str("    volumes:\n");
    for mount in mounts {
        match &mount.kind {
            GeneratedMountKind::Bind {
                source,
                selinux: Some(selinux),
            } => render_selinux_bind(output, source, mount, *selinux),
            kind => render_long_mount(output, kind, mount),
        }
    }
}

fn render_selinux_bind(output: &mut String, source: &str, mount: &GeneratedMount, selinux: GeneratedSelinux) {
    let mut value = format!("{source}:{}:{}", mount.target, selinux.as_str());
    if mount.read_only {
        value.push_str(",ro");
    }
    output.push_str("      - ");
    write_quoted(output, &value);
    output.push('\n');
}

fn render_long_mount(output: &mut String, kind: &GeneratedMountKind, mount: &GeneratedMount) {
    let (mount_type, source) = match kind {
        GeneratedMountKind::Volume { source } => ("volume", Some(source.as_str())),
        GeneratedMountKind::Bind { source, selinux: None } => ("bind", Some(source.as_str())),
        GeneratedMountKind::Anonymous => ("volume", None),
        GeneratedMountKind::Bind { selinux: Some(_), .. } => return,
    };
    output.push_str("      - type: ");
    write_quoted(output, mount_type);
    output.push('\n');
    if let Some(source) = source {
        output.push_str("        source: ");
        write_quoted(output, source);
        output.push('\n');
    }
    output.push_str("        target: ");
    write_quoted(output, &mount.target);
    output.push('\n');
    if mount.read_only {
        output.push_str("        read_only: true\n");
    }
}

fn render_networks(output: &mut String, networks: &[GeneratedNetworkAttachment]) {
    if networks.is_empty() {
        return;
    }
    output.push_str("    networks:\n");
    for network in networks {
        output.push_str("      ");
        write_quoted(output, &network.name);
        if network.aliases.is_empty() && network.ipv4_address.is_none() && network.ipv6_address.is_none() {
            output.push_str(": {}\n");
            continue;
        }
        output.push_str(":\n");
        if !network.aliases.is_empty() {
            output.push_str("        aliases:\n");
            for alias in &network.aliases {
                output.push_str("          - ");
                write_quoted(output, alias);
                output.push('\n');
            }
        }
        for (field, address) in [
            ("ipv4_address", network.ipv4_address.as_ref()),
            ("ipv6_address", network.ipv6_address.as_ref()),
        ] {
            if let Some(address) = address {
                output.push_str("        ");
                output.push_str(field);
                output.push_str(": ");
                write_quoted(output, address.expose());
                output.push('\n');
            }
        }
    }
}

fn render_network_definitions(output: &mut String, networks: &[GeneratedNetwork]) {
    if networks.is_empty() {
        return;
    }
    output.push_str("networks:\n");
    for network in networks {
        match network {
            GeneratedNetwork::Basic(network) => render_basic_resource(output, network),
            GeneratedNetwork::Definition(network) => render_network_definition(output, network),
        }
    }
}

fn render_network_definition(output: &mut String, network: &GeneratedNetworkDefinition) {
    output.push_str("  ");
    write_quoted(output, &network.name);
    if network.custom_name.is_none()
        && network.driver.is_none()
        && network.driver_opts.is_none()
        && network.enable_ipv6.is_none()
        && network.internal.is_none()
        && network.labels.is_none()
    {
        output.push_str(": {}\n");
        return;
    }
    output.push_str(":\n");
    if let Some(custom_name) = &network.custom_name {
        output.push_str("    name: ");
        write_quoted(output, custom_name);
        output.push('\n');
    }
    if let Some(driver) = &network.driver {
        output.push_str("    driver: ");
        write_quoted(output, driver.expose());
        output.push('\n');
    }
    if let Some(driver_opts) = &network.driver_opts {
        if driver_opts.is_empty() {
            output.push_str("    driver_opts: {}\n");
        } else {
            output.push_str("    driver_opts:\n");
            for option in driver_opts {
                output.push_str("      ");
                write_quoted(output, option.name());
                output.push_str(": ");
                match option.value() {
                    GeneratedNetworkDriverOptionValue::String(value) => {
                        write_quoted(output, value.expose());
                    }
                    GeneratedNetworkDriverOptionValue::Number(value) => {
                        output.push_str(value.expose());
                    }
                }
                output.push('\n');
            }
        }
    }
    if let Some(enable_ipv6) = network.enable_ipv6 {
        output.push_str("    enable_ipv6: ");
        output.push_str(if enable_ipv6 { "true\n" } else { "false\n" });
    }
    if let Some(internal) = network.internal {
        output.push_str("    internal: ");
        output.push_str(if internal { "true\n" } else { "false\n" });
    }
    if let Some(labels) = &network.labels {
        if labels.is_empty() {
            output.push_str("    labels: {}\n");
        } else {
            output.push_str("    labels:\n");
            for label in labels {
                output.push_str("      ");
                write_quoted(output, label.name());
                output.push_str(": ");
                write_quoted(output, label.value().expose());
                output.push('\n');
            }
        }
    }
}

fn render_volume_definitions(output: &mut String, volumes: &[GeneratedVolume]) {
    if volumes.is_empty() {
        return;
    }
    output.push_str("volumes:\n");
    for volume in volumes {
        match volume {
            GeneratedVolume::Basic(volume) => render_basic_resource(output, volume),
            GeneratedVolume::Definition(volume) => render_volume_definition(output, volume),
        }
    }
}

fn render_file_definitions<T>(
    output: &mut String,
    field: &str,
    definitions: &[T],
    name: impl Fn(&T) -> &str,
    file: impl Fn(&T) -> &GeneratedString,
) {
    if definitions.is_empty() {
        return;
    }
    output.push_str(field);
    output.push_str(":\n");
    for definition in definitions {
        output.push_str("  ");
        write_quoted(output, name(definition));
        output.push_str(":\n    file: ");
        write_quoted(output, file(definition).expose());
        output.push('\n');
    }
}

fn render_volume_definition(output: &mut String, volume: &GeneratedVolumeDefinition) {
    output.push_str("  ");
    write_quoted(output, &volume.name);
    if volume.custom_name.is_none()
        && volume.driver.is_none()
        && volume.driver_opts.is_none()
        && volume.labels.is_none()
    {
        output.push_str(": {}\n");
        return;
    }
    output.push_str(":\n");
    if let Some(custom_name) = &volume.custom_name {
        output.push_str("    name: ");
        write_quoted(output, custom_name);
        output.push('\n');
    }
    if let Some(driver) = &volume.driver {
        output.push_str("    driver: ");
        write_quoted(output, driver.expose());
        output.push('\n');
    }
    if let Some(driver_opts) = &volume.driver_opts {
        if driver_opts.is_empty() {
            output.push_str("    driver_opts: {}\n");
        } else {
            output.push_str("    driver_opts:\n");
            for option in driver_opts {
                output.push_str("      ");
                write_quoted(output, option.name());
                output.push_str(": ");
                match option.value() {
                    GeneratedVolumeDriverOptionValue::String(value) => write_quoted(output, value.expose()),
                    GeneratedVolumeDriverOptionValue::Number(value) => output.push_str(value.expose()),
                }
                output.push('\n');
            }
        }
    }
    if let Some(labels) = &volume.labels {
        if labels.is_empty() {
            output.push_str("    labels: {}\n");
        } else {
            output.push_str("    labels:\n");
            for label in labels {
                output.push_str("      ");
                write_quoted(output, label.name());
                output.push_str(": ");
                write_quoted(output, label.value().expose());
                output.push('\n');
            }
        }
    }
}

fn render_basic_resource(output: &mut String, resource: &GeneratedResource) {
    output.push_str("  ");
    write_quoted(output, &resource.name);
    if !resource.external && resource.custom_name.is_none() {
        output.push_str(": {}\n");
        return;
    }
    output.push_str(":\n");
    if let Some(custom_name) = &resource.custom_name {
        output.push_str("    name: ");
        write_quoted(output, custom_name);
        output.push('\n');
    }
    if resource.external {
        output.push_str("    external: true\n");
    }
}

fn write_field(output: &mut String, depth: usize, key: &str) {
    write_indent(output, depth);
    output.push_str(key);
    output.push_str(": ");
}

fn write_indent(output: &mut String, depth: usize) {
    for _ in 0..depth {
        output.push_str("  ");
    }
}

fn required(kind: &'static str, value: String) -> Result<String, GenerationError> {
    if value.is_empty() {
        return Err(GenerationError::EmptyValue(kind));
    }
    if value.contains('\0') {
        return Err(GenerationError::ContainsNul(kind));
    }
    Ok(value)
}

fn require_generated_string(kind: &'static str, value: &GeneratedString) -> Result<(), GenerationError> {
    if value.expose().is_empty() {
        return Err(GenerationError::EmptyValue(kind));
    }
    Ok(())
}

fn generated_file_resource_name(value: String) -> Result<String, GenerationError> {
    if value.is_empty() || value.contains(['\0', '\r', '\n', '$']) {
        Err(GenerationError::InvalidFileResourceName)
    } else {
        Ok(value)
    }
}

fn generated_file_resource_path(value: GeneratedString) -> Result<GeneratedString, GenerationError> {
    if value.expose().is_empty() || value.expose().contains(['\0', '\r', '\n', '$']) {
        Err(GenerationError::InvalidFileResourcePath)
    } else {
        Ok(value)
    }
}

fn validate_generated_device_member(
    member: &'static str,
    value: &GeneratedString,
    require_non_empty: bool,
) -> Result<(), GenerationError> {
    if valid_generated_device_string(value.expose(), require_non_empty) {
        Ok(())
    } else {
        Err(GenerationError::InvalidDeviceValue(member))
    }
}

fn validate_generated_ulimit_value(value: &GeneratedString) -> Result<(), GenerationError> {
    let value = value.expose();
    if value.contains(['\r', '\n', '$'])
        || (value != "-1" && (value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit())))
    {
        return Err(GenerationError::InvalidUlimitValue);
    }
    Ok(())
}

fn valid_yaml_number(value: &str) -> bool {
    let ordinary = !value.is_empty()
        && value.bytes().any(|byte| byte.is_ascii_digit())
        && value.bytes().all(|byte| {
            byte.is_ascii_digit()
                || matches!(
                    byte,
                    b'+' | b'-'
                        | b'.'
                        | b'_'
                        | b'e'
                        | b'E'
                        | b'x'
                        | b'X'
                        | b'o'
                        | b'O'
                        | b'a'..=b'f'
                        | b'A'..=b'F'
                )
        });
    let special = matches!(
        value,
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" | "-.inf" | "-.Inf" | "-.INF" | ".nan" | ".NaN" | ".NAN"
    );
    if !ordinary && !special {
        return false;
    }
    let parse = YamlFile::parse(value);
    if !parse.ok() {
        return false;
    }
    let file = parse.tree();
    let Some(document) = file.document() else {
        return false;
    };
    let Some(scalar) = document.as_scalar() else {
        return false;
    };
    let position = scalar.byte_range();
    position.start == 0
        && position.end as usize == value.len()
        && matches!(
            ScalarValue::from_scalar(&scalar).scalar_type(),
            ScalarType::Integer | ScalarType::Float
        )
}

fn environment_name(value: String) -> Result<String, GenerationError> {
    let value = required("environment name", value)?;
    if value.contains('=') {
        return Err(GenerationError::InvalidEnvironmentName);
    }
    Ok(value)
}

fn valid_container_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}

fn short_component(kind: &'static str, value: String, separator: char) -> Result<String, GenerationError> {
    let value = required(kind, value)?;
    if value.contains(separator) {
        return Err(GenerationError::InvalidShortComponent(kind));
    }
    Ok(value)
}

fn set_once<T>(slot: &mut Option<T>, value: T, field: &'static str) -> Result<(), GenerationError> {
    if slot.is_some() {
        return Err(GenerationError::DuplicateField(field));
    }
    *slot = Some(value);
    Ok(())
}

fn insert_named<T>(
    values: &mut Vec<T>,
    value: T,
    kind: &'static str,
    name: impl Fn(&T) -> &str,
) -> Result<(), GenerationError> {
    let value_name = name(&value);
    if values.iter().any(|candidate| name(candidate) == value_name) {
        return Err(GenerationError::DuplicateName {
            kind,
            name: value_name.to_owned(),
        });
    }
    values.push(value);
    Ok(())
}

fn command_is_sensitive(command: &GeneratedCommand) -> bool {
    match command {
        GeneratedCommand::Exec(arguments) => arguments.iter().any(GeneratedString::is_sensitive),
        GeneratedCommand::Shell(command) => command.is_sensitive(),
        GeneratedCommand::Empty => false,
    }
}

fn entrypoint_is_sensitive(entrypoint: &GeneratedEntrypoint) -> bool {
    match entrypoint {
        GeneratedEntrypoint::List(arguments) => arguments.iter().any(GeneratedString::is_sensitive),
        GeneratedEntrypoint::String(entrypoint) => entrypoint.is_sensitive(),
        GeneratedEntrypoint::Empty => false,
    }
}
