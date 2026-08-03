//! Typed health checks and service dependency conditions.

use super::{BooleanValue, FieldReference, Located};
use crate::source::SourceSpan;

/// A service dependency collection with its authored form retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependsOn {
    /// Sequence of service names.
    Short {
        /// The complete sequence span.
        span: SourceSpan,
        /// Dependency service names in authored order.
        services: Vec<Located<String>>,
    },
    /// Mapping of service names to dependency options.
    Long {
        /// The complete mapping span.
        span: SourceSpan,
        /// Dependency entries in authored order.
        services: Vec<ServiceDependency>,
    },
}

impl DependsOn {
    /// Returns the complete collection span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Short { span, .. } | Self::Long { span, .. } => *span,
        }
    }
}

/// One long-syntax service dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDependency {
    service: Located<String>,
    span: SourceSpan,
    condition: Option<Located<DependencyCondition>>,
    restart: Option<Located<BooleanValue>>,
    required: Option<Located<BooleanValue>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl ServiceDependency {
    pub(super) const fn new(service: Located<String>, span: SourceSpan) -> Self {
        Self {
            service,
            span,
            condition: None,
            restart: None,
            required: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_condition(&mut self, value: Located<DependencyCondition>) {
        self.condition = Some(value);
    }

    pub(super) fn set_restart(&mut self, value: Located<BooleanValue>) {
        self.restart = Some(value);
    }

    pub(super) fn set_required(&mut self, value: Located<BooleanValue>) {
        self.required = Some(value);
    }

    pub(super) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(super) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the dependency service name.
    #[must_use]
    pub const fn service(&self) -> &Located<String> {
        &self.service
    }

    /// Returns the complete dependency mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the explicitly authored dependency condition.
    #[must_use]
    pub const fn condition(&self) -> Option<&Located<DependencyCondition>> {
        self.condition.as_ref()
    }

    /// Returns whether explicit Compose updates restart the dependent service.
    #[must_use]
    pub const fn restart(&self) -> Option<&Located<BooleanValue>> {
        self.restart.as_ref()
    }

    /// Returns whether the dependency is required.
    #[must_use]
    pub const fn required(&self) -> Option<&Located<BooleanValue>> {
        self.required.as_ref()
    }

    /// Returns retained `x-` fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns unrecognized dependency fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// A long-syntax dependency condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyCondition {
    /// Wait until the dependency has started.
    ServiceStarted,
    /// Wait until the dependency's health check succeeds.
    ServiceHealthy,
    /// Wait until the dependency exits successfully.
    ServiceCompletedSuccessfully,
    /// A deferred or provider-specific condition.
    Other(String),
}

impl DependencyCondition {
    pub(super) fn parse(value: String) -> Self {
        match value.as_str() {
            "service_started" => Self::ServiceStarted,
            "service_healthy" => Self::ServiceHealthy,
            "service_completed_successfully" => Self::ServiceCompletedSuccessfully,
            _ => Self::Other(value),
        }
    }

    /// Reports whether the condition is defined by the Compose Specification.
    #[must_use]
    pub const fn is_known(&self) -> bool {
        !matches!(self, Self::Other(_))
    }
}

/// A health-check duration retained before interpolation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthcheckDuration {
    /// A syntactically valid Compose duration.
    Value(String),
    /// A deferred interpolation expression.
    Expression(String),
    /// An invalid or provider-specific scalar retained for diagnostics.
    Other(String),
}

impl HealthcheckDuration {
    pub(crate) fn parse(value: String) -> Self {
        if value.contains('$') {
            Self::Expression(value)
        } else if valid_duration(&value) {
            Self::Value(value)
        } else {
            Self::Other(value)
        }
    }

    /// Reports whether this is a valid or deferred value.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        !matches!(self, Self::Other(_))
    }

    /// Returns the retained scalar spelling.
    #[must_use]
    pub fn raw(&self) -> &str {
        match self {
            Self::Value(value) | Self::Expression(value) | Self::Other(value) => value,
        }
    }
}

/// A health-check retry count retained before interpolation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthcheckRetries {
    /// A non-negative integer with its spelling retained.
    Count(String),
    /// A deferred interpolation expression.
    Expression(String),
    /// An invalid or provider-specific scalar retained for diagnostics.
    Other(String),
}

impl HealthcheckRetries {
    pub(crate) fn parse(value: String) -> Self {
        if value.contains('$') {
            Self::Expression(value)
        } else if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
            Self::Count(value)
        } else {
            Self::Other(value)
        }
    }

    /// Reports whether this is a valid or deferred value.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        !matches!(self, Self::Other(_))
    }

    /// Returns the retained scalar spelling.
    #[must_use]
    pub fn raw(&self) -> &str {
        match self {
            Self::Count(value) | Self::Expression(value) | Self::Other(value) => value,
        }
    }
}

