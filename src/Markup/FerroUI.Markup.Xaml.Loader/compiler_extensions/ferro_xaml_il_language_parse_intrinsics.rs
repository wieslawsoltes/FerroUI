//! Port of `CompilerExtensions/FerroXamlIlLanguageParseIntrinsics.cs`.
//!
//! Compile-time conversion of text to the framework's value types. The text is parsed by the
//! framework's own parsers (`Thickness::parse`, `Color::try_parse`, ...), so the compiler
//! accepts exactly what the runtime accepts, and the result is an AST node that builds the
//! value without parsing at run time.

use std::rc::Rc;

use ferroui_base::animation::TimeSpan;
use ferroui_base::media::Color;
use ferroui_base::utilities::span_helpers::{parse_double, try_parse_double, NumberStyles};
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::{
    CornerRadius, Matrix, Point, RelativePoint, RelativeUnit, Size, Thickness, Vector,
};
use xamlx::ast::{
    IXamlAstValueNode, XamlAstClrTypeReference, XamlAstExtensions, XamlAstNewClrObjectNode,
    XamlAstObjectNode, XamlAstTextNode, XamlConstantNode, XamlStaticExtensionNode,
    XamlStaticOrTargetedReturnMethodCallNode,
};
use xamlx::diagnostics::{XamlDiagnostic, XamlDiagnosticSeverity};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{AstTransformationContext, XamlTransformHelpers};
use xamlx::type_system::{
    FindMethodMethodSignature, IXamlProperty, IXamlType, TypeSystemHelpers, XamlValue,
};

use super::ast_nodes::{
    FerroXamlIlArrayConstantAstNode, FerroXamlIlFerroListConstantAstNode,
    FerroXamlIlFontFamilyAstNode, FerroXamlIlGridLength, FerroXamlIlGridLengthAstNode,
    FerroXamlIlGridUnitType, FerroXamlIlVectorLikeConstantAstNode,
};
use super::transformers::FerroXamlIlWellKnownTypes;
use super::xaml_il_ferro_property_helper::ordinal_ignore_case_equals;
use super::FerroXamlDiagnosticCodes;

/// `StringSplitOptions.RemoveEmptyEntries`.
const REMOVE_EMPTY_ENTRIES: i32 = 1;
/// `StringSplitOptions.TrimEntries`.
const TRIM_ENTRIES: i32 = 2;

/// A value produced by a compile-time value parser.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum XamlCompileTimeValue {
    /// A `GridLength`: its value and unit.
    GridLength(FerroXamlIlGridLength),
}

/// A backend-neutral hook for compile-time parsing of framework values whose parser does not
/// live in the base library (the compiler must not link the controls library).
///
/// Parsers are registered on the compiler configuration through
/// [`XamlCompileTimeValueParsers`] (a configuration extra). The parse intrinsics ask them
/// first and fall back to their own port of the value's text grammar; today that concerns
/// `GridLength` (also inside `ColumnDefinition`/`RowDefinition`). A run-time back end that has
/// the framework's type metadata at hand may register a parser that resolves the target type's
/// own parse function (`ferroui_base::metadata::MarkupType::parse`) instead.
pub trait IXamlCompileTimeValueParser: 'static {
    /// `None`: this parser does not handle `type_`. `Some(Err(message))`: the text is not a
    /// valid value of the type. `Some(Ok(value))`: the parsed value.
    fn try_parse(
        &self,
        type_: &Rc<dyn IXamlType>,
        text: &str,
    ) -> Option<Result<XamlCompileTimeValue, String>>;
}

/// The compile-time value parsers of a configuration
/// (`configuration.get_or_create_extra::<XamlCompileTimeValueParsers>()`).
#[derive(Default)]
pub struct XamlCompileTimeValueParsers {
    parsers: std::cell::RefCell<Vec<Rc<dyn IXamlCompileTimeValueParser>>>,
}

impl XamlCompileTimeValueParsers {
    pub fn add(&self, parser: Rc<dyn IXamlCompileTimeValueParser>) {
        self.parsers.borrow_mut().push(parser);
    }

