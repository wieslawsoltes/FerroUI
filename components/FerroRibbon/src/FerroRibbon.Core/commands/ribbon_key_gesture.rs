use crate::text::unicode_data as data;
use crate::{
    RibbonString,
    text::{ordinal::OrdinalKey, ordinal_equals_ascii, trim},
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct RibbonModifierKeys: i32 {
        const NONE = 0;
        const CONTROL = 1;
        const SHIFT = 2;
        const ALT = 4;
        const META = 8;
        // CLR flags accept every i32 bit pattern. Retain those bits under ! as well.
        const _ = !0;
    }
}
impl RibbonModifierKeys {
    pub const fn from_raw(value: i32) -> Self {
        Self::from_bits_retain(value)
    }
    pub const fn raw(self) -> i32 {
        self.bits()
    }
    pub const fn has_flag(self, flag: Self) -> bool {
        self.contains(flag)
    }
}
impl std::fmt::Display for RibbonModifierKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.bits() == 0 {
            return f.write_str("None");
        }
        if self.bits() & !15 != 0 {
            return write!(f, "{}", self.bits());
        }
        let mut separator = "";
        for (flag, label) in [
            (Self::CONTROL, "Control"),
            (Self::SHIFT, "Shift"),
            (Self::ALT, "Alt"),
            (Self::META, "Meta"),
        ] {
            if self.has_flag(flag) {
                f.write_str(separator)?;
                f.write_str(label)?;
                separator = ", ";
            }
        }
        Ok(())
    }
}
/// Bridge for the two current-culture number symbols used by the source's int.TryParse.
/// Native FerroRibbon installs a provider backed by FerroUI CultureInfo. Core alone
/// starts with the invariant format and may install its own thread-local provider.
#[derive(Clone, Debug)]
pub struct RibbonGestureNumberFormat {
    pub positive_sign: RibbonString,
    pub negative_sign: RibbonString,
}
impl Default for RibbonGestureNumberFormat {
    fn default() -> Self {
        Self {
            positive_sign: "+".into(),
            negative_sign: "-".into(),
        }
    }
}
pub type RibbonGestureNumberFormatProvider = dyn Fn() -> RibbonGestureNumberFormat;
ferroui_system::ferro_thread_local! {
    static NUMBER_FORMAT: RefCell<Option<Rc<RibbonGestureNumberFormatProvider>>> = RefCell::default();
}
static TREAT_META_AS_CONTROL: AtomicBool = AtomicBool::new(true);

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RibbonKeyGesture {
    pub modifiers: RibbonModifierKeys,
    pub key: Option<RibbonString>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonKeyGestureFormatError {
    pub message: RibbonString,
}
impl std::fmt::Display for RibbonKeyGestureFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Diagnostics escape malformed units; the public message stays lossless.
        for character in char::decode_utf16(self.message.as_utf16().iter().copied()) {
            match character {
                Ok(character) => write!(f, "{character}")?,
                Err(error) => write!(f, "\\u{{{:x}}}", error.unpaired_surrogate())?,
            }
        }
        Ok(())
    }
}
impl std::error::Error for RibbonKeyGestureFormatError {}
impl RibbonKeyGesture {
    pub fn new(modifiers: RibbonModifierKeys, key: Option<RibbonString>) -> Self {
        Self { modifiers, key }
    }
    pub fn treat_meta_as_control() -> bool {
        TREAT_META_AS_CONTROL.load(Ordering::SeqCst)
    }
    pub fn set_treat_meta_as_control(value: bool) {
        TREAT_META_AS_CONTROL.store(value, Ordering::SeqCst);
    }
    /// Returns the old provider, allowing explicit restoration at a composition boundary.
    pub fn set_number_format_provider(
        provider: Option<Rc<RibbonGestureNumberFormatProvider>>,
    ) -> Option<Rc<RibbonGestureNumberFormatProvider>> {
        NUMBER_FORMAT.with(|current| current.replace(provider))
    }
    pub fn parse(text: Option<&RibbonString>) -> Result<Self, RibbonKeyGestureFormatError> {
        let mut gesture = Self::default();
        if Self::try_parse(text, &mut gesture) {
            return Ok(gesture);
        }
        let mut message = vec![b'\'' as u16];
        if let Some(text) = text {
            message.extend_from_slice(text.as_utf16());
        }
        message.extend("' is not a valid key gesture.".encode_utf16());
        Err(RibbonKeyGestureFormatError {
            message: RibbonString::from_utf16(message),
        })
    }
    pub fn try_parse(text: Option<&RibbonString>, gesture: &mut Self) -> bool {
        *gesture = Self::default();
        let Some(text) = text else { return false };
        let text = trim(text.as_utf16());
        if text.is_empty() {
            return false;
        }
        let mut parts = Vec::new();
        let mut current = Vec::new();
        for &c in text {
            if c == b'+' as u16 && !trim(&current).is_empty() {
                parts.push(trim(&current).to_vec());
                current.clear();
            } else {
                current.push(c);
            }
        }
        if !current.is_empty() {
            parts.push(trim(&current).to_vec());
        }
        let mut modifiers = RibbonModifierKeys::NONE;
        let mut key = None;
        for part in parts {
            if part.is_empty() {
                return false;
            }
            let upper = upper_invariant(&part);
            let flag = match upper.to_utf8().ok().as_deref() {
                Some("CTRL" | "CONTROL" | "STRG") => Some(RibbonModifierKeys::CONTROL),
                Some("SHIFT") => Some(RibbonModifierKeys::SHIFT),
                Some("ALT" | "OPTION" | "OPT") => Some(RibbonModifierKeys::ALT),
                Some("WIN" | "META" | "CMD" | "COMMAND") => Some(RibbonModifierKeys::META),
                _ => None,
            };
            if let Some(flag) = flag {
                modifiers |= flag;
            } else {
                if key.is_some() {
                    return false;
                }
                key = Some(normalize_key(&part));
            }
        }
        if key.is_none() {
            return false;
        }
        *gesture = Self { modifiers, key };
        true
    }
    pub fn matches(&self, key: Option<&RibbonString>, mut modifiers: RibbonModifierKeys) -> bool {
        let (Some(key), Some(stored)) = (key, &self.key) else {
            return false;
        };
        if key.is_empty() || stored.is_empty() {
            return false;
        }
        let normalized = normalize_key(key.as_utf16());
        if !equal(&normalized, stored) {
            let mut number_pad = "NumberPad".encode_utf16().collect::<Vec<_>>();
            number_pad.push(*stored.as_utf16().last().unwrap());
            if !starts_number(stored) || !equal(key, &RibbonString::from_utf16(number_pad)) {
                return false;
            }
        }
        if Self::treat_meta_as_control()
            && modifiers.has_flag(RibbonModifierKeys::META)
            && !self.modifiers.has_flag(RibbonModifierKeys::META)
        {
            modifiers = (modifiers & !RibbonModifierKeys::META) | RibbonModifierKeys::CONTROL;
        }
        modifiers == self.modifiers
    }
    pub fn to_ribbon_string(&self) -> RibbonString {
        self.to_display_string(false)
    }
    pub fn to_display_string(&self, mac_style: bool) -> RibbonString {
        let key = self.key.as_ref().expect("ArgumentNull: key");
        let key = if let Some(alias) = lookup(key.as_utf16(), DISPLAY) {
            alias.into()
        } else if starts_number(key) && key.len() == 7 {
            RibbonString::from(&key.as_utf16()[6..])
        } else {
            key.clone()
        };
        let mut parts = Vec::new();
        if mac_style {
            let command =
                Self::treat_meta_as_control() && !self.modifiers.has_flag(RibbonModifierKeys::META);
            for (flag, label) in [
                (RibbonModifierKeys::CONTROL, if command { "⌘" } else { "⌃" }),
                (RibbonModifierKeys::ALT, "⌥"),
                (RibbonModifierKeys::SHIFT, "⇧"),
                (RibbonModifierKeys::META, "⌘"),
            ] {
                if self.modifiers.has_flag(flag) {
                    parts.extend(label.encode_utf16());
                }
            }
        } else {
            for (flag, label) in [
                (RibbonModifierKeys::CONTROL, "Ctrl"),
                (RibbonModifierKeys::SHIFT, "Shift"),
                (RibbonModifierKeys::ALT, "Alt"),
                (RibbonModifierKeys::META, "Win"),
            ] {
                if self.modifiers.has_flag(flag) {
                    parts.extend(label.encode_utf16());
                    parts.push(b'+' as u16);
                }
            }
        }
        if parts.is_empty() {
            return key;
        }
        parts.extend_from_slice(key.as_utf16());
        RibbonString::from_utf16(parts)
    }
}
fn equal(a: &RibbonString, b: &RibbonString) -> bool {
    OrdinalKey(a.clone()) == OrdinalKey(b.clone())
}
fn starts_number(key: &RibbonString) -> bool {
    key.as_utf16().starts_with(&[78, 117, 109, 98, 101, 114])
}
fn lookup(key: &[u16], entries: &[(&'static str, &'static str)]) -> Option<&'static str> {
    entries
        .iter()
        .find_map(|&(name, value)| ordinal_equals_ascii(key, name).then_some(value))
}
fn in_ranges(c: u16, ranges: &[(u16, u16)]) -> bool {
    let index = ranges.partition_point(|&(start, _)| start <= c);
    index > 0 && c <= ranges[index - 1].1
}
fn upper_invariant(text: &[u16]) -> RibbonString {
    let mut out = Vec::with_capacity(text.len());
    for decoded in char::decode_utf16(text.iter().copied()) {
        match decoded {
            Err(error) => out.push(error.unpaired_surrogate()),
            Ok(c) => {
                let c = c as u32;
                let upper = data::UPPER
                    .binary_search_by_key(&c, |&(from, _)| from)
                    .map_or(c, |i| data::UPPER[i].1);
                out.extend_from_slice(
                    char::from_u32(upper)
                        .expect("validated casing scalar")
                        .encode_utf16(&mut [0; 2]),
                );
            }
        }
    }
    RibbonString::from_utf16(out)
}
fn normalize_key(key: &[u16]) -> RibbonString {
    let key = trim(key);
    if key.is_empty() {
        return RibbonString::default();
    }
    if let Some(alias) = lookup(key, ALIASES) {
        return alias.into();
    }
    if let Some(canonical) = lookup(key, CANONICAL) {
        return canonical.into();
    }
    if (key.len() == 2 || key.len() == 3) && matches!(key[0], 70 | 102) {
        let format = current_number_format();
        if let Some(function) = function_number(&key[1..], &format) {
            return format!("F{function}").into();
        }
    }
    if key.len() == 1 && in_ranges(key[0], data::LETTERS) {
        return upper_invariant(key);
    }
    if key.len() == 1 && in_ranges(key[0], data::DIGITS) {
        let mut out = "Number".encode_utf16().collect::<Vec<_>>();
        out.push(key[0]);
        return RibbonString::from_utf16(out);
    }
    let mut out = key.to_vec();
    if let Ok(i) = data::CHAR_UPPER.binary_search_by_key(&key[0], |&(from, _)| from) {
        out[0] = data::CHAR_UPPER[i].1;
    }
    RibbonString::from_utf16(out)
}
// The source only enters int.TryParse for a one- or two-unit suffix, so overflow,
// exponents, grouping and decimal syntax cannot produce a function number here.
fn function_number(mut text: &[u16], format: &RibbonGestureNumberFormat) -> Option<i32> {
    let white = |c: u16| matches!(c, 9..=13 | 32);
    while text.first().is_some_and(|&c| white(c)) {
        text = &text[1..];
    }
    let mut negative = false;
    {
        let positive = format.positive_sign.as_utf16();
        let minus = format.negative_sign.as_utf16();
        if !positive.is_empty() && text.starts_with(positive) {
            text = &text[positive.len()..];
        } else if !minus.is_empty() && text.starts_with(minus) {
            text = &text[minus.len()..];
            negative = true;
        }
    }
    let mut digits = 0;
    let mut value = 0;
    while text.first().is_some_and(|&c| (48..=57).contains(&c)) {
        value = value * 10 + i32::from(text[0] - 48);
        digits += 1;
        text = &text[1..];
    }
    while text.first().is_some_and(|&c| white(c)) {
        text = &text[1..];
    }
    (digits > 0 && !negative && text.iter().all(|&c| c == 0) && (1..=24).contains(&value))
        .then_some(value)
}
const ALIASES: &[(&str, &str)] = &[
    ("Esc", "Escape"),
    ("Del", "Delete"),
    ("Ins", "Insert"),
    ("Return", "Enter"),
    ("PgUp", "PageUp"),
    ("PgDn", "PageDown"),
    ("Plus", "Add"),
    ("+", "Add"),
    ("-", "Subtract"),
    ("Minus", "Subtract"),
    ("=", "Add"),
    ("Spacebar", "Space"),
    ("Backspace", "Back"),
    ("*", "Multiply"),
    ("Up", "Up"),
    ("Down", "Down"),
    ("Left", "Left"),
    ("Right", "Right"),
    ("[", "OemOpenBracket"),
    ("]", "OemCloseBracket"),
    (",", "OemComma"),
    (".", "OemPeriod"),
    ("/", "OemQuestion"),
    (";", "OemSemicolon"),
    ("'", "OemQuote"),
    ("\\", "OemBackslash"),
    ("`", "OemTilde"),
];
const CANONICAL: &[(&str, &str)] = &[
    ("Escape", "Escape"),
    ("Delete", "Delete"),
    ("Insert", "Insert"),
    ("Enter", "Enter"),
    ("PageUp", "PageUp"),
    ("PageDown", "PageDown"),
    ("Home", "Home"),
    ("End", "End"),
    ("Tab", "Tab"),
    ("Back", "Back"),
    ("Space", "Space"),
    ("Up", "Up"),
    ("Down", "Down"),
    ("Left", "Left"),
    ("Right", "Right"),
    ("Add", "Add"),
    ("Subtract", "Subtract"),
    ("Multiply", "Multiply"),
    ("Divide", "Divide"),
    ("Decimal", "Decimal"),
    ("Pause", "Pause"),
    ("Print", "Print"),
    ("Snapshot", "Snapshot"),
    ("Scroll", "Scroll"),
    ("Apps", "Apps"),
    ("Help", "Help"),
    ("Clear", "Clear"),
    ("Select", "Select"),
    ("Execute", "Execute"),
    ("Sleep", "Sleep"),
];
const DISPLAY: &[(&str, &str)] = &[
    ("Add", "+"),
    ("Subtract", "-"),
    ("OemOpenBracket", "["),
    ("OemCloseBracket", "]"),
    ("OemComma", ","),
    ("OemPeriod", "."),
    ("OemQuestion", "/"),
    ("OemSemicolon", ";"),
    ("OemQuote", "'"),
    ("OemBackslash", "\\"),
    ("OemTilde", "`"),
    ("Escape", "Esc"),
    ("Delete", "Del"),
    ("PageUp", "PgUp"),
    ("PageDown", "PgDn"),
];

pub(crate) fn current_number_format() -> RibbonGestureNumberFormat {
    let provider = NUMBER_FORMAT.with(|current| current.borrow().clone());
    provider.map_or_else(RibbonGestureNumberFormat::default, |p| p())
}
