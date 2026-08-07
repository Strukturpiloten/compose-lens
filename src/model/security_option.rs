//! Raw-preserving service security options.

use crate::source::SourceSpan;

use super::Located;

/// The narrow native classification of one service `security_opt` string.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SecurityOptionKind {
    /// An exact lowercase `apparmor=<profile>` candidate with no whitespace alteration.
    AppArmor {
        /// The exact non-empty profile spelling after `apparmor=`.
        profile: String,
    },
    /// An exact lowercase `seccomp=<profile>` candidate with no whitespace alteration.
    Seccomp {
        /// The exact non-empty profile spelling after `seccomp=`.
        profile: String,
    },
    /// An exact lowercase whitespace-free `no-new-privileges:<bool>` candidate.
    NoNewPrivileges {
        /// Whether no-new-privileges is explicitly enabled or disabled.
        enabled: bool,
    },
    /// An exact lowercase whitespace-free `label:disable` candidate.
    SecurityLabelDisable {
        /// Whether `SELinux` label separation is explicitly disabled.
        enabled: bool,
    },
    /// An exact lowercase whitespace-free `label:filetype:<type>` candidate.
    SecurityLabelFileType {
        /// The exact non-empty file-type spelling after `label:filetype:`.
        file_type: String,
    },
    /// An exact lowercase whitespace-free `label:level:<level>` candidate.
    SecurityLabelLevel {
        /// The exact non-empty level spelling after `label:level:`.
        level: String,
    },
    /// An exact lowercase whitespace-free `label:nested` candidate.
    SecurityLabelNested {
        /// Whether nested `SELinux` labeling is explicitly requested.
        enabled: bool,
    },
    /// An exact lowercase whitespace-free `label:type:<type>` candidate.
    SecurityLabelType {
        /// The exact non-empty type spelling after `label:type:`.
        label_type: String,
    },
    /// An exact lowercase whitespace-free `mask=<paths>` candidate.
    Mask {
        /// The exact non-empty path payload after `mask=`, including colon separators.
        paths: String,
    },
    /// An exact lowercase whitespace-free `unmask=<paths>` candidate.
    Unmask {
        /// The exact payload after `unmask=`, either `ALL` or colon-separated absolute-looking paths.
        paths: String,
    },
    /// A value whose effective spelling depends on Compose interpolation.
    Expression,
    /// An explicitly authored empty string.
    Empty,
    /// An AppArmor-shaped spelling that is not the exact narrow candidate form.
    AppArmorNearMiss,
    /// A seccomp-shaped spelling that is not the exact narrow candidate form.
    SeccompNearMiss,
    /// A no-new-privileges-shaped spelling that is not an exact narrow candidate.
    NoNewPrivilegesNearMiss,
    /// A label-disable-shaped spelling that is not the exact narrow candidate.
    SecurityLabelDisableNearMiss,
    /// A label-filetype-shaped spelling that is not the exact narrow candidate.
    SecurityLabelFileTypeNearMiss,
    /// A label-level-shaped spelling that is not the exact narrow candidate.
    SecurityLabelLevelNearMiss,
    /// A label-nested-shaped spelling that is not the exact narrow candidate.
    SecurityLabelNestedNearMiss,
    /// A label-type-shaped spelling that is not the exact narrow candidate.
    SecurityLabelTypeNearMiss,
    /// A mask-shaped spelling that is not the exact narrow candidate.
    MaskNearMiss,
    /// An unmask-shaped spelling that is not the exact narrow candidate.
    UnmaskNearMiss,
    /// Another raw provider option whose grammar is deliberately uninterpreted.
    Other,
}

/// One exact source-aware service `security_opt` item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityOptionItem {
    raw: Located<String>,
    kind: SecurityOptionKind,
}

impl SecurityOptionItem {
    pub(crate) fn parse(raw: Located<String>) -> Self {
        let kind = classify_security_option(raw.value());
        Self { raw, kind }
    }

    /// Returns the exact semantic string spelling without separator or case normalization.
    #[must_use]
    pub fn value(&self) -> &str {
        self.raw.value()
    }

    /// Returns the exact source span of this sequence item.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.raw.span()
    }

    /// Returns the conservative option classification.
    #[must_use]
    pub const fn kind(&self) -> &SecurityOptionKind {
        &self.kind
    }
}

