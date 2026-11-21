//! Public source-style markup and actual routed keyboard command invocation.
use ferroribbon::controls::{Ribbon, RibbonButton};
use ferroui_base::{
    Ref, StaticType,
    data::core::{ValueType, ValueTypes},
    input::{InputElement, Key, KeyEventArgs, KeyModifiers},
};
use ferroui_controls::{Control, StackPanel, TextBlock, Window};
use std::rc::Rc;
#[test]
fn independent_shortcut_markup_binding_metadata_and_same_command_once() {
    const NAME: &str =
        "shortcut_tests::independent_shortcut_markup_binding_metadata_and_same_command_once";
    if std::env::var_os("FERRORIBBON_SHORTCUT_CONSUMER_ISOLATED").is_none() {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_SHORTCUT_CONSUMER_ISOLATED", "1")
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        return;
    }
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Shortcuts.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Shortcuts.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let r = panel.children().get(0).cast::<Ribbon>().unwrap();
            let label = panel.children().get(1).cast::<TextBlock>().unwrap();
            assert!(r.is_shortcut_routing_enabled());
            assert_eq!(label.text().as_deref(), Some("True"));
            let vm = ValueTypes::try_convert(
                r.data_context().as_ref(),
                ValueType::of::<Rc<crate::command_consumer::CommandViewModel>>(),
            )
            .flatten()
            .unwrap()
            .downcast_ref::<Rc<crate::command_consumer::CommandViewModel>>()
            .unwrap()
            .clone();
            let w = Window::new();
            w.set_content(Some(Control::boxed(panel)));
            w.show();
            w.measure(ferroui_base::Size::new(800., 300.));
            w.arrange(ferroui_base::Rect::new(0., 0., 800., 300.));
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            assert!(r.is_loaded());
            let send = |key| {
                let mut e = KeyEventArgs::new();
                e.key = key;
                e.key_modifiers = KeyModifiers::CONTROL;
                e.set_routed_event(Some(InputElement::key_down_event()));
                r.raise_event(&e);
                e.handled()
            };
            assert!(r.select_tab_id("Other"));
            assert!(send(Key::B));
            assert_eq!(vm.executed(), 1);
            let info = <Ribbon as StaticType>::TYPE.markup().unwrap();
            (info
                .find_methods("InvalidateShortcuts")
                .next()
                .unwrap()
                .invoke)(&[Some(Control::boxed(r.clone()))])
            .unwrap();
            r.set_is_shortcut_routing_enabled(false);
            assert_eq!(label.text().as_deref(), Some("False"));
            assert!(!send(Key::B));
            assert_eq!(vm.executed(), 1);
            r.set_is_shortcut_routing_enabled(true);
            let first: Ref<RibbonButton> = r
                .tabs()
                .get(0)
                .unwrap()
                .groups()
                .get(0)
                .unwrap()
                .items()
                .get(0)
                .unwrap()
                .cast()
                .unwrap();
            first.set_shortcut(Some("Ctrl+C".into()));
            assert!(send(Key::C));
            assert_eq!(vm.executed(), 2);
            w.close();
            w.set_content(None);
        }
    });
}
