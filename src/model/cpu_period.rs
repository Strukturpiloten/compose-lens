//! Raw-preserving service CPU-period values.

/// A service `cpu_period` scalar category with exact authored spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CpuPeriod {
    /// A YAML numeric scalar retained without conversion or normalization.
    YamlNumber(String),
    /// A YAML string scalar retained without numeric coercion.
    String(String),
}

impl CpuPeriod {
    /// Returns whether a plain scalar spelling is a YAML numeric form.
    pub(crate) fn yaml_number_spelling(value: &str) -> bool {
        let value = value
            .strip_prefix('+')
            .or_else(|| value.strip_prefix('-'))
            .unwrap_or(value);
        if let Some(digits) = value.strip_prefix("0b") {
            return digit_sequence(digits, 2);
        }
        if let Some(digits) = value.strip_prefix("0o") {
            return digit_sequence(digits, 8);
        }
        if let Some(digits) = value.strip_prefix("0x") {
            return digit_sequence(digits, 16);
        }

        let (mantissa, exponent) = value
            .split_once(['e', 'E'])
            .map_or((value, None), |(mantissa, exponent)| (mantissa, Some(exponent)));
        if exponent.is_some_and(|exponent| {
            let exponent = exponent
                .strip_prefix('+')
                .or_else(|| exponent.strip_prefix('-'))
                .unwrap_or(exponent);
            !digit_sequence(exponent, 10)
        }) {
            return false;
        }
        let (integer, fraction) = mantissa
            .split_once('.')
            .map_or((mantissa, None), |(integer, fraction)| (integer, Some(fraction)));
        digit_sequence(integer, 10)
            && fraction.is_none_or(|fraction| fraction.is_empty() || digit_sequence(fraction, 10))
    }
}

fn digit_sequence(value: &str, radix: u8) -> bool {
    let mut saw_digit = false;
    let mut previous_separator = false;
    for byte in value.bytes() {
        if byte == b'_' {
            if !saw_digit || previous_separator {
                return false;
            }
            previous_separator = true;
            continue;
        }
        let digit = if byte.is_ascii_digit() {
            byte - b'0'
        } else if byte.is_ascii_alphabetic() {
            byte.to_ascii_lowercase() - b'a' + 10
        } else {
            return false;
        };
        if digit >= radix {
            return false;
        }
        saw_digit = true;
        previous_separator = false;
    }
    saw_digit && !previous_separator
}

#[cfg(test)]
mod tests {
    use super::CpuPeriod;

    #[test]
    fn recognizes_plain_yaml_number_spellings() {
        for value in ["-0xF_F", "+1.5", "1e+6", "1.", "0b1_0", "0o7_7"] {
            assert!(CpuPeriod::yaml_number_spelling(value));
        }
        for value in ["", "1_", "0x_1", "1e", ".5", "opaque"] {
            assert!(!CpuPeriod::yaml_number_spelling(value));
        }
    }
}
