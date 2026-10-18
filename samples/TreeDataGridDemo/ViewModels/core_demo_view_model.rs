//! Port of `CoreDemoViewModel.cs` of the reference's core demo (`TreeDataGridCoreDemo`): the
//! sources of every page of the demo built with the framework-neutral core only, its sources and
//! its columns, and the presentation options that give the view columns of the presentation keys
//! of the core's template columns. The models and the data are the demo's own (`crate::models`),
//! as the reference's core demo links the model files of the demo.
//!
//! The view models of the reference derive from ReactiveUI's `ReactiveObject`; here they notify
//! through `mini_mvvm::ViewModelBase`, and the models of the people page are described to the
//! model type metadata of the framework (`Model`), through which the core observes the nested
//! expansion state of a person.

use super::random::Random;
use crate::models::{
    Countries, Country, DragDropItem, FileTreeNodeModel, HttpRequestException, OnThisDayArticle,
    OnThisDayJsonSerializerContext, OnThisDayTitles, WikipediaHttpClient,
};
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model, ModelTypeBuilder};
use ferroui_base::data::ReflectionBinding;
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{Brushes, IBrush, IImage, Stretch};
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::{DateTimeOffset, StringComparison, Uri};
use ferroui_base::{ferro_markup_type, BoxedValue, CornerRadius, Thickness};
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{Border, CheckBox, Control, Image, ItemsSource, StackPanel, TextBlock};
use ferroui_controls_treedatagrid::models::{ColumnRef as ViewColumnRef, TemplateColumn as ViewTemplateColumn};
use ferroui_controls_treedatagrid::presentation::{ITreeDataGridPresentationOptions, TreeDataGridPresentationOptions};
use ferroui_controls_treedatagrid::TreeDataGridSourceRef;
use ferroui_treedatagrid_core::models::{
    CheckBoxColumn, ColumnOptions, ColumnRef, Comparison, HierarchicalExpanderColumn, IColumn as CoreColumn, TemplateColumn,
    TextColumn,
};
use ferroui_treedatagrid_core::{
    Accessor, FlatTreeDataGridSource, GridLength, GridUnitType, HierarchicalTreeDataGridSource, IndexPath, Models,
};
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    // The icons of the name cells of the files page (`Assets/folder.png`, `Assets/file.png`),
    // loaded on first use, on the UI thread.
    static FILE_ICONS: RefCell<Option<(Rc<dyn IImage>, Rc<dyn IImage>)>> = const { RefCell::new(None) };
}

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

/// The view model of the core demo: the sources of its pages and their state.
pub struct CoreDemoViewModel {
    base: ViewModelBase,
    countries: FerroList<Rc<Country>>,
    visible_countries: FerroList<Rc<Country>>,
    wikipedia_items: FerroList<Rc<OnThisDayArticle>>,
    files_source: RefCell<TreeDataGridSourceRef>,
    country_status: RefCell<String>,
    files_status: RefCell<String>,
    find_status: RefCell<String>,
    wikipedia_status: RefCell<String>,
    files_flat: Cell<bool>,
    file_tree_source: Rc<HierarchicalTreeDataGridSource<FileTreeNodeModel>>,
    file_flat_source: RefCell<Option<Rc<FlatTreeDataGridSource<FileTreeNodeModel>>>>,
    country_items: FerroList<Rc<Country>>,
    variable_country_items: FerroList<Rc<Country>>,
    countries_source: Rc<FlatTreeDataGridSource<Country>>,
    find_source: Rc<FlatTreeDataGridSource<Country>>,
    variable_countries_source: Rc<FlatTreeDataGridSource<Country>>,
    files_options: Rc<TreeDataGridPresentationOptions>,
    wikipedia_source: Rc<FlatTreeDataGridSource<OnThisDayArticle>>,
    wikipedia_options: Rc<TreeDataGridPresentationOptions>,
    drag_drop_source: Rc<HierarchicalTreeDataGridSource<DragDropItem>>,
    people_source: Rc<HierarchicalTreeDataGridSource<DemoPerson>>,
    template_source: Rc<FlatTreeDataGridSource<DemoTemplateItem>>,
    template_options: Rc<TreeDataGridPresentationOptions>,
    // `LoadingTask`: whether the load of the Wikipedia articles has completed.
    loading_completed: Rc<Cell<bool>>,
}

