//! Port of `Behaviors/DisposingMessageBehavior.cs` of the reference sample.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{InputElement, PointerPressedEventArgs};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::{Control, TextBlock};
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl,
};
use std::rc::Rc;

/// Writes `Pressed` to its target text block when its control is pressed; the handler is removed
/// by the disposable of the behavior (`DisposingMessageBehavior : DisposingBehavior<Control>`).
#[repr(C)]
pub struct DisposingMessageBehavior {
    base: DisposingBehavior,
}

ferro_class!(DisposingMessageBehavior: DisposingBehavior);
ferro_class_info!(DisposingMessageBehavior {
    new: DisposingMessageBehavior::new,
    markup: {
        property_attributes: [
            Target: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    DisposingMessageBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
);

ferro_properties! {
    impl DisposingMessageBehavior {
        /// Identifies the `Target` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors gallery): the behavior does not own the text block,
        /// so the value is an element reference.
        pub fn target_property() -> StyledProperty<Option<ElementRef<TextBlock>>> {
            ValueTypes::register_element_ref::<TextBlock>();
            FerroProperty::register::<DisposingMessageBehavior, _>("Target", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for DisposingMessageBehavior {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                &InputElement::pointer_pressed_event(),
                move |_sender: &Interactive, _e: &PointerPressedEventArgs| {
                    if let Some(target) = behavior.upgrade().and_then(|behavior| behavior.target()) {
                        target.set_text(Some("Pressed"));
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );

            let associated_object = associated_object.downgrade();
            DisposableAction::create(move || {
                if let Some(associated_object) = associated_object.upgrade() {
                    associated_object.remove_handler(&InputElement::pointer_pressed_event(), handler);
                }
            })
        }
    }
}

impl DisposingMessageBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: DisposingBehavior::construct(<Control as StaticType>::TYPE),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// `Target`: the text block the behavior writes to. Markup resolves a name given for it.
    pub fn target(&self) -> Option<Ref<TextBlock>> {
        ElementRef::resolve(&self.get_value(Self::target_property()))
    }

    pub fn set_target(&self, value: impl Into<Nullable<TextBlock>>) {
        self.set_value(Self::target_property(), ElementRef::from_nullable(value.into().0))
    }
}
