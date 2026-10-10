// Generated from the upstream enum definition; do not reorder: the numeric
// values are part of the contract with platform backends.

use std::fmt;
use std::str::FromStr;

/// Represents a keyboard physical key.
/// 
/// The names follow the W3C codes, see <https://www.w3.org/TR/uievents-code/>.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum PhysicalKey {
    /// Represents no key.
    #[default]
    None = 0,
    /// ``~` on a US keyboard.
    /// This is the `半角/全角/漢字` (hankaku/zenkaku/kanji) key on Japanese keyboards.
    Backquote = 1,
    /// Used for both the US `\|` (on the 101-key layout) and also for the key located between the `"` and
    /// `Enter` keys on row C of the 102-, 104- and 106-key layouts.
    /// `#~` on a UK (102) keyboard.
    Backslash = 2,
    /// `[{` on a US keyboard.
    BracketLeft = 3,
    /// `]}` on a US keyboard.
    BracketRight = 4,
    /// `,&lt;` on a US keyboard.
    Comma = 5,
    /// `0)` on a US keyboard.
    Digit0 = 6,
    /// `1!` on a US keyboard.
    Digit1 = 7,
    /// `2@` on a US keyboard.
    Digit2 = 8,
    /// `3#` on a US keyboard.
    Digit3 = 9,
    /// `4$` on a US keyboard.
    Digit4 = 10,
    /// `5%` on a US keyboard.
    Digit5 = 11,
    /// `6^` on a US keyboard.
    Digit6 = 12,
    /// `7&amp;` on a US keyboard.
    Digit7 = 13,
    /// `8*` on a US keyboard.
    Digit8 = 14,
    /// `9(` on a US keyboard.
    Digit9 = 15,
    /// `=+` on a US keyboard.
    Equal = 16,
    /// Located between the left `Shift` and `Z` keys.
    /// `\|` on a UK keyboard.
    IntlBackslash = 17,
    /// Located between the `/` and right `Shift` keys.
    /// `\ろ` (ro) on a Japanese keyboard.
    IntlRo = 18,
    /// Located between the `=` and `Backspace` keys.
    /// `¥` (yen) on a Japanese keyboard.
    /// `\/` on a Russian keyboard.
    IntlYen = 19,
    /// `a` on a US keyboard.
    /// `q` on an AZERTY (e.g., French) keyboard.
    A = 20,
    /// `b` on a US keyboard.
    B = 21,
    /// `c` on a US keyboard.
    C = 22,
    /// `d` on a US keyboard.
    D = 23,
    /// `e` on a US keyboard.
    E = 24,
    /// `f` on a US keyboard.
    F = 25,
    /// `g` on a US keyboard.
    G = 26,
    /// `h` on a US keyboard.
    H = 27,
    /// `i` on a US keyboard.
    I = 28,
    /// `j` on a US keyboard.
    J = 29,
    /// `k` on a US keyboard.
    K = 30,
    /// `l` on a US keyboard.
    L = 31,
    /// `m` on a US keyboard.
    M = 32,
    /// `n` on a US keyboard.
    N = 33,
    /// `o` on a US keyboard.
    O = 34,
    /// `p` on a US keyboard.
    P = 35,
    /// `q` on a US keyboard.
    /// `a` on an AZERTY (e.g., French) keyboard.
    Q = 36,
    /// `r` on a US keyboard.
    R = 37,
    /// `s` on a US keyboard.
    S = 38,
    /// `t` on a US keyboard.
    T = 39,
    /// `u` on a US keyboard.
    U = 40,
    /// `v` on a US keyboard.
    V = 41,
    /// `w` on a US keyboard.
    /// `z` on an AZERTY (e.g., French) keyboard.
    W = 42,
    /// `x` on a US keyboard.
    X = 43,
    /// `y` on a US keyboard.
    /// `z` on a QWERTZ (e.g., German) keyboard.
    Y = 44,
    /// `z` on a US keyboard.
    /// `w` on an AZERTY (e.g., French) keyboard.
    /// `y` on a QWERTZ (e.g., German) keyboard.
    Z = 45,
    /// `-_` on a US keyboard.
    Minus = 46,
    /// `.&gt;` on a US keyboard.
    Period = 47,
    /// `'"` on a US keyboard.
    Quote = 48,
    /// `;:` on a US keyboard.
    Semicolon = 49,
    /// `/?` on a US keyboard.
    Slash = 50,
    /// `Alt`, `Option` or `⌥`.
    AltLeft = 51,
    /// `Alt`, `Option` or `⌥`.
    /// This is labelled `AltGr` key on many keyboard layouts.
    AltRight = 52,
    /// `Backspace` or `⌫`.
    /// Labelled `Delete` on Apple keyboards.
    Backspace = 53,
    /// `CapsLock` or `⇪`.
    CapsLock = 54,
    /// The application context menu key, which is typically found between the right `Meta` key
    /// and the right `Control` key.
    ContextMenu = 55,
    /// `Control` or `⌃`.
    ControlLeft = 56,
    /// `Control` or `⌃`.
    ControlRight = 57,
    /// `Enter` or `↵`.
    /// Labelled `Return` on Apple keyboards.
    Enter = 58,
    /// The `⊞` (Windows), `⌘`, `Command` or other OS symbol key.
    MetaLeft = 59,
    /// The `⊞` (Windows), `⌘`, `Command` or other OS symbol key.
    MetaRight = 60,
    /// `Shift` or `⇧`.
    ShiftLeft = 61,
    /// `Shift` or `⇧`.
    ShiftRight = 62,
    /// ` ` (space).
    Space = 63,
    /// `Tab` or `⇥`.
    Tab = 64,
    /// Japanese: `変換` (henkan).
    Convert = 65,
    /// Japanese: `カタカナ/ひらがな/ローマ字` (katakana/hiragana/romaji).
    KanaMode = 66,
    /// Korean: HangulMode `한/영` (han/yeong).
    /// Japanese (Mac keyboard): `かな` (kana).
    Lang1 = 67,
    /// Korean: Hanja `한자` (hanja).
    /// Japanese (Mac keyboard): `英数` (eisu).
    Lang2 = 68,
    /// Japanese (word-processing keyboard): Katakana.
    Lang3 = 69,
    /// Japanese (word-processing keyboard): Hiragana.
    Lang4 = 70,
    /// Japanese (word-processing keyboard): Zenkaku/Hankaku.
    Lang5 = 71,
    /// Japanese: `無変換` (muhenkan).
    NonConvert = 72,
    /// `⌦`. The forward delete key.
    /// Note that on Apple keyboards, the key labelled `Delete` on the main part of the keyboard is
    /// `Backspace`.
    Delete = 73,
    /// `End` or `↘`.
    End = 74,
    /// `Help`.
    /// Not present on standard PC keyboards.
    Help = 75,
    /// `Home` or `↖`.
    Home = 76,
    /// `Insert` or `Ins`.
    /// Not present on Apple keyboards.
    Insert = 77,
    /// `Page Down`, `PgDn` or `⇟`.
    PageDown = 78,
    /// `Page Up`, `PgUp` or `⇞`.
    PageUp = 79,
    /// `↓`.
    ArrowDown = 80,
    /// `←`.
    ArrowLeft = 81,
    /// `→`.
    ArrowRight = 82,
    /// `↑`.
    ArrowUp = 83,
    /// Numeric keypad `Num Lock`.
    /// On the Mac, this is used for the numpad `Clear` key.
    NumLock = 84,
    /// Numeric keypad `0 Ins` on a keyboard.
    /// `0` on a phone or remote control.
    NumPad0 = 85,
    /// Numeric keypad `1 End` on a keyboard.
    /// `1` or `1 QZ` on a phone or remote control.
    NumPad1 = 86,
    /// Numeric keypad `2 ↓` on a keyboard.
    /// `2 ABC` on a phone or remote control.
    NumPad2 = 87,
    /// Numeric keypad `3 PgDn` on a keyboard.
    /// `3 DEF` on a phone or remote control.
    NumPad3 = 88,
    /// Numeric keypad `4 ←` on a keyboard.
    /// `4 GHI` on a phone or remote control.
    NumPad4 = 89,
    /// Numeric keypad `5` on a keyboard.
    /// `5 JKL` on a phone or remote control.
    NumPad5 = 90,
    /// Numeric keypad `6 →` on a keyboard.
    /// `6 MNO` on a phone or remote control.
    NumPad6 = 91,
    /// Numeric keypad `7 Home` on a keyboard.
    /// `7 PQRS` or `7 PRS` on a phone or remote control.
    NumPad7 = 92,
    /// Numeric keypad `8 ↑` on a keyboard.
    /// `8 TUV` on a phone or remote control.
    NumPad8 = 93,
    /// Numeric keypad `9 PgUp` on a keyboard.
    /// `9 WXYZ` or `9 WXY` on a phone or remote control.
    NumPad9 = 94,
    /// Numeric keypad `+`.
    NumPadAdd = 95,
    /// Numeric keypad `C` or `AC` (All Clear).
    /// Also for use with numpads that have a `Clear` key that is separate from the `NumLock` key.
    /// On the Mac, the numpad `Clear` key is `NumLock`.
    NumPadClear = 96,
    /// Numeric keypad `,` (thousands separator).
    /// For locales where the thousands separator is a "." (e.g., Brazil), this key may generate a `.`.
    NumPadComma = 97,
    /// Numeric keypad `. Del`.
    /// For locales where the decimal separator is "," (e.g., Brazil), this key may generate a `,`.
    NumPadDecimal = 98,
    /// Numeric keypad `/`.
    NumPadDivide = 99,
    /// Numeric keypad `Enter`.
    NumPadEnter = 100,
    /// Numeric keypad `=`.
    NumPadEqual = 101,
    /// Numeric keypad `*` on a keyboard.
    /// For use with numpads that provide mathematical operations (`+`, `-`, `*` and `/`).
    NumPadMultiply = 102,
    /// Numeric keypad `(`.
    /// Found on the Microsoft Natural Keyboard.
    NumPadParenLeft = 103,
    /// Numeric keypad `)`.
    /// Found on the Microsoft Natural Keyboard.
    NumPadParenRight = 104,
    /// Numeric keypad `-`.
    NumPadSubtract = 105,
    /// `Esc` or `⎋`.
    Escape = 106,
    /// `F1`.
    F1 = 107,
    /// `F2`.
    F2 = 108,
    /// `F3`.
    F3 = 109,
    /// `F4`.
    F4 = 110,
    /// `F5`.
    F5 = 111,
    /// `F6`.
    F6 = 112,
    /// `F7`.
    F7 = 113,
    /// `F8`.
    F8 = 114,
    /// `F9`.
    F9 = 115,
    /// `F10`.
    F10 = 116,
    /// `F11`.
    F11 = 117,
    /// `F12`.
    F12 = 118,
    /// `F13`.
    F13 = 119,
    /// `F14`.
    F14 = 120,
    /// `F15`.
    F15 = 121,
    /// `F16`.
    F16 = 122,
    /// `F17`.
    F17 = 123,
    /// `F18`.
    F18 = 124,
    /// `F19`.
    F19 = 125,
    /// `F20`.
    F20 = 126,
    /// `F21`.
    F21 = 127,
    /// `F22`.
    F22 = 128,
    /// `F23`.
    F23 = 129,
    /// `F24`.
    F24 = 130,
    /// `PrtScr SysRq` or `Print Screen`.
    PrintScreen = 131,
    /// `Scroll Lock`.
    ScrollLock = 132,
    /// `Pause Break`.
    Pause = 133,
    /// Browser `Back`.
    /// Some laptops place this key to the left of the `↑` key.
    BrowserBack = 134,
    /// Browser `Favorites`.
    BrowserFavorites = 135,
    /// Browser `Forward`.
    /// Some laptops place this key to the right of the `↑` key.
    BrowserForward = 136,
    /// Browser `Home`.
    BrowserHome = 137,
    /// Browser `Refresh`.
    BrowserRefresh = 138,
    /// Browser `Search`.
    BrowserSearch = 139,
    /// Browser `Stop`.
    BrowserStop = 140,
    /// `Eject` or `⏏`.
    /// This key is placed in the function section on some Apple keyboards.
    Eject = 141,
    /// `App 1`.
    /// Sometimes labelled `My Computer` on the keyboard.
    LaunchApp1 = 142,
    /// `App 2`.
    /// Sometimes labelled `Calculator` on the keyboard.
    LaunchApp2 = 143,
    /// `Mail`.
    LaunchMail = 144,
    /// Media `Play/Pause` or `⏵⏸`.
    MediaPlayPause = 145,
    /// Media `Select`.
    MediaSelect = 146,
    /// Media `Stop` or `⏹`.
    MediaStop = 147,
    /// Media `Next` or `⏭`.
    MediaTrackNext = 148,
    /// Media `Previous` or `⏮`.
    MediaTrackPrevious = 149,
    /// `Power`.
    Power = 150,
    /// `Sleep`.
    Sleep = 151,
    /// `Volume Down`.
    AudioVolumeDown = 152,
    /// `Mute`.
    AudioVolumeMute = 153,
    /// `Volume Up`.
    AudioVolumeUp = 154,
    /// `Wake Up`.
    WakeUp = 155,
    /// `Again`.
    /// Legacy.
    /// Found on Sun’s USB keyboard.
    Again = 156,
    /// `Copy`.
    /// Legacy.
    /// Found on Sun’s USB keyboard.
    Copy = 157,
    /// `Cut`.
    /// Legacy.
    /// Found on Sun’s USB keyboard.
    Cut = 158,
    /// `Find`.
    /// Legacy.
    /// Found on Sun’s USB keyboard.
    Find = 159,
    /// `Open`.
    /// Legacy.
    /// Found on Sun’s USB keyboard.
    Open = 160,
    /// `Paste`.
    /// Legacy.
    /// Found on Sun’s USB keyboard.
    Paste = 161,
    /// `Props`.
    /// Legacy.
    /// Found on Sun’s USB keyboard.
    Props = 162,
    /// `Select`.
    /// Legacy.
    /// Found on Sun’s USB keyboard.
    Select = 163,
    /// `Undo`.
    /// Legacy.
    /// Found on Sun’s USB keyboard.
    Undo = 164,
}

