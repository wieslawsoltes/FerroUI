// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::IDockable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
ferroui_base::ferro_thread_local! {
    static IS_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static DOCKABLE: RefCell<Option<Rc<dyn IDockable>>> = RefCell::new(None);
}
/// Original shared drag-preview state on the native UI thread.
pub(crate) struct DragPreviewContext;
impl DragPreviewContext {
    pub(crate) fn is_active() -> bool {
        IS_ACTIVE.with(Cell::get)
    }
    pub(crate) fn set_is_active(value: bool) {
        IS_ACTIVE.with(|active| active.set(value));
    }
    pub(crate) fn dockable() -> Option<Rc<dyn IDockable>> {
        DOCKABLE.with(|dockable| dockable.borrow().clone())
    }
    pub(crate) fn set_dockable(value: Option<Rc<dyn IDockable>>) {
        let previous = DOCKABLE.with(|dockable| dockable.replace(value));
        drop(previous);
    }
    pub(crate) fn is_previewing(dockable: &Rc<dyn IDockable>) -> bool {
        Self::is_active() && Self::dockable().is_some_and(|current| Rc::ptr_eq(&current, dockable))
    }
    pub(crate) fn clear() {
        Self::set_is_active(false);
        Self::set_dockable(None);
    }
}
