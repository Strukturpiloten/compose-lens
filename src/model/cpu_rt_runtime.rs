//! Raw-preserving service real-time CPU-runtime values.

use super::lifecycle::valid_stop_grace_period;

/// A service `cpu_rt_runtime` scalar category with exact authored spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CpuRtRuntime {
    /// An integer microsecond spelling retained without conversion.
    Microseconds(String),
    /// A raw Compose duration retained without conversion.
    Duration(String),
    /// A dollar-bearing string retained as a deferred expression.
    Expression(String),
    /// An invalid or unsupported spelling retained with a dedicated diagnostic.
    Other(String),
}

impl CpuRtRuntime {
    pub(crate) fn parse_number(value: String, is_integer: bool) -> Self {
        if is_integer && value.bytes().all(|byte| byte.is_ascii_digit()) {
            Self::Microseconds(value)
        } else {
            Self::Other(value)
        }
    }
    pub(crate) fn parse_string(value: String) -> Self {
        if value.contains('$') {
            Self::Expression(value)
        } else if valid_stop_grace_period(value.trim_end_matches(['\r', '\n'])) {
            Self::Duration(value)
        } else {
            Self::Other(value)
        }
    }
    /// Reports whether the category is an accepted authored form.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        !matches!(self, Self::Other(_))
    }
}
