use crate::backstage_consumer::{ConsumerBackstage, ConsumerBackstageItem};
use ferroui_base::{
    TypeInfo,
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_controls::{
    Control, TextBlock, Window, automation::peers::ControlAutomationPeer,
    templates::IControlTemplate,
};
use std::rc::Rc;
#[test]
fn independent_backstage_compiled_runtime_bindings_collection_templates_and_subclasses() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::backstage_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[ConsumerBackstage::TYPE, ConsumerBackstageItem::TYPE]);
        MarkupType::register_all(&[crate::text_input_consumer::InputViewModel::MARKUP]);
        ValueTypes::register_reference::<crate::text_input_consumer::InputViewModel>();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Backstage.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Backstage.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let b = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ConsumerBackstage>()
                .unwrap();
            assert_eq!(b.items().count(), 3);
            assert_eq!(b.navigation_pane_width(), 241.5);
            let page = b.items().get(0).unwrap();
            assert!(page.is::<ConsumerBackstageItem>());
            let content = ValueTypes::as_object(&*page.content().unwrap())
                .unwrap()
                .cast::<TextBlock>()
                .unwrap();
            let model = b
                .resources()
                .try_get_resource(&"Model".into(), None)
                .flatten()
                .unwrap();
            let model = ValueTypes::try_convert(
                Some(&model),
                ferroui_base::data::core::ValueType::of::<
                    Rc<crate::text_input_consumer::InputViewModel>,
                >(),
            )
            .flatten()
            .unwrap();
            let vm = model
                .downcast_ref::<Rc<crate::text_input_consumer::InputViewModel>>()
                .unwrap()
                .clone();
            assert_eq!(page.header().as_deref(), Some("initial"));
            page.set_header(Some("from-item".into()));
            assert_eq!(vm.text(), "from-item");
            vm.set_text("from-model".into());
            assert_eq!(page.header().as_deref(), Some("from-model"));
            let w = Window::new();
            w.set_content(Some(Control::boxed(b.clone())));
            w.set_width(800.);
            w.set_height(500.);
            w.show();
            w.measure(ferroui_base::Size::new(800., 500.));
            w.arrange(ferroui_base::Rect::new(0., 0., 800., 500.));
            w.update_layout();
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            assert!(b.apply_count() > 0);
            assert!(content.is_loaded());
            assert_eq!(b.page_title().as_deref(), Some("from-model"));
            let title = b
                .get_visual_descendants()
                .filter_map(|visual| visual.cast::<TextBlock>())
                .find(|text| text.name().as_deref() == Some("PART_PageTitle"))
                .unwrap();
            let title_peer = ControlAutomationPeer::create_peer_for_element(&title);
            assert!(title_peer.is_control_element());
            assert!(title_peer.is_content_element());
            assert_eq!(title_peer.get_name(), "from-model");
            vm.set_text("updated-page".into());
            assert_eq!(title_peer.get_name(), "updated-page");
            let action = b.items().get(2).unwrap();
            assert!(action.command().is_some());
            action.navigation_button().unwrap().on_click();
            assert_eq!(vm.calls(), vec!["from-backstage".to_owned()]);
            let old = page.navigation_button().unwrap();
            let original = b.template();
            let custom = b
                .resources()
                .try_get_resource(&"Replacement".into(), None)
                .flatten()
                .unwrap();
            let template = ValueTypes::try_cast(
                &custom,
                ferroui_base::data::core::ValueType::of::<Rc<dyn IControlTemplate>>(),
            )
            .unwrap()
            .downcast_ref::<Rc<dyn IControlTemplate>>()
            .unwrap()
            .clone();
            b.set_template(Some(template));
            b.apply_template();
            assert!(b.apply_count() >= 2);
            assert_ne!(page.navigation_button(), Some(old));
            assert_eq!(
                ValueTypes::as_object(&*page.content().unwrap())
                    .unwrap()
                    .cast::<TextBlock>(),
                Some(content)
            );
            b.set_template(original);
            b.apply_template();
            b.items().move_item(2, 0);
            assert_eq!(b.items().get(0), Some(action));
            b.set_selected_item(None);
            b.ensure_selection();
            assert_eq!(b.selected_item(), Some(page));
            w.close();
            w.set_content(None);
        }
    });
}
