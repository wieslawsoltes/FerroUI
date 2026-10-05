use crate::presenters::ScrollContentPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{test_scope, TestRoot};
use crate::test_support_scrolling::TestScrollable;
use crate::{Control, Panel, ScrollViewer};
use ferroui_base::{Ref, Size, Vector};

fn create_target(content: &Ref<TestScrollable>) -> (Ref<ScrollViewer>, Ref<TestRoot>) {
    let result = ScrollViewer::new();
    result.set_content(Some(Control::boxed(content)));
    result.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));

        let panel = Panel::new();
        panel.children().add(presenter.register_in_name_scope(&**scope));
        panel.upcast()
    })));

    let root = TestRoot::with_child(&result);
    root.layout_manager().execute_initial_layout_pass();

    (result, root)
}

#[test]
fn extent_offset_and_viewport_should_be_read_from_i_logical_scrollable() {
    let _scope = test_scope();
    let scrollable =
        TestScrollable::with_state(Size::new(100.0, 100.0), Vector::new(50.0, 50.0), Size::new(25.0, 25.0));

    let (target, _root) = create_target(&scrollable);

    assert_eq!(scrollable.extent(), target.extent());
    assert_eq!(scrollable.offset(), target.offset());
    assert_eq!(scrollable.viewport(), target.viewport());

    scrollable.set_extent(Size::new(200.0, 200.0));
    scrollable.set_offset(Vector::new(100.0, 100.0));
    scrollable.set_viewport(Size::new(50.0, 50.0));

    assert_eq!(scrollable.extent(), target.extent());
    assert_eq!(scrollable.offset(), target.offset());
    assert_eq!(scrollable.viewport(), target.viewport());
}