impl PartialEq for CoreDemoViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for CoreDemoViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl CoreDemoViewModel {
    /// The view model of the demo; with `load_remote_content` the Wikipedia articles of today
    /// are loaded from the network, without it the page shows a deterministic offline article.
    pub fn new(load_remote_content: bool) -> Rc<Self> {
        let countries = FerroList::from_items(Countries::all());
        let visible_countries = FerroList::from_items(countries.to_vec());
        let countries_source = Self::create_country_source(&visible_countries);
        let find_source = Self::create_country_source(&countries);
        let country_items = countries.clone();

        let variable_country_items = FerroList::from_items(Self::create_variable_countries());
        let variable_countries_source = Self::create_country_source(&variable_country_items);

        let root_path = std::env::current_dir().map(|path| path.to_string_lossy().into_owned()).unwrap_or_default();
        let file_root = FileTreeNodeModel::new(root_path.clone(), true, true);
        let file_tree_source = Self::create_file_tree_source(file_root);
        let files_source = TreeDataGridSourceRef::of::<FileTreeNodeModel, _>(file_tree_source.clone());

        let wikipedia_items = FerroList::new();
        let wikipedia_source = Self::create_wikipedia_source(&wikipedia_items);

        let this = Rc::new(Self {
            base: ViewModelBase::new(),
            countries,
            visible_countries,
            wikipedia_items,
            files_source: RefCell::new(files_source),
            country_status: RefCell::new(String::new()),
            files_status: RefCell::new(String::new()),
            find_status: RefCell::new("No country selected.".to_string()),
            wikipedia_status: RefCell::new("Loading Wikipedia's On This Day feed\u{2026}".to_string()),
            files_flat: Cell::new(false),
            file_tree_source,
            file_flat_source: RefCell::new(None),
            country_items,
            variable_country_items,
            countries_source,
            find_source,
            variable_countries_source,
            files_options: Self::create_files_options(),
            wikipedia_source,
            wikipedia_options: Self::create_wikipedia_options(),
            drag_drop_source: Self::create_drag_drop_source(),
            people_source: Self::create_people_source(),
            template_source: Self::create_template_source(),
            template_options: Self::create_template_options(),
            loading_completed: Rc::new(Cell::new(false)),
        });

        this.set_files_status(root_path);
        if load_remote_content {
            this.load_wikipedia_async();
        } else {
            this.add_wikipedia_fixture();
            this.set_wikipedia_status("Deterministic offline data for visual validation.".to_string());
            this.loading_completed.set(true);
        }

        let count = this.visible_countries.count();
        this.set_country_status(format!("{count} rows through TreeDataGridCore"));
        this
    }

    pub fn country_items(&self) -> FerroList<Rc<Country>> {
        self.country_items.clone()
    }

    pub fn variable_country_items(&self) -> FerroList<Rc<Country>> {
        self.variable_country_items.clone()
    }

    pub fn countries_source(&self) -> Rc<FlatTreeDataGridSource<Country>> {
        self.countries_source.clone()
    }

    pub fn find_source(&self) -> Rc<FlatTreeDataGridSource<Country>> {
        self.find_source.clone()
    }

    pub fn variable_countries_source(&self) -> Rc<FlatTreeDataGridSource<Country>> {
        self.variable_countries_source.clone()
    }

    /// `FilesSource`: the hierarchical source of the files, or the flat source of the entries of
    /// the root directory while [`files_flat`](Self::files_flat) is set.
    pub fn files_source(&self) -> TreeDataGridSourceRef {
        self.files_source.borrow().clone()
    }

    fn set_files_source(&self, value: TreeDataGridSourceRef) {
        self.base.raise_and_set_if_changed(&self.files_source, value, "FilesSource");
    }

    pub fn files_options(&self) -> Rc<TreeDataGridPresentationOptions> {
        self.files_options.clone()
    }

    pub fn wikipedia_source(&self) -> Rc<FlatTreeDataGridSource<OnThisDayArticle>> {
        self.wikipedia_source.clone()
    }

    pub fn wikipedia_options(&self) -> Rc<TreeDataGridPresentationOptions> {
        self.wikipedia_options.clone()
    }

    pub fn drag_drop_source(&self) -> Rc<HierarchicalTreeDataGridSource<DragDropItem>> {
        self.drag_drop_source.clone()
    }

    pub fn people_source(&self) -> Rc<HierarchicalTreeDataGridSource<DemoPerson>> {
        self.people_source.clone()
    }

    pub fn template_source(&self) -> Rc<FlatTreeDataGridSource<DemoTemplateItem>> {
        self.template_source.clone()
    }

    pub fn template_options(&self) -> Rc<TreeDataGridPresentationOptions> {
        self.template_options.clone()
    }

    /// `LoadingTask.IsCompleted`: whether the load of the Wikipedia articles has completed.
    pub fn is_loading_completed(&self) -> bool {
        self.loading_completed.get()
    }

    pub fn files_flat(&self) -> bool {
        self.files_flat.get()
    }

    pub fn set_files_flat(&self, value: bool) {
        if self.files_flat.get() == value {
            return;
        }
        self.base.raise_and_set_if_changed_cell(&self.files_flat, value, "FilesFlat");
        let source = if value {
            let existing = self.file_flat_source.borrow().clone();
            let flat_source = match existing {
                Some(flat_source) => flat_source,
                None => {
                    let root = self.file_tree_source.items().get(0);
                    let flat_source = Self::create_file_flat_source(root.children() as Models<FileTreeNodeModel>);
                    *self.file_flat_source.borrow_mut() = Some(flat_source.clone());
                    flat_source
                }
            };
            TreeDataGridSourceRef::of::<FileTreeNodeModel, _>(flat_source)
        } else {
            TreeDataGridSourceRef::of::<FileTreeNodeModel, _>(self.file_tree_source.clone())
        };
        self.set_files_source(source);
    }

    pub fn country_status(&self) -> String {
        self.country_status.borrow().clone()
    }

    fn set_country_status(&self, value: String) {
        self.base.raise_and_set_if_changed(&self.country_status, value, "CountryStatus");
    }

    pub fn files_status(&self) -> String {
        self.files_status.borrow().clone()
    }

    fn set_files_status(&self, value: String) {
        self.base.raise_and_set_if_changed(&self.files_status, value, "FilesStatus");
    }

