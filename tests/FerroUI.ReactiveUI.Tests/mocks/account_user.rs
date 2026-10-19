//! `AccountUser` (`ReactiveUI.Tests/ReactiveObjects/Mocks/AccountUser.cs`).

use ferroui_reactiveui::{reactive_object, reactive_properties, ReactiveObject};
use std::rc::Rc;

reactive_properties! {
    /// A user of an account (`AccountUser`).
    #[derive(Default)]
    pub struct AccountUser {
        reactive: ReactiveObject,
        #[reactive]
        last_name: Option<String>,
    }
}
reactive_object!(AccountUser);

impl AccountUser {
    /// A user with the last name `last_name` (`new() { LastName = .. }`).
    pub fn with_last_name(last_name: &str) -> Rc<Self> {
        let user = Rc::new(Self::default());
        user.set_last_name(Some(last_name.to_owned()));
        user
    }
}
