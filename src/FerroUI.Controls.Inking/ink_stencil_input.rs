//! Bounded stencil manipulation state. State transitions return a value to apply
//! after the caller releases its borrow; applying a transform can raise events.

use super::ink_point::valid_position;
use super::ink_presenter_stencil::MAX_STENCIL_SIZE;
use super::{InkError, InkPresenter, InkPresenterStencilKind};
use ferroui_base::input::{KeyModifiers, PointerPointProperties, PointerType};
use ferroui_base::{Matrix, Point};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
enum SingleContactMode {
    Translate,
    Rotate,
    Resize,
}

struct Gesture {
    kind: InkPresenterStencilKind,
    positions: BTreeMap<i32, Point>,
    initial_positions: BTreeMap<i32, Point>,
    initial_transform: Matrix,
    initial_center: Point,
    initial_radius: f64,
    mode: SingleContactMode,
}

pub(crate) struct StencilUpdate {
    kind: InkPresenterStencilKind,
    transform: Matrix,
    radius: f64,
}

impl StencilUpdate {
    pub fn apply(self, presenter: &InkPresenter) -> Result<(), InkError> {
        match self.kind {
            InkPresenterStencilKind::Other => Err(InkError::InvalidOperation(
                "custom stencil manipulation is not supported",
            )),
            InkPresenterStencilKind::Ruler => presenter.ruler().set_transform(self.transform),
            InkPresenterStencilKind::Protractor => presenter
                .protractor()
                .apply_manipulation(self.transform, self.radius),
        }
    }
}

#[derive(Default)]
pub(crate) struct InkStencilInput {
    gesture: Option<Gesture>,
}

