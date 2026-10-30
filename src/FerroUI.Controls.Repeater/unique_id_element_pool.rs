// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

use ferroui_controls::Control;
use ferroui_base::Ref;
use rustc_hash::FxHashMap;
use std::cell::RefCell;

/// A reset transaction keeps controls indexed by their previous stable key.
#[derive(Default)]
pub(crate) struct UniqueIdElementPool(RefCell<FxHashMap<String, Ref<Control>>>);

impl UniqueIdElementPool {
    pub fn add(&self, key: String, element: Ref<Control>) {
        let mut elements = self.0.borrow_mut();
        assert!(!elements.contains_key(&key), "The unique id provided ({key}) is not unique.");
        elements.insert(key, element);
    }
    pub fn remove(&self, key: &str) -> Option<Ref<Control>> { self.0.borrow_mut().remove(key) }
    pub fn take_all(&self) -> Vec<Ref<Control>> {
        std::mem::take(&mut *self.0.borrow_mut()).into_values().collect()
    }
}
