use ferroribbon::{controls::*, metadata::models::RibbonGridPickerModelMetadata};
use ferroribbon_core::{commands::RibbonRelayCommand, model::*};
use ferroui_base::{
    BoxedValue, Rect, Size, StaticType, TypeInfo,
    data::core::{ValueType, ValueTypes},
    input::{Key, PhysicalKey, RawInputModifiers},
    metadata::MarkupTyped,
    threading::Dispatcher,
};
use ferroui_controls::{Control, Flyout, StackPanel, TopLevel, Window};
use ferroui_headless::HeadlessWindowExtensions;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
fn cast<T: Clone + 'static>(v: &BoxedValue) -> T {
    ValueTypes::try_cast(v, ValueType::of::<T>())
        .unwrap()
        .downcast_ref::<T>()
        .unwrap()
        .clone()
}
#[test]
fn original_grid_model_constructor_arities_and_defaults_match_metadata() {
    crate::tests::run(|| {
        let source: Value =
            serde_json::from_str(include_str!("../fixtures/model-metadata.json")).unwrap();
        let args: Vec<Option<BoxedValue>> = vec![
            Some(Rc::new("id".to_owned())),
            Some(Rc::new("Label".to_owned())),
            None,
        ];
        let mut actual = vec![];
        for ctor in RibbonGridPickerModelMetadata::MARKUP.constructors {
            let arity = ctor.parameters.len();
            let v = (ctor.invoke)(&args[..arity]).unwrap().unwrap();
            let m = cast::<Rc<dyn RibbonGridPickerModelImpl>>(&v);
            let id = m.id().unwrap().to_utf8().unwrap();
            if arity == 0 {
                assert!(id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()));
            }
            actual.push(json!({"arity":arity,"id":if arity==0{"<generated-guid>"}else{&id},"label":m.label().map(|s|s.to_utf8().unwrap()),"size":m.size().raw(),"Rows":m.rows(),"Columns":m.columns(),"command":m.command().is_some()}));
        }
        assert_eq!(json!(actual), source["corpus"]["gridConstructors"]);
    });
}
#[test]
fn compiled_runtime_grid_value_events_subclass_and_model_generated_popup() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::compiled_runtime_grid_value_events_subclass_and_model_generated_popup"
    )) {
        return;
    }
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::grid_picker_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[
            <crate::grid_picker_consumer::ConsumerGridPicker as StaticType>::TYPE,
            <crate::grid_picker_consumer::GridPickView as StaticType>::TYPE,
        ]);
        for runtime in [false, true] {
            let v = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("GridPicker.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/GridPicker.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let host = ValueTypes::as_object(&*v)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let resource = |key: &str| {
                host.resources()
                    .try_get_resource(&key.into(), None)
                    .flatten()
                    .unwrap()
            };
            let model = cast::<Rc<dyn RibbonGridPickerModelImpl>>(&resource("Model"));
            let meta = ferroribbon::metadata::grid_size::RibbonGridSizeMetadata::MARKUP;
            let v = (meta.constructors[1].invoke)(&[Some(Rc::new(2i32)), Some(Rc::new(3i32))])
                .unwrap()
                .unwrap();
            let size = cast::<RibbonGridSize>(&resource("Size"));
            assert_eq!(cast::<RibbonGridSize>(&v), size);
            assert_eq!(size, RibbonGridSize::new(2, 3));
            let r = host.children().get(0).cast::<Ribbon>().unwrap();
            let direct = host
                .children()
                .get(1)
                .cast::<crate::grid_picker_consumer::ConsumerGridPicker>()
                .unwrap();
            let observer = host
                .cast::<crate::grid_picker_consumer::GridPickView>()
                .unwrap();
            let w = Window::new();
            w.set_content(Some(Control::boxed(host.clone())));
            w.show();
            w.measure(Size::new(900., 500.));
            w.arrange(Rect::new(0., 0., 900., 500.));
            Dispatcher::ui_thread().run_jobs(None);
            assert!(direct.get_visual_descendants().count() > 6);
            direct.pick(size);
            assert_eq!(observer.calls(), vec![size]);
            assert_eq!(direct.highlighted_size(), RibbonGridSize::default());
            let button = r
                .tabs()
                .get(0)
                .unwrap()
                .groups()
                .get(0)
                .unwrap()
                .items()
                .get(0)
                .unwrap()
                .cast::<RibbonDropDownButton>()
                .unwrap();
            let flyout = button.flyout().unwrap().cast::<Flyout>().unwrap();
            let picker = Control::from_boxed(&flyout.content().unwrap())
                .unwrap()
                .cast::<RibbonGridPicker>()
                .unwrap();
            assert_eq!((picker.rows(), picker.columns()), (3, 4));
            let calls = Rc::new(RefCell::new(vec![]));
            let out = calls.clone();
            model.set_command(Some(RibbonRelayCommand::new_with_parameter(
                move |p| {
                    out.borrow_mut()
                        .push(*p.unwrap().downcast_ref::<RibbonGridSize>().unwrap())
                },
                None,
            )));
            flyout.show_at(&button);
            assert!(flyout.is_open());
            picker.pick(size);
            assert_eq!(*calls.borrow(), vec![size]);
            assert!(!flyout.is_open());
            let mut expected_calls = vec![size];
            for opening_key in [Key::Space, Key::Enter] {
                for _ in 0..2 {
                    assert!(button.focus());
                    w.key_press(
                        opening_key,
                        RawInputModifiers::NONE,
                        PhysicalKey::None,
                        None,
                    );
                    w.key_release(
                        opening_key,
                        RawInputModifiers::NONE,
                        PhysicalKey::None,
                        None,
                    );
                    Dispatcher::ui_thread().run_jobs(None);
                    assert!(flyout.is_open());
                    assert!(
                        picker.is_focused(),
                        "runtime={runtime}, key={opening_key:?}"
                    );
                    let input = TopLevel::get_top_level(Some(&picker)).unwrap();
                    for key in [Key::Right, Key::Down, Key::Enter] {
                        input.key_press(key, RawInputModifiers::NONE, PhysicalKey::None, None);
                        if flyout.is_open() {
                            input.key_release(
                                key,
                                RawInputModifiers::NONE,
                                PhysicalKey::None,
                                None,
                            );
                        } else {
                            w.key_release(key, RawInputModifiers::NONE, PhysicalKey::None, None);
                        }
                        Dispatcher::ui_thread().run_jobs(None);
                    }
                    expected_calls.push(RibbonGridSize::new(2, 2));
                    assert_eq!(*calls.borrow(), expected_calls, "exact command count");
                    assert!(!flyout.is_open());
                    assert_eq!(picker.highlighted_size(), RibbonGridSize::default());
                    assert!(
                        Control::from_boxed(&flyout.content().unwrap())
                            .unwrap()
                            .ptr_eq(&picker)
                    );
                }
            }
            // Real native data, both public presentations, actual generated popup
            // and the downstream subclass, under each saved source culture.
            let source: Value =
                serde_json::from_str(include_str!("../fixtures/native-numbers.json")).unwrap();
            for row in source["cultures"].as_array().unwrap() {
                let culture = row["name"].as_str().unwrap().to_owned();
                std::thread::spawn(move || {
                    use ferroribbon_core::localization::RibbonStrings;
                    let culture = ferroribbon::localization::RibbonFrameworkCulture::new(
                        ferroui_base::utilities::CultureInfo::get_culture_info(&culture),
                    );
                    RibbonStrings::set_current(Some(
                        RibbonStrings::for_culture(Some(culture)).unwrap(),
                    ))
                    .unwrap();
                })
                .join()
                .unwrap();
                Dispatcher::ui_thread().run_jobs(None);
                flyout.show_at(&button);
                Dispatcher::ui_thread().run_jobs(None);
                assert!(
                    Control::from_boxed(&flyout.content().unwrap())
                        .unwrap()
                        .ptr_eq(&picker)
                );
                direct.highlight(RibbonGridSize::new(1, 2));
                picker.highlight(RibbonGridSize::new(1, 2));
                let expected = String::from_utf16(
                    &row["table"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().unwrap() as u16)
                        .collect::<Vec<_>>(),
                )
                .unwrap();
                for presentation in [
                    direct.upcast_ref::<Control>(),
                    picker.upcast_ref::<Control>(),
                ] {
                    assert!(
                        presentation
                            .get_visual_descendants()
                            .filter_map(|v| v.cast::<ferroui_controls::TextBlock>())
                            .any(|v| v.text().as_deref() == Some(&expected)),
                        "{row}"
                    );
                }
                flyout.hide();
            }
            assert_eq!(*calls.borrow(), expected_calls);

            model.set_rows(0);
            model.set_columns(-5);
            assert_eq!((model.rows(), model.columns()), (1, 1));
            assert_eq!((picker.rows(), picker.columns()), (1, 1));
            r.set_model(None);
            model.set_rows(7);
            assert_eq!(picker.rows(), 1);
            w.close();
        }
    });
}

