//! WinUI's `BitmapIcon`.

use crate::placement;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{DrawingContext, IBrush, IImageBrushSource, ImageBrush, Stretch};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Layoutable, Ref, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use ferroui_controls::{ControlImpl, IconElement};
use std::cell::RefCell;
use std::rc::Rc;

/// An icon that shows a bitmap, by default in the single color of its `Foreground`, with the
/// alpha of the bitmap as its shape. WinUI's `UriSource` is [`source`](Self::source): a bitmap
/// that is already loaded.
#[repr(C)]
pub struct BitmapIcon {
    base: IconElement,
    /// The alpha of the source, as the mask of the monochrome drawing.
    mask: RefCell<Option<Rc<dyn IBrush>>>,
}

ferro_class!(BitmapIcon: IconElement);
ferroui_base::ferro_class_info!(BitmapIcon { new: BitmapIcon::new });
ferro_impl_classes!(BitmapIcon: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl);

impl FerroObjectImpl for BitmapIcon {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::source_property().as_property() {
            *this.mask.borrow_mut() = None;
        }
    }
}

impl LayoutableImpl for BitmapIcon {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.source().map_or_else(Size::default, |source| placement::measure(available_size, source.size()))
    }

    fn arrange_override(_this: &Self, final_size: Size) -> Size {
        final_size
    }
}

impl VisualImpl for BitmapIcon {
    fn render(this: &Self, context: &mut DrawingContext) {
        let Some(source) = this.source() else { return };
        let Some(rect) = placement::image_rect(this.bounds().size(), source.size()) else { return };
        if !this.show_as_monochrome() {
            context.draw_image(&*source, rect);
            return;
        }
        let Some(foreground) = this.get_value(TemplatedControl::foreground_property()) else { return };
        let mask = this.mask_of(&source);
        // The mask is a brush drawn on the GPU in every backend.
        let state = context.push_opacity_mask(&mask, rect);
        context.fill_rectangle(&foreground, rect, 0.0);
        context.pop(state);
    }
}

ferro_properties! {
    impl BitmapIcon {
        /// Defines the `Source` property.
        pub fn source_property() -> StyledProperty<Option<Rc<Bitmap>>> {
            FerroProperty::register::<BitmapIcon, _>("Source", None)
        }

        /// Defines the `ShowAsMonochrome` property.
        pub fn show_as_monochrome_property() -> StyledProperty<bool> {
            FerroProperty::register::<BitmapIcon, _>("ShowAsMonochrome", true)
        }
    }
}

impl BitmapIcon {
    fn static_constructor() {
        Visual::affects_render::<BitmapIcon>(&[
            Self::source_property().as_property(),
            Self::show_as_monochrome_property().as_property(),
            TemplatedControl::foreground_property().as_property(),
        ]);
        Layoutable::affects_measure::<BitmapIcon>(&[Self::source_property().as_property()]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: IconElement::construct(), mask: RefCell::new(None) }
    }

    /// Creates an icon without a bitmap.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The bitmap of the icon.
    pub fn source(&self) -> Option<Rc<Bitmap>> {
        self.get_value(Self::source_property())
    }

    pub fn set_source(&self, value: Option<Rc<Bitmap>>) {
        self.set_value(Self::source_property(), value);
    }

    /// Whether the bitmap is drawn in the single color of the foreground. The default is true.
    pub fn show_as_monochrome(&self) -> bool {
        self.get_value(Self::show_as_monochrome_property())
    }

    pub fn set_show_as_monochrome(&self, value: bool) {
        self.set_value(Self::show_as_monochrome_property(), value);
    }

    fn mask_of(&self, source: &Rc<Bitmap>) -> Rc<dyn IBrush> {
        self.mask
            .borrow_mut()
            .get_or_insert_with(|| {
                let brush = ImageBrush::with_source(Some(source.clone() as Rc<dyn IImageBrushSource>));
                brush.set_stretch(Stretch::Fill);
                brush.into()
            })
            .clone()
    }
}
