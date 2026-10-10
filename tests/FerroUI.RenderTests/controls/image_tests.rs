//! Port of upstream's `Controls/ImageTests.cs`.
//!
//! Upstream's class loads its two bitmaps in its constructor from the
//! directory of the expected images; `ImageTests::new` does the same.

use crate::test_base::TestBase;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{Brushes, EdgeMode, IImage, RenderOptions, RotateTransform, Stretch};
use ferroui_base::Thickness;
use ferroui_controls::{Border, Decorator, Image};
use std::rc::Rc;

struct ImageTests {
    base: TestBase,
    bitmap: Rc<Bitmap>,
    bitmap2: Rc<Bitmap>,
}

impl ImageTests {
    fn new() -> ImageTests {
        let base = TestBase::new(r"Controls\Image");
        let bitmap = Rc::new(
            Bitmap::from_file(base.expected_path().join("test.png").to_str().expect("the path is text"))
                .expect("the bitmap is loaded"),
        );
        let bitmap2 = Rc::new(
            Bitmap::from_file(base.expected_path().join("test2.png").to_str().expect("the path is text"))
                .expect("the bitmap is loaded"),
        );
        ImageTests { base, bitmap, bitmap2 }
    }

    fn bitmap(&self) -> Rc<dyn IImage> {
        self.bitmap.clone()
    }

    fn bitmap2(&self) -> Rc<dyn IImage> {
        self.bitmap2.clone()
    }
}

#[test]
fn image_stretch_none() {
    let t = ImageTests::new();
    let target = Decorator::new();
    target.set_padding(Thickness::symmetric(20.0, 8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    let image = Image::new();
    image.set_source(Some(t.bitmap()));
    image.set_stretch(Stretch::None);
    border.set_child(image);
    target.set_child(border);

    t.base.render_to_file(&target, "Image_Stretch_None");
    t.base.compare_images("Image_Stretch_None");
}

#[test]
fn image_stretch_fill() {
    let t = ImageTests::new();
    let target = Decorator::new();
    target.set_padding(Thickness::symmetric(20.0, 8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    let image = Image::new();
    image.set_source(Some(t.bitmap()));
    image.set_stretch(Stretch::Fill);
    border.set_child(image);
    target.set_child(border);

    t.base.render_to_file(&target, "Image_Stretch_Fill");
    t.base.compare_images("Image_Stretch_Fill");
}

#[test]
fn image_stretch_uniform() {
    let t = ImageTests::new();
    let target = Decorator::new();
    target.set_padding(Thickness::symmetric(20.0, 8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    let image = Image::new();
    image.set_source(Some(t.bitmap()));
    image.set_stretch(Stretch::Uniform);
    border.set_child(image);
    target.set_child(border);

    t.base.render_to_file(&target, "Image_Stretch_Uniform");
    t.base.compare_images("Image_Stretch_Uniform");
}

#[test]
fn image_stretch_uniform_to_fill() {
    let t = ImageTests::new();
    let target = Decorator::new();
    target.set_padding(Thickness::symmetric(20.0, 8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    let image = Image::new();
    image.set_source(Some(t.bitmap()));
    image.set_stretch(Stretch::UniformToFill);
    border.set_child(image);
    target.set_child(border);

    t.base.render_to_file(&target, "Image_Stretch_UniformToFill");
    t.base.compare_images("Image_Stretch_UniformToFill");
}

#[test]
fn image_rotated_edge_mode_unspecified() {
    let t = ImageTests::new();
    let target = Decorator::new();
    target.set_padding(Thickness::symmetric(32.0, 32.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let image = Image::new();
    image.set_source(Some(t.bitmap2()));
    image.set_stretch(Stretch::Uniform);
    image.set_render_transform(Some(RotateTransform::with_angle(30.0).into()));
    target.set_child(image);
    RenderOptions::set_edge_mode(&target, EdgeMode::Unspecified);

    t.base.render_to_file(&target, "Image_Rotated_EdgeMode_Unspecified");
    t.base.compare_images("Image_Rotated_EdgeMode_Unspecified");
}

#[test]
fn image_rotated_edge_mode_antialias() {
    let t = ImageTests::new();
    let target = Decorator::new();
    target.set_padding(Thickness::symmetric(32.0, 32.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let image = Image::new();
    image.set_source(Some(t.bitmap2()));
    image.set_stretch(Stretch::Uniform);
    image.set_render_transform(Some(RotateTransform::with_angle(30.0).into()));
    target.set_child(image);
    RenderOptions::set_edge_mode(&target, EdgeMode::Antialias);

    t.base.render_to_file(&target, "Image_Rotated_EdgeMode_Antialias");
    t.base.compare_images("Image_Rotated_EdgeMode_Antialias");
}

#[test]
fn image_rotated_edge_mode_aliased() {
    let t = ImageTests::new();
    let target = Decorator::new();
    target.set_padding(Thickness::symmetric(32.0, 32.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let image = Image::new();
    image.set_source(Some(t.bitmap2()));
    image.set_stretch(Stretch::Uniform);
    image.set_render_transform(Some(RotateTransform::with_angle(30.0).into()));
    target.set_child(image);
    RenderOptions::set_edge_mode(&target, EdgeMode::Aliased);

    t.base.render_to_file(&target, "Image_Rotated_EdgeMode_Aliased");
    t.base.compare_images("Image_Rotated_EdgeMode_Aliased");
}
