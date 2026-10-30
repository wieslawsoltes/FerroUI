// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::automation::peers::RootDockControlAutomationPeer;
use crate::model_projection;
use ferroui_base::input::{InputElement, InputElementImpl};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use ferroui_controls::{ContentControl, ControlImpl};
use std::cell::{Cell, RefCell};
/// The source root-dock control, with its original main-content template part.
#[repr(C)]
pub struct RootDockControl {
    base: TemplatedControl,
    main_content: RefCell<Option<Ref<ContentControl>>>,
    pressed: Cell<Option<RoutedEventHandlerToken>>,
}
ferro_class!(RootDockControl:TemplatedControl);
ferro_class_info!(RootDockControl {new:RootDockControl::new,markup:{attributes:[TemplatePart("PART_MainContent",type(Ref<ContentControl>))]}});
ferro_impl_classes!(RootDockControl:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl);
impl ControlImpl for RootDockControl {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        RootDockControlAutomationPeer::new(this).upcast()
    }
}
impl TemplatedControlImpl for RootDockControl {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.remove_handler_from_main_content();
        this.main_content
            .replace(e.name_scope().find_as::<ContentControl>("PART_MainContent"));
        let main = this.main_content.borrow().clone();
        if let Some(main) = main {
            let weak = this.to_ref().downgrade();
            let token = main.add_handler_with(
                InputElement::pointer_pressed_event(),
                move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        if let Some(root) = model_projection::root(this.data_context().as_ref()) {
                            if let Some(factory) = root.factory() {
                                factory.hide_previewing_dockables(Some(root));
                            }
                        }
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            this.pressed.set(Some(token));
        }
    }
}
impl VisualImpl for RootDockControl {
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.remove_handler_from_main_content();
        this.main_content.replace(None);
    }
}
impl RootDockControl {
    /// Constructs the native source class state for this or a derived control.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            main_content: RefCell::new(None),
            pressed: Cell::new(None),
        }
    }
    /// Creates the source root-dock control.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn remove_handler_from_main_content(&self) {
        let old = self.main_content.borrow().clone();
        if let Some(old) = old {
            if let Some(token) = self.pressed.take() {
                old.remove_handler(InputElement::pointer_pressed_event(), token);
            }
        }
    }
}
