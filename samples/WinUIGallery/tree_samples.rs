//! The code behind of the pinned TreeView page in Rust: `TreeViewPage.xaml.cs` with its model
//! `ExplorerItem` and its `ExplorerItemTemplateSelector`.
//!
//! FerroUI's TreeView has no `TreeViewNode`s: `InitializeSampleTreeView`'s nodes are TreeViewItems
//! with their `Content` as the Header, created here as the source creates its nodes. The source's
//! item templates are TreeViewItems whose `ItemsSource` is `{x:Bind Children}`: here they are
//! hierarchical data templates (`FuncTreeDataTemplate`) whose children are the item's `Children`,
//! the content of the source's TreeViewItem their content. The template selector is a pair of data
//! templates that match the folders and the files, which a FerroUI items control picks from its
//! `DataTemplates`.
//!
//! The first tree's `CanDragItems` and `AllowDrop` are the drag of this file: a press on an item
//! that moves starts it, and a drop puts the item before, inside or after the item under the
//! pointer, as WinUI's TreeView reorders its nodes.

use ferroui_base::collections::FerroList;
use ferroui_base::input::{
    DataFormat, DataFormatOf, DataTransfer, DataTransferExtensions, DataTransferItem, DragDrop, DragDropEffects,
    DragEventArgs, IDataTransfer, InputElement, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::Orientation;
use ferroui_base::media::IImage;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Point, Ref, Thickness, Visual};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::templates::{FuncTreeDataTemplate, IDataTemplate};
use ferroui_controls::{
    Control, FontIcon, Image, ItemsControl, ItemsSource, SelectionMode, StackPanel, TextBlock, TreeView, TreeViewItem,
};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id == "TreeView" {
        wire_tree_view(root);
    }
}

/// `ExplorerItem.ExplorerItemType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExplorerItemType {
    Folder,
    File,
}

/// `ExplorerItem`: a folder or a file of the bound trees.
pub(crate) struct ExplorerItem {
    pub(crate) name: &'static str,
    pub(crate) kind: ExplorerItemType,
    pub(crate) children: Rc<FerroList<BoxedValue>>,
}

impl PartialEq for ExplorerItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ExplorerItem {
    fn new(name: &'static str, kind: ExplorerItemType, children: Vec<Rc<ExplorerItem>>) -> Rc<ExplorerItem> {
        Rc::new(ExplorerItem {
            name,
            kind,
            children: Rc::new(FerroList::from_items(
                children.into_iter().map(|child| Rc::new(child) as BoxedValue),
            )),
        })
    }
}

/// `GetData`: the Documents and Projects folders with their files.
pub(crate) fn get_data() -> Rc<FerroList<BoxedValue>> {
    use ExplorerItemType::{File, Folder};
    let documents = ExplorerItem::new(
        "Documents",
        Folder,
        vec![
            ExplorerItem::new("ProjectProposal", File, vec![]),
            ExplorerItem::new("BudgetReport", File, vec![]),
        ],
    );
    let projects = ExplorerItem::new(
        "Projects",
        Folder,
        vec![ExplorerItem::new("Project Plan", File, vec![])],
    );
    Rc::new(FerroList::from_items(
        [documents, projects].map(|item| Rc::new(item) as BoxedValue),
    ))
}

/// The explorer item of an item of a tree, if it is one.
pub(crate) fn explorer_item(item: &BoxedValue) -> Option<Rc<ExplorerItem>> {
    item.downcast_ref::<Rc<ExplorerItem>>().cloned()
}

fn children(item: &Rc<ExplorerItem>) -> BoxedValue {
    Rc::new(Some(ItemsSource::from(item.children.clone())))
}

/// A TreeViewNode of `InitializeSampleTreeView`: a TreeViewItem with the node's content.
fn node(content: &str, expanded: bool, children: Vec<Ref<TreeViewItem>>) -> Ref<TreeViewItem> {
    let item = TreeViewItem::new();
    item.set_header(Some(Rc::new(content.to_string()) as BoxedValue));
    item.set_is_expanded(expanded);
    for child in children {
        item.items().add(Some(Control::boxed(child)));
    }
    item
}

