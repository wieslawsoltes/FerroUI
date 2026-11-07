use crate::*;
use ferroboard_core::settings::{KeyboardSettings, ThemeMode};
use ferroui_base::metadata::{
    MarkupType, MarkupTypeKind, MarkupTyped, from_markup_value, into_markup_value,
};
use ferroui_build_scan::model::{AssemblyModel, TypeKind};
use std::rc::Rc;
#[test]
fn dependency_model_and_runtime_have_identical_settings_and_enum_facts() {
    register_types();
    let model = AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferroboard_core_ferro.xamlmeta"
    )))
    .unwrap();
    assert_eq!(model.name, "FerroBoard.Core");
    let settings = model
        .types
        .iter()
        .find(|t| t.name == "KeyboardSettings")
        .unwrap();
    let runtime = KeyboardSettingsMetadata::MARKUP;
    let mut actual = runtime
        .properties
        .iter()
        .map(|p| p.name)
        .collect::<Vec<_>>();
    actual.sort();
    let mut expected = settings
        .properties
        .iter()
        .map(|p| p.name.as_str())
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(actual.len(), 64);
    assert_eq!(actual, expected);
    let methods = KeyboardSettingsMetadata::MARKUP
        .methods
        .iter()
        .map(|m| m.name)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        methods,
        std::collections::BTreeSet::from(["FromJson", "ToJson", "CopyFrom", "Clone"])
    );
    let runtime = MarkupType::registered_types();
    let enums = model
        .types
        .iter()
        .filter(|t| t.kind == TypeKind::Enum)
        .collect::<Vec<_>>();
    assert_eq!(enums.len(), 22);
    for t in enums {
        let r = runtime
            .iter()
            .find(|r| {
                r.kind == MarkupTypeKind::Enum && r.name == t.name && r.namespace() == t.namespace
            })
            .unwrap();
        let expected = t
            .enum_members
            .iter()
            .map(|m| (m.name.as_str(), m.value.unwrap()))
            .collect::<Vec<_>>();
        let actual = r
            .enum_members
            .iter()
            .map(|m| (m.name, m.value))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
        for m in r.enum_members {
            let value = m.get(r).unwrap();
            assert_eq!((r.enum_to_value.unwrap())(value.as_ref()), Some(m.value));
            let roundtrip = (r.enum_from_value.unwrap())(m.value).unwrap();
            assert_eq!(
                (r.enum_to_value.unwrap())(roundtrip.as_ref()),
                Some(m.value)
            );
        }
    }
}
#[test]
fn every_settings_accessor_roundtrips_the_original_object_and_native_errors_survive() {
    register_types();
    let original = Rc::new(KeyboardSettings::new());
    let before = original.to_json().unwrap();
    let value = into_markup_value(original.clone());
    assert!(Rc::ptr_eq(
        &from_markup_value::<Rc<KeyboardSettings>>(&value).unwrap(),
        &original
    ));
    for p in KeyboardSettingsMetadata::MARKUP.properties {
        let v = (p.get.unwrap())(std::slice::from_ref(&value)).unwrap();
        (p.set.unwrap())(&[value.clone(), v]).unwrap();
    }
    assert_eq!(original.to_json().unwrap(), before);
    let clone = KeyboardSettingsMetadata::MARKUP
        .find_methods("Clone")
        .next()
        .unwrap();
    let cloned = (clone.invoke)(std::slice::from_ref(&value)).unwrap();
    let cloned = from_markup_value::<Rc<KeyboardSettings>>(&cloned).unwrap();
    assert!(!Rc::ptr_eq(&original, &cloned));
    assert_eq!(cloned.to_json().unwrap(), before);

    let p = KeyboardSettingsMetadata::MARKUP
        .find_property("ThemeMode")
        .unwrap();
    (p.set.unwrap())(&[value, into_markup_value(ThemeMode(i32::MIN))]).unwrap();
    assert_eq!(original.theme_mode().0, i32::MIN);
    let parse = KeyboardSettingsMetadata::MARKUP
        .find_methods("FromJson")
        .next()
        .unwrap();
    assert!((parse.invoke)(&[into_markup_value("{".to_owned())]).is_err());
}

#[test]
fn component_name_struct_facade_uses_the_represented_nullable_value_type() {
    use ferroboard_core::layouts::ComponentName;
    use ferroui_base::data::core::ValueType;
    register_types();
    let descriptor = ComponentNameMetadata::MARKUP;
    assert_eq!(descriptor.kind, MarkupTypeKind::Struct);
    assert_eq!(descriptor.handles.len(), 1);
    assert_eq!(descriptor.handle(), Some(ValueType::of::<ComponentName>()));
    assert!(MarkupType::find_by_handle(std::any::TypeId::of::<Option<ComponentName>>()).is_none());
    assert!(std::ptr::eq(
        MarkupType::find_by_nullable_handle(std::any::TypeId::of::<Option<ComponentName>>())
            .unwrap(),
        descriptor
    ));
    assert_eq!(
        (descriptor.nullable.unwrap())(),
        ValueType::of::<Option<ComponentName>>()
    );
    let model = AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferroboard_core_ferro.xamlmeta"
    )))
    .unwrap();
    assert_eq!(
        model
            .types
            .iter()
            .find(|t| t.name == "ComponentName")
            .unwrap()
            .kind,
        TypeKind::Struct
    );
    let original = ComponentName::new_nullable(None, Some("id".into()));
    let boxed = into_markup_value(Some(original.clone()));
    assert_eq!(
        from_markup_value::<Option<ComponentName>>(&boxed).unwrap(),
        Some(original)
    );
    assert!(
        from_markup_value::<Option<ComponentName>>(&into_markup_value(None::<ComponentName>))
            .unwrap()
            .is_none()
    );
}

#[test]
fn key_factory_positional_overloads_match_dependency_model() {
    register_types();
    let methods = TextKeyDataMetadata::MARKUP
        .find_methods("Function")
        .map(|m| m.parameters.len())
        .collect::<Vec<_>>();
    assert_eq!(methods, [1, 2, 3]);
    let model = AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferroboard_core_ferro.xamlmeta"
    )))
    .unwrap();
    let key = model
        .types
        .iter()
        .find(|t| t.name == "TextKeyData")
        .unwrap();
    let exported = key
        .methods
        .iter()
        .filter(|m| m.name == "Function")
        .map(|m| m.parameters.len())
        .collect::<Vec<_>>();
    assert_eq!(methods, exported);
}
