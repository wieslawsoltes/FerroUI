use super::{Inspector, Permissions};
use ferroui_base::input::raw::{
    RawKeyEventArgs, RawKeyEventType, RawMouseWheelEventArgs, RawPointerEventArgs,
    RawPointerEventType, RawTextInputEventArgs,
};
use ferroui_base::input::{
    InputManager, Key, KeyDeviceType, KeyGesture, KeyboardDevice, MouseDevice, PhysicalKey,
    RawInputModifiers,
};
use ferroui_base::{FerroObject, Point, Ref, Vector};
use ferroui_controls::automation::provider::*;
use ferroui_controls::{Control, TopLevel};
use serde_json::Value;
use std::rc::Rc;
use std::time::Instant;

pub(super) fn execute(
    name: &str,
    object: &Ref<FerroObject>,
    args: &Value,
    permissions: Permissions,
) -> Result<(), String> {
    if name == "invoke_pattern" {
        let peer = Inspector::peer(object)?;
        let unsupported = || "Automation pattern is not supported by this node".to_owned();
        let pattern = args["pattern"].as_str().unwrap();
        if pattern == "set_value" {
            if !permissions.allow_property_writes {
                return Err("Property writes are disabled".into());
            }
            let value = args["value"]
                .as_str()
                .ok_or("value required for set_value")?;
            let provider = peer
                .get_provider::<dyn IValueProvider>()
                .ok_or_else(unsupported)?;
            if provider.is_read_only() {
                return Err("Value is read-only".into());
            }
            return provider.set_value(Some(value)).map_err(|e| e.to_string());
        }
        return match pattern {
            "invoke" => peer
                .get_provider::<dyn IInvokeProvider>()
                .ok_or_else(unsupported)?
                .invoke(),
            "toggle" => peer
                .get_provider::<dyn IToggleProvider>()
                .ok_or_else(unsupported)?
                .toggle(),
            "expand" => peer
                .get_provider::<dyn IExpandCollapseProvider>()
                .ok_or_else(unsupported)?
                .expand(),
            "collapse" => peer
                .get_provider::<dyn IExpandCollapseProvider>()
                .ok_or_else(unsupported)?
                .collapse(),
            "select" => peer
                .get_provider::<dyn ISelectionItemProvider>()
                .ok_or_else(unsupported)?
                .select(),
            "add_to_selection" => peer
                .get_provider::<dyn ISelectionItemProvider>()
                .ok_or_else(unsupported)?
                .add_to_selection(),
            "remove_from_selection" => peer
                .get_provider::<dyn ISelectionItemProvider>()
                .ok_or_else(unsupported)?
                .remove_from_selection(),
            _ => return Err("Unknown pattern".into()),
        }
        .map_err(|e| e.to_string());
    }
    let control = Inspector::visual_object(object)
        .cast::<Control>()
        .ok_or("Node is not a control")?;
    if !control.is_effectively_enabled() || !control.is_effectively_visible() {
        return Err("Control is disabled or invisible".into());
    }
    let root =
        TopLevel::get_top_level(Some(&control)).ok_or("Control is not attached to a top level")?;
    if root.platform_impl().is_none() {
        return Err("Window is closed".into());
    }
    let manager = InputManager::instance().ok_or("Input manager is unavailable")?;
    let input_root = root.input_root();
    // One monotonic clock for all injected events (including multi-click recognition).
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let timestamp = START.get_or_init(Instant::now).elapsed().as_millis() as u64;
    if name == "type_text" || name == "press_key" {
        if !control.focus() {
            return Err("Control cannot receive keyboard focus".into());
        }
        let keyboard = KeyboardDevice::instance().ok_or("Keyboard is unavailable")?;
        if name == "type_text" {
            manager.process_input(Rc::new(RawTextInputEventArgs::new(
                keyboard,
                timestamp,
                input_root,
                args["text"].as_str().unwrap(),
            )));
        } else {
            let gesture =
                KeyGesture::parse(args["key"].as_str().unwrap()).map_err(|e| e.to_string())?;
            if gesture.key() == Key::None {
                return Err("Gesture has no key".into());
            }
            let modifiers = RawInputModifiers::from_bits_truncate(gesture.key_modifiers().bits());
            for event in [RawKeyEventType::KeyDown, RawKeyEventType::KeyUp] {
                manager.process_input(Rc::new(RawKeyEventArgs::new(
                    keyboard.clone(),
                    timestamp,
                    input_root.clone(),
                    event,
                    gesture.key(),
                    modifiers,
                    PhysicalKey::None,
                    None,
                    KeyDeviceType::Keyboard,
                )));
            }
        }
        return Ok(());
    }
    let local = Point::new(
        args["x"].as_f64().unwrap_or(control.bounds().width / 2.0),
        args["y"].as_f64().unwrap_or(control.bounds().height / 2.0),
    );
    let transform = control
        .transform_to_visual(&root.clone().upcast())
        .ok_or("Cannot transform node to window")?;
    let point = local * transform;
    let mouse = MouseDevice::primary();
    if name == "scroll" {
        manager.process_input(Rc::new(RawMouseWheelEventArgs::new(
            mouse,
            timestamp,
            input_root,
            point,
            Vector::new(
                args["delta_x"].as_f64().unwrap_or(0.0),
                args["delta_y"].as_f64().unwrap(),
            ),
            RawInputModifiers::NONE,
        )));
    } else {
        let (down, up, pressed) = match args["button"].as_str().unwrap_or("left") {
            "right" => (
                RawPointerEventType::RightButtonDown,
                RawPointerEventType::RightButtonUp,
                RawInputModifiers::RIGHT_MOUSE_BUTTON,
            ),
            "middle" => (
                RawPointerEventType::MiddleButtonDown,
                RawPointerEventType::MiddleButtonUp,
                RawInputModifiers::MIDDLE_MOUSE_BUTTON,
            ),
            _ => (
                RawPointerEventType::LeftButtonDown,
                RawPointerEventType::LeftButtonUp,
                RawInputModifiers::LEFT_MOUSE_BUTTON,
            ),
        };
        for (event, modifiers) in [
            (RawPointerEventType::Move, RawInputModifiers::NONE),
            (down, pressed),
            (up, RawInputModifiers::NONE),
        ] {
            manager.process_input(Rc::new(RawPointerEventArgs::new(
                mouse.clone(),
                timestamp,
                input_root.clone(),
                event,
                point,
                modifiers,
            )));
        }
    }
    Ok(())
}
