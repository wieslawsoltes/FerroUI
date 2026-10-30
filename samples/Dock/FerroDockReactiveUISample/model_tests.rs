//! Source sample layout, context identity, commands, navigation and reactive inheritance.
use crate::{
    models::{documents::DemoDocument, tools::*, DemoData},
    view_models::{
        documents::DocumentViewModel,
        tools::*,
        views::{DashboardViewModel, HomeViewModel},
        DockFactory, MainWindowViewModel,
    },
};
use ferrodock_model::{controls::*, core::*, BoxedValue};
use ferroui_base::threading::CancellationToken;
use ferroui_controls::{AppBuilder, Application};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::{cell::RefCell, rc::Rc};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "original-reactiveui-sample-models",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(f, CancellationToken::none())
        .wait()
        .unwrap();
}
fn find(root: Rc<dyn IDockable>, id: &str) -> Option<Rc<dyn IDockable>> {
    if root.id() == id {
        return Some(root);
    }
    if let Some(children) = root.as_dock().and_then(|dock| dock.visible_dockables()) {
        for item in children.iter() {
            if let Some(found) = find(item.expect("nonnull sample layout item"), id) {
                return Some(found);
            }
        }
    }
    None
}
#[test]
fn original_factory_builds_the_complete_source_layout_and_permissions() {
    run(|| {
        let factory = DockFactory::new(DemoData::new());
        let root = factory.create_layout().unwrap();
        assert!(!root.is_collapsable());
        assert_eq!(root.active_dockable().unwrap().id(), "Dashboard");
        assert_eq!(root.default_dockable().unwrap().id(), "Home");
        assert_eq!(root.visible_dockables().unwrap().count(), 2);
        for list in [
            root.left_pinned_dockables(),
            root.right_pinned_dockables(),
            root.top_pinned_dockables(),
            root.bottom_pinned_dockables(),
        ] {
            assert_eq!(list.unwrap().count(), 0);
        }
        assert!(root.pinned_dock().is_none());
        let home = find(root.clone(), "Home").unwrap().as_root_dock().unwrap();
        let main = home
            .active_dockable()
            .unwrap()
            .as_proportional_dock()
            .unwrap();
        assert_eq!(main.orientation(), Orientation::Horizontal);
        let children = main.visible_dockables().unwrap();
        assert_eq!(children.count(), 5);
        for index in [1, 3] {
            assert!(children
                .get(index)
                .and_then(|item| item.as_proportional_dock_splitter())
                .is_some());
        }
        let left = children
            .get(0)
            .and_then(|item| item.as_proportional_dock())
            .unwrap();
        let right = children
            .get(4)
            .and_then(|item| item.as_proportional_dock())
            .unwrap();
        for dock in [&left, &right] {
            assert_eq!(dock.proportion(), 0.25);
            assert_eq!(dock.orientation(), Orientation::Vertical);
            assert_eq!(dock.visible_dockables().unwrap().count(), 3);
            assert!(dock.active_dockable().is_none());
        }
        let groups = [
            left.visible_dockables()
                .unwrap()
                .get(0)
                .and_then(|item| item.as_tool_dock())
                .unwrap(),
            left.visible_dockables()
                .unwrap()
                .get(2)
                .and_then(|item| item.as_tool_dock())
                .unwrap(),
            right
                .visible_dockables()
                .unwrap()
                .get(0)
                .and_then(|item| item.as_tool_dock())
                .unwrap(),
            right
                .visible_dockables()
                .unwrap()
                .get(2)
                .and_then(|item| item.as_tool_dock())
                .unwrap(),
        ];
        for (dock, alignment) in groups.iter().zip([
            Alignment::Left,
            Alignment::Bottom,
            Alignment::Top,
            Alignment::Right,
        ]) {
            assert_eq!(dock.alignment(), alignment);
            assert_eq!(dock.visible_dockables().unwrap().count(), 2);
        }
        assert!(!groups[1].can_drag());
        assert!(!groups[1].can_drop());
        assert_eq!(groups[2].grip_mode(), GripMode::Hidden);
        assert_eq!(groups[3].grip_mode(), GripMode::AutoHide);
        for i in 1..=8 {
            let tool = find(root.clone(), &format!("Tool{i}")).unwrap();
            assert_eq!(tool.runtime_type_name(), format!("Tool{i}ViewModel"));
            assert_eq!(tool.title(), format!("Tool{i}"));
            if i <= 2 {
                assert!(tool.keep_pinned_dockable_visible());
            }
            if i == 3 || i == 4 {
                assert!(!tool.can_drag());
            }
        }
        let seven = find(root.clone(), "Tool7").unwrap();
        assert!(!seven.can_close() && !seven.can_pin());
        let eight = find(root.clone(), "Tool8").unwrap();
        assert!(!eight.can_close() && eight.can_pin());
        let six = find(root.clone(), "Tool6").unwrap();
        assert!(six.can_close() && six.can_pin());
        let docs = children
            .get(2)
            .and_then(|item| item.as_document_dock())
            .unwrap();
        assert_eq!(docs.runtime_type_name(), "CustomDocumentDock");
        assert!(!docs.is_collapsable());
        assert!(docs.can_create_document() && docs.enable_window_drag());
        assert_eq!(docs.active_dockable().unwrap().id(), "Document1");
        assert_eq!(docs.visible_dockables().unwrap().count(), 3);
    });
}
#[test]
fn source_contexts_are_fresh_objects_and_locators_retain_live_factory_fields() {
    run(|| {
        let context = DemoData::new();
        let factory = DockFactory::new(context.clone());
        let root = factory.create_layout().unwrap();
        factory.init_layout(root.clone());
        assert!(Rc::ptr_eq(
            &factory.get_context("Home".into()).unwrap(),
            &(context as BoxedValue)
        ));
        let first = factory.get_context("Document1".into()).unwrap();
        let second = factory.get_context("Document1".into()).unwrap();
        assert!(first.is::<DemoDocument>());
        assert!(!Rc::ptr_eq(&first, &second));
        macro_rules! tool {
            ($id:literal,$type:ty) => {
                assert!(factory.get_context($id.into()).unwrap().is::<$type>());
            };
        }
        tool!("Tool1", Tool1);
        tool!("Tool2", Tool2);
        tool!("Tool3", Tool3);
        tool!("Tool4", Tool4);
        tool!("Tool5", Tool5);
        tool!("Tool6", Tool6);
        tool!("Tool7", Tool7);
        tool!("Tool8", Tool8);
        let dashboard = factory.get_context("Dashboard".into()).unwrap();
        assert!(Rc::ptr_eq(
            dashboard.downcast_ref::<Rc<dyn IDockable>>().unwrap(),
            &(root.clone() as Rc<dyn IDockable>)
        ));
        let old_map = factory.dockable_locator().unwrap();
        let second_root = factory.create_layout().unwrap();
        let located = old_map
            .borrow()
            .get(&"Root".to_owned())
            .unwrap()
            .clone()
            .unwrap()()
        .unwrap();
        assert!(Rc::ptr_eq(
            &located,
            &(second_root.clone() as Rc<dyn IDockable>)
        ));
        assert!(!Rc::ptr_eq(&located, &(root as Rc<dyn IDockable>)));
        assert_eq!(
            old_map
                .borrow()
                .get(&"Documents".to_owned())
                .unwrap()
                .clone()
                .unwrap()()
            .unwrap()
            .runtime_type_name(),
            "CustomDocumentDock"
        );
    });
}
#[test]
fn source_document_command_creates_activates_and_focuses_custom_document_models() {
    run(|| {
        let factory = DockFactory::new(DemoData::new());
        let root = factory.create_layout().unwrap();
        factory.init_layout(root.clone());
        let home = find(root.clone(), "Home").unwrap().as_root_dock().unwrap();
        let document = find(root, "Document1").unwrap();
        let dock = document.owner().unwrap().as_document_dock().unwrap();
        let command = dock.create_document().unwrap();
        dock.set_can_create_document(false);
        command.execute(None);
        assert_eq!(dock.visible_dockables().unwrap().count(), 3);
        dock.set_can_create_document(true);
        command.execute(None);
        assert_eq!(dock.visible_dockables().unwrap().count(), 4);
        let created = dock
            .visible_dockables()
            .unwrap()
            .get(3)
            .expect("created source document");
        assert_eq!(created.id(), "Document4");
        assert_eq!(created.title(), "Document4");
        assert_eq!(created.runtime_type_name(), "DocumentViewModel");
        assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &created));
        assert!(Rc::ptr_eq(&home.focused_dockable().unwrap(), &created));
        assert!(created.factory().is_some());
    });
}
#[test]
fn source_host_locator_reads_the_managed_setting_when_it_is_invoked() {
    run(|| {
        use ferrodock_settings::DockSettings;
        let previous = DockSettings::use_managed_windows();
        let factory = DockFactory::new(DemoData::new());
        factory.init_layout(factory.create_layout().unwrap());
        let create = factory
            .host_window_locator()
            .unwrap()
            .borrow()
            .get(&"IDockWindow".to_owned())
            .unwrap()
            .clone()
            .unwrap();
        DockSettings::set_use_managed_windows(true);
        assert_eq!(create().unwrap().runtime_type_name(), "ManagedHostWindow");
        DockSettings::set_use_managed_windows(false);
        assert_eq!(create().unwrap().runtime_type_name(), "HostWindow");
        DockSettings::set_use_managed_windows(previous);
    });
}
#[test]
fn source_floating_window_factory_uses_custom_document_docks_and_original_demo_title() {
    run(|| {
        let factory = DockFactory::new(DemoData::new());
        let doc = DocumentViewModel::new();
        let window = factory.create_window_from(Some(doc)).unwrap();
        assert_eq!(window.title(), "Dock Ferro Demo");
        let root = window.layout().unwrap();
        let child = root.active_dockable().unwrap();
        assert_eq!(child.runtime_type_name(), "CustomDocumentDock");
    });
}
#[test]
fn main_view_model_startup_and_reset_use_source_default_home_selection() {
    run(|| {
        let model = MainWindowViewModel::new();
        let old = model.layout().unwrap();
        assert_eq!(old.active_dockable().unwrap().id(), "Home");
        assert!(model.global_status().contains("Dockable:"));
        assert!(model.new_layout().can_execute(None));
        model.new_layout().execute(None);
        let next = model.layout().unwrap();
        assert!(!Rc::ptr_eq(&old, &next));
        assert_eq!(next.active_dockable().unwrap().id(), "Home");
        model.close_layout();
        model.set_layout(None);
        model.close_layout();
    });
}
#[test]
fn source_main_status_follows_real_factory_focus_and_setters_notify_only_on_changes() {
    run(|| {
        use ferroui_system::component_model::INotifyPropertyChanged;
        let model = MainWindowViewModel::new();
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        model
            .property_changed()
            .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
        let status = model.global_status();
        model.set_global_status(status);
        assert!(events.borrow().is_empty());
        model.set_global_status("changed".into());
        assert_eq!(&*events.borrow(), &["GlobalStatus"]);
        events.borrow_mut().clear();
        let layout = model.layout().unwrap();
        model.set_layout(Some(layout.clone()));
        assert!(events.borrow().is_empty());
        let doc = find(layout, "Document2").unwrap();
        let owner = doc.owner().unwrap().as_dock().unwrap();
        doc.factory()
            .unwrap()
            .set_focused_dockable(Some(owner), Some(doc));
        assert!(model.global_status().contains("Document2"));
        assert!(events.borrow().iter().any(|name| name == "GlobalStatus"));
    });
}
#[test]
fn source_leaf_inheritance_keeps_outer_reactive_and_interface_identity() {
    run(|| {
        use ferroui_reactive::ObservableExt;
        use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
        let tool = Tool1ViewModel::new();
        let weak = Rc::downgrade(&tool);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = seen.clone();
        let subscription = tool.changed().subscribe_next(move |change| {
            assert!(Rc::ptr_eq(&change.sender, &weak.upgrade().unwrap()));
            log.borrow_mut().push(change.property_name);
        });
        tool.set_title("outer".into());
        tool.set_mdi_state(MdiWindowState::Maximized);
        tool.set_is_open(true);
        assert_eq!(&*seen.borrow(), &["Title", "MdiState", "IsOpen"]);
        let erased: Rc<dyn IDockable> = tool;
        let typed = erased.clone().boxed_reference();
        let typed = typed.downcast_ref::<Rc<Tool1ViewModel>>().unwrap();
        assert!(Rc::ptr_eq(&erased, &(typed.clone() as Rc<dyn IDockable>)));
        subscription.dispose();
        let values: Vec<Rc<dyn IDockable>> = vec![
            DocumentViewModel::new(),
            Tool2ViewModel::new(),
            Tool3ViewModel::new(),
            Tool4ViewModel::new(),
            Tool5ViewModel::new(),
            Tool6ViewModel::new(),
            Tool7ViewModel::new(),
            Tool8ViewModel::new(),
        ];
        for value in values {
            assert!(value.clone().as_document().is_some());
            assert!(value.as_mdi_document().is_some());
        }
        assert!(DashboardViewModel::new().visible_dockables().is_none());
        assert!(HomeViewModel::new().visible_dockables().is_none());
    });
}

