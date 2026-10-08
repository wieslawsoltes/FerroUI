//! Not from upstream: a content presenter that recognizes access keys
//! shows string content in an access text. Upstream's content presenter
//! suites are ported exactly in `content_presenter_tests_*.rs`.

use crate::presenters::ContentPresenter;
use crate::test_support::{boxed_str, test_scope, TestRoot};

#[test]
fn recognizes_access_key_should_create_access_text() {
    let _scope = test_scope();
    let presenter = ContentPresenter::new();
    presenter.set_recognizes_access_key(true);
    let _root = TestRoot::with_child(&presenter);

    presenter.set_content(boxed_str("_Foo"));
    presenter.update_child();

    let access_text = presenter.child().and_then(|child| child.cast::<crate::primitives::AccessText>()).unwrap();
    assert_eq!(access_text.text().as_deref(), Some("_Foo"));
    assert_eq!(access_text.access_key().as_deref(), Some("F"));
}
