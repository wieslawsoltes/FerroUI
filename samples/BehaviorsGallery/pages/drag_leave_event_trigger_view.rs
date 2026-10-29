//! Port of `Views/Pages/DragLeaveEventTriggerView` of the reference sample: the class of
//! `pages/DragLeaveEventTriggerView.xaml`, which starts a drag from its button and logs when the
//! drag leaves its drop zone (the page is its own data context).

use crate::markup::{object_as, user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::items::{text_list, ItemList};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{
    DataTransfer, DataTransferItem, DragDrop, DragDropEffects, IDataTransfer,
    PointerPressedEventArgs,
};
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, FerroSynchronizationContext};
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, ElementRef, Ref, TypeInfo};
use ferroui_controls::{Control, ItemsSource, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct DragLeaveEventTriggerView {
    base: UserControl,
    events: ItemList,
}

user_control_class!(DragLeaveEventTriggerView);
ferro_class_info!(DragLeaveEventTriggerView {
    new: DragLeaveEventTriggerView::new,
    markup: {
        properties: [
            Events: ItemsSource { get: |this: &Ref<DragLeaveEventTriggerView>| ItemsSource::from(this.events()) },
        ],
        methods: [
            fn OnDragLeave(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<DragLeaveEventTriggerView>, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| {
                    this.on_drag_leave(sender, parameter)
                },
            fn OnStartDrag(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |_this: &Ref<DragLeaveEventTriggerView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        DragLeaveEventTriggerView::on_start_drag(sender, e)
                    }
                },
        ],
    },
});
xaml_class!(
    DragLeaveEventTriggerView,
    "/pages/DragLeaveEventTriggerView.xaml"
);

impl DragLeaveEventTriggerView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            events: text_list([]),
        }
    }

    /// `DragLeaveEventTriggerView()`: the page is its own data context.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(Rc::new(ElementRef::of(&this)) as BoxedValue));
        this
    }

    /// `Events`.
    pub fn events(&self) -> ItemList {
        self.events.clone()
    }

    /// `OnDragLeave(object?, object?)`: the newest entry first, at most 20.
    pub fn on_drag_leave(&self, _sender: Option<BoxedValue>, _parameter: Option<BoxedValue>) {
        let entry = format!(
            "{} - Drag left drop zone",
            DateTimeOffset::now().to_string_with("T")
        );
        self.events.insert(0, Rc::new(entry) as BoxedValue);
        if self.events.count() > 20 {
            self.events.remove_at(self.events.count() - 1);
        }
    }

    /// `async void OnStartDrag(object?, PointerPressedEventArgs)`: drags a text from the control
    /// that is pressed.
    fn on_start_drag(sender: Option<BoxedValue>, e: &PointerPressedEventArgs) {
        if sender.as_ref().and_then(object_as::<Control>).is_none() {
            return;
        }

        let data = DataTransfer::new();
        data.add(DataTransferItem::create_text(Some("DragLeave sample")));
        let data: Rc<dyn IDataTransfer> = data;
        let pending = DragDrop::do_drag_drop_async(e, data, DragDropEffects::COPY);
        // The rest of the `async void` method runs on the dispatcher of the thread.
        let context = FerroSynchronizationContext::current().unwrap_or_else(|| {
            FerroSynchronizationContext::with_dispatcher(
                &Dispatcher::current_dispatcher(),
                DispatcherPriority::NORMAL,
            )
        });
        drop(context.to_task_scheduler().start_local(async move {
            let _ = pending.await;
        }));
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DragLeaveEventTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DragLeaveEventTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "DragLeaveEventTrigger",
    reference: "Views/Pages/DragLeaveEventTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&DragLeaveEventTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types,
};

/// The page is its own data context (a reference to the element).
fn register_value_types() {
    ValueTypes::register_element_ref::<DragLeaveEventTriggerView>();
}
