//! Caller-authorized service-environment and secret-value resolution.

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLabel, Severity};
use crate::interpolation::{EnvironmentProvider, EnvironmentValue, InterpolationInput, interpolate};
use crate::merge::EntrySyntax;
use crate::model::{BooleanValue, ComposeScalar, EnvironmentFileFormatKind, SecretDefinition};
use crate::project::{ProjectEnvironmentFile, ProjectResource, ProjectService, ProjectValue, ProjectView};
use crate::source::{SourceId, SourceSpan};
use std::collections::BTreeMap;
use std::fmt;

/// A required `env_file` was not supplied by the caller-authorized provider.
pub const ENVIRONMENT_FILE_UNAVAILABLE: DiagnosticCode = DiagnosticCode::new("compose.environment.file-unavailable");
/// A supplied `env_file` contains an entry `ComposeLens` cannot interpret safely.
pub const ENVIRONMENT_FILE_INVALID_ENTRY: DiagnosticCode =
    DiagnosticCode::new("compose.environment.file-invalid-entry");
/// The caller-authorized environment-file provider denied a request.
pub const ENVIRONMENT_FILE_DENIED: DiagnosticCode = DiagnosticCode::new("compose.environment.file-denied");
/// A selected secret source could not be resolved by the caller-authorized provider.
pub const SECRET_VALUE_UNAVAILABLE: DiagnosticCode = DiagnosticCode::new("compose.secret.value-unavailable");
/// A secret definition does not identify one resolvable source.
pub const SECRET_SOURCE_UNRESOLVED: DiagnosticCode = DiagnosticCode::new("compose.secret.source-unresolved");
/// The caller-authorized secret provider denied a request.
pub const SECRET_VALUE_DENIED: DiagnosticCode = DiagnosticCode::new("compose.secret.value-denied");

/// Parser mode selected by one effective Compose `env_file` declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EnvironmentFileMode {
    /// Compose syntax including quote, escape, and interpolation handling.
    Compose,
    /// Compose `format: raw`; retain the right-hand side literally.
    Raw,
}

/// One explicit request made to a caller-owned environment-file provider.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EnvironmentFileRequest<'a> {
    path: &'a str,
    required: bool,
    mode: EnvironmentFileMode,
    source: SourceSpan,
    sensitive: bool,
}

impl fmt::Debug for EnvironmentFileRequest<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EnvironmentFileRequest")
            .field("path", &if self.sensitive { "<redacted>" } else { self.path })
            .field("required", &self.required)
            .field("mode", &self.mode)
            .field("source", &self.source)
            .field("sensitive", &self.sensitive)
            .finish()
    }
}

impl<'a> EnvironmentFileRequest<'a> {
    /// Returns the authored path without opening or normalizing it.
    #[must_use]
    pub const fn path(&self) -> &'a str {
        self.path
    }

    /// Reports whether Compose requires this file to exist.
    #[must_use]
    pub const fn required(&self) -> bool {
        self.required
    }

    /// Returns the selected parser mode.
    #[must_use]
    pub const fn mode(&self) -> EnvironmentFileMode {
        self.mode
    }

    /// Returns the declaration source span.
    #[must_use]
    pub const fn source(&self) -> SourceSpan {
        self.source
    }

    /// Reports whether the path itself came from sensitive interpolation.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

/// UTF-8 environment-file content supplied through an explicit authorization boundary.
#[derive(Clone, PartialEq, Eq)]
pub struct EnvironmentFileContent {
    text: String,
    sensitive: bool,
}

impl fmt::Debug for EnvironmentFileContent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EnvironmentFileContent")
            .field("text", &if self.sensitive { "<redacted>" } else { &self.text })
            .field("sensitive", &self.sensitive)
            .finish()
    }
}

impl EnvironmentFileContent {
    /// Creates non-sensitive content.
    #[must_use]
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            sensitive: false,
        }
    }

    /// Creates content whose derived values must remain redacted in debug output.
    #[must_use]
    pub fn sensitive(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            sensitive: true,
        }
    }

    /// Returns content after the caller explicitly crosses the sensitivity boundary.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.text
    }

    /// Reports whether derived values must be treated as sensitive.
    #[must_use]
    pub const fn is_sensitive(&self) -> bool {
        self.sensitive
    }
}

