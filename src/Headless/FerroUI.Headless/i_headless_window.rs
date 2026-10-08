//! Port of `IHeadlessWindow.cs`.

use ferroui_base::input::raw::{RawDragEventType, RawPointerEventType};
use ferroui_base::input::{DragDropEffects, IDataTransfer, Key, MouseButton, PhysicalKey, RawInputModifiers};
use ferroui_base::media::imaging::WriteableBitmap;
use ferroui_base::{Point, Vector};
use std::rc::Rc;

/// What the input and rendering extensions of a top-level need from the
/// platform implementation of a headless window.
pub(crate) trait IHeadlessWindow {
    fn get_last_rendered_frame(&self) -> Option<WriteableBitmap>;
    fn key_press(&self, key: Key, modifiers: RawInputModifiers, physical_key: PhysicalKey, key_symbol: Option<&str>);
    fn key_release(&self, key: Key, modifiers: RawInputModifiers, physical_key: PhysicalKey, key_symbol: Option<&str>);
    fn text_input(&self, text: &str);
    fn mouse_down(&self, point: Point, button: MouseButton, modifiers: RawInputModifiers);
    fn mouse_move(&self, point: Point, modifiers: RawInputModifiers);
    fn mouse_up(&self, point: Point, button: MouseButton, modifiers: RawInputModifiers);
    fn mouse_wheel(&self, point: Point, delta: Vector, modifiers: RawInputModifiers);
    fn touch(&self, point: Point, touch_point_id: i64, type_: RawPointerEventType, modifiers: RawInputModifiers);
    fn drag_drop(
        &self,
        point: Point,
        type_: RawDragEventType,
        data: Rc<dyn IDataTransfer>,
        effects: DragDropEffects,
        modifiers: RawInputModifiers,
    );
    fn set_render_scaling(&self, scaling: f64);
}
