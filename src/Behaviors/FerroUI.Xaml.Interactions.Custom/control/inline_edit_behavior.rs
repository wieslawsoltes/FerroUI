//! Port of `Control/InlineEditBehavior.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{FocusChangedEventArgs, InputElement, Key, KeyEventArgs, TappedEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectImpl, FerroProperty, Nullable,
    Ref, StaticType, StyledElementImpl, StyledProperty, WeakRef,
};
use ferroui_controls::{Control, TextBox};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::RefCell;

/// Edits in place: a display control is swapped for an edit control when the display control is
/// double tapped (or the associated control, with
/// [`edit_on_associated_object_double_tapped`](Self::edit_on_associated_object_double_tapped)) or
/// the edit key is pressed on it, and swapped back when the accept or the cancel key is pressed in
/// the edit control or it loses the focus.
#[repr(C)]
pub struct InlineEditBehavior {
    base: StyledElementBehaviorOfT,
    // The handlers added to the controls, with the control and the event of each (the reference
    // removes its delegates from the controls the properties name).
    handlers: RefCell<Vec<(WeakRef<Control>, InlineEditHandler, RoutedEventHandlerToken)>>,
}

/// The event a handler of [`InlineEditBehavior`] is added to.
#[derive(Clone, Copy)]
enum InlineEditHandler {
    DoubleTapped,
    KeyDown,
    LostFocus,
}

ferro_class!(InlineEditBehavior: StyledElementBehaviorOfT);
ferro_class_info!(InlineEditBehavior {
    new: InlineEditBehavior::new,
    markup: {
        property_attributes: [
            EditControl: [ResolveByName],
            DisplayControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(InlineEditBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl InlineEditBehavior {
        /// Identifies the `EditControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the control, so the
        /// value is an element reference.
        pub fn edit_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<InlineEditBehavior, _>("EditControl", None)
        }

        /// Identifies the `DisplayControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the control, so the
        /// value is an element reference.
        pub fn display_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<InlineEditBehavior, _>("DisplayControl", None)
        }

        /// Identifies the `EditKey` property.
        pub fn edit_key_property() -> StyledProperty<Key> {
            FerroProperty::register::<InlineEditBehavior, _>("EditKey", Key::F2)
        }

        /// Identifies the `AcceptKey` property.
        pub fn accept_key_property() -> StyledProperty<Key> {
            FerroProperty::register::<InlineEditBehavior, _>("AcceptKey", Key::Enter)
        }

        /// Identifies the `CancelKey` property.
        pub fn cancel_key_property() -> StyledProperty<Key> {
            FerroProperty::register::<InlineEditBehavior, _>("CancelKey", Key::Escape)
        }

        /// Identifies the `EditOnAssociatedObjectDoubleTapped` property.
        pub fn edit_on_associated_object_double_tapped_property() -> StyledProperty<bool> {
            FerroProperty::register::<InlineEditBehavior, _>("EditOnAssociatedObjectDoubleTapped", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for InlineEditBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            if let Some(display_control) = this.display_control() {
                let behavior = this.to_ref().downgrade();
                let token = display_control.add_handler_with(
                    InputElement::double_tapped_event(),
                    move |_sender: &Interactive, _e: &TappedEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.begin_edit();
                        }
                    },
                    RoutingStrategies::BUBBLE,
                    false,
                );
                this.track(&display_control, InlineEditHandler::DoubleTapped, token);

                let behavior = this.to_ref().downgrade();
                let token = display_control.add_handler_with(
                    InputElement::key_down_event(),
                    move |_sender: &Interactive, e: &KeyEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.on_display_key_down(e);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    false,
                );
                this.track(&display_control, InlineEditHandler::KeyDown, token);
            }

            if this.edit_on_associated_object_double_tapped() {
                if let Some(associated_object) = this.associated_object_as::<Control>() {
                    let behavior = this.to_ref().downgrade();
                    let token = associated_object.add_handler_with(
                        InputElement::double_tapped_event(),
                        move |_sender: &Interactive, _e: &TappedEventArgs| {
                            if let Some(behavior) = behavior.upgrade() {
                                behavior.begin_edit();
                            }
                        },
                        RoutingStrategies::BUBBLE,
                        false,
                    );
                    this.track(&associated_object, InlineEditHandler::DoubleTapped, token);
                }
            }

            if let Some(edit_control) = this.edit_control() {
                let behavior = this.to_ref().downgrade();
                let token = edit_control.add_handler_with(
                    InputElement::key_down_event(),
                    move |_sender: &Interactive, e: &KeyEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.on_edit_key_down(e);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    false,
                );
                this.track(&edit_control, InlineEditHandler::KeyDown, token);

                let behavior = this.to_ref().downgrade();
                let token = edit_control.add_handler_with(
                    InputElement::lost_focus_event(),
                    move |_sender: &Interactive, _e: &FocusChangedEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.end_edit();
                        }
                    },
                    RoutingStrategies::BUBBLE,
                    false,
                );
                this.track(&edit_control, InlineEditHandler::LostFocus, token);

                edit_control.set_is_visible(false);
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let handlers = std::mem::take(&mut *this.handlers.borrow_mut());
            for (control, handler, token) in handlers {
                let Some(control) = control.upgrade() else {
                    continue;
                };
                match handler {
                    InlineEditHandler::DoubleTapped => {
                        control.remove_handler(InputElement::double_tapped_event(), token);
                    }
                    InlineEditHandler::KeyDown => {
                        control.remove_handler(InputElement::key_down_event(), token);
                    }
                    InlineEditHandler::LostFocus => {
                        control.remove_handler(InputElement::lost_focus_event(), token);
                    }
                }
            }
        }
    }
}

