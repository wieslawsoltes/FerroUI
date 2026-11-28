//! The class of the document `CollisionStyles.xaml`, which includes two public documents whose
//! established build functions are one name.

use ferroui_base::styling::Styles;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};

/// A styles collection populated from its document.
#[repr(C)]
pub struct CollisionStyles {
    base: Styles,
}

ferro_class!(CollisionStyles: Styles);
ferro_impl_classes!(CollisionStyles: FerroObjectImpl);
ferro_class_info!(CollisionStyles { new: CollisionStyles::new });

impl CollisionStyles {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: Styles::construct() }
    }

    /// Creates the styles from the document of the class.
    ///
    /// # Panics
    /// Panics if the document fails to load.
    pub fn new() -> Ref<Self> {
        crate::register_types();
        let this = instantiate(Self::construct());
        if let Err(error) = crate::compiled_collision_styles::populate(None, &this) {
            panic!("{error}");
        }
        this
    }
}
