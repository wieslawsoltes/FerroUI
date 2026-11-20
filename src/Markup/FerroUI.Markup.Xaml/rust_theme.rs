//! Themes written in Rust: the small API that the Rust form of a theme is written against
//! (#988, a prototype; docs/xaml-themes.md, section 10).
//!
//! `scripts/themes/xaml_to_rust.py` converts the documents of a theme (`Resources/*.xaml`,
//! `Controls/*.xaml`) into Rust modules, one per document, that call this API. The API is a
//! thin layer over the object model of the framework (`ResourceDictionary`, `ControlTheme`,
//! `Style`, `Setter`, the controls themselves) that does what the compiled markup of the same
//! document does, in the same order and with the same lookups:
//!
//! - [`Resources`] adds the entries of a resource dictionary: values at once, brushes, aliases
//!   and objects when they are first looked up (deferred), control themes when their control is
//!   first styled.
//! - [`StyleBuilder`] adds the setters and the nested styles of a control theme or a style.
//! - [`Element`] builds an element of a template: its properties (with the template priority in
//!   a template), its template bindings, dynamic and static resources, bindings, name and
//!   children.
//! - [`Scope`] is where `{StaticResource}` looks resources up from: the dictionaries and styles
//!   the place is declared in, innermost first, and the theme variant of the theme dictionary it
//!   is in, as the parent stack of compiled markup gives them.
//!
//! The API is the same for every theme; what is particular to a theme (its class, its units,
//! the variants of its theme dictionaries) is in the generated `mod.rs` of the theme.

use crate::converters::ColorToBrushConverter;
use crate::data::{DynamicResourceAnchor, DynamicResourceExpression, HeldAnchor};
use crate::xaml_il::runtime::compiled as rt;
use crate::xaml_il::runtime::XamlIlContext;
use crate::XamlLoadError;
use ferroui_base::controls::{
    IResourceNode, IResourceProvider, NameScope, NameScopeRef, ResourceDictionary, ResourceKey,
};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::{
    BindingBase, BindingExpressionBase, BindingMode, BindingPriority, CompiledBinding, CompiledBindingPathBuilder,
    TemplateBinding,
};
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::metadata::{IServiceProvider, MarkupValue};
use ferroui_base::styling::{ControlTheme, Selector, Setter, SetterBase, Style, StyleBase, Styles, ThemeVariant};
use ferroui_base::{
    AttachedProperty, BoxedValue, DirectProperty, FerroObject, FerroProperty, ObjectType, PropertyValue, Ref,
    StyledElement, StyledProperty, TypeInfo, Upcast, WeakRef,
};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{FuncControlTemplate, FuncDataTemplate, FuncTemplate, IControlTemplate, IDataTemplate};
use ferroui_controls::{Control, Panel};
use std::cell::RefCell;
use std::rc::Rc;

/// The result of a step of a theme: the error of compiled markup when a step fails.
pub type Result<T = ()> = std::result::Result<T, XamlLoadError>;

/// What the Rust form of a theme imports: this API and the types its values are made of.
pub mod prelude {
    pub use super::{
        literal, nesting, of, path, template_binding, value, AnyProperty, Element, PathExt, Resources, Result, Scope,
        SelectorExt, StyleBuilder, Template, ThemeProperty,
    };
    pub use ferroui_base::controls::ResourceKey;
    pub use ferroui_base::styling::{Selector, ThemeVariant};
    pub use ferroui_base::{ObjectType, Ref, StaticType, Upcast};
}

// --- properties ------------------------------------------------------------------------------

