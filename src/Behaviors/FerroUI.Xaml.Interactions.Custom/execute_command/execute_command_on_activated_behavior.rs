//! Port of `ExecuteCommand/ExecuteCommandOnActivatedBehavior.cs`.

use crate::{AttachedToVisualTreeBehaviorImpl, ExecuteCommandBehaviorBase, ExecuteCommandBehaviorBaseImpl};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl, WeakRef};
use ferroui_controls::{Application, Window};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// A behavior that executes its command when the main window of the application (or the
/// [`source_control`](ExecuteCommandBehaviorBase::source_control) when it is a window) is
/// activated, in an application with a classic desktop lifetime.
#[repr(C)]
pub struct ExecuteCommandOnActivatedBehavior {
    base: ExecuteCommandBehaviorBase,
}

ferro_class!(ExecuteCommandOnActivatedBehavior: ExecuteCommandBehaviorBase);
ferro_class_info!(ExecuteCommandOnActivatedBehavior { new: ExecuteCommandOnActivatedBehavior::new });
ferroui_base::ferro_impl_classes!(
    ExecuteCommandOnActivatedBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
    ExecuteCommandBehaviorBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ExecuteCommandOnActivatedBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let lifetime = Application::current().and_then(|application| application.application_lifetime());
            let main_window = lifetime.as_ref().and_then(|lifetime| {
                lifetime.as_classic_desktop_style_application_lifetime().map(|lifetime| {
                    this.source_control()
                        .and_then(|source_control| source_control.cast::<Window>())
                        .or_else(|| lifetime.main_window())
                })
            });

            if let Some(Some(main_window)) = main_window {
                let behavior: WeakRef<ExecuteCommandOnActivatedBehavior> = this.to_ref().downgrade();
                return main_window.activated(move || {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.window_on_activated();
                    }
                });
            }

            DisposableAction::empty()
        }
    }
}

impl ExecuteCommandOnActivatedBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ExecuteCommandBehaviorBase::construct() }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn window_on_activated(&self) {
        self.execute_command();
    }
}
