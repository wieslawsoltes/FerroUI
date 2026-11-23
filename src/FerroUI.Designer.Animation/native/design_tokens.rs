//! DTCG transitions use the same checked native property edit as the designer.
use super::plan_xaml_property_transition;
use crate::xaml::escape;
use crate::{Easing, PropertyTransitionCommand, PropertyTransitionList, PropertyTransitionTiming};
use ferroui_designer_core::Document;
use ferroui_designer_tools::{
    token_handoff::{self, Direction, Options, Plan, TransitionMapping, TransitionTiming},
    Result as ToolResult,
};

const FRAMEWORK: &str = "https://github.com/ferroui";
const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

pub fn plan_design_tokens(
    direction: Direction,
    source: &str,
    options: &Options,
) -> ToolResult<Plan> {
    ferroui_base::register_types();
    token_handoff::plan_with_transitions(direction, source, options, render_transition)
}

fn render_transition(
    key: &str,
    target: &TransitionMapping,
    timing: TransitionTiming,
) -> Result<String, String> {
    let (owner, property) = target.parts()?;
    let (namespace, owner) = if let Some(namespace) = &target.namespace {
        (
            format!(" xmlns:target=\"{}\"", escape(namespace)),
            format!("target:{owner}"),
        )
    } else {
        (String::new(), owner.to_owned())
    };
    let key = escape(key);
    let mut document = Document::new(&format!(
        "<Transitions xmlns=\"{FRAMEWORK}\" xmlns:x=\"{XAML}\"{namespace} x:Key=\"{key}\"/>\n"
    ))
    .map_err(|error| error.to_string())?;
    let collection = PropertyTransitionList::read(&document, document.ast().root())
        .map_err(|error| error.to_string())?;
    let [x1, y1, x2, y2] = timing.curve;
    let command = PropertyTransitionCommand::Add {
        property: property.into(),
        type_name: target.class.clone(),
        timing: PropertyTransitionTiming {
            duration: Some(timing.duration_ms / 1000.0),
            delay: Some(timing.delay_ms / 1000.0),
            easing: Some(Easing::CubicBezier { x1, y1, x2, y2 }),
        },
    };
    let edits =
        plan_xaml_property_transition(&document, &collection, Some(&owner), |_| None, &command)?;
    document
        .apply("Import design transition", edits)
        .map_err(|error| error.to_string())?;
    Ok(format!(
        "    {}\n",
        document.source().trim_end().replace('\n', "\n    ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn review_uses_registered_type_and_round_trips_the_timing() {
        let tokens = json!({
            "fast":{"$type":"duration","$value":{"value":0.15,"unit":"s"}},
            "spline":{"$type":"cubicBezier","$value":[0.2,0.0,0.6,1.1]},
            "fade":{"$type":"transition","$value":{"duration":"{fast}","delay":{"value":25,"unit":"ms"},"timingFunction":"{spline}"}}
        });
        let source = tokens.to_string();
        let review = plan_design_tokens(Direction::Import, &source, &Options::default()).unwrap();
        assert_eq!(review.issues[0].kind.as_deref(), Some("transition"));
        let mut options = Options::default();
        options
            .mappings
            .entry("fade".into())
            .or_default()
            .transition = Some(TransitionMapping {
            property: "Visual.Opacity".into(),
            namespace: None,
            class: Some("ColorTransition".into()),
        });
        let invalid = plan_design_tokens(Direction::Import, &source, &options).unwrap();
        assert_eq!(invalid.issues.len(), 1);
        assert!(invalid.issues[0].message.contains("compatible property"));
        options
            .mappings
            .get_mut("fade")
            .unwrap()
            .transition
            .as_mut()
            .unwrap()
            .class = None;
        let imported = plan_design_tokens(Direction::Import, &source, &options).unwrap();
        assert!(imported.issues.is_empty(), "{:?}", imported.issues);
        assert!(imported.source.contains("<DoubleTransition"));
        assert!(imported.source.contains("Visual.OpacityProperty"));
        let exported =
            plan_design_tokens(Direction::Export, &imported.source, &Options::default()).unwrap();
        assert!(exported.issues.is_empty(), "{:?}", exported.issues);
        let tokens: Value = serde_json::from_str(&exported.source).unwrap();
        assert_eq!(
            tokens["fade"]["$value"],
            json!({
                "duration":{"value":150.0,"unit":"ms"}, "delay":{"value":25.0,"unit":"ms"}, "timingFunction":[0.2,0.0,0.6,1.1],
            })
        );
        let restored =
            plan_design_tokens(Direction::Import, &exported.source, &Options::default()).unwrap();
        assert!(restored.issues.is_empty(), "{:?}", restored.issues);
        assert!(restored.source.contains("Visual.OpacityProperty"));
    }
}
