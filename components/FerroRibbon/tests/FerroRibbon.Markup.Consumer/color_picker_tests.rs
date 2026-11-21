use ferroribbon::{
    controls::{RibbonColorDialog, RibbonColorPicker},
    mvvm::{RibbonElementFactory, RibbonModelBindings},
};
use ferroribbon_core::{model::*, theming::RibbonColor};
use ferroui_base::data::core::plugins::TaskValue;
use ferroui_base::{
    Rect, Size, TypeInfo,
    data::core::ValueTypes,
    media::Color,
    metadata::{MarkupTyped, from_markup_value, into_markup_value},
};
use ferroui_controls::{Control, StackPanel, Window};
#[test]
fn independent_picker_model_collections_compiled_runtime_subclass_events_factory_and_dialog_api() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::palette_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[
            crate::palette_consumer::PaletteView::TYPE,
            crate::palette_consumer::ConsumerColorPicker::TYPE,
        ]);
        for runtime in [false, true] {
            let v = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ColorPicker.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ColorPicker.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let view = ValueTypes::as_object(&*v)
                .unwrap()
                .cast::<crate::palette_consumer::PaletteView>()
                .unwrap();
            let panel = view.clone().upcast::<StackPanel>();
            let p = panel.children().get(0).cast::<RibbonColorPicker>().unwrap();
            let w = Window::new();
            w.set_content(Some(Control::boxed(view.clone())));
            w.show();
            w.measure(Size::new(650., 650.));
            w.arrange(Rect::new(0., 0., 650., 650.));
            assert_eq!(p.style_key(), RibbonColorPicker::TYPE);
            assert!(std::rc::Rc::ptr_eq(
                &p.tag().unwrap(),
                &RibbonColorPicker::no_color_parameter()
            ));
            assert!(!p.is_split());
            assert!(
                p.get_visual_descendants()
                    .any(|c| c.name().as_deref() == Some("PART_PrimaryButton"))
            );
            p.palette().select(Some(Color::from_rgb(7, 8, 9)));
            assert_eq!(view.selections(), vec![Some(Color::from_rgb(7, 8, 9))]);
            let model = panel
                .resources()
                .try_get_resource(&"ColorModel".into(), None)
                .flatten()
                .unwrap();
            let m = from_markup_value::<std::rc::Rc<dyn RibbonColorPickerModelImpl>>(&Some(model))
                .unwrap();
            assert_eq!(m.theme_colors().count(), 1);
            assert_eq!(m.recent_colors().count(), 1);
            let bindings = RibbonModelBindings::new();
            let factory = RibbonElementFactory::new();
            let picker = factory
                .create_item(Some(m.clone()), bindings.clone())
                .unwrap()
                .cast::<RibbonColorPicker>()
                .unwrap();
            assert_eq!(
                picker.selected_color(),
                Some(Color::from_argb(128, 1, 2, 3))
            );
            assert_eq!(picker.palette().color_names().len(), 1);
            picker
                .palette()
                .select(Some(Color::from_argb(128, 8, 7, 6)));
            assert_eq!(m.selected_color(), Some(RibbonColor::new(128, 8, 7, 6)));
            m.set_selected_color(None);
            assert_eq!(picker.selected_color(), None);
            bindings.dispose();
            m.set_selected_color(Some(RibbonColor::BLACK));
            assert_eq!(picker.selected_color(), None);
            let method = RibbonColorDialog::MARKUP
                .methods
                .iter()
                .find(|m| m.parameters.len() == 2)
                .unwrap();
            let out =
                (method.invoke)(&[None, into_markup_value(Color::from_rgb(1, 2, 3))]).unwrap();
            let task = from_markup_value::<TaskValue>(&out).unwrap();
            assert_eq!(task.result(), Some(Ok(None)));
            w.close();
            w.set_content(None);
        }
    });
}
#[test]
fn independent_dialog_fault_tasks_nullable_root_and_utf16_diagnostic_through_public_metadata() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::independent_dialog_fault_tasks_nullable_root_and_utf16_diagnostic_through_public_metadata"
    )) {
        return;
    }
    crate::tests::run_standalone(|| {
        use ferroribbon::controls::RibbonColorDialogError;
        use ferroribbon_core::{
            RibbonString,
            localization::{RibbonCulture, RibbonStrings},
        };
        let _theme = crate::tests::NativeThemeScope::new();
        let w = Window::new();
        w.show();
        let previous = RibbonStrings::current();
        for input in [
            None,
            Some(RibbonString::from_utf16(vec![0xd800, 46, 46, 46])),
        ] {
            RibbonStrings::register(
                Some("en-US".into()),
                Some(RibbonItemsSource::from_values(vec![(
                    Some("MoreColors".into()),
                    input.clone(),
                )])),
            )
            .unwrap();
            RibbonStrings::set_current(Some(
                RibbonStrings::for_culture(Some(RibbonCulture::new("en-US".into(), "en".into())))
                    .unwrap(),
            ))
            .unwrap();
            let expected =
                input
                    .as_ref()
                    .map_or(RibbonColorDialogError::NullLocalizedTitle, |_| {
                        RibbonColorDialogError::UnsupportedUtf16 {
                            property: "ContentDialog.Title",
                            units: vec![0xd800],
                        }
                    });
            let mut direct = RibbonColorDialog::show_async(
                Some(w.clone().upcast()),
                Color::from_rgb(1, 2, 3),
                None,
            );
            use std::task::{Context, Poll, Waker};
            assert_eq!(
                direct
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop())),
                Poll::Ready(Err(expected.clone()))
            );
            for method in RibbonColorDialog::MARKUP.methods {
                let mut args = vec![
                    into_markup_value(Some(w.clone().upcast::<ferroui_controls::TopLevel>())),
                    into_markup_value(Color::from_rgb(1, 2, 3)),
                ];
                if method.parameters.len() == 3 {
                    args.push(None);
                }
                let out = (method.invoke)(&args).unwrap();
                let task = from_markup_value::<TaskValue>(&out).unwrap();
                let error = task.result().unwrap().unwrap_err();
                assert_eq!(
                    error.inner().downcast_ref::<RibbonColorDialogError>(),
                    Some(&expected)
                );
                args[0] = None;
                let out = (method.invoke)(&args).unwrap();
                let null_task = from_markup_value::<TaskValue>(&out).unwrap();
                assert_eq!(null_task.result(), Some(Ok(None)));
            }
        }
        RibbonStrings::set_current(Some(previous)).unwrap();
        w.close();
    });
}
