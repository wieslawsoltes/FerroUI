//! Text with a coverage for each colour channel (docs/porting/progpu-backend.md, section 2.7).
//!
//! Used only where the resolved text mode is subpixel and what the text is drawn on is known to be
//! opaque: the root of a target without transparency, outside any layer. Everything else stays
//! greyscale. The run keeps the quarter-pixel phases and the weighted outlines of its greyscale
//! form and the measured coverage transfer, applied to each channel. Each run is drawn in a
//! transient opaque layer that starts from what is under it, because the engine blends the three
//! channels against an opaque destination it owns.

use super::*;
use crate::glyph_run::Placement;

impl ProGpuDrawingContext {
    /// Whether text drawn now has a coverage per channel.
    pub(super) fn subpixel_text(&self, aliased: bool) -> bool {
        if aliased || !self.opaque_target || self.origin != (0.0, 0.0) || !self.layers.is_empty() {
            return false;
        }
        if self.clips.iter().any(|clip| matches!(clip, ClipEntry::Layer)) {
            return false;
        }
        let text = self.text_options.last().map(|options| options.text_rendering_mode).unwrap_or_default();
        let render = self.render_options.last().map(|options| options.text_rendering_mode).unwrap_or_default();
        match text {
            TextRenderingMode::SubpixelAntialias => true,
            TextRenderingMode::Unspecified => render == TextRenderingMode::SubpixelAntialias,
            _ => false,
        }
    }

    /// Draws `run` with a coverage per channel, from the whole device pixel `base`. `false`, with
    /// nothing drawn, when it has to be drawn in greyscale.
    pub(super) fn draw_lcd_run(
        &mut self,
        run: &ProGpuGlyphRun,
        placement: Placement,
        base: Point,
        color: progpu_native_color,
    ) -> bool {
        let Some(lcd) = run.lcd(placement) else { return false };
        if lcd.tiles.is_empty() {
            return true;
        }
        let Some((width, height)) = self.target_size else { return false };
        let (base_x, base_y) = (base.x as i64, base.y as i64);
        let mut tiles = Vec::with_capacity(lcd.tiles.len());
        let (mut left, mut top, mut right, mut bottom) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
        for tile in &lcd.tiles {
            let (x, y) = (base_x + i64::from(tile.x), base_y + i64::from(tile.y));
            // The engine places a tile within 4096 pixels of the origin of its target.
            if !(-4096..=4096).contains(&x) || !(-4096..=4096).contains(&y) {
                return false;
            }
            left = left.min(x);
            top = top.min(y);
            right = right.max(x + i64::from(tile.width));
            bottom = bottom.max(y + i64::from(tile.height));
            tiles.push(progpu_native_scene_rgb_glyph_tile {
                outline_index: tile.outline,
                width: tile.width,
                height: tile.height,
                reserved: 0,
                x_start: tile.x_start,
                y_start: tile.y_start,
                scale: tile.scale,
                subpixel_x: tile.subpixel_x,
                target_x: x as i32,
                target_y: y as i32,
                foreground: color,
            });
        }
        let (left, top) = (left.max(0), top.max(0));
        let (right, bottom) = (right.min(i64::from(width)), bottom.min(i64::from(height)));
        if left >= right || top >= bottom {
            return true;
        }
        let area = Rect::new(left as f64, top as f64, (right - left) as f64, (bottom - top) as f64);
        self.refuse_fragment();
        let Some(state) = self.glyph_state() else { return false };
        let result = self.scene().add_glyph_outlines(&lcd.outlines, &lcd.segments);
        let Some(resource) = self.record(result) else { return false };
        let layer = progpu_native_scene_layer {
            struct_size: std::mem::size_of::<progpu_native_scene_layer>() as u32,
            flags: (PROGPU_NATIVE_SCENE_LAYER_BOUNDS
                | PROGPU_NATIVE_SCENE_LAYER_INITIALIZE_FROM_BACKGROUND
                | PROGPU_NATIVE_SCENE_LAYER_IGNORE_ALPHA) as u32,
            bounds: image_rect(area),
            opacity: 1.0,
            blend_mode: PROGPU_NATIVE_BLEND_SRC_OVER as u32,
            mask_resource_index: NO_INDEX,
            effect_resource_index: NO_INDEX,
            content_revision: 0,
            composite_revision: 0,
            reserved0: 0,
            reserved1: 0,
        };
        self.needs_whole_replay = true;
        let result = self.scene().push_layer(&layer);
        self.record(result);
        let draw = progpu_native_scene_rgb_glyph_draw {
            struct_size: std::mem::size_of::<progpu_native_scene_rgb_glyph_draw>() as u32,
            glyph_count: tiles.len() as u32,
            filter_model: PROGPU_NATIVE_RGB_GLYPH_FIR5_8X8 as u32,
            // Red on the left, the geometry the other backends assume by default.
            pixel_geometry: 1,
            gamma: 1.0,
            enhanced_contrast: 0.0,
            cleartype_level: 1.0,
            dpi_scale: 1.0,
            coverage_transfer: text_coverage_transfer(color),
            reserved1: 0,
        };
        let result = self.scene().draw_rgb_glyph_run(resource, &draw, &tiles, image_rect(area), state);
        let drawn = self.record(result).is_some();
        let result = self.scene().pop_layer();
        self.record(result);
        drawn
    }
}