/// An explicitly authored ordered service `security_opt` sequence.
///
/// Omission is represented by absence of this value. An empty item vector therefore retains an
/// explicitly authored empty sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityOptions {
    span: SourceSpan,
    items: Vec<SecurityOptionItem>,
}

#[derive(Debug, Default)]
pub(crate) struct SecurityOptionCandidateCounts {
    pub(crate) apparmor: usize,
    pub(crate) seccomp: usize,
    pub(crate) no_new_privileges: usize,
    pub(crate) security_label_disable: usize,
    pub(crate) security_label_filetype: usize,
    pub(crate) security_label_level: usize,
    pub(crate) security_label_nested: usize,
    pub(crate) security_label_type: usize,
}

impl SecurityOptions {
    pub(crate) const fn new(span: SourceSpan, items: Vec<SecurityOptionItem>) -> Self {
        Self { span, items }
    }

    /// Returns the span of the complete authored sequence.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns every valid string item in authored order, including duplicates and empty strings.
    #[must_use]
    pub fn items(&self) -> &[SecurityOptionItem] {
        &self.items
    }
}

pub(crate) fn classify_security_option(value: &str) -> SecurityOptionKind {
    if value.contains('$') {
        return SecurityOptionKind::Expression;
    }
    if value.is_empty() {
        return SecurityOptionKind::Empty;
    }
    if let Some(profile) = value.strip_prefix("apparmor=") {
        if !profile.is_empty() && !profile.chars().any(char::is_whitespace) {
            return SecurityOptionKind::AppArmor {
                profile: profile.to_owned(),
            };
        }
        return SecurityOptionKind::AppArmorNearMiss;
    }
    if value
        .trim()
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("apparmor"))
    {
        return SecurityOptionKind::AppArmorNearMiss;
    }
    if let Some(profile) = value.strip_prefix("seccomp=") {
        if !profile.is_empty() && !profile.chars().any(char::is_whitespace) {
            return SecurityOptionKind::Seccomp {
                profile: profile.to_owned(),
            };
        }
        return SecurityOptionKind::SeccompNearMiss;
    }
    let compact = value
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    if let (Some(prefix), Some(suffix)) = (compact.get(..7), compact.get(7..)) {
        if prefix.eq_ignore_ascii_case("seccomp")
            && (suffix.is_empty() || suffix.starts_with(':') || suffix.starts_with('='))
        {
            return SecurityOptionKind::SeccompNearMiss;
        }
    }
    if let Some(kind) = classify_exact_boolean_option(value) {
        return kind;
    }
    if let (Some(prefix), Some(suffix)) = (compact.get(..17), compact.get(17..)) {
        if prefix.eq_ignore_ascii_case("no-new-privileges")
            && (suffix.is_empty() || suffix.starts_with(':') || suffix.starts_with('='))
        {
            return SecurityOptionKind::NoNewPrivilegesNearMiss;
        }
    }
    if let Some(paths) = value.strip_prefix("mask=") {
        return if !paths.is_empty() && !paths.chars().any(char::is_whitespace) {
            SecurityOptionKind::Mask {
                paths: paths.to_owned(),
            }
        } else {
            SecurityOptionKind::MaskNearMiss
        };
    }
    if is_mask_near_miss(value, &compact) {
        return SecurityOptionKind::MaskNearMiss;
    }
    if let Some(paths) = value.strip_prefix("unmask=") {
        return if valid_unmask_payload(paths) && !value.chars().any(char::is_whitespace) {
            SecurityOptionKind::Unmask {
                paths: paths.to_owned(),
            }
        } else {
            SecurityOptionKind::UnmaskNearMiss
        };
    }
    if is_unmask_near_miss(value, &compact) {
        return SecurityOptionKind::UnmaskNearMiss;
    }
    classify_security_label_option(value, &compact).unwrap_or(SecurityOptionKind::Other)
}

fn valid_unmask_payload(paths: &str) -> bool {
    paths == "ALL"
        || paths
            .split(':')
            .all(|segment| !segment.is_empty() && segment.starts_with('/'))
}