/// The error returned when a string does not name a [`PhysicalKey`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsePhysicalKeyError(pub String);

impl fmt::Display for ParsePhysicalKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Requested value '{}' was not found.", self.0)
    }
}

impl std::error::Error for ParsePhysicalKeyError {}

/// Every name of the enumeration, including aliases, with its value.
static NAMES: &[(&str, PhysicalKey)] = &[
    ("None", PhysicalKey::None),
    ("Backquote", PhysicalKey::Backquote),
    ("Backslash", PhysicalKey::Backslash),
    ("BracketLeft", PhysicalKey::BracketLeft),
    ("BracketRight", PhysicalKey::BracketRight),
    ("Comma", PhysicalKey::Comma),
    ("Digit0", PhysicalKey::Digit0),
    ("Digit1", PhysicalKey::Digit1),
    ("Digit2", PhysicalKey::Digit2),
    ("Digit3", PhysicalKey::Digit3),
    ("Digit4", PhysicalKey::Digit4),
    ("Digit5", PhysicalKey::Digit5),
    ("Digit6", PhysicalKey::Digit6),
    ("Digit7", PhysicalKey::Digit7),
    ("Digit8", PhysicalKey::Digit8),
    ("Digit9", PhysicalKey::Digit9),
    ("Equal", PhysicalKey::Equal),
    ("IntlBackslash", PhysicalKey::IntlBackslash),
    ("IntlRo", PhysicalKey::IntlRo),
    ("IntlYen", PhysicalKey::IntlYen),
    ("A", PhysicalKey::A),
    ("B", PhysicalKey::B),
    ("C", PhysicalKey::C),
    ("D", PhysicalKey::D),
    ("E", PhysicalKey::E),
    ("F", PhysicalKey::F),
    ("G", PhysicalKey::G),
    ("H", PhysicalKey::H),
    ("I", PhysicalKey::I),
    ("J", PhysicalKey::J),
    ("K", PhysicalKey::K),
    ("L", PhysicalKey::L),
    ("M", PhysicalKey::M),
    ("N", PhysicalKey::N),
    ("O", PhysicalKey::O),
    ("P", PhysicalKey::P),
    ("Q", PhysicalKey::Q),
    ("R", PhysicalKey::R),
    ("S", PhysicalKey::S),
    ("T", PhysicalKey::T),
    ("U", PhysicalKey::U),
    ("V", PhysicalKey::V),
    ("W", PhysicalKey::W),
    ("X", PhysicalKey::X),
    ("Y", PhysicalKey::Y),
    ("Z", PhysicalKey::Z),
    ("Minus", PhysicalKey::Minus),
    ("Period", PhysicalKey::Period),
    ("Quote", PhysicalKey::Quote),
    ("Semicolon", PhysicalKey::Semicolon),
    ("Slash", PhysicalKey::Slash),
    ("AltLeft", PhysicalKey::AltLeft),
    ("AltRight", PhysicalKey::AltRight),
    ("Backspace", PhysicalKey::Backspace),
    ("CapsLock", PhysicalKey::CapsLock),
    ("ContextMenu", PhysicalKey::ContextMenu),
    ("ControlLeft", PhysicalKey::ControlLeft),
    ("ControlRight", PhysicalKey::ControlRight),
    ("Enter", PhysicalKey::Enter),
    ("MetaLeft", PhysicalKey::MetaLeft),
    ("MetaRight", PhysicalKey::MetaRight),
    ("ShiftLeft", PhysicalKey::ShiftLeft),
    ("ShiftRight", PhysicalKey::ShiftRight),
    ("Space", PhysicalKey::Space),
    ("Tab", PhysicalKey::Tab),
    ("Convert", PhysicalKey::Convert),
    ("KanaMode", PhysicalKey::KanaMode),
    ("Lang1", PhysicalKey::Lang1),
    ("Lang2", PhysicalKey::Lang2),
    ("Lang3", PhysicalKey::Lang3),
    ("Lang4", PhysicalKey::Lang4),
    ("Lang5", PhysicalKey::Lang5),
    ("NonConvert", PhysicalKey::NonConvert),
    ("Delete", PhysicalKey::Delete),
    ("End", PhysicalKey::End),
    ("Help", PhysicalKey::Help),
    ("Home", PhysicalKey::Home),
    ("Insert", PhysicalKey::Insert),
    ("PageDown", PhysicalKey::PageDown),
    ("PageUp", PhysicalKey::PageUp),
    ("ArrowDown", PhysicalKey::ArrowDown),
    ("ArrowLeft", PhysicalKey::ArrowLeft),
    ("ArrowRight", PhysicalKey::ArrowRight),
    ("ArrowUp", PhysicalKey::ArrowUp),
    ("NumLock", PhysicalKey::NumLock),
    ("NumPad0", PhysicalKey::NumPad0),
    ("NumPad1", PhysicalKey::NumPad1),
    ("NumPad2", PhysicalKey::NumPad2),
    ("NumPad3", PhysicalKey::NumPad3),
    ("NumPad4", PhysicalKey::NumPad4),
    ("NumPad5", PhysicalKey::NumPad5),
    ("NumPad6", PhysicalKey::NumPad6),
    ("NumPad7", PhysicalKey::NumPad7),
    ("NumPad8", PhysicalKey::NumPad8),
    ("NumPad9", PhysicalKey::NumPad9),
    ("NumPadAdd", PhysicalKey::NumPadAdd),
    ("NumPadClear", PhysicalKey::NumPadClear),
    ("NumPadComma", PhysicalKey::NumPadComma),
    ("NumPadDecimal", PhysicalKey::NumPadDecimal),
    ("NumPadDivide", PhysicalKey::NumPadDivide),
    ("NumPadEnter", PhysicalKey::NumPadEnter),
    ("NumPadEqual", PhysicalKey::NumPadEqual),
    ("NumPadMultiply", PhysicalKey::NumPadMultiply),
    ("NumPadParenLeft", PhysicalKey::NumPadParenLeft),
    ("NumPadParenRight", PhysicalKey::NumPadParenRight),
    ("NumPadSubtract", PhysicalKey::NumPadSubtract),
    ("Escape", PhysicalKey::Escape),
    ("F1", PhysicalKey::F1),
    ("F2", PhysicalKey::F2),
    ("F3", PhysicalKey::F3),
    ("F4", PhysicalKey::F4),
    ("F5", PhysicalKey::F5),
    ("F6", PhysicalKey::F6),
    ("F7", PhysicalKey::F7),
    ("F8", PhysicalKey::F8),
    ("F9", PhysicalKey::F9),
    ("F10", PhysicalKey::F10),
    ("F11", PhysicalKey::F11),
    ("F12", PhysicalKey::F12),
    ("F13", PhysicalKey::F13),
    ("F14", PhysicalKey::F14),
    ("F15", PhysicalKey::F15),
    ("F16", PhysicalKey::F16),
    ("F17", PhysicalKey::F17),
    ("F18", PhysicalKey::F18),
    ("F19", PhysicalKey::F19),
    ("F20", PhysicalKey::F20),
    ("F21", PhysicalKey::F21),
    ("F22", PhysicalKey::F22),
    ("F23", PhysicalKey::F23),
    ("F24", PhysicalKey::F24),
    ("PrintScreen", PhysicalKey::PrintScreen),
    ("ScrollLock", PhysicalKey::ScrollLock),
    ("Pause", PhysicalKey::Pause),
    ("BrowserBack", PhysicalKey::BrowserBack),
    ("BrowserFavorites", PhysicalKey::BrowserFavorites),
    ("BrowserForward", PhysicalKey::BrowserForward),
    ("BrowserHome", PhysicalKey::BrowserHome),
    ("BrowserRefresh", PhysicalKey::BrowserRefresh),
    ("BrowserSearch", PhysicalKey::BrowserSearch),
    ("BrowserStop", PhysicalKey::BrowserStop),
    ("Eject", PhysicalKey::Eject),
    ("LaunchApp1", PhysicalKey::LaunchApp1),
    ("LaunchApp2", PhysicalKey::LaunchApp2),
    ("LaunchMail", PhysicalKey::LaunchMail),
    ("MediaPlayPause", PhysicalKey::MediaPlayPause),
    ("MediaSelect", PhysicalKey::MediaSelect),
    ("MediaStop", PhysicalKey::MediaStop),
    ("MediaTrackNext", PhysicalKey::MediaTrackNext),
    ("MediaTrackPrevious", PhysicalKey::MediaTrackPrevious),
    ("Power", PhysicalKey::Power),
    ("Sleep", PhysicalKey::Sleep),
    ("AudioVolumeDown", PhysicalKey::AudioVolumeDown),
    ("AudioVolumeMute", PhysicalKey::AudioVolumeMute),
    ("AudioVolumeUp", PhysicalKey::AudioVolumeUp),
    ("WakeUp", PhysicalKey::WakeUp),
    ("Again", PhysicalKey::Again),
    ("Copy", PhysicalKey::Copy),
    ("Cut", PhysicalKey::Cut),
    ("Find", PhysicalKey::Find),
    ("Open", PhysicalKey::Open),
    ("Paste", PhysicalKey::Paste),
    ("Props", PhysicalKey::Props),
    ("Select", PhysicalKey::Select),
    ("Undo", PhysicalKey::Undo),
];

