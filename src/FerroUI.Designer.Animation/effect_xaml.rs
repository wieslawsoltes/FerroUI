use super::{
    require, AnimationEffect, AnimationError, AnimationValue, EffectForm, Result, ValueKind,
};
use crate::xaml::EffectColorResolver;
use roxmltree::Node;

impl AnimationEffect {
    pub(super) fn parse_object(source: &str, color: Option<EffectColorResolver>) -> Result<Self> {
        let document = roxmltree::Document::parse(source)
            .map_err(|error| AnimationError(error.to_string()))?;
        Self::read_object(document.root_element(), color)
    }

    pub(crate) fn read_setter_object(
        setter: Node<'_, '_>,
        color: Option<EffectColorResolver>,
    ) -> Result<Self> {
        Self::read_object(only_value(setter)?, color)
    }

    fn read_object(node: Node<'_, '_>, color: Option<EffectColorResolver>) -> Result<Self> {
        let (object, form) = if node.tag_name().name() == "Setter.Value" {
            require(
                crate::xaml::native_namespace(node.tag_name().namespace())
                    && node.attributes().len() == 0,
                "Choose a native Setter.Value property element",
            )?;
            (only_value(node)?, EffectForm::PropertyElement)
        } else {
            (node, EffectForm::Object)
        };
        require(
            object.tag_name().namespace().is_none_or(|namespace| {
                matches!(
                    namespace,
                    "https://github.com/ferroui"
                        | "using:FerroUI.Media"
                        | "clr-namespace:FerroUI.Media;assembly=FerroUI.Base"
                )
            }) && object
                .children()
                .all(|child| child.is_comment() || whitespace(child)),
            "Only static built-in effect objects are available in typed keyframes",
        )?;
        let number = |attribute: &str, default: f64| {
            object.attribute(attribute).map_or(Ok(default), |text| {
                text.trim()
                    .parse::<f64>()
                    .map_err(|_| AnimationError(format!("{attribute} requires a literal number")))
            })
        };
        let kind = object.tag_name().name();
        if kind == "BlurEffect" {
            attributes(object, &["Radius"])?;
            let effect = Self::Blur {
                radius: number("Radius", 5.0)?,
                form,
            };
            effect.validate()?;
            return Ok(effect);
        }
        let geometry =
            match kind {
                "DropShadowEffect" => ["OffsetX", "OffsetY"],
                "DropShadowDirectionEffect" => ["Direction", "ShadowDepth"],
                _ => return Err(AnimationError(
                    "Choose a native BlurEffect, DropShadowEffect, or DropShadowDirectionEffect"
                        .into(),
                )),
            };
        attributes(
            object,
            &[geometry[0], geometry[1], "BlurRadius", "Opacity", "Color"],
        )?;
        let blur_radius = number("BlurRadius", 5.0)?;
        let opacity = number("Opacity", 1.0)?;
        let argb = match object.attribute("Color") {
            Some(text) => color
                .and_then(|resolve| resolve(text))
                .or_else(|| AnimationValue::parse(text, ValueKind::Color).ok()?.argb8())
                .ok_or_else(|| AnimationError("Effect Color requires a literal color".into()))?,
            None => [255, 0, 0, 0],
        };
        let effect = if kind == "DropShadowEffect" {
            Self::DropShadow {
                offset_x: number("OffsetX", 3.5355)?,
                offset_y: number("OffsetY", 3.5355)?,
                blur_radius,
                argb,
                opacity,
                form,
            }
        } else {
            Self::DirectionalDropShadow {
                direction: number("Direction", 315.0)?,
                shadow_depth: number("ShadowDepth", 5.0)?,
                blur_radius,
                argb,
                opacity,
                form,
            }
        };
        effect.validate()?;
        Ok(effect)
    }

    pub(super) fn object_xaml(self) -> String {
        self.write_object("")
    }

    pub(crate) fn setter_object_xaml(self) -> String {
        self.write_object(r#" xmlns="https://github.com/ferroui""#)
    }

    fn write_object(self, namespace: &str) -> String {
        let (tag, fields) = match self {
            Self::Blur { radius, .. } => ("BlurEffect", format!(r#"Radius="{radius}""#)),
            Self::DropShadow {
                offset_x,
                offset_y,
                blur_radius,
                opacity,
                argb,
                ..
            } => (
                "DropShadowEffect",
                format!(
                    r##"OffsetX="{offset_x}" OffsetY="{offset_y}" BlurRadius="{blur_radius}" Color="#{:08X}" Opacity="{opacity}""##,
                    u32::from_be_bytes(argb)
                ),
            ),
            Self::DirectionalDropShadow {
                direction,
                shadow_depth,
                blur_radius,
                opacity,
                argb,
                ..
            } => (
                "DropShadowDirectionEffect",
                format!(
                    r##"Direction="{direction}" ShadowDepth="{shadow_depth}" BlurRadius="{blur_radius}" Color="#{:08X}" Opacity="{opacity}""##,
                    u32::from_be_bytes(argb)
                ),
            ),
        };
        let object = format!("<{tag}{namespace} {fields} />");
        if self.form() == EffectForm::PropertyElement {
            format!("<Setter.Value>{object}</Setter.Value>")
        } else {
            object
        }
    }
}

fn only_value<'a, 'input>(parent: Node<'a, 'input>) -> Result<Node<'a, 'input>> {
    let mut nodes = parent.children().filter(Node::is_element);
    let value = nodes
        .next()
        .ok_or_else(|| AnimationError("Setter.Value is required".into()))?;
    require(
        nodes.next().is_none()
            && parent
                .children()
                .all(|child| child.is_element() || child.is_comment() || whitespace(child)),
        "Choose one static effect object per setter",
    )?;
    Ok(value)
}

fn whitespace(node: Node<'_, '_>) -> bool {
    node.is_text() && node.text().is_none_or(|text| text.trim().is_empty())
}

fn attributes(object: Node<'_, '_>, allowed: &[&str]) -> Result<()> {
    require(
        object.attributes().all(|attribute| {
            attribute.namespace().is_none() && allowed.contains(&attribute.name())
        }),
        "Use literal attributes on the native effect object",
    )
}
