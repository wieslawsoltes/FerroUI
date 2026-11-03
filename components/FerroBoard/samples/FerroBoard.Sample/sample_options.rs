//! SampleOptions.cs: process-shared options, UTF-16 URI decoding and ordered synchronous Apply.
use ferroboard_core::{
    settings::{KeyboardSettings, OneHandedMode, ThemeMode},
    text::{OrdinalKey, TextUtils, Utf16Text, ordinal_ignore_case_equal},
};
use ferroui_system::collections::generic::Dictionary;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, LazyLock, Mutex, MutexGuard},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionErrorKind {
    Format,
    Overflow,
    UnrepresentableText,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionError {
    pub option: &'static str,
    pub kind: OptionErrorKind,
}
impl std::fmt::Display for OptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "startup option {}: {:?}", self.option, self.kind)
    }
}
impl std::error::Error for OptionError {}
#[derive(Default)]
pub struct SampleOptions {
    values: Mutex<Dictionary<OrdinalKey, Option<Utf16Text>>>,
}
impl SampleOptions {
    pub fn shared() -> Arc<Self> {
        static SHARED: LazyLock<Arc<SampleOptions>> =
            LazyLock::new(|| Arc::new(SampleOptions::default()));
        SHARED.clone()
    }
    /// The stable mutable Values collection. Guards never span application callbacks.
    pub fn values(&self) -> MutexGuard<'_, Dictionary<OrdinalKey, Option<Utf16Text>>> {
        self.values
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub fn parse(&self, arguments: Option<&Utf16Text>) {
        let Some(arguments) =
            arguments.filter(|a| !a.units().iter().all(|&u| TextUtils::is_white_space(u)))
        else {
            return;
        };
        let input = arguments.units();
        let start = input.iter().take_while(|&&u| u == b'?' as u16).count();
        for part in input[start..]
            .split(|&u| u == b'&' as u16 || u == b' ' as u16)
            .filter(|p| !p.is_empty())
        {
            let start = part.iter().take_while(|&&u| u == b'-' as u16).count();
            let part = &part[start..];
            let split = part.iter().position(|&u| u == b'=' as u16);
            let (key, value) = match split {
                Some(i) => (unescape(&part[..i]), unescape(&part[i + 1..])),
                None => (unescape(part), "true".into()),
            };
            self.values().insert(OrdinalKey(key), Some(value));
        }
    }
    pub fn get(&self, key: &Utf16Text) -> Option<Utf16Text> {
        self.values()
            .get(&OrdinalKey(key.clone()))
            .cloned()
            .flatten()
    }
    pub fn apply(&self, settings: &KeyboardSettings) -> Result<(), OptionError> {
        // Fetch each option at its source position. A settings notification may synchronously
        // change Values before a later Get; a snapshot would change this contract.
        if let Some(value) = self.get(&"theme".into())
            && let Some(mode) = parse_enum(&value, &["FollowSystem", "Light", "Dark"])
        {
            settings.set_theme_mode(ThemeMode(mode));
        }
        if let Some(value) = self.get(&"lighttheme".into()) {
            settings.set_light_theme(Some(utf8(&value, "lighttheme")?));
        }
        if let Some(value) = self.get(&"darktheme".into()) {
            settings.set_dark_theme(Some(utf8(&value, "darktheme")?));
        }
        if let Some(value) = self.get(&"onehanded".into())
            && let Some(mode) = parse_enum(&value, &["Off", "Left", "Right"])
        {
            settings.set_one_handed_mode(OneHandedMode(mode));
        }
        for name in ["split", "floating", "numberrow", "glide"] {
            if let Some(value) = self.get(&name.into()) {
                let value = parse_bool(&value).ok_or(OptionError {
                    option: name,
                    kind: OptionErrorKind::Format,
                })?;
                match name {
                    "split" => settings.set_split_keyboard(value),
                    "floating" => settings.set_floating(value),
                    "numberrow" => settings.set_number_row(value),
                    "glide" => settings.set_glide_typing(value),
                    _ => unreachable!(),
                }
            }
        }
        if let Some(value) = self.get(&"keyheight".into()) {
            settings.set_key_height(parse_double(&value).ok_or(OptionError {
                option: "keyheight",
                kind: OptionErrorKind::Format,
            })?);
        }
        if let Some(value) = self.get(&"langs".into()) {
            let langs = value
                .units()
                .split(|&u| u == b',' as u16)
                .filter(|p| !p.is_empty())
                .map(|p| utf8(&Utf16Text::from_units(p.to_vec()), "langs").map(Some))
                .collect::<Result<Vec<_>, _>>()?;
            settings.set_subtypes(Some(Rc::new(RefCell::new(langs))));
        }
        if let Some(value) = self.get(&"longpress".into()) {
            settings.set_long_press_delay(parse_int(&value).map_err(|kind| OptionError {
                option: "longpress",
                kind,
            })?);
        }
        Ok(())
    }
}
fn utf8(value: &Utf16Text, option: &'static str) -> Result<String, OptionError> {
    value.to_utf8().map_err(|_| OptionError {
        option,
        kind: OptionErrorKind::UnrepresentableText,
    })
}
fn trim(mut value: &[u16], predicate: impl Fn(u16) -> bool) -> &[u16] {
    while value.first().is_some_and(|&u| predicate(u)) {
        value = &value[1..];
    }
    while value.last().is_some_and(|&u| predicate(u)) {
        value = &value[..value.len() - 1];
    }
    value
}
fn equals(value: &[u16], expected: &str) -> bool {
    ordinal_ignore_case_equal(&Utf16Text::from_units(value.to_vec()), &expected.into())
}
fn parse_bool(value: &Utf16Text) -> Option<bool> {
    let value = trim(value.units(), |u| u == 0 || TextUtils::is_white_space(u));
    if equals(value, "true") {
        Some(true)
    } else if equals(value, "false") {
        Some(false)
    } else {
        None
    }
}
fn number_white(u: u16) -> bool {
    u == 32 || (9..=13).contains(&u)
}
fn number_text(value: &Utf16Text) -> &[u16] {
    // Number parsing accepts trailing NULs, but whitespace after a NUL is not accepted.
    let u = value.units();
    let end = u.len() - u.iter().rev().take_while(|&&c| c == 0).count();
    trim(&u[..end], number_white)
}
fn parse_int(value: &Utf16Text) -> Result<i32, OptionErrorKind> {
    let u = number_text(value);
    let digits = if matches!(u.first(), Some(43 | 45)) {
        &u[1..]
    } else {
        u
    };
    if digits.is_empty() || digits.iter().any(|u| !(48..=57).contains(u)) {
        return Err(OptionErrorKind::Format);
    }
    let text = String::from_utf16(u).map_err(|_| OptionErrorKind::Format)?;
    text.parse().map_err(|_| OptionErrorKind::Overflow)
}
fn parse_enum(value: &Utf16Text, names: &[&str]) -> Option<i32> {
    let u = value.units();
    let start = u
        .iter()
        .take_while(|&&u| TextUtils::is_white_space(u))
        .count();
    let u = &u[start..];
    if matches!(u.first(), Some(43 | 45 | 48..=57)) {
        // Only leading Unicode whitespace is trimmed before the numeric branch; trailing
        // numeric whitespace and NULs follow the runtime integer parser.
        if let Ok(value) = parse_int(&Utf16Text::from_units(u.to_vec())) {
            return Some(value);
        }
    }
    let mut bits = 0;
    for part in u.split(|&u| u == 44) {
        let part = trim(part, TextUtils::is_white_space);
        bits |= names.iter().position(|name| equals(part, name))? as i32;
    }
    Some(bits)
}
fn parse_double(value: &Utf16Text) -> Option<f64> {
    let special = trim(value.units(), TextUtils::is_white_space);
    let unsigned = if matches!(special.first(), Some(43 | 45)) {
        &special[1..]
    } else {
        special
    };
    if equals(unsigned, "NaN") {
        return Some(f64::from_bits(0xfff8000000000000));
    }
    if equals(unsigned, "Infinity") {
        return Some(if special.first() == Some(&45) {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    let u = number_text(value);
    let mut i = usize::from(matches!(u.first(), Some(43 | 45)));
    let mut digits = 0;
    let mut text = String::new();
    if i == 1 {
        text.push(u[0] as u8 as char);
    }
    while i < u.len() {
        match u[i] {
            48..=57 => {
                digits += 1;
                text.push(u[i] as u8 as char);
            }
            44 if digits > 0 => {}
            _ => break,
        }
        i += 1;
    }
    if u.get(i) == Some(&46) {
        text.push('.');
        i += 1;
        while u.get(i).is_some_and(|u| (48..=57).contains(u)) {
            digits += 1;
            text.push(u[i] as u8 as char);
            i += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if matches!(u.get(i), Some(69 | 101)) {
        text.push('e');
        i += 1;
        if matches!(u.get(i), Some(43 | 45)) {
            text.push(u[i] as u8 as char);
            i += 1;
        }
        let start = i;
        while u.get(i).is_some_and(|u| (48..=57).contains(u)) {
            text.push(u[i] as u8 as char);
            i += 1;
        }
        if start == i {
            return None;
        }
    }
    if i != u.len() {
        return None;
    }
    text.parse().ok()
}
fn hex(u: u16) -> Option<u8> {
    match u {
        48..=57 => Some((u - 48) as u8),
        65..=70 => Some((u - 55) as u8),
        97..=102 => Some((u - 87) as u8),
        _ => None,
    }
}
fn byte(input: &[u16], i: usize) -> Option<u8> {
    if input.get(i) != Some(&37) {
        return None;
    }
    Some(hex(*input.get(i + 1)?)? * 16 + hex(*input.get(i + 2)?)?)
}
fn unescape(input: &[u16]) -> Utf16Text {
    let mut out = Vec::new();
    let mut i = 0;
    while i < input.len() {
        if byte(input, i).is_none() {
            out.push(input[i]);
            i += 1;
            continue;
        }
        let start = i;
        let mut bytes = Vec::new();
        while let Some(b) = byte(input, i) {
            bytes.push(b);
            i += 3;
        }
        let mut offset = 0;
        while offset < bytes.len() {
            match std::str::from_utf8(&bytes[offset..]) {
                Ok(valid) => {
                    out.extend(valid.encode_utf16());
                    break;
                }
                Err(error) => {
                    let end = offset + error.valid_up_to();
                    out.extend(
                        std::str::from_utf8(&bytes[offset..end])
                            .expect("validated UTF-8 prefix")
                            .encode_utf16(),
                    );
                    out.extend_from_slice(&input[start + end * 3..start + (end + 1) * 3]);
                    offset = end + 1;
                }
            }
        }
    }
    Utf16Text::from_units(out)
}