    /// The result of the first registered parser that handles `type_`.
    pub fn try_parse(
        &self,
        type_: &Rc<dyn IXamlType>,
        text: &str,
    ) -> Option<Result<XamlCompileTimeValue, String>> {
        let parsers = self.parsers.borrow().clone();
        parsers.iter().find_map(|p| p.try_parse(type_, text))
    }
}

pub struct FerroXamlIlLanguageParseIntrinsics;

type ConvertResult = XamlResult<Option<Rc<dyn IXamlAstValueNode>>>;

impl FerroXamlIlLanguageParseIntrinsics {
    /// `TryConvert(context, node, text, type, types, out result)`: `Ok(Some(result))` is `true`,
    /// `Ok(None)` is `false`.
    ///
    /// A text that cannot be parsed as the requested type is reported as a
    /// `FRN2005` diagnostic (an error the diagnostics handler may lower to a warning) and the
    /// conversion fails with `Ok(None)`, so that the caller can try other conversions.
    pub fn try_convert(
        context: &AstTransformationContext,
        node: &Rc<dyn IXamlAstValueNode>,
        text: &str,
        type_: &Rc<dyn IXamlType>,
        types: &Rc<FerroXamlIlWellKnownTypes>,
    ) -> ConvertResult {
        let return_on_parse_error = |title: String| -> ConvertResult {
            let mut diagnostic = XamlDiagnostic::with_line_info(
                FerroXamlDiagnosticCodes::FERRO_INTRINSICS_ERROR,
                XamlDiagnosticSeverity::Error,
                title,
                Some(&**node),
            );
            // Only one instance when we can lower Error to a Warning
            diagnostic.min_severity = XamlDiagnosticSeverity::Warning;
            context.report_diagnostic(diagnostic, true)?;
            Ok(None)
        };
        // `try { ... } catch { return ReturnOnParseError(...); }`: the value parser and the node
        // constructor both run inside the `try` block upstream.
        let vector_like = |parsed: Option<Vec<f64>>,
                           value_type: &Rc<dyn IXamlType>,
                           constructor: &Rc<dyn xamlx::type_system::IXamlConstructor>|
         -> Option<Rc<dyn IXamlAstValueNode>> {
            let values = parsed?;
            let result: Rc<dyn IXamlAstValueNode> = FerroXamlIlVectorLikeConstantAstNode::new(
                &**node,
                types,
                value_type.clone(),
                constructor.clone(),
                values,
            )
            .ok()?;
            Some(result)
        };

        if type_.is("System", "TimeSpan") {
            let ts_text = text.trim();

            let time_span = match TimeSpan::parse(ts_text) {
                Ok(time_span) => time_span,
                Err(_) => {
                    // shorthand seconds format (ie. "0.25")
                    let seconds = if !ts_text.contains(':') {
                        try_parse_double(ts_text, NumberStyles::FLOAT_THOUSANDS)
                    } else {
                        None
                    };
                    match seconds {
                        Some(seconds) => time_span_from_seconds(seconds)?,
                        None => {
                            return return_on_parse_error(format!(
                                "Unable to parse {text} as a time span"
                            ));
                        }
                    }
                }
            };

            let result = XamlStaticOrTargetedReturnMethodCallNode::new(
                &**node,
                type_.get_method_by_name(
                    "FromTicks",
                    &**type_,
                    false,
                    std::slice::from_ref(&types.long),
                )?,
                Some(vec![XamlConstantNode::new(
                    &**node,
                    types.long.clone(),
                    XamlValue::Int64(time_span.ticks()),
                )?]),
            );
            return Ok(Some(result));
        }

        if type_.equals(&*types.font_family) {
            return Ok(Some(FerroXamlIlFontFamilyAstNode::new(types, text, &**node)));
        }

        if type_.equals(&*types.thickness) {
            let parsed = Thickness::parse(text)
                .ok()
                .map(|t| vec![t.left, t.top, t.right, t.bottom]);
            return match vector_like(parsed, &types.thickness, &types.thickness_full_constructor) {
                Some(result) => Ok(Some(result)),
                None => return_on_parse_error(format!("Unable to parse \"{text}\" as a thickness")),
            };
        }

        if type_.equals(&*types.point) {
            let parsed = Point::parse(text).ok().map(|p| vec![p.x, p.y]);
            return match vector_like(parsed, &types.point, &types.point_full_constructor) {
                Some(result) => Ok(Some(result)),
                None => return_on_parse_error(format!("Unable to parse \"{text}\" as a point")),
            };
        }

        if type_.equals(&*types.vector) {
            let parsed = Vector::parse(text).ok().map(|v| vec![v.x, v.y]);
            return match vector_like(parsed, &types.vector, &types.vector_full_constructor) {
                Some(result) => Ok(Some(result)),
                None => return_on_parse_error(format!("Unable to parse \"{text}\" as a vector")),
            };
        }

        if type_.equals(&*types.size) {
            let parsed = Size::parse(text).ok().map(|s| vec![s.width, s.height]);
            return match vector_like(parsed, &types.size, &types.size_full_constructor) {
                Some(result) => Ok(Some(result)),
                None => return_on_parse_error(format!("Unable to parse \"{text}\" as a size")),
            };
        }

        if type_.equals(&*types.matrix) {
            let parsed = Matrix::parse(text)
                .ok()
                .map(|m| vec![m.m11, m.m12, m.m21, m.m22, m.m31, m.m32]);
            return match vector_like(parsed, &types.matrix, &types.matrix_full_constructor) {
                Some(result) => Ok(Some(result)),
                None => return_on_parse_error(format!("Unable to parse \"{text}\" as a matrix")),
            };
        }

        if type_.equals(&*types.corner_radius) {
            let parsed = CornerRadius::parse(text)
                .ok()
                .map(|c| vec![c.top_left, c.top_right, c.bottom_right, c.bottom_left]);
            return match vector_like(
                parsed,
                &types.corner_radius,
                &types.corner_radius_full_constructor,
            ) {
                Some(result) => Ok(Some(result)),
                None => {
                    return_on_parse_error(format!("Unable to parse \"{text}\" as a corner radius"))
                }
            };
        }

        if type_.equals(&*types.color) {
            let text = text.trim();
            let Some(color) = Color::try_parse(text) else {
                return return_on_parse_error(format!("Unable to parse \"{text}\" as a color"));
            };

            let mut signature =
                FindMethodMethodSignature::new("FromUInt32", type_.clone(), vec![types.u_int.clone()]);
            signature.is_static = true;
            let result = XamlStaticOrTargetedReturnMethodCallNode::new(
                &**node,
                type_.get_method_by_signature(&signature)?,
                Some(vec![XamlConstantNode::new(
                    &**node,
                    types.u_int.clone(),
                    XamlValue::UInt32(color.to_uint32()),
                )?]),
            );

            return Ok(Some(result));
        }

        if type_.equals(&*types.relative_point) {
            let converted = (|| -> Option<Rc<dyn IXamlAstValueNode>> {
                let relative_point = RelativePoint::parse(text).ok()?;

                let relative_point_type_ref =
                    XamlAstClrTypeReference::new(&**node, types.relative_point.clone(), false);

                let unit = match relative_point.unit {
                    RelativeUnit::Relative => 0,
                    RelativeUnit::Absolute => 1,
                };
                let arguments: Vec<Rc<dyn IXamlAstValueNode>> = vec![
                    XamlConstantNode::new(
                        &**node,
                        types.xaml_il_types.double.clone(),
                        XamlValue::Double(relative_point.point.x),
                    )
                    .ok()?,
                    XamlConstantNode::new(
                        &**node,
                        types.xaml_il_types.double.clone(),
                        XamlValue::Double(relative_point.point.y),
                    )
                    .ok()?,
                    XamlConstantNode::new(
                        &**node,
                        types.relative_unit.clone(),
                        XamlValue::Int32(unit),
                    )
                    .ok()?,
                ];
                Some(XamlAstNewClrObjectNode::new(
                    &**node,
                    relative_point_type_ref,
                    types.relative_point_full_constructor.clone(),
                    arguments,
                ))
            })();
            return match converted {
                Some(result) => Ok(Some(result)),
                None => {
                    return_on_parse_error(format!("Unable to parse \"{text}\" as a relative point"))
                }
            };
        }

        // `GridLength.Parse(text)`: a registered compile-time parser, or the built-in grammar.
        let parse_grid_length = || -> Option<FerroXamlIlGridLength> {
            let registered = context
                .configuration()
                .get_extra::<XamlCompileTimeValueParsers>()
                .ok()
                .and_then(|parsers| parsers.try_parse(&types.grid_length, text));
            match registered {
                Some(Ok(XamlCompileTimeValue::GridLength(grid_length))) => Some(grid_length),
                Some(Err(_)) => None,
                None => parse_grid_length_text(text),
            }
        };

        if type_.equals(&*types.grid_length) {
            return match parse_grid_length() {
                Some(grid_length) => Ok(Some(FerroXamlIlGridLengthAstNode::new(
                    &**node,
                    types,
                    grid_length,
                ))),
                None => {
                    return_on_parse_error(format!("Unable to parse \"{text}\" as a grid length"))
                }
            };
        }

        if type_.equals(&*types.column_definition) || type_.equals(&*types.row_definition) {
            let converted = (|| -> Option<Rc<dyn IXamlAstValueNode>> {
                let grid_length = parse_grid_length()?;

                let definition_constructor_grid_length = type_
                    .get_constructor(Some(std::slice::from_ref(&types.grid_length)))
                    .ok()?;
                let length_node: Rc<dyn IXamlAstValueNode> =
                    FerroXamlIlGridLengthAstNode::new(&**node, types, grid_length);
                let definition_type_ref = XamlAstClrTypeReference::new(&**node, type_.clone(), false);

                Some(XamlAstNewClrObjectNode::new(
                    &**node,
                    definition_type_ref,
                    definition_constructor_grid_length,
                    vec![length_node],
                ))
            })();
            return match converted {
                Some(result) => Ok(Some(result)),
                None => {
                    return_on_parse_error(format!("Unable to parse \"{text}\" as a grid length"))
                }
            };
        }

        if type_.equals(&*types.cursor) {
            if let Some(enum_constant_node) = TypeSystemHelpers::try_get_enum_value_node(
                &types.standard_cursor_type,
                text,
                &**node,
                false,
            )? {
                let cursor_type_ref = XamlAstClrTypeReference::new(&**node, types.cursor.clone(), false);

                let result = XamlAstNewClrObjectNode::new(
                    &**node,
                    cursor_type_ref,
                    types.cursor_type_constructor.clone(),
                    vec![enum_constant_node],
                );

                return Ok(Some(result));
            }
        }

        if types.i_brush.is_assignable_from(&**type_) {
            if let Some(color) = Color::try_parse(text) {
                let brush_type_ref =
                    XamlAstClrTypeReference::new(&**node, types.immutable_solid_color_brush.clone(), false);

                let result = XamlAstNewClrObjectNode::new(
                    &**node,
                    brush_type_ref,
                    types.immutable_solid_color_brush_constructor_color.clone(),
                    vec![XamlConstantNode::new(
                        &**node,
                        types.u_int.clone(),
                        XamlValue::UInt32(color.to_uint32()),
                    )?],
                );

                return Ok(Some(result));
            }
        }

        if type_.equals(&*types.text_trimming) {
            if let Some(result) = static_property_getter_call(
                node,
                &types.text_trimming,
                &types.text_trimming,
                text,
            )? {
                return Ok(Some(result));
            }
        }

        if type_.equals(&*types.text_decoration_collection) {
            if let Some(result) = static_property_getter_call(
                node,
                &types.text_decorations,
                &types.text_decoration_collection,
                text,
            )? {
                return Ok(Some(result));
            }
        }

        if type_.equals(&*types.window_transparency_level) {
            if let Some(result) = static_property_getter_call(
                node,
                &types.window_transparency_level,
                &types.window_transparency_level,
                text,
            )? {
                return Ok(Some(result));
            }
        }

        if type_.equals(&*types.uri) {
            let uri_text = text.trim();

            let kind = if !uri_text.starts_with('/') {
                UriKind::RelativeOrAbsolute
            } else {
                UriKind::Relative
            };

            if uri_text.is_empty() || Uri::try_create(uri_text, kind).is_none() {
                return return_on_parse_error(format!(
                    "Unable to parse text \"{uri_text}\" as a {} uri",
                    uri_kind_name(kind)
                ));
            }
            let result = XamlAstNewClrObjectNode::new(
                &**node,
                XamlAstClrTypeReference::new(&**node, types.uri.clone(), false),
                types.uri_constructor.clone(),
                vec![
                    XamlConstantNode::new(
                        &**node,
                        context.configuration().well_known_types().string.clone(),
                        XamlValue::String(uri_text.to_string()),
                    )?,
                    XamlConstantNode::new(
                        &**node,
                        types.uri_kind.clone(),
                        XamlValue::Int32(uri_kind_value(kind)),
                    )?,
                ],
            );
            return Ok(Some(result));
        }

        if type_.equals(&*types.theme_variant) {
            let variant_text = text.trim();
            let found_const_property = types.theme_variant.properties().into_iter().find(|p| {
                p.name() == variant_text && p.property_type().equals(&*types.theme_variant)
            });
            let theme_variant_type_ref =
                XamlAstClrTypeReference::new(&**node, types.theme_variant.clone(), false);
            if let Some(found_const_property) = found_const_property {
                let result = XamlStaticExtensionNode::new(
                    &XamlAstObjectNode::new(&**node, node.type_()),
                    Some(theme_variant_type_ref),
                    &found_const_property.name(),
                );
                return Ok(Some(result));
            }
        }

        // Keep it in the end, so more specific parsers can be applied.
        let element_type = Self::get_element_type(type_);
        if let Some(element_type) = element_type {
            let items: Vec<String>;
            // Normalize special case of Points collection.
            if element_type.equals(&*types.point) {
                let point_parts = split(
                    text,
                    Some(&[",".to_string(), " ".to_string()]),
                    REMOVE_EMPTY_ENTRIES,
                );
                if point_parts.len() % 2 == 0 {
                    items = point_parts
                        .chunks(2)
                        .map(|pair| format!("{} {}", pair[0], pair[1]))
                        .collect();
                } else {
                    return return_on_parse_error(format!(
                        "Unable to parse text \"{text}\" as a Points list"
                    ));
                }
            } else {
                let trim_option = TRIM_ENTRIES;
                let mut separators: Option<Vec<String>> = Some(vec![",".to_string()]);
                let mut split_options = REMOVE_EMPTY_ENTRIES | trim_option;

                let attribute = type_
                    .get_all_custom_attributes()
                    .into_iter()
                    .find(|a| a.type_().equals(&*types.ferro_list_attribute));
                if let Some(attribute) = attribute {
                    let properties = attribute.properties();
                    if let Some(separators_array) = properties.get("Separators") {
                        separators = match separators_array {
                            XamlValue::Array(values) => Some(
                                values
                                    .iter()
                                    .filter_map(|v| v.as_str().map(str::to_string))
                                    .collect(),
                            ),
                            XamlValue::Null => None,
                            other => {
                                return Err(XamlError::invalid_cast(format!(
                                    "Unable to cast object of type '{}' to type 'System.Array'.",
                                    other.type_name()
                                )));
                            }
                        };
                    }

                    if let Some(split_options_obj) = properties.get("SplitOptions") {
                        split_options = match split_options_obj {
                            XamlValue::Null => {
                                return Err(XamlError::internal(
                                    "NullReferenceException",
                                    "Object reference not set to an instance of an object.",
                                ));
                            }
                            other => TypeSystemHelpers::convert_literal_to_int(other)?,
                        };
                    }
                }

                let mut split_items = split(text, separators.as_deref(), split_options ^ trim_option);
                // The upstream compiler targets a runtime without StringSplitOptions.TrimEntries,
                // so it emulates the option, if it was requested.
                if split_options & trim_option == trim_option {
                    split_items = split_items
                        .into_iter()
                        .map(|i| i.trim().to_string())
                        .collect();
                }
                items = split_items;
            }

            let mut nodes: Vec<Rc<dyn IXamlAstValueNode>> = Vec::with_capacity(items.len());
            for item in &items {
                let text_node: Rc<dyn IXamlAstValueNode> = XamlAstTextNode::with_type(
                    &**node,
                    item,
                    true,
                    Some(context.configuration().well_known_types().string.clone()),
                );
                let item_node = XamlTransformHelpers::try_get_correctly_typed_value(
                    context,
                    &text_node,
                    &element_type,
                )?;
                let Some(item_node) = item_node else {
                    return Ok(None);
                };

                nodes.push(item_node);
            }

            for element in &nodes {
                let element_clr_type = element.type_().get_clr_type()?;
                if !element_type.is_assignable_from(&*element_clr_type) {
                    return return_on_parse_error(format!(
                        "x:Array element {} is not assignable to the array element type {}",
                        element_clr_type.name(),
                        element_type.name()
                    ));
                }
            }

            let well_known_types = context.configuration().well_known_types();
            if types
                .ferro_list
                .make_generic_type(std::slice::from_ref(&element_type))?
                .is_assignable_from(&**type_)
            {
                let result = FerroXamlIlFerroListConstantAstNode::new(
                    &**node,
                    types,
                    type_.clone(),
                    element_type,
                    nodes,
                )?;
                return Ok(Some(result));
            } else if type_.is_array() {
                let result = FerroXamlIlArrayConstantAstNode::new(
                    &**node,
                    element_type.make_array_type(1)?,
                    element_type,
                    nodes,
                );
                return Ok(Some(result));
            } else if type_.equals(
                &*well_known_types
                    .i_list_of_t
                    .make_generic_type(std::slice::from_ref(&element_type))?,
            ) || type_.equals(
                &*types
                    .i_read_only_list_of_t
                    .make_generic_type(std::slice::from_ref(&element_type))?,
            ) {
                let list_type = well_known_types
                    .i_list_of_t
                    .make_generic_type(std::slice::from_ref(&element_type))?;
                let result =
                    FerroXamlIlArrayConstantAstNode::new(&**node, list_type, element_type, nodes);
                return Ok(Some(result));
            }
        }

        Ok(None)
    }

