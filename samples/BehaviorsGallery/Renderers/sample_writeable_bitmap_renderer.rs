//! Port of `Renderers/SampleWriteableBitmapRenderer.cs` of the reference sample.

use ferroui_base::ferro_markup_type;
use ferroui_base::media::imaging::WriteableBitmap;
use ferroui_base::media::Color;
use ferroui_xaml_interactions_custom::IWriteableBitmapRenderer;
use std::rc::Rc;
use std::time::Instant;

/// Fills the bitmap with green whose alpha grows with the time since the renderer was created.
pub struct SampleWriteableBitmapRenderer {
    st: Instant,
}

impl Default for SampleWriteableBitmapRenderer {
    fn default() -> Self {
        Self { st: Instant::now() }
    }
}

impl PartialEq for SampleWriteableBitmapRenderer {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl SampleWriteableBitmapRenderer {
    /// `ColorToInt(Color)`: the color as a BGRA pixel.
    fn color_to_int(color: Color) -> u32 {
        color.b as u32 | (color.g as u32) << 8 | (color.r as u32) << 16 | (color.a as u32) << 24
    }
}

impl IWriteableBitmapRenderer for SampleWriteableBitmapRenderer {
    /// `Render(WriteableBitmap)`.
    fn render(&self, bitmap: &WriteableBitmap) {
        let fb = bitmap.lock();
        let alpha = ((self.st.elapsed().as_millis() / 10) % 256) as u8;
        let pixel = Self::color_to_int(Color::from_argb(alpha, 0, 255, 0)).to_le_bytes();
        let size = fb.size();
        let row_bytes = fb.row_bytes().max(0) as usize;
        let width = size.width.max(0) as usize;
        let height = size.height.max(0) as usize;
        fb.with_data(&mut |data| {
            for y in 0..height {
                for x in 0..width {
                    let offset = y * row_bytes + x * 4;
                    if let Some(target) = data.get_mut(offset..offset + 4) {
                        target.copy_from_slice(&pixel);
                    }
                }
            }
        });
        fb.dispose();
    }
}

ferro_markup_type!(class SampleWriteableBitmapRenderer {
    this: Rc<SampleWriteableBitmapRenderer>,
    handles: [SampleWriteableBitmapRenderer, Rc<SampleWriteableBitmapRenderer>, Option<Rc<SampleWriteableBitmapRenderer>>],
    interfaces: [Rc<dyn IWriteableBitmapRenderer>],
    constructors: [() => || Rc::new(SampleWriteableBitmapRenderer::default())],
});
