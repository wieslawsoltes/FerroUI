use crate::{event::Event, Subscription, TranslationError};
use icu_locale::{Direction, Locale, LocaleDirectionality, LocaleExpander};
use std::{cell::RefCell, fmt, rc::Rc, str::FromStr};

/// Canonical Unicode BCP-47 culture identifier. The empty name is invariant.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Culture {
    name: String,
}
impl Culture {
    pub fn parse(name: &str) -> Result<Self, TranslationError> {
        if name.is_empty() {
            return Ok(Self::invariant());
        }
        if name.trim() != name || name.contains('_') {
            return Err(TranslationError::InvalidCulture(name.into()));
        }
        let locale: Locale = name
            .parse()
            .map_err(|_| TranslationError::InvalidCulture(name.into()))?;
        Ok(Self {
            name: locale.to_string(),
        })
    }
    pub fn invariant() -> Self {
        Self {
            name: String::new(),
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn is_invariant(&self) -> bool {
        self.name.is_empty()
    }
    pub(crate) fn locale(&self) -> Locale {
        if self.is_invariant() {
            return icu_locale::locale!("en-US");
        }
        self.name
            .parse()
            .expect("culture was validated at construction")
    }
    /// Parent chain follows identifier structure and terminates at invariant.
    pub fn parent(&self) -> Option<Self> {
        if self.is_invariant() {
            return None;
        }
        let locale = self.locale();
        let base = locale.id.to_string();
        if base != self.name {
            return Some(Self { name: base });
        }
        match self.name.rsplit_once('-') {
            Some((name, _)) => Some(Self { name: name.into() }),
            None => Some(Self::invariant()),
        }
    }
    pub fn flow_direction(&self) -> TextFlowDirection {
        match LocaleDirectionality::new_extended().get(&self.locale().id) {
            Some(Direction::RightToLeft) => TextFlowDirection::RightToLeft,
            _ => TextFlowDirection::LeftToRight,
        }
    }
    /// Region from the tag, or Unicode CLDR's likely-subtags data.
    /// Invariant has the deterministic region US; no process-global locale is read.
    pub fn region_code(&self) -> Option<String> {
        let mut id = self.locale().id;
        LocaleExpander::new_extended().maximize(&mut id);
        id.region.map(|region| region.to_string())
    }
}
impl Default for Culture {
    fn default() -> Self {
        Self::invariant()
    }
}
impl fmt::Debug for Culture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Culture").field(&self.name).finish()
    }
}
impl fmt::Display for Culture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}
impl FromStr for Culture {
    type Err = TranslationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextFlowDirection {
    #[default]
    LeftToRight,
    RightToLeft,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CultureSnapshot {
    pub culture: Culture,
    pub ui_culture: Culture,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CultureChangedEventArgs {
    pub old: CultureSnapshot,
    pub new: CultureSnapshot,
}
struct CultureServiceInner {
    snapshot: RefCell<CultureSnapshot>,
    changed: Event<CultureChangedEventArgs>,
}
/// Explicit, application-scoped culture state. Does not mutate any process locale.
#[derive(Clone)]
pub struct CultureService(Rc<CultureServiceInner>);
impl CultureService {
    pub fn new(culture: Culture) -> Self {
        Self(Rc::new(CultureServiceInner {
            snapshot: RefCell::new(CultureSnapshot {
                ui_culture: culture.clone(),
                culture,
            }),
            changed: Event::default(),
        }))
    }
    pub fn snapshot(&self) -> CultureSnapshot {
        self.0.snapshot.borrow().clone()
    }
    pub fn current_culture(&self) -> Culture {
        self.0.snapshot.borrow().culture.clone()
    }
    pub fn current_ui_culture(&self) -> Culture {
        self.0.snapshot.borrow().ui_culture.clone()
    }
    pub fn flow_direction(&self) -> TextFlowDirection {
        self.current_ui_culture().flow_direction()
    }
    pub fn set_culture(&self, culture: Culture) -> bool {
        self.set_cultures(culture.clone(), culture)
    }
    pub fn set_cultures(&self, culture: Culture, ui_culture: Culture) -> bool {
        let new = CultureSnapshot {
            culture,
            ui_culture,
        };
        let old = {
            let mut current = self.0.snapshot.borrow_mut();
            if *current == new {
                return false;
            }
            std::mem::replace(&mut *current, new.clone())
        };
        self.0.changed.emit(CultureChangedEventArgs { old, new });
        true
    }
    pub fn subscribe(&self, callback: impl Fn(&CultureChangedEventArgs) + 'static) -> Subscription {
        self.0.changed.subscribe(callback)
    }
    pub fn subscriber_count(&self) -> usize {
        self.0.changed.listener_count()
    }
}