    fn get_element_type(type_: &Rc<dyn IXamlType>) -> Option<Rc<dyn IXamlType>> {
        if type_.is_array() {
            return type_.array_element_type();
        }

        type_
            .get_all_interfaces()
            .into_iter()
            .find(|i| {
                i.name().starts_with("IEnumerable`1")
                    && i.namespace().as_deref() == Some("System.Collections.Generic")
            })
            .and_then(|i| i.generic_arguments().into_iter().next())
    }
}

/// The text grammar of `GridLength.Parse`: `Auto` (any case), `[n]*` (star, `n` defaults to 1)
/// or a pixel value. Values must be finite and not negative.
fn parse_grid_length_text(s: &str) -> Option<FerroXamlIlGridLength> {
    fn parse_value(s: &str) -> Option<f64> {
        parse_double(s).filter(|value| !(*value < 0.0 || value.is_nan() || value.is_infinite()))
    }

    let s = s.to_uppercase();

    if s == "AUTO" {
        Some(FerroXamlIlGridLength {
            value: 0.0,
            grid_unit_type: FerroXamlIlGridUnitType::Auto,
        })
    } else if let Some(value_string) = s.strip_suffix('*') {
        let value_string = value_string.trim();
        let value = if !value_string.is_empty() {
            parse_value(value_string)?
        } else {
            1.0
        };
        Some(FerroXamlIlGridLength {
            value,
            grid_unit_type: FerroXamlIlGridUnitType::Star,
        })
    } else {
        Some(FerroXamlIlGridLength {
            value: parse_value(&s)?,
            grid_unit_type: FerroXamlIlGridUnitType::Pixel,
        })
    }
}

