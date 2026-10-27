//! Native inert extension construction, source presentation, and protected metadata.
use compose_lens::{
    loader::{DocumentInput, DocumentOrigin, LoadedProject},
    merge::{MergedEntry, MergedProject, MergedScalarKind, MergedValueKind, merge_project},
    render::{
        ComposeDocumentBuilder, GENERATED_EXTENSION_MAX_DEPTH, GENERATED_EXTENSION_MAX_NODES, GeneratedExtension,
        GeneratedExtensionEntry, GeneratedExtensionValue as Value, GeneratedService, GeneratedString, GenerationError,
    },
    source::SourceId,
    syntax::ScalarPresentation,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn plain(value: &str) -> Result<GeneratedString, GenerationError> {
    GeneratedString::plain(value)
}
fn entry(key: &str, value: Value) -> Result<GeneratedExtensionEntry, GenerationError> {
    Ok(GeneratedExtensionEntry::new(plain(key)?, value))
}
fn merged(text: &str) -> Result<MergedProject, Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(1),
        DocumentOrigin::new("compose.yaml", "extension-test"),
        text,
    )])?;
    let result = merge_project(&loaded, None);
    assert!(result.is_valid(), "{:?}", result.diagnostics());
    Ok(result.project().ok_or("missing merged project")?.clone())
}
fn builder() -> Result<ComposeDocumentBuilder, GenerationError> {
    let mut builder = ComposeDocumentBuilder::new();
    let mut service = GeneratedService::new("app")?;
    service.set_image(plain("example.invalid/app:1")?)?;
    builder.add_service(service)?;
    Ok(builder)
}

#[test]
fn recursive_extensions_preserve_native_kinds_order_case_and_numeric_spelling() -> TestResult {
    let nested = Value::mapping(vec![
        entry("Mixed", Value::null())?,
        entry("mixed", Value::boolean(true))?,
        entry("number", Value::number(plain("900719925474099312345678901234567890")?)?)?,
        entry("spelling", Value::number(plain("0xDeAd")?)?)?,
        entry("string", Value::string(plain("false")?))?,
        entry(
            "",
            Value::sequence(vec![
                Value::null(),
                Value::boolean(false),
                Value::mapping(vec![])?,
                Value::sequence(vec![])?,
            ])?,
        )?,
    ])?;
    let mut service = GeneratedService::new("app")?;
    service.set_image(plain("example.invalid/app:1")?)?;
    service.add_extension(GeneratedExtension::new(plain("x-Service")?, nested.clone())?)?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(service)?;
    builder.add_extension(GeneratedExtension::new(plain("x-Vendor")?, nested)?)?;
    builder.add_extension(GeneratedExtension::new(plain("x-vendor")?, Value::null())?)?;
    let generated = builder.build(SourceId::new(2))?;
    assert_eq!(
        generated.text(),
        concat!(
            "---\nservices:\n  app:\n    image: example.invalid/app:1\n    x-Service:\n",
            "      Mixed: null\n      mixed: true\n      number: 900719925474099312345678901234567890\n",
            "      spelling: 0xDeAd\n      string: \"false\"\n      \"\":\n",
            "        - null\n        - false\n        - {}\n        - []\n",
            "x-Vendor:\n  Mixed: null\n  mixed: true\n  number: 900719925474099312345678901234567890\n",
            "  spelling: 0xDeAd\n  string: \"false\"\n  \"\":\n",
            "    - null\n    - false\n    - {}\n    - []\nx-vendor: null\n",
        )
    );
    let project = merged(generated.text())?;
    let extension = project.root().get("x-Vendor").ok_or("extension")?;
    let entries = extension.as_mapping().ok_or("mapping")?;
    assert_eq!(
        entries.iter().map(MergedEntry::key).collect::<Vec<_>>(),
        ["Mixed", "mixed", "number", "spelling", "string", ""]
    );
    assert!(entries.iter().all(MergedEntry::is_strict_yaml_string));
    let number = extension
        .get("number")
        .and_then(|value| value.as_scalar())
        .ok_or("number")?;
    assert_eq!(number.kind(), MergedScalarKind::Number);
    assert_eq!(number.value(), "900719925474099312345678901234567890");
    assert_eq!(
        extension
            .get("spelling")
            .and_then(|value| value.as_scalar())
            .ok_or("hex")?
            .raw(),
        "0xDeAd"
    );
    assert_eq!(
        extension
            .get("string")
            .and_then(|value| value.as_scalar())
            .ok_or("string")?
            .kind(),
        MergedScalarKind::String
    );
    assert_eq!(generated.document().extension_fields().len(), 2);
    assert_eq!(
        generated
            .document()
            .service("app")
            .ok_or("app")?
            .extension_fields()
            .len(),
        1
    );
    Ok(())
}