impl InkStencilInput {
    pub fn contains(&self, id: i32) -> bool {
        self.gesture
            .as_ref()
            .is_some_and(|gesture| gesture.positions.contains_key(&id))
    }
    pub fn is_manipulating(&self, kind: InkPresenterStencilKind) -> bool {
        self.gesture
            .as_ref()
            .is_some_and(|gesture| gesture.kind == kind)
    }
    pub fn clear(&mut self) {
        self.gesture = None;
    }
    pub fn reconcile(&mut self, presenter: &InkPresenter) {
        if !presenter.is_input_enabled()
            || self
                .gesture
                .as_ref()
                .is_some_and(|gesture| match gesture.kind {
                    InkPresenterStencilKind::Other => true,
                    InkPresenterStencilKind::Ruler => !presenter.ruler_visible(),
                    InkPresenterStencilKind::Protractor => !presenter.protractor_visible(),
                })
        {
            self.clear();
        }
    }
    pub fn press(
        &mut self,
        presenter: &InkPresenter,
        id: i32,
        device: PointerType,
        position: Point,
        properties: PointerPointProperties,
        modifiers: KeyModifiers,
    ) -> bool {
        self.reconcile(presenter);
        // Pen input remains dedicated to ink and edge snapping. Touch gestures
        // manipulate stencils even when only pen input is admitted for inking.
        if device == PointerType::Pen
            || !valid_position(position)
            || !presenter.is_input_enabled()
            || (device == PointerType::Mouse
                && !properties.is_left_button_pressed
                && !properties.is_right_button_pressed)
        {
            return false;
        }
        let kind = if presenter.ruler().hit_test(position) {
            InkPresenterStencilKind::Ruler
        } else if presenter.protractor().hit_test(position) {
            InkPresenterStencilKind::Protractor
        } else {
            return false;
        };
        if let Some(gesture) = &mut self.gesture {
            if gesture.kind != kind
                || (gesture.positions.len() == 2 && !gesture.positions.contains_key(&id))
            {
                return false;
            }
            gesture.positions.insert(id, position);
            gesture.rebase(presenter);
            return true;
        }
        let mode = if device == PointerType::Mouse
            && kind == InkPresenterStencilKind::Protractor
            && presenter.protractor().is_resizable()
            && modifiers.contains(KeyModifiers::CONTROL)
        {
            SingleContactMode::Resize
        } else if device == PointerType::Mouse
            && (properties.is_right_button_pressed || modifiers.contains(KeyModifiers::SHIFT))
        {
            SingleContactMode::Rotate
        } else {
            SingleContactMode::Translate
        };
        let mut gesture = Gesture {
            kind,
            positions: BTreeMap::from([(id, position)]),
            initial_positions: BTreeMap::new(),
            initial_transform: Matrix::IDENTITY,
            initial_center: Point::default(),
            initial_radius: 0.0,
            mode,
        };
        gesture.rebase(presenter);
        self.gesture = Some(gesture);
        true
    }
    pub fn moved(
        &mut self,
        presenter: &InkPresenter,
        id: i32,
        position: Point,
    ) -> Option<StencilUpdate> {
        self.reconcile(presenter);
        if !valid_position(position) {
            self.release(presenter, id);
            return None;
        }
        let gesture = self.gesture.as_mut()?;
        *gesture.positions.get_mut(&id)? = position;
        let mut initial = [Point::default(); 2];
        let mut current = [Point::default(); 2];
        for (index, (old, new)) in gesture
            .initial_positions
            .values()
            .zip(gesture.positions.values())
            .enumerate()
        {
            initial[index] = *old;
            current[index] = *new;
        }
        let resizing = gesture.kind == InkPresenterStencilKind::Protractor
            && presenter.protractor().is_resizable();
        let mut radius = gesture.initial_radius;
        let mut center = gesture.initial_center;
        let mut rotation = 0.0;
        if gesture.positions.len() == 2 {
            let midpoint =
                |p: &[Point]| Point::new((p[0].x + p[1].x) * 0.5, (p[0].y + p[1].y) * 0.5);
            let before = midpoint(&initial);
            let after = midpoint(&current);
            let old_dx = initial[1].x - initial[0].x;
            let old_dy = initial[1].y - initial[0].y;
            let new_dx = current[1].x - current[0].x;
            let new_dy = current[1].y - current[0].y;
            let distance = old_dx.hypot(old_dy);
            let current_distance = new_dx.hypot(new_dy);
            if distance >= 1.0 && current_distance >= 1.0 {
                rotation = new_dy.atan2(new_dx) - old_dy.atan2(old_dx);
                if resizing {
                    radius = (gesture.initial_radius * current_distance / distance)
                        .clamp(8.0, MAX_STENCIL_SIZE);
                }
            }
            let scale = if resizing {
                radius / gesture.initial_radius
            } else {
                1.0
            };
            let relative = Point::new((center.x - before.x) * scale, (center.y - before.y) * scale)
                * Matrix::create_rotation(rotation);
            center = Point::new(after.x + relative.x, after.y + relative.y);
        } else {
            match gesture.mode {
                SingleContactMode::Translate => {
                    center = Point::new(
                        center.x + current[0].x - initial[0].x,
                        center.y + current[0].y - initial[0].y,
                    );
                }
                SingleContactMode::Rotate => {
                    let before = Point::new(initial[0].x - center.x, initial[0].y - center.y);
                    let after = Point::new(current[0].x - center.x, current[0].y - center.y);
                    if before.x.hypot(before.y) >= 1.0 && after.x.hypot(after.y) >= 1.0 {
                        rotation = after.y.atan2(after.x) - before.y.atan2(before.x);
                    }
                }
                SingleContactMode::Resize if resizing => {
                    let before = (initial[0].x - center.x).hypot(initial[0].y - center.y);
                    let after = (current[0].x - center.x).hypot(current[0].y - center.y);
                    if before >= 1.0 {
                        radius =
                            (gesture.initial_radius * after / before).clamp(8.0, MAX_STENCIL_SIZE);
                    }
                }
                SingleContactMode::Resize => {}
            }
        }
        let local_center = match gesture.kind {
            InkPresenterStencilKind::Other => return None,
            InkPresenterStencilKind::Ruler => presenter.ruler().center(),
            InkPresenterStencilKind::Protractor => Point::new(radius, radius),
        };
        let mut linear = gesture.initial_transform * Matrix::create_rotation(rotation);
        linear.m31 = 0.0;
        linear.m32 = 0.0;
        let transformed_center = local_center * linear;
        linear.m31 = center.x - transformed_center.x;
        linear.m32 = center.y - transformed_center.y;
        Some(StencilUpdate {
            kind: gesture.kind,
            transform: linear,
            radius,
        })
    }
    pub fn release(&mut self, presenter: &InkPresenter, id: i32) -> bool {
        let Some(gesture) = self.gesture.as_mut() else {
            return false;
        };
        if gesture.positions.remove(&id).is_none() {
            return false;
        }
        if gesture.positions.is_empty() {
            self.gesture = None;
        } else {
            gesture.mode = SingleContactMode::Translate;
            gesture.rebase(presenter);
        }
        true
    }
}

