//! SettingsPanel.BuildLanguages. Source names/order are fixed at Build; rows are rebuilt on TextChanged.
//! No native controls, focus, dispatcher or Base types belong in this presentation/domain layer.
use ferroboard_core::{
    engine::KeyboardEngine,
    layouts::{KeyboardResources, LayoutType, Subtype},
    settings::KeyboardSettings,
    text::{TextUtils, Utf16Text},
};
use ferroui_reactiveui::{IReactiveObject, IReactiveObjectExt, ReactiveObject, reactive_object};
use ferroui_system::events::Event;
use std::{cell::RefCell, cmp::Ordering, rc::Rc, sync::Arc};

pub type LanguageResult<T> = Result<T, String>;
pub type SettingsSource = Rc<dyn Fn() -> LanguageResult<Rc<KeyboardSettings>>>;
pub type LanguageSearch = Rc<dyn Fn(&Utf16Text, &Utf16Text) -> LanguageResult<bool>>;
#[derive(Clone, Debug)]
pub struct LanguagePreset {
    pub subtype: Arc<Subtype>,
    pub name: Utf16Text,
}
#[derive(Default)]
pub struct LanguageCatalog {
    presets: Vec<LanguagePreset>,
}
impl LanguageCatalog {
    pub fn snapshot(
        resources: &KeyboardResources,
        mut compare: impl FnMut(&Utf16Text, &Utf16Text) -> LanguageResult<Ordering>,
    ) -> LanguageResult<Self> {
        let mut presets = Vec::new();
        for subtype in resources.subtype_presets() {
            let display =
                KeyboardEngine::get_subtype_display_name(&subtype).map_err(|e| e.to_string())?;
            let layout = resources
                .get_layout(
                    LayoutType::Characters,
                    &subtype
                        .layouts()
                        .ok_or_else(|| "NullReferenceException".to_owned())?
                        .characters(),
                )
                .map_err(|e| e.to_string())?;
            let label = match layout.as_ref().and_then(|l| l.label()) {
                Some(value) => Some(value),
                None => {
                    subtype
                        .layouts()
                        .ok_or_else(|| "NullReferenceException".to_owned())?
                        .characters()
                        .component_id
                }
            };
            let language = subtype.language_tag();
            let name = Utf16Text::from_units(
                [
                    display.units(),
                    &[0x20, 0x28],
                    language.as_ref().map_or(&[][..], |value| value.units()),
                    &[0x29, 0x20, 0x2014, 0x20],
                    label.as_ref().map_or(&[][..], |s| s.units()),
                ]
                .concat(),
            );
            presets.push(LanguagePreset { subtype, name });
        }
        let mut failure = None;
        presets.sort_by(|a, b| {
            if failure.is_some() {
                return Ordering::Equal;
            }
            match compare(&a.name, &b.name) {
                Ok(order) => order,
                Err(e) => {
                    failure = Some(e);
                    Ordering::Equal
                }
            }
        });
        if let Some(e) = failure {
            return Err(e);
        }
        Ok(Self { presets })
    }
    pub fn presets(&self) -> &[LanguagePreset] {
        &self.presets
    }
}

pub struct LanguageRowViewModel {
    reactive: ReactiveObject,
    settings: SettingsSource,
    name: Utf16Text,
    id: Utf16Text,
    checked: RefCell<Option<bool>>,
}
reactive_object!(LanguageRowViewModel);
impl PartialEq for LanguageRowViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl LanguageRowViewModel {
    fn new(settings: SettingsSource, preset: &LanguagePreset) -> LanguageResult<Rc<Self>> {
        let id = preset.subtype.id();
        let checked = settings()?
            .subtypes()
            .borrow()
            .iter()
            .any(|s| s.as_deref().map(Utf16Text::from).as_ref() == Some(&id));
        Ok(Rc::new(Self {
            reactive: ReactiveObject::new(),
            settings,
            name: preset.name.clone(),
            id,
            checked: RefCell::new(Some(checked)),
        }))
    }
    pub fn name(&self) -> &Utf16Text {
        &self.name
    }
    pub fn id(&self) -> &Utf16Text {
        &self.id
    }
    pub fn is_checked(&self) -> Option<bool> {
        *self.checked.borrow()
    }
    pub fn set_is_checked(&self, value: Option<bool>) {
        if self.is_checked() == value {
            return;
        }
        match value {
            Some(v) => self
                .dispatch_checked(v)
                .expect("source language checkbox callback"),
            None => {
                self.raise_and_set_if_changed(&self.checked, None, "IsChecked");
            }
        }
    }
    /// Event entry point: source Checked/Unchecked may also be raised explicitly by a consumer.
    /// An unchecked callback always assigns a NEW list, even when the id was already absent.
    pub fn dispatch_checked(&self, checked: bool) -> LanguageResult<()> {
        self.raise_and_set_if_changed(&self.checked, Some(checked), "IsChecked");
        let id = self
            .id
            .to_utf8()
            .map_err(|_| "language id contains unpaired UTF-16 surrogates".to_string())?;
        let settings = (self.settings)()?;
        let current = settings.subtypes();
        let mut values = current.borrow().clone();
        if checked {
            if values.iter().any(|v| v.as_deref() == Some(id.as_str())) {
                return Ok(());
            }
            values.push(Some(id));
        } else {
            values.retain(|v| v.as_deref() != Some(id.as_str()));
            if values.is_empty() {
                values.push(Some("en-US/qwerty".into()));
            }
        }
        settings.set_subtypes(Some(Rc::new(RefCell::new(values))));
        Ok(())
    }
}

