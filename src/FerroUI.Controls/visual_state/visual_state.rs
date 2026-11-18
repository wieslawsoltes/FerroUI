//! WinUI's `VisualState` (`Microsoft.UI.Xaml.VisualState`; the IDL
//! `dxaml/xcp/dxaml/idl/winrt/core/microsoft.ui.xaml.coretypes.idl`, the engine
//! `dxaml/xcp/components/vsm/` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use ferroui_base::collections::FerroList;
use ferroui_base::styling::SetterBase;
use super::StateTriggerBase;
use crate::animation::Storyboard;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl, Ref};
use std::cell::RefCell;
use std::rc::Rc;

/// A visual state: the values its [`setters`](Self::setters) set while it is the current state
/// of its [`VisualStateGroup`](super::VisualStateGroup).
///
/// The [`storyboard`](Self::storyboard) of the state runs while it is the current state; the
/// storyboard of the state it leaves stops (its values are removed).
///
/// The setters are setters with a [`target`](ferroui_base::styling::Setter::target): markup writes
/// `<Setter Target="Element.Property" Value="..."/>`. They are applied at animation priority, above
/// local values and styles, and removed when the group leaves the state
/// (docs/porting/winui-calculator-gaps.md, section 5.5).
///
/// Deviation (docs/porting/winui-controls.md, VisualStateManager): `x:Name` names the state but
/// does not register it in the name scope, so `find_name` does not find it.
#[repr(C)]
pub struct VisualState {
    base: FerroObject,
    name: RefCell<String>,
    setters: FerroList<Rc<dyn SetterBase>>,
    state_triggers: FerroList<Ref<StateTriggerBase>>,
    storyboard: RefCell<Option<Ref<Storyboard>>>,
}

ferro_class!(VisualState: FerroObject);
ferro_class_info!(VisualState {
    new: VisualState::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Storyboard,
        properties: [
            Name: String { get: VisualState::name, set: |this: &Ref<VisualState>, value: String| this.set_name(value) },
            Setters: FerroList<Rc<dyn SetterBase>> { get: VisualState::setters },
            StateTriggers: FerroList<Ref<StateTriggerBase>> { get: VisualState::state_triggers },
            Storyboard: Option<Ref<Storyboard>> {
                get: VisualState::storyboard,
                set: |this: &Ref<VisualState>, value: Option<Ref<Storyboard>>| this.set_storyboard(value)
            },
        ],
    },
});
ferro_impl_classes!(VisualState: FerroObjectImpl);

impl VisualState {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            name: RefCell::new(String::new()),
            setters: FerroList::new(),
            state_triggers: FerroList::new(),
            storyboard: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a state with a name.
    pub fn with_name(name: impl Into<String>) -> Ref<Self> {
        let state = Self::new();
        state.set_name(name);
        state
    }

    /// The name of the state, which `VisualStateManager::go_to_state` is given.
    pub fn name(&self) -> String {
        self.name.borrow().clone()
    }

    /// Sets the name (WinUI's `x:Name`; the name is read-only in WinUI once the state is
    /// loaded).
    pub fn set_name(&self, value: impl Into<String>) {
        *self.name.borrow_mut() = value.into();
    }

    /// Whether the name of the state is `name` (case-sensitive, as WinUI compares).
    pub(crate) fn is_named(&self, name: &str) -> bool {
        *self.name.borrow() == name
    }

    /// The setters of the state (WinUI's `SetterBaseCollection`).
    pub fn setters(&self) -> FerroList<Rc<dyn SetterBase>> {
        self.setters.clone()
    }

    /// The triggers of the state: the state is a candidate to be the current state of its group
    /// while one of them is active (several triggers of a state are alternatives).
    pub fn state_triggers(&self) -> FerroList<Ref<StateTriggerBase>> {
        self.state_triggers.clone()
    }

    /// The storyboard that runs while the state is current (WinUI's content property).
    pub fn storyboard(&self) -> Option<Ref<Storyboard>> {
        self.storyboard.borrow().clone()
    }

    /// Sets the storyboard. A new storyboard of the current state starts the next time the state
    /// is entered.
    pub fn set_storyboard(&self, value: Option<Ref<Storyboard>>) {
        *self.storyboard.borrow_mut() = value;
    }
}
