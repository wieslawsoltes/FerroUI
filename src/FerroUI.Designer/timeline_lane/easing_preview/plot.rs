use super::{EasingPreview, Point, Rc, Rect};
use crate::timeline_lane::brush;
use ferroui_base::media::{BoxShadows, DrawingContext, IPen, Pen};
use ferroui_base::VisualImpl;

#[derive(Clone, Copy)]
pub(super) struct Plot {
    rect: Rect,
    low: f64,
    high: f64,
    scale: f64,
}

impl Plot {
    pub(super) fn new(bounds: Rect, samples: &[f64], handles: Option<[Point; 2]>) -> Self {
        let (min, max) = handles.map_or((0.0, 1.0), |points| {
            (
                points[0].y.min(points[1].y).min(-0.25),
                points[0].y.max(points[1].y).max(1.25),
            )
        });
        let low = samples.iter().copied().fold(min, f64::min);
        let high = samples.iter().copied().fold(max, f64::max);
        let scale = low.abs().max(high).max(1.0);
        let (horizontal, vertical) = if handles.is_some() {
            (12.0, 10.0)
        } else {
            (5.0, 4.0)
        };
        let rect = Rect::new(
            horizontal,
            vertical,
            (bounds.width - 2.0 * horizontal).max(0.0),
            (bounds.height - 2.0 * vertical).max(0.0),
        );
        Self {
            rect,
            low: low / scale,
            high: high / scale,
            scale,
        }
    }

    pub(super) fn can_edit(self) -> bool {
        self.rect.width > 0.0 && self.rect.height > 0.0
    }

    pub(super) fn point(self, value: Point) -> Point {
        Point::new(
            self.rect.x + value.x * self.rect.width,
            self.rect.y
                + (self.high - value.y / self.scale) / (self.high - self.low) * self.rect.height,
        )
    }

    pub(super) fn value(self, position: Point) -> Point {
        let y = (position.y - self.rect.y) / self.rect.height;
        Point::new(
            (position.x - self.rect.x) / self.rect.width,
            (self.high - y * (self.high - self.low)) * self.scale,
        )
    }
}

impl VisualImpl for EasingPreview {
    fn render(this: &Self, context: &mut DrawingContext) {
        let bounds = Rect::from_size(this.bounds().size());
        let fill = brush(0xff303138);
        let border: Rc<dyn IPen> = Pen::from_uint32(
            if this.is_focused() {
                0xff79a8ff
            } else {
                0xff3b3d46
            },
            1.0,
        )
        .into();
        context.draw_rectangle(
            Some(&fill),
            Some(&border),
            bounds.deflate(0.5),
            4.0,
            4.0,
            &BoxShadows::default(),
        );
        let plot = this.plot();
        let samples = this.samples.borrow();
        if samples.is_empty() {
            let invalid: Rc<dyn IPen> = Pen::from_uint32(0xffe89595, 1.0).into();
            context.draw_line(&invalid, plot.rect.top_left(), plot.rect.bottom_right());
            context.draw_line(&invalid, plot.rect.bottom_left(), plot.rect.top_right());
            return;
        }
        let clip = context.push_clip(bounds);
        let guide: Rc<dyn IPen> = Pen::from_uint32(0xff575465, 1.0).into();
        let endpoints = [
            plot.point(Point::new(0.0, 0.0)),
            plot.point(Point::new(1.0, 1.0)),
        ];
        context.draw_line(&guide, endpoints[0], endpoints[1]);
        let handles = this
            .handles()
            .map(|points| points.map(|point| plot.point(point)));
        if let Some(handles) = handles {
            for (from, to) in endpoints.into_iter().zip(handles) {
                context.draw_line(&guide, from, to);
            }
        }
        let curve: Rc<dyn IPen> = Pen::from_uint32(0xffb6a2f6, 1.5).into();
        let at = |index: usize| {
            plot.point(Point::new(
                index as f64 / (samples.len() - 1) as f64,
                samples[index],
            ))
        };
        for index in 1..samples.len() {
            context.draw_line(&curve, at(index - 1), at(index));
        }
        if let Some(handles) = handles {
            for (index, point) in handles.into_iter().enumerate() {
                let color = if index == this.handle.get() {
                    0xff79a8ff
                } else {
                    0xffb6a2f6
                };
                let handle = brush(color);
                context.draw_rectangle(
                    Some(&handle),
                    None,
                    Rect::new(point.x - 4.0, point.y - 4.0, 8.0, 8.0),
                    4.0,
                    4.0,
                    &BoxShadows::default(),
                );
            }
        }
        context.pop(clip);
    }
}
