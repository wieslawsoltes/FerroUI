//! Port of `ViewModels/SectionViewModel.cs`.

use crate::models::HomeSection;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use mini_mvvm::ViewModelBase;
use std::rc::Rc;

/// The view model of the page of a section.
pub struct SectionViewModel {
    base: ViewModelBase,
    home_section: Rc<HomeSection>,
}

impl PartialEq for SectionViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for SectionViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl SectionViewModel {
    /// `new SectionViewModel(homeSection)`; the view model is returned as the
    /// shared object bindings hold.
    pub fn new(home_section: Rc<HomeSection>) -> Rc<SectionViewModel> {
        Rc::new(Self { base: ViewModelBase::new(), home_section })
    }

    pub fn home_section(&self) -> Rc<HomeSection> {
        self.home_section.clone()
    }
}

ferro_markup_type!(class SectionViewModel {
    this: Rc<SectionViewModel>,
    handles: [SectionViewModel, Rc<SectionViewModel>, Option<Rc<SectionViewModel>>],
    constructors: [(Rc<HomeSection>) => SectionViewModel::new],
    properties: [HomeSection: Rc<HomeSection> { get: |this: &Rc<SectionViewModel>| this.home_section() }],
    notify_property_changed: SectionViewModel,
});
