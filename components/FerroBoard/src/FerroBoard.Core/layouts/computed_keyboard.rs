// Derived from Layouts/ComputedKeyboard.cs; see ../NOTICE.md.
use super::*;
use crate::{keys::*, settings::*, text::Utf16Text};
use std::sync::Arc;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};
// Rust f64::max ignores NaN; System.Math.Max propagates it.
fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || a > b || a == b && !a.is_sign_negative() {
        a
    } else {
        b
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct KeyRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl std::hash::Hash for KeyRect {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for value in [self.x, self.y, self.width, self.height] {
            super::float_hash::hash_float(value, state);
        }
    }
}
impl KeyRect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn left(self) -> f64 {
        self.x
    }
    pub fn top(self) -> f64 {
        self.y
    }
    pub fn right(self) -> f64 {
        self.x + self.width
    }
    pub fn bottom(self) -> f64 {
        self.y + self.height
    }
    pub fn center_x(self) -> f64 {
        self.x + self.width / 2.0
    }
    pub fn center_y(self) -> f64 {
        self.y + self.height / 2.0
    }
    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
    pub fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }
    pub fn deflate(self, left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Self {
            x: self.x + left,
            y: self.y + top,
            width: max(0.0, self.width - left - right),
            height: max(0.0, self.height - top - bottom),
        }
    }
    pub fn center_distance_squared(self, x: f64, y: f64) -> f64 {
        let dx = self.center_x() - x;
        let dy = self.center_y() - y;
        dx * dx + dy * dy
    }
}
#[derive(Clone)]
struct KeyState {
    data: TextKey,
    label: Option<Utf16Text>,
    icon: KeyIcon,
    hint_label: Option<Utf16Text>,
    sub_label: Option<Utf16Text>,
    hint_key: Option<TextKey>,
    popup_keys: PopupKeys,
    popup_default_index: i32,
    is_visible: bool,
    is_enabled: bool,
    width_factor: f64,
    grow: f64,
    shrink: f64,
    touch_bounds: KeyRect,
    visible_bounds: KeyRect,
}
pub struct ComputedKey {
    source: Option<Key>,
    row: i32,
    column: i32,
    state: RefCell<KeyState>,
    pub(crate) symbol_hint_source: RefCell<Option<Key>>,
    pub(crate) number_hint_source: RefCell<Option<Key>>,
}
// Source ComputedKey is a class with inherited reference equality.
impl PartialEq for ComputedKey {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for ComputedKey {}
macro_rules! getters {($($name:ident:$t:ty),* $(,)?)=>{$(pub fn $name(&self)->$t {self.state.borrow().$name.clone()})*}}
impl ComputedKey {
    pub(crate) fn new(source: Option<Key>, row: i32, column: i32) -> Self {
        Self {
            source,
            row,
            column,
            state: RefCell::new(KeyState {
                data: TextKeyData::unspecified(),
                label: None,
                icon: KeyIcon::None,
                hint_label: None,
                sub_label: None,
                hint_key: None,
                popup_keys: PopupKeys::default(),
                popup_default_index: 0,
                is_visible: true,
                is_enabled: true,
                width_factor: 1.0,
                grow: 0.0,
                shrink: 1.0,
                touch_bounds: KeyRect::default(),
                visible_bounds: KeyRect::default(),
            }),
            symbol_hint_source: RefCell::new(None),
            number_hint_source: RefCell::new(None),
        }
    }
    pub fn source(&self) -> Option<Key> {
        self.source.clone()
    }
    pub fn row(&self) -> i32 {
        self.row
    }
    pub fn column(&self) -> i32 {
        self.column
    }
    getters! {data:TextKey,label:Option<Utf16Text>,icon:KeyIcon,hint_label:Option<Utf16Text>,sub_label:Option<Utf16Text>,hint_key:Option<TextKey>,popup_keys:PopupKeys,popup_default_index:i32,is_visible:bool,is_enabled:bool,width_factor:f64,grow:f64,shrink:f64,touch_bounds:KeyRect,visible_bounds:KeyRect}
    pub fn code(&self) -> i32 {
        self.data().data().code()
    }
    pub fn key_type(&self) -> KeyType {
        self.data().data().key_type()
    }
    pub fn has_popup(&self) -> bool {
        !self.popup_keys().is_empty()
    }
    pub fn is_character_key(&self) -> Result<bool, KeyDataError> {
        let d = self.data();
        Ok(d.produces_text()?
            && !d.data().is_space()
            && d.data().code() != KeyCode::Enter
            && d.data().code() != KeyCode::Tab)
    }
    pub fn output(&self) -> Result<Option<Utf16Text>, KeyDataError> {
        self.data().as_string(false)
    }
    /// UTF-16 diagnostic text with Rust numeric formatting and custom data dispatch.
    /// Labels retain their original units, including unpaired surrogates.
    pub fn to_text(&self) -> Result<Utf16Text, KeyDataError> {
        let mut result: Vec<u16> = "Key[".encode_utf16().collect();
        result.extend(self.row().to_string().encode_utf16());
        result.push(b',' as u16);
        result.extend(self.column().to_string().encode_utf16());
        result.extend("] ".encode_utf16());
        if let Some(label) = self.label() {
            result.extend_from_slice(label.units());
        } else {
            let icon = self.icon();
            result.extend(icon.to_string().encode_utf16());
        }
        result.extend(" (".encode_utf16());
        if let Some(data) = self.data().to_text()? {
            result.extend_from_slice(data.units());
        }
        result.push(b')' as u16);
        Ok(Utf16Text::from_units(result))
    }

