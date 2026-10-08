//! Port of `HeadlessWindowExtensions.cs`: the extensions of a top-level
//! that simulate input on a headless window and read what it rendered.

use crate::ferro_headless_platform::FerroHeadlessPlatform;
use crate::headless_platform_render_interface::HeadlessPlatformRenderInterface;
use crate::headless_window_impl::HeadlessWindowImpl;
use crate::i_headless_touch_pointer::IHeadlessTouchPointer;
use crate::i_headless_window::IHeadlessWindow;
use ferroui_base::input::raw::{RawDragEventType, RawPointerEventType};
use ferroui_base::input::{DragDropEffects, IDataTransfer, Key, MouseButton, PhysicalKey, RawInputModifiers};
use ferroui_base::media::imaging::WriteableBitmap;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{Point, Ref, Vector};
use ferroui_controls::TopLevel;
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};

static NEXT_TOUCH_POINT_ID: AtomicI64 = AtomicI64::new(0);

/// Set of extension methods to simplify usage of headless windows in the
/// headless scenarios.
///
/// The parameters that are optional in the original
/// (`RawInputModifiers.None`) are always given.
pub trait HeadlessWindowExtensions {
    /// Triggers a renderer timer tick and captures last rendered frame.
    ///
    /// Returns the bitmap with last rendered frame; `None` if nothing was
    /// rendered.
    fn capture_rendered_frame(&self) -> Option<WriteableBitmap>;

    /// Reads last rendered frame.
    /// Note, in order to trigger rendering timer,
    /// [`FerroHeadlessPlatform::force_render_timer_tick`] method should be
    /// called.
    ///
    /// Returns the bitmap with last rendered frame; `None` if nothing was
    /// rendered.
    ///
    /// # Panics
    /// Panics if the application uses the headless drawing
    /// (`NotSupportedException`): it draws nothing.
    fn get_last_rendered_frame(&self) -> Option<WriteableBitmap>;

    /// Simulates a keyboard press on the headless window/toplevel.
    fn key_press(&self, key: Key, modifiers: RawInputModifiers, physical_key: PhysicalKey, key_symbol: Option<&str>);

    /// Simulates a keyboard press on the headless window/toplevel, as if
    /// typed on a QWERTY keyboard.
    fn key_press_qwerty(&self, physical_key: PhysicalKey, modifiers: RawInputModifiers);

    /// Simulates a keyboard release on the headless window/toplevel.
    fn key_release(&self, key: Key, modifiers: RawInputModifiers, physical_key: PhysicalKey, key_symbol: Option<&str>);

    /// Simulates a keyboard release on the headless window/toplevel, as if
    /// typed on a QWERTY keyboard.
    fn key_release_qwerty(&self, physical_key: PhysicalKey, modifiers: RawInputModifiers);

    /// Simulates a text input event on the headless window/toplevel.
    ///
    /// This event is independent of [`key_press`](Self::key_press) and
    /// [`key_release`](Self::key_release). If you need to simulate text
    /// input to a TextBox or a similar control, please use this method.
    fn key_text_input(&self, text: &str);

    /// Simulates a mouse down on the headless window/toplevel.
    ///
    /// In the headless platform, there is a single mouse pointer. There are
    /// no helper methods for pen input; for touch, see
    /// [`touch_begin`](Self::touch_begin).
    fn mouse_down(&self, point: Point, button: MouseButton, modifiers: RawInputModifiers);

    /// Simulates a mouse move on the headless window/toplevel.
    fn mouse_move(&self, point: Point, modifiers: RawInputModifiers);

    /// Simulates a mouse up on the headless window/toplevel.
    fn mouse_up(&self, point: Point, button: MouseButton, modifiers: RawInputModifiers);

    /// Simulates a mouse wheel on the headless window/toplevel.
    fn mouse_wheel(&self, point: Point, delta: Vector, modifiers: RawInputModifiers);

    /// Begins a simulated touch contact on the headless window/toplevel.
    ///
    /// Each call begins an independent contact with its own unique touch
    /// id, so several contacts can be active at the same time to simulate
    /// multi-touch gestures.
    ///
    /// Returns a touch pointer representing the contact. Pass it to
    /// [`touch_move`](Self::touch_move) and [`touch_end`](Self::touch_end),
    /// or dispose it to cancel the contact.
    fn touch_begin(&self, point: Point, modifiers: RawInputModifiers) -> Rc<dyn IHeadlessTouchPointer>;

    /// Moves a simulated touch contact started by
    /// [`touch_begin`](Self::touch_begin).
    fn touch_move(&self, touch_pointer: &dyn IHeadlessTouchPointer, point: Point, modifiers: RawInputModifiers);