/// Bounded failure categories for a caller-owned environment-file provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EnvironmentFileLoadError {
    /// The caller did not authorize this request.
    Denied,
}

/// Supplies environment-file bytes without granting `ComposeLens` ambient filesystem access.
pub trait EnvironmentFileProvider {
    /// Returns `None` when the selected path is unavailable.
    ///
    /// # Errors
    ///
    /// Returns [`EnvironmentFileLoadError::Denied`] when the caller did not authorize the request.
    fn load(
        &self,
        request: &EnvironmentFileRequest<'_>,
    ) -> Result<Option<EnvironmentFileContent>, EnvironmentFileLoadError>;
}

/// One final effective service-environment value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResolvedEnvironmentValue {
    /// A concrete, possibly empty value.
    Value(EnvironmentValue),
    /// A key-only entry whose caller-authorized host lookup was unavailable.
    Unset,
}

/// Where one final effective environment entry came from.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResolvedEnvironmentOrigin {
    /// One caller-supplied environment file.
    File {
        /// Authored environment-file path.
        path: String,
        /// Path declaration source span.
        source: SourceSpan,
    },
    /// The service's explicit `environment` collection.
    Service {
        /// Mapping, list key/value, or list key-only syntax.
        syntax: EntrySyntax,
        /// Effective entry source span.
        source: SourceSpan,
    },
}

/// One effective service-environment entry, sorted by name in the result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEnvironmentEntry {
    name: String,
    value: ResolvedEnvironmentValue,
    origin: ResolvedEnvironmentOrigin,
}

impl ResolvedEnvironmentEntry {
    /// Returns the variable name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns concrete or explicitly unset state.
    #[must_use]
    pub const fn value(&self) -> &ResolvedEnvironmentValue {
        &self.value
    }

    /// Returns the final value's source category and span.
    #[must_use]
    pub const fn origin(&self) -> &ResolvedEnvironmentOrigin {
        &self.origin
    }
}

/// Result of one explicit service-environment resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceEnvironmentResolution {
    entries: Vec<ResolvedEnvironmentEntry>,
    diagnostics: Vec<Diagnostic>,
}

impl ServiceEnvironmentResolution {
    /// Returns final entries in deterministic key order.
    #[must_use]
    pub fn entries(&self) -> &[ResolvedEnvironmentEntry] {
        &self.entries
    }

    /// Returns file-loading, parsing, and interpolation diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether resolution emitted no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == Severity::Error)
    }
}

/// Resolves one service's environment only through caller-owned providers.
///
/// Environment files are applied in declaration order, then service `environment` entries
/// override them. The result is sorted by key. Key-only entries remain explicitly unset when
/// the supplied environment provider has no value.
#[must_use]
pub fn resolve_service_environment(
    service: &ProjectService,
    environment: &dyn EnvironmentProvider,
    files: &dyn EnvironmentFileProvider,
) -> ServiceEnvironmentResolution {
    let mut entries = BTreeMap::new();
    let mut diagnostics = Vec::new();

    if let Some(environment_files) = service.environment_files() {
        for file in environment_files.value() {
            resolve_environment_file(file, environment, files, &mut entries, &mut diagnostics);
        }
    }

    if let Some(service_environment) = service.environment() {
        for entry in service_environment.value().entries() {
            let source = entry
                .value()
                .effective_source()
                .or_else(|| entry.name().effective_source())
                .unwrap_or_else(empty_span);
            let value = match entry.value().value() {
                ComposeScalar::Null => environment
                    .get(entry.name().value())
                    .map_or(ResolvedEnvironmentValue::Unset, ResolvedEnvironmentValue::Value),
                scalar => {
                    ResolvedEnvironmentValue::Value(scalar_environment_value(scalar, entry.value().is_sensitive()))
                }
            };
            entries.insert(
                entry.name().value().to_owned(),
                ResolvedEnvironmentEntry {
                    name: entry.name().value().to_owned(),
                    value,
                    origin: ResolvedEnvironmentOrigin::Service {
                        syntax: entry.syntax(),
                        source,
                    },
                },
            );
        }
    }

    ServiceEnvironmentResolution {
        entries: entries.into_values().collect(),
        diagnostics,
    }
}

