//! The countries page: the tab "Countries" of `MainWindow.axaml` of the reference demo, with the
//! handlers of its buttons from `MainWindow.axaml.cs`, as a view of its own that the main window
//! and the ControlCatalog both show.

use crate::markup::{user_control_class, xaml_class};
use crate::models::Country;
use crate::view_models::CountriesPageViewModel;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{TextBox, UserControl};
use ferroui_controls_treedatagrid::TreeDataGrid;
use ferroui_treedatagrid_core::ITreeDataGridSource;
use std::rc::Rc;

#[repr(C)]
pub struct CountriesPage {
    base: UserControl,
}

user_control_class!(CountriesPage);
ferro_class_info!(CountriesPage {
    new: CountriesPage::new,
    markup: {
        methods: [
            fn AddCountryClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CountriesPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.add_country_click(&sender, e.as_routed_event_args())
                },
            fn ClearCountrySortClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CountriesPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.clear_country_sort_click(&sender, e.as_routed_event_args())
                },
            fn RefreshCountryFilterClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CountriesPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.refresh_country_filter_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CountriesPage, "/Views/CountriesPage.xaml");

impl CountriesPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn view_model(&self) -> Option<Rc<CountriesPageViewModel>> {
        from_markup_value::<Rc<CountriesPageViewModel>>(&self.data_context())
    }

    pub fn add_country_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let countries = self.find_control::<TreeDataGrid>("countries");
        let text = |name: &str| self.find_control::<TextBox>(name).and_then(|text_box| text_box.text());
        let (
            Some(countries),
            Some(country_text),
            Some(region_text),
            Some(population_text),
            Some(area_text),
            Some(gdp_text),
        ) = (
            countries,
            text("countryTextBox"),
            text("regionTextBox"),
            text("populationTextBox"),
            text("areaTextBox"),
            text("gdpTextBox"),
        )
        else {
            return;
        };

        let country = Country::new(
            country_text,
            region_text,
            population_text.trim().parse().unwrap_or(0),
            area_text.trim().parse().unwrap_or(0),
            0.0,
            0.0,
            None,
            None,
            gdp_text.trim().parse().unwrap_or(0),
            None,
            None,
            None,
            None,
        );
        let Some(vm) = self.view_model() else {
            return;
        };
        vm.add_country(country);

        let index = vm.source().rows().count() as i32 - 1;
        if let Some(rows_presenter) = countries.rows_presenter() {
            rows_presenter.bring_into_view(index, None);
        }
        if let Some(row) = countries.try_get_row(index) {
            row.focus();
        }
    }

    pub fn clear_country_sort_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(vm) = self.view_model() {
            vm.clear_sort();
        }
    }

    pub fn refresh_country_filter_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(vm) = self.view_model() {
            vm.refresh_filter();
        }
    }
}
