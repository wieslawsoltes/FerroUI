use super::{
    bezier, native_type,
    references::{Resolved, Tokens},
    scalars::{duration, duration_ms},
    theme::trivia,
    ValueResult,
};
use crate::components::{key_attribute, FRAMEWORK, XAML};
use ferroui_designer_core::{Document, MarkupValue, Node};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub(super) const METADATA: &str = "transition";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TransitionMapping {
    pub property: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
}

impl TransitionMapping {
    pub fn parts(&self) -> ValueResult<(&str, &str)> {
        self.property
            .split_once('.')
            .filter(|(owner, property)| identifier(owner) && identifier(property))
            .ok_or_else(|| "Choose a registered Type.Property for this transition".into())
    }
}

#[derive(Clone, Copy)]
pub struct TransitionTiming {
    pub duration_ms: f64,
    pub delay_ms: f64,
    pub curve: [f64; 4],
}

pub type TransitionRenderer = fn(&str, &TransitionMapping, TransitionTiming) -> ValueResult<String>;

pub(super) struct Transition {
    pub binding: TransitionMapping,
    pub timing: TransitionTiming,
}

pub(super) fn import<'a>(
    tokens: &Tokens<'a>,
    resolved: &Resolved<'a>,
    mapping: Option<&TransitionMapping>,
) -> ValueResult<Transition> {
    let binding = if let Some(mapping) = mapping {
        mapping.clone()
    } else if let Some(metadata) = resolved.transition {
        serde_json::from_value(metadata.clone())
            .map_err(|error| format!("Invalid transition mapping: {error}"))?
    } else {
        return Err("Choose the registered target property, for example Visual.Opacity".into());
    };
    binding.parts()?;
    let field = |name, kind| {
        let value = resolved
            .value
            .get(name)
            .ok_or_else(|| format!("Missing transition {name}"))?;
        tokens.typed_property(value, kind)
    };
    let time = |name| duration_ms(&duration(tokens, field(name, "duration")?)?);
    let (curve, _) = bezier::export(&bezier::import(
        tokens,
        field("timingFunction", "cubicBezier")?,
    )?)?;
    Ok(Transition {
        binding,
        timing: TransitionTiming {
            duration_ms: time("duration")?,
            delay_ms: time("delay")?,
            curve,
        },
    })
}

pub(super) fn export(document: &Document, collection: &Node) -> ValueResult<Transition> {
    if collection
        .attributes
        .iter()
        .any(|attribute| !attribute.is_namespace() && Some(attribute) != key_attribute(collection))
    {
        return Err("Export a keyed Transitions collection without extra attributes".into());
    }
    let entries: Vec<_> = content(document, collection).collect();
    let [entry] = entries.as_slice() else {
        return Err("A DTCG transition maps to one property transition per resource".into());
    };
    if !native_type(entry, "FerroUI.Animation")
        || entry.attributes.iter().any(|attribute| {
            !attribute.is_namespace()
                && !matches!(
                    attribute.name.as_str(),
                    "Property" | "Duration" | "Delay" | "Easing"
                )
        })
    {
        return Err(
            "Export a native transition with literal Property, Duration, Delay and Easing".into(),
        );
    }
    let (property, namespace) = property(document, entry)?;
    let binding = TransitionMapping {
        property,
        namespace,
        class: Some(entry.local_name().to_owned()),
    };
    binding.parts()?;
    let timing = TransitionTiming {
        duration_ms: duration_ms(entry.attribute("Duration").unwrap_or("0:0:0"))?,
        delay_ms: duration_ms(entry.attribute("Delay").unwrap_or("0:0:0"))?,
        curve: curve(document, entry)?,
    };
    Ok(Transition { binding, timing })
}

impl Transition {
    pub(super) fn xaml(
        &self,
        key: &str,
        render: Option<TransitionRenderer>,
    ) -> ValueResult<String> {
        render.ok_or("A native transition catalog is required to verify the selected property")?(
            key,
            &self.binding,
            self.timing,
        )
    }

