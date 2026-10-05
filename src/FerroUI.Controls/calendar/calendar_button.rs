// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

use super::{Calendar, DateTimeHelper};
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl};
use crate::{Button, ButtonImpl, ContentControl, ContentControlImpl, ControlImpl};
use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, MouseButton, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    Ref, StyledElementImpl, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Represents a button on a [`Calendar`].
#[repr(C)]
pub struct CalendarButton {
    base: Button,

    /// A value indicating whether the button is focused.
    is_calendar_button_focused: Cell<bool>,

    /// A value indicating whether the button is inactive.
    is_inactive: Cell<bool>,

    /// A value indicating whether the button is selected.
    is_selected: Cell<bool>,

    /// The Calendar associated with this button.
    owner: RefCell<Option<WeakRef<Calendar>>>,

    calendar_left_mouse_button_down: HandlerList<dyn Fn(&CalendarButton, &PointerPressedEventArgs)>,
    calendar_left_mouse_button_up: HandlerList<dyn Fn(&CalendarButton, &PointerReleasedEventArgs)>,
}

ferro_class!(CalendarButton: Button);

ferro_class_info!(CalendarButton {
    new: CalendarButton::new,
    markup: {
        attributes: [PseudoClasses(":selected", ":inactive", ":btnfocused")],
    },
});

ferro_impl_classes!(
    CalendarButton: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    ContentControlImpl,
    ButtonImpl
);

impl FerroObjectImpl for CalendarButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let content: BoxedValue = Rc::new(DateTimeHelper::get_current_date_format().abbreviated_month_names()[0].clone());
        this.set_current_value(ContentControl::content_property(), Some(content));
    }
}

impl InputElementImpl for CalendarButton {
    /// Provides class handling for the pointer pressed event that occurs
    /// when the left mouse button is pressed while the mouse pointer is
    /// over this control.
    ///
    /// This method marks the event as handled and cannot be handled by
    /// subscribing to the event through the routed event of the control.
    /// Subscribe through [`CalendarButton::calendar_left_mouse_button_down`]
    /// instead.
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);

        if e.get_current_point(Some(this)).properties.is_left_button_pressed {
            for (_, handler) in this.calendar_left_mouse_button_down.snapshot().iter() {
                handler(this, e);
            }
        }
    }

    /// Provides handling for the pointer released event that occurs when
    /// the left mouse button is released while the mouse pointer is over
    /// this control.
    ///
    /// This method marks the event as handled and cannot be handled by
    /// subscribing to the event through the routed event of the control.
    /// Subscribe through [`CalendarButton::calendar_left_mouse_button_up`]
    /// instead.
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);

        if e.initial_press_mouse_button() == MouseButton::Left {
            for (_, handler) in this.calendar_left_mouse_button_up.snapshot().iter() {
                handler(this, e);
            }
        }
    }
}

impl TemplatedControlImpl for CalendarButton {
    /// Builds the visual tree for the [`CalendarButton`] when a new
    /// template is applied.
    fn on_apply_template(this: &Self, _e: &TemplateAppliedEventArgs) {
        this.set_pseudo_classes();
    }
}

impl CalendarButton {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Button::construct(),
            is_calendar_button_focused: Cell::new(false),
            is_inactive: Cell::new(false),
            is_selected: Cell::new(false),
            owner: RefCell::new(None),
            calendar_left_mouse_button_down: HandlerList::new(),
            calendar_left_mouse_button_up: HandlerList::new(),
        }
    }

    /// Initializes a new instance of the [`CalendarButton`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the Calendar associated with this button.
    #[allow(dead_code)] // the reference declares the getter and reads it nowhere
    pub(crate) fn owner(&self) -> Option<Ref<Calendar>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Sets the Calendar associated with this button.
    pub(crate) fn set_owner(&self, value: Option<&Ref<Calendar>>) {
        *self.owner.borrow_mut() = value.map(Ref::downgrade);
    }

    /// Gets a value indicating whether the button is focused.
    pub(crate) fn is_calendar_button_focused(&self) -> bool {
        self.is_calendar_button_focused.get()
    }

    /// Sets a value indicating whether the button is focused.
    pub(crate) fn set_is_calendar_button_focused(&self, value: bool) {
        if self.is_calendar_button_focused.get() != value {
            self.is_calendar_button_focused.set(value);
            self.set_pseudo_classes();
        }
    }

    /// Gets a value indicating whether the button is inactive.
    pub(crate) fn is_inactive(&self) -> bool {
        self.is_inactive.get()
    }

    /// Sets a value indicating whether the button is inactive.
    pub(crate) fn set_is_inactive(&self, value: bool) {
        if self.is_inactive.get() != value {
            self.is_inactive.set(value);
            self.set_pseudo_classes();
        }
    }

    /// Gets a value indicating whether the button is selected.
    pub(crate) fn is_selected(&self) -> bool {
        self.is_selected.get()
    }

    /// Sets a value indicating whether the button is selected.
    pub(crate) fn set_is_selected(&self, value: bool) {
        if self.is_selected.get() != value {
            self.is_selected.set(value);
            self.set_pseudo_classes();
        }
    }

    /// Sets the pseudo classes of the button from its state.
    fn set_pseudo_classes(&self) {
        self.pseudo_classes().set(":selected", self.is_selected());
        self.pseudo_classes().set(":inactive", self.is_inactive());
        self.pseudo_classes().set(":btnfocused", self.is_calendar_button_focused() && self.is_enabled());
    }

    /// Occurs when the left mouse button is pressed (or when the tip of the
    /// stylus touches the tablet PC) while the mouse pointer is over a UI
    /// element. The handler receives the button that raises the event.
    pub fn calendar_left_mouse_button_down(
        &self,
        handler: impl Fn(&CalendarButton, &PointerPressedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.calendar_left_mouse_button_down.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.calendar_left_mouse_button_down.remove(token);
            }
        })
    }

    /// Occurs when the left mouse button is released (or the tip of the
    /// stylus is removed from the tablet PC) while the mouse (or the
    /// stylus) is over a UI element (or while a UI element holds mouse
    /// capture). The handler receives the button that raises the event.
    pub fn calendar_left_mouse_button_up(
        &self,
        handler: impl Fn(&CalendarButton, &PointerReleasedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.calendar_left_mouse_button_up.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.calendar_left_mouse_button_up.remove(token);
            }
        })
    }
}
