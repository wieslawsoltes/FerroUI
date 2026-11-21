//! The state mappers of the upstream-ported controls (docs/porting/winui-calculator-gaps.md,
//! section 5.10, decision D1): the ports of WinUI's `ChangeVisualState` of `Button`,
//! `ToggleButton`, `CheckBox`, `RadioButton`, `ListBoxItem` and the `UpdateVisualState` of
//! `TextBox` (`dxaml/xcp/dxaml/lib/Button_Partial.cpp`, `ToggleButton_Partial.cpp`,
//! `CheckBox_Partial.cpp`, `RadioButton_Partial.cpp`, `ListBoxItem_Partial.cpp`,
//! `dxaml/xcp/core/native/text/Controls/TextBoxBase.cpp` and `TextBox.cpp` of microsoft-ui-xaml
//! commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).
//!
//! A mapper turns the state of the control (its pseudo-classes and properties) into
//! `GoToState` calls. It is installed when a template is applied whose root has visual state
//! groups, and only then; it runs again, with transitions, when the classes or the properties the
//! states depend on change. It never changes a pseudo-class: the styles of the control keep
//! working beside it.
//!
//! The class of a family states its mapper from its static constructor ([`register`]), so a mapper
//! and the class it reads are linked into an application with the class alone (#612): naming the
//! families here would keep the list box item, the text box and their classes in every
//! application with a templated control.

use super::VisualStateManager;
use crate::primitives::{TemplatedControl, ToggleButton};
use crate::{Control, ListBoxItem, TextBox};
use ferroui_base::media::TextWrapping;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ObjectType, Ref, TypeInfo, WeakRef};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The mapper of a control family: sets the visual states of the control.
type Mapper = fn(&Ref<Control>, bool);

/// The properties whose changes the mappers follow (the rest of the state is in pseudo-classes).
const FOLLOWED_PROPERTIES: &[&str] =
    &["IsChecked", "IsSelected", "IsEnabled", "Text", "IsReadOnly", "AcceptsReturn", "TextWrapping"];

/// An installed mapper: what to remove when the template changes.
struct Installed {
    control: WeakRef<Control>,
    classes_token: u64,
    properties: Rc<dyn IDisposable>,
}

ferroui_base::ferro_thread_local! {
    static INSTALLED: RefCell<HashMap<usize, Installed>> = RefCell::new(HashMap::new());
}

