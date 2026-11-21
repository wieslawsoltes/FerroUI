//! Source EventHandler equality uses target identity and method, not callback allocation.
use ferroui_base::{
    BoxedValue,
    data::core::ValueTypes,
    ferro_markup_type,
    metadata::{MarkupDelegate, MarkupTyped},
};
use std::rc::Rc;
pub struct Listener;
impl PartialEq for Listener {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Listener {
    pub fn first(&self) {}
    pub fn second(&self) {}
    pub fn static_handler() {}
}
ferro_markup_type!(class Listener {
    namespace:"FerroRibbon.FrameworkRepro",this:Rc<Listener>,handles:[Listener,Rc<Listener>],
    methods:[
        fn First()=>|this:&Rc<Listener>|this.first(),
        fn Second()=>|this:&Rc<Listener>|this.second(),
        static fn Static()=>Listener::static_handler,
    ],
});
#[test]
fn separately_created_method_delegates_compare_by_target_identity_and_method() {
    ValueTypes::register_reference::<Listener>();
    let target: BoxedValue = Rc::new(Listener);
    let other: BoxedValue = Rc::new(Listener);
    let method = Listener::MARKUP.find_methods("First").next().unwrap();
    let first = MarkupDelegate::for_method(Some(target.clone()), Listener::MARKUP, method);
    let same = MarkupDelegate::for_method(Some(target.clone()), Listener::MARKUP, method);
    assert_eq!(
        first, same,
        "two EventHandlers for the same method and target must compare equal"
    );
    assert_ne!(
        first,
        MarkupDelegate::for_method(Some(other), Listener::MARKUP, method),
        "overridden target Equals does not define delegate equality"
    );
    assert_ne!(
        first,
        MarkupDelegate::for_method(
            Some(target),
            Listener::MARKUP,
            Listener::MARKUP.find_methods("Second").next().unwrap()
        )
    );
    let method = Listener::MARKUP.find_methods("Static").next().unwrap();
    assert_eq!(
        MarkupDelegate::for_method(None, Listener::MARKUP, method),
        MarkupDelegate::for_method(None, Listener::MARKUP, method)
    );
    assert_ne!(MarkupDelegate::new(|_| None), MarkupDelegate::new(|_| None));
}
