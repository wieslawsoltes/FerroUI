//! Port of `RenderTarget/RenderRenderTargetBitmapAction.cs`.

use crate::IRenderTargetBitmapRenderHost;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Action that invokes [`IRenderTargetBitmapRenderHost::render`] on the specified target.
#[repr(C)]
pub struct RenderRenderTargetBitmapAction {
    base: StyledElementAction,
}

ferro_class!(RenderRenderTargetBitmapAction: StyledElementAction);
ferro_class_info!(RenderRenderTargetBitmapAction {
    new: RenderRenderTargetBitmapAction::new,
    markup: {
        property_attributes: [
            Target: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(RenderRenderTargetBitmapAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl RenderRenderTargetBitmapAction {
        /// Identifies the `Target` property.
        pub fn target_property() -> StyledProperty<Option<Rc<dyn IRenderTargetBitmapRenderHost>>> {
            FerroProperty::register::<RenderRenderTargetBitmapAction, _>("Target", None)
        }
    }
}

impl StyledElementActionImpl for RenderRenderTargetBitmapAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        if let Some(target) = this.target() {
            target.render();
        }

        Some(Rc::new(true))
    }
}

impl RenderRenderTargetBitmapAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the render host. This is a property of the object model; markup resolves a
    /// name given for it to the named host.
    pub fn target(&self) -> Option<Rc<dyn IRenderTargetBitmapRenderHost>> {
        self.get_value(Self::target_property())
    }

    pub fn set_target(&self, value: Option<Rc<dyn IRenderTargetBitmapRenderHost>>) {
        self.set_value(Self::target_property(), value)
    }
}
