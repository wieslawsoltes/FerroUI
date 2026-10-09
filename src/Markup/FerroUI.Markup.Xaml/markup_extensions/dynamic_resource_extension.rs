//! Port of `MarkupExtensions/DynamicResourceExtension.cs`.

use super::StaticResourceExtension;
use crate::data::{DynamicResourceAnchor, DynamicResourceExpression, HeldAnchor};
use crate::object_casts::resource_key_of;
use crate::xaml_il::runtime::IFerroXamlIlParentStackProvider;
use crate::{FromXamlObject, IProvideValueTarget, ServiceProviderExtensions};
use ferroui_base::controls::{IResourceProvider, ResourceHostRef};
use ferroui_base::data::{BindingBase, BindingExpressionBase, BindingPriority};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroObject, FerroProperty, Ref, StyledElement};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// `{DynamicResource Key}`: a binding to a resource, which follows changes
/// of the resources and of the theme variant.
pub struct DynamicResourceExtension {
    this: Weak<DynamicResourceExtension>,
    // Deviation (DEVIATIONS.md, Markup metadata and markup events): held as
    // its expressions hold it, an element or a host weakly, where the managed
    // extension holds `_anchor`.
    anchor: RefCell<Option<HeldAnchor>>,
    priority: Cell<BindingPriority>,
    theme_variant: RefCell<Option<ThemeVariant>>,
    resource_key: RefCell<Option<BoxedValue>>,
}

impl DynamicResourceExtension {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            anchor: RefCell::new(None),
            priority: Cell::new(BindingPriority::LocalValue),
            theme_variant: RefCell::new(None),
            resource_key: RefCell::new(None),
        })
    }

    /// Creates the extension with a resource key.
    pub fn with_resource_key(resource_key: Option<BoxedValue>) -> Rc<Self> {
        let this = Self::new();
        this.set_resource_key(resource_key);
        this
    }

    /// The key of the resource: text, a type or any other value.
    pub fn resource_key(&self) -> Option<BoxedValue> {
        self.resource_key.borrow().clone()
    }

    pub fn set_resource_key(&self, value: Option<BoxedValue>) {
        *self.resource_key.borrow_mut() = value;
    }

    /// Captures the context of the place the extension is declared in (the
    /// anchor to find resources from, the theme variant of a theme
    /// dictionary, the priority inside a control template) and returns the
    /// extension itself: it is the binding.
    pub fn provide_value(&self, service_provider: &Rc<dyn IServiceProvider>) -> Rc<DynamicResourceExtension> {
        if service_provider.is_in_control_template() {
            self.priority.set(BindingPriority::Template);
        }

        let provide_target = service_provider.get_service_of::<Rc<dyn IProvideValueTarget>>();
        let target_object = provide_target.and_then(|t| t.target_object());

        if !target_object.as_ref().is_some_and(|t| <Ref<StyledElement>>::from_xaml_object(t).is_some()) {
            let anchor = service_provider
                .get_first_parent::<Ref<StyledElement>>()
                .map(DynamicResourceAnchor::Element)
                .or_else(|| {
                    service_provider.get_first_parent::<Rc<dyn IResourceProvider>>().map(DynamicResourceAnchor::Provider)
                })
                .or_else(|| service_provider.get_first_parent::<ResourceHostRef>().map(DynamicResourceAnchor::Host));
            *self.anchor.borrow_mut() = anchor.map(HeldAnchor::from);
        }

        *self.theme_variant.borrow_mut() = StaticResourceExtension::get_dictionary_variant(
            service_provider.get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>().as_ref(),
        );

        self.this.upgrade().expect("the extension is alive while it is used")
    }

    /// The extension as the binding contract.
    pub fn as_binding(&self) -> Rc<dyn BindingBase> {
        self.this.upgrade().expect("the extension is alive while it is used")
    }
}

impl BindingBase for DynamicResourceExtension {
    /// # Panics
    /// Panics if the resource key is not set.
    fn create_instance(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        _anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        let Some(resource_key) = self.resource_key() else {
            panic!("DynamicResource must have a ResourceKey.");
        };
        let _ = (target, target_property);
        DynamicResourceExpression::with_held_anchor(
            resource_key_of(&resource_key),
            self.anchor.borrow().clone(),
            self.theme_variant.borrow().clone(),
            self.priority.get(),
        )
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

crate::identity_eq!(DynamicResourceExtension);

ferro_markup_type!(class DynamicResourceExtension {
    this: Rc<DynamicResourceExtension>,
    handles: [Rc<DynamicResourceExtension>, Option<Rc<DynamicResourceExtension>>],
    base: Rc<dyn BindingBase>,
    constructors: [
        () => DynamicResourceExtension::new,
        (Option<BoxedValue>) => DynamicResourceExtension::with_resource_key,
    ],
    properties: [
        ResourceKey: Option<BoxedValue> {
            get: DynamicResourceExtension::resource_key,
            set: DynamicResourceExtension::set_resource_key
        } [ConstructorArgument("resourceKey")],
    ],
    methods: [
        fn ProvideValue(Rc<dyn IServiceProvider>) -> Rc<DynamicResourceExtension> =>
            |this: &Rc<DynamicResourceExtension>, service_provider: Rc<dyn IServiceProvider>| {
                this.provide_value(&service_provider)
            },
    ],
});
