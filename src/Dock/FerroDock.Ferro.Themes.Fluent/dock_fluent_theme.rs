// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Port of the code-behind of the original DockFluentTheme markup with native Styles resource interfaces.
use ferrodock_ferro::{controls::DocumentControl, themes::DockDensityStyle};
use ferroui_base::{
    controls::{
        IResourceNode, IResourceProvider, ResourceDictionary, ResourceHostRef, ResourceKey,
        ResourceValue, ResourcesChangedEventArgs,
    },
    ferro_class, ferro_class_info, ferro_properties, instantiate,
    metadata::{from_markup_value, IServiceProvider},
    reactive::IDisposable,
    styling::{IStyle, Selectors, Setter, Style, Styles, ThemeVariant},
    DirectProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref,
};
use ferroui_controls::{primitives::TemplatedControl, templates::IControlTemplate};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
};
/// The Dock Fluent theme, with compact density and optional cached document content.
#[repr(C)]
pub struct DockFluentTheme {
    base: Styles,
    compact_styles: OnceCell<Ref<ResourceDictionary>>,
    initial_service_provider: RefCell<Option<Rc<dyn IServiceProvider>>>,
    density_style: Cell<DockDensityStyle>,
    cache_document_tab_content: Cell<bool>,
    cached_document_template_override_style: RefCell<Option<Ref<Style>>>,
}
ferro_class!(DockFluentTheme:Styles);
ferro_class_info!(DockFluentTheme{new:DockFluentTheme::new,interfaces:[Rc<dyn IStyle> =>DockFluentTheme::style_of,Rc<dyn IResourceProvider> =>DockFluentTheme::resource_provider_of],markup:{constructors:[(Option<Rc<dyn IServiceProvider>>)=>DockFluentTheme::with_service_provider]}});
impl FerroObjectImpl for DockFluentTheme {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        Self::initialize_resources(this);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::density_style_property().as_property() {
            if let Some(owner) = this.owner() {
                owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
            }
        }
        if change.property() == Self::cache_document_tab_content_property().as_property() {
            this.update_document_template_override();
            if let Some(owner) = this.owner() {
                owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
            }
        }
    }
}
ferro_properties! {impl DockFluentTheme{
/// Identifies the density style property.
pub fn density_style_property()->DirectProperty<DockFluentTheme,DockDensityStyle>{FerroProperty::register_direct::<Self,_>("DensityStyle",|o|o.density_style(),Some(|o,v|o.set_density_style(v)),DockDensityStyle::Normal)}
/// Identifies the document content caching property.
pub fn cache_document_tab_content_property()->DirectProperty<DockFluentTheme,bool>{FerroProperty::register_direct::<Self,_>("CacheDocumentTabContent",|o|o.cache_document_tab_content(),Some(|o,v|o.set_cache_document_tab_content(v)),false)}
}}
impl DockFluentTheme {
    /// Creates fields for this class or a derived theme; instantiation loads the source markup.
    pub fn construct() -> Self {
        Self {
            base: Styles::construct(),
            compact_styles: OnceCell::new(),
            initial_service_provider: RefCell::new(None),
            density_style: Cell::new(DockDensityStyle::Normal),
            cache_document_tab_content: Cell::new(false),
            cached_document_template_override_style: RefCell::new(None),
        }
    }
    /// Loads the source theme with no parent markup service provider.
    pub fn new() -> Ref<Self> {
        Self::with_service_provider(None)
    }
    /// Loads the complete source theme and extracts its compact resources.
    pub fn with_service_provider(sp: Option<Rc<dyn IServiceProvider>>) -> Ref<Self> {
        let mut value = Self::construct();
        *value.initial_service_provider.get_mut() = sp;
        instantiate(value)
    }
    fn initialize_resources(this: &Self) {
        crate::register_types();
        let sp = this.initial_service_provider.borrow_mut().take();
        let this = this.to_ref();
        crate::compiled_xaml::populate(sp, &this).unwrap_or_else(|error| panic!("{error}"));
        let resources = this.resources();
        let key = ResourceKey::from("CompactStyles");
        let value = resources
            .try_get_value(&key)
            .flatten()
            .unwrap_or_else(|| panic!("Key 'CompactStyles' was not found in resources."));
        resources.remove(&key);
        let compact = from_markup_value::<Ref<ResourceDictionary>>(&Some(value))
            .expect("CompactStyles must be a ResourceDictionary");
        let _ = this.compact_styles.set(compact);
        this.update_document_template_override();
    }
    /// Gets the theme density.
    pub fn density_style(&self) -> DockDensityStyle {
        self.density_style.get()
    }
    /// Sets the theme density and invalidates hosted resources.
    pub fn set_density_style(&self, value: DockDensityStyle) {
        self.set_and_raise_cell(Self::density_style_property(), &self.density_style, value);
    }
    /// Gets whether document content remains alive across tab switches.
    pub fn cache_document_tab_content(&self) -> bool {
        self.cache_document_tab_content.get()
    }
    /// Sets document caching and updates the document template override.
    pub fn set_cache_document_tab_content(&self, value: bool) {
        self.set_and_raise_cell(
            Self::cache_document_tab_content_property(),
            &self.cache_document_tab_content,
            value,
        );
    }
    /// Looks up compact resources first when compact density is active.
    pub fn try_get_resource(
        &self,
        key: &ResourceKey,
        theme: Option<&ThemeVariant>,
    ) -> Option<ResourceValue> {
        if self.density_style.get() == DockDensityStyle::Compact {
            if let Some(value) = self
                .compact_styles
                .get()
                .and_then(|styles| styles.try_get_resource(key, theme))
            {
                return Some(value);
            }
        }
        let styles: &Styles = self;
        styles.try_get_resource(key, theme)
    }
    fn update_document_template_override(&self) {
        if !self.cache_document_tab_content.get() {
            let cached = self
                .cached_document_template_override_style
                .borrow()
                .clone();
            if let Some(style) = cached {
                self.remove(style);
                *self.cached_document_template_override_style.borrow_mut() = None;
            }
            return;
        }
        let styles: &Styles = self;
        let Some(value) = styles.try_get_resource(
            &ResourceKey::from("DockDocumentControlCachedContentTemplate"),
            None,
        ) else {
            return;
        };
        let Some(template) = from_markup_value::<Rc<dyn IControlTemplate>>(&value) else {
            return;
        };
        if self
            .cached_document_template_override_style
            .borrow()
            .is_none()
        {
            *self.cached_document_template_override_style.borrow_mut() =
                Some(Style::with_selector(Selectors::of_type::<DocumentControl>()));
            let style = self
                .cached_document_template_override_style
                .borrow()
                .clone()
                .expect("created style");
            style.add_setter(Setter::new(
                TemplatedControl::template_property(),
                Some(template),
            ));
            let style = self
                .cached_document_template_override_style
                .borrow()
                .clone()
                .expect("created style");
            self.add(style);
            return;
        }
        let style = self
            .cached_document_template_override_style
            .borrow()
            .clone()
            .expect("cached style");
        style.setters().clear();
        let style = self
            .cached_document_template_override_style
            .borrow()
            .clone()
            .expect("cached style");
        style.add_setter(Setter::new(
            TemplatedControl::template_property(),
            Some(template),
        ));
    }
    /// Provides this theme to a native style collection.
    pub fn as_style(&self) -> Rc<dyn IStyle> {
        Self::style_of(self.to_ref())
    }
    /// Provides this theme with its compact resource lookup semantics.
    pub fn as_resource_provider(&self) -> Rc<dyn IResourceProvider> {
        Self::resource_provider_of(self.to_ref())
    }
    fn style_of(this: Ref<Self>) -> Rc<dyn IStyle> {
        Rc::new(DockFluentThemeHandle(this))
    }
    fn resource_provider_of(this: Ref<Self>) -> Rc<dyn IResourceProvider> {
        Rc::new(DockFluentThemeHandle(this))
    }
}
// Native interface adapters retain the source theme identity and ownership.
struct DockFluentThemeHandle(Ref<DockFluentTheme>);
impl IResourceNode for DockFluentThemeHandle {
    fn reference_id(&self) -> *const () {
        let object: &FerroObject = &self.0;
        object as *const FerroObject as *const ()
    }
    fn has_resources(&self) -> bool {
        self.0.has_resources()
    }
    fn try_get_resource(
        &self,
        key: &ResourceKey,
        theme: Option<&ThemeVariant>,
    ) -> Option<ResourceValue> {
        self.0.try_get_resource(key, theme)
    }
}
impl IResourceProvider for DockFluentThemeHandle {
    fn owner(&self) -> Option<ResourceHostRef> {
        self.0.owner()
    }
    fn owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.0.owner_changed(move || handler())
    }
    fn add_owner(&self, owner: &ResourceHostRef) {
        self.0.add_owner(owner)
    }
    fn remove_owner(&self, owner: &ResourceHostRef) {
        self.0.remove_owner(owner)
    }
    fn as_object(&self) -> Option<&FerroObject> {
        Some(&self.0)
    }
}
impl IStyle for DockFluentThemeHandle {
    fn children(&self) -> Rc<Vec<Rc<dyn IStyle>>> {
        self.0.snapshot()
    }
    fn as_object(&self) -> Option<&FerroObject> {
        Some(&self.0)
    }
    fn as_resource_provider(&self) -> Option<&dyn IResourceProvider> {
        Some(self)
    }
}
