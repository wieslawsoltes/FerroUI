use crate::primitives::{Popup, TemplatedControlImpl};
use crate::{ContentControl, ContentControlImpl, ControlImpl};
use ferroui_base::input::{InputElementImpl, InputElementImplExt, Key, KeyEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};

/// The control that hosts the content of a flyout.
#[repr(C)]
pub struct FlyoutPresenter {
    base: ContentControl,
}

ferro_class!(FlyoutPresenter: ContentControl);
ferroui_base::ferro_class_info!(FlyoutPresenter { new: FlyoutPresenter::new });
ferro_impl_classes!(
    FlyoutPresenter: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl InputElementImpl for FlyoutPresenter {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        if e.key == Key::Escape {
            let mut host = None;
            let mut parent = this.parent();
            while let Some(current) = parent {
                if let Some(popup) = current.cast::<Popup>() {
                    host = Some(popup);
                    break;
                }
                parent = current.parent();
            }

            if let Some(host) = host {
                host.set_is_open(false);
                e.set_handled(true);
            }
        }

        Self::parent_on_key_down(this, e);
    }
}

impl FlyoutPresenter {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ContentControl::construct() }
    }

    /// Creates a flyout presenter.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
