//! Compatibility-feature discovery and report generation.

use super::{CompatibilityClassification, CompatibilityFeature, CompatibilityProfile, CompatibilityRule};
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::merge::{MergeOperation, MergedEntry, MergedProject, MergedValue, MergedValueKind};
use crate::model::{Located, ShortVolumeMount};
use crate::profiles::ProfileSelection;
use crate::resolution::{effective_span, selection_matches, service_in_scope};
use crate::source::SourceSpan;

/// A construct is unsupported by the selected compatibility profile.
pub const UNSUPPORTED_FEATURE: DiagnosticCode = DiagnosticCode::new("compose.compatibility.unsupported");

/// A construct relies on implementation-specific behavior.
pub const IMPLEMENTATION_SPECIFIC_FEATURE: DiagnosticCode =
    DiagnosticCode::new("compose.compatibility.implementation-specific");

/// Compatibility evidence is insufficient for the selected versions.
pub const UNKNOWN_FEATURE_SUPPORT: DiagnosticCode = DiagnosticCode::new("compose.compatibility.unknown");

/// A construct is deprecated by the selected compatibility profile.
pub const DEPRECATED_FEATURE: DiagnosticCode = DiagnosticCode::new("compose.compatibility.deprecated");

/// One source occurrence of a compatibility-sensitive construct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityOccurrence {
    feature: CompatibilityFeature,
    path: Vec<String>,
    source: SourceSpan,
    sensitive: bool,
}

impl CompatibilityOccurrence {
    /// Returns the detected feature.
    #[must_use]
    pub const fn feature(&self) -> CompatibilityFeature {
        self.feature
    }

    /// Returns semantic mapping/sequence path segments without embedding the authored value.
    #[must_use]
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// Returns the source span identifying the construct.
    #[must_use]
    pub const fn source(&self) -> SourceSpan {
        self.source
    }

    /// Reports whether the source value includes sensitive interpolation output.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

/// One compatibility occurrence and the selected profile's decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityFinding {
    occurrence: CompatibilityOccurrence,
    rule: CompatibilityRule,
}

impl CompatibilityFinding {
    /// Returns the detected source occurrence.
    #[must_use]
    pub const fn occurrence(&self) -> &CompatibilityOccurrence {
        &self.occurrence
    }

    /// Returns the profile rule applied to that occurrence.
    #[must_use]
    pub const fn rule(&self) -> &CompatibilityRule {
        &self.rule
    }
}

/// A non-destructive compatibility assessment for one merged project view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityReport {
    profile: CompatibilityProfile,
    findings: Vec<CompatibilityFinding>,
    diagnostics: Vec<Diagnostic>,
}

impl CompatibilityReport {
    /// Returns the exact compatibility context selected by the caller.
    #[must_use]
    pub const fn profile(&self) -> CompatibilityProfile {
        self.profile
    }

    /// Returns all discovered constructs, including supported ones, in deterministic source order.
    #[must_use]
    pub fn findings(&self) -> &[CompatibilityFinding] {
        &self.findings
    }

    /// Returns compatibility diagnostics for non-portable, unknown, deprecated, or unsupported constructs.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether compatibility assessment emitted no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }
}

/// Detects and classifies compatibility-sensitive constructs in the selected project view.
#[must_use]
pub fn validate_compatibility(
    project: &MergedProject,
    selection: Option<&ProfileSelection>,
    profile: CompatibilityProfile,
) -> CompatibilityReport {
    let mut diagnostics = Vec::new();
    if !selection_matches(project, selection, &mut diagnostics) {
        return CompatibilityReport {
            profile,
            findings: Vec::new(),
            diagnostics,
        };
    }

    let occurrences = discover(project, selection);
    let findings = occurrences
        .into_iter()
        .map(|occurrence| {
            let rule = profile.classify(occurrence.feature);
            if let Some(severity) = rule.diagnostic_severity() {
                diagnostics.push(diagnostic(&occurrence, &rule, severity));
            }
            CompatibilityFinding { occurrence, rule }
        })
        .collect();
    CompatibilityReport {
        profile,
        findings,
        diagnostics,
    }
}

fn discover(project: &MergedProject, selection: Option<&ProfileSelection>) -> Vec<CompatibilityOccurrence> {
    let mut occurrences = Vec::new();
    let Some(root) = project.root().as_mapping() else {
        return occurrences;
    };
    let mut path = Vec::new();
    collect_operation(project.root(), &path, &mut occurrences);
    for entry in root {
        if entry.key() == "services" {
            collect_services(entry, selection, &mut path, &mut occurrences);
        } else {
            collect_entry(entry, &mut path, &mut occurrences);
        }
    }
    occurrences
}

fn collect_services(
    services: &MergedEntry,
    selection: Option<&ProfileSelection>,
    path: &mut Vec<String>,
    occurrences: &mut Vec<CompatibilityOccurrence>,
) {
    path.push("services".to_owned());
    collect_operation(services.value(), path, occurrences);
    if let Some(entries) = services.value().as_mapping() {
        for service in entries {
            if !service_in_scope(selection, service.key()) {
                continue;
            }
            path.push(service.key().to_owned());
            collect_service_features(service.value(), path, occurrences);
            collect_value(service.value(), path, occurrences);
            let _ = path.pop();
        }
    }
    let _ = path.pop();
}

