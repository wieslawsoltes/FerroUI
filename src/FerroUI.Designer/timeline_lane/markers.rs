use super::*;

pub(super) type MarkerAction = Rc<dyn Fn(&str)>;

impl TimelineLane {
    pub(crate) fn on_marker(&self, action: impl Fn(&str) + 'static) {
        *self.marker_action.borrow_mut() = Some(Rc::new(action));
    }

    pub(crate) fn set_preview_markers(&self, markers: &[PreviewMarker]) {
        if self.preview_markers.borrow().as_slice() != markers {
            markers.clone_into(&mut self.preview_markers.borrow_mut());
            self.invalidate_visual();
        }
    }

    pub(super) fn marker_at(&self, position: Point) -> Option<String> {
        if position.y < self.bounds().height - 12.0 {
            return None;
        }
        let view = self.view_start.get()..=self.view_end.get();
        self.preview_markers
            .borrow()
            .iter()
            .filter(|marker| view.contains(&marker.time))
            .map(|marker| (marker, (self.x(marker.time) - position.x).abs()))
            .filter(|(_, distance)| *distance <= 6.0)
            .min_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(marker, _)| marker.name.clone())
    }

    pub(super) fn draw_preview_markers(&self, context: &mut DrawingContext, bounds: Rect) {
        let markers = self.preview_markers.borrow();
        if markers.is_empty() {
            return;
        }
        let accent = brush(0xffe6ad64);
        let view = self.view_start.get()..=self.view_end.get();
        for marker in markers.iter().filter(|marker| view.contains(&marker.time)) {
            let x = self.x(marker.time).round();
            context.draw_rectangle(
                Some(&accent),
                None,
                Rect::new(x - 3.0, bounds.height - 10.0, 6.0, 7.0),
                1.0,
                1.0,
                &BoxShadows::default(),
            );
        }
    }
}
