//! Raw-preserving service cgroup namespace values.

use super::Located;

/// A service `cgroup` namespace value with a non-destructive classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CgroupNamespace {
    raw: Located<String>,
    kind: CgroupNamespaceKind,
}

impl CgroupNamespace {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let kind = match raw.value().as_str() {
            "host" => CgroupNamespaceKind::Host,
            "private" => CgroupNamespaceKind::Private,
            value if value.contains('$') => CgroupNamespaceKind::Expression(value.to_owned()),
            value => CgroupNamespaceKind::Other(value.to_owned()),
        };
        Self { raw, kind }
    }

    /// Returns the exact authored cgroup namespace scalar and source span.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns the non-destructive namespace classification.
    #[must_use]
    pub const fn kind(&self) -> &CgroupNamespaceKind {
        &self.kind
    }

    /// Reports whether the value is a documented literal or deferred expression.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        matches!(
            self.kind,
            CgroupNamespaceKind::Host | CgroupNamespaceKind::Private | CgroupNamespaceKind::Expression(_)
        )
    }
}

/// The recognized family of a service `cgroup` namespace value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CgroupNamespaceKind {
    /// Share the host cgroup namespace.
    Host,
    /// Request a private cgroup namespace.
    Private,
    /// A dollar-bearing value deferred to interpolation.
    Expression(String),
    /// An empty, provider-specific, or otherwise unsupported strict YAML string.
    Other(String),
}
