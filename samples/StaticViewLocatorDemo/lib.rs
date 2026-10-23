//! static-view-locator-demo
//!
//! The sample of `ferroui-static-view-locator`: a port of `StaticViewLocatorDemo` and
//! `StaticViewLocatorReactiveUIDemo` of StaticViewLocator
//! (<https://github.com/wieslawsoltes/StaticViewLocator>). The views of the view models are found
//! by locators that the build script of the crate generates (`build.rs`): no view is looked up by
//! the name of a type at run time.
//!
//! - The pages of the window (dashboard, test, settings, reports) are view models; the tab control
//!   shows each through [`ViewLocator`], the generated data template of the window. The dashboard
//!   and the reports show their metrics and reports through the same locator.
//! - The page of ReactiveUI shows one navigation twice: through [`ReactiveViewLocator`] as the
//!   data template of a content control, and as the view locator of a `ViewModelViewHost`. Its
//!   views implement `IControlViewFor` (ReactiveUI's `IViewFor<TViewModel>`), which the generator
//!   discovers; a view model wrapped in a [`ContextHost`](view_models::reactive::ContextHost) is
//!   shown by the fallback hook of the locator.
//!
//! The view models are one module per type (`view_models::dashboard_view_model`) and so are the
//! views (`views::dashboard_view`): the default rules of the generator map the one to the other.

/// What the macro of the views names, imported by every view module.
pub(crate) mod class_prelude {
    pub use ferroui_base::input::InputElementImpl;
    pub use ferroui_base::interactivity::InteractiveImpl;
    pub use ferroui_base::layout::LayoutableImpl;
    pub use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl};
    pub use ferroui_controls::primitives::TemplatedControlImpl;
    pub use ferroui_controls::{ContentControlImpl, Control, ControlImpl, UserControl};
    pub use ferroui_view::Owner;
    pub use std::cell::RefCell;
}

/// A view of the sample: a class of the object model derived from `UserControl`, created by
/// `new()`, whose content `$content` builds from the view model of its data context, a
/// `$view_model`, again each time the data context changes.
macro_rules! located_view {
    ($name:ident, $view_model:ty, $content:ident) => {
        #[repr(C)]
        pub struct $name {
            base: UserControl,
            /// The owner of the reactive values of the content.
            owner: RefCell<Option<Owner>>,
        }

        ferro_class!($name: UserControl);
        ferro_impl_classes!(
            $name: FerroObjectImpl,
            StyledElementImpl,
            VisualImpl,
            LayoutableImpl,
            InteractiveImpl,
            InputElementImpl,
            ControlImpl,
            TemplatedControlImpl,
            ContentControlImpl
        );
        ferro_class_info!($name { new: $name::new });

        impl $name {
            pub fn construct() -> Self {
                Self { base: UserControl::construct(), owner: RefCell::new(None) }
            }

            pub fn new() -> Ref<Self> {
                let this = instantiate(Self::construct());
                let weak = this.downgrade();
                let _subscription = this.data_context_changed(move || {
                    if let Some(this) = weak.upgrade() {
                        this.show_view_model();
                    }
                });
                this
            }

            /// The view model of the data context.
            pub fn view_model(&self) -> Option<std::rc::Rc<$view_model>> {
                $crate::views::view_model_of::<$view_model>(self.data_context())
            }

            fn show_view_model(&self) {
                if let Some(owner) = self.owner.borrow_mut().take() {
                    owner.dispose();
                }
                let content = self.view_model().map(|view_model| {
                    let (control, owner) = $content(&view_model).mount_root();
                    *self.owner.borrow_mut() = Some(owner);
                    Control::boxed(&control)
                });
                self.set_content(content);
            }
        }
    };
}

pub(crate) use located_view;

pub mod main_window;
pub mod reactive_view_locator;
pub mod view_locator;
pub mod view_models;
pub mod views;

pub use main_window::main_window;
pub use reactive_view_locator::ReactiveViewLocator;
pub use view_locator::ViewLocator;

#[cfg(test)]
mod tests;