    pub fn find_status(&self) -> String {
        self.find_status.borrow().clone()
    }

    pub fn set_find_status(&self, value: String) {
        self.base.raise_and_set_if_changed(&self.find_status, value, "FindStatus");
    }

    pub fn wikipedia_status(&self) -> String {
        self.wikipedia_status.borrow().clone()
    }

    fn set_wikipedia_status(&self, value: String) {
        self.base.raise_and_set_if_changed(&self.wikipedia_status, value, "WikipediaStatus");
    }

    /// Shows the countries whose name or region contains `text` (all of them for an empty
    /// text), keeping the selected country selected when it is still shown.
    pub fn apply_country_filter(&self, text: Option<&str>) {
        let selection = self.countries_source.row_selection();
        let selected = selection.as_ref().and_then(|selection| selection.base().selected_item());
        self.visible_countries.clear();
        let filter = text.filter(|text| !text.trim().is_empty()).map(str::to_lowercase);
        for country in self.countries.to_vec() {
            let shown = match &filter {
                None => true,
                Some(filter) => {
                    country.name().is_some_and(|name| name.to_lowercase().contains(filter.as_str()))
                        || country.region().to_lowercase().contains(filter.as_str())
                }
            };
            if shown {
                self.visible_countries.add(country);
            }
        }

        if let Some(selected) = selected {
            if let (Some(index), Some(selection)) = (self.visible_countries.index_of(&selected), selection) {
                selection.base().set_selected_index(IndexPath::new(index as i32));
            }
        }

        let (visible, all) = (self.visible_countries.count(), self.countries.count());
        self.set_country_status(format!("{visible} of {all} rows through TreeDataGridCore"));
    }

    pub fn add_country(&self) {
        let country = Country::new(
            "Sealand".to_string(),
            "WESTERN EUROPE".to_string(),
            2,
            1,
            2.0,
            100.0,
            None,
            None,
            600000,
            None,
            None,
            None,
            None,
        );
        self.countries.add(country.clone());
        self.visible_countries.add(country);
        let count = self.visible_countries.count();
        self.set_country_status(format!("{count} rows; Sealand added through the model collection"));
    }

    pub fn remove_selected_country(&self) {
        let selected = self.countries_source.row_selection().and_then(|selection| selection.base().selected_item());
        let Some(selected) = selected else {
            return;
        };
        self.visible_countries.remove(&selected);
        self.countries.remove(&selected);
        let count = self.visible_countries.count();
        self.set_country_status(format!("{count} rows; selected model removed"));
    }

    fn create_country_source(items: &FerroList<Rc<Country>>) -> Rc<FlatTreeDataGridSource<Country>> {
        let source = FlatTreeDataGridSource::new(Rc::new(items.clone()) as Models<Country>);
        let columns = source.columns();
        columns.add(TextColumn::new(
            header("Country"),
            Accessor::member("Name", |x: &Rc<Country>| x.name())
                .with_setter(|x: &Rc<Country>, value: Option<String>| x.set_name(value)),
            Some(GridLength::new(3.0, GridUnitType::Star)),
            None,
            None,
        ) as ColumnRef<Country>);
        columns.add(TextColumn::new(
            header("Region"),
            Accessor::member("Region", |x: &Rc<Country>| x.region()),
            Some(GridLength::new(2.0, GridUnitType::Star)),
            None,
            None,
        ) as ColumnRef<Country>);
        columns.add(TextColumn::new(
            header("Population"),
            Accessor::member("Population", |x: &Rc<Country>| x.population()),
            Some(GridLength::new(1.0, GridUnitType::Star)),
            None,
            None,
        ) as ColumnRef<Country>);
        columns.add(TextColumn::new(
            header("Area"),
            Accessor::member("Area", |x: &Rc<Country>| x.area()),
            Some(GridLength::new(1.0, GridUnitType::Star)),
            None,
            None,
        ) as ColumnRef<Country>);
        columns.add(TextColumn::new(
            header("GDP"),
            Accessor::member("GDP", |x: &Rc<Country>| x.gdp()),
            Some(GridLength::new(1.0, GridUnitType::Star)),
            None,
            None,
        ) as ColumnRef<Country>);
        source.row_selection().expect("the row selection of a new source").base().set_single_select(false);
        source
    }

    fn create_variable_countries() -> Vec<Rc<Country>> {
        let mut random = Random::with_seed(42);
        Countries::all()
            .into_iter()
            .map(|country| {
                let lines = random.next_range(1, 5);
                let name = country.name().unwrap_or_default();
                Country::new(
                    vec![name; lines as usize].join("\n"),
                    country.region(),
                    country.population(),
                    country.area(),
                    country.population_density(),
                    country.coast_line(),
                    country.net_migration(),
                    country.infant_mortality(),
                    country.gdp(),
                    country.literacy_percent(),
                    country.phones(),
                    country.birth_rate(),
                    country.death_rate(),
                )
            })
            .collect()
    }

