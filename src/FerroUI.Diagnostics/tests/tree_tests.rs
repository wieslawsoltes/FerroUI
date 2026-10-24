//! Not from upstream: the upstream project has no tests. The nodes of the
//! logical and visual trees and their children following the trees, the
//! collection of the children, the filter view model, the observable of a
//! property of a view model, the selection of a control in a tree page, and
//! the rows of the TreeDataGrid of a tree page (the port shows the nodes in a
//! TreeDataGrid where upstream has a tree view).

use crate::diagnostics::view_models::{
    CombinedTreeNode, FilterViewModel, LogicalTreeNode, ReactiveExtensions, TreeNodeCollection, TreeNodeKind,
    TreePageViewModel, VisualTreeNode,
};
use crate::diagnostics::views::TreePageView;
use ferroui_base::data::model::{CollectionChange, INotifyCollectionChanged, INotifyDataErrorInfo, INotifyPropertyChanged};
use ferroui_base::media::FontWeight;
use ferroui_base::reactive::ObservableExt;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::{FerroObject, Ref};
use ferroui_controls::{Border, Button, Control, IItemsList, StackPanel, TextBlock};
use ferroui_treedatagrid_core::models::IRows;
use ferroui_treedatagrid_core::IndexPath;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::{Rc, Weak};

fn scope() -> UnitTestDispatcherScope {
    // The names the tools give properties and types read the markup metadata of the process: it is
    // registered before the test, not part way through it by a test of another thread.
    crate::register_types();
    Dispatcher::unit_test_scope()
}

fn object<T: ferroui_base::ObjectType + ferroui_base::Upcast<FerroObject>>(value: &Ref<T>) -> Ref<FerroObject> {
    value.clone().upcast()
}

fn types(nodes: &Rc<TreeNodeCollection>) -> Vec<String> {
    nodes.to_vec().iter().map(|node| node.type_()).collect()
}

/// A panel with a named border that has a class, and a button.
fn panel() -> (Ref<StackPanel>, Ref<Border>, Ref<Button>) {
    let panel = StackPanel::new();
    let border = Border::new();
    border.set_name(Some("border".to_string()));
    border.classes().add("foo");
    let button = Button::new();
    panel.children().add(border.clone());
    panel.children().add(button.clone());
    (panel, border, button)
}

#[test]
fn a_logical_tree_node_shows_the_object_and_its_logical_children() {
    let _scope = scope();
    let (panel, _border, _button) = panel();

    let root = LogicalTreeNode::new(object(&panel), None);

    assert_eq!(root.type_(), "StackPanel");
    assert_eq!(root.kind(), TreeNodeKind::Logical);
    assert!(root.parent().is_none());
    assert!(root.visual().ptr_eq(&panel));
    assert!(root.font_weight() == FontWeight::Normal);
    assert!(!root.is_in_template());
    assert_eq!(types(&root.children()), ["Border", "Button"]);

    let border_node = root.children().get(0);
    assert_eq!(border_node.element_name().as_deref(), Some("border"));
    assert_eq!(border_node.classes(), "(foo)");
    assert!(border_node.parent().is_some_and(|parent| Rc::ptr_eq(&parent, &root)));
}

#[test]
fn create_makes_one_root_node_or_none() {
    let _scope = scope();
    let (panel, ..) = panel();

    assert_eq!(LogicalTreeNode::create(object(&panel)).len(), 1);
    assert_eq!(VisualTreeNode::create(object(&panel)).len(), 1);
    assert!(LogicalTreeNode::create(None::<Ref<FerroObject>>).is_empty());
    assert!(VisualTreeNode::create(None::<Ref<FerroObject>>).is_empty());
}

#[test]
fn the_children_of_a_logical_tree_node_follow_the_logical_tree() {
    let _scope = scope();
    let (panel, border, _button) = panel();
    let root = LogicalTreeNode::new(object(&panel), None);
    let children = root.children();
    assert_eq!(children.count(), 2);

    panel.children().insert(1, TextBlock::new());
    assert_eq!(types(&children), ["Border", "TextBlock", "Button"]);

    panel.children().remove(border.clone());
    assert_eq!(types(&children), ["TextBlock", "Button"]);

    panel.children().clear();
    assert_eq!(children.count(), 0);
}

