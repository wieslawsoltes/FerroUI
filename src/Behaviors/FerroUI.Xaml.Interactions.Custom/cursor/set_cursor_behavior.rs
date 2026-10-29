//! Port of `Cursor/SetCursorBehavior.cs`.

use ferroui_base::input::{Cursor, InputElement};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::rc::Rc;

/// Sets the cursor for the associated control when attached.
#[repr(C)]
pub struct SetCursorBehavior {
    base: StyledElementBehaviorOfT,
}

ferro_class!(SetCursorBehavior: StyledElementBehaviorOfT);
ferro_class_info!(SetCursorBehavior { new: SetCursorBehavior::new });
ferroui_base::ferro_impl_classes!(SetCursorBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SetCursorBehavior {
        /// Identifies the `Cursor` property.
        pub fn cursor_property() -> StyledProperty<Option<Rc<Cursor>>> {
            FerroProperty::register::<SetCursorBehavior, _>("Cursor", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for SetCursorBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            if let (Some(associated_object), Some(cursor)) = (this.associated_object_as::<InputElement>(), this.cursor()) {
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

impl SetCursorBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<InputElement as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the cursor to apply.
    pub fn cursor(&self) -> Option<Rc<Cursor>> {
        self.get_value(Self::cursor_property())
    }

    pub fn set_cursor(&self, value: Option<Rc<Cursor>>) {
        self.set_value(Self::cursor_property(), value)
    }
}