pub enum LanguageRowsChanged {
    Reset,
    Add(Rc<LanguageRowViewModel>),
}
pub struct LanguagePanelViewModel {
    reactive: ReactiveObject,
    settings: SettingsSource,
    catalog: LanguageCatalog,
    search: LanguageSearch,
    filter: RefCell<Option<Utf16Text>>,
    rows: RefCell<Vec<Rc<LanguageRowViewModel>>>,
    rows_changed: Event<LanguageRowsChanged>,
}
reactive_object!(LanguagePanelViewModel);
impl PartialEq for LanguagePanelViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl LanguagePanelViewModel {
    pub fn new(
        settings: Rc<KeyboardSettings>,
        catalog: LanguageCatalog,
        search: LanguageSearch,
    ) -> LanguageResult<Rc<Self>> {
        Self::with_settings_source(Rc::new(move || Ok(settings.clone())), catalog, search)
    }
    pub(crate) fn with_settings_source(
        settings: SettingsSource,
        catalog: LanguageCatalog,
        search: LanguageSearch,
    ) -> LanguageResult<Rc<Self>> {
        let this = Rc::new(Self {
            reactive: ReactiveObject::new(),
            settings,
            catalog,
            search,
            filter: RefCell::new(None),
            rows: RefCell::new(Vec::new()),
            rows_changed: Event::new(),
        });
        this.populate()?;
        Ok(this)
    }
    pub fn catalog(&self) -> &LanguageCatalog {
        &self.catalog
    }
    pub fn filter(&self) -> Option<Utf16Text> {
        self.filter.borrow().clone()
    }
    pub fn rows(&self) -> Vec<Rc<LanguageRowViewModel>> {
        self.rows.borrow().clone()
    }
    pub fn rows_changed(&self) -> &Event<LanguageRowsChanged> {
        &self.rows_changed
    }
    pub fn set_filter(&self, value: Option<Utf16Text>) {
        if self.filter() == value {
            return;
        }
        self.text_changed(value).expect("source language filtering");
    }
    /// The actual source TextChanged handler also supports repeated notifications of the same value.
    pub fn text_changed(&self, value: Option<Utf16Text>) -> LanguageResult<()> {
        *self.filter.borrow_mut() = value;
        self.raise_property_changed("Filter");
        self.populate()
    }
    fn populate(&self) -> LanguageResult<()> {
        self.rows.borrow_mut().clear();
        self.rows_changed.raise(&LanguageRowsChanged::Reset);
        let filter = self.filter().unwrap_or_default();
        let mut units = filter.units();
        while units.first().is_some_and(|u| TextUtils::is_white_space(*u)) {
            units = &units[1..];
        }
        while units.last().is_some_and(|u| TextUtils::is_white_space(*u)) {
            units = &units[..units.len() - 1];
        }
        let filter = Utf16Text::from_units(units.to_vec());
        for preset in self.catalog.presets() {
            if !filter.is_empty() && !(self.search)(&preset.name, &filter)? {
                continue;
            }
            let row = LanguageRowViewModel::new(self.settings.clone(), preset)?;
            self.rows.borrow_mut().push(row.clone());
            self.rows_changed.raise(&LanguageRowsChanged::Add(row));
        }
        Ok(())
    }
}
