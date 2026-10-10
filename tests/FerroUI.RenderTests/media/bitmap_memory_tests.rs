//! Port of upstream's `Media/BitmapMemoryTests.cs`.
//!
//! The memory of a bitmap and the enumeration of the pixel formats are
//! internal upstream and visible to its test assembly; the port has them
//! public and hidden from the documentation.

use ferroui_base::media::imaging::BitmapMemory;
use ferroui_base::platform::{AlphaFormat, PixelFormat, PixelFormatEnum};
use ferroui_base::PixelSize;

fn should_align_row_bytes_to_four_bytes(pixel_format_enum: PixelFormatEnum, alpha_format: AlphaFormat) {
    let bitmap_memory = BitmapMemory::new(PixelFormat::new(pixel_format_enum), alpha_format, PixelSize::new(33, 1));

    assert!(bitmap_memory.row_bytes() % 4 == 0);
}

#[test]
fn should_align_row_bytes_to_four_bytes_bgr24() {
    should_align_row_bytes_to_four_bytes(PixelFormatEnum::Bgr24, AlphaFormat::Opaque);
}

#[test]
fn should_align_row_bytes_to_four_bytes_bgr555() {
    should_align_row_bytes_to_four_bytes(PixelFormatEnum::Bgr555, AlphaFormat::Opaque);
}

#[test]
fn should_align_row_bytes_to_four_bytes_bgr565() {
    should_align_row_bytes_to_four_bytes(PixelFormatEnum::Bgr565, AlphaFormat::Opaque);
}

#[test]
fn should_align_row_bytes_to_four_bytes_black_white() {
    should_align_row_bytes_to_four_bytes(PixelFormatEnum::BlackWhite, AlphaFormat::Opaque);
}