#[test]
fn the_classes_of_a_node_follow_the_classes_of_the_element_until_it_is_disposed() {
    let _scope = scope();
    let (_panel, border, _button) = panel();
    let node = LogicalTreeNode::new(object(&border), None);
    let changes = Rc::new(RefCell::new(Vec::new()));
    let sink = changes.clone();
    node.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

    border.classes().add("bar");
    assert_eq!(node.classes(), "(foo bar)");
    border.classes().remove("foo");
    assert_eq!(node.classes(), "(bar)");
    assert_eq!(*changes.borrow(), ["Classes", "Classes"]);

    node.dispose();
    border.classes().add("baz");
    assert_eq!(node.classes(), "(bar)");
}

#[test]
fn is_expanded_notifies_its_changes() {
    let _scope = scope();
    let node = LogicalTreeNode::new(object(&Border::new()), None);
    let changes = Rc::new(Cell::new(0));
    let sink = changes.clone();
    node.property_changed().add(Rc::new(move |name: &str| {
        assert_eq!(name, "IsExpanded");
        sink.set(sink.get() + 1);
    }));

    node.set_is_expanded(true);
    node.set_is_expanded(true);
    node.set_is_expanded(false);

    assert!(!node.is_expanded());
    assert_eq!(changes.get(), 2);
}

#[test]
fn a_visual_tree_node_shows_the_visual_children() {
    let _scope = scope();
    let border = Border::new();
    let text = TextBlock::new();
    border.set_child(text.clone());

    let root = VisualTreeNode::new(object(&border), None, None);
    assert_eq!(root.kind(), TreeNodeKind::Visual { is_in_template: false });
    assert_eq!(types(&root.children()), ["TextBlock"]);

    let replacement = Button::new();
    border.set_child(replacement.clone());
    assert_eq!(types(&root.children()), ["Button"]);

    border.set_child(None::<Ref<Control>>);
    assert_eq!(root.children().count(), 0);
}

#[test]
fn a_visual_tree_node_has_the_name_it_was_created_with_and_knows_whether_it_is_in_a_template() {
    let _scope = scope();
    let button = Button::new();
    let text = TextBlock::new();
    text.set_templated_parent(button.clone().upcast::<FerroObject>());

    let node = VisualTreeNode::new(object(&text), None, Some("Popup (TextBlock)".to_string()));

    assert_eq!(node.type_(), "Popup (TextBlock)");
    assert!(node.is_in_template());
}

#[test]
fn a_node_of_an_object_without_children_has_the_empty_collection() {
    let _scope = scope();
    let object: Ref<FerroObject> = FerroObject::new();

    let logical = LogicalTreeNode::new(object.clone(), None);
    let visual = VisualTreeNode::new(object, None, None);

    assert!(Rc::ptr_eq(&logical.children(), &TreeNodeCollection::empty()));
    assert!(Rc::ptr_eq(&visual.children(), &TreeNodeCollection::empty()));
    assert_eq!(TreeNodeCollection::empty().count(), 0);
    assert!(TreeNodeCollection::empty().owner().is_none());
}

#[test]
fn a_tree_node_collection_is_filled_when_it_is_first_used_and_notifies_its_changes() {
    let _scope = scope();
    let (panel, border, _button) = panel();
    let root = LogicalTreeNode::new(object(&panel), None);
    let children = root.children();
    assert!(children.owner().is_some_and(|owner| Rc::ptr_eq(&owner, &root)));

    // Subscribing fills the collection.
    let changes = Rc::new(RefCell::new(Vec::new()));
    let sink = changes.clone();
    children.collection_changed().add(Rc::new(move |e: &CollectionChange| sink.borrow_mut().push(e.action)));
    assert_eq!(IItemsList::count(&*children), 2);

    panel.children().add(TextBlock::new());
    assert_eq!(changes.borrow().len(), 1);

    let first = children.get(0);
    assert!(children.contains(&first));
    assert_eq!(children.index_of(&first), 0);
    let item = <TreeNodeCollection as IItemsList>::get_at(&children, 0);
    assert!(item.is_some());
    assert_eq!(<TreeNodeCollection as IItemsList>::index_of(&children, &item), 0);

    // Disposing the collection disposes the nodes: their classes stop following the elements.
    children.dispose();
    border.classes().add("bar");
    assert_eq!(first.classes(), "(foo)");
}

#[test]
fn the_filter_matches_text_case_insensitively_by_default() {
    let filter = FilterViewModel::new();
    assert!(filter.filter("anything"));

    filter.set_filter_string("but".to_string());
    assert!(filter.filter("Button"));
    assert!(!filter.filter("Border"));

    // The text is not a regular expression unless asked.
    filter.set_filter_string("B.*n".to_string());
    assert!(!filter.filter("Button"));
    assert!(filter.filter("AB.*nC"));
}

