//! Port of `MarkupExtensions/DynamicResourceExtension.cs`.

use super::StaticResourceExtension;
use crate::data::{DynamicResourceAnchor, DynamicResourceExpression, HeldAnchor};
use crate::object_casts::resource_key_of;
use crate::xaml_il::runtime::IFerroXamlIlParentStackProvider;
use crate::{FromXamlObject, IProvideValueTarget, ServiceProviderExtensions};
use ferroui_base::controls::{IResourceProvider, ResourceHostRef, ResourceKey};
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
    /// The key the expressions of the extension look up, made from
    /// `resource_key` by the first of them: a text key is one shared string
    /// for all of them, where each made a copy of its own.
    lookup_key: RefCell<Option<ResourceKey>>,
}

impl DynamicResourceExtension {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            anchor: RefCell::new(None),
            priority: Cell::new(BindingPriority::LocalValue),
            theme_variant: RefCell::new(None),
            resource_key: RefCell::new(None),
            lookup_key: RefCell::new(None),
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
        *self.lookup_key.borrow_mut() = None;
    }

    /// The key the expressions look up, `None` without a resource key.
    fn lookup_key(&self) -> Option<ResourceKey> {
        if let Some(key) = &*self.lookup_key.borrow() {
            return Some(key.clone());
        }
        let key = resource_key_of(self.resource_key.borrow().as_ref()?);
        *self.lookup_key.borrow_mut() = Some(key.clone());
        Some(key)
    }

    /// Captures the context of the place the extension is declared in (the
    /// anchor to find resources from, the theme variant of a theme
    /// dictionary, the priority inside a control template) and returns the
    /// extension itself: it is the binding.
    pub fn provide_value(&self, service_provider: &Rc<dyn IServiceProvider>) -> Rc<DynamicResourceExtension> {
        let place = Place::of(service_provider);
        if place.in_control_template {
            self.priority.set(BindingPriority::Template);
        }
        if let Some(anchor) = place.anchor {
            *self.anchor.borrow_mut() = anchor;
        }
        *self.theme_variant.borrow_mut() = place.theme_variant;

        let this = self.this.upgrade().expect("the extension is alive while it is used");
        #[cfg(all(feature = "hot-reload", debug_assertions))]
        crate::reload_resources::record(this.clone());
        this
    }

    /// The extension as the binding contract.
    pub fn as_binding(&self) -> Rc<dyn BindingBase> {
        self.this.upgrade().expect("the extension is alive while it is used")
    }
}

/// What a dynamic resource takes from the place it is declared in, read from
/// the service provider of that place.
struct Place {
    /// The content of a control template is being built: the resource is
    /// bound at the template priority.
    in_control_template: bool,
    /// The anchor to find resources from, when the provide-value target is
    /// not an element (an element finds them itself): the nearest parent
    /// that is an element, a resource provider or a resource host, if there
    /// is one. `None` for a target that is an element.
    anchor: Option<Option<HeldAnchor>>,
    /// The theme variant of the theme dictionary the place is in.
    theme_variant: Option<ThemeVariant>,
}

impl Place {
    fn of(service_provider: &Rc<dyn IServiceProvider>) -> Self {
        let in_control_template = service_provider.is_in_control_template();

        let provide_target = service_provider.get_service_of::<Rc<dyn IProvideValueTarget>>();
        let target_object = provide_target.and_then(|t| t.target_object());

        let anchor = if target_object.as_ref().is_some_and(|t| <Ref<StyledElement>>::from_xaml_object(t).is_some()) {
            None
        } else {
            let anchor = service_provider
                .get_first_parent::<Ref<StyledElement>>()
                .map(DynamicResourceAnchor::Element)
                .or_else(|| {
                    service_provider.get_first_parent::<Rc<dyn IResourceProvider>>().map(DynamicResourceAnchor::Provider)
                })
                .or_else(|| service_provider.get_first_parent::<ResourceHostRef>().map(DynamicResourceAnchor::Host));
            Some(anchor.map(HeldAnchor::from))
        };

        let theme_variant = StaticResourceExtension::get_dictionary_variant(
            service_provider.get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>().as_ref(),
        );

        Self { in_control_template, anchor, theme_variant }
    }
}

/// `{DynamicResource key}` with a text key, bound to `property` of `target`
/// at the place `service_provider` describes: what creating the extension
/// with the key, [`provide_value`](DynamicResourceExtension::provide_value)
/// and binding the provided extension do, without the extension. The
/// extension of that sequence is reachable from nothing once its one
/// expression is made (the expression keeps the key, the anchor, the theme
/// variant and the priority, not the extension), so the expression is made
/// from the place directly.
///
/// Not used by a development build with hot reload, which records the
/// extension to move its anchor when the document is loaded again.
#[cfg(not(all(feature = "hot-reload", debug_assertions)))]
pub(crate) fn bind_dynamic_resource_at(
    service_provider: &Rc<dyn IServiceProvider>,
    target: &Ref<FerroObject>,
    property: &'static FerroProperty,
    key: &str,
) {
    struct AtPlace<'a> {
        key: &'a str,
        place: Place,
    }

    impl BindingBase for AtPlace<'_> {
        fn create_instance(
            &self,
            _target: &FerroObject,
            _target_property: Option<&'static FerroProperty>,
            _anchor: Option<&Ref<FerroObject>>,
        ) -> Rc<dyn BindingExpressionBase> {
            DynamicResourceExpression::with_held_anchor(
                ResourceKey::from(self.key),
                self.place.anchor.clone().flatten(),
                self.place.theme_variant.clone(),
                if self.place.in_control_template { BindingPriority::Template } else { BindingPriority::LocalValue },
            )
        }
    }

    let _ = target.bind_binding(property, &AtPlace { key, place: Place::of(service_provider) });
}

#[cfg(all(feature = "hot-reload", debug_assertions))]
impl crate::reload_resources::AnchorOwner for DynamicResourceExtension {
    fn anchor(&self) -> Option<HeldAnchor> {
        self.anchor.borrow().clone()
    }
    fn replace_anchor(&self, anchor: Option<HeldAnchor>) {
        *self.anchor.borrow_mut() = anchor;
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
        let Some(resource_key) = self.lookup_key() else {
            panic!("DynamicResource must have a ResourceKey.");
        };
        let _ = (target, target_property);
        DynamicResourceExpression::with_held_anchor(
            resource_key,
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
