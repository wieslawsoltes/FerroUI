//! Port of `Pages/NumericUpDownPage.xaml.cs`: the class of the document
//! `Pages/NumericUpDownPage.xaml`, its view model and the format of the
//! view model.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::collections::FerroList;
use ferroui_base::data::converters::{FuncValueConverter, IValueConverter};
use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::utilities::{CultureInfo, CultureTypes, Decimal, NumberFormatInfo};
use ferroui_base::{ferro_class_info, ferro_markup_type, instantiate, BoxedValue, Ref};
use ferroui_controls::{ContentPage, ItemsSource, Location};
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

ferroui_controls::ferro_markup_list!(pub FormatObjectList: Rc<FormatObject>);

thread_local! {
    static CULTURE_CONVERTER: Rc<dyn IValueConverter> =
        Rc::new(FuncValueConverter::<Option<CultureInfo>, Rc<NumberFormatInfo>>::new(|c| {
            c.unwrap_or_else(CultureInfo::current_culture).number_format()
        }));
}

#[repr(C)]
pub struct NumericUpDownPage {
    base: ContentPage,
}

content_page_class!(NumericUpDownPage);
ferro_class_info!(NumericUpDownPage {
    new: NumericUpDownPage::new,
    markup: {
        fields: [CultureConverter: Rc<dyn IValueConverter> => NumericUpDownPage::culture_converter],
    },
});
xaml_class!(NumericUpDownPage, "/Pages/NumericUpDownPage.xaml");

impl NumericUpDownPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let view_model = NumbersPageViewModel::new();
        this.set_data_context(Some(view_model as BoxedValue));
        this
    }

    /// The number format of a culture; of the current culture for no culture.
    pub fn culture_converter() -> Rc<dyn IValueConverter> {
        CULTURE_CONVERTER.with(Clone::clone)
    }
}

/// The values of `Location`, in the order of their declaration
/// (`Enum.GetValues<Location>()`).
const LOCATIONS: [Location; 2] = [Location::Left, Location::Right];

/// The names of the cultures the page offers, of the specific cultures of
/// the runtime.
const CULTURE_NAMES: [&str; 6] = ["en-US", "en-GB", "fr-FR", "ar-DZ", "zh-CH", "cs-CZ"];

/// The view model of the numeric up-down page.
pub struct NumbersPageViewModel {
    base: ViewModelBase,
    formats: RefCell<Option<Rc<BindableList<Rc<FormatObject>>>>>,
    selected_format: RefCell<Option<Rc<FormatObject>>>,
    spinner_locations: RefCell<Option<Rc<BindableList<Location>>>>,
    double_value: Cell<f64>,
    decimal_value: Cell<Decimal>,
    cultures: Rc<BindableList<CultureInfo>>,
}

impl PartialEq for NumbersPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for NumbersPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl NumbersPageViewModel {
    pub fn new() -> Rc<NumbersPageViewModel> {
        let this = Rc::new(Self {
            base: ViewModelBase::new(),
            formats: RefCell::new(None),
            selected_format: RefCell::new(None),
            spinner_locations: RefCell::new(None),
            double_value: Cell::new(0.0),
            decimal_value: Cell::new(Decimal::ZERO),
            // Trimmed-mode friendly where we might not have cultures
            cultures: BindableList::new(
                CultureInfo::get_cultures(CultureTypes::SPECIFIC_CULTURES)
                    .into_iter()
                    .filter(|c| CULTURE_NAMES.contains(&c.name())),
            ),
        });
        let first = this.formats().items().to_vec().into_iter().next();
        *this.selected_format.borrow_mut() = first;
        this
    }

    pub fn double_value(&self) -> f64 {
        self.double_value.get()
    }

    pub fn set_double_value(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.double_value, value, "DoubleValue");
    }

    pub fn decimal_value(&self) -> Decimal {
        self.decimal_value.get()
    }

    pub fn set_decimal_value(&self, value: Decimal) {
        self.base.raise_and_set_if_changed_cell(&self.decimal_value, value, "DecimalValue");
    }

    pub fn formats(&self) -> Rc<BindableList<Rc<FormatObject>>> {
        let formats = self.formats.borrow().clone();
        formats.unwrap_or_else(|| {
            let format = |name: &str, value: &str| {
                let format = FormatObject::new();
                format.set_name(Some(name.to_string()));
                format.set_value(Some(value.to_string()));
                format
            };
            let formats = BindableList::new([
                format("Currency", "C2"),
                format("Fixed point", "F2"),
                format("General", "G"),
                format("Number", "N"),
                format("Percent", "P"),
                format("Degrees", "{0:N2} \u{00B0}"),
            ]);
            *self.formats.borrow_mut() = Some(formats.clone());
            formats
        })
    }

    pub fn spinner_locations(&self) -> Rc<BindableList<Location>> {
        let spinner_locations = self.spinner_locations.borrow().clone();
        spinner_locations.unwrap_or_else(|| {
            let spinner_locations = BindableList::new([]);
            for value in LOCATIONS {
                spinner_locations.items().add(value);
            }
            *self.spinner_locations.borrow_mut() = Some(spinner_locations.clone());
            spinner_locations
        })
    }

    pub fn cultures(&self) -> Rc<BindableList<CultureInfo>> {
        self.cultures.clone()
    }

    pub fn selected_format(&self) -> Option<Rc<FormatObject>> {
        self.selected_format.borrow().clone()
    }

    pub fn set_selected_format(&self, value: Option<Rc<FormatObject>>) {
        self.base.raise_and_set_if_changed(&self.selected_format, value, "SelectedFormat");
    }
}

