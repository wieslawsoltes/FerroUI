//! Port of `Converters/ConverterTests.cs`.

use std::rc::Rc;

use ferroui_base::metadata::MarkupTyped;

use crate::support::app::xaml_test_base;
use crate::support::converters::converter_tests::TestClassWithUri;
use crate::support::loader::parse_local;

#[test]
fn bug_2228_relative_uris_should_be_correctly_parsed() {
    let _base = xaml_test_base();
    let test_class = <TestClassWithUri as MarkupTyped>::MARKUP;
    let parsed = parse_local::<Rc<TestClassWithUri>>(&format!(
        "<{} xmlns='clr-namespace:{}' Uri='/test'/>",
        test_class.name,
        test_class.namespace()
    ));

    let uri = parsed.uri();
    assert!(uri.is_some());
    assert!(!uri.unwrap().is_absolute_uri());
}