#[test]
fn the_filter_can_be_case_sensitive_and_match_whole_words() {
    let filter = FilterViewModel::new();
    filter.set_filter_string("but".to_string());
    filter.set_use_case_sensitive_filter(true);
    assert!(!filter.filter("Button"));
    assert!(filter.filter("button"));

    filter.set_use_case_sensitive_filter(false);
    filter.set_filter_string("Back".to_string());
    filter.set_use_whole_word_filter(true);
    assert!(!filter.filter("Background"));
    assert!(filter.filter("Go back now"));
}

#[test]
fn the_filter_uses_regular_expressions_and_reports_the_ones_that_do_not_parse() {
    let filter = FilterViewModel::new();
    let refreshes = Rc::new(Cell::new(0));
    let sink = refreshes.clone();
    let subscription = filter.refresh_filter(move || sink.set(sink.get() + 1));
    let errors = Rc::new(RefCell::new(Vec::new()));
    let sink = errors.clone();
    filter.errors_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

    filter.set_use_regex_filter(true);
    filter.set_filter_string("^B.*n$".to_string());
    assert!(filter.filter("Button"));
    assert!(!filter.filter("Border"));
    assert!(!filter.has_errors());

    filter.set_filter_string("(".to_string());
    assert!(filter.has_errors());
    assert_eq!(filter.get_errors(Some("FilterString")).len(), 1);
    assert!(filter.get_errors(None).is_empty());
    assert_eq!(INotifyDataErrorInfo::get_errors(&*filter, Some("FilterString")).len(), 1);
    // The previous expression stays in effect.
    assert!(filter.filter("Button"));

    filter.set_filter_string("B".to_string());
    assert!(!filter.has_errors());
    assert_eq!(*errors.borrow(), ["FilterString", "FilterString"]);
    assert_eq!(refreshes.get(), 4);

    subscription.dispose();
    filter.set_filter_string("C".to_string());
    assert_eq!(refreshes.get(), 4);
}

#[test]
fn get_observable_produces_the_values_of_a_property_of_a_view_model() {
    let filter = FilterViewModel::new();
    let values = Rc::new(RefCell::new(Vec::new()));
    let sink = values.clone();
    let subscription = ReactiveExtensions::get_observable(&filter, "FilterString", |vm: &FilterViewModel| vm.filter_string(), true)
        .subscribe_fn(move |value: String| sink.borrow_mut().push(value));

    filter.set_filter_string("a".to_string());
    filter.set_use_regex_filter(true);
    filter.set_filter_string("b".to_string());
    subscription.dispose();
    filter.set_filter_string("c".to_string());

    assert_eq!(*values.borrow(), ["", "a", "b"]);

    let values = Rc::new(RefCell::new(Vec::new()));
    let sink = values.clone();
    let _subscription = ReactiveExtensions::get_observable(&filter, "UseRegexFilter", |vm: &FilterViewModel| vm.use_regex_filter(), false)
        .subscribe_fn(move |value: bool| sink.borrow_mut().push(value));
    filter.set_use_regex_filter(false);
    assert_eq!(*values.borrow(), [false]);
}

/// A panel with a border around a text block: the trees of a tree page.
fn tree_page() -> (Rc<TreePageViewModel>, Ref<StackPanel>, Ref<Border>, Ref<TextBlock>) {
    let panel = StackPanel::new();
    let border = Border::new();
    let text = TextBlock::new();
    text.set_name(Some("text".to_string()));
    text.classes().add("title");
    border.set_child(text.clone());
    panel.children().add(border.clone());

    let page = TreePageViewModel::with_weak_main_view(
        Weak::new(),
        LogicalTreeNode::create(object(&panel)),
        Rc::new(RefCell::new(HashSet::new())),
    );
    (page, panel, border, text)
}

#[test]
fn a_tree_page_finds_the_node_of_a_control() {
    let _scope = scope();
    let (page, panel, _border, text) = tree_page();

    assert_eq!(page.nodes().count(), 1);
    let node = page.find_node(&text.clone().upcast()).expect("the node of the text block");
    assert_eq!(node.type_(), "TextBlock");
    assert!(page.find_node(&panel.clone().upcast()).is_some_and(|root| Rc::ptr_eq(&root, &page.nodes().get(0))));
    assert!(page.find_node(&Button::new().upcast()).is_none());
}

