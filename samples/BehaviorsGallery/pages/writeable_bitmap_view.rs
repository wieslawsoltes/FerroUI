//! Port of `Views/Pages/WriteableBitmapView` of the reference sample: the class of `pages/WriteableBitmapView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::renderers::sample_writeable_bitmap_renderer::SampleWriteableBitmapRenderer;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_xaml_interactions_custom::IWriteableBitmapRenderer;
use std::rc::Rc;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct WriteableBitmapView {
    base: UserControl,
}

user_control_class!(WriteableBitmapView);
ferro_class_info!(WriteableBitmapView { new: WriteableBitmapView::new });
xaml_class!(WriteableBitmapView, "/pages/WriteableBitmapView.xaml");

impl WriteableBitmapView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `WriteableBitmapView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    WriteableBitmapView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[WriteableBitmapView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "WriteableBitmap",
    reference: "Views/Pages/WriteableBitmapView",
    create: Some(create),
    types: TYPES,
    classes: &[&WriteableBitmapView::XAML_CLASS],
    markup_types: &[<SampleWriteableBitmapRenderer as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the renderer of the page: the renderer as a
/// writeable bitmap renderer.
fn register_value_types() {
    ValueTypes::register_reference::<SampleWriteableBitmapRenderer>();
    ValueTypes::register_cast::<Rc<SampleWriteableBitmapRenderer>, Rc<dyn IWriteableBitmapRenderer>>(
        |renderer| renderer.clone(),
    );
}