/// The value of a static property named `text` (ignoring case) of type `property_type`
/// declared by `owner`, as a call of its getter.
fn static_property_getter_call(
    node: &Rc<dyn IXamlAstValueNode>,
    owner: &Rc<dyn IXamlType>,
    property_type: &Rc<dyn IXamlType>,
    text: &str,
) -> ConvertResult {
    for property in owner.properties() {
        if property.property_type().equals(&**property_type)
            && ordinal_ignore_case_equals(&property.name(), text)
        {
            let getter = property_getter(&*property)?;
            let result = XamlStaticOrTargetedReturnMethodCallNode::new(&**node, getter, Some(Vec::new()));

            return Ok(Some(result));
        }
    }
    Ok(None)
}

/// `property.Getter!`.
fn property_getter(property: &dyn IXamlProperty) -> XamlResult<Rc<dyn xamlx::type_system::IXamlMethod>> {
    property.getter().ok_or_else(|| {
        XamlError::internal(
            "NullReferenceException",
            format!("Property {} doesn't have a getter", property.name()),
        )
    })
}

/// `TimeSpan.FromSeconds(seconds)` with the argument checks reported as errors.
fn time_span_from_seconds(seconds: f64) -> XamlResult<TimeSpan> {
    if seconds.is_nan() {
        return Err(XamlError::argument(
            "TimeSpan does not accept floating point Not-a-Number values.",
        ));
    }
    let ticks = seconds * TimeSpan::TICKS_PER_SECOND as f64;
    if !(ticks <= i64::MAX as f64 && ticks >= i64::MIN as f64) {
        return Err(XamlError::internal(
            "OverflowException",
            "TimeSpan overflowed because the duration is too long.",
        ));
    }
    Ok(TimeSpan::from_seconds(seconds))
}

