//! External consumer coverage for composed reactive model inheritance.

use ferrodock_model_reactiveui::{
    core::{DockableBase, DockableBaseImpl},
    impl_dockable,
};
use ferroui_reactiveui::reactive_object;
use std::rc::{Rc, Weak};

// No domain crate/type imports are needed by the exported forwarding macro.
struct UserModel {
    base: DockableBase,
}
reactive_object!(UserModel, base);
impl_dockable!(UserModel);
impl DockableBaseImpl for UserModel {
    fn on_close(_this: &Self) -> bool {
        false
    }
}

#[test]
fn a_consuming_crate_can_compose_and_customize_the_base_model() {
    let model = Rc::new_cyclic(|weak: &Weak<UserModel>| {
        let runtime: Weak<dyn DockableBaseImpl> = weak.clone();
        UserModel {
            base: DockableBase::construct(runtime),
        }
    });
    model.set_title("User model".into());
    assert_eq!(model.title(), "User model");
    assert!(!model.on_close());
    model.set_pinned_bounds(1.0, 2.0, 3.0, 4.0);
    assert_eq!(model.get_pinned_bounds(), (1.0, 2.0, 3.0, 4.0));
}

struct UserDock {
    base: ferrodock_model_reactiveui::core::DockBase,
}
reactive_object!(UserDock, base);
ferrodock_model_reactiveui::impl_dock!(UserDock);
impl DockableBaseImpl for UserDock {}

#[test]
fn a_consuming_crate_can_compose_the_full_dock_base_and_use_its_commands() {
    use ferrodock_model_reactiveui::{controls::Document, core::DockBase};
    let model = Rc::new_cyclic(|weak: &Weak<UserDock>| UserDock {
        base: DockBase::construct(weak.clone()),
    });
    model.base.initialize(&model);
    let document = Document::new();
    document.set_title("Document".into());
    model.navigate().execute(Some(&document.boxed()));
    assert_eq!(model.active_dockable().unwrap().title(), "Document");
    model.navigate().execute(None);
    assert!(model.active_dockable().is_none());
}

struct UserWindow {
    base: ferrodock_model_reactiveui::core::DockWindow,
}
reactive_object!(UserWindow, base);
ferrodock_model_reactiveui::impl_dock_window!(UserWindow);
impl ferrodock_model_reactiveui::core::DockWindowImpl for UserWindow {
    fn on_close(_this: &Self) -> bool {
        false
    }
}

#[test]
fn a_consuming_crate_can_compose_a_window_with_custom_virtual_behavior() {
    use ferrodock_model_reactiveui::core::DockWindow;
    let window = Rc::new_cyclic(|weak: &Weak<UserWindow>| UserWindow {
        base: DockWindow::construct(weak.clone()),
    });
    window.set_title("custom".into());
    assert_eq!(window.title(), "custom");
    assert!(!window.on_close());
    window.save();
    window.exit();
    window.set_active();
}

#[test]
fn adapter_classes_expose_their_source_methods_without_interface_imports() {
    use ferrodock_model::adapters::{HostAdapter, NavigateAdapter};
    use ferrodock_model_reactiveui::{controls::StackDock, core::DockWindow};
    let dock = StackDock::new();
    let navigation = NavigateAdapter::new(dock.clone());
    assert!(!navigation.can_go_back() && !navigation.can_go_forward());
    navigation.go_back();
    navigation.go_forward();
    navigation.navigate(None, true);
    navigation.show_windows();
    navigation.exit_windows();
    navigation.close();
    let window = DockWindow::new();
    let host = HostAdapter::new(window.clone());
    host.save();
    host.present(false);
    host.exit();
    host.set_active();
}
