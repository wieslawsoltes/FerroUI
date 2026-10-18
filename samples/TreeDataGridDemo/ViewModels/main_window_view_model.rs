//! Port of `ViewModels/MainWindowViewModel.cs`: the view models of the pages, created when a
//! page first asks for its view model.

use super::{
    CoreDemoViewModel, CountriesPageViewModel, DragDropPageViewModel, FilesPageViewModel, PeopleXamlPageViewModel, TemplateColumnBugPageViewModel,
    WikipediaPageViewModel,
};
use ferroui_base::ferro_markup_type;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
pub struct MainWindowViewModel {
    countries: RefCell<Option<Rc<CountriesPageViewModel>>>,
    people_xaml: RefCell<Option<Rc<PeopleXamlPageViewModel>>>,
    template_column_bug: RefCell<Option<Rc<TemplateColumnBugPageViewModel>>>,
    wikipedia: RefCell<Option<Rc<WikipediaPageViewModel>>>,
    drag_drop: RefCell<Option<Rc<DragDropPageViewModel>>>,
    files: RefCell<Option<Rc<FilesPageViewModel>>>,
    find_displayed_row_index: RefCell<Option<Rc<CountriesPageViewModel>>>,
    bring_into_view_non_uniform_rows: RefCell<Option<Rc<CountriesPageViewModel>>>,
    core: RefCell<Option<Rc<CoreDemoViewModel>>>,
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

    pub fn people_xaml(&self) -> Rc<PeopleXamlPageViewModel> {
        self.people_xaml.borrow_mut().get_or_insert_with(PeopleXamlPageViewModel::new).clone()
    }

    pub fn template_column_bug(&self) -> Rc<TemplateColumnBugPageViewModel> {
        self.template_column_bug.borrow_mut().get_or_insert_with(TemplateColumnBugPageViewModel::new).clone()
    }

    pub fn wikipedia(&self) -> Rc<WikipediaPageViewModel> {
        self.wikipedia.borrow_mut().get_or_insert_with(WikipediaPageViewModel::new).clone()
    }

    pub fn drag_drop(&self) -> Rc<DragDropPageViewModel> {
        self.drag_drop.borrow_mut().get_or_insert_with(DragDropPageViewModel::new).clone()
    }

    pub fn files(&self) -> Rc<FilesPageViewModel> {
        self.files.borrow_mut().get_or_insert_with(FilesPageViewModel::new).clone()
    }

    pub fn find_displayed_row_index(&self) -> Rc<CountriesPageViewModel> {
        self.find_displayed_row_index.borrow_mut().get_or_insert_with(|| CountriesPageViewModel::new(false)).clone()
    }

    pub fn bring_into_view_non_uniform_rows(&self) -> Rc<CountriesPageViewModel> {
        self.bring_into_view_non_uniform_rows.borrow_mut().get_or_insert_with(|| CountriesPageViewModel::new(true)).clone()
    }

    /// The view model of the core demo (`TreeDataGridCoreDemo`, a page of this demo in the port).
    pub fn core(&self) -> Rc<CoreDemoViewModel> {
        self.core.borrow_mut().get_or_insert_with(|| CoreDemoViewModel::new(true)).clone()
    }
}

ferro_markup_type!(class MainWindowViewModel {
    this: Rc<MainWindowViewModel>,
    handles: [MainWindowViewModel, Rc<MainWindowViewModel>, Option<Rc<MainWindowViewModel>>],
    constructors: [() => MainWindowViewModel::new],
    properties: [
        Countries: Rc<CountriesPageViewModel> { get: |this: &Rc<MainWindowViewModel>| this.countries() },
        PeopleXaml: Rc<PeopleXamlPageViewModel> { get: |this: &Rc<MainWindowViewModel>| this.people_xaml() },
        TemplateColumnBug: Rc<TemplateColumnBugPageViewModel> {
            get: |this: &Rc<MainWindowViewModel>| this.template_column_bug()
        },
        Wikipedia: Rc<WikipediaPageViewModel> { get: |this: &Rc<MainWindowViewModel>| this.wikipedia() },
        DragDrop: Rc<DragDropPageViewModel> { get: |this: &Rc<MainWindowViewModel>| this.drag_drop() },
        Files: Rc<FilesPageViewModel> { get: |this: &Rc<MainWindowViewModel>| this.files() },
        FindDisplayedRowIndex: Rc<CountriesPageViewModel> {
            get: |this: &Rc<MainWindowViewModel>| this.find_displayed_row_index()
        },
        BringIntoViewNonUniformRows: Rc<CountriesPageViewModel> {
            get: |this: &Rc<MainWindowViewModel>| this.bring_into_view_non_uniform_rows()
        },
    ],
});