fn resolve_environment_file(
    file: &ProjectValue<ProjectEnvironmentFile>,
    environment: &dyn EnvironmentProvider,
    files: &dyn EnvironmentFileProvider,
    entries: &mut BTreeMap<String, ResolvedEnvironmentEntry>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (path, required, mode, source, sensitive) = match file.value() {
        ProjectEnvironmentFile::Short(path) => (
            path.as_str(),
            true,
            EnvironmentFileMode::Compose,
            file.effective_source().unwrap_or_else(empty_span),
            file.is_sensitive(),
        ),
        ProjectEnvironmentFile::Long(long) => {
            let Some(path) = long.path() else {
                return;
            };
            let required = match long.required().map(ProjectValue::value) {
                Some(BooleanValue::Literal(value)) => *value,
                _ => true,
            };
            let mode = match long.format().map(ProjectValue::value) {
                Some(format) if format.kind() == EnvironmentFileFormatKind::Raw => EnvironmentFileMode::Raw,
                _ => EnvironmentFileMode::Compose,
            };
            (
                path.value().as_str(),
                required,
                mode,
                path.effective_source().unwrap_or_else(empty_span),
                path.is_sensitive(),
            )
        }
    };
    let request = EnvironmentFileRequest {
        path,
        required,
        mode,
        source,
        sensitive,
    };
    let content = match files.load(&request) {
        Ok(Some(content)) => content,
        Ok(None) => {
            if required {
                diagnostics.push(diagnostic_at(
                    ENVIRONMENT_FILE_UNAVAILABLE,
                    Severity::Error,
                    "required environment file was not supplied by the caller-authorized provider",
                    source,
                    "required environment file declared here",
                ));
            }
            return;
        }
        Err(EnvironmentFileLoadError::Denied) => {
            diagnostics.push(diagnostic_at(
                ENVIRONMENT_FILE_DENIED,
                Severity::Error,
                "caller-owned environment-file provider denied the request",
                source,
                "environment file requested here",
            ));
            return;
        }
    };
    parse_environment_file(&request, &content, environment, entries, diagnostics);
}

fn parse_environment_file(
    request: &EnvironmentFileRequest<'_>,
    content: &EnvironmentFileContent,
    environment: &dyn EnvironmentProvider,
    entries: &mut BTreeMap<String, ResolvedEnvironmentEntry>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for raw_line in content.expose().lines() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (name, raw_value) = line
            .split_once('=')
            .map_or((trimmed, None), |(name, value)| (name.trim(), Some(value)));
        if !valid_environment_name(name) {
            diagnostics.push(diagnostic_at(
                ENVIRONMENT_FILE_INVALID_ENTRY,
                Severity::Error,
                "environment file contains an invalid variable name",
                request.source,
                "environment file declared here",
            ));
            continue;
        }
        let value = match raw_value {
            None => environment
                .get(name)
                .map_or(ResolvedEnvironmentValue::Unset, ResolvedEnvironmentValue::Value),
            Some(raw_value) => {
                let Ok(decoded) = decode_environment_file_value(raw_value, request.mode) else {
                    diagnostics.push(diagnostic_at(
                        ENVIRONMENT_FILE_INVALID_ENTRY,
                        Severity::Error,
                        "environment file contains an unterminated quoted value",
                        request.source,
                        "environment file declared here",
                    ));
                    continue;
                };
                let (value, interpolate_value) = decoded;
                let value = if interpolate_value {
                    let input = if content.is_sensitive() {
                        InterpolationInput::new(&value, request.source).sensitive()
                    } else {
                        InterpolationInput::new(&value, request.source)
                    };
                    let interpolation = interpolate(input, environment);
                    diagnostics.extend(interpolation.diagnostics().iter().cloned());
                    if interpolation.is_sensitive() {
                        EnvironmentValue::sensitive(interpolation.resolved())
                    } else {
                        EnvironmentValue::plain(interpolation.resolved())
                    }
                } else if content.is_sensitive() {
                    EnvironmentValue::sensitive(value)
                } else {
                    EnvironmentValue::plain(value)
                };
                ResolvedEnvironmentValue::Value(value)
            }
        };
        entries.insert(
            name.to_owned(),
            ResolvedEnvironmentEntry {
                name: name.to_owned(),
                value,
                origin: ResolvedEnvironmentOrigin::File {
                    path: request.path.to_owned(),
                    source: request.source,
                },
            },
        );
    }
}

