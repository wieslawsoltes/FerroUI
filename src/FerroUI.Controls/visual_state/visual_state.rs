//! WinUI's `VisualState` (`Microsoft.UI.Xaml.VisualState`; the IDL
//! `dxaml/xcp/dxaml/idl/winrt/core/microsoft.ui.xaml.coretypes.idl`, the engine
//! `dxaml/xcp/components/vsm/` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use ferroui_base::collections::FerroList;
use ferroui_base::styling::SetterBase;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl, Ref};
use std::cell::RefCell;
use std::rc::Rc;

/// A visual state: the values its [`setters`](Self::setters) set while it is the current state
/// of its [`VisualStateGroup`](super::VisualStateGroup).
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
}

ferro_class!(VisualState: FerroObject);
ferro_class_info!(VisualState {
    new: VisualState::new,
    markup: {
        namespace: "FerroUI.Controls",
        properties: [
            Name: String { get: VisualState::name, set: |this: &Ref<VisualState>, value: String| this.set_name(value) },
            Setters: FerroList<Rc<dyn SetterBase>> { get: VisualState::setters },
        ],
    },
});
ferro_impl_classes!(VisualState: FerroObjectImpl);

impl VisualState {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), name: RefCell::new(String::new()), setters: FerroList::new() }
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
}
