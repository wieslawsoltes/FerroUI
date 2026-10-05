use super::PresentationSource;
use ferroui_base::input::raw::{IRawInputEventArgs, RawPointerEventArgs};
use ferroui_base::input::{IInputRoot, InputElement};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use std::rc::Rc;

impl PresentationSource {
    /// The source as the input root of its tree.
    pub fn input_root(&self) -> Rc<dyn IInputRoot> {
        self.rc()
    }

    /// Handles input from the platform implementation of the top-level.
    fn handle_input_core(&self, e: Rc<dyn IRawInputEventArgs>) {
        if let Some(pointer_args) = e.downcast_ref::<RawPointerEventArgs>() {
            let hit_test_element = self.root_element().input_hit_test_with(pointer_args.position(), false);
            let first_enabled_ancestor = first_enabled_ancestor(hit_test_element.clone());

            pointer_args.set_input_hit_test_result(hit_test_element, first_enabled_ancestor);
        }

        if let Some(input_manager) = &self.input_manager {
            input_manager.process_input(e);
        }
    }

    pub(super) fn handle_input(&self, e: Rc<dyn IRawInputEventArgs>) {
        if self.platform_impl().is_some() {
            let this = self.rc();
            // The dispatcher runs the callback right away (send priority);
            // it only fails when the dispatcher has shut down.
            let _ = Dispatcher::ui_thread().invoke_local(move || this.handle_input_core(e));
        } else if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
            logger.log(None, "PlatformImpl is null, couldn't handle input.");
        }
    }

    /// The element keyboard input is sent to when nothing is focused.
    ///
    /// # Panics
    /// Panics when the element has been dropped.
    pub fn focus_root(&self) -> Ref<InputElement> {
        self.focus_root.upgrade().expect("the focus root of the presentation source has been dropped")
    }
}

fn first_enabled_ancestor(hit_test_element: Option<Ref<InputElement>>) -> Option<Ref<InputElement>> {
    let mut candidate = hit_test_element;
    while let Some(element) = &candidate {
        if element.is_effectively_enabled() {
            break;
        }
        candidate = element.visual_parent().and_then(|parent| parent.downcast::<InputElement>().ok());
    }
    candidate
}
