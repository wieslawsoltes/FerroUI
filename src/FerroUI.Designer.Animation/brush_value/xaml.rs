use super::{
    AnimationBrush, AnimationGradient, AnimationGradientStop, BrushForm, GradientShape,
    GradientSpread,
};
use crate::{AnimationError, AnimationTransformList, Result, require};
use ferroui_base::{RelativePoint, RelativeScalar, media::Color};
use roxmltree::Node;

const NS: &str = "https://github.com/ferroui";

impl AnimationBrush {
    pub fn parse(source: &str) -> Result<Self> {
        if !source.trim_start().starts_with('<') {
            return Ok(Self::solid(argb(source)?));
        }
        let document = roxmltree::Document::parse(source)
            .map_err(|error| AnimationError(error.to_string()))?;
        Self::read_object(document.root_element())
    }

    pub(crate) fn is_brush_setter(setter: Node<'_, '_>) -> bool {
        setter.attribute("Value").is_none()
            && setter.descendants().any(|node| {
                node.is_element() && brush_namespace(node) && brush_class(node.tag_name().name())
            })
    }

    pub fn is_authored_brush_object(document: &ferroui_designer_core::Document, root: u64) -> bool {
        document.ast().subtree(root).any(|node| {
            node.namespace_uri.as_deref().is_none_or(|ns| {
                matches!(
                    ns,
                    NS | "using:FerroUI.Media"
                        | "clr-namespace:FerroUI.Media;assembly=FerroUI.Base"
                )
            }) && brush_class(node.local_name())
        })
    }

    pub(crate) fn read_setter(setter: Node<'_, '_>) -> Result<Self> {
        if let Some(value) = setter.attribute("Value") {
            require(
                setter.children().all(trivia),
                "Choose one brush value per keyframe",
            )?;
            let value = value.strip_prefix("{}").unwrap_or(value);
            return Ok(Self::solid(argb(value)?));
        }
        Self::read_object(only_value(setter)?)
    }

    fn read_object(node: Node<'_, '_>) -> Result<Self> {
        let wrapped = node.tag_name().name() == "Setter.Value";
        let object = if wrapped {
            require(
                crate::xaml::native_namespace(node.tag_name().namespace())
                    && node.attributes().len() == 0,
                "Choose a native Setter.Value property element",
            )?;
            only_value(node)?
        } else {
            node
        };
        require(brush_namespace(object), "Choose a native brush value")?;
        let class = object.tag_name().name();
        let value = if class == "SolidColorBrush" {
            attributes(object, &["Color", "Opacity"])?;
            require(
                object.children().all(trivia),
                "Use literal solid brush fields",
            )?;
            Self::Solid {
                argb: object
                    .attribute("Color")
                    .map(argb)
                    .transpose()?
                    .unwrap_or_default(),
                opacity: number(object, "Opacity", 1.0)?,
                form: if wrapped {
                    BrushForm::PropertyElement
                } else {
                    BrushForm::Object
                },
            }
        } else {
            Self::Gradient {
                value: Box::new(gradient(object, class)?),
                wrapped,
            }
        };
        value.validate()?;
        Ok(value)
    }
}

fn gradient(object: Node<'_, '_>, class: &str) -> Result<AnimationGradient> {
    let shape = GradientShape::defaults(class).ok_or_else(|| {
        AnimationError("Choose a native linear, radial, or conic gradient".into())
    })?;
    let fields = shape.fields();
    let mut allowed = vec![
        "Opacity",
        "SpreadMethod",
        "TransformOrigin",
        "Transform",
        "RelativeTransform",
    ];
    allowed.extend(fields.iter().map(|(name, _)| *name));
    attributes(object, &allowed)?;
    let shape = match shape {
        GradientShape::Linear { start, end } => GradientShape::Linear {
            start: point(object, "StartPoint", start.native())?.into(),
            end: point(object, "EndPoint", end.native())?.into(),
        },
        GradientShape::Radial {
            center,
            origin,
            radius_x,
            radius_y,
        } => GradientShape::Radial {
            center: point(object, "Center", center.native())?.into(),
            origin: point(object, "GradientOrigin", origin.native())?.into(),
            radius_x: distance(object, "RadiusX", radius_x.native())?.into(),
            radius_y: distance(object, "RadiusY", radius_y.native())?.into(),
        },
        GradientShape::Conic { center, angle } => GradientShape::Conic {
            center: point(object, "Center", center.native())?.into(),
            angle: number(object, "Angle", angle)?,
        },
    };
    let spread = GradientSpread::parse(object.attribute("SpreadMethod").unwrap_or("Pad"))
        .ok_or_else(|| AnimationError("Use Pad, Reflect, or Repeat for SpreadMethod".into()))?;
    let stop_nodes = elements(object)?;
    let wrapper = format!("{class}.GradientStops");
    let stop_nodes = match stop_nodes.as_slice() {
        [node]
            if brush_namespace(*node)
                && (node.tag_name().name() == wrapper
                    || node.tag_name().name() == "GradientBrush.GradientStops") =>
        {
            attributes(*node, &[])?;
            elements(*node)?
        }
        _ => stop_nodes,
    };
    let stops = stop_nodes.into_iter().map(stop).collect::<Result<_>>()?;
    Ok(AnimationGradient {
        shape,
        stops,
        opacity: number(object, "Opacity", 1.0)?,
        spread,
        transform_origin: point(object, "TransformOrigin", RelativePoint::default())?.into(),
        transform: object
            .attribute("Transform")
            .map(AnimationTransformList::parse)
            .transpose()?,
        relative_transform: object
            .attribute("RelativeTransform")
            .map(AnimationTransformList::parse)
            .transpose()?,
    })
}

