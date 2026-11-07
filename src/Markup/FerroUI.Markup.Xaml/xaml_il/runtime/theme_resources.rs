//! The row tables of theme resources (#568, step 1 of docs/porting/theme-trimming.md): the
//! builders that generated code calls, through `rt`, for a run of resources of one shape added
//! to one dictionary.
//!
//! Not a port of an upstream file. A theme dictionary holds thousands of resources of a few
//! shapes. Written one by one, every deferred resource is a build function of its own and every
//! plain value a statement of its own. Written as rows, a run of additions is a static table and
//! one call of the builder of its shape: one non-generic function per shape serves every row of
//! every theme, and the build of a row makes the calls the build function of the resource made,
//! in the same order, with the same errors at the same positions.
//!
//! The deferred shapes (section 5.1 of the design note):
//!
//! | Shape | Row | Builder |
//! |---|---|---|
//! | `{StaticResource Key}` alone | [`ResourceAliasRow`] | [`add_resource_aliases`] |
//! | a `SolidColorBrush` with a literal colour | [`LiteralBrushRow`] | [`add_literal_brushes`] |
//! | a `SolidColorBrush` whose colour is `{DynamicResource Key}` | [`DynamicBrushRow`] | [`add_dynamic_brushes`] |
//! | a `SolidColorBrush` whose colour is `{StaticResource Key}` | [`StaticBrushRow`] | [`add_static_brushes`] |
//!
//! Each row is still added as deferred content of its own (one `DeferredContent` per row,
//! created with the context of the declaration as `rt::defer` creates it), built on first
//! lookup. The plain values (section 5.2) are [`ValueRow`]s: [`add_double_resources`],
//! [`add_thickness_resources`], [`add_corner_radius_resources`], [`add_color_resources`] and
//! [`add_string_resources`]. Every builder adds its rows in order and stops at the first error,
//! as the sequence of statements it replaces does.

use std::rc::Rc;

use ferroui_base::controls::{IDeferredContent, IResourceDictionary, ResourceDictionary};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingBase;
use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_base::metadata::{IServiceProvider, MarkupTyped, MarkupValue};
use ferroui_base::{CornerRadius, FerroObject, Ref, Thickness, UnsetValueType};

use super::compiled::{
    bind, bind_dynamic_resource, cast, deferred_content, deferred_context, exact, invoked, is_instance, markup_handle,
    no_setter, property_value, provide_value_invoked, static_resource_content, string_object, to_object, to_value,
    unset_value, DocumentInfo, XmlNamespaceTable,
};
use super::{DeferredContentBuilder, XamlIlContext};
use crate::markup_extensions::StaticResourceExtension;
use crate::XamlLoadError;

/// A resource that is `{StaticResource target}` alone (a resource that names another one),
/// added under `key`.
///
/// `line`, `position` are the position of the addition (where the deferred content is created
/// and added); `value_line`, `value_position` the position at which a failed lookup of the
/// target is reported. `namespaces` are the XML namespaces of the declaration when they are
/// not those of the document.
#[derive(Clone, Copy)]
pub struct ResourceAliasRow {
    pub key: &'static str,
    pub target: &'static str,
    pub namespaces: Option<XmlNamespaceTable>,
    pub line: i32,
    pub position: i32,
    pub value_line: i32,
    pub value_position: i32,
}

/// A `SolidColorBrush` whose colour is the literal `color` (`Color.FromUInt32`), added under
/// `key` at `line`, `position`.
#[derive(Clone, Copy)]
pub struct LiteralBrushRow {
    pub key: &'static str,
    pub color: u32,
    pub namespaces: Option<XmlNamespaceTable>,
    pub line: i32,
    pub position: i32,
}

/// A `SolidColorBrush` whose colour is `{DynamicResource color_key}`, added under `key` at
/// `line`, `position`; `value_line`, `value_position` are the position of the colour.
#[derive(Clone, Copy)]
pub struct DynamicBrushRow {
    pub key: &'static str,
    pub color_key: &'static str,
    pub namespaces: Option<XmlNamespaceTable>,
    pub line: i32,
    pub position: i32,
    pub value_line: i32,
    pub value_position: i32,
}

