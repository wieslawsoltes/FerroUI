//! External compiled/runtime source XAML plus constructor/property/interface metadata.
use ferroribbon::controls::*;
use ferroui_base::{StaticType, data::core::ValueTypes};
use ferroui_controls::{
    Control,
    automation::{
        peers::ControlAutomationPeer,
        provider::{IInvokeProvider, ISelectionItemProvider, ISelectionProvider},
    },
};
#[test]
fn independent_header_constructor_arguments_keep_tab_alive_for_base_and_downstream_controls() {
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        for (name, text) in [
            ("HeaderOwner", include_str!("HeaderOwner.xaml")),
            (
                "DerivedHeaderOwner",
                include_str!("DerivedHeaderOwner.xaml"),
            ),
        ] {
            for runtime in [false, true] {
                let (weak_header, weak_tab) = {
                    let value = if runtime {
                        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                            text,
                            Some(&crate::CONSUMER_ASSEMBLY),
                        )
                        .unwrap()
                    } else {
                        crate::compiled_xaml::try_load(
                            None,
                            &format!("ferres://ferroribbon_markup_consumer/{name}.xaml"),
                        )
                        .unwrap()
                        .unwrap()
                    };
                    let header = ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<RibbonTabHeader>()
                        .unwrap();
                    if name == "DerivedHeaderOwner" {
                        assert!(
                            header
                                .cast::<crate::tab_header_consumer::ConsumerTabHeader>()
                                .unwrap()
                                .base_initialized()
                        );
                    }
                    assert_eq!(header.tab().header().as_deref(), Some("Detached"));
                    assert_eq!(header.tab().id().as_deref(), Some("detached"));
                    let info = <RibbonTabHeader as StaticType>::TYPE.markup().unwrap();
                    let args = [Some(Control::boxed(header.clone()))];
                    let tab = (info.find_property("Tab").unwrap().get.unwrap())(&args)
                        .unwrap()
                        .unwrap();
                    let tab = ValueTypes::as_object(&*tab)
                        .unwrap()
                        .cast::<RibbonTab>()
                        .unwrap();
                    assert_eq!(tab, header.tab());
                    (info.find_methods("Select").next().unwrap().invoke)(&args).unwrap();
                    let peer = ControlAutomationPeer::create_peer_for_element(&header);
                    assert_eq!(peer.get_name(), "Detached");
                    assert_eq!(
                        peer.get_automation_id().as_deref(),
                        Some("RibbonTab_detached")
                    );
                    assert_eq!(
                        header.tag().unwrap().downcast_ref::<String>().unwrap(),
                        "Detached"
                    );
                    tab.set_header(Some("Renamed".into()));
                    assert_eq!(
                        header.tag().unwrap().downcast_ref::<String>().unwrap(),
                        "Renamed"
                    );
                    (header.downgrade(), tab.downgrade())
                };
                assert!(weak_header.upgrade().is_none(), "{name}, runtime={runtime}");
                assert!(weak_tab.upgrade().is_none(), "{name}, runtime={runtime}");
            }
        }
    });
}
#[test]
fn independent_source_markup_constructs_and_dispatches_native_selection_peers() {
    const NAME: &str =
        "key_tip_peers::independent_source_markup_constructs_and_dispatches_native_selection_peers";
    if std::env::var_os("FERRORIBBON_KEYTIP_CONSUMER_ISOLATED").is_none() {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_KEYTIP_CONSUMER_ISOLATED", "1")
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
        let custom = PlainPeerRibbon::new();
        let t = RibbonTab::new();
        custom.tabs().add(Some(t.clone()));
        let h = RibbonTabHeader::new(t);
        let hp = RibbonTabHeaderAutomationPeer::new(&h);
        let cp = ControlAutomationPeer::create_peer_for_element(&custom);
        let provider = hp.get_provider::<dyn ISelectionItemProvider>().unwrap();
        assert!(cp.get_provider::<dyn ISelectionProvider>().is_none());
        assert!(provider.selection_container().is_none());
        assert_eq!(provider.selection_container_peer(), Some(cp));
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Ribbon.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Ribbon.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let root = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<Ribbon>()
                .unwrap();
            let w = ferroui_controls::Window::new();
            w.set_content(Some(Control::boxed(root.clone())));
            w.show();
            w.measure(ferroui_base::Size::new(700., 300.));
            w.arrange(ferroui_base::Rect::new(0., 0., 700., 300.));
            let peer = ControlAutomationPeer::create_peer_for_element(&root);
            let selection = peer.get_provider::<dyn ISelectionProvider>().unwrap();
            let tab = root.tabs().get(1).unwrap();
            let header = tab.header_element().unwrap();
            let info = <RibbonTabHeaderAutomationPeer as StaticType>::TYPE
                .markup()
                .unwrap();
            let p = (info.constructors[0].invoke)(&[Some(Control::boxed(header.clone()))])
                .unwrap()
                .unwrap();
            let p = ValueTypes::as_object(&*p)
                .unwrap()
                .cast::<RibbonTabHeaderAutomationPeer>()
                .unwrap();
            let provider = p.get_provider::<dyn ISelectionItemProvider>().unwrap();
            assert_eq!(provider.selection_container_peer(), Some(peer));
            provider.select().unwrap();
            assert_eq!(root.selected_tab(), Some(tab.clone()));
            assert_eq!(
                (info.find_property("IsSelected").unwrap().get.unwrap())(&[Some(
                    std::rc::Rc::new(p.clone())
                )])
                .unwrap()
                .unwrap()
                .downcast_ref::<bool>(),
                Some(&true)
            );
            p.get_provider::<dyn IInvokeProvider>()
                .unwrap()
                .invoke()
                .unwrap();
            // GetSelection uses FromElement, so construct the attached header's cached peer explicitly.
            let attached = ControlAutomationPeer::create_peer_for_element(&header);
            assert_eq!(selection.get_selection(), vec![attached]);
            let group = tab.groups().get(0).unwrap();
            let meta = <RibbonGroupAutomationPeer as StaticType>::TYPE
                .markup()
                .unwrap();
            let gp = (meta.constructors[0].invoke)(&[Some(Control::boxed(group))])
                .unwrap()
                .unwrap();
            assert!(
                ValueTypes::as_object(&*gp)
                    .unwrap()
                    .is::<RibbonGroupAutomationPeer>()
            );
            w.close();
            w.set_content(None);
        }
    });
}

#[repr(C)]
struct PlainPeerRibbon {
    base: Ribbon,
}
ferroui_base::ferro_class!(PlainPeerRibbon:Ribbon);
ferroui_base::ferro_class_info!(PlainPeerRibbon{new:PlainPeerRibbon::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferroui_base::ferro_impl_classes!(PlainPeerRibbon:ferroui_base::FerroObjectImpl,ferroui_base::StyledElementImpl,ferroui_base::VisualImpl,ferroui_base::layout::LayoutableImpl,ferroui_base::interactivity::InteractiveImpl,ferroui_base::input::InputElementImpl,ferroui_controls::primitives::TemplatedControlImpl);
impl PlainPeerRibbon {
    fn new() -> ferroui_base::Ref<Self> {
        ferroui_base::instantiate(Self {
            base: Ribbon::construct(),
        })
    }
}
impl ferroui_controls::ControlImpl for PlainPeerRibbon {
    fn on_create_automation_peer(
        this: &Self,
    ) -> ferroui_base::Ref<ferroui_controls::automation::peers::AutomationPeer> {
        ControlAutomationPeer::new(this).upcast()
    }
}
