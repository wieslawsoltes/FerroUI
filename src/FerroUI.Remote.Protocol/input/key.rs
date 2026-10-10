// Generated from the upstream enum definition; do not reorder: the numeric
// values are part of the contract with platform backends.

use std::fmt;
use std::str::FromStr;

/// Defines the keys available on a keyboard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum Key {
    /// No key pressed.
    #[default]
    None = 0,
    /// The Cancel key.
    Cancel = 1,
    /// The Back key.
    Back = 2,
    /// The Tab key.
    Tab = 3,
    /// The Linefeed key.
    LineFeed = 4,
    /// The Clear key.
    Clear = 5,
    /// The Return key.
    Return = 6,
    /// The Pause key.
    Pause = 7,
    /// The Caps Lock key.
    CapsLock = 8,
    /// The IME Hangul mode key.
    HangulMode = 9,
    /// The IME Junja mode key.
    JunjaMode = 10,
    /// The IME Final mode key.
    FinalMode = 11,
    /// The IME Kanji mode key.
    KanjiMode = 12,
    /// The Escape key.
    Escape = 13,
    /// The IME Convert key.
    ImeConvert = 14,
    /// The IME NonConvert key.
    ImeNonConvert = 15,
    /// The IME Accept key.
    ImeAccept = 16,
    /// The IME Mode change key.
    ImeModeChange = 17,
    /// The space bar.
    Space = 18,
    /// The Page Up key.
    PageUp = 19,
    /// The Page Down key.
    PageDown = 20,
    /// The End key.
    End = 21,
    /// The Home key.
    Home = 22,
    /// The Left arrow key.
    Left = 23,
    /// The Up arrow key.
    Up = 24,
    /// The Right arrow key.
    Right = 25,
    /// The Down arrow key.
    Down = 26,
    /// The Select key.
    Select = 27,
    /// The Print key.
    Print = 28,
    /// The Execute key.
    Execute = 29,
    /// The Print Screen key.
    Snapshot = 30,
    /// The Insert key.
    Insert = 31,
    /// The Delete key.
    Delete = 32,
    /// The Help key.
    Help = 33,
    /// The 0 key.
    D0 = 34,
    /// The 1 key.
    D1 = 35,
    /// The 2 key.
    D2 = 36,
    /// The 3 key.
    D3 = 37,
    /// The 4 key.
    D4 = 38,
    /// The 5 key.
    D5 = 39,
    /// The 6 key.
    D6 = 40,
    /// The 7 key.
    D7 = 41,
    /// The 8 key.
    D8 = 42,
    /// The 9 key.
    D9 = 43,
    /// The A key.
    A = 44,
    /// The B key.
    B = 45,
    /// The C key.
    C = 46,
    /// The D key.
    D = 47,
    /// The E key.
    E = 48,
    /// The F key.
    F = 49,
    /// The G key.
    G = 50,
    /// The H key.
    H = 51,
    /// The I key.
    I = 52,
    /// The J key.
    J = 53,
    /// The K key.
    K = 54,
    /// The L key.
    L = 55,
    /// The M key.
    M = 56,
    /// The N key.
    N = 57,
    /// The O key.
    O = 58,
    /// The P key.
    P = 59,
    /// The Q key.
    Q = 60,
    /// The R key.
    R = 61,
    /// The S key.
    S = 62,
    /// The T key.
    T = 63,
    /// The U key.
    U = 64,
    /// The V key.
    V = 65,
    /// The W key.
    W = 66,
    /// The X key.
    X = 67,
    /// The Y key.
    Y = 68,
    /// The Z key.
    Z = 69,
    /// The left Windows key.
    LWin = 70,
    /// The right Windows key.
    RWin = 71,
    /// The Application key.
    Apps = 72,
    /// The Sleep key.
    Sleep = 73,
    /// The 0 key on the numeric keypad.
    NumPad0 = 74,
    /// The 1 key on the numeric keypad.
    NumPad1 = 75,
    /// The 2 key on the numeric keypad.
    NumPad2 = 76,
    /// The 3 key on the numeric keypad.
    NumPad3 = 77,
    /// The 4 key on the numeric keypad.
    NumPad4 = 78,
    /// The 5 key on the numeric keypad.
    NumPad5 = 79,
    /// The 6 key on the numeric keypad.
    NumPad6 = 80,
    /// The 7 key on the numeric keypad.
    NumPad7 = 81,
    /// The 8 key on the numeric keypad.
    NumPad8 = 82,
    /// The 9 key on the numeric keypad.
    NumPad9 = 83,
    /// The Multiply key.
    Multiply = 84,
    /// The Add key.
    Add = 85,
    /// The Separator key.
    Separator = 86,
    /// The Subtract key.
    Subtract = 87,
    /// The Decimal key.
    Decimal = 88,
    /// The Divide key.
    Divide = 89,
    /// The F1 key.
    F1 = 90,
    /// The F2 key.
    F2 = 91,
    /// The F3 key.
    F3 = 92,
    /// The F4 key.
    F4 = 93,
    /// The F5 key.
    F5 = 94,
    /// The F6 key.
    F6 = 95,
    /// The F7 key.
    F7 = 96,
    /// The F8 key.
    F8 = 97,
    /// The F9 key.
    F9 = 98,
    /// The F10 key.
    F10 = 99,
    /// The F11 key.
    F11 = 100,
    /// The F12 key.
    F12 = 101,
    /// The F13 key.
    F13 = 102,
    /// The F14 key.
    F14 = 103,
    /// The F15 key.
    F15 = 104,
    /// The F16 key.
    F16 = 105,
    /// The F17 key.
    F17 = 106,
    /// The F18 key.
    F18 = 107,
    /// The F19 key.
    F19 = 108,
    /// The F20 key.
    F20 = 109,
    /// The F21 key.
    F21 = 110,
    /// The F22 key.
    F22 = 111,
    /// The F23 key.
    F23 = 112,
    /// The F24 key.
    F24 = 113,
    /// The Numlock key.
    NumLock = 114,
    /// The Scroll key.
    Scroll = 115,
    /// The left Shift key.
    LeftShift = 116,
    /// The right Shift key.
    RightShift = 117,
    /// The left Ctrl key.
    LeftCtrl = 118,
    /// The right Ctrl key.
    RightCtrl = 119,
    /// The left Alt key.
    LeftAlt = 120,
    /// The right Alt key.
    RightAlt = 121,
    /// The browser Back key.
    BrowserBack = 122,
    /// The browser Forward key.
    BrowserForward = 123,
    /// The browser Refresh key.
    BrowserRefresh = 124,
    /// The browser Stop key.
    BrowserStop = 125,
    /// The browser Search key.
    BrowserSearch = 126,
    /// The browser Favorites key.
    BrowserFavorites = 127,
    /// The browser Home key.
    BrowserHome = 128,
    /// The Volume Mute key.
    VolumeMute = 129,
    /// The Volume Down key.
    VolumeDown = 130,
    /// The Volume Up key.
    VolumeUp = 131,
    /// The media Next Track key.
    MediaNextTrack = 132,
    /// The media Previous Track key.
    MediaPreviousTrack = 133,
    /// The media Stop key.
    MediaStop = 134,
    /// The media Play/Pause key.
    MediaPlayPause = 135,
    /// The Launch Mail key.
    LaunchMail = 136,
    /// The Select Media key.
    SelectMedia = 137,
    /// The Launch Application 1 key.
    LaunchApplication1 = 138,
    /// The Launch Application 2 key.
    LaunchApplication2 = 139,
    /// The OEM Semicolon key.
    OemSemicolon = 140,
    /// The OEM Plus key.
    OemPlus = 141,
    /// The OEM Comma key.
    OemComma = 142,
    /// The OEM Minus key.
    OemMinus = 143,
    /// The OEM Period key.
    OemPeriod = 144,
    /// The OEM Question Mark key.
    OemQuestion = 145,
    /// The OEM Tilde key.
    OemTilde = 146,
    /// The ABNT_C1 (Brazilian) key.
    AbntC1 = 147,
    /// The ABNT_C2 (Brazilian) key.
    AbntC2 = 148,
    /// The OEM Open Brackets key.
    OemOpenBrackets = 149,
    /// The OEM Pipe key.
    OemPipe = 150,
    /// The OEM Close Brackets key.
    OemCloseBrackets = 151,
    /// The OEM Quotes key.
    OemQuotes = 152,
    /// The OEM 8 key.
    Oem8 = 153,
    /// The OEM Backslash key.
    OemBackslash = 154,
    /// A special key masking the real key being processed by an IME.
    ImeProcessed = 155,
    /// A special key masking the real key being processed as a system key.
    System = 156,
    /// The OEM ATTN key.
    OemAttn = 157,
    /// The OEM Finish key.
    OemFinish = 158,
    /// The DBE_HIRAGANA key.
    DbeHiragana = 159,
    /// The DBE_SBCSCHAR key.
    DbeSbcsChar = 160,
    /// The DBE_DBCSCHAR key.
    DbeDbcsChar = 161,
    /// The OEM BackTab key.
    OemBackTab = 162,
    /// The DBE_NOROMAN key.
    DbeNoRoman = 163,
    /// The CRSEL key.
    CrSel = 164,
    /// The EXSEL key.
    ExSel = 165,
    /// The ERASE EOF Key.
    EraseEof = 166,
    /// The Play key.
    Play = 167,
    /// The DBE_NOCODEINPUT key.
    DbeNoCodeInput = 168,
    /// Reserved for future use.
    NoName = 169,
    /// The DBE_ENTERDLGCONVERSIONMODE key.
    DbeEnterDialogConversionMode = 170,
    /// The OEM Clear key.
    OemClear = 171,
    /// The key is used with another key to create a single combined character.
    DeadCharProcessed = 172,
    /// OSX Platform-specific Fn+Left key
    FnLeftArrow = 10001,
    /// OSX Platform-specific Fn+Right key
    FnRightArrow = 10002,
    /// OSX Platform-specific Fn+Up key
    FnUpArrow = 10003,
    /// OSX Platform-specific Fn+Down key
    FnDownArrow = 10004,
    /// Remove control home button
    MediaHome = 100000,
    /// TV Channel up
    MediaChannelList = 100001,
    /// TV Channel up
    MediaChannelRaise = 100002,
    /// TV Channel down
    MediaChannelLower = 100003,
    /// TV Channel down
    MediaRecord = 100005,
    /// Remote control Red button
    MediaRed = 100010,
    /// Remote control Green button
    MediaGreen = 100011,
    /// Remote control Yellow button
    MediaYellow = 100012,
    /// Remote control Blue button
    MediaBlue = 100013,
    /// Remote control Menu button
    MediaMenu = 100020,
    /// Remote control dots button
    MediaMore = 100021,
    /// Remote control option button
    MediaOption = 100022,
    /// Remote control channel info button
    MediaInfo = 100023,
    /// Remote control search button
    MediaSearch = 100024,
    /// Remote control subtitle/caption button
    MediaSubtitle = 100025,
    /// Remote control Tv guide detail button
    MediaTvGuide = 100026,
    /// Remote control Previous Channel
    MediaPreviousChannel = 100027,
}