fn is_mask_near_miss(value: &str, compact: &str) -> bool {
    let Some(prefix) = compact.get(..4) else {
        return false;
    };
    if !prefix.eq_ignore_ascii_case("mask") {
        return false;
    }
    let suffix = &compact[4..];
    suffix.is_empty()
        || suffix.starts_with(['=', ':'])
        || suffix
            .chars()
            .next()
            .is_some_and(|delimiter| !delimiter.is_ascii_alphanumeric() && delimiter != '_')
        || value
            .trim_start()
            .get(4..)
            .and_then(|suffix| suffix.chars().next())
            .is_some_and(char::is_whitespace)
}

fn is_unmask_near_miss(value: &str, compact: &str) -> bool {
    let Some(prefix) = compact.get(..6) else {
        return false;
    };
    if !prefix.eq_ignore_ascii_case("unmask") {
        return false;
    }
    let suffix = &compact[6..];
    suffix.is_empty()
        || suffix.starts_with(['=', ':'])
        || suffix
            .chars()
            .next()
            .is_some_and(|delimiter| !delimiter.is_ascii_alphanumeric() && delimiter != '_')
        || value
            .trim_start()
            .get(6..)
            .and_then(|suffix| suffix.chars().next())
            .is_some_and(char::is_whitespace)
}

fn classify_security_label_option(value: &str, compact: &str) -> Option<SecurityOptionKind> {
    if let Some(file_type) = value.strip_prefix("label:filetype:") {
        return Some(if !file_type.is_empty() && !value.chars().any(char::is_whitespace) {
            SecurityOptionKind::SecurityLabelFileType {
                file_type: file_type.to_owned(),
            }
        } else {
            SecurityOptionKind::SecurityLabelFileTypeNearMiss
        });
    }
    if let Some(level) = value.strip_prefix("label:level:") {
        return Some(if !level.is_empty() && !value.chars().any(char::is_whitespace) {
            SecurityOptionKind::SecurityLabelLevel {
                level: level.to_owned(),
            }
        } else {
            SecurityOptionKind::SecurityLabelLevelNearMiss
        });
    }
    if let Some(label_type) = value.strip_prefix("label:type:") {
        return Some(
            if !label_type.is_empty() && !value.chars().any(char::is_whitespace) && !label_type.contains([':', '=']) {
                SecurityOptionKind::SecurityLabelType {
                    label_type: label_type.to_owned(),
                }
            } else {
                SecurityOptionKind::SecurityLabelTypeNearMiss
            },
        );
    }
    if compact.eq_ignore_ascii_case("label")
        || compact.eq_ignore_ascii_case("label=disable")
        || compact.eq_ignore_ascii_case("label:disable")
        || compact
            .get(..14)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("label:disable:"))
    {
        return Some(SecurityOptionKind::SecurityLabelDisableNearMiss);
    }
    let lowercase_compact = compact.to_ascii_lowercase();
    if lowercase_compact == "label:filetype"
        || lowercase_compact.starts_with("label:filetype:")
        || lowercase_compact.starts_with("label:filetype=")
        || lowercase_compact.starts_with("label=filetype:")
        || lowercase_compact.starts_with("label=filetype=")
    {
        return Some(SecurityOptionKind::SecurityLabelFileTypeNearMiss);
    }
    if lowercase_compact == "label:level"
        || lowercase_compact == "label=level"
        || lowercase_compact.starts_with("label:level:")
        || lowercase_compact.starts_with("label:level=")
        || lowercase_compact.starts_with("label=level:")
        || lowercase_compact.starts_with("label=level=")
    {
        return Some(SecurityOptionKind::SecurityLabelLevelNearMiss);
    }
    if is_security_label_nested_near_miss(value, &lowercase_compact) {
        return Some(SecurityOptionKind::SecurityLabelNestedNearMiss);
    }
    if is_security_label_type_near_miss(&lowercase_compact) {
        return Some(SecurityOptionKind::SecurityLabelTypeNearMiss);
    }
    None
}

