//! Raw-preserving service memory limits.

use super::Located;

/// A service-level Compose `mem_limit` value with its authored scalar retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemLimit {
    raw: Located<String>,
    scalar_kind: MemLimitScalarKind,
    kind: MemLimitKind,
}

impl MemLimit {
    pub(crate) fn parse(raw: Located<String>, scalar_kind: MemLimitScalarKind) -> Self {
        let value = raw.value();
        let kind = if matches!(scalar_kind, MemLimitScalarKind::String) && value.contains('$') {
            MemLimitKind::Expression
        } else if let Some((amount_raw, unit)) = split_documented_unit(value) {
            if lexical_zero(amount_raw) {
                MemLimitKind::Zero {
                    amount_raw: amount_raw.to_owned(),
                    unit: Some(unit),
                }
            } else {
                MemLimitKind::Documented {
                    amount_raw: amount_raw.to_owned(),
                    unit,
                }
            }
        } else if lexical_zero(value) {
            MemLimitKind::Zero {
                amount_raw: value.to_owned(),
                unit: None,
            }
        } else {
            match scalar_kind {
                MemLimitScalarKind::Number => MemLimitKind::SchemaNumber,
                MemLimitScalarKind::String => MemLimitKind::ProviderDependentString,
            }
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
    pub const fn scalar_kind(&self) -> MemLimitScalarKind {
        self.scalar_kind
    }

    /// Returns the non-destructive service memory-limit classification.
    #[must_use]
    pub const fn kind(&self) -> &MemLimitKind {
        &self.kind
    }
}

/// The YAML scalar category of an authored service memory limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MemLimitScalarKind {
    /// A YAML integer or floating-point scalar.
    Number,
    /// A YAML string scalar, including quoted numeric spelling.
    String,
}

/// The raw-preserving semantic family of a service memory limit.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MemLimitKind {
    /// A string ending in one documented lowercase suffix.
    Documented {
        /// Exact text before the suffix; no amount grammar is inferred.
        amount_raw: String,
        /// Exact documented suffix family.
        unit: MemLimitUnit,
    },
    /// An all-zero integral spelling whose portable runtime meaning is not inferred.
    Zero {
        /// Exact all-zero amount spelling.
        amount_raw: String,
        /// Documented suffix when one was present.
        unit: Option<MemLimitUnit>,
    },
    /// A dollar-bearing string deferred to Compose interpolation.
    Expression,
    /// A schema-accepted YAML number without a documented explicit unit.
    SchemaNumber,
    /// A schema-accepted YAML string outside the documented lowercase-suffix family.
    ProviderDependentString,
}

/// One lowercase byte-unit suffix documented for service `mem_limit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MemLimitUnit {
    /// Bytes (`b`).
    B,
    /// Kilobytes (`k`).
    K,
    /// Kilobytes (`kb`).
    Kb,
    /// Megabytes (`m`).
    M,
    /// Megabytes (`mb`).
    Mb,
    /// Gigabytes (`g`).
    G,
    /// Gigabytes (`gb`).
    Gb,
}

impl MemLimitUnit {
    /// Returns the exact lowercase documented suffix.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::B => "b",
            Self::K => "k",
            Self::Kb => "kb",
            Self::M => "m",
            Self::Mb => "mb",
            Self::G => "g",
            Self::Gb => "gb",
        }
    }
}

pub(crate) fn valid_generated_mem_amount(value: &str) -> bool {
    value
        .as_bytes()
        .split_first()
        .is_some_and(|(first, rest)| (b'1'..=b'9').contains(first) && rest.iter().all(u8::is_ascii_digit))
}

fn split_documented_unit(value: &str) -> Option<(&str, MemLimitUnit)> {
    for (suffix, unit) in [
        ("kb", MemLimitUnit::Kb),
        ("mb", MemLimitUnit::Mb),
        ("gb", MemLimitUnit::Gb),
        ("b", MemLimitUnit::B),
        ("k", MemLimitUnit::K),
        ("m", MemLimitUnit::M),
        ("g", MemLimitUnit::G),
    ] {
        if let Some(amount) = value.strip_suffix(suffix) {
            if !amount.is_empty() {
                return Some((amount, unit));
            }
        }
    }
    None
}

fn lexical_zero(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte == b'0')
}

#[cfg(test)]
mod tests {
    use super::{MemLimit, MemLimitKind, MemLimitScalarKind, MemLimitUnit, valid_generated_mem_amount};
    use crate::model::Located;
    use crate::source::{SourceId, SourceSpan};

    fn classify(value: &str, scalar_kind: MemLimitScalarKind) -> Result<MemLimit, &'static str> {
        let span = SourceSpan::new(SourceId::new(1), 0, value.len()).ok_or("valid test span expected")?;
        Ok(MemLimit::parse(Located::new(value.to_owned(), span), scalar_kind))
    }

    #[test]
    fn keeps_documented_units_raw_and_schema_forms_distinct() -> Result<(), &'static str> {
        let bytes = classify("001b", MemLimitScalarKind::String)?;
        assert!(matches!(
            bytes.kind(),
            MemLimitKind::Documented { amount_raw, unit: MemLimitUnit::B } if amount_raw == "001"
        ));
        assert_eq!(
            classify("${LIMIT:-64m}", MemLimitScalarKind::String)?.kind(),
            &MemLimitKind::Expression
        );
        assert_eq!(
            classify("64", MemLimitScalarKind::Number)?.kind(),
            &MemLimitKind::SchemaNumber
        );
        assert_eq!(
            classify("64", MemLimitScalarKind::String)?.kind(),
            &MemLimitKind::ProviderDependentString
        );
        assert!(matches!(
            classify("000mb", MemLimitScalarKind::String)?.kind(),
            MemLimitKind::Zero { amount_raw, unit: Some(MemLimitUnit::Mb) } if amount_raw == "000"
        ));
        Ok(())
    }

    #[test]
    fn validates_only_canonical_positive_generated_amounts() {
        for value in ["1", "64", "18446744073709551616000000000000000000000000000000"] {
            assert!(valid_generated_mem_amount(value));
        }
        for value in ["", "0", "00", "01", "-1", "+1", "1.0", "1e3", " 1", "1 ", "${LIMIT}"] {
            assert!(!valid_generated_mem_amount(value));
        }
    }
}
