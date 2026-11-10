//! Composition selection-indicator motion shared by NavigationView and TabControl.
//!
//! Port of NavigationView::{AnimateSelectionChanged, PlayIndicatorAnimations,
//! PlayIndicatorNonSameLevelAnimations, OnAnimationComplete} in microsoft-ui-xaml,
//! commit 388069e3f18abafdd74d68b0b3cd1b3fa4dbb097 (MIT, Microsoft Corporation).
//! Motion uses the element's handoff Offset plus its layout origin, as in WinUI.

use crate::Control;
use ferroui_base::numerics::Vector2;
use ferroui_base::platform::IPlatformSettings;
use ferroui_base::reactive::Subscription;
use ferroui_base::rendering::composition::{
    CompositionBatchTypes, CompositionScopedBatch, CompositionVisual, ElementComposition, ICompositionObjectAnimations,
};
use ferroui_base::{FerroLocator, LocatorExtensions, Point, Ref, Size, Vector3D, Visual, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[derive(Default)]
pub(crate) struct SelectionIndicatorAnimation {
    state: Rc<RefCell<State>>,
}

#[derive(Default)]
struct State {
    active: Option<WeakRef<Control>>,
    horizontal: bool,
    generation: u64,
    running: Option<Running>,
}

struct Indicator {
    element: WeakRef<Control>,
    visual: Rc<CompositionVisual>,
    layout_offset: Vector3D,
}

struct Running {
    previous: Indicator,
    next: Indicator,
    next_position: Point,
    next_size: Size,
    _batch: Rc<CompositionScopedBatch>,
    _completed: Subscription,
}

impl Indicator {
    fn new(element: &Ref<Control>) -> Option<Self> {
        Some(Self {
            element: element.downgrade(),
            visual: ElementComposition::get_element_visual(element)?,
            layout_offset: Vector3D::new(element.bounds().x, element.bounds().y, 0.0),
        })
    }

    fn reset(&self, opacity: f32) {
        let offset = self
            .element
            .upgrade()
            .map(|element| Vector3D::new(element.bounds().x, element.bounds().y, 0.0))
            .unwrap_or(self.layout_offset);
        self.visual.set_offset(offset);
        self.visual.set_scale(Vector3D::new(1.0, 1.0, 1.0));
        self.visual.set_center_point(Vector3D::new(0.0, 0.0, 0.0));
        self.visual.set_opacity(opacity);
        if let Some(element) = self.element.upgrade() {
            element.set_opacity(opacity as f64);
        }
    }
}

impl Running {
    fn finish(self) {
        self.previous.reset(0.0);
        self.next.reset(1.0);
    }
}

fn position(element: &Control, owner: &Visual) -> Option<Point> {
    Some(element.transform_to_visual(owner)?.transform(Point::new(0.0, 0.0)))
}

fn valid_size(size: Size) -> bool {
    size.width.is_finite() && size.height.is_finite() && size.width > 0.001 && size.height > 0.001
}

impl SelectionIndicatorAnimation {
    fn finish(&self) {
        let running = {
            let mut state = self.state.borrow_mut();
            state.generation = state.generation.wrapping_add(1);
            state.running.take()
        };
        if let Some(running) = running {
            running.finish();
        }
    }

    pub(crate) fn clear(&self) {
        self.finish();
        self.state.borrow_mut().active = None;
    }

    /// Call after layout: indicator dimensions, hierarchy and template parts must be current.
    /// Calling again for the same target preserves an in-flight animation, as WinUI does.
    pub(crate) fn update(&self, owner: &Visual, next: Option<Ref<Control>>, horizontal: bool) {
        let next = next.filter(|element| element.is_effectively_visible());
        let enabled = FerroLocator::current()
            .get_service::<dyn IPlatformSettings>()
            .is_none_or(|settings| settings.animations_enabled());
        let (previous, same_axis, same_target, moved) = {
            let state = self.state.borrow();
            let previous = state.active.as_ref().and_then(WeakRef::upgrade);
            let same_target = previous == next;
            let moved = state.running.as_ref().is_some_and(|running| {
                next.as_ref().is_none_or(|next| {
                    position(next, owner) != Some(running.next_position) || next.bounds().size() != running.next_size
                })
            });
            (previous, state.horizontal == horizontal, same_target, moved)
        };
        if same_target && same_axis {
            if moved || !enabled {
                self.finish();
            }
            return;
        }
        let plan = previous.as_ref().zip(next.as_ref()).and_then(|(previous, next)| {
            if !enabled || !same_axis || !previous.is_effectively_visible() {
                return None;
            }
            let previous_position = position(previous, owner)?;
            let next_position = position(next, owner)?;
            let previous_size = previous.bounds().size();
            let next_size = next.bounds().size();
            if !valid_size(previous_size)
                || !valid_size(next_size)
                || ![
                    previous_position.x,
                    previous_position.y,
                    next_position.x,
                    next_position.y,
                ]
                .iter()
                .all(|v| v.is_finite())
            {
                return None;
            }
            let previous = Indicator::new(previous)?;
            let next = Indicator::new(next)?;
            if !Rc::ptr_eq(previous.visual.compositor(), next.visual.compositor()) {
                return None;
            }
            Some((
                previous,
                next,
                previous_position,
                next_position,
                previous_size,
                next_size,
            ))
        });
        self.finish();
        {
            let mut state = self.state.borrow_mut();
            state.active = next.as_ref().map(Ref::downgrade);
            state.horizontal = horizontal;
        }
        // First/cleared selection, absent compositor and unarranged template parts settle.
        if let Some(previous) = &previous {
            previous.set_opacity(0.0);
        }
        if let Some(next) = &next {
            next.set_opacity(1.0);
        }
        let Some((previous, next, previous_position, next_position, previous_size, next_size)) = plan else {
            return;
        };
        previous.reset(1.0);
        next.reset(1.0);
        let compositor = next.visual.compositor();
        let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
        let same_depth = if horizontal {
            (previous_position.y - next_position.y).abs() < 0.001
        } else {
            (previous_position.x - next_position.x).abs() < 0.001
        };
        if same_depth {
            let distance = if horizontal {
                next_position.x - previous_position.x
            } else {
                next_position.y - previous_position.y
            };
            play(
                &previous.visual,
                0.0,
                distance,
                previous_size,
                next_size,
                horizontal,
                true,
            );
            play(
                &next.visual,
                -distance,
                0.0,
                previous_size,
                next_size,
                horizontal,
                false,
            );
        } else {
            let below = previous_position.y < next_position.y;
            play_cross_depth(&previous.visual, previous_size, true, !below);
            play_cross_depth(&next.visual, next_size, false, below);
        }
        let generation = self.state.borrow().generation;
        let weak = Rc::downgrade(&self.state);
        let completed = batch.completed(move |_| {
            let Some(state) = weak.upgrade() else { return };
            let running = {
                let mut state = state.borrow_mut();
                if state.generation != generation {
                    return;
                }
                state.running.take()
            };
            if let Some(running) = running {
                running.finish();
            }
        });
        self.state.borrow_mut().running = Some(Running {
            previous,
            next,
            next_position,
            next_size,
            _batch: batch.clone(),
            _completed: completed,
        });
        batch.end();
    }
}

impl Drop for SelectionIndicatorAnimation {
    fn drop(&mut self) {
        self.clear();
    }
}

fn play(visual: &Rc<CompositionVisual>, from: f64, to: f64, begin: Size, end: Size, horizontal: bool, outgoing: bool) {
    let compositor = visual.compositor();
    // Layout may not yet have synchronized Size to the handoff visual this frame.
    let dimension = if outgoing {
        if horizontal {
            begin.width
        } else {
            begin.height
        }
    } else if horizontal {
        end.width
    } else {
        end.height
    };
    let begin_scale = if horizontal { begin.width / dimension } else { 1.0 };
    let end_scale = if horizontal { end.width / dimension } else { 1.0 };
    let forward = from < to;
    let single_step = compositor.create_step_easing_function();
    single_step.set_is_final_step_single_frame(true);
    // The WinUI stretch has unbounded endpoint velocity: distant selections can cover
    // most of their travel in one frame. Preserve adjacent-item motion, blending to
    // easing with zero endpoint velocity over longer distances without extending the duration.
    let smoothing = (((to - from).abs() - 80.0) / 160.0).clamp(0.0, 1.0) as f32;
    let blend = |near: f32, far: f32| near + (far - near) * smoothing;
    let stretch = compositor.create_cubic_bezier_easing_function(
        Vector2::new(blend(0.9, 0.4), blend(0.1, 0.0)),
        Vector2::new(blend(1.0, 0.6), blend(0.2, 1.0)),
    );
    let contract = compositor.create_cubic_bezier_easing_function(
        Vector2::new(blend(0.1, 0.4), blend(0.9, 0.0)),
        Vector2::new(blend(0.2, 0.6), 1.0),
    );
    let duration = Duration::from_millis(600);
    // Use the same key and duration for the pivot and the offset. Separate 200 ms
    // and 0.333 * 600 ms timelines briefly translate the fully stretched indicator
    // without its compensating pivot, producing a jump of the entire travel distance.
    let pivot = 1.0 / 3.0;
    if outgoing {
        let opacity = compositor.create_scalar_key_frame_animation();
        opacity.insert_key_frame(0.0, 1.0);
        opacity.insert_key_frame_with_easing(pivot, 1.0, single_step.clone());
        opacity.insert_key_frame_with_easing(1.0, 0.0, contract.clone());
        opacity.set_duration(duration);
        visual.start_animation("Opacity", &*opacity);
    }
    let base = if horizontal {
        visual.offset().x
    } else {
        visual.offset().y
    };
    let offset = compositor.create_scalar_key_frame_animation();
    offset.insert_key_frame(
        0.0,
        (base
            + if forward {
                from
            } else {
                from + dimension * (begin_scale - 1.0)
            }) as f32,
    );
    offset.insert_key_frame_with_easing(
        pivot,
        (base
            + if forward {
                to + dimension * (end_scale - 1.0)
            } else {
                to
            }) as f32,
        single_step.clone(),
    );
    offset.set_duration(duration);
    let scale = compositor.create_scalar_key_frame_animation();
    scale.insert_key_frame(0.0, begin_scale as f32);
    scale.insert_key_frame_with_easing(
        pivot,
        ((to - from).abs() / dimension + if forward { end_scale } else { begin_scale }) as f32,
        stretch,
    );
    scale.insert_key_frame_with_easing(1.0, end_scale as f32, contract);
    scale.set_duration(duration);
    let center = compositor.create_scalar_key_frame_animation();
    center.insert_key_frame(0.0, if forward { 0.0 } else { dimension as f32 });
    center.insert_key_frame_with_easing(pivot, if forward { dimension as f32 } else { 0.0 }, single_step);
    center.set_duration(duration);
    visual.start_animation(if horizontal { "Offset.X" } else { "Offset.Y" }, &*offset);
    visual.start_animation(if horizontal { "Scale.X" } else { "Scale.Y" }, &*scale);
    visual.start_animation(if horizontal { "CenterPoint.X" } else { "CenterPoint.Y" }, &*center);
}

fn play_cross_depth(visual: &Rc<CompositionVisual>, size: Size, outgoing: bool, from_top: bool) {
    let horizontal = size.width >= size.height;
    let dimension = if horizontal { size.width } else { size.height };
    // Horizontal indicators shrink/grow about their centre; vertical ones about the edge
    // facing the destination. Use the matching pivot axis (WinUI's top-primary code uses Y).
    let center = if horizontal {
        dimension / 2.0
    } else if from_top {
        0.0
    } else {
        dimension
    };
    visual.set_center_point(if horizontal {
        Vector3D::new(center, 0.0, 0.0)
    } else {
        Vector3D::new(0.0, center, 0.0)
    });
    let scale = visual.compositor().create_scalar_key_frame_animation();
    scale.insert_key_frame(0.0, if outgoing { 1.0 } else { 0.0 });
    scale.insert_key_frame(1.0, if outgoing { 0.0 } else { 1.0 });
    scale.set_duration(Duration::from_millis(600));
    visual.start_animation(if horizontal { "Scale.X" } else { "Scale.Y" }, &*scale);
}

#[cfg(test)]
mod tests;