#[test]
fn folded_extensions_keep_exact_semantics_for_empty_paragraphs_indentation_and_chomping() -> TestResult {
    let cases = [
        "",
        "\n",
        "\n\n",
        "one",
        "one\n",
        "one\n\n",
        "one\n\n\n",
        "one\ntwo",
        "one\n\ntwo",
        "one\n\n\ntwo\n",
        "  indented\nordinary",
        "ordinary\n  indented\nordinary\n",
        "\nleading\n",
        " \n  \n",
        "one\n \ntwo",
        "quote ' and \"\n# content\n",
    ];
    for (index, expected) in cases.iter().enumerate() {
        let mut builder = builder()?;
        builder.add_extension(GeneratedExtension::new(
            plain("x-folded")?,
            Value::folded_string(plain(expected)?).map_err(|error| format!("folded input {expected:?}: {error}"))?,
        )?)?;
        let generated = builder.build(SourceId::new(u32::try_from(index + 10)?))?;
        assert!(generated.text().contains("x-folded: >2"));
        let project = merged(generated.text())?;
        let scalar = project
            .root()
            .get("x-folded")
            .and_then(|value| value.as_scalar())
            .ok_or("folded scalar")?;
        assert_eq!(scalar.value(), *expected, "input {expected:?}");
        assert_eq!(scalar.presentation(), ScalarPresentation::Folded);
        assert!(scalar.is_strict_yaml_string());
        assert_eq!(generated.document().extension_fields().len(), 1);
    }
    // Exercise nested map/sequence block indentation independently of the top-level case.
    let mut builder = builder()?;
    builder.add_extension(GeneratedExtension::new(
        plain("x-nested")?,
        Value::sequence(vec![Value::mapping(vec![entry(
            "text",
            Value::folded_string(plain("one\ntwo\n\n")?)?,
        )?])?])?,
    )?)?;
    let generated = builder.build(SourceId::new(50))?;
    let project = merged(generated.text())?;
    let items = project
        .root()
        .get("x-nested")
        .and_then(|value| value.as_sequence())
        .ok_or("sequence")?;
    assert_eq!(
        items[0]
            .get("text")
            .and_then(|value| value.as_scalar())
            .ok_or("text")?
            .value(),
        "one\ntwo\n\n"
    );
    Ok(())
}