fn collect_service_features(service: &MergedValue, path: &[String], occurrences: &mut Vec<CompatibilityOccurrence>) {
    if let Some(image) = service.get("image") {
        if let Some(scalar) = image.as_scalar() {
            if has_tag_and_digest(scalar.value()) {
                let mut image_path = path.to_owned();
                image_path.push("image".to_owned());
                occurrences.push(CompatibilityOccurrence {
                    feature: CompatibilityFeature::ImageTagAndDigest,
                    path: image_path,
                    source: effective_span(image),
                    sensitive: scalar.is_sensitive(),
                });
            }
        }
    }

    let Some(volumes) = service.get("volumes").and_then(MergedValue::as_sequence) else {
        return;
    };
    for (index, volume) in volumes.iter().enumerate() {
        let mut volume_path = path.to_owned();
        volume_path.push("volumes".to_owned());
        volume_path.push(index.to_string());
        if let Some(scalar) = volume.as_scalar() {
            let source = effective_span(volume);
            let mount = ShortVolumeMount::new(Located::new(scalar.value().to_owned(), source));
            if mount.selinux_relabel().is_some() {
                occurrences.push(CompatibilityOccurrence {
                    feature: CompatibilityFeature::ShortBindSelinuxRelabel,
                    path: volume_path,
                    source,
                    sensitive: scalar.is_sensitive(),
                });
            }
            continue;
        }
        let Some(selinux) = volume.get("bind").and_then(|bind| bind.get("selinux")) else {
            continue;
        };
        volume_path.push("bind".to_owned());
        volume_path.push("selinux".to_owned());
        occurrences.push(CompatibilityOccurrence {
            feature: CompatibilityFeature::LongBindSelinuxRelabel,
            path: volume_path,
            source: effective_span(selinux),
            sensitive: selinux.is_sensitive(),
        });
    }
}

fn collect_entry(entry: &MergedEntry, path: &mut Vec<String>, occurrences: &mut Vec<CompatibilityOccurrence>) {
    path.push(entry.key().to_owned());
    if entry.key().starts_with("x-") {
        occurrences.push(CompatibilityOccurrence {
            feature: CompatibilityFeature::ExtensionField,
            path: path.clone(),
            source: entry
                .key_sources()
                .last()
                .copied()
                .unwrap_or_else(|| effective_span(entry.value())),
            sensitive: entry.value().is_sensitive(),
        });
    }
    collect_value(entry.value(), path, occurrences);
    let _ = path.pop();
}

fn collect_value(value: &MergedValue, path: &mut Vec<String>, occurrences: &mut Vec<CompatibilityOccurrence>) {
    collect_operation(value, path, occurrences);
    match value.kind() {
        MergedValueKind::Mapping(entries) => {
            for entry in entries {
                collect_entry(entry, path, occurrences);
            }
        }
        MergedValueKind::Sequence(values) => {
            for (index, value) in values.iter().enumerate() {
                path.push(index.to_string());
                collect_value(value, path, occurrences);
                let _ = path.pop();
            }
        }
        MergedValueKind::Tagged { value, .. } => collect_value(value, path, occurrences),
        MergedValueKind::Null(_) | MergedValueKind::Scalar(_) | MergedValueKind::Alias(_) => {}
    }
}

fn collect_operation(value: &MergedValue, path: &[String], occurrences: &mut Vec<CompatibilityOccurrence>) {
    let feature = match value.provenance().operation() {
        MergeOperation::Reset => Some(CompatibilityFeature::ResetTag),
        MergeOperation::Override => Some(CompatibilityFeature::OverrideTag),
        MergeOperation::Authored
        | MergeOperation::Added
        | MergeOperation::Replaced
        | MergeOperation::Merged
        | MergeOperation::Appended => None,
    };
    if let Some(feature) = feature {
        occurrences.push(CompatibilityOccurrence {
            feature,
            path: path.to_vec(),
            source: effective_span(value),
            sensitive: value.is_sensitive(),
        });
    }
}

fn has_tag_and_digest(value: &str) -> bool {
    let Some((name_and_tag, _)) = value.split_once('@') else {
        return false;
    };
    let last_slash = name_and_tag.rfind('/');
    name_and_tag
        .rfind(':')
        .is_some_and(|separator| last_slash.is_none_or(|slash| separator > slash))
}

fn diagnostic(occurrence: &CompatibilityOccurrence, rule: &CompatibilityRule, severity: Severity) -> Diagnostic {
    let (code, message) = match rule.classification() {
        CompatibilityClassification::ImplementationSpecific => (
            IMPLEMENTATION_SPECIFIC_FEATURE,
            "construct has implementation-specific compatibility",
        ),
        CompatibilityClassification::Deprecated => (
            DEPRECATED_FEATURE,
            "construct is deprecated by the compatibility profile",
        ),
        CompatibilityClassification::Unsupported => (
            UNSUPPORTED_FEATURE,
            "construct is unsupported by the compatibility profile",
        ),
        CompatibilityClassification::Unknown => (
            UNKNOWN_FEATURE_SUPPORT,
            "construct has no established support for the compatibility profile",
        ),
        CompatibilityClassification::Supported | CompatibilityClassification::Extension => (
            UNKNOWN_FEATURE_SUPPORT,
            "construct has an unexpected compatibility diagnostic",
        ),
    };
    Diagnostic::new(code, severity, message)
        .with_label(DiagnosticLabel::primary(
            occurrence.source,
            "compatibility-sensitive construct",
        ))
        .with_note(rule.explanation())
}
