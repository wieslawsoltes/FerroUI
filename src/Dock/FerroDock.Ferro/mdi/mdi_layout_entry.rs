// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::controls::IMdiDocument;
use ferroui_base::Ref;
use ferroui_controls::Control;
use std::rc::Rc;
/// Immutable original native control/document pair used by MDI layout.
#[derive(Clone)]
pub struct MdiLayoutEntry {
    control: Ref<Control>,
    document: Rc<dyn IMdiDocument>,
}
impl MdiLayoutEntry {
    /// Creates a pair retaining the original non-null native control and model.
    pub fn new(control: Ref<Control>, document: Rc<dyn IMdiDocument>) -> Self {
        Self { control, document }
    }
    /// Gets the original native document control.
    pub fn control(&self) -> Ref<Control> {
        self.control.clone()
    }
    /// Gets the original MDI document model.
    pub fn document(&self) -> Rc<dyn IMdiDocument> {
        self.document.clone()
    }
}

impl PartialEq for MdiLayoutEntry {
    fn eq(&self, other: &Self) -> bool {
        self.control == other.control && Rc::ptr_eq(&self.document, &other.document)
    }
}

ferroui_base::ferro_markup_type!(struct MdiLayoutEntry {
    handles: [MdiLayoutEntry],
    constructors: [(Ref<Control>, Rc<dyn IMdiDocument>) => MdiLayoutEntry::new],
    properties: [
        Control: Ref<Control> { get: MdiLayoutEntry::control },
        Document: Rc<dyn IMdiDocument> { get: MdiLayoutEntry::document },
    ],
});
