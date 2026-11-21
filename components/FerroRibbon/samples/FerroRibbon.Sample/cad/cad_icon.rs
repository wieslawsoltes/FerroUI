// Copyright (c) 2026 Wiesław Šoltés. MIT. Native Rust/FerroUI CAD extension.
use super::{cad_art::CadArt, cad_icons::CadIcons};
use ferroribbon_core::model::{RibbonIcon, RibbonIconKind};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::{BoxedValue, ferro_markup_type};
use ferroui_markup_xaml::{MarkupExtension, XamlLoadError};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
// Cache belongs to the UI thread, following native control/markup ownership.
ferroui_base::ferro_thread_local! {static CACHE:RefCell<HashMap<String,Option<Rc<RibbonIcon>>>>=RefCell::new(HashMap::new());}
pub struct CadIcon {
    name: RefCell<String>,
}
impl PartialEq for CadIcon {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl CadIcon {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            name: RefCell::new(String::new()),
        })
    }
    pub fn name(&self) -> String {
        self.name.borrow().clone()
    }
    pub fn set_name(&self, name: String) {
        self.name.replace(name);
    }
    pub fn get(name: &str) -> Option<Rc<RibbonIcon>> {
        if let Some(value) = CACHE.with_borrow(|cache| cache.get(name).cloned()) {
            return value;
        }
        let value = if let Some(data) = CadIcons::find(name) {
            Some(CadIcons::icon(Some(data.into())))
        } else {
            CadArt::find(name).map(|data| {
                RibbonIcon::new_with_options(
                    RibbonIconKind::Path,
                    Some(data.into()),
                    None,
                    None,
                    CadArt::size_of(name),
                )
            })
        };
        CACHE.with_borrow_mut(|cache| {
            cache.insert(name.to_owned(), value.clone());
        });
        value
    }
    pub fn provide_value(&self) -> BoxedValue {
        match Self::get(&self.name()) {
            Some(icon) => Rc::new(icon),
            None => Rc::new("?".to_owned()),
        }
    }
}
impl MarkupExtension for CadIcon {
    fn provide_value(
        &self,
        _provider: &Rc<dyn IServiceProvider>,
    ) -> Result<Option<BoxedValue>, XamlLoadError> {
        Ok(Some(self.provide_value()))
    }
}
ferro_markup_type!(class CadIcon {
 namespace:"FerroRibbon.Sample.Cad",this:Rc<CadIcon>,handles:[Rc<CadIcon>,Option<Rc<CadIcon>>],
 constructors:[()=>CadIcon::new],
 properties:[Name:String {get:|this:&Rc<CadIcon>|this.name(),set:|this:&Rc<CadIcon>,name|this.set_name(name)}],
 methods:[
  static fn Get(String)->Option<Rc<RibbonIcon>> => |name:String|CadIcon::get(&name),
  fn ProvideValue()->BoxedValue => |this:&Rc<CadIcon>|this.provide_value(),
 ]
});