    /// Ends a simulated touch contact started by
    /// [`touch_begin`](Self::touch_begin), as if the finger was lifted.
    fn touch_end(&self, touch_pointer: &dyn IHeadlessTouchPointer, point: Point, modifiers: RawInputModifiers);

    /// Simulates a drag and drop target event on the headless
    /// window/toplevel. This event simulates a user moving files from
    /// another app to the current app.
    fn drag_drop(
        &self,
        point: Point,
        type_: RawDragEventType,
        data: Rc<dyn IDataTransfer>,
        effects: DragDropEffects,
        modifiers: RawInputModifiers,
    );

    /// Changes the render scaling (DPI scaling) of the headless
    /// window/toplevel. This simulates a DPI change, triggering scaling
    /// changed notifications and a layout pass.
    fn set_render_scaling(&self, scaling: f64);
}

impl HeadlessWindowExtensions for TopLevel {
    fn capture_rendered_frame(&self) -> Option<WriteableBitmap> {
        let mut bitmap = None;
        run_jobs_on_impl(self, |w| bitmap = w.get_last_rendered_frame());
        bitmap
    }

    fn get_last_rendered_frame(&self) -> Option<WriteableBitmap> {
        if HeadlessPlatformRenderInterface::is_current() {
            panic!(
                "To capture a rendered frame, make sure that headless application was initialized with the Skia backend and disabled 'use_headless_drawing' in the 'FerroHeadlessPlatformOptions'."
            );
        }

        with_impl(self, |w| w.get_last_rendered_frame())
    }

    fn key_press(&self, key: Key, modifiers: RawInputModifiers, physical_key: PhysicalKey, key_symbol: Option<&str>) {
        run_jobs_on_impl(self, |w| w.key_press(key, modifiers, physical_key, key_symbol));
    }

    fn key_press_qwerty(&self, physical_key: PhysicalKey, modifiers: RawInputModifiers) {
        run_jobs_on_impl(self, |w| {
            w.key_press(physical_key.to_qwerty_key(), modifiers, physical_key, physical_key.to_qwerty_key_symbol(false))
        });
    }

    fn key_release(&self, key: Key, modifiers: RawInputModifiers, physical_key: PhysicalKey, key_symbol: Option<&str>) {
        run_jobs_on_impl(self, |w| w.key_release(key, modifiers, physical_key, key_symbol));
    }

    fn key_release_qwerty(&self, physical_key: PhysicalKey, modifiers: RawInputModifiers) {
        run_jobs_on_impl(self, |w| {
            w.key_release(physical_key.to_qwerty_key(), modifiers, physical_key, physical_key.to_qwerty_key_symbol(false))
        });
    }

    fn key_text_input(&self, text: &str) {
        run_jobs_on_impl(self, |w| w.text_input(text));
    }

    fn mouse_down(&self, point: Point, button: MouseButton, modifiers: RawInputModifiers) {
        run_jobs_on_impl(self, |w| w.mouse_down(point, button, modifiers));
    }

    fn mouse_move(&self, point: Point, modifiers: RawInputModifiers) {
        run_jobs_on_impl(self, |w| w.mouse_move(point, modifiers));
    }

    fn mouse_up(&self, point: Point, button: MouseButton, modifiers: RawInputModifiers) {
        run_jobs_on_impl(self, |w| w.mouse_up(point, button, modifiers));
    }

    fn mouse_wheel(&self, point: Point, delta: Vector, modifiers: RawInputModifiers) {
        run_jobs_on_impl(self, |w| w.mouse_wheel(point, delta, modifiers));
    }

    fn touch_begin(&self, point: Point, modifiers: RawInputModifiers) -> Rc<dyn IHeadlessTouchPointer> {
        let touch_point_id = NEXT_TOUCH_POINT_ID.fetch_add(1, Ordering::SeqCst) + 1;
        run_jobs_on_impl(self, |w| w.touch(point, touch_point_id, RawPointerEventType::TouchBegin, modifiers));
        Rc::new(HeadlessTouchPointer {
            top_level: self.to_ref(),
            touch_point_id,
            position: Cell::new(point),
            pressed: Cell::new(true),
        })
    }

    fn touch_move(&self, touch_pointer: &dyn IHeadlessTouchPointer, point: Point, modifiers: RawInputModifiers) {
        get_touch_pointer(self, touch_pointer).move_(point, modifiers);
    }

    fn touch_end(&self, touch_pointer: &dyn IHeadlessTouchPointer, point: Point, modifiers: RawInputModifiers) {
        get_touch_pointer(self, touch_pointer).end(point, modifiers);
    }

    fn drag_drop(
        &self,
        point: Point,
        type_: RawDragEventType,
        data: Rc<dyn IDataTransfer>,
        effects: DragDropEffects,
        modifiers: RawInputModifiers,
    ) {
        run_jobs_on_impl(self, |w| w.drag_drop(point, type_, data, effects, modifiers));
    }

