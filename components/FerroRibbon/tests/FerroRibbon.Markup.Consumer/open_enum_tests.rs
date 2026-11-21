//! Independent package consumer: ordinary source enum names retain unnamed values.
use ferroui_base::{TypeInfo, data::core::ValueTypes};
#[test]
fn compiled_runtime_unknown_density_and_live_integer_backed_property() {
    crate::tests::run_standalone(|| {
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::metrics_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[crate::metrics_consumer::MetricsConsumer::TYPE]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("UnknownDensity.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/UnknownDensity.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let view = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<crate::metrics_consumer::MetricsConsumer>()
                .unwrap();
            assert_eq!(view.density().raw(), 99);
            for value in [i32::MIN, i32::MAX, 0, -99] {
                view.set_density(ferroribbon_core::RibbonDensity::from_raw(value));
                assert_eq!(view.density().raw(), value);
            }
        }
    });
}

#[test]
fn native_enum_metadata_numeric_conversion_matches_unchanged_clr_reference() {
    use ferroui_base::metadata::MarkupTyped;
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/open-enum-clr.json")).unwrap();
    let m = ferroribbon::metadata::content_dialog_button::ContentDialogButtonMetadata::MARKUP;
    for case in data["cases"].as_array().unwrap() {
        let value = case["text"].as_str().unwrap().parse::<i64>().unwrap();
        let native = m.enum_from_value.unwrap()(value);
        if let Some(expected) = case["value"].as_i64() {
            assert_eq!(
                m.enum_to_value.unwrap()(&*native.unwrap()),
                Some(expected),
                "{case}"
            );
        } else {
            assert_eq!(case["error"], "OverflowException");
            assert!(
                native.is_none(),
                "untransformed integer conversion rejects overflow: {case}"
            );
        }
    }
}
