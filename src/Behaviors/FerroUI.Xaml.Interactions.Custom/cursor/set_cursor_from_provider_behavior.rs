//! Port of `Cursor/SetCursorFromProviderBehavior.cs`.

use crate::ICursorProvider;
use ferroui_base::input::InputElement;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::rc::Rc;

/// Sets the cursor provided by an [`ICursorProvider`] when attached.
#[repr(C)]
pub struct SetCursorFromProviderBehavior {
    base: StyledElementBehaviorOfT,
}

ferro_class!(SetCursorFromProviderBehavior: StyledElementBehaviorOfT);
ferro_class_info!(SetCursorFromProviderBehavior { new: SetCursorFromProviderBehavior::new });
ferroui_base::ferro_impl_classes!(SetCursorFromProviderBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SetCursorFromProviderBehavior {
        /// Identifies the `Provider` property.
        pub fn provider_property() -> StyledProperty<Option<Rc<dyn ICursorProvider>>> {
            FerroProperty::register::<SetCursorFromProviderBehavior, _>("Provider", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for SetCursorFromProviderBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            if let (Some(associated_object), Some(provider)) =
                (this.associated_object_as::<InputElement>(), this.provider())
            {
                let cursor = provider.create_cursor();
                associated_object.set_current_value(InputElement::cursor_property(), Some(cursor));
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            if let Some(associated_object) = this.associated_object_as::<InputElement>() {
                associated_object.clear_value(InputElement::cursor_property());
            }
        }
    }
}

impl SetCursorFromProviderBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<InputElement as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the [`ICursorProvider`] that supplies the cursor.
    pub fn provider(&self) -> Option<Rc<dyn ICursorProvider>> {
        self.get_value(Self::provider_property())
    }

    pub fn set_provider(&self, value: Option<Rc<dyn ICursorProvider>>) {
        self.set_value(Self::provider_property(), value)
    }
}
