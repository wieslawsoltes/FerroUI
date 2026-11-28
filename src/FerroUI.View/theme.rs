//! Control themes, control templates and resource dictionaries in Rust, with no XAML. They make
//! the runtime objects compiled XAML makes (`ControlTheme`, `Setter`, `ResourceDictionary`).
//!
//! ```ignore
//! let chip = control_theme::<ToggleButton>()
//!     .set(TemplatedControl::padding_property(), (12.0, 4.0))
//!     .dynamic_resource(TemplatedControl::background_property(), keys::SystemControlBackgroundAltHighBrush)
//!     .template(|| {
//!         border()
//!             .name("PART_Border")
//!             .template_bind(Border::background_property(), TemplatedControl::background_property())
//!             .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
//!             .child(
//!                 content_presenter()
//!                     .name("PART_ContentPresenter")
//!                     .template_bind(ContentPresenter::content_property(), ContentControl::content_property()),
//!             )
//!             .into_element()
//!     })
//!     .nested(
//!         style(nesting().checked().template().of::<Border>())
//!             .dynamic_resource(Border::background_property(), keys::SystemControlHighlightAccentBrush),
//!     )
//!     .build();
//! ```
//!
//! A template body is a plain `fn`: a closure that captures nothing coerces to it, so there is no
//! generic copy per closure. A template that must capture uses [`control_template_with`], a thin
//! entry that boxes the closure and calls the one function that builds templates.
//!
//! The values an element of a template gives are at the priority of a template, as in XAML: a
//! style with a condition (`^:pointerover /template/ Border`) overrides them. A reactive value
//! (a signal, a closure) of an element of a template stays a local value and is not overridden.

use crate::builder::{IntoValue, TypedProperty};
use crate::element::{Element, PropValue};
use crate::reactive::Owner;
use crate::resources::{resource_binding, ResourceKeyFor};
use crate::style::StyleBuilder;
use ferroui_base::controls::{NameScopeRef, ResourceDictionary, ResourceKey, ThemeResourceKey};
use ferroui_base::data::{BindingBase, BindingPriority};
use ferroui_base::styling::{ControlTheme, Setter, ThemeVariant};
use ferroui_base::{
    ferro_property, AttachedProperty, BoxedValue, FerroObject, FerroProperty, ObjectType, Ref, StyledElement, TypeInfo,
    Upcast,
};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{FuncControlTemplate, IControlTemplate};
use ferroui_controls::Control;
use std::rc::Rc;

/// Disposes the owner of a built template when the control lets the template go.
struct TemplateOwner(Owner);

impl Drop for TemplateOwner {
    fn drop(&mut self) {
        self.0.try_dispose();
    }
}

/// The owner of the template a control has, as a value of the control.
#[derive(Clone)]
pub(crate) struct HeldTemplateOwner(Rc<TemplateOwner>);

