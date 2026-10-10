//! Port of `Models/Page.cs`.

use ferroui_base::ferro_markup_type;
use ferroui_base::Ref;
use ferroui_controls::Control;
use std::rc::Rc;

// The list of the pages of the main window view model (`ObservableCollection<Page>`), with its
// item type: the display member binding of the list box bound to it takes the data type of its
// path from it.
ferroui_controls::ferro_markup_list!(pub PageList: Rc<Page>);

/// A page of the main window: its name in the list of pages and the function that creates its
/// content (`record Page(string Name, Func<Control> CreateContent)`).
pub struct Page {
    name: String,
    create_content: Rc<dyn Fn() -> Ref<Control>>,
}

/// The equality of a record: the name and the delegate (which compares by reference).
impl PartialEq for Page {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && Rc::ptr_eq(&self.create_content, &other.create_content)
    }
}

impl Page {
    pub fn new(name: &str, create_content: impl Fn() -> Ref<Control> + 'static) -> Rc<Page> {
        Rc::new(Self { name: name.to_string(), create_content: Rc::new(create_content) })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// `CreateContent`: the function that creates the content of the page.
    pub fn create_content(&self) -> Rc<dyn Fn() -> Ref<Control>> {
        self.create_content.clone()
    }
}

ferro_markup_type!(class Page {
    this: Rc<Page>,
    handles: [Page, Rc<Page>, Option<Rc<Page>>],
    properties: [
        Name: String { get: |this: &Rc<Page>| this.name() },
    ],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use ferroui_base::metadata::MarkupTyped;

    #[test]
    fn a_page_has_its_name_and_is_declared_to_markup() {
        let markup = <Page as MarkupTyped>::MARKUP;
        assert!(markup.find_property("Name").is_some());
    }
}