impl InlineEditBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            handlers: RefCell::new(Vec::new()),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the control used for editing. Markup resolves a name given for it to the
    /// named element. This is a property of the object model.
    pub fn edit_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::edit_control_property()))
    }

    pub fn set_edit_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::edit_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the control that displays the value. Markup resolves a name given for it to
    /// the named element. This is a property of the object model.
    pub fn display_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::display_control_property()))
    }

    pub fn set_display_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::display_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the key that begins the edit in the display control. This is a property of
    /// the object model.
    pub fn edit_key(&self) -> Key {
        self.get_value(Self::edit_key_property())
    }

    pub fn set_edit_key(&self, value: Key) {
        self.set_value(Self::edit_key_property(), value)
    }

    /// Gets or sets the key that accepts the edit. This is a property of the object model.
    pub fn accept_key(&self) -> Key {
        self.get_value(Self::accept_key_property())
    }

    pub fn set_accept_key(&self, value: Key) {
        self.set_value(Self::accept_key_property(), value)
    }

    /// Gets or sets the key that cancels the edit. This is a property of the object model.
    pub fn cancel_key(&self) -> Key {
        self.get_value(Self::cancel_key_property())
    }

    pub fn set_cancel_key(&self, value: Key) {
        self.set_value(Self::cancel_key_property(), value)
    }

    /// Gets or sets whether a double tap on the associated control begins the edit. This is a
    /// property of the object model.
    pub fn edit_on_associated_object_double_tapped(&self) -> bool {
        self.get_value(Self::edit_on_associated_object_double_tapped_property())
    }

    pub fn set_edit_on_associated_object_double_tapped(&self, value: bool) {
        self.set_value(Self::edit_on_associated_object_double_tapped_property(), value)
    }

    fn track(&self, control: &Ref<Control>, handler: InlineEditHandler, token: RoutedEventHandlerToken) {
        self.handlers.borrow_mut().push((control.downgrade(), handler, token));
    }

    fn on_display_key_down(&self, e: &KeyEventArgs) {
        if e.key == self.edit_key() {
            self.begin_edit();
            e.set_handled(true);
        }
    }

    fn on_edit_key_down(&self, e: &KeyEventArgs) {
        if e.key == self.accept_key() || e.key == self.cancel_key() {
            self.end_edit();
        }
    }

    fn begin_edit(&self) {
        let (Some(edit_control), Some(display_control)) = (self.edit_control(), self.display_control()) else {
            return;
        };
        if edit_control.is_visible() {
            return;
        }

        display_control.set_is_visible(false);
        edit_control.set_is_visible(true);
        edit_control.focus();
        if let Some(text_box) = edit_control.cast::<TextBox>() {
            text_box.select_all();
        }
    }

    fn end_edit(&self) {
        let (Some(edit_control), Some(display_control)) = (self.edit_control(), self.display_control()) else {
            return;
        };

        edit_control.set_is_visible(false);
        display_control.set_is_visible(true);
    }
}
