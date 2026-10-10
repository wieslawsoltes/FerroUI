//! Port of `Pages/DragDropPage.xaml.cs`: the class of the document `Pages/DragDropPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::input::{
    DataTransfer, DataTransferExtensions, DataTransferItem, DragDrop, DragDropEffects, DragEventArgs, IDataTransfer,
    PointerPressedEventArgs,
};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, FerroObject, Ref, Visual};
use ferroui_controls::{Border, TextBlock, UserControl};
use mini_mvvm::start_async;
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
            fn DragSource_PointerPressed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DragDropPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        this.drag_source_pointer_pressed(&sender, e)
                    }
                },
            fn ResetDragDrop_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DragDropPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.reset_drag_drop_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(DragDropPage, "/Pages/DragDropPage.xaml");

impl DragDropPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // Set up drag-drop event handlers
        //
        // The page holds the handlers of its events: the handlers hold the page weakly.
        {
            let weak = this.downgrade();
            this.add_handler(DragDrop::drag_over_event(), move |sender, e| {
                if let Some(this) = weak.upgrade() {
                    this.drop_target_drag_over(sender, e);
                }
            });
        }
        {
            let weak = this.downgrade();
            this.add_handler(DragDrop::drop_event(), move |sender, e| {
                if let Some(this) = weak.upgrade() {
                    this.drop_target_drop(sender, e);
                }
            });
        }
        this
    }

    fn drop_position(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DropPosition")
    }

    fn drag_drop_status(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DragDropStatus")
    }

    fn drop_target(&self) -> Ref<Border> {
        self.get_control::<Border>("DropTarget")
    }

    fn drop_target_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DropTargetText")
    }

    /// `DragSource_PointerPressed` (`async void`).
    fn drag_source_pointer_pressed(&self, _sender: &Option<BoxedValue>, e: &PointerPressedEventArgs) {
        let visual: &Visual = self;
        if e.get_current_point(Some(visual)).properties.is_left_button_pressed {
            let e = e.clone();
            let this = self.to_ref();
            drop(start_async(async move {
                let drag_data = DataTransfer::new();
                drag_data.add(DataTransferItem::create_text(Some("TestDragData")));

                this.drag_drop_status().set_text(Some("Dragging..."));

                let data_transfer: Rc<dyn IDataTransfer> = drag_data;
                let result =
                    DragDrop::do_drag_drop_async(&e, data_transfer, DragDropEffects::COPY | DragDropEffects::MOVE).await;

                let text = if result == DragDropEffects::COPY {
                    String::from("Copied")
                } else if result == DragDropEffects::MOVE {
                    String::from("Moved")
                } else if result == DragDropEffects::NONE {
                    String::from("Cancelled")
                } else {
                    format!("Result: {result:?}")
                };
                this.drag_drop_status().set_text(Some(&text));
            }));
        }
    }

    /// Whether the event is one of the drop target: its source is the drop target or a child
    /// of it.
    fn is_for_drop_target(e: &DragEventArgs, drop_target: &Ref<Border>) -> bool {
        let source: Option<Ref<FerroObject>> = e.source();
        let is_drop_target = source.as_ref().is_some_and(|source| source.ptr_eq(drop_target));
        let source_visual = source.and_then(|source| source.cast::<Visual>());
        let drop_target_visual: Ref<Visual> = drop_target.clone().upcast();
        is_drop_target || Self::is_child_of(source_visual.as_ref(), Some(&drop_target_visual))
    }

    fn drop_target_drag_over(&self, _sender: &Interactive, e: &DragEventArgs) {
        // Only handle events for the drop target
        let drop_target = self.drop_target();
        if !Self::is_for_drop_target(e, &drop_target) {
            return;
        }

        e.set_drag_effects(DragDropEffects::COPY);

        // Get the position relative to the drop target
        let position = e.get_position(&drop_target);
        self.drop_position().set_text(Some(&format!("DragOver: ({:.0}, {:.0})", position.x, position.y)));
    }

    fn drop_target_drop(&self, _sender: &Interactive, e: &DragEventArgs) {
        // Only handle events for the drop target
        let drop_target = self.drop_target();
        if !Self::is_for_drop_target(e, &drop_target) {
            return;
        }

        // Get the position relative to the drop target
        let position = e.get_position(&drop_target);
        self.drop_position().set_text(Some(&format!("Drop: ({:.0}, {:.0})", position.x, position.y)));

        // Check if the position is within reasonable bounds of the drop target
        let bounds = drop_target.bounds();
        let is_within_bounds =
            position.x >= 0.0 && position.x <= bounds.width && position.y >= 0.0 && position.y <= bounds.height;

        let text = e.data_transfer().try_get_text();
        if let Some(text) = text {
            self.drop_target_text().set_text(Some(&if is_within_bounds {
                format!("Dropped: {text} at ({:.0}, {:.0})", position.x, position.y)
            } else {
                format!("ERROR: Position out of bounds! ({:.0}, {:.0})", position.x, position.y)
            }));
            self.drag_drop_status().set_text(Some(if is_within_bounds { "Drop OK" } else { "Drop position ERROR" }));
        }

        e.set_drag_effects(DragDropEffects::COPY);
    }

    fn is_child_of(child: Option<&Ref<Visual>>, parent: Option<&Ref<Visual>>) -> bool {
        let (Some(child), Some(parent)) = (child, parent) else {
            return false;
        };

        let mut current = child.parent().and_then(|current| current.cast::<Visual>());
        while let Some(visual) = current {
            if visual.ptr_eq(parent) {
                return true;
            }
            current = visual.parent().and_then(|current| current.cast::<Visual>());
        }
        false
    }

    fn reset_drag_drop_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.drop_position().set_text(Some(""));
        self.drag_drop_status().set_text(Some(""));
        self.drop_target_text().set_text(Some("Drop items here"));
    }
}
