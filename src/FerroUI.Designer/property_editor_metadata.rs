//! Native metadata is resolved once per inspector row. Editors receive data and guarded
//! source callbacks, never a live object they could mutate behind the document engine.
use super::PropertyEditorContext;
use ferroui_base::FerroProperty;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTypeKind};
use ferroui_designer_core::{Node, NodeId};
use ferroui_designer_tools::PropertyValueShape;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyValueEdit {
    Literal(String),
    Expression(String),
    Object(String),
    Resource {
        node: NodeId,
        dynamic: bool,
    },
    /// Extract the current authored value into the chosen owner scope and replace it
    /// with a resource reference. Hosts expose this only for an eligible single target.
    ExtractResource {
        owner: NodeId,
        key: String,
    },
    Reset,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyEditorKind {
    Text,
    Boolean,
    Number,
    Enumeration { values: Vec<String>, flags: bool },
    Components(PropertyValueShape),
    Thickness,
    Color,
    Brush,
    Transform,
    GridLength,
    FontFamily,
    FontWeight,
    TimeSpan,
    Duration,
    Object,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PropertyValueSource {
    #[default]
    Inherited,
    Local,
    Expression,
    Object,
    Mixed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyResourceChoice {
    pub node: NodeId,
    pub key: String,
    pub label: String,
}

pub type PropertyValueCommit = Rc<dyn Fn(PropertyValueEdit) -> Result<(), String>>;
pub type PropertyLiteralValidator = Rc<dyn Fn(&str) -> Result<(), String>>;

#[derive(Clone)]
pub struct PropertyEditorMetadata {
    pub kind: PropertyEditorKind,
    pub source: PropertyValueSource,
    pub nullable: bool,
    pub description: Option<String>,
    /// Standalone XAML with inherited namespace declarations retained.
    pub object_source: Option<String>,
    pub resources: Vec<PropertyResourceChoice>,
    /// Eligible resource owner scopes for extracting the accepted authored value.
    /// An empty list suppresses the command (for example for mixed/inherited values).
    pub resource_owners: Vec<(NodeId, String)>,
    pub named_elements: Vec<String>,
    /// Qualified framework Binding extension in the target's namespace scope.
    pub binding_extension: String,
    pub validate_literal: Option<PropertyLiteralValidator>,
    /// The host checks document/selection identity and permissions again on every call.
    pub commit: Option<PropertyValueCommit>,
}

impl PropertyEditorMetadata {
    pub fn new(kind: PropertyEditorKind) -> Self {
        Self {
            kind,
            source: PropertyValueSource::Inherited,
            nullable: false,
            description: None,
            object_source: None,
            resources: Vec::new(),
            resource_owners: Vec::new(),
            named_elements: Vec::new(),
            binding_extension: "Binding".into(),
            validate_literal: None,
            commit: None,
        }
    }

    pub fn validate(&self, value: &str) -> Result<(), String> {
        // Validate the converted literal while the original source spelling remains
        // in the edit. XAML's leading {} escape is not part of the runtime value.
        let decoded = value.strip_prefix("{}").unwrap_or(value);
        let value = if matches!(
            self.kind,
            PropertyEditorKind::Text | PropertyEditorKind::Object
        ) {
            decoded
        } else {
            decoded.trim()
        };
        if let Some(validate) = &self.validate_literal {
            validate(value)?;
        }
        match &self.kind {
            PropertyEditorKind::Components(shape) => {
                shape.validate(value).map_err(|e| e.to_string())
            }
            PropertyEditorKind::Enumeration { values, flags } => {
                let parts = if *flags {
                    value.split(',').collect::<Vec<_>>()
                } else {
                    vec![value]
                };
                if parts.iter().all(|part| {
                    values
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case(part.trim()))
                }) {
                    Ok(())
                } else {
                    Err(format!("Choose a value from {}", values.join(", ")))
                }
            }
            PropertyEditorKind::GridLength => ferroui_designer_tools::GridLength::parse(value)
                .map(|_| ())
                .map_err(|e| e.to_string()),
            PropertyEditorKind::Thickness => ferroui_base::Thickness::parse(value)
                .map(|_| ())
                .map_err(|error| error.to_string()),
            PropertyEditorKind::Transform => ferroui_base::Matrix::parse(value)
                .map(|_| ())
                .or_else(|_| {
                    ferroui_base::media::transformation::TransformOperations::parse(value)
                        .map(|_| ())
                })
                .map_err(|error| error.to_string()),
            PropertyEditorKind::TimeSpan => ferroui_base::animation::TimeSpan::parse(value)
                .map(|_| ())
                .map_err(|error| error.to_string()),
            PropertyEditorKind::Duration => PropertyValueShape::Duration
                .validate(value)
                .map_err(|error| error.to_string()),
            PropertyEditorKind::FontFamily => PropertyValueShape::FontFamily
                .validate(value)
                .map_err(|error| error.to_string()),
            PropertyEditorKind::FontWeight => value
                .parse::<ferroui_base::media::FontWeight>()
                .map(|_| ())
                .map_err(|error| error.to_string()),
            PropertyEditorKind::Color | PropertyEditorKind::Brush => {
                ferroui_base::media::Color::parse(value)
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            }
            PropertyEditorKind::Boolean
                if !matches!(value.to_ascii_lowercase().as_str(), "true" | "false") =>
            {
                Err("Enter True or False".into())
            }
            _ => Ok(()),
        }
    }

    pub fn apply(
        &self,
        context: &PropertyEditorContext,
        edit: PropertyValueEdit,
    ) -> Result<(), String> {
        if let PropertyValueEdit::Literal(value) = &edit {
            self.validate(value)?;
            validate_primitive(
                context.runtime_type.as_deref().unwrap_or(""),
                value.strip_prefix("{}").unwrap_or(value),
            )?;
        }
        if let PropertyValueEdit::Expression(value) = &edit {
            if !matches!(
                ferroui_designer_core::MarkupValue::parse(value).map_err(|e| e.to_string())?,
                ferroui_designer_core::MarkupValue::Extension(_)
            ) {
                return Err(
                    "Enter a XAML markup expression, or use the Value editor for a literal".into(),
                );
            }
        }
        if let Some(commit) = &self.commit {
            commit(edit)
        } else {
            match edit {
                PropertyValueEdit::Literal(value) | PropertyValueEdit::Expression(value) => {
                    (context.apply)(Some(value))
                }
                PropertyValueEdit::Reset => (context.apply)(None),
                PropertyValueEdit::Object(_)
                | PropertyValueEdit::Resource { .. }
                | PropertyValueEdit::ExtractResource { .. } => {
                    return Err(
                        "The host did not provide object/resource-property authoring".into(),
                    );
                }
            }
            Ok(())
        }
    }
}

/// Resolve actual runtime enum members, nullable forms and parsers when a property is
/// available. The type-name fallback also supports source-only/template setter editors.
pub fn resolve_property_editor_metadata(
    context: &PropertyEditorContext,
    property: Option<&FerroProperty>,
) -> PropertyEditorMetadata {
    let runtime_type = property
        .map(FerroProperty::property_type_name)
        .or(context.runtime_type.as_deref())
        .unwrap_or("");
    let native_class = property.and_then(|property| {
        ferroui_base::TypeInfo::find_by_handle(property.property_type()).map(|(type_, _)| type_)
    });
    let markup = property
        .and_then(|property| {
            MarkupType::find_by_handle(property.property_type())
                .or_else(|| MarkupType::find_by_nullable_handle(property.property_type()))
        })
        .or_else(|| native_class.and_then(MarkupType::find_by_type_info))
        .or_else(|| {
            // Use a unique fully qualified runtime identity. A short custom type name must
            // not accidentally inherit another library's editor or parser.
            let unwrapped = type_identity(runtime_type);
            let matches = registered_markup_types()
                .into_iter()
                .filter(|metadata| {
                    format!("{}::{}", metadata.module_path, metadata.name) == unwrapped
                        || (!unwrapped.contains("::")
                            && metadata.name == unwrapped
                            && framework_type(metadata))
                })
                .collect::<Vec<_>>();
            (matches.len() == 1).then(|| matches[0])
        });
    let leaf = markup
        .map(|metadata| metadata.name)
        .unwrap_or_else(|| type_leaf(runtime_type));
    let standard = markup.map_or_else(
        || {
            let identity = type_identity(runtime_type);
            !identity.contains("::")
                || identity.starts_with("ferroui_base::")
                || identity.starts_with("ferroui_controls::")
        },
        framework_type,
    );
    let kind = if let Some(metadata) =
        markup.filter(|metadata| metadata.kind == MarkupTypeKind::Enum)
    {
        PropertyEditorKind::Enumeration {
            values: metadata
                .enum_members
                .iter()
                .map(|member| member.name.to_owned())
                .collect(),
            flags: metadata.is_flags,
        }
    } else {
        match if standard { leaf } else { "" } {
            "bool" | "Boolean" => PropertyEditorKind::Boolean,
            "f32" | "f64" | "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32"
            | "u64" | "usize" | "Decimal" => PropertyEditorKind::Number,
            "Point" | "PixelPoint" => PropertyEditorKind::Components(PropertyValueShape::Point),
            "Size" | "PixelSize" => PropertyEditorKind::Components(PropertyValueShape::Size),
            "Vector" => PropertyEditorKind::Components(PropertyValueShape::Vector),
            "Rect" | "PixelRect" => PropertyEditorKind::Components(PropertyValueShape::Rect),
            "Matrix" => PropertyEditorKind::Components(PropertyValueShape::Matrix),
            "RelativePoint" => PropertyEditorKind::Components(PropertyValueShape::RelativePoint),
            "RelativeRect" => PropertyEditorKind::Components(PropertyValueShape::RelativeRect),
            "CornerRadius" => PropertyEditorKind::Components(PropertyValueShape::CornerRadius),
            "Thickness" => PropertyEditorKind::Thickness,
            "Color" => PropertyEditorKind::Color,
            "IBrush"
            | "Brush"
            | "SolidColorBrush"
            | "LinearGradientBrush"
            | "RadialGradientBrush"
            | "ConicGradientBrush" => PropertyEditorKind::Brush,
            "ITransform" | "Transform" | "MatrixTransform" | "TransformGroup" => {
                PropertyEditorKind::Transform
            }
            "GridLength" => PropertyEditorKind::GridLength,
            "FontFamily" => PropertyEditorKind::FontFamily,
            "FontWeight" => PropertyEditorKind::FontWeight,
            "TimeSpan" => PropertyEditorKind::TimeSpan,
            "Duration" => PropertyEditorKind::Duration,
            _ if runtime_type.is_empty() && super::is_color_property(context) => {
                PropertyEditorKind::Brush
            }
            _ if runtime_type.is_empty()
                && matches!(
                    context.property.as_str(),
                    "Margin" | "Padding" | "BorderThickness"
                ) =>
            {
                PropertyEditorKind::Thickness
            }
            _ if runtime_type.is_empty() && context.property == "CornerRadius" => {
                PropertyEditorKind::Components(PropertyValueShape::CornerRadius)
            }
            _ if runtime_type.is_empty()
                && matches!(
                    context.property.as_str(),
                    "HorizontalAlignment" | "HorizontalContentAlignment"
                ) =>
            {
                enumeration(&["Left", "Center", "Right", "Stretch"])
            }
            _ if runtime_type.is_empty()
                && matches!(
                    context.property.as_str(),
                    "VerticalAlignment" | "VerticalContentAlignment"
                ) =>
            {
                enumeration(&["Top", "Center", "Bottom", "Stretch"])
            }
            _ if runtime_type.is_empty() && context.property == "TextAlignment" => {
                enumeration(&["Left", "Center", "Right", "Justify"])
            }
            _ if runtime_type.is_empty() && context.property == "Orientation" => {
                enumeration(&["Horizontal", "Vertical"])
            }
            _ if runtime_type.is_empty() && context.property == "DockPanel.Dock" => {
                enumeration(&["Left", "Top", "Right", "Bottom"])
            }
            _ if runtime_type.is_empty()
                && matches!(
                    context.property.as_str(),
                    "RenderTransform" | "LayoutTransform"
                ) =>
            {
                PropertyEditorKind::Transform
            }
            _ if runtime_type.is_empty() && context.property == "RenderTransformOrigin" => {
                PropertyEditorKind::Components(PropertyValueShape::RelativePoint)
            }
            _ if (native_class.is_some()
                || markup.is_some_and(|metadata| {
                    matches!(
                        metadata.kind,
                        MarkupTypeKind::Class | MarkupTypeKind::Interface
                    )
                }))
                && !matches!(leaf, "String" | "str") =>
            {
                PropertyEditorKind::Object
            }
            _ => PropertyEditorKind::Text,
        }
    };
    let mut result = PropertyEditorMetadata::new(kind);
    result.description = property.and_then(|property| {
        property
            .owner_type()
            .markup()
            .and_then(|owner| {
                owner
                    .find_property_attributes(property.name())
                    .iter()
                    .find(|attribute| {
                        attribute.name.ends_with("DescriptionAttribute")
                            || attribute.name == "Description"
                    })
            })
            .and_then(|attribute| attribute.arguments.first())
            .and_then(|argument| match argument {
                ferroui_base::metadata::MarkupAttributeValue::Str(value) => {
                    Some((*value).to_owned())
                }
                _ => None,
            })
    });
    result.nullable = runtime_type.contains("Option<");
    result.source = if context.mixed {
        PropertyValueSource::Mixed
    } else if context.value.as_deref().is_some_and(is_expression) {
        PropertyValueSource::Expression
    } else if context.value.is_some() {
        PropertyValueSource::Local
    } else {
        PropertyValueSource::Inherited
    };
    let parse = markup.and_then(|metadata| metadata.parse);
    let native_property = property.and_then(|property| {
        ferroui_base::FerroPropertyRegistry::instance().find_registered_by_id(property.id())
    });
    if parse.is_some() || native_property.is_some() {
        result.validate_literal = Some(Rc::new(move |value| {
            let text: ferroui_base::BoxedValue = Rc::new(value.to_owned());
            let auto = native_property.is_some_and(|property| {
                matches!(property.name(), "Width" | "Height") && value == "Auto"
            });
            let parsed = if auto {
                Some(Rc::new(f64::NAN) as ferroui_base::BoxedValue)
            } else if let Some(parse) = parse {
                parse(&[Some(text.clone())]).map_err(|error| error.to_string())?
            } else {
                Some(text)
            };
            if let Some(property) = native_property {
                let target =
                    ValueType::new(property.property_type(), property.property_type_name());
                if let Some(converted) = ValueTypes::try_convert(parsed.as_ref(), target).flatten()
                {
                    if !property.is_valid_value(converted.as_any()) {
                        return Err(format!("The value is not valid for {}", property.name()));
                    }
                }
            }
            Ok(())
        }));
    }
    result
}

pub(super) fn is_expression(value: &str) -> bool {
    value.starts_with('{') && !value.starts_with("{}")
}

/// Unwrap only recognized storage wrappers. A user generic such as Envelope<Point>
/// is its own type and must never borrow Point's parser or editor accidentally.
fn type_identity(mut value: &str) -> &str {
    loop {
        value = value.trim().trim_start_matches("dyn ");
        let Some((wrapper, inner)) = value.split_once('<') else {
            return value;
        };
        if !value.ends_with('>')
            || !matches!(
                wrapper,
                "Option"
                    | "core::option::Option"
                    | "std::option::Option"
                    | "Rc"
                    | "alloc::rc::Rc"
                    | "std::rc::Rc"
                    | "Arc"
                    | "alloc::sync::Arc"
                    | "std::sync::Arc"
                    | "Box"
                    | "alloc::boxed::Box"
                    | "std::boxed::Box"
                    | "Ref"
                    | "ferroui_base::type_system::Ref"
            )
        {
            return value;
        }
        value = &inner[..inner.len() - 1];
    }
}

pub(super) fn type_leaf(value: &str) -> &str {
    let identity = type_identity(value);
    identity.rsplit("::").next().unwrap_or(identity)
}

fn framework_type(metadata: &MarkupType) -> bool {
    metadata.module_path.starts_with("ferroui_base::")
        || metadata.module_path.starts_with("ferroui_controls::")
}

/// Conservative filtering for a resource picker. Registered object identities and
/// casts determine compatibility. Unregistered/custom conversion types remain visible
/// so applications can supply their own editor and loader rules.
pub(crate) fn resource_is_compatible(property: &FerroProperty, resource: &Node) -> Option<bool> {
    let resource_type = markup_type_for_node(resource)?;
    let target = ValueType::new(property.property_type(), property.property_type_name());
    let source = resource_type.handle().or_else(|| {
        resource_type
            .type_info
            .and_then(|type_| type_().handle())
            .map(|handle| ValueType::new(handle, resource_type.name))
    });
    if source.is_some_and(|source| ValueTypes::is_assignable(source, target)) {
        return Some(true);
    }
    if let Some(class) = resource_type.type_info.map(|type_| type_()) {
        if let Some((target_class, _)) = ferroui_base::TypeInfo::find_by_handle(target.id()) {
            return Some(target_class.is_assignable_from(class));
        }
        // A resource type may not have been instantiated yet. Its declared interfaces
        // are already inspectable without running class initialization or constructors.
        let mut current = Some(class);
        while let Some(class) = current {
            if class
                .interfaces()
                .iter()
                .any(|interface| ValueTypes::is_assignable(interface(), target))
            {
                return Some(true);
            }
            current = class.base_type();
        }
    }
    let target_metadata = MarkupType::find_by_handle(target.id())
        .or_else(|| MarkupType::find_by_nullable_handle(target.id()));
    // The runtime supports conversion of string/numeric values and color resources.
    // Keep these options rather than mistaking a legal conversion for an object cast.
    if matches!(resource_type.name, "String" | "str" | "Color")
        || is_numeric_type(resource_type.name)
        || target.is_string()
    {
        return None;
    }
    target_metadata.map(|target| {
        let mut pending = vec![resource_type];
        let mut seen = std::collections::BTreeSet::new();
        while let Some(current) = pending.pop() {
            if !seen.insert(current as *const MarkupType as usize) {
                continue;
            }
            if std::ptr::eq(current, target) {
                return true;
            }
            pending.extend(current.base_type());
            pending.extend(
                current
                    .interfaces
                    .iter()
                    .filter_map(|type_| MarkupType::find_by_handle(type_().id())),
            );
        }
        false
    })
}

pub(super) struct ObjectPropertyType {
    pub name: String,
    pub runtime_type: String,
    pub property: Option<&'static FerroProperty>,
}

pub(super) fn object_property_types(node: &Node) -> Vec<ObjectPropertyType> {
    let Some(type_) = markup_type_for_node(node) else {
        return Vec::new();
    };
    let mut values = std::collections::BTreeMap::new();
    if let Some(type_info) = type_.type_info {
        for property in ferroui_base::FerroPropertyRegistry::instance()
            .get_registered(type_info())
            .iter()
            .copied()
            .filter(|property| !property.is_read_only())
        {
            values.insert(
                property.name().to_owned(),
                ObjectPropertyType {
                    name: property.name().to_owned(),
                    runtime_type: property.property_type_name().into(),
                    property: Some(property),
                },
            );
        }
    }
    let mut current = Some(type_);
    let mut seen = std::collections::BTreeSet::new();
    while let Some(type_) = current {
        if !seen.insert(type_ as *const MarkupType as usize) {
            break;
        }
        for property in type_
            .properties
            .iter()
            .filter(|property| property.set.is_some())
        {
            values
                .entry(property.name.to_owned())
                .or_insert_with(|| ObjectPropertyType {
                    name: property.name.to_owned(),
                    runtime_type: (property.type_)().name().into(),
                    property: None,
                });
        }
        current = type_.base_type();
    }
    values.into_values().collect()
}

pub(crate) fn markup_type_for_node(node: &Node) -> Option<&'static MarkupType> {
    let namespace = node.namespace_uri.as_deref()?;
    let candidates = registered_markup_types()
        .into_iter()
        .filter(|type_| {
            type_.name == node.local_name() && markup_namespace_matches(type_, namespace)
        })
        .collect::<Vec<_>>();
    let [resource_type] = candidates.as_slice() else {
        return None;
    };
    Some(*resource_type)
}

fn registered_markup_types() -> Vec<&'static MarkupType> {
    // Native classes expose their metadata through TypeInfo; the MarkupType table
    // also includes independent value/contract types. Neither table subsumes the other.
    let mut seen = std::collections::BTreeSet::new();
    MarkupType::registered_types()
        .into_iter()
        .chain(
            ferroui_base::TypeInfo::registered_types()
                .into_iter()
                .filter_map(MarkupType::find_by_type_info),
        )
        .filter(|type_| seen.insert(*type_ as *const MarkupType as usize))
        .collect()
}

