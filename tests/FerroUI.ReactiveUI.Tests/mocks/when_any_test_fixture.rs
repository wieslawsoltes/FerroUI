//! `WhenAnyTestFixture` (`ReactiveUI.Tests/ReactiveObjects/Mocks/WhenAnyTestFixture.cs`): the view
//! model of the tests of `WhenAny` with many properties and nested objects.

use super::{AccountService, ProjectService};
use ferroui_reactiveui::{reactive_object, reactive_properties, ObservableAsPropertyHelper, ReactiveObject};
use std::cell::RefCell;
use std::rc::Rc;

reactive_properties! {
    /// Twelve string properties, half of them with a value, two services, and the helper of a
    /// derived count (`WhenAnyTestFixture`).
    pub struct WhenAnyTestFixture {
        reactive: ReactiveObject,
        #[reactive]
        account_service: Rc<AccountService>,
        #[reactive]
        project_service: Rc<ProjectService>,
        #[reactive]
        value1: String,
        #[reactive]
        value2: Option<String>,
        #[reactive]
        value3: String,
        #[reactive]
        value4: Option<String>,
        #[reactive]
        value5: String,
        #[reactive]
        value6: Option<String>,
        #[reactive]
        value7: String,
        #[reactive]
        value8: Option<String>,
        #[reactive]
        value9: String,
        #[reactive]
        value10: Option<String>,
        #[reactive]
        value11: String,
        #[reactive]
        value12: Option<String>,
        accounts_found_helper: RefCell<Option<ObservableAsPropertyHelper<i32>>>,
    }
}
reactive_object!(WhenAnyTestFixture);

impl Default for WhenAnyTestFixture {
    fn default() -> Self {
        Self {
            reactive: ReactiveObject::new(),
            account_service: RefCell::new(AccountService::new()),
            project_service: RefCell::new(ProjectService::new()),
            value1: RefCell::new("1".to_owned()),
            value2: RefCell::new(None),
            value3: RefCell::new("3".to_owned()),
            value4: RefCell::new(None),
            value5: RefCell::new("5".to_owned()),
            value6: RefCell::new(None),
            value7: RefCell::new("7".to_owned()),
            value8: RefCell::new(None),
            value9: RefCell::new("9".to_owned()),
            value10: RefCell::new(None),
            value11: RefCell::new("11".to_owned()),
            value12: RefCell::new(None),
            accounts_found_helper: RefCell::new(None),
        }
    }
}

impl WhenAnyTestFixture {
    /// A fixture with its initial values, shared as view models are.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// `AccountsFound`: the value of [`accounts_found_helper`](Self::accounts_found_helper).
    ///
    /// Panics when the helper is not set, as upstream's `AccountsFoundHelper!.Value` throws.
    pub fn accounts_found(&self) -> i32 {
        self.accounts_found_helper.borrow().as_ref().expect("AccountsFoundHelper is set").value()
    }

    /// `AccountsFoundHelper`.
    pub fn accounts_found_helper(&self) -> Option<ObservableAsPropertyHelper<i32>> {
        self.accounts_found_helper.borrow().clone()
    }

    /// Sets `AccountsFoundHelper`.
    pub fn set_accounts_found_helper(&self, helper: Option<ObservableAsPropertyHelper<i32>>) {
        *self.accounts_found_helper.borrow_mut() = helper;
    }
}
