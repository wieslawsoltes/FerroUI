//! Port of `ViewModels/MainWindowViewModel.cs`: the view models of the pages, created when a
//! page first asks for its view model.

use super::CountriesPageViewModel;
use ferroui_base::ferro_markup_type;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
pub struct MainWindowViewModel {
    countries: RefCell<Option<Rc<CountriesPageViewModel>>>,
}

impl PartialEq for MainWindowViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl MainWindowViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn countries(&self) -> Rc<CountriesPageViewModel> {
        self.countries.borrow_mut().get_or_insert_with(|| CountriesPageViewModel::new(false)).clone()
    }
}

ferro_markup_type!(class MainWindowViewModel {
    this: Rc<MainWindowViewModel>,
    handles: [MainWindowViewModel, Rc<MainWindowViewModel>, Option<Rc<MainWindowViewModel>>],
    constructors: [() => MainWindowViewModel::new],
    properties: [
        Countries: Rc<CountriesPageViewModel> { get: |this: &Rc<MainWindowViewModel>| this.countries() },
    ],
});
