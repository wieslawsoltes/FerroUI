use super::{decode, node_id, schema as s, state};
use crate::DesignerSession;
use ferroui_designer_animation::PropertyTransitionTiming;
use ferroui_designer_language::LineIndex;
use serde_json::{json, Value};

const QUERY: &str = "designer_property_transitions";
const EDIT: &str = "designer_property_transition_edit";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, QUERY | EDIT)
}

pub(super) fn tools() -> [Value; 2] {
    let timing = s::object(
        json!({"duration":s::time(),"delay":s::time(),"easing":s::easing()}),
        &[],
    );
    [
        s::tool(
            QUERY,
            "Read a control or style's authored XAML property transitions, source ranges, and editable timing fields.",
            s::object(json!({"owner":s::id()}), &["owner"]),
            false,
        ),
        s::actions(
            EDIT,
            "Change literal property transition timing in the current XAML revision. Times are in seconds. Omitted fields retain their authored values.",
            vec![s::action("timing", json!({"owner":s::id(),"transition":s::id(),"timing":timing}),
                &["owner","transition","timing"], true)],
            true,
        ),
    ]
}

pub(super) fn call(
    session: &mut DesignerSession,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    let owner = node_id(args, "owner")?;
    if name == EDIT {
        session.edit_property_transition_timing(
            owner,
            node_id(args, "transition")?,
            &decode::<PropertyTransitionTiming>(args, "timing")?,
        )?;
        return Ok(json!({"document":state(session)}));
    }
    let model = session.property_transitions(owner)?;
    let source = LineIndex::new(session.document().valid_source());
    let entries = model
        .entries
        .iter()
        .map(|entry| {
            json!({
                "id":entry.id, "typeName":entry.type_name, "property":entry.property,
                "duration":entry.duration, "delay":entry.delay, "easing":entry.easing,
            "durationSeconds":entry.duration_seconds, "delaySeconds":entry.delay_seconds,
                "editableTiming":entry.editable_timing, "range":source.range(entry.source).ok()
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "owner":owner, "collection":model.collection, "canEdit":session.can_manipulate(),
        "range":model.source.and_then(|span| source.range(span).ok()), "entries":entries
    }))
}
