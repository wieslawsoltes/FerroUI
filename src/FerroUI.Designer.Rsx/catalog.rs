//! The two formats share the generated XAML schema. The entries here describe
//! the builder child placements and value types with a static codec in both formats.
use crate::literal::Literal;
use ferroui_designer_language::{PropertyInfo, Schema, FRAMEWORK_NAMESPACE};
use ferroui_view_syntax::snake_case;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Children {
    None,
    Panel,
    Single,
    Content,
}

pub(crate) fn children(class: &str) -> Option<Children> {
    Some(match class {
        "Canvas" | "DockPanel" | "Grid" | "StackPanel" | "UniformGrid" | "WrapPanel" => {
            Children::Panel
        }
        "Border" | "Viewbox" => Children::Single,
        "Button" | "CheckBox" | "ContentControl" | "RadioButton" | "ScrollViewer"
        | "ToggleButton" | "ToggleSwitch" | "UserControl" | "Window" => Children::Content,
        "ComboBox" | "Ellipse" | "Image" | "ProgressBar" | "Rectangle" | "Separator" | "Slider"
        | "TableView" | "TextBlock" | "TextBox" => Children::None,
        _ => return None,
    })
}

pub(crate) fn text_property(class: &str, placement: Children) -> Option<&'static str> {
    match (class, placement) {
        ("TextBlock", _) => Some("Text"),
        (_, Children::Content) => Some("Content"),
        _ => None,
    }
}

pub(crate) fn schema() -> &'static Schema {
    static SCHEMA: OnceLock<Schema> = OnceLock::new();
    SCHEMA.get_or_init(Schema::ferroui)
}

pub(crate) fn framework_type(class: &str, namespace: Option<&str>) -> bool {
    schema().type_info(class, namespace).is_some_and(|info| {
        info.namespace == FRAMEWORK_NAMESPACE && info.name == class && children(class).is_some()
    })
}

pub(crate) fn property(class: &str, name: &str) -> Option<Literal> {
    if name == "Name" || name == "Classes" {
        return Some(Literal::Text);
    }
    let info = schema().property(class, None, name)?;
    property_codec(&info)
}

pub(crate) fn property_codec(info: &PropertyInfo) -> Option<Literal> {
    if info.event || info.read_only {
        return None;
    }
    Some(match info.type_name.as_str() {
        "String"
        | "Option<String>"
        | "Option<BoxedValue>"
        | "Option<Rc<dyn IBrush>>"
        | "RowDefinitions"
        | "ColumnDefinitions" => Literal::Text,
        "bool" | "Option<bool>" => Literal::Bool,
        "i32" => Literal::Integer,
        "f64" => Literal::Number,
        "CornerRadius" => Literal::UniformRadius,
        "Thickness" => Literal::Thickness,
        "Option<Rc<dyn IImage>>" => Literal::EmbeddedImage,
        "HorizontalAlignment" | "VerticalAlignment" | "Orientation" => {
            Literal::enumeration("ferroui_base::layout", info)
        }
        "TextAlignment" | "TextWrapping" | "FlowDirection" | "Stretch" | "StretchDirection" => {
            Literal::enumeration("ferroui_base::media", info)
        }
        "Dock" => Literal::enumeration("ferroui_controls", info),
        _ => return None,
    })
}

pub(crate) fn from_setter(class: &str, owner: Option<&str>, setter: &str) -> Option<String> {
    if owner.is_none() && matches!(setter, "name" | "classes") {
        return Some(if setter == "name" { "Name" } else { "Classes" }.into());
    }
    let properties = schema().properties(owner.unwrap_or(class), None);
    let info = properties.values().find(|info| {
        (owner.is_none() || info.attached)
            && snake_case(&info.name) == setter
            && property_codec(info).is_some()
    })?;
    Some(match owner {
        Some(owner) => format!("{owner}.{}", info.name),
        None => info.name.clone(),
    })
}

pub(crate) fn setter(name: &str) -> String {
    let (owner, property) = name.split_once('.').unwrap_or(("", name));
    let mut property = snake_case(property);
    if matches!(
        property.as_str(),
        "type" | "loop" | "in" | "move" | "ref" | "match" | "where"
    ) {
        property.insert_str(0, "r#");
    }
    if owner.is_empty() {
        property
    } else {
        format!("{owner}.{property}")
    }
}
