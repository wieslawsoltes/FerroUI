// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! UI-private dispatch of original concrete managed-host members through IHostWindow.
use super::event_type::EventType;
use ferrodock_model::core::{DockRect, IHostWindow};
use ferroui_base::PixelPoint;
use std::{any::TypeId, rc::Rc};
// Only the managed host and in-crate test probes expose this private cast target.
// This preserves the original concrete-type test without coupling domain to UI types.
pub(crate) trait ManagedHostWindowAccess {
    fn managed_z_index(&self) -> i32;
    fn process_drag(&self, point: PixelPoint, phase: EventType);
    fn last_close_canceled(&self) -> bool;
    fn update_bounds_from_document(&self, bounds: DockRect);
}
pub(crate) fn managed_host(host: &Rc<dyn IHostWindow>) -> Option<Rc<dyn ManagedHostWindowAccess>> {
    host.clone()
        .boxed_interface(TypeId::of::<Rc<dyn ManagedHostWindowAccess>>())?
        .downcast_ref::<Rc<dyn ManagedHostWindowAccess>>()
        .cloned()
}
