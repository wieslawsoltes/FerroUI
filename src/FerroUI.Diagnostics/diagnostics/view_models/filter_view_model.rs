//! Port of `Diagnostics/ViewModels/FilterViewModel.cs`.

use super::ViewModelBase;
use ferroui_base::data::model::{Event, INotifyDataErrorInfo, INotifyPropertyChanged};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// The filter of a list of the developer tools (properties, setters,
/// events): a text that items must match, as plain text or as a regular
/// expression, case sensitive or not, and as whole words or not. A regular
/// expression that does not parse is reported as an error of
/// `FilterString`.
pub struct FilterViewModel {
    base: ViewModelBase,
    errors: RefCell<HashMap<String, String>>,
    filter_string: RefCell<String>,
    use_regex_filter: Cell<bool>,
    use_case_sensitive_filter: Cell<bool>,
    use_whole_word_filter: Cell<bool>,
    filter_regex: RefCell<Option<regex::Regex>>,
    refresh_filter: Rc<HandlerList<dyn Fn()>>,
    errors_changed: Event<str>,
}

impl PartialEq for FilterViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for FilterViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl INotifyDataErrorInfo for FilterViewModel {
    fn has_errors(&self) -> bool {
        FilterViewModel::has_errors(self)
    }

    fn get_errors(&self, property_name: Option<&str>) -> Vec<BoxedValue> {
        FilterViewModel::get_errors(self, property_name).into_iter().map(|error| Rc::new(error) as BoxedValue).collect()
    }

    fn errors_changed(&self) -> &Event<str> {
        &self.errors_changed
    }
}

impl FilterViewModel {
    const FILTER_STRING: &'static str = "FilterString";

    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            errors: RefCell::new(HashMap::new()),
            filter_string: RefCell::new(String::new()),
            use_regex_filter: Cell::new(false),
            use_case_sensitive_filter: Cell::new(false),
            use_whole_word_filter: Cell::new(false),
            filter_regex: RefCell::new(None),
            refresh_filter: Rc::new(HandlerList::new()),
            errors_changed: Event::new(),
        })
    }

    /// `RefreshFilter`: raised when the filter changes, so that the lists it
    /// filters are filtered again. Disposing the returned handle removes the
    /// handler.
    pub fn refresh_filter(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.refresh_filter.add(Rc::new(handler));
        let handlers = self.refresh_filter.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }

    fn raise_refresh_filter(&self) {
        for (_, handler) in self.refresh_filter.snapshot().iter() {
            handler();
        }
    }

    /// Whether the filter has errors: the regular expression does not parse.
    pub fn has_errors(&self) -> bool {
        !self.errors.borrow().is_empty()
    }

    /// `ErrorsChanged`: raised with the name of the property whose errors
    /// changed.
    pub fn errors_changed(&self) -> &Event<str> {
        &self.errors_changed
    }

    /// Whether `input` passes the filter. Everything passes an empty filter
    /// and a filter whose regular expression does not parse.
    pub fn filter(&self, input: &str) -> bool {
        self.filter_regex.borrow().as_ref().is_none_or(|filter_regex| filter_regex.is_match(input))
    }

    fn update_filter_regex(&self) {
        let clear_error = || {
            let removed = self.errors.borrow_mut().remove(Self::FILTER_STRING).is_some();
            if removed {
                self.errors_changed.raise(Self::FILTER_STRING);
            }
        };

        // Deviation (DEVIATIONS.md, Developer tools): the regular expressions of the port are those
        // of the `regex` crate, not the engine of .NET: the syntax they do not share (lookarounds,
        // backreferences, .NET-only constructs) does not parse and is reported as an error of the
        // filter.
        let filter_string = self.filter_string();
        let mut pattern =
            if self.use_regex_filter() { filter_string.trim().to_string() } else { regex::escape(filter_string.trim()) };
        if self.use_whole_word_filter() {
            pattern = format!("\\b(?:{pattern})\\b");
        }

        match regex::RegexBuilder::new(&pattern).case_insensitive(!self.use_case_sensitive_filter()).build() {
            Ok(filter_regex) => {
                *self.filter_regex.borrow_mut() = Some(filter_regex);
                clear_error();
            }
            Err(exception) => {
                self.errors.borrow_mut().insert(Self::FILTER_STRING.to_string(), exception.to_string());
                self.errors_changed.raise(Self::FILTER_STRING);
            }
        }
    }

    pub fn filter_string(&self) -> String {
        self.filter_string.borrow().clone()
    }

    pub fn set_filter_string(&self, value: String) {
        if self.base.raise_and_set_if_changed(&self.filter_string, value, Self::FILTER_STRING) {
            self.update_filter_regex();
            self.raise_refresh_filter();
        }
    }

    pub fn use_regex_filter(&self) -> bool {
        self.use_regex_filter.get()
    }

    pub fn set_use_regex_filter(&self, value: bool) {
        if self.base.raise_and_set_if_changed_cell(&self.use_regex_filter, value, "UseRegexFilter") {
            self.update_filter_regex();
            self.raise_refresh_filter();
        }
    }

    pub fn use_case_sensitive_filter(&self) -> bool {
        self.use_case_sensitive_filter.get()
    }

    pub fn set_use_case_sensitive_filter(&self, value: bool) {
        if self.base.raise_and_set_if_changed_cell(&self.use_case_sensitive_filter, value, "UseCaseSensitiveFilter") {
            self.update_filter_regex();
            self.raise_refresh_filter();
        }
    }

    pub fn use_whole_word_filter(&self) -> bool {
        self.use_whole_word_filter.get()
    }

    pub fn set_use_whole_word_filter(&self, value: bool) {
        if self.base.raise_and_set_if_changed_cell(&self.use_whole_word_filter, value, "UseWholeWordFilter") {
            self.update_filter_regex();
            self.raise_refresh_filter();
        }
    }

    /// `GetErrors(propertyName)`: the error of the property, if it has one.
    pub fn get_errors(&self, property_name: Option<&str>) -> Vec<String> {
        property_name.and_then(|property_name| self.errors.borrow().get(property_name).cloned()).into_iter().collect()
    }
}

ferro_markup_type!(class FilterViewModel {
    this: Rc<FilterViewModel>,
    handles: [FilterViewModel, Rc<FilterViewModel>, Option<Rc<FilterViewModel>>],
    constructors: [() => FilterViewModel::new],
    properties: [
        HasErrors: bool { get: FilterViewModel::has_errors },
        FilterString: String { get: FilterViewModel::filter_string, set: FilterViewModel::set_filter_string },
        UseRegexFilter: bool { get: FilterViewModel::use_regex_filter, set: FilterViewModel::set_use_regex_filter },
        UseCaseSensitiveFilter: bool {
            get: FilterViewModel::use_case_sensitive_filter,
            set: FilterViewModel::set_use_case_sensitive_filter
        },
        UseWholeWordFilter: bool {
            get: FilterViewModel::use_whole_word_filter,
            set: FilterViewModel::set_use_whole_word_filter
        },
    ],
    methods: [
        fn Filter(String) -> bool => (|this: &Rc<FilterViewModel>, input: String| this.filter(&input)),
    ],
    notify_property_changed: FilterViewModel,
});
