//! Raw-preserving service shared-memory sizes.

use super::Located;

/// A service-level Compose `shm_size` value with its authored scalar retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShmSize {
    raw: Located<String>,
    scalar_kind: ShmSizeScalarKind,
    kind: ShmSizeKind,
}

impl ShmSize {
    pub(crate) fn parse(raw: Located<String>, scalar_kind: ShmSizeScalarKind) -> Self {
        let value = raw.value();
        let kind = if matches!(scalar_kind, ShmSizeScalarKind::String) && value.contains('$') {
            ShmSizeKind::Expression
        } else if let Some((amount_raw, unit)) = split_documented_unit(value) {
            if lexical_zero(amount_raw) {
                ShmSizeKind::Zero {
                    amount_raw: amount_raw.to_owned(),
                    unit: Some(unit),
                }
            } else {
                ShmSizeKind::Documented {
                    amount_raw: amount_raw.to_owned(),
                    unit,
                }
            }
        } else if lexical_zero(value) {
            ShmSizeKind::Zero {
                amount_raw: value.to_owned(),
                unit: None,
            }
        } else {
            match scalar_kind {
                ShmSizeScalarKind::Number => ShmSizeKind::ProviderDependentNumber,
                ShmSizeScalarKind::String => ShmSizeKind::ProviderDependentString,
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
    pub const fn scalar_kind(&self) -> ShmSizeScalarKind {
        self.scalar_kind
    }

    /// Returns the non-destructive shared-memory-size classification.
    #[must_use]
    pub const fn kind(&self) -> &ShmSizeKind {
        &self.kind
    }
}

/// The YAML scalar category of an authored service shared-memory size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShmSizeScalarKind {
    /// A YAML integer or floating-point scalar.
    Number,
    /// A YAML string scalar, including quoted numeric spelling.
    String,
}

/// The raw-preserving semantic family of a service shared-memory size.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShmSizeKind {
    /// A string ending in one documented lowercase suffix.
    Documented {
        /// Exact text before the suffix; no amount grammar is inferred.
        amount_raw: String,
        /// Exact documented suffix family.
        unit: ShmSizeUnit,
    },
    /// An all-zero integral spelling whose Compose semantics are unspecified.
    Zero {
        /// Exact all-zero amount spelling.
        amount_raw: String,
        /// Documented suffix when one was present.
        unit: Option<ShmSizeUnit>,
    },
    /// A dollar-bearing string deferred to Compose interpolation.
    Expression,
    /// A schema-accepted YAML number outside the documented explicit-suffix family.
    ProviderDependentNumber,
    /// A schema-accepted YAML string outside the documented lowercase-suffix family.
    ProviderDependentString,
}

/// One lowercase byte-unit suffix documented for service `shm_size`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShmSizeUnit {
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

impl ShmSizeUnit {
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

pub(crate) fn valid_generated_shm_amount(value: &str) -> bool {
    value
        .as_bytes()
        .split_first()
        .is_some_and(|(first, rest)| (b'1'..=b'9').contains(first) && rest.iter().all(u8::is_ascii_digit))
}

fn split_documented_unit(value: &str) -> Option<(&str, ShmSizeUnit)> {
    for (suffix, unit) in [
        ("kb", ShmSizeUnit::Kb),
        ("mb", ShmSizeUnit::Mb),
        ("gb", ShmSizeUnit::Gb),
        ("b", ShmSizeUnit::B),
        ("k", ShmSizeUnit::K),
        ("m", ShmSizeUnit::M),
        ("g", ShmSizeUnit::G),
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
    use super::{ShmSize, ShmSizeKind, ShmSizeScalarKind, ShmSizeUnit, valid_generated_shm_amount};
    use crate::model::Located;
    use crate::source::{SourceId, SourceSpan};

    fn classify(value: &str, scalar_kind: ShmSizeScalarKind) -> Result<ShmSize, &'static str> {
        let span = SourceSpan::new(SourceId::new(1), 0, value.len()).ok_or("valid test span expected")?;
        Ok(ShmSize::parse(Located::new(value.to_owned(), span), scalar_kind))
    }

    #[test]
    fn retains_documented_suffixes_without_inventing_an_amount_grammar() -> Result<(), &'static str> {
        for (value, amount_raw, unit) in [
            ("1b", "1", ShmSizeUnit::B),
            ("01k", "01", ShmSizeUnit::K),
            ("+1kb", "+1", ShmSizeUnit::Kb),
            ("1.5m", "1.5", ShmSizeUnit::M),
            ("1e3mb", "1e3", ShmSizeUnit::Mb),
            ("-2g", "-2", ShmSizeUnit::G),
            ("hugegb", "huge", ShmSizeUnit::Gb),
        ] {
            let size = classify(value, ShmSizeScalarKind::String)?;
            assert_eq!(size.raw().value(), value);
            assert!(matches!(
                size.kind(),
                ShmSizeKind::Documented { amount_raw: actual, unit: actual_unit }
                    if actual == amount_raw && *actual_unit == unit
            ));
        }
        Ok(())
    }

    #[test]
    fn keeps_zero_expressions_and_schema_scalar_categories_distinct() -> Result<(), &'static str> {
        assert!(matches!(
            classify("000mb", ShmSizeScalarKind::String)?.kind(),
            ShmSizeKind::Zero { amount_raw, unit: Some(ShmSizeUnit::Mb) } if amount_raw == "000"
        ));
        assert!(matches!(
            classify("0", ShmSizeScalarKind::Number)?.kind(),
            ShmSizeKind::Zero { amount_raw, unit: None } if amount_raw == "0"
        ));
        assert_eq!(
            classify("${SHM_SIZE:-64m}", ShmSizeScalarKind::String)?.kind(),
            &ShmSizeKind::Expression
        );
        assert_eq!(
            classify("64", ShmSizeScalarKind::Number)?.kind(),
            &ShmSizeKind::ProviderDependentNumber
        );
        assert_eq!(
            classify("64", ShmSizeScalarKind::String)?.kind(),
            &ShmSizeKind::ProviderDependentString
        );
        Ok(())
    }

    #[test]
    fn validates_only_canonical_positive_generated_amounts() {
        for value in ["1", "64", "18446744073709551616000000000000000000000000000000"] {
            assert!(valid_generated_shm_amount(value));
        }
        for value in ["", "0", "00", "01", "-1", "+1", "1.0", "1e3", " 1", "1 ", "${SIZE}"] {
            assert!(!valid_generated_shm_amount(value));
        }
    }
}
