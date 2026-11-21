use crate::{
    ASSEMBLY,
    cad::{cad_icon::CadIcon, cad_models::*},
};
use ferroui_base::{
    media::{Color, MediaCollection},
    threading::CancellationToken,
};
use ferroui_controls::{AppBuilder, Application};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
use serde_json::Value;
use std::rc::Rc;
fn source() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/cad-models.json")).unwrap()
}
fn text(value: &Value) -> String {
    String::from_utf16(
        &value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
    .unwrap()
}
fn color(value: Color) -> Value {
    serde_json::json!([value.a, value.r, value.g, value.b])
}
fn layer(value: &CadLayer, row: &Value) {
    assert_eq!(value.name(), text(&row["name"]));
    assert_eq!(value.to_string(), text(&row["text"]));
    assert_eq!(color(value.color()), row["color"]);
    assert_eq!(color(value.color_brush().color()), row["brush"]);
    assert_eq!(value.is_on(), row["on"].as_bool().unwrap());
    assert_eq!(value.is_frozen(), row["frozen"].as_bool().unwrap());
    assert_eq!(value.is_locked(), row["locked"].as_bool().unwrap());
    assert!(value.color_brush().ptr_eq(&value.color_brush()));
    assert!(Rc::ptr_eq(
        &value.on_icon().unwrap(),
        &CadIcon::get(if value.is_on() { "LayerOn" } else { "LayerOff" }).unwrap()
    ));
    assert!(Rc::ptr_eq(
        &value.freeze_icon().unwrap(),
        &CadIcon::get(if value.is_frozen() {
            "LayerFreeze"
        } else {
            "LayerThaw"
        })
        .unwrap()
    ));
    assert!(Rc::ptr_eq(
        &value.lock_icon().unwrap(),
        &CadIcon::get(if value.is_locked() {
            "LayerLock"
        } else {
            "LayerUnlock"
        })
        .unwrap()
    ));
}
fn option(value: &CadColorOption, row: &Value) {
    assert_eq!(value.name(), text(&row["name"]));
    assert_eq!(value.to_string(), text(&row["text"]));
    assert_eq!(color(value.swatch().color()), row["swatch"]);
    assert_eq!(
        value.special_opacity().to_bits(),
        row["opacityBits"].as_u64().unwrap()
    );
    assert!(value.swatch().ptr_eq(&value.swatch()));
}
fn linetype(value: &CadLinetype, row: &Value) {
    assert_eq!(value.name(), text(&row["name"]));
    assert_eq!(value.to_string(), text(&row["text"]));
    assert_eq!(value.description(), text(&row["description"]));
    let bits = value
        .dashes()
        .map(|d| d.iter().map(f64::to_bits).collect::<Vec<_>>());
    let expected = row["dashes"]
        .as_array()
        .map(|a| a.iter().map(|v| v.as_u64().unwrap()).collect::<Vec<_>>());
    assert_eq!(bits, expected);
    if let Some(a) = value.dashes() {
        assert!(a.ptr_eq(&value.dashes().unwrap()));
    }
}
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
#[test]
fn all39_original_cad_models_defaults_flags_and_shared_native_objects() {
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                ferroribbon::register_types();
                let source = source();
                let layers = CadLayer::samples();
                assert_eq!(layers.len(), 11);
                assert!(Rc::ptr_eq(&layers, &CadLayer::samples()));
                for (value, row) in layers.iter().zip(source["layers"].as_array().unwrap()) {
                    layer(value, row);
                }
                let colors = CadColorOption::samples();
                assert_eq!(colors.len(), 10);
                assert!(Rc::ptr_eq(&colors, &CadColorOption::samples()));
                for (value, row) in colors.iter().zip(source["colors"].as_array().unwrap()) {
                    option(value, row);
                }
                let types = CadLinetype::samples();
                assert_eq!(types.len(), 8);
                assert!(Rc::ptr_eq(&types, &CadLinetype::samples()));
                for (value, row) in types.iter().zip(source["linetypes"].as_array().unwrap()) {
                    linetype(value, row);
                }
                let weights = CadLineweight::samples();
                assert_eq!(weights.len(), 10);
                assert!(Rc::ptr_eq(&weights, &CadLineweight::samples()));
                for (value, row) in weights.iter().zip(source["weights"].as_array().unwrap()) {
                    assert_eq!(value.name(), text(&row["name"]));
                    assert_eq!(value.to_string(), text(&row["text"]));
                    assert_eq!(
                        value.thickness().to_bits(),
                        row["thicknessBits"].as_u64().unwrap()
                    );
                }
                for (flags, row) in source["flags"].as_array().unwrap().iter().enumerate() {
                    layer(
                        &CadLayer::with_options(
                            "custom 👩",
                            Color::from_argb(123, 1, 2, 3),
                            flags & 1 != 0,
                            flags & 2 != 0,
                            flags & 4 != 0,
                        ),
                        row,
                    );
                }
                layer(
                    &CadLayer::new("default", Color::from_argb(255, 3, 2, 1)),
                    &source["defaults"],
                );
                option(&CadColorOption::new("none", None), &source["options"][0]);
                option(
                    &CadColorOption::with_special("none-special", None, true),
                    &source["options"][1],
                );
                option(
                    &CadColorOption::with_special(
                        "color-special",
                        Some(Color::from_argb(123, 1, 2, 3)),
                        true,
                    ),
                    &source["options"][2],
                );
                let dashes = MediaCollection::from_items([1., 2.]);
                let model = CadLinetype::new("custom", "description", Some(dashes.clone()));
                dashes.add(3.);
                linetype(&model, &source["sharing"]);
                let line = ferroui_controls::shapes::Line::new();
                line.set_stroke_dash_array(model.dashes());
                assert!(line.stroke_dash_array().unwrap().ptr_eq(&dashes));
                dashes.set(0, 9.);
                assert_eq!(line.stroke_dash_array().unwrap().get(0), 9.);
                let standalone = CadLayer::new("brush", Color::from_argb(255, 1, 2, 3));
                let brush = standalone.color_brush();
                brush.set_color(Color::from_argb(4, 5, 6, 7));
                assert_eq!(
                    standalone.color_brush().color(),
                    Color::from_argb(4, 5, 6, 7)
                );
                assert_eq!(standalone.color(), Color::from_argb(255, 1, 2, 3));
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
fn native_compiled_and_runtime_source_templates_keep_model_and_collection_identity() {
    use ferroribbon::controls::RibbonComboBox;
    use ferroui_base::{
        controls::NameScopeExtensions,
        data::core::{ValueType, ValueTypes},
        metadata::{MarkupTyped, from_markup_value},
    };
    use ferroui_controls::{ItemsSource, StackPanel, TextBlock};
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                ferroribbon::register_types();
                crate::cad::cad_models::register_types();
                ferroui_base::metadata::MarkupAssembly::register(&ASSEMBLY);
                ferroui_base::metadata::MarkupType::register_all(&[
                    crate::cad::cad_art::CadArt::MARKUP,
                    crate::cad::cad_icons::CadIcons::MARKUP,
                    crate::cad::cad_icon::CadIcon::MARKUP,
                ]);
                crate::generated::compiled_markup::register();
                let dictionary = ferroui_markup_xaml::FerroXamlLoader::load(
                    &ferroui_base::utilities::UriRef::parse_absolute(
                        "ferres://FerroRibbon/Themes/Generic.xaml",
                    )
                    .unwrap()
                    .into(),
                    None,
                )
                .unwrap();
                let dictionary = ValueTypes::as_object(&*dictionary)
                    .unwrap()
                    .cast::<ferroui_base::controls::ResourceDictionary>()
                    .unwrap();
                Application::current()
                    .unwrap()
                    .resources()
                    .merged_dictionaries()
                    .add(dictionary.into());
                let compiled = crate::generated::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_sample/CadModels.xaml",
                )
                .unwrap()
                .unwrap();
                let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("CadModels.xaml"),
                    Some(&ASSEMBLY),
                )
                .unwrap();
                for value in [compiled, runtime] {
                    let root = ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<StackPanel>()
                        .unwrap();
                    for (name, count) in [
                        ("Layers", 11),
                        ("Colors", 10),
                        ("Linetypes", 8),
                        ("Weights", 10),
                    ] {
                        let combo =
                            NameScopeExtensions::find::<RibbonComboBox>(&root, name).unwrap();
                        let boxed = combo.items_source().unwrap();
                        let converted =
                            ValueTypes::try_convert(Some(&boxed), ValueType::of::<ItemsSource>())
                                .unwrap()
                                .unwrap();
                        let items = converted.downcast_ref::<ItemsSource>().unwrap();
                        assert_eq!(items.count(), count);
                        let item = items.get_at(0).unwrap();
                        let presenter = ferroui_controls::presenters::ContentPresenter::new();
                        presenter.set_content_template(combo.item_template());
                        presenter.set_content(Some(item.clone()));
                        presenter.update_child();
                        let built = presenter.child().unwrap();
                        assert!(
                            built
                                .get_visual_descendants()
                                .filter_map(|v| v.cast::<TextBlock>())
                                .any(|b| b.text().as_deref()
                                    == Some(if name == "Layers" { "0" } else { "ByLayer" }))
                        );
                        if name == "Layers" {
                            let model = from_markup_value::<Rc<CadLayer>>(&Some(item)).unwrap();
                            assert!(Rc::ptr_eq(&model, &CadLayer::samples()[0]));
                            let meta = CadLayer::MARKUP.find_static_property("Samples").unwrap();
                            let a = (meta.get.unwrap())(&[]).unwrap().unwrap();
                            let b = (meta.get.unwrap())(&[]).unwrap().unwrap();
                            let a = a.downcast_ref::<ItemsSource>().unwrap();
                            let b = b.downcast_ref::<ItemsSource>().unwrap();
                            assert!(a.ptr_eq(b));
                        }
                    }
                }
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
