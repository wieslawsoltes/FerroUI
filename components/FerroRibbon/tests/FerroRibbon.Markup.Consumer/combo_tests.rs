use ferroribbon::controls::{RibbonComboBox, RibbonFontComboBox, RibbonFontSizeComboBox};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    input::{InputElement, Key, KeyEventArgs},
    threading::Dispatcher,
};
use ferroui_controls::{Control, StackPanel, TextBlock, TextBox, Window};
use std::rc::Rc;
#[test]
fn independent_combo_arrow_peer_names_follow_labels_and_preserve_custom_templates() {
    use ferroui_controls::{Button, automation::peers::ControlAutomationPeer};
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ComboAutomation.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ComboAutomation.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let window = Window::new();
            window.set_width(500.);
            window.set_height(260.);
            window.set_content(Some(Control::boxed(&panel)));
            window.show();
            Dispatcher::ui_thread().run_jobs(None);
            for (index, expected) in ["Choose", "Font", "Font Size", "Downstream options"]
                .iter()
                .enumerate()
            {
                let combo = panel
                    .children()
                    .get(index)
                    .cast::<RibbonComboBox>()
                    .unwrap();
                let arrow = combo
                    .get_visual_descendants()
                    .filter_map(|c| c.cast::<Button>())
                    .find(|c| c.name().as_deref() == Some("PART_DropDownButton"))
                    .unwrap();
                let peer = ControlAutomationPeer::create_peer_for_element(&arrow);
                assert_eq!(
                    peer.get_name(),
                    *expected,
                    "runtime={runtime}, index={index}"
                );
                for label in ["خط A\u{301} 😀", "", "字体"] {
                    combo.set_label(Some(label.into()));
                    if index != 3 {
                        assert_eq!(
                            ferroui_controls::automation::AutomationProperties::get_name(&arrow)
                                .as_deref(),
                            Some(label)
                        );
                    }
                    if !label.is_empty() || index == 3 {
                        assert_eq!(
                            peer.get_name(),
                            if index == 3 {
                                "Downstream options"
                            } else {
                                label
                            }
                        );
                    }
                }
                combo.set_label(None);
                if index == 3 {
                    assert_eq!(peer.get_name(), "Downstream options");
                } else {
                    assert_eq!(
                        ferroui_controls::automation::AutomationProperties::get_name(&arrow),
                        None
                    );
                    ferroui_controls::automation::AutomationProperties::set_name(
                        &arrow,
                        Some("Local options"),
                    );
                    combo.set_label(Some("Changed under local name".into()));
                    assert_eq!(peer.get_name(), "Local options");
                    arrow.clear_value(
                        ferroui_controls::automation::AutomationProperties::name_property(),
                    );
                    assert_eq!(peer.get_name(), "Changed under local name");
                }
            }
            window.close();
            window.set_content(None);
        }
    });
}
#[test]
fn independent_combo_compiled_runtime_content_binding_font_defaults_and_copy_override() {
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        ferroui_base::TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::combo_consumer",
            "FerroRibbon.Consumer",
        )]);
        ferroui_base::TypeInfo::register_all(&[crate::combo_consumer::ConsumerCombo::TYPE]);
        use ferroui_base::metadata::{MarkupType, MarkupTyped};
        MarkupType::register_all(&[crate::text_input_consumer::InputViewModel::MARKUP]);
        ValueTypes::register_reference::<crate::text_input_consumer::InputViewModel>();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Combo.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Combo.xaml",
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
                .cast::<crate::combo_consumer::ConsumerCombo>()
                .unwrap();
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
            w.set_content(Some(Control::boxed(panel.clone())));
            w.show();
            w.measure(ferroui_base::Size::new(500., 220.));
            w.arrange(ferroui_base::Rect::new(0., 0., 500., 220.));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(c.items().count(), 2);
            assert_eq!(
                panel
                    .children()
                    .get(3)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("2")
            );
            assert_eq!(
                panel
                    .children()
                    .get(4)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("30")
            );
            assert!(panel.children().get(1).is::<RibbonFontComboBox>());
            assert!(panel.children().get(2).is::<RibbonFontSizeComboBox>());
            assert_eq!(
                panel
                    .children()
                    .get(5)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("Aptos")
            );
            assert!(c.item_template().is_some());
            assert!(c.selection_box_template().is_some());
            let inner = c
                .get_visual_descendants()
                .find_map(|p| p.cast::<TextBox>())
                .unwrap();
            assert_eq!(inner.text().as_deref(), Some("initial"));
            inner.set_text(Some("second"));
            assert_eq!(vm.text(), "initial", "editing is not a source commit");
            let mut e = KeyEventArgs::new();
            e.key = Key::Enter;
            e.set_routed_event(Some(InputElement::key_down_event()));
            inner.raise_event(&e);
            assert!(e.handled());
            assert_eq!(vm.text(), "second");
            assert_eq!(vm.calls(), vec!["second"]);
            assert_eq!(
                c.selected_item()
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap(),
                "second"
            );
            let copy = c.create_linked_copy_core().unwrap();
            assert!(copy.is::<crate::combo_consumer::ConsumerCombo>());
            let copy = copy.cast::<RibbonComboBox>().unwrap();
            assert_eq!(copy.effective_items().len(), 2);
            copy.commit(Some(Rc::new("first".to_string())));
            assert_eq!(vm.text(), "first");
            assert_eq!(vm.calls(), vec!["second", "first"]);
            c.set_is_editable(false);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(inner.opacity(), 0.);
            let selected = c
                .get_visual_descendants()
                .find_map(|p| {
                    let p = p.cast::<ferroui_controls::presenters::ContentPresenter>()?;
                    (p.name().as_deref() == Some("PART_SelectionBox")).then_some(p)
                })
                .unwrap();
            assert!(selected.is_visible());
            assert_eq!(
                selected
                    .content()
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap(),
                "first"
            );
            assert!(selected.get_visual_descendants().any(|p| {
                p.cast::<TextBlock>()
                    .is_some_and(|p| p.text().as_deref() == Some("Selection template"))
            }));
            c.open_drop_down();
            assert!(c.is_drop_down_open());
            c.close_drop_down();
            w.close();
        }
    });
}
