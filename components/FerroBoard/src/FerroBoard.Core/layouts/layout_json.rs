// Derived from Layouts/LayoutJson.cs. See NOTICE.md.
use super::*;
use crate::{
    keys::*,
    text::{ComposerRules, TextUtils, Utf16Text},
};
use std::sync::Arc;
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutJsonError(pub String);
impl std::fmt::Display for LayoutJsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for LayoutJsonError {}
impl From<String> for LayoutJsonError {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl From<KeyDataError> for LayoutJsonError {
    fn from(e: KeyDataError) -> Self {
        Self(format!("{e:?}"))
    }
}
impl From<UnsupportedLayoutType> for LayoutJsonError {
    fn from(value: UnsupportedLayoutType) -> Self {
        Self(value.to_string())
    }
}
impl From<ResourceTextListError> for LayoutJsonError {
    fn from(value: ResourceTextListError) -> Self {
        Self(format!("Collection::{value:?}"))
    }
}
impl From<crate::serialization::JsonBridgeError> for LayoutJsonError {
    fn from(value: crate::serialization::JsonBridgeError) -> Self {
        Self(value.to_string())
    }
}
type Result<T> = std::result::Result<T, LayoutJsonError>;
fn array(e: &JsonElement) -> Result<&[JsonElement]> {
    if let JsonElement::Array(a) = e {
        Ok(a)
    } else {
        Err(LayoutJsonError("Expected a JSON array".into()))
    }
}
fn object(e: &JsonElement) -> Result<super::json_element::JsonProperties<'_>> {
    e.properties().map_err(Into::into)
}
fn property<'a>(e: &'a JsonElement, n: &str) -> Result<Option<&'a JsonElement>> {
    e.get(n).map_err(Into::into)
}
fn string(e: &JsonElement, n: &str) -> Result<Option<Utf16Text>> {
    if !e.is_object() {
        return Ok(None);
    }
    match e.get(n)? {
        Some(value) if value.is_string() => value.string().map_err(Into::into),
        _ => Ok(None),
    }
}
fn str_value(e: &JsonElement) -> Result<Option<Utf16Text>> {
    e.string().map_err(Into::into)
}
fn integer(e: &JsonElement) -> Result<i32> {
    e.int32().map_err(Into::into)
}
fn get_int(e: &JsonElement, n: &str) -> Result<Option<i32>> {
    match property(e, n)? {
        Some(v @ (JsonElement::Number(_) | JsonElement::NumberLexeme(_))) => Ok(Some(integer(v)?)),
        _ => Ok(None),
    }
}
fn strings(e: &JsonElement, n: &str) -> Result<ResourceTexts> {
    let values = match property(e, n)? {
        Some(JsonElement::Array(a)) => a
            .iter()
            .map(|v| str_value(v).map(Option::unwrap_or_default))
            .collect::<Result<Vec<_>>>()?,
        _ => vec![],
    };
    Ok(ResourceTexts::new(values))
}
fn direction(value: Option<Utf16Text>) -> LayoutDirection {
    if value.is_some_and(|s| s.to_utf8().is_ok_and(|s| s.eq_ignore_ascii_case("rtl"))) {
        LayoutDirection::Rtl
    } else {
        LayoutDirection::Ltr
    }
}
fn utf8(text: &Utf16Text) -> Result<String> {
    text.to_utf8().map_err(|e| LayoutJsonError(e.to_string()))
}
fn shared<T>(v: Vec<T>) -> SharedList<T> {
    Rc::new(RefCell::new(v))
}
pub struct LayoutJson;
impl LayoutJson {
    /// Source JsonDocument.Parse(string) rejects null and invalid UTF-16 before parsing,
    /// including invalid code units in comments or unused properties.
    fn json_utf8(json: Option<&Utf16Text>) -> Result<String> {
        json.ok_or_else(|| LayoutJsonError("Layout JSON input is missing".into()))?
            .to_utf8()
            .map_err(|_| LayoutJsonError("Layout JSON contains unpaired UTF-16 code units".into()))
    }
    pub fn parse(json: &str) -> Result<JsonElement> {
        super::json_element::parse(json).map_err(Into::into)
    }
    pub fn read_arrangement(json: &str) -> Result<Rc<LayoutArrangement>> {
        let root = Self::parse(json)?;
        Self::read_arrangement_element(if root.is_object() {
            root.get("arrangement")?.unwrap_or(&root)
        } else {
            &root
        })
    }
    pub fn read_arrangement_element(root: &JsonElement) -> Result<Rc<LayoutArrangement>> {
        if !matches!(root, JsonElement::Array(_)) {
            return Err(LayoutJsonError(
                "Layout arrangement must be a JSON array of rows.".into(),
            ));
        }
        let mut rows = Vec::new();
        for row in array(root)? {
            rows.push(
                array(row)?
                    .iter()
                    .map(Self::read_key)
                    .collect::<Result<Vec<_>>>()?,
            );
        }
        Ok(Rc::new(LayoutArrangement::from_rows(rows)))
    }
    pub fn read_layout_document(json: &str, extension_id: Utf16Text) -> Result<KeyboardLayout> {
        Self::read_layout_document_nullable(json, Some(extension_id))
    }
    /// The optional extension here is an explicit source argument; callers implementing
    /// the omitted default pass Some("custom"). It is never transcoded with the JSON.
    pub fn read_layout_document_utf16(
        json: Option<&Utf16Text>,
        extension_id: Option<Utf16Text>,
    ) -> Result<KeyboardLayout> {
        Self::read_layout_document_nullable(&Self::json_utf8(json)?, extension_id)
    }
    pub fn read_layout_document_nullable(
        json: &str,
        extension_id: Option<Utf16Text>,
    ) -> Result<KeyboardLayout> {
        let root = Self::parse(json)?;
        let type_id = string(&root, "type")?.unwrap_or_else(|| "characters".into());
        let layout_type = LayoutType::try_parse(&utf8(&type_id)?)
            .ok_or_else(|| LayoutJsonError("Unknown layout type".into()))?;
        let id = string(&root, "id")?
            .ok_or_else(|| LayoutJsonError("Layout document requires an 'id'.".into()))?;
        let label = string(&root, "label")?.unwrap_or_else(|| id.clone());
        let authors = strings(&root, "authors")?;
        let direction = direction(string(&root, "direction")?);
        let modifier = string(&root, "modifier")?
            .map(|v| ComponentName::parse_nullable(Some(&v), extension_id.as_ref()))
            .transpose()?;
        let arrangement = match property(&root, "arrangement")? {
            Some(a) => Self::read_arrangement_element(a)?,
            None => LayoutArrangement::empty(),
        };
        let layout = KeyboardLayout::new_nullable(
            layout_type,
            extension_id.clone(),
            Some(id.clone()),
            Some(arrangement),
        );
        layout.set_label(Some(label));
        layout.set_authors(Some(authors));
        layout.set_direction(direction);
        layout.set_modifier(modifier);
        Ok(layout)
    }

