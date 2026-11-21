//! Source auto-events expose nullable add/remove accessors, without an instance.
use ferroui_base::{
    ferro_markup_type, ferro_static_type,
    metadata::{MarkupDelegate, MarkupTyped},
};
pub struct Events;
ferro_static_type!(Events);
impl Events {
    pub fn add(_: Option<MarkupDelegate>) {}
    pub fn remove(_: Option<MarkupDelegate>) {}
}
ferro_markup_type!(static Events {type_info:Events,
    events:[static Changed() {add:Events::add,remove:Events::remove}],
});
#[test]
fn source_auto_event_add_accepts_null() {
    let event = Events::MARKUP.find_event("Changed").unwrap();
    assert!(
        (event.add)(&[None]).is_ok(),
        "source event addition accepts null"
    );
}
#[test]
fn source_auto_event_has_public_accessor_methods() {
    assert!(
        Events::MARKUP.find_methods("add_Changed").next().is_some(),
        "missing public add accessor"
    );
    assert!(
        Events::MARKUP
            .find_methods("remove_Changed")
            .next()
            .is_some(),
        "missing public remove accessor"
    );
}

#[test]
fn source_auto_event_remove_descriptor_accepts_null() {
    let event = Events::MARKUP.find_event("Changed").unwrap();
    assert!(event.handler_nullable && event.is_static);
    assert_eq!((event.remove.unwrap())(&[None]), Ok(None));
    for name in ["add_Changed", "remove_Changed"] {
        let method = Events::MARKUP.find_methods(name).next().unwrap();
        assert!(method.is_static && method.return_type.is_none());
        assert_eq!((method.invoke)(&[None]), Ok(None));
    }
}
