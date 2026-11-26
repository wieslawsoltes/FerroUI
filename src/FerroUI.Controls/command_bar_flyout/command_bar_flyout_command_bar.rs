//! WinUI's `CommandBarFlyoutCommandBar` (`controls/dev/CommandBarFlyout/CommandBarFlyoutCommandBar.cpp`
//! of microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::CommandBarFlyout;
use crate::command_bar::CommandBar;
use crate::primitives::TemplatedControlImpl;
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl, WeakRef,
};
use std::cell::RefCell;

/// The command bar of a [`CommandBarFlyout`] (WinUI's `CommandBarFlyoutCommandBar`, in
/// `Microsoft.UI.Xaml.Controls.Primitives`): a [`CommandBar`] whose theme shows the primary
/// commands as a row of icons and the secondary commands as the menu below them.
#[repr(C)]
pub struct CommandBarFlyoutCommandBar {
    base: CommandBar,
    owning_flyout: RefCell<Option<WeakRef<CommandBarFlyout>>>,
}

ferro_class!(CommandBarFlyoutCommandBar: CommandBar);
ferro_class_info!(CommandBarFlyoutCommandBar {
    new: CommandBarFlyoutCommandBar::new,
    markup: { namespace: "FerroUI.Controls.Primitives" },
});
ferro_impl_classes!(
    CommandBarFlyoutCommandBar: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl CommandBarFlyoutCommandBar {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: CommandBar::construct(), owning_flyout: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The flyout that shows the bar (`SetOwningFlyout`).
    pub fn owning_flyout(&self) -> Option<Ref<CommandBarFlyout>> {
        self.owning_flyout.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(super) fn set_owning_flyout(&self, flyout: &Ref<CommandBarFlyout>) {
        *self.owning_flyout.borrow_mut() = Some(flyout.downgrade());
    }
}