    fn create_file_tree_source(root: Rc<FileTreeNodeModel>) -> Rc<HierarchicalTreeDataGridSource<FileTreeNodeModel>> {
        let source = HierarchicalTreeDataGridSource::new(Rc::new(vec![root]) as Models<FileTreeNodeModel>);
        let columns = source.columns();
        columns.add(CheckBoxColumn::new(
            header("\u{2713}"),
            Accessor::member("IsChecked", |x: &Rc<FileTreeNodeModel>| x.is_checked())
                .with_setter(|x: &Rc<FileTreeNodeModel>, value: bool| x.set_is_checked(value)),
            Some(GridLength::pixels(44.0)),
            None,
            None,
        ) as ColumnRef<FileTreeNodeModel>);
        let name = TemplateColumn::new::<FileTreeNodeModel>(
            header("Name"),
            "file-name",
            Some(GridLength::new(3.0, GridUnitType::Star)),
            Some(Self::file_options(|x| x.name())),
            None,
        );
        columns.add(HierarchicalExpanderColumn::new(
            name as ColumnRef<FileTreeNodeModel>,
            |x: &Rc<FileTreeNodeModel>| Some(x.children() as Models<FileTreeNodeModel>),
            Some(Accessor::member("HasChildren", |x: &Rc<FileTreeNodeModel>| x.has_children())),
            Some(Accessor::member("IsExpanded", |x: &Rc<FileTreeNodeModel>| x.is_expanded())),
            Some(Rc::new(|x: &Rc<FileTreeNodeModel>, value: bool| x.set_is_expanded(value)) as Rc<dyn Fn(&Rc<FileTreeNodeModel>, bool)>),
        ) as ColumnRef<FileTreeNodeModel>);
        columns.add(TextColumn::new(
            header("Size"),
            Accessor::member("Size", |x: &Rc<FileTreeNodeModel>| x.size()),
            Some(GridLength::new(1.0, GridUnitType::Star)),
            Some(Self::file_options(|x| x.size())),
            None,
        ) as ColumnRef<FileTreeNodeModel>);
        columns.add(TextColumn::new(
            header("Modified"),
            Accessor::member("Modified", |x: &Rc<FileTreeNodeModel>| x.modified()),
            Some(GridLength::new(2.0, GridUnitType::Star)),
            Some(Self::file_options(|x| x.modified())),
            None,
        ) as ColumnRef<FileTreeNodeModel>);
        source
    }

    fn create_file_flat_source(items: Models<FileTreeNodeModel>) -> Rc<FlatTreeDataGridSource<FileTreeNodeModel>> {
        let source = FlatTreeDataGridSource::new(items);
        let columns = source.columns();
        columns.add(CheckBoxColumn::new(
            header("\u{2713}"),
            Accessor::member("IsChecked", |x: &Rc<FileTreeNodeModel>| x.is_checked())
                .with_setter(|x: &Rc<FileTreeNodeModel>, value: bool| x.set_is_checked(value)),
            Some(GridLength::pixels(44.0)),
            None,
            None,
        ) as ColumnRef<FileTreeNodeModel>);
        columns.add(TemplateColumn::new::<FileTreeNodeModel>(
            header("Name"),
            "file-name",
            Some(GridLength::new(3.0, GridUnitType::Star)),
            Some(Self::file_options(|x| x.name())),
            None,
        ) as ColumnRef<FileTreeNodeModel>);
        columns.add(TextColumn::new(
            header("Size"),
            Accessor::member("Size", |x: &Rc<FileTreeNodeModel>| x.size()),
            Some(GridLength::new(1.0, GridUnitType::Star)),
            Some(Self::file_options(|x| x.size())),
            None,
        ) as ColumnRef<FileTreeNodeModel>);
        columns.add(TextColumn::new(
            header("Modified"),
            Accessor::member("Modified", |x: &Rc<FileTreeNodeModel>| x.modified()),
            Some(GridLength::new(2.0, GridUnitType::Star)),
            Some(Self::file_options(|x| x.modified())),
            None,
        ) as ColumnRef<FileTreeNodeModel>);
        source
    }

