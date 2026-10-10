//! Port of upstream's `Controls/ImageBlendTests.cs`.
//!
//! Upstream's class loads its two bitmaps in its constructor from the
//! directory of the expected images; `ImageBlendTests::new` does the same.

use crate::test_base::TestBase;
use ferroui_base::media::imaging::{Bitmap, BitmapBlendingMode};
use ferroui_base::media::{Brushes, IImage};
use ferroui_controls::{Border, Decorator, Image, Panel};
use std::rc::Rc;

struct ImageBlendTests {
    base: TestBase,
    bitmap_base: Rc<Bitmap>,
    bitmap_over: Rc<Bitmap>,
}

impl ImageBlendTests {
    fn new() -> ImageBlendTests {
        let base = TestBase::new(r"Controls\Image\blend");
        let bitmap_base = Rc::new(
            Bitmap::from_file(base.expected_path().join("Cat.jpg").to_str().expect("the path is text"))
                .expect("the bitmap is loaded"),
        );
        let bitmap_over = Rc::new(
            Bitmap::from_file(base.expected_path().join("ColourShading - by Stib.png").to_str().expect("the path is text"))
                .expect("the bitmap is loaded"),
        );
        ImageBlendTests { base, bitmap_base, bitmap_over }
    }

    fn test_blend_mode(&self, blend_mode: BitmapBlendingMode, test_name: &str) {
        let panel = Panel::new();
        let image = Image::new();
        image.set_source(Some(self.bitmap_base.clone() as Rc<dyn IImage>));
        panel.children().add(image);
        let image = Image::new();
        image.set_source(Some(self.bitmap_over.clone() as Rc<dyn IImage>));
        image.set_blend_mode(blend_mode);
        panel.children().add(image);

        let target = Decorator::new();
        target.set_width(512.0);
        target.set_height(512.0);
        let border = Border::new();
        border.set_background(Some(Brushes::red()));
        border.set_child(panel);
        target.set_child(border);

        self.base.render_to_file(&target, test_name);
        self.base.compare_images(test_name);
    }
}

#[test]
fn image_blend_nothing() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::Unspecified, "Image_Blend_Nothing");
}

#[test]
fn image_blend_plus() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::Plus, "Image_Blend_Plus");
}

#[test]
fn image_blend_screen() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::Screen, "Image_Blend_Screen");
}

#[test]
fn image_blend_overlay() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::Overlay, "Image_Blend_Overlay");
}

#[test]
fn image_blend_darken() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::Darken, "Image_Blend_Darken");
}

#[test]
fn image_blend_lighten() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::Lighten, "Image_Blend_Lighten");
}

#[test]
fn image_blend_color_dodge() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::ColorDodge, "Image_Blend_ColorDodge");
}

#[test]
fn image_blend_color_burn() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::ColorBurn, "Image_Blend_ColorBurn");
}

#[test]
fn image_blend_hard_light() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::HardLight, "Image_Blend_HardLight");
}

#[test]
fn image_blend_soft_light() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::SoftLight, "Image_Blend_SoftLight");
}

#[test]
fn image_blend_difference() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::Difference, "Image_Blend_Difference");
}

#[test]
fn image_blend_exclusion() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::Exclusion, "Image_Blend_Exclusion");
}

#[test]
fn image_blend_multiply() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::Multiply, "Image_Blend_Multiply");
}

#[test]
fn image_blend_hue() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::Hue, "Image_Blend_Hue");
}

#[test]
fn image_blend_saturation() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::Saturation, "Image_Blend_Saturation");
}

#[test]
fn image_blend_color() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::Color, "Image_Blend_Color");
}

#[test]
fn image_blend_luminosity() {
    ImageBlendTests::new().test_blend_mode(BitmapBlendingMode::Luminosity, "Image_Blend_Luminosity");
}
