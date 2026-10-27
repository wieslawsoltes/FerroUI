use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::pretext_layout_measurement::FontState;
use crate::pretext_layout_shaping::{PreparedShapedCacheEntry, ShaperState};
use crate::{IPretextTextMeasurerFactory, IPretextTextShaperFactory, PretextError, PretextTextDirection};

/// An isolated preparation session. Replacing a backend affects subsequent
/// preparation; already prepared metrics and shaped objects remain usable.
pub struct PretextLayout {
    pub(crate) text_measurer_factory: RefCell<Rc<dyn IPretextTextMeasurerFactory>>,
    pub(crate) text_shaper_factory: RefCell<Option<Rc<dyn IPretextTextShaperFactory>>>,
    pub(crate) font_states: RefCell<HashMap<String, Rc<FontState>>>,
    pub(crate) shaper_states: RefCell<HashMap<String, Rc<ShaperState>>>,
    pub(crate) prepared_shaped: RefCell<HashMap<(usize, PretextTextDirection), PreparedShapedCacheEntry>>,
    pub(crate) locale: RefCell<Option<String>>,
    pub(crate) cache_generation: Cell<u64>,
}

impl PretextLayout {
    pub fn new(factory: Rc<dyn IPretextTextMeasurerFactory>) -> Self {
        Self {
            text_measurer_factory: RefCell::new(factory),
            text_shaper_factory: RefCell::new(None),
            font_states: RefCell::new(HashMap::new()),
            shaper_states: RefCell::new(HashMap::new()),
            prepared_shaped: RefCell::new(HashMap::new()),
            locale: RefCell::new(None),
            cache_generation: Cell::new(0),
        }
    }

    pub fn with_shaper_factory(self, factory: Rc<dyn IPretextTextShaperFactory>) -> Self {
        self.set_text_shaper_factory(Some(factory));
        self
    }

    /// Selects the highest-priority supported registered measurement backend.
    /// Equal priorities retain registration order. Reflection is unnecessary.
    pub fn from_factories(factories: impl IntoIterator<Item = Rc<dyn IPretextTextMeasurerFactory>>) -> Result<Self, PretextError> {
        let mut selected: Option<Rc<dyn IPretextTextMeasurerFactory>> = None;
        for factory in factories {
            if factory.is_supported() && selected.as_ref().is_none_or(|current| factory.priority() > current.priority()) {
                selected = Some(factory);
            }
        }
        selected.map(Self::new).ok_or_else(|| PretextError::Backend("no supported text measurement backend is registered".into()))
    }

    pub fn text_measurer_factory(&self) -> Rc<dyn IPretextTextMeasurerFactory> {
        self.text_measurer_factory.borrow().clone()
    }

    pub fn text_shaper_factory(&self) -> Result<Rc<dyn IPretextTextShaperFactory>, PretextError> {
        let explicit = self.text_shaper_factory.borrow().clone();
        let factory = match explicit {
            Some(factory) => Some(factory),
            None => self.text_measurer_factory().shaper_factory(),
        };
        factory.filter(|factory| factory.is_supported()).ok_or(PretextError::NoShapingBackend)
    }

    pub fn set_text_measurer_factory(&self, factory: Rc<dyn IPretextTextMeasurerFactory>) {
        let previous = self.text_measurer_factory.replace(factory);
        self.clear_cache();
        drop(previous);
    }

    pub fn set_text_shaper_factory(&self, factory: Option<Rc<dyn IPretextTextShaperFactory>>) {
        let previous = self.text_shaper_factory.replace(factory);
        self.clear_cache();
        drop(previous);
    }

    pub fn clear_cache(&self) {
        // Drop backend handles after releasing every RefCell borrow: backend
        // destructors can invoke application callbacks.
        let fonts = std::mem::take(&mut *self.font_states.borrow_mut());
        let shapers = std::mem::take(&mut *self.shaper_states.borrow_mut());
        let prepared = std::mem::take(&mut *self.prepared_shaped.borrow_mut());
        self.cache_generation.set(self.cache_generation.get().wrapping_add(1));
        drop((fonts, shapers, prepared));
    }

    pub fn set_locale(&self, locale: Option<&str>) {
        *self.locale.borrow_mut() = locale.map(str::to_owned);
        self.clear_cache();
    }

    pub fn locale(&self) -> Option<String> { self.locale.borrow().clone() }
}
