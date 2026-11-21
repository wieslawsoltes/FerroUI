use ferroribbon::controls::{RibbonSpinner, RibbonValueChangedEventArgs};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    input::{InputElement, Key, KeyEventArgs},
    threading::Dispatcher,
};
use ferroui_controls::{Control, StackPanel, TextBox, Window};
use std::rc::Rc;
#[test]
fn independent_spinner_compiled_runtime_two_way_binding_commands_and_subclass() {
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        ferroui_base::TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::spinner_consumer",
            "FerroRibbon.Consumer",
        )]);
        ferroui_base::TypeInfo::register_all(&[crate::spinner_consumer::ConsumerSpinner::TYPE]);
        use ferroui_base::metadata::{MarkupType, MarkupTyped};
        MarkupType::register_all(&[crate::spinner_consumer::NumberViewModel::MARKUP]);
        ValueTypes::register_reference::<crate::spinner_consumer::NumberViewModel>();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Spinner.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Spinner.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let c = panel
                .children()
                .get(0)
                .cast::<crate::spinner_consumer::ConsumerSpinner>()
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
            let label = panel
                .children()
                .get(3)
                .cast::<ferroui_controls::TextBlock>()
                .unwrap();
            assert_eq!(label.text().as_deref(), Some("1.5"));
            let args = ValueTypes::try_convert(
                label.data_context().as_ref(),
                ValueType::of::<Rc<RibbonValueChangedEventArgs>>(),
            )
            .flatten()
            .unwrap();
            let args = args
                .downcast_ref::<Rc<RibbonValueChangedEventArgs>>()
                .unwrap();
            assert_eq!(args.old_value(), 1.5);
            assert_eq!(args.new_value(), 2.5);
            let w = Window::new();
            w.set_content(Some(Control::boxed(panel.clone())));
            w.show();
            w.measure(ferroui_base::Size::new(500., 220.));
            w.arrange(ferroui_base::Rect::new(0., 0., 500., 220.));
            Dispatcher::ui_thread().run_jobs(None);
            let p = c
                .get_visual_descendants()
                .find_map(|p| p.cast::<TextBox>())
                .unwrap();
            assert_eq!(p.text().as_deref(), Some("4.0 cm"));
            assert!(!c.shows_label_by_default());
            vm.set_value(6.);
            assert_eq!(c.value(), 6., "runtime={runtime}, model={}", vm.value());
            assert!(vm.calls().is_empty());
            p.set_text(Some("7.5 cm"));
            let mut e = KeyEventArgs::new();
            e.key = Key::Enter;
            e.set_routed_event(Some(InputElement::key_down_event()));
            p.raise_event(&e);
            assert!(e.handled());
            assert_eq!(vm.value(), 7.5);
            assert_eq!(vm.calls(), vec![7.5]);
            c.set_value(RibbonSpinner::value_property(), 25.);
            assert_eq!(c.value(), 10.);
            // Pinned Uno dispatches the outer change value after the nested
            // coercion. The source is 25 while the actual control remains 10.
            assert_eq!(vm.value(), 25.);
            assert_eq!(vm.calls(), vec![7.5]);
            vm.set_value(6.);
            assert_eq!(c.value(), 6.);
            let copy = c.create_linked_copy_core().unwrap();
            assert!(copy.is::<RibbonSpinner>());
            assert!(!copy.is::<crate::spinner_consumer::ConsumerSpinner>());
            w.close();
        }
    })
}
