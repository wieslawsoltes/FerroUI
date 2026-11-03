//! Emoji/EmojiCatalog.cs. Bundled CLDR data remains unchanged.
use super::EmojiSkinTone;
use crate::{
    resources::EmbeddedResources,
    serialization::{JsonBridgeError, StrictJsonValue},
    text::{TextCulture, TextUtils, Utf16Text, ordinal_ignore_case_units},
};
use parking_lot::Mutex;
use std::{
    collections::{HashMap, hash_map::Entry},
    sync::{Arc, LazyLock},
};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EmojiCategory(pub i32);
#[allow(non_upper_case_globals)]
impl EmojiCategory {
    pub const Recent: Self = Self(0);
    pub const SmileysEmotion: Self = Self(1);
    pub const PeopleBody: Self = Self(2);
    pub const AnimalsNature: Self = Self(3);
    pub const FoodDrink: Self = Self(4);
    pub const TravelPlaces: Self = Self(5);
    pub const Activities: Self = Self(6);
    pub const Objects: Self = Self(7);
    pub const Symbols: Self = Self(8);
    pub const Flags: Self = Self(9);
}
#[derive(Debug)]
pub enum EmojiError {
    NullReference,
    ArgumentNull(&'static str),
    InvalidScalar,
    Index,
    Json(JsonBridgeError),
    MissingProperty,
    Resource(std::io::Error),
    ResourceEncoding,
}
impl std::fmt::Display for EmojiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NullReference => f.write_str("Emoji text is missing"),
            Self::ArgumentNull(name) => write!(f, "Emoji argument {name} is missing"),
            Self::InvalidScalar => f.write_str("Skin tone does not produce a Unicode scalar"),
            Self::Index => {
                f.write_str("Recent emoji limit requires removing from an empty history")
            }
            Self::Json(error) => write!(f, "Invalid emoji JSON: {error}"),
            Self::MissingProperty => f.write_str("Emoji resource is missing a required property"),
            Self::Resource(error) => write!(f, "Cannot read emoji resource: {error}"),
            Self::ResourceEncoding => f.write_str("Emoji resource contains invalid Unicode"),
        }
    }
}
impl std::error::Error for EmojiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Resource(error) => Some(error),
            _ => None,
        }
    }
}
impl From<JsonBridgeError> for EmojiError {
    fn from(e: JsonBridgeError) -> Self {
        Self::Json(e)
    }
}
pub type EmojiResult<T> = Result<T, EmojiError>;
pub struct EmojiInfo {
    pub value: Utf16Text,
    pub name: Utf16Text,
    /// Shared native keyword vector. Release its guard before calling back into the catalog.
    pub keywords: Arc<Mutex<Vec<Utf16Text>>>,
    pub category: EmojiCategory,
    variants: Arc<Mutex<Vec<Utf16Text>>>,
}
impl PartialEq for EmojiInfo {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for EmojiInfo {}
impl EmojiInfo {
    pub fn new(value: Utf16Text) -> Self {
        Self {
            value,
            name: Utf16Text::default(),
            keywords: Arc::default(),
            category: EmojiCategory::Recent,
            variants: Arc::default(),
        }
    }
    /// Live shared vector; edits affect subsequent tone selection.
    pub fn variants(&self) -> Arc<Mutex<Vec<Utf16Text>>> {
        self.variants.clone()
    }
    pub fn has_skin_tones(&self) -> bool {
        !self.variants.lock().is_empty()
    }
    pub fn with_skin_tone(&self, tone: EmojiSkinTone) -> EmojiResult<Utf16Text> {
        let variants = self.variants.lock().clone();
        if tone == EmojiSkinTone::Default || variants.is_empty() {
            return Ok(self.value.clone());
        }
        let cp = 0x1f3fbi32.wrapping_add(tone.0).wrapping_sub(1);
        let c = char::from_u32(cp as u32).ok_or(EmojiError::InvalidScalar)?;
        let modifier: Utf16Text = c.to_string().as_str().into();
        for v in variants.iter() {
            if contains(v, &modifier)
                && count_modifiers(v)
                    == TextUtils::get_code_points(v)
                        .iter()
                        .filter(|&&p| p == cp)
                        .count()
            {
                return Ok(v.clone());
            }
        }
        for v in variants.iter() {
            if contains(v, &modifier) {
                return Ok(v.clone());
            }
        }
        Ok(self.value.clone())
    }
    pub fn to_text(&self) -> Utf16Text {
        Utf16Text::from_units([self.value.units(), &[32], self.name.units()].concat())
    }
}
#[derive(Clone)]
pub struct Emoticon {
    pub text: Utf16Text,
    pub meanings: Arc<Mutex<Vec<Utf16Text>>>,
}
impl PartialEq for Emoticon {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text && Arc::ptr_eq(&self.meanings, &other.meanings)
    }
}
impl Eq for Emoticon {}
pub struct EmojiCatalog {
    locale: Utf16Text,
    by_category: HashMap<EmojiCategory, Arc<Mutex<Vec<Arc<EmojiInfo>>>>>,
    by_value: HashMap<Utf16Text, Arc<EmojiInfo>>,
}
fn contains(text: &Utf16Text, part: &Utf16Text) -> bool {
    text.units().windows(part.len()).any(|v| v == part.units())
}
fn trim(text: &[u16]) -> &[u16] {
    let mut a = 0;
    let mut b = text.len();
    while a < b && TextUtils::is_white_space(text[a]) {
        a += 1;
    }
    while b > a && TextUtils::is_white_space(text[b - 1]) {
        b -= 1;
    }
    &text[a..b]
}
fn count_modifiers(s: &Utf16Text) -> usize {
    TextUtils::get_code_points(s)
        .iter()
        .filter(|&&p| (0x1f3fb..=0x1f3ff).contains(&p))
        .count()
}
impl EmojiCatalog {
    pub const BUNDLED_LOCALES: [&'static str; 6] = ["en", "de", "es", "fr", "it", "pt"];
    pub fn locale(&self) -> Utf16Text {
        self.locale.clone()
    }
    pub fn load(language: &Utf16Text) -> EmojiResult<Arc<Self>> {
        Self::load_nullable(Some(language))
    }
    pub fn load_nullable(language: Option<&Utf16Text>) -> EmojiResult<Arc<Self>> {
        let language = language.ok_or(EmojiError::NullReference)?;
        let lang = TextCulture::to_lower_invariant(&Utf16Text::from_units(
            language
                .units()
                .split(|&c| c == 45 || c == 95)
                .next()
                .unwrap()
                .to_vec(),
        ));
        let lang = Self::BUNDLED_LOCALES
            .iter()
            .find(|l| Utf16Text::from(**l) == lang)
            .copied()
            .unwrap_or("en");
        static CACHE: LazyLock<Mutex<HashMap<&'static str, Arc<EmojiCatalog>>>> =
            LazyLock::new(|| Mutex::new(HashMap::new()));
        let mut cache = CACHE.lock();
        if let Some(c) = cache.get(lang) {
            return Ok(c.clone());
        }
        let text = match EmbeddedResources::read_text(&format!("media/emoji/{lang}.txt"))
            .map_err(EmojiError::Resource)?
        {
            Some(text) => text,
            None => EmbeddedResources::read_text("media/emoji/root.txt")
                .map_err(EmojiError::Resource)?
                .unwrap_or_default(),
        };
        let c = Self::parse(&text, lang.into())?;
        cache.insert(lang, c.clone());
        Ok(c)
    }
    pub fn parse(text: &Utf16Text, locale: Utf16Text) -> EmojiResult<Arc<Self>> {
        let mut catalog = Self {
            locale,
            by_category: HashMap::new(),
            by_value: HashMap::new(),
        };
        // Alias collisions are resolved in first-base encounter order, even after duplicates.
        let mut base_order = Vec::new();
        let mut category = EmojiCategory::SmileysEmotion;
        let mut last: Option<Arc<EmojiInfo>> = None;
        let mut variants = Vec::new();
        fn flush(last: &Option<Arc<EmojiInfo>>, variants: &mut Vec<Utf16Text>) {
            if let Some(info) = last
                && !variants.is_empty()
            {
                *info.variants.lock() = std::mem::take(variants);
            }
            variants.clear();
        }
        let ids = [
            "smileys_emotion",
            "people_body",
            "animals_nature",
            "food_drink",
            "travel_places",
            "activities",
            "objects",
            "symbols",
            "flags",
        ];
        for raw in text.units().split(|&c| c == 10) {
            let mut end = raw.len();
            while end > 0 && raw[end - 1] == 13 {
                end -= 1;
            }
            let line = &raw[..end];
            if line.is_empty() || line[0] == 35 {
                continue;
            }
            if line[0] == 91 {
                flush(&last, &mut variants);
                last = None;
                let mut a = 0;
                let mut b = line.len();
                while a < b && matches!(line[a], 91 | 93 | 32) {
                    a += 1;
                }
                while b > a && matches!(line[b - 1], 91 | 93 | 32) {
                    b -= 1;
                }
                if let Some(i) = ids
                    .iter()
                    .position(|s| Utf16Text::from(*s).units() == &line[a..b])
                {
                    category = EmojiCategory(i as i32 + 1);
                }
                continue;
            }
            let is_variant = matches!(line[0], 9 | 32);
            let parts: Vec<_> = trim(line).split(|&c| c == 59).collect();
            let value = Utf16Text::from_units(parts[0].to_vec());
            if value.is_empty() {
                continue;
            }
            if is_variant {
                variants.push(value);
                continue;
            }
            flush(&last, &mut variants);
            let keywords = if parts.len() > 2 && !parts[2].is_empty() {
                parts[2]
                    .split(|&c| c == 124)
                    .map(|s| Utf16Text::from_units(s.to_vec()))
                    .collect()
            } else {
                Vec::new()
            };
            let info = Arc::new(EmojiInfo {
                value: value.clone(),
                name: parts
                    .get(1)
                    .map_or_else(Utf16Text::default, |s| Utf16Text::from_units(s.to_vec())),
                keywords: Arc::new(Mutex::new(keywords)),
                category,
                variants: Arc::default(),
            });
            catalog
                .by_category
                .entry(category)
                .or_default()
                .lock()
                .push(info.clone());
            match catalog.by_value.entry(value) {
                Entry::Occupied(mut entry) => {
                    entry.insert(info.clone());
                }
                Entry::Vacant(entry) => {
                    base_order.push(entry.key().clone());
                    entry.insert(info.clone());
                }
            }
            last = Some(info);
        }
        flush(&last, &mut variants);
        let infos: Vec<_> = base_order
            .iter()
            .map(|key| catalog.by_value[key].clone())
            .collect();
        for info in infos {
            for v in info.variants.lock().iter() {
                catalog
                    .by_value
                    .entry(v.clone())
                    .or_insert_with(|| info.clone());
            }
        }
        Ok(Arc::new(catalog))
    }
    pub fn categories(&self) -> Vec<EmojiCategory> {
        let mut v: Vec<_> = self.by_category.keys().copied().collect();
        v.sort();
        v
    }
    pub fn get_emojis(&self, category: EmojiCategory) -> Arc<Mutex<Vec<Arc<EmojiInfo>>>> {
        self.by_category.get(&category).cloned().unwrap_or_default()
    }
    /// Owns a snapshot of the shared catalog entries; no collection lock survives iteration.
    pub fn all(&self) -> impl Iterator<Item = Arc<EmojiInfo>> + use<> {
        self.categories()
            .into_iter()
            .flat_map(|category| self.get_emojis(category).lock().clone())
            .collect::<Vec<_>>()
            .into_iter()
    }
    pub fn get_nullable(&self, value: Option<&Utf16Text>) -> EmojiResult<Option<Arc<EmojiInfo>>> {
        Ok(self.get(value.ok_or(EmojiError::ArgumentNull("key"))?))
    }
    pub fn get(&self, value: &Utf16Text) -> Option<Arc<EmojiInfo>> {
        self.by_value.get(value).cloned()
    }
    /// Ranked owned results. Each search snapshots entries and reads each live keyword vector.
    pub fn search(&self, query: &Utf16Text, max_results: i32) -> Vec<Arc<EmojiInfo>> {
        let lower =
            TextCulture::to_lower_invariant(&Utf16Text::from_units(trim(query.units()).to_vec()));
        let terms: Vec<_> = lower
            .units()
            .split(|&c| c == 32)
            .filter(|s| !s.is_empty())
            .collect();
        if terms.is_empty() {
            return Vec::new();
        }
        let mut scored = Vec::new();
        for info in self.all() {
            let name = TextCulture::to_lower_invariant(&info.name);
            let words = name.units().split(|&c| matches!(c, 32 | 45 | 58));
            // Scoring has no user callbacks. Borrow keyword storage for this record instead
            // of cloning it, then release its guard before moving the shared result.
            let keywords = info.keywords.lock();
            let mut total = 0i32;
            let mut ok = true;
            for &term in &terms {
                let score = if ordinal_ignore_case_units(info.name.units(), term) {
                    100
                } else if words.clone().any(|word| word == term) {
                    60
                } else if words.clone().any(|word| word.starts_with(term)) {
                    40
                } else {
                    let mut exact = false;
                    for k in keywords.iter() {
                        if ordinal_ignore_case_units(k.units(), term) {
                            exact = true;
                            break;
                        }
                    }
                    if exact {
                        35
                    } else {
                        let mut prefix = false;
                        for k in keywords.iter() {
                            if k.len() >= term.len()
                                && ordinal_ignore_case_units(&k.units()[..term.len()], term)
                            {
                                prefix = true;
                                break;
                            }
                        }
                        if prefix { 20 } else { 0 }
                    }
                };
                if score == 0 {
                    ok = false;
                    break;
                }
                total = total.wrapping_add(score);
            }
            drop(keywords);
            if ok {
                let score = total.wrapping_mul(100).wrapping_sub(info.name.len() as i32);
                scored.push((info, score));
            }
        }
        scored.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
        scored.truncate(max_results.max(0) as usize);
        scored.into_iter().map(|(info, _)| info).collect()
    }
    pub fn load_emoticons() -> EmojiResult<Vec<Arc<Emoticon>>> {
        let Some(json) = EmbeddedResources::read_text("media/emoticon/emoticons.json")
            .map_err(EmojiError::Resource)?
        else {
            return Ok(Vec::new());
        };
        let json = json.to_utf8().map_err(|_| EmojiError::ResourceEncoding)?;
        let root = StrictJsonValue::parse(&json)?;
        let mut result = Vec::new();
        for row in root
            .get("arrangement")?
            .ok_or(EmojiError::MissingProperty)?
            .array()?
        {
            for item in row.array()? {
                let meanings = if let Some(m) = item.get("meaning")? {
                    m.array()?
                        .iter()
                        .map(|s| s.string().map(|s| s.unwrap_or_default()))
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    Vec::new()
                };
                let text = item
                    .get("icon")?
                    .ok_or(EmojiError::MissingProperty)?
                    .string()?
                    .unwrap_or_default();
                result.push(Arc::new(Emoticon {
                    text,
                    meanings: Arc::new(Mutex::new(meanings)),
                }));
            }
        }
        Ok(result)
    }
    pub fn strip_skin_tone(emoji: &Utf16Text) -> EmojiResult<Utf16Text> {
        let mut units = Vec::new();
        for cp in TextUtils::get_code_points(emoji) {
            if !(0x1f3fb..=0x1f3ff).contains(&cp) {
                let c = char::from_u32(cp as u32).ok_or(EmojiError::InvalidScalar)?;
                let mut b = [0; 2];
                units.extend_from_slice(c.encode_utf16(&mut b));
            }
        }
        Ok(Utf16Text::from_units(units))
    }
}
