//! Port of `Diagnostics/Views/EventsPageView.xaml.cs`: the class of the
//! document `EventsPageView.xaml`.
//!
//! The tree of the routed events of the document is a TreeDataGrid, where upstream has a tree
//! view (docs/porting/DEVIATIONS.md, Developer tools).

use crate::diagnostics::controls::ControlHighlightAdorner;
use crate::diagnostics::models::EventChainLink;
use crate::diagnostics::view_models::{EventsPageViewModel, FiredEvent};
use ferroui_base::collections::NotifyCollectionChangedEventArgs;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::input::{InputElementImpl, PointerEventArgs, TappedEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEvent};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{from_markup_value, into_markup_value};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, StyledElementImplExt, Visual, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, Control, ControlImpl, ListBox, UserControl};
use ferroui_controls_treedatagrid::TreeDataGrid;
use std::cell::RefCell;
use std::rc::Rc;

/// The view of the events page: the tree of the routed events to record,
/// the recorded events and the chain of the selected event.
#[repr(C)]
pub struct EventsPageView {
    base: UserControl,
    events: RefCell<Option<Ref<ListBox>>>,
    adorner: RefCell<Option<Rc<dyn IDisposable>>>,
    // Deviation (DEVIATIONS.md, Developer tools): the TreeDataGrid of the routed events, and the
    // handler of the changes of the selected node of the view model, which brings its row into
    // view.
    events_tree: RefCell<Option<Ref<TreeDataGrid>>>,
    selected_node_changed: RefCell<Option<(Rc<EventsPageViewModel>, u64)>>,
}

ferro_class!(EventsPageView: UserControl);
ferro_impl_classes!(
    EventsPageView: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(EventsPageView {
    new: EventsPageView::new,
    markup: {
        methods: [
            fn NavigateTo(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EventsPageView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<TappedEventArgs>() {
                        this.navigate_to(&sender, e)
                    }
                },
            fn ListBoxItem_PointerEntered(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EventsPageView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerEventArgs>() {
                        this.list_box_item_pointer_entered(&sender, e)
                    }
                },
            fn ListBoxItem_PointerExited(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EventsPageView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerEventArgs>() {
                        this.list_box_item_pointer_exited(&sender, e)
                    }
                },
        ],
    },
});

impl StyledElementImpl for EventsPageView {
    fn on_data_context_changed(this: &Self) {
        Self::parent_on_data_context_changed(this);

        let selected_node_changed = this.selected_node_changed.borrow_mut().take();
        if let Some((old_view_model, token)) = selected_node_changed {
            old_view_model.property_changed().remove(token);
        }

        if let Some(vm) = this.view_model() {
            // Deviation (DEVIATIONS.md, Developer tools): the row of a node the view model selects
            // (a double tap on the name of a recorded event) is brought into view once the grid
            // has its rows.
            let view = this.to_ref().downgrade();
            let token = vm.property_changed().add(Rc::new(move |property_name: &str| {
                if property_name != "SelectedNode" {
                    return;
                }
                let view = view.clone();
                Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(view) = view.upgrade() {
                            view.bring_selected_node_into_view();
                        }
                    },
                    DispatcherPriority::default(),
                );
            }));
            *this.selected_node_changed.borrow_mut() = Some((vm.clone(), token));

            // As upstream, the handler is never removed. It reaches the view and the view model
            // through weak references, so that the list of the view model does not keep them
            // alive.
            let view = this.to_ref().downgrade();
            let view_model = Rc::downgrade(&vm);
            let _ = vm.recorded_events().add_collection_changed(Rc::new(
                move |_: &NotifyCollectionChangedEventArgs<'_, Rc<FiredEvent>>| {
                    if let (Some(view), Some(view_model)) = (view.upgrade(), view_model.upgrade()) {
                        view.on_recorded_events_changed(&view_model);
                    }
                },
            ));
        }
    }
}

