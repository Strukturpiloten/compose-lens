//! Raw-preserving service real-time CPU-period values.

use super::lifecycle::valid_stop_grace_period;

/// A service `cpu_rt_period` scalar category with exact authored spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CpuRtPeriod {
    /// A YAML numeric scalar retained without conversion or normalization.
    YamlNumber(String),
    /// A raw Compose duration retained without conversion or normalization.
    Duration(String),
    /// A dollar-bearing string retained as a deferred expression.
    Expression(String),
    /// A schema-valid string outside the duration policy, retained with a diagnostic.
    Other(String),
}

impl CpuRtPeriod {
    pub(crate) fn parse_string(value: String) -> Self {
        if value.contains('$') {
            Self::Expression(value)
        } else if valid_stop_grace_period(value.trim_end_matches(['\r', '\n'])) {
            Self::Duration(value)
        } else {
            Self::Other(value)
        }
    }

    pub(crate) const fn is_valid(&self) -> bool {
        !matches!(self, Self::Other(_))
    }
}

#[cfg(test)]
mod tests {
    use super::CpuRtPeriod;

    #[test]
    fn classifies_duration_expression_and_other_spellings() {
        for value in ["1us", "1m30s", "1.5s", ".5s"] {
            assert!(matches!(
                CpuRtPeriod::parse_string(value.to_owned()),
                CpuRtPeriod::Duration(actual) if actual == value
            ));
        }
        assert!(matches!(
            CpuRtPeriod::parse_string("1m30s\n".to_owned()),
            CpuRtPeriod::Duration(value) if value == "1m30s\n"
        ));
        assert!(matches!(
            CpuRtPeriod::parse_string("${CPU_RT_PERIOD}".to_owned()),
            CpuRtPeriod::Expression(value) if value == "${CPU_RT_PERIOD}"
        ));
        for value in ["", "1", "1ns", "1µs", "1μs", "1.s"] {
            assert!(matches!(
                CpuRtPeriod::parse_string(value.to_owned()),
                CpuRtPeriod::Other(actual) if actual == value
            ));
        }
    }
}
