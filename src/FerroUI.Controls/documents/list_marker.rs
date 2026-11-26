//! Paragraph list markers: the `MarkerType`, `MarkerStyle` and `MarkerAlignment` of the rich
//! text object model's paragraph format (`ListType`, `ListStyle`, `ListAlignment`), and the text
//! of a list paragraph's marker. A list is a format of each of its paragraphs, as in the text
//! object model: consecutive list paragraphs number together (docs/porting/winui-rich-text.md
//! 11.10).

/// The kind of a paragraph's list marker (`ITextParagraphFormat.ListType`). `None` is a
/// paragraph that is not in a list. The text object model's `Undefined` (a mixed range) is a
/// `TextFormatValue::Mixed` query result here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum MarkerType {
    /// Not a list paragraph.
    #[default]
    None = 0,
    /// A bullet; nested levels cycle through a filled circle, a hollow circle and a square.
    Bullet = 1,
    /// 1, 2, 3.
    Arabic = 2,
    /// a, b, c, then aa, ab.
    LowercaseEnglishLetter = 3,
    /// A, B, C, then AA, AB.
    UppercaseEnglishLetter = 4,
    /// i, ii, iii.
    LowercaseRoman = 5,
    /// I, II, III.
    UppercaseRoman = 6,
    /// The character whose code point is the number, starting at `ListStart`.
    UnicodeSequence = 7,
    /// Circled digits from one to fifty, then Arabic numbers.
    CircledNumber = 8,
    /// Black circled digits from one to twenty, then circled digits.
    BlackCircleWingding = 9,
    /// White circled sans-serif digits from one to ten, then circled digits.
    WhiteCircleWingding = 10,
    /// Full-width Arabic digits.
    ArabicWide = 11,
    /// Chinese numerals.
    SimplifiedChinese = 12,
    /// Chinese financial numerals.
    TraditionalChinese = 13,
    /// Chinese numerals, as Japanese text writes them.
    JapanSimplifiedChinese = 14,
    /// Full-width Arabic digits, as Japanese and Korean text write them.
    JapanKorean = 15,
    /// Arabic letters in dictionary order.
    ArabicDictionary = 16,
    /// Arabic letters in abjad order.
    ArabicAbjad = 17,
    /// Hebrew numerals.
    Hebrew = 18,
    /// Thai consonants.
    ThaiAlphabetic = 19,
    /// Thai digits.
    ThaiNumeric = 20,
    /// Devanagari vowels.
    DevanagariVowel = 21,
    /// Devanagari consonants.
    DevanagariConsonant = 22,
    /// Devanagari digits.
    DevanagariNumeric = 23,
}

ferroui_base::ferro_markup_enum!(MarkerType {
    None, Bullet, Arabic, LowercaseEnglishLetter, UppercaseEnglishLetter, LowercaseRoman,
    UppercaseRoman, UnicodeSequence, CircledNumber, BlackCircleWingding, WhiteCircleWingding,
    ArabicWide, SimplifiedChinese, TraditionalChinese, JapanSimplifiedChinese, JapanKorean,
    ArabicDictionary, ArabicAbjad, Hebrew, ThaiAlphabetic, ThaiNumeric, DevanagariVowel,
    DevanagariConsonant, DevanagariNumeric
}, { namespace: "FerroUI.Controls.Documents" });

/// The punctuation around a list number (`ITextParagraphFormat.ListStyle`); the values are the
/// text object model's list style flags. `Parenthesis` is the default of the rich edit control.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum MarkerStyle {
    /// 1)
    #[default]
    Parenthesis = 0,
    /// (1)
    Parentheses = 0x10000,
    /// 1.
    Period = 0x20000,
    /// 1
    Plain = 0x30000,
    /// The paragraph is indented as a list paragraph and numbered, but shows no number.
    NoNumber = 0x40000,
    /// 1-
    Minus = 0x80000,
}

ferroui_base::ferro_markup_enum!(MarkerStyle {
    Parenthesis, Parentheses, Period, Plain, NoNumber, Minus
}, { namespace: "FerroUI.Controls.Documents" });