fn classify_exact_boolean_option(value: &str) -> Option<SecurityOptionKind> {
    match value {
        "no-new-privileges:true" => Some(SecurityOptionKind::NoNewPrivileges { enabled: true }),
        "no-new-privileges:false" => Some(SecurityOptionKind::NoNewPrivileges { enabled: false }),
        "label:disable" => Some(SecurityOptionKind::SecurityLabelDisable { enabled: true }),
        "label:nested" => Some(SecurityOptionKind::SecurityLabelNested { enabled: true }),
        _ => None,
    }
}

fn is_security_label_nested_near_miss(value: &str, lowercase_compact: &str) -> bool {
    lowercase_compact == "nested"
        || lowercase_compact == "label=nested"
        || lowercase_compact.starts_with("label:nested:")
        || lowercase_compact.starts_with("label:nested=")
        || lowercase_compact.starts_with("label=nested:")
        || lowercase_compact.starts_with("label=nested=")
        || (lowercase_compact == "label:nested" && value != "label:nested")
}

fn is_security_label_type_near_miss(lowercase_compact: &str) -> bool {
    lowercase_compact == "type"
        || lowercase_compact == "label:type"
        || lowercase_compact == "label=type"
        || lowercase_compact.starts_with("label:type:")
        || lowercase_compact.starts_with("label:type=")
        || lowercase_compact.starts_with("label=type:")
        || lowercase_compact.starts_with("label=type=")
}

#[cfg(test)]
mod tests {
    use super::{SecurityOptionKind, classify_security_option};

    #[test]
    fn classifies_only_exact_lowercase_whitespace_free_security_option_candidates() {
        assert!(matches!(
            classify_security_option("apparmor=profile-a"),
            SecurityOptionKind::AppArmor { profile } if profile == "profile-a"
        ));
        assert_eq!(
            classify_security_option("${SECURITY_OPT}"),
            SecurityOptionKind::Expression
        );
        assert_eq!(classify_security_option(""), SecurityOptionKind::Empty);
        for (value, expected) in [
            ("seccomp=unconfined", "unconfined"),
            ("seccomp=/workspace/seccomp.json", "/workspace/seccomp.json"),
        ] {
            assert!(matches!(
                classify_security_option(value),
                SecurityOptionKind::Seccomp { profile } if profile == expected
            ));
        }
        assert_eq!(
            classify_security_option("seccomp=${SECCOMP_PROFILE}"),
            SecurityOptionKind::Expression
        );
        for value in [
            "seccomp",
            "seccomp:",
            "seccomp:unconfined",
            "seccomp=",
            "Seccomp=unconfined",
            " seccomp=unconfined",
            "seccomp =unconfined",
            "seccomp=profile name",
        ] {
            assert_eq!(classify_security_option(value), SecurityOptionKind::SeccompNearMiss);
        }
        assert_eq!(
            classify_security_option("seccomp-extra=true"),
            SecurityOptionKind::Other
        );
        for value in [
            "apparmor=",
            "apparmor=profile name",
            "AppArmor=profile-a",
            "apparmor:profile-a",
            " apparmor=profile-a",
        ] {
            assert_eq!(classify_security_option(value), SecurityOptionKind::AppArmorNearMiss);
        }
        assert_eq!(
            classify_security_option("no-new-privileges:true"),
            SecurityOptionKind::NoNewPrivileges { enabled: true }
        );
        assert_eq!(
            classify_security_option("no-new-privileges:false"),
            SecurityOptionKind::NoNewPrivileges { enabled: false }
        );
        for value in [
            "no-new-privileges",
            "no-new-privileges=true",
            "No-New-Privileges:true",
            " no-new-privileges:true",
            "no-new-privileges: true",
        ] {
            assert_eq!(
                classify_security_option(value),
                SecurityOptionKind::NoNewPrivilegesNearMiss
            );
        }
        assert_eq!(
            classify_security_option("no-new-privileges-extra:true"),
            SecurityOptionKind::Other
        );
        assert_eq!(
            classify_security_option("label:disable"),
            SecurityOptionKind::SecurityLabelDisable { enabled: true }
        );
        for value in [
            "label=disable",
            "label:disable:false",
            "Label:disable",
            " label:disable",
            "label : disable",
            "label",
        ] {
            assert_eq!(
                classify_security_option(value),
                SecurityOptionKind::SecurityLabelDisableNearMiss
            );
        }
        for value in ["label:user:USER", "label:role:ROLE"] {
            assert_eq!(classify_security_option(value), SecurityOptionKind::Other);
        }
        assert!(matches!(
            classify_security_option("label:type:TYPE"),
            SecurityOptionKind::SecurityLabelType { label_type } if label_type == "TYPE"
        ));
        assert_eq!(
            classify_security_option("label:${LABEL_MODE}"),
            SecurityOptionKind::Expression
        );
    }

