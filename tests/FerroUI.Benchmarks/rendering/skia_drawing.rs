//! The Skia drawing context recording a frame: what a frame of many small visuals asks of it, per
//! operation one drawing context over a picture recorder (the recording of a frame, as the render
//! thread and the bands of the pipelined layer make it), with the work of the context in front and
//! Skia's recording behind it.
//!
//! - `clipped_visuals`: a transform, a clip and a rectangle per visual, the clip popped after it
//!   (the restore of the canvas and the transform after it).
//! - `rounded_rectangles`: filled and stroked rounded rectangles with four equal corners and rounded
//!   clips, with and without rounding.
//! - `solid_fills_and_strokes`: solid fills and undashed solid strokes in turn, with changing colours,
//!   widths and joins (the paint state the context keeps).
//! - `images_gradients_and_dashes`: images, gradient fills and dashed strokes, as charts draw them.

use crate::harness::Registry;
use ferroui_base::media::immutable::{
    ImmutableDashStyle, ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen, ImmutableSolidColorBrush,
};
use ferroui_base::media::{BoxShadows, Color, Colors, GradientSpreadMethod, IBrush, IPen, PenLineCap, PenLineJoin};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::{Matrix, Point, Rect, RelativePoint, RelativeUnit, RoundedRect, Vector};
use ferroui_skia::{CanvasSource, CreateInfo, DrawingContextImpl, ImmutableBitmap};
use skia_safe::PictureRecorder;
use std::rc::Rc;

/// The visuals of one operation.
const VISUALS: usize = 200;

pub struct SkiaDrawing {
    solids: Vec<Rc<dyn IBrush>>,
    pens: Vec<Rc<dyn IPen>>,
    gradient: Rc<dyn IBrush>,
    dashed: Rc<dyn IPen>,
    image: ImmutableBitmap,
}