/// The error returned when a string does not name a [`Key`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseKeyError(pub String);

impl fmt::Display for ParseKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Requested value '{}' was not found.", self.0)
    }
}

impl std::error::Error for ParseKeyError {}

/// Every name of the enumeration, including aliases, with its value.
static NAMES: &[(&str, Key)] = &[
    ("None", Key::None),
    ("Cancel", Key::Cancel),
    ("Back", Key::Back),
    ("Tab", Key::Tab),
    ("LineFeed", Key::LineFeed),
    ("Clear", Key::Clear),
    ("Return", Key::Return),
    ("Enter", Key::Return),
    ("Pause", Key::Pause),
    ("CapsLock", Key::CapsLock),
    ("Capital", Key::CapsLock),
    ("HangulMode", Key::HangulMode),
    ("KanaMode", Key::HangulMode),
    ("JunjaMode", Key::JunjaMode),
    ("FinalMode", Key::FinalMode),
    ("KanjiMode", Key::KanjiMode),
    ("HanjaMode", Key::KanjiMode),
    ("Escape", Key::Escape),
    ("ImeConvert", Key::ImeConvert),
    ("ImeNonConvert", Key::ImeNonConvert),
    ("ImeAccept", Key::ImeAccept),
    ("ImeModeChange", Key::ImeModeChange),
    ("Space", Key::Space),
    ("PageUp", Key::PageUp),
    ("Prior", Key::PageUp),
    ("PageDown", Key::PageDown),
    ("Next", Key::PageDown),
    ("End", Key::End),
    ("Home", Key::Home),
    ("Left", Key::Left),
    ("Up", Key::Up),
    ("Right", Key::Right),
    ("Down", Key::Down),
    ("Select", Key::Select),
    ("Print", Key::Print),
    ("Execute", Key::Execute),
    ("Snapshot", Key::Snapshot),
    ("PrintScreen", Key::Snapshot),
    ("Insert", Key::Insert),
    ("Delete", Key::Delete),
    ("Help", Key::Help),
    ("D0", Key::D0),
    ("D1", Key::D1),
    ("D2", Key::D2),
    ("D3", Key::D3),
    ("D4", Key::D4),
    ("D5", Key::D5),
    ("D6", Key::D6),
    ("D7", Key::D7),
    ("D8", Key::D8),
    ("D9", Key::D9),
    ("A", Key::A),
    ("B", Key::B),
    ("C", Key::C),
    ("D", Key::D),
    ("E", Key::E),
    ("F", Key::F),
    ("G", Key::G),
    ("H", Key::H),
    ("I", Key::I),
    ("J", Key::J),
    ("K", Key::K),
    ("L", Key::L),
    ("M", Key::M),
    ("N", Key::N),
    ("O", Key::O),
    ("P", Key::P),
    ("Q", Key::Q),
    ("R", Key::R),
    ("S", Key::S),
    ("T", Key::T),
    ("U", Key::U),
    ("V", Key::V),
    ("W", Key::W),
    ("X", Key::X),
    ("Y", Key::Y),
    ("Z", Key::Z),
    ("LWin", Key::LWin),
    ("RWin", Key::RWin),
    ("Apps", Key::Apps),
    ("Sleep", Key::Sleep),
    ("NumPad0", Key::NumPad0),
    ("NumPad1", Key::NumPad1),
    ("NumPad2", Key::NumPad2),
    ("NumPad3", Key::NumPad3),
    ("NumPad4", Key::NumPad4),
    ("NumPad5", Key::NumPad5),
    ("NumPad6", Key::NumPad6),
    ("NumPad7", Key::NumPad7),
    ("NumPad8", Key::NumPad8),
    ("NumPad9", Key::NumPad9),
    ("Multiply", Key::Multiply),
    ("Add", Key::Add),
    ("Separator", Key::Separator),
    ("Subtract", Key::Subtract),
    ("Decimal", Key::Decimal),
    ("Divide", Key::Divide),
    ("F1", Key::F1),
    ("F2", Key::F2),
    ("F3", Key::F3),
    ("F4", Key::F4),
    ("F5", Key::F5),
    ("F6", Key::F6),
    ("F7", Key::F7),
    ("F8", Key::F8),
    ("F9", Key::F9),
    ("F10", Key::F10),
    ("F11", Key::F11),
    ("F12", Key::F12),
    ("F13", Key::F13),
    ("F14", Key::F14),
    ("F15", Key::F15),
    ("F16", Key::F16),
    ("F17", Key::F17),
    ("F18", Key::F18),
    ("F19", Key::F19),
    ("F20", Key::F20),
    ("F21", Key::F21),
    ("F22", Key::F22),
    ("F23", Key::F23),
    ("F24", Key::F24),
    ("NumLock", Key::NumLock),
    ("Scroll", Key::Scroll),
    ("LeftShift", Key::LeftShift),
    ("RightShift", Key::RightShift),
    ("LeftCtrl", Key::LeftCtrl),
    ("RightCtrl", Key::RightCtrl),
    ("LeftAlt", Key::LeftAlt),
    ("RightAlt", Key::RightAlt),
    ("BrowserBack", Key::BrowserBack),
    ("BrowserForward", Key::BrowserForward),
    ("BrowserRefresh", Key::BrowserRefresh),
    ("BrowserStop", Key::BrowserStop),
    ("BrowserSearch", Key::BrowserSearch),
    ("BrowserFavorites", Key::BrowserFavorites),
    ("BrowserHome", Key::BrowserHome),
    ("VolumeMute", Key::VolumeMute),
    ("VolumeDown", Key::VolumeDown),
    ("VolumeUp", Key::VolumeUp),
    ("MediaNextTrack", Key::MediaNextTrack),
    ("MediaPreviousTrack", Key::MediaPreviousTrack),
    ("MediaStop", Key::MediaStop),
    ("MediaPlayPause", Key::MediaPlayPause),
    ("LaunchMail", Key::LaunchMail),
    ("SelectMedia", Key::SelectMedia),
    ("LaunchApplication1", Key::LaunchApplication1),
    ("LaunchApplication2", Key::LaunchApplication2),
    ("OemSemicolon", Key::OemSemicolon),
    ("Oem1", Key::OemSemicolon),
    ("OemPlus", Key::OemPlus),
    ("OemComma", Key::OemComma),
    ("OemMinus", Key::OemMinus),
    ("OemPeriod", Key::OemPeriod),
    ("OemQuestion", Key::OemQuestion),
    ("Oem2", Key::OemQuestion),
    ("OemTilde", Key::OemTilde),
    ("Oem3", Key::OemTilde),
    ("AbntC1", Key::AbntC1),
    ("AbntC2", Key::AbntC2),
    ("OemOpenBrackets", Key::OemOpenBrackets),
    ("Oem4", Key::OemOpenBrackets),
    ("OemPipe", Key::OemPipe),
    ("Oem5", Key::OemPipe),
    ("OemCloseBrackets", Key::OemCloseBrackets),
    ("Oem6", Key::OemCloseBrackets),
    ("OemQuotes", Key::OemQuotes),
    ("Oem7", Key::OemQuotes),
    ("Oem8", Key::Oem8),
    ("OemBackslash", Key::OemBackslash),
    ("Oem102", Key::OemBackslash),
    ("ImeProcessed", Key::ImeProcessed),
    ("System", Key::System),
    ("OemAttn", Key::OemAttn),
    ("DbeAlphanumeric", Key::OemAttn),
    ("OemFinish", Key::OemFinish),
    ("DbeKatakana", Key::OemFinish),
    ("DbeHiragana", Key::DbeHiragana),
    ("OemCopy", Key::DbeHiragana),
    ("DbeSbcsChar", Key::DbeSbcsChar),
    ("OemAuto", Key::DbeSbcsChar),
    ("DbeDbcsChar", Key::DbeDbcsChar),
    ("OemEnlw", Key::DbeDbcsChar),
    ("OemBackTab", Key::OemBackTab),
    ("DbeRoman", Key::OemBackTab),
    ("DbeNoRoman", Key::DbeNoRoman),
    ("Attn", Key::DbeNoRoman),
    ("CrSel", Key::CrSel),
    ("DbeEnterWordRegisterMode", Key::CrSel),
    ("ExSel", Key::ExSel),
    ("DbeEnterImeConfigureMode", Key::ExSel),
    ("EraseEof", Key::EraseEof),
    ("DbeFlushString", Key::EraseEof),
    ("Play", Key::Play),
    ("DbeCodeInput", Key::Play),
    ("DbeNoCodeInput", Key::DbeNoCodeInput),
    ("Zoom", Key::DbeNoCodeInput),
    ("NoName", Key::NoName),
    ("DbeDetermineString", Key::NoName),
    ("DbeEnterDialogConversionMode", Key::DbeEnterDialogConversionMode),
    ("Pa1", Key::DbeEnterDialogConversionMode),
    ("OemClear", Key::OemClear),
    ("DeadCharProcessed", Key::DeadCharProcessed),
    ("FnLeftArrow", Key::FnLeftArrow),
    ("FnRightArrow", Key::FnRightArrow),
    ("FnUpArrow", Key::FnUpArrow),
    ("FnDownArrow", Key::FnDownArrow),
    ("MediaHome", Key::MediaHome),
    ("MediaChannelList", Key::MediaChannelList),
    ("MediaChannelRaise", Key::MediaChannelRaise),
    ("MediaChannelLower", Key::MediaChannelLower),
    ("MediaRecord", Key::MediaRecord),
    ("MediaRed", Key::MediaRed),
    ("MediaGreen", Key::MediaGreen),
    ("MediaYellow", Key::MediaYellow),
    ("MediaBlue", Key::MediaBlue),
    ("MediaMenu", Key::MediaMenu),
    ("MediaMore", Key::MediaMore),
    ("MediaOption", Key::MediaOption),
    ("MediaInfo", Key::MediaInfo),
    ("MediaSearch", Key::MediaSearch),
    ("MediaSubtitle", Key::MediaSubtitle),
    ("MediaTvGuide", Key::MediaTvGuide),
    ("MediaPreviousChannel", Key::MediaPreviousChannel),
];