    fn compute_nullable(
        &self,
        ctx: Option<&KeyComputeContext>,
        mapping: Option<&PopupMapping>,
        default_mapping: Option<&PopupMapping>,
    ) -> Result<(), KeyDataError> {
        let source = self
            .source
            .clone()
            .ok_or(ResourceTextListError::NullReference)?;
        let Some(data) = source
            .clone()
            .compute_nullable(ctx.map(|v| v as &dyn IKeyEvaluator))?
        else {
            let mut s = self.state.borrow_mut();
            s.data = TextKeyData::unspecified();
            s.is_visible = false;
            s.is_enabled = false;
            s.popup_keys = PopupKeys::default();
            s.label = None;
            s.hint_label = None;
            s.hint_key = None;
            s.icon = KeyIcon::None;
            return Ok(());
        };
        let data = Self::resolve_currency_nullable(data, ctx)?;
        let ctx = {
            let mut s = self.state.borrow_mut();
            s.data = data.clone();
            s.is_visible = Self::evaluate_visible_nullable(data.data(), ctx)?;
            s.is_enabled = data.data().key_type() != KeyType::Placeholder
                && !(data.data().code() == KeyCode::Unspecified
                    && data
                        .data()
                        .label()
                        .as_ref()
                        .ok_or(KeyDataError::NullReference)?
                        .is_empty());
            let ctx = ctx.ok_or(KeyDataError::NullReference)?;
            Self::compute_flay(&mut s, data.data(), ctx.mode());
            ctx
        };
        let (label, icon) = KeyLabels::get_display(data.as_ref(), ctx)?;
        {
            let mut s = self.state.borrow_mut();
            s.label = label;
            s.icon = icon;
            s.sub_label = if matches!(ctx.mode(), KeyboardMode::Phone | KeyboardMode::Phone2)
                && data.data().key_type() == KeyType::Numeric
            {
                KeyLabels::get_phone_sub_label(data.data().code())
            } else {
                None
            };
        }
        let mut accents = Vec::new();
        let mut main_accent = None;
        let lookup = source
            .compute(&ctx.unshifted())?
            .unwrap_or_else(|| data.clone());
        Self::collect_popups(
            data.data().popup().clone(),
            ctx,
            &mut accents,
            &mut main_accent,
        )?;
        let mapping_set = Self::find_mapping_set(
            &lookup.data().popup_lookup_label(),
            ctx.variation(),
            mapping,
            default_mapping,
        )?;
        if lookup.data().popup_lookup_label() != lookup.data().label()
            && !lookup
                .data()
                .label()
                .as_ref()
                .ok_or(KeyDataError::NullReference)?
                .is_empty()
        {
            Self::collect_popups(
                Self::find_mapping_set(
                    &lookup.data().label(),
                    ctx.variation(),
                    mapping,
                    default_mapping,
                )?,
                ctx,
                &mut accents,
                &mut main_accent,
            )?;
        }
        Self::collect_popups(mapping_set, ctx, &mut accents, &mut main_accent)?;
        let mut hint = None;
        let mut hint_popups = Vec::new();
        if ctx.hint_mode() != KeyHintMode::Disabled
            && data.data().key_type() == KeyType::Character
            && !data.data().is_space()
        {
            let unshifted = ctx.unshifted();
            if ctx.show_number_hints() {
                let source = self.number_hint_source.borrow().clone();
                if let Some(source) = source
                    && let Some(number) = source.compute(&unshifted)?
                    && number.data().key_type() == KeyType::Numeric
                    && number.data().code() != data.data().code()
                {
                    hint = Some(number);
                }
            }
            if hint.is_none() && ctx.show_symbol_hints() {
                let source = self.symbol_hint_source.borrow().clone();
                if let Some(source) = source
                    && let Some(symbol) = source.compute(&unshifted)?
                    && symbol.data().key_type() == KeyType::Character
                    && symbol.data().code() != data.data().code()
                {
                    hint = Some(Self::resolve_currency(symbol, ctx)?);
                }
            }
            if let Some(h) = &hint {
                let mut ignored = None;
                Self::collect_popups(
                    h.data().popup().clone(),
                    ctx,
                    &mut hint_popups,
                    &mut ignored,
                )?;
                if let Some(key) = &ignored {
                    hint_popups.insert(0, key.clone());
                }
                Self::collect_popups(
                    Self::find_mapping_set(
                        &h.data().label(),
                        KeyVariation::All,
                        mapping,
                        default_mapping,
                    )?,
                    ctx,
                    &mut hint_popups,
                    &mut ignored,
                )?;
            }
        }
        self.state.borrow_mut().hint_key = hint.clone();
        let hint_label = hint
            .as_ref()
            .map(|h| h.as_string(true))
            .transpose()?
            .flatten();
        self.state.borrow_mut().hint_label = hint_label;
        let mut keys = Vec::new();
        if hint.is_some() && ctx.hint_mode() == KeyHintMode::HintPriority {
            keys.extend(hint);
            keys.extend(main_accent);
        } else {
            keys.extend(main_accent);
            keys.extend(hint);
        }
        keys.extend(accents);
        keys.extend(hint_popups);
        let keys = Self::deduplicate(keys, &data)?;
        let index = if keys.is_empty() { -1 } else { 0 };
        let mut s = self.state.borrow_mut();
        s.popup_keys = PopupKeys::new(keys);
        s.popup_default_index = index;
        Ok(())
    }
    fn deduplicate(keys: Vec<TextKey>, this: &TextKey) -> Result<Vec<TextKey>, KeyDataError> {
        let mut seen = HashSet::from([Self::key_identity(this)?]);
        let mut result = Vec::with_capacity(keys.len());
        for k in keys {
            if k.data().key_type() == KeyType::Placeholder
                || k.data().code() == KeyCode::Unspecified
                    && k.data()
                        .label()
                        .as_ref()
                        .ok_or(KeyDataError::NullReference)?
                        .is_empty()
            {
                continue;
            }
            if seen.insert(Self::key_identity(&k)?) {
                result.push(k);
            }
        }
        Ok(result)
    }
    fn key_identity(k: &TextKey) -> Result<(bool, Utf16Text), KeyDataError> {
        Ok(if k.produces_text()? {
            (true, k.as_string(false)?.unwrap_or_default())
        } else {
            (false, k.data().code().to_string().as_str().into())
        })
    }
    fn find_mapping_set(
        label: &Option<Utf16Text>,
        variation: KeyVariation,
        mapping: Option<&PopupMapping>,
        default_mapping: Option<&PopupMapping>,
    ) -> Result<Option<Rc<PopupSet>>, KeyDataError> {
        let label = label.as_ref().ok_or(KeyDataError::NullReference)?;
        if label.is_empty() {
            return Ok(None);
        }
        let get = |v| -> Result<Option<Rc<PopupSet>>, KeyDataError> {
            if let Some(m) = mapping
                && let Some(value) = m.get(v, label)?
            {
                return Ok(Some(value));
            }
            match default_mapping {
                Some(m) => m.get(v, label),
                None => Ok(None),
            }
        };
        let mut set = get(variation)?;
        if set.is_none() && matches!(variation, KeyVariation::EmailAddress | KeyVariation::Uri) {
            set = get(KeyVariation::Uri)?;
        }
        if set.is_some() {
            Ok(set)
        } else {
            get(KeyVariation::All)
        }
    }
    fn collect_popups(
        set: Option<Rc<PopupSet>>,
        ctx: &KeyComputeContext,
        target: &mut Vec<TextKey>,
        main: &mut Option<TextKey>,
    ) -> Result<(), KeyDataError> {
        let Some(set) = set else {
            return Ok(());
        };
        if let Some(source) = set.main()
            && let Some(m) = source.clone().compute(ctx)?
        {
            let m = Self::resolve_currency(m, ctx)?;
            if main.is_none() {
                *main = Some(m)
            } else {
                target.push(m)
            }
        }
        let relevant = set.relevant().ok_or(ResourceTextListError::NullReference)?;
        relevant.visit::<KeyDataError>(|r| {
            let r = r.ok_or(ResourceTextListError::NullReference)?;
            if let Some(c) = r.compute(ctx)? {
                target.push(Self::resolve_currency(c, ctx)?);
            }
            Ok(())
        })?;
        Ok(())
    }
    pub(crate) fn resolve_currency(
        data: TextKey,
        ctx: &KeyComputeContext,
    ) -> Result<TextKey, KeyDataError> {
        Self::resolve_currency_nullable(data, Some(ctx))
    }
    fn resolve_currency_nullable(
        data: TextKey,
        ctx: Option<&KeyComputeContext>,
    ) -> Result<TextKey, KeyDataError> {
        if !KeyCode::is_currency_slot(data.data().code()) {
            return Ok(data);
        }
        let slot = match ctx
            .ok_or(KeyDataError::NullReference)?
            .currency_set()
            .as_ref()
            .map(|c| c.get_slot(data.data().code()))
            .transpose()?
            .flatten()
        {
            Some(s) => s,
            None => TextKeyData::char(
                ["$", "¢", "£", "€", "¥", "₩"]
                    [(KeyCode::CurrencySlot1 - data.data().code()).clamp(0, 5) as usize]
                    .into(),
                None,
            )?,
        };
        Ok(slot.with(KeyChanges {
            popup: data
                .data()
                .popup()
                .clone()
                .or_else(|| slot.data().popup().clone()),
            group_id: Some(data.data().group_id()),
            ..Default::default()
        }))
    }
    fn evaluate_visible_nullable(
        data: &TextKeyData,
        ctx: Option<&KeyComputeContext>,
    ) -> Result<bool, KeyDataError> {
        Ok(match data.code() {
            KeyCode::LanguageSwitch => {
                let ctx = ctx.ok_or(KeyDataError::NullReference)?;
                ctx.utility_key_action() == UtilityKeyAction::LanguageSwitch
                    || ctx.utility_key_action() == UtilityKeyAction::Dynamic
                        && ctx.has_multiple_subtypes()
            }
            KeyCode::ImeUiModeMedia => {
                let ctx = ctx.ok_or(KeyDataError::NullReference)?;
                ctx.utility_key_action() == UtilityKeyAction::Emoji
                    || ctx.utility_key_action() == UtilityKeyAction::Dynamic
                        && !ctx.has_multiple_subtypes()
            }
            _ => data.key_type() != KeyType::Placeholder,
        })
    }
    fn compute_flay(s: &mut KeyState, d: &TextKeyData, mode: KeyboardMode) {
        s.shrink = match mode {
            KeyboardMode::Numeric
            | KeyboardMode::NumericAdvanced
            | KeyboardMode::Phone
            | KeyboardMode::Phone2 => 1.0,
            _ => match d.code() {
                KeyCode::Shift | KeyCode::Delete => 1.5,
                KeyCode::ViewCharacters
                | KeyCode::ViewSymbols
                | KeyCode::ViewSymbols2
                | KeyCode::Enter => 0.0,
                _ => 1.0,
            },
        };
        s.grow = match mode {
            KeyboardMode::Numeric | KeyboardMode::Phone | KeyboardMode::Phone2 => 0.0,
            KeyboardMode::NumericAdvanced => {
                if d.key_type() == KeyType::Numeric {
                    1.0
                } else {
                    0.0
                }
            }
            _ => {
                if matches!(d.code(), KeyCode::Space | KeyCode::CjkSpace) {
                    1.0
                } else {
                    0.0
                }
            }
        };
        s.width_factor = match mode {
            KeyboardMode::Numeric | KeyboardMode::Phone | KeyboardMode::Phone2 => 2.68,
            KeyboardMode::NumericAdvanced => match d.code() {
                44 | 46 => 1.0,
                KeyCode::ViewSymbols | 61 => 1.26,
                _ => 1.56,
            },
            _ => match d.code() {
                KeyCode::Shift
                | KeyCode::Delete
                | KeyCode::ViewCharacters
                | KeyCode::ViewSymbols
                | KeyCode::ViewSymbols2
                | KeyCode::Enter => 1.56,
                _ => 1.0,
            },
        };
    }
}
pub type ComputedRow = ComputedList<Rc<ComputedKey>>;
pub type ComputedRows = ComputedList<ComputedRow>;

/// Fallible deferred traversal of the actual shared rows. Nullable keys remain
/// explicit values; no runtime Current/Reset/Dispose or panic transport is exposed.
pub struct ComputedKeys {
    rows: ComputedRows,
    visible_only: bool,
    outer: Option<ComputedItems<ComputedRow>>,
    inner: Option<ComputedItems<Rc<ComputedKey>>>,
    ended: bool,
}
impl ComputedKeys {
    fn new(rows: ComputedRows, visible_only: bool) -> Self {
        Self {
            rows,
            visible_only,
            outer: None,
            inner: None,
            ended: false,
        }
    }
    /// Start an independent deferred traversal of the same shared rows.
    pub fn iter(&self) -> Self {
        Self::new(self.rows.clone(), self.visible_only)
    }
    /// Engine operations require a non-null key; failures use Result directly.
    pub fn try_next(&mut self) -> Result<Option<Rc<ComputedKey>>, KeyDataError> {
        self.next()
            .transpose()?
            .map(|value| value.ok_or_else(|| ResourceTextListError::NullReference.into()))
            .transpose()
    }
}
impl Iterator for ComputedKeys {
    type Item = Result<Option<Rc<ComputedKey>>, KeyDataError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        if self.outer.is_none() {
            self.outer = Some(self.rows.iter_nullable());
        }
        loop {
            if let Some(inner) = &mut self.inner {
                match inner.next() {
                    Some(Ok(key)) => {
                        if !self.visible_only {
                            return Some(Ok(key));
                        }
                        let Some(key) = key else {
                            return Some(Err(ResourceTextListError::NullReference.into()));
                        };
                        if key.is_visible() {
                            return Some(Ok(Some(key)));
                        }
                        continue;
                    }
                    Some(Err(error)) => return Some(Err(error.into())),
                    None => self.inner = None,
                }
            }
            match self
                .outer
                .as_mut()
                .expect("initialized row iterator")
                .next()
            {
                Some(Ok(Some(row))) => self.inner = Some(row.iter_nullable()),
                Some(Ok(None)) => return Some(Err(ResourceTextListError::NullReference.into())),
                Some(Err(error)) => return Some(Err(error.into())),
                None => {
                    self.ended = true;
                    self.inner = None;
                    self.outer = None;
                    return None;
                }
            }
        }
    }
}
pub struct ComputedKeyboard {
    mode: KeyboardMode,
    subtype: Arc<Subtype>,
    direction: LayoutDirection,
    rows: ComputedRows,
    popup_mapping: Option<Rc<PopupMapping>>,
    default_popup_mapping: Option<Rc<PopupMapping>>,
    version: Cell<i32>,
    context: RefCell<Option<Rc<KeyComputeContext>>>,
    width: Cell<f64>,
    height: Cell<f64>,
}
impl PartialEq for ComputedKeyboard {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for ComputedKeyboard {}
impl ComputedKeyboard {
    pub(crate) fn new(
        mode: KeyboardMode,
        subtype: Arc<Subtype>,
        direction: LayoutDirection,
        rows: Vec<Vec<Rc<ComputedKey>>>,
        popup_mapping: Option<Rc<PopupMapping>>,
        default_popup_mapping: Option<Rc<PopupMapping>>,
    ) -> Self {
        Self {
            mode,
            subtype,
            direction,
            rows: ComputedRows::new(rows.into_iter().map(ComputedRow::new).collect()),
            popup_mapping,
            default_popup_mapping,
            version: Cell::new(0),
            context: RefCell::new(None),
            width: Cell::new(0.0),
            height: Cell::new(0.0),
        }
    }
    pub fn mode(&self) -> KeyboardMode {
        self.mode
    }
    pub fn subtype(&self) -> Arc<Subtype> {
        self.subtype.clone()
    }
    pub fn direction(&self) -> LayoutDirection {
        self.direction
    }
    pub fn rows(&self) -> ComputedRows {
        self.rows.clone()
    }
    pub fn popup_mapping(&self) -> Option<Rc<PopupMapping>> {
        self.popup_mapping.clone()
    }
    pub fn default_popup_mapping(&self) -> Option<Rc<PopupMapping>> {
        self.default_popup_mapping.clone()
    }
    pub fn keys(&self) -> ComputedKeys {
        ComputedKeys::new(self.rows.clone(), false)
    }
    pub fn visible_keys(&self) -> ComputedKeys {
        ComputedKeys::new(self.rows.clone(), true)
    }
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
    pub fn version(&self) -> i32 {
        self.version.get()
    }
    pub fn context(&self) -> Option<Rc<KeyComputeContext>> {
        self.context.borrow().clone()
    }
    pub fn width(&self) -> f64 {
        self.width.get()
    }
    pub fn height(&self) -> f64 {
        self.height.get()
    }
    pub fn compute(&self, context: Rc<KeyComputeContext>) -> Result<(), KeyDataError> {
        self.compute_nullable(Some(context))
    }
    /// Runtime-null source Compute argument. Keys may handle it without dereferencing it;
    /// failures preserve the original partially updated state and leave Version unchanged.
    pub fn compute_nullable(
        &self,
        context: Option<Rc<KeyComputeContext>>,
    ) -> Result<(), KeyDataError> {
        // Releasing the previous reference must not retain a storage borrow.
        let old = std::mem::replace(&mut *self.context.borrow_mut(), context.clone());
        drop(old);
        let mut keys = self.keys();
        while let Some(key) = keys.try_next()? {
            key.compute_nullable(
                context.as_deref(),
                self.popup_mapping.as_deref(),
                self.default_popup_mapping.as_deref(),
            )?;
        }
        self.version.set(self.version.get().wrapping_add(1));
        Ok(())
    }
    pub fn layout(
        &self,
        width: f64,
        height: f64,
        options: Option<&KeyboardGeometryOptions>,
    ) -> Result<(), KeyDataError> {
        let default = KeyboardGeometryOptions::default();
        let options = options.unwrap_or(&default);
        self.width.set(width);
        self.height.set(height);
        let rows: Vec<Vec<_>> = self
            .rows
            .iter_nullable()
            .map(|row| {
                let row = row?.ok_or(ResourceTextListError::ArgumentNull)?;
                row.iter_nullable()
                    .map(|key| {
                        key?.ok_or_else(|| KeyDataError::from(ResourceTextListError::NullReference))
                    })
                    .filter(|key| match key {
                        Ok(key) => key.is_visible(),
                        Err(_) => true,
                    })
                    .collect::<Result<Vec<_>, KeyDataError>>()
            })
            .collect::<Result<Vec<_>, KeyDataError>>()?
            .into_iter()
            .filter(|row| !row.is_empty())
            .collect();
        if rows.is_empty() || width <= 0.0 || height <= 0.0 {
            return Ok(());
        }
        let split = options.split_gap() > 0.0 && options.split_gap() < width * 0.5;
        let layout_width = if split {
            width - options.split_gap()
        } else {
            width
        };
        let desired_w = layout_width / options.units_per_row();
        let row_h = height / rows.len() as f64;
        let sh = options.key_spacing_horizontal() / 2.0;
        let sv = options.key_spacing_vertical() / 2.0;
        let mid = layout_width / 2.0;
        for (r, row) in rows.iter().enumerate() {
            let pos_y = r as f64 * row_h;
            let available = layout_width / desired_w;
            let (mut requested, mut shrink_sum, mut grow_sum) = (0.0, 0.0, 0.0);
            for key in row {
                requested += key.width_factor();
                shrink_sum += key.shrink();
                grow_sum += key.grow();
            }
            let mut pos_x = 0.0;
            if requested <= available {
                let additional = available - requested;
                for (k, key) in row.iter().enumerate() {
                    let first = k == 0;
                    let last = k == row.len() - 1;
                    let units = if grow_sum == 0.0 {
                        if row.len() == 1 {
                            key.width_factor() + additional
                        } else if first || last {
                            key.width_factor() + additional / 2.0
                        } else {
                            key.width_factor()
                        }
                    } else {
                        key.width_factor() + additional * (key.grow() / grow_sum)
                    };
                    let key_w = desired_w * units;
                    let touch = KeyRect {
                        x: pos_x,
                        y: pos_y,
                        width: key_w,
                        height: row_h,
                    };
                    let left = if grow_sum == 0.0 && first {
                        additional / 2.0 * desired_w
                    } else {
                        0.0
                    };
                    let right = if grow_sum == 0.0 && last {
                        additional / 2.0 * desired_w
                    } else {
                        0.0
                    };
                    let mut s = key.state.borrow_mut();
                    s.touch_bounds = touch;
                    s.visible_bounds = touch.deflate(sh + left, sv, sh + right, sv);
                    pos_x += key_w;
                }
            } else {
                let clipping = requested - available;
                for key in row {
                    let units = if key.shrink() == 0.0 || shrink_sum == 0.0 {
                        key.width_factor()
                    } else {
                        key.width_factor() - clipping * (key.shrink() / shrink_sum)
                    };
                    let key_w = desired_w * units;
                    let touch = KeyRect {
                        x: pos_x,
                        y: pos_y,
                        width: key_w,
                        height: row_h,
                    };
                    let mut s = key.state.borrow_mut();
                    s.touch_bounds = touch;
                    s.visible_bounds = touch.deflate(sh, sv, sh, sv);
                    pos_x += key_w;
                }
            }
            if split {
                Self::apply_split(row, mid, options.split_gap(), layout_width, desired_w);
            }
        }
        self.version.set(self.version.get().wrapping_add(1));
        Ok(())
    }
    fn apply_split(
        row: &[Rc<ComputedKey>],
        mid: f64,
        gap: f64,
        layout_width: f64,
        unit_width: f64,
    ) {
        let mut left = Vec::new();
        let mut right = Vec::new();
        for key in row {
            let t = key.touch_bounds();
            if t.width > unit_width * 3.0 && t.left() < mid && t.right() > mid {
                let mut s = key.state.borrow_mut();
                s.touch_bounds.width += gap;
                s.visible_bounds.width += gap;
                continue;
            }
            if t.center_x() <= mid + 0.01 {
                left.push(key)
            } else {
                right.push(key)
            }
        }
        if !left.is_empty() {
            let end = left
                .iter()
                .map(|k| k.touch_bounds().right())
                .fold(f64::NEG_INFINITY, max);
            if end > mid + 0.5 {
                let scale = mid / end;
                for key in left {
                    let mut s = key.state.borrow_mut();
                    s.touch_bounds = Self::scale(s.touch_bounds, 0.0, scale, 0.0);
                    s.visible_bounds = Self::scale(s.visible_bounds, 0.0, scale, 0.0);
                }
            }
        }
        if !right.is_empty() {
            let start =
                right
                    .iter()
                    .map(|k| k.touch_bounds().left())
                    .fold(
                        f64::INFINITY,
                        |a, b| if a.is_nan() || a < b { a } else { b },
                    );
            let target = max(start, mid) + gap;
            let scale = if start < mid - 0.5 {
                (layout_width - mid) / (layout_width - start)
            } else {
                1.0
            };
            for key in right {
                let mut s = key.state.borrow_mut();
                s.touch_bounds = Self::scale(s.touch_bounds, start, scale, target);
                s.visible_bounds = Self::scale(s.visible_bounds, start, scale, target);
            }
        }
    }
    fn scale(r: KeyRect, origin: f64, scale: f64, target: f64) -> KeyRect {
        KeyRect {
            x: target + (r.x - origin) * scale,
            width: r.width * scale,
            ..r
        }
    }
    pub fn hit_test(&self, x: f64, y: f64) -> Result<Option<Rc<ComputedKey>>, KeyDataError> {
        let mut best = None;
        let mut distance = f64::MAX;
        let mut keys = self.keys();
        while let Some(key) = keys.try_next()? {
            if !key.is_visible() || !key.is_enabled() {
                continue;
            }
            let bounds = key.touch_bounds();
            if bounds.contains(x, y) {
                return Ok(Some(key));
            }
            if y >= bounds.top() && y < bounds.bottom() {
                let d = bounds.center_distance_squared(x, y);
                if d < distance {
                    distance = d;
                    best = Some(key);
                }
            }
        }
        Ok(best)
    }
    pub fn find_key(&self, code: i32) -> Result<Option<Rc<ComputedKey>>, KeyDataError> {
        let mut keys = self.visible_keys();
        while let Some(key) = keys.try_next()? {
            if key.code() == code {
                return Ok(Some(key));
            }
        }
        Ok(None)
    }
    pub fn character_keys(&self) -> Result<Vec<Rc<ComputedKey>>, KeyDataError> {
        let mut result = Vec::new();
        let mut keys = self.visible_keys();
        while let Some(key) = keys.try_next()? {
            if key.is_character_key()? && key.key_type() == KeyType::Character {
                result.push(key);
            }
        }
        Ok(result)
    }
}

fn double_equals(a: f64, b: f64) -> bool {
    a == b || a.is_nan() && b.is_nan()
}
impl PartialEq for KeyRect {
    fn eq(&self, other: &Self) -> bool {
        double_equals(self.x, other.x)
            && double_equals(self.y, other.y)
            && double_equals(self.width, other.width)
            && double_equals(self.height, other.height)
    }
}
impl Eq for KeyRect {}

#[cfg(test)]
#[path = "computed_key_text_tests.rs"]
mod computed_key_text_tests;
