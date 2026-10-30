// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::model_projection;
use ferrodock_model::core::SplitViewDisplayMode;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::{
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
    ControlImpl, SplitView,
};
use std::cell::{Cell, RefCell};
/// Original dock container synchronizing native pane events with its split-view model.
#[repr(C)]
pub struct SplitViewDockControl {
    base: TemplatedControl,
    split_view: RefCell<Option<Ref<SplitView>>>,
    closed: Cell<Option<RoutedEventHandlerToken>>,
    opened: Cell<Option<RoutedEventHandlerToken>>,
    is_pane_open_changing: Cell<bool>,
}
ferro_class!(SplitViewDockControl:TemplatedControl);
ferro_class_info!(SplitViewDockControl {
    new: SplitViewDockControl::new
});
ferro_impl_classes!(SplitViewDockControl:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl TemplatedControlImpl for SplitViewDockControl {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        let old = this.split_view.borrow().clone();
        if let Some(old) = old {
            if let Some(token) = this.closed.take() {
                old.remove_handler(SplitView::pane_closed_event(), token);
            }
            if let Some(token) = this.opened.take() {
                old.remove_handler(SplitView::pane_opened_event(), token);
            }
        }
        this.split_view
            .replace(e.name_scope().find_as::<SplitView>("PART_SplitView"));
        let current = this.split_view.borrow().clone();
        if let Some(current) = current {
            let weak = this.to_ref().downgrade();
            this.closed.set(Some(current.pane_closed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.on_pane_closed();
                }
            })));
            let weak = this.to_ref().downgrade();
            this.opened.set(Some(current.pane_opened(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.on_pane_opened();
                }
            })));
        }
    }
}
struct Changing<'a>(&'a Cell<bool>);
impl Drop for Changing<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
impl SplitViewDockControl {
    /// Constructs original source state for native inheritance.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            split_view: RefCell::new(None),
            closed: Cell::new(None),
            opened: Cell::new(None),
            is_pane_open_changing: Cell::new(false),
        }
    }
    /// Creates the original native split-view dock container.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn on_pane_closed(&self) {
        if self.is_pane_open_changing.get() {
            return;
        }
        if let Some(model) = model_projection::dockable(self.data_context().as_ref())
            .and_then(|model| model.as_split_view_dock())
        {
            if !model.use_light_dismiss_overlay_mode()
                && model.is_pane_open()
                && (model.display_mode() == SplitViewDisplayMode::Overlay
                    || model.display_mode() == SplitViewDisplayMode::CompactOverlay)
            {
                self.is_pane_open_changing.set(true);
                let _changing = Changing(&self.is_pane_open_changing);
                let split = self.split_view.borrow().clone();
                if let Some(split) = split {
                    split.set_is_pane_open(true);
                }
            } else {
                model.set_is_pane_open(false);
            }
        }
    }
    fn on_pane_opened(&self) {
        if let Some(model) = model_projection::dockable(self.data_context().as_ref())
            .and_then(|model| model.as_split_view_dock())
        {
            model.set_is_pane_open(true);
        }
    }
}