/// A registered property of a typed value: a styled, attached or direct property.
pub trait ThemeProperty: 'static {
    /// The type of the value of the property.
    type Value: PropertyValue;
    /// The property, untyped.
    fn untyped(&'static self) -> &'static FerroProperty;
    /// Sets the property of `object` to `value` with `priority` (a direct property has none).
    fn set_on(&'static self, object: &FerroObject, value: Self::Value, priority: BindingPriority);
}

impl<T: PropertyValue> ThemeProperty for StyledProperty<T> {
    type Value = T;
    fn untyped(&'static self) -> &'static FerroProperty {
        self.as_property()
    }
    fn set_on(&'static self, object: &FerroObject, value: T, priority: BindingPriority) {
        match priority {
            BindingPriority::LocalValue => object.set_value(self, value),
            priority => {
                let _ = object.set_value_with_priority(self, value, priority);
            }
        }
    }
}

impl<T: PropertyValue> ThemeProperty for AttachedProperty<T> {
    type Value = T;
    fn untyped(&'static self) -> &'static FerroProperty {
        let styled: &'static StyledProperty<T> = self;
        styled.as_property()
    }
    fn set_on(&'static self, object: &FerroObject, value: T, priority: BindingPriority) {
        let styled: &'static StyledProperty<T> = self;
        styled.set_on(object, value, priority)
    }
}

impl<O: ObjectType, T: PropertyValue> ThemeProperty for DirectProperty<O, T> {
    type Value = T;
    fn untyped(&'static self) -> &'static FerroProperty {
        let base: &'static ferroui_base::DirectPropertyBase<T> = self;
        base.as_property()
    }
    fn set_on(&'static self, object: &FerroObject, value: T, _priority: BindingPriority) {
        object.set_direct_value(self, value);
    }
}

/// A registered property of any kind, as the property itself: what the steps of a theme that
/// take any property accept (a typed accessor or a `FerroProperty`).
pub trait AnyProperty: 'static {
    /// The property, untyped.
    fn property(&'static self) -> &'static FerroProperty;
}

impl AnyProperty for FerroProperty {
    fn property(&'static self) -> &'static FerroProperty {
        self
    }
}

impl<T: PropertyValue> AnyProperty for StyledProperty<T> {
    fn property(&'static self) -> &'static FerroProperty {
        self.untyped()
    }
}

impl<T: PropertyValue> AnyProperty for AttachedProperty<T> {
    fn property(&'static self) -> &'static FerroProperty {
        self.untyped()
    }
}

impl<O: ObjectType, T: PropertyValue> AnyProperty for DirectProperty<O, T> {
    fn property(&'static self) -> &'static FerroProperty {
        self.untyped()
    }
}

/// The value of a literal of a theme that is parsed (a geometry, a transform, an easing): the
/// error of the parser as the error of the step.
pub fn literal<T, E: std::fmt::Display>(parsed: std::result::Result<T, E>) -> Result<T> {
    parsed.map_err(|error| XamlLoadError::with_message(error.to_string()))
}

/// `{TemplateBinding property}` as a binding (the value of a setter).
pub fn template_binding(property: &'static impl AnyProperty) -> Rc<dyn BindingBase> {
    TemplateBinding::new(property.property())
}

/// The start of a compiled binding path.
pub fn path() -> CompiledBindingPathBuilder {
    CompiledBindingPathBuilder::new()
}

/// The steps of a compiled binding path, in the form of a theme.
pub trait PathExt {
    /// `$parent[T, level]`: the ancestor of class `T` in the logical tree.
    fn ancestor_of<T: ObjectType>(&self, level: usize) -> CompiledBindingPathBuilder;
    /// A registered property.
    fn prop(&self, property: &'static impl AnyProperty) -> CompiledBindingPathBuilder;
    /// `#name`: the element named `name` in the name scope of the template.
    fn named(&self, names: Option<&NameScopeRef>, name: &str) -> CompiledBindingPathBuilder;
    /// A registered property that may be null on the way (`Property?`).
    fn prop_or_null(&self, property: &'static impl AnyProperty) -> CompiledBindingPathBuilder;
    /// A declared property of the type `T` (a view model, a settings object).
    fn member<T: MarkupTyped + ?Sized>(&self, name: &'static str) -> CompiledBindingPathBuilder;
    /// A declared property of the class `T` that is no registered property (`TemplateSettings`).
    fn class_member<T: ObjectType>(&self, name: &'static str) -> CompiledBindingPathBuilder;
    /// A method used as a command.
    fn method<O: 'static>(&self, name: &str, execute: fn(&O)) -> CompiledBindingPathBuilder;
    /// The binding of the path.
    fn binding(&self) -> Rc<CompiledBinding>;
}

impl PathExt for CompiledBindingPathBuilder {
    fn ancestor_of<T: ObjectType>(&self, level: usize) -> CompiledBindingPathBuilder {
        self.ancestor(Some(T::TYPE), level)
    }
    fn prop(&self, property: &'static impl AnyProperty) -> CompiledBindingPathBuilder {
        self.ferro_property(property.property())
    }
    fn prop_or_null(&self, property: &'static impl AnyProperty) -> CompiledBindingPathBuilder {
        self.ferro_property_with(property.property(), true)
    }
    fn named(&self, names: Option<&NameScopeRef>, name: &str) -> CompiledBindingPathBuilder {
        match names {
            Some(names) => self.element_name(names.clone(), name),
            None => self.clone(),
        }
    }
    fn member<T: MarkupTyped + ?Sized>(&self, name: &'static str) -> CompiledBindingPathBuilder {
        let property_type = rt::declared_property_type(T::MARKUP, name, false);
        rt::path_property(self, T::MARKUP, name, false, property_type, false, false, true)
    }
    fn class_member<T: ObjectType>(&self, name: &'static str) -> CompiledBindingPathBuilder {
        let markup = rt::class_markup(T::TYPE);
        let property_type = rt::declared_property_type(markup, name, false);
        rt::path_property(self, markup, name, false, property_type, false, false, true)
    }
    fn method<O: 'static>(&self, name: &str, execute: fn(&O)) -> CompiledBindingPathBuilder {
        self.command::<O>(name, move |owner, _| execute(owner), None, &[])
    }
    fn binding(&self) -> Rc<CompiledBinding> {
        CompiledBinding::new(self.build())
    }
}

/// `value` as a value of markup (the untyped form a resource dictionary and a setter hold).
pub fn value<T: PartialEq + 'static>(value: T) -> MarkupValue {
    rt::to_value(value)
}

// --- selectors -------------------------------------------------------------------------------

/// The selector `^`, which nested styles start with.
pub fn nesting() -> Selector {
    ferroui_base::styling::Selectors::nesting(None)
}

/// The selector of the class `T` (`Button`): by the identity of the class, which keeps the class
/// out of an application that never creates one (docs/xaml-themes.md, section 2.4).
pub fn of<T: ObjectType>() -> Selector {
    ferroui_base::styling::Selectors::of_type_identity(None, T::IDENTITY)
}

/// The selector `:is(T)` (`T` or a class derived from it), by identity, as [`of`].
pub fn is<T: ObjectType>() -> Selector {
    ferroui_base::styling::Selectors::is_type_identity(None, T::IDENTITY)
}

/// The selector of the style class `name` (`.accent`, `:pointerover`).
pub fn class(name: &str) -> Selector {
    ferroui_base::styling::Selectors::class(None, name)
}

/// The selector of the element named `name` (`#PART_Border`).
pub fn named(name: &str) -> Selector {
    ferroui_base::styling::Selectors::name(None, name)
}

/// The selectors of the classes a selector names, by identity, as [`of`].
pub trait SelectorExt {
    /// `.. T`: the class `T`.
    fn of<T: ObjectType>(self) -> Selector;
    /// `:is(T)`: the class `T` or a class derived from it.
    fn is<T: ObjectType>(self) -> Selector;
    /// `[property=value]`: the value of a property.
    fn property_is<P: ThemeProperty>(self, property: &'static P, value: P::Value) -> Selector;
}

impl SelectorExt for Selector {
    fn of<T: ObjectType>(self) -> Selector {
        ferroui_base::styling::Selectors::of_type_identity(Some(self), T::IDENTITY)
    }
    fn is<T: ObjectType>(self) -> Selector {
        ferroui_base::styling::Selectors::is_type_identity(Some(self), T::IDENTITY)
    }
    fn property_is<P: ThemeProperty>(self, property: &'static P, value: P::Value) -> Selector {
        self.property_equals_untyped(property.untyped(), Rc::new(value))
    }
}

// --- the scope of a lookup -------------------------------------------------------------------

/// A node of a [`Scope`]: held weakly, as compiled markup holds what a deferred resource
/// captures.
enum ScopeNode {
    Dictionary(WeakRef<ResourceDictionary>),
    Style(WeakRef<StyleBase>),
    Styles(WeakRef<Styles>),
}

struct ScopeLink {
    node: ScopeNode,
    /// The theme variant of the theme dictionary this node is, if it is one.
    variant: Option<ThemeVariant>,
    parent: Option<Scope>,
}

/// Where `{StaticResource}` looks resources up from: the dictionaries and styles a place is
/// declared in, innermost first.
#[derive(Clone)]
pub struct Scope(Option<Rc<ScopeLink>>);

impl Scope {
    /// The empty scope.
    pub fn empty() -> Self {
        Scope(None)
    }

    fn push(&self, node: ScopeNode, variant: Option<ThemeVariant>) -> Self {
        Scope(Some(Rc::new(ScopeLink { node, variant, parent: Some(self.clone()) })))
    }

    /// This scope with `dictionary` innermost.
    pub fn with_dictionary(&self, dictionary: &Ref<ResourceDictionary>) -> Self {
        self.push(ScopeNode::Dictionary(dictionary.downgrade()), None)
    }

    /// This scope with the theme dictionary `dictionary` of `variant` innermost.
    pub fn with_theme_dictionary(&self, dictionary: &Ref<ResourceDictionary>, variant: ThemeVariant) -> Self {
        self.push(ScopeNode::Dictionary(dictionary.downgrade()), Some(variant))
    }

    /// This scope with the style (or control theme) `style` innermost.
    pub fn with_style(&self, style: &Ref<StyleBase>) -> Self {
        self.push(ScopeNode::Style(style.downgrade()), None)
    }

    /// This scope with the styles `styles` innermost.
    pub fn with_styles(&self, styles: &Ref<Styles>) -> Self {
        self.push(ScopeNode::Styles(styles.downgrade()), None)
    }

    fn links(&self) -> impl Iterator<Item = &ScopeLink> {
        let mut next = self.0.as_deref();
        std::iter::from_fn(move || {
            let link = next?;
            next = link.parent.as_ref().and_then(|parent| parent.0.as_deref());
            Some(link)
        })
    }

    /// The theme variant of the nearest theme dictionary of the scope.
    pub fn variant(&self) -> Option<ThemeVariant> {
        self.links().find_map(|link| link.variant.clone())
    }

    /// The resource `key` for `variant`, from the innermost node that has it.
    pub fn find(&self, key: &ResourceKey, variant: Option<&ThemeVariant>) -> Option<MarkupValue> {
        for link in self.links() {
            let found = match &link.node {
                ScopeNode::Dictionary(dictionary) => {
                    dictionary.upgrade().and_then(|dictionary| dictionary.try_get_resource(key, variant))
                }
                ScopeNode::Style(style) => style.upgrade().and_then(|style| style.try_get_resource(key, variant)),
                ScopeNode::Styles(styles) => styles.upgrade().and_then(|styles| styles.try_get_resource(key, variant)),
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// `{StaticResource key}` for a value of `target_type`: the resource for the variant of the
    /// scope, a colour converted to a brush for a brush; the error of compiled markup when no
    /// node has it.
    pub fn static_resource(&self, key: &str, target_type: Option<ValueType>) -> Result<MarkupValue> {
        self.static_resource_for(key, target_type, self.variant().as_ref())
    }

    fn static_resource_for(&self, key: &str, target_type: Option<ValueType>, variant: Option<&ThemeVariant>) -> Result<MarkupValue> {
        let resource_key = ResourceKey::from(key);
        match self.find(&resource_key, variant) {
            Some(found) => Ok(ColorToBrushConverter::convert_to(found, target_type)),
            None => Err(XamlLoadError::with_message(format!("Static resource '{key}' not found."))),
        }
    }

    /// The value converter that is the static resource `key`.
    pub fn converter(&self, key: &str) -> Result<Rc<dyn IValueConverter>> {
        let found = self.static_resource(key, None)?;
        ferroui_base::metadata::from_markup_value::<Rc<dyn IValueConverter>>(&found)
            .ok_or_else(|| XamlLoadError::with_message(format!("The resource '{key}' is not a value converter.")))
    }

    /// The multi value converter that is the static resource `key`.
    pub fn multi_converter(&self, key: &str) -> Result<Rc<dyn ferroui_base::data::converters::IMultiValueConverter>> {
        let found = self.static_resource(key, None)?;
        ferroui_base::metadata::from_markup_value::<Rc<dyn ferroui_base::data::converters::IMultiValueConverter>>(&found)
            .ok_or_else(|| XamlLoadError::with_message(format!("The resource '{key}' is not a multi value converter.")))
    }

    /// The static resource `key` as a value of type `T`.
    pub fn resource<T: Clone + 'static>(&self, key: &str) -> Result<T> {
        let found = self.static_resource(key, Some(ValueType::of::<T>()))?;
        ferroui_base::metadata::from_markup_value::<T>(&found)
            .ok_or_else(|| XamlLoadError::with_message(format!("The resource '{key}' is not of the type of its place.")))
    }

    /// Starts the element `object`, made outside a template in this scope (the value of a setter
    /// or a resource).
    pub fn make<E: ObjectType + Upcast<FerroObject>>(&self, object: Ref<E>) -> Element<'_, E> {
        Element::local(object, self)
    }

    /// The anchor of a dynamic resource declared at this place on an object that is not an
    /// element: the innermost resource provider of the scope.
    fn anchor(&self) -> Option<HeldAnchor> {
        self.links().find_map(|link| match &link.node {
            ScopeNode::Dictionary(dictionary) => {
                dictionary.upgrade().map(|dictionary| HeldAnchor::from(DynamicResourceAnchor::Provider(Rc::<dyn IResourceProvider>::from(dictionary))))
            }
            ScopeNode::Style(style) => style.upgrade().map(|style| HeldAnchor::from(DynamicResourceAnchor::Provider(Rc::<dyn IResourceProvider>::from(style)))),
            ScopeNode::Styles(styles) => styles.upgrade().map(|styles| HeldAnchor::from(DynamicResourceAnchor::Provider(Rc::<dyn IResourceProvider>::from(styles)))),
        })
    }
}

/// The theme being populated: its scope, the units it applies and the context the units are
/// called with.
struct Populating {
    scope: Scope,
    units: rt::ThemeParts,
    context: Rc<XamlIlContext>,
}

ferroui_base::ferro_thread_local! {
    /// The themes being populated: the units of a theme, which compiled markup calls with a
    /// dictionary alone, add their entries in the scope of the innermost.
    static POPULATING: RefCell<Vec<Populating>> = const { RefCell::new(Vec::new()) };
}

/// Runs `populate` as the population of a theme in `scope` that applies the units `units`
/// ([`Resources::of`], [`Resources::include`]).
pub fn populating<R>(scope: Scope, units: rt::ThemeParts, context: Rc<XamlIlContext>, populate: impl FnOnce() -> R) -> R {
    POPULATING.with(|stack| stack.borrow_mut().push(Populating { scope, units, context }));
    let result = populate();
    POPULATING.with(|stack| stack.borrow_mut().pop());
    result
}

/// A dynamic resource as a binding: `{DynamicResource key}` declared at a place whose
/// anchor, theme variant and priority are given.
fn dynamic_resource(key: &str, anchor: Option<HeldAnchor>, variant: Option<ThemeVariant>, priority: BindingPriority) -> DynamicBinding {
    DynamicBinding { key: ResourceKey::from(key), anchor, variant, priority }
}

/// `{DynamicResource key}` as a binding.
struct DynamicBinding {
    key: ResourceKey,
    anchor: Option<HeldAnchor>,
    variant: Option<ThemeVariant>,
    priority: BindingPriority,
}

impl PartialEq for DynamicBinding {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl BindingBase for DynamicBinding {
    fn create_instance(
        &self,
        _target: &FerroObject,
        _target_property: Option<&'static FerroProperty>,
        _anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        DynamicResourceExpression::with_held_anchor(self.key.clone(), self.anchor.clone(), self.variant.clone(), self.priority)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

// --- resources -------------------------------------------------------------------------------

/// The entries of a resource dictionary being populated, with the scope its entries look
/// static resources up from.
pub struct Resources {
    dictionary: Ref<ResourceDictionary>,
    scope: Scope,
}

impl Resources {
    /// The entries of `dictionary`, declared in `scope` (which does not have `dictionary`).
    pub fn new(dictionary: &Ref<ResourceDictionary>, scope: &Scope) -> Self {
        Self { dictionary: dictionary.clone(), scope: scope.with_dictionary(dictionary) }
    }

    /// The entries of `dictionary`, in the scope of the theme being populated: what a unit of
    /// a theme adds its entries with.
    pub fn of(dictionary: &Ref<ResourceDictionary>) -> Self {
        let scope = POPULATING.with(|stack| stack.borrow().last().map(|populating| populating.scope.clone())).unwrap_or_else(Scope::empty);
        Self::new(dictionary, &scope)
    }

    /// The dictionary.
    pub fn dictionary(&self) -> &Ref<ResourceDictionary> {
        &self.dictionary
    }

    /// The scope of the entries.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Adds `value` under `key`.
    pub fn add<T: PartialEq + 'static>(&self, key: &str, value: T) {
        self.dictionary.add(key, rt::to_object(value));
    }

    /// Adds the text `text` under `key`.
    pub fn text(&self, key: &str, text: &str) {
        self.dictionary.add(key, rt::string_object(text));
    }

    /// Adds what `build` makes under `key`, made when the resource is first looked up, with the
    /// scope of the entry. `build` is a function, not a closure, so that the step is compiled
    /// once for every resource of a theme.
    pub fn deferred(&self, key: impl Into<ResourceKey>, build: fn(&Scope) -> Result<MarkupValue>) {
        self.deferred_with(key.into(), Rc::new(build));
    }

    /// [`deferred`](Self::deferred) of a build that holds values (the steps of this API).
    #[inline(never)]
    fn deferred_with(&self, key: ResourceKey, build: Rc<dyn Fn(&Scope) -> Result<MarkupValue>>) {
        let scope = self.scope.clone();
        let shown = key.clone();
        self.dictionary.add_deferred_fn(key, move |_| match build(&scope) {
            Ok(value) => value,
            Err(error) => {
                report(&shown, &error);
                None
            }
        });
    }

    /// Adds a solid colour brush of the colour `argb` under `key`.
    pub fn color_brush(&self, key: &str, argb: u32) {
        self.deferred_with(key.into(), Rc::new(move |_| {
            let brush = ferroui_base::media::SolidColorBrush::new();
            brush.set_value(ferroui_base::media::SolidColorBrush::color_property(), ferroui_base::media::Color::from_uint32(argb));
            Ok(value(brush))
        }));
    }

    /// Adds a solid colour brush whose colour is the static resource `color_key` under `key`.
    pub fn static_color_brush(&self, key: &str, color_key: &'static str) {
        self.deferred_with(key.into(), Rc::new(move |scope| {
            let brush = ferroui_base::media::SolidColorBrush::new();
            let color = scope.static_resource(color_key, Some(ValueType::of::<ferroui_base::media::Color>()))?;
            assign(brush.upcast_ref::<FerroObject>(), ferroui_base::media::SolidColorBrush::color_property(), color, BindingPriority::LocalValue)?;
            Ok(value(brush))
        }));
    }

    /// Adds a solid colour brush whose colour is the dynamic resource `color_key` under `key`.
    pub fn dynamic_color_brush(&self, key: &str, color_key: &'static str) {
        self.deferred_with(key.into(), Rc::new(move |scope| {
            let brush = ferroui_base::media::SolidColorBrush::new();
            let binding = dynamic_resource(color_key, scope.anchor(), scope.variant(), BindingPriority::LocalValue);
            let _ = brush.bind_binding(ferroui_base::media::SolidColorBrush::color_property().as_property(), &binding);
            Ok(value(brush))
        }));
    }

    /// Adds the resource `target` under `key` (`<StaticResource x:Key="key" ResourceKey="target" />`).
    pub fn alias(&self, key: &str, target: &'static str) {
        self.deferred_with(key.into(), Rc::new(move |scope| scope.static_resource(target, None)));
    }

    /// Adds the control theme `build` makes under `key`, made when it is first looked up.
    pub fn control_theme(&self, key: impl Into<ResourceKey>, build: fn(&StyleBuilder) -> Result) {
        self.deferred_with(key.into(), Rc::new(move |scope| {
            let theme = ControlTheme::new();
            let builder = StyleBuilder::control_theme(&theme, scope);
            build(&builder)?;
            Ok(value(theme))
        }));
    }

    /// Adds the entries `populate` adds to the theme dictionary of `variant`, which is created
    /// when the dictionary has none.
    pub fn theme(&self, variant: ThemeVariant, populate: fn(&Resources) -> Result) -> Result {
        let dictionary = match self.dictionary.theme_dictionary(&variant) {
            Some(existing) => existing
                .as_object()
                .and_then(|object| object.to_ref().cast::<ResourceDictionary>())
                .ok_or_else(|| XamlLoadError::with_message("The theme dictionary is not a resource dictionary."))?,
            None => {
                let dictionary = ResourceDictionary::new();
                dictionary.set_key(Some(variant.clone()));
                self.dictionary.add_theme_dictionary(variant.clone(), dictionary.clone());
                dictionary
            }
        };
        let resources = Resources { scope: self.scope.with_theme_dictionary(&dictionary, variant), dictionary };
        populate(&resources)
    }

    /// Merges the unit `index` of the theme being populated (`<MergeResourceInclude>` of one of
    /// its documents), when the units it applies have it: an application that trims the theme
    /// leaves the unit out.
    pub fn include(&self, index: u32) -> Result {
        let frame = POPULATING.with(|stack| stack.borrow().last().map(|populating| (populating.units, populating.context.clone())));
        match frame {
            Some((units, context)) => rt::apply_theme_unit(units, index, 0, &context, &self.dictionary),
            None => Ok(()),
        }
    }

    /// Adds `provider` to the merged dictionaries.
    pub fn merge(&self, provider: impl Into<Rc<dyn IResourceProvider>>) {
        self.dictionary.add_merged_dictionary(provider);
    }
}

/// Reports the failure of a deferred resource, as compiled markup reports it to the error scope
/// of the load.
fn report(key: &ResourceKey, error: &XamlLoadError) {
    ferroui_base::logging::Logger::try_get(ferroui_base::logging::LogEventLevel::Error, ferroui_base::logging::LogArea::PROPERTY)
        .map(|log| log.log(None, &format!("The resource {key} failed: {error}")));
}

/// Assigns the markup value `value` (a static resource) to `property` of `target`: a binding is
/// bound, the unset marker clears the property, anything else is set with `priority`.
fn assign(target: &FerroObject, property: &'static FerroProperty, value: MarkupValue, priority: BindingPriority) -> Result {
    if value.is_some() {
        if let Some(binding) = ferroui_base::metadata::from_markup_value::<Rc<dyn BindingBase>>(&value) {
            let _ = target.bind_binding(property, &*binding);
            return Ok(());
        }
    }
    let target_type = ValueType::new(property.property_type(), property.property_type_name());
    let whole: BoxedValue = Rc::new(value.clone());
    let converted = ferroui_base::data::core::ValueTypes::try_cast(&whole, target_type)
        .or_else(|| value.as_ref().and_then(|boxed| ferroui_base::data::core::ValueTypes::try_cast(boxed, target_type)))
        .ok_or_else(|| {
            XamlLoadError::with_message(format!(
                "The value of the property '{}' is not of its type ({}).",
                property.name(),
                property.property_type_name()
            ))
        })?;
    let _ = target.set_value_untyped(property, &*converted, priority);
    Ok(())
}

// --- styles and control themes ---------------------------------------------------------------

/// The setters and nested styles of a control theme or a style being built.
pub struct StyleBuilder {
    style: Ref<StyleBase>,
    theme: Option<Ref<ControlTheme>>,
    scope: Scope,
}

impl StyleBuilder {
    fn control_theme(theme: &Ref<ControlTheme>, scope: &Scope) -> Self {
        let style = theme.clone().upcast::<StyleBase>();
        Self { scope: scope.with_style(&style), style, theme: Some(theme.clone()) }
    }

    /// The scope of the setters.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// `TargetType`: the class the control theme is for.
    pub fn target<T: ObjectType>(&self) -> &Self {
        if let Some(theme) = &self.theme {
            theme.set_target_type(Some(T::TYPE));
        }
        self
    }

    /// `BasedOn="{StaticResource key}"`.
    pub fn based_on(&self, key: impl Into<ResourceKey>) -> Result<&Self> {
        let key = key.into();
        let found = self.scope.find(&key, self.scope.variant().as_ref()).flatten();
        let based_on = found
            .as_ref()
            .and_then(|boxed| boxed.downcast_ref::<Ref<ControlTheme>>().cloned())
            .ok_or_else(|| XamlLoadError::with_message(format!("Static resource '{key}' not found.")))?;
        if let Some(theme) = &self.theme {
            theme.set_based_on(Some(based_on));
        }
        Ok(self)
    }

    /// A setter of `property` to `value`.
    pub fn set<P: ThemeProperty>(&self, property: &'static P, value: P::Value) -> &Self {
        rt::add_setter(&self.style, property.untyped(), rt::to_object(value));
        self
    }

    /// A setter of `property` to the markup value `value` (made by a converter or a markup
    /// extension).
    pub fn set_value(&self, property: &'static impl AnyProperty, value: MarkupValue) -> &Self {
        rt::add_setter(&self.style, property.property(), value);
        self
    }

    /// A setter of `property` to null (`{x:Null}`).
    pub fn set_null(&self, property: &'static impl AnyProperty) -> &Self {
        rt::add_setter(&self.style, property.property(), None);
        self
    }

    /// A setter of `property` to `{DynamicResource key}`.
    pub fn dynamic(&self, property: &'static impl AnyProperty, key: &str) -> &Self {
        let property = property.property();
        let anchor = HeldAnchor::from(DynamicResourceAnchor::Provider(Rc::<dyn IResourceProvider>::from(&self.style)));
        let extension = crate::markup_extensions::DynamicResourceExtension::at_place(
            key,
            Some(anchor),
            self.scope.variant(),
            BindingPriority::LocalValue,
        );
        let setter = rt::new_setter(property);
        rt::add_setter_value(&self.style, &setter, rt::to_object(extension));
        self
    }

    /// A setter of `property` to `{StaticResource key}`.
    pub fn static_resource(&self, property: &'static impl AnyProperty, key: &str) -> Result<&Self> {
        let property = property.property();
        let target = ValueType::new(property.property_type(), property.property_type_name());
        let value = self.scope.static_resource(key, Some(target))?;
        rt::add_setter(&self.style, property, value);
        Ok(self)
    }

    /// A setter of `property` to a binding.
    pub fn bind(&self, property: &'static impl AnyProperty, binding: Rc<dyn BindingBase>) -> &Self {
        rt::add_setter(&self.style, property.property(), rt::to_value(binding));
        self
    }

    /// A setter of `property` to `{TemplateBinding source}`.
    pub fn template_bind(&self, property: &'static impl AnyProperty, source: &'static impl AnyProperty) -> &Self {
        self.bind(property, template_binding(source))
    }

    /// A setter of `property` to a value that `build` makes for each control the setter applies
    /// to (`<Template>` as the value of a setter).
    pub fn set_template<P: ThemeProperty>(&self, property: &'static P, build: fn(&Template<Control>) -> Result<P::Value>) -> &Self {
        let scope = self.scope.clone();
        let template: Rc<dyn ferroui_base::styling::ITemplate> = FuncTemplate::new(move || {
            let template = Template::new(None, Some(NameScopeRef::new(NameScope::new())), &scope);
            build(&template).unwrap_or_else(|error| panic!("{error}"))
        });
        self.style.add_setter(Setter::new_template(property.untyped(), template));
        self
    }

    /// A setter of `Template` to the control template `build` makes for a control of class `T`.
    pub fn template<T: ObjectType + Upcast<TemplatedControl>>(&self, build: fn(&Template<T>) -> Result<Ref<Control>>) -> &Self {
        let template = control_template(&self.scope, build);
        rt::add_setter(&self.style, TemplatedControl::template_property().as_property(), rt::to_value(template));
        self
    }

    /// A nested style of `selector`, with the setters and styles `populate` adds.
    #[inline]
    pub fn style(&self, selector: Selector, populate: impl FnOnce(&StyleBuilder) -> Result) -> Result<&Self> {
        let mut populate = Some(populate);
        self.nested_style(selector, &mut |builder| (populate.take().expect("populated once"))(builder))
    }

    /// [`style`](Self::style), compiled once: the closure of each style is called through it.
    #[inline(never)]
    fn nested_style(&self, selector: Selector, populate: &mut dyn FnMut(&StyleBuilder) -> Result) -> Result<&Self> {
        let nested = Style::new();
        nested.set_selector(Some(selector));
        nested.enable_selector_index();
        let style = nested.clone().upcast::<StyleBase>();
        let builder = StyleBuilder { scope: self.scope.with_style(&style), style, theme: None };
        populate(&builder)?;
        self.style.add_style(nested);
        Ok(self)
    }

    /// An animation of the style.
    pub fn animation(&self, animation: Ref<ferroui_base::animation::Animation>) -> &Self {
        self.style.add_animation(animation);
        self
    }

    /// The style being built.
    pub fn style_base(&self) -> &Ref<StyleBase> {
        &self.style
    }
}

/// A control template for a control of class `T` whose content `build` makes, with `scope` as
/// the scope of its static resources.
pub fn control_template<T: ObjectType + Upcast<TemplatedControl>>(
    scope: &Scope,
    build: fn(&Template<T>) -> Result<Ref<Control>>,
) -> Rc<dyn IControlTemplate> {
    let scope = scope.clone();
    FuncControlTemplate::new(move |parent, names| {
        // A template applied to a control of another class (a test, a theme of a base class)
        // builds without the typed parent; its bindings find the templated parent at run time.
        let parent = parent.cast::<T>();
        let template = Template::new(parent.as_ref(), Some(names.clone()), &scope);
        match build(&template) {
            Ok(content) => content,
            Err(error) => panic!("{error}"),
        }
    })
}

/// A template of a panel (`ItemsPanelTemplate`) whose panel `build` makes.
pub fn items_panel(scope: &Scope, build: fn(&Template<Control>) -> Result<Ref<Panel>>) -> Rc<FuncTemplate<Option<Ref<Panel>>>> {
    let scope = scope.clone();
    FuncTemplate::new(move || {
        let template = Template::new(None, None, &scope);
        Some(build(&template).unwrap_or_else(|error| panic!("{error}")))
    })
}

/// A focus adorner template (`FocusAdornerTemplate`) whose adorner `build` makes.
pub fn focus_adorner(scope: &Scope, build: fn(&Template<Control>) -> Result<Ref<Control>>) -> Rc<FuncTemplate<Option<Ref<Control>>>> {
    let scope = scope.clone();
    FuncTemplate::new(move || {
        let names = NameScopeRef::new(NameScope::new());
        let template = Template::new(None, Some(names), &scope);
        Some(build(&template).unwrap_or_else(|error| panic!("{error}")))
    })
}

/// A template of a control (`<Template>` as the value of a property of a template type): the
/// control `build` makes, each time the template is built.
pub fn template_of(scope: &Scope, build: fn(&Template<Control>) -> Result<Ref<Control>>) -> Rc<FuncTemplate<Ref<Control>>> {
    let scope = scope.clone();
    FuncTemplate::new(move || {
        let names = NameScopeRef::new(NameScope::new());
        let template = Template::new(None, Some(names), &scope);
        build(&template).unwrap_or_else(|error| panic!("{error}"))
    })
}

/// The template of the decorations of a window (`WindowDrawnDecorationsTemplate`) whose
/// content `build` makes.
pub fn decorations_template<T: ObjectType>(
    scope: &Scope,
    build: fn(&Template<T>) -> Result<Ref<ferroui_controls::chrome::WindowDrawnDecorationsContent>>,
) -> Rc<dyn ferroui_controls::chrome::IWindowDrawnDecorationsTemplate> {
    Rc::new(DecorationsTemplate { scope: scope.clone(), build })
}

struct DecorationsTemplate<T: ObjectType> {
    scope: Scope,
    build: fn(&Template<T>) -> Result<Ref<ferroui_controls::chrome::WindowDrawnDecorationsContent>>,
}

impl<T: ObjectType> DecorationsTemplate<T> {
    fn build_content(&self) -> ferroui_controls::templates::TemplateResult<Ref<ferroui_controls::chrome::WindowDrawnDecorationsContent>> {
        let names = NameScopeRef::new(NameScope::new());
        let template = Template::new(None, Some(names.clone()), &self.scope);
        let content = (self.build)(&template).unwrap_or_else(|error| panic!("{error}"));
        ferroui_controls::templates::TemplateResult::new(content, names)
    }
}

impl<T: ObjectType> ferroui_base::styling::ITemplate for DecorationsTemplate<T> {
    fn build(&self) -> BoxedValue {
        Rc::new(self.build_content().result().clone())
    }
}

impl<T: ObjectType> ferroui_controls::chrome::IWindowDrawnDecorationsTemplate for DecorationsTemplate<T> {
    fn build_typed(&self) -> ferroui_controls::templates::TemplateResult<Ref<ferroui_controls::chrome::WindowDrawnDecorationsContent>> {
        self.build_content()
    }
}

/// A data template (`DataTemplate`) whose content `build` makes for any data.
pub fn data_template(scope: &Scope, build: fn(&Template<Control>) -> Result<Ref<Control>>) -> Rc<dyn IDataTemplate> {
    let scope = scope.clone();
    FuncDataTemplate::new(
        |_| true,
        move |_, names| {
            let template = Template::new(None, Some(names.clone()), &scope);
            Some(build(&template).unwrap_or_else(|error| panic!("{error}")))
        },
        true,
    )
}

/// A data template (`DataTemplate DataType="T"`) for data of the type `T`, whose content
/// `build` makes.
pub fn data_template_of<T: MarkupTyped + ?Sized>(scope: &Scope, build: fn(&Template<Control>) -> Result<Ref<Control>>) -> Rc<dyn IDataTemplate> {
    let scope = scope.clone();
    FuncDataTemplate::new(
        |data| rt::is_instance(&data.cloned(), rt::markup_handle(T::MARKUP)),
        move |_, names| {
            let template = Template::new(None, Some(names.clone()), &scope);
            Some(build(&template).unwrap_or_else(|error| panic!("{error}")))
        },
        true,
    )
}

/// A data template for data of the Rust type `T` (`DataType="x:String"` is `String`), whose
/// content `build` makes.
pub fn data_template_for<T: 'static>(scope: &Scope, build: fn(&Template<Control>) -> Result<Ref<Control>>) -> Rc<dyn IDataTemplate> {
    let scope = scope.clone();
    FuncDataTemplate::new(
        |data| data.is_some_and(|data| data.downcast_ref::<T>().is_some()),
        move |_, names| {
            let template = Template::new(None, Some(names.clone()), &scope);
            Some(build(&template).unwrap_or_else(|error| panic!("{error}")))
        },
        true,
    )
}

/// A setter of an animation (`<Setter>` of a `KeyFrame`): `property` to `value`.
pub fn setter<P: ThemeProperty>(property: &'static P, value: P::Value) -> Rc<Setter> {
    let setter = rt::new_setter(property.untyped());
    Setter::__markup_set_Value(&setter, rt::setter_value(&setter, rt::to_object(value)));
    setter
}

/// A setter of an animation whose value is a binding.
pub fn bound_setter(property: &'static impl AnyProperty, binding: Rc<dyn BindingBase>) -> Rc<Setter> {
    let setter = rt::new_setter(property.property());
    Setter::__markup_set_Value(&setter, rt::to_value(binding));
    setter
}

/// `{DynamicResource key}` as a binding of its own (an item of a multi binding).
pub fn dynamic_resource_binding(key: &str) -> Rc<dyn BindingBase> {
    Rc::new(dynamic_resource(key, None, None, BindingPriority::LocalValue))
}

/// A style of `selector` declared in `scope`, with the setters and styles `populate` adds (a
/// style of a styles document).
pub fn style(scope: &Scope, selector: Selector, populate: impl FnOnce(&StyleBuilder) -> Result) -> Result<Ref<Style>> {
    let nested = Style::new();
    nested.set_selector(Some(selector));
    let style = nested.clone().upcast::<StyleBase>();
    let builder = StyleBuilder { scope: scope.with_style(&style), style, theme: None };
    populate(&builder)?;
    Ok(nested)
}

// --- templates and elements ------------------------------------------------------------------

/// The content of a template being built for the control `parent`.
pub struct Template<'a, T: ObjectType> {
    parent: Option<&'a Ref<T>>,
    names: Option<NameScopeRef>,
    scope: &'a Scope,
    priority: BindingPriority,
    /// The elements being built, innermost last: the anchor of the bindings of an object of
    /// the template that is not an element (a transform, a key binding).
    elements: RefCell<Vec<Ref<FerroObject>>>,
}

impl<'a, T: ObjectType> Template<'a, T> {
    fn new(parent: Option<&'a Ref<T>>, names: Option<NameScopeRef>, scope: &'a Scope) -> Self {
        Template { parent, names, scope, priority: BindingPriority::Template, elements: RefCell::new(Vec::new()) }
    }

    /// The control the template is applied to (none for a template of a panel, an adorner or
    /// data).
    pub fn parent(&self) -> Option<&Ref<T>> {
        self.parent
    }

    /// The scope of the static resources of the template.
    pub fn scope(&self) -> &Scope {
        self.scope
    }

    /// The name scope of the template.
    pub fn names(&self) -> Option<&NameScopeRef> {
        self.names.as_ref()
    }

    /// A reference to the element named `name` in the name scope of the template (a value of
    /// an `ElementRef` property: `PlacementTarget="Background"`).
    pub fn element_ref(&self, name: &str) -> Result<Option<ferroui_base::ElementRef<Control>>> {
        let control = self.names.as_ref().and_then(|names| names.find(name)).and_then(|object| object.cast::<Control>());
        match control {
            Some(control) => Ok(Some(ferroui_base::ElementRef::of(&control))),
            None => Err(XamlLoadError::with_message(format!("The element '{name}' is not in the name scope of the template."))),
        }
    }

    /// Starts the element `object` of the template.
    pub fn make<E: ObjectType + Upcast<FerroObject>>(&self, object: Ref<E>) -> Element<'_, E> {
        Element::start_in(object, self.scope, self.names.as_ref(), self.priority, Some(&self.elements))
    }
}

/// An element being built: its properties, bindings, name and children. A styled element is
/// initialised between [`start`](Self::start) and [`end`](Self::end), as compiled markup
/// initialises it.
///
/// The steps are functions of [`ElementCore`], which does not depend on the class of the
/// element: the typed methods here are thin and inlined, so a step is compiled once, not once per
/// class of element.
pub struct Element<'a, E: ObjectType> {
    object: Ref<E>,
    core: ElementCore<'a>,
}

/// What a step of an [`Element`] needs besides the element: the scope, the name scope, the
/// priority of its values and the anchor of the bindings of an object that is not an element.
pub struct ElementCore<'a> {
    scope: &'a Scope,
    names: Option<&'a NameScopeRef>,
    priority: BindingPriority,
    /// The nearest element around this object, for the bindings of an object that is not an
    /// element (a brush, a transform): what they find named elements and ancestors from.
    anchor: Option<Ref<FerroObject>>,
    /// The elements of the template being built.
    elements: Option<&'a RefCell<Vec<Ref<FerroObject>>>>,
    /// The object as a styled element, if it is one.
    styled: Option<Ref<StyledElement>>,
}

impl<'a> ElementCore<'a> {
    #[inline(never)]
    fn start(
        object: &FerroObject,
        scope: &'a Scope,
        names: Option<&'a NameScopeRef>,
        priority: BindingPriority,
        elements: Option<&'a RefCell<Vec<Ref<FerroObject>>>>,
    ) -> Self {
        let styled = object.to_ref().cast::<StyledElement>();
        let mut core = ElementCore { scope, names, priority, anchor: None, elements, styled };
        match &core.styled {
            Some(element) => {
                element.begin_init();
                if let Some(elements) = elements {
                    elements.borrow_mut().push(element.clone().upcast());
                }
            }
            None => core.anchor = elements.and_then(|elements| elements.borrow().last().cloned()),
        }
        core
    }

    #[inline(never)]
    fn child(&self, object: &FerroObject) -> ElementCore<'a> {
        let mut child = ElementCore::start(object, self.scope, self.names, self.priority, self.elements);
        if child.styled.is_none() {
            child.anchor = match &self.styled {
                Some(element) => Some(element.clone().upcast()),
                None => self.anchor.clone(),
            };
        }
        child
    }

    #[inline(never)]
    fn template_bind(&self, object: &FerroObject, property: &'static FerroProperty, source: &'static FerroProperty) {
        let binding = TemplateBinding::default();
        binding.set_property(Some(source));
        let _ = object.bind_binding(property, &binding);
    }

    #[inline(never)]
    fn dynamic(&self, object: &FerroObject, property: &'static FerroProperty, key: &str) {
        let anchor = match self.styled {
            Some(_) => None,
            None => self.scope.anchor(),
        };
        let binding = dynamic_resource(key, anchor, self.scope.variant(), self.priority);
        let _ = object.bind_binding(property, &binding);
    }

    #[inline(never)]
    fn static_resource(&self, object: &FerroObject, property: &'static FerroProperty, key: &str) -> Result {
        let target = ValueType::new(property.property_type(), property.property_type_name());
        let variant = self.styled.as_ref().and_then(|element| element.actual_theme_variant()).or_else(|| self.scope.variant());
        let value = self.scope.static_resource_for(key, Some(target), variant.as_ref())?;
        assign(object, property, value, self.priority)
    }

    #[inline(never)]
    fn bind(&self, object: &FerroObject, property: &'static FerroProperty, binding: Rc<dyn BindingBase>) {
        let anchor = match self.styled {
            Some(_) => None,
            None => self.anchor.as_ref(),
        };
        let _ = object.bind_binding_with_anchor(property, &*binding, anchor);
    }

    #[inline(never)]
    fn name(&self, name: &str, value: BoxedValue) -> Result {
        if let Some(element) = &self.styled {
            element.set_direct_value(StyledElement::name_property(), Some(String::from(name)));
        }
        if let Some(names) = self.names {
            names
                .try_register_value(name, value)
                .map_err(|error| XamlLoadError::with_message(error.to_string()))?;
        }
        Ok(())
    }

    #[inline(never)]
    fn template_bind_with(
        &self,
        object: &FerroObject,
        property: &'static FerroProperty,
        source: Option<&'static FerroProperty>,
        converter: Option<Rc<dyn IValueConverter>>,
        mode: Option<BindingMode>,
        parameter: Option<BoxedValue>,
    ) {
        let binding = TemplateBinding::default();
        binding.set_property(source);
        binding.set_converter(converter);
        if let Some(mode) = mode {
            binding.set_mode(mode);
        }
        binding.set_converter_parameter(parameter);
        let _ = object.bind_binding(property, &binding);
    }

    #[inline(never)]
    fn style(&self, selector: Selector, populate: &mut dyn FnMut(&StyleBuilder) -> Result) -> Result {
        let nested = Style::new();
        nested.set_selector(Some(selector));
        let style = nested.clone().upcast::<StyleBase>();
        let builder = StyleBuilder { scope: self.scope.with_style(&style), style, theme: None };
        populate(&builder)?;
        if let Some(element) = &self.styled {
            element.styles().add(nested);
        }
        Ok(())
    }

    #[inline(never)]
    fn class(&self, name: &str) {
        if let Some(element) = &self.styled {
            element.classes().add(name);
        }
    }

    #[inline(never)]
    fn children(&self, object: &FerroObject, children: &mut dyn Iterator<Item = Ref<Control>>) {
        if let Some(panel) = object.to_ref().cast::<Panel>() {
            let collection = panel.children();
            for child in children {
                collection.add(child);
            }
        }
    }

    #[inline(never)]
    fn end(&self) {
        if let Some(element) = &self.styled {
            if let Some(elements) = self.elements {
                elements.borrow_mut().pop();
            }
            element.end_init();
        }
    }
}

impl<'a, E: ObjectType + Upcast<FerroObject>> Element<'a, E> {
    /// Starts `object`, of an element made outside a template (a value of a setter or a
    /// resource): its values are local values.
    pub fn local(object: Ref<E>, scope: &'a Scope) -> Self {
        Self::start_in(object, scope, None, BindingPriority::LocalValue, None)
    }

    #[inline]
    fn start_in(
        object: Ref<E>,
        scope: &'a Scope,
        names: Option<&'a NameScopeRef>,
        priority: BindingPriority,
        elements: Option<&'a RefCell<Vec<Ref<FerroObject>>>>,
    ) -> Self {
        let core = ElementCore::start(object.upcast_ref::<FerroObject>(), scope, names, priority, elements);
        Self { object, core }
    }

    #[inline]
    fn as_object(&self) -> &FerroObject {
        self.object.upcast_ref::<FerroObject>()
    }

    /// Starts the element `object` as a child of this one (with its scope, names and priority).
    #[inline]
    pub fn make<C: ObjectType + Upcast<FerroObject>>(&self, object: Ref<C>) -> Element<'a, C> {
        let core = self.core.child(object.upcast_ref::<FerroObject>());
        Element { object, core }
    }

    /// Sets `property` to `value`.
    #[inline]
    pub fn set<P: ThemeProperty>(self, property: &'static P, value: P::Value) -> Self {
        property.set_on(self.as_object(), value, self.core.priority);
        self
    }

    /// Sets `property` to the markup value `value`.
    #[inline]
    pub fn set_value(self, property: &'static impl AnyProperty, value: MarkupValue) -> Result<Self> {
        assign(self.as_object(), property.property(), value, self.core.priority)?;
        Ok(self)
    }

    /// Binds `property` to the property `source` of the templated parent (`{TemplateBinding}`).
    #[inline]
    pub fn template_bind(self, property: &'static impl AnyProperty, source: &'static impl AnyProperty) -> Self {
        self.core.template_bind(self.as_object(), property.property(), source.property());
        self
    }

    /// Binds `property` to the dynamic resource `key` (`{DynamicResource key}`).
    #[inline]
    pub fn dynamic(self, property: &'static impl AnyProperty, key: &str) -> Self {
        self.core.dynamic(self.as_object(), property.property(), key);
        self
    }

    /// Sets `property` to the static resource `key` (`{StaticResource key}`), looked up for the
    /// theme variant of the element.
    #[inline]
    pub fn static_resource(self, property: &'static impl AnyProperty, key: &str) -> Result<Self> {
        self.core.static_resource(self.as_object(), property.property(), key)?;
        Ok(self)
    }

    /// Binds `property` to `binding`.
    #[inline]
    pub fn bind(self, property: &'static impl AnyProperty, binding: Rc<dyn BindingBase>) -> Self {
        self.core.bind(self.as_object(), property.property(), binding);
        self
    }

    /// Names the element (`x:Name`) and registers it in the name scope of the template.
    #[inline]
    pub fn name(self, name: &str) -> Result<Self> {
        self.core.name(name, Rc::new(self.object.clone()))?;
        Ok(self)
    }

    /// Binds `property` to a property of the templated parent with a converter, a mode or a
    /// converter parameter (`{TemplateBinding source, Converter=..}`); `source` `None` binds to
    /// the templated parent itself (`{TemplateBinding}`).
    #[inline]
    pub fn template_bind_with(
        self,
        property: &'static impl AnyProperty,
        source: Option<&'static FerroProperty>,
        converter: Option<Rc<dyn IValueConverter>>,
        mode: Option<BindingMode>,
        parameter: Option<BoxedValue>,
    ) -> Self {
        self.core.template_bind_with(self.as_object(), property.property(), source, converter, mode, parameter);
        self
    }

    /// Sets `property` to `child`, an element (`Border.Child`, `Popup.Child`).
    #[inline]
    pub fn child<P, U, C>(self, property: &'static P, child: Element<'_, C>) -> Self
    where
        P: ThemeProperty<Value = Option<Ref<U>>>,
        U: ObjectType,
        C: ObjectType + Upcast<FerroObject> + Upcast<U>,
    {
        let child = child.end().upcast::<U>();
        self.set(property, Some(child))
    }

    /// Sets `property`, of any value, to `child`, an element (`ContentControl.Content`).
    #[inline]
    pub fn content<C: ObjectType + Upcast<FerroObject>>(self, property: &'static impl AnyProperty, child: Element<'_, C>) -> Result<Self> {
        let child = child.end();
        self.set_value(property, value(child))
    }

    /// A style of the element (`<Border.Styles>`), of `selector`, with the setters and styles
    /// `populate` adds.
    #[inline]
    pub fn style(self, selector: Selector, populate: impl FnOnce(&StyleBuilder) -> Result) -> Result<Self>
    where
        E: Upcast<StyledElement>,
    {
        let mut populate = Some(populate);
        self.core.style(selector, &mut |builder| (populate.take().expect("populated once"))(builder))?;
        Ok(self)
    }

    /// Runs `f` with the element (an addition to a collection of it, a call of a method).
    #[inline]
    pub fn with(self, f: impl FnOnce(&Ref<E>) -> Result) -> Result<Self> {
        f(&self.object)?;
        Ok(self)
    }

    /// Adds the style class `name`.
    #[inline]
    pub fn class(self, name: &str) -> Self
    where
        E: Upcast<StyledElement>,
    {
        self.core.class(name);
        self
    }

    /// Adds `children` to the children of the panel.
    #[inline]
    pub fn children<const N: usize>(self, children: [Ref<Control>; N]) -> Self
    where
        E: Upcast<Panel>,
    {
        self.core.children(self.as_object(), &mut children.into_iter());
        self
    }

    /// Ends the initialisation of the element and returns it.
    #[inline]
    pub fn end(self) -> Ref<E> {
        self.core.end();
        self.object
    }

    /// [`end`](Self::end) as a control.
    #[inline]
    pub fn control(self) -> Ref<Control>
    where
        E: Upcast<Control>,
    {
        self.end().upcast()
    }
}

/// Starts `object` as an element made outside a template, in `scope` (the value of a setter or
/// a resource).
pub fn element<E: ObjectType + Upcast<FerroObject>>(object: Ref<E>, scope: &Scope) -> Element<'_, E> {
    Element::local(object, scope)
}

/// The type key of the class `T` (`x:Key="{x:Type T}"`).
pub fn type_key<T: ObjectType>() -> ResourceKey {
    ResourceKey::Type(T::TYPE)
}

#[doc(hidden)]
pub fn __unused(_: &TypeInfo, _: &dyn IServiceProvider, _: &Rc<Setter>, _: &Rc<dyn SetterBase>) {}