fn stop(node: Node<'_, '_>) -> Result<AnimationGradientStop> {
    require(
        brush_namespace(node)
            && node.tag_name().name() == "GradientStop"
            && node.children().all(trivia),
        "Use native GradientStop elements with literal fields",
    )?;
    attributes(node, &["Offset", "Color"])?;
    Ok(AnimationGradientStop {
        offset: number(node, "Offset", 0.0)?,
        argb: node
            .attribute("Color")
            .map(argb)
            .transpose()?
            .unwrap_or_default(),
    })
}

fn literal<T>(
    node: Node<'_, '_>,
    name: &str,
    default: T,
    parse: impl Fn(&str) -> Option<T>,
) -> Result<T> {
    node.attribute(name).map_or(Ok(default), |text| {
        parse(text).ok_or_else(|| AnimationError(format!("{name} requires a literal value")))
    })
}

fn number(node: Node<'_, '_>, name: &str, default: f64) -> Result<f64> {
    literal(node, name, default, |text| text.trim().parse().ok())
}

fn point(node: Node<'_, '_>, name: &str, default: RelativePoint) -> Result<RelativePoint> {
    literal(node, name, default, |text| RelativePoint::parse(text).ok())
}

fn distance(node: Node<'_, '_>, name: &str, default: RelativeScalar) -> Result<RelativeScalar> {
    literal(node, name, default, |text| RelativeScalar::parse(text).ok())
}

fn argb(source: &str) -> Result<[u8; 4]> {
    let color = Color::try_parse(source.trim())
        .ok_or_else(|| AnimationError("Brush colors require a literal native color".into()))?;
    Ok([color.a, color.r, color.g, color.b])
}

fn only_value<'a, 'i>(node: Node<'a, 'i>) -> Result<Node<'a, 'i>> {
    match elements(node)?.as_slice() {
        [value] => Ok(*value),
        _ => Err(AnimationError(
            "Choose one native brush object per keyframe".into(),
        )),
    }
}

fn elements<'a, 'i>(node: Node<'a, 'i>) -> Result<Vec<Node<'a, 'i>>> {
    require(
        node.children()
            .all(|child| child.is_element() || trivia(child)),
        "Use a static brush object",
    )?;
    Ok(node.children().filter(Node::is_element).collect())
}

fn trivia(node: Node<'_, '_>) -> bool {
    node.is_comment() || node.is_text() && node.text().is_none_or(|text| text.trim().is_empty())
}

fn attributes(node: Node<'_, '_>, allowed: &[&str]) -> Result<()> {
    require(
        node.attributes()
            .all(|attr| attr.namespace().is_none() && allowed.contains(&attr.name())),
        "Use literal attributes on the native brush",
    )
}

fn brush_namespace(node: Node<'_, '_>) -> bool {
    node.tag_name().namespace().is_none_or(|namespace| {
        matches!(
            namespace,
            NS | "using:FerroUI.Media" | "clr-namespace:FerroUI.Media;assembly=FerroUI.Base"
        )
    })
}

fn brush_class(name: &str) -> bool {
    matches!(
        name,
        "SolidColorBrush" | "LinearGradientBrush" | "RadialGradientBrush" | "ConicGradientBrush"
    )
}
