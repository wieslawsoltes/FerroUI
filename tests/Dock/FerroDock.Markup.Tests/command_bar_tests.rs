//! The complete original host template across a downstream compiled boundary.
use ferrodock_ferro::{command_bars::DockCommandBarManager, controls::DockCommandBarHost};
use ferrodock_model::{
    collections::ReadOnlyListHandle,
    command_bars::{DockCommandBarDefinition, DockCommandBarItem, DockCommandBarKind},
    core::IDockable,
};
use ferrodock_model_reactiveui::{controls::RootDock, Factory};
use ferrodock_settings::DockSettings;
use ferroui_base::{
    threading::{CancellationToken, Dispatcher},
    Ref,
};
use ferroui_controls::{
    automation::{peers::ControlAutomationPeer, provider::IInvokeProvider},
    AppBuilder, Application, Button, ItemsControl, Panel, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::{any::Any, cell::Cell, rc::Rc};
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
    name: "command-bar-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
struct Command(Rc<Cell<usize>>);
impl ferroui_reactiveui::ICommand for Command {
    fn can_execute(&self, _: Option<&dyn Any>) -> bool {
        true
    }
    fn execute(&self, p: Option<&dyn Any>) {
        assert_eq!(
            p.unwrap().downcast_ref::<String>().unwrap(),
            "original parameter"
        );
        self.0.set(self.0.get() + 1);
    }
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn ferroui_system::IDisposable> {
        ferroui_reactive::disposables::Disposable::empty()
    }
}
struct SettingsGuard(bool);
impl Drop for SettingsGuard {
    fn drop(&mut self) {
        DockSettings::set_command_bar_merging_enabled(self.0);
    }
}
#[test]
fn complete_source_host_template_renders_three_bar_kinds_runs_command_and_refreshes_nullable_lists()
{
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro_themes_fluent::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/CommandBarConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
                let host = panel
                    .children()
                    .get(0)
                    .cast::<DockCommandBarHost>()
                    .unwrap();
                assert!(!host.is_visible());
                let calls = Rc::new(Cell::new(0));
                let mut definitions = Vec::new();
                for kind in [
                    DockCommandBarKind::Menu,
                    DockCommandBarKind::ToolBar,
                    DockCommandBarKind::Ribbon,
                ] {
                    let definition = DockCommandBarDefinition::new(format!("{kind:?}"), kind);
                    let item = DockCommandBarItem::new(Some(Rc::new(format!("{kind:?} action"))));
                    item.set_command(Some(Rc::new(Command(calls.clone()))));
                    item.set_command_parameter(Some(Rc::new("original parameter".to_owned())));
                    definition.set_items(Some(ReadOnlyListHandle::from_items(vec![item])));
                    definitions.push(definition);
                }
                host.set_base_command_bars(Some(ReadOnlyListHandle::from_items(definitions)));
                let root = RootDock::new();
                let factory = Factory::new();
                root.set_factory(Some(factory.clone()));
                let _settings = SettingsGuard(DockSettings::command_bar_merging_enabled());
                DockSettings::set_command_bar_merging_enabled(true);
                let manager = DockCommandBarManager::new(host.clone(), None);
                manager.attach(root.clone());
                assert!(host.is_visible());
                let window = Window::new();
                window.set_width(640.0);
                window.set_height(480.0);
                window.set_content(Some(Rc::new(panel.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                // The three source ItemsControls consume original IReadOnlyList control handles.
                let mut parent = host
                    .find_descendant_of_type::<ItemsControl>(false)
                    .unwrap()
                    .visual_parent()
                    .unwrap();
                while !parent.is::<ferroui_controls::StackPanel>() {
                    parent = parent.visual_parent().unwrap();
                }
                let stack = parent.cast::<ferroui_controls::StackPanel>().unwrap();
                assert_eq!(stack.children().count(), 3);
                for index in 0..3 {
                    let items = stack.children().get(index).cast::<ItemsControl>().unwrap();
                    assert_eq!(items.items().count(), 1);
                }
                let button = host
                    .find_descendant_of_type_where::<Button>(false, |button| {
                        button.content().is_some_and(|v| {
                            v.downcast_ref::<String>()
                                .is_some_and(|v| v == "ToolBar action")
                        })
                    })
                    .unwrap();
                let peer = ControlAutomationPeer::create_peer_for_element(&button);
                peer.get_provider::<dyn IInvokeProvider>()
                    .unwrap()
                    .invoke()
                    .unwrap();
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(calls.get(), 1);
                host.set_base_command_bars(None);
                manager.attach(root);
                window.update_layout();
                assert!(!host.is_visible());
                assert!(host.menu_bars().is_none());
                assert!(host.tool_bars().is_none());
                assert!(host.ribbon_bars().is_none());
                for index in 0..3 {
                    assert_eq!(
                        stack
                            .children()
                            .get(index)
                            .cast::<ItemsControl>()
                            .unwrap()
                            .items()
                            .count(),
                        0
                    );
                }
                manager.detach();
                window.set_content(None);
                window.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
