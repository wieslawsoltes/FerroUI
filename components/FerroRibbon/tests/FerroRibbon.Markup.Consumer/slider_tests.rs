use ferroribbon::{compat::RibbonInputSlider, controls::RibbonSlider};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    input::{InputElement, Key, KeyEventArgs},
    threading::Dispatcher,
};
use ferroui_controls::{Control, StackPanel, Window, templates::IControlTemplate};
use std::rc::Rc;
#[test]
fn independent_slider_compiled_runtime_two_way_commands_template_and_subclass() {
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        ferroui_base::TypeInfo::register_namespaces(&[
            (
                "ferroribbon_markup_consumer::slider_consumer",
                "FerroRibbon.Consumer",
            ),
            (
                "ferroribbon_markup_consumer::spinner_consumer",
                "FerroRibbon.Consumer",
            ),
        ]);
        ferroui_base::TypeInfo::register_all(&[crate::slider_consumer::ConsumerSlider::TYPE]);
        use ferroui_base::metadata::{MarkupType, MarkupTyped};
        MarkupType::register_all(&[crate::spinner_consumer::NumberViewModel::MARKUP]);
        ValueTypes::register_reference::<crate::spinner_consumer::NumberViewModel>();
        for (name, expected) in [
            (
                "RibbonInputSlider",
                ferroribbon::compat::RibbonInputSlider::TYPE,
            ),
            (
                "RibbonInputTextBox",
                ferroribbon::compat::RibbonInputTextBox::TYPE,
            ),
            (
                "RibbonInputRepeatButton",
                ferroribbon::compat::RibbonInputRepeatButton::TYPE,
            ),
        ] {
            let markup = format!("<i:{name} xmlns:i=\"using:FerroRibbon.Internal\" />");
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                &markup,
                Some(&crate::CONSUMER_ASSEMBLY),
            )
            .unwrap();
            assert_eq!(ValueTypes::as_object(&*value).unwrap().get_type(), expected);
        }
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Slider.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Slider.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let thumb = panel
                .children()
                .get(3)
                .cast::<ferroui_controls::primitives::Thumb>()
                .unwrap();
            assert_eq!(
                thumb.drag_start_policy(),
                ferroui_controls::primitives::ThumbDragStartPolicy::PrimaryPressWhenIdle
            );
            let c = panel
                .children()
                .get(0)
                .cast::<crate::slider_consumer::ConsumerSlider>()
                .unwrap();
            let vm = ValueTypes::try_convert(
                panel.data_context().as_ref(),
                ValueType::of::<Rc<crate::spinner_consumer::NumberViewModel>>(),
            )
            .flatten()
            .unwrap();
            let vm = vm
                .downcast_ref::<Rc<crate::spinner_consumer::NumberViewModel>>()
                .unwrap()
                .clone();
            let w = Window::new();
            w.set_content(Some(Control::boxed(panel.clone())));
            w.show();
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            assert!(!c.shows_label_by_default());
            let part = c
                .get_visual_descendants()
                .find_map(|v| v.cast::<RibbonInputSlider>())
                .unwrap();
            assert_eq!(part.step_frequency(), 2.);
            assert_eq!(c.value(), 4.);
            assert_eq!(part.value(), 4.);
            vm.set_value(6.);
            assert_eq!(c.value(), 6.);
            assert_eq!(part.value(), 6.);
            assert!(vm.calls().is_empty());
            let mut key = KeyEventArgs::new();
            key.key = Key::Right;
            key.set_routed_event(Some(InputElement::key_down_event()));
            part.raise_event(&key);
            assert!(key.handled());
            assert_eq!(c.value(), 7.);
            assert_eq!(vm.value(), 7.);
            assert_eq!(vm.calls(), vec![7.]);
            c.set_numeric_value(25.);
            assert_eq!(c.value(), 10.);
            assert_eq!(vm.value(), 25.);
            assert_eq!(vm.calls(), vec![7.]);
            vm.set_value(6.);
            let replacement = panel
                .resources()
                .try_get_resource(&"Replacement".into(), None)
                .flatten()
                .unwrap();
            let replacement = ValueTypes::try_convert(
                Some(&replacement),
                ValueType::of::<Rc<dyn IControlTemplate>>(),
            )
            .flatten()
            .unwrap();
            let replacement = replacement
                .downcast_ref::<Rc<dyn IControlTemplate>>()
                .unwrap()
                .clone();
            c.set_template(Some(replacement));
            c.apply_template();
            w.update_layout();
            let new_part = c
                .get_visual_descendants()
                .find_map(|v| v.cast::<RibbonInputSlider>())
                .unwrap();
            assert!(!new_part.ptr_eq(&part));
            assert_eq!(new_part.value(), 6.);
            part.set_numeric_value(9.);
            assert_eq!(c.value(), 6.);
            assert_eq!(vm.calls(), vec![7.]);
            new_part.set_numeric_value(8.);
            assert_eq!(c.value(), 8.);
            assert_eq!(vm.value(), 8.);
            assert_eq!(vm.calls(), vec![7., 8.]);
            let copy = c.create_linked_copy_core().unwrap();
            assert!(copy.is::<RibbonSlider>());
            assert!(!copy.is::<crate::slider_consumer::ConsumerSlider>());
            w.close();
        }
    });
}
