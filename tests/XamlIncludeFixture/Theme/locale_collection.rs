//! Port of `LocaleCollection` of upstream's `MarkupExtensions/ResourceIncludeTests.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::controls::{IResourceProvider, ResourceKey, ResourceProvider, ResourceProviderImpl, ResourceValue};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref};

/// A resource provider that is a keyed collection of resource providers (see issue
/// 11172 of the upstream project).
#[repr(C)]
pub struct LocaleCollection {
    base: ResourceProvider,
    langs: RefCell<Vec<(Option<BoxedValue>, Rc<dyn IResourceProvider>)>>,
}

ferro_class!(LocaleCollection: ResourceProvider);
ferro_impl_classes!(LocaleCollection: FerroObjectImpl);
ferro_class_info!(LocaleCollection {
    new: LocaleCollection::new,
    interfaces: [Rc<dyn IResourceProvider>],
    markup: {
        methods: [
            // Allows the class to be used as a collection; requires x:Key on the IResourceProvider.
            fn Add(Option<BoxedValue>, Rc<dyn IResourceProvider>) =>
                |this: &Ref<LocaleCollection>, k: Option<BoxedValue>, v: Rc<dyn IResourceProvider>| this.add(k, v),
        ],
    },
});

impl ResourceProviderImpl for LocaleCollection {
    fn has_resources(_this: &Self) -> bool {
        true
    }

    fn try_get_resource(this: &Self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        let res = this
            .langs
            .borrow()
            .iter()
            .find(|(k, _)| k.as_ref().and_then(|k| k.downcast_ref::<String>()).map(String::as_str) == Some("English"))
            .map(|(_, v)| v.clone());
        res.and_then(|res| res.try_get_resource(key, theme))
    }
}

impl LocaleCollection {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: ResourceProvider::construct(), langs: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// `Add(object k, IResourceProvider v)`.
    pub fn add(&self, k: Option<BoxedValue>, v: Rc<dyn IResourceProvider>) {
        self.langs.borrow_mut().push((k, v));
    }
}
