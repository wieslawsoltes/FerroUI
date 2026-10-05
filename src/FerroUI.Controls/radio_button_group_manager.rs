use crate::MenuItemToggleType;
use ferroui_base::rendering::IPresentationSource;
use ferroui_base::{FerroObject, ObjectType, Ref, StyledElement, TypeInfo, WeakRef};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// An element that takes part in the mutual exclusion of a radio group.
///
/// A class cannot implement the trait itself (its methods would shadow the
/// inherent ones of the class): it provides a small adapter handle that
/// does, and makes it known with [`register_radio_button`], normally from
/// its class initialization. [`as_radio_button`] then views an object of
/// that class (or of a class derived from it) as a radio button.
pub(crate) trait IRadioButton {
    /// The logical element behind the handle.
    fn logical(&self) -> Ref<StyledElement>;

    /// The name of the group the element belongs to.
    fn group_name(&self) -> Option<String>;

    /// How the element reacts to clicks.
    fn toggle_type(&self) -> MenuItemToggleType;

    /// Whether the element is checked.
    fn is_checked(&self) -> bool;

    /// Checks or unchecks the element.
    fn set_is_checked(&self, value: bool);
}

type Adapter = Rc<dyn Fn(&FerroObject) -> Option<Rc<dyn IRadioButton>>>;

thread_local! {
    static RADIO_BUTTON_TYPES: RefCell<Vec<(&'static TypeInfo, Adapter)>> = const { RefCell::new(Vec::new()) };
    static DEFAULT: Rc<RadioButtonGroupManager> = Rc::new(RadioButtonGroupManager::new());
    static REGISTERED_VISUAL_ROOTS: RefCell<Vec<(Weak<dyn IPresentationSource>, Rc<RadioButtonGroupManager>)>> =
        const { RefCell::new(Vec::new()) };
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`IRadioButton`] through the handle created by `adapter`.
pub(crate) fn register_radio_button<T: ObjectType>(adapter: fn(Ref<T>) -> Rc<dyn IRadioButton>) {
    RADIO_BUTTON_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            let adapter: Adapter =
                Rc::new(move |object| object.downcast_ref::<T>().map(|target| adapter(FerroObject::ref_of(target))));
            types.push((T::TYPE, adapter));
        }
    });
}

/// The object viewed as a radio button, if its class implements
/// [`IRadioButton`].
pub(crate) fn as_radio_button(object: &FerroObject) -> Option<Rc<dyn IRadioButton>> {
    let adapter = RADIO_BUTTON_TYPES.with(|types| {
        let types = types.borrow();
        let mut current = Some(object.get_type());
        while let Some(type_) = current {
            if let Some((_, adapter)) = types.iter().find(|(candidate, _)| *candidate == type_) {
                return Some(adapter.clone());
            }
            current = type_.base_type();
        }
        None
    });
    adapter.and_then(|adapter| adapter(object))
}

fn is_null_or_empty(value: Option<&str>) -> bool {
    value.is_none_or(str::is_empty)
}

/// Resets the "ignore checked changes" flag when the update that set it
/// ends, however it ends.
struct IgnoreCheckedChanges<'a>(&'a Cell<bool>);

impl Drop for IgnoreCheckedChanges<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// Keeps the radio buttons of a root mutually exclusive within their
/// groups.
pub(crate) struct RadioButtonGroupManager {
    registered_groups: RefCell<HashMap<String, Vec<WeakRef<StyledElement>>>>,
    ignore_checked_changes: Cell<bool>,
}

impl RadioButtonGroupManager {
    fn new() -> Self {
        Self { registered_groups: RefCell::new(HashMap::new()), ignore_checked_changes: Cell::new(false) }
    }