#[test]
fn source_grid_size_value_constructor_can_be_loaded_from_markup() {
    crate::tests::run(|| {
        let v = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("../fixtures/framework/grid-size-constructor.xaml"),
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .expect("Native value constructor metadata should be usable by markup");
        let root = ValueTypes::as_object(&*v)
            .unwrap()
            .cast::<ferroui_controls::ContentControl>()
            .unwrap();
        assert_eq!(
            *root
                .content()
                .unwrap()
                .downcast_ref::<RibbonGridSize>()
                .unwrap(),
            RibbonGridSize::new(2, 3)
        );
    });
}

#[test]
fn compiled_core_grid_size_roots_return_the_original_value_type() {
    crate::tests::run(|| {
        let provider = || {
            Some(ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers::create_root_service_provider_v3(None))
        };
        let size: RibbonGridSize =
            crate::compiled_xaml::build_gridsizevalue_xaml(provider()).unwrap();
        assert_eq!(size, RibbonGridSize::new(2, 3));
        let zero: RibbonGridSize =
            crate::compiled_xaml::build_gridsizedefault_xaml(provider()).unwrap();
        assert_eq!(zero, RibbonGridSize::default());
        let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("GridSizeValue.xaml"),
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap();
        assert_eq!(*value.downcast_ref::<RibbonGridSize>().unwrap(), size);
    });
}
#[test]
fn runtime_emitter_uses_the_core_value_public_path_for_grid_size() {
    crate::tests::run(|| {
        use ferroui_markup_xaml_loader::rust_emitter::compile_documents;
        let mut configuration = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::new();
        configuration.local_assembly = Some(&crate::CONSUMER_ASSEMBLY);
        let documents = compile_documents(
            &[("GridSizeValue.xaml", include_str!("GridSizeValue.xaml"))],
            None,
            &configuration,
            &[],
        );
        let source = documents[0].source.as_ref().unwrap();
        assert!(
            source.contains("Result<::ferroribbon_core::model::RibbonGridSize,"),
            "{source}"
        );
        assert_eq!(
            source
                .matches("RibbonGridSizeMetadata::__markup_new_1(")
                .count(),
            1,
            "{source}"
        );
        assert!(
            !source.contains("model::ribbon_items::RibbonGridSize"),
            "{source}"
        );
    });
}
