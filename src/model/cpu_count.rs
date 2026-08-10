//! Raw-preserving service CPU-count values.

/// A service `cpu_count` scalar category with exact authored spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CpuCount {
    /// A nonnegative YAML integer retained without fixed-width conversion or normalization.
    YamlInteger(String),
    /// A YAML string scalar retained without numeric coercion.
    String(String),
    /// A negative YAML integer retained as typed invalid evidence.
    NegativeYamlInteger(String),
}

impl CpuCount {
    pub(crate) fn yaml_integer(raw: String) -> Self {
        if raw.starts_with('-') && !negative_integer_is_zero(&raw) {
            Self::NegativeYamlInteger(raw)
        } else {
            Self::YamlInteger(raw)
        }
    }

    /// Returns whether this scalar satisfies the schema's nonnegative-integer rule.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        !matches!(self, Self::NegativeYamlInteger(_))
    }

    pub(crate) fn yaml_integer_spelling(value: &str) -> bool {
        let digits = value
            .strip_prefix('+')
            .or_else(|| value.strip_prefix('-'))
            .unwrap_or(value);
        let (radix, digits) = if let Some(value) = digits.strip_prefix("0b") {
            (2, value)
        } else if let Some(value) = digits.strip_prefix("0o") {
            (8, value)
        } else if let Some(value) = digits.strip_prefix("0x") {
            (16, value)
        } else {
            (10, digits)
        };
        let mut saw_digit = false;
        let mut previous_separator = false;
        for byte in digits.bytes() {
            if byte == b'_' {
                if !saw_digit || previous_separator {
                    return false;
                }
                previous_separator = true;
            } else if if radix == 16 {
                byte.is_ascii_hexdigit()
            } else {
                byte.is_ascii_digit() && (byte - b'0') < radix
            } {
                saw_digit = true;
                previous_separator = false;
            } else {
                return false;
            }
        }
        saw_digit && !previous_separator
    }
}

fn negative_integer_is_zero(value: &str) -> bool {
    let Some(value) = value.strip_prefix('-') else {
        return false;
    };
    let digits = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0o"))
        .or_else(|| value.strip_prefix("0x"))
        .unwrap_or(value);
    digits.bytes().all(|byte| matches!(byte, b'0' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::CpuCount;

    #[test]
    fn retains_unbounded_integer_spelling_and_negative_zero() {
        for value in [
            "0",
            "-0",
            "-0x0",
            "0b1_0",
            "0o7_7",
            "0xCA_FE",
            "999999999999999999999999999999",
        ] {
            assert!(CpuCount::yaml_integer(value.to_owned()).is_valid());
            assert!(CpuCount::yaml_integer_spelling(value));
        }
        assert!(matches!(
            CpuCount::yaml_integer("-1".to_owned()),
            CpuCount::NegativeYamlInteger(value) if value == "-1"
        ));
        for value in ["1.0", "1e3", "0x_1", "1_"] {
            assert!(!CpuCount::yaml_integer_spelling(value));
        }
    }
}
