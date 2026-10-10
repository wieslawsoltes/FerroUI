//! What the drag and drop classes ask of a window (the port of
//! `IXdndWindow.cs`).

use crate::xlib::XID;
use ferroui_base::input::IInputRoot;
use ferroui_base::{PixelPoint, Point};
use std::rc::Rc;

pub trait IXdndWindow {
    fn handle(&self) -> XID;

    fn input_root(&self) -> Option<Rc<dyn IInputRoot>>;

    fn point_to_client(&self, point: PixelPoint) -> Point;
}
