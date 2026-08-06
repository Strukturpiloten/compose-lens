//! Raw-preserving service lifecycle controls.

/// A service stop grace period retained before interpolation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopGracePeriod {
    /// A duration accepted by `ComposeLens`'s raw-preserving policy, with its spelling retained.
    Value(String),
    /// An interpolation-shaped scalar classified by the existing dollar-marker convention.
    Expression(String),
    /// An invalid or provider-specific scalar retained for diagnostics.
    Other(String),
}

impl StopGracePeriod {
    pub(crate) fn parse(value: String) -> Self {
        if value.contains('$') {
            Self::Expression(value)
        } else if valid_stop_grace_period(&value) {
            Self::Value(value)
        } else {
            Self::Other(value)
        }
    }

    /// Reports whether this is accepted by the raw-preserving policy or is interpolation-shaped.
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

pub(crate) fn valid_stop_grace_period(mut value: &str) -> bool {
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
        if number.matches('.').count() > 1 || number == "." || number.ends_with('.') {
            return false;
        }
        value = &value[number_end..];
        let Some(unit) = ["us", "ms", "s", "m", "h"]
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
    use super::{StopGracePeriod, valid_stop_grace_period};

    #[test]
    fn applies_the_raw_preserving_policy_using_documented_compose_units() {
        for value in ["1us", "1ms", "1s", "1m", "1h", "1m30s", "0s", "1.5s", ".5s"] {
            assert!(valid_stop_grace_period(value), "expected valid duration {value}");
        }
        for value in ["", "0", "1", "1ns", "1µs", "1μs", "1d", "s", ".s", "1.s", "1..5s"] {
            assert!(!valid_stop_grace_period(value), "expected invalid duration {value}");
        }
    }

    #[test]
    fn retains_valid_expression_and_other_spelling() {
        assert_eq!(
            StopGracePeriod::parse("1m30s".to_owned()),
            StopGracePeriod::Value("1m30s".to_owned())
        );
        assert_eq!(
            StopGracePeriod::parse("${STOP_GRACE_PERIOD:-1s}".to_owned()),
            StopGracePeriod::Expression("${STOP_GRACE_PERIOD:-1s}".to_owned())
        );
        assert_eq!(
            StopGracePeriod::parse("1ns".to_owned()),
            StopGracePeriod::Other("1ns".to_owned())
        );
    }

    #[test]
    fn uses_the_existing_dollar_marker_as_a_lexical_expression_classification() {
        assert_eq!(
            StopGracePeriod::parse("literal$5".to_owned()),
            StopGracePeriod::Expression("literal$5".to_owned())
        );
    }
}
