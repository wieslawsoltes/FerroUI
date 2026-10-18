//! Port of `Models/TemplateColumnItem.cs`.

use ferroui_base::ferro_markup_type;
use std::rc::Rc;

/// An item of the template column reuse page: its values are set once, when it is created
/// (`required ... { get; init; }`).
pub struct TemplateColumnItem {
    name: String,
    r#type: String,
    details: String,
    is_flagged: bool,
}

impl PartialEq for TemplateColumnItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl TemplateColumnItem {
    /// `new TemplateColumnItem { Name = .., Type = .., Details = .., IsFlagged = .. }`.
    pub fn new(name: String, r#type: String, details: String, is_flagged: bool) -> Rc<Self> {
        Rc::new(Self { name, r#type, details, is_flagged })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn r#type(&self) -> String {
        self.r#type.clone()
    }

    pub fn details(&self) -> String {
        self.details.clone()
    }

    pub fn is_flagged(&self) -> bool {
        self.is_flagged
    }
}

ferro_markup_type!(class TemplateColumnItem {
    this: Rc<TemplateColumnItem>,
    handles: [TemplateColumnItem, Rc<TemplateColumnItem>, Option<Rc<TemplateColumnItem>>],
    properties: [
        Name: String { get: |this: &Rc<TemplateColumnItem>| this.name() },
        Type: String { get: |this: &Rc<TemplateColumnItem>| this.r#type() },
        Details: String { get: |this: &Rc<TemplateColumnItem>| this.details() },
        IsFlagged: bool { get: |this: &Rc<TemplateColumnItem>| this.is_flagged() },
    ],
});
