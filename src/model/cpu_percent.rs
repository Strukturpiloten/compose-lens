//! Raw-preserving service CPU-percentage values.

/// A service `cpu_percent` scalar category with exact authored spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CpuPercent {
    /// A YAML integer in the schema's inclusive `0..=100` range.
    YamlInteger(String),
    /// A YAML string scalar retained without numeric coercion.
    String(String),
    /// A YAML integer outside the schema's inclusive `0..=100` range, retained as invalid evidence.
    OutOfRangeYamlInteger(String),
}

impl CpuPercent {
    pub(crate) fn yaml_integer(raw: String) -> Self {
        if yaml_integer_in_percent_range(&raw) {
            Self::YamlInteger(raw)
        } else {
            Self::OutOfRangeYamlInteger(raw)
        }
    }

    /// Returns whether this scalar satisfies the schema's inclusive integer range.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        !matches!(self, Self::OutOfRangeYamlInteger(_))
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

fn yaml_integer_in_percent_range(value: &str) -> bool {
    let (negative, value) = match value.strip_prefix('-') {
        Some(value) => (true, value),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let (radix, digits) = if let Some(value) = value.strip_prefix("0b") {
        (2_u16, value)
    } else if let Some(value) = value.strip_prefix("0o") {
        (8_u16, value)
    } else if let Some(value) = value.strip_prefix("0x") {
        (16_u16, value)
    } else {
        (10_u16, value)
    };

    let mut result = 0_u16;
    let mut nonzero = false;
    for byte in digits.bytes().filter(|byte| *byte != b'_') {
        let digit = if byte.is_ascii_digit() {
            u16::from(byte - b'0')
        } else {
            u16::from(byte.to_ascii_lowercase() - b'a' + 10)
        };
        nonzero |= digit != 0;
        result = result.saturating_mul(radix).saturating_add(digit);
        if result > 100 {
            return false;
        }
    }
    !negative || !nonzero
}

#[cfg(test)]
mod tests {
    use super::CpuPercent;

    #[test]
    fn classifies_unbounded_yaml_integer_spellings_without_conversion() {
        for value in ["0", "-0", "-0x0", "+100", "0b110_0100", "0o1_44", "0x64"] {
            assert!(matches!(
                CpuPercent::yaml_integer(value.to_owned()),
                CpuPercent::YamlInteger(actual) if actual == value
            ));
            assert!(CpuPercent::yaml_integer_spelling(value));
        }
        for value in ["-1", "101", "0x65", "999999999999999999999999999999"] {
            assert!(matches!(
                CpuPercent::yaml_integer(value.to_owned()),
                CpuPercent::OutOfRangeYamlInteger(actual) if actual == value
            ));
        }
        for value in ["1.0", "1e3", "0x_1", "1_"] {
            assert!(!CpuPercent::yaml_integer_spelling(value));
        }
    }
}