/// A `SolidColorBrush` whose colour is `{StaticResource color_key}`, added under `key` at
/// `line`, `position`; `value_line`, `value_position` are the position of the colour, where the
/// setter of the provided value is chosen at run time.
#[derive(Clone, Copy)]
pub struct StaticBrushRow {
    pub key: &'static str,
    pub color_key: &'static str,
    pub namespaces: Option<XmlNamespaceTable>,
    pub line: i32,
    pub position: i32,
    pub value_line: i32,
    pub value_position: i32,
}

/// A plain value added to a dictionary under `key` at `line`, `position`: a number (`f64`), the
/// four components of a `Thickness` or a `CornerRadius` (`[f64; 4]`), the ARGB value of a
/// `Color` (`u32`) or a text (`&'static str`).
#[derive(Clone, Copy)]
pub struct ValueRow<T: 'static> {
    pub key: &'static str,
    pub value: T,
    pub line: i32,
    pub position: i32,
}

/// The document a deferred row is built in: the base URI of `document` with the namespaces of
/// the declaration of the row, or those of the document when the row states none (the two
/// forms of `rt::DocumentInfo` a build function of generated code creates its context with).
fn row_document(document: &DocumentInfo, namespaces: Option<XmlNamespaceTable>) -> DocumentInfo {
    DocumentInfo { base_uri: document.base_uri, namespaces: namespaces.unwrap_or(document.namespaces) }
}

/// `rt::add_deferred_resource` of a row: the deferred content of `builder`, with the result
/// type `object` and the context of the declaration, then `ResourceDictionary.AddDeferred(key,
/// content)`; a failure of either is the load error at `line`, `position`.
#[inline(never)]
fn add_deferred_row(
    dictionary: &Ref<ResourceDictionary>,
    key: &str,
    context: &Rc<XamlIlContext>,
    builder: DeferredContentBuilder,
    line: i32,
    position: i32,
) -> Result<(), XamlLoadError> {
    let deferred = deferred_content(ValueType::object(), context, builder, line, position)?;
    invoked(
        ResourceDictionary::__markup_AddDeferred_1(dictionary, string_object(key), deferred as Rc<dyn IDeferredContent>),
        line,
        position,
    )
}

/// Adds the resource aliases `rows` to `dictionary`, in order, each as deferred content
/// created with `context` that builds `build_resource_alias`; the first failure is the
/// error.
#[inline(never)]
pub fn add_resource_aliases(
    dictionary: &Ref<ResourceDictionary>,
    context: &Rc<XamlIlContext>,
    document: &'static DocumentInfo,
    rows: &'static [ResourceAliasRow],
) -> Result<(), XamlLoadError> {
    for row in rows {
        let builder = DeferredContentBuilder::try_new(move |service_provider| build_resource_alias(service_provider, document, row));
        add_deferred_row(dictionary, row.key, context, builder, row.line, row.position)?;
    }
    Ok(())
}

/// The build of a resource alias: [`static_resource_content`] of the target in the document of
/// the row, the error of a failed lookup at the value position of the row.
fn build_resource_alias(
    service_provider: &Rc<dyn IServiceProvider>,
    document: &DocumentInfo,
    row: &ResourceAliasRow,
) -> Result<MarkupValue, XamlLoadError> {
    static_resource_content(service_provider, &row_document(document, row.namespaces), row.target, row.value_line, row.value_position)
}

/// Adds the brushes with a literal colour `rows` to `dictionary`, in order, each as deferred
/// content created with `context` that builds `build_literal_brush`; the first failure is the
/// error.
#[inline(never)]
pub fn add_literal_brushes(
    dictionary: &Ref<ResourceDictionary>,
    context: &Rc<XamlIlContext>,
    document: &'static DocumentInfo,
    rows: &'static [LiteralBrushRow],
) -> Result<(), XamlLoadError> {
    for row in rows {
        let builder = DeferredContentBuilder::try_new(move |service_provider| build_literal_brush(service_provider, document, row));
        add_deferred_row(dictionary, row.key, context, builder, row.line, row.position)?;
    }
    Ok(())
}

