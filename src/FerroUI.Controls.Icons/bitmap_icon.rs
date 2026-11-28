//! WinUI's `BitmapIcon`.

use crate::{placement, uri_bitmap};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{DrawingContext, IBrush, IImageBrushSource, ImageBrush, Stretch};
use ferroui_base::utilities::Uri;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use ferroui_controls::{ControlImpl, IconElement};
use std::cell::RefCell;
use std::rc::Rc;

/// An icon that shows a bitmap, by default in the single color of its `Foreground`, with the
/// alpha of the bitmap as its shape. [`uri_source`](Self::uri_source) is WinUI's `UriSource`, loaded
/// when the icon is first measured or drawn; [`source`](Self::source), a bitmap that is already
/// loaded, takes precedence over it.
#[repr(C)]
pub struct BitmapIcon {
    base: IconElement,
    /// The alpha of the source, as the mask of the monochrome drawing.
    mask: RefCell<Option<Rc<dyn IBrush>>>,
    /// The result of loading `UriSource`; `None` until it is first needed.
    uri_bitmap: RefCell<Option<Option<Rc<Bitmap>>>>,
}

ferro_class!(BitmapIcon: IconElement);
ferroui_base::ferro_class_info!(BitmapIcon { new: BitmapIcon::new });
ferro_impl_classes!(BitmapIcon: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl);

impl FerroObjectImpl for BitmapIcon {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::uri_source_property().as_property() {
            *this.uri_bitmap.borrow_mut() = None;
            *this.mask.borrow_mut() = None;
        } else if change.property() == Self::source_property().as_property() {
            *this.mask.borrow_mut() = None;
        }
    }
}

impl LayoutableImpl for BitmapIcon {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.displayed_bitmap().map_or_else(Size::default, |source| placement::measure(available_size, source.size()))
    }

    fn arrange_override(_this: &Self, final_size: Size) -> Size {
        final_size
    }
}

impl VisualImpl for BitmapIcon {
    fn render(this: &Self, context: &mut DrawingContext) {
        let Some(source) = this.displayed_bitmap() else { return };
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

        /// Defines the `UriSource` property.
        pub fn uri_source_property() -> StyledProperty<Option<Uri>> {
            FerroProperty::register::<BitmapIcon, _>("UriSource", None)
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
            Self::uri_source_property().as_property(),
            Self::show_as_monochrome_property().as_property(),
            TemplatedControl::foreground_property().as_property(),
        ]);
        Layoutable::affects_measure::<BitmapIcon>(&[
            Self::source_property().as_property(),
            Self::uri_source_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: IconElement::construct(), mask: RefCell::new(None), uri_bitmap: RefCell::new(None) }
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

    /// The URI of the bitmap.
    pub fn uri_source(&self) -> Option<Uri> {
        self.get_value(Self::uri_source_property())
    }

    pub fn set_uri_source(&self, value: Option<Uri>) {
        self.set_value(Self::uri_source_property(), value);
    }

    /// The bitmap the icon shows: `Source`, or the one loaded from `UriSource`, once.
    pub(crate) fn displayed_bitmap(&self) -> Option<Rc<Bitmap>> {
        if let Some(source) = self.source() {
            return Some(source);
        }
        let uri = self.uri_source()?;
        self.uri_bitmap.borrow_mut().get_or_insert_with(|| uri_bitmap::load(&uri)).clone()
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
