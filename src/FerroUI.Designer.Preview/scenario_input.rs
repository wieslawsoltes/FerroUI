//! Input and assertions against the same native inspector used by the designer.
use ferroui_base::{threading::Dispatcher, Ref};
use ferroui_controls::Control;
use ferroui_designer_tools::scenario::ScenarioStep;
use ferroui_inspect::{Inspector, Permissions};
use serde_json::{json, Value};

pub(crate) fn execute(host: &Ref<Control>, step: ScenarioStep) -> Result<String, String> {
    Dispatcher::ui_thread().run_jobs(None);
    host.update_layout();
    let shared = Inspector::shared();
    let mut inspector = shared.borrow_mut();
    let root = inspector.node_id(host);
    let permissions = Permissions {
        allow_input: true,
        ..Default::default()
    };
    match step {
        ScenarioStep::ExpectAccessible {
            name,
            role,
            automation_id,
        } => {
            let tree = inspector.execute(
                "get_tree",
                &json!({"node_id":root,"tree":"automation","depth":64}),
                permissions,
            )?;
            if !accessible(&tree, &name, &role, automation_id.as_deref()) {
                return Err(format!("Expected an accessible {role} named {name:?}"));
            }
            Ok(format!("Found accessible {role} {name:?}"))
        }
        ScenarioStep::ExpectProperty {
            selector,
            property,
            value,
        } => {
            let node = find(&mut inspector, root, &selector, permissions)?;
            let properties = inspector.execute(
                "get_properties",
                &json!({"node_id":node,"properties":[property]}),
                permissions,
            )?;
            let actual = properties
                .get(0)
                .and_then(|property| property.get("value"))
                .ok_or_else(|| format!("{selector:?} has no property {property:?}"))?;
            let expected = json!(value);
            if actual != &expected
                && !actual
                    .as_f64()
                    .zip(expected.as_f64())
                    .is_some_and(|(a, b)| a == b)
            {
                return Err(format!(
                    "{selector}.{property} expected {expected}, received {actual}"
                ));
            }
            Ok(format!("{selector}.{property} matched {expected}"))
        }
        ScenarioStep::Click { selector } => {
            let node = find(&mut inspector, root, &selector, permissions)?;
            inspector.execute("click", &json!({"node_id":node}), permissions)?;
            Ok(format!("Clicked {selector}"))
        }
        ScenarioStep::TypeText { selector, text } => {
            let node = find(&mut inspector, root, &selector, permissions)?;
            inspector.execute(
                "type_text",
                &json!({"node_id":node,"text":text}),
                permissions,
            )?;
            Ok(format!("Typed in {selector}"))
        }
        ScenarioStep::PressKey { selector, key } => {
            let node = find(&mut inspector, root, &selector, permissions)?;
            inspector.execute("press_key", &json!({"node_id":node,"key":key}), permissions)?;
            Ok(format!("Sent {key} to {selector}"))
        }
        ScenarioStep::ExpectScreenshot { .. } => unreachable!(),
        ScenarioStep::ActivateLink { .. }
        | ScenarioStep::PressActiveKey { .. }
        | ScenarioStep::ExpectFocus { .. }
        | ScenarioStep::ExpectStage { .. }
        | ScenarioStep::ExpectOverlay { .. } => {
            Err("This step requires a prototype stage set".into())
        }
    }
}

fn find(
    inspector: &mut Inspector,
    root: u64,
    selector: &str,
    permissions: Permissions,
) -> Result<u64, String> {
    let matches = inspector.execute(
        "find_elements",
        &json!({"node_id":root,"selector":selector,"limit":2}),
        permissions,
    )?;
    match matches.as_array().map(Vec::as_slice) {
        Some([item]) => item["node_id"]
            .as_u64()
            .ok_or_else(|| "The matching control has no runtime identity".into()),
        _ => Err(format!(
            "Expected exactly one control matching {selector:?}"
        )),
    }
}

fn accessible(tree: &Value, name: &str, role: &str, automation_id: Option<&str>) -> bool {
    if let Some(roots) = tree.as_array() {
        return roots
            .iter()
            .any(|root| accessible(root, name, role, automation_id));
    }
    (tree["text"] == name
        && tree["role"] == role
        && automation_id.is_none_or(|id| tree["automation_id"] == id))
        || tree
            .get("children")
            .is_some_and(|children| accessible(children, name, role, automation_id))
}
