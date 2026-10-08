//! Port of upstream's `Media/ImmutableBitmapTests.cs` of the Skia unit tests.
//!
//! Upstream passes a pointer to the first (top) logical row, which for a
//! negative stride sits at the highest address of the buffer. The port's
//! `ImmutableBitmap::from_pixels` takes the whole buffer as a slice, and a
//! negative stride means the rows are stored bottom-up in it; the buffer is
//! laid out exactly as upstream lays it out. The slice is borrowed only for
//! the constructor, so overwriting the buffer afterwards checks the copy as
//! upstream does.

use crate::ImmutableBitmap;
use ferroui_base::platform::{AlphaFormat, IBitmapImpl, IReadableBitmapImpl, PixelFormat};
use ferroui_base::{PixelSize, Vector};

#[test]
fn constructor_from_pixels_copies_source_data() {
    let rows: [(i32, i32, bool); 6] =
        [(1, 1, false), (3, 5, false), (64, 64, false), (1, 1, true), (3, 5, true), (64, 64, true)];

    for (width, height, negative_stride) in rows {
        constructor_from_pixels_copies_source_data_row(width, height, negative_stride);
    }
}

fn constructor_from_pixels_copies_source_data_row(width: i32, height: i32, negative_stride: bool) {
    let row_name = format!("({width}, {height}, {negative_stride})");

    let size = PixelSize::new(width, height);
    let row_bytes = width * 4;
    let abs_stride = row_bytes;
    let byte_size = abs_stride * height;

    // Logical pixel byte: deterministic function of (row, byteIndexWithinRow).
    let expected = |row: i32, x: i32| ((row * row_bytes + x) * 7 + 1) as u8;

    let mut buffer = vec![0u8; byte_size as usize];

    // Lay the logical rows out in physical memory. For a negative stride the rows are stored
    // bottom-up and the data pointer addresses the first (top) logical row, which sits at the
    // highest address.
    for row in 0..height {
        let physical_row = if negative_stride { height - 1 - row } else { row };
        for x in 0..row_bytes {
            buffer[(physical_row * abs_stride + x) as usize] = expected(row, x);
        }
    }

    let stride = if negative_stride { -abs_stride } else { abs_stride };

    let bitmap = ImmutableBitmap::from_pixels(
        size,
        Vector::new(96.0, 96.0),
        stride,
        PixelFormat::BGRA8888,
        AlphaFormat::Premul,
        &buffer,
    )
    .unwrap();

    // The constructor must take its own copy: corrupting (and freeing) the source
    // afterwards must not affect the bitmap's pixels.
    buffer.fill(0xCD);
    drop(buffer);

    assert_eq!(size, bitmap.pixel_size(), "{row_name}");

    let locked = bitmap.lock();
    assert_eq!(size, locked.size(), "{row_name}");

    let locked_row_bytes = locked.row_bytes();
    locked.with_data(&mut |dst| {
        for row in 0..height {
            for x in 0..row_bytes {
                assert_eq!(expected(row, x), dst[(row * locked_row_bytes + x) as usize], "{row_name}: row {row}, byte {x}");
            }
        }
    });

    locked.dispose();
    bitmap.dispose();
}
