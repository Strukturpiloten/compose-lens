//! Caller-authorized service-environment and secret-resolution contracts.

use compose_lens::interpolation::{EnvironmentValue, InterpolationInput, MapEnvironment, interpolate};
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject};
use compose_lens::merge::merge_project;
use compose_lens::project::{ProjectView, ProjectViewResult, build_project_view};
use compose_lens::resolution::{
    ENVIRONMENT_FILE_INVALID_ENTRY, ENVIRONMENT_FILE_UNAVAILABLE, EnvironmentFileContent, EnvironmentFileLoadError,
    EnvironmentFileProvider, EnvironmentFileRequest, ResolvedEnvironmentOrigin, ResolvedEnvironmentValue,
    SECRET_SOURCE_UNRESOLVED, SECRET_VALUE_DENIED, SecretProvider, SecretRequest, SecretResolveError, SecretSource,
    SecretValue, resolve_project_secrets, resolve_service_environment,
};
use compose_lens::source::SourceId;

struct SyntheticEnvironmentFiles;

impl EnvironmentFileProvider for SyntheticEnvironmentFiles {
    fn load(
        &self,
        request: &EnvironmentFileRequest<'_>,
    ) -> Result<Option<EnvironmentFileContent>, EnvironmentFileLoadError> {
        let content = match request.path() {
            "./base.env" => EnvironmentFileContent::sensitive(concat!(
                "Z_LAST=from-file\n",
                "A_DEFAULT=${MISSING:-fallback}\n",
                "B_ESCAPED=\"line\\n${SET}\"\n",
                "C_LITERAL='${SET}'\n",
                "EMPTY_FILE=\n",
                "UNSET_FILE\n",
            )),
            "./raw.env" => EnvironmentFileContent::plain("RAW=${SET} # literal\n"),
            _ => return Ok(None),
        };
        Ok(Some(content))
    }
}

struct SyntheticSecrets;

impl SecretProvider for SyntheticSecrets {
    fn resolve(&self, request: &SecretRequest) -> Result<Option<SecretValue>, SecretResolveError> {
        match request.source() {
            SecretSource::File(path) if path == "./secret.txt" => Ok(Some(SecretValue::new("synthetic-file-secret"))),
            SecretSource::Environment(name) if name == "SECRET_FROM_ENV" => {
                Ok(Some(SecretValue::new("synthetic-environment-secret")))
            }
            _ => Ok(None),
        }
    }
}

struct DeniedSecrets;

impl SecretProvider for DeniedSecrets {
    fn resolve(&self, _request: &SecretRequest) -> Result<Option<SecretValue>, SecretResolveError> {
        Err(SecretResolveError::Denied)
    }
}

struct ExternalSecrets;

impl SecretProvider for ExternalSecrets {
    fn resolve(&self, request: &SecretRequest) -> Result<Option<SecretValue>, SecretResolveError> {
        match request.source() {
            SecretSource::External(name)
                if matches!(name.as_str(), "deprecated-platform-name" | "preferred-platform-name") =>
            {
                Ok(Some(SecretValue::new("synthetic-external-secret")))
            }
            _ => Ok(None),
        }
    }
}

fn project(source: &str, source_id: u32) -> Result<ProjectViewResult, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(source_id),
        DocumentOrigin::new("compose.yaml", "synthetic"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    Ok(build_project_view(
        merged.project().ok_or("merged project expected")?,
        None,
    ))
}

