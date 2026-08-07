//! Raw-preserving service exposed-port declarations.

use crate::source::SourceSpan;

use super::Located;

/// The YAML scalar category of one authored service `expose` item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExposeScalarKind {
    /// A YAML number scalar.
    Number,
    /// A YAML string scalar, including quoted decimal spelling.
    String,
}

/// A documented transport protocol suffix on one service `expose` item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExposeProtocol {
    /// Transmission Control Protocol (`tcp`).
    Tcp,
    /// User Datagram Protocol (`udp`).
    Udp,
}

/// A decimal exposed port or inclusive decimal range, retained without integer parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExposePort {
    start: String,
    end: Option<String>,
}

impl ExposePort {
    /// Returns the exact first decimal spelling.
    #[must_use]
    pub fn start(&self) -> &str {
        &self.start
    }

    /// Returns the exact range end spelling when one was authored.
    #[must_use]
    pub fn end(&self) -> Option<&str> {
        self.end.as_deref()
    }
}

/// The conservative semantic family of one service `expose` item.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExposeItemKind {
    /// A documented decimal port or range with an omitted, `tcp`, or `udp` suffix.
    Documented {
        /// Exact decimal port or range components.
        port: ExposePort,
        /// Exact documented protocol when explicitly present.
        protocol: Option<ExposeProtocol>,
    },
    /// A well-shaped decimal port or range using the schema-recognized `sctp` suffix.
    Sctp {
        /// Exact decimal port or range components.
        port: ExposePort,
    },
    /// A well-shaped decimal port or range using another raw protocol token.
    UnknownProtocol {
        /// Exact decimal port or range components.
        port: ExposePort,
        /// Exact unrecognized protocol spelling.
        protocol: String,
    },
    /// A string whose effective spelling depends on Compose interpolation.
    Expression,
    /// An empty or otherwise malformed scalar retained for diagnostics.
    Malformed,
}

/// One exact source-aware service `expose` sequence item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExposeItem {
    raw: Located<String>,
    scalar_kind: ExposeScalarKind,
    kind: ExposeItemKind,
}

impl ExposeItem {
    pub(crate) fn parse(raw: Located<String>, scalar_kind: ExposeScalarKind) -> Self {
        let kind = classify_expose_item(raw.value(), scalar_kind);
        Self { raw, scalar_kind, kind }
    }

    /// Returns the exact scalar value without normalizing its port, range, or protocol.
    #[must_use]
    pub fn value(&self) -> &str {
        self.raw.value()
    }

    /// Returns the exact source span of this scalar item.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.raw.span()
    }

    /// Returns whether the authored YAML scalar was a string or number.
    #[must_use]
    pub const fn scalar_kind(&self) -> ExposeScalarKind {
        self.scalar_kind
    }

    /// Returns the conservative raw-preserving item classification.
    #[must_use]
    pub const fn kind(&self) -> &ExposeItemKind {
        &self.kind
    }
}

/// An explicitly authored ordered service `expose` sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expose {
    span: SourceSpan,
    items: Vec<ExposeItem>,
}

impl Expose {
    pub(crate) const fn new(span: SourceSpan, items: Vec<ExposeItem>) -> Self {
        Self { span, items }
    }

    /// Returns the exact span of the complete authored sequence.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns items in authored order, including exact duplicates.
    #[must_use]
    pub fn items(&self) -> &[ExposeItem] {
        &self.items
    }
}

pub(crate) fn classify_expose_item(value: &str, scalar_kind: ExposeScalarKind) -> ExposeItemKind {
    if scalar_kind == ExposeScalarKind::String && value.contains('$') {
        return ExposeItemKind::Expression;
    }
    let (port_value, protocol) = match value.split_once('/') {
        Some((port, protocol)) if !protocol.is_empty() && !protocol.contains('/') => (port, Some(protocol)),
        Some(_) => return ExposeItemKind::Malformed,
        None => (value, None),
    };
    let Some(port) = parse_port(port_value) else {
        return ExposeItemKind::Malformed;
    };
    match protocol {
        None => ExposeItemKind::Documented { port, protocol: None },
        Some("tcp") => ExposeItemKind::Documented {
            port,
            protocol: Some(ExposeProtocol::Tcp),
        },
        Some("udp") => ExposeItemKind::Documented {
            port,
            protocol: Some(ExposeProtocol::Udp),
        },
        Some("sctp") => ExposeItemKind::Sctp { port },
        Some(protocol) => ExposeItemKind::UnknownProtocol {
            port,
            protocol: protocol.to_owned(),
        },
    }
}

pub(crate) fn valid_generated_expose_item(value: &str) -> bool {
    matches!(
        classify_expose_item(value, ExposeScalarKind::String),
        ExposeItemKind::Documented { .. }
    ) && !value.contains(['$', '\r', '\n', '\0'])
}

fn parse_port(value: &str) -> Option<ExposePort> {
    let (start, end) = value
        .split_once('-')
        .map_or((value, None), |(start, end)| (start, Some(end)));
    if !decimal(start) || end.is_some_and(|end| !decimal(end)) {
        return None;
    }
    Some(ExposePort {
        start: start.to_owned(),
        end: end.map(str::to_owned),
    })
}

fn decimal(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{ExposeItemKind, ExposeProtocol, ExposeScalarKind, classify_expose_item, valid_generated_expose_item};

    #[test]
    fn classifies_without_fixed_width_or_default_protocol_normalization() {
        let huge = "18446744073709551616000000000000000000000000000000";
        assert!(matches!(
            classify_expose_item(huge, ExposeScalarKind::Number),
            ExposeItemKind::Documented { protocol: None, .. }
        ));
        assert!(matches!(
            classify_expose_item("080-090/tcp", ExposeScalarKind::String),
            ExposeItemKind::Documented {
                protocol: Some(ExposeProtocol::Tcp),
                ..
            }
        ));
        assert!(matches!(
            classify_expose_item("53/udp", ExposeScalarKind::String),
            ExposeItemKind::Documented {
                protocol: Some(ExposeProtocol::Udp),
                ..
            }
        ));
        assert!(matches!(
            classify_expose_item("80/sctp", ExposeScalarKind::String),
            ExposeItemKind::Sctp { .. }
        ));
        assert!(matches!(
            classify_expose_item("80/HTTP", ExposeScalarKind::String),
            ExposeItemKind::UnknownProtocol { protocol, .. } if protocol == "HTTP"
        ));
        assert_eq!(
            classify_expose_item("${PORT:-80}", ExposeScalarKind::String),
            ExposeItemKind::Expression
        );
        for value in ["", "80-", "-90", "80-90-100", "80/", "80/tcp/extra", "port"] {
            assert_eq!(
                classify_expose_item(value, ExposeScalarKind::String),
                ExposeItemKind::Malformed
            );
        }
    }

    #[test]
    fn generated_items_accept_only_resolved_documented_grammar() {
        for value in ["0", "80", "080-090", "53/udp", "80/tcp"] {
            assert!(valid_generated_expose_item(value));
        }
        for value in ["", "$PORT", "80/sctp", "80/HTTP", "80-", "80\n90"] {
            assert!(!valid_generated_expose_item(value));
        }
    }
}
