//! Port of `Control/ShowPointerPositionBehavior.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{InputElement, PointerEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectImpl, FerroProperty, Nullable,
    Ref, StaticType, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::{Control, TextBlock};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::Cell;

/// Shows the position of the pointer over the associated control in the target text block.
#[repr(C)]
pub struct ShowPointerPositionBehavior {
    base: StyledElementBehaviorOfT,
    // The handler added to the associated control (the reference removes its delegate).
    pointer_moved_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(ShowPointerPositionBehavior: StyledElementBehaviorOfT);
ferro_class_info!(ShowPointerPositionBehavior {
    new: ShowPointerPositionBehavior::new,
    markup: {
        property_attributes: [
            TargetTextBlock: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ShowPointerPositionBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ShowPointerPositionBehavior {
        /// Identifies the `TargetTextBlock` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the text block, so the
        /// value is an element reference.
        pub fn target_text_block_property() -> StyledProperty<Option<ElementRef<TextBlock>>> {
            ValueTypes::register_element_ref::<TextBlock>();
            FerroProperty::register::<ShowPointerPositionBehavior, _>("TargetTextBlock", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ShowPointerPositionBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler(
                InputElement::pointer_moved_event(),
                move |_sender: &Interactive, e: &PointerEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.associated_object_pointer_moved(e);
                    }
                },
            );
            this.pointer_moved_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            if let Some(handler) = this.pointer_moved_handler.take() {
                associated_object.remove_handler(InputElement::pointer_moved_event(), handler);
            }
        }
    }
}

impl ShowPointerPositionBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            pointer_moved_handler: Cell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the text block that shows the position. Markup resolves a name given for it
    /// to the named element. This is a property of the object model.
    pub fn target_text_block(&self) -> Option<Ref<TextBlock>> {
        ElementRef::resolve(&self.get_value(Self::target_text_block_property()))
    }

    pub fn set_target_text_block(&self, value: impl Into<Nullable<TextBlock>>) {
        self.set_value(Self::target_text_block_property(), ElementRef::from_nullable(value.into().0))
    }

    fn associated_object_pointer_moved(&self, e: &PointerEventArgs) {
        let Some(target_text_block) = self.target_text_block() else {
            return;
        };

        let associated_object = self.associated_object_as::<Control>();
        let relative_to = associated_object.as_ref().map(|associated_object| {
            let visual: &Visual = associated_object;
            visual
        });
        target_text_block.set_text(Some(e.get_position(relative_to).to_string().as_str()));
    }
}