#[test]
fn invalid_names_numbers_duplicates_and_tree_bounds_are_explicit_and_redacted() -> TestResult {
    for key in ["x-", "X-vendor", "image", "x-line\nbreak", "x-line\rbreak"] {
        assert_eq!(
            GeneratedExtension::new(GeneratedString::sensitive(key)?, Value::null()).err(),
            Some(GenerationError::InvalidExtensionName)
        );
    }
    for spelling in [
        "",
        "1 2",
        "1\n",
        "1 # comment",
        "true",
        "null",
        "1: 2",
        "[1]",
        "!!int 1",
        "1\nx: 2",
    ] {
        assert_eq!(
            Value::number(GeneratedString::sensitive(spelling)?).err(),
            Some(GenerationError::InvalidExtensionNumber)
        );
    }
    for spelling in ["-7", "0", "1.2500", "1e1000", "0o777", ".inf", ".nan"] {
        assert!(Value::number(plain(spelling)?).is_ok(), "{spelling}");
    }
    for value in ["one\rtwo", "a\u{0001}b", "\tfirst\nsecond"] {
        assert_eq!(
            Value::folded_string(plain(value)?).err(),
            Some(GenerationError::InvalidFoldedExtensionString)
        );
    }
    let duplicate = Value::mapping(vec![
        entry("private-key", Value::null())?,
        GeneratedExtensionEntry::new(GeneratedString::sensitive("private-key")?, Value::null()),
    ]);
    let error = duplicate.err().ok_or("duplicate error")?;
    assert_eq!(error, GenerationError::DuplicateExtensionKey);
    assert!(!format!("{error:?} {error}").contains("private-key"));
    let mut builder = builder()?;
    builder.add_extension(GeneratedExtension::new(
        GeneratedString::sensitive("x-private-key")?,
        Value::null(),
    )?)?;
    assert_eq!(
        builder.add_extension(GeneratedExtension::new(plain("x-private-key")?, Value::null())?),
        Err(GenerationError::DuplicateExtensionKey)
    );
    let mut service = GeneratedService::new("app")?;
    service.add_extension(GeneratedExtension::new(plain("x-one")?, Value::null())?)?;
    assert_eq!(
        service.add_extension(GeneratedExtension::new(plain("x-one")?, Value::null())?),
        Err(GenerationError::DuplicateExtensionKey)
    );
    let mut deep = Value::null();
    for _ in 0..GENERATED_EXTENSION_MAX_DEPTH {
        deep = Value::sequence(vec![deep])?;
    }
    let mut maximum_depth = builder.clone();
    maximum_depth.add_extension(GeneratedExtension::new(plain("x-deep")?, deep.clone())?)?;
    let generated = maximum_depth.build(SourceId::new(85))?;
    assert!(generated.text().contains("x-deep:"));
    assert_eq!(Value::sequence(vec![deep]).err(), Some(GenerationError::ExtensionLimit));
    assert!(Value::sequence(vec![Value::null(); GENERATED_EXTENSION_MAX_NODES - 1]).is_ok());
    assert_eq!(
        Value::sequence(vec![Value::null(); GENERATED_EXTENSION_MAX_NODES]).err(),
        Some(GenerationError::ExtensionLimit)
    );
    assert_eq!(
        Value::mapping(
            (0..GENERATED_EXTENSION_MAX_NODES / 2)
                .map(|index| entry(&index.to_string(), Value::null()))
                .collect::<Result<Vec<_>, _>>()?
        )
        .err(),
        Some(GenerationError::ExtensionLimit)
    );
    Ok(())
}

#[test]
fn protected_keys_and_values_redact_at_each_generated_and_parse_back_boundary() -> TestResult {
    let value = Value::mapping(vec![GeneratedExtensionEntry::new(
        GeneratedString::sensitive("nested-secret-key")?,
        Value::sequence(vec![
            Value::string(GeneratedString::sensitive("private-value")?),
            Value::number(GeneratedString::sensitive("123456789")?)?,
            Value::folded_string(GeneratedString::sensitive("folded-secret\nvalue\n")?)?,
        ])?,
    )])?;
    let extension = GeneratedExtension::new(GeneratedString::sensitive("x-secret-key")?, value)?;
    let mut service = GeneratedService::new("app")?;
    service.add_extension(extension.clone())?;
    let mut builder = ComposeDocumentBuilder::new();
    builder.add_service(service.clone())?;
    builder.add_extension(extension.clone())?;
    for debug in [format!("{extension:?}"), format!("{service:?}"), format!("{builder:?}")] {
        for secret in [
            "nested-secret-key",
            "private-value",
            "123456789",
            "folded-secret",
            "x-secret-key",
        ] {
            assert!(!debug.contains(secret));
        }
    }
    let generated = builder.build(SourceId::new(60))?;
    assert!(generated.is_sensitive());
    assert!(generated.text().contains("private-value"));
    for debug in [
        format!("{generated:?}"),
        format!("{:?}", generated.document()),
        format!("{:?}", generated.document().service("app")),
    ] {
        assert!(!debug.contains("x-secret-key"));
    }
    for field in generated
        .document()
        .extension_fields()
        .iter()
        .chain(generated.document().service("app").ok_or("app")?.extension_fields())
    {
        assert_eq!(field.name().value(), "x-secret-key");
        assert!(field.name().is_sensitive());
        assert!(!format!("{field:?}").contains("x-secret-key"));
        assert!(!format!("{:?}", field.name()).contains("x-secret-key"));
    }
    Ok(())
}

