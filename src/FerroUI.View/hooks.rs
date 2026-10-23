//! Hooks: the state of a component that runs again.
//!
//! A component made with [`rerender`](crate::rerender) or
//! `#[component(rerender)]` runs its function again whenever a signal it read
//! while it ran changes, or its parent runs again; the elements it returns are
//! compared with those of the run before and the controls are patched
//! (`docs/declarative-ui.md`, section 8). What has to outlive one run is kept
//! in hooks: each `use_*` call has a slot of the component, found by the order
//! of the calls, so a hook is called on every run, in the same order, never in
//! a condition or a loop whose length changes.
//!
//! - [`use_hook`]: a value made on the first run and handed back on the next
//!   ones.
//! - [`use_signal`]: a signal kept across runs. Reading it in the function
//!   makes the component run again when it changes; reading it in a property
//!   (`text: count`) changes only that property.
//! - [`use_memo`]: a memo kept across runs.
//! - [`use_effect`]: an effect started once the first run's controls are in
//!   place, which runs again when the signals it read change.
//! - [`use_effect_with`]: a closure run once the controls are in place, and
//!   again on a run whose dependencies differ; what it registered with
//!   [`on_cleanup`](crate::on_cleanup) runs before the next call and when the
//!   component is taken out.
//! - [`use_future`](crate::use_future) (`async_data.rs`): a future started
//!   once the controls are in place and again on a run whose dependencies
//!   differ, and its state as a signal.
//!
//! Everything a hook makes belongs to the component and is disposed when the
//! component is taken out. Signals made with [`signal`](crate::signal) in the
//! function itself belong to that run and are made again on the next one.

use crate::reactive::{effect, on_cleanup, Memo, Owner, Signal};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{BoxedValue, Ref, StyledElement};
use ferroui_controls::Control;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// The run of a component that is in progress.
struct Frame {
    /// The slots of the component's hooks.
    slots: Rc<RefCell<Vec<Rc<dyn Any>>>>,
    /// The slot of the next hook.
    index: usize,
    /// The owner of the component: what the hooks make belongs to it.
    owner: Owner,
    /// What runs once the controls of the run are in place.
    after: Vec<Box<dyn FnOnce()>>,
    /// The control the component is mounted in, if any.
    parent: Option<Ref<Control>>,
}

ferroui_base::ferro_thread_local! {
    static FRAMES: RefCell<Vec<Frame>> = RefCell::new(Vec::new());
}

/// Runs `render` as a run of the component whose hooks are `slots` and whose
/// owner is `owner`, and returns what it returned with what has to run once
/// its controls are in place.
pub(crate) fn render_with<R>(
    slots: &Rc<RefCell<Vec<Rc<dyn Any>>>>,
    owner: Owner,
    parent: Option<Ref<Control>>,
    render: impl FnOnce() -> R,
) -> (R, Vec<Box<dyn FnOnce()>>) {
    FRAMES.with(|frames| {
        frames.borrow_mut().push(Frame { slots: slots.clone(), index: 0, owner, after: Vec::new(), parent });
    });
    let result = render();
    let frame = FRAMES.with(|frames| frames.borrow_mut().pop()).expect("the frame of the run");
    let count = frame.slots.borrow().len();
    assert!(
        frame.index == count,
        "a component called {} hooks on this run and {count} on the first one: hooks are called in the same order on every run",
        frame.index,
    );
    (result, frame.after)
}

/// [`render_with`] for a component below an error boundary: a panic of
/// `render` is caught and returned as its message, with the runtime and the
/// frames of the hooks put back as they were before the run. A run that
/// calls its hooks in another order is an error too.
pub(crate) fn render_catching<R>(
    slots: &Rc<RefCell<Vec<Rc<dyn Any>>>>,
    owner: Owner,
    parent: Option<Ref<Control>>,
    render: impl FnOnce() -> R,
) -> Result<(R, Vec<Box<dyn FnOnce()>>), String> {
    let depth = FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        frames.push(Frame { slots: slots.clone(), index: 0, owner, after: Vec::new(), parent });
        frames.len()
    });
    let scope = crate::reactive::Scope::save();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(render));
    // A panic leaves the frames of the runs it went through.
    let frame = FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        frames.truncate(depth);
        frames.pop()
    });
    let frame = frame.expect("the frame of the run");
    match result {
        Ok(result) => {
            let count = frame.slots.borrow().len();
            if frame.index != count {
                return Err(format!(
                    "a component called {} hooks on this run and {count} on the first one: hooks are called in the same order on every run",
                    frame.index,
                ));
            }
            Ok((result, frame.after))
        }
        Err(payload) => {
            scope.restore();
            Err(panic_message(&*payload))
        }
    }
}