#[test]
fn selecting_a_control_selects_its_node_and_expands_its_ancestors() {
    let _scope = scope();
    let (page, _panel, _border, text) = tree_page();
    let selected = Rc::new(Cell::new(0));
    let sink = selected.clone();
    page.property_changed().add(Rc::new(move |name: &str| {
        if name == "SelectedNode" {
            sink.set(sink.get() + 1);
        }
    }));

    page.select_control(&text.clone().upcast());

    let node = page.selected_node().expect("a selected node");
    assert!(node.visual().ptr_eq(&text));
    assert_eq!(selected.get(), 1);
    assert!(!node.is_expanded());
    let border_node = node.parent().expect("the node of the border");
    assert!(border_node.is_expanded());
    assert!(border_node.parent().is_some_and(|root| root.is_expanded()));

    page.collapse_children();
    assert!(!node.is_expanded());

    // The row of a node without children does not stay expanded (the TreeDataGrid of the page
    // writes the state of the row back to the node), so the nodes with children are the ones
    // expand_recursively leaves expanded.
    page.set_selected_node(Some(border_node.clone()));
    page.collapse_children();
    assert!(!border_node.is_expanded());
    page.expand_recursively();
    assert!(border_node.is_expanded());
}

#[test]
fn the_rows_of_a_tree_page_list_the_children_of_a_node_when_it_is_expanded() {
    let _scope = scope();
    let (page, _panel, border, text) = tree_page();
    let root = page.nodes().get(0);
    let rows = page.nodes_source().typed_rows();

    // The row of the root only: the children of the root are not listed.
    assert_eq!(1, IRows::count(&*rows));
    assert!(!root.children().is_initialized());

    // Expanding the root lists its children and makes their rows; the children of the border are
    // not listed.
    root.set_is_expanded(true);
    assert_eq!(2, IRows::count(&*rows));
    assert!(root.children().is_initialized());
    let border_node = root.children().get(0);
    assert!(border_node.visual().ptr_eq(&border));
    assert!(!border_node.children().is_initialized());
    assert!(Rc::ptr_eq(rows.to_vec_typed()[1].model(), &border_node));

    // Expanding a row expands its node.
    rows.to_vec_typed()[1].set_is_expanded(true);
    assert!(border_node.is_expanded());
    assert_eq!(3, IRows::count(&*rows));
    assert!(rows.to_vec_typed()[2].model().visual().ptr_eq(&text));

    // A child added to an expanded node gets a row.
    border.set_child(Button::new());
    assert_eq!(3, IRows::count(&*rows));
    assert_eq!("Button", rows.to_vec_typed()[2].model().type_());

    // Collapsing a node collapses its row.
    root.set_is_expanded(false);
    assert_eq!(1, IRows::count(&*rows));
}

#[test]
fn selecting_a_control_selects_the_row_of_its_node_and_expands_the_rows_of_its_ancestors() {
    let _scope = scope();
    let (page, _panel, _border, text) = tree_page();
    let source = page.nodes_source();
    let rows = source.typed_rows();
    assert_eq!(1, IRows::count(&*rows));
    assert_eq!(-1, page.selected_row_index());

    page.select_control(&text.clone().upcast());

    // The rows of the root and of the border were made and expanded; the row of the text block is
    // selected.
    let node = page.selected_node().expect("a selected node");
    assert_eq!(3, IRows::count(&*rows));
    let selection = source.row_selection().expect("the selection of the rows");
    assert!(selection.base().selected_item().is_some_and(|selected| Rc::ptr_eq(&selected, &node)));
    assert!(page.index_path_of(&node) == Some(IndexPath::from_indexes(&[0, 0, 0])));
    assert_eq!(2, page.selected_row_index());

    // Selecting a row selects its node.
    selection.base().set_selected_index(IndexPath::new(0));
    assert!(page.selected_node().is_some_and(|selected| Rc::ptr_eq(&selected, &page.nodes().get(0))));
    assert_eq!(0, page.selected_row_index());

    // No selected node, no selected row.
    page.set_selected_node(None);
    assert!(selection.base().selected_item().is_none());

    // A node that is not in the trees of the page has no row.
    let other = LogicalTreeNode::new(object(&Border::new()), None);
    assert!(page.index_path_of(&other).is_none());
}

#[test]
fn copying_the_selector_of_the_selected_control_requests_a_copy() {
    let _scope = scope();
    let (page, _panel, _border, text) = tree_page();
    let copied = Rc::new(RefCell::new(Vec::new()));
    let sink = copied.clone();
    let _subscription = page.clipboard_copy_requested(move |selector: &str| sink.borrow_mut().push(selector.to_string()));

    page.copy_selector();
    assert!(copied.borrow().is_empty());

    page.select_control(&text.clone().upcast());
    page.copy_selector();

    let selector = copied.borrow()[0].clone();
    assert!(selector.starts_with('{'));
    assert!(selector.contains("|TextBlock#text.title"), "{selector}");
    assert_eq!(TreePageView::to_text(&selector), selector[selector.find('}').unwrap() + 1..]);
}

