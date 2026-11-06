// Derived from Layouts/LayoutModels.cs; see NOTICE.md.
use super::{Key, PopupSet, ResourceObjectList, ResourceReadOnlyList, ResourceTexts};
use crate::{
    keys::{KeyVariation, LayoutDirection},
    text::{ComposerRules, Utf16Text},
};
use std::sync::Arc;
use std::sync::LazyLock;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
/// Source Int32 enum: preserves values outside the declared members.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LayoutType(pub i32);
#[allow(non_upper_case_globals)]
impl LayoutType {
    pub const Characters: Self = Self(0);
    pub const CharactersMod: Self = Self(1);
    pub const Extension: Self = Self(2);
    pub const Numeric: Self = Self(3);
    pub const NumericAdvanced: Self = Self(4);
    pub const NumericRow: Self = Self(5);
    pub const Phone: Self = Self(6);
    pub const Phone2: Self = Self(7);
    pub const Symbols: Self = Self(8);
    pub const SymbolsMod: Self = Self(9);
    pub const Symbols2: Self = Self(10);
    pub const Symbols2Mod: Self = Self(11);
    pub const ALL: [Self; 12] = [
        Self::Characters,
        Self::CharactersMod,
        Self::Extension,
        Self::Numeric,
        Self::NumericAdvanced,
        Self::NumericRow,
        Self::Phone,
        Self::Phone2,
        Self::Symbols,
        Self::SymbolsMod,
        Self::Symbols2,
        Self::Symbols2Mod,
    ];
    pub fn to_id(self) -> Result<&'static str, UnsupportedLayoutType> {
        Ok(match self {
            Self::Characters => "characters",
            Self::CharactersMod => "charactersMod",
            Self::Extension => "extension",
            Self::Numeric => "numeric",
            Self::NumericAdvanced => "numericAdvanced",
            Self::NumericRow => "numericRow",
            Self::Phone => "phone",
            Self::Phone2 => "phone2",
            Self::Symbols => "symbols",
            Self::SymbolsMod => "symbolsMod",
            Self::Symbols2 => "symbols2",
            Self::Symbols2Mod => "symbols2Mod",
            _ => return Err(UnsupportedLayoutType(self)),
        })
    }
    pub fn try_parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| {
            t.to_id()
                .expect("ALL contains only declared layout types")
                .eq_ignore_ascii_case(id)
        })
    }
    /// Nullable UTF-16 bridge for the source TryParse input; None represents false.
    /// Rust callers use unwrap_or_default() when they also need the source out value.
    pub fn try_parse_nullable(id: Option<&Utf16Text>) -> Option<Self> {
        let units = id?.units();
        Self::ALL.into_iter().find(|t| {
            let name = t.to_id().expect("ALL contains only declared layout types");
            units.len() == name.len()
                && units.iter().zip(name.bytes()).all(|(&unit, byte)| {
                    unit == u16::from(byte)
                        || (unit <= 0x7f && (unit as u8).eq_ignore_ascii_case(&byte))
                })
        })
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ComponentName {
    pub extension_id: Option<Utf16Text>,
    pub component_id: Option<Utf16Text>,
}
impl ComponentName {
    pub fn new(extension_id: impl Into<Utf16Text>, component_id: impl Into<Utf16Text>) -> Self {
        Self {
            extension_id: Some(extension_id.into()),
            component_id: Some(component_id.into()),
        }
    }
    pub fn new_nullable(extension_id: Option<Utf16Text>, component_id: Option<Utf16Text>) -> Self {
        Self {
            extension_id,
            component_id,
        }
    }
    pub fn parse_nullable(
        value: Option<&Utf16Text>,
        default_extension: Option<&Utf16Text>,
    ) -> Result<Self, super::ResourceTextListError> {
        let value = value.ok_or(super::ResourceTextListError::NullReference)?;
        Ok(match value.units().iter().position(|c| *c == b':' as u16) {
            Some(_) => Self::parse(value, &Utf16Text::default()),
            None => Self::new_nullable(default_extension.cloned(), Some(value.clone())),
        })
    }
    pub fn parse(value: &Utf16Text, default_extension: &Utf16Text) -> Self {
        match value.units().iter().position(|c| *c == b':' as u16) {
            Some(i) => Self::new(
                Utf16Text::from_units(value.units()[..i].to_vec()),
                Utf16Text::from_units(value.units()[i + 1..].to_vec()),
            ),
            None => Self::new(default_extension.clone(), value.clone()),
        }
    }
    pub fn to_text(&self) -> Utf16Text {
        let empty = Utf16Text::default();
        concat(&[
            self.extension_id.as_ref().unwrap_or(&empty),
            &":".into(),
            self.component_id.as_ref().unwrap_or(&empty),
        ])
    }
}
impl From<Utf16Text> for ComponentName {
    fn from(value: Utf16Text) -> Self {
        Self::parse(&value, &Utf16Text::default())
    }
}
impl TryFrom<Option<Utf16Text>> for ComponentName {
    type Error = super::ResourceTextListError;
    fn try_from(value: Option<Utf16Text>) -> Result<Self, Self::Error> {
        Self::parse_nullable(value.as_ref(), Some(&Utf16Text::default()))
    }
}
impl From<&str> for ComponentName {
    fn from(value: &str) -> Self {
        Self::parse(&value.into(), &Utf16Text::default())
    }
}
fn concat(parts: &[&Utf16Text]) -> Utf16Text {
    Utf16Text::from_units(
        parts
            .iter()
            .flat_map(|p| p.units().iter().copied())
            .collect::<Vec<_>>(),
    )
}
pub type SharedList<T> = Rc<RefCell<Vec<T>>>;
pub type SharedMap<K, V> = Rc<RefCell<HashMap<K, V>>>;
pub type ArrangementRow = ResourceReadOnlyList<Key>;
pub type ArrangementRows = ResourceReadOnlyList<ArrangementRow>;
pub struct LayoutArrangement {
    pub rows: Option<ArrangementRows>,
}
impl PartialEq for LayoutArrangement {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for LayoutArrangement {}
impl LayoutArrangement {
    pub fn new(rows: ArrangementRows) -> Self {
        Self { rows: Some(rows) }
    }
    pub fn new_nullable(rows: Option<ArrangementRows>) -> Self {
        Self { rows }
    }
    /// Additional native convenience; both levels are fresh mutable Lists, as in ReadArrangement.
    pub fn from_rows(rows: Vec<Vec<Key>>) -> Self {
        Self::new(
            ResourceObjectList::new(
                rows.into_iter()
                    .map(|row| ResourceObjectList::new(row).into())
                    .collect(),
            )
            .into(),
        )
    }
    pub fn empty() -> Rc<Self> {
        ferroui_system::ferro_thread_local! {static VALUE:Rc<LayoutArrangement>=Rc::new(LayoutArrangement::new(ArrangementRows::from_values(Vec::new())));}
        VALUE.with(Clone::clone)
    }
}
/// Shared owner-thread layout configuration. Setters keep the original allocation;
/// Rust callers and XAML construction update the same model.
pub struct KeyboardLayout {
    values: RefCell<KeyboardLayoutValues>,
}
struct KeyboardLayoutValues {
    layout_type: LayoutType,
    extension_id: Option<Utf16Text>,
    id: Option<Utf16Text>,
    label: Option<Utf16Text>,
    authors: Option<ResourceTexts>,
    direction: LayoutDirection,
    modifier: Option<ComponentName>,
    arrangement: Option<Rc<LayoutArrangement>>,
}
impl PartialEq for KeyboardLayout {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for KeyboardLayout {}
impl Default for KeyboardLayout {
    fn default() -> Self {
        Self::new_nullable(LayoutType::Characters, None, None, None)
    }
}
macro_rules! layout_property {
    ($get:ident, $init:ident, $type:ty) => {
        pub fn $get(&self) -> $type {
            self.values.borrow().$get.clone()
        }
        /// Updates the shared configuration and releases the previous value after borrowing.
        pub fn $init(&self, value: $type) {
            let _previous = {
                let mut fields = self.values.borrow_mut();
                std::mem::replace(&mut fields.$get, value)
            };
        }
    };
}
impl KeyboardLayout {
    pub fn new(
        layout_type: LayoutType,
        extension_id: Utf16Text,
        id: Utf16Text,
        arrangement: Rc<LayoutArrangement>,
    ) -> Self {
        Self::new_nullable(layout_type, Some(extension_id), Some(id), Some(arrangement))
    }
    pub fn new_nullable(
        layout_type: LayoutType,
        extension_id: Option<Utf16Text>,
        id: Option<Utf16Text>,
        arrangement: Option<Rc<LayoutArrangement>>,
    ) -> Self {
        Self {
            values: RefCell::new(KeyboardLayoutValues {
                layout_type,
                extension_id,
                id,
                arrangement,
                label: Some(Utf16Text::default()),
                authors: Some(ResourceTexts::default()),
                direction: LayoutDirection::Ltr,
                modifier: None,
            }),
        }
    }
    layout_property!(layout_type, set_type, LayoutType);
    layout_property!(extension_id, set_extension_id, Option<Utf16Text>);
    layout_property!(id, set_id, Option<Utf16Text>);
    layout_property!(label, set_label, Option<Utf16Text>);
    layout_property!(authors, set_authors, Option<ResourceTexts>);
    layout_property!(direction, set_direction, LayoutDirection);
    layout_property!(modifier, set_modifier, Option<ComponentName>);
    layout_property!(arrangement, set_arrangement, Option<Rc<LayoutArrangement>>);
    pub fn name(&self) -> ComponentName {
        ComponentName::new_nullable(self.extension_id(), self.id())
    }
}
pub type PopupLabelMappings = super::ResourceReadOnlyDictionary<Utf16Text, Rc<PopupSet>>;
pub type PopupVariationMappings =
    super::ResourceReadOnlyDictionary<KeyVariation, PopupLabelMappings>;
/// Additional native constructor/snapshot. Each nullable value retains its UTF-16 handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PunctuationRuleValues {
    pub extension_id: Option<Utf16Text>,
    pub id: Option<Utf16Text>,
    pub label: Option<Utf16Text>,
    pub symbols_preceding_auto_space: Option<Utf16Text>,
    pub symbols_following_auto_space: Option<Utf16Text>,
    pub symbols_preceding_phantom_space: Option<Utf16Text>,
    pub symbols_following_phantom_space: Option<Utf16Text>,
    pub symbols_terminating_sentence: Option<Utf16Text>,
}
impl Default for PunctuationRuleValues {
    fn default() -> Self {
        Self {
            extension_id: Some("".into()),
            id: Some("default".into()),
            label: Some("Default".into()),
            symbols_preceding_auto_space: Some(".,?‽!\"&%)]}»".into()),
            symbols_following_auto_space: Some("".into()),
            symbols_preceding_phantom_space: Some(".,;:?‽!&%)]}»©®™".into()),
            symbols_following_phantom_space: Some("¿⸘¡([{".into()),
            symbols_terminating_sentence: Some(".?‽!".into()),
        }
    }
}
/// Original sealed class, with reference equality and in-place runtime init accessors.
pub struct PunctuationRule {
    values: parking_lot::RwLock<PunctuationRuleValues>,
}
impl PartialEq for PunctuationRule {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for PunctuationRule {}
impl Default for PunctuationRule {
    fn default() -> Self {
        Self::from_values(PunctuationRuleValues::default())
    }
}
impl PunctuationRule {
    pub fn from_values(values: PunctuationRuleValues) -> Self {
        Self {
            values: parking_lot::RwLock::new(values),
        }
    }
    pub fn values(&self) -> PunctuationRuleValues {
        self.values.read().clone()
    }
    pub fn shared_default() -> Arc<Self> {
        static VALUE: LazyLock<Arc<PunctuationRule>> =
            LazyLock::new(|| Arc::new(PunctuationRule::default()));
        VALUE.clone()
    }
    pub fn extension_id(&self) -> Option<Utf16Text> {
        self.values.read().extension_id.clone()
    }
    pub fn set_extension_id(&self, value: Option<Utf16Text>) {
        self.values.write().extension_id = value;
    }
    pub fn id(&self) -> Option<Utf16Text> {
        self.values.read().id.clone()
    }
    pub fn set_id(&self, value: Option<Utf16Text>) {
        self.values.write().id = value;
    }
    pub fn label(&self) -> Option<Utf16Text> {
        self.values.read().label.clone()
    }
    pub fn set_label(&self, value: Option<Utf16Text>) {
        self.values.write().label = value;
    }
    pub fn symbols_preceding_auto_space(&self) -> Option<Utf16Text> {
        self.values.read().symbols_preceding_auto_space.clone()
    }
    pub fn set_symbols_preceding_auto_space(&self, value: Option<Utf16Text>) {
        self.values.write().symbols_preceding_auto_space = value;
    }
    pub fn symbols_following_auto_space(&self) -> Option<Utf16Text> {
        self.values.read().symbols_following_auto_space.clone()
    }
    pub fn set_symbols_following_auto_space(&self, value: Option<Utf16Text>) {
        self.values.write().symbols_following_auto_space = value;
    }
    pub fn symbols_preceding_phantom_space(&self) -> Option<Utf16Text> {
        self.values.read().symbols_preceding_phantom_space.clone()
    }
    pub fn set_symbols_preceding_phantom_space(&self, value: Option<Utf16Text>) {
        self.values.write().symbols_preceding_phantom_space = value;
    }
    pub fn symbols_following_phantom_space(&self) -> Option<Utf16Text> {
        self.values.read().symbols_following_phantom_space.clone()
    }
    pub fn set_symbols_following_phantom_space(&self, value: Option<Utf16Text>) {
        self.values.write().symbols_following_phantom_space = value;
    }
    pub fn symbols_terminating_sentence(&self) -> Option<Utf16Text> {
        self.values.read().symbols_terminating_sentence.clone()
    }
    pub fn set_symbols_terminating_sentence(&self, value: Option<Utf16Text>) {
        self.values.write().symbols_terminating_sentence = value;
    }
}
impl std::fmt::Display for PunctuationRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FerroBoard.Layouts.PunctuationRule")
    }
}
pub use super::subtype_records::{
    Subtype, SubtypeLayoutMap, SubtypeLayoutMapValues, SubtypeValues,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsupportedLayoutType(pub LayoutType);
impl UnsupportedLayoutType {
    pub const fn parameter_name(&self) -> &'static str {
        "type"
    }
}
impl std::fmt::Display for UnsupportedLayoutType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported layout type: {}", self.0.0)
    }
}
impl std::error::Error for UnsupportedLayoutType {}
pub struct ComposerDefinition {
    pub extension_id: Utf16Text,
    pub kind: Utf16Text,
    pub id: Utf16Text,
    pub label: Utf16Text,
    pub rules: ComposerRules,
}
impl ComposerDefinition {
    pub fn new(kind: Utf16Text, id: Utf16Text) -> Self {
        Self {
            kind,
            id,
            extension_id: "".into(),
            label: "".into(),
            rules: ComposerRules::default(),
        }
    }
}
