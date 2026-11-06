//! Derived from ScreenKeyboard.Uno/Themes/KeyboardTheme.cs (Apache-2.0).
use super::ThemeList;
use crate::enum_bridge::parse_enum;
use ferroboard_core::{
    keys::{KeyType, KeyboardMode, ShiftState},
    serialization::{JsonBridgeError, StrictJsonValue, write_json_string},
    text::{TextUtils, Utf16Text},
};
use ferroui_base::media::Color;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThemeError {
    Json(JsonBridgeError),
    NullReference,
    ArgumentNull,
    InvalidColor,
    NonFiniteNumber,
    UnrepresentedEnum(i32),
}
impl From<JsonBridgeError> for ThemeError {
    fn from(e: JsonBridgeError) -> Self {
        Self::Json(e)
    }
}
impl std::fmt::Display for ThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ThemeError {}

/// Shared plain model, not a native DependencyObject. Raw-value companions retain unnamed source
/// enum values while the general Core enum migration is pending. No property notifications upstream.
pub struct KeyStyleRule {
    state: RwLock<RuleState>,
}
struct RuleState {
    codes: Option<ThemeList<i32>>,
    type_: Option<i32>,
    mode: Option<i32>,
    shift_state: Option<i32>,
    background: Option<Color>,
    pressed_background: Option<Color>,
    foreground: Option<Color>,
    font_size: Option<f64>,
    corner_radius: Option<f64>,
}
impl Default for KeyStyleRule {
    fn default() -> Self {
        Self {
            state: RwLock::new(RuleState {
                codes: Some(ThemeList::default()),
                type_: None,
                mode: None,
                shift_state: None,
                background: None,
                pressed_background: None,
                foreground: None,
                font_size: None,
                corner_radius: None,
            }),
        }
    }
}
macro_rules! rule_property {
    ($get:ident, $set:ident, $type:ty) => {
        pub fn $get(&self) -> $type {
            self.state
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .$get
                .clone()
        }
        pub fn $set(&self, value: $type) {
            self.state
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .$get = value;
        }
    };
}
impl KeyStyleRule {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    rule_property!(codes, set_codes, Option<ThemeList<i32>>);
    rule_property!(background, set_background, Option<Color>);
    rule_property!(pressed_background, set_pressed_background, Option<Color>);
    rule_property!(foreground, set_foreground, Option<Color>);
    rule_property!(font_size, set_font_size, Option<f64>);
    rule_property!(corner_radius, set_corner_radius, Option<f64>);
    pub fn type_value(&self) -> Option<i32> {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .type_
    }
    pub fn set_type_value(&self, v: Option<i32>) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .type_ = v;
    }
    pub fn mode_value(&self) -> Option<i32> {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .mode
    }
    pub fn set_mode_value(&self, v: Option<i32>) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .mode = v;
    }
    pub fn shift_state_value(&self) -> Option<i32> {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .shift_state
    }
    pub fn set_shift_state_value(&self, v: Option<i32>) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .shift_state = v;
    }
    pub fn set_type(&self, v: Option<KeyType>) {
        self.set_type_value(v.map(|v| v.0));
    }
    pub fn set_mode(&self, v: Option<KeyboardMode>) {
        self.set_mode_value(v.map(|v| v.0));
    }
    pub fn set_shift_state(&self, v: Option<ShiftState>) {
        self.set_shift_state_value(v.map(|v| v.0));
    }
    pub fn type_(&self) -> Option<KeyType> {
        self.type_value().map(KeyType)
    }
    pub fn mode(&self) -> Option<KeyboardMode> {
        self.mode_value().map(KeyboardMode)
    }
    pub fn shift_state(&self) -> Option<ShiftState> {
        self.shift_state_value().map(ShiftState)
    }
    pub fn matches(
        &self,
        code: i32,
        type_: KeyType,
        mode: KeyboardMode,
        shift: ShiftState,
    ) -> Result<bool, ThemeError> {
        self.matches_values(code, type_.0, mode.0, shift.0)
    }
    pub fn matches_values(
        &self,
        code: i32,
        type_: i32,
        mode: i32,
        shift: i32,
    ) -> Result<bool, ThemeError> {
        let codes = self.codes().ok_or(ThemeError::NullReference)?;
        let codes = codes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok((codes.is_empty() || codes.contains(&code))
            && self.type_value().is_none_or(|v| v == type_)
            && self.mode_value().is_none_or(|v| v == mode)
            && self.shift_state_value().is_none_or(|v| v == shift))
    }
}
impl PartialEq for KeyStyleRule {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for KeyStyleRule {}
pub(super) const MODE_NAMES: &[&str] = &[
    "Characters",
    "Symbols",
    "Symbols2",
    "Numeric",
    "NumericAdvanced",
    "Phone",
    "Phone2",
];
pub(super) const SHIFT_NAMES: &[&str] =
    &["Unshifted", "ShiftedManual", "ShiftedAutomatic", "CapsLock"];
include!("keyboard_theme_properties.rs");

impl KeyboardTheme {
    pub fn hex(text: &str) -> Result<Color, ThemeError> {
        Self::hex_text(&text.into())
    }
    pub fn hex_nullable(text: Option<&Utf16Text>) -> Result<Color, ThemeError> {
        Self::hex_text(text.ok_or(ThemeError::NullReference)?)
    }
    pub fn hex_text(text: &Utf16Text) -> Result<Color, ThemeError> {
        let mut units = trim(text.units());
        while units.first() == Some(&35) {
            units = &units[1..];
        }
        let mut expanded;
        if units.len() == 3 {
            expanded = units.iter().flat_map(|u| [*u, *u]).collect::<Vec<_>>();
            units = &expanded;
        }
        if units.len() == 6 {
            expanded = [vec![70, 70], units.to_vec()].concat();
            units = &expanded;
        }
        if units.len() != 8 {
            return Err(ThemeError::InvalidColor);
        }
        // UInt32.TryParse(HexNumber) accepts ASCII numeric whitespace and legacy trailing NUL.
        let mut body = units;
        while body.last() == Some(&0) {
            body = &body[..body.len() - 1];
        }
        while body.first().is_some_and(|u| matches!(*u, 9..=13 | 32)) {
            body = &body[1..];
        }
        while body.last().is_some_and(|u| matches!(*u, 9..=13 | 32)) {
            body = &body[..body.len() - 1];
        }
        if body.is_empty() {
            return Err(ThemeError::InvalidColor);
        }
        let mut v = 0u32;
        for u in body {
            let digit = match *u {
                48..=57 => u - 48,
                65..=70 => u - 55,
                97..=102 => u - 87,
                _ => return Err(ThemeError::InvalidColor),
            };
            v = (v << 4) | u32::from(digit);
        }
        Ok(Color::from_uint32(v))
    }
    pub fn to_hex(color: Color) -> String {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            color.a, color.r, color.g, color.b
        )
    }
    pub fn clone_theme(&self) -> Result<Arc<Self>, ThemeError> {
        Self::from_json(&self.to_json()?, None)
    }
    pub fn from_json(
        json: &str,
        base_theme: Option<&KeyboardTheme>,
    ) -> Result<Arc<Self>, ThemeError> {
        let theme = match base_theme {
            Some(base) => base.clone_theme()?,
            None => Self::new(),
        };
        let root = StrictJsonValue::parse_with_comments(json)?;
        if let Some(v) = root.get("id")?
            && let Some(v) = v.string()?
        {
            theme.set_id(Some(v));
        }
        if let Some(v) = root.get("name")?
            && let Some(v) = v.string()?
        {
            theme.set_name(Some(v));
        }
        if let Some(v) = root.get("isDark")? {
            theme.set_is_dark(v.boolean()?);
        }
        for (name, _, set) in COLOR_PROPERTIES {
            if let Some(v) = root.get(name)?
                && let Some(v) = v.string()?
            {
                set(&theme, Self::hex_text(&v)?);
            }
        }
        for (name, _, set) in NUMBER_PROPERTIES {
            if let Some(v @ StrictJsonValue::Number(_)) = root.get(name)? {
                set(&theme, v.double()?);
            }
        }
        if let Some(v) = root.get("fontFamily")? {
            theme.set_font_family(v.string()?);
        }
        if let Some(v) = root.get("uppercaseLabelsWhenShifted")? {
            theme.set_uppercase_labels_when_shifted(v.boolean()?);
        }
        if let Some(values) = root.get("keyRules")? {
            let rules = ThemeList::default();
            theme.set_key_rules(Some(rules.clone()));
            for r in values.array()? {
                let rule = KeyStyleRule::new();
                if let Some(v) = r.get("codes")? {
                    rule.set_codes(Some(ThemeList::from(
                        v.array()?
                            .iter()
                            .map(StrictJsonValue::int32)
                            .collect::<Result<Vec<_>, _>>()?,
                    )));
                }
                if let Some(v) = r.get("type")? {
                    let text = v.string()?;
                    rule.set_type(Some(KeyType::parse_key_type(
                        text.as_ref()
                            .map(|s| s.to_utf8().expect("JSON validated UTF16"))
                            .as_deref(),
                    )));
                }
                if let Some(v) = r.get("mode")? {
                    rule.set_mode_value(parse_enum(v.string()?.as_ref(), MODE_NAMES));
                }
                if let Some(v) = r.get("shiftState")? {
                    rule.set_shift_state_value(parse_enum(v.string()?.as_ref(), SHIFT_NAMES));
                }
                if let Some(v) = r.get("background")? {
                    rule.set_background(Some(Self::hex_nullable(v.string()?.as_ref())?));
                }
                if let Some(v) = r.get("pressedBackground")? {
                    rule.set_pressed_background(Some(Self::hex_nullable(v.string()?.as_ref())?));
                }
                if let Some(v) = r.get("foreground")? {
                    rule.set_foreground(Some(Self::hex_nullable(v.string()?.as_ref())?));
                }
                if let Some(v) = r.get("fontSize")? {
                    rule.set_font_size(Some(v.double()?));
                }
                if let Some(v) = r.get("cornerRadius")? {
                    rule.set_corner_radius(Some(v.double()?));
                }
                rules
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(Some(rule));
            }
        }
        Ok(theme)
    }
    pub fn to_json(&self) -> Result<String, ThemeError> {
        let mut props = vec![
            ("id", Out::String(self.id())),
            ("name", Out::String(self.name())),
            ("isDark", Out::Bool(self.is_dark())),
        ];
        for (name, get, _) in COLOR_PROPERTIES {
            props.push((
                name,
                Out::String(Some(Self::to_hex(get(self)).as_str().into())),
            ));
        }
        for (name, get, _) in NUMBER_PROPERTIES {
            props.push((name, Out::number(get(self))?));
        }
        if let Some(font) = self.font_family() {
            props.push(("fontFamily", Out::String(Some(font))));
        }
        props.push((
            "uppercaseLabelsWhenShifted",
            Out::Bool(self.uppercase_labels_when_shifted()),
        ));
        let rules = self.key_rules().ok_or(ThemeError::NullReference)?;
        let mut result = Vec::new();
        let rules = rules
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        for rule in rules {
            let rule = rule.ok_or(ThemeError::NullReference)?;
            let mut p = Vec::new();
            let codes = rule.codes().ok_or(ThemeError::NullReference)?;
            let codes = codes
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            if !codes.is_empty() {
                p.push((
                    "codes",
                    Out::Array(codes.into_iter().map(Out::Int).collect()),
                ));
            }
            if let Some(t) = rule.type_value() {
                let name = KeyType(t).to_json_string();
                p.push(("type", Out::String(Some(name.into()))));
            }
            if let Some(v) = rule.mode_value() {
                p.push((
                    "mode",
                    Out::String(Some(enum_name(v, MODE_NAMES).as_str().into())),
                ));
            }
            if let Some(v) = rule.shift_state_value() {
                p.push((
                    "shiftState",
                    Out::String(Some(enum_name(v, SHIFT_NAMES).as_str().into())),
                ));
            }
            if let Some(v) = rule.background() {
                p.push((
                    "background",
                    Out::String(Some(Self::to_hex(v).as_str().into())),
                ));
            }
            if let Some(v) = rule.pressed_background() {
                p.push((
                    "pressedBackground",
                    Out::String(Some(Self::to_hex(v).as_str().into())),
                ));
            }
            if let Some(v) = rule.foreground() {
                p.push((
                    "foreground",
                    Out::String(Some(Self::to_hex(v).as_str().into())),
                ));
            }
            if let Some(v) = rule.font_size() {
                p.push(("fontSize", Out::number(v)?));
            }
            if let Some(v) = rule.corner_radius() {
                p.push(("cornerRadius", Out::number(v)?));
            }
            result.push(Out::Object(p));
        }
        props.push(("keyRules", Out::Array(result)));
        let mut out = String::new();
        Out::Object(props).write(&mut out, 0);
        Ok(out)
    }
}
fn trim(mut units: &[u16]) -> &[u16] {
    while units.first().is_some_and(|u| TextUtils::is_white_space(*u)) {
        units = &units[1..];
    }
    while units.last().is_some_and(|u| TextUtils::is_white_space(*u)) {
        units = &units[..units.len() - 1];
    }
    units
}

