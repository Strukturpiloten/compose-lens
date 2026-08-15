//! Raw-preserving service resource and namespace values.

use super::{Located, MemLimitUnit};
use crate::source::SourceSpan;

/// A malformed item retained from a sequence that requires YAML string scalars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidServiceStringItem {
    span: SourceSpan,
}
impl InvalidServiceStringItem {
    pub(crate) const fn new(span: SourceSpan) -> Self {
        Self { span }
    }
    /// Returns the exact malformed item's source span.
    #[must_use]
    pub const fn span(self) -> SourceSpan {
        self.span
    }
}

/// The raw spelling of a service integer setting, with a conservative validity classification.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ServiceInteger {
    /// An integral YAML scalar in the documented range for its field.
    Valid(String),
    /// An integral YAML scalar retained outside the field's documented range.
    OutOfRange(String),
    /// A string or scalar expression retained without numeric coercion.
    Other(String),
}

impl ServiceInteger {
    pub(crate) fn parse(value: String, min: i128, max: i128) -> Self {
        match value.parse::<i128>() {
            Ok(number) if (min..=max).contains(&number) => Self::Valid(value),
            Ok(_) => Self::OutOfRange(value),
            Err(_) => Self::Other(value),
        }
    }

    /// Returns whether this is a documented in-range integer spelling.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        matches!(self, Self::Valid(_))
    }
}

/// A service-level Compose `memswap_limit` value with its authored scalar retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemswapLimit {
    raw: Located<String>,
    scalar_kind: MemswapLimitScalarKind,
    kind: MemswapLimitKind,
}

impl MemswapLimit {
    pub(crate) fn parse(raw: Located<String>, scalar_kind: MemswapLimitScalarKind) -> Self {
        let value = raw.value();
        let kind = if value == "-1" {
            MemswapLimitKind::Unlimited
        } else if matches!(scalar_kind, MemswapLimitScalarKind::String) && value.contains('$') {
            MemswapLimitKind::Expression
        } else if let Some((amount_raw, unit)) = quantity_parts(value) {
            if amount_raw.bytes().all(|byte| byte == b'0') {
                MemswapLimitKind::Zero {
                    amount_raw: amount_raw.to_owned(),
                    unit,
                }
            } else {
                MemswapLimitKind::Positive {
                    amount_raw: amount_raw.to_owned(),
                    unit,
                }
            }
        } else {
            MemswapLimitKind::Other(value.to_owned())
        };
        Self { raw, scalar_kind, kind }
    }

    /// Returns the complete scalar value and its source span without normalization.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns whether the authored YAML scalar was a number or string.
    #[must_use]
    pub const fn scalar_kind(&self) -> MemswapLimitScalarKind {
        self.scalar_kind
    }

    /// Returns the raw-preserving memory-plus-swap classification.
    #[must_use]
    pub const fn kind(&self) -> &MemswapLimitKind {
        &self.kind
    }

    pub(crate) const fn is_positive(&self) -> bool {
        matches!(self.kind, MemswapLimitKind::Positive { .. })
    }
}

/// The YAML scalar category of an authored service memory-plus-swap limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MemswapLimitScalarKind {
    /// A YAML integer or floating-point scalar.
    Number,
    /// A YAML string scalar, including quoted numeric spelling.
    String,
}

/// The raw-preserving semantic family of a service memory-plus-swap limit.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MemswapLimitKind {
    /// Compose's explicit unlimited `-1` spelling.
    Unlimited,
    /// An all-zero quantity, kept distinct from omitted and unlimited values.
    Zero {
        /// Exact amount spelling before the optional documented unit.
        amount_raw: String,
        /// The documented suffix when present.
        unit: Option<MemLimitUnit>,
    },
    /// A positive decimal quantity with an optional documented unit.
    Positive {
        /// Exact amount spelling before the optional documented unit.
        amount_raw: String,
        /// The documented suffix when present.
        unit: Option<MemLimitUnit>,
    },
    /// A dollar-bearing string deferred to Compose interpolation.
    Expression,
    /// A malformed or provider-specific spelling retained for inspection.
    Other(String),
}

