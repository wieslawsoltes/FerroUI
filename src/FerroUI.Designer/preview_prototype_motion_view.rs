//! A native transition keeps the destination's live controls and at most one
//! outgoing bitmap. Only the changing surface moves during an overlay transition.
use super::{PreviewPrototypeInput, PreviewPrototypePlayerView};
use crate::preview_prototype_transition::MotionSurface;
use crate::preview_stage_export::capture_preview_stage_bitmap;
use crate::{PreviewPrototypeEffect, PreviewPrototypeMotion, PreviewStageKeyboardInput};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::{SolidColorBrush, Stretch, TranslateTransform};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{Rect, Ref, Size, Vector};
use ferroui_controls::{Border, Canvas, Control, Image};
use std::rc::Rc;
use std::time::{Duration, Instant};

struct Outgoing {
    bitmap: RenderTargetBitmap,
    overlay: Option<(Rect, bool)>,
}

/// The native picture of a scene that is about to be replaced.
pub struct PreviewPrototypeMotionCapture {
    motion: PreviewPrototypeMotion,
    outgoing: Option<Outgoing>,
}

enum Target {
    Fade {
        control: Ref<Control>,
        from: f64,
        to: f64,
    },
    Move {
        control: Ref<Control>,
        transform: Ref<TranslateTransform>,
        offset: Vector,
        entering: bool,
    },
}

impl Target {
    fn move_from(control: Ref<Control>, offset: Vector, entering: bool) -> Self {
        let transform = TranslateTransform::new();
        control.set_render_transform(Some(transform.clone().into()));
        Self::Move {
            control,
            transform,
            offset,
            entering,
        }
    }

    fn apply(&self, progress: f64) {
        match self {
            Self::Fade { control, from, to } => control.set_opacity(from + (to - from) * progress),
            Self::Move {
                transform,
                offset,
                entering,
                ..
            } => {
                let amount = if *entering { 1.0 - progress } else { progress };
                transform.set_x(offset.x * amount);
                transform.set_y(offset.y * amount);
            }
        }
    }

    fn reset(self) {
        match self {
            Self::Fade { control, .. } => control.set_opacity(1.0),
            Self::Move { control, .. } => control.set_render_transform(None),
        }
    }
}

pub(super) struct PlayingMotion {
    motion: PreviewPrototypeMotion,
    start: Instant,
    timer: Rc<DispatcherTimer>,
    targets: Vec<Target>,
    outgoing: Option<Ref<Canvas>>,
}

impl PreviewPrototypePlayerView {
    pub fn set_reduced_motion(&self, reduced: bool) {
        self.reduced_motion.set(reduced);
        if reduced {
            self.finish_motion();
        }
    }

    fn motion_allowed(&self) -> bool {
        !self.reduced_motion.get()
            && self
                .root
                .get_platform_settings()
                .is_none_or(|settings| settings.animations_enabled())
    }

    pub fn motion_running(&self) -> bool {
        self.playing_motion.borrow().is_some()
    }

    pub fn check_motion_input(&self, input: &PreviewPrototypeInput) -> Result<(), String> {
        self.advance_motion();
        let focus = matches!(
            input,
            PreviewPrototypeInput::Keyboard {
                input: PreviewStageKeyboardInput::Focus
            }
        );
        let escape = matches!(input, PreviewPrototypeInput::Keyboard {
            input: PreviewStageKeyboardInput::Key { key }
        } if key == "Escape");
        if self.motion_running() && !focus && !escape {
            return Err("Wait for the prototype transition to finish".into());
        }
        Ok(())
    }

    /// Sample the accepted prior scene before swapping native layers. An empty
    /// capture means the transition is instant for the current viewer.
    pub fn capture_motion(
        &self,
        motion: Option<PreviewPrototypeMotion>,
    ) -> Result<Option<PreviewPrototypeMotionCapture>, String> {
        self.finish_motion();
        let Some(motion) = motion.filter(|_| self.motion_allowed()) else {
            return Ok(None);
        };
        self.artboard.update_layout();
        let outgoing = match (motion.surface, motion.transition.effect) {
            (MotionSurface::ScreenIn | MotionSurface::ScreenOut, _)
            | (MotionSurface::OverlaySwap, PreviewPrototypeEffect::Dissolve) => Some(Outgoing {
                bitmap: capture_preview_stage_bitmap(&self.artboard())?,
                overlay: None,
            }),
            (MotionSurface::OverlayIn, _) => None,
            (MotionSurface::OverlayOut | MotionSurface::OverlaySwap, _) => {
                let (plane, dim) = {
                    let layers = self.layers.borrow();
                    let layer = layers
                        .last()
                        .ok_or("The previous overlay is no longer mounted")?;
                    (
                        layer.plane.clone(),
                        layer.options.is_some_and(|options| options.dim_background),
                    )
                };
                let bounds = plane.bounds();
                Some(Outgoing {
                    bitmap: capture_preview_stage_bitmap(&plane.upcast())?,
                    overlay: Some((bounds, dim)),
                })
            }
        };
        Ok(Some(PreviewPrototypeMotionCapture { motion, outgoing }))
    }

