//! Port of `Styling/MergeResourceInclude.cs`.

use super::ResourceInclude;
use ferroui_base::controls::{IResourceProvider, IThemeVariantProvider};
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::utilities::Uri;
use std::ops::Deref;
use std::rc::Rc;

/// Loads a resource dictionary from a specified URL.
///
/// When used in runtime, this type behaves like [`ResourceInclude`]. But
/// when the XAML compiler sees it in the merged dictionaries of a resource
/// dictionary, it merges the included document into the including one at
/// compile time, which makes the lookup of its resources cheaper.
///
/// The managed class derives from [`ResourceInclude`] and adds nothing;
/// here it holds one and dereferences to it.
pub struct MergeResourceInclude {
    base: Rc<ResourceInclude>,
}

impl MergeResourceInclude {
    pub fn new(base_uri: Option<Uri>) -> Rc<Self> {
        Rc::new(Self { base: ResourceInclude::new(base_uri) })
    }

    pub fn with_service_provider(service_provider: Rc<dyn IServiceProvider>) -> Rc<Self> {
        Rc::new(Self { base: ResourceInclude::with_service_provider(service_provider) })
    }

    /// The include this one is (the base class part).
    pub fn base(&self) -> &Rc<ResourceInclude> {
        &self.base
    }
}

impl Deref for MergeResourceInclude {
    type Target = ResourceInclude;

    fn deref(&self) -> &ResourceInclude {
        &self.base
    }
}

crate::identity_eq!(MergeResourceInclude);

ferro_markup_type!(class MergeResourceInclude {
    this: Rc<MergeResourceInclude>,
    handles: [MergeResourceInclude, Rc<MergeResourceInclude>, Option<Rc<MergeResourceInclude>>],
    base: Rc<ResourceInclude>,
    interfaces: [Rc<dyn IResourceProvider>, Rc<dyn IThemeVariantProvider>],
    constructors: [
        (Option<Uri>) => MergeResourceInclude::new,
        (Rc<dyn IServiceProvider>) => MergeResourceInclude::with_service_provider,
    ],
});
