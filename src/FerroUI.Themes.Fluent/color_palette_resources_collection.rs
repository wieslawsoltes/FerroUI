//! Port of `ColorPaletteResourcesCollection.cs`.

use crate::ColorPaletteResources;
use ferroui_base::collections::{FerroDictionary, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::controls::{
    IResourceProvider, ResourceHostRef, ResourceKey, ResourceProvider, ResourceProviderImpl, ResourceProviderImplExt,
    ResourceValue,
};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref};
use std::rc::Rc;

/// The colour palettes of a [`FluentTheme`](crate::FluentTheme) by theme
/// variant: a dictionary that is also the resource provider of the palette
/// of the requested variant.
///
/// The managed class implements `IDictionary<ThemeVariant,
/// ColorPaletteResources>` over an inner observable dictionary; here the
/// dictionary members are inherent methods.
#[repr(C)]
pub struct ColorPaletteResourcesCollection {
    base: ResourceProvider,
    inner: FerroDictionary<ThemeVariant, Ref<ColorPaletteResources>>,
}

ferro_class!(ColorPaletteResourcesCollection: ResourceProvider);
ferro_class_info!(ColorPaletteResourcesCollection {
    new: ColorPaletteResourcesCollection::new,
    markup: {
        methods: [
            try fn Add(ThemeVariant, Ref<ColorPaletteResources>) => ColorPaletteResourcesCollection::try_add,
        ],
    },
});

const ONLY_LIGHT_AND_DARK: &str = "FluentTheme.Palettes only supports Light and Dark variants.";

impl FerroObjectImpl for ColorPaletteResourcesCollection {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        // `_inner.ForEachItem(added, removed, reset)`.
        let weak = this.to_ref().downgrade();
        this.inner.add_collection_changed(Rc::new(
            move |e: &NotifyCollectionChangedEventArgs<'_, (ThemeVariant, Ref<ColorPaletteResources>)>| {
                let Some(this) = weak.upgrade() else { return };
                match e.action {
                    NotifyCollectionChangedAction::Add => this.items_added(e.new_items),
                    NotifyCollectionChangedAction::Remove => this.items_removed(e.old_items),
                    NotifyCollectionChangedAction::Replace => {
                        this.items_removed(e.old_items);
                        this.items_added(e.new_items);
                    }
                    NotifyCollectionChangedAction::Reset => panic!("Dictionary reset not supported"),
                    _ => {}
                }
            },
        ));
    }
}

impl ResourceProviderImpl for ColorPaletteResourcesCollection {
    fn has_resources(this: &Self) -> bool {
        this.inner.count() > 0
    }

    fn try_get_resource(this: &Self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        let theme = match theme {
            Some(theme) if *theme != ThemeVariant::default() => theme.clone(),
            _ => ThemeVariant::light(),
        };

        this.inner
            .try_get_value(&theme)
            .and_then(|theme_palette_resources| theme_palette_resources.try_get_resource(key, Some(&theme)))
    }

    fn on_add_owner(this: &Self, owner: &ResourceHostRef) {
        Self::parent_on_add_owner(this, owner);
        for palette in this.inner.values() {
            palette.add_owner(owner);
        }
    }

    fn on_remove_owner(this: &Self, owner: &ResourceHostRef) {
        Self::parent_on_remove_owner(this, owner);
        for palette in this.inner.values() {
            palette.remove_owner(owner);
        }
    }
}

impl ColorPaletteResourcesCollection {
    /// Field initialisation; see [`ResourceProvider::construct`].
    pub fn construct() -> Self {
        Self { base: ResourceProvider::construct(), inner: FerroDictionary::with_capacity(2) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn items_added(&self, items: &[(ThemeVariant, Ref<ColorPaletteResources>)]) {
        for (key, x) in items {
            if let Some(owner) = self.owner() {
                x.add_owner(&owner);
            }

            if *key != ThemeVariant::dark() && *key != ThemeVariant::light() {
                panic!("{ONLY_LIGHT_AND_DARK}");
            }
        }
    }

    fn items_removed(&self, items: &[(ThemeVariant, Ref<ColorPaletteResources>)]) {
        for (_, x) in items {
            if let Some(owner) = self.owner() {
                x.remove_owner(&owner);
            }
        }
    }

    /// The provider as the resource provider interface.
    pub fn as_resource_provider(&self) -> Rc<dyn IResourceProvider> {
        self.to_ref().into()
    }

    /// The number of palettes.
    pub fn count(&self) -> usize {
        self.inner.count()
    }

    /// Adds the palette of a variant.
    ///
    /// # Panics
    /// Panics if the variant already has a palette, or if it is neither the
    /// light nor the dark variant.
    pub fn add(&self, key: ThemeVariant, value: Ref<ColorPaletteResources>) {
        self.inner.add(key, value);
    }

    /// [`add`](Self::add) for untyped callers (markup): the failures are
    /// errors. A variant that is not supported is rejected before the
    /// palette is added.
    pub fn try_add(&self, key: ThemeVariant, value: Ref<ColorPaletteResources>) -> Result<(), String> {
        if key != ThemeVariant::dark() && key != ThemeVariant::light() {
            return Err(ONLY_LIGHT_AND_DARK.to_string());
        }
        if self.inner.contains_key(&key) {
            return Err(format!("An item with the same key has already been added. Key: {key}"));
        }
        self.inner.add(key, value);
        Ok(())
    }

    /// Removes all palettes.
    pub fn clear(&self) {
        self.inner.clear();
    }

    /// Whether the variant has a palette.
    pub fn contains_key(&self, key: &ThemeVariant) -> bool {
        self.inner.contains_key(key)
    }

    /// Removes the palette of a variant; returns whether there was one.
    pub fn remove(&self, key: &ThemeVariant) -> bool {
        self.inner.remove(key)
    }

    /// The palette of a variant.
    pub fn try_get_value(&self, key: &ThemeVariant) -> Option<Ref<ColorPaletteResources>> {
        self.inner.try_get_value(key)
    }

    /// The palette of a variant.
    ///
    /// # Panics
    /// Panics if the variant has no palette.
    pub fn get(&self, key: &ThemeVariant) -> Ref<ColorPaletteResources> {
        self.inner.get(key)
    }

    /// Sets the palette of a variant, replacing the one it has.
    pub fn set(&self, key: ThemeVariant, value: Ref<ColorPaletteResources>) {
        self.inner.set(key, value);
    }

    /// The variants with a palette.
    pub fn keys(&self) -> Vec<ThemeVariant> {
        self.inner.keys()
    }

    /// The palettes.
    pub fn values(&self) -> Vec<Ref<ColorPaletteResources>> {
        self.inner.values()
    }

    /// The palettes with their variants.
    pub fn to_vec(&self) -> Vec<(ThemeVariant, Ref<ColorPaletteResources>)> {
        self.inner.to_vec()
    }
}