#[test]
fn to_text_removes_the_parts_in_braces() {
    assert_eq!(
        TreePageView::to_text("{FerroUI.Controls}FerroUI.Controls|Button#ok.primary:pointerover"),
        "FerroUI.Controls|Button#ok.primary:pointerover"
    );
    assert_eq!(TreePageView::to_text("a}b"), "a}b");
    assert_eq!(TreePageView::to_text("{x}A /template/ {y}B"), "A /template/ B");
}

#[test]
fn the_nodes_of_a_tree_page_are_disposed_with_it() {
    let _scope = scope();
    let (page, _panel, _border, text) = tree_page();
    let node = page.find_node(&text.clone().upcast()).expect("the node of the text block");

    page.dispose();
    text.classes().add("other");

    assert_eq!(node.classes(), "(title)");
}

/// A button with a template made in code (a border around a content presenter) and a text block
/// as its content.
fn templated_button() -> (Ref<Button>, Ref<TextBlock>) {
    use ferroui_base::data::TemplateBinding;
    use ferroui_controls::presenters::ContentPresenter;
    use ferroui_controls::templates::FuncControlTemplate;
    use ferroui_controls::ContentControl;

    let button = Button::new();
    button.set_template(Some(FuncControlTemplate::new(|_, _scope| {
        let border = Border::new();
        border.set_name(Some("PART_Border".to_string()));
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let content = ContentControl::content_property().as_property();
        presenter.bind_binding(content, &TemplateBinding::new(content));
        border.set_child(Some(presenter.upcast::<Control>()));
        border.upcast()
    })));
    let text = TextBlock::new();
    text.set_text(Some("content"));
    button.set_content(Some(Control::boxed(text.clone())));
    button.apply_template();
    if let Some(presenter) = button.get_visual_descendants().find_map(|v| v.cast::<ferroui_controls::presenters::ContentPresenter>()) {
        presenter.update_child();
    }
    (button, text)
}

// Deviation (DEVIATIONS.md, Developer tools): the combined tree, an addition of the port.
#[test]
fn the_combined_tree_shows_the_parts_of_a_template_greyed_out_before_the_logical_children() {
    let _scope = scope();
    let panel = StackPanel::new();
    let (button, text) = templated_button();
    panel.children().add(button.clone());

    let root = CombinedTreeNode::new_logical(object(&panel), None);
    assert_eq!(root.kind(), TreeNodeKind::Combined { visual_only: false });
    assert!(!root.is_visual_only());
    // The children of the panel are logical: no part of a template.
    assert_eq!(types(&root.children()), vec!["Button"]);

    let button_node = root.children().to_vec()[0].clone();
    let children = button_node.children().to_vec();
    // The root of the template, greyed out, then the content: a logical child of the button.
    assert_eq!(children.iter().map(|node| (node.type_(), node.is_visual_only())).collect::<Vec<_>>(),
        vec![("Border".to_string(), true), ("TextBlock".to_string(), false)]);
    assert!(children[1].visual().ptr_eq(&text));
    assert!(children[0].is_in_template());

    // Below the part: the presenter, greyed out; the content it shows is not repeated there.
    let presenter = children[0].children().to_vec();
    assert_eq!(presenter.iter().map(|node| (node.type_(), node.is_visual_only())).collect::<Vec<_>>(),
        vec![("ContentPresenter".to_string(), true)]);
    assert!(presenter[0].children().to_vec().is_empty());

    assert!(CombinedTreeNode::is_logical_child(&text.clone().upcast()));
    root.dispose();
}

#[test]
fn the_combined_tree_follows_a_new_logical_child_and_keeps_the_other_nodes() {
    let _scope = scope();
    let (panel, border, _button) = panel();
    let root = CombinedTreeNode::new_logical(object(&panel), None);
    let before = root.children().to_vec();
    assert_eq!(types(&root.children()), vec!["Border", "Button"]);

    let added = TextBlock::new();
    panel.children().add(added.clone());

    let after = root.children().to_vec();
    assert_eq!(types(&root.children()), vec!["Border", "Button", "TextBlock"]);
    assert!(Rc::ptr_eq(&before[0], &after[0]) && after[0].visual().ptr_eq(&border));
    root.dispose();
}
