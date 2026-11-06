// Derived from ScreenKeyboardControl Layouts/KeyData.cs; see NOTICE.md.
use crate::{
    keys::*,
    text::{TextCaseError, TextCulture, TextUtils, Utf16Text},
};
use std::sync::LazyLock;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyDataError {
    Collection(super::ResourceTextListError),
    EmptyCharacter,
    NullCharacter,
    TextCase(TextCaseError),
    InvalidCodePoint(i32),
    InvalidSurrogate(u16),
    NullReference,
    ArrayIndexOutOfRange,
}
impl std::fmt::Display for KeyDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Collection(error) => write!(f, "key collection operation failed: {error}"),
            Self::TextCase(error) => write!(f, "key casing failed: {error}"),
            Self::EmptyCharacter => f.write_str("key character text is empty"),
            Self::NullCharacter => f.write_str("key character text is missing"),
            Self::InvalidCodePoint(value) => write!(f, "invalid key Unicode code point: {value}"),
            Self::InvalidSurrogate(value) => {
                write!(f, "invalid key UTF-16 surrogate: {value:#06x}")
            }
            Self::NullReference => f.write_str("required key data is missing"),
            Self::ArrayIndexOutOfRange => f.write_str("key code-point index is out of range"),
        }
    }
}
impl std::error::Error for KeyDataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Collection(error) => Some(error),
            Self::TextCase(error) => Some(error),
            _ => None,
        }
    }
}
impl From<super::ResourceTextListError> for KeyDataError {
    fn from(error: super::ResourceTextListError) -> Self {
        Self::Collection(error)
    }
}
/// Open source hierarchy: downstream definitions can override computation and output.
pub trait AbstractKeyData: std::any::Any {
    fn into_text(self: Rc<Self>) -> Option<TextKey> {
        None
    }

