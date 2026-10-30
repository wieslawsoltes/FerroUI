// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::controls::DockControl;
use ferroui_base::Ref;
use ferroui_controls::Control;
/// An external control surface participating in dock start resolution and drop hit testing.
pub trait IExternalDockSurface {
    /// Native receiver used by weak surface references and interface identity.
    #[doc(hidden)]
    fn as_object(&self) -> Option<Ref<ferroui_base::FerroObject>> {
        None
    }

    /// Gets the owning source dock control.
    fn dock_control(&self) -> Option<Ref<DockControl>>;
    /// Sets the owning source dock control.
    fn set_dock_control(&self, value: Option<Ref<DockControl>>);
    /// Gets the original control root for surface hit testing.
    fn surface_control(&self) -> Ref<Control>;
}
impl PartialEq for dyn IExternalDockSurface {
    fn eq(&self, other: &Self) -> bool {
        match (self.as_object(), other.as_object()) {
            (Some(left), Some(right)) => left.ptr_eq(&right),
            (None, None) => std::ptr::addr_eq(self as *const Self, other as *const Self),
            _ => false,
        }
    }
}
ferroui_base::ferro_markup_type!(interface dyn IExternalDockSurface as "IExternalDockSurface" {
    namespace:"FerroDock.Ferro.Contract",this:std::rc::Rc<dyn IExternalDockSurface>,
    properties:[
        DockControl:Option<Ref<DockControl>>{get:|this:&std::rc::Rc<dyn IExternalDockSurface>|this.dock_control(),set:|this:&std::rc::Rc<dyn IExternalDockSurface>,v|this.set_dock_control(v)},
        SurfaceControl:Ref<Control>{get:|this:&std::rc::Rc<dyn IExternalDockSurface>|this.surface_control()}
    ]
});
