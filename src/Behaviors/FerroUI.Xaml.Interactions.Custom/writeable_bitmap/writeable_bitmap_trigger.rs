//! Port of `WriteableBitmap/WriteableBitmapTrigger.cs`.

use crate::WriteableBitmapRef;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTrigger};
use std::rc::Rc;

/// A trigger that executes its actions when [`trigger`](Self::trigger) is called, passing a
/// [`WriteableBitmap`](ferroui_base::media::imaging::WriteableBitmap) as parameter.
#[repr(C)]
pub struct WriteableBitmapTrigger {
    base: StyledElementTrigger,
}

ferro_class!(WriteableBitmapTrigger: StyledElementTrigger);
ferro_class_info!(WriteableBitmapTrigger {
    new: WriteableBitmapTrigger::new,
    markup: {
        property_attributes: [
            Bitmap: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(WriteableBitmapTrigger: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl WriteableBitmapTrigger {
        /// Identifies the `Bitmap` property.
        pub fn bitmap_property() -> StyledProperty<Option<WriteableBitmapRef>> {
            FerroProperty::register::<WriteableBitmapTrigger, _>("Bitmap", None)
        }
    }
}

impl WriteableBitmapTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTrigger::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the bitmap passed to actions. This is a property of the object model.
    pub fn bitmap(&self) -> Option<WriteableBitmapRef> {
        self.get_value(Self::bitmap_property())
    }

    pub fn set_bitmap(&self, value: Option<WriteableBitmapRef>) {
        self.set_value(Self::bitmap_property(), value)
    }

    /// Manually invokes the trigger.
    pub fn trigger(&self) {
        if !self.is_enabled() {
            return;
        }

        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        let parameter = self.bitmap().map(|bitmap| Rc::new(bitmap) as BoxedValue);
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), parameter.as_ref());
    }
}