    #[test]
    fn classifies_repeatable_mask_candidates_without_interpreting_payload_paths() {
        for (value, expected) in [
            ("mask=/run/secrets", "/run/secrets"),
            ("mask=/proc/acpi:/proc/kcore", "/proc/acpi:/proc/kcore"),
            ("mask=relative:opaque=value", "relative:opaque=value"),
        ] {
            assert!(matches!(
                classify_security_option(value),
                SecurityOptionKind::Mask { paths } if paths == expected
            ));
        }
        for value in [
            "mask",
            "mask=",
            "mask:/run/secrets",
            "Mask=/run/secrets",
            "MASK=/run/secrets",
            " mask=/run/secrets",
            "mask =/run/secrets",
            "mask=/run/secret path",
            "mask-/run/secrets",
        ] {
            assert_eq!(classify_security_option(value), SecurityOptionKind::MaskNearMiss);
        }
        assert_eq!(classify_security_option("masking=true"), SecurityOptionKind::Other);
        assert_eq!(classify_security_option("masking true"), SecurityOptionKind::Other);
        assert_eq!(
            classify_security_option("mask=${MASK_PATHS}"),
            SecurityOptionKind::Expression
        );
    }

    #[test]
    fn classifies_only_exact_repeatable_unmask_candidates() {
        for (value, expected) in [
            ("unmask=ALL", "ALL"),
            ("unmask=/proc/acpi", "/proc/acpi"),
            ("unmask=/proc/acpi:/sys/firmware", "/proc/acpi:/sys/firmware"),
            ("unmask=/proc/*", "/proc/*"),
        ] {
            assert!(matches!(
                classify_security_option(value),
                SecurityOptionKind::Unmask { paths } if paths == expected
            ));
        }
        for value in [
            "unmask",
            "unmask=",
            "unmask=all",
            "Unmask=ALL",
            "UNMASK=/proc/acpi",
            "unmask:/proc/acpi",
            " unmask=/proc/acpi",
            "unmask=/proc/acpi ",
            "unmask =/proc/acpi",
            "unmask=proc/acpi",
            "unmask=/proc/acpi:relative",
            "unmask=/proc/acpi:",
            "unmask=:/proc/acpi",
            "unmask=/proc/acpi::/sys/firmware",
            "unmask=ALL:/proc/acpi",
            "unmask=/proc/acpi:ALL",
            "unmask-/proc/acpi",
        ] {
            assert_eq!(classify_security_option(value), SecurityOptionKind::UnmaskNearMiss);
        }
        assert_eq!(classify_security_option("unmasking=true"), SecurityOptionKind::Other);
        assert_eq!(
            classify_security_option("unmask=${UNMASK_PATHS}"),
            SecurityOptionKind::Expression
        );
    }

    #[test]
    fn classifies_only_exact_label_filetype_candidates_and_precise_near_misses() {
        assert!(matches!(
            classify_security_option("label:filetype:container_file_t"),
            SecurityOptionKind::SecurityLabelFileType { file_type }
                if file_type == "container_file_t"
        ));
        for value in [
            "label=filetype:container_file_t",
            "label:filetype=container_file_t",
            "Label:filetype:container_file_t",
            "label:FileType:container_file_t",
            " label:filetype:container_file_t",
            "label:filetype:container file t",
            "label:filetype:",
            "label:filetype",
        ] {
            assert_eq!(
                classify_security_option(value),
                SecurityOptionKind::SecurityLabelFileTypeNearMiss
            );
        }
        for value in ["label:user:USER", "label:role:ROLE"] {
            assert_eq!(classify_security_option(value), SecurityOptionKind::Other);
        }
        assert!(matches!(
            classify_security_option("label:type:TYPE"),
            SecurityOptionKind::SecurityLabelType { label_type } if label_type == "TYPE"
        ));
        assert!(matches!(
            classify_security_option("label:level:LEVEL"),
            SecurityOptionKind::SecurityLabelLevel { level } if level == "LEVEL"
        ));
        assert_eq!(
            classify_security_option("label:filetype:${LABEL_TYPE}"),
            SecurityOptionKind::Expression
        );
    }