fn enum_name(value: i32, names: &[&str]) -> String {
    usize::try_from(value)
        .ok()
        .and_then(|i| names.get(i))
        .map_or_else(|| value.to_string(), |s| (*s).into())
}
enum Out {
    String(Option<Utf16Text>),
    Bool(bool),
    Number(f64),
    Int(i32),
    Array(Vec<Out>),
    Object(Vec<(&'static str, Out)>),
}
impl Out {
    fn number(value: f64) -> Result<Self, ThemeError> {
        if value.is_finite() {
            Ok(Self::Number(value))
        } else {
            Err(ThemeError::NonFiniteNumber)
        }
    }
    fn write(&self, out: &mut String, depth: usize) {
        match self {
            Self::String(Some(s)) => write_json_string(out, s),
            Self::String(None) => out.push_str("null"),
            Self::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
            Self::Int(v) => out.push_str(&v.to_string()),
            Self::Number(v) => out.push_str(&json_double(*v)),
            Self::Array(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('\n');
                    out.push_str(&"  ".repeat(depth + 1));
                    v.write(out, depth + 1);
                }
                if !a.is_empty() {
                    out.push('\n');
                    out.push_str(&"  ".repeat(depth));
                }
                out.push(']');
            }
            Self::Object(a) => {
                out.push('{');
                for (i, (name, v)) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('\n');
                    out.push_str(&"  ".repeat(depth + 1));
                    write_json_string(out, &(*name).into());
                    out.push_str(": ");
                    v.write(out, depth + 1);
                }
                if !a.is_empty() {
                    out.push('\n');
                    out.push_str(&"  ".repeat(depth));
                }
                out.push('}');
            }
        }
    }
}
fn json_double(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0".into()
        } else {
            "0".into()
        };
    }
    let text = value.abs().to_string();
    let decimal = text.find('.').unwrap_or(text.len());
    let mut digits: String = text.chars().filter(|c| *c != '.').collect();
    let first = digits.find(|c| c != '0').expect("nonzero finite number");
    let exponent = decimal as i32 - first as i32 - 1;
    digits = digits[first..].trim_end_matches('0').to_owned();
    let sign = if value.is_sign_negative() { "-" } else { "" };
    if !(-4..17).contains(&exponent) {
        let fraction = if digits.len() > 1 {
            format!(".{}", &digits[1..])
        } else {
            String::new()
        };
        format!(
            "{sign}{}{fraction}E{}{:02}",
            &digits[..1],
            if exponent >= 0 { "+" } else { "-" },
            exponent.abs()
        )
    } else if exponent < 0 {
        format!("{sign}0.{}{}", "0".repeat((-exponent - 1) as usize), digits)
    } else {
        let n = exponent as usize + 1;
        if digits.len() > n {
            format!("{sign}{}.{}", &digits[..n], &digits[n..])
        } else {
            format!("{sign}{digits}{}", "0".repeat(n - digits.len()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::KeyboardThemes;
    use serde_json::{Value, json};
    fn error(e: ThemeError) -> Value {
        let kind = match e {
            ThemeError::NullReference => "System.NullReferenceException",
            ThemeError::ArgumentNull => "System.ArgumentNullException",
            ThemeError::InvalidColor => "System.FormatException",
            ThemeError::NonFiniteNumber => "System.ArgumentException",
            ThemeError::Json(JsonBridgeError::Syntax { .. }) => {
                "System.Text.Json.JsonReaderException"
            }
            ThemeError::Json(JsonBridgeError::NumberFormat) => "System.FormatException",
            ThemeError::Json(
                JsonBridgeError::InvalidString | JsonBridgeError::InvalidOperation,
            ) => "System.InvalidOperationException",
            ThemeError::UnrepresentedEnum(_) => {
                panic!("raw enum bridge must retain unknown values")
            }
        };
        json!({"error":kind})
    }
    fn capture<T: Into<Value>>(value: Result<T, ThemeError>) -> Value {
        value.map_or_else(error, Into::into)
    }
    fn bits(v: f64) -> String {
        format!("{:016x}", v.to_bits())
    }
    fn pascal(s: &str) -> String {
        s[..1].to_uppercase() + &s[1..]
    }
    fn snapshot(t: &KeyboardTheme) -> Value {
        let text = json!({"Id":t.id().map(|s|s.units().to_vec()),"Name":t.name().map(|s|s.units().to_vec()),"FontFamily":t.font_family().map(|s|s.units().to_vec())});
        let colors: serde_json::Map<_, _> = COLOR_PROPERTIES
            .iter()
            .map(|(n, get, _)| (pascal(n), json!(KeyboardTheme::to_hex(get(t)))))
            .collect();
        let numbers: serde_json::Map<_, _> = NUMBER_PROPERTIES
            .iter()
            .map(|(n, get, _)| (pascal(n), json!(bits(get(t)))))
            .collect();
        let rules=t.key_rules().map(|rules|rules.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone().into_iter().map(|rule|rule.map(|r|json!({
            "codes":r.codes().map(|c|c.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()),
            "type":r.type_().map(|v|v.0),"mode":r.mode().map(|m|m.0),"shift":r.shift_state().map(|v|v.0),
            "background":r.background().map(KeyboardTheme::to_hex),"pressed":r.pressed_background().map(KeyboardTheme::to_hex),
            "foreground":r.foreground().map(KeyboardTheme::to_hex),"font":r.font_size().map(bits),"corner":r.corner_radius().map(bits)
        }))).collect::<Vec<_>>());
        json!({"text":text,"booleans":{"IsDark":t.is_dark(),"UppercaseLabelsWhenShifted":t.uppercase_labels_when_shifted()},"colors":colors,"numbers":numbers,"rules":rules})
    }
    fn inspect(t: &KeyboardTheme) -> Value {
        json!({"state":snapshot(t),"json":capture(t.to_json()),"clone":capture(t.clone_theme().map(|c|snapshot(&c)))})
    }
    fn difference(a: &Value, b: &Value, path: String) -> String {
        match (a, b) {
            (Value::Object(a), Value::Object(b)) => {
                for key in a.keys().chain(b.keys()) {
                    if a.get(key) != b.get(key) {
                        return difference(
                            a.get(key).unwrap_or(&Value::Null),
                            b.get(key).unwrap_or(&Value::Null),
                            format!("{path}/{key}"),
                        );
                    }
                }
            }
            (Value::Array(a), Value::Array(b)) => {
                for i in 0..a.len().max(b.len()) {
                    if a.get(i) != b.get(i) {
                        return difference(
                            a.get(i).unwrap_or(&Value::Null),
                            b.get(i).unwrap_or(&Value::Null),
                            format!("{path}/{i}"),
                        );
                    }
                }
            }
            (Value::String(a), Value::String(b)) if a.contains('\n') && b.contains('\n') => {
                for (i, (a, b)) in a.lines().zip(b.lines()).enumerate() {
                    if a != b {
                        return format!("{path}: line {i} actual {a:?} expected {b:?}");
                    }
                }
            }
            _ => (),
        }
        format!("{path}: actual {a} expected {b}")
    }
    #[test]
    fn unchanged_theme_model_color_json_and_builtin_reference() {
        // This oracle observes a fresh process-wide registry and deliberately installs throwing
        // handlers. Native owner tests also register themes on their UI thread. Isolate the
        // corpus instead of resetting production global state or relying on libtest ordering.
        const CHILD: &str = "FERROBOARD_THEME_REFERENCE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "themes::keyboard_theme::tests::unchanged_theme_model_color_json_and_builtin_reference",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "isolated theme reference failed:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let mut count = 0;
        for line in include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/themes.jsonl"
        ))
        .lines()
        {
            let row: Value = serde_json::from_str(line).unwrap();
            let input = &row["input"];
            let actual = match input["op"].as_str().unwrap() {
                "default" => inspect(&KeyboardTheme::new()),
                "builtin" => inspect(&KeyboardThemes::get(input["id"].as_str().unwrap()).unwrap()),
                "json" => {
                    let base = input["baseId"]
                        .as_str()
                        .map(|id| KeyboardThemes::get(id).unwrap());
                    KeyboardTheme::from_json(input["json"].as_str().unwrap(), base.as_deref())
                        .map_or_else(error, |t| inspect(&t))
                }
                "hex" => {
                    let text = input["text"].as_array().map(|a| {
                        Utf16Text::from_units(
                            a.iter()
                                .map(|v| v.as_u64().unwrap() as u16)
                                .collect::<Vec<_>>(),
                        )
                    });
                    capture(KeyboardTheme::hex_nullable(text.as_ref()).map(KeyboardTheme::to_hex))
                }
                "number" => {
                    let n = f64::from_bits(
                        u64::from_str_radix(input["bits"].as_str().unwrap(), 16).unwrap(),
                    );
                    let t = KeyboardTheme::new();
                    t.set_key_font_size(n);
                    json!({"json":capture(t.to_json()),"clone":capture(t.clone_theme().map(|c|bits(c.key_font_size())))})
                }
                "mutation" => {
                    let t = KeyboardTheme::new();
                    match input["mutation"].as_str().unwrap() {
                        "nullId" => t.set_id(None),
                        "nullName" => t.set_name(None),
                        "invalidName" => {
                            t.set_name(Some(Utf16Text::from_units(vec![97, 0xd800, 98])))
                        }
                        "nullRules" => t.set_key_rules(None),
                        "nullRule" => t
                            .key_rules()
                            .unwrap()
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .push(None),
                        name => {
                            let r = KeyStyleRule::new();
                            match name {
                                "nullCodes" => r.set_codes(None),
                                "unknownEnums" => {
                                    r.set_type_value(Some(999));
                                    r.set_mode_value(Some(-1));
                                    r.set_shift_state_value(Some(999));
                                }
                                "nanRule" => {
                                    r.set_font_size(Some(f64::from_bits(0xfff8_0000_0000_0000)))
                                }
                                _ => panic!("{name}"),
                            }
                            t.key_rules()
                                .unwrap()
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                                .push(Some(r));
                        }
                    }
                    inspect(&t)
                }
                "matches" => {
                    let r = KeyStyleRule::new();
                    r.set_codes(input["codes"].as_array().map(|a| {
                        ThemeList::from(
                            a.iter()
                                .map(|v| v.as_i64().unwrap() as i32)
                                .collect::<Vec<_>>(),
                        )
                    }));
                    r.set_type_value(input["type"].as_i64().map(|v| v as i32));
                    r.set_mode_value(input["mode"].as_i64().map(|v| v as i32));
                    r.set_shift_state_value(input["shift"].as_i64().map(|v| v as i32));
                    json!([
                        capture(r.matches_values(1, 7, 3, 3)),
                        capture(r.matches_values(8, 999, 999, -1))
                    ])
                }
                "registry" => {
                    use std::sync::Mutex;
                    let before = KeyboardThemes::all();
                    let events = Arc::new(Mutex::new(Vec::new()));
                    let list = events.clone();
                    let mut handler = KeyboardThemes::changed(move || {
                        list.lock().unwrap().push(
                            KeyboardThemes::get("parity_a")
                                .and_then(|t| t.name())
                                .map(|s| s.to_utf8().unwrap())
                                .unwrap_or("missing".into()),
                        );
                    });
                    let first = KeyboardTheme::new();
                    first.set_id(Some("PARITY_a".into()));
                    first.set_name(Some("first".into()));
                    KeyboardThemes::register(first.clone()).unwrap();
                    let same = Arc::ptr_eq(&first, &KeyboardThemes::get("parity_A").unwrap());
                    let all = KeyboardThemes::all();
                    let replacement = KeyboardTheme::new();
                    replacement.set_id(Some("parity_A".into()));
                    replacement.set_name(Some("replacement".into()));
                    KeyboardThemes::register(replacement.clone()).unwrap();
                    first.set_id(Some("parity_alt".into()));
                    first.set_name(Some("original mutated".into()));
                    KeyboardThemes::register(first.clone()).unwrap();
                    replacement.set_id(None);
                    let invalid =
                        capture(KeyboardThemes::register(replacement.clone()).map(|_| true));
                    let null_theme = capture(KeyboardThemes::register_nullable(None).map(|_| true));
                    let null_id = capture(KeyboardThemes::get_text(None).map(|v| v.map(|_| true)));
                    handler.unsubscribe();
                    json!({"same":same,"before":before.len(),"snapshot":all.len(),"current":KeyboardThemes::all().len(),
                        "snapshotRetainsFirst":Arc::ptr_eq(all.last().unwrap(),&first),
                        "oldKeyRetainsReplacement":Arc::ptr_eq(&KeyboardThemes::get("parity_a").unwrap(),&replacement),
                        "newKeyRetainsFirst":Arc::ptr_eq(&KeyboardThemes::get("PARITY_ALT").unwrap(),&first),
                        "events":*events.lock().unwrap(),"invalid":invalid,"nullTheme":null_theme,"nullId":null_id,
                        "order":KeyboardThemes::all().iter().skip(before.len()).map(|t|t.id().map(|s|s.units().to_vec())).collect::<Vec<_>>()})
                }
                "registryEvents" => {
                    use std::sync::{
                        Mutex,
                        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
                    };
                    let trace = Arc::new(Mutex::new(Vec::new()));
                    let nested = Arc::new(AtomicBool::new(false));
                    let b_id = Arc::new(AtomicU64::new(u64::MAX));
                    let c_id = Arc::new(AtomicU64::new(u64::MAX));
                    let (t, n, b, c) = (trace.clone(), nested.clone(), b_id.clone(), c_id.clone());
                    let a = KeyboardThemes::add_changed(move || {
                        t.lock().unwrap().push("A");
                        if !n.swap(true, Ordering::SeqCst) {
                            KeyboardThemes::remove_changed(b.load(Ordering::SeqCst));
                            let log = t.clone();
                            c.store(
                                KeyboardThemes::add_changed(move || log.lock().unwrap().push("C")),
                                Ordering::SeqCst,
                            );
                            let theme = KeyboardTheme::new();
                            theme.set_id(Some("event_inner".into()));
                            KeyboardThemes::register(theme).unwrap();
                        }
                    });
                    let t = trace.clone();
                    b_id.store(
                        KeyboardThemes::add_changed(move || t.lock().unwrap().push("B")),
                        Ordering::SeqCst,
                    );
                    let outer = KeyboardTheme::new();
                    outer.set_id(Some("event_outer".into()));
                    KeyboardThemes::register(outer).unwrap();
                    KeyboardThemes::remove_changed(a);
                    KeyboardThemes::remove_changed(c_id.load(Ordering::SeqCst));
                    let later = Arc::new(AtomicUsize::new(0));
                    let count = later.clone();
                    let throws = KeyboardThemes::add_changed(|| {
                        panic!("source synchronous callback failure")
                    });
                    let e = KeyboardThemes::add_changed(move || {
                        count.fetch_add(1, Ordering::SeqCst);
                    });
                    let value = KeyboardTheme::new();
                    value.set_id(Some("event_throw".into()));
                    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        KeyboardThemes::register(value.clone())
                    }))
                    .is_err();
                    KeyboardThemes::remove_changed(throws);
                    KeyboardThemes::remove_changed(e);
                    let flags = Arc::new(Mutex::new(Vec::new()));
                    let expected = Arc::new(Mutex::new(std::thread::current().id()));
                    let (log, thread) = (flags.clone(), expected.clone());
                    let mut threaded = KeyboardThemes::changed(move || {
                        log.lock()
                            .unwrap()
                            .push(std::thread::current().id() == *thread.lock().unwrap())
                    });
                    let shared = KeyboardTheme::new();
                    shared.set_id(Some("event_thread".into()));
                    KeyboardThemes::register(shared.clone()).unwrap();
                    let original = shared.clone();
                    let worker_same = std::thread::spawn(move || {
                        *expected.lock().unwrap() = std::thread::current().id();
                        let t = KeyboardThemes::get("EVENT_THREAD").unwrap();
                        t.set_name(Some("worker".into()));
                        KeyboardThemes::register(t.clone()).unwrap();
                        Arc::ptr_eq(&t, &original)
                    })
                    .join()
                    .unwrap();
                    threaded.unsubscribe();
                    json!({"trace":*trace.lock().unwrap(),"failure":if failed {json!({"error":"System.InvalidOperationException"})}else{json!(true)},
                        "later":later.load(Ordering::SeqCst),"retainedAfterFailure":Arc::ptr_eq(&KeyboardThemes::get("event_throw").unwrap(),&value),
                        "flags":*flags.lock().unwrap(),"workerSame":worker_same,"name":shared.name().unwrap().to_utf8().unwrap()})
                }
                name => panic!("unknown case {name}"),
            };
            assert!(
                actual == row["result"],
                "theme row {} input {}: {}",
                row["index"],
                input,
                difference(&actual, &row["result"], String::new())
            );
            count += 1;
        }
        assert_eq!(count, 2157);
    }
    #[test]
    fn shared_native_theme_vectors_keep_identity_snapshots_and_clone_normalization() {
        let t = KeyboardTheme::new();
        let rules = t.key_rules().unwrap();
        assert!(Arc::ptr_eq(&rules, &t.key_rules().unwrap()));
        let rule = KeyStyleRule::new();
        let codes = rule.codes().unwrap();
        codes.lock().unwrap().extend([1, 2]);
        rules.lock().unwrap().push(Some(rule.clone()));
        let mut snapshot = codes.lock().unwrap().clone().into_iter();
        assert_eq!(snapshot.next(), Some(1));
        codes.lock().unwrap().clear();
        assert_eq!(snapshot.next(), Some(2));
        assert_eq!(snapshot.next(), None);
        assert!(codes.lock().unwrap().is_empty());
        rule.set_type_value(Some(999));
        rule.set_mode_value(Some(999));
        let cloned = t.clone_theme().unwrap();
        assert!(!Arc::ptr_eq(&t, &cloned));
        let clone_rules = cloned.key_rules().unwrap();
        assert!(!Arc::ptr_eq(&rules, &clone_rules));
        let clone_rule = clone_rules.lock().unwrap()[0].clone().unwrap();
        assert!(!Arc::ptr_eq(&rule, &clone_rule));
        assert!(!Arc::ptr_eq(
            &clone_rule.codes().unwrap(),
            &rule.codes().unwrap()
        ));
        assert_eq!(clone_rule.type_(), Some(KeyType::Unspecified));
        assert_eq!(clone_rule.mode_value(), Some(999));
        assert_eq!(clone_rule.mode(), Some(KeyboardMode(999)));
        t.set_key_rules(Some(ThemeList::default()));
        assert_eq!(rules.lock().unwrap().len(), 1);
        assert!(t.key_rules().unwrap().lock().unwrap().is_empty());
        t.set_key_font_size(f64::INFINITY);
        t.set_key_rules(None);
        assert_eq!(
            KeyboardTheme::from_json("{", Some(&t)).err(),
            Some(ThemeError::NonFiniteNumber)
        );
    }
}