    /// `FileOptions(selector)`: the sorts of a column of the files, the directories before the
    /// files, each ordered by the value `selector` reads.
    fn file_options<T: crate::models::DefaultComparer + 'static>(
        selector: impl Fn(&FileTreeNodeModel) -> T + Clone + 'static,
    ) -> ColumnOptions<FileTreeNodeModel> {
        let ascending = FileTreeNodeModel::sort_ascending(selector.clone());
        let descending = FileTreeNodeModel::sort_descending(selector);
        ColumnOptions {
            compare_ascending: Some(Rc::new(move |x: &Rc<FileTreeNodeModel>, y: &Rc<FileTreeNodeModel>| {
                ascending(Some(&**x), Some(&**y))
            }) as Comparison<FileTreeNodeModel>),
            compare_descending: Some(Rc::new(move |x: &Rc<FileTreeNodeModel>, y: &Rc<FileTreeNodeModel>| {
                descending(Some(&**x), Some(&**y))
            }) as Comparison<FileTreeNodeModel>),
            ..ColumnOptions::default()
        }
    }

    fn create_files_options() -> Rc<TreeDataGridPresentationOptions> {
        let options = TreeDataGridPresentationOptions::new();
        options.set_column("file-name", |column: &Rc<dyn CoreColumn>| {
            ViewTemplateColumn::<FileTreeNodeModel>::new(column.header(), file_name_template(), None, None, None)
                as ViewColumnRef
        });
        options
    }

    fn create_wikipedia_source(items: &FerroList<Rc<OnThisDayArticle>>) -> Rc<FlatTreeDataGridSource<OnThisDayArticle>> {
        let source = FlatTreeDataGridSource::new(Rc::new(items.clone()) as Models<OnThisDayArticle>);
        let columns = source.columns();
        columns.add(TemplateColumn::new::<OnThisDayArticle>(
            header("Image"),
            "wikipedia-image",
            Some(GridLength::pixels(110.0)),
            None,
            None,
        ) as ColumnRef<OnThisDayArticle>);
        columns.add(TextColumn::new(
            header("Title"),
            Accessor::new(|x: &Rc<OnThisDayArticle>| x.titles().and_then(|titles| titles.normalized)),
            Some(GridLength::new(2.0, GridUnitType::Star)),
            None,
            None,
        ) as ColumnRef<OnThisDayArticle>);
        columns.add(TextColumn::new(
            header("Extract"),
            Accessor::member("Extract", |x: &Rc<OnThisDayArticle>| x.extract()),
            Some(GridLength::new(4.0, GridUnitType::Star)),
            None,
            None,
        ) as ColumnRef<OnThisDayArticle>);
        source
    }

    fn create_wikipedia_options() -> Rc<TreeDataGridPresentationOptions> {
        let options = TreeDataGridPresentationOptions::new();
        options.set_column("wikipedia-image", |column: &Rc<dyn CoreColumn>| {
            let template = FuncDataTemplate::for_type::<OnThisDayArticle>(
                |_, _| {
                    let image = Image::new();
                    image.set_width(96.0);
                    image.set_height(64.0);
                    image.set_stretch(Stretch::UniformToFill);
                    image.bind_binding(Image::source_property().as_property(), &ReflectionBinding::new("Image"));
                    Some(image.upcast::<Control>())
                },
                false,
            );
            ViewTemplateColumn::<OnThisDayArticle>::new(column.header(), template, None, None, None) as ViewColumnRef
        });
        options
    }

    /// `LoadWikipediaAsync`: the articles of the events of today in the "On this day" feed, or
    /// the offline article when the feed cannot be loaded.
    fn load_wikipedia_async(self: &Rc<Self>) {
        let now = DateTimeOffset::now();
        let uri = format!("https://api.wikimedia.org/feed/v1/wikipedia/en/onthisday/all/{:02}/{:02}", now.month(), now.day());
        let this = Rc::downgrade(self);
        let loading_completed = self.loading_completed.clone();
        WikipediaHttpClient::shared().get_string_async(uri, move |json| {
            if let Some(this) = this.upgrade() {
                this.wikipedia_loaded(json);
            }
            loading_completed.set(true);
        });
    }

    /// The continuation of the request of the feed, on the UI thread.
    fn wikipedia_loaded(&self, json: Result<String, HttpRequestException>) {
        let data = json.map_err(|e| e.to_string()).and_then(|json| {
            OnThisDayJsonSerializerContext::deserialize_on_this_day(&json).map_err(|e| e.to_string())
        });
        match data {
            Ok(data) => {
                let articles = data.and_then(|data| data.selected).unwrap_or_default().into_iter().flat_map(|x| x.pages.unwrap_or_default());
                for article in articles {
                    self.wikipedia_items.add(Rc::new(article));
                }
                let count = self.wikipedia_items.count();
                self.set_wikipedia_status(format!("{count} virtualized Wikipedia articles loaded"));
            }
            Err(message) => {
                self.add_wikipedia_fixture();
                self.set_wikipedia_status(format!("Network unavailable; showing offline fixture. {message}"));
            }
        }
    }

    fn add_wikipedia_fixture(&self) {
        let article = OnThisDayArticle::new();
        article.set_titles(Some(OnThisDayTitles { normalized: Some("TreeDataGrid Core sample".to_string()) }));
        article.set_extract(Some(
            "A deterministic row confirms that the virtualized Core source renders without a legacy adapter.".to_string(),
        ));
        self.wikipedia_items.add(article);
    }

    fn create_drag_drop_source() -> Rc<HierarchicalTreeDataGridSource<DragDropItem>> {
        let source = HierarchicalTreeDataGridSource::new(DragDropItem::create_random_items() as Models<DragDropItem>);
        let columns = source.columns();
        columns.add(HierarchicalExpanderColumn::new(
            TextColumn::new(
                header("Name"),
                Accessor::member("Name", |x: &Rc<DragDropItem>| x.name()),
                Some(GridLength::new(2.0, GridUnitType::Star)),
                None,
                None,
            ) as ColumnRef<DragDropItem>,
            |x: &Rc<DragDropItem>| Some(x.children() as Models<DragDropItem>),
            None,
            None,
            None,
        ) as ColumnRef<DragDropItem>);
        columns.add(CheckBoxColumn::new(
            header("Allow Drag"),
            Accessor::member("AllowDrag", |x: &Rc<DragDropItem>| x.allow_drag())
                .with_setter(|x: &Rc<DragDropItem>, value: bool| x.set_allow_drag(value)),
            None,
            None,
            None,
        ) as ColumnRef<DragDropItem>);
        columns.add(CheckBoxColumn::new(
            header("Allow Drop"),
            Accessor::member("AllowDrop", |x: &Rc<DragDropItem>| x.allow_drop())
                .with_setter(|x: &Rc<DragDropItem>, value: bool| x.set_allow_drop(value)),
            None,
            None,
            None,
        ) as ColumnRef<DragDropItem>);
        source.row_selection().expect("the row selection of a new source").base().set_single_select(false);
        source
    }

    fn create_people_source() -> Rc<HierarchicalTreeDataGridSource<DemoPerson>> {
        let people = DemoPerson::create();
        let source = HierarchicalTreeDataGridSource::new(people as Models<DemoPerson>);
        let columns = source.columns();
        columns.add(HierarchicalExpanderColumn::new(
            TextColumn::new(
                header("Name"),
                Accessor::member("Name", |x: &Rc<DemoPerson>| x.name()),
                Some(GridLength::new(2.0, GridUnitType::Star)),
                None,
                None,
            ) as ColumnRef<DemoPerson>,
            |x: &Rc<DemoPerson>| Some(x.children() as Models<DemoPerson>),
            Some(Accessor::new(|x: &Rc<DemoPerson>| x.children().count() > 0)),
            // `x => x.Expansion.IsExpanded`: read from the expansion state, observed on it.
            Some(
                Accessor::member("Expansion", |x: &Rc<DemoPerson>| x.expansion().is_expanded())
                    .through(|x: &Rc<DemoPerson>| Some(x.expansion() as BoxedValue), "IsExpanded"),
            ),
            Some(Rc::new(|x: &Rc<DemoPerson>, value: bool| x.expansion().set_is_expanded(value)) as Rc<dyn Fn(&Rc<DemoPerson>, bool)>),
        ) as ColumnRef<DemoPerson>);
        columns.add(TextColumn::new(
            header("Title"),
            Accessor::member("Title", |x: &Rc<DemoPerson>| x.title()),
            Some(GridLength::new(2.0, GridUnitType::Star)),
            None,
            None,
        ) as ColumnRef<DemoPerson>);
        columns.add(TextColumn::new(header("Age"), Accessor::member("Age", |x: &Rc<DemoPerson>| x.age()), None, None, None)
            as ColumnRef<DemoPerson>);
        columns.add(CheckBoxColumn::new(
            header("Active"),
            Accessor::member("IsActive", |x: &Rc<DemoPerson>| x.is_active())
                .with_setter(|x: &Rc<DemoPerson>, value: bool| x.set_is_active(value)),
            None,
            None,
            None,
        ) as ColumnRef<DemoPerson>);
        source
    }

    fn create_template_source() -> Rc<FlatTreeDataGridSource<DemoTemplateItem>> {
        let items = FerroList::from_items((1..=200).map(|index: u32| {
            let kind = char::from(b'A' + (index % 4) as u8);
            Rc::new(DemoTemplateItem::new(
                index % 3 == 0,
                format!("Item {index:03}"),
                format!("Type {kind}"),
                format!("Details for item {index:03}"),
            ))
        }));
        let source = FlatTreeDataGridSource::new(Rc::new(items) as Models<DemoTemplateItem>);
        let columns = source.columns();
        columns.add(TemplateColumn::new::<DemoTemplateItem>(
            header("Status"),
            "status",
            Some(GridLength::pixels(70.0)),
            Some(Self::item_options(|x, y| x.is_flagged().cmp(&y.is_flagged()))),
            None,
        ) as ColumnRef<DemoTemplateItem>);
        columns.add(TextColumn::new(
            header("Name"),
            Accessor::member("Name", |x: &Rc<DemoTemplateItem>| x.name()),
            Some(GridLength::STAR),
            None,
            None,
        ) as ColumnRef<DemoTemplateItem>);
        columns.add(TextColumn::new(
            header("Type"),
            Accessor::member("Type", |x: &Rc<DemoTemplateItem>| x.r#type()),
            Some(GridLength::STAR),
            None,
            None,
        ) as ColumnRef<DemoTemplateItem>);
        columns.add(TemplateColumn::new::<DemoTemplateItem>(
            header("Details"),
            "details",
            Some(GridLength::new(2.0, GridUnitType::Star)),
            Some(Self::item_options(|x, y| StringComparison::CurrentCulture.compare(&x.details(), &y.details()))),
            None,
        ) as ColumnRef<DemoTemplateItem>);
        source
    }

    /// `ItemOptions(selector)`: the sorts of a template column of the items by the comparison
    /// of the values the column shows (`Comparer<T>.Default` of the reference), descending with
    /// the arguments swapped.
    fn item_options(
        compare: impl Fn(&DemoTemplateItem, &DemoTemplateItem) -> Ordering + 'static,
    ) -> ColumnOptions<DemoTemplateItem> {
        let compare = Rc::new(compare);
        let descending = compare.clone();
        ColumnOptions {
            compare_ascending: Some(Rc::new(move |x: &Rc<DemoTemplateItem>, y: &Rc<DemoTemplateItem>| compare(x, y))
                as Comparison<DemoTemplateItem>),
            compare_descending: Some(Rc::new(move |x: &Rc<DemoTemplateItem>, y: &Rc<DemoTemplateItem>| descending(y, x))
                as Comparison<DemoTemplateItem>),
            ..ColumnOptions::default()
        }
    }

    fn create_template_options() -> Rc<TreeDataGridPresentationOptions> {
        let options = TreeDataGridPresentationOptions::new();
        options.set_column("status", |column: &Rc<dyn CoreColumn>| {
            let template = FuncDataTemplate::for_type::<DemoTemplateItem>(
                |item, _| {
                    let check_box = CheckBox::new();
                    check_box.set_is_checked(Some(item.is_flagged()));
                    check_box.set_horizontal_alignment(HorizontalAlignment::Center);
                    Some(check_box.upcast::<Control>())
                },
                false,
            );
            ViewTemplateColumn::<DemoTemplateItem>::new(column.header(), template, None, None, None) as ViewColumnRef
        });
        options.set_column("details", |column: &Rc<dyn CoreColumn>| {
            let template = FuncDataTemplate::for_type::<DemoTemplateItem>(
                |item, _| {
                    let border = Border::new();
                    border.set_background(Some(Brushes::steel_blue() as Rc<dyn IBrush>));
                    border.set_corner_radius(CornerRadius::uniform(8.0));
                    border.set_padding(Thickness::symmetric(8.0, 2.0));
                    let text = TextBlock::new();
                    text.set_text(Some(item.details().as_str()));
                    text.set_foreground(Some(Brushes::white() as Rc<dyn IBrush>));
                    border.set_child(&text);
                    Some(border.upcast::<Control>())
                },
                false,
            );
            ViewTemplateColumn::<DemoTemplateItem>::new(column.header(), template, None, None, None) as ViewColumnRef
        });
        options
    }
}

