//! Tool contracts shared by transports. Validation happens before any UI operation.
use serde_json::{json, Value};

pub fn tools() -> Vec<Value> {
    let id = json!({"type":"integer","minimum":1});
    let string = json!({"type":"string"});
    let strings = json!({"type":"array","items":{"type":"string"},"maxItems":128});
    let mut out = Vec::new();
    let mut add = |name: &str,
                   description: &str,
                   properties: Value,
                   required: &[&str],
                   write: bool| {
        out.push(json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"annotations":{"readOnlyHint":!write,"destructiveHint":write,"openWorldHint":false}}));
    };
    add(
        "list_windows",
        "List live application windows and their node IDs.",
        json!({}),
        &[],
        false,
    );
    add("get_tree", "Snapshot a visual, logical or automation tree. Omit node_id for all windows. Depth defaults to 8; at most 10000 nodes.", json!({"node_id":id,"tree":{"enum":["visual","logical","automation"]},"depth":{"type":"integer","minimum":0,"maximum":64},"properties":strings}), &[], false);
    let search = json!({"node_id":id,"selector":string,"type":string,"name":string,"automation_id":string,"text":string,"limit":{"type":"integer","minimum":1,"maximum":10000}});
    add("find_elements", "Find elements by combined filters. Text is a substring; other fields are exact. Selectors use FerroUI syntax.", search.clone(), &[], false);
    add(
        "get_properties",
        "Read registered properties, types, read-only flags and effective value priorities.",
        json!({"node_id":id,"properties":strings}),
        &["node_id"],
        false,
    );
    add("set_property", "Set a local property value using the framework converters, or clear it. Requires allow_property_writes.", json!({"node_id":id,"property":string,"value":{"type":["boolean","number","string","null"]},"clear":{"type":"boolean"}}), &["node_id","property"], true);
    add(
        "get_styles",
        "Read applied style/theme/template/local frames, active states, priorities and setters.",
        json!({"node_id":id}),
        &["node_id"],
        false,
    );
    add(
        "get_resources",
        "Look up resource keys through the node's resource scope, or list its local dictionary.",
        json!({"node_id":id,"keys":strings}),
        &["node_id"],
        false,
    );
    add(
        "click",
        "Inject pointer move, down and up at the node's center or local x/y. Requires allow_input.",
        json!({"node_id":id,"x":{"type":"number"},"y":{"type":"number"},"button":{"enum":["left","right","middle"]}}),
        &["node_id"],
        true,
    );
    add(
        "type_text",
        "Focus the node and inject text through the input manager. Requires allow_input.",
        json!({"node_id":id,"text":string}),
        &["node_id", "text"],
        true,
    );
    add(
        "press_key",
        "Focus the node and inject a key gesture (e.g. Ctrl+A, Tab, Return). Requires allow_input.",
        json!({"node_id":id,"key":string}),
        &["node_id", "key"],
        true,
    );
    add(
        "scroll",
        "Inject a wheel event at the node center. Deltas are wheel units. Requires allow_input.",
        json!({"node_id":id,"delta_x":{"type":"number"},"delta_y":{"type":"number"}}),
        &["node_id", "delta_y"],
        true,
    );
    add("invoke_pattern", "Use an automation provider. Requires allow_input; set_value also requires allow_property_writes.", json!({"node_id":id,"pattern":{"enum":["invoke","toggle","expand","collapse","select","add_to_selection","remove_from_selection","set_value"]},"value":string}), &["node_id","pattern"], true);
    add(
        "screenshot",
        "Render a node (or the first window) as PNG at scale 0.1–4, bounded to 16 megapixels.",
        json!({"node_id":id,"scale":{"type":"number","minimum":0.1,"maximum":4}}),
        &[],
        false,
    );
    let mut wait = search;
    wait.as_object_mut().unwrap().extend(json!({"condition":{"enum":["exists","absent","visible","enabled","property_equals"]},"property":string,"value":{},"timeout_ms":{"type":"integer","minimum":0,"maximum":60000}}).as_object().unwrap().clone());
    add("wait_for", "Wait without blocking the UI for matching elements and a condition (default exists), up to timeout_ms (default 5000).", wait, &[], false);
    add("get_logs", "Read the bounded diagnostic log, including binding errors. Use after as a sequence cursor.", json!({"after":{"type":"integer","minimum":0}}), &[], false);
    add(
        "get_frame_stats",
        "Read observed rendered scene updates, update intervals and latest layout timing for each window.",
        json!({}),
        &[],
        false,
    );
    out
}

pub fn validate(name: &str, args: &Value) -> Result<(), String> {
    let tool = tools()
        .into_iter()
        .find(|tool| tool["name"] == name)
        .ok_or_else(|| format!("Unknown tool: {name}"))?;
    let schema = &tool["inputSchema"];
    let args = args.as_object().ok_or("arguments must be an object")?;
    for required in schema["required"].as_array().unwrap() {
        if !args.contains_key(required.as_str().unwrap()) {
            return Err(format!("Missing argument: {required}"));
        }
    }
    for (key, value) in args {
        let spec = schema["properties"]
            .get(key)
            .ok_or_else(|| format!("Unknown argument: {key}"))?;
        let valid = match spec["type"].as_str() {
            Some("integer") => value.as_u64().is_some(),
            Some("number") => value.as_f64().is_some_and(f64::is_finite),
            Some("string") => value.as_str().is_some_and(|s| s.len() <= 65536),
            Some("boolean") => value.is_boolean(),
            Some("array") => value
                .as_array()
                .is_some_and(|a| a.len() <= 128 && a.iter().all(Value::is_string)),
            _ => true,
        };
        if !valid {
            return Err(format!("Invalid type or size for {key}"));
        }
        if let Some(choices) = spec["enum"].as_array() {
            if !choices.contains(value) {
                return Err(format!("Invalid {key}: {value}"));
            }
        }
        if let Some(n) = value.as_f64() {
            if spec["minimum"].as_f64().is_some_and(|min| n < min)
                || spec["maximum"].as_f64().is_some_and(|max| n > max)
            {
                return Err(format!("{key} out of range"));
            }
        }
    }
    Ok(())
}
