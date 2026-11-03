//! `AccountService` (`ReactiveUI.Tests/ReactiveObjects/Mocks/AccountService.cs`).

use super::AccountUser;
use ferroui_reactiveui::{reactive_object, ReactiveObject};
use ferroui_system::collections::generic::Dictionary;
use std::rc::Rc;
use uuid::Uuid;

/// A service with three users, and the same users plus a missing one (`AccountService`).
pub struct AccountService {
    reactive: ReactiveObject,
    account_users: Dictionary<Uuid, Rc<AccountUser>>,
    account_users_nullable: Dictionary<Uuid, Option<Rc<AccountUser>>>,
}
reactive_object!(AccountService);

impl Default for AccountService {
    fn default() -> Self {
        let mut account_users = Dictionary::new();
        account_users.add(Uuid::new_v4(), AccountUser::with_last_name("Harris"));
        account_users.add(Uuid::new_v4(), AccountUser::with_last_name("Jones"));
        account_users.add(Uuid::new_v4(), AccountUser::with_last_name("Smith"));

        let mut account_users_nullable = Dictionary::new();
        account_users_nullable.add(Uuid::new_v4(), Some(AccountUser::with_last_name("Harris")));
        account_users_nullable.add(Uuid::new_v4(), Some(AccountUser::with_last_name("Jones")));
        account_users_nullable.add(Uuid::new_v4(), Some(AccountUser::with_last_name("Smith")));
        account_users_nullable.add(Uuid::new_v4(), None);

        Self { reactive: ReactiveObject::new(), account_users, account_users_nullable }
    }
}

/// Services compare by identity, as the reference types of upstream do
/// (`EqualityComparer<AccountService>.Default`).
impl PartialEq for AccountService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl AccountService {
    /// A service with its users, shared as view models are.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// `AccountUsers`.
    pub fn account_users(&self) -> &Dictionary<Uuid, Rc<AccountUser>> {
        &self.account_users
    }

    /// `AccountUsersNullable`.
    pub fn account_users_nullable(&self) -> &Dictionary<Uuid, Option<Rc<AccountUser>>> {
        &self.account_users_nullable
    }
}