#[test]
fn native_accessors_distinguish_unsupported_tags_non_string_keys_and_scalar_presentations() -> TestResult {
    let source = concat!(
        "---\nservices: {app: {image: example.invalid/app:1}}\nx-data:\n",
        "  plain: ordinary\n  single: 'ordinary'\n  double: \"ordinary\"\n",
        "  literal: |-\n    one\n    two\n  folded: >-\n    one\n    two\n",
        "  tagged: !vendor ordinary\n  date: 2026-10-27\n  null: null\n  true: true\n  12: 12\n  alias: *missing\n  !vendor key: tagged-key\n  [a, b]: retained\n"
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(80),
        DocumentOrigin::new("compose.yaml", "unsupported-keys"),
        source,
    )])?;
    let result = merge_project(&loaded, None);
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code().as_str() == "compose.model.expected-scalar")
    );
    let project = result.project().ok_or("project with retained unsupported key")?;
    let data = project.root().get("x-data").ok_or("data")?;
    for (key, presentation) in [
        ("plain", ScalarPresentation::Plain),
        ("single", ScalarPresentation::SingleQuoted),
        ("double", ScalarPresentation::DoubleQuoted),
        ("literal", ScalarPresentation::Literal),
        ("folded", ScalarPresentation::Folded),
    ] {
        assert_eq!(
            data.get(key)
                .and_then(|value| value.as_scalar())
                .ok_or("scalar")?
                .presentation(),
            presentation
        );
    }
    assert!(
        matches!(data.get("tagged").ok_or("tagged")?.kind(), MergedValueKind::Tagged { tag, .. } if tag == "!vendor")
    );
    assert!(
        !data
            .get("12")
            .and_then(|value| value.as_scalar())
            .ok_or("number")?
            .is_strict_yaml_string()
    );
    assert!(matches!(data.get("alias").ok_or("alias")?.kind(), MergedValueKind::Alias(name) if name == "missing"));
    let entries = data.as_mapping().ok_or("mapping")?;
    assert!(
        entries
            .iter()
            .any(|entry| !entry.is_strict_yaml_string() && entry.key().starts_with('['))
    );
    assert!(
        entries
            .iter()
            .any(|entry| !entry.is_strict_yaml_string() && entry.key().starts_with("!vendor")),
        "{:?}",
        entries
            .iter()
            .map(|entry| (entry.key(), entry.is_strict_yaml_string()))
            .collect::<Vec<_>>()
    );
    let tagged_key = entries
        .iter()
        .find(|entry| entry.key() == "!vendor key")
        .ok_or("tagged key evidence")?;
    assert_eq!(&source[tagged_key.key_sources()[0].range()], "!vendor key");
    for key in ["null", "true", "12"] {
        assert!(
            !entries
                .iter()
                .find(|entry| entry.key() == key)
                .ok_or("key")?
                .is_strict_yaml_string()
        );
    }
    Ok(())
}

#[test]
fn merged_sensitivity_includes_protected_mapping_keys() -> TestResult {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(70),
        DocumentOrigin::new("compose.yaml", "key-taint"),
        "---\nservices:\n  app:\n    image: example.invalid/app:1\n    labels: [\"${KEY}\"]\n",
    )])?;
    let mut environment = compose_lens::interpolation::MapEnvironment::new();
    let _ = environment.insert_sensitive("KEY", "protected-key");
    let interpolation = loaded.interpolate(&environment);
    let result = merge_project(&loaded, Some(&interpolation));
    let root = result.project().ok_or("project")?.root();
    assert!(root.is_sensitive());
    assert!(
        root.get("services")
            .and_then(|services| services.get("app"))
            .and_then(|app| app.get("labels"))
            .ok_or("labels")?
            .is_sensitive()
    );
    assert!(!format!("{root:?}").contains("protected-key"));
    Ok(())
}

