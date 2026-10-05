//! Port of `Xaml/InitializationOrderTracker.cs`.

use std::cell::{Cell, RefCell};

use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl,
    StyledElementImplExt, VisualImpl,
};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_controls::{Control, ControlImpl};

/// A control that records the order of the initialisation calls it
/// receives.
#[repr(C)]
pub struct InitializationOrderTracker {
    base: Control,
    order: RefCell<Vec<String>>,
    init_state: Cell<i32>,
}

ferro_class!(InitializationOrderTracker: Control);
ferro_impl_classes!(InitializationOrderTracker: VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferro_class_info!(InitializationOrderTracker {
    new: InitializationOrderTracker::new,
    markup: { namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml" },
});

impl FerroObjectImpl for InitializationOrderTracker {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        this.order.borrow_mut().push(format!("Property {} Changed", change.property().name()));
        Self::parent_on_property_changed(this, change);
    }
}

impl StyledElementImpl for InitializationOrderTracker {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        this.order.borrow_mut().push("AttachedToLogicalTree".to_string());
        Self::parent_on_attached_to_logical_tree(this, e);
    }

    fn begin_init(this: &Self) {
        this.init_state.set(this.init_state.get() + 1);
        Self::parent_begin_init(this);
        this.order.borrow_mut().push(format!("BeginInit {}", this.init_state.get()));
    }

    fn try_end_init(this: &Self) -> Result<(), ferroui_base::InitializationError> {
        this.init_state.set(this.init_state.get() - 1);
        Self::parent_try_end_init(this)?;
        this.order.borrow_mut().push(format!("EndInit {}", this.init_state.get()));
        Ok(())
    }
}

impl InitializationOrderTracker {
    pub fn construct() -> Self {
        Self { base: Control::construct(), order: RefCell::new(Vec::new()), init_state: Cell::new(0) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The calls received so far, in order.
    pub fn order(&self) -> Vec<String> {
        self.order.borrow().clone()
    }

    /// `Order.IndexOf(entry)`: -1 when the entry is missing.
    pub fn index_of(&self, entry: &str) -> i32 {
        self.order.borrow().iter().position(|item| item == entry).map_or(-1, |index| index as i32)
    }

    pub fn init_state(&self) -> i32 {
        self.init_state.get()
    }
}