fn markup_namespace_matches(type_: &MarkupType, namespace: &str) -> bool {
    if let Some(using) = namespace.strip_prefix("using:") {
        let (name, assembly) = using
            .split_once(";assembly=")
            .map_or((using, None), |(name, assembly)| (name, Some(assembly)));
        return type_.namespace() == name
            && assembly.is_none_or(|assembly| {
                MarkupAssembly::of_module(type_.module_path)
                    .is_some_and(|candidate| candidate.name == assembly)
            });
    }
    if let Some(using) = namespace.strip_prefix("clr-namespace:") {
        let (name, assembly) = using
            .split_once(";assembly=")
            .map_or((using, None), |(name, assembly)| (name, Some(assembly)));
        return type_.namespace() == name
            && assembly.is_none_or(|assembly| {
                MarkupAssembly::of_module(type_.module_path)
                    .is_some_and(|candidate| candidate.name == assembly)
            });
    }
    MarkupAssembly::of_module(type_.module_path).is_some_and(|assembly| {
        assembly.xmlns_definitions.iter().any(|definition| {
            definition.xml_namespace == namespace && definition.namespace == type_.namespace()
        })
    })
}

fn is_numeric_type(name: &str) -> bool {
    matches!(
        name,
        "Byte"
            | "SByte"
            | "Int16"
            | "Int32"
            | "Int64"
            | "UInt16"
            | "UInt32"
            | "UInt64"
            | "Single"
            | "Double"
            | "Decimal"
            | "f32"
            | "f64"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "usize"
    )
}

fn enumeration(values: &[&str]) -> PropertyEditorKind {
    PropertyEditorKind::Enumeration {
        values: values.iter().map(|value| (*value).to_owned()).collect(),
        flags: false,
    }
}

fn validate_primitive(type_name: &str, source: &str) -> Result<(), String> {
    let value = source.trim();
    let valid = match type_identity(type_name) {
        "i8" => value.parse::<i8>().is_ok(),
        "i16" => value.parse::<i16>().is_ok(),
        "i32" => value.parse::<i32>().is_ok(),
        "i64" => value.parse::<i64>().is_ok(),
        "isize" => value.parse::<isize>().is_ok(),
        "u8" => value.parse::<u8>().is_ok(),
        "u16" => value.parse::<u16>().is_ok(),
        "u32" => value.parse::<u32>().is_ok(),
        "u64" => value.parse::<u64>().is_ok(),
        "usize" => value.parse::<usize>().is_ok(),
        "f32" => value.parse::<f32>().is_ok() || value == "Auto",
        "f64" => value.parse::<f64>().is_ok() || value == "Auto",
        _ => true,
    };
    if valid {
        Ok(())
    } else {
        Err(format!(
            "The value is outside the range or syntax of {type_name}"
        ))
    }
}