fn quantity_parts(value: &str) -> Option<(&str, Option<MemLimitUnit>)> {
    let with_unit = [
        ("kb", MemLimitUnit::Kb),
        ("mb", MemLimitUnit::Mb),
        ("gb", MemLimitUnit::Gb),
        ("b", MemLimitUnit::B),
        ("k", MemLimitUnit::K),
        ("m", MemLimitUnit::M),
        ("g", MemLimitUnit::G),
    ]
    .into_iter()
    .find_map(|(suffix, unit)| value.strip_suffix(suffix).map(|amount| (amount, Some(unit))));
    let (amount, unit) = with_unit.unwrap_or((value, None));
    (!amount.is_empty() && amount.bytes().all(|byte| byte.is_ascii_digit())).then_some((amount, unit))
}

/// A raw decimal CPU allocation spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Cpus {
    /// A decimal spelling, including `0.000`.
    Decimal(String),
    /// A deferred interpolation expression.
    Expression(String),
    /// A retained non-decimal spelling.
    Other(String),
}

impl Cpus {
    pub(crate) fn parse(value: String) -> Self {
        if value.contains('$') {
            return Self::Expression(value);
        }
        if !value.is_empty()
            && value.bytes().all(|byte| byte.is_ascii_digit() || byte == b'.')
            && value.bytes().filter(|byte| *byte == b'.').count() <= 1
        {
            Self::Decimal(value)
        } else {
            Self::Other(value)
        }
    }
    /// Returns whether the spelling is decimal or deferred.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        !matches!(self, Self::Other(_))
    }
}

/// A service IPC mode retaining recognized portable service references separately.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum IpcMode {
    /// A documented shareable IPC mode.
    Shareable,
    /// A local service namespace reference.
    Service(String),
    /// Any other scalar spelling retained as evidence.
    Raw(String),
}
impl IpcMode {
    pub(crate) fn parse(value: String) -> Self {
        if value == "shareable" {
            Self::Shareable
        } else if let Some(name) = value.strip_prefix("service:") {
            Self::Service(name.to_owned())
        } else {
            Self::Raw(value)
        }
    }
}

/// A service network mode retaining local namespace reference shapes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NetworkMode {
    /// No network namespace.
    None,
    /// The host network namespace.
    Host,
    /// A local service namespace reference.
    Service(String),
    /// A container namespace reference.
    Container(String),
    /// An unclassified raw scalar.
    Raw(String),
}
impl NetworkMode {
    pub(crate) fn parse(value: String) -> Self {
        match value.as_str() {
            "none" => Self::None,
            "host" => Self::Host,
            _ if value.starts_with("service:") => Self::Service(value[8..].to_owned()),
            _ if value.starts_with("container:") => Self::Container(value[10..].to_owned()),
            _ => Self::Raw(value),
        }
    }
}

/// A PID namespace mode retaining local reference-shaped forms without runtime interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PidMode {
    /// A service namespace reference.
    Service(String),
    /// A container namespace reference.
    Container(String),
    /// Any other raw scalar.
    Raw(String),
}
impl PidMode {
    pub(crate) fn parse(value: String) -> Self {
        if let Some(name) = value.strip_prefix("service:") {
            Self::Service(name.to_owned())
        } else if let Some(name) = value.strip_prefix("container:") {
            Self::Container(name.to_owned())
        } else {
            Self::Raw(value)
        }
    }
}

/// A raw `volumes_from` entry with its default access mode made explicit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumesFrom {
    raw: Located<String>,
    source: String,
    read_only: bool,
}
impl VolumesFrom {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let value = raw.value();
        let (source, read_only) = match value.rsplit_once(':') {
            Some((source, "ro")) => (source.to_owned(), true),
            Some((source, "rw")) => (source.to_owned(), false),
            _ => (value.to_owned(), false),
        };
        Self { raw, source, read_only }
    }
    /// Returns the original complete entry and span.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }
    /// Returns the referenced service or container spelling.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Returns whether `ro` was requested; omitted access defaults to `rw`.
    #[must_use]
    pub const fn read_only(&self) -> bool {
        self.read_only
    }
}