/// The message of a panic: its text, or a placeholder for a payload of
/// another type.
pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| String::from("a panic without a message"))
}

/// The owner of the component that is running, if one is.
pub(crate) fn render_owner() -> Option<Owner> {
    FRAMES.with(|frames| frames.borrow().last().map(|frame| frame.owner))
}

fn no_frame() -> ! {
    panic!("a hook is called while a component that runs again runs (`rerender`, `#[component(rerender)]`)")
}

/// A value made by `init` on the first run of the component and handed back
/// (cloned) on the next ones: the slot every other hook is built on. `init`
/// runs with the component as the owner, so what it makes lives as long as
/// the component.
pub fn use_hook<T: Clone + 'static>(init: impl FnOnce() -> T) -> T {
    let (slots, index, owner) = FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        let Some(frame) = frames.last_mut() else { no_frame() };
        let index = frame.index;
        frame.index += 1;
        (frame.slots.clone(), index, frame.owner)
    });
    let existing = slots.borrow().get(index).cloned();
    match existing {
        Some(value) => value.downcast_ref::<T>().cloned().unwrap_or_else(|| {
            panic!(
                "hook {index} of the component held another type than {}: hooks are called in the same order on every run",
                std::any::type_name::<T>()
            )
        }),
        None => {
            let value = owner.with(init);
            slots.borrow_mut().push(Rc::new(value.clone()));
            value
        }
    }
}

/// A signal that holds `init()` on the first run and is kept across runs.
pub fn use_signal<T: 'static>(init: impl FnOnce() -> T) -> Signal<T> {
    use_hook(|| crate::reactive::signal(init()))
}

/// A memo of `compute`, made on the first run and kept across runs. It
/// follows the signals it reads; it does not see the values the function was
/// given on later runs (read those in the function, or keep them in a
/// signal).
pub fn use_memo<T: PartialEq + 'static>(compute: impl Fn() -> T + 'static) -> Memo<T> {
    use_hook(|| crate::reactive::memo(move |_| compute()))
}

/// Starts an effect once the controls of the first run are in place; it runs
/// again whenever the signals it read change, until the component is taken
/// out.
pub fn use_effect(f: impl Fn() + 'static) {
    let started = use_hook(|| Rc::new(std::cell::Cell::new(false)));
    if !started.replace(true) {
        after(move |owner| owner.with(|| effect(f)));
    }
}

/// Calls `f` with `deps` once the controls of the run are in place, on the
/// first run and on every run whose `deps` differ from those of the call
/// before. What `f` registers with [`on_cleanup`](crate::on_cleanup) runs
/// before the next call and when the component is taken out.
pub fn use_effect_with<D: PartialEq + 'static>(deps: D, f: impl FnOnce(&D) + 'static) {
    type State<D> = Rc<RefCell<(Option<D>, Option<Owner>)>>;
    let state: State<D> = use_hook(|| Rc::new(RefCell::new((None, None))));
    let changed = state.borrow().0.as_ref() != Some(&deps);
    if changed {
        after(move |owner| {
            let previous = state.borrow_mut().1.take();
            if let Some(previous) = previous {
                previous.dispose();
            }
            let scope = owner.with(Owner::new);
            scope.with(|| f(&deps));
            *state.borrow_mut() = (Some(deps), Some(scope));
        });
    }
}

/// Runs `f` with the owner of the component once the controls of the run are
/// in place.
pub(crate) fn after(f: impl FnOnce(Owner) + 'static) {
    FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        let Some(frame) = frames.last_mut() else { no_frame() };
        let owner = frame.owner;
        frame.after.push(Box::new(move || f(owner)));
    });
}

