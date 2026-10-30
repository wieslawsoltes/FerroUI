// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original converter invalidation paths and overloaded factory methods in downstream XAML.
use ferrodock_model::{
    controls::IRootDock,
    core::{
        DockCapabilityOverrides, DockCapabilityPolicy, DocumentCloseButtonShowMode,
        DocumentLayoutMode, DocumentTabLayout, IDock, IDockCapabilityPolicy, IDockable, IFactory,
    },
};
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock},
    Factory,
};
use ferroui_base::{
    threading::{CancellationToken, Dispatcher},
    Ref,
};
use ferroui_controls::{
    automation::{peers::ControlAutomationPeer, provider::IInvokeProvider},
    AppBuilder, Application, Button, Control, StackPanel, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .after_setup(|_| {
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style())
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "capability-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
struct Scene {
    window: Ref<Window>,
    panel: Ref<StackPanel>,
    root: Rc<dyn IRootDock>,
    dock: Rc<DocumentDock>,
    document: Rc<Document>,
    _factory: Rc<Factory>,
}
impl Scene {
    fn new() -> Self {
        ferrodock_ferro::register_types();
        ferroui_markup_xaml::register_types();
        crate::compiled_markup::register();
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/CapabilityConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap().clone();
        let factory = Factory::new();
        let root = factory.create_layout().unwrap();
        let dock = DocumentDock::new();
        let document = Document::new();
        document.set_id("one".into());
        factory.add_dockable(Some(root.clone()), Some(dock.clone()));
        factory.add_dockable(Some(dock.clone()), Some(document.clone()));
        factory.init_layout(root.clone());
        panel.set_data_context(Some(ferrodock_ferro::model_projection::to_dockable_value(
            document.clone(),
        )));
        let window = Window::new();
        window.set_width(600.0);
        window.set_height(500.0);
        window.set_content(Some(Control::boxed(panel.clone())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
        Self {
            window,
            panel,
            root,
            dock,
            document,
            _factory: factory,
        }
    }
    fn button(&self, name: &str) -> Ref<Button> {
        self.panel
            .find_descendant_of_type_where::<Button>(false, |b| b.name().as_deref() == Some(name))
            .unwrap()
    }
    fn invoke(&self, name: &str) {
        ControlAutomationPeer::create_peer_for_element(&self.button(name))
            .get_provider::<dyn IInvokeProvider>()
            .unwrap()
            .invoke()
            .unwrap();
        Dispatcher::ui_thread().run_jobs(None);
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.window.close()
    }
}
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(f, CancellationToken::none())
        .wait()
        .unwrap()
}
#[test]
fn original_multibinding_invalidates_on_dockable_flag_and_policy_reference_replacement() {
    run(|| {
        let s = Scene::new();
        let b = s.button("Capability");
        assert!(b.is_visible());
        s.document.set_can_close(false);
        assert!(!b.is_visible());
        s.document.set_can_close(true);
        assert!(b.is_visible());
        let allow = DockCapabilityPolicy::new();
        allow.set_can_close(Some(true));
        s.root.set_root_dock_capability_policy(Some(allow));
        assert!(b.is_visible());
        let deny = DockCapabilityPolicy::new();
        deny.set_can_close(Some(false));
        s.root.set_root_dock_capability_policy(Some(deny));
        assert!(!b.is_visible());
        let allow = DockCapabilityPolicy::new();
        allow.set_can_close(Some(true));
        s.dock.set_dock_capability_policy(Some(allow));
        assert!(b.is_visible());
        let deny = DockCapabilityOverrides::new();
        deny.set_can_close(Some(false));
        s.document.set_dock_capability_overrides(Some(deny));
        assert!(!b.is_visible());
        s.document.set_dock_capability_overrides(None);
        assert!(b.is_visible());
        s.dock.set_dock_capability_policy(None);
        assert!(!b.is_visible());
        s.root.set_root_dock_capability_policy(None);
        assert!(b.is_visible());
        let old = s.document.clone();
        let replacement = Document::new();
        replacement.set_can_close(false);
        replacement.set_owner(Some(s.dock.clone()));
        s.panel
            .set_data_context(Some(ferrodock_ferro::model_projection::to_dockable_value(
                replacement.clone(),
            )));
        assert!(!b.is_visible());
        old.set_can_close(false);
        old.set_can_close(true);
        assert!(!b.is_visible());
        replacement.set_can_close(true);
        assert!(b.is_visible());
    });
}
#[test]
fn source_removal_and_close_visibility_converters_follow_live_count_mode_and_active_fields() {
    run(|| {
        let s = Scene::new();
        s.dock.set_can_close_last_dockable(false);
        let b = s.button("Remove");
        assert!(!b.is_visible());
        let list = s.dock.visible_dockables().unwrap();
        let extra = Document::new();
        list.add(Some(extra.clone()));
        assert!(b.is_visible());
        list.remove(&Some((extra as Rc<dyn IDockable>).clone()));
        assert!(!b.is_visible());
        s.dock.set_can_close_last_dockable(true);
        assert!(b.is_visible());
        let b = s.button("CloseVisibility");
        s.dock
            .set_close_button_show_mode(DocumentCloseButtonShowMode::Never);
        assert!(!b.is_visible());
        s.dock
            .set_close_button_show_mode(DocumentCloseButtonShowMode::Active);
        s.panel
            .find_descendant_of_type::<ferrodock_ferro::controls::MdiDocumentWindow>(false)
            .unwrap()
            .set_is_active(false);
        assert!(!b.is_visible());
        s.panel
            .find_descendant_of_type::<ferrodock_ferro::controls::MdiDocumentWindow>(false)
            .unwrap()
            .set_is_active(true);
        assert!(b.is_visible());
        s.dock
            .set_close_button_show_mode(DocumentCloseButtonShowMode::Always);
        s.panel
            .find_descendant_of_type::<ferrodock_ferro::controls::MdiDocumentWindow>(false)
            .unwrap()
            .set_is_active(false);
        assert!(b.is_visible());
    });
}
#[test]
fn original_nested_factory_command_overloads_change_layout_and_close_same_document() {
    run(|| {
        let s = Scene::new();
        assert_eq!(s.dock.layout_mode(), DocumentLayoutMode::Tabbed);
        s.invoke("Mdi");
        assert_eq!(s.dock.layout_mode(), DocumentLayoutMode::Mdi);
        s.invoke("Tabbed");
        assert_eq!(s.dock.layout_mode(), DocumentLayoutMode::Tabbed);
        s.invoke("Left");
        assert_eq!(s.dock.tabs_layout(), DocumentTabLayout::Left);
        s.invoke("Close");
        assert!(s.dock.visible_dockables().is_none_or(|v| v.count() == 0));
    });
}
