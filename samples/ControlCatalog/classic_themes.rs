//! The class of the document `ClassicThemes.xaml`, with the feature `classic-themes` only (not in
//! the original, whose `App.xaml` declares the two themes itself).

use crate::markup::xaml_class;
use ferroui_base::controls::{ResourceDictionary, ResourceProviderImpl};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref};

/// The Simple and Fluent themes of the original with the Simple styles of the colour picker and
/// of the TreeDataGrid: the resources `App.xaml` of the original declares for them, which the
/// application merges into its resources when the catalog is built with the feature
/// `classic-themes` (docs/binary-size.md).
#[repr(C)]
pub struct ClassicThemes {
    base: ResourceDictionary,
}

ferro_class!(ClassicThemes: ResourceDictionary);
ferro_class_info!(ClassicThemes { new: ClassicThemes::new });
xaml_class!(ClassicThemes, "/ClassicThemes.xaml");

impl FerroObjectImpl for ClassicThemes {}

impl ResourceProviderImpl for ClassicThemes {}

impl ClassicThemes {
    pub fn construct() -> Self {
        Self { base: ResourceDictionary::construct() }
    }

    /// The constructor: the class declares none, so the compiler makes it populate the
    /// instance from the document.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
