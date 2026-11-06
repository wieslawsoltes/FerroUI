// Derived from Layouts/LayoutComputer.cs; see ../NOTICE.md.
use super::*;
use crate::keys::{KeyboardMode, LayoutDirection};
use std::sync::Arc;
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsupportedKeyboardMode(pub KeyboardMode);
impl UnsupportedKeyboardMode {
    pub const fn parameter_name(&self) -> &'static str {
        "mode"
    }
}
impl std::fmt::Display for UnsupportedKeyboardMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported keyboard mode: {}", self.0.0)
    }
}
impl std::error::Error for UnsupportedKeyboardMode {}
pub struct LayoutComputer {
    resources: Option<Rc<KeyboardResources>>,
    default_modifier: RefCell<ComponentName>,
}
impl PartialEq for LayoutComputer {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for LayoutComputer {}
impl LayoutComputer {
    pub fn new(resources: Rc<KeyboardResources>) -> Self {
        Self::new_nullable(Some(resources))
    }
    /// Source constructor: a runtime null registry is retained until an operation dereferences it.
    pub fn new_nullable(resources: Option<Rc<KeyboardResources>>) -> Self {
        Self {
            resources,
            default_modifier: RefCell::new(ComponentName::new(
                KeyboardResources::CORE_LAYOUTS_EXTENSION,
                "default",
            )),
        }
    }
    /// Nonnull convenience; the native source property uses resources_nullable.
    pub fn resources(&self) -> Rc<KeyboardResources> {
        self.resources_nullable()
            .unwrap_or_else(|| std::panic::panic_any(ResourceError::NullReference))
    }
    pub fn resources_nullable(&self) -> Option<Rc<KeyboardResources>> {
        self.resources.clone()
    }
    fn required_resources(&self) -> ResourceResult<&KeyboardResources> {
        self.resources
            .as_deref()
            .ok_or(ResourceError::NullReference)
    }
    pub fn default_modifier(&self) -> ComponentName {
        self.default_modifier.borrow().clone()
    }
    pub fn set_default_modifier(&self, value: ComponentName) {
        *self.default_modifier.borrow_mut() = value;
    }
    pub fn compute(
        &self,
        mode: KeyboardMode,
        subtype: Arc<Subtype>,
        number_row: bool,
    ) -> ResourceResult<ComputedKeyboard> {
        self.compute_nullable(mode, Some(subtype), number_row)
    }
    /// Preserve null-subtype failure before mode dispatch. Null Subtype.Layouts is still a separate model gap.
    pub fn compute_nullable(
        &self,
        mode: KeyboardMode,
        subtype: Option<Arc<Subtype>>,
        number_row: bool,
    ) -> ResourceResult<ComputedKeyboard> {
        let subtype = subtype.ok_or(ResourceError::NullReference)?;
        let mut modifier = None;
        let mut extension = None;
        let map = subtype.layouts();
        let get = |t, name: &ComponentName| self.required_resources()?.get_layout(t, name);
        let main = match mode {
            KeyboardMode::Characters => {
                if number_row {
                    extension = get(
                        LayoutType::NumericRow,
                        &map.as_ref()
                            .ok_or(ResourceError::NullReference)?
                            .numeric_row(),
                    )?;
                }
                let main = match get(
                    LayoutType::Characters,
                    &map.as_ref()
                        .ok_or(ResourceError::NullReference)?
                        .characters(),
                )? {
                    Some(m) => Some(m),
                    None => get(
                        LayoutType::Characters,
                        &ComponentName::new(KeyboardResources::CORE_LAYOUTS_EXTENSION, "qwerty"),
                    )?,
                };
                modifier = self.resolve_modifier(main.as_deref(), LayoutType::CharactersMod)?;
                main
            }
            KeyboardMode::Symbols => {
                extension = get(
                    LayoutType::NumericRow,
                    &map.as_ref()
                        .ok_or(ResourceError::NullReference)?
                        .numeric_row(),
                )?;
                let main = get(
                    LayoutType::Symbols,
                    &map.as_ref().ok_or(ResourceError::NullReference)?.symbols(),
                )?;
                modifier = self.resolve_modifier(main.as_deref(), LayoutType::SymbolsMod)?;
                main
            }
            KeyboardMode::Symbols2 => {
                let main = get(
                    LayoutType::Symbols2,
                    &map.as_ref().ok_or(ResourceError::NullReference)?.symbols2(),
                )?;
                modifier = self.resolve_modifier(main.as_deref(), LayoutType::Symbols2Mod)?;
                main
            }
            KeyboardMode::Numeric => get(
                LayoutType::Numeric,
                &map.as_ref().ok_or(ResourceError::NullReference)?.numeric(),
            )?,
            KeyboardMode::NumericAdvanced => get(
                LayoutType::NumericAdvanced,
                &map.as_ref()
                    .ok_or(ResourceError::NullReference)?
                    .numeric_advanced(),
            )?,
            KeyboardMode::Phone => get(
                LayoutType::Phone,
                &map.as_ref().ok_or(ResourceError::NullReference)?.phone(),
            )?,
            KeyboardMode::Phone2 => get(
                LayoutType::Phone2,
                &map.as_ref().ok_or(ResourceError::NullReference)?.phone2(),
            )?,
            _ => return Err(UnsupportedKeyboardMode(mode).into()),
        };
        let arrangement = Self::merge(main.as_deref(), modifier.as_deref(), extension.as_deref())?;
        let mut rows = Vec::with_capacity(arrangement.len());
        for (r, source_row) in arrangement.iter().enumerate() {
            let source_row = source_row
                .as_ref()
                .ok_or(ResourceTextListError::NullReference)?;
            let capacity = source_row.count()?;
            if capacity < 0 {
                return Err(ResourceTextListError::IndexOutOfRange.into());
            }
            let mut row = Vec::with_capacity(capacity as usize);
            let mut c = 0;
            while c < source_row.count()? {
                row.push(Rc::new(ComputedKey::new(source_row.get(c)?, r as i32, c)));
                c += 1;
            }
            rows.push(row);
        }
        if mode == KeyboardMode::Characters && !rows.is_empty() {
            self.add_hints(&rows, &subtype)?;
        }
        let mapping = self
            .required_resources()?
            .get_popup_mapping(&subtype.popup_mapping())?;
        let default_mapping = self
            .required_resources()?
            .get_popup_mapping(&ComponentName::new(Subtype::CORE_LOCALIZATION, "default"))?;
        Ok(ComputedKeyboard::new(
            mode,
            subtype,
            main.map_or(LayoutDirection::Ltr, |m| m.direction()),
            rows,
            mapping,
            default_mapping,
        ))
    }
    fn resolve_modifier(
        &self,
        main: Option<&KeyboardLayout>,
        mod_type: LayoutType,
    ) -> ResourceResult<Option<Rc<KeyboardLayout>>> {
        if let Some(modifier) = main.and_then(|m| m.modifier())
            && let Some(custom) = self.required_resources()?.get_layout(mod_type, &modifier)?
        {
            return Ok(Some(custom));
        }
        let name = self.default_modifier();
        self.required_resources()?.get_layout(mod_type, &name)
    }
    pub(crate) fn merge(
        main: Option<&KeyboardLayout>,
        modifier: Option<&KeyboardLayout>,
        extension: Option<&KeyboardLayout>,
    ) -> Result<Vec<Option<ArrangementRow>>, ResourceTextListError> {
        let mut result = Vec::new();
        if let Some(e) = extension {
            e.arrangement()
                .as_ref()
                .ok_or(ResourceTextListError::NullReference)?
                .rows
                .as_ref()
                .ok_or(ResourceTextListError::ArgumentNull)?
                .append_to(&mut result)?;
        }
        let main_rows = main
            .map(|m| {
                m.arrangement()
                    .as_ref()
                    .ok_or(ResourceTextListError::NullReference)
                    .map(|a| a.rows.clone())
            })
            .transpose()?
            .flatten()
            .unwrap_or_default();
        let mod_rows = modifier
            .map(|m| {
                m.arrangement()
                    .as_ref()
                    .ok_or(ResourceTextListError::NullReference)
                    .map(|a| a.rows.clone())
            })
            .transpose()?
            .flatten()
            .unwrap_or_default();
        if main_rows.count()? > 0 && mod_rows.count()? > 0 {
            let mut i = 0;
            while i < main_rows.count()? {
                if i + 1 < main_rows.count()? {
                    result.push(main_rows.get(i)?);
                    i += 1;
                    continue;
                }
                let mut merged = Vec::new();
                let mut replaced = false;
                mod_rows
                    .get(0)?
                    .ok_or(ResourceTextListError::NullReference)?
                    .visit(|key| {
                        if key
                            .as_ref()
                            .and_then(|k| k.clone().into_text())
                            .is_some_and(|k| k.data().code() == 0)
                            && !replaced
                        {
                            main_rows
                                .get(i)?
                                .ok_or(ResourceTextListError::ArgumentNull)?
                                .append_to(&mut merged)?;
                            replaced = true;
                        } else {
                            merged.push(key);
                        }
                        Ok::<_, ResourceTextListError>(())
                    })?;
                result.push(Some(ArrangementRow::from_nullable(
                    ResourceObjectList::new(merged),
                )));
                i += 1;
            }
            let mut i = 1;
            while i < mod_rows.count()? {
                result.push(mod_rows.get(i)?);
                i += 1;
            }
        } else if main_rows.count()? > 0 {
            main_rows.append_to(&mut result)?;
        } else {
            mod_rows.append_to(&mut result)?;
        }
        Ok(result)
    }
    fn add_hints(&self, rows: &[Vec<Rc<ComputedKey>>], subtype: &Subtype) -> ResourceResult<()> {
        // Preserve the source's two independent Symbols lookups and their order.
        let main = self.required_resources()?.get_layout(
            LayoutType::Symbols,
            &subtype
                .layouts()
                .ok_or(ResourceError::NullReference)?
                .symbols(),
        )?;
        let modifier = self.resolve_modifier(
            self.required_resources()?
                .get_layout(
                    LayoutType::Symbols,
                    &subtype
                        .layouts()
                        .ok_or(ResourceError::NullReference)?
                        .symbols(),
                )?
                .as_deref(),
            LayoutType::SymbolsMod,
        )?;
        let extension = self.required_resources()?.get_layout(
            LayoutType::NumericRow,
            &subtype
                .layouts()
                .ok_or(ResourceError::NullReference)?
                .numeric_row(),
        )?;
        let symbols = Self::merge(main.as_deref(), modifier.as_deref(), extension.as_deref())?;
        if symbols.is_empty() {
            return Ok(());
        }
        let mut k = 0;
        while k < rows[0].len()
            && (k as i32)
                < symbols[0]
                    .as_ref()
                    .ok_or(ResourceTextListError::NullReference)?
                    .count()?
        {
            *rows[0][k].number_hint_source.borrow_mut() = symbols[0]
                .as_ref()
                .ok_or(ResourceTextListError::NullReference)?
                .get(k as i32)?;
            k += 1;
        }
        let offset = rows.len() as isize - symbols.len() as isize;
        for (r, row) in rows.iter().enumerate() {
            let sr = r as isize - offset;
            if sr < 0 || sr >= symbols.len() as isize {
                continue;
            }
            let symbol_row = symbols[sr as usize].as_ref();
            let mut k = 0;
            while k < row.len()
                && (k as i32)
                    < symbol_row
                        .ok_or(ResourceTextListError::NullReference)?
                        .count()?
            {
                let hint = symbol_row
                    .ok_or(ResourceTextListError::NullReference)?
                    .get(k as i32)?;
                *row[k].symbol_hint_source.borrow_mut() = hint;
                k += 1;
            }
        }
        Ok(())
    }
}