/// The template of the name cells of the files: the icon of a directory or of a file, and the
/// name.
///
/// The reference shows a folder or a page pictograph as text before the name; the port shows
/// the folder and file icons of the demo's assets, which the files page shows too.
fn file_name_template() -> Rc<FuncDataTemplate> {
    FuncDataTemplate::for_type::<FileTreeNodeModel>(
        |model, _| {
            let panel = StackPanel::new();
            panel.set_orientation(Orientation::Horizontal);
            panel.set_spacing(6.0);
            let (folder, file) = file_icons();
            let icon = Image::new();
            icon.set_width(16.0);
            icon.set_height(16.0);
            icon.set_source(Some(if model.is_directory() { folder } else { file }));
            panel.children().add(icon);
            let name = TextBlock::new();
            name.bind_binding(TextBlock::text_property().as_property(), &ReflectionBinding::new("Name"));
            panel.children().add(name);
            Some(panel.upcast::<Control>())
        },
        false,
    )
}

/// The folder and the file icons of the demo, loaded on first use.
///
/// # Panics
/// When an icon cannot be opened or decoded.
fn file_icons() -> (Rc<dyn IImage>, Rc<dyn IImage>) {
    FILE_ICONS.with(|icons| {
        if let Some(icons) = icons.borrow().as_ref() {
            return icons.clone();
        }
        let loaded = (load_bitmap("ferres://TreeDataGridDemo/Assets/folder.png"), load_bitmap("ferres://TreeDataGridDemo/Assets/file.png"));
        *icons.borrow_mut() = Some(loaded.clone());
        loaded
    })
}

