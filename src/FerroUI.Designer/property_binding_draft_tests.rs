use super::*;

#[test]
fn unchanged_and_unmodified_arguments_retain_exact_lexical_source() {
    let source = "{ f:Binding  \"Items['a,b'].Name\" ,  Converter = {f:StaticResource 'C,1'}, FallbackValue=\"keep 'quoted'\", Unknown={app:Custom One='a,b', Two={f:StaticResource D}} }  ";
    let mut draft = PropertyBindingDraft::new(source, "f:Binding").unwrap();
    assert_eq!(draft.source(), source);
    draft.set_path("Items['a,b'].Name");
    assert_eq!(draft.source(), source);
    draft.set_path("Items['c,d'].Name");
    assert!(draft.source().contains("  Converter = {f:StaticResource 'C,1'}, FallbackValue=\"keep 'quoted'\", Unknown={app:Custom One='a,b', Two={f:StaticResource D}} }  "));
    assert_eq!(
        PropertyBindingDraft::new(&draft.source(), "f:Binding")
            .unwrap()
            .literal("Path"),
        Some("Items['c,d'].Name")
    );
    for empty in ["{Binding}", "{Binding   }\n"] {
        assert_eq!(
            PropertyBindingDraft::new(empty, "Binding")
                .unwrap()
                .source(),
            empty
        );
    }
}

#[test]
fn arguments_with_escaped_separators_and_literal_braces_stay_literals() {
    let mut draft = PropertyBindingDraft::new(
        "{Binding A\\,B, StringFormat='{}{0}, x', ConverterParameter='back\\\\slash'}",
        "Binding",
    )
    .unwrap();
    assert_eq!(draft.literal("Path"), Some("A,B"));
    let value = "{literal}, O'Neil \\ + unicode żółć";
    draft
        .set_option(
            PropertyBindingOption::FallbackValue,
            Some(MarkupValue::Literal(value.into())),
        )
        .unwrap();
    let parsed = PropertyBindingDraft::new(&draft.source(), "Binding").unwrap();
    assert_eq!(parsed.literal("FallbackValue"), Some(value));
    assert!(
        draft
            .source()
            .contains("StringFormat='{}{0}, x', ConverterParameter='back\\\\slash'")
    );
}

#[test]
fn source_changes_are_exclusive_qualified_and_atomic_on_validation_error() {
    let mut draft = PropertyBindingDraft::new("{f:Binding Name, Source={f:StaticResource Model}, ElementName=Other, Converter={f:StaticResource C}}", "f:Binding").unwrap().with_relative_source_extension(Some("f:RelativeSource"));
    assert!(draft.validate().is_err());
    draft
        .set_source(PropertyBindingSource::Ancestor {
            type_name: "app:Editor".into(),
            level: 2,
            logical_tree: true,
        })
        .unwrap();
    assert!(draft.argument("Source").is_none());
    assert!(draft.argument("ElementName").is_none());
    assert_eq!(
        draft.source_specification(),
        Some(PropertyBindingSource::Ancestor {
            type_name: "app:Editor".into(),
            level: 2,
            logical_tree: true
        })
    );
    assert!(draft.source().contains("f:RelativeSource"));
    assert!(draft.source().contains("Converter={f:StaticResource C}"));
    let accepted = draft.source();
    assert!(
        draft
            .set_source(PropertyBindingSource::Ancestor {
                type_name: "Grid".into(),
                level: 0,
                logical_tree: false
            })
            .is_err()
    );
    assert_eq!(draft.source(), accepted);
    draft
        .set_source(PropertyBindingSource::DataContext)
        .unwrap();
    assert!(draft.argument("RelativeSource").is_none());
    draft.validate().unwrap();
}

#[test]
fn optional_values_keep_missing_empty_literal_and_null_expression_distinct() {
    let mut draft = PropertyBindingDraft::empty("Binding").unwrap();
    draft
        .set_option(
            PropertyBindingOption::FallbackValue,
            Some(MarkupValue::Literal(String::new())),
        )
        .unwrap();
    draft
        .set_option(
            PropertyBindingOption::TargetNullValue,
            Some(MarkupValue::parse("{lang:Null}").unwrap()),
        )
        .unwrap();
    assert_eq!(
        draft.argument("FallbackValue"),
        Some(&MarkupValue::Literal(String::new()))
    );
    assert!(
        matches!(draft.argument("TargetNullValue"), Some(MarkupValue::Extension(extension)) if extension.name == "lang:Null")
    );
    draft
        .set_option(PropertyBindingOption::FallbackValue, None)
        .unwrap();
    assert!(draft.argument("FallbackValue").is_none());
    assert!(draft.argument("TargetNullValue").is_some());
}

#[test]
fn invalid_native_modes_delays_and_ambiguous_paths_are_rejected_without_mutation() {
    let mut draft = PropertyBindingDraft::new("{Binding Name, Mode=OneWay}", "Binding").unwrap();
    let original = draft.source();
    assert!(draft.set_mode("OneWayOrOther").is_err());
    assert!(draft.set_update_source_trigger("Whenever").is_err());
    assert!(draft.set_delay(Some(-1)).is_err());
    assert!(
        draft
            .set_option(
                PropertyBindingOption::Converter,
                Some(MarkupValue::Literal("NotAnObject".into()))
            )
            .is_err()
    );
    assert_eq!(draft.source(), original);
    assert!(PropertyBindingDraft::new("{Binding A, Path=B}", "Binding").is_err());
    draft.set_update_source_trigger("LostFocus").unwrap();
    draft.set_delay(Some(i32::MAX)).unwrap();
    draft.validate().unwrap();
}

#[test]
fn framework_binding_flavors_preserve_aliases_and_custom_extensions_are_never_reinterpreted() {
    for name in ["ui:Binding", "ui:CompiledBinding", "ui:ReflectionBinding"] {
        let mut draft =
            PropertyBindingDraft::new(&format!("{{{name} Name, Custom={{app:Data x}}}}"), name)
                .unwrap();
        draft = draft.with_relative_source_extension(Some("ui:RelativeSource"));
        draft.set_source(PropertyBindingSource::SelfValue).unwrap();
        assert!(draft.source().starts_with(&format!("{{{name} ")));
        assert!(draft.source().contains("ui:RelativeSource"));
        assert!(draft.source().contains("Custom={app:Data x}"));
    }
    assert!(PropertyBindingDraft::new("{app:Binding Token=x}", "ui:Binding").is_err());
    let draft = PropertyBindingDraft::new(
        "{ui:Binding RelativeSource={app:RelativeSource Self}, Token=keep}",
        "ui:Binding",
    )
    .unwrap();
    assert!(draft.source_specification().is_none());
    assert_eq!(
        draft.source(),
        "{ui:Binding RelativeSource={app:RelativeSource Self}, Token=keep}"
    );
}
