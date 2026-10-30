// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! A named plain object backed by the actual source Document engine model.
use ferrodock_model::core::IDockable;
use ferrodock_model_reactiveui::controls::Document;
use ferroui_base::INamed;
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{cell::RefCell, rc::Rc};

/// Consumer-only fixture; naming is provided by the application class.
pub struct NamedDocument {
    model: Rc<Document>,
    name: RefCell<Option<String>>,
}
impl NamedDocument {
    /// Constructs the real source document, without a native control base.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            model: Document::new(),
            name: RefCell::new(None),
        })
    }
    /// Gets the original engine model.
    pub fn model(&self) -> Rc<Document> {
        self.model.clone()
    }
    /// Assigns the consumer object's name.
    pub fn set_name(&self, name: Option<String>) {
        self.name.replace(name);
    }
    /// Gets the original document title.
    pub fn title(&self) -> String {
        self.model.title()
    }
    /// Sets the original document title and raises its native notification.
    pub fn set_title(&self, title: String) {
        self.model.set_title(title);
    }
}
impl INamed for NamedDocument {
    fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }
}
impl INotifyPropertyChanged for NamedDocument {
    fn property_changed(&self) -> &Event<str> {
        self.model.property_changed()
    }
}
impl PartialEq for NamedDocument {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class NamedDocument {
    namespace: "FerroDock.Markup.Tests",
    this: Rc<NamedDocument>,
    handles: [NamedDocument, Rc<NamedDocument>, Option<Rc<NamedDocument>>],
    interfaces: [Rc<dyn INamed>],
    constructors: [() => NamedDocument::new],
    notify_property_changed: NamedDocument,
    properties: [
        Name: Option<String> { get: |this: &Rc<NamedDocument>| this.name(), set: |this: &Rc<NamedDocument>, value| this.set_name(value) },
        Title: String { get: |this: &Rc<NamedDocument>| this.title(), set: |this: &Rc<NamedDocument>, value| this.set_title(value) },
    ],
});
