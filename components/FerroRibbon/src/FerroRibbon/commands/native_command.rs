//! Boundary between existing UI-independent and native UI ICommand contracts.
use ferroribbon_core::commands::ICommand as CoreCommand;
use ferroribbon_core::model::RibbonObject;
use ferroui_base::BoxedValue;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::ICommand as NativeCommand;
use ferroui_base::reactive::IDisposable;
use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

/// The SAME source payload, without an Rc<BoxedValue> or Rc<RibbonObject> proxy.
pub fn command_parameter(parameter: Option<&BoxedValue>) -> Option<Rc<dyn Any>> {
    let value = ValueTypes::normalize(parameter?.clone())?;
    let value = ValueTypes::reference_object(&value).unwrap_or(value);
    if let Some(value) = value.downcast_ref::<RibbonObject>() {
        return Some(value.payload());
    }
    let value: Rc<dyn Any> = value;
    Some(value)
}
struct CoreAdapter {
    source: Rc<dyn CoreCommand>,
}
impl NativeCommand for CoreAdapter {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        let value = command_parameter(parameter);
        self.source.can_execute(value.as_deref())
    }
    fn execute(&self, parameter: Option<&BoxedValue>) {
        let value = command_parameter(parameter);
        self.source.execute(value.as_deref());
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.source.can_execute_changed(handler)
    }
    fn register_can_execute_changed(
        &self,
        handler: Rc<dyn Fn()>,
        publish: &mut dyn FnMut(Rc<dyn IDisposable>),
    ) {
        self.source.register_can_execute_changed(handler, publish)
    }
}
struct Adapted {
    source: Weak<dyn CoreCommand>,
    native: Weak<dyn NativeCommand>,
}
#[derive(Default)]
struct Adapters {
    by_source: HashMap<usize, Adapted>,
    by_native: HashMap<usize, Adapted>,
    next_prune: usize,
}
fn core_key(source: &Rc<dyn CoreCommand>) -> usize {
    Rc::as_ptr(source) as *const () as usize
}
fn native_key(native: &Rc<dyn NativeCommand>) -> usize {
    Rc::as_ptr(native) as *const () as usize
}
ferroui_base::ferro_thread_local! {static ADAPTERS:RefCell<Adapters>=RefCell::new(Adapters::default());}
/// Reuses one live native view for each actual Core command, without rooting either in a registry.
pub fn adapt_core_command(source: Rc<dyn CoreCommand>) -> Rc<dyn NativeCommand> {
    let key = core_key(&source);
    if let Some(existing) = ADAPTERS.with_borrow(|entries| {
        entries.by_source.get(&key).and_then(|row| {
            let original = row.source.upgrade()?;
            Rc::ptr_eq(&original, &source)
                .then(|| row.native.upgrade())
                .flatten()
        })
    }) {
        return existing;
    }
    let native: Rc<dyn NativeCommand> = Rc::new(CoreAdapter {
        source: source.clone(),
    });
    ADAPTERS.with_borrow_mut(|entries| {
        if entries.by_source.len().max(entries.by_native.len()) >= entries.next_prune {
            entries
                .by_source
                .retain(|_, row| row.source.strong_count() != 0 && row.native.strong_count() != 0);
            entries
                .by_native
                .retain(|_, row| row.source.strong_count() != 0 && row.native.strong_count() != 0);
            entries.next_prune = entries
                .by_source
                .len()
                .max(entries.by_native.len())
                .max(64)
                .saturating_mul(2);
        }
        entries.by_source.insert(
            key,
            Adapted {
                source: Rc::downgrade(&source),
                native: Rc::downgrade(&native),
            },
        );
        entries.by_native.insert(
            native_key(&native),
            Adapted {
                source: Rc::downgrade(&source),
                native: Rc::downgrade(&native),
            },
        );
    });
    native
}
/// Only succeeds for a native view created by this bridge; no fake reverse command is produced.
pub fn source_core_command(native: &Rc<dyn NativeCommand>) -> Option<Rc<dyn CoreCommand>> {
    ADAPTERS.with_borrow(|entries| {
        entries.by_native.get(&native_key(native)).and_then(|row| {
            let candidate = row.native.upgrade()?;
            Rc::ptr_eq(&candidate, native)
                .then(|| row.source.upgrade())
                .flatten()
        })
    })
}
pub(crate) fn register_conversions() {
    // Native Button.Command is nullable. Do not rely on a different library
    // control's property metadata having registered that value form first.
    ValueTypes::register_nullable::<Rc<dyn NativeCommand>>();
    ValueTypes::register_nullable::<Rc<dyn CoreCommand>>();
    ValueTypes::register_conversion::<Rc<dyn CoreCommand>, Option<Rc<dyn NativeCommand>>>(
        |source| Some(Some(adapt_core_command(source.clone()))),
    );
    ValueTypes::register_cast::<Rc<dyn CoreCommand>, Rc<dyn NativeCommand>>(|source| {
        adapt_core_command(source.clone())
    });
    // Only known original commands have a faithful reverse identity.
    ValueTypes::register_conversion::<Rc<dyn NativeCommand>, Rc<dyn CoreCommand>>(
        source_core_command,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeatedly_recreated_views_do_not_leave_unbounded_reverse_cache_entries() {
        let source: ferroribbon_core::commands::RibbonRelayCommand =
            ferroribbon_core::commands::RibbonRelayCommand::construct(Rc::new(|_| {}), None);
        let source: Rc<dyn CoreCommand> = Rc::new(source);
        for _ in 0..4096 {
            drop(adapt_core_command(source.clone()));
        }
        ADAPTERS.with_borrow(|entries| {
            assert!(entries.by_native.len() <= 128);
            assert!(entries.by_source.len() <= 128);
        });
    }
}
