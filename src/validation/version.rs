//! Numeric implementation versions and evidence ranges.

use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// An exact three-component implementation version.
///
/// `ComposeLens` deliberately does not infer a current version or interpret pre-release/build
/// metadata. Callers select the exact released implementation they intend to assess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImplementationVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl ImplementationVersion {
    /// Creates an exact numeric version.
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Returns the major component.
    #[must_use]
    pub const fn major(self) -> u32 {
        self.major
    }

    /// Returns the minor component.
    #[must_use]
    pub const fn minor(self) -> u32 {
        self.minor
    }

    /// Returns the patch component.
    #[must_use]
    pub const fn patch(self) -> u32 {
        self.patch
    }
}

impl fmt::Display for ImplementationVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for ImplementationVersion {
    type Err = VersionParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.strip_prefix('v').unwrap_or(value);
        let mut components = value.split('.');
        let major = parse_component(components.next())?;
        let minor = parse_component(components.next())?;
        let patch = parse_component(components.next())?;
        if components.next().is_some() {
            return Err(VersionParseError);
        }
        Ok(Self::new(major, minor, patch))
    }
}

fn parse_component(component: Option<&str>) -> Result<u32, VersionParseError> {
    let component = component
        .filter(|component| !component.is_empty())
        .ok_or(VersionParseError)?;
    if !component.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(VersionParseError);
    }
    component.parse().map_err(|_| VersionParseError)
}

/// An exact implementation version could not be parsed.
///
/// The error does not retain or display the supplied value so version parsing can safely be used
/// with untrusted command input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionParseError;

impl fmt::Display for VersionParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("expected a three-component numeric implementation version")
    }
}

impl Error for VersionParseError {}

/// An inclusive implementation-version range attached to compatibility evidence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct VersionRange {
    minimum: Option<ImplementationVersion>,
    maximum: Option<ImplementationVersion>,
}

impl VersionRange {
    /// Creates an unbounded range.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            minimum: None,
            maximum: None,
        }
    }

    /// Creates a range with an inclusive minimum and no maximum.
    #[must_use]
    pub const fn from_minimum(minimum: ImplementationVersion) -> Self {
        Self {
            minimum: Some(minimum),
            maximum: None,
        }
    }

    /// Creates a range containing one exact version.
    #[must_use]
    pub const fn exact(version: ImplementationVersion) -> Self {
        Self {
            minimum: Some(version),
            maximum: Some(version),
        }
    }

    /// Creates a checked inclusive range.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidVersionRange`] when both bounds exist and the minimum is newer than the
    /// maximum.
    pub fn new(
        minimum: Option<ImplementationVersion>,
        maximum: Option<ImplementationVersion>,
    ) -> Result<Self, InvalidVersionRange> {
        if minimum.zip(maximum).is_some_and(|(minimum, maximum)| minimum > maximum) {
            return Err(InvalidVersionRange);
        }
        Ok(Self { minimum, maximum })
    }

    /// Returns the inclusive minimum.
    #[must_use]
    pub const fn minimum(self) -> Option<ImplementationVersion> {
        self.minimum
    }

    /// Returns the inclusive maximum.
    #[must_use]
    pub const fn maximum(self) -> Option<ImplementationVersion> {
        self.maximum
    }

    /// Reports whether an exact version is inside the inclusive range.
    #[must_use]
    pub fn contains(self, version: ImplementationVersion) -> bool {
        self.minimum.is_none_or(|minimum| version >= minimum) && self.maximum.is_none_or(|maximum| version <= maximum)
    }
}

/// A version range has its minimum after its maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidVersionRange;

impl fmt::Display for InvalidVersionRange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("minimum implementation version is newer than maximum")
    }
}

impl Error for InvalidVersionRange {}
