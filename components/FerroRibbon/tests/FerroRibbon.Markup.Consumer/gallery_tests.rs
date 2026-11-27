use ferroribbon::controls::*;
use ferroui_base::{
    Rect, Size, TypeInfo,
    controls::NameScope,
    data::core::{ValueType, ValueTypes},
};
use ferroui_controls::{Control, Window};
use std::rc::Rc;

#[test]
fn independent_gallery_custom_selection_states_and_replacement_templates() {
    crate::tests::run_standalone(|| {
        use ferroui_base::{
            controls::ResourceDictionary,
            media::{Colors, IBrush, SolidColorBrush},
            styling::ControlTheme,
            threading::Dispatcher,
        };
        use ferroui_controls::{Application, Border, Button, automation::AutomationProperties};
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::gallery_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[crate::gallery_consumer::ConsumerGallery::TYPE]);
        let app = Application::current().unwrap();
        let key = "RibbonGalleryItemStyle".into();
        let previous = app.resources().try_get_resource(&key, None);
        let color =
            |brush: Option<Rc<dyn IBrush>>| brush.unwrap().as_solid_color_brush().unwrap().color();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("GallerySelection.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/GallerySelection.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let dictionary = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ResourceDictionary>()
                .unwrap();
            let theme = |name: &str| {
                ValueTypes::as_object(
                    &*dictionary
                        .try_get_resource(&name.into(), None)
                        .flatten()
                        .unwrap(),
                )
                .unwrap()
                .cast::<ControlTheme>()
                .unwrap()
            };
            app.resources()
                .set(key.clone(), Some(Rc::new(theme("NoStates"))));
            let value = load(runtime, false);
            let g = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<RibbonGallery>()
                .unwrap();
            let w = mount(&g.clone().upcast());
            let b = g
                .get_visual_descendants()
                .filter_map(|v| v.cast::<Button>())
                .find(|v| AutomationProperties::get_name(v).as_deref() == Some("Normal"))
                .unwrap();
            g.set_selected_item(Some(Rc::new("normal".to_owned())));
            let selected: Rc<dyn IBrush> = ferroribbon::theming::RibbonTheme::get_brush(
                Some(&b),
                Some("RibbonGalleryItemSelectedBorderBrush"),
            )
            .unwrap()
            .unwrap()
            .into();
            assert_eq!(
                color(b.border_brush()),
                selected.as_solid_color_brush().unwrap().color()
            );
            g.set_selected_item(Some(Rc::new("heading".to_owned())));
            assert_eq!(color(b.border_brush()), Colors::TRANSPARENT);

            b.set_theme(Some(theme("BothStates")));
            b.apply_styling();
            b.apply_template();
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            let red = ferroui_base::media::Color::from_uint32(0xFFCC1122);
            b.set_border_brush(Some(SolidColorBrush::with_color(red).into()));
            g.set_selected_item(Some(Rc::new("normal".to_owned())));
            Dispatcher::ui_thread().run_jobs(None);
            let root = b.visual_children().get(0).cast::<Border>().unwrap();
            assert!(RibbonGallery::get_supports_selected_state(&root));
            assert!(RibbonGallery::get_supports_unselected_state(&root));
            assert_eq!(
                color(b.border_brush()),
                red,
                "declared state owns selection rather than the fallback"
            );
            assert_eq!(color(root.border_brush()).to_uint32(), 0xFF00AA22);
            g.set_selected_item(Some(Rc::new("heading".to_owned())));
            assert_eq!(
                color(b.border_brush()),
                red,
                "present empty Unselected state must not assign transparent"
            );

            b.set_theme(Some(theme("SelectedOnly")));
            b.apply_styling();
            b.apply_template();
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            g.set_selected_item(Some(Rc::new("normal".to_owned())));
            let only = b.visual_children().get(0).cast::<Border>().unwrap();
            assert_eq!(color(only.border_brush()).to_uint32(), 0xFF00AA22);
            g.set_selected_item(Some(Rc::new("heading".to_owned())));
            assert_eq!(color(b.border_brush()), Colors::TRANSPARENT);
            assert_eq!(
                color(only.border_brush()).to_uint32(),
                0xFF00AA22,
                "missing Unselected must retain the template's prior Selected visual"
            );

            b.set_theme(Some(theme("DerivedNoStates")));
            b.apply_styling();
            b.apply_template();
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            g.set_selected_item(Some(Rc::new("normal".to_owned())));
            Dispatcher::ui_thread().run_jobs(None);
            let replacement = b.visual_children().get(0).cast::<Border>().unwrap();
            assert!(!replacement.ptr_eq(&root));
            assert!(
                !RibbonGallery::get_supports_selected_state(&replacement),
                "replacement template must not inherit its base theme template state declarations"
            );
            assert_eq!(
                color(b.border_brush()),
                selected.as_solid_color_brush().unwrap().color()
            );
            b.set_border_brush(Some(SolidColorBrush::with_color(red).into()));
            b.set_theme(Some(theme("UnselectedOnly")));
            b.apply_styling();
            b.apply_template();
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            let fresh = b.visual_children().get(0).cast::<Border>().unwrap();
            assert!(!fresh.pseudo_classes().contains(":ribbon-gallery-selected"));
            assert!(
                !fresh
                    .pseudo_classes()
                    .contains(":ribbon-gallery-unselected")
            );
            assert_eq!(
                color(fresh.border_brush()),
                red,
                "new template starts without a visual state even when the control data signal is selected"
            );
            g.set_selected_item(Some(Rc::new("heading".to_owned())));
            assert_eq!(color(fresh.border_brush()).to_uint32(), 0xFFFFAA00);
            g.set_selected_item(Some(Rc::new("normal".to_owned())));
            assert_eq!(
                color(fresh.border_brush()).to_uint32(),
                0xFFFFAA00,
                "missing Selected retains prior Unselected visual"
            );
            w.close();
            w.set_content(None);
        }
        if let Some(value) = previous {
            app.resources().set(key, value);
        } else {
            app.resources().remove(&key);
        }
    });
}

