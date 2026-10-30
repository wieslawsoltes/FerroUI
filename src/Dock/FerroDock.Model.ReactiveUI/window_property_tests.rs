//! Public window property contracts: default transitions, identity and concrete notifications.
use crate::controls::{Document, RootDock};
use crate::core::DockWindow;
use crate::factory_parameter_tests::RecordingFactory;
use crate::test_host::RecordingHost;
use ferrodock_model::controls::IRootDock;
use ferrodock_model::core::*;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn id_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: String = "changed".to_owned();
    window.set_id(first.clone());
    window.set_id(first.clone());
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.id();
    assert!(value == first);
    let second: String = "next".to_owned();
    erased.set_id(second.clone());
    erased.set_id(second.clone());
    assert!(window.id() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Id");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn x_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: f64 = f64::NAN;
    window.set_x(first);
    window.set_x(first);
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.x();
    assert!(value.is_nan());
    let second: f64 = 42.0;
    erased.set_x(second);
    erased.set_x(second);
    assert!(window.x() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "X");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn y_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: f64 = f64::NAN;
    window.set_y(first);
    window.set_y(first);
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.y();
    assert!(value.is_nan());
    let second: f64 = 42.0;
    erased.set_y(second);
    erased.set_y(second);
    assert!(window.y() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Y");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn width_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: f64 = f64::NAN;
    window.set_width(first);
    window.set_width(first);
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.width();
    assert!(value.is_nan());
    let second: f64 = 42.0;
    erased.set_width(second);
    erased.set_width(second);
    assert!(window.width() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Width");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn height_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: f64 = f64::NAN;
    window.set_height(first);
    window.set_height(first);
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.height();
    assert!(value.is_nan());
    let second: f64 = 42.0;
    erased.set_height(second);
    erased.set_height(second);
    assert!(window.height() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Height");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn window_state_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: DockWindowState = DockWindowState::Maximized;
    window.set_window_state(first);
    window.set_window_state(first);
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.window_state();
    assert!(value == first);
    let second: DockWindowState = DockWindowState::Normal;
    erased.set_window_state(second);
    erased.set_window_state(second);
    assert!(window.window_state() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "WindowState");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn topmost_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = true;
    window.set_topmost(first);
    window.set_topmost(first);
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.topmost();
    assert!(value == first);
    let second: bool = false;
    erased.set_topmost(second);
    erased.set_topmost(second);
    assert!(window.topmost() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Topmost");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn title_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: String = "changed".to_owned();
    window.set_title(first.clone());
    window.set_title(first.clone());
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.title();
    assert!(value == first);
    let second: String = "next".to_owned();
    erased.set_title(second.clone());
    erased.set_title(second.clone());
    assert!(window.title() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Title");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn owner_mode_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: DockWindowOwnerMode = DockWindowOwnerMode::ParentWindow;
    window.set_owner_mode(first);
    window.set_owner_mode(first);
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.owner_mode();
    assert!(value == first);
    let second: DockWindowOwnerMode = DockWindowOwnerMode::Default;
    erased.set_owner_mode(second);
    erased.set_owner_mode(second);
    assert!(window.owner_mode() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "OwnerMode");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn parent_window_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<Rc<dyn IDockWindow>> = Some(DockWindow::new() as Rc<dyn IDockWindow>);
    window.set_parent_window(first.clone());
    window.set_parent_window(first.clone());
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.parent_window();
    assert!(value == first);
    let second: Option<Rc<dyn IDockWindow>> = None;
    erased.set_parent_window(second.clone());
    erased.set_parent_window(second.clone());
    assert!(window.parent_window() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "ParentWindow");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn is_modal_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: bool = true;
    window.set_is_modal(first);
    window.set_is_modal(first);
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.is_modal();
    assert!(value == first);
    let second: bool = false;
    erased.set_is_modal(second);
    erased.set_is_modal(second);
    assert!(window.is_modal() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "IsModal");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn show_in_taskbar_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<bool> = Some(false);
    window.set_show_in_taskbar(first);
    window.set_show_in_taskbar(first);
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.show_in_taskbar();
    assert!(value == first);
    let second: Option<bool> = None;
    erased.set_show_in_taskbar(second);
    erased.set_show_in_taskbar(second);
    assert!(window.show_in_taskbar() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "ShowInTaskbar");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn owner_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<Rc<dyn IDockable>> = Some(Document::new() as Rc<dyn IDockable>);
    window.set_owner(first.clone());
    window.set_owner(first.clone());
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.owner();
    assert!(value == first);
    let second: Option<Rc<dyn IDockable>> = None;
    erased.set_owner(second.clone());
    erased.set_owner(second.clone());
    assert!(window.owner() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Owner");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn factory_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<Rc<dyn IFactory>> =
        Some(Rc::new(RecordingFactory::default()) as Rc<dyn IFactory>);
    window.set_factory(first.clone());
    window.set_factory(first.clone());
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.factory();
    assert!(value == first);
    let second: Option<Rc<dyn IFactory>> = None;
    erased.set_factory(second.clone());
    erased.set_factory(second.clone());
    assert!(window.factory() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Factory");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn layout_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<Rc<dyn IRootDock>> = Some(RootDock::new() as Rc<dyn IRootDock>);
    window.set_layout(first.clone());
    window.set_layout(first.clone());
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.layout();
    assert!(value == first);
    let second: Option<Rc<dyn IRootDock>> = None;
    erased.set_layout(second.clone());
    erased.set_layout(second.clone());
    assert!(window.layout() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Layout");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn host_compares_values_and_notifies_on_the_concrete_window() {
    let window = DockWindow::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let subscription = window
        .changed()
        .subscribe_next(move |change| log.borrow_mut().push(change));
    let first: Option<Rc<dyn IHostWindow>> = Some(RecordingHost::new() as Rc<dyn IHostWindow>);
    window.set_host(first.clone());
    window.set_host(first.clone());
    let erased: Rc<dyn IDockWindow> = window.clone();
    let value = erased.host();
    assert!(value == first);
    let second: Option<Rc<dyn IHostWindow>> = None;
    erased.set_host(second.clone());
    erased.set_host(second.clone());
    assert!(window.host() == second);
    assert_eq!(changes.borrow().len(), 2);
    for change in changes.borrow().iter() {
        assert_eq!(change.property_name, "Host");
        assert!(Rc::ptr_eq(&change.sender, &window));
    }
    changes.borrow_mut().clear();
    subscription.dispose();
}

#[test]
fn geometry_setters_preserve_nan_payload_and_signed_zero_equality() {
    type Setter = fn(&DockWindow, f64);
    type Getter = fn(&DockWindow) -> f64;
    for (set, get) in [
        (DockWindow::set_x as Setter, DockWindow::x as Getter),
        (DockWindow::set_y, DockWindow::y),
        (DockWindow::set_width, DockWindow::width),
        (DockWindow::set_height, DockWindow::height),
    ] {
        let window = DockWindow::new();
        let changes = Rc::new(RefCell::new(Vec::new()));
        let log = changes.clone();
        let subscription = window
            .changed()
            .subscribe_next(move |change| log.borrow_mut().push(change.property_name));
        set(&window, -0.0);
        assert_eq!(get(&window).to_bits(), 0.0_f64.to_bits());
        let first_nan = f64::from_bits(0x7ff8_0000_0000_0012);
        set(&window, first_nan);
        set(&window, f64::NAN);
        assert_eq!(get(&window).to_bits(), first_nan.to_bits());
        set(&window, f64::INFINITY);
        set(&window, f64::INFINITY);
        assert_eq!(changes.borrow().len(), 2);
        subscription.dispose();
    }
}