ferro_markup_type!(class NumbersPageViewModel {
    this: Rc<NumbersPageViewModel>,
    handles: [NumbersPageViewModel, Rc<NumbersPageViewModel>, Option<Rc<NumbersPageViewModel>>],
    constructors: [() => NumbersPageViewModel::new],
    properties: [
        DoubleValue: f64 {
            get: |this: &Rc<NumbersPageViewModel>| this.double_value(),
            set: |this: &Rc<NumbersPageViewModel>, value: f64| this.set_double_value(value)
        },
        DecimalValue: Decimal {
            get: |this: &Rc<NumbersPageViewModel>| this.decimal_value(),
            set: |this: &Rc<NumbersPageViewModel>, value: Decimal| this.set_decimal_value(value)
        },
        // The list with its item type: the template of the combo box bound to it takes the
        // data type of its bindings from it.
        Formats: FerroList<Rc<FormatObject>> { get: |this: &Rc<NumbersPageViewModel>| this.formats().items().clone() },
        // Lists a binding delivers to an items source property.
        SpinnerLocations: ItemsSource {
            get: |this: &Rc<NumbersPageViewModel>| ItemsSource::from(this.spinner_locations())
        },
        Cultures: ItemsSource { get: |this: &Rc<NumbersPageViewModel>| ItemsSource::from(this.cultures()) },
        SelectedFormat: Option<Rc<FormatObject>> {
            get: |this: &Rc<NumbersPageViewModel>| this.selected_format(),
            set: |this: &Rc<NumbersPageViewModel>, value: Option<Rc<FormatObject>>| this.set_selected_format(value)
        },
    ],
    notify_property_changed: NumbersPageViewModel,
});

/// A format of the page: its name and its format string.
#[derive(Default)]
pub struct FormatObject {
    value: RefCell<Option<String>>,
    name: RefCell<Option<String>>,
}

impl PartialEq for FormatObject {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl FormatObject {
    pub fn new() -> Rc<FormatObject> {
        Rc::new(Self::default())
    }

    pub fn value(&self) -> Option<String> {
        self.value.borrow().clone()
    }

    pub fn set_value(&self, value: Option<String>) {
        *self.value.borrow_mut() = value;
    }

    pub fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    pub fn set_name(&self, value: Option<String>) {
        *self.name.borrow_mut() = value;
    }
}

ferro_markup_type!(class FormatObject {
    this: Rc<FormatObject>,
    handles: [FormatObject, Rc<FormatObject>, Option<Rc<FormatObject>>],
    constructors: [() => FormatObject::new],
    properties: [
        Value: Option<String> {
            get: |this: &Rc<FormatObject>| this.value(),
            set: |this: &Rc<FormatObject>, value: Option<String>| this.set_value(value)
        },
        Name: Option<String> {
            get: |this: &Rc<FormatObject>| this.name(),
            set: |this: &Rc<FormatObject>, value: Option<String>| this.set_name(value)
        },
    ],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_view_model_starts_with_the_first_format() {
        let view_model = NumbersPageViewModel::new();
        let formats = view_model.formats().items().to_vec();
        assert_eq!(6, formats.len());
        assert_eq!(Some("Currency".to_string()), formats[0].name());
        assert_eq!(Some("C2".to_string()), formats[0].value());
        assert_eq!(Some("{0:N2} \u{00B0}".to_string()), formats[5].value());
        assert!(view_model.selected_format().is_some_and(|format| Rc::ptr_eq(&format, &formats[0])));
        // The lists are created once.
        assert!(Rc::ptr_eq(&view_model.formats(), &view_model.formats()));
        assert!(Rc::ptr_eq(&view_model.spinner_locations(), &view_model.spinner_locations()));
        assert_eq!(vec![Location::Left, Location::Right], view_model.spinner_locations().items().to_vec());
        assert_eq!(0.0, view_model.double_value());
        assert_eq!(Decimal::ZERO, view_model.decimal_value());
    }

    #[test]
    fn the_setters_notify_when_the_value_changes() {
        let view_model = NumbersPageViewModel::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        view_model.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        view_model.set_double_value(1.5);
        view_model.set_double_value(1.5);
        view_model.set_decimal_value(Decimal::from(2));
        let format = view_model.formats().items().get(1);
        view_model.set_selected_format(Some(format.clone()));
        view_model.set_selected_format(Some(format));
        assert_eq!(vec!["DoubleValue", "DecimalValue", "SelectedFormat"], *seen.borrow());
    }
}