/// Where a list marker sits in the space before the paragraph's text
/// (`ITextParagraphFormat.ListAlignment`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum MarkerAlignment {
    /// At the start of the marker space.
    #[default]
    Left = 1,
    /// Centered in the marker space.
    Center = 2,
    /// At the end of the marker space, before the gap to the text.
    Right = 3,
}

ferroui_base::ferro_markup_enum!(MarkerAlignment {
    Left, Center, Right
}, { namespace: "FerroUI.Controls.Documents" });

/// The deepest list level (`ListLevelIndex`); 0 is no level, a list paragraph at level 0 is
/// laid out and numbered at level 1.
pub(crate) const MAX_LIST_LEVEL: i32 = 9;

impl MarkerType {
    /// Whether the marker is a number (every type but `None` and `Bullet`).
    pub fn is_numbered(self) -> bool {
        !matches!(self, Self::None | Self::Bullet)
    }

    /// Whether the marker carries its own delimiters, so `MarkerStyle` adds no punctuation.
    fn is_self_delimited(self) -> bool {
        matches!(
            self,
            Self::Bullet
                | Self::UnicodeSequence
                | Self::CircledNumber
                | Self::BlackCircleWingding
                | Self::WhiteCircleWingding
        )
    }
}

/// The text of the marker of a list paragraph at `level` (1-based) with `number`: empty for
/// `MarkerType::None` and for `MarkerStyle::NoNumber`.
pub(crate) fn list_marker_text(kind: MarkerType, style: MarkerStyle, level: i32, number: i32) -> String {
    if kind == MarkerType::None || (kind.is_numbered() && style == MarkerStyle::NoNumber) {
        return String::new();
    }
    let value = match kind {
        MarkerType::None => return String::new(),
        MarkerType::Bullet => {
            return ["\u{2022}", "\u{25E6}", "\u{25AA}"][((level.max(1) - 1) % 3) as usize].to_owned();
        }
        _ => format_number(kind, number),
    };
    if kind.is_self_delimited() {
        return value;
    }
    match style {
        MarkerStyle::Parenthesis => format!("{value})"),
        MarkerStyle::Parentheses => format!("({value})"),
        MarkerStyle::Period => format!("{value}."),
        MarkerStyle::Plain | MarkerStyle::NoNumber => value,
        MarkerStyle::Minus => format!("{value}-"),
    }
}

fn format_number(kind: MarkerType, number: i32) -> String {
    let positive = u32::try_from(number).ok().filter(|value| *value > 0);
    let arabic = || number.to_string();
    match kind {
        MarkerType::None | MarkerType::Bullet | MarkerType::Arabic => arabic(),
        MarkerType::LowercaseEnglishLetter => positive.map_or_else(arabic, |n| alphabetic(n, &LATIN_LOWER)),
        MarkerType::UppercaseEnglishLetter => positive.map_or_else(arabic, |n| alphabetic(n, &LATIN_UPPER)),
        MarkerType::LowercaseRoman => roman(number).map_or_else(arabic, |text| text.to_lowercase()),
        MarkerType::UppercaseRoman => roman(number).unwrap_or_else(arabic),
        MarkerType::UnicodeSequence => positive
            .and_then(char::from_u32)
            .filter(|ch| !ch.is_control())
            .map_or_else(arabic, String::from),
        MarkerType::CircledNumber => circled(number).unwrap_or_else(arabic),
        MarkerType::BlackCircleWingding => match number {
            1..=10 => offset_char(0x2776, number - 1),
            11..=20 => offset_char(0x24EB, number - 11),
            _ => circled(number).unwrap_or_else(arabic),
        },
        MarkerType::WhiteCircleWingding => match number {
            1..=10 => offset_char(0x2780, number - 1),
            _ => circled(number).unwrap_or_else(arabic),
        },
        MarkerType::ArabicWide | MarkerType::JapanKorean => digits(number, 0xFF10),
        MarkerType::ThaiNumeric => digits(number, 0x0E50),
        MarkerType::DevanagariNumeric => digits(number, 0x0966),
        MarkerType::SimplifiedChinese | MarkerType::JapanSimplifiedChinese => {
            chinese(number, &CHINESE_DIGITS, &CHINESE_UNITS).unwrap_or_else(arabic)
        }
        MarkerType::TraditionalChinese => {
            chinese(number, &CHINESE_FINANCIAL_DIGITS, &CHINESE_FINANCIAL_UNITS).unwrap_or_else(arabic)
        }
        MarkerType::ArabicDictionary => positive.map_or_else(arabic, |n| alphabetic(n, &ARABIC_DICTIONARY)),
        MarkerType::ArabicAbjad => positive.map_or_else(arabic, |n| alphabetic(n, &ARABIC_ABJAD)),
        MarkerType::Hebrew => hebrew(number).unwrap_or_else(arabic),
        MarkerType::ThaiAlphabetic => positive.map_or_else(arabic, |n| alphabetic(n, &THAI_CONSONANTS)),
        MarkerType::DevanagariVowel => positive.map_or_else(arabic, |n| alphabetic(n, &DEVANAGARI_VOWELS)),
        MarkerType::DevanagariConsonant => {
            positive.map_or_else(arabic, |n| alphabetic(n, &DEVANAGARI_CONSONANTS))
        }
    }
}