/// `InitializeSampleTreeView`: the Work Documents and Personal Documents folders, expanded.
pub(crate) fn initialize_sample_tree_view(tree: &Ref<TreeView>) {
    let work_folder = node(
        "Work Documents",
        true,
        vec![
            node("XYZ Functional Spec", false, vec![]),
            node("Feature Schedule", false, vec![]),
        ],
    );
    let remodel_folder = node(
        "Home Remodel",
        true,
        vec![
            node("Contractor Contact Info", false, vec![]),
            node("Paint Color Scheme", false, vec![]),
        ],
    );
    let personal_folder = node("Personal Documents", true, vec![remodel_folder]);
    tree.items().add(Some(Control::boxed(work_folder)));
    tree.items().add(Some(Control::boxed(personal_folder)));
}

/// Where a dragged item lands relative to the item under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TreeDrop {
    Before,
    Inside,
    After,
}

impl TreeDrop {
    /// The top quarter of a row is before it, the bottom quarter after it, the rest inside it.
    pub(crate) fn at(y: f64, height: f64) -> Self {
        if y < height / 4.0 {
            Self::Before
        } else if y > height * 3.0 / 4.0 {
            Self::After
        } else {
            Self::Inside
        }
    }

    fn class(self) -> &'static str {
        match self {
            Self::Before => "DropBefore",
            Self::Inside => "DropInside",
            Self::After => "DropAfter",
        }
    }
}

/// The in-process format of the dragged item.
pub(crate) fn tree_item_format() -> DataFormatOf<Ref<TreeViewItem>> {
    DataFormat::create_in_process_format::<Ref<TreeViewItem>>("WinUIGallery.TreeView.item")
}

/// The items control that holds `item`: its parent item or the tree.
fn owner_of(item: &TreeViewItem) -> Option<Ref<ItemsControl>> {
    item.parent().and_then(|parent| parent.cast::<ItemsControl>())
}

fn index_in(owner: &ItemsControl, item: &Ref<TreeViewItem>) -> Option<usize> {
    let items = owner.items();
    (0..items.count()).find(|&index| {
        items
            .get_at(index)
            .and_then(|value| Control::from_boxed(&value))
            .and_then(|control| control.cast::<TreeViewItem>())
            .is_some_and(|candidate| candidate.ptr_eq(item))
    })
}

/// Whether `target` is `item` or inside it: an item cannot move into itself.
fn is_self_or_inside(item: &Ref<TreeViewItem>, target: &Ref<TreeViewItem>) -> bool {
    let mut current = Some(target.clone());
    while let Some(candidate) = current {
        if candidate.ptr_eq(item) {
            return true;
        }
        current = candidate.parent().and_then(|parent| parent.cast::<TreeViewItem>());
    }
    false
}

/// Moves `item` before, inside or after `target`. Returns whether it moved.
pub(crate) fn move_tree_item(item: &Ref<TreeViewItem>, target: &Ref<TreeViewItem>, position: TreeDrop) -> bool {
    if is_self_or_inside(item, target) {
        return false;
    }
    let Some(source) = owner_of(item) else {
        return false;
    };
    let Some(source_index) = index_in(&source, item) else {
        return false;
    };
    let destination = match position {
        TreeDrop::Inside => target.clone().upcast::<ItemsControl>(),
        TreeDrop::Before | TreeDrop::After => match owner_of(target) {
            Some(owner) => owner,
            None => return false,
        },
    };
    let value = source.items().get_at(source_index);
    source.items().remove_at(source_index);
    let index = match position {
        TreeDrop::Inside => destination.items().count(),
        TreeDrop::Before => index_in(&destination, target).unwrap_or(0),
        TreeDrop::After => index_in(&destination, target).map_or(0, |index| index + 1),
    };
    destination.items().insert(index, value);
    if position == TreeDrop::Inside {
        target.set_is_expanded(true);
    }
    true
}

