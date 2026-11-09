//! Native subtype configuration with shared mutable fields; derived from LayoutModels.cs.
use crate::{
    layouts::{ComponentName, LayoutType, ResourceError, ResourceResult, UnsupportedLayoutType},
    text::{TextCulture, Utf16Text},
};
use parking_lot::RwLock;
use std::{
    hash::{Hash, Hasher},
    sync::{Arc, LazyLock},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubtypeLayoutMapValues {
    pub characters: ComponentName,
    pub symbols: ComponentName,
    pub symbols2: ComponentName,
    pub numeric: ComponentName,
    pub numeric_advanced: ComponentName,
    pub numeric_row: ComponentName,
    pub phone: ComponentName,
    pub phone2: ComponentName,
}
impl Default for SubtypeLayoutMapValues {
    fn default() -> Self {
        let name = |id| ComponentName::new(SubtypeLayoutMap::CORE_LAYOUTS, id);
        Self {
            characters: name("qwerty"),
            symbols: name("western"),
            symbols2: name("western"),
            numeric: name("western_arabic"),
            numeric_advanced: name("western_arabic"),
            numeric_row: name("western_arabic"),
            phone: name("telpad"),
            phone2: name("telpad"),
        }
    }
}
#[derive(Debug)]
pub struct SubtypeLayoutMap {
    values: RwLock<SubtypeLayoutMapValues>,
}
macro_rules! field {
    ($get:ident,$set:ident,$ty:ty) => {
        pub fn $get(&self) -> $ty {
            self.values.read().$get.clone()
        }
        pub fn $set(&self, value: $ty) {
            let old = {
                let mut fields = self.values.write();
                std::mem::replace(&mut fields.$get, value)
            };
            drop(old);
        }
    };
}
impl SubtypeLayoutMap {
    pub const CORE_LAYOUTS: &'static str = "org.florisboard.layouts";
    pub fn from_values(values: SubtypeLayoutMapValues) -> Self {
        Self {
            values: RwLock::new(values),
        }
    }
    pub fn values(&self) -> SubtypeLayoutMapValues {
        self.values.read().clone()
    }
    field!(characters, set_characters, ComponentName);
    field!(symbols, set_symbols, ComponentName);
    field!(symbols2, set_symbols2, ComponentName);
    field!(numeric, set_numeric, ComponentName);
    field!(numeric_advanced, set_numeric_advanced, ComponentName);
    field!(numeric_row, set_numeric_row, ComponentName);
    field!(phone, set_phone, ComponentName);
    field!(phone2, set_phone2, ComponentName);
    pub fn get(&self, kind: LayoutType) -> Result<ComponentName, UnsupportedLayoutType> {
        Ok(match kind {
            LayoutType::Characters => self.characters(),
            LayoutType::Symbols => self.symbols(),
            LayoutType::Symbols2 => self.symbols2(),
            LayoutType::Numeric => self.numeric(),
            LayoutType::NumericAdvanced => self.numeric_advanced(),
            LayoutType::NumericRow => self.numeric_row(),
            LayoutType::Phone => self.phone(),
            LayoutType::Phone2 => self.phone2(),
            _ => return Err(UnsupportedLayoutType(kind)),
        })
    }
}
impl Default for SubtypeLayoutMap {
    fn default() -> Self {
        Self::from_values(Default::default())
    }
}
impl Clone for SubtypeLayoutMap {
    fn clone(&self) -> Self {
        Self::from_values(self.values())
    }
}
impl PartialEq for SubtypeLayoutMap {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || self.values() == other.values()
    }
}
impl Eq for SubtypeLayoutMap {}
// Rust collection hashing only. This is not a claimed port of the generated CLR GetHashCode API.
impl Hash for SubtypeLayoutMap {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.values().hash(state)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubtypeValues {
    pub language_tag: Option<Utf16Text>,
    pub display_name: Option<Utf16Text>,
    pub composer: ComponentName,
    pub currency_set: ComponentName,
    pub popup_mapping: ComponentName,
    pub punctuation_rule: ComponentName,
    pub layouts: Option<Arc<SubtypeLayoutMap>>,
}
impl Default for SubtypeValues {
    fn default() -> Self {
        Self {
            language_tag: None,
            display_name: None,
            composer: ComponentName::new(Subtype::CORE_COMPOSERS, "appender"),
            currency_set: ComponentName::new(Subtype::CORE_CURRENCY_SETS, "dollar"),
            popup_mapping: ComponentName::new(Subtype::CORE_LOCALIZATION, "default"),
            punctuation_rule: ComponentName::new(Subtype::CORE_LOCALIZATION, "default"),
            layouts: Some(Arc::new(SubtypeLayoutMap::default())),
        }
    }
}
#[derive(Debug)]
pub struct Subtype {
    values: RwLock<SubtypeValues>,
}
impl Subtype {
    pub const CORE_COMPOSERS: &'static str = "org.florisboard.composers";
    pub const CORE_CURRENCY_SETS: &'static str = "org.florisboard.currencysets";
    pub const CORE_LOCALIZATION: &'static str = "org.florisboard.localization";
    pub fn from_values(values: SubtypeValues) -> Self {
        Self {
            values: RwLock::new(values),
        }
    }
    pub fn new(language_tag: Utf16Text) -> Self {
        Self::from_values(SubtypeValues {
            language_tag: Some(language_tag),
            ..Default::default()
        })
    }
    pub fn values(&self) -> SubtypeValues {
        self.values.read().clone()
    }
    field!(language_tag, set_language_tag, Option<Utf16Text>);
    field!(display_name, set_display_name, Option<Utf16Text>);
    field!(composer, set_composer, ComponentName);
    field!(currency_set, set_currency_set, ComponentName);
    field!(popup_mapping, set_popup_mapping, ComponentName);
    field!(punctuation_rule, set_punctuation_rule, ComponentName);
    field!(layouts, set_layouts, Option<Arc<SubtypeLayoutMap>>);
    pub fn shared_default() -> Arc<Self> {
        static VALUE: LazyLock<Arc<Subtype>> = LazyLock::new(|| {
            Arc::new(Subtype::from_values(SubtypeValues {
                language_tag: Some("en-US".into()),
                popup_mapping: ComponentName::new(Subtype::CORE_LOCALIZATION, "en"),
                ..Default::default()
            }))
        });
        VALUE.clone()
    }
    pub fn id(&self) -> Utf16Text {
        self.try_id().unwrap_or_else(|e| std::panic::panic_any(e))
    }
    pub fn short_label(&self) -> Utf16Text {
        self.try_short_label()
            .unwrap_or_else(|e| std::panic::panic_any(e))
    }
    pub fn try_id(&self) -> ResourceResult<Utf16Text> {
        let mut units = Vec::new();
        if let Some(language) = self.language_tag() {
            units.extend_from_slice(language.units());
        }
        units.push(b'/' as u16);
        let map = self.layouts().ok_or(ResourceError::NullReference)?;
        if let Some(component) = map.characters().component_id {
            units.extend_from_slice(component.units());
        }
        Ok(Utf16Text::from_units(units))
    }

    pub fn try_short_label(&self) -> ResourceResult<Utf16Text> {
        let tag = self.language_tag().ok_or(ResourceError::NullReference)?;
        let text = match tag.units().iter().position(|c| *c == b'-' as u16) {
            Some(i) if i > 0 => Utf16Text::from_units(tag.units()[..i].to_vec()),
            _ => tag,
        };
        Ok(TextCulture::invariant().to_upper(&text))
    }
}
impl Default for Subtype {
    fn default() -> Self {
        Self::from_values(Default::default())
    }
}
impl Clone for Subtype {
    fn clone(&self) -> Self {
        Self::from_values(self.values())
    }
}
impl PartialEq for Subtype {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || self.values() == other.values()
    }
}
impl Eq for Subtype {}
impl Hash for Subtype {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.values().hash(state)
    }
}
