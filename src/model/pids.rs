//! Raw-preserving service PID limits.

use super::Located;

/// A service-level Compose `pids_limit` value with its authored scalar retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PidsLimit {
    raw: Located<String>,
    kind: PidsLimitKind,
}

impl PidsLimit {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let kind = match raw.value().as_str() {
            "-1" => PidsLimitKind::Unlimited,
            value if value.contains('$') => PidsLimitKind::Expression,
            value if decimal_digits(value) => {
                if value.bytes().all(|byte| byte == b'0') {
                    PidsLimitKind::Zero
                } else {
                    PidsLimitKind::Finite {
                        decimal: value.to_owned(),
                    }
                }
            }
            _ => PidsLimitKind::Other,
        };
        Self { raw, kind }
    }

    /// Returns the complete authored scalar and its source span.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns the non-destructive semantic classification.
    #[must_use]
    pub const fn kind(&self) -> &PidsLimitKind {
        &self.kind
    }
}

/// The semantic family of a service-level Compose PID limit.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PidsLimitKind {
    /// No PID limit, authored as `-1`.
    Unlimited,
    /// A positive integral decimal retained without fixed-width parsing or normalization.
    Finite {
        /// Exact positive decimal spelling.
        decimal: String,
    },
    /// An all-zero integral spelling retained as an ambiguous and unportable native state.
    Zero,
    /// A scalar that still contains a Compose interpolation marker.
    Expression,
    /// A fractional, signed, exponent, or otherwise unsupported scalar retained for diagnostics.
    Other,
}

pub(crate) fn valid_positive_pids_decimal(value: &str) -> bool {
    decimal_digits(value) && value.bytes().any(|byte| byte != b'0')
}

fn decimal_digits(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{PidsLimit, PidsLimitKind, valid_positive_pids_decimal};
    use crate::model::Located;
    use crate::source::{SourceId, SourceSpan};

    #[test]
    fn classifies_without_fixed_width_integer_parsing_or_normalization() -> Result<(), &'static str> {
        let cases = [
            ("-1", PidsLimitKind::Unlimited),
            (
                "00042",
                PidsLimitKind::Finite {
                    decimal: "00042".to_owned(),
                },
            ),
            (
                "18446744073709551616000000000000000000000000000000",
                PidsLimitKind::Finite {
                    decimal: "18446744073709551616000000000000000000000000000000".to_owned(),
                },
            ),
            ("000", PidsLimitKind::Zero),
            ("${PIDS_LIMIT:-64}", PidsLimitKind::Expression),
            ("1.5", PidsLimitKind::Other),
            ("1e3", PidsLimitKind::Other),
            ("+1", PidsLimitKind::Other),
        ];
        for (value, expected) in cases {
            let span = SourceSpan::new(SourceId::new(1), 0, value.len()).ok_or("valid test span expected")?;
            let limit = PidsLimit::parse(Located::new(value.to_owned(), span));
            assert_eq!(limit.raw().value(), value);
            assert_eq!(limit.kind(), &expected);
        }
        Ok(())
    }

    #[test]
    fn validates_generated_positive_decimals_without_overflow() {
        for value in ["1", "0001", "18446744073709551616000000000000000000000000000000"] {
            assert!(valid_positive_pids_decimal(value));
        }
        for value in ["", "0", "000", "-1", "+1", "1.0", "1e3", "64MiB"] {
            assert!(!valid_positive_pids_decimal(value));
        }
    }
}