    fn serialization(&self) -> KeySerialization<'_> {
        KeySerialization::Unsupported
    }

    /// Nullable source virtual contract; concrete keys can ignore a null evaluator.
    fn compute_nullable(
        self: Rc<Self>,
        evaluator: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError>;

    fn compute(
        self: Rc<Self>,
        evaluator: &dyn IKeyEvaluator,
    ) -> Result<Option<TextKey>, KeyDataError> {
        self.compute_nullable(Some(evaluator))
    }
    fn as_string(&self, is_for_display: bool) -> Result<Option<Utf16Text>, KeyDataError>;
}
pub type Key = Rc<dyn AbstractKeyData>;
impl PartialEq for dyn AbstractKeyData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
impl Eq for dyn AbstractKeyData {}
pub type TextKey = Rc<dyn ITextKeyData>;
pub trait IKeyEvaluator {
    fn mode(&self) -> KeyboardMode;
    fn shift_state(&self) -> ShiftState;
    fn variation(&self) -> KeyVariation;
    fn direction(&self) -> LayoutDirection;
    fn is_char_half_width(&self) -> bool;
    fn is_kana_kata(&self) -> bool;
    fn culture(&self) -> Option<TextCulture>;
}
/// Virtual extension points shared by concrete text-key subclasses.
pub trait ITextKeyData: AbstractKeyData {
    /// Diagnostic label for this key. Custom Rust types work without a metadata registration.
    /// Built-in keys keep their established short labels; implementations may choose a label.
    fn diagnostic_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    fn to_text(&self) -> Result<Option<Utf16Text>, KeyDataError> {
        let name = self.diagnostic_name();
        let mut out: Vec<u16> = name.encode_utf16().collect();
        out.extend("(code=".encode_utf16());
        out.extend(self.data().code().to_string().encode_utf16());
        out.extend(", label=\"".encode_utf16());
        if let Some(label) = self.data().label() {
            out.extend_from_slice(label.units());
        }
        out.extend("\", type=".encode_utf16());
        out.extend(self.data().key_type().to_json_string().encode_utf16());
        out.push(41);
        Ok(Some(Utf16Text::from_units(out)))
    }
    fn is_multi_text(&self) -> bool {
        false
    }
    fn data(&self) -> &TextKeyData;
    fn produces_text(&self) -> Result<bool, KeyDataError> {
        Ok(self.data().produces_text())
    }
    fn with(&self, changes: KeyChanges) -> TextKey {
        Rc::new(self.data().modified(changes))
    }
}
#[derive(Default)]
pub struct KeyChanges {
    pub code: Option<i32>,
    pub label: Option<Utf16Text>,
    pub key_type: Option<KeyType>,
    pub popup: Option<Rc<PopupSet>>,
    pub group_id: Option<i32>,
}
/// Native shared key configuration; setters retain the allocation and replace its fields.
#[derive(Clone)]
pub struct TextKeyData {
    values: RefCell<TextKeyDataFields>,
}
/// Native construction values for the five configurable key properties.
#[derive(Clone)]
pub struct TextKeyDataFields {
    pub key_type: KeyType,
    pub code: i32,
    pub label: Option<Utf16Text>,
    pub group_id: i32,
    pub popup: Option<Rc<PopupSet>>,
}
impl Default for TextKeyDataFields {
    fn default() -> Self {
        Self {
            key_type: KeyType::Character,
            code: 0,
            label: Some(Utf16Text::default()),
            group_id: 0,
            popup: None,
        }
    }
}
impl Default for TextKeyData {
    fn default() -> Self {
        Self::from_fields(TextKeyDataFields::default())
    }
}
impl PartialEq for dyn ITextKeyData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
impl Eq for dyn ITextKeyData {}
#[allow(non_upper_case_globals)]
impl TextKeyData {
    pub fn from_fields(values: TextKeyDataFields) -> Self {
        Self {
            values: RefCell::new(values),
        }
    }
    pub fn key_type(&self) -> KeyType {
        self.values.borrow().key_type
    }
    pub fn code(&self) -> i32 {
        self.values.borrow().code
    }
    pub fn label(&self) -> Option<Utf16Text> {
        self.values.borrow().label.clone()
    }
    pub fn group_id(&self) -> i32 {
        self.values.borrow().group_id
    }
    pub fn popup(&self) -> Option<Rc<PopupSet>> {
        self.values.borrow().popup.clone()
    }
    pub fn set_type(&self, value: KeyType) {
        self.values.borrow_mut().key_type = value;
    }
    pub fn set_code(&self, value: i32) {
        self.values.borrow_mut().code = value;
    }
    pub fn set_label(&self, value: Option<Utf16Text>) {
        let old = std::mem::replace(&mut self.values.borrow_mut().label, value);
        drop(old);
    }
    pub fn set_group_id(&self, value: i32) {
        self.values.borrow_mut().group_id = value;
    }
    pub fn set_popup(&self, value: Option<Rc<PopupSet>>) {
        let old = std::mem::replace(&mut self.values.borrow_mut().popup, value);
        drop(old);
    }