#[allow(non_upper_case_globals)]
impl Key {
    /// The Enter key.
    pub const Enter: Key = Key::Return;

    /// The Caps Lock key.
    pub const Capital: Key = Key::CapsLock;

    /// The IME Kana mode key.
    pub const KanaMode: Key = Key::HangulMode;

    /// The IME Hanja mode key.
    pub const HanjaMode: Key = Key::KanjiMode;

    /// The Page Up key.
    pub const Prior: Key = Key::PageUp;

    /// The Page Down key.
    pub const Next: Key = Key::PageDown;

    /// The Print Screen key.
    pub const PrintScreen: Key = Key::Snapshot;

    /// The OEM 1 key.
    pub const Oem1: Key = Key::OemSemicolon;

    /// The OEM 2 key.
    pub const Oem2: Key = Key::OemQuestion;

    /// The OEM 3 key.
    pub const Oem3: Key = Key::OemTilde;

    /// The OEM 4 key.
    pub const Oem4: Key = Key::OemOpenBrackets;

    /// The OEM 5 key.
    pub const Oem5: Key = Key::OemPipe;

    /// The OEM 6 key.
    pub const Oem6: Key = Key::OemCloseBrackets;

    /// The OEM 7 key.
    pub const Oem7: Key = Key::OemQuotes;