#[test]
fn unsupported_and_authored_string_key_collisions_survive_multi_file_merge() -> TestResult {
    let sources = [
        "---\nservices: {app: {image: example.invalid/app:1}}\nx-data: {[a, b]: unsupported-base, \"[a, b]\": string-base, 12: numeric-base, \"12\": string-number-base}\n",
        "---\nx-data: {\"[a, b]\": string-overlay, [a, b]: unsupported-overlay, \"12\": string-number-overlay, 12: numeric-overlay}\n",
    ];
    let mut inputs = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        inputs.push(DocumentInput::new(
            SourceId::new(u32::try_from(index + 90)?),
            DocumentOrigin::new(format!("{index}.yaml"), "collision"),
            *source,
        ));
    }
    let loaded = LoadedProject::load(inputs)?;
    let result = merge_project(&loaded, None);
    let entries = result
        .project()
        .ok_or("recoverable project")?
        .root()
        .get("x-data")
        .and_then(|value| value.as_mapping())
        .ok_or("entries")?;
    assert_eq!(entries.len(), 4, "{entries:?}");
    for (key, strict, expected) in [
        ("[a, b]", false, "unsupported-overlay"),
        ("[a, b]", true, "string-overlay"),
        ("12", false, "numeric-overlay"),
        ("12", true, "string-number-overlay"),
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry.key() == key && entry.is_strict_yaml_string() == strict)
            .ok_or("distinct key entry")?;
        assert_eq!(
            entry.value().as_scalar().ok_or("value")?.value(),
            expected,
            "{entries:?}"
        );
        assert_eq!(entry.key_sources().len(), 2);
        for span in entry.key_sources() {
            assert!(span.start() < span.end());
        }
    }
    Ok(())
}

#[test]
fn literal_merge_key_strings_are_inert_and_do_not_expand_into_metadata() -> TestResult {
    let mut builder = builder()?;
    builder.add_extension(GeneratedExtension::new(
        plain("x-data")?,
        Value::mapping(vec![entry(
            "<<",
            Value::mapping(vec![entry("inert", Value::boolean(true))?])?,
        )?])?,
    )?)?;
    let generated = builder.build(SourceId::new(95))?;
    assert!(generated.text().contains("\"<<\":\n"));
    let project = merged(generated.text())?;
    let entries = project
        .root()
        .get("x-data")
        .and_then(|value| value.as_mapping())
        .ok_or("entries")?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key(), "<<");
    assert!(entries[0].is_strict_yaml_string());
    assert!(entries[0].value().get("inert").is_some());
    Ok(())
}

#[test]
fn plain_yaml_merge_keys_still_expand_and_local_values_win() -> TestResult {
    let project = merged(concat!(
        "---\nservices: {app: {image: example.invalid/app:1}}\n",
        "x-base: &base\n  inherited: from-anchor\n  local: from-anchor\n",
        "x-data:\n  <<: *base\n  local: explicit\n"
    ))?;
    let data = project.root().get("x-data").ok_or("merged extension")?;
    assert_eq!(
        data.get("inherited")
            .and_then(|value| value.as_scalar())
            .ok_or("inherited")?
            .value(),
        "from-anchor"
    );
    assert_eq!(
        data.get("local")
            .and_then(|value| value.as_scalar())
            .ok_or("local")?
            .value(),
        "explicit"
    );
    assert!(data.get("<<").is_none());
    Ok(())
}

#[test]
fn generated_string_keys_and_values_preserve_unicode_newlines_and_literal_expressions() -> TestResult {
    let key = "unicode-ä\nkey: value";
    let value = "${COMPOSE_LENS_METADATA_TEST}\n雪: yes\n";
    let mut builder = builder()?;
    builder.add_extension(GeneratedExtension::new(
        plain("x-strings")?,
        Value::mapping(vec![GeneratedExtensionEntry::new(
            plain(key)?,
            Value::string(plain(value)?),
        )])?,
    )?)?;
    let generated = builder.build(SourceId::new(100))?;
    assert!(!generated.is_sensitive());
    let project = merged(generated.text())?;
    let entries = project
        .root()
        .get("x-strings")
        .and_then(|value| value.as_mapping())
        .ok_or("mapping")?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key(), key);
    assert!(entries[0].is_strict_yaml_string());
    assert_eq!(entries[0].value().as_scalar().ok_or("string")?.value(), value);
    assert!(!generated.document().extension_fields()[0].name().is_sensitive());
    Ok(())
}