fn load_bitmap(uri: &str) -> Rc<dyn IImage> {
    let uri = Uri::absolute(uri).unwrap_or_else(|e| panic!("{e}"));
    let mut stream = AssetLoader::open(&uri, None).unwrap_or_else(|e| panic!("{e}"));
    let bitmap = Bitmap::from_stream(&mut stream).unwrap_or_else(|e| panic!("{e}"));
    Rc::new(bitmap) as Rc<dyn IImage>
}

ferro_markup_type!(class CoreDemoViewModel {
    this: Rc<CoreDemoViewModel>,
    handles: [CoreDemoViewModel, Rc<CoreDemoViewModel>, Option<Rc<CoreDemoViewModel>>],
    properties: [
        CountryItems: ItemsSource {
            get: |this: &Rc<CoreDemoViewModel>| ItemsSource::from(Rc::new(this.country_items()))
        },
        VariableCountryItems: ItemsSource {
            get: |this: &Rc<CoreDemoViewModel>| ItemsSource::from(Rc::new(this.variable_country_items()))
        },
        CountriesSource: TreeDataGridSourceRef {
            get: |this: &Rc<CoreDemoViewModel>| TreeDataGridSourceRef::of::<Country, _>(this.countries_source())
        },
        FindSource: TreeDataGridSourceRef {
            get: |this: &Rc<CoreDemoViewModel>| TreeDataGridSourceRef::of::<Country, _>(this.find_source())
        },
        VariableCountriesSource: TreeDataGridSourceRef {
            get: |this: &Rc<CoreDemoViewModel>| TreeDataGridSourceRef::of::<Country, _>(this.variable_countries_source())
        },
        FilesSource: TreeDataGridSourceRef { get: |this: &Rc<CoreDemoViewModel>| this.files_source() },
        FilesOptions: Option<Rc<dyn ITreeDataGridPresentationOptions>> {
            get: |this: &Rc<CoreDemoViewModel>| Some(this.files_options() as Rc<dyn ITreeDataGridPresentationOptions>)
        },
        WikipediaSource: TreeDataGridSourceRef {
            get: |this: &Rc<CoreDemoViewModel>| TreeDataGridSourceRef::of::<OnThisDayArticle, _>(this.wikipedia_source())
        },
        WikipediaOptions: Option<Rc<dyn ITreeDataGridPresentationOptions>> {
            get: |this: &Rc<CoreDemoViewModel>| Some(this.wikipedia_options() as Rc<dyn ITreeDataGridPresentationOptions>)
        },
        DragDropSource: TreeDataGridSourceRef {
            get: |this: &Rc<CoreDemoViewModel>| TreeDataGridSourceRef::of::<DragDropItem, _>(this.drag_drop_source())
        },
        PeopleSource: TreeDataGridSourceRef {
            get: |this: &Rc<CoreDemoViewModel>| TreeDataGridSourceRef::of::<DemoPerson, _>(this.people_source())
        },
        TemplateSource: TreeDataGridSourceRef {
            get: |this: &Rc<CoreDemoViewModel>| TreeDataGridSourceRef::of::<DemoTemplateItem, _>(this.template_source())
        },
        TemplateOptions: Option<Rc<dyn ITreeDataGridPresentationOptions>> {
            get: |this: &Rc<CoreDemoViewModel>| Some(this.template_options() as Rc<dyn ITreeDataGridPresentationOptions>)
        },
        FilesFlat: bool {
            get: |this: &Rc<CoreDemoViewModel>| this.files_flat(),
            set: |this: &Rc<CoreDemoViewModel>, value: bool| this.set_files_flat(value)
        },
        CountryStatus: String { get: |this: &Rc<CoreDemoViewModel>| this.country_status() },
        FilesStatus: String { get: |this: &Rc<CoreDemoViewModel>| this.files_status() },
        FindStatus: String {
            get: |this: &Rc<CoreDemoViewModel>| this.find_status(),
            set: |this: &Rc<CoreDemoViewModel>, value: String| this.set_find_status(value)
        },
        WikipediaStatus: String { get: |this: &Rc<CoreDemoViewModel>| this.wikipedia_status() },
    ],
    notify_property_changed: CoreDemoViewModel,
});