/// The build of a brush with a literal colour: the context of the build, the brush, the colour
/// (`Color.FromUInt32`) and its assignment, as the build function of generated code makes them.
fn build_literal_brush(
    service_provider: &Rc<dyn IServiceProvider>,
    document: &DocumentInfo,
    row: &LiteralBrushRow,
) -> Result<MarkupValue, XamlLoadError> {
    let _context = deferred_context(service_provider, &row_document(document, row.namespaces));
    let brush = SolidColorBrush::new();
    let color = Color::__markup_FromUInt32_2(row.color);
    brush.set_value(SolidColorBrush::color_property(), color);
    Ok(to_value(brush.clone()))
}

/// Adds the brushes bound to a dynamic resource `rows` to `dictionary`, in order, each as
/// deferred content created with `context` that builds `build_dynamic_brush`; the first
/// failure is the error.
#[inline(never)]
pub fn add_dynamic_brushes(
    dictionary: &Ref<ResourceDictionary>,
    context: &Rc<XamlIlContext>,
    document: &'static DocumentInfo,
    rows: &'static [DynamicBrushRow],
) -> Result<(), XamlLoadError> {
    for row in rows {
        let builder = DeferredContentBuilder::try_new(move |service_provider| build_dynamic_brush(service_provider, document, row));
        add_deferred_row(dictionary, row.key, context, builder, row.line, row.position)?;
    }
    Ok(())
}

/// The build of a brush whose colour is `{DynamicResource key}`: the context of the build, the
/// brush on the parent stack while its colour is bound ([`bind_dynamic_resource`] at the value
/// position of the row), as the build function of generated code makes them. A failed binding
/// returns with the brush still on the stack of that context, as there.
fn build_dynamic_brush(
    service_provider: &Rc<dyn IServiceProvider>,
    document: &DocumentInfo,
    row: &DynamicBrushRow,
) -> Result<MarkupValue, XamlLoadError> {
    let context = deferred_context(service_provider, &row_document(document, row.namespaces));
    let brush = SolidColorBrush::new();
    context.push_parent(to_value(brush.clone()));
    bind_dynamic_resource(
        &context,
        brush.upcast_ref::<FerroObject>(),
        SolidColorBrush::color_property(),
        row.color_key,
        row.value_line,
        row.value_position,
    )?;
    context.pop_parent();
    Ok(to_value(brush.clone()))
}

/// Adds the brushes whose colour is a static resource `rows` to `dictionary`, in order, each as
/// deferred content created with `context` that builds `build_static_brush`; the first
/// failure is the error.
#[inline(never)]
pub fn add_static_brushes(
    dictionary: &Ref<ResourceDictionary>,
    context: &Rc<XamlIlContext>,
    document: &'static DocumentInfo,
    rows: &'static [StaticBrushRow],
) -> Result<(), XamlLoadError> {
    for row in rows {
        let builder = DeferredContentBuilder::try_new(move |service_provider| build_static_brush(service_provider, document, row));
        add_deferred_row(dictionary, row.key, context, builder, row.line, row.position)?;
    }
    Ok(())
}

/// The build of a brush whose colour is `{StaticResource key}`: the context of the build, the
/// brush on the parent stack, the extension created with the key and `ProvideValue` with the
/// colour property as the provide-value target, then the setter chosen at run time from the
/// provided value (the unset marker clears the colour, a binding is bound, a colour is set,
/// anything else is the error of no setter), all at the value position of the row: the calls
/// and the errors of the build function of generated code, in the same order.
fn build_static_brush(
    service_provider: &Rc<dyn IServiceProvider>,
    document: &DocumentInfo,
    row: &StaticBrushRow,
) -> Result<MarkupValue, XamlLoadError> {
    let (line, position) = (row.value_line, row.value_position);
    let context = deferred_context(service_provider, &row_document(document, row.namespaces));
    let brush = SolidColorBrush::new();
    context.push_parent(to_value(brush.clone()));
    let extension = StaticResourceExtension::__markup_new_1(string_object(row.color_key));
    let provided = provide_value_invoked(
        &context,
        property_value(SolidColorBrush::color_property()),
        &extension,
        StaticResourceExtension::__markup_ProvideValue_0,
        line,
        position,
    )?;
    let value: MarkupValue = to_value(provided);
    if is_instance(&value, markup_handle(<UnsetValueType as MarkupTyped>::MARKUP)) {
        unset_value(brush.upcast_ref::<FerroObject>(), SolidColorBrush::color_property());
    } else if is_instance(&value, markup_handle(<dyn BindingBase as MarkupTyped>::MARKUP)) {
        bind(brush.upcast_ref::<FerroObject>(), SolidColorBrush::color_property(), value.clone(), line, position)?;
    } else if is_instance(&value, markup_handle(<Color as MarkupTyped>::MARKUP)) {
        brush.set_value(
            SolidColorBrush::color_property(),
            exact(value.clone(), "FerroUI.Media.SolidColorBrush.set_Color", 0, line, position)?,
        );
    } else {
        return Err(no_setter("Color", &value, line, position));
    }
    context.pop_parent();
    Ok(to_value(brush.clone()))
}

