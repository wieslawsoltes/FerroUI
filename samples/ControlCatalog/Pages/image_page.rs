//! Port of `Pages/ImagePage.xaml.cs`: the class of the document
//! `Pages/ImagePage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::media::imaging::CroppedBitmap;
use ferroui_base::media::Stretch;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, PixelPoint, PixelRect, PixelSize, Ref};
use ferroui_controls::{ComboBox, ContentPage, Image, SelectionChangedEventArgs};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct ImagePage {
    base: ContentPage,
    // The fields of the named elements: assigned once the document is
    // loaded, so that the handlers that run while it loads see none.
    bitmap_image: RefCell<Option<Ref<Image>>>,
    cropped_image: RefCell<Option<Ref<Image>>>,
    drawing_image: RefCell<Option<Ref<Image>>>,
}

content_page_class!(ImagePage);
ferro_class_info!(ImagePage {
    new: ImagePage::new,
    markup: {
        methods: [
            fn BitmapStretchChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ImagePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.bitmap_stretch_changed(&sender, selection_changed_args(&e))
                },
            fn DrawingStretchChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ImagePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.drawing_stretch_changed(&sender, selection_changed_args(&e))
                },
            fn BitmapCropChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ImagePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.bitmap_crop_changed(&sender, selection_changed_args(&e))
                },
        ],
    },
});
xaml_class!(ImagePage, "/Pages/ImagePage.xaml");

fn selection_changed_args(e: &Rc<dyn IRoutedEventArgs>) -> &SelectionChangedEventArgs {
    e.downcast_ref::<SelectionChangedEventArgs>().expect("the arguments of a selection changed event")
}

/// The sender of a handler as the combo box it is attached to.
///
/// # Panics
/// Panics if the sender is not a `ComboBox` (an invalid cast in the managed
/// original).
fn combo_box(sender: &Option<BoxedValue>) -> Ref<ComboBox> {
    sender
        .as_ref()
        .and_then(|sender| ValueTypes::as_object(&**sender))
        .and_then(|sender| sender.cast::<ComboBox>())
        .expect("the sender of a selection changed event handled by the page is a ComboBox")
}

/// The stretch with the numeric value `index` (the cast of the managed
/// original); `None` for an index that is not a value of the enumeration.
fn stretch_of(index: i32) -> Option<Stretch> {
    match index {
        0 => Some(Stretch::None),
        1 => Some(Stretch::Fill),
        2 => Some(Stretch::Uniform),
        3 => Some(Stretch::UniformToFill),
        _ => None,
    }
}

impl ImagePage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            bitmap_image: RefCell::new(None),
            cropped_image: RefCell::new(None),
            drawing_image: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        *this.bitmap_image.borrow_mut() = this.find_control::<Image>("bitmapImage");
        *this.cropped_image.borrow_mut() = this.find_control::<Image>("croppedImage");
        *this.drawing_image.borrow_mut() = this.find_control::<Image>("drawingImage");
        this
    }

    pub fn bitmap_stretch_changed(&self, sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let bitmap_image = self.bitmap_image.borrow().clone();
        if let Some(bitmap_image) = bitmap_image {
            let combox_box = combo_box(sender);
            if let Some(stretch) = stretch_of(combox_box.selected_index()) {
                bitmap_image.set_stretch(stretch);
            }
        }
    }

    pub fn drawing_stretch_changed(&self, sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let drawing_image = self.drawing_image.borrow().clone();
        if let Some(drawing_image) = drawing_image {
            let combox_box = combo_box(sender);
            if let Some(stretch) = stretch_of(combox_box.selected_index()) {
                drawing_image.set_stretch(stretch);
            }
        }
    }

    pub fn bitmap_crop_changed(&self, sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let cropped_image = self.cropped_image.borrow().clone();
        if let Some(cropped_image) = cropped_image {
            let combox_box = combo_box(sender);
            let cropped_bitmap = cropped_image
                .source()
                .and_then(|source| source.as_object().and_then(|object| object.to_ref().cast::<CroppedBitmap>()));
            if let Some(cropped_bitmap) = cropped_bitmap {
                cropped_bitmap.set_source_rect(Self::get_crop_rect(combox_box.selected_index()));
            }
        }
    }

    fn get_crop_rect(index: i32) -> PixelRect {
        let bitmap_width = 640;
        let bitmap_height = 426;
        let crop_size = PixelSize::new(320, 240);
        match index {
            1 => PixelRect::from_position_size(
                PixelPoint::new((bitmap_width - crop_size.width) / 2, (bitmap_height - crop_size.width) / 2),
                crop_size,
            ),
            2 => PixelRect::from_position_size(PixelPoint::new(0, 0), crop_size),
            3 => PixelRect::from_position_size(PixelPoint::new(bitmap_width - crop_size.width, 0), crop_size),
            4 => PixelRect::from_position_size(PixelPoint::new(0, bitmap_height - crop_size.height), crop_size),
            5 => PixelRect::from_position_size(
                PixelPoint::new(bitmap_width - crop_size.width, bitmap_height - crop_size.height),
                crop_size,
            ),
            _ => PixelRect::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn crop_rect_of_each_choice() {
        assert_eq!(ImagePage::get_crop_rect(0), PixelRect::default());
        assert_eq!(ImagePage::get_crop_rect(1), PixelRect::new(160, 53, 320, 240));
        assert_eq!(ImagePage::get_crop_rect(2), PixelRect::new(0, 0, 320, 240));
        assert_eq!(ImagePage::get_crop_rect(3), PixelRect::new(320, 0, 320, 240));
        assert_eq!(ImagePage::get_crop_rect(4), PixelRect::new(0, 186, 320, 240));
        assert_eq!(ImagePage::get_crop_rect(5), PixelRect::new(320, 186, 320, 240));
        assert_eq!(ImagePage::get_crop_rect(6), PixelRect::default());
    }

    #[test]
    fn stretch_of_an_index() {
        assert_eq!(stretch_of(2), Some(Stretch::Uniform));
        assert_eq!(stretch_of(-1), None);
    }
}