impl PartialEq for HeldTemplateOwner {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

ferroui_base::styled_value!(HeldTemplateOwner);

/// Where a control keeps the owner of the template built for it. The owner is held by the
/// templated control and not by the root of the template, which the owner holds.
pub(crate) struct TemplateOwners;

ferroui_base::ferro_static_type!(TemplateOwners);

ferroui_base::ferro_properties! { impl TemplateOwners {
    ferro_property!(
        /// The owner of the template built for the control.
        pub fn owner_property() -> AttachedProperty<Option<HeldTemplateOwner>> {
            FerroProperty::register_attached::<TemplateOwners, TemplatedControl, _>("Owner", None)
        }
    );
} }

impl TemplateOwners {
    fn static_constructor() {}
}

/// Gives the values of `element` and of what is below it the priority of a template. A component
/// below it keeps its own values: it runs again and patches them.
fn at_template_priority(element: &mut Element) {
    match element {
        Element::Control(control) => {
            for entry in &mut control.props {
                let value = match &entry.value {
                    PropValue::Value(value) => PropValue::TemplateValue(value.clone()),
                    PropValue::Binding(binding) => PropValue::TemplateBinding(binding.clone()),
                    PropValue::Resource { key, dynamic, convert, .. } => PropValue::Resource {
                        key: key.clone(),
                        dynamic: *dynamic,
                        convert: *convert,
                        priority: BindingPriority::Template,
                    },
                    PropValue::Element(held) | PropValue::TypedElement { element: held, .. } => {
                        if let Some(inner) = held.borrow_mut().as_mut() {
                            at_template_priority(inner);
                        }
                        continue;
                    }
                    _ => continue,
                };
                entry.value = value;
            }
            control.children.iter_mut().for_each(at_template_priority);
        }
        Element::Fragment(items) | Element::List(items) => items.iter_mut().for_each(at_template_priority),
        _ => {}
    }
}

/// Builds a template for `parent`: mounts its element, registers its named parts, and leaves the
/// owner of what it started with the control.
fn build_template(view: &dyn Fn() -> Element, parent: &Ref<TemplatedControl>, scope: &NameScopeRef) -> Ref<Control> {
    let mut element = view();
    at_template_priority(&mut element);
    let (root, owner) = element.mount_root();
    let root_element: Ref<StyledElement> = root.clone().upcast();
    for part in root_element.get_self_and_logical_descendants() {
        if let Some(name) = part.name() {
            scope.register(&name, part.upcast::<FerroObject>());
        }
    }
    parent.set_value(TemplateOwners::owner_property(), Some(HeldTemplateOwner(Rc::new(TemplateOwner(owner)))));
    root
}

fn boxed_template(view: Rc<dyn Fn() -> Element>) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(move |parent, scope| build_template(&*view, parent, scope))
}

/// A control template written with builders. Elements named `PART_..` are its template parts.
/// `view` is a plain function: a closure that captures nothing coerces to it.
pub fn control_template(view: fn() -> Element) -> Rc<dyn IControlTemplate> {
    boxed_template(Rc::new(view))
}

/// A control template whose body captures values. Only boxes the closure: what builds the
/// template is one function for every closure.
pub fn control_template_with(view: impl Fn() -> Element + 'static) -> Rc<dyn IControlTemplate> {
    boxed_template(Rc::new(view))
}

/// A control theme being built ([`control_theme`]).
pub struct ControlThemeBuilder {
    target: &'static TypeInfo,
    based_on: Option<Ref<ControlTheme>>,
    setters: Vec<Rc<Setter>>,
    nested: Vec<StyleBuilder>,
}

/// A control theme of the class `T`.
pub fn control_theme<T: ObjectType + Upcast<StyledElement>>() -> ControlThemeBuilder {
    control_theme_of(T::TYPE)
}

/// A control theme of the class `target`.
pub fn control_theme_of(target: &'static TypeInfo) -> ControlThemeBuilder {
    ControlThemeBuilder { target, based_on: None, setters: Vec::new(), nested: Vec::new() }
}

impl ControlThemeBuilder {
    /// The theme this one is based on (`BasedOn`).
    pub fn based_on(mut self, theme: Ref<ControlTheme>) -> Self {
        self.based_on = Some(theme);
        self
    }

    /// Sets `property` to `value`, as [`StyleBuilder::set`] does.
    pub fn set<P: TypedProperty + ?Sized>(mut self, property: &'static P, value: impl IntoValue<P::Value>) -> Self {
        self.setters.push(Setter::new_untyped(property.untyped(), Rc::new(value.into_value())));
        self
    }

