//! Type tests on untyped objects: the `is`/`as` casts the managed original
//! performs on the objects of a parent stack, on provide-value targets and
//! on root objects.
//!
//! Not a port of an upstream file. An untyped object ([`BoxedValue`]) holds
//! exactly one Rust type, so "is this parent an `IResourceNode`" needs to
//! know how each kind of object exposes the contract: classes of the object
//! model through their handle conversions, the shared plain types of this
//! crate directly.

use crate::styling::{MergeResourceInclude, ResourceInclude, StyleInclude};
use ferroui_base::controls::{
    IResourceHost, IResourceNode, IResourceProvider, IThemeVariantProvider, ResourceDictionary, ResourceHostRef,
    ResourceKey, ResourceProvider, ResourceValue,
};
use ferroui_base::data::core::{IPropertyInfo, ValueType, ValueTypes};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::{IStyle, StyleBase, Styles, ThemeVariant};
use ferroui_base::{
    AnyValue, BoxedValue, FerroObject, FerroProperty, ObjectType, Ref, StyledElement, TypeInfo,
};
use ferroui_controls::Application;
use std::any::Any;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// A type an untyped object can be tested for and viewed as: the `T` of the
/// managed `obj as T`.
///
/// Implemented for the handles of classes (`Ref<T>`, which also accepts
/// handles of derived classes), for shared plain types (`Rc<T>`) and for
/// the contracts the markup runtime looks for.
pub trait FromXamlObject: Sized {
    /// The object as a `Self`, if it is one.
    fn from_xaml_object(value: &BoxedValue) -> Option<Self>;
}

impl<T: ObjectType> FromXamlObject for Ref<T> {
    fn from_xaml_object(value: &BoxedValue) -> Option<Self> {
        as_object(value)?.cast::<T>()
    }
}

impl<T: 'static> FromXamlObject for Rc<T> {
    fn from_xaml_object(value: &BoxedValue) -> Option<Self> {
        rc_of::<T>(value)
    }
}

/// The class instance an untyped value holds, if it holds one.
pub(crate) fn as_object(value: &BoxedValue) -> Option<Ref<FerroObject>> {
    let any: &dyn AnyValue = &**value;
    ValueTypes::as_object(any)
}

/// The shared plain object an untyped value holds: the object itself (the
/// canonical untyped form), its handle or its nullable handle.
pub(crate) fn rc_of<T: 'static>(value: &BoxedValue) -> Option<Rc<T>> {
    if let Some(handle) = value.downcast_ref::<Rc<T>>() {
        return Some(handle.clone());
    }
    if let Some(handle) = value.downcast_ref::<Option<Rc<T>>>() {
        return handle.clone();
    }
    if !value.is::<T>() {
        return None;
    }
    let any: Rc<dyn Any> = value.clone();
    any.downcast::<T>().ok()
}

/// The resource include an untyped value holds: an include, or the include
/// a merging include is.
fn resource_include_of(value: &BoxedValue) -> Option<Rc<ResourceInclude>> {
    rc_of::<ResourceInclude>(value).or_else(|| rc_of::<MergeResourceInclude>(value).map(|merge| merge.base().clone()))
}

/// A resource node found among untyped objects.
///
/// Elements are kept as they are, so that walking a parent stack does not
/// allocate an adapter for each of them.
#[derive(Clone)]
pub enum XamlResourceNode {
    Element(Ref<StyledElement>),
    Node(Rc<dyn IResourceNode>),
}

impl XamlResourceNode {
    /// `IResourceNode.HasResources`.
    pub fn has_resources(&self) -> bool {
        match self {
            XamlResourceNode::Element(element) => IResourceNode::has_resources(&**element),
            XamlResourceNode::Node(node) => node.has_resources(),
        }
    }

    /// `IResourceNode.TryGetResource`.
    pub fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        match self {
            XamlResourceNode::Element(element) => IResourceNode::try_get_resource(&**element, key, theme),
            XamlResourceNode::Node(node) => node.try_get_resource(key, theme),
        }
    }
}

fn style_of_object(object: &Ref<FerroObject>) -> Option<Rc<dyn IStyle>> {
    if let Some(style) = object.cast::<StyleBase>() {
        return Some(style.into());
    }
    object.cast::<Styles>().map(Into::into)
}

