use crate::primitives::{TemplatedControl, TemplatedControlImpl};
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, FerroObjectImpl, StyledElementImpl, VisualImpl};
use std::rc::Rc;

/// Defines the base class for the date and time picker presenters.
#[repr(C)]
pub struct PickerPresenterBase {
    base: TemplatedControl,
    confirmed: HandlerList<dyn Fn()>,
    dismissed: HandlerList<dyn Fn()>,
}

ferro_class! {
    PickerPresenterBase: TemplatedControl, virtuals PickerPresenterBaseImpl: TemplatedControlImpl {
        /// Raises the `Confirmed` event.
        fn on_confirmed(this);
        /// Raises the `Dismissed` event.
        fn on_dismiss(this);
    }
}

ferro_class_info!(PickerPresenterBase {});

ferro_impl_classes!(
    PickerPresenterBase: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl PickerPresenterBaseImpl for PickerPresenterBase {
    fn on_confirmed(this: &Self) {
        for (_, handler) in this.confirmed.snapshot().iter() {
            handler();
        }
    }

    fn on_dismiss(this: &Self) {
        for (_, handler) in this.dismissed.snapshot().iter() {
            handler();
        }
    }
}

impl PickerPresenterBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TemplatedControl::construct(), confirmed: HandlerList::new(), dismissed: HandlerList::new() }
    }

    /// Raised when the selection of the presenter is confirmed. Disposing
    /// the returned handle removes the handler.
    pub fn confirmed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.confirmed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.confirmed.remove(token);
            }
        })
    }

    /// Raised when the presenter is dismissed. Disposing the returned handle
    /// removes the handler.
    pub fn dismissed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.dismissed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.dismissed.remove(token);
            }
        })
    }
}
