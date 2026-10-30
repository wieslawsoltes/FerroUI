//! Port of `Dialog/DialogClosedTrigger.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::EventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::{Control, Window};
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::cell::RefCell;
use std::rc::Rc;

/// Executes actions when the dialog window is closed.
///
/// The trigger listens to the window [`source_object`](Self::source_object) while the
/// associated control is in the visual tree; the actions run with the args of the event
/// ([`EventArgs`]) as the parameter.
#[repr(C)]
pub struct DialogClosedTrigger {
    base: StyledElementTriggerOfT,
    // The subscription to the event of the window (the reference removes its delegate).
    // Deviation (DEVIATIONS.md, Behaviors): the subscription ends on the window it was made on; the
    // reference removes its delegate from the current source object.
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(DialogClosedTrigger: StyledElementTriggerOfT);
ferro_class_info!(DialogClosedTrigger {
    new: DialogClosedTrigger::new,
    markup: {
        property_attributes: [
            SourceObject: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(DialogClosedTrigger: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl DialogClosedTrigger {
        /// Identifies the `SourceObject` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the trigger does not own the window, so the value
        /// is an element reference.
        pub fn source_object_property() -> StyledProperty<Option<ElementRef<Window>>> {
            ValueTypes::register_element_ref::<Window>();
            FerroProperty::register::<DialogClosedTrigger, _>("SourceObject", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for DialogClosedTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            if let Some(window) = this.source_object() {
                let trigger = this.to_ref().downgrade();
                let subscription = window.closed(move || {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.on_closed();
                    }
                });
                let previous = this.subscription.borrow_mut().replace(subscription);
                if let Some(previous) = previous {
                    previous.dispose();
                }
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let subscription = this.subscription.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        }
    }
}

impl DialogClosedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTriggerOfT::construct(<Control as StaticType>::TYPE), subscription: RefCell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the source object from which this behavior listens for events. This is a
    /// property of the object model. Markup resolves a name given for it to the named element.
    pub fn source_object(&self) -> Option<Ref<Window>> {
        ElementRef::resolve(&self.get_value(Self::source_object_property()))
    }

    pub fn set_source_object(&self, value: impl Into<Nullable<Window>>) {
        self.set_value(Self::source_object_property(), ElementRef::from_nullable(value.into().0))
    }

    fn on_closed(&self) {
        let e: BoxedValue = Rc::new(EventArgs::EMPTY);
        self.execute(Some(&e));
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if !self.is_enabled() {
            return;
        }

        if let Some(associated_object) = self.associated_object() {
            let sender: BoxedValue = Rc::new(associated_object);
            Interaction::execute_actions(Some(&sender), self.actions().as_ref(), parameter);
        }
    }
}