impl Gesture {
    fn rebase(&mut self, presenter: &InkPresenter) {
        self.initial_positions.clone_from(&self.positions);
        self.initial_transform = match self.kind {
            InkPresenterStencilKind::Other => Matrix::IDENTITY,
            InkPresenterStencilKind::Ruler => presenter.ruler().transform(),
            InkPresenterStencilKind::Protractor => presenter.protractor().transform(),
        };
        let center = match self.kind {
            InkPresenterStencilKind::Other => Point::default(),
            InkPresenterStencilKind::Ruler => presenter.ruler().center(),
            InkPresenterStencilKind::Protractor => presenter.protractor().center(),
        };
        self.initial_center = center * self.initial_transform;
        self.initial_radius = presenter.protractor().radius();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn contact() -> PointerPointProperties {
        PointerPointProperties::default()
    }
    #[test]
    fn two_touches_rotate_and_releasing_one_rebases_without_a_jump() {
        let p = InkPresenter::new();
        p.ruler().set_length(300.0).unwrap();
        p.ruler().set_width(120.0).unwrap();
        p.ruler().set_is_visible(true);
        let mut input = InkStencilInput::default();
        assert!(input.press(
            &p,
            1,
            PointerType::Touch,
            Point::new(125.0, 60.0),
            contact(),
            KeyModifiers::NONE
        ));
        assert!(input.press(
            &p,
            2,
            PointerType::Touch,
            Point::new(175.0, 60.0),
            contact(),
            KeyModifiers::NONE
        ));
        input
            .moved(&p, 1, Point::new(150.0, 35.0))
            .unwrap()
            .apply(&p)
            .unwrap();
        input
            .moved(&p, 2, Point::new(150.0, 85.0))
            .unwrap()
            .apply(&p)
            .unwrap();
        assert!((p.ruler().angle() - 90.0).abs() < 1e-9);
        assert!(input.release(&p, 1));
        let old = p.ruler().transform();
        input
            .moved(&p, 2, Point::new(150.0, 85.0))
            .unwrap()
            .apply(&p)
            .unwrap();
        assert_eq!(p.ruler().transform(), old);
    }
    #[test]
    fn protractor_pinch_respects_resizability_and_keeps_center_fixed() {
        for resize in [true, false] {
            let p = InkPresenter::new();
            let guide = p.protractor();
            guide.set_radius(100.0).unwrap();
            guide.set_is_visible(true);
            guide.set_is_resizable(resize);
            let mut input = InkStencilInput::default();
            assert!(input.press(
                &p,
                1,
                PointerType::Touch,
                Point::new(50.0, 100.0),
                contact(),
                KeyModifiers::NONE
            ));
            assert!(input.press(
                &p,
                2,
                PointerType::Touch,
                Point::new(150.0, 100.0),
                contact(),
                KeyModifiers::NONE
            ));
            input
                .moved(&p, 1, Point::new(25.0, 100.0))
                .unwrap()
                .apply(&p)
                .unwrap();
            input
                .moved(&p, 2, Point::new(175.0, 100.0))
                .unwrap()
                .apply(&p)
                .unwrap();
            assert_eq!(guide.radius(), if resize { 150.0 } else { 100.0 });
            assert_eq!(guide.center() * guide.transform(), Point::new(100.0, 100.0));
        }
    }
    #[test]
    fn pen_and_third_contacts_are_not_taken_and_hidden_guides_cancel_manipulation() {
        let p = InkPresenter::new();
        p.ruler().set_is_visible(true);
        let mut input = InkStencilInput::default();
        assert!(!input.press(
            &p,
            1,
            PointerType::Pen,
            Point::new(10.0, 10.0),
            contact(),
            KeyModifiers::NONE
        ));
        for id in [1, 2] {
            assert!(input.press(
                &p,
                id,
                PointerType::Touch,
                Point::new(f64::from(id) * 10.0, 10.0),
                contact(),
                KeyModifiers::NONE
            ));
        }
        assert!(!input.press(
            &p,
            3,
            PointerType::Touch,
            Point::new(30.0, 10.0),
            contact(),
            KeyModifiers::NONE
        ));
        p.ruler().set_is_visible(false);
        input.reconcile(&p);
        assert!(!input.contains(1));
        assert!(!input.contains(2));
    }
}