    /// Makes `property` follow a resource (`{DynamicResource}` in a setter).
    pub fn dynamic_resource<P, K>(mut self, property: &'static P, key: K) -> Self
    where
        P: TypedProperty + ?Sized,
        K: ResourceKeyFor<P::Value>,
    {
        let binding: Rc<dyn BindingBase> = Rc::new(resource_binding::<P::Value, K>(key, true));
        self.setters.push(Setter::new_binding_base(property.untyped(), binding));
        self
    }

    /// Binds `property` with a binding of the property system.
    pub fn bind<P: TypedProperty + ?Sized>(mut self, property: &'static P, binding: Rc<dyn BindingBase>) -> Self {
        self.setters.push(Setter::new_binding_base(property.untyped(), binding));
        self
    }

    /// The template of the control, written with builders. See [`control_template`].
    pub fn template(self, view: fn() -> Element) -> Self {
        self.template_of(control_template(view))
    }

    /// The template of the control, made elsewhere ([`control_template_with`]).
    pub fn template_of(mut self, template: Rc<dyn IControlTemplate>) -> Self {
        let property = TemplatedControl::template_property().untyped();
        self.setters.push(Setter::new_untyped(property, Rc::new(Some(template))));
        self
    }

    /// Adds a nested style, whose selector starts with `nesting()`.
    pub fn nested(mut self, style: StyleBuilder) -> Self {
        self.nested.push(style);
        self
    }

    /// The control theme.
    pub fn build(self) -> Ref<ControlTheme> {
        let theme = ControlTheme::with_setters(self.target, self.setters);
        theme.set_based_on(self.based_on);
        for nested in self.nested {
            theme.children().add(nested.build());
        }
        theme
    }
}

/// A resource dictionary being built ([`resource_dictionary`]).
pub struct ResourceDictionaryBuilder {
    dictionary: Ref<ResourceDictionary>,
}

/// A resource dictionary, with values, control themes and theme variants.
///
/// ```ignore
/// resource_dictionary()
///     .value("CardPadding", Thickness::uniform(12.0))
///     .theme_variant(ThemeVariant::light(), resource_dictionary().value("Accent", rgb(0x0063B1)))
///     .theme_variant(ThemeVariant::dark(), resource_dictionary().value("Accent", rgb(0x60CDFF)))
///     .control_theme(chip)
///     .build()
/// ```
pub fn resource_dictionary() -> ResourceDictionaryBuilder {
    ResourceDictionaryBuilder { dictionary: ResourceDictionary::new() }
}

impl ResourceDictionaryBuilder {
    /// Adds a value under `key`.
    pub fn value<T: PartialEq + 'static>(self, key: impl Into<ResourceKey>, value: T) -> Self {
        self.dictionary.add_value(key, value);
        self
    }

    /// Adds a value under a typed key, of the type of the key.
    pub fn typed<T: PartialEq + 'static>(self, key: ThemeResourceKey<T>, value: T) -> Self {
        self.dictionary.add_value(key, value);
        self
    }

    /// Adds a value made the first time it is asked for. `factory` is a plain function: a closure
    /// that captures nothing coerces to it.
    pub fn deferred(self, key: impl Into<ResourceKey>, factory: fn() -> BoxedValue) -> Self {
        self.dictionary.add_deferred_fn(key, move |_| Some(factory()));
        self
    }

    /// Adds a control theme under the class it targets, so that controls of the class find it.
    ///
    /// # Panics
    /// When the theme has no target type.
    pub fn control_theme(self, theme: Ref<ControlTheme>) -> Self {
        let target = theme.target_type().expect("a control theme has a target type");
        self.dictionary.set(ResourceKey::Type(target), Some(Rc::new(theme) as BoxedValue));
        self
    }

    /// The resources of a theme variant.
    pub fn theme_variant(self, variant: ThemeVariant, resources: ResourceDictionaryBuilder) -> Self {
        self.dictionary.add_theme_dictionary(variant, resources.dictionary);
        self
    }

    /// Merges another dictionary.
    pub fn merged(self, resources: ResourceDictionaryBuilder) -> Self {
        self.dictionary.add_merged_dictionary(resources.dictionary);
        self
    }

    /// The dictionary.
    pub fn build(self) -> Ref<ResourceDictionary> {
        self.dictionary
    }
}
