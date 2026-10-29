use crate::controls::Document;
use ferrodock_model::core::*;
use ferroui_system::component_model::INotifyPropertyChanged;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn id_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = "changed".to_owned();
    doc.set_id(first.clone());
    doc.set_id(first.clone());
    assert_eq!(doc.id(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_id(String::new());
    assert_eq!(*changes.borrow(), ["Id", "Id"]);
}

#[test]
fn title_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = "changed".to_owned();
    doc.set_title(first.clone());
    doc.set_title(first.clone());
    assert_eq!(doc.title(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_title(String::new());
    assert_eq!(*changes.borrow(), ["Title", "Title"]);
}

#[test]
fn is_empty_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = true;
    doc.set_is_empty(first);
    doc.set_is_empty(first);
    assert_eq!(doc.is_empty(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_is_empty(!first);
    assert_eq!(*changes.borrow(), ["IsEmpty", "IsEmpty"]);
}

#[test]
fn is_collapsable_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = false;
    doc.set_is_collapsable(first);
    doc.set_is_collapsable(first);
    assert_eq!(doc.is_collapsable(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_is_collapsable(!first);
    assert_eq!(*changes.borrow(), ["IsCollapsable", "IsCollapsable"]);
}

#[test]
fn dock_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = DockMode::Right;
    doc.set_dock(first);
    doc.set_dock(first);
    assert_eq!(doc.dock(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_dock(DockMode::Bottom);
    assert_eq!(*changes.borrow(), ["Dock", "Dock"]);
}

#[test]
fn column_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = -2;
    doc.set_column(first);
    doc.set_column(first);
    assert_eq!(doc.column(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_column(0);
    assert_eq!(*changes.borrow(), ["Column", "Column"]);
}

#[test]
fn row_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = -2;
    doc.set_row(first);
    doc.set_row(first);
    assert_eq!(doc.row(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_row(0);
    assert_eq!(*changes.borrow(), ["Row", "Row"]);
}

#[test]
fn column_span_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = -2;
    doc.set_column_span(first);
    doc.set_column_span(first);
    assert_eq!(doc.column_span(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_column_span(0);
    assert_eq!(*changes.borrow(), ["ColumnSpan", "ColumnSpan"]);
}

#[test]
fn row_span_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = -2;
    doc.set_row_span(first);
    doc.set_row_span(first);
    assert_eq!(doc.row_span(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_row_span(0);
    assert_eq!(*changes.borrow(), ["RowSpan", "RowSpan"]);
}

#[test]
fn is_shared_size_scope_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments(
) {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = true;
    doc.set_is_shared_size_scope(first);
    doc.set_is_shared_size_scope(first);
    assert_eq!(doc.is_shared_size_scope(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_is_shared_size_scope(!first);
    assert_eq!(
        *changes.borrow(),
        ["IsSharedSizeScope", "IsSharedSizeScope"]
    );
}

#[test]
fn can_close_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = false;
    doc.set_can_close(first);
    doc.set_can_close(first);
    assert_eq!(doc.can_close(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_can_close(!first);
    assert_eq!(*changes.borrow(), ["CanClose", "CanClose"]);
}

#[test]
fn can_pin_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = false;
    doc.set_can_pin(first);
    doc.set_can_pin(first);
    assert_eq!(doc.can_pin(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_can_pin(!first);
    assert_eq!(*changes.borrow(), ["CanPin", "CanPin"]);
}

#[test]
fn keep_pinned_dockable_visible_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments(
) {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = true;
    doc.set_keep_pinned_dockable_visible(first);
    doc.set_keep_pinned_dockable_visible(first);
    assert_eq!(doc.keep_pinned_dockable_visible(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_keep_pinned_dockable_visible(!first);
    assert_eq!(
        *changes.borrow(),
        ["KeepPinnedDockableVisible", "KeepPinnedDockableVisible"]
    );
}

#[test]
fn pinned_dock_display_mode_override_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments(
) {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = Some(PinnedDockDisplayMode::Inline);
    doc.set_pinned_dock_display_mode_override(first);
    doc.set_pinned_dock_display_mode_override(first);
    assert_eq!(doc.pinned_dock_display_mode_override(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_pinned_dock_display_mode_override(None);
    assert_eq!(
        *changes.borrow(),
        [
            "PinnedDockDisplayModeOverride",
            "PinnedDockDisplayModeOverride"
        ]
    );
}

#[test]
fn can_float_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = false;
    doc.set_can_float(first);
    doc.set_can_float(first);
    assert_eq!(doc.can_float(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_can_float(!first);
    assert_eq!(*changes.borrow(), ["CanFloat", "CanFloat"]);
}

#[test]
fn can_drag_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = false;
    doc.set_can_drag(first);
    doc.set_can_drag(first);
    assert_eq!(doc.can_drag(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_can_drag(!first);
    assert_eq!(*changes.borrow(), ["CanDrag", "CanDrag"]);
}

#[test]
fn can_drop_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = false;
    doc.set_can_drop(first);
    doc.set_can_drop(first);
    assert_eq!(doc.can_drop(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_can_drop(!first);
    assert_eq!(*changes.borrow(), ["CanDrop", "CanDrop"]);
}

#[test]
fn can_dock_as_document_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments(
) {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = false;
    doc.set_can_dock_as_document(first);
    doc.set_can_dock_as_document(first);
    assert_eq!(doc.can_dock_as_document(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_can_dock_as_document(!first);
    assert_eq!(
        *changes.borrow(),
        ["CanDockAsDocument", "CanDockAsDocument"]
    );
}

#[test]
fn dock_capability_overrides_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments(
) {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = Some(DockCapabilityOverrides::new());
    doc.set_dock_capability_overrides(first.clone());
    doc.set_dock_capability_overrides(first.clone());
    assert!(Rc::ptr_eq(
        &doc.dock_capability_overrides().unwrap(),
        first.as_ref().unwrap()
    ));
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_dock_capability_overrides(None);
    assert_eq!(
        *changes.borrow(),
        ["DockCapabilityOverrides", "DockCapabilityOverrides"]
    );
}

#[test]
fn is_modified_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = true;
    doc.set_is_modified(first);
    doc.set_is_modified(first);
    assert_eq!(doc.is_modified(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_is_modified(!first);
    assert_eq!(*changes.borrow(), ["IsModified", "IsModified"]);
}

#[test]
fn dock_group_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = Some("changed".to_owned());
    doc.set_dock_group(first.clone());
    doc.set_dock_group(first.clone());
    assert_eq!(doc.dock_group(), first);
    let erased: Rc<dyn IDockable> = doc.clone();
    erased.set_dock_group(None);
    assert_eq!(*changes.borrow(), ["DockGroup", "DockGroup"]);
}

#[test]
fn allowed_dock_operations_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments(
) {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = DockOperationMask::NONE;
    doc.set_allowed_dock_operations(first);
    doc.set_allowed_dock_operations(first);
    assert_eq!(doc.allowed_dock_operations(), first);
    let erased: Rc<dyn IDockableDockingRestrictions> = doc.clone();
    erased.set_allowed_dock_operations(DockOperationMask::ALL);
    assert_eq!(
        *changes.borrow(),
        ["AllowedDockOperations", "AllowedDockOperations"]
    );
}

#[test]
fn allowed_drop_operations_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments(
) {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = DockOperationMask::NONE;
    doc.set_allowed_drop_operations(first);
    doc.set_allowed_drop_operations(first);
    assert_eq!(doc.allowed_drop_operations(), first);
    let erased: Rc<dyn IDockableDockingRestrictions> = doc.clone();
    erased.set_allowed_drop_operations(DockOperationMask::ALL);
    assert_eq!(
        *changes.borrow(),
        ["AllowedDropOperations", "AllowedDropOperations"]
    );
}

#[test]
fn show_in_selector_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = false;
    doc.set_show_in_selector(first);
    doc.set_show_in_selector(first);
    assert_eq!(doc.show_in_selector(), first);
    let erased: Rc<dyn IDockSelectorInfo> = doc.clone();
    erased.set_show_in_selector(!first);
    assert_eq!(*changes.borrow(), ["ShowInSelector", "ShowInSelector"]);
}

#[test]
fn selector_title_is_shared_between_inherent_and_interface_access_and_skips_equal_assignments() {
    let doc = Document::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    doc.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    let first = Some("changed".to_owned());
    doc.set_selector_title(first.clone());
    doc.set_selector_title(first.clone());
    assert_eq!(doc.selector_title(), first);
    let erased: Rc<dyn IDockSelectorInfo> = doc.clone();
    erased.set_selector_title(None);
    assert_eq!(*changes.borrow(), ["SelectorTitle", "SelectorTitle"]);
}
