//! Raw-preserving service restart policies.

use super::Located;

/// A service-level Compose `restart` value with its authored scalar retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartPolicy {
    raw: Located<String>,
    kind: RestartPolicyKind,
}

impl RestartPolicy {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let kind = match raw.value().as_str() {
            "no" => RestartPolicyKind::No,
            "always" => RestartPolicyKind::Always,
            "on-failure" => RestartPolicyKind::OnFailure { maximum_retries: None },
            "unless-stopped" => RestartPolicyKind::UnlessStopped,
            value if value.contains('$') => RestartPolicyKind::Expression,
            value => parse_maximum_retries(value).map_or(RestartPolicyKind::Other, |maximum_retries| {
                RestartPolicyKind::OnFailure {
                    maximum_retries: Some(maximum_retries.to_owned()),
                }
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
    pub const fn kind(&self) -> &RestartPolicyKind {
        &self.kind
    }

    /// Reports whether the value is defined by Compose or is deferred through interpolation.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        !matches!(self.kind, RestartPolicyKind::Other)
    }
}

/// The recognized family of a service-level Compose restart policy.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RestartPolicyKind {
    /// Never restart the container automatically.
    No,
    /// Always restart the container until it is removed.
    Always,
    /// Restart after an error, optionally with the authored maximum-retry spelling.
    OnFailure {
        /// Decimal maximum-retry spelling, retained without numeric normalization.
        maximum_retries: Option<String>,
    },
    /// Restart except after an explicit stop or removal.
    UnlessStopped,
    /// A policy that still contains a Compose interpolation expression.
    Expression,
    /// An invalid or provider-specific value retained for diagnostics.
    Other,
}

fn parse_maximum_retries(value: &str) -> Option<&str> {
    let retries = value.strip_prefix("on-failure:")?;
    (!retries.is_empty() && retries.bytes().all(|byte| byte.is_ascii_digit())).then_some(retries)
}

#[cfg(test)]
mod tests {
    use super::{RestartPolicy, RestartPolicyKind};
    use crate::model::Located;
    use crate::source::{SourceId, SourceSpan};

    #[test]
    fn retains_maximum_retry_spelling_without_normalizing_it() -> Result<(), &'static str> {
        let value = "on-failure:003";
        let span = SourceSpan::new(SourceId::new(1), 0, value.len()).ok_or("valid test span expected")?;
        let policy = RestartPolicy::parse(Located::new(value.to_owned(), span));

        assert_eq!(policy.raw().value(), value);
        assert_eq!(
            policy.kind(),
            &RestartPolicyKind::OnFailure {
                maximum_retries: Some("003".to_owned()),
            }
        );
        assert!(policy.is_valid());
        Ok(())
    }
}
