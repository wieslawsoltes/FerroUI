//! Typed deferred resource factories shared by compiled theme tables. Each entry point
//! references only its own factory, so a dictionary of aliases does not retain any brush code.

use std::rc::Rc;

use ferroui_base::controls::{ResourceDictionary, ResourceKey};
use ferroui_base::data::core::ValueType;
use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_base::metadata::{IServiceProvider, MarkupValue};
use ferroui_base::{FerroObject, Ref};

use super::{
    deferred_content, deferred_context, invoked, string_object, to_value, DeferredContentBuilder, DocumentInfo,
    XamlIlContext, XamlLoadError,
};

fn register(
    dictionary: &Ref<ResourceDictionary>,
    context: &Rc<XamlIlContext>,
    key: &str,
    line: i32,
    column: i32,
    build: impl Fn(&Rc<dyn IServiceProvider>) -> Result<MarkupValue, XamlLoadError> + 'static,
) -> Result<(), XamlLoadError> {
    let deferred = deferred_content(
        ValueType::object(),
        context,
        DeferredContentBuilder::try_new(build),
        line,
        column,
    )?;
    invoked(dictionary.try_add_deferred(ResourceKey::from(key), deferred), line, column)
}

/// Registers a deferred StaticResource lookup against its declaration's namespace table.
#[allow(clippy::too_many_arguments)]
pub fn alias(
    dictionary: &Ref<ResourceDictionary>,
    context: &Rc<XamlIlContext>,
    document: &'static DocumentInfo,
    key: &str,
    target: impl AsRef<str> + 'static,
    line: i32,
    column: i32,
    value_line: i32,
    value_column: i32,
) -> Result<(), XamlLoadError> {
    register(dictionary, context, key, line, column, move |provider| {
        super::static_resource_content(provider, document, target.as_ref(), value_line, value_column)
    })
}

/// Registers a deferred brush with a literal color. The object is created only on first lookup.
#[allow(clippy::too_many_arguments)]
pub fn literal_brush(
    dictionary: &Ref<ResourceDictionary>,
    context: &Rc<XamlIlContext>,
    document: &'static DocumentInfo,
    key: &str,
    argb: u32,
    line: i32,
    column: i32,
    _value_line: i32,
    _value_column: i32,
) -> Result<(), XamlLoadError> {
    register(dictionary, context, key, line, column, move |provider| {
        let _context = deferred_context(provider, document);
        let brush = SolidColorBrush::new();
        brush.set_value(SolidColorBrush::color_property(), Color::from_uint32(argb));
        Ok(to_value(brush))
    })
}

/// Registers a deferred brush whose color remains bound to the named dynamic resource.
#[allow(clippy::too_many_arguments)]
pub fn dynamic_brush(
    dictionary: &Ref<ResourceDictionary>,
    context: &Rc<XamlIlContext>,
    document: &'static DocumentInfo,
    key: &str,
    target: impl AsRef<str> + 'static,
    line: i32,
    column: i32,
    value_line: i32,
    value_column: i32,
) -> Result<(), XamlLoadError> {
    register(dictionary, context, key, line, column, move |provider| {
        let context = deferred_context(provider, document);
        let brush = SolidColorBrush::new();
        context.push_parent(to_value(brush.clone()));
        super::bind_dynamic_resource(
            &context,
            brush.upcast_ref::<FerroObject>(),
            SolidColorBrush::color_property(),
            target.as_ref(),
            value_line,
            value_column,
        )?;
        context.pop_parent();
        Ok(to_value(brush))
    })
}

/// Registers a deferred brush whose Color is provided by a StaticResource extension.
#[allow(clippy::too_many_arguments)]
pub fn static_brush(
    dictionary: &Ref<ResourceDictionary>,
    context: &Rc<XamlIlContext>,
    document: &'static DocumentInfo,
    key: &str,
    target: impl AsRef<str> + 'static,
    line: i32,
    column: i32,
    value_line: i32,
    value_column: i32,
) -> Result<(), XamlLoadError> {
    use ferroui_base::metadata::MarkupTyped;
    register(dictionary, context, key, line, column, move |provider| {
        let context = deferred_context(provider, document);
        let brush = SolidColorBrush::new();
        context.push_parent(to_value(brush.clone()));
        let provided = super::provide_static_resource(&context, target.as_ref(),
            || super::property_value(SolidColorBrush::color_property()), value_line, value_column)?;
        super::set_local_value(brush.upcast_ref::<FerroObject>(), SolidColorBrush::color_property, to_value(provided),
            || super::markup_handle(Color::MARKUP), false, "FerroUI.Media.SolidColorBrush.set_Color", "Color", value_line, value_column)?;
        context.pop_parent();
        Ok(to_value(brush))
    })
}

