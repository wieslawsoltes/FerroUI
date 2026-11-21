use super::current;
pub use super::registry::RibbonLocalizationKeys;
use super::{
    formatting::*,
    registry::{Ordered, Values, registry},
};
use crate::{
    RibbonString,
    model::RibbonItemsSource,
    text::{empty, ordinal::OrdinalKey},
};
use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonLocalizationErrorKind {
    ArgumentNull,
    Argument,
    Format,
    InvalidOperation,
    Unavailable,
}
#[derive(Clone, Debug)]
pub struct RibbonLocalizationError {
    pub kind: RibbonLocalizationErrorKind,
    pub parameter: Option<&'static str>,
    pub message: String,
}
impl RibbonLocalizationError {
    pub fn new(
        kind: RibbonLocalizationErrorKind,
        parameter: Option<&'static str>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            parameter,
            message: message.into(),
        }
    }

    pub(super) fn null(parameter: &'static str) -> Self {
        Self::new(
            RibbonLocalizationErrorKind::ArgumentNull,
            Some(parameter),
            format!("Value cannot be null. ({parameter})"),
        )
    }
    pub(super) fn required(
        value: Option<RibbonString>,
        parameter: &'static str,
    ) -> Result<RibbonString, Self> {
        let value = value.ok_or_else(|| Self::null(parameter))?;
        if value.is_empty() {
            Err(Self::new(
                RibbonLocalizationErrorKind::Argument,
                Some(parameter),
                "The value cannot be an empty string.",
            ))
        } else {
            Ok(value)
        }
    }
}
impl std::fmt::Display for RibbonLocalizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for RibbonLocalizationError {}
/// Core boundary for the actual culture object. Native bridges use framework
/// CultureInfo; explicit implementations retain virtual property/format dispatch.
pub trait RibbonCultureInfo: Any + Send + Sync {
    fn name(&self) -> Option<RibbonString>;
    fn two_letter_iso_language_name(&self) -> Option<RibbonString>;
    fn custom_formatter(&self) -> Option<Rc<RibbonCustomFormatter>> {
        None
    }
    fn format_numeric(
        &self,
        _value: &RibbonFormatArgument,
        _format: Option<&RibbonString>,
    ) -> Result<Option<RibbonString>, RibbonLocalizationError> {
        Err(RibbonLocalizationError::new(
            RibbonLocalizationErrorKind::Unavailable,
            None,
            "Faithful numeric culture formatting is not supplied",
        ))
    }
}
/// Explicit named data; this does not pretend to implement CultureInfo name
/// parsing/validation/canonicalization. The native bridge provides that service.
pub struct RibbonCulture {
    pub name: RibbonString,
    pub two_letter_iso_language_name: RibbonString,
    invariant: bool,
}
impl RibbonCulture {
    pub fn new(name: RibbonString, two_letter_iso_language_name: RibbonString) -> Arc<Self> {
        let invariant = name.is_empty();
        Arc::new(Self {
            name,
            two_letter_iso_language_name,
            invariant,
        })
    }
    pub fn invariant() -> Arc<Self> {
        Arc::new(Self {
            name: empty(),
            two_letter_iso_language_name: "iv".into(),
            invariant: true,
        })
    }
}
impl RibbonCultureInfo for RibbonCulture {
    fn name(&self) -> Option<RibbonString> {
        Some(self.name.clone())
    }
    fn two_letter_iso_language_name(&self) -> Option<RibbonString> {
        Some(self.two_letter_iso_language_name.clone())
    }
    fn format_numeric(
        &self,
        value: &RibbonFormatArgument,
        format: Option<&RibbonString>,
    ) -> Result<Option<RibbonString>, RibbonLocalizationError> {
        basic_numeric(value, format, self.invariant)
    }
}
pub type RibbonCurrentUICultureProvider = dyn Fn() -> Arc<dyn RibbonCultureInfo>;
ferroui_system::ferro_thread_local! {
    static LITERALS:RefCell<HashMap<&'static str,RibbonString>>=RefCell::default();
    static UI_CULTURE:RefCell<Option<Rc<RibbonCurrentUICultureProvider>>>=RefCell::default();
}
pub(super) fn literal(value: &'static str) -> RibbonString {
    if value.is_empty() {
        return empty();
    }
    LITERALS.with(|cache| {
        cache
            .borrow_mut()
            .entry(value)
            .or_insert_with(|| value.into())
            .clone()
    })
}
fn ensure() -> Result<(), RibbonLocalizationError> {
    if current::read().is_some() {
        return Ok(());
    }
    let _initialization = registry().gate.enter();
    if current::read().is_none() {
        let provider = UI_CULTURE.with(|s| s.borrow().clone());
        let culture = provider.map_or_else(
            || RibbonCulture::invariant() as Arc<dyn RibbonCultureInfo>,
            |p| p(),
        );
        let value = for_culture(culture)?;
        drop(current::replace(value));
    }
    Ok(())
}
fn for_culture(
    culture: Arc<dyn RibbonCultureInfo>,
) -> Result<Arc<RibbonStrings>, RibbonLocalizationError> {
    let _transaction = registry().gate.enter();
    let name = culture
        .name()
        .ok_or_else(|| RibbonLocalizationError::null("key"))?;
    let values = registry().cultures.get(&OrdinalKey(name));
    let values = if let Some(values) = values {
        values
    } else {
        let iso = culture
            .two_letter_iso_language_name()
            .ok_or_else(|| RibbonLocalizationError::null("key"))?;
        registry()
            .cultures
            .get(&OrdinalKey(iso))
            .unwrap_or_else(|| registry().cultures.get(&OrdinalKey(literal("en"))).unwrap())
    };
    let copy = Ordered::new();
    for (k, v) in values.snapshot() {
        copy.insert(k, v);
    }
    Ok(Arc::new(RibbonStrings {
        values: copy,
        overrides: Ordered::new(),
        culture,
    }))
}
pub struct RibbonStrings {
    values: Values,
    overrides: Values,
    culture: Arc<dyn RibbonCultureInfo>,
}
impl RibbonStrings {
    pub fn set_current_ui_culture_provider(
        provider: Option<Rc<RibbonCurrentUICultureProvider>>,
    ) -> Option<Rc<RibbonCurrentUICultureProvider>> {
        UI_CULTURE.with(|s| s.replace(provider))
    }
    pub fn current() -> Arc<Self> {
        ensure().expect("Current initialization failed");
        current::read().expect("initialized Current")
    }
    pub fn set_current(value: Option<Arc<Self>>) -> Result<(), RibbonLocalizationError> {
        ensure()?;
        let value = value.ok_or_else(|| RibbonLocalizationError::null("value"))?;
        let old = current::replace(value);
        drop(old);
        Self::current_changed().raise();
        Ok(())
    }
    pub fn current_changed() -> Arc<super::RibbonStringsChanged> {
        ensure().expect("Current initialization failed");
        current::current_state().changed.clone()
    }
    pub fn culture(&self) -> Arc<dyn RibbonCultureInfo> {
        self.culture.clone()
    }
    pub fn built_in_cultures() -> Arc<RibbonLocalizationKeys> {
        ensure().expect("Current initialization failed");
        registry().culture_keys.clone()
    }
    pub fn keys() -> Arc<RibbonLocalizationKeys> {
        ensure().expect("Current initialization failed");
        registry().english_keys.clone()
    }
    pub fn for_culture(
        culture: Option<Arc<dyn RibbonCultureInfo>>,
    ) -> Result<Arc<Self>, RibbonLocalizationError> {
        ensure()?;
        for_culture(culture.ok_or_else(|| RibbonLocalizationError::null("culture"))?)
    }
    pub fn get(
        &self,
        key: Option<&RibbonString>,
    ) -> Result<Option<RibbonString>, RibbonLocalizationError> {
        let key = key.ok_or_else(|| RibbonLocalizationError::null("key"))?;
        if let Some(value) = self.values.get(key) {
            return Ok(value);
        }
        Ok(registry()
            .cultures
            .get(&OrdinalKey(literal("en")))
            .unwrap()
            .get(key)
            .unwrap_or_else(|| Some(key.clone())))
    }
    pub fn register(
        culture_name: Option<RibbonString>,
        values: Option<RibbonItemsSource<(Option<RibbonString>, Option<RibbonString>)>>,
    ) -> Result<(), RibbonLocalizationError> {
        ensure()?;
        let name = RibbonLocalizationError::required(culture_name, "cultureName")?;
        let values = values.ok_or_else(|| RibbonLocalizationError::null("values"))?;
        {
            let _transaction = registry().gate.enter();
            let existing = {
                let key = OrdinalKey(name.clone());
                if let Some(existing) = registry().cultures.get(&key) {
                    existing
                } else {
                    let existing = Ordered::new();
                    registry().cultures.insert(key, existing.clone());
                    existing
                }
            };
            for (key, value) in values {
                existing.insert(
                    key.ok_or_else(|| RibbonLocalizationError::null("key"))?,
                    value,
                );
            }
        }
        let current = Self::current();
        let matches = current
            .culture
            .name()
            .is_some_and(|n| OrdinalKey(n) == OrdinalKey(name.clone()))
            || current
                .culture
                .two_letter_iso_language_name()
                .is_some_and(|n| OrdinalKey(n) == OrdinalKey(name));
        if matches {
            let refreshed = Self::for_culture(Some(current.culture()))?;
            for (key, value) in current.overrides.cursor() {
                refreshed.override_string(Some(key), value)?;
            }
            Self::set_current(Some(refreshed))?;
        }
        Ok(())
    }
    pub fn override_string(
        self: &Arc<Self>,
        key: Option<RibbonString>,
        value: Option<RibbonString>,
    ) -> Result<Arc<Self>, RibbonLocalizationError> {
        let key = RibbonLocalizationError::required(key, "key")?;
        let value = value.ok_or_else(|| RibbonLocalizationError::null("value"))?;
        self.values.insert(key.clone(), Some(value.clone()));
        self.overrides.insert(key, Some(value));
        if Arc::ptr_eq(self, &Self::current()) {
            Self::current_changed().raise();
        }
        Ok(self.clone())
    }
    pub fn format(
        &self,
        key: Option<&RibbonString>,
        args: Option<crate::collections::RibbonArray<RibbonFormatArgument>>,
    ) -> Result<RibbonString, RibbonLocalizationError> {
        let culture = self.culture();
        let pattern = self.get(key)?;
        let args = args.ok_or_else(|| RibbonLocalizationError::null("args"))?;
        let pattern = pattern.ok_or_else(|| RibbonLocalizationError::null("format"))?;
        format_composite(culture, &pattern, &args)
    }
}
