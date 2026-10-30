//! Port of `SplitView/SplitViewPaneClosingTrigger.cs`.

use ferroui_base::interactivity::{CancelRoutedEventArgs, IRoutedEventArgs, Interactive, RoutedEventHandlerToken};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StaticType, StyledElementImpl,
};
use ferroui_controls::SplitView;
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::cell::Cell;
use std::rc::Rc;

/// Executes actions when the [`PaneClosing`](SplitView::pane_closing_event) event is raised.
///
/// The args of the event are the parameter of the actions.
#[repr(C)]
pub struct SplitViewPaneClosingTrigger {
    base: StyledElementTriggerOfT,
    handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(SplitViewPaneClosingTrigger: StyledElementTriggerOfT);
ferro_class_info!(SplitViewPaneClosingTrigger { new: SplitViewPaneClosingTrigger::new });
ferroui_base::ferro_impl_classes!(SplitViewPaneClosingTrigger: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for SplitViewPaneClosingTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            if let Some(associated_object) = this.associated_object_as::<SplitView>() {
                let trigger = this.to_ref().downgrade();
                let token = associated_object.pane_closing(move |_sender: &Interactive, e: &CancelRoutedEventArgs| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.on_pane_closing(e);
                    }
                });
                this.remove_handler();
                this.handler.set(Some(token));
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.remove_handler();
        }
    }
}

impl SplitViewPaneClosingTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTriggerOfT::construct(<SplitView as StaticType>::TYPE), handler: Cell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    // The handler is removed by its token (a closure has no identity to remove it by, as the
    // reference removes its method).
    fn remove_handler(&self) {
        let Some(token) = self.handler.take() else {
            return;
        };
        if let Some(associated_object) = self.associated_object_as::<SplitView>() {
            associated_object.remove_handler(SplitView::pane_closing_event(), token);
        }
    }

    fn on_pane_closing(&self, e: &CancelRoutedEventArgs) {
        // The args are the parameter of the actions (a shared handle to the live args).
        let parameter: BoxedValue = Rc::new(IRoutedEventArgs::share(e));
        self.execute(Some(&parameter));
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if !self.is_enabled() {
            return;
        }

        if let Some(associated_object) = self.associated_object() {
            let sender: BoxedValue = Rc::new(associated_object);
            Interaction::execute_actions(Some(&sender), self.actions().as_ref(), parameter);
        }
    }
}