#[allow(non_upper_case_globals)]
impl PhysicalKey {
    /// The numeric value of the member.
    #[inline]
    pub const fn value(self) -> i32 {
        self as i32
    }

    /// The member with the given numeric value, if one is defined.
    pub fn from_value(value: i32) -> Option<PhysicalKey> {
        NAMES.iter().map(|(_, k)| *k).find(|k| *k as i32 == value)
    }

    /// The name of the member. For values with several names this is the
    /// first one declared.
    pub fn name(self) -> &'static str {
        match self {
            PhysicalKey::None => "None",
            PhysicalKey::Backquote => "Backquote",
            PhysicalKey::Backslash => "Backslash",
            PhysicalKey::BracketLeft => "BracketLeft",
            PhysicalKey::BracketRight => "BracketRight",
            PhysicalKey::Comma => "Comma",
            PhysicalKey::Digit0 => "Digit0",
            PhysicalKey::Digit1 => "Digit1",
            PhysicalKey::Digit2 => "Digit2",
            PhysicalKey::Digit3 => "Digit3",
            PhysicalKey::Digit4 => "Digit4",
            PhysicalKey::Digit5 => "Digit5",
            PhysicalKey::Digit6 => "Digit6",
            PhysicalKey::Digit7 => "Digit7",
            PhysicalKey::Digit8 => "Digit8",
            PhysicalKey::Digit9 => "Digit9",
            PhysicalKey::Equal => "Equal",
            PhysicalKey::IntlBackslash => "IntlBackslash",
            PhysicalKey::IntlRo => "IntlRo",
            PhysicalKey::IntlYen => "IntlYen",
            PhysicalKey::A => "A",
            PhysicalKey::B => "B",
            PhysicalKey::C => "C",
            PhysicalKey::D => "D",
            PhysicalKey::E => "E",
            PhysicalKey::F => "F",
            PhysicalKey::G => "G",
            PhysicalKey::H => "H",
            PhysicalKey::I => "I",
            PhysicalKey::J => "J",
            PhysicalKey::K => "K",
            PhysicalKey::L => "L",
            PhysicalKey::M => "M",
            PhysicalKey::N => "N",
            PhysicalKey::O => "O",
            PhysicalKey::P => "P",
            PhysicalKey::Q => "Q",
            PhysicalKey::R => "R",
            PhysicalKey::S => "S",
            PhysicalKey::T => "T",
            PhysicalKey::U => "U",
            PhysicalKey::V => "V",
            PhysicalKey::W => "W",
            PhysicalKey::X => "X",
            PhysicalKey::Y => "Y",
            PhysicalKey::Z => "Z",
            PhysicalKey::Minus => "Minus",
            PhysicalKey::Period => "Period",
            PhysicalKey::Quote => "Quote",
            PhysicalKey::Semicolon => "Semicolon",
            PhysicalKey::Slash => "Slash",
            PhysicalKey::AltLeft => "AltLeft",
            PhysicalKey::AltRight => "AltRight",
            PhysicalKey::Backspace => "Backspace",
            PhysicalKey::CapsLock => "CapsLock",
            PhysicalKey::ContextMenu => "ContextMenu",
            PhysicalKey::ControlLeft => "ControlLeft",
            PhysicalKey::ControlRight => "ControlRight",
            PhysicalKey::Enter => "Enter",
            PhysicalKey::MetaLeft => "MetaLeft",
            PhysicalKey::MetaRight => "MetaRight",
            PhysicalKey::ShiftLeft => "ShiftLeft",
            PhysicalKey::ShiftRight => "ShiftRight",
            PhysicalKey::Space => "Space",
            PhysicalKey::Tab => "Tab",
            PhysicalKey::Convert => "Convert",
            PhysicalKey::KanaMode => "KanaMode",
            PhysicalKey::Lang1 => "Lang1",
            PhysicalKey::Lang2 => "Lang2",
            PhysicalKey::Lang3 => "Lang3",
            PhysicalKey::Lang4 => "Lang4",
            PhysicalKey::Lang5 => "Lang5",
            PhysicalKey::NonConvert => "NonConvert",
            PhysicalKey::Delete => "Delete",
            PhysicalKey::End => "End",
            PhysicalKey::Help => "Help",
            PhysicalKey::Home => "Home",
            PhysicalKey::Insert => "Insert",
            PhysicalKey::PageDown => "PageDown",
            PhysicalKey::PageUp => "PageUp",
            PhysicalKey::ArrowDown => "ArrowDown",
            PhysicalKey::ArrowLeft => "ArrowLeft",
            PhysicalKey::ArrowRight => "ArrowRight",
            PhysicalKey::ArrowUp => "ArrowUp",
            PhysicalKey::NumLock => "NumLock",
            PhysicalKey::NumPad0 => "NumPad0",
            PhysicalKey::NumPad1 => "NumPad1",
            PhysicalKey::NumPad2 => "NumPad2",
            PhysicalKey::NumPad3 => "NumPad3",
            PhysicalKey::NumPad4 => "NumPad4",
            PhysicalKey::NumPad5 => "NumPad5",
            PhysicalKey::NumPad6 => "NumPad6",
            PhysicalKey::NumPad7 => "NumPad7",
            PhysicalKey::NumPad8 => "NumPad8",
            PhysicalKey::NumPad9 => "NumPad9",
            PhysicalKey::NumPadAdd => "NumPadAdd",
            PhysicalKey::NumPadClear => "NumPadClear",
            PhysicalKey::NumPadComma => "NumPadComma",
            PhysicalKey::NumPadDecimal => "NumPadDecimal",
            PhysicalKey::NumPadDivide => "NumPadDivide",
            PhysicalKey::NumPadEnter => "NumPadEnter",
            PhysicalKey::NumPadEqual => "NumPadEqual",
            PhysicalKey::NumPadMultiply => "NumPadMultiply",
            PhysicalKey::NumPadParenLeft => "NumPadParenLeft",
            PhysicalKey::NumPadParenRight => "NumPadParenRight",
            PhysicalKey::NumPadSubtract => "NumPadSubtract",
            PhysicalKey::Escape => "Escape",
            PhysicalKey::F1 => "F1",
            PhysicalKey::F2 => "F2",
            PhysicalKey::F3 => "F3",
            PhysicalKey::F4 => "F4",
            PhysicalKey::F5 => "F5",
            PhysicalKey::F6 => "F6",
            PhysicalKey::F7 => "F7",
            PhysicalKey::F8 => "F8",
            PhysicalKey::F9 => "F9",
            PhysicalKey::F10 => "F10",
            PhysicalKey::F11 => "F11",
            PhysicalKey::F12 => "F12",
            PhysicalKey::F13 => "F13",
            PhysicalKey::F14 => "F14",
            PhysicalKey::F15 => "F15",
            PhysicalKey::F16 => "F16",
            PhysicalKey::F17 => "F17",
            PhysicalKey::F18 => "F18",
            PhysicalKey::F19 => "F19",
            PhysicalKey::F20 => "F20",
            PhysicalKey::F21 => "F21",
            PhysicalKey::F22 => "F22",
            PhysicalKey::F23 => "F23",
            PhysicalKey::F24 => "F24",
            PhysicalKey::PrintScreen => "PrintScreen",
            PhysicalKey::ScrollLock => "ScrollLock",
            PhysicalKey::Pause => "Pause",
            PhysicalKey::BrowserBack => "BrowserBack",
            PhysicalKey::BrowserFavorites => "BrowserFavorites",
            PhysicalKey::BrowserForward => "BrowserForward",
            PhysicalKey::BrowserHome => "BrowserHome",
            PhysicalKey::BrowserRefresh => "BrowserRefresh",
            PhysicalKey::BrowserSearch => "BrowserSearch",
            PhysicalKey::BrowserStop => "BrowserStop",
            PhysicalKey::Eject => "Eject",
            PhysicalKey::LaunchApp1 => "LaunchApp1",
            PhysicalKey::LaunchApp2 => "LaunchApp2",
            PhysicalKey::LaunchMail => "LaunchMail",
            PhysicalKey::MediaPlayPause => "MediaPlayPause",
            PhysicalKey::MediaSelect => "MediaSelect",
            PhysicalKey::MediaStop => "MediaStop",
            PhysicalKey::MediaTrackNext => "MediaTrackNext",
            PhysicalKey::MediaTrackPrevious => "MediaTrackPrevious",
            PhysicalKey::Power => "Power",
            PhysicalKey::Sleep => "Sleep",
            PhysicalKey::AudioVolumeDown => "AudioVolumeDown",
            PhysicalKey::AudioVolumeMute => "AudioVolumeMute",
            PhysicalKey::AudioVolumeUp => "AudioVolumeUp",
            PhysicalKey::WakeUp => "WakeUp",
            PhysicalKey::Again => "Again",
            PhysicalKey::Copy => "Copy",
            PhysicalKey::Cut => "Cut",
            PhysicalKey::Find => "Find",
            PhysicalKey::Open => "Open",
            PhysicalKey::Paste => "Paste",
            PhysicalKey::Props => "Props",
            PhysicalKey::Select => "Select",
            PhysicalKey::Undo => "Undo",
        }
    }

    /// Parses a member name (ignoring case, aliases included) or the
    /// numeric value of a defined member.
    pub fn parse(s: &str) -> Result<PhysicalKey, ParsePhysicalKeyError> {
        let s = s.trim();
        if let Some((_, value)) = NAMES.iter().find(|(name, _)| name.eq_ignore_ascii_case(s)) {
            return Ok(*value);
        }
        if let Ok(number) = s.parse::<i32>() {
            if let Some(value) = Self::from_value(number) {
                return Ok(value);
            }
        }
        Err(ParsePhysicalKeyError(s.to_string()))
    }
}

impl FromStr for PhysicalKey {
    type Err = ParsePhysicalKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for PhysicalKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
