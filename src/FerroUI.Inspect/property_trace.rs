//! A single presentation of native value precedence for the runtime and editors.
use crate::{Inspector, display_value, source};
use ferroui_base::data::BindingPriority;
use ferroui_base::diagnostics::{PropertyValueTrace, ValueContributorKind, ValueContributorState};
use ferroui_base::media::IBrush;
use ferroui_base::styling::{ControlTheme, Style};
use ferroui_base::{BoxedValue, FerroObject, Ref, StyledElement, TypeInfo};
use serde_json::{Value, json};
use std::rc::Rc;

pub struct InspectedPropertyTrace {
    pub property: String,
    pub effective: String,
    pub value_type: &'static str,
    pub read_only: bool,
    pub contributors: Vec<PropertyContributor>,
}

pub struct PropertyContributor {
    pub kind: ValueContributorKind,
    pub state: ValueContributorState,
    pub priority: BindingPriority,
    pub label: String,
    pub value: Option<String>,
    pub source: Option<Ref<FerroObject>>,
    pub binding: bool,
}

impl PropertyContributor {
    pub fn state_name(&self) -> &'static str {
        use ValueContributorState::*;
        match self.state {
            Effective => "effective",
            Base => "base",
            Overridden => "overridden",
            Inactive => "inactive",
            Unset => "unset",
        }
    }

    pub fn source_location(&self) -> Option<Value> {
        self.source
            .as_ref()
            .and_then(|object| source::location(object))
    }
}

impl InspectedPropertyTrace {
    /// Each host resolves sources in its own coordinate space. The order and
    /// status of contributors always come from the live value store.
    pub fn json(&self, locate: impl Fn(usize, &PropertyContributor) -> Option<Value>) -> Value {
        let contributors = self
            .contributors
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let priority = if entry.kind == ValueContributorKind::Direct {
                    "Direct".into()
                } else {
                    format!("{:?}", entry.priority)
                };
                json!({
                    "kind":kind_name(entry.kind), "state":entry.state_name(), "label":entry.label,
                    "priority":priority, "value":entry.value,
                    "binding":entry.binding, "source":locate(index, entry)
                })
            })
            .collect::<Vec<_>>();
        json!({"property":self.property,"effective":self.effective,"valueType":self.value_type,
            "readOnly":self.read_only,"contributors":contributors})
    }
}

impl Inspector {
    pub fn property_trace(
        object: &FerroObject,
        name: &str,
    ) -> Result<InspectedPropertyTrace, String> {
        let property = Self::property(object, name)?;
        let trace = PropertyValueTrace::read(object, property);
        let contributors = trace
            .contributors
            .into_iter()
            .map(|entry| PropertyContributor {
                kind: entry.kind,
                state: entry.state,
                priority: entry.priority,
                label: contributor_label(entry.kind, entry.source.as_ref(), entry.binding),
                value: entry.value.as_ref().map(trace_value),
                source: entry.source,
                binding: entry.binding,
            })
            .collect();
        Ok(InspectedPropertyTrace {
            property: name.to_owned(),
            effective: trace_value(trace.effective.value()),
            value_type: property.property_type_name(),
            read_only: property.is_read_only(),
            contributors,
        })
    }
}

fn kind_name(kind: ValueContributorKind) -> &'static str {
    use ValueContributorKind::*;
    match kind {
        Direct => "direct",
        CurrentValue => "currentValue",
        Animation => "animation",
        Local => "local",
        Style => "style",
        Theme => "theme",
        Template => "template",
        Inherited => "inherited",
        Default => "default",
    }
}

fn contributor_label(
    kind: ValueContributorKind,
    source: Option<&Ref<FerroObject>>,
    binding: bool,
) -> String {
    use ValueContributorKind as Kind;
    match kind {
        Kind::Direct => "Direct property".into(),
        Kind::CurrentValue => "Current value override".into(),
        Kind::Animation => "Animation".into(),
        Kind::Local if binding => "Local binding or resource".into(),
        Kind::Local => "Local value".into(),
        Kind::Style => source
            .and_then(|object| object.cast::<Style>())
            .and_then(|style| style.selector())
            .map_or_else(
                || "Style".into(),
                |selector| format!("Style {}", selector.to_string_with_owner(None)),
            ),
        Kind::Theme => source
            .and_then(|object| object.cast::<ControlTheme>())
            .and_then(|theme| theme.target_type())
            .map_or_else(
                || "Control theme".into(),
                |target| format!("Control theme {}", TypeInfo::name(target)),
            ),
        Kind::Template => "Template".into(),
        Kind::Inherited => {
            let Some(source) = source else {
                return "Inherited".into();
            };
            let name = source
                .cast::<StyledElement>()
                .and_then(|element| element.name());
            let type_name = source.get_type().name();
            match name {
                Some(name) => format!("Inherited from {type_name}#{name}"),
                None => format!("Inherited from {type_name}"),
            }
        }
        Kind::Default => "Type default".into(),
    }
}

/// Human-readable property values, including the effective brushes used by
/// both the isolated preview and the runtime inspector.
pub fn display_property_value(value: &BoxedValue) -> Option<String> {
    if let Some(value) = value.downcast_ref::<Option<Rc<dyn IBrush>>>() {
        return value.as_deref().map(display_brush);
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn IBrush>>() {
        return Some(display_brush(value.as_ref()));
    }
    Some(display_value(Some(value)))
}

fn trace_value(value: &BoxedValue) -> String {
    display_property_value(value).unwrap_or_else(|| "Null".into())
}

fn display_brush(brush: &dyn IBrush) -> String {
    if let Some(solid) = brush.as_solid_color_brush() {
        format!("#{:08X}", solid.color().to_uint32())
    } else if brush.as_gradient_brush().is_some() {
        "Gradient brush".into()
    } else {
        "Brush".into()
    }
}
