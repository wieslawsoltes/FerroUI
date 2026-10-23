use crate::protocol::{number, string, Error, Result};
use ferroui_base::input::raw::*;
use ferroui_base::input::{
    InputManager, Key, KeyDeviceType, KeyboardDevice, MouseDevice, PhysicalKey, RawInputModifiers,
    TouchDevice,
};
use ferroui_base::{Point, Ref, Vector};
use ferroui_controls::TopLevel;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Instant;

pub(crate) struct Input {
    touch: Rc<TouchDevice>,
    points: BTreeMap<i64, Point>,
    started: Instant,
}
impl Default for Input {
    fn default() -> Self {
        Self {
            touch: TouchDevice::new(),
            points: BTreeMap::new(),
            started: Instant::now(),
        }
    }
}
impl Input {
    pub fn dispatch(&mut self, root: &Ref<TopLevel>, method: &str, p: &Value) -> Result<Value> {
        let manager = InputManager::instance()
            .ok_or_else(|| Error::server("Input manager is unavailable"))?;
        let timestamp = self.started.elapsed().as_millis() as u64;
        let mut modifiers = RawInputModifiers::empty();
        let flags = p
            .get("modifiers")
            .map(|v| v.as_u64().ok_or_else(|| Error::params("Invalid modifiers")))
            .transpose()?
            .unwrap_or(0);
        for (bit, modifier) in [
            (1, RawInputModifiers::ALT),
            (2, RawInputModifiers::CONTROL),
            (4, RawInputModifiers::META),
            (8, RawInputModifiers::SHIFT),
        ] {
            if flags & bit != 0 {
                modifiers |= modifier;
            }
        }
        match method {
            "Input.dispatchMouseEvent" => {
                let position = Point::new(number(p, "x")?, number(p, "y")?);
                let buttons = p["buttons"].as_u64().unwrap_or(0);
                for (bit, modifier) in [
                    (1, RawInputModifiers::LEFT_MOUSE_BUTTON),
                    (2, RawInputModifiers::RIGHT_MOUSE_BUTTON),
                    (4, RawInputModifiers::MIDDLE_MOUSE_BUTTON),
                ] {
                    if buttons & bit != 0 {
                        modifiers |= modifier;
                    }
                }
                let type_ = string(p, "type")?;
                if type_ == "mouseWheel" {
                    // CDP wheel deltas are pixels; the raw platform contract uses wheel notches.
                    manager.process_input(Rc::new(RawMouseWheelEventArgs::new(
                        MouseDevice::primary(),
                        timestamp,
                        root.input_root(),
                        position,
                        Vector::new(-number(p, "deltaX")? / 120.0, -number(p, "deltaY")? / 120.0),
                        modifiers,
                    )));
                } else {
                    let button = p["button"].as_str().unwrap_or("none");
                    let kind = match (type_, button) {
                        ("mouseMoved", _) => RawPointerEventType::Move,
                        ("mousePressed", "left") => RawPointerEventType::LeftButtonDown,
                        ("mouseReleased", "left") => RawPointerEventType::LeftButtonUp,
                        ("mousePressed", "right") => RawPointerEventType::RightButtonDown,
                        ("mouseReleased", "right") => RawPointerEventType::RightButtonUp,
                        ("mousePressed", "middle") => RawPointerEventType::MiddleButtonDown,
                        ("mouseReleased", "middle") => RawPointerEventType::MiddleButtonUp,
                        _ => return Err(Error::params("Unsupported mouse type/button")),
                    };
                    let button_modifier = match button {
                        "left" => RawInputModifiers::LEFT_MOUSE_BUTTON,
                        "right" => RawInputModifiers::RIGHT_MOUSE_BUTTON,
                        "middle" => RawInputModifiers::MIDDLE_MOUSE_BUTTON,
                        _ => RawInputModifiers::empty(),
                    };
                    if type_ == "mousePressed" {
                        modifiers |= button_modifier;
                    }
                    if type_ == "mouseReleased" {
                        modifiers.remove(button_modifier);
                    }
                    manager.process_input(Rc::new(RawPointerEventArgs::new(
                        MouseDevice::primary(),
                        timestamp,
                        root.input_root(),
                        kind,
                        position,
                        modifiers,
                    )));
                }
            }
            "Input.insertText" => self.text(root, string(p, "text")?, timestamp)?,
            "Input.dispatchKeyEvent" => {
                let type_ = string(p, "type")?;
                if type_ == "char" {
                    self.text(root, string(p, "text")?, timestamp)?;
                } else {
                    let kind = match type_ {
                        "keyDown" | "rawKeyDown" => RawKeyEventType::KeyDown,
                        "keyUp" => RawKeyEventType::KeyUp,
                        _ => return Err(Error::params("Invalid key event type")),
                    };
                    let name = string(p, "key")?;
                    let name = match name {
                        "ArrowLeft" => "Left",
                        "ArrowRight" => "Right",
                        "ArrowUp" => "Up",
                        "ArrowDown" => "Down",
                        " " => "Space",
                        "Meta" => "LWin",
                        "Backspace" => "Back",
                        "Control" => "LeftCtrl",
                        "Shift" => "LeftShift",
                        "Alt" => "LeftAlt",
                        other => other,
                    };
                    let digit;
                    let name = if name.len() == 1 && name.as_bytes()[0].is_ascii_digit() {
                        digit = format!("D{name}");
                        digit.as_str()
                    } else {
                        name
                    };
                    // The text a key produces: a control character (Enter's "\r", Tab, Backspace)
                    // is the key's own action, not text input, as the platform backends treat it.
                    let text = p["text"]
                        .as_str()
                        .filter(|s| !s.is_empty() && !s.chars().any(char::is_control));
                    // A key with no name of its own (punctuation of the layout, `,` or `!`) is
                    // `Key::None`, and its key down still delivers its text.
                    let key = match Key::parse(name).or_else(|_| Key::parse(&name.to_uppercase())) {
                        Ok(key) => key,
                        Err(_) if text.is_some() || name.chars().count() == 1 => Key::None,
                        Err(_) => return Err(Error::params("Unsupported key")),
                    };
                    let physical = p["code"]
                        .as_str()
                        .and_then(|s| PhysicalKey::parse(s.strip_prefix("Key").unwrap_or(s)).ok())
                        .unwrap_or(PhysicalKey::None);
                    let keyboard = KeyboardDevice::instance()
                        .ok_or_else(|| Error::server("Keyboard is unavailable"))?;
                    manager.process_input(Rc::new(RawKeyEventArgs::new(
                        keyboard,
                        timestamp,
                        root.input_root(),
                        kind,
                        key,
                        modifiers,
                        physical,
                        text.map(str::to_owned),
                        KeyDeviceType::Keyboard,
                    )));
                    if type_ == "keyDown" {
                        if let Some(text) = text {
                            self.text(root, text, timestamp)?;
                        }
                    }
                }
            }
            "Input.dispatchTouchEvent" => {
                let kind = string(p, "type")?;
                let points = p["touchPoints"]
                    .as_array()
                    .ok_or_else(|| Error::params("touchPoints must be an array"))?;
                if !matches!(
                    kind,
                    "touchStart" | "touchMove" | "touchEnd" | "touchCancel"
                ) {
                    return Err(Error::params("Invalid touch event type"));
                }
                if matches!(kind, "touchEnd" | "touchCancel") && !points.is_empty() {
                    return Err(Error::params("touchEnd/touchCancel require no points"));
                }
                if matches!(kind, "touchStart" | "touchMove") && points.is_empty() {
                    return Err(Error::params("Touch event requires points"));
                }
                let mut next = BTreeMap::new();
                for (index, p) in points.iter().enumerate() {
                    let id = p["id"].as_i64().unwrap_or(index as i64);
                    if next
                        .insert(id, Point::new(number(p, "x")?, number(p, "y")?))
                        .is_some()
                    {
                        return Err(Error::params("Duplicate touch id"));
                    }
                }
                for (id, point) in &self.points {
                    if !next.contains_key(id) {
                        let type_ = if kind == "touchCancel" {
                            RawPointerEventType::TouchCancel
                        } else {
                            RawPointerEventType::TouchEnd
                        };
                        manager.process_input(Rc::new(RawTouchEventArgs::new(
                            self.touch.clone(),
                            timestamp,
                            root.input_root(),
                            type_,
                            *point,
                            modifiers,
                            *id,
                        )));
                    }
                }
                for (id, point) in &next {
                    let type_ = if self.points.contains_key(id) {
                        RawPointerEventType::TouchUpdate
                    } else {
                        RawPointerEventType::TouchBegin
                    };
                    manager.process_input(Rc::new(RawTouchEventArgs::new(
                        self.touch.clone(),
                        timestamp,
                        root.input_root(),
                        type_,
                        *point,
                        modifiers,
                        *id,
                    )));
                }
                self.points = next;
            }
            _ => return Err(Error::method(method)),
        }
        Ok(json!({}))
    }
    fn text(&self, root: &TopLevel, text: &str, timestamp: u64) -> Result<()> {
        let keyboard =
            KeyboardDevice::instance().ok_or_else(|| Error::server("Keyboard is unavailable"))?;
        let manager = InputManager::instance()
            .ok_or_else(|| Error::server("Input manager is unavailable"))?;
        manager.process_input(Rc::new(RawTextInputEventArgs::new(
            keyboard,
            timestamp,
            root.input_root(),
            text,
        )));
        Ok(())
    }
}
impl Drop for Input {
    fn drop(&mut self) {
        self.touch.dispose();
    }
}