/// The control the component is mounted in: the control whose content or
/// children hold it (`None` at the root of a tree). It is the same control on
/// every run.
pub fn use_parent_control() -> Option<Ref<Control>> {
    let parent = FRAMES.with(|frames| {
        let frames = frames.borrow();
        let Some(frame) = frames.last() else { no_frame() };
        frame.parent.clone()
    });
    // A hook, so that the order of the hooks does not depend on the parent.
    use_hook(|| ());
    parent
}

/// The value of `property` on the control the component is mounted in, as
/// it is now; the component runs again when it changes. For an inherited
/// property (`DataContext`, `ActualThemeVariant`, `FlowDirection`) it is the
/// value the control inherits. `None` at the root of a tree.
pub fn use_parent_property<T: ferroui_base::PropertyValue>(
    property: &'static ferroui_base::StyledProperty<T>,
) -> Option<T> {
    let parent = use_parent_control();
    let value = use_hook(move || {
        let value = crate::reactive::signal(parent.as_ref().map(|control| control.get_value(property)));
        if let Some(control) = parent {
            let untyped = property.as_property();
            let subscription = control.property_changed(move |e| {
                if std::ptr::eq(e.property(), untyped) {
                    if let Some(new) = e.new_value().downcast_ref::<T>() {
                        if value.is_alive() {
                            value.set(Some(new.clone()));
                        }
                    }
                }
            });
            on_cleanup(move || subscription.dispose());
        }
        value
    });
    value.get()
}

/// The `DataContext` of the control the component is mounted in (inherited
/// from its ancestors), as it is now; the component runs again when it
/// changes. This is how a component reads the item of an item template
/// ([`template`](crate::template)) or the view model of a view it is hosted
/// in ([`ComponentHost`](crate::ComponentHost)).
pub fn use_data_context_value() -> Option<BoxedValue> {
    use_parent_property(StyledElement::data_context_property()).flatten()
}

/// The theme variant of the control the component is mounted in (`Light`,
/// `Dark`, or one of the application), as it is now; the component runs again
/// when it changes.
pub fn use_theme_variant() -> Option<ferroui_base::styling::ThemeVariant> {
    use_parent_property(ferroui_base::styling::ThemeVariant::actual_theme_variant_property()).flatten()
}

/// The resource `key` as the control the component is mounted in finds it
/// (its resources, those of its ancestors, the application's and the
/// theme's, for its theme variant), as it is now; the component runs again
/// when it changes (another theme variant, a resource dictionary changed).
pub fn use_resource(key: &str) -> Option<BoxedValue> {
    let parent = use_parent_control();
    let key = key.to_string();
    let value = use_hook(move || {
        let value = crate::reactive::signal(None::<BoxedValue>);
        if let Some(control) = parent {
            let host = ferroui_base::controls::ResourceHostRef::from(&control);
            let subscription = host.resource_observable(key.as_str(), None).subscribe_fn(move |resource: Option<BoxedValue>| {
                let resource = resource.filter(|resource| !resource.is::<ferroui_base::UnsetValueType>());
                if value.is_alive() {
                    value.set(resource);
                }
            });
            on_cleanup(move || subscription.dispose());
        }
        value
    });
    value.get()
}

/// The `DataContext` of the control the component is mounted in, when it
/// holds a `T` (see [`use_data_context_value`]).
pub fn use_data_context<T: Clone + 'static>() -> Option<T> {
    use_data_context_value().and_then(|value| {
        let value: &dyn ferroui_base::AnyValue = &*value;
        value.downcast_ref::<T>().cloned()
    })
}

/// The resource `key` as a `T` (see [`use_resource`]): the conversion of
/// markup values, so a brush of a theme is a `Rc<dyn IBrush>` and a colour a
/// `Color`. `None` when the resource is missing or not a `T`.
pub fn use_resource_as<T: Clone + 'static>(key: &str) -> Option<T> {
    let value = use_resource(key);
    ferroui_base::metadata::from_markup_value::<T>(&value)
}
