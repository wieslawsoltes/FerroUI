//! Port of `Diagnostics/VisualExtensions.cs`.

use ferroui_base::data::BindingPriority;
use ferroui_base::media::imaging::{PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::media::{ITransform, RectangleGeometry, TranslateTransform};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{PixelSize, Rect, Ref, RelativePoint, RelativeUnit, Vector, Visual};
use ferroui_controls::Control;
use std::io::Write;
use std::rc::Rc;

/// Extension methods of controls for DevTools.
pub trait VisualExtensions {
    /// Render control to the destination stream, at 96 DPI.
    fn render_to(&self, destination: &mut dyn Write) -> std::io::Result<()> {
        self.render_to_with_dpi(destination, 96.0)
    }

    /// Render control to the destination stream.
    ///
    /// `destination` is the destination stream; `dpi` the dpi quality. An error is what writing
    /// the image to the stream fails with.
    fn render_to_with_dpi(&self, destination: &mut dyn Write, dpi: f64) -> std::io::Result<()>;
}

impl VisualExtensions for Control {
    fn render_to_with_dpi(&self, destination: &mut dyn Write, dpi: f64) -> std::io::Result<()> {
        // Deviation (DEVIATIONS.md, Developer tools):
        // `CompositionVisual.TryGetServerGlobalTransform()` (the transform of the visual on the
        // render thread) does not exist in the port; the transform of the visual to the root of its
        // visual tree is the same transform, computed from the layout.
        if self.composition_visual().is_none() {
            return Ok(());
        }
        let Some(transform) = self.visual_root().and_then(|visual_root| self.transform_to_visual(&visual_root)) else {
            return Ok(());
        };

        let rect = Rect::from_size(self.bounds().size()).transform_to_aabb(transform);
        let top = rect.top_left();
        let pixel_size = PixelSize::new(rect.width as i32, rect.height as i32);
        let dpi_vector = Vector::new(dpi, dpi);

        // get Visual root
        let root: Ref<Control> = self
            .visual_root()
            .or_else(|| self.get_visual_root())
            .and_then(|root| root.cast::<Control>())
            .unwrap_or_else(|| self.to_ref());

        // Set clip region
        let clip_region = RectangleGeometry::with_rect(rect);
        let clip_to_bounds_setter =
            root.set_value_with_priority(Visual::clip_to_bounds_property(), true, BindingPriority::Animation);
        let clip_setter =
            root.set_value_with_priority(Visual::clip_property(), Some(clip_region.upcast()), BindingPriority::Animation);

        // Translate origin
        let render_transform_origin_setter = root.set_value_with_priority(
            Visual::render_transform_origin_property(),
            RelativePoint::from_point(top, RelativeUnit::Absolute),
            BindingPriority::Animation,
        );

        let translate: Rc<dyn ITransform> = (&TranslateTransform::with_offset(-top.x, -top.y)).into();
        let render_transform_setter = root.set_value_with_priority(
            Visual::render_transform_property(),
            Some(translate),
            BindingPriority::Animation,
        );

        let result = {
            let bitmap = RenderTargetBitmap::with_dpi(pixel_size, dpi_vector);
            bitmap.render(&root.clone().upcast());
            let saved = bitmap.save(&mut *destination, &PngBitmapEncoderOptions::DEFAULT.into());
            bitmap.dispose();
            saved
        };

        // Restore values before trasformation
        let setters: [Option<Rc<dyn IDisposable>>; 4] =
            [render_transform_setter, render_transform_origin_setter, clip_setter, clip_to_bounds_setter];
        for setter in setters.into_iter().flatten() {
            setter.dispose();
        }
        self.invalidate_visual();

        result
    }
}
