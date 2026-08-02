//! Service port forms.

use super::{FieldReference, Located};
use crate::source::SourceSpan;

/// A short-syntax published port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortPort {
    raw: Located<String>,
    host_ip: Option<String>,
    published: Option<String>,
    target: String,
    protocol: Option<String>,
}

impl ShortPort {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let (without_protocol, protocol) = raw
            .value()
            .rsplit_once('/')
            .map_or((raw.value().as_str(), None), |(value, protocol)| {
                (value, Some(protocol.to_owned()))
            });
        let fields = split_port_fields(without_protocol);
        let (host_ip, published, target) = match fields.as_slice() {
            [] => (None, None, String::new()),
            [target] => (None, None, (*target).to_owned()),
            [published, target] => (None, Some((*published).to_owned()), (*target).to_owned()),
            [host @ .., published, target] => (
                Some(host.join(":")),
                Some((*published).to_owned()),
                (*target).to_owned(),
            ),
        };
        Self {
            raw,
            host_ip,
            published,
            target,
            protocol,
        }
    }

    /// Returns the complete semantic short string and source span.
    #[must_use]
    pub const fn raw(&self) -> &Located<String> {
        &self.raw
    }

    /// Returns the conservatively parsed host address.
    #[must_use]
    pub fn host_ip(&self) -> Option<&str> {
        self.host_ip.as_deref()
    }

    /// Returns the conservatively parsed published port or range.
    #[must_use]
    pub fn published(&self) -> Option<&str> {
        self.published.as_deref()
    }

    /// Returns the conservatively parsed container port or range.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Returns the explicitly authored protocol.
    #[must_use]
    pub fn protocol(&self) -> Option<&str> {
        self.protocol.as_deref()
    }
}

/// A long-syntax published port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongPort {
    span: SourceSpan,
    target: Option<Located<String>>,
    published: Option<Located<String>>,
    host_ip: Option<Located<String>>,
    protocol: Option<Located<String>>,
    app_protocol: Option<Located<String>>,
    mode: Option<Located<String>>,
    name: Option<Located<String>>,
    extension_fields: Vec<FieldReference>,
    unknown_fields: Vec<FieldReference>,
}

impl LongPort {
    pub(super) fn new(span: SourceSpan) -> Self {
        Self {
            span,
            target: None,
            published: None,
            host_ip: None,
            protocol: None,
            app_protocol: None,
            mode: None,
            name: None,
            extension_fields: Vec::new(),
            unknown_fields: Vec::new(),
        }
    }

    pub(super) fn set_target(&mut self, value: Located<String>) {
        self.target = Some(value);
    }
    pub(super) fn set_published(&mut self, value: Located<String>) {
        self.published = Some(value);
    }
    pub(super) fn set_host_ip(&mut self, value: Located<String>) {
        self.host_ip = Some(value);
    }
    pub(super) fn set_protocol(&mut self, value: Located<String>) {
        self.protocol = Some(value);
    }
    pub(super) fn set_app_protocol(&mut self, value: Located<String>) {
        self.app_protocol = Some(value);
    }
    pub(super) fn set_mode(&mut self, value: Located<String>) {
        self.mode = Some(value);
    }
    pub(super) fn set_name(&mut self, value: Located<String>) {
        self.name = Some(value);
    }
    pub(super) fn push_extension(&mut self, value: FieldReference) {
        self.extension_fields.push(value);
    }
    pub(super) fn push_unknown(&mut self, value: FieldReference) {
        self.unknown_fields.push(value);
    }

    /// Returns the complete mapping span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    /// Returns the container port or range.
    #[must_use]
    pub const fn target(&self) -> Option<&Located<String>> {
        self.target.as_ref()
    }
    /// Returns the published port or range.
    #[must_use]
    pub const fn published(&self) -> Option<&Located<String>> {
        self.published.as_ref()
    }
    /// Returns the host address.
    #[must_use]
    pub const fn host_ip(&self) -> Option<&Located<String>> {
        self.host_ip.as_ref()
    }
    /// Returns the protocol.
    #[must_use]
    pub const fn protocol(&self) -> Option<&Located<String>> {
        self.protocol.as_ref()
    }
    /// Returns the application protocol hint.
    #[must_use]
    pub const fn app_protocol(&self) -> Option<&Located<String>> {
        self.app_protocol.as_ref()
    }
    /// Returns the publication mode.
    #[must_use]
    pub const fn mode(&self) -> Option<&Located<String>> {
        self.mode.as_ref()
    }
    /// Returns the human-readable port name.
    #[must_use]
    pub const fn name(&self) -> Option<&Located<String>> {
        self.name.as_ref()
    }
    /// Returns retained `x-` fields.
    #[must_use]
    pub fn extension_fields(&self) -> &[FieldReference] {
        &self.extension_fields
    }
    /// Returns unrecognized fields.
    #[must_use]
    pub fn unknown_fields(&self) -> &[FieldReference] {
        &self.unknown_fields
    }
}

/// A service port with its authored short or long form retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Port {
    /// String or numeric scalar short syntax.
    Short(ShortPort),
    /// Mapping-based long syntax.
    Long(Box<LongPort>),
}

impl Port {
    /// Returns the complete port value span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Short(value) => value.raw().span(),
            Self::Long(value) => value.span(),
        }
    }
}

fn split_port_fields(value: &str) -> Vec<&str> {
    if let Some(close) = value.find(']').filter(|_| value.starts_with('[')) {
        let host = &value[..=close];
        let suffix = value[close + 1..].strip_prefix(':').unwrap_or(&value[close + 1..]);
        let mut fields = vec![host];
        fields.extend(suffix.split(':'));
        fields
    } else {
        value.split(':').collect()
    }
}

#[cfg(test)]
mod tests {
    use super::ShortPort;
    use crate::model::Located;
    use crate::source::{SourceId, SourceSpan};

    fn located(value: &str) -> Located<String> {
        Located::new(
            value.to_owned(),
            SourceSpan::from_valid_offsets(SourceId::new(1), 0, value.len()),
        )
    }

    #[test]
    fn splits_bracketed_ipv6_and_protocol() {
        let port = ShortPort::parse(located("[::1]:6001:6001/udp"));
        assert_eq!(port.host_ip(), Some("[::1]"));
        assert_eq!(port.published(), Some("6001"));
        assert_eq!(port.target(), "6001");
        assert_eq!(port.protocol(), Some("udp"));
    }
}