    /// The OEM 3 key.
    pub const Oem102: Key = Key::OemBackslash;

    /// The DBE_ALPHANUMERIC key.
    pub const DbeAlphanumeric: Key = Key::OemAttn;

    /// The DBE_KATAKANA key.
    pub const DbeKatakana: Key = Key::OemFinish;

    /// The OEM Copy key.
    pub const OemCopy: Key = Key::DbeHiragana;

    /// The OEM Auto key.
    pub const OemAuto: Key = Key::DbeSbcsChar;

    /// The OEM ENLW key.
    pub const OemEnlw: Key = Key::DbeDbcsChar;

    /// The DBE_ROMAN key.
    pub const DbeRoman: Key = Key::OemBackTab;

    /// The ATTN key.
    pub const Attn: Key = Key::DbeNoRoman;

    /// The DBE_ENTERWORDREGISTERMODE key.
    pub const DbeEnterWordRegisterMode: Key = Key::CrSel;

    /// The DBE_ENTERIMECONFIGMODE key.
    pub const DbeEnterImeConfigureMode: Key = Key::ExSel;

    /// The DBE_FLUSHSTRING key.
    pub const DbeFlushString: Key = Key::EraseEof;

    /// The DBE_CODEINPUT key.
    pub const DbeCodeInput: Key = Key::Play;

