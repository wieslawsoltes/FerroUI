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

/// Default content for the CalendarDayButton.
const DEFAULT_CONTENT: i32 = 1;

/// Represents a day on a [`Calendar`].
#[repr(C)]
pub struct CalendarDayButton {
    base: Button,

    is_current: Cell<bool>,
    ignoring_mouse_over_state: Cell<bool>,
    is_blackout: Cell<bool>,
    is_today: Cell<bool>,
    is_inactive: Cell<bool>,
    is_selected: Cell<bool>,

    /// The Calendar associated with this button.
    owner: RefCell<Option<WeakRef<Calendar>>>,
    index: Cell<i32>,

    calendar_day_button_mouse_down: HandlerList<dyn Fn(&CalendarDayButton, &PointerPressedEventArgs)>,
    calendar_day_button_mouse_up: HandlerList<dyn Fn(&CalendarDayButton, &PointerReleasedEventArgs)>,
}

ferro_class!(CalendarDayButton: Button);

ferro_class_info!(CalendarDayButton {
    new: CalendarDayButton::new,
    markup: {
        attributes: [PseudoClasses(":pressed", ":disabled", ":selected", ":inactive", ":today", ":blackout", ":dayfocused")],
    },
});

ferro_impl_classes!(
    CalendarDayButton: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    ContentControlImpl,
    ButtonImpl
);

impl FerroObjectImpl for CalendarDayButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let content: BoxedValue = Rc::new(DateTimeHelper::format_number(DEFAULT_CONTENT));
        this.set_current_value(ContentControl::content_property(), Some(content));
    }
}

impl InputElementImpl for CalendarDayButton {
    /// Provides class handling for the pointer pressed event that occurs
    /// when the left mouse button is pressed while the mouse pointer is
    /// over this control.
    ///
    /// This method marks the event as handled and cannot be handled by
    /// subscribing to the event through the routed event of the control.
    /// Subscribe through [`CalendarDayButton::calendar_day_button_mouse_down`]
    /// instead.
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);

        if e.get_current_point(Some(this)).properties.is_left_button_pressed {
            for (_, handler) in this.calendar_day_button_mouse_down.snapshot().iter() {
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
    /// Subscribe through [`CalendarDayButton::calendar_day_button_mouse_up`]
    /// instead.
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);

        if e.initial_press_mouse_button() == MouseButton::Left {
            for (_, handler) in this.calendar_day_button_mouse_up.snapshot().iter() {
                handler(this, e);
            }
        }
    }
}

impl TemplatedControlImpl for CalendarDayButton {
    /// Builds the visual tree for the [`CalendarDayButton`] when a new
    /// template is applied.
    fn on_apply_template(this: &Self, _e: &TemplateAppliedEventArgs) {
        this.set_pseudo_classes();
    }

    // AUTOMATION-SEAM: OnCreateAutomationPeer -> CalendarDayButtonAutomationPeer (automation pass)
}

impl CalendarDayButton {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Button::construct(),
            is_current: Cell::new(false),
            ignoring_mouse_over_state: Cell::new(false),
            is_blackout: Cell::new(false),
            is_today: Cell::new(false),
            is_inactive: Cell::new(false),
            is_selected: Cell::new(false),
            owner: RefCell::new(None),
            index: Cell::new(0),
            calendar_day_button_mouse_down: HandlerList::new(),
            calendar_day_button_mouse_up: HandlerList::new(),
        }
    }

    /// Initializes a new instance of the [`CalendarDayButton`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the Calendar associated with this button.
    #[allow(dead_code)] // AUTOMATION-SEAM: read by the automation peers of the calendar (automation pass)
    pub(crate) fn owner(&self) -> Option<Ref<Calendar>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Sets the Calendar associated with this button.
    pub(crate) fn set_owner(&self, value: Option<&Ref<Calendar>>) {
        *self.owner.borrow_mut() = value.map(Ref::downgrade);
    }

    /// The index of the button among the children of the month view.
    pub(crate) fn index(&self) -> i32 {
        self.index.get()
    }

    pub(crate) fn set_index(&self, value: i32) {
        self.index.set(value);
    }

    /// Gets a value indicating whether the button is the focused element
    /// on the Calendar control.
    pub(crate) fn is_current(&self) -> bool {
        self.is_current.get()
    }

    /// Sets a value indicating whether the button is the focused element
    /// on the Calendar control.
    pub(crate) fn set_is_current(&self, value: bool) {
        if self.is_current.get() != value {
            self.is_current.set(value);
            self.set_pseudo_classes();
        }
    }

    /// Ensure the button is not in the MouseOver state.
    ///
    /// If a button is in the MouseOver state when a Popup is closed (as is
    /// the case when you select a date in the DatePicker control), it will
    /// continue to think it's in the mouse over state even when the Popup
    /// opens again and it's not.  This method is used to forcibly clear the
    /// state by changing the CommonStates state group.
    pub(crate) fn ignore_mouse_over_state(&self) {
        // TODO: Investigate whether this needs to be done by changing the
        // state everytime we change any state, or if it can be done once
        // to properly reset the control.

        self.ignoring_mouse_over_state.set(false);

        // If the button thinks it's in the MouseOver state (which can
        // happen when a Popup is closed before the button can change state)
        // we will override the state so it shows up as normal.
        if self.is_pointer_over() {
            self.ignoring_mouse_over_state.set(true);
            self.set_pseudo_classes();
        }
    }

    /// Gets a value indicating whether this is a blackout date.
    pub(crate) fn is_blackout(&self) -> bool {
        self.is_blackout.get()
    }

    /// Sets a value indicating whether this is a blackout date.
    pub(crate) fn set_is_blackout(&self, value: bool) {
        if self.is_blackout.get() != value {
            self.is_blackout.set(value);
            self.set_pseudo_classes();
        }
    }

    /// Gets a value indicating whether this button represents today.
    pub(crate) fn is_today(&self) -> bool {
        self.is_today.get()
    }

    /// Sets a value indicating whether this button represents today.
    pub(crate) fn set_is_today(&self, value: bool) {
        if self.is_today.get() != value {
            self.is_today.set(value);
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

    fn set_pseudo_classes(&self) {
        if self.ignoring_mouse_over_state.get() {
            self.pseudo_classes().set(":pressed", self.is_pressed());
            self.pseudo_classes().set(":disabled", !self.is_enabled());
        }

        self.pseudo_classes().set(":selected", self.is_selected());
        self.pseudo_classes().set(":inactive", self.is_inactive());
        self.pseudo_classes().set(":today", self.is_today());
        self.pseudo_classes().set(":blackout", self.is_blackout());
        self.pseudo_classes().set(":dayfocused", self.is_current() && self.is_enabled());
    }

    /// Occurs when the left mouse button is pressed (or when the tip of the
    /// stylus touches the tablet PC) while the mouse pointer is over a UI
    /// element. The handler receives the button that raises the event.
    pub fn calendar_day_button_mouse_down(
        &self,
        handler: impl Fn(&CalendarDayButton, &PointerPressedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.calendar_day_button_mouse_down.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.calendar_day_button_mouse_down.remove(token);
            }
        })
    }

    /// Occurs when the left mouse button is released (or the tip of the
    /// stylus is removed from the tablet PC) while the mouse (or the
    /// stylus) is over a UI element (or while a UI element holds mouse
    /// capture). The handler receives the button that raises the event.
    pub fn calendar_day_button_mouse_up(
        &self,
        handler: impl Fn(&CalendarDayButton, &PointerReleasedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.calendar_day_button_mouse_up.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.calendar_day_button_mouse_up.remove(token);
            }
        })
    }
}
