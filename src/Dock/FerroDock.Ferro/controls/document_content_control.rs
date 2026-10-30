// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObject,
    FerroObjectExtensions, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    primitives::{TemplatedControl, TemplatedControlImpl},
    ControlImpl,
};

use crate::{controls::generated_theme, model_projection};
use ferroui_base::{
    reactive::{AnonymousObserver, IDisposable},
    BoxedValue,
};
use std::cell::RefCell;
use std::rc::Rc;
/// Original native DocumentContentControl, with generated themes and factory registration.
#[repr(C)]
pub struct DocumentContentControl {
    base: TemplatedControl,
    data_context_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}
ferro_class!(DocumentContentControl:TemplatedControl);
ferro_class_info!(DocumentContentControl {
    new: DocumentContentControl::new
});
ferro_impl_classes!(DocumentContentControl:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl VisualImpl for DocumentContentControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let weak = this.to_ref().downgrade();
        let subscription =
            FerroObject::get_observable(this, ferroui_base::StyledElement::data_context_property())
                .subscribe(Rc::new(AnonymousObserver::new(
                    move |value: Option<BoxedValue>| {
                        if let Some(this) = weak.upgrade() {
                            this.apply_generated_theme(value.as_ref());
                        }
                    },
                )));
        let previous = this.data_context_subscription.replace(Some(subscription));
        drop(previous);
        this.apply_generated_theme(this.data_context().as_ref());
        if let Some(dockable) = model_projection::dockable(this.data_context().as_ref()) {
            if let Some(factory) = dockable.factory() {
                let previous = factory
                    .document_controls()
                    .borrow_mut()
                    .insert(dockable, Rc::new(this.to_ref()));
                drop(previous);
            }
        }
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        let subscription = this.data_context_subscription.borrow().clone();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        let previous = this.data_context_subscription.replace(None);
        drop(previous);
        this.clear_value(ferroui_base::StyledElement::theme_property());
        if let Some(dockable) = model_projection::dockable(this.data_context().as_ref()) {
            if let Some(factory) = dockable.factory() {
                let removed = factory.document_controls().borrow_mut().remove(&dockable);
                drop(removed);
            }
        }
    }
}
impl DocumentContentControl {
    /// Constructs original source state for native inheritance.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            data_context_subscription: RefCell::new(None),
        }
    }
    /// Creates the original native content host.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn apply_generated_theme(&self, data_context: Option<&BoxedValue>) {
        generated_theme::apply(self, data_context);
    }
}
