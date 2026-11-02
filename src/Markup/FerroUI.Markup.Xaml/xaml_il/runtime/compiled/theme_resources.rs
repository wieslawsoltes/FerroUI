//! Typed deferred resource factories shared by compiled theme tables. Each entry point
//! references only its own factory, so a dictionary of aliases does not retain any brush code.

use std::rc::Rc;

use ferroui_base::controls::{IDeferredContent, ResourceDictionary, ResourceKey};
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
    let resource_key = ResourceKey::from(key);
    if !dictionary.contains_key(&resource_key) {
        // The compiler proved this operand is a string. Insert its final key directly,
        // bypassing the intermediate String, BoxedValue and dynamic key conversion.
        dictionary.add_deferred(resource_key, deferred);
        return Ok(());
    }
    // Preserve the markup member's exact duplicate-key failure and source position.
    invoked(
        ResourceDictionary::__markup_AddDeferred_1(
            dictionary,
            string_object(key),
            deferred as Rc<dyn IDeferredContent>,
        ),
        line,
        column,
    )
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
    use crate::markup_extensions::StaticResourceExtension;
    use ferroui_base::{data::BindingBase, metadata::MarkupTyped, UnsetValueType};
    register(dictionary, context, key, line, column, move |provider| {
        let context = deferred_context(provider, document);
        let brush = SolidColorBrush::new();
        context.push_parent(to_value(brush.clone()));
        let property = SolidColorBrush::color_property();
        let extension = StaticResourceExtension::__markup_new_1(string_object(target.as_ref()));
        let provided = super::provide_value_invoked(
            &context,
            super::property_value(property),
            &extension,
            StaticResourceExtension::__markup_ProvideValue_0,
            value_line,
            value_column,
        )?;
        let value = to_value(provided);
        let object = brush.upcast_ref::<FerroObject>();
        if super::is_instance(&value, super::markup_handle(UnsetValueType::MARKUP)) {
            super::unset_value(object, property);
        } else if super::is_instance(&value, super::markup_handle(<dyn BindingBase>::MARKUP)) {
            super::bind(object, property, value, value_line, value_column)?;
        } else if super::is_instance(&value, super::markup_handle(Color::MARKUP)) {
            let color: Color = super::exact(
                value,
                "FerroUI.Media.SolidColorBrush.set_Color",
                0,
                value_line,
                value_column,
            )?;
            brush.set_value(property, color);
        } else {
            return Err(super::no_setter("Color", &value, value_line, value_column));
        }
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
