//! Port of upstream's `Controls/ImageCompositionTests.cs`.
//!
//! Upstream's class loads its two bitmaps in its constructor from the
//! directory of the expected images; `ImageCompositionTests::new` does the same.

use crate::test_base::TestBase;
use ferroui_base::media::imaging::{Bitmap, BitmapBlendingMode};
use ferroui_base::media::{Brushes, IImage};
use ferroui_controls::{Border, Decorator, Image, Panel};
use std::rc::Rc;

struct ImageCompositionTests {
    base: TestBase,
    bitmap_a: Rc<Bitmap>,
    bitmap_b: Rc<Bitmap>,
}

impl ImageCompositionTests {
    fn new() -> ImageCompositionTests {
        let base = TestBase::new(r"Controls\Image\composition");
        let bitmap_a = Rc::new(
            Bitmap::from_file(base.expected_path().join("A.png").to_str().expect("the path is text"))
                .expect("the bitmap is loaded"),
        );
        let bitmap_b = Rc::new(
            Bitmap::from_file(base.expected_path().join("B.png").to_str().expect("the path is text"))
                .expect("the bitmap is loaded"),
        );
        ImageCompositionTests { base, bitmap_a, bitmap_b }
    }

    fn test_composite_mode(&self, blend_mode: BitmapBlendingMode, test_name: &str) {
        let panel = Panel::new();
        let image = Image::new();
        image.set_source(Some(self.bitmap_a.clone() as Rc<dyn IImage>));
        panel.children().add(image);
        let image = Image::new();
        image.set_source(Some(self.bitmap_b.clone() as Rc<dyn IImage>));
        image.set_blend_mode(blend_mode);
        panel.children().add(image);

        let target = Decorator::new();
        target.set_width(512.0);
        target.set_height(512.0);
        let border = Border::new();
        border.set_background(Some(Brushes::transparent()));
        border.set_child(panel);
        target.set_child(border);

        self.base.render_to_file(&target, test_name);
        self.base.compare_images(test_name);
    }
}

#[test]
fn image_blend_source_over() {
    ImageCompositionTests::new().test_composite_mode(BitmapBlendingMode::SourceOver, "Image_Blend_SourceOver");
}

#[test]
fn image_blend_source() {
    ImageCompositionTests::new().test_composite_mode(BitmapBlendingMode::Source, "Image_Blend_Source");
}

#[test]
fn image_blend_source_in() {
    ImageCompositionTests::new().test_composite_mode(BitmapBlendingMode::SourceIn, "Image_Blend_SourceIn");
}

#[test]
fn image_blend_source_out() {
    ImageCompositionTests::new().test_composite_mode(BitmapBlendingMode::SourceOut, "Image_Blend_SourceOut");
}

#[test]
fn image_blend_source_atop() {
    ImageCompositionTests::new().test_composite_mode(BitmapBlendingMode::SourceAtop, "Image_Blend_SourceAtop");
}

#[test]
fn image_blend_destination() {
    ImageCompositionTests::new().test_composite_mode(BitmapBlendingMode::Destination, "Image_Blend_Destination");
}

#[test]
fn image_blend_destination_in() {
    ImageCompositionTests::new().test_composite_mode(BitmapBlendingMode::DestinationIn, "Image_Blend_DestinationIn");
}

#[test]
fn image_blend_destination_out() {
    ImageCompositionTests::new().test_composite_mode(BitmapBlendingMode::DestinationOut, "Image_Blend_DestinationOut");
}

#[test]
fn image_blend_destination_over() {
    ImageCompositionTests::new().test_composite_mode(BitmapBlendingMode::DestinationOver, "Image_Blend_DestinationOver");
}

#[test]
fn image_blend_destination_atop() {
    ImageCompositionTests::new().test_composite_mode(BitmapBlendingMode::DestinationAtop, "Image_Blend_DestinationAtop");
}

#[test]
fn image_blend_xor() {
    ImageCompositionTests::new().test_composite_mode(BitmapBlendingMode::Xor, "Image_Blend_Xor");
}