#[test]
fn resolves_environment_precedence_states_escaping_and_key_order_without_ambient_access()
-> Result<(), Box<dyn std::error::Error>> {
    let result = project(
        concat!(
            "---\n",
            "services:\n",
            "  app:\n",
            "    env_file:\n",
            "      - ./base.env\n",
            "      - path: ./raw.env\n",
            "        required: false\n",
            "        format: raw\n",
            "    environment:\n",
            "      Z_LAST: from-service\n",
            "      EMPTY: \"\"\n",
            "      FROM_HOST:\n",
            "      NUMBER: 7\n",
            "secrets:\n",
            "  from_file:\n",
            "    file: ./secret.txt\n",
            "  from_environment:\n",
            "    environment: SECRET_FROM_ENV\n",
        ),
        81_001,
    )?;
    let view = result.view().ok_or("project view expected")?;
    let service = view.service("app").ok_or("service expected")?;
    let mut environment = MapEnvironment::new();
    environment.insert_sensitive("SET", "protected");
    environment.insert("FROM_HOST", "host-value");

    let resolution = resolve_service_environment(service, &environment, &SyntheticEnvironmentFiles);
    assert!(resolution.is_valid(), "{:?}", resolution.diagnostics());
    let names: Vec<_> = resolution
        .entries()
        .iter()
        .map(compose_lens::resolution::ResolvedEnvironmentEntry::name)
        .collect();
    assert_eq!(
        names,
        [
            "A_DEFAULT",
            "B_ESCAPED",
            "C_LITERAL",
            "EMPTY",
            "EMPTY_FILE",
            "FROM_HOST",
            "NUMBER",
            "RAW",
            "UNSET_FILE",
            "Z_LAST",
        ]
    );

    let value = |name: &str| {
        resolution
            .entries()
            .iter()
            .find(|entry| entry.name() == name)
            .map(compose_lens::resolution::ResolvedEnvironmentEntry::value)
    };
    assert!(matches!(
        value("A_DEFAULT"),
        Some(ResolvedEnvironmentValue::Value(value)) if value.value() == "fallback"
    ));
    assert!(matches!(
        value("B_ESCAPED"),
        Some(ResolvedEnvironmentValue::Value(value))
            if value.value() == "line\nprotected" && value.is_sensitive()
    ));
    assert!(matches!(
        value("C_LITERAL"),
        Some(ResolvedEnvironmentValue::Value(value)) if value.value() == "${SET}"
    ));
    assert!(matches!(
        value("EMPTY"),
        Some(ResolvedEnvironmentValue::Value(value)) if value.value().is_empty()
    ));
    assert!(matches!(
        value("FROM_HOST"),
        Some(ResolvedEnvironmentValue::Value(value)) if value.value() == "host-value"
    ));
    assert!(matches!(
        value("RAW"),
        Some(ResolvedEnvironmentValue::Value(value)) if value.value() == "${SET} # literal"
    ));
    assert!(matches!(value("UNSET_FILE"), Some(ResolvedEnvironmentValue::Unset)));
    let overridden = resolution
        .entries()
        .iter()
        .find(|entry| entry.name() == "Z_LAST")
        .ok_or("overridden value expected")?;
    assert!(matches!(
        overridden.value(),
        ResolvedEnvironmentValue::Value(value) if value.value() == "from-service"
    ));
    assert!(matches!(overridden.origin(), ResolvedEnvironmentOrigin::Service { .. }));

    let debug = format!("{resolution:?}");
    assert!(!debug.contains("protected"));

    assert_secret_resolution(view);
    Ok(())
}

