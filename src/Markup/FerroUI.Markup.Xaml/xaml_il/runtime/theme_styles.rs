//! The tables of control themes (#568, step 5 of docs/porting/theme-trimming.md): the
//! interpreters that generated code calls, through `rt`, for a control theme, a selector chain,
//! the run-time choice of the setter of a resource value and a run of template bindings.
//!
//! Not a port of an upstream file. A compiled theme builds each control theme once, on the first
//! lookup of its key, with hundreds of statements that differ only in their operands: a style
//! created, a selector chain built step by step, a setter added with a dynamic resource, a
//! constant, a static resource or a template. Written as a declarative table, a control theme is
//! data that one non-generic function interprets, making the calls the statements made, in the
//! same order, with the same errors at the same positions:
//!
//! | Generated code | Table | Interpreter |
//! |---|---|---|
//! | the factory of a `ControlTheme` | [`ControlThemeTable`] of [`StyleMember`]s | [`build_control_theme`] |
//! | a selector chain (`Selectors::nesting(..)`, `class`, `template`, ..) | a static slice of [`SelectorStep`]s | [`build_selector`] |
//! | the `is_instance` chain of a `{StaticResource}` value | [`ResourceChoice`] | [`assign_resource_value`] |
//! | consecutive `rt::bind_template_binding` on one object | [`TemplateBindingRow`]s | [`bind_template_bindings`] |
//!
//! What a table cannot hold as data stays code of the factory: the definition of a registered
//! property is a function ([`PropertyFn`], one per property and generated file), and a setter
//! value that is neither a constant, a resource reference nor a control template is a function
//! of the factory ([`ThemeValue::Expression`], [`ThemeValue::Computed`]) with the statements
//! generated code wrote for it. Templates stay native code: a template setter names the build
//! function of its content.

use std::rc::Rc;

use ferroui_base::data::core::ValueType;
use ferroui_base::data::{BindingBase, BindingPriority};
use ferroui_base::metadata::{MarkupType, MarkupTyped, MarkupValue};
use ferroui_base::styling::{ControlTheme, Selector, Selectors, Setter, SetterBase, Style, StyleBase};
use ferroui_base::{
    BoxedValue, CornerRadius, FerroObject, FerroProperty, PropertyValue, Ref, StaticType, StyledProperty, Thickness, TypeInfo,
    UnsetValueType,
};

use super::compiled::{
    add_dynamic_resource_setter_with_parent, add_setter, add_setter_value, add_setter_value_with_parent, bind,
    bind_template_binding, boxed, cast, cast_checked, class_handle, class_markup, clr_property_info, defer, exact, is_instance,
    markup_handle, new_setter, new_setter_with_parent, no_setter, provide_value_invoked, setter_value_property, string_object,
    to_object, to_value, unset_value, DeferredBuild,
};
use super::XamlIlContext;
use crate::markup_extensions::StaticResourceExtension;
use crate::templates::ControlTemplate;
use crate::XamlLoadError;

/// The definition of a registered property, as generated code names it
/// (`rt::property(Button::background_property())`, `rt::class_property("pressed")`): one
/// function per property and generated file, shared by its tables.
pub type PropertyFn = fn() -> &'static FerroProperty;

/// A setter value with statements of its own: the statements generated code wrote for the
/// value, in a function of the factory, called with the context of the build.
pub type ThemeValueFn = fn(&Rc<XamlIlContext>) -> Result<MarkupValue, XamlLoadError>;

// --- selectors ----------------------------------------------------------------------------

