//! Native bindings only. Source row/filter algorithms live in languages.rs.
use crate::languages::{LanguagePanelViewModel, LanguageRowViewModel, LanguageRowsChanged};
use crate::settings_callback_owner::SettingsCallbackOwner;
use ferroui_base::{
    collections::FerroList,
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_controls::ItemsSource;
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{cell::Cell, rc::Rc};

pub struct LanguageRowBindings {
    model: Rc<LanguageRowViewModel>,
    name: String,
    owner: Option<Rc<SettingsCallbackOwner>>,
}
impl LanguageRowBindings {
    fn new(
        model: Rc<LanguageRowViewModel>,
        owner: Option<Rc<SettingsCallbackOwner>>,
    ) -> Result<Rc<Self>, std::string::FromUtf16Error> {
        Ok(Rc::new(Self {
            name: model.name().to_utf8()?,
            model,
            owner,
        }))
    }
    pub fn model(&self) -> Rc<LanguageRowViewModel> {
        self.model.clone()
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn is_checked(&self) -> Option<bool> {
        self.model.is_checked()
    }
    pub fn set_is_checked(&self, value: Option<bool>) {
        let _owner = self.owner.as_ref().and_then(|o| o.retain());
        if self.owner.is_some() && _owner.is_none() {
            return;
        }
        self.model.set_is_checked(value);
    }
}
impl PartialEq for LanguageRowBindings {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl INotifyPropertyChanged for LanguageRowBindings {
    fn property_changed(&self) -> &Event<str> {
        self.model.property_changed()
    }
}

pub struct LanguagePanelBindings {
    model: Rc<LanguagePanelViewModel>,
    items: Rc<FerroList<Rc<LanguageRowBindings>>>,
    source: ItemsSource,
    rows_token: Cell<Option<u64>>,
    owner: Option<Rc<SettingsCallbackOwner>>,
}
impl LanguagePanelBindings {
    pub fn new(model: Rc<LanguagePanelViewModel>) -> Result<Rc<Self>, std::string::FromUtf16Error> {
        Self::with_owner(model, None)
    }
    pub(crate) fn new_owned(
        model: Rc<LanguagePanelViewModel>,
        owner: Rc<SettingsCallbackOwner>,
    ) -> Result<Rc<Self>, std::string::FromUtf16Error> {
        Self::with_owner(model, Some(owner))
    }
    fn with_owner(
        model: Rc<LanguagePanelViewModel>,
        owner: Option<Rc<SettingsCallbackOwner>>,
    ) -> Result<Rc<Self>, std::string::FromUtf16Error> {
        let rows = model
            .rows()
            .into_iter()
            .map(|row| LanguageRowBindings::new(row, owner.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let items = Rc::new(FerroList::from_items(rows));
        let source = ItemsSource::from(items.clone());
        let this = Rc::new(Self {
            model,
            items,
            source,
            rows_token: Cell::new(None),
            owner,
        });
        let weak = Rc::downgrade(&this);
        let token = this.model.rows_changed().add(Rc::new(move |change| {
            if let Some(this) = weak.upgrade() {
                match change {
                    LanguageRowsChanged::Reset => this.items.clear(),
                    LanguageRowsChanged::Add(row) => this.items.add(LanguageRowBindings::new(row.clone(), this.owner.clone()).expect("language label contains unpaired UTF-16 surrogates; native controls cannot represent it")),
                }
            }
        }));
        this.rows_token.set(Some(token));
        Ok(this)
    }
    pub fn model(&self) -> Rc<LanguagePanelViewModel> {
        self.model.clone()
    }
    pub fn filter(&self) -> Result<Option<String>, std::string::FromUtf16Error> {
        self.model
            .filter()
            .as_ref()
            .map(ferroboard_core::text::Utf16Text::to_utf8)
            .transpose()
    }
    pub fn set_filter(&self, value: Option<String>) {
        let _owner = self.owner.as_ref().and_then(|o| o.retain());
        if self.owner.is_some() && _owner.is_none() {
            return;
        }
        self.model.set_filter(value.as_deref().map(Into::into));
    }
    pub fn items(&self) -> ItemsSource {
        self.source.clone()
    }
    pub fn release(&self) {
        if let Some(token) = self.rows_token.take() {
            self.model.rows_changed().remove(token);
        }
    }
}
impl Drop for LanguagePanelBindings {
    fn drop(&mut self) {
        self.release();
    }
}
impl PartialEq for LanguagePanelBindings {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl INotifyPropertyChanged for LanguagePanelBindings {
    fn property_changed(&self) -> &Event<str> {
        self.model.property_changed()
    }
}
ferroui_base::ferro_markup_type!(class LanguageRowBindings {
 namespace:"FerroBoard.Sample",this:Rc<LanguageRowBindings>,handles:[Rc<LanguageRowBindings>,LanguageRowBindings],notify_property_changed:LanguageRowBindings,
 properties:[Name:String{get:LanguageRowBindings::name},IsChecked:Option<bool>{get:LanguageRowBindings::is_checked,set:LanguageRowBindings::set_is_checked}]
});
ferroui_base::ferro_markup_type!(class LanguagePanelBindings {
 namespace:"FerroBoard.Sample",this:Rc<LanguagePanelBindings>,handles:[Rc<LanguagePanelBindings>,LanguagePanelBindings],notify_property_changed:LanguagePanelBindings,
 properties:[Filter:Option<String>{try_get:LanguagePanelBindings::filter,set:LanguagePanelBindings::set_filter},Items:ItemsSource{get:LanguagePanelBindings::items}]
});
pub fn register() {
    MarkupType::register_all(&[LanguageRowBindings::MARKUP, LanguagePanelBindings::MARKUP]);
    ValueTypes::register_reference::<LanguageRowBindings>();
    ValueTypes::register_reference::<LanguagePanelBindings>();
}
