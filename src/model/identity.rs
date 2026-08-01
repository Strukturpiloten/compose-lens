//! Raw-preserving container identity and user-namespace values.

use super::Located;

/// A service `user` value with optional user/group decomposition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSpec {
    raw: Located<String>,
    user: IdentityComponent,
    group: Option<IdentityComponent>,
}

impl UserSpec {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let (user, group) = split_user_group(raw.value())
            .map_or_else(|| (raw.value().as_str(), None), |(user, group)| (user, Some(group)));
        Self {
            user: IdentityComponent::parse(user),
            group: group.map(IdentityComponent::parse),
            raw,
        }
    }

    /// Returns the complete authored scalar.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns the user component without resolving names or IDs.
    #[must_use]
    pub const fn user(&self) -> &IdentityComponent {
        &self.user
    }

    /// Returns the optional group component.
    #[must_use]
    pub const fn group(&self) -> Option<&IdentityComponent> {
        self.group.as_ref()
    }
}

/// One lexical component of a container user/group value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityComponent {
    /// An unsigned numeric UID or GID, with spelling retained.
    Numeric(String),
    /// A user or group name.
    Name(String),
    /// A deferred Compose interpolation expression.
    Expression(String),
    /// An explicitly empty component.
    Empty,
}

impl IdentityComponent {
    fn parse(value: &str) -> Self {
        if value.is_empty() {
            Self::Empty
        } else if value.contains("${") || value.contains("$$") {
            Self::Expression(value.to_owned())
        } else if value.bytes().all(|byte| byte.is_ascii_digit()) {
            Self::Numeric(value.to_owned())
        } else {
            Self::Name(value.to_owned())
        }
    }

    /// Returns the retained component spelling.
    #[must_use]
    pub fn raw(&self) -> &str {
        match self {
            Self::Numeric(value) | Self::Name(value) | Self::Expression(value) => value,
            Self::Empty => "",
        }
    }
}

/// A service `userns_mode` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserNamespaceMode {
    raw: Located<String>,
    kind: UserNamespaceModeKind,
}

impl UserNamespaceMode {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let kind = match raw.value().as_str() {
            "host" | "" => UserNamespaceModeKind::Host,
            value if value == "keep-id" || value.starts_with("keep-id:") => UserNamespaceModeKind::PodmanKeepId,
            value if value == "auto" || value.starts_with("auto:") => UserNamespaceModeKind::PodmanAuto,
            "nomap" => UserNamespaceModeKind::PodmanNoMap,
            value if value.starts_with("container:") => UserNamespaceModeKind::Container,
            _ => UserNamespaceModeKind::Other,
        };
        Self { raw, kind }
    }

    /// Returns the complete authored scalar.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns the non-destructive mode classification.
    #[must_use]
    pub const fn kind(&self) -> UserNamespaceModeKind {
        self.kind
    }

    /// Reports whether the value selects a Podman-specific namespace mode.
    #[must_use]
    pub const fn is_podman_specific(&self) -> bool {
        matches!(
            self.kind,
            UserNamespaceModeKind::PodmanKeepId
                | UserNamespaceModeKind::PodmanAuto
                | UserNamespaceModeKind::PodmanNoMap
        )
    }
}

/// The recognized family of a `userns_mode` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserNamespaceModeKind {
    /// Host user namespace.
    Host,
    /// Podman's `keep-id` mode and options.
    PodmanKeepId,
    /// Podman's `auto` mode and options.
    PodmanAuto,
    /// Podman's `nomap` mode.
    PodmanNoMap,
    /// A container namespace reference.
    Container,
    /// A deferred or provider-specific value.
    Other,
}

fn split_user_group(value: &str) -> Option<(&str, &str)> {
    let bytes = value.as_bytes();
    let mut interpolation_depth = 0_usize;
    let mut index = 0_usize;
    while index < bytes.len() {
        if bytes[index] == b'$' && bytes.get(index + 1) == Some(&b'{') {
            interpolation_depth += 1;
            index += 2;
            continue;
        }
        if bytes[index] == b'}' && interpolation_depth > 0 {
            interpolation_depth -= 1;
        } else if bytes[index] == b':' && interpolation_depth == 0 {
            return Some((&value[..index], &value[index + 1..]));
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{IdentityComponent, UserSpec};
    use crate::model::Located;
    use crate::source::{SourceId, SourceSpan};

    #[test]
    fn does_not_split_interpolation_default_operators() -> Result<(), &'static str> {
        let value = "${UID:-1000}:${GID:-1000}";
        let span = SourceSpan::new(SourceId::new(1), 0, value.len()).ok_or("valid test span expected")?;
        let parsed = UserSpec::parse(Located::new(value.to_owned(), span));
        assert_eq!(parsed.user(), &IdentityComponent::Expression("${UID:-1000}".to_owned()));
        assert_eq!(
            parsed.group(),
            Some(&IdentityComponent::Expression("${GID:-1000}".to_owned()))
        );
        Ok(())
    }
}