impl FromXamlObject for XamlResourceNode {
    fn from_xaml_object(value: &BoxedValue) -> Option<Self> {
        if let Some(object) = as_object(value) {
            if let Some(element) = object.cast::<StyledElement>() {
                return Some(XamlResourceNode::Element(element));
            }
            if let Some(application) = object.cast::<Application>() {
                return Some(XamlResourceNode::Node(application.as_resource_host()));
            }
            if let Some(provider) = object.cast::<ResourceProvider>() {
                let provider: Rc<dyn IResourceProvider> = provider.into();
                return Some(XamlResourceNode::Node(provider));
            }
            return style_of_object(&object).map(|style| XamlResourceNode::Node(style));
        }
        if let Some(include) = resource_include_of(value) {
            return Some(XamlResourceNode::Node(include));
        }
        if let Some(include) = rc_of::<StyleInclude>(value) {
            return Some(XamlResourceNode::Node(include));
        }
        match value.downcast_ref::<ResourceHostRef>()? {
            ResourceHostRef::Element(element) => Some(XamlResourceNode::Element(element.clone())),
            ResourceHostRef::Other(host) => Some(XamlResourceNode::Node(host.clone())),
        }
    }
}

impl FromXamlObject for Rc<dyn IResourceNode> {
    fn from_xaml_object(value: &BoxedValue) -> Option<Self> {
        match XamlResourceNode::from_xaml_object(value)? {
            XamlResourceNode::Element(element) => Some(Rc::new(ElementResourceNode(element))),
            XamlResourceNode::Node(node) => Some(node),
        }
    }
}

/// The resource node contract of an element, as a shared handle.
struct ElementResourceNode(Ref<StyledElement>);

impl IResourceNode for ElementResourceNode {
    fn has_resources(&self) -> bool {
        IResourceNode::has_resources(&*self.0)
    }

    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        IResourceNode::try_get_resource(&*self.0, key, theme)
    }
}

impl FromXamlObject for ResourceHostRef {
    fn from_xaml_object(value: &BoxedValue) -> Option<Self> {
        if let Some(object) = as_object(value) {
            if let Some(element) = object.cast::<StyledElement>() {
                return Some(ResourceHostRef::Element(element));
            }
            let application = object.cast::<Application>()?;
            let host: Rc<dyn IResourceHost> = application.as_resource_host();
            return Some(ResourceHostRef::Other(host));
        }
        value.downcast_ref::<ResourceHostRef>().cloned()
    }
}

impl FromXamlObject for Rc<dyn IStyle> {
    fn from_xaml_object(value: &BoxedValue) -> Option<Self> {
        if let Some(object) = as_object(value) {
            return style_of_object(&object);
        }
        let include: Rc<dyn IStyle> = rc_of::<StyleInclude>(value)?;
        Some(include)
    }
}

impl FromXamlObject for Rc<dyn IResourceProvider> {
    fn from_xaml_object(value: &BoxedValue) -> Option<Self> {
        if let Some(object) = as_object(value) {
            if let Some(provider) = object.cast::<ResourceProvider>() {
                return Some(provider.into());
            }
            let style = style_of_object(&object)?;
            style.as_resource_provider()?;
            return Some(Rc::new(StyleResourceProvider(style)));
        }
        if let Some(include) = resource_include_of(value) {
            return Some(include);
        }
        let include: Rc<dyn IResourceProvider> = rc_of::<StyleInclude>(value)?;
        Some(include)
    }
}

impl FromXamlObject for Rc<dyn IThemeVariantProvider> {
    fn from_xaml_object(value: &BoxedValue) -> Option<Self> {
        if let Some(object) = as_object(value) {
            return object.cast::<ResourceDictionary>().map(Into::into);
        }
        let include: Rc<dyn IThemeVariantProvider> = resource_include_of(value)?;
        Some(include)
    }
}

/// The resource provider contract of a style object, as a shared handle.
/// (Style handles expose the contract by reference only.)
struct StyleResourceProvider(Rc<dyn IStyle>);

impl StyleResourceProvider {
    fn provider(&self) -> &dyn IResourceProvider {
        self.0.as_resource_provider().expect("checked when the adapter was created")
    }
}

impl IResourceNode for StyleResourceProvider {
    fn has_resources(&self) -> bool {
        self.0.has_resources()
    }

    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        self.0.try_get_resource(key, theme)
    }
}

impl IResourceProvider for StyleResourceProvider {
    fn owner(&self) -> Option<ResourceHostRef> {
        self.provider().owner()
    }