#[test]
fn anchored_mapping_keys_consumed_as_plain_content_are_diagnosed_without_affecting_strings() -> TestResult {
    let source = "---\nservices: {app: {image: example.invalid/app:1}}\nx-data:\n  ordinary: first\n  &label plainkey: anchored-key\n  next: last\n";
    let syntax = compose_lens::syntax::SyntaxDocument::parse(SourceId::new(110), source)?;
    assert!(!syntax.is_valid());
    let diagnostic = syntax
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code() == compose_lens::syntax::YAML_UNPARSED_INPUT)
        .ok_or("unparsed structure diagnostic")?;
    assert_eq!(
        diagnostic.labels()[0].span().start(),
        source.find("&label").ok_or("anchored key")?
    );
    assert_eq!(syntax.document().render_preserved(), source);
    let mut builder = builder()?;
    for (name, value) in [
        ("x-quoted", Value::string(plain("first\n&label plainkey: content")?)),
        (
            "x-folded",
            Value::folded_string(plain("first\n&label plainkey: content")?)?,
        ),
    ] {
        builder.add_extension(GeneratedExtension::new(plain(name)?, value)?)?;
    }
    assert!(builder.build(SourceId::new(111)).is_ok());
    Ok(())
}

#[test]
fn yaml_merge_inheritance_retains_typed_and_unsupported_key_evidence() -> TestResult {
    let source = concat!(
        "---\nservices: {app: {image: example.invalid/app:1}}\n",
        "x-base: &base {12: numeric, \"12\": string, [a, b]: unsupported}\n",
        "x-data: {<<: *base}\n"
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(120),
        DocumentOrigin::new("compose.yaml", "inherited-key-evidence"),
        source,
    )])?;
    let result = merge_project(&loaded, None);
    let project = result.project().ok_or("recoverable project")?;
    let inherited = project
        .root()
        .get("x-data")
        .and_then(|value| value.as_mapping())
        .ok_or("inherited entries")?;
    assert_eq!(inherited.len(), 3);
    for (key, strict, expected) in [
        ("12", false, "numeric"),
        ("12", true, "string"),
        ("[a, b]", false, "unsupported"),
    ] {
        let entry = inherited
            .iter()
            .find(|entry| entry.key() == key && entry.is_strict_yaml_string() == strict)
            .ok_or("distinct inherited key")?;
        assert_eq!(entry.value().as_scalar().ok_or("inherited value")?.value(), expected);
        assert_eq!(entry.key_sources().len(), 1);
        let raw = &source[entry.key_sources()[0].range()];
        assert_eq!(raw, if strict { "\"12\"" } else { key });
    }
    Ok(())
}

#[test]
fn quoted_merge_key_never_donates_fields_alongside_a_real_merge() -> TestResult {
    let project = merged(concat!(
        "---\nservices: {app: {image: example.invalid/app:1}}\n",
        "x-a: &a {kept: from-a}\nx-b: &b {ghost: from-b}\n",
        "x-data: {<<: *a, \"<<\": *b}\n"
    ))?;
    let data = project.root().get("x-data").ok_or("extension")?;
    assert_eq!(data.as_mapping().ok_or("mapping")?.len(), 2);
    assert_eq!(
        data.get("kept")
            .and_then(|value| value.as_scalar())
            .ok_or("inherited from a")?
            .value(),
        "from-a"
    );
    assert!(data.get("ghost").is_none());
    assert_eq!(
        data.get("<<")
            .and_then(|value| value.get("ghost"))
            .and_then(|value| value.as_scalar())
            .ok_or("literal subtree")?
            .value(),
        "from-b"
    );
    Ok(())
}

