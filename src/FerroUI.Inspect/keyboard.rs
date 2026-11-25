use super::{InspectResult, Permissions, input};
use ferroui_base::input::text_input::TextInputMethodClientRequestedEventArgs;
use ferroui_base::input::{
    FocusManager, InputElement, Key, KeyGesture, KeyModifiers, KeyboardNavigationHandler,
    NavigationDirection, NavigationMethod,
};
use ferroui_base::{FerroObject, Ref, Visual};
use ferroui_controls::automation::peers::ControlAutomationPeer;
use ferroui_controls::{Control, TopLevel};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum KeyboardInput {
    Focus,
    Key { key: String },
    Text { text: String },
}

#[derive(Serialize)]
pub struct KeyboardFocus {
    pub name: String,
    pub role: String,
}

pub(super) fn window_input(object: &Ref<FerroObject>, args: &Value) -> InspectResult {
    let window = object
        .cast::<TopLevel>()
        .ok_or("Choose an open app window")?;
    if window.platform_impl().is_none() {
        return Err("The app window is closed".into());
    }
    let input = &args["input"];
    let action = KeyboardInput::deserialize(input).map_err(|error| error.to_string())?;
    if matches!(action, KeyboardInput::Focus)
        && input.as_object().is_some_and(|fields| fields.len() != 1)
    {
        return Err("Focus takes only a kind".into());
    }
    let focus = keyboard_input(&window.upcast::<Control>(), action)?;
    serde_json::to_value(focus).map_err(|error| error.to_string())
}

pub(super) fn keyboard_schema() -> Value {
    let action = |kind: &str, field: &str, maximum: u64| {
        json!({"type":"object","properties":{"kind":{"const":kind},(field):{"type":"string","minLength":1,"maxLength":maximum}},
            "required":["kind",field],"additionalProperties":false})
    };
    json!({"oneOf":[
        {"type":"object","properties":{"kind":{"const":"focus"}},"required":["kind"],"additionalProperties":false},
        action("key", "key", 64),
        action("text", "text", 8192)
    ]})
}

pub fn keyboard_focus(scope: &Ref<Control>) -> Option<KeyboardFocus> {
    let top = TopLevel::get_top_level(Some(scope))?;
    describe_focus(scope, &top)
}

/// Deliver input through the presentation source of the current control, with
/// native focus traversal confined to the supplied visual scope.
pub fn keyboard_input(
    scope: &Ref<Control>,
    action: KeyboardInput,
) -> Result<Option<KeyboardFocus>, String> {
    let top = TopLevel::get_top_level(Some(scope)).ok_or("Mount the input surface first")?;
    let focused = focused(scope, &top);
    match action {
        KeyboardInput::Focus => {
            if focused.is_none() {
                move_focus(scope, None, KeyModifiers::NONE);
            }
        }
        KeyboardInput::Key { key } if !key.is_empty() && key.len() <= 64 => {
            let gesture = KeyGesture::parse(&key).map_err(|error| error.to_string())?;
            match gesture.key() {
                Key::None => return Err("Enter a valid key gesture".into()),
                Key::Tab => move_focus(scope, focused.as_ref(), gesture.key_modifiers()),
                Key::Space
                    if gesture
                        .key_modifiers()
                        .difference(KeyModifiers::SHIFT)
                        .is_empty()
                        && focused.as_ref().is_some_and(accepts_text) =>
                {
                    send_input(focused, "type_text", json!({"text":" "}))?;
                }
                _ => send_input(focused, "press_key", json!({"key":key}))?,
            }
        }
        KeyboardInput::Text { text } if !text.is_empty() && text.len() <= 8192 => {
            send_input(focused, "type_text", json!({"text":text}))?;
        }
        _ => return Err("Enter a key gesture or up to 8 KiB of text".into()),
    }
    Ok(describe_focus(scope, &top))
}

fn focused(scope: &Ref<Control>, top: &TopLevel) -> Option<Ref<InputElement>> {
    let visual = scope.upcast_ref::<Visual>();
    top.focus_manager().get_focused_element().filter(|input| {
        let input = input.upcast_ref::<Visual>();
        visual == input || visual.is_visual_ancestor_of(input)
    })
}

fn describe_focus(scope: &Ref<Control>, top: &TopLevel) -> Option<KeyboardFocus> {
    let control = focused(scope, top)?.cast::<Control>()?;
    let peer = ControlAutomationPeer::create_peer_for_element(&control);
    let name = peer.get_name();
    Some(KeyboardFocus {
        name: if name.is_empty() {
            control.name().unwrap_or_default()
        } else {
            name
        },
        role: format!("{:?}", peer.get_automation_control_type()),
    })
}

fn move_focus(scope: &Ref<Control>, from: Option<&Ref<InputElement>>, modifiers: KeyModifiers) {
    let scope = scope.clone().upcast::<InputElement>();
    if let Some(candidate) = tab_target(&scope, from, modifiers.contains(KeyModifiers::SHIFT)) {
        candidate.focus_with(NavigationMethod::Tab, modifiers);
    }
}

pub fn tab_target(
    scope: &Ref<InputElement>,
    from: Option<&Ref<InputElement>>,
    backwards: bool,
) -> Option<Ref<InputElement>> {
    let direction = if backwards {
        NavigationDirection::Previous
    } else {
        NavigationDirection::Next
    };
    let in_scope = |input: &Ref<InputElement>| {
        scope
            .upcast_ref::<Visual>()
            .is_visual_ancestor_of(input.upcast_ref::<Visual>())
    };
    let can_tab_to = |input: &Ref<InputElement>| {
        in_scope(input)
            && input.is_effectively_enabled()
            && input.is_effectively_visible()
            && input.focusable()
            && input.is_tab_stop()
    };
    let next = |input: &Ref<InputElement>| {
        KeyboardNavigationHandler::get_next(input, direction).filter(can_tab_to)
    };
    from.and_then(next).or_else(|| {
        let entry = if backwards {
            FocusManager::find_last_focusable_element_in(scope)
        } else {
            FocusManager::find_first_focusable_element_in(scope)
        }
        .filter(in_scope)?;
        // The focus manager's initial search can suggest a disabled element.
        // Native Tab traversal skips it in the same navigation group.
        if can_tab_to(&entry) {
            Some(entry)
        } else {
            next(&entry)
        }
    })
}

fn accepts_text(input: &Ref<InputElement>) -> bool {
    let query = TextInputMethodClientRequestedEventArgs::new();
    query.set_routed_event(Some(
        InputElement::text_input_method_client_requested_event(),
    ));
    input.raise_event(&query);
    query.client().is_some()
}

fn send_input(focused: Option<Ref<InputElement>>, tool: &str, args: Value) -> Result<(), String> {
    let control = focused
        .and_then(|input| input.cast::<Control>())
        .ok_or("Focus a control first")?;
    input::execute(
        tool,
        &control.upcast::<FerroObject>(),
        &args,
        Permissions {
            allow_input: true,
            ..Default::default()
        },
    )
}
