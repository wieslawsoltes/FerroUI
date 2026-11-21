use crate::zoom_consumer::{ConsumerZoom, ZoomView};
use ferroribbon::{
    compat::RibbonInputSlider,
    controls::{IRibbonItem, RibbonZoomControl},
};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    input::{InputElement, Key, KeyEventArgs},
    interactivity::RoutedEventArgs,
    metadata::{MarkupType, MarkupTyped},
    threading::Dispatcher,
};
use ferroui_controls::{Button, Control, TextBlock, Window, templates::IControlTemplate};
use std::rc::Rc;
fn button(c: &RibbonZoomControl, name: &str) -> ferroui_base::Ref<Button> {
    c.get_visual_descendants()
        .find_map(|v| {
            v.cast::<Button>()
                .filter(|v| v.name().as_deref() == Some(name))
        })
        .unwrap()
}
#[test]
fn independent_zoom_compiled_runtime_two_way_commands_events_templates_and_subclass() {
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        ferroui_base::TypeInfo::register_namespaces(&[
            (
                "ferroribbon_markup_consumer::zoom_consumer",
                "FerroRibbon.Consumer",
            ),
            (
                "ferroribbon_markup_consumer::spinner_consumer",
                "FerroRibbon.Consumer",
            ),
        ]);
        ferroui_base::TypeInfo::register_all(&[ConsumerZoom::TYPE, ZoomView::TYPE]);
        MarkupType::register_all(&[crate::spinner_consumer::NumberViewModel::MARKUP]);
        ValueTypes::register_reference::<crate::spinner_consumer::NumberViewModel>();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Zoom.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Zoom.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ZoomView>()
                .unwrap();
            let c = panel.children().get(0).cast::<ConsumerZoom>().unwrap();
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
            assert_eq!(c.value(), 4.);
            assert!(!c.can_add_to_quick_access());
            assert_eq!(
                panel
                    .children()
                    .get(1)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("4%")
            );
            panel.reset();
            vm.set_value(30.);
            assert_eq!(c.value(), 30.);
            assert!(vm.calls().is_empty());
            button(&c, "PART_ZoomIn")
                .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
            assert_eq!(c.value(), 40.);
            assert_eq!(vm.value(), 40.);
            assert_eq!(vm.calls(), [40.]);
            assert_eq!(panel.changes(), [30., 40.]);
            let part = c
                .get_visual_descendants()
                .find_map(|v| v.cast::<RibbonInputSlider>())
                .unwrap();
            let mut key = KeyEventArgs::new();
            key.key = Key::Right;
            key.set_routed_event(Some(InputElement::key_down_event()));
            part.raise_event(&key);
            assert_eq!(c.value(), 41.);
            assert_eq!(vm.value(), 41.);
            assert_eq!(vm.calls(), [40., 41.]);
            let boxed = Control::boxed(c.clone());
            let item =
                ValueTypes::try_convert(Some(&boxed), ValueType::of::<Rc<dyn IRibbonItem>>())
                    .flatten()
                    .unwrap();
            let item = item.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap();
            assert!(item.invoke());
            assert_eq!(c.invocations(), 1);
            assert_eq!(panel.dialogs(), 1);
            assert_eq!(vm.calls(), [40., 41.]);
            let old_button = button(&c, "PART_ZoomIn");
            let template = panel
                .resources()
                .try_get_resource(&"Replacement".into(), None)
                .flatten()
                .unwrap();
            let template = ValueTypes::try_convert(
                Some(&template),
                ValueType::of::<Rc<dyn IControlTemplate>>(),
            )
            .flatten()
            .unwrap();
            c.set_template(Some(
                template
                    .downcast_ref::<Rc<dyn IControlTemplate>>()
                    .unwrap()
                    .clone(),
            ));
            c.apply_template();
            old_button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
            part.set_numeric_value(80.);
            assert_eq!(c.value(), 41.);
            button(&c, "PART_ZoomIn")
                .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
            assert_eq!(c.value(), 50.);
            assert_eq!(vm.calls(), [40., 41., 50.]);
            button(&c, "PART_Percent")
                .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
            assert_eq!(panel.dialogs(), 2);
            let weak_view = panel.downgrade();
            w.set_content(None);
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            w.close();
            drop((panel, value));
            assert!(
                weak_view.upgrade().is_none(),
                "XAML handlers must not retain their view"
            );
        }
    });
}