ferroui_base::ferro_thread_local! {
    /// The mapper of each family, by its class: stated by the static constructor of the class.
    static MAPPERS: RefCell<Vec<(&'static TypeInfo, Mapper)>> = const { RefCell::new(Vec::new()) };
}

/// States the mapper of the family of class `T`, for `T` and the classes deriving from it that
/// state none of their own. Called by the static constructor of `T`: a control of the class has
/// its mapper on the thread that created it.
pub(crate) fn register<T: ObjectType>(mapper: Mapper) {
    MAPPERS.with(|mappers| {
        let mut mappers = mappers.borrow_mut();
        if !mappers.iter().any(|(type_, _)| std::ptr::eq(*type_, T::TYPE)) {
            mappers.push((T::TYPE, mapper));
        }
    });
}

/// The mapper of the family of `control`: the one its class or its nearest base class states.
fn mapper_of(control: &Control) -> Option<Mapper> {
    MAPPERS.with(|mappers| {
        let mappers = mappers.borrow();
        if mappers.is_empty() {
            return None;
        }
        let mut current = Some(control.get_type());
        while let Some(type_) = current {
            if let Some((_, mapper)) = mappers.iter().find(|(candidate, _)| std::ptr::eq(*candidate, type_)) {
                return Some(*mapper);
            }
            current = type_.base_type();
        }
        None
    })
}

fn key(control: &Control) -> usize {
    control as *const Control as usize
}

fn uninstall(control: &Control) {
    let installed = INSTALLED.with(|installed| installed.borrow_mut().remove(&key(control)));
    if let Some(installed) = installed {
        control.classes().remove_listener(installed.classes_token);
        installed.properties.dispose();
    }
}

/// The template of `control` was applied: installs the mapper of its family when the root of the
/// template has visual state groups (and sets the states without transitions, as WinUI's
/// `OnApplyTemplate` calls `UpdateVisualState(false)`), else removes the mapper it had.
pub(crate) fn on_template_applied(control: &TemplatedControl) {
    let control: Ref<Control> = control.to_ref().upcast();
    // Controls of other families never have a mapper (nothing else runs for them).
    let Some(mapper) = mapper_of(&control) else {
        return;
    };
    uninstall(&control);
    let has_groups = VisualStateManager::implementation_root(&control).is_some_and(|root| {
        root.get_value(VisualStateManager::visual_state_groups_property()).is_some_and(|groups| groups.count() > 0)
    });
    if !has_groups {
        return;
    }
    let weak = control.downgrade();
    let on_classes = {
        let weak = weak.clone();
        Rc::new(move || {
            if let Some(control) = weak.upgrade() {
                mapper(&control, true);
            }
        })
    };
    let classes_token = control.classes().add_listener(on_classes);
    let properties = {
        let weak = weak.clone();
        control.property_changed(move |change| {
            if !FOLLOWED_PROPERTIES.contains(&change.property().name()) {
                return;
            }
            if let Some(control) = weak.upgrade() {
                mapper(&control, true);
            }
        })
    };
    INSTALLED.with(|installed| {
        let mut installed = installed.borrow_mut();
        // Entries of controls that are gone.
        installed.retain(|_, entry| entry.control.upgrade().is_some());
        installed.insert(key(&control), Installed { control: weak, classes_token, properties });
    });
    mapper(&control, false);
}

/// Whether the class `type_` itself has stated a mapper on this thread.
#[cfg(test)]
pub(crate) fn has_mapper(type_: &'static TypeInfo) -> bool {
    MAPPERS.with(|mappers| mappers.borrow().iter().any(|(candidate, _)| std::ptr::eq(*candidate, type_)))
}

/// Whether a mapper is installed on `control`.
#[cfg(test)]
pub(crate) fn is_installed(control: &Control) -> bool {
    INSTALLED.with(|installed| installed.borrow().contains_key(&key(control)))
}

fn has(control: &Control, pseudo_class: &str) -> bool {
    control.classes().contains(pseudo_class)
}

fn go(control: &Ref<Control>, state: &str, use_transitions: bool) -> bool {
    VisualStateManager::go_to_state(control, state, use_transitions)
}

/// The common state of the button family: `Disabled`, `Pressed`, `PointerOver` or `Normal`.
fn common_state(control: &Control) -> &'static str {
    if has(control, ":disabled") {
        "Disabled"
    } else if has(control, ":pressed") {
        "Pressed"
    } else if has(control, ":pointerover") {
        "PointerOver"
    } else {
        "Normal"
    }
}

/// The focus states: `Focused` for keyboard focus, `PointerFocused` for other focus (FerroUI
/// shows the focus visual for keyboard focus only), `Unfocused`.
fn focus_states(control: &Ref<Control>, use_transitions: bool) {
    let state = if has(control, ":focus") && !has(control, ":disabled") {
        if has(control, ":focus-visible") {
            "Focused"
        } else {
            "PointerFocused"
        }
    } else {
        "Unfocused"
    };
    go(control, state, use_transitions);
}

/// WinUI's `Button::ChangeVisualState`.
pub(crate) fn button(control: &Ref<Control>, use_transitions: bool) {
    go(control, common_state(control), use_transitions);
    focus_states(control, use_transitions);
}

fn is_checked(control: &Control) -> Option<bool> {
    control.downcast_ref::<ToggleButton>().and_then(ToggleButton::is_checked)
}

/// WinUI's `ToggleButton::ChangeVisualState`: the check state and the common state in one group
/// (`Checked`, `CheckedPointerOver`, `IndeterminatePressed`, ...).
pub(crate) fn toggle_button(control: &Ref<Control>, use_transitions: bool) {
    let common = common_state(control);
    let prefix = match is_checked(control) {
        None => "Indeterminate",
        Some(true) => "Checked",
        Some(false) => "",
    };
    let state = match (prefix, common) {
        ("", common) => common.to_string(),
        (prefix, "Normal") => prefix.to_string(),
        (prefix, common) => format!("{prefix}{common}"),
    };
    go(control, &state, use_transitions);
    focus_states(control, use_transitions);
}