    #[test]
    fn classifies_only_exact_label_level_candidates_and_precise_near_misses() {
        assert!(matches!(
            classify_security_option("label:level:s0:c1,c2"),
            SecurityOptionKind::SecurityLabelLevel { level }
                if level == "s0:c1,c2"
        ));
        for value in [
            "label=level:s0:c1,c2",
            "label:level=s0:c1,c2",
            "Label:level:s0:c1,c2",
            "label:Level:s0:c1,c2",
            " label:level:s0:c1,c2",
            "label:level:s0 c1",
            "label:level:",
            "label:level",
            "label=level",
        ] {
            assert_eq!(
                classify_security_option(value),
                SecurityOptionKind::SecurityLabelLevelNearMiss
            );
        }
        for value in [
            "label:type:TYPE",
            "label:user:USER",
            "label:role:ROLE",
            "label:filetype:container_file_t",
            "label:disable",
        ] {
            assert!(!matches!(
                classify_security_option(value),
                SecurityOptionKind::SecurityLabelLevel { .. } | SecurityOptionKind::SecurityLabelLevelNearMiss
            ));
        }
        assert_eq!(
            classify_security_option("label:level:${LABEL_LEVEL}"),
            SecurityOptionKind::Expression
        );
    }

    #[test]
    fn classifies_only_exact_label_nested_candidates_and_precise_near_misses() {
        assert_eq!(
            classify_security_option("label:nested"),
            SecurityOptionKind::SecurityLabelNested { enabled: true }
        );
        for value in [
            "label=nested",
            "Label:nested",
            "label:Nested",
            " label:nested",
            "label : nested",
            "label:nested:true",
            "label:nested=",
            "nested",
        ] {
            assert_eq!(
                classify_security_option(value),
                SecurityOptionKind::SecurityLabelNestedNearMiss
            );
        }
        for value in [
            "label:disable",
            "label:filetype:container_file_t",
            "label:level:s0:c1,c2",
            "label:type:TYPE",
            "label:user:USER",
            "label:role:ROLE",
        ] {
            assert!(!matches!(
                classify_security_option(value),
                SecurityOptionKind::SecurityLabelNested { .. } | SecurityOptionKind::SecurityLabelNestedNearMiss
            ));
        }
        assert_eq!(
            classify_security_option("${LABEL_NESTED_OPTION}"),
            SecurityOptionKind::Expression
        );
    }

    #[test]
    fn classifies_only_exact_label_type_candidates_and_precise_near_misses() {
        assert!(matches!(
            classify_security_option("label:type:container_t"),
            SecurityOptionKind::SecurityLabelType { label_type }
                if label_type == "container_t"
        ));
        assert!(matches!(
            classify_security_option("label:type:TYPE"),
            SecurityOptionKind::SecurityLabelType { label_type } if label_type == "TYPE"
        ));
        for value in [
            "label=type:container_t",
            "label:type=container_t",
            "label=type=container_t",
            "Label:type:container_t",
            "label:Type:container_t",
            " label:type:container_t",
            "label : type : container_t",
            "label:type:container t",
            "label:type:",
            "label:type",
            "label=type",
            "type",
            "label:type:container_t:extended",
            "label:type:container_t=extended",
        ] {
            assert_eq!(
                classify_security_option(value),
                SecurityOptionKind::SecurityLabelTypeNearMiss
            );
        }
        for value in [
            "label:disable",
            "label:filetype:container_file_t",
            "label:level:s0:c1,c2",
            "label:nested",
            "label:user:USER",
            "label:role:ROLE",
        ] {
            assert!(!matches!(
                classify_security_option(value),
                SecurityOptionKind::SecurityLabelType { .. } | SecurityOptionKind::SecurityLabelTypeNearMiss
            ));
        }
        assert_eq!(
            classify_security_option("label:type:${LABEL_TYPE}"),
            SecurityOptionKind::Expression
        );
    }
}