fn offset_char(base: u32, offset: i32) -> String {
    char::from_u32(base + offset as u32).map(String::from).unwrap_or_default()
}

fn circled(number: i32) -> Option<String> {
    match number {
        1..=20 => Some(offset_char(0x2460, number - 1)),
        21..=35 => Some(offset_char(0x3251, number - 21)),
        36..=50 => Some(offset_char(0x32B1, number - 36)),
        _ => None,
    }
}

fn digits(number: i32, zero: u32) -> String {
    number
        .to_string()
        .chars()
        .map(|ch| match ch.to_digit(10) {
            Some(digit) => char::from_u32(zero + digit).unwrap_or(ch),
            None => ch,
        })
        .collect()
}

/// a..z, then aa..az, ba.. (bijective numbering over the alphabet).
fn alphabetic(mut number: u32, alphabet: &[&str]) -> String {
    let base = alphabet.len() as u32;
    let mut parts = Vec::new();
    while number > 0 {
        number -= 1;
        parts.push(alphabet[(number % base) as usize]);
        number /= base;
    }
    parts.iter().rev().copied().collect()
}

fn roman(number: i32) -> Option<String> {
    if !(1..=3999).contains(&number) {
        return None;
    }
    const PARTS: [(i32, &str); 13] = [
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"),
        (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ];
    let mut rest = number;
    let mut text = String::new();
    for (value, part) in PARTS {
        while rest >= value {
            text.push_str(part);
            rest -= value;
        }
    }
    Some(text)
}

/// Chinese numerals up to 9999: 一, 十, 十一, 二十, 一百零一.
fn chinese(number: i32, digits: &[&str; 10], units: &[&str; 3]) -> Option<String> {
    if !(1..=9999).contains(&number) {
        return None;
    }
    let places = [number / 1000, number / 100 % 10, number / 10 % 10, number % 10];
    let mut text = String::new();
    let mut pending_zero = false;
    for (index, digit) in places.iter().copied().enumerate() {
        if digit == 0 {
            pending_zero = !text.is_empty();
            continue;
        }
        if pending_zero {
            text.push_str(digits[0]);
            pending_zero = false;
        }
        let unit = 3 - index;
        // 十 alone is ten, 十一 eleven: the leading one of the tens is not written.
        if !(unit == 1 && digit == 1 && text.is_empty()) {
            text.push_str(digits[digit as usize]);
        }
        if unit > 0 {
            text.push_str(units[unit - 1]);
        }
    }
    Some(text)
}

/// Hebrew numerals up to 999, with 15 and 16 written as 9+6 and 9+7.
fn hebrew(number: i32) -> Option<String> {
    if !(1..=999).contains(&number) {
        return None;
    }
    const HUNDREDS: [&str; 10] = ["", "ק", "ר", "ש", "ת", "תק", "תר", "תש", "תת", "תתק"];
    const TENS: [&str; 10] = ["", "י", "כ", "ל", "מ", "נ", "ס", "ע", "פ", "צ"];
    const ONES: [&str; 10] = ["", "א", "ב", "ג", "ד", "ה", "ו", "ז", "ח", "ט"];
    let mut text = String::from(HUNDREDS[(number / 100) as usize]);
    match number % 100 {
        15 => text.push_str("טו"),
        16 => text.push_str("טז"),
        rest => {
            text.push_str(TENS[(rest / 10) as usize]);
            text.push_str(ONES[(rest % 10) as usize]);
        }
    }
    Some(text)
}

const LATIN_LOWER: [&str; 26] = [
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s",
    "t", "u", "v", "w", "x", "y", "z",
];
const LATIN_UPPER: [&str; 26] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z",
];
const CHINESE_DIGITS: [&str; 10] = ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
const CHINESE_UNITS: [&str; 3] = ["十", "百", "千"];
const CHINESE_FINANCIAL_DIGITS: [&str; 10] = ["零", "壹", "貳", "參", "肆", "伍", "陸", "柒", "捌", "玖"];
const CHINESE_FINANCIAL_UNITS: [&str; 3] = ["拾", "佰", "仟"];
const ARABIC_DICTIONARY: [&str; 28] = [
    "أ", "ب", "ت", "ث", "ج", "ح", "خ", "د", "ذ", "ر", "ز", "س", "ش", "ص", "ض", "ط", "ظ", "ع", "غ",
    "ف", "ق", "ك", "ل", "م", "ن", "ه", "و", "ي",
];
const ARABIC_ABJAD: [&str; 28] = [
    "أ", "ب", "ج", "د", "ه", "و", "ز", "ح", "ط", "ي", "ك", "ل", "م", "ن", "س", "ع", "ف", "ص", "ق",
    "ر", "ش", "ت", "ث", "خ", "ذ", "ض", "ظ", "غ",
];
const THAI_CONSONANTS: [&str; 41] = [
    "ก", "ข", "ค", "ง", "จ", "ฉ", "ช", "ซ", "ฌ", "ญ", "ฎ", "ฏ", "ฐ", "ฑ", "ฒ", "ณ", "ด", "ต", "ถ",
    "ท", "ธ", "น", "บ", "ป", "ผ", "ฝ", "พ", "ฟ", "ภ", "ม", "ย", "ร", "ล", "ว", "ศ", "ษ", "ส", "ห",
    "ฬ", "อ", "ฮ",
];
const DEVANAGARI_VOWELS: [&str; 13] = [
    "अ", "आ", "इ", "ई", "उ", "ऊ", "ऋ", "ए", "ऐ", "ओ", "औ", "अं", "अः",
];
const DEVANAGARI_CONSONANTS: [&str; 33] = [
    "क", "ख", "ग", "घ", "ङ", "च", "छ", "ज", "झ", "ञ", "ट", "ठ", "ड", "ढ", "ण", "त", "थ", "द", "ध",
    "न", "प", "फ", "ब", "भ", "म", "य", "र", "ल", "व", "श", "ष", "स", "ह",
];