    pub(super) fn token_value(&self) -> Value {
        json!({
            "duration":{"value":self.timing.duration_ms,"unit":"ms"},
            "delay":{"value":self.timing.delay_ms,"unit":"ms"},
            "timingFunction":self.timing.curve,
        })
    }

    pub(super) fn summary(&self) -> String {
        format!(
            "{} · {} ms · {} ms delay",
            self.binding.property, self.timing.duration_ms, self.timing.delay_ms
        )
    }
}

fn property(document: &Document, entry: &Node) -> ValueResult<(String, Option<String>)> {
    let value = entry
        .attribute("Property")
        .ok_or("A transition needs a registered Property")?;
    let namespaces = document.ast().namespaces(entry.id);
    let (property, static_field) = match MarkupValue::parse(value.trim())
        .map_err(|error| error.to_string())?
    {
        MarkupValue::Literal(property) => (property, false),
        MarkupValue::Extension(extension) => {
            let Some((prefix, "Static")) = extension.name.split_once(':') else {
                return Err("Export a literal or x:Static transition property".into());
            };
            let [argument] = extension.arguments.as_slice() else {
                return Err("Export a single static transition property".into());
            };
            if namespaces.get(prefix).map(String::as_str) != Some(XAML) || argument.name.is_some() {
                return Err("Export a literal or x:Static transition property".into());
            }
            let MarkupValue::Literal(property) = &argument.value else {
                return Err("Export a literal or x:Static transition property".into());
            };
            (property.clone(), true)
        }
    };
    let (prefix, property) = property.split_once(':').unwrap_or(("", &property));
    let namespace = namespaces
        .get(prefix)
        .ok_or("The property owner namespace is unknown")?;
    let property = if static_field {
        property
            .strip_suffix("Property")
            .ok_or("Use a static FerroProperty field")?
    } else {
        property
    };
    Ok((
        property.into(),
        (namespace != FRAMEWORK).then(|| namespace.clone()),
    ))
}

fn curve(document: &Document, entry: &Node) -> ValueResult<[f64; 4]> {
    let children: Vec<_> = content(document, entry).collect();
    if children.is_empty() {
        return match entry.attribute("Easing") {
            None | Some("LinearEasing") => Ok([0.0, 0.0, 1.0, 1.0]),
            Some(easing) => Ok(bezier::export(easing)?.0),
        };
    }
    let invalid = "Export literal cubic Bézier or linear easing";
    let [field] = children.as_slice() else {
        return Err(invalid.into());
    };
    let property = format!("{}.Easing", entry.local_name());
    let contents: Vec<_> = content(document, field).collect();
    let [spline] = contents.as_slice() else {
        return Err(invalid.into());
    };
    let points = ["X1", "Y1", "X2", "Y2"];
    if field.local_name() != property
        || field.namespace_uri != entry.namespace_uri
        || field
            .attributes
            .iter()
            .any(|attribute| !attribute.is_namespace())
        || entry.attribute("Easing").is_some()
        || spline.local_name() != "SplineEasing"
        || !native_type(spline, "FerroUI.Animation.Easings")
        || content(document, spline).next().is_some()
        || spline.attributes.iter().any(|attribute| {
            !attribute.is_namespace() && !points.contains(&attribute.name.as_str())
        })
    {
        return Err(invalid.into());
    }
    let points = points
        .into_iter()
        .map(|name| {
            spline
                .attribute(name)
                .ok_or("Define four literal spline control points")
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(bezier::export(&points.join(","))?.0)
}

fn content<'a>(document: &'a Document, node: &'a Node) -> impl Iterator<Item = &'a Node> {
    node.children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|child| !trivia(child))
}

fn identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|ch| ch.is_alphabetic() || ch == '_')
        && chars.all(|ch| ch.is_alphanumeric() || ch == '_')
}
