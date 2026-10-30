use super::ink_events::InkEvent;
use super::{InkPoint, InkPresenter};
use ferroui_base::input::PointerType;
use ferroui_base::reactive::IDisposable;
use std::cell::Cell;
use std::rc::{Rc, Weak};

bitflags::bitflags! {
    /// Devices admitted to the inking pipeline. Pen is the default.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct CoreInputDeviceTypes: u32 {
        const NONE = 0;
        const TOUCH = 1;
        const PEN = 2;
        const MOUSE = 4;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkInputProcessingMode {
    None = 0,
    #[default]
    Inking = 1,
    Erasing = 2,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkInputRightDragAction {
    #[default]
    LeaveUnprocessed = 0,
    AllowProcessing = 1,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkHighContrastAdjustment {
    #[default]
    UseSystemColorsWhenNecessary = 0,
    UseSystemColors = 1,
    UseOriginalColors = 2,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkPresenterPredefinedConfiguration {
    #[default]
    SimpleSinglePointer = 0,
    SimpleMultiplePointer = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InkPointerPhase {
    Entered,
    Hovered,
    Exited,
    Pressed,
    Moved,
    Released,
    Lost,
}

/// Device-independent input; coordinates are in the canvas's local DIPs.
/// `handled` is shared through synchronous callback delivery.
#[derive(Clone, Debug, PartialEq)]
pub struct InkPointerEventArgs {
    pub pointer_id: i32,
    pub device_type: PointerType,
    pub point: InkPoint,
    pub is_barrel_button_pressed: bool,
    pub is_eraser: bool,
    pub is_right_button_pressed: bool,
    handled: Rc<Cell<bool>>,
}

macro_rules! identity_equality {
    ($($type:ty),* $(,)?) => { $(impl PartialEq for $type {
        fn eq(&self, other: &Self) -> bool { std::ptr::eq(self, other) }
    })* };
}
identity_equality!(
    InkInputProcessingConfiguration,
    InkInputConfiguration,
    InkStrokeInput,
    InkUnprocessedInput
);

impl InkPointerEventArgs {
    pub fn new(pointer_id: i32, device_type: PointerType, point: InkPoint) -> Self {
        Self {
            pointer_id,
            device_type,
            point,
            is_barrel_button_pressed: false,
            is_eraser: false,
            is_right_button_pressed: false,
            handled: Rc::new(Cell::new(false)),
        }
    }
    pub fn handled(&self) -> bool {
        self.handled.get()
    }
    pub fn set_handled(&self, value: bool) {
        self.handled.set(value);
    }
}

pub struct InkInputProcessingConfiguration {
    presenter: Weak<InkPresenter>,
    mode: Cell<InkInputProcessingMode>,
    right_drag_action: Cell<InkInputRightDragAction>,
}

impl InkInputProcessingConfiguration {
    pub(crate) fn new(presenter: Weak<InkPresenter>) -> Self {
        Self {
            presenter,
            mode: Cell::new(InkInputProcessingMode::Inking),
            right_drag_action: Cell::new(InkInputRightDragAction::LeaveUnprocessed),
        }
    }
    pub fn mode(&self) -> InkInputProcessingMode {
        self.mode.get()
    }
    pub fn set_mode(&self, value: InkInputProcessingMode) {
        if self.mode.replace(value) != value {
            if let Some(p) = self.presenter.upgrade() {
                p.cancel_all();
            }
        }
    }
    pub fn right_drag_action(&self) -> InkInputRightDragAction {
        self.right_drag_action.get()
    }
    pub fn set_right_drag_action(&self, value: InkInputRightDragAction) {
        if self.right_drag_action.replace(value) != value {
            if let Some(p) = self.presenter.upgrade() {
                p.cancel_all();
            }
        }
    }
}

pub struct InkInputConfiguration {
    primary_barrel: Cell<bool>,
    eraser: Cell<bool>,
}

impl Default for InkInputConfiguration {
    fn default() -> Self {
        Self {
            primary_barrel: Cell::new(true),
            eraser: Cell::new(true),
        }
    }
}

impl InkInputConfiguration {
    pub fn is_primary_barrel_button_input_enabled(&self) -> bool {
        self.primary_barrel.get()
    }
    pub fn set_is_primary_barrel_button_input_enabled(&self, value: bool) {
        self.primary_barrel.set(value);
    }
    pub fn is_eraser_input_enabled(&self) -> bool {
        self.eraser.get()
    }
    pub fn set_is_eraser_input_enabled(&self, value: bool) {
        self.eraser.set(value);
    }
}

/// Stable presenter-owned input event service, with a weak back-pointer.
pub struct InkStrokeInput {
    presenter: Weak<InkPresenter>,
    pub(crate) started: InkEvent<InkPointerEventArgs>,
    pub(crate) continued: InkEvent<InkPointerEventArgs>,
    pub(crate) ended: InkEvent<InkPointerEventArgs>,
    pub(crate) canceled: InkEvent<InkPointerEventArgs>,
}

impl InkStrokeInput {
    pub(crate) fn new(presenter: Weak<InkPresenter>) -> Self {
        Self {
            presenter,
            started: InkEvent::default(),
            continued: InkEvent::default(),
            ended: InkEvent::default(),
            canceled: InkEvent::default(),
        }
    }
    pub fn ink_presenter(&self) -> Option<Rc<InkPresenter>> {
        self.presenter.upgrade()
    }
    pub fn stroke_started(
        &self,
        handler: impl Fn(&InkPointerEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.started.subscribe(handler)
    }
    pub fn stroke_continued(
        &self,
        handler: impl Fn(&InkPointerEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.continued.subscribe(handler)
    }
    pub fn stroke_ended(
        &self,
        handler: impl Fn(&InkPointerEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.ended.subscribe(handler)
    }
    pub fn stroke_canceled(
        &self,
        handler: impl Fn(&InkPointerEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.canceled.subscribe(handler)
    }
}

pub struct InkUnprocessedInput {
    presenter: Weak<InkPresenter>,
    entered: InkEvent<InkPointerEventArgs>,
    hovered: InkEvent<InkPointerEventArgs>,
    exited: InkEvent<InkPointerEventArgs>,
    pressed: InkEvent<InkPointerEventArgs>,
    moved: InkEvent<InkPointerEventArgs>,
    released: InkEvent<InkPointerEventArgs>,
    lost: InkEvent<InkPointerEventArgs>,
}

impl InkUnprocessedInput {
    pub(crate) fn new(presenter: Weak<InkPresenter>) -> Self {
        Self {
            presenter,
            entered: InkEvent::default(),
            hovered: InkEvent::default(),
            exited: InkEvent::default(),
            pressed: InkEvent::default(),
            moved: InkEvent::default(),
            released: InkEvent::default(),
            lost: InkEvent::default(),
        }
    }
    pub fn ink_presenter(&self) -> Option<Rc<InkPresenter>> {
        self.presenter.upgrade()
    }
    pub fn pointer_entered(
        &self,
        h: impl Fn(&InkPointerEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.entered.subscribe(h)
    }
    pub fn pointer_hovered(
        &self,
        h: impl Fn(&InkPointerEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.hovered.subscribe(h)
    }
    pub fn pointer_exited(
        &self,
        h: impl Fn(&InkPointerEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.exited.subscribe(h)
    }
    pub fn pointer_pressed(
        &self,
        h: impl Fn(&InkPointerEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.pressed.subscribe(h)
    }
    pub fn pointer_moved(&self, h: impl Fn(&InkPointerEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.moved.subscribe(h)
    }
    pub fn pointer_released(
        &self,
        h: impl Fn(&InkPointerEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.released.subscribe(h)
    }
    pub fn pointer_lost(&self, h: impl Fn(&InkPointerEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.lost.subscribe(h)
    }
    pub(crate) fn raise(&self, phase: InkPointerPhase, e: &InkPointerEventArgs) {
        match phase {
            InkPointerPhase::Entered => &self.entered,
            InkPointerPhase::Hovered => &self.hovered,
            InkPointerPhase::Exited => &self.exited,
            InkPointerPhase::Pressed => &self.pressed,
            InkPointerPhase::Moved => &self.moved,
            InkPointerPhase::Released => &self.released,
            InkPointerPhase::Lost => &self.lost,
        }
        .raise(e);
    }
}
