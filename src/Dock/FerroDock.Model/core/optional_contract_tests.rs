use super::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct State {
    visible: Cell<bool>,
    title: RefCell<Option<String>>,
    source: Cell<DockOperationMask>,
    target: Cell<DockOperationMask>,
    open: Cell<bool>,
    active: Cell<bool>,
    selected: Cell<bool>,
    docking: Cell<DockingWindowState>,
    closed: Cell<bool>,
}

impl IDockSelectorInfo for State {
    fn show_in_selector(&self) -> bool {
        self.visible.get()
    }
    fn set_show_in_selector(&self, value: bool) {
        self.visible.set(value);
    }
    fn selector_title(&self) -> Option<String> {
        self.title.borrow().clone()
    }
    fn set_selector_title(&self, value: Option<String>) {
        *self.title.borrow_mut() = value;
    }
}
impl IDockableDockingRestrictions for State {
    fn allowed_dock_operations(&self) -> DockOperationMask {
        self.source.get()
    }
    fn set_allowed_dock_operations(&self, value: DockOperationMask) {
        self.source.set(value);
    }
    fn allowed_drop_operations(&self) -> DockOperationMask {
        self.target.get()
    }
    fn set_allowed_drop_operations(&self, value: DockOperationMask) {
        self.target.set(value);
    }
}
impl IDockingWindowState for State {
    fn is_open(&self) -> bool {
        self.open.get()
    }
    fn set_is_open(&self, value: bool) {
        self.open.set(value);
    }
    fn is_active(&self) -> bool {
        self.active.get()
    }
    fn set_is_active(&self, value: bool) {
        self.active.set(value);
    }
    fn is_selected(&self) -> bool {
        self.selected.get()
    }
    fn set_is_selected(&self, value: bool) {
        self.selected.set(value);
    }
    fn docking_state(&self) -> DockingWindowState {
        self.docking.get()
    }
    fn set_docking_state(&self, value: DockingWindowState) {
        self.docking.set(value);
    }
}
impl IHostWindowCloseState for State {
    fn is_closed(&self) -> bool {
        self.closed.get()
    }
}

#[test]
fn optional_contracts_can_share_one_model_without_conflating_state() {
    let model = Rc::new(State::default());
    let selector: Rc<dyn IDockSelectorInfo> = model.clone();
    let restrictions: Rc<dyn IDockableDockingRestrictions> = model.clone();
    let window: Rc<dyn IDockingWindowState> = model.clone();
    let closing: Rc<dyn IHostWindowCloseState> = model.clone();
    selector.set_show_in_selector(true);
    selector.set_selector_title(Some(String::new()));
    restrictions.set_allowed_dock_operations(DockOperationMask::LEFT | DockOperationMask::WINDOW);
    restrictions.set_allowed_drop_operations(DockOperationMask::FILL);
    window.set_is_open(true);
    window.set_is_active(false);
    window.set_is_selected(true);
    window.set_docking_state(DockingWindowState::DOCKED | DockingWindowState::FLOATING);
    assert!(selector.show_in_selector());
    assert_eq!(selector.selector_title(), Some(String::new()));
    assert!(restrictions
        .allowed_dock_operations()
        .allows(DockOperation::OuterLeft));
    assert!(restrictions
        .allowed_dock_operations()
        .allows(DockOperation::Window));
    assert!(!restrictions
        .allowed_dock_operations()
        .allows(DockOperation::Fill));
    assert!(restrictions
        .allowed_drop_operations()
        .allows(DockOperation::Fill));
    assert!(!restrictions
        .allowed_drop_operations()
        .allows(DockOperation::OuterLeft));
    assert!(window.is_open() && window.is_selected() && !window.is_active());
    assert_eq!(
        window.docking_state(),
        DockingWindowState::DOCKED | DockingWindowState::FLOATING
    );
    assert!(!closing.is_closed());
    model.closed.set(true);
    assert!(closing.is_closed());
    selector.set_show_in_selector(false);
    selector.set_selector_title(None);
    restrictions.set_allowed_dock_operations(DockOperationMask::NONE);
    restrictions.set_allowed_drop_operations(DockOperationMask::ALL);
    window.set_is_open(false);
    window.set_is_selected(false);
    window.set_is_active(true);
    window.set_docking_state(DockingWindowState::HIDDEN);
    assert!(!selector.show_in_selector());
    assert_eq!(selector.selector_title(), None);
    assert_eq!(
        restrictions.allowed_dock_operations(),
        DockOperationMask::NONE
    );
    assert_eq!(
        restrictions.allowed_drop_operations(),
        DockOperationMask::ALL
    );
    assert!(!window.is_open() && !window.is_selected() && window.is_active());
    assert_eq!(window.docking_state(), DockingWindowState::HIDDEN);
}
