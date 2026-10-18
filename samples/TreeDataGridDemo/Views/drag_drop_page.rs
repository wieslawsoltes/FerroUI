//! The drag and drop page: the tab "Drag/Drop" of the reference demo's main window
//! (`MainWindow`), with the handlers of the drags of its grid from the reference's code-behind
//! (`DragDrop_RowDragStarted`, `DragDrop_RowDragOver`), as a view of its own that the main
//! window and the ControlCatalog both show.

use crate::markup::{user_control_class, xaml_class};
use crate::models::DragDropItem;
use ferroui_base::input::DragDropEffects;
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::UserControl;
use ferroui_controls_treedatagrid::{
    TreeDataGridRowDragEventArgs, TreeDataGridRowDragStartedEventArgs, TreeDataGridRowDropPosition,
};
use std::rc::Rc;

#[repr(C)]
pub struct DragDropPage {
    base: UserControl,
}

user_control_class!(DragDropPage);
ferro_class_info!(DragDropPage {
    new: DragDropPage::new,
    markup: {
        methods: [
            fn DragDrop_RowDragStarted(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DragDropPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<TreeDataGridRowDragStartedEventArgs>() {
                        this.drag_drop_row_drag_started(&sender, e)
                    }
                },
            fn DragDrop_RowDragOver(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DragDropPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<TreeDataGridRowDragEventArgs>() {
                        this.drag_drop_row_drag_over(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(DragDropPage, "/Views/DragDropPage.xaml");

impl DragDropPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// `DragDrop_RowDragStarted`: rows that do not allow a drag make the drag of the selection
    /// impossible.
    pub fn drag_drop_row_drag_started(&self, _sender: &Option<BoxedValue>, e: &TreeDataGridRowDragStartedEventArgs) {
        for i in e.models() {
            let Some(i) = from_markup_value::<Rc<DragDropItem>>(i) else {
                continue;
            };
            if !i.allow_drag() {
                e.set_allowed_effects(DragDropEffects::NONE);
            }
        }
    }

    /// `DragDrop_RowDragOver`: rows cannot be dropped into a row that does not allow a drop.
    pub fn drag_drop_row_drag_over(&self, _sender: &Option<BoxedValue>, e: &TreeDataGridRowDragEventArgs) {
        if e.position() == TreeDataGridRowDropPosition::Inside {
            let target = e.target_row().and_then(|row| from_markup_value::<Rc<DragDropItem>>(&row.model()));
            if let Some(i) = target {
                if !i.allow_drop() {
                    e.inner().set_drag_effects(DragDropEffects::NONE);
                }
            }
        }
    }
}
