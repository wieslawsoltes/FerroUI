//! Children: panels, contents and decorators.

use super::support::*;
use crate::*;

#[test]
fn a_panel_holds_children_and_a_content_one() {
    ui(|| {
        let panel = create("StackPanel");
        let text = create("TextBlock");
        let button = create("Button");
        let label = create("TextBlock");
        assert_eq!(ferro_add_child(panel, text), FERRO_OK, "{}", last_error());
        assert_eq!(ferro_add_child(panel, button), FERRO_OK);
        assert_eq!(ferro_add_child(button, label), FERRO_OK, "{}", last_error());
        assert_eq!(get_handle(button, "Content"), Ok(label));
        assert_eq!(ferro_release(label), FERRO_OK);
        // A content holds one child.
        let other = create("TextBlock");
        assert_eq!(ferro_add_child(button, other), FERRO_INVALID_STATE);
        assert_eq!(ferro_remove_child(button, label), FERRO_OK);
        assert_eq!(ferro_add_child(button, other), FERRO_OK);
        assert_eq!(ferro_remove_child(button, label), FERRO_NOT_FOUND);
        assert_eq!(ferro_remove_child(panel, text), FERRO_OK);
        assert_eq!(ferro_remove_child(panel, text), FERRO_NOT_FOUND);
        // The parent keeps its children alive: the handles can go.
        for handle in [other, label, button, text, panel] {
            assert_eq!(ferro_release(handle), FERRO_OK);
        }
    });
}

#[test]
fn a_decorator_holds_one_child() {
    ui(|| {
        let border = create("Border");
        let first = create("TextBlock");
        let second = create("TextBlock");
        assert_eq!(ferro_add_child(border, first), FERRO_OK, "{}", last_error());
        assert_eq!(ferro_add_child(border, second), FERRO_INVALID_STATE);
        assert_eq!(get_handle(border, "Child"), Ok(first));
        assert_eq!(ferro_release(first), FERRO_OK);
        for handle in [second, first, border] {
            assert_eq!(ferro_release(handle), FERRO_OK);
        }
    });
}

#[test]
fn only_controls_are_children_and_only_containers_hold_them() {
    ui(|| {
        let text = create("TextBlock");
        let other = create("TextBlock");
        let brush = create("SolidColorBrush");
        let panel = create("StackPanel");
        assert_eq!(ferro_add_child(panel, brush), FERRO_TYPE_MISMATCH);
        assert_eq!(ferro_add_child(text, other), FERRO_NOT_SUPPORTED);
        assert_eq!(ferro_add_child(panel, 0), FERRO_BAD_HANDLE);
        for handle in [panel, brush, other, text] {
            assert_eq!(ferro_release(handle), FERRO_OK);
        }
    });
}
