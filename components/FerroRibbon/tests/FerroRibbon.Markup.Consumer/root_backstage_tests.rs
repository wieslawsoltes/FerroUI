use crate::root_backstage_consumer::RootBackstageConsumer;
use ferroribbon::controls::{Ribbon, RibbonHandledEventArgs};
use ferroui_base::{
    Rect, Size, TypeInfo,
    data::core::{ValueType, ValueTypes},
};
use ferroui_controls::{Control, TextBlock, Window};
use std::rc::Rc;
#[test]
fn independent_root_backstage_compiled_runtime_events_cancellation_args_binding_identity_and_keyboard()
 {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::root_backstage_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[RootBackstageConsumer::TYPE]);
        for runtime in [false, true] {
            let v = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("RootBackstage.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/RootBackstage.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let host = ValueTypes::as_object(&*v)
                .unwrap()
                .cast::<RootBackstageConsumer>()
                .unwrap();
            let r = host.children().get(0).cast::<Ribbon>().unwrap();
            let b = r.backstage().unwrap();
            let page = b.items().get(0).unwrap();
            let content = ValueTypes::as_object(&*page.content().unwrap())
                .unwrap()
                .cast::<TextBlock>()
                .unwrap();
            let args = host
                .resources()
                .try_get_resource(&"Arguments".into(), None)
                .flatten()
                .unwrap();
            let args =
                ValueTypes::try_convert(Some(&args), ValueType::of::<Rc<RibbonHandledEventArgs>>())
                    .flatten()
                    .unwrap();
            let args = args.downcast_ref::<Rc<RibbonHandledEventArgs>>().unwrap();
            assert!(args.handled());
            let text = host.children().get(1).cast::<TextBlock>().unwrap();
            assert_eq!(text.text().as_deref(), Some("True"));
            args.set_handled(false); // Ordinary source property is not notifying: compiled binding keeps its initial value.
            assert_eq!(text.text().as_deref(), Some("True"));
            let w = Window::new();
            w.set_width(900.);
            w.set_height(600.);
            w.set_content(Some(Control::boxed(host.clone())));
            w.show();
            w.measure(Size::new(900., 600.));
            w.arrange(Rect::new(0., 0., 900., 600.));
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            r.application_button().unwrap().on_click();
            assert!(!r.is_backstage_open());
            assert_eq!(host.counts(), (1, 0, 0));
            host.set_cancel(false);
            r.application_button().unwrap().on_click();
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            assert!(r.is_backstage_open());
            assert_eq!(host.counts(), (2, 1, 0));
            assert!(content.is_loaded());
            assert_eq!(b.selected_item(), Some(page));
            r.show_key_tips();
            assert!(r.process_key_tip_input(u16::from(b'C')));
            assert!(!r.is_backstage_open());
            assert_eq!(host.counts(), (2, 1, 1));
            r.invoke_application_button();
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            assert!(content.is_loaded());
            assert_eq!(host.counts(), (3, 2, 1));
            r.suspend_popups();
            assert_eq!(host.counts(), (3, 2, 2));
            w.close();
            w.set_content(None);
        }
    });
}