fn assert_secret_resolution(view: &ProjectView) {
    let secrets = resolve_project_secrets(view, &SyntheticSecrets);
    assert!(secrets.is_valid(), "{:?}", secrets.diagnostics());
    assert_eq!(
        secrets
            .secrets()
            .iter()
            .map(|secret| secret.request().name())
            .collect::<Vec<_>>(),
        ["from_environment", "from_file"]
    );
    assert_eq!(secrets.secrets()[0].value().expose(), "synthetic-environment-secret");
    let debug = format!("{secrets:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("synthetic-environment-secret"));
    assert!(!debug.contains("synthetic-file-secret"));
}

#[test]
fn external_false_stays_project_owned_and_external_names_follow_compose_precedence()
-> Result<(), Box<dyn std::error::Error>> {
    let result = project(
        concat!(
            "---\n",
            "services:\n",
            "  app:\n",
            "    image: example.invalid/app:1\n",
            "secrets:\n",
            "  project_owned:\n",
            "    external: false\n",
            "  deprecated_external:\n",
            "    external:\n",
            "      name: deprecated-platform-name\n",
            "  named_external:\n",
            "    name: preferred-platform-name\n",
            "    external:\n",
            "      name: ignored-deprecated-name\n",
        ),
        81_004,
    )?;
    let view = result.view().ok_or("project view expected")?;
    let resolution = resolve_project_secrets(view, &ExternalSecrets);

    assert_eq!(
        resolution
            .secrets()
            .iter()
            .map(|secret| { (secret.request().name().to_owned(), secret.request().source().clone(),) })
            .collect::<Vec<_>>(),
        vec![
            (
                "deprecated_external".to_owned(),
                SecretSource::External("deprecated-platform-name".to_owned()),
            ),
            (
                "named_external".to_owned(),
                SecretSource::External("preferred-platform-name".to_owned()),
            ),
        ]
    );
    assert_eq!(resolution.diagnostics().len(), 1);
    assert_eq!(resolution.diagnostics()[0].code(), SECRET_SOURCE_UNRESOLVED);
    assert!(!format!("{resolution:?}").contains("synthetic-external-secret"));
    Ok(())
}

#[test]
fn reports_missing_invalid_and_denied_inputs_without_echoing_payloads() -> Result<(), Box<dyn std::error::Error>> {
    struct InvalidFiles;
    impl EnvironmentFileProvider for InvalidFiles {
        fn load(
            &self,
            request: &EnvironmentFileRequest<'_>,
        ) -> Result<Option<EnvironmentFileContent>, EnvironmentFileLoadError> {
            match request.path() {
                "./invalid.env" => Ok(Some(EnvironmentFileContent::sensitive(
                    "INVALID NAME=must-not-appear\n",
                ))),
                _ => Ok(None),
            }
        }
    }

    let result = project(
        concat!(
            "---\n",
            "services:\n",
            "  app:\n",
            "    env_file:\n",
            "      - ./required.env\n",
            "      - ./invalid.env\n",
            "      - path: ./optional.env\n",
            "        required: false\n",
            "secrets:\n",
            "  unresolved: {}\n",
            "  denied:\n",
            "    environment: DENIED_SECRET\n",
        ),
        81_002,
    )?;
    let view = result.view().ok_or("project view expected")?;
    let service = view.service("app").ok_or("service expected")?;
    let resolution = resolve_service_environment(service, &MapEnvironment::new(), &InvalidFiles);
    assert!(!resolution.is_valid());
    assert!(
        resolution
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == ENVIRONMENT_FILE_UNAVAILABLE)
    );
    assert!(
        resolution
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == ENVIRONMENT_FILE_INVALID_ENTRY)
    );
    assert!(!format!("{:?}", resolution.diagnostics()).contains("must-not-appear"));

    let secrets = resolve_project_secrets(view, &DeniedSecrets);
    assert!(!secrets.is_valid());
    assert!(
        secrets
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SECRET_SOURCE_UNRESOLVED)
    );
    assert!(
        secrets
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == SECRET_VALUE_DENIED)
    );
    Ok(())
}

#[test]
fn environment_value_debug_redacts_only_values_marked_sensitive() {
    let plain = EnvironmentValue::plain("public");
    let sensitive = EnvironmentValue::sensitive("private");
    assert!(format!("{plain:?}").contains("public"));
    assert!(format!("{sensitive:?}").contains("<redacted>"));
    assert!(!format!("{sensitive:?}").contains("private"));
}

#[test]
fn interpolation_debug_redacts_sensitive_inputs_and_results() -> Result<(), Box<dyn std::error::Error>> {
    let mut environment = MapEnvironment::new();
    environment.insert_sensitive("TOKEN", "synthetic-private-token");
    let span = compose_lens::source::SourceSpan::new(SourceId::new(81_003), 0, 8).ok_or("synthetic span expected")?;
    let input = InterpolationInput::new("${TOKEN}", span).sensitive();
    let input_debug = format!("{input:?}");
    assert!(input_debug.contains("<redacted>"));
    assert!(!input_debug.contains("${TOKEN}"));

    let result = interpolate(input, &environment);
    let result_debug = format!("{result:?}");
    assert!(result_debug.contains("<redacted>"));
    assert!(!result_debug.contains("synthetic-private-token"));
    assert!(!format!("{environment:?}").contains("synthetic-private-token"));
    Ok(())
}
