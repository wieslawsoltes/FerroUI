//! Native search UI port. Copyright Wiesław Šoltés, MIT.
use ferroribbon_core::search::RibbonSearchEntry;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::{cell::RefCell, rc::Rc};

/// Shared query contributions. Mutations are ordered and intentionally do not
/// publish collection notifications, matching the query-event contract.
pub type RibbonQuerySearchEntries = Rc<RefCell<Vec<Option<Rc<RibbonSearchEntry>>>>>;

pub struct RibbonSearchQueryEventArgs {
    query: Option<String>,
    additional_entries: RibbonQuerySearchEntries,
}
impl PartialEq for RibbonSearchQueryEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl RibbonSearchQueryEventArgs {
    pub fn new(query: Option<String>) -> Rc<Self> {
        Rc::new(Self {
            query,
            additional_entries: Rc::default(),
        })
    }
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }
    pub fn additional_entries(&self) -> RibbonQuerySearchEntries {
        self.additional_entries.clone()
    }
}
pub struct RibbonSearchQueryEventArgsMetadata;
ferroui_base::ferro_markup_type!(class RibbonSearchQueryEventArgsMetadata as "RibbonSearchQueryEventArgs" {
    namespace:"FerroRibbon.Controls", this:Rc<RibbonSearchQueryEventArgs>,
    handles:[Rc<RibbonSearchQueryEventArgs>,Option<Rc<RibbonSearchQueryEventArgs>>,RibbonSearchQueryEventArgs],
    constructors:[(Option<String>)=>RibbonSearchQueryEventArgs::new],
    properties:[Query:Option<String>{get:|args:&Rc<RibbonSearchQueryEventArgs>|args.query().map(str::to_owned)}]
});

pub(crate) fn register() {
    ValueTypes::register_reference::<RibbonSearchQueryEventArgs>();
    MarkupType::register_all(&[
        RibbonSearchQueryEventArgsMetadata::MARKUP,
        super::RibbonCommandPalette::MARKUP,
    ]);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_contributions_share_mutations_and_preserve_order_and_identity() {
        let args = RibbonSearchQueryEventArgs::new(None);
        assert_eq!(args.query(), None);
        let first = args.additional_entries();
        let second = args.additional_entries();
        assert!(Rc::ptr_eq(&first, &second));
        let entry = RibbonSearchEntry::new(Some("id".into()), Some("Label".into()));
        first
            .borrow_mut()
            .extend([None, Some(entry.clone()), Some(entry.clone())]);
        assert_eq!(second.borrow().len(), 3);
        assert!(second.borrow()[0].is_none());
        assert!(Rc::ptr_eq(second.borrow()[1].as_ref().unwrap(), &entry));
        second.borrow_mut().remove(1);
        assert_eq!(first.borrow().len(), 2);
        second.borrow_mut().clear();
        assert!(first.borrow().is_empty());
    }
    #[test]
    fn native_query_preserves_unicode_and_compiled_event_metadata() {
        crate::register_types();
        let query = "Otwórz e\u{301} 🦀 العربية";
        let args = RibbonSearchQueryEventArgs::new(Some(query.into()));
        assert_eq!(args.query(), Some(query));
        let metadata = RibbonSearchQueryEventArgsMetadata::MARKUP;
        let value = (metadata.find_property("Query").unwrap().get.unwrap())(&[Some(args)]).unwrap();
        assert_eq!(
            ferroui_base::metadata::from_markup_value::<Option<String>>(&value)
                .unwrap()
                .as_deref(),
            Some(query)
        );
        assert!(metadata.find_property("Query").unwrap().set.is_none());
    }
}
