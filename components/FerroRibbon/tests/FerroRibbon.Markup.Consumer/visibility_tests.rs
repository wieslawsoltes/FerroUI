//! Source-style visibility markup through the independently built library.
use ferroribbon::controls::{Ribbon, RibbonAutomationPeer, RibbonButton};
use ferroribbon_core::RibbonVisibilityMode;
use ferroui_base::{
    StaticType,
    data::core::{ValueType, ValueTypes},
};
use ferroui_controls::{
    Control, StackPanel, TextBlock, Window,
    automation::{peers::ControlAutomationPeer, provider::IExpandCollapseProvider},
};
use std::rc::Rc;
#[test]
fn independent_visibility_markup_metadata_popup_identity_and_command_binding() {
    const NAME: &str = "visibility_tests::independent_visibility_markup_metadata_popup_identity_and_command_binding";
    if std::env::var_os("FERRORIBBON_VISIBILITY_CONSUMER_ISOLATED").is_none() {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_VISIBILITY_CONSUMER_ISOLATED", "1")
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
                    include_str!("Visibility.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Visibility.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let root = panel.children().get(0).cast::<Ribbon>().unwrap();
            let label = panel.children().get(1).cast::<TextBlock>().unwrap();
            assert_eq!(label.text().as_deref(), Some("True"));
            let w = Window::new();
            w.set_content(Some(Control::boxed(panel)));
            w.show();
            w.measure(ferroui_base::Size::new(800., 300.));
            w.arrange(ferroui_base::Rect::new(0., 0., 800., 300.));
            let vm = ValueTypes::try_convert(
                root.data_context().as_ref(),
                ValueType::of::<Rc<crate::command_consumer::CommandViewModel>>(),
            )
            .flatten()
            .unwrap()
            .downcast_ref::<Rc<crate::command_consumer::CommandViewModel>>()
            .unwrap()
            .clone();
            let button = root
                .tabs()
                .get(0)
                .unwrap()
                .groups()
                .get(0)
                .unwrap()
                .items()
                .get(0)
                .unwrap()
                .cast::<RibbonButton>()
                .unwrap();
            let weak = button.downgrade();
            let info = <Ribbon as StaticType>::TYPE.markup().unwrap();
            (info
                .find_methods("OpenMinimizedPopup")
                .next()
                .unwrap()
                .invoke)(&[Some(Control::boxed(root.clone()))])
            .unwrap();
            assert!(root.is_minimized_popup_open());
            assert!(button.invoke_core());
            assert_eq!(vm.executed(), 1);
            assert!(!root.is_minimized_popup_open());
            assert!(weak.upgrade().unwrap().ptr_eq(&button));
            let peer = ControlAutomationPeer::create_peer_for_element(&root);
            let expand = peer.get_provider::<dyn IExpandCollapseProvider>().unwrap();
            assert!(peer.is::<RibbonAutomationPeer>());
            expand.expand().unwrap();
            assert_eq!(root.visibility_mode(), RibbonVisibilityMode::AlwaysShow);
            assert_eq!(label.text().as_deref(), Some("False"));
            root.set_is_collapsible(false);
            assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_none());
            let menu = (info
                .find_methods("BuildMinimizeBehaviorMenu")
                .next()
                .unwrap()
                .invoke)(&[Some(Control::boxed(root.clone()))])
            .unwrap()
            .unwrap();
            assert!(
                ValueTypes::as_object(&*menu)
                    .unwrap()
                    .is::<ferroui_controls::MenuFlyout>()
            );
            w.close();
            w.set_content(None);
        }
    });
}