fn decode_environment_file_value(raw: &str, mode: EnvironmentFileMode) -> Result<(String, bool), ()> {
    if mode == EnvironmentFileMode::Raw {
        return Ok((raw.to_owned(), false));
    }
    let value = raw.trim();
    if let Some(quoted) = value.strip_prefix('\'') {
        let Some(quoted) = quoted.strip_suffix('\'') else {
            return Err(());
        };
        return Ok((quoted.replace("\\'", "'"), false));
    }
    if let Some(quoted) = value.strip_prefix('"') {
        let Some(quoted) = quoted.strip_suffix('"') else {
            return Err(());
        };
        let mut decoded = String::with_capacity(quoted.len());
        let mut characters = quoted.chars();
        while let Some(character) = characters.next() {
            if character != '\\' {
                decoded.push(character);
                continue;
            }
            match characters.next() {
                Some('n') => decoded.push('\n'),
                Some('r') => decoded.push('\r'),
                Some('t') => decoded.push('\t'),
                Some('\\') | None => decoded.push('\\'),
                Some('"') => decoded.push('"'),
                Some(other) => {
                    decoded.push('\\');
                    decoded.push(other);
                }
            }
        }
        return Ok((decoded, true));
    }
    let value = value.find(" #").map_or(value, |comment| value[..comment].trim_end());
    Ok((value.to_owned(), true))
}

fn valid_environment_name(name: &str) -> bool {
    !name.is_empty()
        && !name
            .chars()
            .any(|character| character == '=' || character == '\0' || character.is_whitespace())
}

fn scalar_environment_value(value: &ComposeScalar, sensitive: bool) -> EnvironmentValue {
    let value = match value {
        ComposeScalar::Null => String::new(),
        ComposeScalar::Boolean(value) => value.to_string(),
        ComposeScalar::Number(value) | ComposeScalar::String(value) => value.clone(),
    };
    if sensitive {
        EnvironmentValue::sensitive(value)
    } else {
        EnvironmentValue::plain(value)
    }
}

/// One top-level Compose secret's selected native source.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SecretSource {
    /// Caller-owned file path.
    File(String),
    /// Caller-owned host environment-variable name.
    Environment(String),
    /// Platform-managed external secret name.
    External(String),
    /// Opaque provider driver name.
    Driver(String),
}

/// One explicit request to a caller-owned secret provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretRequest {
    name: String,
    source: SecretSource,
    source_span: SourceSpan,
}

impl SecretRequest {
    /// Returns the Compose secret name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the source `ComposeLens` selected without reading it.
    #[must_use]
    pub const fn source(&self) -> &SecretSource {
        &self.source
    }

    /// Returns the source declaration span.
    #[must_use]
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
}

/// Secret payload exposed only through an explicit accessor and always redacted in `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretValue(String);

impl fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretValue(<redacted>)")
    }
}

impl SecretValue {
    /// Wraps a caller-authorized secret payload.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the payload after the caller explicitly crosses the sensitivity boundary.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// Bounded failure categories for a caller-owned secret provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SecretResolveError {
    /// The caller did not authorize this request.
    Denied,
}

/// Resolves selected secret sources without granting `ComposeLens` ambient access.
pub trait SecretProvider {
    /// Returns `None` when the provider cannot resolve the selected source.
    ///
    /// # Errors
    ///
    /// Returns [`SecretResolveError::Denied`] when the caller did not authorize the request.
    fn resolve(&self, request: &SecretRequest) -> Result<Option<SecretValue>, SecretResolveError>;
}

/// One caller-authorized resolved top-level secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSecret {
    request: SecretRequest,
    value: SecretValue,
}

impl ResolvedSecret {
    /// Returns the source request retained as provenance.
    #[must_use]
    pub const fn request(&self) -> &SecretRequest {
        &self.request
    }

