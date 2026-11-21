use ferroribbon::controls::{RibbonInputBase, RibbonTextBox};
use ferroui_base::{
    StaticType,
    data::core::{ValueType, ValueTypes},
    input::{InputElement, Key, KeyEventArgs},
    threading::Dispatcher,
};
use ferroui_controls::{Control, StackPanel, TextBlock, TextBox, Window};
use std::rc::Rc;
#[test]
fn downstream_input_subclass_uses_source_culture_then_invariant_number_helper() {
    use ferroui_base::utilities::{CultureInfo, NumberFormatInfo};
    struct Restore(CultureInfo);
    impl Drop for Restore {
        fn drop(&mut self) {
            CultureInfo::set_current_formatting(self.0.clone());
        }
    }
    let _restore = Restore(CultureInfo::current_formatting());
    CultureInfo::set_current_formatting(
        CultureInfo::invariant_culture()
            .with_number_format(NumberFormatInfo::new().with_number_decimal_separator(",")),
    );
    let mut value = 99.;
    assert!(crate::text_input_consumer::ConsumerTextBox::parse_number(
        Some("1,5"),
        &mut value
    ));
    assert_eq!(value, 1.5);
    assert!(crate::text_input_consumer::ConsumerTextBox::parse_number(
        Some("1.25"),
        &mut value
    ));
    assert_eq!(value, 1.25);
    assert!(!crate::text_input_consumer::ConsumerTextBox::parse_number(
        Some("1,2,3"),
        &mut value
    ));
    assert_eq!(value.to_bits(), 0);
}
#[test]
fn independent_text_input_compiled_runtime_bindings_events_enums_and_subclass() {
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        ferroui_base::TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::text_input_consumer",
            "FerroRibbon.Consumer",
        )]);
        ferroui_base::TypeInfo::register_all(&[
            <crate::text_input_consumer::ConsumerTextBox as StaticType>::TYPE,
        ]);
        use ferroui_base::metadata::{MarkupType, MarkupTyped};
        MarkupType::register_all(&[crate::text_input_consumer::InputViewModel::MARKUP]);
        ValueTypes::register_reference::<crate::text_input_consumer::InputViewModel>();
        for runtime in [false, true] {
            let v = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("TextInput.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/TextInput.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*v)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let c = panel.children().get(0).cast::<RibbonTextBox>().unwrap();
            let custom = panel
                .children()
                .get(1)
                .cast::<crate::text_input_consumer::ConsumerTextBox>()
                .unwrap();
            let label_visibility = panel.children().get(2).cast::<TextBlock>().unwrap();
            let vm = ValueTypes::try_convert(
                panel.data_context().as_ref(),
                ValueType::of::<Rc<crate::text_input_consumer::InputViewModel>>(),
            )
            .flatten()
            .unwrap();
            let vm = vm
                .downcast_ref::<Rc<crate::text_input_consumer::InputViewModel>>()
                .unwrap()
                .clone();
            let w = Window::new();
            w.set_content(Some(Control::boxed(panel)));
            w.show();
            w.measure(ferroui_base::Size::new(500., 200.));
            w.arrange(ferroui_base::Rect::new(0., 0., 500., 200.));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(c.input_width(), 140.);
            assert!(
                c.is_enabled(),
                "input ignores null-parameter CanExecute when deciding enabled state"
            );
            assert_eq!(c.text().as_deref(), Some("initial"));
            assert_eq!(label_visibility.text().as_deref(), Some("Visible"));
            assert_eq!(
                custom.label_visibility(),
                ferroribbon::compat::Visibility::Collapsed
            );
            let inner = c
                .get_visual_descendants()
                .find_map(|c| c.cast::<TextBox>())
                .unwrap();
            assert_eq!(
                ferroui_controls::automation::AutomationProperties::get_name(&inner).as_deref(),
                Some("Find")
            );
            inner.set_text(Some("café 😀"));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(vm.text(), "café 😀");
            assert_eq!(custom.text().as_deref(), Some("café 😀"));
            let mut e = KeyEventArgs::new();
            e.key = Key::Enter;
            e.set_routed_event(Some(InputElement::key_down_event()));
            inner.raise_event(&e);
            assert!(e.handled());
            assert_eq!(vm.calls(), vec!["café 😀"]);
            vm.set_text("blocked".into());
            Dispatcher::ui_thread().run_jobs(None);
            assert!(c.is_enabled());
            let mut e = KeyEventArgs::new();
            e.key = Key::Enter;
            e.set_routed_event(Some(InputElement::key_down_event()));
            inner.raise_event(&e);
            assert!(e.handled());
            assert_eq!(vm.calls(), vec!["café 😀"]);
            assert!(
                <RibbonInputBase as StaticType>::TYPE
                    .markup()
                    .unwrap()
                    .constructors
                    .is_empty()
            );
            let copy = custom.create_linked_copy_core().unwrap();
            assert!(!copy.is::<crate::text_input_consumer::ConsumerTextBox>());
            assert!(copy.is::<RibbonTextBox>());
            w.close();
        }
    });
}