/// `IDictionary.Add(key, value)` of a plain value, as the statement of generated code makes it:
/// the dictionary as its contract (`rt::cast`), the key, the value, then the call; a failure of
/// the cast or of the call is the load error at `line`, `position`. `value` is made after the
/// cast and the key, as the arguments of the statement are evaluated.
#[inline(never)]
fn add_value_row(
    dictionary: &Ref<ResourceDictionary>,
    key: &str,
    value: &dyn Fn() -> MarkupValue,
    line: i32,
    position: i32,
) -> Result<(), XamlLoadError> {
    let target: Rc<dyn IResourceDictionary> = cast(dictionary.clone(), line, position)?;
    let key = string_object(key);
    invoked(<dyn IResourceDictionary>::__markup_Add_0(&target, key, value()), line, position)
}

/// Adds the numbers `rows` to `dictionary`, in order; the first failure is the error.
#[inline(never)]
pub fn add_double_resources(dictionary: &Ref<ResourceDictionary>, rows: &[ValueRow<f64>]) -> Result<(), XamlLoadError> {
    for row in rows {
        add_value_row(dictionary, row.key, &|| to_object(row.value), row.line, row.position)?;
    }
    Ok(())
}

/// Adds the thicknesses `rows` (`Thickness(left, top, right, bottom)`) to `dictionary`, in
/// order; the first failure is the error.
#[inline(never)]
pub fn add_thickness_resources(
    dictionary: &Ref<ResourceDictionary>,
    rows: &[ValueRow<[f64; 4]>],
) -> Result<(), XamlLoadError> {
    for row in rows {
        let [left, top, right, bottom] = row.value;
        add_value_row(dictionary, row.key, &|| to_object(Thickness::__markup_new_3(left, top, right, bottom)), row.line, row.position)?;
    }
    Ok(())
}

/// Adds the corner radii `rows` (`CornerRadius(top_left, top_right, bottom_right,
/// bottom_left)`) to `dictionary`, in order; the first failure is the error.
#[inline(never)]
pub fn add_corner_radius_resources(
    dictionary: &Ref<ResourceDictionary>,
    rows: &[ValueRow<[f64; 4]>],
) -> Result<(), XamlLoadError> {
    for row in rows {
        let [top_left, top_right, bottom_right, bottom_left] = row.value;
        add_value_row(
            dictionary,
            row.key,
            &|| to_object(CornerRadius::__markup_new_3(top_left, top_right, bottom_right, bottom_left)),
            row.line,
            row.position,
        )?;
    }
    Ok(())
}

/// Adds the colours `rows` (`Color.FromUInt32(argb)`) to `dictionary`, in order; the first
/// failure is the error. The colour is made before the dictionary is cast, as generated code
/// holds it in a local before the statement of the addition.
#[inline(never)]
pub fn add_color_resources(dictionary: &Ref<ResourceDictionary>, rows: &[ValueRow<u32>]) -> Result<(), XamlLoadError> {
    for row in rows {
        let color = Color::__markup_FromUInt32_2(row.value);
        add_value_row(dictionary, row.key, &|| to_object(color), row.line, row.position)?;
    }
    Ok(())
}

/// Adds the texts `rows` to `dictionary`, in order; the first failure is the error.
#[inline(never)]
pub fn add_string_resources(
    dictionary: &Ref<ResourceDictionary>,
    rows: &[ValueRow<&'static str>],
) -> Result<(), XamlLoadError> {
    for row in rows {
        add_value_row(dictionary, row.key, &|| string_object(row.value), row.line, row.position)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "theme_resources_tests.rs"]
mod tests;
