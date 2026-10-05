use crate::NativeMenu;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, DirectProperty, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, WeakRef,
};
use std::cell::RefCell;

/// The base class of the items of a [`NativeMenu`].
#[repr(C)]
pub struct NativeMenuItemBase {
    base: FerroObject,
    /// The menu that holds the item. A back pointer: the menu owns its
    /// items.
    parent: RefCell<Option<WeakRef<NativeMenu>>>,
}

ferro_class!(NativeMenuItemBase: FerroObject);
ferro_class_info!(NativeMenuItemBase {});
ferro_impl_classes!(NativeMenuItemBase: FerroObjectImpl);

ferro_properties! {
    impl NativeMenuItemBase {
        /// Defines the `Parent` property.
        pub fn parent_property() -> DirectProperty<NativeMenuItemBase, Option<Ref<NativeMenu>>> {
            FerroProperty::register_direct::<NativeMenuItemBase, _>("Parent", |o| o.parent(), None, None)
        }
    }
}

impl NativeMenuItemBase {
    /// Creates the class data; see [`FerroObject::construct`]. For the
    /// classes of this crate that derive from this one.
    pub(crate) fn construct() -> Self {
        Self { base: FerroObject::construct(), parent: RefCell::new(None) }
    }

    /// The menu that holds the item.
    pub fn parent(&self) -> Option<Ref<NativeMenu>> {
        let parent = self.parent.borrow().clone();
        parent.and_then(|parent| parent.upgrade())
    }

    pub(crate) fn set_parent(&self, value: Option<Ref<NativeMenu>>) {
        let old = self.parent();
        if old == value {
            return;
        }
        *self.parent.borrow_mut() = value.as_ref().map(|value| value.downgrade());
        self.raise_direct_property_changed(Self::parent_property(), &old, &value);
    }
}
