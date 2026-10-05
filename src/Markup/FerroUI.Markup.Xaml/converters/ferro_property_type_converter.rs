//! Port of `Converters/FerroPropertyTypeConverter.cs`.

use super::type_converter::{service_provider_of, text_of, type_converter_markup};
use super::{ITypeDescriptorContext, TypeConverter};
use crate::parsers::PropertyParser;
use crate::templates::ControlTemplate;
use crate::{ServiceProviderExtensions, XamlLoadException};
use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::data::core::ValueType;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::styling::Style;
use ferroui_base::utilities::{CharacterReader, CultureInfo};
use ferroui_base::{BoxedValue, FerroPropertyRegistry, Ref, StaticType, TypeInfo};
use ferroui_controls::Control;
use std::any::Any;
use std::rc::Rc;

/// Converts text (`Name`, `Owner.Name`, `ns:Owner.Name`) to the registered
/// property it names. Without an owner, the property is looked up on the
/// target type of the nearest control template or style, or on the control
/// class.
pub struct FerroPropertyTypeConverter;

impl FerroPropertyTypeConverter {
    fn try_resolve_owner_by_name(
        context: Option<&Rc<dyn ITypeDescriptorContext>>,
        ns: Option<&str>,
        owner: Option<&str>,
    ) -> Result<Option<&'static TypeInfo>, XamlLoadException> {
        let Some(owner) = owner else { return Ok(None) };

        let result = match service_provider_of(context) {
            Some(context) => match context.resolve_type(ns, owner)? {
                CastTarget::Class(class) => Some(class),
                CastTarget::Value(_) => None,
            },
            None => None,
        };

        match result {
            Some(result) => Ok(Some(result)),
            None => {
                let name = match ns {
                    Some(ns) if !ns.is_empty() => format!("{ns}:{owner}"),
                    _ => owner.to_string(),
                };
                Err(XamlLoadException::with_message(format!("Could not find type '{name}'.")))
            }
        }
    }
}

impl TypeConverter for FerroPropertyTypeConverter {
    fn can_convert_from(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, source_type: ValueType) -> bool {
        source_type.is_string()
    }

    fn convert_from(
        &self,
        context: Option<&Rc<dyn ITypeDescriptorContext>>,
        _culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        let registry = FerroPropertyRegistry::instance();
        let (ns, owner, property_name) = PropertyParser::parse_reader(CharacterReader::new(text_of(value)?))
            .map_err(|e| XamlLoadException::with_inner(e.to_string(), e))?;
        let owner_type = Self::try_resolve_owner_by_name(context, ns.as_deref(), owner.as_deref())?;
        let service_provider = service_provider_of(context);
        let target_type = service_provider
            .as_ref()
            .and_then(|sp| sp.get_first_parent::<Rc<ControlTemplate>>())
            .and_then(|template| template.target_type())
            .or_else(|| {
                service_provider
                    .as_ref()
                    .and_then(|sp| sp.get_first_parent::<Ref<Style>>())
                    .and_then(|style| style.selector())
                    .and_then(|selector| selector.target_type())
            })
            .unwrap_or(<Control as StaticType>::TYPE);
        let effective_owner = owner_type.unwrap_or(target_type);
        let Some(property) = registry.find_registered(effective_owner, &property_name) else {
            return Err(XamlLoadException::with_message(format!(
                "Could not find property '{}.{}'.",
                effective_owner.name(),
                property_name
            )));
        };

        if !std::ptr::eq(effective_owner, target_type)
            && !property.is_attached()
            && !registry.is_registered(target_type, property)
        {
            if let Some(log) = Logger::try_get(LogEventLevel::Warning, LogArea::PROPERTY) {
                log.log_with_values(
                    Some(self as &dyn Any),
                    "Property '{Owner}.{Name}' is not registered on '{Type}'.",
                    &[&effective_owner, &property_name, &target_type],
                );
            }
        }

        Ok(Some(Rc::new(property)))
    }
}

type_converter_markup!(FerroPropertyTypeConverter);
