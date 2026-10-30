// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{controls::*, core::*};
use ferrodock_model_reactiveui::{controls::*, core::DockableBaseImpl, impl_dockable};
use ferroui_reactiveui::reactive_object;
use std::rc::Rc;
/// Original Tool5ViewModel, composing its source Tool base.
pub struct Tool5ViewModel {
    base: Tool,
}
reactive_object!(Tool5ViewModel, base);
impl DockableBaseImpl for Tool5ViewModel {}
impl_dockable!(@base Tool5ViewModel,Tool,fn as_document(self:Rc<Self>)->Option<Rc<dyn IDocument>>{Some(self)},fn as_mdi_document(self:Rc<Self>)->Option<Rc<dyn IMdiDocument>>{Some(self)},fn as_docking_window_state(&self)->Option<&dyn IDockingWindowState>{Some(self)},fn as_tool(self:Rc<Self>)->Option<Rc<dyn ITool>>{Some(self)});
impl ILocalTarget for Tool5ViewModel {}
impl IDocument for Tool5ViewModel {}
impl ITool for Tool5ViewModel {}
impl IMdiDocument for Tool5ViewModel {
    fn mdi_bounds(&self) -> DockRect {
        self.base.mdi_bounds()
    }
    fn set_mdi_bounds(&self, value: DockRect) {
        self.base.set_mdi_bounds(value);
    }
    fn mdi_state(&self) -> MdiWindowState {
        self.base.mdi_state()
    }
    fn set_mdi_state(&self, value: MdiWindowState) {
        self.base.set_mdi_state(value);
    }
    fn mdi_z_index(&self) -> i32 {
        self.base.mdi_z_index()
    }
    fn set_mdi_z_index(&self, value: i32) {
        self.base.set_mdi_z_index(value);
    }
}
impl IDockingWindowState for Tool5ViewModel {
    fn is_open(&self) -> bool {
        self.base.is_open()
    }
    fn set_is_open(&self, value: bool) {
        self.base.set_is_open(value);
    }
    fn is_active(&self) -> bool {
        self.base.is_active()
    }
    fn set_is_active(&self, value: bool) {
        self.base.set_is_active(value);
    }
    fn is_selected(&self) -> bool {
        self.base.is_selected()
    }
    fn set_is_selected(&self, value: bool) {
        self.base.set_is_selected(value);
    }
    fn docking_state(&self) -> DockingWindowState {
        self.base.docking_state()
    }
    fn set_docking_state(&self, value: DockingWindowState) {
        self.base.set_docking_state(value);
    }
}
impl Tool5ViewModel {
    /// Creates the source view model with the enclosing reactive identity.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            base: Tool::construct(runtime.clone()),
        })
    }
}