/// A text-keyed factory operand and its declaration and evaluation positions.
pub type Row<T> = (&'static str, T, i32, i32, i32, i32);

macro_rules! table {
    ($name:ident, $register:ident, $operand:ty) => {
        #[doc = "Registers the table in declaration order, stopping at the first load error."]
        #[inline(never)]
        pub fn $name(
            dictionary: &Ref<ResourceDictionary>,
            context: &Rc<XamlIlContext>,
            document: &'static DocumentInfo,
            rows: &'static [Row<$operand>],
        ) -> Result<(), XamlLoadError> {
            for &(key, value, line, column, value_line, value_column) in rows {
                $register(
                    dictionary,
                    context,
                    document,
                    key,
                    value,
                    line,
                    column,
                    value_line,
                    value_column,
                )?;
            }
            Ok(())
        }
    };
}

table!(aliases, alias, &'static str);
table!(literal_brushes, literal_brush, u32);
table!(dynamic_brushes, dynamic_brush, &'static str);
table!(static_brushes, static_brush, &'static str);

/// A local typed builder may construct a template, custom control or any other deferred
/// XAML body. Its own generated function owns the declaration's namespace scope.
pub type FactoryRow = (&'static str, super::DeferredBuild, i32, i32);

#[inline(never)]
pub fn deferred_factories(
    dictionary: &Ref<ResourceDictionary>,
    context: &Rc<XamlIlContext>,
    rows: &[FactoryRow],
) -> Result<(), XamlLoadError> {
    for &(key, build, line, column) in rows {
        super::add_deferred_resource(dictionary, key, ValueType::object(), context, build, line, column)?;
    }
    Ok(())
}

/// Immediate dictionary values retain their XAML evaluation and duplicate-key ordering.
pub type ConstantRow<T> = (&'static str, T, i32, i32);

pub(super) fn add_constant(
    dictionary: &Ref<ResourceDictionary>, key: &str,
    value: MarkupValue, line: i32, column: i32,
) -> Result<(), XamlLoadError> {
    use ferroui_base::controls::IResourceDictionary;
    // A derived dictionary can supply its own interface implementation.
    if std::ptr::eq(dictionary.get_type(), ResourceDictionary::TYPE) {
        return invoked(dictionary.try_add(ResourceKey::from(key), value), line, column);
    }
    invoked(<dyn IResourceDictionary>::__markup_Add_0(
        &super::cast(dictionary.clone(), line, column)?, string_object(key), value,
    ), line, column)
}

pub(super) fn color_value(argb: u32) -> MarkupValue {
    super::to_object(Color::from_uint32(argb))
}

pub(super) fn double_value(bits: u64) -> MarkupValue {
    super::to_object(f64::from_bits(bits))
}

pub(super) fn thickness_value(bits: [u64; 4]) -> MarkupValue {
    let [left, top, right, bottom] = bits.map(f64::from_bits);
    super::to_object(ferroui_base::Thickness::new(left, top, right, bottom))
}

pub(super) fn corner_radius_value(bits: [u64; 4]) -> MarkupValue {
    let [tl, tr, br, bl] = bits.map(f64::from_bits);
    super::to_object(ferroui_base::CornerRadius::new(tl, tr, br, bl))
}

macro_rules! constants {
    ($name:ident, $operand:ty, $make:ident) => {
        #[inline(never)]
        pub fn $name(dictionary: &Ref<ResourceDictionary>, rows: &[ConstantRow<$operand>]) -> Result<(), XamlLoadError> {
            for &(key, value, line, column) in rows {
                add_constant(dictionary, key, $make(value), line, column)?;
            }
            Ok(())
        }
    };
}

constants!(colors, u32, color_value);
constants!(doubles, u64, double_value);
constants!(thicknesses, [u64; 4], thickness_value);
constants!(corner_radii, [u64; 4], corner_radius_value);