/// One step of a selector chain: the call of `Selectors` that generated code wrote for it, with
/// the selector so far as its previous selector.
#[derive(Clone, Copy)]
pub enum SelectorStep {
    /// `Selectors::nesting(previous)` (`^`).
    Nesting,
    /// `Selectors::class(previous, name)` (`.name`, `:pseudo-class`).
    Class(&'static str),
    /// `Selectors::name(previous, name)` (`#name`).
    Name(&'static str),
    /// `Selectors::template(previous)` (`/template/`).
    Template,
    /// `Selectors::child(previous)` (`>`).
    Child,
    /// `Selectors::descendant(previous)` (a space).
    Descendant,
    /// `Selectors::of_type_info(previous, type)`: exactly the type.
    OfType(&'static TypeInfo),
    /// `Selectors::is_type_info(previous, type)` (`:is(Type)`): the type or a derived one.
    Is(&'static TypeInfo),
    /// `Selectors::nth_child(previous, step, offset)`.
    NthChild(i32, i32),
    /// `Selectors::nth_last_child(previous, step, offset)`.
    NthLastChild(i32, i32),
    /// `Selectors::not(previous, argument)`, the argument built after the selector so far.
    Not { argument: &'static [SelectorStep] },
    /// `Selectors::or([..])` of the alternatives, built in order. It starts a chain: generated
    /// code calls it without a previous selector.
    Or { alternatives: &'static [&'static [SelectorStep]] },
    /// `Selectors::property_equals_untyped(previous, property, value)`, the property before the
    /// value.
    PropertyEquals { property: PropertyFn, value: fn() -> BoxedValue },
}

/// The selector of the chain `steps`: each step applied to the selector the steps before it
/// built, in order; `None` for an empty chain (generated code writes no empty chain).
#[inline(never)]
pub fn build_selector(steps: &[SelectorStep]) -> Option<Selector> {
    let mut current: Option<Selector> = None;
    for step in steps {
        let next = match *step {
            SelectorStep::Nesting => Selectors::nesting(current.take()),
            SelectorStep::Class(name) => Selectors::class(current.take(), name),
            SelectorStep::Name(name) => Selectors::name(current.take(), name),
            SelectorStep::Template => Selectors::template(current.take()),
            SelectorStep::Child => Selectors::child(current.take()),
            SelectorStep::Descendant => Selectors::descendant(current.take()),
            SelectorStep::OfType(type_) => Selectors::of_type_info(current.take(), type_),
            SelectorStep::Is(type_) => Selectors::is_type_info(current.take(), type_),
            SelectorStep::NthChild(step, offset) => Selectors::nth_child(current.take(), step, offset),
            SelectorStep::NthLastChild(step, offset) => Selectors::nth_last_child(current.take(), step, offset),
            SelectorStep::Not { argument } => {
                let Some(argument) = build_selector(argument) else { continue };
                Selectors::not(current.take(), argument)
            }
            SelectorStep::Or { alternatives } => {
                Selectors::or(alternatives.iter().filter_map(|alternative| build_selector(alternative)).collect::<Vec<_>>())
            }
            SelectorStep::PropertyEquals { property, value } => {
                let property = property();
                Selectors::property_equals_untyped(current.take(), property, value())
            }
        };
        current = Some(next);
    }
    current
}

// --- control themes -----------------------------------------------------------------------

/// The factory of a control theme as data: whether the theme is on the parent stack of the
/// context while its members are added (pushed after it is created, popped before it is
/// returned), and its members in the order of the document.
pub struct ControlThemeTable {
    pub parent: bool,
    pub members: &'static [StyleMember],
}

/// The key of the `{StaticResource}` of `BasedOn`: a text, or a type (`{x:Type Button}`).
#[derive(Clone, Copy)]
pub enum ThemeKey {
    Text(&'static str),
    Type(&'static TypeInfo),
}

/// A member of a control theme or of a style nested in one, in the order of the document.
pub enum StyleMember {
    /// `ControlTheme.TargetType`; of the control theme only.
    TargetType(&'static TypeInfo),
    /// `ControlTheme.BasedOn` as `{StaticResource key}` at `line`, `position`; of the control
    /// theme only.
    BasedOn { key: ThemeKey, line: i32, position: i32 },
    /// A setter.
    Setter(ThemeSetter),
    /// A nested style: created, on the parent stack while its members are added when `parent`
    /// is set, its selector set, its members added, then added to the style around it (cast at
    /// `line`, `position`).
    Style {
        parent: bool,
        selector: Option<&'static [SelectorStep]>,
        members: &'static [StyleMember],
        line: i32,
        position: i32,
    },
}

/// How generated code made a setter, which decides when the setter is created and whether it is
/// on the parent stack while its value is made.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SetterForm {
    /// `rt::add_setter(style, property, value)`: the value is made first.
    Plain,
    /// `rt::new_setter(property)`, the value, then `rt::add_setter_value`.
    Created,
    /// `rt::new_setter_with_parent(context, property)`, the value, then
    /// `rt::add_setter_value_with_parent`: the setter is on the parent stack while its value is
    /// made.
    Parented,
}

/// A setter of a style: its property, how it is made and its value.
#[derive(Clone, Copy)]
pub struct ThemeSetter {
    pub property: PropertyFn,
    pub form: SetterForm,
    pub value: ThemeValue,
}

/// A control template as the value of a setter: `ControlTemplate`, on the parent stack while its
/// members are set when `parent` is set, its `TargetType` when the document states one, and its
/// content, the deferred content of `build` with the result type `content_type`, created at
/// `line`, `position`.
#[derive(Clone, Copy)]
pub struct ThemeTemplate {
    pub target_type: Option<&'static TypeInfo>,
    pub content_type: &'static TypeInfo,
    pub build: DeferredBuild,
    pub parent: bool,
    pub line: i32,
    pub position: i32,
}

/// The value of a setter.
#[derive(Clone, Copy)]
pub enum ThemeValue {
    /// `{DynamicResource key}` (`rt::add_dynamic_resource_setter_with_parent`, whatever the
    /// form).
    Dynamic(&'static str),
    /// `{StaticResource key}`, provided with `Setter.Value` as the target property at `line`,
    /// `position`.
    Static { key: &'static str, line: i32, position: i32 },
    /// A control template.
    Template(ThemeTemplate),
    /// `{x:Null}`.
    Null,
    Bool(bool),
    Int(i32),
    Double(f64),
    Text(&'static str),
    /// `Thickness(left, top, right, bottom)`.
    Thickness([f64; 4]),
    /// `CornerRadius(top_left, top_right, bottom_right, bottom_left)`.
    CornerRadius([f64; 4]),
    /// An expression of generated code that needs no context and cannot fail (an enumeration
    /// member, a constructor of literals).
    Expression(fn() -> MarkupValue),
    /// The statements of generated code for the value, then the value.
    Computed(ThemeValueFn),
    /// No value: `new Setter { Property = .. }` added as it is.
    Absent,
}

/// Builds the control theme `table` with `context`, the context of the build of its deferred
/// content: the calls the factory of generated code made, in the same order. The first failure
/// is the error, with the theme left on the parent stack of that context, as there.
#[inline(never)]
pub fn build_control_theme(context: &Rc<XamlIlContext>, table: &ControlThemeTable) -> Result<MarkupValue, XamlLoadError> {
    let theme = ControlTheme::new();
    if table.parent {
        context.push_parent(to_value(theme.clone()));
    }
    for member in table.members {
        match member {
            StyleMember::TargetType(type_) => {
                ControlTheme::__markup_set_TargetType(&theme, Some(*type_));
            }
            StyleMember::BasedOn { key, line, position } => based_on(context, &theme, *key, *line, *position)?,
            member => add_member(context, theme.upcast_ref::<StyleBase>(), member)?,
        }
    }
    if table.parent {
        context.pop_parent();
    }
    Ok(to_value(theme.clone()))
}

/// `BasedOn="{StaticResource key}"`: the extension created with the key, `ProvideValue` with
/// `ControlTheme.BasedOn` as the provide-value target property, then the provided value cast to
/// a control theme and assigned, all at `line`, `position`, with the errors of the statements of
/// generated code.
fn based_on(context: &Rc<XamlIlContext>, theme: &Ref<ControlTheme>, key: ThemeKey, line: i32, position: i32) -> Result<(), XamlLoadError> {
    let key = match key {
        ThemeKey::Text(text) => string_object(text),
        ThemeKey::Type(type_) => boxed(type_),
    };
    let extension = StaticResourceExtension::__markup_new_1(key);
    let theme_type = <ControlTheme as StaticType>::TYPE;
    let provided = provide_value_invoked(
        context,
        boxed(clr_property_info(class_markup(theme_type), "BasedOn", false, class_handle(theme_type), false)),
        &extension,
        StaticResourceExtension::__markup_ProvideValue_0,
        line,
        position,
    )?;
    ControlTheme::__markup_set_BasedOn(
        theme,
        exact(
            cast_checked(provided, class_handle(theme_type), "FerroUI.Styling.ControlTheme", line, position)?,
            "FerroUI.Styling.ControlTheme.set_BasedOn",
            0,
            line,
            position,
        )?,
    );
    Ok(())
}

/// Adds the setter or the nested style `member` to `style`.
fn add_member(context: &Rc<XamlIlContext>, style: &Ref<StyleBase>, member: &StyleMember) -> Result<(), XamlLoadError> {
    match member {
        StyleMember::Setter(setter) => add_theme_setter(context, style, setter),
        StyleMember::Style { parent, selector, members, line, position } => {
            let nested = Style::new();
            if *parent {
                context.push_parent(to_value(nested.clone()));
            }
            if let Some(steps) = selector {
                Style::__markup_set_Selector(&nested, build_selector(steps));
            }
            for member in members.iter() {
                add_member(context, nested.upcast_ref::<StyleBase>(), member)?;
            }
            if *parent {
                context.pop_parent();
            }
            StyleBase::__markup_Add_1(style, cast(nested.clone(), *line, *position)?);
            Ok(())
        }
        // The members of the control theme itself; the emitter writes them in no nested style.
        StyleMember::TargetType(_) | StyleMember::BasedOn { .. } => Ok(()),
    }
}

/// Adds the setter `setter` to `style`, as the statements of generated code for its form made it.
fn add_theme_setter(context: &Rc<XamlIlContext>, style: &Ref<StyleBase>, setter: &ThemeSetter) -> Result<(), XamlLoadError> {
    match setter.value {
        ThemeValue::Dynamic(key) => {
            add_dynamic_resource_setter_with_parent(context, style, (setter.property)(), key);
            return Ok(());
        }
        ThemeValue::Absent => {
            let created = Setter::__markup_new_0();
            Setter::__markup_set_Property(&created, Some((setter.property)()));
            StyleBase::__markup_Add_0(style, created.clone() as Rc<dyn SetterBase>);
            return Ok(());
        }
        _ => {}
    }
    match setter.form {
        SetterForm::Plain => {
            let property = (setter.property)();
            let value = theme_value(context, &setter.value)?;
            add_setter(style, property, value);
        }
        SetterForm::Created => {
            let created = new_setter((setter.property)());
            let value = theme_value(context, &setter.value)?;
            add_setter_value(style, &created, value);
        }
        SetterForm::Parented => {
            let created = new_setter_with_parent(context, (setter.property)());
            let value = theme_value(context, &setter.value)?;
            add_setter_value_with_parent(context, style, &created, value);
        }
    }
    Ok(())
}

/// The value `value` of a setter, made with `context`.
fn theme_value(context: &Rc<XamlIlContext>, value: &ThemeValue) -> Result<MarkupValue, XamlLoadError> {
    Ok(match *value {
        ThemeValue::Static { key, line, position } => {
            let extension = StaticResourceExtension::__markup_new_1(string_object(key));
            let provided: MarkupValue = provide_value_invoked(
                context,
                setter_value_property(),
                &extension,
                StaticResourceExtension::__markup_ProvideValue_0,
                line,
                position,
            )?;
            provided
        }
        ThemeValue::Template(template) => theme_template(context, &template)?,
        ThemeValue::Null => None,
        ThemeValue::Bool(value) => to_object(value),
        ThemeValue::Int(value) => to_object(value),
        ThemeValue::Double(value) => to_object(value),
        ThemeValue::Text(value) => string_object(value),
        ThemeValue::Thickness([left, top, right, bottom]) => to_object(Thickness::__markup_new_3(left, top, right, bottom)),
        ThemeValue::CornerRadius([top_left, top_right, bottom_right, bottom_left]) => {
            to_object(CornerRadius::__markup_new_3(top_left, top_right, bottom_right, bottom_left))
        }
        ThemeValue::Expression(value) => value(),
        ThemeValue::Computed(value) => value(context)?,
        // Made by `add_theme_setter` itself.
        ThemeValue::Dynamic(_) | ThemeValue::Absent => None,
    })
}

/// A control template: created, on the parent stack while its members are set when the table
/// says so, its target type, then its content, the deferred content of its build function.
fn theme_template(context: &Rc<XamlIlContext>, template: &ThemeTemplate) -> Result<MarkupValue, XamlLoadError> {
    let created = ControlTemplate::__markup_new_0();
    if template.parent {
        context.push_parent(to_value(created.clone()));
    }
    if let Some(target_type) = template.target_type {
        ControlTemplate::__markup_set_TargetType(&created, Some(target_type));
    }
    let deferred = defer(class_handle(template.content_type), context, template.build, template.line, template.position)?;
    ControlTemplate::__markup_set_Content(&created, to_object(deferred.clone()));
    if template.parent {
        context.pop_parent();
    }
    Ok(to_object(created.clone()))
}

// --- the setter of a resource value -------------------------------------------------------

/// A type a value is checked against, as generated code names it: the handle of a class
/// (`rt::class_handle`), of a markup type (`rt::markup_handle`) or of a Rust type
/// (`ValueType::of`). The handle is made when the check is made.
#[derive(Clone, Copy)]
pub enum ValueHandle {
    Class(&'static TypeInfo),
    Markup(&'static MarkupType),
    Of(fn() -> ValueType),
}

impl ValueHandle {
    fn value_type(self) -> ValueType {
        match self {
            Self::Class(class) => class_handle(class),
            Self::Markup(markup) => markup_handle(markup),
            Self::Of(of) => of(),
        }
    }
}

/// The run-time choice of the setter of a registered property for a value that is known only
/// at run time (a `{StaticResource}`), as generated code wrote it: when `unset` is set, the unset
/// marker clears the property; a binding is bound; an instance of `instance`, or null when
/// `null` is set, is assigned through the typed setter (the conversion of the argument `index`
/// of `member`); anything else is the error of no setter of the property `name`. Every step is
/// at `line`, `position`.
#[derive(Clone, Copy)]
pub struct ResourceChoice {
    pub name: &'static str,
    pub member: &'static str,
    pub index: usize,
    pub unset: bool,
    pub instance: ValueHandle,
    pub null: bool,
    pub line: i32,
    pub position: i32,
}

/// The untyped part of [`assign_resource_value`]: the checks in the order of generated code,
/// the unset marker and the binding handled here. `Ok(true)` when the typed setter takes the
/// value.
#[inline(never)]
fn untyped_resource_choice(
    target: &Ref<FerroObject>,
    property: &'static FerroProperty,
    value: &MarkupValue,
    choice: &ResourceChoice,
) -> Result<bool, XamlLoadError> {
    if choice.unset && is_instance(value, markup_handle(<UnsetValueType as MarkupTyped>::MARKUP)) {
        unset_value(target, property);
        return Ok(false);
    }
    if is_instance(value, markup_handle(<dyn BindingBase as MarkupTyped>::MARKUP)) {
        bind(target, property, value.clone(), choice.line, choice.position)?;
        return Ok(false);
    }
    if is_instance(value, choice.instance.value_type()) || (choice.null && value.is_none()) {
        return Ok(true);
    }
    Err(no_setter(choice.name, value, choice.line, choice.position))
}

/// Assigns `value` to `property` of `target` through the setter `choice` picks
/// ([`ResourceChoice`]): what the `is_instance` chain of generated code did, with its order and
/// its errors. The typed setter sets the local value, or the value at `priority` when generated
/// code set it with a priority (in a template). The choice is one non-generic function; this one
/// is instantiated once per property value type.
pub fn assign_resource_value<T: PropertyValue>(
    target: &Ref<FerroObject>,
    property: &'static StyledProperty<T>,
    value: MarkupValue,
    choice: &ResourceChoice,
    priority: Option<BindingPriority>,
) -> Result<(), XamlLoadError> {
    if !untyped_resource_choice(target, property, &value, choice)? {
        return Ok(());
    }
    let typed: T = exact(value, choice.member, choice.index, choice.line, choice.position)?;
    match priority {
        None => target.set_value(property, typed),
        Some(priority) => {
            let _ = target.set_value_with_priority(property, typed, priority);
        }
    }
    Ok(())
}

// --- template bindings --------------------------------------------------------------------

/// `{TemplateBinding source}` as the value of the registered property `target` of an object, at
/// `line`, `position` ([`bind_template_binding`]).
#[derive(Clone, Copy)]
pub struct TemplateBindingRow {
    pub target: PropertyFn,
    pub source: PropertyFn,
    pub line: i32,
    pub position: i32,
}

/// Binds the template bindings `rows` on `target`, in order ([`bind_template_binding`] of each,
/// its target property before its source); the first failure is the error.
#[inline(never)]
pub fn bind_template_bindings(target: &Ref<FerroObject>, rows: &[TemplateBindingRow]) -> Result<(), XamlLoadError> {
    for row in rows {
        let property = (row.target)();
        let source = (row.source)();
        bind_template_binding(target, property, source, row.line, row.position)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "theme_styles_tests.rs"]
mod tests;