    fn owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.provider().owner_changed(handler)
    }

    fn add_owner(&self, owner: &ResourceHostRef) {
        self.provider().add_owner(owner)
    }

    fn remove_owner(&self, owner: &ResourceHostRef) {
        self.provider().remove_owner(owner)
    }

    fn as_object(&self) -> Option<&FerroObject> {
        self.provider().as_object()
    }
}

/// Boxes an object as the handle of its own class (`Ref<Button>` for a
/// button), the canonical untyped form of a class instance.
pub(crate) fn box_object(object: Ref<FerroObject>) -> BoxedValue {
    let class = object.get_type();
    let root: BoxedValue = Rc::new(object);
    match class.handle() {
        Some(handle) => match ValueTypes::try_convert(Some(&root), ValueType::new(handle, class.name())) {
            Some(Some(typed)) => typed,
            _ => root,
        },
        None => root,
    }
}

/// `(obj as IThemeVariantHost)?.ActualThemeVariant`.
pub(crate) fn actual_theme_variant_of(value: &BoxedValue) -> Option<ThemeVariant> {
    let object = as_object(value)?;
    if let Some(element) = object.cast::<StyledElement>() {
        return element.actual_theme_variant();
    }
    object.cast::<Application>()?.actual_theme_variant()
}

/// Whether the object provides a data context: an element or the
/// application object (the implementers of the data context provider
/// contract of the managed original).
pub(crate) fn is_data_context_provider(object: &Ref<FerroObject>) -> bool {
    object.is::<StyledElement>() || object.is::<Application>()
}

/// A resource key that is an arbitrary untyped value: equal to another one
/// if the values are equal.
struct ObjectKey(BoxedValue);

impl PartialEq for ObjectKey {
    fn eq(&self, other: &Self) -> bool {
        ValueTypes::identity_equals(Some(&self.0), Some(&other.0))
    }
}

impl Eq for ObjectKey {}

impl Hash for ObjectKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Equal values hold the same type.
        self.0.value_type_id().hash(state)
    }
}

/// The resource key an untyped value stands for: text, a type, a key or any
/// other value.
pub(crate) fn resource_key_of(value: &BoxedValue) -> ResourceKey {
    if let Some(key) = value.downcast_ref::<ResourceKey>() {
        return key.clone();
    }
    if let Some(text) = value.downcast_ref::<String>() {
        return ResourceKey::from(text.as_str());
    }
    if let Some(text) = value.downcast_ref::<&'static str>() {
        return ResourceKey::from(*text);
    }
    if let Some(type_) = value.downcast_ref::<&'static TypeInfo>() {
        return ResourceKey::Type(type_);
    }
    if let Some(type_) = value.downcast_ref::<ValueType>() {
        // A type given by its handle: classes are keyed by their class.
        if let Some((class, _)) = TypeInfo::find_by_handle(type_.id()) {
            return ResourceKey::Type(class);
        }
    }
    ResourceKey::object(ObjectKey(value.clone()))
}

/// The target property of a provide-value target, as the markup runtime
/// distinguishes it.
pub(crate) enum TargetProperty {
    /// A registered property.
    Ferro(&'static FerroProperty),
    /// A plain property.
    Info(Rc<dyn IPropertyInfo>),
}

impl TargetProperty {
    /// `TargetProperty switch { FerroProperty .., IPropertyInfo .. }`.
    pub(crate) fn of(value: Option<&BoxedValue>) -> Option<TargetProperty> {
        let value = value?;
        if let Some(property) = value.downcast_ref::<&'static FerroProperty>() {
            return Some(TargetProperty::Ferro(property));
        }
        let info = value.downcast_ref::<Rc<dyn IPropertyInfo>>()?;
        match info.as_ferro_property() {
            Some(property) => Some(TargetProperty::Ferro(property)),
            None => Some(TargetProperty::Info(info.clone())),
        }
    }

    pub(crate) fn property_type(&self) -> ValueType {
        match self {
            TargetProperty::Ferro(property) => ValueType::new(property.property_type(), property.property_type_name()),
            TargetProperty::Info(info) => info.property_type(),
        }
    }

    /// The property through the plain property contract.
    pub(crate) fn into_property_info(self) -> Rc<dyn IPropertyInfo> {
        match self {
            TargetProperty::Ferro(property) => property.as_property_info(),
            TargetProperty::Info(info) => info,
        }
    }
}
