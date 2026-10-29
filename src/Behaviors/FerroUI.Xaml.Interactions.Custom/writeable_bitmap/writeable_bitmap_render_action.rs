//! Port of `WriteableBitmap/WriteableBitmapRenderAction.cs`.

use crate::{IWriteableBitmapRenderer, WriteableBitmapRef};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Invokes an [`IWriteableBitmapRenderer`] to render into a bitmap.
#[repr(C)]
pub struct WriteableBitmapRenderAction {
    base: StyledElementAction,
}

ferro_class!(WriteableBitmapRenderAction: StyledElementAction);
ferro_class_info!(WriteableBitmapRenderAction {
    new: WriteableBitmapRenderAction::new,
    markup: {
        property_attributes: [
            Bitmap: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(WriteableBitmapRenderAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl WriteableBitmapRenderAction {
        /// Identifies the `Renderer` property.
        pub fn renderer_property() -> StyledProperty<Option<Rc<dyn IWriteableBitmapRenderer>>> {
            FerroProperty::register::<WriteableBitmapRenderAction, _>("Renderer", None)
        }

        /// Identifies the `Bitmap` property.
        pub fn bitmap_property() -> StyledProperty<Option<WriteableBitmapRef>> {
            FerroProperty::register::<WriteableBitmapRenderAction, _>("Bitmap", None)
        }
    }
}

impl StyledElementActionImpl for WriteableBitmapRenderAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let (Some(renderer), Some(bitmap)) = (this.renderer(), this.bitmap()) else {
            return Some(Rc::new(false));
        };

        renderer.render(&bitmap);
        Some(Rc::new(true))
    }
}

impl WriteableBitmapRenderAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the renderer used when executing the action. This is a property of the object
    /// model.
    pub fn renderer(&self) -> Option<Rc<dyn IWriteableBitmapRenderer>> {
        self.get_value(Self::renderer_property())
    }

    pub fn set_renderer(&self, value: Option<Rc<dyn IWriteableBitmapRenderer>>) {
        self.set_value(Self::renderer_property(), value)
    }

    /// Gets or sets the target bitmap. This is a property of the object model.
    pub fn bitmap(&self) -> Option<WriteableBitmapRef> {
        self.get_value(Self::bitmap_property())
    }

    pub fn set_bitmap(&self, value: Option<WriteableBitmapRef>) {
        self.set_value(Self::bitmap_property(), value)
    }
}
