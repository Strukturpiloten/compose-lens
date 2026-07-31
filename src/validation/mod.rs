//! Version-aware implementation compatibility classification.

mod detect;
mod profile;
mod version;

pub use detect::{
    CompatibilityFinding, CompatibilityOccurrence, CompatibilityReport, DEPRECATED_FEATURE,
    IMPLEMENTATION_SPECIFIC_FEATURE, UNKNOWN_FEATURE_SUPPORT, UNSUPPORTED_FEATURE, validate_compatibility,
};
pub use profile::{
    CompatibilityClassification, CompatibilityEvidence, CompatibilityFeature, CompatibilityProfile, CompatibilityRule,
    ComposeProvider, ContainerRuntime, EvidenceKind,
};
pub use version::{ImplementationVersion, InvalidVersionRange, VersionParseError, VersionRange};