#[test]
fn source_create_document_preserves_nullable_count_and_missing_factory_behavior() {
    run(|| {
        use crate::view_models::docks::CustomDocumentDock;
        let dock = CustomDocumentDock::new();
        dock.set_can_create_document(true);
        assert!(dock.visible_dockables().is_none());
        dock.create_document().unwrap().execute(None);
        assert!(dock.visible_dockables().is_none());
        let factory = ferrodock_model_reactiveui::Factory::new();
        dock.set_factory(Some(factory.clone()));
        dock.create_document().unwrap().execute(None);
        let document = dock
            .visible_dockables()
            .unwrap()
            .get(0)
            .expect("created source document");
        assert_eq!(document.id(), "Document");
        assert_eq!(document.title(), "Document");
        dock.create_document().unwrap().execute(None);
        assert_eq!(
            dock.visible_dockables()
                .unwrap()
                .get(1)
                .expect("created source document")
                .id(),
            "Document2"
        );
    });
}

#[repr(C)]
struct DerivedHostWindow {
    base: ferrodock_ferro::controls::HostWindow,
}
use ferrodock_ferro::controls::HostWindow;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, input::InputElementImpl,
    interactivity::InteractiveImpl, layout::LayoutableImpl, FerroObjectImpl, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::{
    primitives::TemplatedControlImpl, ContentControlImpl, ControlImpl, TopLevelImpl,
    WindowBaseImpl, WindowImpl,
};
ferro_class!(DerivedHostWindow:HostWindow);
ferro_class_info!(DerivedHostWindow {});
ferro_impl_classes!(DerivedHostWindow:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,TopLevelImpl,WindowBaseImpl,WindowImpl);
#[test]
fn native_host_status_uses_the_actual_derived_runtime_class() {
    run(|| {
        use ferrodock_ferro::controls::HostWindow;
        let host = ferroui_base::instantiate(DerivedHostWindow {
            base: HostWindow::construct(),
        });
        let handle = HostWindow::as_host_window(host.upcast());
        assert_eq!(handle.runtime_type_name(), "DerivedHostWindow");
    });
}