    pub fn read_key(el: &JsonElement) -> Result<Key> {
        if el.is_string() {
            let text = el.string()?.unwrap_or_default();
            let cps = TextUtils::get_code_points(&text);
            return Ok(if cps.len() == 1 {
                Rc::new(AutoTextKeyData {
                    base: TextKeyData::from_fields(TextKeyDataFields {
                        code: cps[0],
                        label: Some(text),
                        ..Default::default()
                    }),
                })
            } else {
                Rc::new(MultiTextKeyData::from_parts(
                    TextKeyData::from_fields(TextKeyDataFields {
                        label: Some(text),
                        code: KeyCode::MultipleCodePoints,
                        ..Default::default()
                    }),
                    Some(CodePointArray::new(cps)),
                ))
            });
        }
        if !el.is_object() {
            return Err(LayoutJsonError("Invalid key definition".into()));
        }
        let kind = string(el, "$")?.unwrap_or_else(|| "text_key".into());
        let required = |n: &str| -> Result<Key> {
            Self::read_key(
                property(el, n)?
                    .ok_or_else(|| LayoutJsonError(format!("Missing '{n}' in selector.")))?,
            )
        };
        let optional = |n: &str| -> Result<Option<Key>> {
            match property(el, n)? {
                None | Some(JsonElement::Null) => Ok(None),
                Some(e) => Self::read_key(e).map(Some),
            }
        };
        let popup = || -> Result<Option<Rc<PopupSet>>> {
            property(el, "popup")?.map(Self::read_popup_set).transpose()
        };
        Ok(match utf8(&kind)?.as_str() {
            "text_key" | "auto_text_key" => {
                let type_name = string(el, "type")?.map(|v| utf8(&v)).transpose()?;
                let key_type = KeyType::parse_key_type(type_name.as_deref());
                let mut code = get_int(el, "code")?.unwrap_or(0);
                let mut label = string(el, "label")?.unwrap_or_default();
                if code == 0 && key_type != KeyType::Placeholder && !label.is_empty() {
                    if let Some(function) = KeyCode::try_get_code(&utf8(&label)?) {
                        code = function;
                    } else {
                        let cps = TextUtils::get_code_points(&label);
                        if cps.len() == 1 {
                            code = cps[0];
                        }
                    }
                }
                if label.is_empty() && KeyCode::is_character(code) {
                    label = scalar(code)?;
                }
                let base = TextKeyData::from_fields(TextKeyDataFields {
                    key_type,
                    code,
                    label: Some(label),
                    group_id: get_int(el, "groupId")?.unwrap_or(0),
                    popup: popup()?,
                });
                if kind == Utf16Text::from("auto_text_key") {
                    Rc::new(AutoTextKeyData { base })
                } else {
                    Rc::new(base)
                }
            }
            "multi_text_key" => {
                let cps = match property(el, "codePoints")? {
                    None => vec![],
                    Some(a) => array(a)?.iter().map(integer).collect::<Result<Vec<_>>>()?,
                };
                let label = match string(el, "label")? {
                    Some(s) => s,
                    None => {
                        let mut units = Vec::new();
                        for cp in &cps {
                            units.extend_from_slice(scalar(*cp)?.units());
                        }
                        Utf16Text::from_units(units)
                    }
                };
                let type_name = string(el, "type")?.map(|v| utf8(&v)).transpose()?;
                Rc::new(MultiTextKeyData::from_parts(
                    TextKeyData::from_fields(TextKeyDataFields {
                        code: KeyCode::MultipleCodePoints,
                        label: Some(label),
                        key_type: KeyType::parse_key_type(type_name.as_deref()),
                        group_id: get_int(el, "groupId")?.unwrap_or(0),
                        popup: popup()?,
                    }),
                    Some(CodePointArray::new(cps)),
                ))
            }
            "case_selector" => Rc::new(CaseSelector::from_parts(
                Some(required("lower")?),
                Some(required("upper")?),
            )),
            "shift_state_selector" => Rc::new(ShiftStateSelector::from_parts(
                optional("unshifted")?,
                optional("shifted")?,
                optional("shiftedManual")?,
                optional("shiftedAutomatic")?,
                optional("capsLock")?,
                optional("default")?,
            )),
            "variation_selector" => Rc::new(VariationSelector::from_parts(
                optional("default")?,
                optional("email")?,
                optional("uri")?,
                optional("normal")?,
                optional("password")?,
            )),
            "layout_direction_selector" => Rc::new(LayoutDirectionSelector::from_parts(
                Some(required("ltr")?),
                Some(required("rtl")?),
            )),
            "char_width_selector" => Rc::new(CharWidthSelector::from_parts(
                optional("full")?,
                optional("half")?,
            )),
            "kana_selector" => Rc::new(KanaSelector::from_parts(
                Some(required("hira")?),
                Some(required("kata")?),
            )),
            _ => {
                return Err(LayoutJsonError(format!(
                    "Unknown key data type '{}'",
                    utf8(&kind)?
                )));
            }
        })
    }
    pub fn read_popup_set(el: &JsonElement) -> Result<Rc<PopupSet>> {
        if let JsonElement::Array(a) = el {
            return Ok(Rc::new(PopupSet::from_parts(
                None,
                (ResourceObjectList::new(
                    a.iter().map(Self::read_key).collect::<Result<Vec<_>>>()?,
                ))
                .into(),
            )));
        }
        let main = match property(el, "main")? {
            None | Some(JsonElement::Null) => None,
            Some(v) => Some(Self::read_key(v)?),
        };
        let relevant = match property(el, "relevant")? {
            Some(JsonElement::Array(a)) => {
                a.iter().map(Self::read_key).collect::<Result<Vec<_>>>()?
            }
            _ => vec![],
        };
        Ok(Rc::new(PopupSet::from_parts(
            main,
            (ResourceObjectList::new(relevant)).into(),
        )))
    }
    pub fn read_popup_mapping(
        json: &str,
        id: Utf16Text,
        extension_id: Utf16Text,
        authors: Option<ResourceTexts>,
    ) -> Result<PopupMapping> {
        let root = Self::parse(json)?;
        let mapping = PopupMapping::from_values(PopupMappingValues {
            id: Some(id),
            extension_id: Some(extension_id),
            authors: Some(authors.unwrap_or_default().into()),
            ..Default::default()
        });
        for entry in object(&root)? {
            let (name, value) = entry;
            let name = name.text()?;
            let variation = match name.as_str() {
                "all" => KeyVariation::All,
                "normal" => KeyVariation::Normal,
                "email" | "emailAddress" => KeyVariation::EmailAddress,
                "uri" => KeyVariation::Uri,
                "password" => KeyVariation::Password,
                _ => continue,
            };
            let map = ResourceDictionary::default();
            for entry in object(value)? {
                let (label, popup) = entry;
                let label = label.text()?;
                map.set(label.as_str().into(), Some(Self::read_popup_set(popup)?));
            }
            mapping
                .mappings()
                .as_ref()
                .unwrap()
                .as_concrete()
                .unwrap()
                .set(variation, Some(map.into()));
        }
        Ok(mapping)
    }
    pub fn read_subtype(json: &str) -> Result<Subtype> {
        Self::read_subtype_element(&Self::parse(json)?)
    }
    pub fn read_subtype_element(st: &JsonElement) -> Result<Subtype> {
        let mut map = SubtypeLayoutMapValues::default();
        let preferred = match property(st, "preferred")? {
            Some(v) => Some(v),
            None => property(st, "layouts")?,
        };
        if let Some(pref) = preferred {
            for entry in object(pref)? {
                let (key, value) = entry;
                let text = str_value(value)?
                    .ok_or_else(|| LayoutJsonError("Required layout data is missing".into()))?;
                let name = ComponentName::parse(&text, &SubtypeLayoutMap::CORE_LAYOUTS.into());
                let key = key.text()?;
                match key.as_str() {
                    "characters" => map.characters = name,
                    "symbols" => map.symbols = name,
                    "symbols2" => map.symbols2 = name,
                    "numeric" => map.numeric = name,
                    "numericAdvanced" => map.numeric_advanced = name,
                    "numericRow" => map.numeric_row = name,
                    "phone" => map.phone = name,
                    "phone2" => map.phone2 = name,
                    _ => {}
                }
            }
        }
        let mut subtype = SubtypeValues {
            language_tag: Some(string(st, "languageTag")?.unwrap_or_else(|| "en-US".into())),
            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(map))),
            display_name: string(st, "displayName")?,
            ..Default::default()
        };
        for (prop, ext, target) in [
            ("composer", Subtype::CORE_COMPOSERS, &mut subtype.composer),
            (
                "currencySet",
                Subtype::CORE_CURRENCY_SETS,
                &mut subtype.currency_set,
            ),
            (
                "popupMapping",
                Subtype::CORE_LOCALIZATION,
                &mut subtype.popup_mapping,
            ),
            (
                "punctuationRule",
                Subtype::CORE_LOCALIZATION,
                &mut subtype.punctuation_rule,
            ),
        ] {
            if let Some(s) = string(st, prop)? {
                *target = ComponentName::parse(&s, &ext.into());
            }
        }
        Ok(Subtype::from_values(subtype))
    }
}
fn scalar(cp: i32) -> Result<Utf16Text> {
    let c = u32::try_from(cp)
        .ok()
        .and_then(char::from_u32)
        .ok_or_else(|| LayoutJsonError(format!("Invalid Unicode scalar {cp}")))?;
    let mut units = [0; 2];
    Ok(Utf16Text::from_units(c.encode_utf16(&mut units).to_vec()))
}
fn text_value(text: &Utf16Text) -> Result<JsonElement> {
    Ok(JsonElement::String(utf8(text)?))
}
// Utf8JsonWriter.WriteString preserves null and replaces ill-formed UTF-16 with U+FFFD.
// This applies to serialized key labels; JSON input decoding remains strict/deferred as upstream.
fn nullable_text_value(text: Option<&Utf16Text>) -> JsonElement {
    text.map(|s| JsonElement::String(String::from_utf16_lossy(s.units())))
        .unwrap_or(JsonElement::Null)
}

