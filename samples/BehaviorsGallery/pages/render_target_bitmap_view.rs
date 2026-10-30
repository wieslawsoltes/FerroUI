//! Port of `Views/Pages/RenderTargetBitmapView` of the reference sample: the class of `pages/RenderTargetBitmapView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::renderers::rotating_square_renderer::RotatingSquareRenderer;
use crate::renderers::solid_square_renderer::SolidSquareRenderer;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_xaml_interactions_custom::{
    IRenderTargetBitmapRenderer, IRenderTargetBitmapSimpleRenderer, RenderRenderTargetBitmapAction,
    RenderTargetBitmapBehavior, RenderTargetBitmapTrigger, StaticRenderTargetBitmapBehavior,
};
use std::rc::Rc;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct RenderTargetBitmapView {
    base: UserControl,
}

user_control_class!(RenderTargetBitmapView);
ferro_class_info!(RenderTargetBitmapView { new: RenderTargetBitmapView::new });
xaml_class!(RenderTargetBitmapView, "/pages/RenderTargetBitmapView.xaml");

impl RenderTargetBitmapView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `RenderTargetBitmapView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // Deviation (DEVIATIONS.md, Behaviors gallery): `Target="AnimatedRenderTarget"` and
        // `Target="StaticBehavior"`; markup resolves the names to the behaviors but does not
        // assign them to a property of the render host contract, so the page sets them.
        if let (Some(trigger), Some(behavior)) = (
            this.find_control::<RenderTargetBitmapTrigger>("AnimatedRenderTrigger"),
            this.find_control::<RenderTargetBitmapBehavior>("AnimatedRenderTarget"),
        ) {
            trigger.set_target(Some(behavior.as_render_target_bitmap_render_host()));
        }
        if let (Some(action), Some(behavior)) = (
            this.find_control::<RenderRenderTargetBitmapAction>("RenderStaticAction"),
            this.find_control::<StaticRenderTargetBitmapBehavior>("StaticBehavior"),
        ) {
            action.set_target(Some(behavior.as_render_target_bitmap_render_host()));
        }
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    RenderTargetBitmapView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[RenderTargetBitmapView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "RenderTargetBitmap",
    reference: "Views/Pages/RenderTargetBitmapView",
    create: Some(create),
    types: TYPES,
    classes: &[&RenderTargetBitmapView::XAML_CLASS],
    markup_types: &[
        <RotatingSquareRenderer as MarkupTyped>::MARKUP,
        <SolidSquareRenderer as MarkupTyped>::MARKUP,
    ],
    register_value_types,
};

/// What the untyped value conversions must know about the renderers of the page: each renderer as
/// its render target contract.
fn register_value_types() {
    ValueTypes::register_reference::<RotatingSquareRenderer>();
    ValueTypes::register_cast::<Rc<RotatingSquareRenderer>, Rc<dyn IRenderTargetBitmapRenderer>>(
        |renderer| renderer.clone(),
    );
    ValueTypes::register_reference::<SolidSquareRenderer>();
    ValueTypes::register_cast::<Rc<SolidSquareRenderer>, Rc<dyn IRenderTargetBitmapSimpleRenderer>>(
        |renderer| renderer.clone(),
    );
}