/// The row of an item, without its children.
pub(crate) fn row_of(item: &Ref<TreeViewItem>) -> Ref<Visual> {
    item.get_visual_descendants()
        .find(|visual| {
            visual
                .clone()
                .cast::<Control>()
                .is_some_and(|control| control.name().as_deref() == Some("PART_Header"))
        })
        .unwrap_or_else(|| item.clone().upcast())
}

fn clear_drop_classes(tree: &TreeView) {
    for container in tree.get_realized_tree_containers() {
        for position in [TreeDrop::Before, TreeDrop::Inside, TreeDrop::After] {
            container.classes().remove(position.class());
        }
    }
}

/// The item under a drag and where the dragged item would land on it.
fn drop_target(tree: &TreeView, e: &DragEventArgs) -> Option<(Ref<TreeViewItem>, Ref<TreeViewItem>, TreeDrop)> {
    let item = e.data_transfer().try_get_value(&tree_item_format())?;
    let target = tree.get_container_from_event_source(e.source())?;
    if is_self_or_inside(&item, &target) {
        return None;
    }
    let row = row_of(&target);
    let position = TreeDrop::at(e.get_position(&row).y, row.bounds().height);
    Some((item, target, position))
}

/// WinUI's `CanDragItems` and `AllowDrop` of the first tree.
fn wire_item_drag(tree: &Ref<TreeView>) {
    DragDrop::set_allow_drop(tree, true);
    let pressed: Rc<RefCell<Option<(PointerPressedEventArgs, Point, Ref<TreeViewItem>)>>> = Rc::default();
    {
        let pressed = pressed.clone();
        let weak_tree = tree.downgrade();
        tree.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e: &PointerPressedEventArgs| {
                let Some(tree) = weak_tree.upgrade() else {
                    return;
                };
                *pressed.borrow_mut() = tree
                    .get_container_from_event_source(e.source())
                    .filter(|_| e.properties().is_left_button_pressed)
                    .map(|item| (e.clone(), e.get_position(Some(&tree)), item));
            },
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
            true,
        );
    }
    {
        let pressed = pressed.clone();
        let weak_tree = tree.downgrade();
        tree.add_handler_with(
            InputElement::pointer_moved_event(),
            move |_, e: &PointerEventArgs| {
                let Some(tree) = weak_tree.upgrade() else {
                    return;
                };
                let moved = pressed.borrow().as_ref().is_some_and(|(_, start, _)| {
                    let point = e.get_position(Some(&tree));
                    (point.x - start.x).abs() > 4.0 || (point.y - start.y).abs() > 4.0
                });
                if !moved {
                    return;
                }
                let Some((trigger, _, item)) = pressed.borrow_mut().take() else {
                    return;
                };
                let data = DataTransferItem::new();
                data.set(&tree_item_format(), Some(item));
                let transfer = DataTransfer::new();
                transfer.add(data);
                let transfer: Rc<dyn IDataTransfer> = transfer;
                let operation = DragDrop::do_drag_drop_async(&trigger, transfer, DragDropEffects::MOVE);
                let weak_tree = tree.downgrade();
                drop(Dispatcher::ui_thread().spawn_local(async move {
                    operation.await;
                    if let Some(tree) = weak_tree.upgrade() {
                        clear_drop_classes(&tree);
                    }
                }));
            },
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
            true,
        );
    }
    tree.add_handler_with(
        InputElement::pointer_released_event(),
        move |_, _: &PointerReleasedEventArgs| {
            pressed.borrow_mut().take();
        },
        RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
        true,
    );
    let weak_tree = tree.downgrade();
    DragDrop::add_drag_over_handler(tree, move |_, e: &DragEventArgs| {
        let Some(tree) = weak_tree.upgrade() else {
            return;
        };
        clear_drop_classes(&tree);
        match drop_target(&tree, e) {
            Some((_, target, position)) => {
                target.classes().add(position.class());
                e.set_drag_effects(DragDropEffects::MOVE);
            }
            None => e.set_drag_effects(DragDropEffects::NONE),
        }
    });
    let weak_tree = tree.downgrade();
    DragDrop::add_drag_leave_handler(tree, move |_, _: &DragEventArgs| {
        if let Some(tree) = weak_tree.upgrade() {
            clear_drop_classes(&tree);
        }
    });
    let weak_tree = tree.downgrade();
    DragDrop::add_drop_handler(tree, move |_, e: &DragEventArgs| {
        let Some(tree) = weak_tree.upgrade() else {
            return;
        };
        let target = drop_target(&tree, e);
        clear_drop_classes(&tree);
        let moved = target.is_some_and(|(item, target, position)| move_tree_item(&item, &target, position));
        e.set_drag_effects(if moved { DragDropEffects::MOVE } else { DragDropEffects::NONE });
    });
}