fn add(out: &mut Vec<(String, JsonElement)>, name: &str, value: JsonElement) {
    out.push((name.into(), value));
}
fn child(out: &mut Vec<(String, JsonElement)>, name: &str, key: Option<&Key>) -> Result<()> {
    if let Some(key) = key {
        add(out, name, LayoutJson::write_key(key.as_ref())?);
    }
    Ok(())
}
fn common(out: &mut Vec<(String, JsonElement)>, key: &TextKeyData, write_code: bool) -> Result<()> {
    if write_code {
        add(out, "code", JsonElement::Number(key.code().into()));
        add(out, "label", nullable_text_value(key.label().as_ref()));
    }
    if key.key_type() != KeyType::Character {
        add(
            out,
            "type",
            JsonElement::String(key.key_type().to_json_string().into()),
        );
    }
    if key.group_id() != 0 {
        add(out, "groupId", JsonElement::Number(key.group_id().into()));
    }
    if let Some(popup) = key.popup()
        && !popup
            .is_empty()
            .map_err(|e| LayoutJsonError(e.to_string()))?
    {
        let mut props = Vec::new();
        child(&mut props, "main", popup.main().as_ref())?;
        let relevant = popup
            .relevant()
            .ok_or_else(|| LayoutJsonError("Required layout data is missing".into()))?;
        if relevant
            .count()
            .map_err(|e| LayoutJsonError(e.to_string()))?
            > 0
        {
            // Source rereads Relevant for foreach after the Count callback.
            let relevant = popup
                .relevant()
                .ok_or_else(|| LayoutJsonError("Required layout data is missing".into()))?;
            let mut values = Vec::new();
            relevant.visit::<LayoutJsonError>(|key| {
                let key =
                    key.ok_or_else(|| LayoutJsonError("Required layout data is missing".into()))?;
                values.push(LayoutJson::write_key(key.as_ref())?);
                Ok(())
            })?;
            add(&mut props, "relevant", JsonElement::Array(values));
        }
        add(out, "popup", JsonElement::Object(props));
    }
    Ok(())
}
impl LayoutJson {
    pub fn write_key(key: &dyn AbstractKeyData) -> Result<JsonElement> {
        use KeySerialization::*;
        let mut out = Vec::new();
        let view = key.serialization();
        let kind = match view {
            Text(_) => None,
            Auto(_) => Some("auto_text_key"),
            Multi(_) => Some("multi_text_key"),
            CaseSelector(_) => Some("case_selector"),
            ShiftStateSelector(_) => Some("shift_state_selector"),
            VariationSelector(_) => Some("variation_selector"),
            LayoutDirectionSelector(_) => Some("layout_direction_selector"),
            CharWidthSelector(_) => Some("char_width_selector"),
            KanaSelector(_) => Some("kana_selector"),
            Unsupported => return Err(LayoutJsonError("Unsupported key serialization".into())),
        };
        if let Some(kind) = kind {
            add(&mut out, "$", JsonElement::String(kind.into()));
        }
        match view {
            Text(k) => common(&mut out, k, true)?,
            Auto(k) => common(&mut out, &k.base, true)?,
            Multi(k) => {
                add(
                    &mut out,
                    "codePoints",
                    JsonElement::Array(
                        k.code_points()
                            .ok_or_else(|| {
                                LayoutJsonError("Required layout data is missing".into())
                            })?
                            .snapshot()
                            .iter()
                            .map(|cp| JsonElement::Number((*cp).into()))
                            .collect(),
                    ),
                );
                add(
                    &mut out,
                    "label",
                    nullable_text_value(k.base.label().as_ref()),
                );
                common(&mut out, &k.base, false)?;
            }
            CaseSelector(k) => {
                child(&mut out, "lower", k.lower().as_ref())?;
                child(&mut out, "upper", k.upper().as_ref())?;
            }
            ShiftStateSelector(k) => {
                child(&mut out, "unshifted", k.unshifted().as_ref())?;
                child(&mut out, "shifted", k.shifted().as_ref())?;
                child(&mut out, "shiftedManual", k.shifted_manual().as_ref())?;
                child(&mut out, "shiftedAutomatic", k.shifted_automatic().as_ref())?;
                child(&mut out, "capsLock", k.caps_lock().as_ref())?;
                child(&mut out, "default", k.default_key().as_ref())?;
            }
            VariationSelector(k) => {
                child(&mut out, "default", k.default_key().as_ref())?;
                child(&mut out, "email", k.email().as_ref())?;
                child(&mut out, "uri", k.uri().as_ref())?;
                child(&mut out, "normal", k.normal().as_ref())?;
                child(&mut out, "password", k.password().as_ref())?;
            }
            LayoutDirectionSelector(k) => {
                child(&mut out, "ltr", k.ltr().as_ref())?;
                child(&mut out, "rtl", k.rtl().as_ref())?;
            }
            CharWidthSelector(k) => {
                child(&mut out, "full", k.full().as_ref())?;
                child(&mut out, "half", k.half().as_ref())?;
            }
            KanaSelector(k) => {
                child(&mut out, "hira", k.hira().as_ref())?;
                child(&mut out, "kata", k.kata().as_ref())?;
            }
            Unsupported => return Err(LayoutJsonError("Unsupported key serialization".into())),
        }
        Ok(JsonElement::Object(out))
    }
    fn arrangement_value(arrangement: &LayoutArrangement) -> Result<JsonElement> {
        let mut result = Vec::new();
        arrangement
            .rows
            .as_ref()
            .ok_or(ResourceTextListError::NullReference)?
            .visit(|row| {
                let row = row.ok_or(ResourceTextListError::NullReference)?;
                let mut keys = Vec::new();
                row.visit(|key| {
                    let key =
                        key.ok_or_else(|| LayoutJsonError("Unsupported key serialization".into()))?;
                    keys.push(Self::write_key(key.as_ref())?);
                    Ok::<_, LayoutJsonError>(())
                })?;
                result.push(JsonElement::Array(keys));
                Ok::<_, LayoutJsonError>(())
            })?;
        Ok(JsonElement::Array(result))
    }
    fn json(value: &JsonElement, indented: bool) -> Result<String> {
        if indented {
            serde_json::to_string_pretty(value)
        } else {
            serde_json::to_string(value)
        }
        .map_err(|e| LayoutJsonError(e.to_string()))
    }
    pub fn write_arrangement(arrangement: &LayoutArrangement, indented: bool) -> Result<String> {
        Self::json(&Self::arrangement_value(arrangement)?, indented)
    }
    pub fn write_layout_document(layout: &KeyboardLayout, indented: bool) -> Result<String> {
        let mut out = vec![];
        add(
            &mut out,
            "type",
            JsonElement::String(layout.layout_type().to_id()?.into()),
        );
        add(
            &mut out,
            "id",
            layout
                .id()
                .as_ref()
                .map(text_value)
                .transpose()?
                .unwrap_or(JsonElement::Null),
        );
        add(
            &mut out,
            "label",
            layout
                .label()
                .as_ref()
                .map(text_value)
                .transpose()?
                .unwrap_or(JsonElement::Null),
        );
        add(
            &mut out,
            "direction",
            JsonElement::String(
                if layout.direction() == LayoutDirection::Rtl {
                    "rtl"
                } else {
                    "ltr"
                }
                .into(),
            ),
        );
        if let Some(modifier) = &layout.modifier() {
            add(&mut out, "modifier", text_value(&modifier.to_text())?);
        }
        let authors = layout
            .authors()
            .ok_or(ResourceTextListError::NullReference)?;
        if !authors.is_empty() {
            add(
                &mut out,
                "authors",
                JsonElement::Array(
                    authors
                        .iter()
                        .map(|value| {
                            value
                                .as_ref()
                                .map(text_value)
                                .transpose()
                                .map(|v| v.unwrap_or(JsonElement::Null))
                        })
                        .collect::<Result<Vec<_>>>()?,
                ),
            );
        }
        add(
            &mut out,
            "arrangement",
            Self::arrangement_value(
                layout
                    .arrangement()
                    .as_ref()
                    .ok_or(ResourceTextListError::NullReference)?,
            )?,
        );
        Self::json(&JsonElement::Object(out), indented)
    }
}
pub struct LayoutMetadata {
    pub layout_type: LayoutType,
    pub id: Option<Utf16Text>,
    pub label: Option<Utf16Text>,
    pub authors: Option<ResourceTexts>,
    pub direction: LayoutDirection,
    pub modifier: Option<ComponentName>,
}
impl LayoutMetadata {
    /// Native required-field construction; remaining defaults match the source initializers.
    /// This is not the missing CLR init-only/parameterless markup-construction contract.
    pub fn new(layout_type: LayoutType, id: Utf16Text) -> Self {
        Self {
            layout_type,
            id: Some(id),
            label: Some(Utf16Text::default()),
            authors: Some(ResourceTexts::default()),
            direction: LayoutDirection::Ltr,
            modifier: None,
        }
    }
}
// Source LayoutMetadata is a class, not a record: object equality is reference identity.
impl PartialEq for LayoutMetadata {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for LayoutMetadata {}
pub struct KeyboardExtensionManifest {
    pub id: Utf16Text,
    pub version: Utf16Text,
    pub title: Utf16Text,
    pub description: Utf16Text,
    pub license: Utf16Text,
    pub dependencies: ResourceTexts,
    pub layouts: SharedList<Arc<LayoutMetadata>>,
    pub popup_mappings: SharedList<(Option<Utf16Text>, ResourceTexts)>,
    pub subtype_presets: SharedList<Arc<Subtype>>,
    pub currency_sets: SharedList<Rc<CurrencySet>>,
    pub composers: SharedList<Arc<ComposerDefinition>>,
    pub punctuation_rules: SharedList<Arc<PunctuationRule>>,
}
impl LayoutJson {
    pub fn read_extension_manifest(json: &str) -> Result<KeyboardExtensionManifest> {
        let root = Self::parse(json)?;
        let meta =
            property(&root, "meta")?.ok_or_else(|| LayoutJsonError("Missing meta".into()))?;
        let id = string(meta, "id")?
            .ok_or_else(|| LayoutJsonError("Extension meta requires an 'id'.".into()))?;
        let mut layouts = Vec::new();
        if let Some(types) = property(&root, "layouts")? {
            for entry in object(types)? {
                let (name, items) = entry;
                let name = name.text()?;
                let Some(layout_type) = LayoutType::try_parse(&name) else {
                    continue;
                };
                for item in array(items)? {
                    let layout_id = string(item, "id")?;
                    layouts.push(Arc::new(LayoutMetadata {
                        layout_type,
                        label: string(item, "label")?.or_else(|| layout_id.clone()),
                        id: layout_id,
                        authors: Some(strings(item, "authors")?),
                        direction: direction(string(item, "direction")?),
                        modifier: string(item, "modifier")?.map(|m| ComponentName::parse(&m, &id)),
                    }));
                }
            }
        }
        let mut popup_mappings = Vec::new();
        if let Some(items) = property(&root, "popupMappings")? {
            for item in array(items)? {
                popup_mappings.push((string(item, "id")?, strings(item, "authors")?));
            }
        }
        let mut subtype_presets = Vec::new();
        if let Some(items) = property(&root, "subtypePresets")? {
            for item in array(items)? {
                subtype_presets.push(Arc::new(Self::read_subtype_element(item)?));
            }
        }
        let mut currency_sets = Vec::new();
        if let Some(items) = property(&root, "currencySets")? {
            for item in array(items)? {
                let values = property(item, "slots")?
                    .ok_or_else(|| LayoutJsonError("Missing slots".into()))?;
                let mut slots = Vec::new();
                for v in array(values)? {
                    slots.push(Self::read_key(v)?.into_text().ok_or_else(|| {
                        LayoutJsonError("Currency slot must be TextKeyData".into())
                    })?);
                }
                let currency_id = string(item, "id")?;
                currency_sets.push(Rc::new(CurrencySet::from_values(CurrencySetValues {
                    extension_id: Some(id.clone()),
                    id: currency_id,
                    label: Some(string(item, "label")?.unwrap_or_default()),
                    slots: Some(ResourceObjectList::new(slots).into()),
                })));
            }
        }
        let mut composers = Vec::new();
        if let Some(items) = property(&root, "composers")? {
            for item in array(items)? {
                let kind = string(item, "$")?.unwrap_or_else(|| "appender".into());
                let rules = ComposerRules::default();
                if let Some(values) = property(item, "rules")? {
                    for entry in object(values)? {
                        let (name, value) = entry;
                        let name = name.text()?;
                        rules
                            .write()
                            .insert(name.as_str().into(), str_value(value)?.unwrap_or_default());
                    }
                }
                composers.push(Arc::new(ComposerDefinition {
                    extension_id: id.clone(),
                    id: string(item, "id")?.unwrap_or_else(|| kind.clone()),
                    label: string(item, "label")?.unwrap_or_else(|| kind.clone()),
                    kind,
                    rules,
                }));
            }
        }
        let mut punctuation_rules = Vec::new();
        if let Some(items) = property(&root, "punctuationRules")? {
            for item in array(items)? {
                punctuation_rules.push(Arc::new(PunctuationRule::from_values(
                    PunctuationRuleValues {
                        extension_id: Some(id.clone()),
                        id: Some(string(item, "id")?.unwrap_or_else(|| "default".into())),
                        label: Some(string(item, "label")?.unwrap_or_else(|| "Default".into())),
                        symbols_preceding_auto_space: Some(
                            string(item, "symbolsPrecedingAutoSpace")?.unwrap_or_default(),
                        ),
                        symbols_following_auto_space: Some(
                            string(item, "symbolsFollowingAutoSpace")?.unwrap_or_default(),
                        ),
                        symbols_preceding_phantom_space: Some(
                            string(item, "symbolsPrecedingPhantomSpace")?.unwrap_or_default(),
                        ),
                        symbols_following_phantom_space: Some(
                            string(item, "symbolsFollowingPhantomSpace")?.unwrap_or_default(),
                        ),
                        symbols_terminating_sentence: Some(
                            string(item, "symbolsTerminatingSentence")?
                                .unwrap_or_else(|| ".?!".into()),
                        ),
                    },
                )));
            }
        }
        Ok(KeyboardExtensionManifest {
            version: string(meta, "version")?.unwrap_or_else(|| "0.0.0".into()),
            title: string(meta, "title")?.unwrap_or_else(|| id.clone()),
            description: string(meta, "description")?.unwrap_or_default(),
            license: string(meta, "license")?.unwrap_or_default(),
            dependencies: strings(&root, "dependencies")?,
            id,
            layouts: shared(layouts),
            popup_mappings: shared(popup_mappings),
            subtype_presets: shared(subtype_presets),
            currency_sets: shared(currency_sets),
            composers: shared(composers),
            punctuation_rules: shared(punctuation_rules),
        })
    }
}