    /// Play against the accepted native layout. Covered modal layers stay still
    /// and the compositor clips both moving planes to the existing artboard.
    pub fn play_motion(self: &Rc<Self>, capture: Option<PreviewPrototypeMotionCapture>) {
        let Some(capture) = capture.filter(|_| self.motion_allowed()) else {
            return;
        };
        let PreviewPrototypeMotionCapture { motion, outgoing } = capture;
        let size = self.artboard.bounds().size();
        let effect = motion.transition.effect;
        let slide = effect != PreviewPrototypeEffect::Dissolve;
        let mut targets = Vec::new();
        let outgoing = outgoing.map(|outgoing| {
            let bounds = outgoing
                .overlay
                .map_or(Rect::from_size(size), |(bounds, _)| bounds);
            let canvas = Canvas::new();
            canvas.set_is_hit_test_visible(false);
            let image = Image::new();
            image.set_source(Some(Rc::new(outgoing.bitmap)));
            image.set_stretch(Stretch::Fill);
            image.set_width(bounds.width);
            image.set_height(bounds.height);
            Canvas::set_left(&image, bounds.x);
            Canvas::set_top(&image, bounds.y);
            if outgoing.overlay.is_some_and(|(_, dim)| dim) {
                let scrim = Border::new();
                scrim.set_width(size.width);
                scrim.set_height(size.height);
                scrim.set_background(Some(SolidColorBrush::parse("#66000000").unwrap().into()));
                targets.push(Target::Fade {
                    control: scrim.clone().upcast(),
                    from: 1.0,
                    to: 0.0,
                });
                canvas.children().add(scrim.upcast::<Control>());
            }
            canvas.children().add(image.clone().upcast::<Control>());
            match motion.surface {
                MotionSurface::ScreenIn if slide => canvas.set_z_index(-1),
                MotionSurface::ScreenOut | MotionSurface::OverlayOut if slide => {
                    let offsets = outside(size, bounds, effect, !motion.reverse);
                    targets.push(Target::move_from(image.upcast(), offsets, false));
                }
                _ => targets.push(Target::Fade {
                    control: image.upcast(),
                    from: 1.0,
                    to: 0.0,
                }),
            }
            self.artboard
                .children()
                .add(canvas.clone().upcast::<Control>());
            canvas
        });
        if motion.surface == MotionSurface::ScreenIn && slide {
            let offset = outside(size, Rect::from_size(size), effect, false);
            targets.push(Target::move_from(
                self.content.clone().upcast(),
                offset,
                true,
            ));
        } else if motion.surface == MotionSurface::OverlayIn
            || (slide && motion.surface == MotionSurface::OverlaySwap)
        {
            let layers = self.layers.borrow();
            let layer = layers.last().expect("Accepted overlay layer");
            let plane = layer.plane.clone().upcast::<Control>();
            targets.push(if slide {
                Target::move_from(
                    plane,
                    outside(size, layer.plane.bounds(), effect, false),
                    true,
                )
            } else {
                Target::Fade {
                    control: plane,
                    from: 0.0,
                    to: 1.0,
                }
            });
            if let Some(scrim) = &layer.scrim {
                targets.push(Target::Fade {
                    control: scrim.clone().upcast(),
                    from: 0.0,
                    to: 1.0,
                });
            }
        }
        for target in &targets {
            target.apply(0.0);
        }
        self.content.set_is_hit_test_visible(false);
        let weak = Rc::downgrade(self);
        let timer = DispatcherTimer::with_callback(
            Duration::from_millis(16),
            DispatcherPriority::RENDER,
            move |timer| {
                if let Some(view) = weak.upgrade() {
                    view.advance_motion();
                } else {
                    timer.stop();
                }
            },
        );
        *self.playing_motion.borrow_mut() = Some(PlayingMotion {
            motion,
            start: Instant::now(),
            timer,
            targets,
            outgoing,
        });
    }

    /// Refresh time before painting the next native frame.
    pub fn advance_motion(&self) {
        let complete = {
            let current = self.playing_motion.borrow();
            let Some(playing) = current.as_ref() else {
                return;
            };
            let elapsed = playing.start.elapsed();
            if !self.motion_allowed()
                || elapsed.as_millis() >= u128::from(playing.motion.transition.duration_ms)
            {
                true
            } else {
                let progress = playing
                    .motion
                    .transition
                    .progress(elapsed, playing.motion.reverse);
                for target in &playing.targets {
                    target.apply(progress);
                }
                false
            }
        };
        if complete {
            self.finish_motion();
        }
    }

    pub fn finish_motion(&self) {
        let Some(playing) = self.playing_motion.borrow_mut().take() else {
            return;
        };
        playing.timer.stop();
        if let Some(outgoing) = playing.outgoing {
            self.artboard
                .children()
                .remove(outgoing.upcast::<Control>());
        }
        for target in playing.targets {
            target.reset();
        }
        self.content.set_is_hit_test_visible(true);
    }
}

fn outside(size: Size, bounds: Rect, effect: PreviewPrototypeEffect, opposite: bool) -> Vector {
    let (mut x, mut y) = effect.travel();
    if opposite {
        x = -x;
        y = -y;
    }
    let offset = |travel: f64, origin: f64, end: f64, extent: f64| {
        if travel < 0.0 {
            extent - origin
        } else if travel > 0.0 {
            -end
        } else {
            0.0
        }
    };
    Vector::new(
        offset(x, bounds.x, bounds.right(), size.width),
        offset(y, bounds.y, bounds.bottom(), size.height),
    )
}
