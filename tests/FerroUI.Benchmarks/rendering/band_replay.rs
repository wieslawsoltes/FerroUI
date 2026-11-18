//! The picture of a frame replayed in bands (#817, desktop-performance.md): a pipelined layer
//! records a frame once as a picture and each of its band threads plays the picture under the
//! clip of its strip (`gpu/graphite/pipelined_layer.rs` of the Skia backend). A picture recorded
//! without a spatial index is walked whole by every band; one recorded with Skia's bounding-box
//! hierarchy is searched for the commands that meet the clip.
//!
//! The frame is a patchwork of tiles over the size of a window (about 23,000 commands). The bands
//! are played into a surface of Skia that tracks the clip and draws nothing, so what is measured
//! is the walk of the picture and the rejection of what lies outside a band, the part the index
//! changes; what a GPU recorder does with the commands inside a band is the same either way.
//!
//! - `record_plain`, `record_indexed`: the recording of the frame, with the index built when the
//!   recording ends (on the render thread, once a frame);
//! - `bands2_plain`, `bands2_indexed`, `bands3_plain`, `bands3_indexed`: every band of a frame of
//!   two or three equal strips played one after the other (the work of all bands; the slowest
//!   band is about that divided by the bands).

use crate::harness::Registry;
use skia_safe::{surfaces, ClipOp, Color, Paint, PaintStyle, Picture, PictureRecorder, RRect, Rect, Surface};

const WIDTH: i32 = 1280;
const HEIGHT: i32 = 820;
/// The tiles of the patchwork: columns and rows.
const COLUMNS: i32 = 64;
const ROWS: i32 = 40;

/// Records the patchwork: each tile is a translated group of a rounded fill, a stroke and four
/// small marks.
fn record(indexed: bool) -> Picture {
    let mut recorder = PictureRecorder::new();
    let canvas = recorder.begin_recording(Rect::from_iwh(WIDTH, HEIGHT), indexed);
    let (tile_width, tile_height) = (WIDTH as f32 / COLUMNS as f32, HEIGHT as f32 / ROWS as f32);
    let mut fill = Paint::default();
    fill.set_anti_alias(true);
    let mut stroke = Paint::default();
    stroke.set_anti_alias(true).set_style(PaintStyle::Stroke).set_stroke_width(1.0);
    stroke.set_color(Color::from_argb(255, 30, 64, 175));
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            canvas.save();
            canvas.translate((column as f32 * tile_width, row as f32 * tile_height));
            fill.set_color(Color::from_argb(255, (column * 4) as u8, (row * 6) as u8, 120));
            let tile = Rect::from_wh(tile_width - 2.0, tile_height - 2.0);
            canvas.draw_rrect(RRect::new_rect_xy(tile, 3.0, 3.0), &fill);
            canvas.draw_rect(tile, &stroke);
            for mark in 0..4 {
                let x = 2.0 + mark as f32 * (tile_width - 6.0) / 4.0;
                canvas.draw_rect(Rect::from_xywh(x, tile_height / 2.0, 3.0, 3.0), &stroke);
            }
            canvas.restore();
        }
    }
    recorder.finish_recording_as_picture(None).expect("a picture")
}

pub struct BandReplay {
    picture: Picture,
    surface: Surface,
    bands: i32,
}

impl BandReplay {
    fn new(indexed: bool, bands: i32) -> Self {
        let surface = surfaces::null((WIDTH, HEIGHT)).expect("a surface that draws nothing");
        Self { picture: record(indexed), surface, bands }
    }

    /// Plays every band of the frame: the picture under the clip of each strip, as a band thread
    /// does.
    fn play(&mut self) -> usize {
        let canvas = self.surface.canvas();
        let height = HEIGHT / self.bands;
        for band in 0..self.bands {
            let bottom = if band == self.bands - 1 { HEIGHT } else { (band + 1) * height };
            canvas.save();
            let clip = Rect::from_ltrb(0.0, (band * height) as f32, WIDTH as f32, bottom as f32);
            canvas.clip_rect(clip, ClipOp::Intersect, false);
            canvas.draw_picture(&self.picture, None, None);
            canvas.restore();
        }
        self.bands as usize
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("rendering", "BandReplay");
    let parameters = format!("Tiles={}", COLUMNS * ROWS);
    class.benchmark("record_plain", parameters.clone(), || (), |_| record(false).approximate_op_count());
    class.benchmark("record_indexed", parameters.clone(), || (), |_| record(true).approximate_op_count());
    class.benchmark("bands2_plain", parameters.clone(), || BandReplay::new(false, 2), BandReplay::play);
    class.benchmark("bands2_indexed", parameters.clone(), || BandReplay::new(true, 2), BandReplay::play);
    class.benchmark("bands3_plain", parameters.clone(), || BandReplay::new(false, 3), BandReplay::play);
    class.benchmark("bands3_indexed", parameters, || BandReplay::new(true, 3), BandReplay::play);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "BandReplay");
    }

    #[test]
    fn the_frame_has_the_commands_of_its_tiles_with_and_without_the_index() {
        let plain = record(false).approximate_op_count();
        assert_eq!(plain, record(true).approximate_op_count());
        assert!(plain >= (COLUMNS * ROWS * 8) as usize, "{plain} commands");
    }
}
