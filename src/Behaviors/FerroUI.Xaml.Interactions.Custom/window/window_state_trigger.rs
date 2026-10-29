//! Port of `Window/WindowStateTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBase, AttachedToVisualTreeTriggerBaseImpl};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObject,
    FerroObjectExtensions, FerroObjectImpl, FerroProperty, Nullable, Ref, StaticType, StyledElementImpl,
    StyledProperty,
};
use ferroui_controls::{Control, TopLevel, Window, WindowState};
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingTriggerImpl, Interaction, StyledElementBehaviorImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Executes actions when the window state matches the specified value.
///
/// The state is the parameter of the actions.
#[repr(C)]
pub struct WindowStateTrigger {
    base: AttachedToVisualTreeTriggerBase,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(WindowStateTrigger: AttachedToVisualTreeTriggerBase);
ferro_class_info!(WindowStateTrigger {
    new: WindowStateTrigger::new,
    markup: {
        property_attributes: [
            Window: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    WindowStateTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
);

ferro_properties! {
    impl WindowStateTrigger {
        /// Identifies the `Window` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the trigger does not own the window, so the
        /// value is an element reference.
        pub fn window_property() -> StyledProperty<Option<ElementRef<Window>>> {
            ValueTypes::register_element_ref::<Window>();
            FerroProperty::register::<WindowStateTrigger, _>("Window", None)
        }

        /// Identifies the `State` property.
        pub fn state_property() -> StyledProperty<WindowState> {
            FerroProperty::register::<WindowStateTrigger, _>("State", WindowState::Normal)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeTriggerBaseImpl for WindowStateTrigger {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let window = match this.window() {
                Some(window) => Some(window),
                None => {
                    let associated_object = this.associated_object_as::<Control>();
                    TopLevel::get_top_level(associated_object.as_deref().map(crate::part_c::control_as_visual))
                        .and_then(|top_level| top_level.cast::<Window>())
                }
            };
            let Some(window) = window else {
                return DisposableAction::empty();
            };

            let trigger = this.to_ref().downgrade();
            let window_object: &FerroObject = &window;
            let subscription = FerroObjectExtensions::get_observable(window_object, Window::window_state_property())
                .subscribe_fn(move |state: WindowState| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.on_state_changed(state);
                    }
                });
            *this.subscription.borrow_mut() = Some(subscription);
            this.on_state_changed(window.window_state());

            let trigger = this.to_ref().downgrade();
            DisposableAction::create(move || {
                if let Some(trigger) = trigger.upgrade() {
                    let subscription = trigger.subscription.borrow().clone();
                    if let Some(subscription) = subscription {
                        subscription.dispose();
                    }
                }
            })
        }
    }
}

impl WindowStateTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: AttachedToVisualTreeTriggerBase::construct(<Control as StaticType>::TYPE),
            subscription: RefCell::new(None),
        }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the window. If not set, the visual root window is used. Markup resolves a name
    /// given for it to the named window.
    pub fn window(&self) -> Option<Ref<Window>> {
        ElementRef::resolve(&self.get_value(Self::window_property()))
    }

    pub fn set_window(&self, value: impl Into<Nullable<Window>>) {
        self.set_value(Self::window_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the window state to trigger on. This is a property of the object model.
    pub fn state(&self) -> WindowState {
        self.get_value(Self::state_property())
    }

    pub fn set_state(&self, value: WindowState) {
        self.set_value(Self::state_property(), value)
    }

    fn on_state_changed(&self, state: WindowState) {
        if !self.is_enabled() || state != self.state() {
            return;
        }

        let trigger: Ref<WindowStateTrigger> = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || {
                let sender =
                    trigger.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
                let parameter: BoxedValue = Rc::new(state);
                Interaction::execute_actions(sender.as_ref(), trigger.actions().as_ref(), Some(&parameter));
            },
            DispatcherPriority::DEFAULT,
        );
    }
}