    pub const GroupDefault: i32 = 0;
    pub const GroupLeft: i32 = 1;
    pub const GroupRight: i32 = 2;
    pub const GroupEnter: i32 = 3;
    pub const GroupKana: i32 = 97;
    pub fn unspecified() -> Rc<Self> {
        ferroui_system::ferro_thread_local! {static VALUE:Rc<TextKeyData>=Rc::new(TextKeyData::from_fields(TextKeyDataFields {key_type:KeyType::Unspecified,..Default::default()}));}
        VALUE.with(Clone::clone)
    }
    pub fn space_key() -> Rc<Self> {
        ferroui_system::ferro_thread_local! {static VALUE:Rc<TextKeyData>=Rc::new(TextKeyData::from_fields(TextKeyDataFields {code:KeyCode::Space,label:Some("space".into()),..Default::default()}));}
        VALUE.with(Clone::clone)
    }
    pub fn char(text: Utf16Text, popup: Option<Rc<PopupSet>>) -> Result<Rc<Self>, KeyDataError> {
        Self::char_nullable(Some(text), popup)
    }
    pub fn char_nullable(
        text: Option<Utf16Text>,
        popup: Option<Rc<PopupSet>>,
    ) -> Result<Rc<Self>, KeyDataError> {
        let text = text.ok_or(KeyDataError::NullCharacter)?;
        let units = text.units();
        let first = *units.first().ok_or(KeyDataError::EmptyCharacter)?;
        let code = if (0xd800..=0xdbff).contains(&first) {
            let low = *units
                .get(1)
                .filter(|v| (0xdc00..=0xdfff).contains(*v))
                .ok_or(KeyDataError::InvalidSurrogate(first))?;
            0x10000 + (((first as i32 - 0xd800) << 10) | (low as i32 - 0xdc00))
        } else if (0xdc00..=0xdfff).contains(&first) {
            return Err(KeyDataError::InvalidSurrogate(first));
        } else {
            first as i32
        };
        Ok(Rc::new(Self::from_fields(TextKeyDataFields {
            code,
            label: Some(text),
            popup,
            ..Default::default()
        })))
    }
    pub fn function(code: i32, key_type: KeyType, label: Option<Utf16Text>) -> Rc<Self> {
        Rc::new(Self::from_fields(TextKeyDataFields {
            code,
            key_type,
            label: Some(label.unwrap_or_else(|| KeyCode::get_name(code).unwrap_or("").into())),
            ..Default::default()
        }))
    }
    pub fn popup_lookup_label(&self) -> Option<Utf16Text> {
        match self.group_id() {
            Self::GroupLeft => Some("~left".into()),
            Self::GroupRight => Some("~right".into()),
            Self::GroupEnter => Some("~enter".into()),
            Self::GroupKana => Some("~kana".into()),
            _ => self.label().clone(),
        }
    }
    pub fn is_space(&self) -> bool {
        (self.key_type() == KeyType::Character && KeyCode::is_space(self.code()))
            || self.code() == KeyCode::Space
    }
    pub fn produces_text(&self) -> bool {
        matches!(self.key_type(), KeyType::Character | KeyType::Numeric)
            && KeyCode::is_character(self.code())
    }
    fn modified(&self, c: KeyChanges) -> Self {
        Self::from_fields(TextKeyDataFields {
            code: c.code.unwrap_or(self.code()),
            label: c.label.or_else(|| self.label().clone()),
            key_type: c.key_type.unwrap_or(self.key_type()),
            popup: c.popup.or_else(|| self.popup().clone()),
            group_id: c.group_id.unwrap_or(self.group_id()),
        })
    }
}
impl AbstractKeyData for TextKeyData {
    fn into_text(self: Rc<Self>) -> Option<TextKey> {
        Some(self)
    }
    fn serialization(&self) -> KeySerialization<'_> {
        KeySerialization::Text(self)
    }
    fn compute_nullable(
        self: Rc<Self>,
        _: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        Ok(Some(self))
    }
    fn as_string(&self, display: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        if KeyCode::is_character(self.code())
            && self.code() != KeyCode::Enter
            && self.code() != KeyCode::Tab
        {
            return code_point(self.code()).map(Some);
        }
        Ok(if display || self.code() == KeyCode::Unspecified {
            self.label().clone()
        } else {
            Some(Utf16Text::default())
        })
    }
}
impl ITextKeyData for TextKeyData {
    fn diagnostic_name(&self) -> &'static str {
        "TextKeyData"
    }
    fn data(&self) -> &TextKeyData {
        self
    }
}
fn code_point(code: i32) -> Result<Utf16Text, KeyDataError> {
    let scalar = u32::try_from(code)
        .ok()
        .and_then(char::from_u32)
        .ok_or(KeyDataError::InvalidCodePoint(code))?;
    let mut units = [0u16; 2];
    Ok(Utf16Text::from_units(
        scalar.encode_utf16(&mut units).to_vec(),
    ))
}
pub struct MultiTextKeyData {
    pub base: TextKeyData,
    code_points: RefCell<Option<super::CodePointArray>>,
}
impl Default for MultiTextKeyData {
    fn default() -> Self {
        Self {
            base: TextKeyData::from_fields(TextKeyDataFields {
                code: KeyCode::MultipleCodePoints,
                ..Default::default()
            }),
            code_points: RefCell::new(Some(super::CodePointArray::default())),
        }
    }
}
impl MultiTextKeyData {
    pub fn from_parts(base: TextKeyData, code_points: Option<super::CodePointArray>) -> Self {
        Self {
            base,
            code_points: RefCell::new(code_points),
        }
    }
    pub fn code_points(&self) -> Option<super::CodePointArray> {
        self.code_points.borrow().clone()
    }
    pub fn set_code_points(&self, value: Option<super::CodePointArray>) {
        let old = self.code_points.replace(value);
        drop(old);
    }
    pub fn try_produces_text(&self) -> Result<bool, KeyDataError> {
        Ok(!self
            .code_points()
            .ok_or(KeyDataError::NullReference)?
            .is_empty())
    }
}
impl PartialEq for MultiTextKeyData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for MultiTextKeyData {}
impl AbstractKeyData for MultiTextKeyData {
    fn into_text(self: Rc<Self>) -> Option<TextKey> {
        Some(self)
    }
    fn serialization(&self) -> KeySerialization<'_> {
        KeySerialization::Multi(self)
    }
    fn compute_nullable(
        self: Rc<Self>,
        _: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        Ok(Some(self))
    }
    fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        let mut output = Vec::new();
        let points = self.code_points().ok_or(KeyDataError::NullReference)?;
        for index in 0..points.len() {
            output.extend_from_slice(code_point(points.get(index as i32)?)?.units());
        }
        Ok(Some(Utf16Text::from_units(output)))
    }
}
impl ITextKeyData for MultiTextKeyData {
    fn diagnostic_name(&self) -> &'static str {
        "MultiTextKeyData"
    }
    fn is_multi_text(&self) -> bool {
        true
    }
    fn data(&self) -> &TextKeyData {
        &self.base
    }
    fn produces_text(&self) -> Result<bool, KeyDataError> {
        self.try_produces_text()
    }
    fn with(&self, mut changes: KeyChanges) -> TextKey {
        changes.code = Some(KeyCode::MultipleCodePoints);
        let points = self.code_points();
        Rc::new(Self::from_parts(self.base.modified(changes), points))
    }
}
/// Shared source object; init accessors can replace Main and the nullable list after construction.
pub struct PopupSet {
    main: RefCell<Option<Key>>,
    relevant: RefCell<Option<super::ResourceReadOnlyList<Key>>>,
}
impl Default for PopupSet {
    fn default() -> Self {
        Self::from_parts(None, super::ResourceReadOnlyList::default())
    }
}
impl PartialEq for PopupSet {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for PopupSet {}
impl PopupSet {
    pub fn from_parts(main: Option<Key>, relevant: super::ResourceReadOnlyList<Key>) -> Self {
        Self {
            main: RefCell::new(main),
            relevant: RefCell::new(Some(relevant)),
        }
    }
    pub fn main(&self) -> Option<Key> {
        self.main.borrow().clone()
    }
    pub fn relevant(&self) -> Option<super::ResourceReadOnlyList<Key>> {
        self.relevant.borrow().clone()
    }
    pub fn set_main(&self, value: Option<Key>) {
        let old = self.main.replace(value);
        drop(old);
    }
    pub fn set_relevant(&self, value: Option<super::ResourceReadOnlyList<Key>>) {
        let old = self.relevant.replace(value);
        drop(old);
    }
    pub fn empty() -> Rc<Self> {
        ferroui_system::ferro_thread_local! {static VALUE:Rc<PopupSet>=Rc::new(PopupSet::from_parts(None, super::ResourceReadOnlyList::from_values(Vec::new())));}
        VALUE.with(Clone::clone)
    }
    pub fn is_empty(&self) -> Result<bool, super::ResourceTextListError> {
        if self.main().is_some() {
            return Ok(false);
        }
        Ok(self
            .relevant()
            .ok_or(super::ResourceTextListError::NullReference)?
            .count()?
            == 0)
    }
}
// Each arm is runtime-nullable, including C# required members. Never retain a borrow
// across evaluator/child callbacks. Replacement drops the old reference after releasing storage.
macro_rules! key_selector_storage {
    ($name:ident, $required:literal, { $($field:ident => $init:ident),* $(,)? }, |$this:ident, $eval:ident| $body:expr) => {
        #[derive(Default)]
        pub struct $name { $($field: RefCell<Option<Key>>),* }
        impl $name {
            pub fn from_parts($($field:Option<Key>),*) -> Self {
                Self { $($field:RefCell::new($field)),* }
            }
            $(
                pub fn $field(&self) -> Option<Key> {self.$field.borrow().clone()}
                pub fn $init(&self, value:Option<Key>) {
                    let old=self.$field.replace(value);
                    drop(old);
                }
            )*
        }
        impl PartialEq for $name {
            fn eq(&self, other:&Self)->bool {std::ptr::eq(self,other)}
        }
        impl Eq for $name {}
        impl AbstractKeyData for $name {
            fn serialization(&self)->KeySerialization<'_> {KeySerialization::$name(self)}
            fn compute_nullable(self:Rc<Self>, evaluator:Option<&dyn IKeyEvaluator>)->Result<Option<TextKey>,KeyDataError> {
                let $eval=evaluator.ok_or(KeyDataError::NullReference)?;
                let $this=&*self;
                let selected:Option<Key>=$body;
                match selected {
                    Some(key)=>key.compute_nullable(Some($eval)),
                    None if $required=>Err(KeyDataError::NullReference),
                    None=>Ok(None)
                }
            }
            fn as_string(&self,_:bool)->Result<Option<Utf16Text>,KeyDataError> {Ok(Some(Utf16Text::default()))}
        }
    }
}
key_selector_storage!(CaseSelector,true,{lower=>set_lower,upper=>set_upper},|s,e|
    if e.shift_state().is_uppercase() {s.upper()} else {s.lower()});
key_selector_storage!(ShiftStateSelector,false,{
    unshifted=>set_unshifted,shifted=>set_shifted,shifted_manual=>set_shifted_manual,
    shifted_automatic=>set_shifted_automatic,caps_lock=>set_caps_lock,default_key=>set_default
},|s,e| match e.shift_state() {
    ShiftState::Unshifted=>s.unshifted().or_else(||s.default_key()),
    ShiftState::ShiftedManual=>s.shifted_manual().or_else(||s.shifted()).or_else(||s.default_key()),
    ShiftState::ShiftedAutomatic=>s.shifted_automatic().or_else(||s.shifted()).or_else(||s.default_key()),
    ShiftState::CapsLock=>s.caps_lock().or_else(||s.shifted()).or_else(||s.default_key()),
    _=>s.default_key(),
});
key_selector_storage!(VariationSelector,false,{
    default_key=>set_default,email=>set_email,uri=>set_uri,normal=>set_normal,password=>set_password
},|s,e| match e.variation() {
    KeyVariation::EmailAddress=>s.email().or_else(||s.default_key()),
    KeyVariation::Uri=>s.uri().or_else(||s.default_key()),
    KeyVariation::Normal=>s.normal().or_else(||s.default_key()),
    KeyVariation::Password=>s.password().or_else(||s.default_key()),
    _=>s.default_key(),
});
key_selector_storage!(LayoutDirectionSelector,true,{ltr=>set_ltr,rtl=>set_rtl},|s,e|
    if e.direction()==LayoutDirection::Rtl {s.rtl()} else {s.ltr()});
key_selector_storage!(CharWidthSelector,false,{full=>set_full,half=>set_half},|s,e|
    if e.is_char_half_width() {s.half()} else {s.full()});
key_selector_storage!(KanaSelector,true,{hira=>set_hira,kata=>set_kata},|s,e|
    if e.is_kana_kata() {s.kata()} else {s.hira()});

/// A character definition that uppercases its label using the evaluator culture.
pub struct AutoTextKeyData {
    pub base: TextKeyData,
}
impl PartialEq for AutoTextKeyData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for AutoTextKeyData {}
impl AbstractKeyData for AutoTextKeyData {
    fn into_text(self: Rc<Self>) -> Option<TextKey> {
        Some(self)
    }
    fn serialization(&self) -> KeySerialization<'_> {
        KeySerialization::Auto(self)
    }
    fn compute_nullable(
        self: Rc<Self>,
        evaluator: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        let evaluator = evaluator.ok_or(KeyDataError::NullReference)?;
        if !evaluator.shift_state().is_uppercase() || !KeyCode::is_character(self.base.code()) {
            return Ok(Some(self));
        }
        // C# evaluates Label, then Culture; the latter can invoke user code and mutate init fields.
        let label = self.base.label();
        let culture = evaluator.culture();
        let upper = Self::to_upper_with_culture(label.as_ref(), culture.as_ref())?;
        if self.base.label().as_ref() == Some(&upper) {
            return Ok(Some(self));
        }
        Ok(Some(Self::create_from_text(
            upper,
            self.base.key_type(),
            self.base.group_id(),
            self.base.popup().clone(),
        )))
    }
    fn as_string(&self, display: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        self.base.as_string(display)
    }
}
impl ITextKeyData for AutoTextKeyData {
    fn diagnostic_name(&self) -> &'static str {
        "AutoTextKeyData"
    }
    fn data(&self) -> &TextKeyData {
        &self.base
    }
    fn with(&self, changes: KeyChanges) -> TextKey {
        Rc::new(Self {
            base: self.base.modified(changes),
        })
    }
}
fn capital_sharp_s() -> Utf16Text {
    static VALUE: LazyLock<Utf16Text> = LazyLock::new(|| "ẞ".into());
    VALUE.clone()
}
impl AutoTextKeyData {
    /// Uppercase with an optional native profile; resolve the host locale only when needed.
    pub fn to_upper_with_culture(
        text: Option<&Utf16Text>,
        culture: Option<&TextCulture>,
    ) -> Result<Utf16Text, KeyDataError> {
        if text.is_some_and(|text| text.units() == [0xdf]) {
            return Ok(capital_sharp_s());
        }
        // C# dereferences the string before resolving a null culture to the ambient object.
        let text = text.ok_or(KeyDataError::NullReference)?;
        let current;
        let culture = match culture {
            Some(culture) => culture,
            None => {
                current = TextCulture::current().map_err(KeyDataError::TextCase)?;
                &current
            }
        };
        Ok(culture.to_upper(text))
    }
    pub fn to_upper(text: &Utf16Text, culture: TextCulture) -> Utf16Text {
        if text.units() == [0xdf] {
            capital_sharp_s()
        } else {
            culture.to_upper(text)
        }
    }
    pub fn create_from_text(
        text: Utf16Text,
        key_type: KeyType,
        group_id: i32,
        popup: Option<Rc<PopupSet>>,
    ) -> TextKey {
        let code_points = TextUtils::get_code_points(&text);
        if code_points.len() == 1 {
            Rc::new(TextKeyData::from_fields(TextKeyDataFields {
                code: code_points[0],
                label: Some(text),
                key_type,
                group_id,
                popup,
            }))
        } else {
            Rc::new(MultiTextKeyData::from_parts(
                TextKeyData::from_fields(TextKeyDataFields {
                    code: KeyCode::MultipleCodePoints,
                    label: Some(text),
                    key_type,
                    group_id,
                    popup,
                }),
                Some(super::CodePointArray::new(code_points)),
            ))
        }
    }
}

/// Serialization projection preserves the open key hierarchy without runtime reflection.
#[derive(Clone, Copy)]
pub enum KeySerialization<'a> {
    Unsupported,
    Text(&'a TextKeyData),
    Auto(&'a AutoTextKeyData),
    Multi(&'a MultiTextKeyData),
    CaseSelector(&'a CaseSelector),
    ShiftStateSelector(&'a ShiftStateSelector),
    VariationSelector(&'a VariationSelector),
    LayoutDirectionSelector(&'a LayoutDirectionSelector),
    CharWidthSelector(&'a CharWidthSelector),
    KanaSelector(&'a KanaSelector),
}