impl SkiaDrawing {
    pub fn new() -> Self {
        let colors = [Colors::RED, Colors::GREEN, Colors::BLUE, Colors::BLACK];
        let solids =
            colors.iter().map(|color| Rc::new(ImmutableSolidColorBrush::new(*color)) as Rc<dyn IBrush>).collect();
        let joins = [PenLineJoin::Miter, PenLineJoin::Round, PenLineJoin::Bevel];
        let pens = (0..6)
            .map(|index| {
                Rc::new(ImmutablePen::new(
                    Some(Rc::new(ImmutableSolidColorBrush::new(colors[index % colors.len()]))),
                    1.0 + (index % 3) as f64,
                    None,
                    PenLineCap::Flat,
                    joins[index % joins.len()],
                    10.0,
                )) as Rc<dyn IPen>
            })
            .collect();
        let gradient = Rc::new(ImmutableLinearGradientBrush::new(
            &[
                ImmutableGradientStop::new(0.0, Colors::RED),
                ImmutableGradientStop::new(0.5, Colors::GREEN),
                ImmutableGradientStop::new(1.0, Colors::BLUE),
            ],
            1.0,
            None,
            None,
            GradientSpreadMethod::Pad,
            Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
            Some(RelativePoint::new(1.0, 1.0, RelativeUnit::Relative)),
            None,
        ));
        let dashed = Rc::new(ImmutablePen::new(
            Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(40, 40, 40)))),
            2.0,
            Some(Rc::new(ImmutableDashStyle::new(Some(&[3.0, 2.0]), 0.0))),
            PenLineCap::Flat,
            PenLineJoin::Miter,
            10.0,
        ));
        let mut surface = skia_safe::surfaces::raster_n32_premul((16, 16)).expect("a raster surface");
        surface.canvas().clear(skia_safe::Color::from_rgb(200, 100, 50));
        let image = ImmutableBitmap::from_image(surface.image_snapshot(), None);
        Self { solids, pens, gradient, dashed, image }
    }

    /// Records a frame with `draw` through a new drawing context.
    fn record(draw: impl FnOnce(&mut DrawingContextImpl)) -> usize {
        let mut recorder = PictureRecorder::new();
        recorder.begin_recording(skia_safe::Rect::from_wh(1024.0, 1024.0), false);
        let recorded = Rc::new(std::cell::Cell::new(0));
        let count = recorded.clone();
        let mut context = DrawingContextImpl::new(
            CreateInfo {
                canvas: Some(CanvasSource::Recorder {
                    recorder,
                    on_finished: Box::new(move |picture| {
                        count.set(picture.map_or(0, |picture| picture.approximate_op_count()));
                    }),
                    leases: None,
                }),
                dpi: Vector::new(96.0, 96.0),
                disable_subpixel_text_rendering: true,
                ..CreateInfo::default()
            },
            Vec::new(),
        );
        draw(&mut context);
        context.dispose();
        recorded.get()
    }

    fn cell(index: usize) -> (f64, f64) {
        ((index % 20) as f64 * 50.0, (index / 20) as f64 * 50.0)
    }

    pub fn clipped_visuals(&mut self) -> usize {
        Self::record(|context| {
            for index in 0..VISUALS {
                let (x, y) = Self::cell(index);
                context.set_transform(Matrix::create_translation(x, y));
                context.push_clip(Rect::new(0.0, 0.0, 40.0, 40.0));
                context.set_transform(Matrix::create_translation(x + 2.0, y + 2.0));
                let brush = &self.solids[index % self.solids.len()];
                let rect = RoundedRect::from(Rect::new(0.0, 0.0, 36.0, 36.0));
                context.draw_rectangle(Some(&**brush), None, rect, &BoxShadows::default());
                context.pop_clip();
            }
        })
    }

    pub fn rounded_rectangles(&mut self) -> usize {
        Self::record(|context| {
            for index in 0..VISUALS {
                let (x, y) = Self::cell(index);
                context.set_transform(Matrix::create_translation(x, y));
                let rounding = if index % 2 == 0 { 0.0 } else { 6.0 };
                context.push_clip_rounded(RoundedRect::from_radius(Rect::new(0.0, 0.0, 44.0, 44.0), rounding));
                let rect = RoundedRect::from_radius(Rect::new(2.0, 2.0, 40.0, 40.0), 5.0);
                let brush = &self.solids[index % self.solids.len()];
                let pen = &self.pens[index % self.pens.len()];
                context.draw_rectangle(Some(&**brush), Some(&**pen), rect, &BoxShadows::default());
                context.pop_clip();
            }
        })
    }

    pub fn solid_fills_and_strokes(&mut self) -> usize {
        Self::record(|context| {
            for index in 0..VISUALS {
                let (x, y) = Self::cell(index);
                context.set_transform(Matrix::create_translation(x, y));
                let brush = &self.solids[(index / 2) % self.solids.len()];
                let rect = RoundedRect::from(Rect::new(0.0, 0.0, 30.0, 10.0));
                context.draw_rectangle(Some(&**brush), None, rect, &BoxShadows::default());
                let pen = &self.pens[(index / 3) % self.pens.len()];
                context.draw_line(Some(&**pen), Point::new(0.0, 20.0), Point::new(40.0, 30.0));
            }
        })
    }

    pub fn images_gradients_and_dashes(&mut self) -> usize {
        Self::record(|context| {
            for index in 0..VISUALS {
                let (x, y) = Self::cell(index);
                context.set_transform(Matrix::create_translation(x, y));
                match index % 3 {
                    0 => {
                        let source = Rect::new(0.0, 0.0, 16.0, 16.0);
                        context.draw_bitmap(&self.image, 1.0, source, Rect::new(0.0, 0.0, 32.0, 32.0));
                    }
                    1 => {
                        let rect = RoundedRect::from(Rect::new(0.0, 0.0, 40.0, 40.0));
                        context.draw_rectangle(Some(&*self.gradient), None, rect, &BoxShadows::default());
                    }
                    _ => context.draw_line(Some(&*self.dashed), Point::new(0.0, 0.0), Point::new(40.0, 40.0)),
                }
            }
        })
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("rendering", "SkiaDrawing");
    class.benchmark("clipped_visuals", "", SkiaDrawing::new, |b| b.clipped_visuals());
    class.benchmark("rounded_rectangles", "", SkiaDrawing::new, |b| b.rounded_rectangles());
    class.benchmark("solid_fills_and_strokes", "", SkiaDrawing::new, |b| b.solid_fills_and_strokes());
    class.benchmark("images_gradients_and_dashes", "", SkiaDrawing::new, |b| b.images_gradients_and_dashes());
}

#[cfg(test)]
mod tests {
    #[test]
    fn skia_drawing() {
        crate::harness::smoke_class(super::register, "SkiaDrawing");
    }
}
