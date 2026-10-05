//! Port of `MarkupExtensions/StaticResourceExtension.cs`.

use crate::converters::ColorToBrushConverter;
use crate::object_casts::{actual_theme_variant_of, rc_of, resource_key_of, TargetProperty};
use crate::xaml_il::runtime::IFerroXamlIlParentStackProvider;
use crate::{
    EagerParentStackEnumerator, FromXamlObject, IProvideValueTarget, XamlLoadException, XamlResourceNode,
};
use ferroui_base::controls::IThemeVariantProvider;
use ferroui_base::data::core::ValueType;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::styling::{Setter, ThemeVariant};
use ferroui_base::{ferro_markup_type, BoxedValue, FerroProperty, Ref};
use ferroui_controls::Control;
use ferroui_markup::markup::data::DelayedBinding;
use std::cell::RefCell;
use std::rc::Rc;

/// `{StaticResource Key}`: looks a resource up once, when the document is
/// loaded, in the resource nodes above the place it is used.
pub struct StaticResourceExtension {
    resource_key: RefCell<Option<BoxedValue>>,
}

impl StaticResourceExtension {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { resource_key: RefCell::new(None) })
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

    /// The resource. If it is not found and the target is a control that is
    /// still being initialised, the lookup is repeated when the control is
    /// initialised and the unset value marker is returned for now.
    pub fn provide_value(&self, service_provider: &Rc<dyn IServiceProvider>) -> Result<Option<BoxedValue>, XamlLoadException> {
        Self::provide_value_for(service_provider, self.resource_key().as_ref())
    }

    fn provide_value_for(
        service_provider: &Rc<dyn IServiceProvider>,
        resource_key: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        let Some(resource_key) = resource_key else {
            return Err(XamlLoadException::with_message("StaticResourceExtension.ResourceKey must be set."));
        };
        let key = resource_key_of(resource_key);

        let stack = service_provider.get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>();
        let provide_target = service_provider.get_service_of::<Rc<dyn IProvideValueTarget>>();
        let target_object = provide_target.as_ref().and_then(|t| t.target_object());
        let target_property =
            TargetProperty::of(provide_target.as_ref().and_then(|t| t.target_property()).as_ref());

        let theme_variant = target_object
            .as_ref()
            .and_then(actual_theme_variant_of)
            .or_else(|| Self::get_dictionary_variant(stack.as_ref()));

        let mut target_type: Option<ValueType> = target_property.as_ref().map(TargetProperty::property_type);

        if let Some(setter_property) =
            target_object.as_ref().and_then(rc_of::<Setter>).and_then(|setter| setter.property())
        {
            target_type = Some(ValueType::new(setter_property.property_type(), setter_property.property_type_name()));
        }

        // Look upwards though the ambient context for IResourceNodes
        // which might be able to give us the resource.
        if let Some(stack) = &stack {
            // avoid allocations iterating the parents when possible
            match stack.clone().as_eager_parent_stack_provider() {
                Some(eager_stack) => {
                    let mut enumerator = EagerParentStackEnumerator::new(Some(eager_stack));
                    while let Some(node) = enumerator.try_get_next_of_type::<XamlResourceNode>() {
                        if let Some(value) = node.try_get_resource(&key, theme_variant.as_ref()) {
                            return Ok(ColorToBrushConverter::convert_to(value, target_type));
                        }
                    }
                }
                None => {
                    for parent in stack.parents() {
                        let Some(node) = XamlResourceNode::from_xaml_object(&parent) else { continue };
                        if let Some(value) = node.try_get_resource(&key, theme_variant.as_ref()) {
                            return Ok(ColorToBrushConverter::convert_to(value, target_type));
                        }
                    }
                }
            }
        }

        if let (Some(target), Some(property)) =
            (target_object.as_ref().and_then(<Ref<Control>>::from_xaml_object), target_property)
        {
            DelayedBinding::add_value(&target, property.into_property_info(), move |x| {
                ColorToBrushConverter::convert_to(x.find_resource(&key), target_type)
            })
            .map_err(|e| XamlLoadException::with_message(e.to_string()))?;
            return Ok(Some(FerroProperty::unset_value()));
        }

        Err(XamlLoadException::with_message(format!("Static resource '{key}' not found.")))
    }

    /// The theme variant of the nearest theme dictionary the place is
    /// declared in: the key of the nearest parent that is a theme variant
    /// provider with a key.
    pub(crate) fn get_dictionary_variant(stack: Option<&Rc<dyn IFerroXamlIlParentStackProvider>>) -> Option<ThemeVariant> {
        let stack = stack?;
        match stack.clone().as_eager_parent_stack_provider() {
            Some(eager) => {
                let mut enumerator = EagerParentStackEnumerator::new(Some(eager));

                while let Some(theme_variant_provider) =
                    enumerator.try_get_next_of_type::<Rc<dyn IThemeVariantProvider>>()
                {
                    if let Some(set_key) = theme_variant_provider.key() {
                        return Some(set_key);
                    }
                }

                None
            }
            None => stack
                .parents()
                .iter()
                .filter_map(<Rc<dyn IThemeVariantProvider>>::from_xaml_object)
                .find_map(|provider| provider.key()),
        }
    }
}

crate::identity_eq!(StaticResourceExtension);

ferro_markup_type!(class StaticResourceExtension {
    this: Rc<StaticResourceExtension>,
    handles: [StaticResourceExtension, Rc<StaticResourceExtension>, Option<Rc<StaticResourceExtension>>],
    constructors: [
        () => StaticResourceExtension::new,
        (Option<BoxedValue>) => StaticResourceExtension::with_resource_key,
    ],
    properties: [
        ResourceKey: Option<BoxedValue> {
            get: StaticResourceExtension::resource_key,
            set: StaticResourceExtension::set_resource_key
        },
    ],
    methods: [
        try fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
            |this: &Rc<StaticResourceExtension>, service_provider: Rc<dyn IServiceProvider>| {
                this.provide_value(&service_provider)
            },
    ],
});
