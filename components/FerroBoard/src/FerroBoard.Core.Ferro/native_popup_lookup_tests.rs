use crate::popup_dictionary_metadata::*;
use ferroboard_core::{keys::KeyVariation, layouts::*, text::Utf16Text};
use ferroui_base::metadata::{MarkupTyped, from_markup_value, into_markup_value};
use std::rc::Rc;

#[test]
fn lookup_distinguishes_absent_null_and_shared_values_without_out_parameters() {
    crate::register_types();
    let labels = ResourceDictionary::<Utf16Text, Rc<PopupSet>>::default();
    let popup = Rc::new(PopupSet::default());
    labels.set("a".into(), Some(popup.clone()));
    labels.set("null".into(), None);
    let view = PopupLabelsDictionary::from_original(labels.clone().into());
    let method = PopupLabelsDictionaryMetadata::MARKUP
        .find_methods("TryGetValue")
        .next()
        .unwrap();
    assert_eq!(method.parameters.len(), 1);
    for (key, found, has_value) in [
        ("missing", false, false),
        ("null", true, false),
        ("a", true, true),
    ] {
        let result = (method.invoke)(&[
            into_markup_value(view.clone()),
            into_markup_value(Some(Utf16Text::from(key))),
        ])
        .unwrap();
        let result: PopupLabelLookup = from_markup_value(&result).unwrap();
        assert_eq!(result.found(), found);
        assert_eq!(result.value().is_some(), has_value);
        if let Some(value) = result.value() {
            assert!(Rc::ptr_eq(&value, &popup));
            value.set_relevant(None);
            assert!(popup.relevant().is_none());
        }
    }
    assert!(view.lookup(None).is_err());
    assert!(view.get(Some("missing".into())).is_err());
    labels.set("missing".into(), Some(popup.clone()));
    assert!(view.lookup(Some("missing".into())).unwrap().found());
    assert!(Rc::ptr_eq(
        &view
            .lookup(Some("missing".into()))
            .unwrap()
            .value()
            .unwrap(),
        &popup
    ));
}

#[test]
fn variation_lookup_returns_live_nested_mapping_and_reports_null_values() {
    crate::register_types();
    let labels = ResourceDictionary::<Utf16Text, Rc<PopupSet>>::default();
    let variations = ResourceDictionary::<KeyVariation, PopupLabelMappings>::default();
    variations.set(KeyVariation::All, Some(labels.clone().into()));
    let view = PopupVariationsDictionary::from_original(variations.clone().into());
    let method = PopupVariationsDictionaryMetadata::MARKUP
        .find_methods("TryGetValue")
        .next()
        .unwrap();
    assert_eq!(method.parameters.len(), 1);
    let result = (method.invoke)(&[
        into_markup_value(view.clone()),
        into_markup_value(KeyVariation::All),
    ])
    .unwrap();
    let result: PopupVariationLookup = from_markup_value(&result).unwrap();
    assert!(result.found());
    let nested = result.value().unwrap();
    assert!(!nested.lookup(Some("a".into())).unwrap().found());
    let popup = Rc::new(PopupSet::default());
    labels.set("a".into(), Some(popup.clone()));
    assert!(Rc::ptr_eq(
        &nested.lookup(Some("a".into())).unwrap().value().unwrap(),
        &popup
    ));
    variations.set(KeyVariation::All, None);
    let null = view.lookup(KeyVariation::All).unwrap();
    assert!(null.found());
    assert!(null.value().is_none());
    assert!(!view.lookup(KeyVariation(i32::MIN)).unwrap().found());
}

#[test]
fn exported_popup_results_match_runtime_domain_metadata() {
    use ferroui_build_scan::model::AssemblyModel;
    crate::register_types();
    let model = AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferroboard_core_ferro.xamlmeta"
    )))
    .unwrap();
    for descriptor in [
        PopupLabelsDictionaryMetadata::MARKUP,
        PopupVariationsDictionaryMetadata::MARKUP,
    ] {
        let exported = model
            .types
            .iter()
            .find(|t| t.name == descriptor.name && t.namespace == "FerroBoard.Layouts")
            .unwrap();
        let method = exported
            .methods
            .iter()
            .find(|m| m.name == "TryGetValue")
            .unwrap();
        assert_eq!(method.parameters.len(), 1);
    }
    for descriptor in [PopupLabelLookup::MARKUP, PopupVariationLookup::MARKUP] {
        assert_eq!(descriptor.namespace(), "FerroBoard.Layouts");
        let exported = model
            .types
            .iter()
            .find(|t| t.name == descriptor.name)
            .unwrap();
        assert_eq!(
            exported
                .properties
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["Found", "Value"]
        );
    }
}
