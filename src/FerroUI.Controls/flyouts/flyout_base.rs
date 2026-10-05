use crate::Control;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_property, AttachedProperty, DirectProperty, ElementRef, FerroObject, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledProperty,
};
use std::cell::RefCell;
use std::rc::Rc;

/// The base class of flyouts.
///
/// This class is abstract: only its subclasses can be created.
#[repr(C)]
pub struct FlyoutBase {
    base: FerroObject,
    /// The flyout does not own its target: the reference is weak.
    target: RefCell<Option<ElementRef<Control>>>,
    opened: HandlerList<dyn Fn()>,
    closed: HandlerList<dyn Fn()>,
}

ferro_class! {
    FlyoutBase: FerroObject, virtuals FlyoutBaseImpl: FerroObjectImpl {
        /// Shows the flyout at the given control.
        fn show_at(this, placement_target: &Control);
        /// Hides the flyout.
        fn hide(this);
        /// Raises the `Opened` event.
        fn on_opened(this);
        /// Raises the `Closed` event.
        fn on_closed(this);
    }
}

impl FerroObjectImpl for FlyoutBase {}

impl FlyoutBaseImpl for FlyoutBase {
    fn show_at(_this: &Self, _placement_target: &Control) {
        panic!("FlyoutBase is abstract.")
    }

    fn hide(_this: &Self) {
        panic!("FlyoutBase is abstract.")
    }

    fn on_opened(this: &Self) {
        for (_, handler) in this.opened.snapshot().iter() {
            handler();
        }
    }

    fn on_closed(this: &Self) {
        for (_, handler) in this.closed.snapshot().iter() {
            handler();
        }
    }
}

ferroui_base::ferro_properties! { impl FlyoutBase {
    ferro_property!(
        /// Defines the `IsOpen` property.
        pub fn is_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<FlyoutBase, _>("IsOpen", false)
        }
    );

    ferro_property!(
        /// Defines the `Target` property.
        pub fn target_property() -> DirectProperty<FlyoutBase, Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register_direct::<FlyoutBase, _>("Target", |x| x.target.borrow().clone(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `AttachedFlyout` attached property.
        pub fn attached_flyout_property() -> AttachedProperty<Option<Ref<FlyoutBase>>> {
            FerroProperty::register_attached::<FlyoutBase, Control, _>("AttachedFlyout", None)
        }
    );
} }

impl FlyoutBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            target: RefCell::new(None),
            opened: HandlerList::new(),
            closed: HandlerList::new(),
        }
    }

    fn subscribe(&self, select: fn(&FlyoutBase) -> &HandlerList<dyn Fn()>, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = select(self).add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                select(&this).remove(token);
            }
        })
    }

    /// Raised when the flyout is opened. Disposing the returned handle
    /// unsubscribes.
    pub fn opened(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscribe(|flyout| &flyout.opened, Rc::new(handler))
    }

    /// Raised when the flyout is closed. Disposing the returned handle
    /// unsubscribes.
    pub fn closed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscribe(|flyout| &flyout.closed, Rc::new(handler))
    }

    /// Whether this flyout is currently open.
    ///
    /// Setting this property to `true` will show the flyout at the last
    /// known placement target. If no target has been set via `show_at`,
    /// setting this to `true` will have no effect.
    pub fn is_open(&self) -> bool {
        self.get_value(Self::is_open_property())
    }

    pub fn set_is_open(&self, value: bool) {
        self.set_value(Self::is_open_property(), value)
    }

    /// The target used for showing the flyout.
    pub fn target(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.target.borrow())
    }

    /// Sets the target used for showing the flyout. For derived classes.
    pub fn set_target(&self, value: impl Into<Nullable<Control>>) {
        self.set_and_raise(Self::target_property(), &self.target, ElementRef::from_nullable(value.into().0));
    }

    /// Gets the flyout attached to `element`.
    pub fn get_attached_flyout(element: &Control) -> Option<Ref<FlyoutBase>> {
        element.get_value(Self::attached_flyout_property())
    }

    /// Attaches a flyout to `element`.
    pub fn set_attached_flyout(element: &Control, value: impl Into<Nullable<FlyoutBase>>) {
        element.set_value(Self::attached_flyout_property(), value.into().0)
    }

    /// Shows the flyout attached to `flyout_owner`, if any, at it.
    pub fn show_attached_flyout(flyout_owner: &Control) {
        let flyout = Self::get_attached_flyout(flyout_owner);
        if let Some(flyout) = flyout {
            flyout.show_at(flyout_owner);
        }
    }
}
