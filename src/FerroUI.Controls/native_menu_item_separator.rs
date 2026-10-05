use crate::NativeMenuItem;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref};

/// A separator between the items of a native menu.
#[repr(C)]
pub struct NativeMenuItemSeparator {
    base: NativeMenuItem,
}

ferro_class!(NativeMenuItemSeparator: NativeMenuItem);
ferro_class_info!(NativeMenuItemSeparator { new: NativeMenuItemSeparator::new });

impl FerroObjectImpl for NativeMenuItemSeparator {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_current_value(NativeMenuItem::header_property(), Some("-".to_string()));
    }
}

impl NativeMenuItemSeparator {
    pub fn new() -> Ref<Self> {
        instantiate(Self { base: NativeMenuItem::construct() })
    }

}
