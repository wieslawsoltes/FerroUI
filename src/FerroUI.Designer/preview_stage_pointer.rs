//! Contact input for a stage rendered as an image or with an emulated input device.
use super::{
    with_runtime_errors, PreviewStageBoard, PreviewStageId, PreviewStageMode, RenderedStage, Stage,
};
use ferroui_base::diagnostics::DesignTextEnvironment;
use ferroui_base::input::raw::{
    RawPointerEventArgs, RawPointerEventType, RawPointerPoint, RawTouchEventArgs,
};
use ferroui_base::input::{InputManager, MouseDevice, PenDevice, RawInputModifiers, TouchDevice};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{Point, Ref, Visual};
use ferroui_controls::{Control, TopLevel};
use ferroui_markup_xaml::source_overlay::SourceSnapshot;
use serde::{Deserialize, Serialize};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStagePointerProfile {
    #[default]
    Automatic,
    Touch,
    Pen,
}

impl PreviewStagePointerProfile {
    pub(crate) const OPTIONS: [Self; 3] = [Self::Automatic, Self::Touch, Self::Pen];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Automatic => "Current device",
            Self::Touch => "Touch",
            Self::Pen => "Pen",
        }
    }

    pub(crate) fn is_default(&self) -> bool {
        *self == Self::Automatic
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStagePointerDevice {
    #[default]
    Mouse,
    Touch,
    Pen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStagePointerPhase {
    Down,
    Move,
    Up,
    Cancel,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PreviewStagePointerInput {
    pub contact: u32,
    pub phase: PreviewStagePointerPhase,
    pub device: PreviewStagePointerDevice,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub pressure: Option<f32>,
    #[serde(default)]
    pub tilt_x: Option<f32>,
    #[serde(default)]
    pub tilt_y: Option<f32>,
    #[serde(default)]
    pub twist: Option<f32>,
}

/// Keep interactive contacts and automation contacts independent, even when IDs overlap.
#[derive(Clone, Copy)]
pub enum PreviewStagePointerOrigin {
    Editor,
    Automation,
}

#[derive(Default)]
pub(super) struct StagePointerStreams {
    editor: Rc<PointerStream>,
    automation: Rc<PointerStream>,
}

impl StagePointerStreams {
    fn get(&self, origin: PreviewStagePointerOrigin) -> &Rc<PointerStream> {
        match origin {
            PreviewStagePointerOrigin::Editor => &self.editor,
            PreviewStagePointerOrigin::Automation => &self.automation,
        }
    }

    pub(super) fn retire(
        &mut self,
        rendered: Option<&RenderedStage>,
        origin: PreviewStagePointerOrigin,
    ) {
        let stream = match origin {
            PreviewStagePointerOrigin::Editor => &mut self.editor,
            PreviewStagePointerOrigin::Automation => &mut self.automation,
        };
        let has_devices = stream.mouse.get().is_some()
            || stream.touch.get().is_some()
            || stream.pen.get().is_some();
        if !has_devices && Rc::strong_count(stream) == 1 {
            return;
        }
        stream.retired.set(true);
        let devices = std::mem::take(stream);
        if !has_devices {
            return;
        }
        let sources = rendered.map(|rendered| rendered.source_snapshot.clone());
        let text = rendered.and_then(|rendered| DesignTextEnvironment::get(&rendered.scope));
        // Native capture loss can run authored handlers. Run it outside the board's
        // mutable borrow. Retiring the stream already prevents further delivery.
        Dispatcher::ui_thread().post_local(
            move || {
                let release = || {
                    if let Some(mouse) = devices.mouse.get() {
                        mouse.dispose();
                    }
                    if let Some(touch) = devices.touch.get() {
                        touch.dispose();
                    }
                    if let Some(pen) = devices.pen.get() {
                        pen.dispose();
                    }
                };
                DesignTextEnvironment::run(text.as_ref(), || {
                    if let Some(sources) = sources {
                        sources.run(release)
                    } else {
                        release()
                    }
                });
            },
            DispatcherPriority::INPUT,
        );
    }
}

#[derive(Default)]
struct PointerStream {
    active: RefCell<BTreeMap<u32, PreviewStagePointerDevice>>,
    mouse: OnceCell<Rc<MouseDevice>>,
    touch: OnceCell<Rc<TouchDevice>>,
    pen: OnceCell<Rc<PenDevice>>,
    retired: Cell<bool>,
}

impl Stage {
    pub(super) fn retire_pointer_streams(&mut self) {
        for origin in [
            PreviewStagePointerOrigin::Editor,
            PreviewStagePointerOrigin::Automation,
        ] {
            self.pointer_streams.retire(self.rendered.as_ref(), origin);
        }
    }
}

/// The accepted native scope. Dispatch immediately outside the board borrow.
pub struct PreviewStagePointerTarget {
    host: Ref<Control>,
    scope: Ref<Control>,
    profile: PreviewStagePointerProfile,
    stream: Rc<PointerStream>,
    sources: SourceSnapshot,
    text: Option<DesignTextEnvironment>,
}

impl PreviewStageBoard {
    pub fn pointer_target(
        &self,
        id: PreviewStageId,
        epoch: u64,
        origin: PreviewStagePointerOrigin,
    ) -> Result<PreviewStagePointerTarget, String> {
        let stage = &self.stages[self.index(id)?];
        if stage.epoch != epoch {
            return Err("The stage changed. Use its current frame.".into());
        }
        let config = &stage.spec.configuration;
        if config.mode != PreviewStageMode::Live || !config.auto_update {
            return Err(
                "Switch this stage to Interact and resume updates to use pointer input".into(),
            );
        }
        let rendered = self.current(stage)?;
        Ok(PreviewStagePointerTarget {
            host: stage.control.clone().upcast(),
            scope: rendered.scope.clone().upcast(),
            profile: config.pointer_input,
            stream: stage.pointer_streams.get(origin).clone(),
            sources: rendered.source_snapshot.clone(),
            text: DesignTextEnvironment::get(&rendered.scope),
        })
    }

    pub fn cancel_pointer_input(&mut self, origin: PreviewStagePointerOrigin) {
        for stage in &mut self.stages {
            stage
                .pointer_streams
                .retire(stage.rendered.as_ref(), origin);
        }
    }
}

impl PreviewStagePointerTarget {
    /// Route through the composite artboard so the modal plane owns hit testing.
    pub fn on_surface(mut self, surface: Ref<Control>) -> Result<Self, String> {
        if !surface.is_visual_ancestor_of(&self.scope) {
            return Err("The native stage is outside its presentation surface".into());
        }
        self.host = surface.clone();
        self.scope = surface;
        Ok(self)
    }

    pub fn input(self, input: PreviewStagePointerInput) -> Result<(), String> {
        if self.stream.retired.get() {
            return Err("The stage changed. Use its current frame.".into());
        }
        DesignTextEnvironment::run(self.text.as_ref(), || {
            self.sources
                .run(|| with_runtime_errors("pointer input", || self.dispatch(input)))
        })
    }

    fn dispatch(&self, input: PreviewStagePointerInput) -> Result<(), String> {
        use PreviewStagePointerDevice::{Mouse, Pen, Touch};
        use PreviewStagePointerPhase::{Cancel, Down, Move, Up};
        use RawPointerEventType as Raw;

        let inside = (0.0..=1.0).contains(&input.x) && (0.0..=1.0).contains(&input.y);
        if (input.phase == Down && !inside)
            || !(-1024.0..=1024.0).contains(&input.x)
            || !(-1024.0..=1024.0).contains(&input.y)
            || input
                .pressure
                .is_some_and(|value| !(0.0..=1.0).contains(&value))
            || [input.tilt_x, input.tilt_y]
                .into_iter()
                .flatten()
                .any(|value| !(-90.0..=90.0).contains(&value))
            || input
                .twist
                .is_some_and(|value| !(0.0..360.0).contains(&value))
        {
            return Err("Use valid stage coordinates, pressure, tilt and twist".into());
        }
        let top = TopLevel::get_top_level(Some(&self.scope)).ok_or("Mount the stage first")?;
        let root = top.input_root();
        let visual = self.host.upcast_ref::<Visual>();
        let scope = self.scope.upcast_ref::<Visual>();
        let size = self.host.bounds().size();
        let position = Point::new(input.x * size.width, input.y * size.height);
        let local = visual
            .translate_point(position, scope)
            .ok_or("Mount the stage first")?;
        let root_position = visual
            .translate_point(position, &root.root_element())
            .ok_or("Mount the stage first")?;
        let manager = InputManager::instance().ok_or("The native input manager is unavailable")?;
        let hit = inside
            .then(|| self.scope.input_hit_test_with(local, false))
            .flatten();
        let enabled = inside.then(|| {
            self.scope
                .input_hit_test(local)
                .unwrap_or_else(|| self.scope.clone().upcast())
        });
        let device = match self.profile {
            PreviewStagePointerProfile::Automatic => input.device,
            PreviewStagePointerProfile::Touch => Touch,
            PreviewStagePointerProfile::Pen => Pen,
        };
        {
            let mut active = self.stream.active.borrow_mut();
            if input.phase == Down {
                if active.len() >= 10
                    || active.contains_key(&input.contact)
                    || (device == Mouse && active.values().any(|current| *current == Mouse))
                {
                    return Err("Release an active stage contact first".into());
                }
                active.insert(input.contact, device);
            } else {
                if active.get(&input.contact) != Some(&device) {
                    return Err("Begin the pointer contact in this stage first".into());
                }
                if matches!(input.phase, Up | Cancel) {
                    active.remove(&input.contact);
                }
            }
        }
        static CLOCK: OnceLock<Instant> = OnceLock::new();
        let timestamp = CLOCK.get_or_init(Instant::now).elapsed().as_millis() as u64;
        let mut point = RawPointerPoint::new();
        point.position = root_position;
        point.pressure = input
            .pressure
            .unwrap_or(if matches!(input.phase, Up | Cancel) {
                0.0
            } else {
                0.5
            });
        point.x_tilt = input.tilt_x.unwrap_or(0.0);
        point.y_tilt = input.tilt_y.unwrap_or(0.0);
        point.twist = input.twist.unwrap_or(0.0);
        let modifiers = if matches!(input.phase, Down | Move) {
            RawInputModifiers::LEFT_MOUSE_BUTTON
        } else {
            RawInputModifiers::NONE
        };
        if device == Touch {
            let event = match input.phase {
                Down => Raw::TouchBegin,
                Move => Raw::TouchUpdate,
                Up => Raw::TouchEnd,
                Cancel => Raw::TouchCancel,
            };
            let touch = self.stream.touch.get_or_init(TouchDevice::new).clone();
            let raw = RawTouchEventArgs::with_point(
                touch,
                timestamp,
                root,
                event,
                point,
                modifiers,
                input.contact.into(),
            );
            // Touch's no-hit fallback is the whole window. Keep it inside this stage
            // when a contact moves off the artboard and the app releases its capture.
            raw.set_input_hit_test_result(
                hit,
                enabled.or_else(|| Some(self.scope.clone().upcast())),
            );
            manager.process_input(Rc::new(raw));
        } else {
            let native: Rc<dyn ferroui_base::input::IInputDevice> = match device {
                Mouse => self.stream.mouse.get_or_init(MouseDevice::new).clone(),
                Pen => self.stream.pen.get_or_init(|| PenDevice::new(true)).clone(),
                Touch => unreachable!(),
            };
            let event = match input.phase {
                Down => Raw::LeftButtonDown,
                Move => Raw::Move,
                Up => Raw::LeftButtonUp,
                Cancel if device == Pen => Raw::LeaveWindow,
                Cancel => Raw::CancelCapture,
            };
            let raw =
                RawPointerEventArgs::with_point(native, timestamp, root, event, point, modifiers);
            raw.set_raw_pointer_id(input.contact.into());
            raw.set_input_hit_test_result(hit, enabled);
            manager.process_input(Rc::new(raw));
        }
        Ok(())
    }
}
