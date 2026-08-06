//! Raw-preserving service image pull policies.

use super::Located;

/// A service-level Compose `pull_policy` value with its authored scalar retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullPolicy {
    raw: Located<String>,
    kind: PullPolicyKind,
}

impl PullPolicy {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let kind = match raw.value().as_str() {
            "always" => PullPolicyKind::Always,
            "never" => PullPolicyKind::Never,
            "missing" => PullPolicyKind::Missing,
            "if_not_present" => PullPolicyKind::IfNotPresentAlias,
            "build" => PullPolicyKind::Build,
            "daily" => PullPolicyKind::Daily,
            "weekly" => PullPolicyKind::Weekly,
            "refresh" => PullPolicyKind::RefreshSchemaOnly,
            value if value.contains('$') => PullPolicyKind::Expression,
            value => parse_every_duration(value).map_or(PullPolicyKind::Other, |duration| PullPolicyKind::Every {
                duration: duration.to_owned(),
            }),
        };
        Self { raw, kind }
    }

    /// Returns the complete authored scalar and its source span.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns the non-destructive policy classification.
    #[must_use]
    pub const fn kind(&self) -> &PullPolicyKind {
        &self.kind
    }

    /// Reports whether the value is documented, deferred, or recognized by the current schema.
    #[must_use]
    pub const fn is_recognized(&self) -> bool {
        !matches!(self.kind, PullPolicyKind::Other)
    }
}

/// The recognized family of a service-level Compose image pull policy.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PullPolicyKind {
    /// Pull the image before every service start.
    Always,
    /// Never pull and rely on a cached image.
    Never,
    /// Pull only when the image is missing.
    Missing,
    /// The retained `if_not_present` alias for [`Self::Missing`].
    IfNotPresentAlias,
    /// Build the image before starting the service.
    Build,
    /// Check for an updated image once per day.
    Daily,
    /// Check for an updated image once per week.
    Weekly,
    /// Check after the retained custom Compose duration.
    Every {
        /// Duration spelling after the `every_` prefix.
        duration: String,
    },
    /// The schema-recognized `refresh` spelling, which lacks matching service-field documentation.
    RefreshSchemaOnly,
    /// A policy that still contains a Compose interpolation expression.
    Expression,
    /// An invalid or provider-specific value retained for diagnostics.
    Other,
}

pub(crate) fn valid_pull_policy_duration(value: &str) -> bool {
    parse_duration(value)
}

fn parse_every_duration(value: &str) -> Option<&str> {
    let duration = value.strip_prefix("every_")?;
    valid_pull_policy_duration(duration).then_some(duration)
}

fn parse_duration(mut value: &str) -> bool {
    let mut found = false;
    while !value.is_empty() {
        let number_end = value.bytes().take_while(u8::is_ascii_digit).count();
        if number_end == 0 {
            return false;
        }
        value = &value[number_end..];
        let Some(unit) = value.bytes().next() else {
            return false;
        };
        if !matches!(unit, b'w' | b'd' | b'h' | b'm' | b's') {
            return false;
        }
        value = &value[1..];
        found = true;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{PullPolicy, PullPolicyKind, valid_pull_policy_duration};
    use crate::model::Located;
    use crate::source::{SourceId, SourceSpan};

    #[test]
    fn classifies_documented_schema_only_deferred_and_other_values() -> Result<(), &'static str> {
        for (value, kind) in [
            ("always", PullPolicyKind::Always),
            ("missing", PullPolicyKind::Missing),
            ("if_not_present", PullPolicyKind::IfNotPresentAlias),
            ("refresh", PullPolicyKind::RefreshSchemaOnly),
            ("${PULL_POLICY:-missing}", PullPolicyKind::Expression),
            ("provider-newest", PullPolicyKind::Other),
        ] {
            let span = SourceSpan::new(SourceId::new(1), 0, value.len()).ok_or("valid test span expected")?;
            assert_eq!(PullPolicy::parse(Located::new(value.to_owned(), span)).kind(), &kind);
        }
        Ok(())
    }

    #[test]
    fn accepts_documented_compose_duration_units_without_normalizing_spelling() {
        for value in ["1w", "2d", "3h", "4m", "5s", "1w2d3h4m5s", "0s", "01h30m"] {
            assert!(valid_pull_policy_duration(value), "expected valid duration {value}");
        }
        for value in ["", "0", "1", "1us", "1ms", "1.5h", ".5h", "1h30", "1x", "h", "1hh"] {
            assert!(!valid_pull_policy_duration(value), "expected invalid duration {value}");
        }
    }
}
