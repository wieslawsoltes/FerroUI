use crate::{SuggestionMetadata, register_types};
use ferroboard_core::text::{Suggestion, SuggestionKind, Utf16Text};
use ferroui_base::{
    BoxedValue,
    data::core::ValueType,
    metadata::{
        MarkupAttributeValue, MarkupInvokeError, MarkupTyped, from_markup_value, into_markup_value,
    },
};
use std::rc::Rc;

#[test]
fn exported_candidate_model_matches_native_constructors_and_properties() {
    register_types();
    use ferroui_build_scan::model::{AssemblyModel, AttributeValueModel};
    let model = AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferroboard_core_ferro.xamlmeta"
    )))
    .unwrap();
    let t = model
        .types
        .iter()
        .find(|t| t.name == "Suggestion" && t.namespace == "FerroBoard.Text")
        .unwrap();
    let runtime = SuggestionMetadata::MARKUP;
    assert_eq!(t.constructors.len(), runtime.constructors.len());
    for (ctor, r) in t.constructors.iter().zip(runtime.constructors) {
        assert_eq!(ctor.parameters.len(), r.parameters.len());
        for (i, p) in ctor.parameters.iter().enumerate() {
            assert_eq!(p.name.as_deref(), r.parameter_name(i));
            let attrs = r.parameter_attributes(i);
            assert_eq!(
                p.attributes
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>(),
                attrs.iter().map(|a| a.name).collect::<Vec<_>>()
            );
            for (a, b) in p.attributes.iter().zip(attrs) {
                assert_eq!(a.arguments.len(), b.arguments.len());
                for (a, b) in a.arguments.iter().zip(b.arguments) {
                    match (a, b) {
                        (AttributeValueModel::Float(a), MarkupAttributeValue::Float(b)) => {
                            assert_eq!(a.parse::<f64>().unwrap(), *b)
                        }
                        (AttributeValueModel::Bool(a), MarkupAttributeValue::Bool(b)) => {
                            assert_eq!(a, b)
                        }
                        _ => panic!("unmatched parameter default"),
                    }
                }
            }
        }
    }
    assert_eq!(
        t.properties
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        runtime
            .properties
            .iter()
            .map(|p| p.name)
            .collect::<Vec<_>>()
    );
    assert!(t.methods.is_empty());
    assert!(runtime.methods.is_empty());
    assert!(
        !model
            .types
            .iter()
            .any(|t| t.name == "SuggestionDeclaration" || t.name == "SuggestionMetadata")
    );
    assert_eq!(
        t.public_path.as_deref(),
        Some("::ferroboard_core_ferro::SuggestionMetadata")
    );
}
#[test]
fn candidate_constructors_defaults_properties_and_shared_identity() {
    register_types();
    let m = SuggestionMetadata::MARKUP;
    assert_eq!(
        (m.name, m.explicit_namespace),
        ("Suggestion", Some("FerroBoard.Text"))
    );
    let c = &m.constructors[2];
    assert_eq!(m.constructors.len(), 3);
    let defaults = (m.constructors[0].invoke)(&[]).unwrap();
    let defaults: Rc<Suggestion> = from_markup_value(&defaults).unwrap();
    assert_eq!(
        defaults.values(),
        ferroboard_core::text::SuggestionValues::new_nullable(
            None,
            SuggestionKind::Typed,
            0.0,
            false
        )
    );
    let short = (m.constructors[1].invoke)(&[
        into_markup_value(Some("short".to_owned())),
        into_markup_value(SuggestionKind::Completion),
    ])
    .unwrap();
    let short: Rc<Suggestion> = from_markup_value(&short).unwrap();
    assert_eq!(short.text(), Some("short".into()));
    assert_eq!(short.kind(), SuggestionKind::Completion);
    assert_eq!(short.confidence(), 0.0);
    assert!(!short.is_auto_commit());
    assert_eq!(
        c.parameters.iter().map(|v| v()).collect::<Vec<_>>(),
        vec![
            ValueType::of::<Option<String>>(),
            ValueType::of::<SuggestionKind>(),
            ValueType::of::<f64>(),
            ValueType::of::<bool>()
        ]
    );
    assert_eq!(
        (0..4)
            .map(|i| c.parameter_name(i).unwrap())
            .collect::<Vec<_>>(),
        ["Text", "Kind", "Confidence", "IsAutoCommit"]
    );
    let boxed = (c.invoke)(&[
        None,
        into_markup_value(SuggestionKind::Emoji),
        into_markup_value(-0.0f64),
        into_markup_value(true),
    ])
    .unwrap();
    let original = from_markup_value::<Rc<Suggestion>>(&boxed).unwrap();
    assert!(original.text().is_none());
    assert_eq!(original.confidence().to_bits(), (-0.0f64).to_bits());
    let alias = from_markup_value::<Option<Rc<Suggestion>>>(&boxed)
        .unwrap()
        .unwrap();
    assert!(Rc::ptr_eq(&original, &alias));
    for p in m.properties {
        assert!(p.get.is_some() && p.set.is_some());
        assert!(!p.attributes.iter().any(|a| a.name == "InitOnly"));
    }
    (m.find_property("Description").unwrap().set.unwrap())(&[
        boxed.clone(),
        into_markup_value(Some("name".to_owned())),
    ])
    .unwrap();
    assert_eq!(alias.description(), Some("name".into()));
    let raw: BoxedValue = original.clone();
    assert!(Rc::ptr_eq(
        &from_markup_value::<Rc<Suggestion>>(&Some(raw)).unwrap(),
        &original
    ));
    let plain = Suggestion::new("default".into(), SuggestionKind::Typed);
    assert_eq!(plain.confidence(), 0.0);
    assert!(!plain.is_auto_commit());
}
#[test]
fn utf16_string_bridge_reports_unrepresentable_data_and_keeps_the_original_units() {
    register_types();
    let units = Utf16Text::from_units(vec![97, 0xd800, 0xd83d, 0xde00, 0xdfff]);
    let s = Rc::new(Suggestion::new(units.clone(), SuggestionKind::Typed));
    let p = SuggestionMetadata::MARKUP.find_property("Text").unwrap();
    assert!(matches!(
        (p.get.unwrap())(&[into_markup_value(s.clone())]),
        Err(MarkupInvokeError::Failed(_))
    ));
    assert_eq!(s.text(), Some(units));
    (p.set.unwrap())(&[into_markup_value(s.clone()), None]).unwrap();
    assert!(s.text().is_none());
    (p.set.unwrap())(&[
        into_markup_value(s.clone()),
        into_markup_value(Some("café 😀".to_owned())),
    ])
    .unwrap();
    assert_eq!(s.text(), Some("café 😀".into()));
    assert_eq!(
        from_markup_value::<Option<String>>(&(p.get.unwrap())(&[into_markup_value(s)]).unwrap()),
        Some(Some("café 😀".to_owned()))
    );
}
