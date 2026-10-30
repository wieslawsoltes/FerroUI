// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::collections::ListHandle;
use crate::controls::*;
use crate::core::*;
use crate::BoxedValue;
use ferroui_system::collections::generic::Dictionary;
use std::cell::RefCell;
use std::rc::Rc;

type StateMap<T> = RefCell<Dictionary<String, Option<T>>>;

/// Preserves tool/document content and dock templates by source ID across layout restoration.
/// Hidden/pinned/preview/window and split-view children follow the original traversal order.
#[derive(Default)]
pub struct DockState {
    tool_contents: StateMap<BoxedValue>,
    document_contents: StateMap<BoxedValue>,
    document_templates: StateMap<Rc<dyn IDocumentTemplate>>,
    tool_templates: StateMap<Rc<dyn IToolTemplate>>,
}
impl DockState {
    /// Creates an empty state cache with four independent source namespaces.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
    fn visit(&self, dock: Rc<dyn IDock>, restore: bool) {
        if let Some(root) = (dock.clone() as Rc<dyn IDockable>).as_root_dock() {
            if let Some(items) = root.hidden_dockables() {
                self.dockables(items, restore);
            }
            if let Some(items) = root.left_pinned_dockables() {
                self.dockables(items, restore);
            }
            if let Some(items) = root.right_pinned_dockables() {
                self.dockables(items, restore);
            }
            if let Some(items) = root.top_pinned_dockables() {
                self.dockables(items, restore);
            }
            if let Some(items) = root.bottom_pinned_dockables() {
                self.dockables(items, restore);
            }
            if let Some(preview) = root.pinned_dock() {
                self.visit(preview, restore);
            }
            if let Some(windows) = root.windows() {
                for window in windows.iter() {
                    let window = window.expect("source window.Layout access");
                    if window.layout().is_some() {
                        self.visit(
                            window.layout().expect("source window layout exists"),
                            restore,
                        );
                    }
                }
            }
        }
        if let Some(split) = (dock.clone() as Rc<dyn IDockable>).as_split_view_dock() {
            self.extra(split.clone(), split.pane_dockable(), restore);
            let content = split.content_dockable();
            let pane = split.pane_dockable();
            let same = match (content, pane) {
                (Some(a), Some(b)) => Rc::ptr_eq(&a, &b),
                (None, None) => true,
                _ => false,
            };
            if !same {
                self.extra(split.clone(), split.content_dockable(), restore);
            }
        }
        if let Some(items) = dock.visible_dockables() {
            self.dockables(items, restore);
        }
    }
    fn dockables(&self, items: ListHandle<Option<Rc<dyn IDockable>>>, restore: bool) {
        for item in items.iter() {
            if let Some(item) = item.clone() {
                self.apply(item, restore);
            }
            if let Some(child) = item.and_then(|item| item.as_dock()) {
                self.visit(child, restore);
            }
        }
    }
    fn extra(&self, owner: Rc<dyn IDock>, item: Option<Rc<dyn IDockable>>, restore: bool) {
        let Some(item) = item else {
            return;
        };
        if owner
            .visible_dockables()
            .is_some_and(|items| items.contains(&Some((item).clone())))
        {
            return;
        }
        self.apply(item.clone(), restore);
        if let Some(child) = item.as_dock() {
            self.visit(child, restore);
        }
    }
    fn apply(&self, item: Rc<dyn IDockable>, restore: bool) {
        if restore {
            self.restore_item(item);
        } else {
            self.save_item(item);
        }
    }
    fn save_item(&self, item: Rc<dyn IDockable>) {
        if let Some(tool) = item.clone().as_tool_content() {
            let id = tool.id();
            if !id.is_empty() {
                store(&self.tool_contents, id, tool.content());
            }
            return;
        }
        if let Some(tool) = item.clone().as_tool_dock_content() {
            let id = tool.id();
            if !id.is_empty() {
                store(&self.tool_templates, id, tool.tool_template());
            }
            return;
        }
        if let Some(document) = item.clone().as_document_content() {
            let id = document.id();
            if !id.is_empty() {
                store(&self.document_contents, id, document.content());
            }
            return;
        }
        if let Some(document) = item.as_document_dock_content() {
            let id = document.id();
            if !id.is_empty() {
                store(&self.document_templates, id, document.document_template());
            }
        }
    }
    fn restore_item(&self, item: Rc<dyn IDockable>) {
        if let Some(tool) = item.clone().as_tool_content() {
            let id = tool.id();
            let saved = if id.is_empty() {
                None
            } else {
                self.tool_contents.borrow().get(&id).cloned()
            };
            if let Some(content) = saved {
                tool.set_content(content);
            } else if let Some(owner) = tool.owner().and_then(|owner| owner.as_tool_dock_content())
            {
                if owner.tool_template().is_some() {
                    tool.set_content(
                        owner
                            .tool_template()
                            .expect("source tool template exists")
                            .content(),
                    );
                }
            }
            return;
        }
        if let Some(tool) = item.clone().as_tool_dock_content() {
            let id = tool.id();
            if !id.is_empty() {
                let saved = self.tool_templates.borrow().get(&id).cloned();
                if let Some(template) = saved {
                    tool.set_tool_template(template);
                }
            }
            return;
        }
        if let Some(document) = item.clone().as_document_content() {
            let id = document.id();
            let saved = if id.is_empty() {
                None
            } else {
                self.document_contents.borrow().get(&id).cloned()
            };
            if let Some(content) = saved {
                document.set_content(content);
            } else if let Some(owner) = document
                .owner()
                .and_then(|owner| owner.as_document_dock_content())
            {
                if owner.document_template().is_some() {
                    document.set_content(
                        owner
                            .document_template()
                            .expect("source document template exists")
                            .content(),
                    );
                }
            }
            return;
        }
        if let Some(document) = item.as_document_dock_content() {
            let id = document.id();
            if !id.is_empty() {
                let saved = self.document_templates.borrow().get(&id).cloned();
                if let Some(template) = saved {
                    document.set_document_template(template);
                }
            }
        }
    }
}
fn store<T>(map: &StateMap<T>, id: String, value: Option<T>) {
    let previous = map.borrow_mut().insert(id, value);
    drop(previous);
}
fn clear<T: Clone>(map: &StateMap<T>) {
    let retained: Vec<_> = map.borrow().values().cloned().collect();
    map.borrow_mut().clear();
    drop(retained);
}
impl IDockState for DockState {
    fn save(&self, dock: Rc<dyn IDock>) {
        self.visit(dock, false);
    }
    fn restore(&self, dock: Rc<dyn IDock>) {
        self.visit(dock, true);
    }
    fn reset(&self) {
        clear(&self.tool_contents);
        clear(&self.document_contents);
        clear(&self.document_templates);
        clear(&self.tool_templates);
    }
}
impl PartialEq for DockState {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