/// The content of the source's TreeViewItem of `TreeView1`: `Content="{x:Bind Name}"`.
fn name_template() -> Rc<dyn IDataTemplate> {
    FuncTreeDataTemplate::for_type::<Rc<ExplorerItem>>(
        |item, _| {
            let text = TextBlock::new();
            text.set_text(Some(item.name));
            Some(text.upcast())
        },
        children,
    )
}

/// The content of the source's `FolderTemplate` or `FileTemplate`: the folder image or the file
/// glyph, a spacer and the name, with the name as the automation name.
fn explorer_content(item: &ExplorerItem, folder_image: Option<Rc<dyn IImage>>) -> Ref<Control> {
    let panel = StackPanel::new();
    panel.set_orientation(Orientation::Horizontal);
    AutomationProperties::set_name(&panel, Some(item.name));
    if item.kind == ExplorerItemType::Folder {
        let image = Image::new();
        image.set_width(20.0);
        image.set_source(folder_image);
        panel.children().add(image);
    } else {
        let icon = FontIcon::new();
        icon.set_glyph("\u{E8A5}");
        panel.children().add(icon);
    }
    let spacer = TextBlock::new();
    spacer.set_margin(Thickness::new(0.0, 0.0, 10.0, 0.0));
    panel.children().add(spacer);
    let name = TextBlock::new();
    name.set_text(Some(item.name));
    panel.children().add(name);
    panel.upcast()
}

/// `ExplorerItemTemplateSelector.SelectTemplateCore`: the folder template for a folder, the file
/// template for a file.
pub(crate) fn explorer_item_templates() -> [Rc<dyn IDataTemplate>; 2] {
    let folder_image = crate::assets::load_bitmap("ferres://WinUIGallery/Assets/SampleMedia/folder.png")
        .ok()
        .map(|bitmap| Rc::new(bitmap) as Rc<dyn IImage>);
    let folder: Rc<dyn IDataTemplate> = FuncTreeDataTemplate::for_type_with_match::<Rc<ExplorerItem>>(
        |item| item.kind == ExplorerItemType::Folder,
        move |item, _| Some(explorer_content(item, folder_image.clone())),
        children,
    );
    let file: Rc<dyn IDataTemplate> = FuncTreeDataTemplate::for_type_with_match::<Rc<ExplorerItem>>(
        |item| item.kind == ExplorerItemType::File,
        |item, _| Some(explorer_content(item, None)),
        children,
    );
    [folder, file]
}

fn wire_tree_view(root: &Ref<Control>) {
    // The constructor: the two sample trees, and the bound trees' `DataSource`.
    let sample = root.get_control::<TreeView>("sampleTreeView");
    initialize_sample_tree_view(&sample);
    wire_item_drag(&sample);
    let multiple = root.get_control::<TreeView>("sampleTreeView2");
    initialize_sample_tree_view(&multiple);
    // WinUI's Multiple mode toggles an item with a click.
    multiple.set_selection_mode(SelectionMode::MULTIPLE | SelectionMode::TOGGLE);
    let data_source = get_data();
    let bound = root.get_control::<TreeView>("TreeView1");
    bound.set_item_template(Some(name_template()));
    bound.set_items_source(Some(data_source.clone().into()));
    let file_tree = root.get_control::<TreeView>("FileTree");
    for template in explorer_item_templates() {
        file_tree.data_templates().add(template);
    }
    file_tree.set_items_source(Some(data_source.into()));
}
