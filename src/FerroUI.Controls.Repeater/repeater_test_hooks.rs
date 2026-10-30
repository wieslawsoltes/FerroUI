// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::layouts::Layout;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{ferro_static_type, FerroObject, Ref};
use std::cell::Cell;
use std::rc::Rc;

struct Hooks {
    build_tree_completed: HandlerList<dyn Fn()>,
    element_factory_element_index: Cell<i32>,
    log_item_index: Cell<i32>,
}
ferroui_base::ferro_thread_local! {
    static HOOKS: Hooks = Hooks {
        build_tree_completed: HandlerList::new(),
        element_factory_element_index: Cell::new(0),
        log_item_index: Cell::new(-1),
    };
}

/// Diagnostics shared by the repeater's realization, layout and phase services.
/// The hooks are local to the UI thread, like the scheduler they observe.
pub struct RepeaterTestHooks;
ferro_static_type!(RepeaterTestHooks);

impl RepeaterTestHooks {
    pub fn build_tree_completed(handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = HOOKS.with(|hooks| hooks.build_tree_completed.add(Rc::new(handler)));
        Disposable::create(move || {
            let _ = HOOKS.try_with(|hooks| hooks.build_tree_completed.remove(token));
        })
    }
    pub fn notify_build_tree_completed() {
        let handlers = HOOKS.with(|hooks| hooks.build_tree_completed.snapshot());
        for (_, handler) in handlers.iter() {
            handler();
        }
    }
    pub fn get_layout_id(layout: &Ref<FerroObject>) -> String {
        layout
            .cast::<Layout>()
            .map(|layout| layout.layout_id())
            .unwrap_or_default()
    }
    pub fn set_layout_id(layout: &Ref<FerroObject>, id: impl Into<String>) {
        if let Some(layout) = layout.cast::<Layout>() {
            layout.set_layout_id(id.into());
        }
    }
    pub fn get_element_factory_element_index() -> i32 {
        HOOKS.with(|hooks| hooks.element_factory_element_index.get())
    }
    pub fn set_element_factory_element_index(index: i32) {
        HOOKS.with(|hooks| hooks.element_factory_element_index.set(index));
    }
    pub fn get_log_item_index() -> i32 {
        HOOKS.with(|hooks| hooks.log_item_index.get())
    }
    /// Selects realization, retention and recycling traces in the native
    /// `Control` log area. A negative index disables these item diagnostics.
    /// The installed logger still controls the accepted level and output.
    pub fn set_log_item_index(index: i32) {
        HOOKS.with(|hooks| hooks.log_item_index.set(index));
    }
}
