//! Port of `TransparentStyles.xaml.cs`: the class of the document
//! `TransparentStyles.xaml`.

use crate::markup::xaml_class;
use ferroui_base::styling::Styles;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};

/// The styles of a window with a transparency level other than none.
#[repr(C)]
pub struct TransparentStyles {
    base: Styles,
}

ferro_class!(TransparentStyles: Styles);
ferro_impl_classes!(TransparentStyles: FerroObjectImpl);
ferro_class_info!(TransparentStyles { new: TransparentStyles::new });
xaml_class!(TransparentStyles, "/TransparentStyles.xaml");

impl TransparentStyles {
    pub fn construct() -> Self {
        Self { base: Styles::construct() }
    }

    /// The constructor: the class declares none, so the compiler makes it
    /// populate the instance from the document.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