/// `UriKind.ToString()`.
fn uri_kind_name(kind: UriKind) -> &'static str {
    match kind {
        UriKind::RelativeOrAbsolute => "RelativeOrAbsolute",
        UriKind::Absolute => "Absolute",
        UriKind::Relative => "Relative",
    }
}

/// `(int) kind` of `System.UriKind`.
fn uri_kind_value(kind: UriKind) -> i32 {
    match kind {
        UriKind::RelativeOrAbsolute => 0,
        UriKind::Absolute => 1,
        UriKind::Relative => 2,
    }
}

/// `string.Split(string[] separators, StringSplitOptions options)`.
///
/// `None` or no usable separator splits at white space. At each position the first separator
/// (in array order) that matches is used. `options` is a combination of
/// `RemoveEmptyEntries` (1) and `TrimEntries` (2); entries are trimmed before empty ones are
/// removed.
fn split(text: &str, separators: Option<&[String]>, options: i32) -> Vec<String> {
    let separators: Vec<&str> = separators
        .unwrap_or(&[])
        .iter()
        .map(String::as_str)
        .filter(|s| !s.is_empty())
        .collect();

    let mut parts: Vec<&str> = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < text.len() {
        let rest = &text[index..];
        let matched = if separators.is_empty() {
            rest.chars()
                .next()
                .filter(|c| c.is_whitespace())
                .map(char::len_utf8)
        } else {
            separators
                .iter()
                .find(|s| rest.starts_with(**s))
                .map(|s| s.len())
        };
        match matched {
            Some(length) => {
                parts.push(&text[start..index]);
                index += length;
                start = index;
            }
            None => {
                index += rest.chars().next().map(char::len_utf8).unwrap_or(1);
            }
        }
    }
    parts.push(&text[start..]);

    parts
        .into_iter()
        .map(|part| {
            if options & TRIM_ENTRIES != 0 {
                part.trim()
            } else {
                part
            }
        })
        .filter(|part| options & REMOVE_EMPTY_ENTRIES == 0 || !part.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
#[path = "ferro_xaml_il_language_parse_intrinsics_tests.rs"]
mod tests;
