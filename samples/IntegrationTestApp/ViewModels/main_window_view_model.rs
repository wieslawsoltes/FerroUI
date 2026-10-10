//! Port of `ViewModels/MainWindowViewModel.cs`.

use super::view_model_base::ViewModelBase;
use crate::models::Page;
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use std::cell::RefCell;
use std::rc::Rc;

/// The view model of the main window: the pages and the selected one.
pub struct MainWindowViewModel {
    base: ViewModelBase,
    selected_page: RefCell<Option<Rc<Page>>>,
    pages: Rc<BindableList<Rc<Page>>>,
}

impl PartialEq for MainWindowViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for MainWindowViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl MainWindowViewModel {
    pub fn new(pages: impl IntoIterator<Item = Rc<Page>>) -> Rc<MainWindowViewModel> {
        Rc::new(Self { base: ViewModelBase::new(), selected_page: RefCell::new(None), pages: BindableList::new(pages) })
    }

    pub fn pages(&self) -> Rc<BindableList<Rc<Page>>> {
        self.pages.clone()
    }

    pub fn selected_page(&self) -> Option<Rc<Page>> {
        self.selected_page.borrow().clone()
    }

    pub fn set_selected_page(&self, value: Option<Rc<Page>>) {
        self.base.raise_and_set_if_changed(&self.selected_page, value, "SelectedPage");
    }
}

ferro_markup_type!(class MainWindowViewModel {
    this: Rc<MainWindowViewModel>,
    handles: [MainWindowViewModel, Rc<MainWindowViewModel>, Option<Rc<MainWindowViewModel>>],
    properties: [
        // The list with its item type: the display member binding of the list box bound to it
        // takes the data type of its path from it.
        Pages: FerroList<Rc<Page>> { get: |this: &Rc<MainWindowViewModel>| this.pages().items().clone() },
        SelectedPage: Option<Rc<Page>> {
            get: |this: &Rc<MainWindowViewModel>| this.selected_page(),
            set: |this: &Rc<MainWindowViewModel>, value: Option<Rc<Page>>| this.set_selected_page(value)
        },
    ],
    notify_property_changed: MainWindowViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use ferroui_controls::{Border, Control};

    fn page(name: &str) -> Rc<Page> {
        Page::new(name, || Border::new().upcast::<Control>())
    }

    #[test]
    fn the_pages_are_the_given_ones_and_no_page_is_selected() {
        let view_model = MainWindowViewModel::new([page("Button"), page("CheckBox")]);
        let pages = view_model.pages().items().to_vec();
        assert_eq!(2, pages.len());
        assert_eq!("Button", pages[0].name());
        assert_eq!("CheckBox", pages[1].name());
        assert!(view_model.selected_page().is_none());
    }

    #[test]
    fn the_selected_page_notifies_when_it_changes() {
        let view_model = MainWindowViewModel::new([page("Button")]);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        view_model.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        let first = view_model.pages().items().get(0);
        view_model.set_selected_page(Some(first.clone()));
        view_model.set_selected_page(Some(first));
        view_model.set_selected_page(None);
        assert_eq!(vec!["SelectedPage", "SelectedPage"], *seen.borrow());
    }
}