    fn set_render_scaling(&self, scaling: f64) {
        run_jobs_on_impl(self, |w| w.set_render_scaling(scaling));
    }
}

fn run_jobs_on_impl(top_level: &TopLevel, action: impl FnOnce(&HeadlessWindowImpl)) {
    run_jobs_and_render();
    with_impl(top_level, action);
    run_jobs_and_render();
}

fn run_jobs_and_render() {
    let dispatcher = Dispatcher::ui_thread();

    // Run jobs and render frames until everything is stable.
    // We use a simple approach: run jobs, render, and repeat until
    // there are no more pending jobs. The render timer tick can schedule
    // new jobs, so we loop until stable.
    for _ in 0..10 {
        dispatcher.run_jobs(None);
        FerroHeadlessPlatform::force_render_timer_tick(1);

        // `DispatcherPriority.MinimumActiveValue`, which is the system idle priority.
        if !dispatcher.has_jobs_with_priority(DispatcherPriority::SYSTEM_IDLE) {
            return;
        }
    }

    // Final attempt: run remaining jobs without rendering
    dispatcher.run_jobs(None);
}

/// `GetImpl`, for the duration of `f`.
///
/// # Panics
/// Panics if the top-level is closed (`ObjectDisposedException`) or is not a
/// headless window (`InvalidOperationException`).
fn with_impl<R>(top_level: &TopLevel, f: impl FnOnce(&HeadlessWindowImpl) -> R) -> R {
    let Some(platform_impl) = top_level.platform_impl() else {
        panic!("Cannot access a disposed object.\nObject name: '{}'.", top_level.get_type().name());
    };
    match platform_impl.as_any().downcast_ref::<HeadlessWindowImpl>() {
        Some(headless) => f(headless),
        None => panic!("TopLevel must be a headless window."),
    }
}

/// `topLevel.PlatformImpl is IHeadlessWindow`.
fn has_headless_impl(top_level: &TopLevel) -> bool {
    top_level
        .platform_impl()
        .is_some_and(|platform_impl| platform_impl.as_any().downcast_ref::<HeadlessWindowImpl>().is_some())
}

/// # Panics
/// Panics if the touch pointer was not created by `touch_begin` or belongs
/// to another top-level (`ArgumentException`).
fn get_touch_pointer<'a>(top_level: &TopLevel, touch_pointer: &'a dyn IHeadlessTouchPointer) -> &'a HeadlessTouchPointer {
    let Some(headless_touch_pointer) = touch_pointer.as_any().downcast_ref::<HeadlessTouchPointer>() else {
        panic!("The touch pointer was not created by TouchBegin. (Parameter 'touchPointer')");
    };
    if headless_touch_pointer.top_level != top_level.to_ref() {
        panic!("The touch pointer belongs to a different toplevel. (Parameter 'touchPointer')");
    }
    headless_touch_pointer
}

struct HeadlessTouchPointer {
    top_level: Ref<TopLevel>,
    touch_point_id: i64,
    position: Cell<Point>,
    pressed: Cell<bool>,
}

impl HeadlessTouchPointer {
    fn move_(&self, point: Point, modifiers: RawInputModifiers) {
        self.throw_if_released();
        let touch_point_id = self.touch_point_id;
        run_jobs_on_impl(&self.top_level, |w| w.touch(point, touch_point_id, RawPointerEventType::TouchUpdate, modifiers));
        self.position.set(point);
    }

    fn end(&self, point: Point, modifiers: RawInputModifiers) {
        self.throw_if_released();
        self.pressed.set(false);
        let touch_point_id = self.touch_point_id;
        run_jobs_on_impl(&self.top_level, |w| w.touch(point, touch_point_id, RawPointerEventType::TouchEnd, modifiers));
    }

    /// # Panics
    /// Panics if the contact was released (`InvalidOperationException`).
    fn throw_if_released(&self) {
        if !self.pressed.get() {
            panic!("The touch pointer has already been released.");
        }
    }
}

impl IDisposable for HeadlessTouchPointer {
    fn dispose(&self) {
        if !self.pressed.get() {
            return;
        }
        self.pressed.set(false);

        // The toplevel might have been closed already, cancelling all of its touch pointers.
        if has_headless_impl(&self.top_level) {
            let position = self.position.get();
            let touch_point_id = self.touch_point_id;
            run_jobs_on_impl(&self.top_level, |w| {
                w.touch(position, touch_point_id, RawPointerEventType::TouchCancel, RawInputModifiers::empty())
            });
        }
    }
}

impl IHeadlessTouchPointer for HeadlessTouchPointer {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