impl EventsPageView {
    /// The URI of the document of the class.
    pub const DOCUMENT_URI: &'static str = "ferres://FerroUI.Diagnostics/diagnostics/views/EventsPageView.xaml";

    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            events: RefCell::new(None),
            adorner: RefCell::new(None),
            events_tree: RefCell::new(None),
            selected_node_changed: RefCell::new(None),
        }
    }

    /// Creates the view and populates it from its document
    /// (`InitializeComponent()`: the compiled markup of the document, which
    /// the build of the crate generates).
    ///
    /// # Panics
    /// Panics when the document fails to load (an exception of the
    /// constructor in the original).
    pub fn new() -> Ref<Self> {
        crate::register_types();
        let this = instantiate(Self::construct());
        Self::initialize_component(&this);
        *this.events.borrow_mut() = Some(this.get_control::<ListBox>("EventsList"));
        *this.events_tree.borrow_mut() = Some(this.get_control::<TreeDataGrid>("EventsTree"));
        this
    }

    /// Brings the row of the selected node of the tree of the routed events into view.
    ///
    /// Deviation (DEVIATIONS.md, Developer tools): the tree is a TreeDataGrid.
    fn bring_selected_node_into_view(&self) {
        let Some(vm) = self.view_model() else {
            return;
        };
        let row_index = vm.selected_row_index();
        if row_index < 0 {
            return;
        }
        let events_tree = self.events_tree.borrow().clone();
        if let Some(rows_presenter) = events_tree.and_then(|events_tree| events_tree.rows_presenter()) {
            rows_presenter.bring_into_view(row_index, None);
        }
    }

    /// `InitializeComponent()`.
    fn initialize_component(this: &Ref<Self>) {
        if let Err(error) = crate::compiled_diagnostics_views_events_page_view::populate(None, this) {
            panic!("{}: {error}", Self::DOCUMENT_URI);
        }
    }

    fn view_model(&self) -> Option<Rc<EventsPageViewModel>> {
        from_markup_value::<Rc<EventsPageViewModel>>(&self.data_context())
    }

    /// Double tapping the name of an event selects its node in the tree;
    /// double tapping an element of a chain shows it in the visual tree.
    pub fn navigate_to(&self, sender: &Option<BoxedValue>, _e: &TappedEventArgs) {
        let control = sender.as_ref().and_then(Control::from_boxed);
        if let (Some(vm), Some(control)) = (self.view_model(), control) {
            let tag = control.tag();
            if let Some(chain_link) = from_markup_value::<Rc<EventChainLink>>(&tag) {
                vm.request_tree_navigate_to(&chain_link);
            } else if let Some(evt) = from_markup_value::<RoutedEvent>(&tag) {
                vm.select_event_by_type(evt);
            }
        }
    }

    /// Scrolls the last recorded event into view.
    fn on_recorded_events_changed(&self, vm: &EventsPageViewModel) {
        let events = vm.recorded_events();
        let Some(evt) = events.try_get(events.count().wrapping_sub(1)) else {
            return;
        };

        let list_box = self.events.borrow().clone();
        if let Some(list_box) = list_box {
            Dispatcher::ui_thread().post_local(
                move || list_box.scroll_into_view_item(&into_markup_value(evt)),
                DispatcherPriority::default(),
            );
        }
    }

    /// Highlights the element of the link of the chain under the pointer.
    pub fn list_box_item_pointer_entered(&self, sender: &Option<BoxedValue>, _e: &PointerEventArgs) {
        let Some(vm) = self.view_model() else {
            return;
        };
        let Some(control) = sender.as_ref().and_then(Control::from_boxed) else {
            return;
        };
        let Some(chain_link) = from_markup_value::<Rc<EventChainLink>>(&control.data_context()) else {
            return;
        };
        let Some(visual) = ValueTypes::as_object(&*chain_link.handler()).and_then(|handler| handler.cast::<Visual>())
        else {
            return;
        };

        if let Some(main_view) = vm.main_view() {
            let adorner = ControlHighlightAdorner::add_visual(&visual, main_view.should_visualize_margin_padding());
            *self.adorner.borrow_mut() = adorner;
        }
    }

    /// Removes the highlight.
    pub fn list_box_item_pointer_exited(&self, _sender: &Option<BoxedValue>, _e: &PointerEventArgs) {
        let adorner = self.adorner.borrow().clone();
        if let Some(adorner) = adorner {
            adorner.dispose();
        }
    }
}
