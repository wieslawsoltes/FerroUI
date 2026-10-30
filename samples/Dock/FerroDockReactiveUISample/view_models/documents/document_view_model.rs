// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{controls::*, core::*};
use ferrodock_model_reactiveui::{controls::*, core::DockableBaseImpl, impl_dockable};
use ferroui_reactiveui::reactive_object;
use std::rc::Rc;
/// Original DocumentViewModel, composing its source Document base.
pub struct DocumentViewModel {
    base: Document,
}
reactive_object!(DocumentViewModel, base);
impl DockableBaseImpl for DocumentViewModel {}
impl_dockable!(@base DocumentViewModel,Document,fn as_document(self:Rc<Self>)->Option<Rc<dyn IDocument>>{Some(self)},fn as_mdi_document(self:Rc<Self>)->Option<Rc<dyn IMdiDocument>>{Some(self)},fn as_docking_window_state(&self)->Option<&dyn IDockingWindowState>{Some(self)});
impl ILocalTarget for DocumentViewModel {}
impl IDocument for DocumentViewModel {}
impl IMdiDocument for DocumentViewModel {
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
impl IDockingWindowState for DocumentViewModel {
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
impl DocumentViewModel {
    /// Creates the source view model with the enclosing reactive identity.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            base: Document::construct(runtime.clone()),
        })
    }
}
