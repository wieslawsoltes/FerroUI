//! The TreeView page: the source's four examples with their code behind (`tree_samples.rs`).

use crate::tree_samples::{explorer_item, explorer_item_templates, ExplorerItemType};
use crate::{examples, tests::start};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{
    Control, FontIcon, Image, ItemsControl, SelectionMode, StackPanel, TextBlock, TreeView, TreeViewItem,
};

fn page() -> Ref<Control> {
    examples::create(examples::find("TreeView").unwrap()).unwrap_or_else(|error| panic!("TreeView: {error}"))
}

/// The TreeViewItems of an items control.
fn nodes(items: &ItemsControl) -> Vec<Ref<TreeViewItem>> {
    items
        .items()
        .view()
        .to_vec()
        .into_iter()
        .flatten()
        .filter_map(|item| Control::from_boxed(&item).and_then(|item| item.cast::<TreeViewItem>()))
        .collect()
}

fn header(item: &TreeViewItem) -> String {
    item.header()
        .and_then(|header| header.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}

#[test]
fn tree_view_sample_trees_have_the_nodes_of_initialize_sample_tree_view() {
    let _application = start();
    let root = page();
    for name in ["sampleTreeView", "sampleTreeView2"] {
        let tree = root.get_control::<TreeView>(name);
        let roots = nodes(&tree);
        assert_eq!(
            roots.iter().map(|node| header(node)).collect::<Vec<_>>(),
            ["Work Documents", "Personal Documents"]
        );
        assert!(roots.iter().all(|node| node.is_expanded()), "{name}");
        let work = nodes(&roots[0]);
        assert_eq!(
            work.iter().map(|node| header(node)).collect::<Vec<_>>(),
            ["XYZ Functional Spec", "Feature Schedule"]
        );
        let remodel = nodes(&roots[1]);
        assert_eq!(remodel.len(), 1);
        assert_eq!(header(&remodel[0]), "Home Remodel");
        assert!(remodel[0].is_expanded());
        assert_eq!(
            nodes(&remodel[0]).iter().map(|node| header(node)).collect::<Vec<_>>(),
            ["Contractor Contact Info", "Paint Color Scheme"]
        );
    }
}

#[test]
fn tree_view_multi_selection_tree_selects_several_items() {
    let _application = start();
    let root = page();
    assert_eq!(
        root.get_control::<TreeView>("sampleTreeView").selection_mode(),
        SelectionMode::SINGLE
    );
    let tree = root.get_control::<TreeView>("sampleTreeView2");
    assert_eq!(tree.selection_mode(), SelectionMode::MULTIPLE | SelectionMode::TOGGLE);
    let roots = nodes(&tree);
    let selected = tree.selected_items();
    selected.add(Some(Control::boxed(roots[0].clone())));
    selected.add(Some(Control::boxed(roots[1].clone())));
    assert_eq!(tree.selected_items().count(), 2);
}

fn data_source(tree: &TreeView) -> Vec<BoxedValue> {
    tree.items_source()
        .expect("the bound tree has items")
        .to_vec()
        .into_iter()
        .flatten()
        .collect()
}

#[test]
fn tree_view_bound_tree_shows_the_names_and_children_of_get_data() {
    let _application = start();
    let root = page();
    let tree = root.get_control::<TreeView>("TreeView1");
    let items = data_source(&tree);
    let folders: Vec<_> = items.iter().filter_map(explorer_item).collect();
    assert_eq!(
        folders.iter().map(|item| item.name).collect::<Vec<_>>(),
        ["Documents", "Projects"]
    );
    assert!(folders.iter().all(|item| item.kind == ExplorerItemType::Folder));
    let files: Vec<_> = folders[0]
        .children
        .to_vec()
        .iter()
        .filter_map(explorer_item)
        .map(|item| item.name)
        .collect();
    assert_eq!(files, ["ProjectProposal", "BudgetReport"]);
    let template = tree.item_template().expect("the item template of the source");
    let content = template
        .build(&Some(items[0].clone()))
        .unwrap()
        .cast::<TextBlock>()
        .unwrap();
    assert_eq!(content.text().as_deref(), Some("Documents"));
    assert!(
        template.as_tree_data_template().is_some(),
        "the children are the item's Children"
    );
}

#[test]
fn tree_view_template_selector_picks_the_folder_and_the_file_template() {
    let _application = start();
    let root = page();
    let tree = root.get_control::<TreeView>("FileTree");
    assert_eq!(tree.data_templates().count(), 2);
    let items = data_source(&tree);
    let folder = Some(items[0].clone());
    let file = explorer_item(&items[0]).unwrap().children.get(0);
    let [folder_template, file_template] = explorer_item_templates();
    assert!(folder_template.match_(folder.as_ref()) && !folder_template.match_(Some(&file)));
    assert!(file_template.match_(Some(&file)) && !file_template.match_(folder.as_ref()));
    let folder_content = folder_template.build(&folder).unwrap().cast::<StackPanel>().unwrap();
    assert!(
        folder_content.children().get(0).cast::<Image>().is_some(),
        "the folder image"
    );
    assert_eq!(
        folder_content
            .children()
            .get(2)
            .cast::<TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("Documents")
    );
    let file_content = file_template.build(&Some(file)).unwrap().cast::<StackPanel>().unwrap();
    assert_eq!(
        file_content.children().get(0).cast::<FontIcon>().unwrap().glyph(),
        "\u{E8A5}"
    );
    assert_eq!(
        file_content
            .children()
            .get(2)
            .cast::<TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("ProjectProposal")
    );
}

fn headers(items: &ItemsControl) -> Vec<String> {
    nodes(items).iter().map(|item| header(item)).collect()
}

#[test]
fn tree_view_drop_position_follows_the_row_quarters() {
    use crate::tree_samples::TreeDrop;
    assert_eq!(TreeDrop::at(1.0, 40.0), TreeDrop::Before);
    assert_eq!(TreeDrop::at(20.0, 40.0), TreeDrop::Inside);
    assert_eq!(TreeDrop::at(39.0, 40.0), TreeDrop::After);
}

#[test]
fn tree_view_drag_moves_items_before_inside_and_after() {
    use crate::tree_samples::{move_tree_item, TreeDrop};
    let _application = start();
    let root = page();
    let tree = root.get_control::<TreeView>("sampleTreeView");
    let top = nodes(&tree);
    let (work, personal) = (top[0].clone(), top[1].clone());
    let spec = nodes(&work)[0].clone();
    let schedule = nodes(&work)[1].clone();

    // After a sibling of the same list.
    assert!(move_tree_item(&spec, &schedule, TreeDrop::After));
    assert_eq!(headers(&work), ["Feature Schedule", "XYZ Functional Spec"]);
    // Before an item of another list.
    assert!(move_tree_item(&spec, &personal, TreeDrop::Before));
    assert_eq!(headers(&tree), ["Work Documents", "XYZ Functional Spec", "Personal Documents"]);
    assert_eq!(headers(&work), ["Feature Schedule"]);
    // Inside an item, at its end, which expands it.
    schedule.set_is_expanded(false);
    assert!(move_tree_item(&spec, &schedule, TreeDrop::Inside));
    assert_eq!(headers(&schedule), ["XYZ Functional Spec"]);
    assert!(schedule.is_expanded());
}

#[test]
fn tree_view_drag_does_not_move_an_item_into_itself() {
    use crate::tree_samples::{move_tree_item, TreeDrop};
    let _application = start();
    let root = page();
    let tree = root.get_control::<TreeView>("sampleTreeView");
    let personal = nodes(&tree)[1].clone();
    let remodel = nodes(&personal)[0].clone();
    let paint = nodes(&remodel)[1].clone();

    for position in [TreeDrop::Before, TreeDrop::Inside, TreeDrop::After] {
        assert!(!move_tree_item(&personal, &personal, position));
        assert!(!move_tree_item(&personal, &paint, position));
    }
    assert_eq!(headers(&tree), ["Work Documents", "Personal Documents"]);
    assert_eq!(headers(&remodel), ["Contractor Contact Info", "Paint Color Scheme"]);
}

/// The drag events of the first tree in a window: the row shows where the item lands, a drop moves
/// it, and a drop into its own subtree is refused.
#[cfg(feature = "theme")]
#[test]
fn tree_view_drag_events_show_the_target_and_move_the_item() {
    use crate::tree_samples::{row_of, tree_item_format};
    use ferroui_base::input::{
        DataTransfer, DataTransferItem, DragDrop, DragDropEffects, DragEventArgs, IDataTransfer, KeyModifiers,
    };
    use ferroui_base::interactivity::Interactive;
    use ferroui_base::Point;
    use ferroui_controls::Window;
    use std::rc::Rc;

    let _application = crate::tests::start_winui();
    let root = page();
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    let tree = root.get_control::<TreeView>("sampleTreeView");
    let work = nodes(&tree)[0].clone();
    let spec = nodes(&work)[0].clone();
    let schedule = nodes(&work)[1].clone();

    let transfer_of = |item: &Ref<TreeViewItem>| -> Rc<dyn IDataTransfer> {
        let data = DataTransferItem::new();
        data.set(&tree_item_format(), Some(item.clone()));
        let transfer = DataTransfer::new();
        transfer.add(data);
        transfer
    };
    let raise = |event, item: &Ref<TreeViewItem>, target: &Ref<TreeViewItem>, y: f64| {
        let args = DragEventArgs::new(
            Some(event),
            transfer_of(item),
            target.clone().upcast::<Interactive>(),
            Point::new(2.0, y),
            KeyModifiers::NONE,
        );
        target.raise_event(&args);
        args.drag_effects()
    };

    let bottom = row_of(&schedule).bounds().height - 1.0;
    assert_eq!(raise(DragDrop::drag_over_event(), &spec, &schedule, bottom), DragDropEffects::MOVE);
    assert!(schedule.classes().contains("DropAfter"));

    assert_eq!(raise(DragDrop::drop_event(), &spec, &schedule, bottom), DragDropEffects::MOVE);
    assert!(!schedule.classes().contains("DropAfter"));
    assert_eq!(headers(&work), ["Feature Schedule", "XYZ Functional Spec"]);

    assert_eq!(raise(DragDrop::drop_event(), &work, &schedule, bottom), DragDropEffects::NONE);
    assert_eq!(headers(&tree), ["Work Documents", "Personal Documents"]);
    window.close();
}
