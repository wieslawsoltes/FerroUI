//! Source ConditionalWeakTable behavior without storing strong control handles or raising extra properties.
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::{Ref, Visual, WeakRef};
use ferroui_controls::primitives::TemplatedControl;
use std::{cell::RefCell, collections::HashMap};

struct Saved {
    control: WeakRef<TemplatedControl>,
    local: Option<bool>,
}
#[derive(Default)]
struct Hidden {
    entries: HashMap<usize, Saved>,
    operations: usize,
}
ferroui_system::ferro_thread_local! { static HIDDEN: RefCell<Hidden> = RefCell::new(Hidden::default()); }
fn key(control: &TemplatedControl) -> usize {
    control as *const TemplatedControl as usize
}
fn is_hidden(control: &TemplatedControl) -> bool {
    HIDDEN.with_borrow_mut(|table| {
        table.operations = table.operations.wrapping_add(1);
        if table.operations % 128 == 0 {
            table.entries.retain(|_, s| s.control.upgrade().is_some());
        }
        table
            .entries
            .get(&key(control))
            .is_some_and(|s| s.control.upgrade().is_some())
    })
}
pub(crate) fn set(element: &Ref<Visual>, hidden: bool) {
    if let Some(control) = element.cast::<TemplatedControl>()
        && is_hidden(&control) == hidden
    {
        return;
    }
    if !hidden {
        let mut parent = element.visual_parent();
        while let Some(current) = parent {
            if let Some(control) = current.cast::<TemplatedControl>()
                && is_hidden(&control)
            {
                return;
            }
            parent = current.visual_parent();
        }
    }
    apply(element, hidden);
}
fn apply(element: &Ref<Visual>, hidden: bool) {
    if let Some(control) = element.cast::<TemplatedControl>() {
        if is_hidden(&control) == hidden {
            return;
        }
        let property = KeyboardNavigation::is_tab_stop_property();
        if hidden {
            // Native SetValue retains bindings; source-platform replacement and
            // disposal ordering still need a separate mapping.
            assert!(
                !control.has_local_value_binding(property.as_property()),
                "FerroRibbon focus hiding: bound IsTabStop restoration is not yet implemented"
            );
            let local = control.get_local_value(property);
            HIDDEN.with_borrow_mut(|table| {
                table.entries.insert(
                    key(&control),
                    Saved {
                        control: control
                            .to_ref()
                            .cast::<TemplatedControl>()
                            .unwrap()
                            .downgrade(),
                        local,
                    },
                );
            });
            KeyboardNavigation::set_is_tab_stop(&control, false);
        } else {
            let saved = HIDDEN
                .with_borrow_mut(|table| table.entries.remove(&key(&control)))
                .expect("hidden entry");
            if let Some(value) = saved.local {
                control.set_value(property, value);
            } else {
                control.clear_local_value(property);
            }
        }
    }
    let count = element.visual_children().count();
    for i in 0..count {
        apply(&element.visual_children().get(i), hidden);
    }
}
