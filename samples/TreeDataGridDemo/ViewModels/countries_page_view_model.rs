//! Port of `ViewModels/CountriesPageViewModel.cs`.

use super::random::Random;
use crate::models::{Countries, Country};
use ferroui_base::collections::FerroList;
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::{BindingBase, ReflectionBinding};
use ferroui_base::ferro_markup_type;
use ferroui_base::media::TextAlignment;
use ferroui_controls::{GridLength, GridUnitType};
use ferroui_controls_treedatagrid::{TreeDataGridSourceExtensions, TreeDataGridSourceRef};
use ferroui_treedatagrid_core::selection::{
    ITreeDataGridSelection, TreeDataGridCellSelectionModel, TreeDataGridRowSelectionModel,
};
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, ITreeDataGridSource, ITreeDataGridSourceOf, Models};
use mini_mvvm::{MiniCommand, ViewModelBase};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The countries page: a flat source of the countries with a row header, text columns and a
/// template column of the region, a filter and a choice of row or cell selection.
pub struct CountriesPageViewModel {
    base: ViewModelBase,
    data: FerroList<Rc<Country>>,
    cell_selection: Cell<bool>,
    filter_text: RefCell<String>,
    source: Rc<FlatTreeDataGridSource<Country>>,
    remove_selected: Rc<MiniCommand>,
}

impl PartialEq for CountriesPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for CountriesPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl CountriesPageViewModel {
    pub fn new(use_variable_height_rows: bool) -> Rc<Self> {
        let data = FerroList::from_items(if use_variable_height_rows {
            Self::create_countries_with_variable_height_rows()
        } else {
            Countries::all()
        });

        let source = FlatTreeDataGridSource::new(Rc::new(data.clone()) as Models<Country>)
            .with_row_header_column(None, |_| {})
            .with_text_column(
                Some(Rc::new("Country".to_string())),
                Accessor::member("Name", |x: &Rc<Country>| x.name()),
                |o| {
                    o.width = GridLength::new(6.0, GridUnitType::Star);
                    o.is_text_search_enabled = true;
                },
            )
            .with_template_column_from_resource_keys(
                Some(Rc::new("Region".to_string())),
                ResourceKey::from("RegionCell"),
                Some(ResourceKey::from("RegionEditCell")),
                |o| {
                    // `CompiledBinding.Create((Country x) => x.Region)`: the binding reads the
                    // property through the markup metadata of the model.
                    o.text_search_binding = Some(ReflectionBinding::new("Region") as Rc<dyn BindingBase>);
                },
            )
            .with_text_column(None, Accessor::member("Population", |x: &Rc<Country>| x.population()), |o| {
                o.width = GridLength::new(3.0, GridUnitType::Star);
            })
            .with_text_column(None, Accessor::member("Area", |x: &Rc<Country>| x.area()), |o| {
                o.width = GridLength::new(3.0, GridUnitType::Star);
            })
            .with_text_column(Some(Rc::new("GDP".to_string())), Accessor::member("GDP", |x: &Rc<Country>| x.gdp()), |o| {
                o.width = GridLength::new(3.0, GridUnitType::Star);
                o.text_alignment = TextAlignment::Right;
                o.max_width = Some(GridLength::new(150.0, GridUnitType::Pixel));
            });
        source.row_selection().expect("the row selection of a new source").base().set_single_select(false);

        Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let weak = weak.clone();
            Self {
                base: ViewModelBase::new(),
                data,
                cell_selection: Cell::new(false),
                filter_text: RefCell::new(String::new()),
                source,
                remove_selected: MiniCommand::create(move || {
                    if let Some(this) = weak.upgrade() {
                        this.remove_selected();
                    }
                }),
            }
        })
    }

    pub fn cell_selection(&self) -> bool {
        self.cell_selection.get()
    }

    pub fn set_cell_selection(&self, value: bool) {
        if self.cell_selection.get() != value {
            self.cell_selection.set(value);
            let source = self.source.clone() as Rc<dyn ITreeDataGridSourceOf<Country>>;
            let selection: Rc<dyn ITreeDataGridSelection> = if value {
                let selection = TreeDataGridCellSelectionModel::new(&source);
                selection.set_single_select(false);
                selection
            } else {
                let selection = TreeDataGridRowSelectionModel::new(&source);
                selection.base().set_single_select(false);
                selection
            };
            self.source.set_selection(Some(selection));
            self.base.raise_property_changed("CellSelection");
        }
    }

    pub fn source(&self) -> Rc<FlatTreeDataGridSource<Country>> {
        self.source.clone()
    }

    pub fn filter_text(&self) -> String {
        self.filter_text.borrow().clone()
    }

    pub fn set_filter_text(&self, value: String) {
        if *self.filter_text.borrow() != value {
            *self.filter_text.borrow_mut() = value;
            self.apply_filter();
            self.base.raise_property_changed("FilterText");
        }
    }

    pub fn add_country(&self, country: Rc<Country>) {
        self.data.add(country);
    }

    pub fn clear_sort(&self) {
        self.source.clear_sort();
    }

    pub fn refresh_filter(&self) {
        self.source.refresh_filter();
    }

    pub fn remove_selected(&self) {
        let Some(selection) = self.source.selection() else {
            return;
        };
        let Some(selection) = selection.row_selection_model() else {
            return;
        };
        let indexes = selection.selected_indexes();
        let selection: Vec<_> = (0..indexes.count()).map(|i| indexes.get(i)).collect();

        for index in selection.iter().rev() {
            self.data.remove_at(index.get(0) as usize);
        }
    }

    fn create_countries_with_variable_height_rows() -> Vec<Rc<Country>> {
        let mut random = Random::with_seed(42);

        Countries::all()
            .into_iter()
            .map(|country| {
                let name = country.name().unwrap_or_default();
                let line_count = random.next_range(1, 6);

                Country::new(
                    vec![name; line_count as usize].join("\n"),
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

    fn apply_filter(&self) {
        let filter_text = self.filter_text.borrow().clone();
        if filter_text.trim().is_empty() {
            self.source.set_filter(None);
            return;
        }

        let filter = filter_text.to_lowercase();
        self.source.set_filter(Some(Rc::new(move |x: &Rc<Country>| {
            x.name().is_some_and(|name| name.to_lowercase().contains(&filter)) || x.region().to_lowercase().contains(&filter)
        })));
    }
}

ferro_markup_type!(class CountriesPageViewModel {
    this: Rc<CountriesPageViewModel>,
    handles: [CountriesPageViewModel, Rc<CountriesPageViewModel>, Option<Rc<CountriesPageViewModel>>],
    properties: [
        CellSelection: bool {
            get: |this: &Rc<CountriesPageViewModel>| this.cell_selection(),
            set: |this: &Rc<CountriesPageViewModel>, value: bool| this.set_cell_selection(value)
        },
        Source: TreeDataGridSourceRef {
            get: |this: &Rc<CountriesPageViewModel>| TreeDataGridSourceRef::of::<Country, _>(this.source())
        },
        FilterText: String {
            get: |this: &Rc<CountriesPageViewModel>| this.filter_text(),
            set: |this: &Rc<CountriesPageViewModel>, value: String| this.set_filter_text(value)
        },
        RemoveSelected: Rc<dyn ferroui_base::input::ICommand> {
            get: |this: &Rc<CountriesPageViewModel>| this.remove_selected.as_command()
        },
    ],
    notify_property_changed: CountriesPageViewModel,
});
