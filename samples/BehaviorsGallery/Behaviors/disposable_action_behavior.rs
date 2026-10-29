//! Port of `Behaviors/DisposableActionBehavior.cs` of the reference sample.

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
    DisposableAction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt,
    StyledElementBehaviorOfT,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Writes `Pressed` to its target text block when its control is pressed; the handler is removed
/// by a disposable action when the behavior is detached
/// (`DisposableActionBehavior : StyledElementBehavior<Control>`).
#[repr(C)]
pub struct DisposableActionBehavior {
    base: StyledElementBehaviorOfT,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(DisposableActionBehavior: StyledElementBehaviorOfT);
ferro_class_info!(DisposableActionBehavior {
    new: DisposableActionBehavior::new,
    markup: {
        property_attributes: [
            Target: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(DisposableActionBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl DisposableActionBehavior {
        /// Identifies the `Target` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors gallery): the behavior does not own the text block,
        /// so the value is an element reference.
        pub fn target_property() -> StyledProperty<Option<ElementRef<TextBlock>>> {
            ValueTypes::register_element_ref::<TextBlock>();
            FerroProperty::register::<DisposableActionBehavior, _>("Target", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for DisposableActionBehavior {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
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
            *this.disposable.borrow_mut() = Some(DisposableAction::create(move || {
                if let Some(associated_object) = associated_object.upgrade() {
                    associated_object.remove_handler(&InputElement::pointer_pressed_event(), handler);
                }
            }));
        }

        fn on_detaching(this: &Self) {
            let disposable = this.disposable.borrow_mut().take();
            if let Some(disposable) = disposable {
                disposable.dispose();
            }
            Self::parent_on_detaching(this);
        }
    }
}

impl DisposableActionBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            disposable: RefCell::new(None),
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
