use super::ink_drawing_attributes::InkDrawingAttributes;
use super::ink_point::{InkError, InkPoint};
use super::ink_stroke::InkStroke;
use ferroui_base::{Matrix, Point};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Stateful builder with separate Begin/Append/End and snapshot APIs.
/// End consumes the pending input only after a stroke has been validated.
pub struct InkStrokeBuilder {
    attributes: Cell<InkDrawingAttributes>,
    pending: RefCell<Option<PendingStroke>>,
}

struct PendingStroke {
    points: Vec<InkPoint>,
    attributes: InkDrawingAttributes,
}

impl Default for InkStrokeBuilder {
    fn default() -> Self {
        Self {
            attributes: Cell::new(InkDrawingAttributes::default()),
            pending: RefCell::new(None),
        }
    }
}

impl InkStrokeBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set_default_drawing_attributes(
        &self,
        value: InkDrawingAttributes,
    ) -> Result<(), InkError> {
        value.validate()?;
        self.attributes.set(value);
        Ok(())
    }
    pub fn default_drawing_attributes(&self) -> InkDrawingAttributes {
        self.attributes.get()
    }
    pub fn create_stroke(
        &self,
        points: impl IntoIterator<Item = Point>,
    ) -> Result<Rc<InkStroke>, InkError> {
        let points = points
            .into_iter()
            .take(super::persistence::InkPersistenceLimits::DEFAULT.max_points_per_stroke + 1)
            .map(|p| InkPoint::new(p, 0.5))
            .collect::<Result<Vec<_>, _>>()?;
        InkStroke::new(points, self.attributes.get())
    }
    pub fn create_stroke_from_ink_points(
        &self,
        points: impl IntoIterator<Item = InkPoint>,
        transform: Matrix,
    ) -> Result<Rc<InkStroke>, InkError> {
        let points = points
            .into_iter()
            .take(super::persistence::InkPersistenceLimits::DEFAULT.max_points_per_stroke + 1)
            .collect();
        let stroke = InkStroke::new(points, self.attributes.get())?;
        stroke.set_point_transform(transform)?;
        Ok(stroke)
    }
    pub fn begin_stroke(&self, point: InkPoint) -> Result<(), InkError> {
        point.validate()?;
        if self.pending.borrow().is_some() {
            return Err(InkError::InvalidOperation(
                "A stroke is already being built",
            ));
        }
        *self.pending.borrow_mut() = Some(PendingStroke {
            points: vec![point],
            attributes: self.attributes.get(),
        });
        Ok(())
    }
    pub fn append_to_stroke(&self, point: InkPoint) -> Result<(), InkError> {
        point.validate()?;
        let mut pending = self.pending.borrow_mut();
        let points = &mut pending
            .as_mut()
            .ok_or(InkError::InvalidOperation(
                "BeginStroke must precede AppendToStroke",
            ))?
            .points;
        if points.last() == Some(&point) {
            return Ok(());
        }
        if points.len() >= super::persistence::InkPersistenceLimits::DEFAULT.max_points_per_stroke {
            return Err(InkError::LimitExceeded);
        }
        if points.last().is_some_and(|p| p.timestamp > point.timestamp) {
            return Err(InkError::InvalidInput(
                "Ink timestamps must be nondecreasing",
            ));
        }
        // Only exact duplicate telemetry is redundant: stationary pressure
        // changes and elapsed time are meaningful and must be retained.
        points.push(point);
        Ok(())
    }
    pub fn end_stroke(&self, point: InkPoint) -> Result<Rc<InkStroke>, InkError> {
        self.append_to_stroke(point)?;
        let stroke = {
            let pending = self.pending.borrow();
            let pending = pending.as_ref().ok_or(InkError::InvalidOperation(
                "BeginStroke must precede EndStroke",
            ))?;
            InkStroke::new(pending.points.clone(), pending.attributes)?
        };
        self.pending.borrow_mut().take();
        Ok(stroke)
    }
    pub fn cancel_stroke(&self) {
        self.pending.borrow_mut().take();
    }
    pub fn is_building_stroke(&self) -> bool {
        self.pending.borrow().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64) -> InkPoint {
        InkPoint::new(Point::new(x, 0.0), 0.5).unwrap()
    }
    #[test]
    fn state_machine_rejects_out_of_order_calls() {
        let b = InkStrokeBuilder::new();
        assert!(b.append_to_stroke(p(0.0)).is_err());
        b.begin_stroke(p(0.0)).unwrap();
        assert!(b.begin_stroke(p(1.0)).is_err());
        assert_eq!(b.end_stroke(p(1.0)).unwrap().ink_points().len(), 2);
        assert!(!b.is_building_stroke());
        assert!(b.end_stroke(p(2.0)).is_err());
    }
    #[test]
    fn stationary_pressure_is_not_deduplicated() {
        let b = InkStrokeBuilder::new();
        b.begin_stroke(p(0.0)).unwrap();
        let mut pressure = p(0.0);
        pressure.pressure = 0.9;
        let s = b.end_stroke(pressure).unwrap();
        assert_eq!(s.ink_points().len(), 2);
    }
    #[test]
    fn cancellation_discards_pending_samples() {
        let b = InkStrokeBuilder::new();
        b.begin_stroke(p(0.0)).unwrap();
        b.cancel_stroke();
        b.begin_stroke(p(1.0)).unwrap();
        assert_eq!(
            b.end_stroke(p(2.0)).unwrap().ink_points()[0].position.x,
            1.0
        );
    }
    #[test]
    fn defaults_changed_midstroke_only_affect_next_stroke() {
        let b = InkStrokeBuilder::new();
        b.begin_stroke(p(0.0)).unwrap();
        let mut a = b.default_drawing_attributes();
        a.size.width = 20.0;
        b.set_default_drawing_attributes(a).unwrap();
        assert_eq!(
            b.end_stroke(p(1.0))
                .unwrap()
                .drawing_attributes()
                .size
                .width,
            2.0
        );
        b.begin_stroke(p(0.0)).unwrap();
        assert_eq!(
            b.end_stroke(p(1.0))
                .unwrap()
                .drawing_attributes()
                .size
                .width,
            20.0
        );
    }
}
