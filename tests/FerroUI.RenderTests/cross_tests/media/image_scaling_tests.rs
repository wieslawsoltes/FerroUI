//! Port of upstream's `CrossTests/Media/ImageScalingTests.cs`.
//!
//! Upstream finds the image next to the test assembly; here it is in the
//! `Assets` directory of the test crate.

use crate::cross_test_base::CrossTestBase;
use crate::cross_ui::*;
use crate::test_render_helper;
use ferroui_base::media::imaging::BitmapInterpolationMode;

fn base() -> CrossTestBase {
    CrossTestBase::new("Media/ImageScaling")
}

#[test]
fn upscaling_with_high_quality_should_be_antialiased() {
    test_high_quality_scaling(1024, "Upscaling_With_HighQuality_Should_Be_Antialiased");
}

#[test]
fn downscaling_with_high_quality_should_be_antialiased() {
    test_high_quality_scaling(128, "Downscaling_With_HighQuality_Should_Be_Antialiased");
}

fn test_high_quality_scaling(size: i32, test_name: &str) {
    let t = base();
    let directory_path = test_render_helper::get_tests_directory();
    let image_path = directory_path.join("Assets").join("Star512.png");

    let mut root = CrossImageControl::new(CrossBitmapImage::new(image_path.to_str().expect("the path is text")));
    root.width = size as f64;
    root.height = size as f64;
    root.bitmap_interpolation_mode = BitmapInterpolationMode::HighQuality;

    t.render_and_compare(root, test_name);
}
