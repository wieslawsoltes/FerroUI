// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::DockState;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A named serialized layout and its optional in-memory content snapshot.
pub struct DockWorkspace {
    id: String,
    name: RefCell<Option<String>>,
    description: RefCell<Option<String>>,
    layout: RefCell<String>,
    dirty: Cell<bool>,
    state: RefCell<Option<Rc<DockState>>>,
}
impl DockWorkspace {
    /// Creates a workspace. Panics for an empty or whitespace-only ID.
    pub fn new(id: String, layout: String, state: Option<Rc<DockState>>) -> Rc<Self> {
        assert!(!is_blank(&id), "Workspace id cannot be null or whitespace.");
        Rc::new(Self {
            id,
            name: RefCell::new(None),
            description: RefCell::new(None),
            layout: RefCell::new(layout),
            dirty: Cell::new(false),
            state: RefCell::new(state),
        })
    }
    /// Gets the original workspace identifier.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Gets the optional display name.
    pub fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }
    /// Sets the optional display name, including empty strings.
    pub fn set_name(&self, value: Option<String>) {
        self.name.replace(value);
    }
    /// Gets the optional description.
    pub fn description(&self) -> Option<String> {
        self.description.borrow().clone()
    }
    /// Sets the optional description, including empty strings.
    pub fn set_description(&self, value: Option<String>) {
        self.description.replace(value);
    }
    /// Gets the serialized layout payload.
    pub fn layout(&self) -> String {
        self.layout.borrow().clone()
    }
    /// Gets whether the workspace has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty.get()
    }
    /// Gets the shared optional in-memory content state.
    pub fn state(&self) -> Option<Rc<DockState>> {
        self.state.borrow().clone()
    }
    pub(crate) fn update(
        &self,
        layout: String,
        state: Option<Rc<DockState>>,
        name: Option<String>,
        description: Option<String>,
    ) {
        self.layout.replace(layout);
        self.state.replace(state);
        self.dirty.set(false);
        if name.as_ref().is_some_and(|name| !is_blank(name)) {
            self.name.replace(name);
        }
        if description
            .as_ref()
            .is_some_and(|description| !is_blank(description))
        {
            self.description.replace(description);
        }
    }
    pub(crate) fn set_dirty(&self, value: bool) -> bool {
        self.dirty.replace(value) != value
    }
}
impl PartialEq for DockWorkspace {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

// The .NET Char.IsWhiteSpace set, rather than trimming or normalizing IDs.
pub(crate) fn is_blank(value: &str) -> bool {
    value.chars().all(|ch| matches!(ch, '\u{0009}'..='\u{000D}' | '\u{0020}' | '\u{0085}' | '\u{00A0}' | '\u{1680}' | '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}'))
}