/// A service health-check definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Healthcheck {
    span: SourceSpan,
    test: Option<HealthcheckTest>,
    interval: Option<Located<HealthcheckDuration>>,
    timeout: Option<Located<HealthcheckDuration>>,
    retries: Option<Located<HealthcheckRetries>>,
    start_period: Option<Located<HealthcheckDuration>>,
    start_interval: Option<Located<HealthcheckDuration>>,
    disable: Option<Located<BooleanValue>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl Healthcheck {
    pub(super) const fn new(span: SourceSpan) -> Self {
        Self {
            span,
            test: None,
            interval: None,
            timeout: None,
            retries: None,
            start_period: None,
            start_interval: None,
            disable: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_test(&mut self, value: HealthcheckTest) {
        self.test = Some(value);
    }

    pub(super) fn set_interval(&mut self, value: Located<HealthcheckDuration>) {
        self.interval = Some(value);
    }

    pub(super) fn set_timeout(&mut self, value: Located<HealthcheckDuration>) {
        self.timeout = Some(value);
    }

    pub(super) fn set_retries(&mut self, value: Located<HealthcheckRetries>) {
        self.retries = Some(value);
    }

    pub(super) fn set_start_period(&mut self, value: Located<HealthcheckDuration>) {
        self.start_period = Some(value);
    }

    pub(super) fn set_start_interval(&mut self, value: Located<HealthcheckDuration>) {
        self.start_interval = Some(value);
    }

    pub(super) fn set_disable(&mut self, value: Located<BooleanValue>) {
        self.disable = Some(value);
    }

    pub(super) fn push_extension(&mut self, field: FieldReference) {
        self.extension_fields.push(field);
    }

    pub(super) fn push_unknown(&mut self, field: FieldReference) {
        self.unknown_fields.push(field);
    }

    /// Returns the complete health-check mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the health command with its scalar/list form retained.
    #[must_use]
    pub const fn test(&self) -> Option<&HealthcheckTest> {
        self.test.as_ref()
    }

    /// Returns the explicitly authored interval.
    #[must_use]
    pub const fn interval(&self) -> Option<&Located<HealthcheckDuration>> {
        self.interval.as_ref()
    }

    /// Returns the explicitly authored timeout.
    #[must_use]
    pub const fn timeout(&self) -> Option<&Located<HealthcheckDuration>> {
        self.timeout.as_ref()
    }

    /// Returns the explicitly authored retry count.
    #[must_use]
    pub const fn retries(&self) -> Option<&Located<HealthcheckRetries>> {
        self.retries.as_ref()
    }

    /// Returns the explicitly authored start period.
    #[must_use]
    pub const fn start_period(&self) -> Option<&Located<HealthcheckDuration>> {
        self.start_period.as_ref()
    }

    /// Returns the explicitly authored start interval.
    #[must_use]
    pub const fn start_interval(&self) -> Option<&Located<HealthcheckDuration>> {
        self.start_interval.as_ref()
    }

    /// Returns whether the image health check is explicitly disabled.
    #[must_use]
    pub const fn disable(&self) -> Option<&Located<BooleanValue>> {
        self.disable.as_ref()
    }

    /// Reports whether the authored health check is explicitly disabled.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        matches!(
            self.disable.as_ref().map(Located::value),
            Some(BooleanValue::Literal(true))
        ) || matches!(
            self.test.as_ref().and_then(HealthcheckTest::kind),
            Some(HealthcheckTestKind::None)
        )
    }

    /// Returns retained `x-` fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }

    /// Returns unrecognized health-check fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// A health-check command with scalar and list forms kept distinct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthcheckTest {
    /// Scalar form, equivalent to `CMD-SHELL` after processing.
    String(Located<String>),
    /// List form, retaining every authored item.
    List {
        /// The complete sequence span.
        span: SourceSpan,
        /// The command mode derived from the first item, when present.
        kind: Option<HealthcheckTestKind>,
        /// Every list item, including the command-mode token.
        values: Vec<Located<String>>,
    },
}

impl HealthcheckTest {
    /// Returns the effective command mode without rewriting the authored form.
    #[must_use]
    pub fn kind(&self) -> Option<HealthcheckTestKind> {
        match self {
            Self::String(_) => Some(HealthcheckTestKind::CmdShell),
            Self::List { kind, .. } => *kind,
        }
    }

    /// Returns the complete value span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::String(value) => value.span(),
            Self::List { span, .. } => *span,
        }
    }
}

/// The command-mode token at the beginning of a health-check list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HealthcheckTestKind {
    /// Disable the image health check.
    None,
    /// Execute the remaining list directly.
    Cmd,
    /// Execute the remaining string through the container shell.
    CmdShell,
    /// An unrecognized command-mode token.
    Other,
}

impl HealthcheckTestKind {
    pub(crate) fn parse(value: &str) -> Self {
        match value {
            "NONE" => Self::None,
            "CMD" => Self::Cmd,
            "CMD-SHELL" => Self::CmdShell,
            _ => Self::Other,
        }
    }
}

fn valid_duration(mut value: &str) -> bool {
    if value == "0" {
        return true;
    }
    let mut found = false;
    while !value.is_empty() {
        let number_end = value
            .char_indices()
            .take_while(|(_, character)| character.is_ascii_digit() || *character == '.')
            .map(|(index, character)| index + character.len_utf8())
            .last()
            .unwrap_or(0);
        if number_end == 0 {
            return false;
        }
        let number = &value[..number_end];
        if number.matches('.').count() > 1 || number == "." {
            return false;
        }
        value = &value[number_end..];
        let Some(unit) = ["ns", "us", "µs", "μs", "ms", "s", "m", "h"]
            .into_iter()
            .find(|unit| value.starts_with(unit))
        else {
            return false;
        };
        value = &value[unit.len()..];
        found = true;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::valid_duration;

    #[test]
    fn accepts_compose_duration_segments_without_runtime_parsing() {
        for value in ["0", "30s", "1m30s", "1.5s", "250ms", "10us"] {
            assert!(valid_duration(value), "expected valid duration {value}");
        }
        for value in ["", "forever", "-1s", "1", "1..5s"] {
            assert!(!valid_duration(value), "expected invalid duration {value}");
        }
    }
}
