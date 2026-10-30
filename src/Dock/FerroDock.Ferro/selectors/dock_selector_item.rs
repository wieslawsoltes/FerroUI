// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::IDockable;
use std::rc::Rc;
/// Immutable original entry in the dock selector overlay.
pub struct DockSelectorItem {
    dockable: Rc<dyn IDockable>,
    title: String,
    activation_order: i64,
    is_document: bool,
    is_tool: bool,
    is_pinned: bool,
    is_hidden: bool,
    is_floating: bool,
}
impl PartialEq for DockSelectorItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl DockSelectorItem {
    /// Captures the original title, activation order and role/state flags.
    pub fn new(
        dockable: Rc<dyn IDockable>,
        activation_order: i64,
        is_document: bool,
        is_tool: bool,
        is_pinned: bool,
        is_hidden: bool,
        is_floating: bool,
    ) -> Rc<Self> {
        let title = dockable
            .as_dock_selector_info()
            .and_then(|info| info.selector_title())
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| dockable.title());
        Rc::new(Self {
            dockable,
            title,
            activation_order,
            is_document,
            is_tool,
            is_pinned,
            is_hidden,
            is_floating,
        })
    }
    /// Gets the original dockable reference.
    pub fn dockable(&self) -> Rc<dyn IDockable> {
        self.dockable.clone()
    }
    /// Gets the captured selector title.
    pub fn title(&self) -> String {
        self.title.clone()
    }
    /// Gets the activation order used for sorting.
    pub fn activation_order(&self) -> i64 {
        self.activation_order
    }
    /// Gets whether this is a document.
    pub fn is_document(&self) -> bool {
        self.is_document
    }
    /// Gets whether this is a tool.
    pub fn is_tool(&self) -> bool {
        self.is_tool
    }
    /// Gets whether the item is pinned.
    pub fn is_pinned(&self) -> bool {
        self.is_pinned
    }
    /// Gets whether the item is hidden.
    pub fn is_hidden(&self) -> bool {
        self.is_hidden
    }
    /// Gets whether the item is floating.
    pub fn is_floating(&self) -> bool {
        self.is_floating
    }
}
ferroui_base::ferro_markup_type!(class DockSelectorItem {
    namespace:"FerroDock.Ferro.Selectors",this:Rc<DockSelectorItem>,handles:[Rc<DockSelectorItem>,Option<Rc<DockSelectorItem>>],
    constructors:[(Rc<dyn IDockable>,i64,bool,bool,bool,bool,bool)=>DockSelectorItem::new],
    properties:[
        Dockable:Rc<dyn IDockable>{get:|this:&Rc<DockSelectorItem>|this.dockable()},
        Title:String{get:|this:&Rc<DockSelectorItem>|this.title()},
        ActivationOrder:i64{get:|this:&Rc<DockSelectorItem>|this.activation_order()},
        IsDocument:bool{get:|this:&Rc<DockSelectorItem>|this.is_document()},
        IsTool:bool{get:|this:&Rc<DockSelectorItem>|this.is_tool()},
        IsPinned:bool{get:|this:&Rc<DockSelectorItem>|this.is_pinned()},
        IsHidden:bool{get:|this:&Rc<DockSelectorItem>|this.is_hidden()},
        IsFloating:bool{get:|this:&Rc<DockSelectorItem>|this.is_floating()},
    ],
});