#[test]
fn yaml_merge_sequence_keeps_first_source_precedence_and_local_overrides() -> TestResult {
    let project = merged(concat!(
        "---\nservices: {app: {image: example.invalid/app:1}}\n",
        "x-a: &a {shared: first, one: from-a}\nx-b: &b {shared: second, two: from-b}\n",
        "x-data: {<<: [*a, *b], tail: local}\nx-local: {<<: [*a, *b], shared: explicit}\n"
    ))?;
    for (name, shared) in [("x-data", "first"), ("x-local", "explicit")] {
        let data = project.root().get(name).ok_or("extension")?;
        assert_eq!(
            data.get("shared")
                .and_then(|value| value.as_scalar())
                .ok_or("precedence")?
                .value(),
            shared
        );
        assert_eq!(
            data.get("one").and_then(|value| value.as_scalar()).ok_or("a")?.value(),
            "from-a"
        );
        assert_eq!(
            data.get("two").and_then(|value| value.as_scalar()).ok_or("b")?.value(),
            "from-b"
        );
    }
    Ok(())
}

#[test]
fn inherited_duplicate_and_invalid_merge_evidence_is_not_silently_deduplicated() -> TestResult {
    let source = concat!(
        "---\nservices: {app: {image: example.invalid/app:1}}\n",
        "x-base: &base {same: first, same: second}\nx-data: {<<: *base}\n",
        "x-invalid: {<<: [{kept: evidence}, *missing], local: unchanged}\n",
        "x-scalar: {<<: not-a-mapping}\n"
    );
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(130),
        DocumentOrigin::new("compose.yaml", "merge-source-failure"),
        source,
    )])?;
    let result = merge_project(&loaded, None);
    let root = result.project().ok_or("recoverable project")?.root();
    let duplicates = root
        .get("x-data")
        .and_then(|value| value.as_mapping())
        .ok_or("duplicate evidence")?;
    assert_eq!(duplicates.len(), 2);
    assert_eq!(
        duplicates
            .iter()
            .map(|entry| entry.value().as_scalar().map(compose_lens::merge::MergedScalar::value))
            .collect::<Vec<_>>(),
        [Some("first"), Some("second")]
    );
    let invalid = root
        .get("x-invalid")
        .and_then(|value| value.as_mapping())
        .ok_or("invalid merge")?;
    let merge_key = invalid
        .iter()
        .find(|entry| entry.key() == "<<")
        .ok_or("retained merge source")?;
    assert!(!merge_key.is_strict_yaml_string());
    assert_eq!(&source[merge_key.key_sources()[0].range()], "<<");
    let sequence = merge_key.value().as_sequence().ok_or("retained sequence")?;
    assert_eq!(sequence.len(), 2);
    assert!(sequence[0].get("kept").is_some());
    assert!(matches!(sequence[1].kind(), MergedValueKind::Alias(name) if name == "missing"));
    assert!(root.get("x-invalid").and_then(|value| value.get("kept")).is_none());
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == compose_lens::merge::UNRESOLVED_ALIAS)
    );
    let scalar = root
        .get("x-scalar")
        .and_then(|value| value.as_mapping())
        .ok_or("scalar merge source")?;
    assert!(!scalar[0].is_strict_yaml_string());
    assert_eq!(
        scalar[0].value().as_scalar().ok_or("scalar source evidence")?.value(),
        "not-a-mapping"
    );
    Ok(())
}

#[test]
fn recursive_merge_aliases_are_bounded_and_preserve_unresolved_evidence() -> TestResult {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(131),
        DocumentOrigin::new("compose.yaml", "recursive-merge-source"),
        "---\nservices: {app: {image: example.invalid/app:1}}\nx-base: &base {<<: *base, kept: evidence}\nx-data: {<<: *base}\n",
    )])?;
    let result = merge_project(&loaded, None);
    let data = result
        .project()
        .ok_or("recoverable project")?
        .root()
        .get("x-data")
        .and_then(|value| value.as_mapping())
        .ok_or("bounded inheritance")?;
    assert_eq!(data.len(), 2);
    let invalid = data
        .iter()
        .find(|entry| entry.key() == "<<")
        .ok_or("unresolved merge source")?;
    assert!(!invalid.is_strict_yaml_string());
    assert!(matches!(invalid.value().kind(), MergedValueKind::Alias(name) if name == "base"));
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == compose_lens::merge::UNRESOLVED_ALIAS)
    );
    Ok(())
}