/// Numbers consecutive list paragraphs, as the rich edit control does: a paragraph continues
/// the sequence of the closest preceding list paragraph at its level when nothing shallower or
/// outside a list comes between them and both have the same type, style and start; deeper
/// paragraphs in between belong to nested sequences and do not interrupt it.
#[derive(Default)]
pub(crate) struct ListNumbering {
    levels: Vec<Option<(MarkerType, MarkerStyle, i32, i32)>>,
}

impl ListNumbering {
    /// The marker text of the next paragraph, or `None` (and the end of every sequence) for a
    /// paragraph that is not in a list.
    pub(crate) fn next(
        &mut self,
        kind: MarkerType,
        style: MarkerStyle,
        start: i32,
        level_index: i32,
    ) -> Option<(i32, String)> {
        if kind == MarkerType::None {
            self.levels.clear();
            return None;
        }
        let level = level_index.clamp(1, MAX_LIST_LEVEL);
        let slot = (level - 1) as usize;
        self.levels.truncate(slot + 1);
        self.levels.resize(slot + 1, None);
        let number = match &mut self.levels[slot] {
            Some((previous_kind, previous_style, previous_start, number))
                if *previous_kind == kind && *previous_style == style && *previous_start == start =>
            {
                *number = number.saturating_add(1);
                *number
            }
            entry => {
                *entry = Some((kind, style, start, start));
                start
            }
        };
        Some((level, list_marker_text(kind, style, level, number)))
    }
}
