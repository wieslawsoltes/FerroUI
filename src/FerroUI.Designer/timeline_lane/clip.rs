use super::*;
use ferroui_designer_animation::ClipSpan;

#[derive(Clone, Copy)]
pub(super) struct Band {
    span: ClipSpan,
    editable: bool,
}

impl TimelineLane {
    pub(crate) fn set_clip(&self, span: ClipSpan, duration: f64, editable: bool) {
        self.configure(duration, vec![span.start]);
        self.clip.set(Some(Band { span, editable }));
        self.set_height(26.0);
        self.invalidate_visual();
    }

    pub(super) fn clip_key_at(&self, point: Point, band: Band) -> Option<f64> {
        let end = band.span.end.unwrap_or(self.duration.get());
        let left = self.x(band.span.start).max(INSET);
        let right = self.x(end).min(self.bounds().width - INSET);
        (band.editable
            && (left..=right).contains(&point.x)
            && (4.0..=self.bounds().height - 4.0).contains(&point.y))
        .then_some(band.span.start)
    }

    pub(super) fn draw_clip(&self, context: &mut DrawingContext, bounds: Rect, band: Band) {
        let drag = self.drag.borrow();
        let offset = match drag.as_ref() {
            Some(Drag::Key { from, to, keys, .. }) if !keys.is_empty() => to - from,
            _ => 0.0,
        };
        let start = band.span.start + offset;
        let end = band
            .span
            .end
            .map_or(self.duration.get(), |end| end + offset);
        let left = self.x(start).max(INSET);
        let right = self.x(end).min(bounds.width - INSET);
        if right <= left {
            return;
        }
        let fill = brush(if self.highlighted.get() || offset != 0.0 {
            0xff5c4e89
        } else {
            0xff3f3a58
        });
        let edge = if band.editable {
            0xffb6a2f6
        } else {
            0xff827a96
        };
        let outline: Rc<dyn IPen> = Pen::from_uint32(edge, 1.0).into();
        context.draw_rectangle(
            Some(&fill),
            Some(&outline),
            Rect::new(left, 4.0, right - left, bounds.height - 8.0),
            3.0,
            3.0,
            &BoxShadows::default(),
        );
        if right - left > 46.0 {
            let caption = if offset != 0.0 {
                format!("{start:.3}s")
            } else if band.span.end.is_none() {
                "∞".into()
            } else {
                format!("{:.2}s", end - start)
            };
            let glyph = text(&caption, 10.0, 0xffeee9fa);
            context.draw_text(&glyph, Point::new(left + 6.0, 5.0));
        }
    }
}