#[cfg(target_os = "macos")]
#[test]
fn independent_compiled_runtime_direct_and_model_popups_filter_and_invoke_once() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::independent_compiled_runtime_direct_and_model_popups_filter_and_invoke_once"
    )) {
        return;
    }
    crate::tests::run_standalone(|| {
        use ferroribbon::primitives::RibbonUniformGridPanel;
        use ferroribbon_core::model::*;
        use ferroui_base::{
            input::{Key, PhysicalKey, RawInputModifiers},
            metadata::MarkupDelegate,
            threading::Dispatcher,
        };
        use ferroui_controls::{Button, StackPanel, TextBox, TopLevel};
        use ferroui_headless::HeadlessWindowExtensions;
        use std::cell::Cell;
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::gallery_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[crate::gallery_consumer::ConsumerGallery::TYPE]);
        for runtime in [false, true] {
            for generated in [false, true] {
                let value = load(runtime, generated);
                let mut model = None;
                let mut bindings = None;
                let (host, gallery) = if generated {
                    let root = ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<StackPanel>()
                        .unwrap();
                    let value = root
                        .resources()
                        .try_get_resource(&"Model".into(), None)
                        .flatten()
                        .unwrap();
                    let converted = ValueTypes::try_cast(
                        &value,
                        ValueType::of::<Rc<dyn RibbonGalleryModelImpl>>(),
                    )
                    .unwrap();
                    let source = converted
                        .downcast_ref::<Rc<dyn RibbonGalleryModelImpl>>()
                        .unwrap()
                        .clone();
                    let owner = ferroribbon::mvvm::RibbonModelBindings::new();
                    let gallery = ferroribbon::mvvm::RibbonElementFactory::new()
                        .create_item(Some(source.clone()), owner.clone())
                        .unwrap()
                        .cast::<RibbonGallery>()
                        .unwrap();
                    root.children().add(gallery.clone());
                    model = Some(source);
                    bindings = Some(owner);
                    (root.upcast::<Control>(), gallery)
                } else {
                    let gallery = ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<RibbonGallery>()
                        .unwrap();
                    (gallery.clone().upcast::<Control>(), gallery)
                };
                let w = mount(&host);
                let count = Rc::new(Cell::new(0));
                let out = count.clone();
                let handler = MarkupDelegate::new(move |_| {
                    out.set(out.get() + 1);
                    None
                });
                gallery.add_item_click(Some(handler.clone()));
                gallery.open_drop_down();
                Dispatcher::ui_thread().run_jobs(None);
                assert!(gallery.is_drop_down_open());
                let focused = w.focus_manager().get_focused_element().unwrap();
                let top = TopLevel::get_top_level(Some(&focused)).unwrap();
                let filter = top
                    .get_visual_descendants()
                    .find_map(|v| v.cast::<TextBox>())
                    .unwrap();
                let root = filter
                    .visual_parent()
                    .unwrap()
                    .cast::<StackPanel>()
                    .unwrap();
                for (query, expected) in [("hEaD", 1), ("absent", 0), ("", 2), ("hEaD", 1)] {
                    filter.set_text(Some(query));
                    Dispatcher::ui_thread().run_jobs(None);
                    let buttons = root
                        .get_visual_descendants()
                        .filter_map(|v| v.cast::<RibbonUniformGridPanel>())
                        .flat_map(|p| p.children().to_vec())
                        .collect::<Vec<_>>();
                    assert_eq!(
                        buttons.len(),
                        expected,
                        "runtime={runtime} generated={generated} query={query}"
                    );
                }
                let button = root
                    .get_visual_descendants()
                    .find_map(|v| v.cast::<RibbonUniformGridPanel>())
                    .unwrap()
                    .children()
                    .get(0)
                    .cast::<Button>()
                    .unwrap();
                assert!(button.focus());
                let input = TopLevel::get_top_level(Some(&button)).unwrap();
                input.key_press(
                    Key::Space,
                    RawInputModifiers::NONE,
                    PhysicalKey::Space,
                    None,
                );
                if gallery.is_drop_down_open() {
                    input.key_release(
                        Key::Space,
                        RawInputModifiers::NONE,
                        PhysicalKey::Space,
                        None,
                    );
                } else {
                    w.key_release(
                        Key::Space,
                        RawInputModifiers::NONE,
                        PhysicalKey::Space,
                        None,
                    );
                }
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(count.get(), 1);
                assert!(!gallery.is_drop_down_open());
                if let Some(model) = model {
                    assert_eq!(
                        model
                            .selected_item()
                            .unwrap()
                            .id()
                            .unwrap()
                            .to_utf8()
                            .unwrap(),
                        "heading"
                    );
                } else {
                    assert_eq!(
                        gallery
                            .selected_item()
                            .unwrap()
                            .downcast_ref::<String>()
                            .unwrap(),
                        "heading"
                    );
                }
                gallery.remove_item_click(Some(handler));
                if let Some(bindings) = bindings {
                    bindings.dispose();
                }
                w.close();
                w.set_content(None);
                Dispatcher::ui_thread().run_jobs(None);
            }
        }
    });
}
fn load(runtime: bool, model: bool) -> ferroui_base::BoxedValue {
    if runtime {
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            if model {
                include_str!("GalleryModel.xaml")
            } else {
                include_str!("Gallery.xaml")
            },
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap()
    } else {
        crate::compiled_xaml::try_load(
            None,
            if model {
                "ferres://ferroribbon_markup_consumer/GalleryModel.xaml"
            } else {
                "ferres://ferroribbon_markup_consumer/Gallery.xaml"
            },
        )
        .unwrap()
        .unwrap()
    }
}
fn mount(control: &ferroui_base::Ref<Control>) -> ferroui_base::Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(control.clone())));
    window.show();
    window.measure(Size::new(900., 600.));
    window.arrange(Rect::new(0., 0., 900., 600.));
    window.update_layout();
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    window
}
#[test]
fn independent_compiled_runtime_gallery_items_names_subclass_layout_popup_and_copy() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::gallery_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[crate::gallery_consumer::ConsumerGallery::TYPE]);
        for runtime in [false, true] {
            let value = load(runtime, false);
            let gallery = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<crate::gallery_consumer::ConsumerGallery>()
                .unwrap();
            let scope = NameScope::get_name_scope(&gallery).unwrap();
            let first = scope
                .find("Normal")
                .unwrap()
                .cast::<RibbonGalleryItem>()
                .unwrap();
            assert_eq!(first.preview_font_size(), 17.5);
            assert_eq!(gallery.items().count(), 2);
            assert_eq!(gallery.footer_items().count(), 1);
            assert!(
                ValueTypes::as_object(&*gallery.effective_items().get(0).unwrap())
                    .unwrap()
                    .ptr_eq(&first)
            );
            let w = mount(&gallery.clone().upcast());
            let before = gallery.layout_calls();
            gallery.apply_layout(RibbonItemLayout::new(
                ferroribbon_core::RibbonItemSize::Large,
                gallery.metrics(),
            ));
            assert_eq!(gallery.columns(), 5);
            assert!(gallery.layout_calls() > before);
            gallery.apply_layout(RibbonItemLayout::new(
                ferroribbon_core::RibbonItemSize::Medium,
                gallery.metrics(),
            ));
            assert_eq!(gallery.columns(), 2);
            gallery.pick(Some(Rc::new(first.clone())));
            assert_eq!(
                gallery
                    .selected_item()
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap(),
                "normal"
            );
            gallery.open_drop_down();
            assert!(gallery.is_drop_down_open());
            gallery.close_drop_down();
            let public = ValueTypes::try_cast(
                &Control::boxed(gallery.clone()),
                ValueType::of::<Rc<dyn IRibbonItem>>(),
            )
            .unwrap();
            let copy = public
                .downcast_ref::<Rc<dyn IRibbonItem>>()
                .unwrap()
                .create_linked_copy()
                .unwrap()
                .cast::<RibbonGallery>()
                .unwrap();
            assert!(copy.is_drop_down_only());
            copy.set_selected_item(Some(Rc::new("heading".to_owned())));
            assert_eq!(
                gallery
                    .selected_item()
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap(),
                "heading"
            );
            RibbonItemHelper::unlink(&copy);
            w.close();
            w.set_content(None);
        }
    });
}
#[test]
fn independent_compiled_runtime_gallery_model_metadata_factory_selection_and_collections() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        use ferroribbon_core::model::*;
        for runtime in [false, true] {
            let value = load(runtime, true);
            let root = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ferroui_controls::StackPanel>()
                .unwrap();
            let value = root
                .resources()
                .try_get_resource(&"Model".into(), None)
                .flatten()
                .unwrap();
            let model =
                ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn RibbonGalleryModelImpl>>())
                    .unwrap();
            let model = model
                .downcast_ref::<Rc<dyn RibbonGalleryModelImpl>>()
                .unwrap()
                .clone();
            assert_eq!(model.items().count(), 2);
            assert_eq!(model.rows(), 2);
            let core_item = RibbonGalleryItemModel::new(None, Some("Core item".into()));
            core_item.set_content(Some(RibbonObject::value(
                ferroribbon_core::RibbonString::from("Core preview 🦀"),
            )));
            model.items().add(Some(core_item));
            let factory = ferroribbon::mvvm::RibbonElementFactory::new();
            let bindings = ferroribbon::mvvm::RibbonModelBindings::new();
            let gallery = factory
                .create_item(Some(model.clone()), bindings.clone())
                .unwrap()
                .cast::<RibbonGallery>()
                .unwrap();
            assert_eq!(gallery.item_height(), 60.);
            assert_eq!(gallery.max_columns(), 5);
            assert_eq!(gallery.footer_items().count(), 1);
            root.children().add(gallery.clone());
            let window = mount(&root.clone().upcast());
            for expected in ["AaBb", "Core preview 🦀"] {
                assert!(
                    gallery.get_visual_descendants().any(|v| {
                        v.cast::<ferroui_controls::TextBlock>()
                            .is_some_and(|t| t.text().as_deref() == Some(expected))
                    }),
                    "compiled/runtime model preview: {expected}"
                );
            }
            assert_eq!(
                root.children()
                    .get(0)
                    .cast::<ferroui_controls::TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("3")
            );
            let x = model.items().get(0).unwrap();
            gallery.pick(Some(Rc::new(x.clone())));
            assert!(Rc::ptr_eq(&model.selected_item().unwrap(), &x));
            model.set_selected_item(None);
            assert!(gallery.selected_item().is_none());
            model.set_rows(3);
            assert_eq!(gallery.rows(), 3);
            assert_eq!(
                root.children()
                    .get(1)
                    .cast::<ferroui_controls::TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("3")
            );

            bindings.dispose();
            model.set_rows(4);
            assert_eq!(gallery.rows(), 3);
            assert_eq!(
                root.children()
                    .get(1)
                    .cast::<ferroui_controls::TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("4")
            );
            window.close();
            window.set_content(None);
        }
    });
}
