// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{IOverlayLayer, IOverlayLayerFactory, OverlayLayerCollection};
use ferroui_base::metadata::IServiceProvider;
use ferroui_system::events::Event;
use std::cell::RefCell;
use std::rc::Rc;
/// A fresh layer enumerator from the source IEnumerable contract.
pub type OverlayLayerEnumerator = Box<dyn Iterator<Item = Option<Rc<dyn IOverlayLayer>>>>;
/// A repeatable source layer enumeration factory.
pub type OverlayLayerEnumeration = dyn Fn() -> OverlayLayerEnumerator;
/// A fresh factory enumerator from the source IEnumerable contract.
pub type OverlayFactoryEnumerator = Box<dyn Iterator<Item = Option<Rc<dyn IOverlayLayerFactory>>>>;
/// A repeatable source factory enumeration factory.
pub type OverlayFactoryEnumeration = dyn Fn() -> OverlayFactoryEnumerator;
/// A nullable, repeatably enumerable layer sequence or the original observable collection.
#[derive(Clone)]
pub enum OverlayLayerSource {
    /// Preserves a provider's actual collection identity and live notifications.
    Collection(OverlayLayerCollection),
    /// Invokes the original enumeration factory for each enumeration.
    Enumerable(Rc<OverlayLayerEnumeration>),
}
impl OverlayLayerSource {
    /// Gets a fresh source enumerator.
    pub fn iter(&self) -> OverlayLayerEnumerator {
        match self {
            Self::Collection(list) => {
                let list = list.clone();
                let storage = list.snapshot();
                let mut index = 0;
                Box::new(std::iter::from_fn(move || {
                    assert!(
                        Rc::ptr_eq(&storage, &list.snapshot()),
                        "source list changed during enumeration"
                    );
                    if index >= storage.len() {
                        None
                    } else {
                        let value = storage[index].clone();
                        index += 1;
                        Some(value)
                    }
                }))
            }
            Self::Enumerable(source) => source(),
        }
    }
    /// Creates a repeatable sequence retaining these entries.
    pub fn from_items(items: Vec<Option<Rc<dyn IOverlayLayer>>>) -> Self {
        let items = Rc::new(items);
        Self::Enumerable(Rc::new(move || {
            Box::new(items.as_ref().clone().into_iter())
        }))
    }
}
/// A repeatably enumerable sequence of optional factories.
#[derive(Clone)]
pub struct OverlayFactorySource(pub Rc<OverlayFactoryEnumeration>);
impl OverlayFactorySource {
    /// Creates a repeatable sequence retaining the factory entries.
    pub fn from_items(items: Vec<Option<Rc<dyn IOverlayLayerFactory>>>) -> Self {
        let items = Rc::new(items);
        Self(Rc::new(move || {
            Box::new(items.as_ref().clone().into_iter())
        }))
    }
}
/// Source layer-provider delegate.
pub type OverlayLayerProvider = dyn Fn() -> Option<OverlayLayerSource>;
/// Source factory-provider delegate.
pub type OverlayFactoryProvider = dyn Fn() -> Option<OverlayFactorySource>;
#[derive(Default)]
struct Registry {
    provider: RefCell<Option<Rc<OverlayLayerProvider>>>,
    factory: RefCell<Option<Rc<OverlayFactoryProvider>>>,
    changed: Rc<Event<()>>,
}
ferroui_base::ferro_thread_local! {static REGISTRY:Registry=Registry::default();}
/// Source static provider API on the shared framework's thread-affine runtime.
pub struct OverlayLayerRegistry;
impl OverlayLayerRegistry {
    /// Gets the current layer provider.
    pub fn provider() -> Option<Rc<OverlayLayerProvider>> {
        REGISTRY.with(|r| r.provider.borrow().clone())
    }
    /// Changes the provider and raises ProviderChanged only on a changed delegate.
    pub fn set_provider(value: Option<Rc<OverlayLayerProvider>>) {
        Self::update(value, Self::factory_provider());
    }
    /// Gets the current factory provider.
    pub fn factory_provider() -> Option<Rc<OverlayFactoryProvider>> {
        REGISTRY.with(|r| r.factory.borrow().clone())
    }
    /// Changes the factory provider.
    pub fn set_factory_provider(value: Option<Rc<OverlayFactoryProvider>>) {
        Self::update(Self::provider(), value);
    }
    /// Gets the shared event handle for source provider changes.
    pub fn provider_changed() -> Rc<Event<()>> {
        REGISTRY.with(|r| r.changed.clone())
    }
    /// Configures both providers from the existing shared service-provider contract.
    pub fn use_service_provider(provider: Option<Rc<dyn IServiceProvider>>) {
        let Some(provider) = provider else {
            Self::update(None, None);
            return;
        };
        let layers = provider.clone();
        Self::update(
            Some(Rc::new(move || {
                Some(
                    layers
                        .get_service_of::<OverlayLayerSource>()
                        .unwrap_or_else(|| OverlayLayerSource::from_items(Vec::new())),
                )
            })),
            Some(Rc::new(move || {
                Some(
                    provider
                        .get_service_of::<OverlayFactorySource>()
                        .unwrap_or_else(|| OverlayFactorySource::from_items(Vec::new())),
                )
            })),
        );
    }
    fn update(
        provider: Option<Rc<OverlayLayerProvider>>,
        factory: Option<Rc<OverlayFactoryProvider>>,
    ) {
        fn same<T: ?Sized>(a: &Option<Rc<T>>, b: &Option<Rc<T>>) -> bool {
            match (a, b) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
        }
        let changed = REGISTRY.with(|r| {
            if same(&r.provider.borrow(), &provider) && same(&r.factory.borrow(), &factory) {
                return None;
            }
            r.provider.replace(provider);
            r.factory.replace(factory);
            Some(r.changed.clone())
        });
        if let Some(changed) = changed {
            changed.raise(&());
        }
    }
}
