//! Port of `FileUpload/ButtonUploadFileBehavior.cs`.

use crate::UploadFileBehaviorBase;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObject, FerroObjectImpl, Ref, StyledElementImpl,
    WeakRef,
};
use ferroui_controls::Button;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt};
use std::cell::RefCell;
use std::rc::Rc;

/// Upload file behavior for [`Button`]: a click uploads the file.
#[repr(C)]
pub struct ButtonUploadFileBehavior {
    base: UploadFileBehaviorBase,
    // The button the click handler is attached to (`button.Click += OnClick`).
    click: RefCell<Option<(WeakRef<Button>, RoutedEventHandlerToken)>>,
}

ferro_class!(ButtonUploadFileBehavior: UploadFileBehaviorBase);
ferro_class_info!(ButtonUploadFileBehavior { new: ButtonUploadFileBehavior::new });
ferroui_base::ferro_impl_classes!(ButtonUploadFileBehavior: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ButtonUploadFileBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            if let Some(button) = this.associated_object_as::<Button>() {
                this.remove_click_handler();
                let behavior = this.to_ref().downgrade();
                let token = button.click(move |sender: &Interactive, e: &RoutedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_click(sender, e);
                    }
                });
                *this.click.borrow_mut() = Some((button.downgrade(), token));
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);

            this.remove_click_handler();
        }
    }
}

impl ButtonUploadFileBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: UploadFileBehaviorBase::construct(), click: RefCell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn remove_click_handler(&self) {
        let click = self.click.borrow_mut().take();
        if let Some((button, token)) = click {
            if let Some(button) = button.upgrade() {
                button.remove_handler(Button::click_event(), token);
            }
        }
    }

    fn on_click(&self, sender: &Interactive, e: &RoutedEventArgs) {
        let sender: Ref<FerroObject> = sender.to_ref().upcast();
        let sender: BoxedValue = Rc::new(sender);
        let parameter: BoxedValue = Rc::new(IRoutedEventArgs::share(e));
        self.execute(Some(&sender), Some(&parameter));
    }
}