    /// The manager of a root: the one shared by every element without a
    /// root when `root` is `None`.
    pub(crate) fn get_or_create_for_root(root: Option<&Rc<dyn IPresentationSource>>) -> Rc<RadioButtonGroupManager> {
        let Some(root) = root else {
            return DEFAULT.with(Rc::clone);
        };

        REGISTERED_VISUAL_ROOTS.with(|roots| {
            let mut roots = roots.borrow_mut();
            // Entries go away with their root.
            roots.retain(|(key, _)| key.strong_count() > 0);

            let root_ptr = Rc::as_ptr(root);
            if let Some((_, manager)) = roots.iter().find(|(key, _)| std::ptr::addr_eq(key.as_ptr(), root_ptr)) {
                return manager.clone();
            }

            let manager = Rc::new(RadioButtonGroupManager::new());
            roots.push((Rc::downgrade(root), manager.clone()));
            manager
        })
    }

    pub(crate) fn add(&self, radio_button: &dyn IRadioButton) {
        let Some(group_name) = radio_button.group_name() else { return };
        if radio_button.toggle_type() != MenuItemToggleType::Radio {
            return;
        }

        let radio_button = radio_button.logical();
        let mut registered_groups = self.registered_groups.borrow_mut();
        let group = registered_groups.entry(group_name).or_default();

        let mut i = 0;
        while i < group.len() {
            let Some(current) = group[i].upgrade() else {
                group.remove(i);
                continue;
            };

            if current == radio_button {
                return;
            }

            i += 1;
        }

        group.push(radio_button.downgrade());
    }

    pub(crate) fn remove(&self, radio_button: &dyn IRadioButton, old_group_name: Option<&str>) {
        let Some(old_group_name) = old_group_name.filter(|name| !name.is_empty()) else { return };

        let radio_button = radio_button.logical();
        let mut registered_groups = self.registered_groups.borrow_mut();
        if let Some(group) = registered_groups.get_mut(old_group_name) {
            group.retain(|entry| entry.upgrade().is_some_and(|button| button != radio_button));

            if group.is_empty() {
                registered_groups.remove(old_group_name);
            }
        }
    }

    pub(crate) fn on_checked_changed(&self, radio_button: &dyn IRadioButton) {
        if self.ignore_checked_changes.get() || radio_button.toggle_type() != MenuItemToggleType::Radio {
            return;
        }

        self.ignore_checked_changes.set(true);
        let _reset = IgnoreCheckedChanges(&self.ignore_checked_changes);

        let this_button = radio_button.logical();
        let group_name = radio_button.group_name();

        if let Some(group_name) = group_name.as_deref().filter(|name| !name.is_empty()) {
            if !self.registered_groups.borrow().contains_key(group_name) {
                return;
            }

            // Unchecking a button raises events: no borrow is held while it
            // happens, and the group is looked up again for each step.
            let mut i = 0;
            loop {
                let current = {
                    let mut registered_groups = self.registered_groups.borrow_mut();
                    let Some(group) = registered_groups.get_mut(group_name) else { break };
                    if i >= group.len() {
                        break;
                    }
                    match group[i].upgrade() {
                        Some(current) => current,
                        None => {
                            group.remove(i);
                            continue;
                        }
                    }
                };

                if current != this_button {
                    if let Some(current) = as_radio_button(&current) {
                        if current.is_checked() {
                            current.set_is_checked(false);
                        }
                    }
                }
                i += 1;
            }

            {
                let mut registered_groups = self.registered_groups.borrow_mut();
                if registered_groups.get(group_name).is_some_and(Vec::is_empty) {
                    registered_groups.remove(group_name);
                }
            }

            let mut parent = this_button.parent().and_then(|parent| as_radio_button(&parent));
            while let Some(current) = parent {
                if current.group_name().as_deref() != Some(group_name) {
                    break;
                }
                current.set_is_checked(true);
                parent = current.logical().parent().and_then(|parent| as_radio_button(&parent));
            }
        } else if let Some(parent) = this_button.parent() {
            for sibling in parent.logical_children().snapshot().iter() {
                if *sibling == this_button {
                    continue;
                }

                if let Some(button) = as_radio_button(sibling) {
                    if button.toggle_type() == MenuItemToggleType::Radio
                        && is_null_or_empty(button.group_name().as_deref())
                        && button.is_checked()
                    {
                        button.set_is_checked(false);
                    }
                }
            }
        }
    }
}