/// `internal sealed record DemoTemplateItem(bool IsFlagged, string Name, string Type, string
/// Details)`: an item of the template column page; two items with the same values are equal, as
/// two records are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DemoTemplateItem {
    is_flagged: bool,
    name: String,
    r#type: String,
    details: String,
}

impl DemoTemplateItem {
    pub fn new(is_flagged: bool, name: String, r#type: String, details: String) -> Self {
        Self { is_flagged, name, r#type, details }
    }

    pub fn is_flagged(&self) -> bool {
        self.is_flagged
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn r#type(&self) -> String {
        self.r#type.clone()
    }

    pub fn details(&self) -> String {
        self.details.clone()
    }
}

/// `DemoPerson`: a person of the people page, whose expansion state is an object of its own
/// (`Expansion`), which the expander column of the page reads and observes through the person.
pub struct DemoPerson {
    base: ViewModelBase,
    is_active: Cell<bool>,
    name: String,
    title: String,
    age: i32,
    expansion: Rc<ExpansionState>,
    children: Rc<FerroList<Rc<DemoPerson>>>,
}

impl PartialEq for DemoPerson {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for DemoPerson {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl Model for DemoPerson {
    fn describe(builder: ModelTypeBuilder<Self>) -> ModelTypeBuilder<Self> {
        builder.notify_property_changed()
    }
}

impl DemoPerson {
    /// `new DemoPerson { Name = name, Title = title, Age = age, IsActive = is_active }`.
    pub fn new(name: &str, title: &str, age: i32, is_active: bool) -> Rc<Self> {
        Self::register();
        Rc::new(Self {
            base: ViewModelBase::new(),
            is_active: Cell::new(is_active),
            name: name.to_string(),
            title: title.to_string(),
            age,
            expansion: ExpansionState::new(),
            children: Rc::new(FerroList::new()),
        })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn age(&self) -> i32 {
        self.age
    }

    pub fn is_active(&self) -> bool {
        self.is_active.get()
    }

    pub fn set_is_active(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_active, value, "IsActive");
    }

    pub fn expansion(&self) -> Rc<ExpansionState> {
        self.expansion.clone()
    }

    pub fn children(&self) -> Rc<FerroList<Rc<DemoPerson>>> {
        self.children.clone()
    }

    /// The people of the page: a lead with an intern, a director with an engineer who leads a
    /// designer, and a support lead.
    pub fn create() -> Rc<FerroList<Rc<DemoPerson>>> {
        let lead = DemoPerson::new("Eleanor Pope", "Engineering Manager", 32, true);
        lead.children().add(DemoPerson::new("Marcel Gutierrez", "Intern", 19, true));
        let director = DemoPerson::new("Jeremy Navarro", "Director", 47, true);
        let engineer = DemoPerson::new("Jane Navarro", "Staff Engineer", 42, true);
        engineer.children().add(DemoPerson::new("Lailah Velazquez", "Product Designer", 28, false));
        director.children().add(engineer);
        Rc::new(FerroList::from_items(vec![lead, director, DemoPerson::new("Jazmine Schroeder", "Support Lead", 36, false)]))
    }
}

/// `ExpansionState`: whether the row of a person is expanded (initially it is).
pub struct ExpansionState {
    base: ViewModelBase,
    is_expanded: Cell<bool>,
}

impl PartialEq for ExpansionState {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ExpansionState {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl Model for ExpansionState {
    fn describe(builder: ModelTypeBuilder<Self>) -> ModelTypeBuilder<Self> {
        builder.notify_property_changed()
    }
}

impl ExpansionState {
    pub fn new() -> Rc<Self> {
        Self::register();
        Rc::new(Self { base: ViewModelBase::new(), is_expanded: Cell::new(true) })
    }

    pub fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    pub fn set_is_expanded(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_expanded, value, "IsExpanded");
    }
}