    /// Returns the protected payload wrapper.
    #[must_use]
    pub const fn value(&self) -> &SecretValue {
        &self.value
    }
}

/// Result of one explicit top-level secret-resolution operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretResolution {
    secrets: Vec<ResolvedSecret>,
    diagnostics: Vec<Diagnostic>,
}

impl SecretResolution {
    /// Returns resolved secrets in deterministic Compose-name order.
    #[must_use]
    pub fn secrets(&self) -> &[ResolvedSecret] {
        &self.secrets
    }

    /// Returns unavailable, ambiguous, or denied-source diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Reports whether resolution emitted no error diagnostics.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == Severity::Error)
    }
}

/// Resolves top-level secret definitions only through a caller-owned provider.
#[must_use]
pub fn resolve_project_secrets(project: &ProjectView, provider: &dyn SecretProvider) -> SecretResolution {
    let mut resources: Vec<_> = project.secrets().iter().collect();
    resources.sort_by(|left, right| left.name().value().cmp(right.name().value()));
    let mut secrets = Vec::new();
    let mut diagnostics = Vec::new();
    for resource in resources {
        let Some(request) = secret_request(resource) else {
            let span = resource.definition().effective_source().unwrap_or_else(empty_span);
            diagnostics.push(diagnostic_at(
                SECRET_SOURCE_UNRESOLVED,
                Severity::Error,
                "secret definition must select exactly one caller-resolvable source",
                span,
                "secret source cannot be selected here",
            ));
            continue;
        };
        match provider.resolve(&request) {
            Ok(Some(value)) => secrets.push(ResolvedSecret { request, value }),
            Ok(None) => diagnostics.push(diagnostic_at(
                SECRET_VALUE_UNAVAILABLE,
                Severity::Error,
                "secret payload was not supplied by the caller-authorized provider",
                request.source_span,
                "secret source declared here",
            )),
            Err(SecretResolveError::Denied) => diagnostics.push(diagnostic_at(
                SECRET_VALUE_DENIED,
                Severity::Error,
                "caller-owned secret provider denied the request",
                request.source_span,
                "secret source requested here",
            )),
        }
    }
    SecretResolution { secrets, diagnostics }
}

fn secret_request(resource: &ProjectResource<SecretDefinition>) -> Option<SecretRequest> {
    let definition = resource.definition().value();
    let mut sources = Vec::new();
    if let Some(file) = definition.file() {
        sources.push((SecretSource::File(file.value().clone()), file.span()));
    }
    if let Some(environment) = definition.environment() {
        sources.push((
            SecretSource::Environment(environment.value().clone()),
            environment.span(),
        ));
    }
    if let Some(driver) = definition.driver() {
        sources.push((SecretSource::Driver(driver.value().clone()), driver.span()));
    }
    if let Some(external) = definition
        .external()
        .filter(|external| external.is_explicitly_external())
    {
        let (name, span) = definition
            .custom_name()
            .map(|name| (name.value().clone(), name.span()))
            .or_else(|| {
                external
                    .name_mapping()
                    .and_then(|mapping| mapping.name().map(|name| (name.value().clone(), name.span())))
            })
            .unwrap_or_else(|| {
                (
                    resource.name().value().to_owned(),
                    external.name_mapping().map_or_else(
                        || resource.definition().effective_source().unwrap_or_else(empty_span),
                        crate::model::ExternalNameMapping::span,
                    ),
                )
            });
        sources.push((SecretSource::External(name), span));
    }
    if sources.len() != 1 {
        return None;
    }
    let (source, source_span) = sources.pop()?;
    Some(SecretRequest {
        name: resource.name().value().to_owned(),
        source,
        source_span,
    })
}

fn diagnostic_at(
    code: DiagnosticCode,
    severity: Severity,
    message: &'static str,
    source: SourceSpan,
    label: &'static str,
) -> Diagnostic {
    Diagnostic::new(code, severity, message).with_label(DiagnosticLabel::primary(source, label))
}

fn empty_span() -> SourceSpan {
    SourceSpan::from_valid_offsets(SourceId::new(0), 0, 0)
}
