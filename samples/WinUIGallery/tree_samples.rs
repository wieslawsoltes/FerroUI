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

use ferroui_base::collections::FerroList;
use ferroui_base::layout::Orientation;
use ferroui_base::media::IImage;
use ferroui_base::{BoxedValue, Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::templates::{FuncTreeDataTemplate, IDataTemplate};
use ferroui_controls::{
    Control, FontIcon, Image, ItemsSource, SelectionMode, StackPanel, TextBlock, TreeView, TreeViewItem,
};
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
    initialize_sample_tree_view(&root.get_control::<TreeView>("sampleTreeView"));
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