    /// The Zoom key.
    pub const Zoom: Key = Key::DbeNoCodeInput;

    /// The DBE_DETERMINESTRING key.
    pub const DbeDetermineString: Key = Key::NoName;

    /// The PA1 key.
    pub const Pa1: Key = Key::DbeEnterDialogConversionMode;

    /// The numeric value of the member.
    #[inline]
    pub const fn value(self) -> i32 {
        self as i32
    }

    /// The member with the given numeric value, if one is defined.
    pub fn from_value(value: i32) -> Option<Key> {
        NAMES.iter().map(|(_, k)| *k).find(|k| *k as i32 == value)
    }

    /// The name of the member. For values with several names this is the
    /// first one declared.
    pub fn name(self) -> &'static str {
        match self {
            Key::None => "None",
            Key::Cancel => "Cancel",
            Key::Back => "Back",
            Key::Tab => "Tab",
            Key::LineFeed => "LineFeed",
            Key::Clear => "Clear",
            Key::Return => "Return",
            Key::Pause => "Pause",
            Key::CapsLock => "CapsLock",
            Key::HangulMode => "HangulMode",
            Key::JunjaMode => "JunjaMode",
            Key::FinalMode => "FinalMode",
            Key::KanjiMode => "KanjiMode",
            Key::Escape => "Escape",
            Key::ImeConvert => "ImeConvert",
            Key::ImeNonConvert => "ImeNonConvert",
            Key::ImeAccept => "ImeAccept",
            Key::ImeModeChange => "ImeModeChange",
            Key::Space => "Space",
            Key::PageUp => "PageUp",
            Key::PageDown => "PageDown",
            Key::End => "End",
            Key::Home => "Home",
            Key::Left => "Left",
            Key::Up => "Up",
            Key::Right => "Right",
            Key::Down => "Down",
            Key::Select => "Select",
            Key::Print => "Print",
            Key::Execute => "Execute",
            Key::Snapshot => "Snapshot",
            Key::Insert => "Insert",
            Key::Delete => "Delete",
            Key::Help => "Help",
            Key::D0 => "D0",
            Key::D1 => "D1",
            Key::D2 => "D2",
            Key::D3 => "D3",
            Key::D4 => "D4",
            Key::D5 => "D5",
            Key::D6 => "D6",
            Key::D7 => "D7",
            Key::D8 => "D8",
            Key::D9 => "D9",
            Key::A => "A",
            Key::B => "B",
            Key::C => "C",
            Key::D => "D",
            Key::E => "E",
            Key::F => "F",
            Key::G => "G",
            Key::H => "H",
            Key::I => "I",
            Key::J => "J",
            Key::K => "K",
            Key::L => "L",
            Key::M => "M",
            Key::N => "N",
            Key::O => "O",
            Key::P => "P",
            Key::Q => "Q",
            Key::R => "R",
            Key::S => "S",
            Key::T => "T",
            Key::U => "U",
            Key::V => "V",
            Key::W => "W",
            Key::X => "X",
            Key::Y => "Y",
            Key::Z => "Z",
            Key::LWin => "LWin",
            Key::RWin => "RWin",
            Key::Apps => "Apps",
            Key::Sleep => "Sleep",
            Key::NumPad0 => "NumPad0",
            Key::NumPad1 => "NumPad1",
            Key::NumPad2 => "NumPad2",
            Key::NumPad3 => "NumPad3",
            Key::NumPad4 => "NumPad4",
            Key::NumPad5 => "NumPad5",
            Key::NumPad6 => "NumPad6",
            Key::NumPad7 => "NumPad7",
            Key::NumPad8 => "NumPad8",
            Key::NumPad9 => "NumPad9",
            Key::Multiply => "Multiply",
            Key::Add => "Add",
            Key::Separator => "Separator",
            Key::Subtract => "Subtract",
            Key::Decimal => "Decimal",
            Key::Divide => "Divide",
            Key::F1 => "F1",
            Key::F2 => "F2",
            Key::F3 => "F3",
            Key::F4 => "F4",
            Key::F5 => "F5",
            Key::F6 => "F6",
            Key::F7 => "F7",
            Key::F8 => "F8",
            Key::F9 => "F9",
            Key::F10 => "F10",
            Key::F11 => "F11",
            Key::F12 => "F12",
            Key::F13 => "F13",
            Key::F14 => "F14",
            Key::F15 => "F15",
            Key::F16 => "F16",
            Key::F17 => "F17",
            Key::F18 => "F18",
            Key::F19 => "F19",
            Key::F20 => "F20",
            Key::F21 => "F21",
            Key::F22 => "F22",
            Key::F23 => "F23",
            Key::F24 => "F24",
            Key::NumLock => "NumLock",
            Key::Scroll => "Scroll",
            Key::LeftShift => "LeftShift",
            Key::RightShift => "RightShift",
            Key::LeftCtrl => "LeftCtrl",
            Key::RightCtrl => "RightCtrl",
            Key::LeftAlt => "LeftAlt",
            Key::RightAlt => "RightAlt",
            Key::BrowserBack => "BrowserBack",
            Key::BrowserForward => "BrowserForward",
            Key::BrowserRefresh => "BrowserRefresh",
            Key::BrowserStop => "BrowserStop",
            Key::BrowserSearch => "BrowserSearch",
            Key::BrowserFavorites => "BrowserFavorites",
            Key::BrowserHome => "BrowserHome",
            Key::VolumeMute => "VolumeMute",
            Key::VolumeDown => "VolumeDown",
            Key::VolumeUp => "VolumeUp",
            Key::MediaNextTrack => "MediaNextTrack",
            Key::MediaPreviousTrack => "MediaPreviousTrack",
            Key::MediaStop => "MediaStop",
            Key::MediaPlayPause => "MediaPlayPause",
            Key::LaunchMail => "LaunchMail",
            Key::SelectMedia => "SelectMedia",
            Key::LaunchApplication1 => "LaunchApplication1",
            Key::LaunchApplication2 => "LaunchApplication2",
            Key::OemSemicolon => "OemSemicolon",
            Key::OemPlus => "OemPlus",
            Key::OemComma => "OemComma",
            Key::OemMinus => "OemMinus",
            Key::OemPeriod => "OemPeriod",
            Key::OemQuestion => "OemQuestion",
            Key::OemTilde => "OemTilde",
            Key::AbntC1 => "AbntC1",
            Key::AbntC2 => "AbntC2",
            Key::OemOpenBrackets => "OemOpenBrackets",
            Key::OemPipe => "OemPipe",
            Key::OemCloseBrackets => "OemCloseBrackets",
            Key::OemQuotes => "OemQuotes",
            Key::Oem8 => "Oem8",
            Key::OemBackslash => "OemBackslash",
            Key::ImeProcessed => "ImeProcessed",
            Key::System => "System",
            Key::OemAttn => "OemAttn",
            Key::OemFinish => "OemFinish",
            Key::DbeHiragana => "DbeHiragana",
            Key::DbeSbcsChar => "DbeSbcsChar",
            Key::DbeDbcsChar => "DbeDbcsChar",
            Key::OemBackTab => "OemBackTab",
            Key::DbeNoRoman => "DbeNoRoman",
            Key::CrSel => "CrSel",
            Key::ExSel => "ExSel",
            Key::EraseEof => "EraseEof",
            Key::Play => "Play",
            Key::DbeNoCodeInput => "DbeNoCodeInput",
            Key::NoName => "NoName",
            Key::DbeEnterDialogConversionMode => "DbeEnterDialogConversionMode",
            Key::OemClear => "OemClear",
            Key::DeadCharProcessed => "DeadCharProcessed",
            Key::FnLeftArrow => "FnLeftArrow",
            Key::FnRightArrow => "FnRightArrow",
            Key::FnUpArrow => "FnUpArrow",
            Key::FnDownArrow => "FnDownArrow",
            Key::MediaHome => "MediaHome",
            Key::MediaChannelList => "MediaChannelList",
            Key::MediaChannelRaise => "MediaChannelRaise",
            Key::MediaChannelLower => "MediaChannelLower",
            Key::MediaRecord => "MediaRecord",
            Key::MediaRed => "MediaRed",
            Key::MediaGreen => "MediaGreen",
            Key::MediaYellow => "MediaYellow",
            Key::MediaBlue => "MediaBlue",
            Key::MediaMenu => "MediaMenu",
            Key::MediaMore => "MediaMore",
            Key::MediaOption => "MediaOption",
            Key::MediaInfo => "MediaInfo",
            Key::MediaSearch => "MediaSearch",
            Key::MediaSubtitle => "MediaSubtitle",
            Key::MediaTvGuide => "MediaTvGuide",
            Key::MediaPreviousChannel => "MediaPreviousChannel",
        }
    }

    /// Parses a member name (ignoring case, aliases included) or the
    /// numeric value of a defined member.
    pub fn parse(s: &str) -> Result<Key, ParseKeyError> {
        let s = s.trim();
        if let Some((_, value)) = NAMES.iter().find(|(name, _)| name.eq_ignore_ascii_case(s)) {
            return Ok(*value);
        }
        if let Ok(number) = s.parse::<i32>() {
            if let Some(value) = Self::from_value(number) {
                return Ok(value);
            }
        }
        Err(ParseKeyError(s.to_string()))
    }
}

impl FromStr for Key {
    type Err = ParseKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_share_values() {
        assert_eq!(Key::Enter, Key::Return);
        assert_eq!(Key::Enter.value(), 6);
        assert_eq!(Key::parse("enter"), Ok(Key::Return));
        assert_eq!(Key::parse("ENTER"), Ok(Key::Enter));
    }

    #[test]
    fn parse_and_display_round_trip() {
        for (name, key) in NAMES {
            assert_eq!(Key::parse(name), Ok(*key));
            assert_eq!(Key::parse(key.name()), Ok(*key));
        }
        assert_eq!(Key::A.to_string(), "A");
        assert_eq!(Key::None.to_string(), "None");
        assert_eq!(Key::parse("44"), Ok(Key::A));
        assert!(Key::parse("NotAKey").is_err());
    }
}
