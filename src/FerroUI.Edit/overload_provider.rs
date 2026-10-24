use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Supplies signatures and documentation to an overload viewer. Providers use
/// FerroUI's normal model notification contract; an empty property name means
/// that every displayed property changed. Notifications run on the UI thread.
pub trait IOverloadProvider: INotifyPropertyChanged {
    fn selected_index(&self) -> usize;
    fn set_selected_index(&self, index: usize);
    fn count(&self) -> usize;
    fn current_index_text(&self) -> String;
    fn current_header(&self) -> Option<BoxedValue>;
    fn current_content(&self) -> Option<BoxedValue>;
}

/// Identity-comparable provider reference for the native `Provider` property.
#[derive(Clone)]
pub struct OverloadProviderRef(pub Rc<dyn IOverloadProvider>);
impl PartialEq for OverloadProviderRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Both fields accept strings, controls or data presented by a content template.
#[derive(Clone, Default)]
pub struct OverloadItem {
    pub header: Option<BoxedValue>,
    pub content: Option<BoxedValue>,
}
impl OverloadItem {
    pub fn new(header: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            header: Some(Rc::new(header.into())),
            content: Some(Rc::new(content.into())),
        }
    }
}

/// An observable provider for applications that already have their signatures.
/// Replacing the collection preserves the selected index when it still exists.
pub struct OverloadProvider {
    items: RefCell<Vec<OverloadItem>>,
    selected_index: Cell<usize>,
    property_changed: Event<str>,
}
impl OverloadProvider {
    pub fn new(items: Vec<OverloadItem>) -> Rc<Self> {
        Rc::new(Self {
            items: RefCell::new(items),
            selected_index: Cell::new(0),
            property_changed: Event::new(),
        })
    }
    pub fn set_items(&self, items: Vec<OverloadItem>) {
        self.selected_index
            .set(self.selected_index.get().min(items.len().saturating_sub(1)));
        *self.items.borrow_mut() = items;
        self.property_changed.raise("");
    }
    /// Publishes changes made to custom values retained in the current item.
    pub fn refresh(&self) {
        self.property_changed.raise("");
    }
}
impl INotifyPropertyChanged for OverloadProvider {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}
impl IOverloadProvider for OverloadProvider {
    fn selected_index(&self) -> usize {
        self.selected_index.get()
    }
    fn set_selected_index(&self, index: usize) {
        let index = index.min(self.count().saturating_sub(1));
        if self.selected_index.replace(index) != index {
            self.property_changed.raise("");
        }
    }
    fn count(&self) -> usize {
        self.items.borrow().len()
    }
    fn current_index_text(&self) -> String {
        let count = self.count();
        format!(
            "{} of {count}",
            if count == 0 {
                0
            } else {
                self.selected_index.get() + 1
            }
        )
    }
    fn current_header(&self) -> Option<BoxedValue> {
        self.items
            .borrow()
            .get(self.selected_index.get())
            .and_then(|item| item.header.clone())
    }
    fn current_content(&self) -> Option<BoxedValue> {
        self.items
            .borrow()
            .get(self.selected_index.get())
            .and_then(|item| item.content.clone())
    }
}