/// WinUI's `CheckBox::ChangeVisualState`: the combined states (`UncheckedNormal`,
/// `CheckedPointerOver`, `IndeterminateDisabled`, ...), else the older common and check states.
pub(crate) fn check_box(control: &Ref<Control>, use_transitions: bool) {
    let common = common_state(control);
    let check = match is_checked(control) {
        None => "Indeterminate",
        Some(true) => "Checked",
        Some(false) => "Unchecked",
    };
    if !go(control, &format!("{check}{common}"), use_transitions) {
        go(control, common, use_transitions);
        go(control, check, use_transitions);
    }
    focus_states(control, use_transitions);
}

/// WinUI's `RadioButton::ChangeVisualState`: the common states and the check states.
pub(crate) fn radio_button(control: &Ref<Control>, use_transitions: bool) {
    go(control, common_state(control), use_transitions);
    let check = match is_checked(control) {
        None => "Indeterminate",
        Some(true) => "Checked",
        Some(false) => "Unchecked",
    };
    go(control, check, use_transitions);
    focus_states(control, use_transitions);
}

/// WinUI's `ListBoxItem::ChangeVisualState`: the common states (a disabled item whose content is
/// a control shows `Normal`, the content shows its own disabled state), the selection states and
/// the focus states.
pub(crate) fn list_box_item(control: &Ref<Control>, use_transitions: bool) {
    let Some(item) = control.downcast_ref::<ListBoxItem>() else {
        return;
    };
    let disabled = has(control, ":disabled");
    let pressed = has(control, ":pressed");
    let pointer_over = has(control, ":pointerover");
    let common = if disabled {
        let content_is_control = item.content().and_then(|content| Control::from_boxed(&content)).is_some();
        if content_is_control {
            "Normal"
        } else {
            "Disabled"
        }
    } else if pressed {
        "Pressed"
    } else if pointer_over {
        "PointerOver"
    } else {
        "Normal"
    };
    go(control, common, use_transitions);
    let selection = if item.is_selected() {
        if disabled {
            "SelectedDisabled"
        } else if pressed {
            "SelectedPressed"
        } else if pointer_over {
            "SelectedPointerOver"
        } else {
            // WinUI's `IsSelectionActive` of the parent selector: the keyboard focus is in it.
            let active = control
                .get_logical_ancestors()
                .filter_map(|ancestor| ancestor.cast::<Control>())
                .find(|ancestor| ancestor.downcast_ref::<crate::primitives::SelectingItemsControl>().is_some())
                .is_some_and(|selector| selector.classes().contains(":focus-within"));
            if active {
                "Selected"
            } else {
                "SelectedUnfocused"
            }
        }
    } else {
        "Unselected"
    };
    go(control, selection, use_transitions);
    if has(control, ":focus") && !disabled {
        go(control, if has(control, ":focus-visible") { "Focused" } else { "PointerFocused" }, use_transitions);
    } else {
        go(control, "Unfocused", use_transitions);
    }
}

/// WinUI's `CTextBoxBase::UpdateVisualState` and `CTextBox::UpdateVisualState`: `Disabled`,
/// `Focused`, `PointerOver` or `Normal` (pressed and pointer focus look focused), and
/// `ButtonVisible` while the delete button can be used (focused, not empty, not read-only, one
/// line), else `ButtonCollapsed`.
pub(crate) fn text_box(control: &Ref<Control>, use_transitions: bool) {
    let Some(text_box) = control.downcast_ref::<TextBox>() else {
        return;
    };
    let focused = has(control, ":focus");
    let common = if has(control, ":disabled") {
        "Disabled"
    } else if focused {
        "Focused"
    } else if has(control, ":pointerover") {
        "PointerOver"
    } else {
        "Normal"
    };
    go(control, common, use_transitions);
    let delete_button = focused
        && text_box.text().is_some_and(|text| !text.is_empty())
        && !text_box.is_read_only()
        && text_box.text_wrapping() == TextWrapping::NoWrap
        && !text_box.accepts_return();
    go(control, if delete_button { "ButtonVisible" } else { "ButtonCollapsed" }, use_transitions);
}
