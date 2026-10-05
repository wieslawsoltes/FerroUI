use crate::generators::ItemContainerGenerator;
use crate::items_source::ItemsChangedEventArgs;
use crate::utils::CollectionUtils;
use crate::{Control, ControlImpl, ItemsControl, ItemsSourceView, Panel, PanelImpl};
use ferroui_base::input::{INavigableContainer, InputElement, InputElementImpl, NavigationDirection};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{ferro_class, ferro_impl_classes, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Base class for panels that can be used to virtualize items for an
/// [`ItemsControl`].
///
/// Panels should implement the abstract members of this class to provide
/// virtualization of items in an [`ItemsControl`]. Derived panels can
/// manage scrolling by implementing the logical scrollable contract or by
/// listening to the effective viewport changed event.
///
/// The methods on the [`ItemContainerGenerator`] should be used to create,
/// prepare and clear containers for items.
///
/// This class is abstract: its abstract members panic unless overridden.
#[repr(C)]
pub struct VirtualizingPanel {
    base: Panel,
    items_control: RefCell<Option<WeakRef<ItemsControl>>>,
    items_changed_token: Cell<u64>,
}

ferro_class! {
    VirtualizingPanel: Panel, virtuals VirtualizingPanelImpl: PanelImpl {
        /// Scrolls the specified item into view.
        ///
        /// Returns the element with the specified index, or `None` if the
        /// element could not be brought into view.
        fn scroll_into_view(this, index: i32) -> Option<Ref<Control>>;
        /// Returns the container for the item at the specified index.
        ///
        /// Returns the container for the item at the specified index within
        /// the item collection, if the item is realized; otherwise, `None`.
        ///
        /// Note for implementors: if the item at the specified index is an
        /// item-is-self-container item that has previously been realized,
        /// then the item should be returned even if it currently falls
        /// outside the realized viewport.
        fn container_from_index(this, index: i32) -> Option<Ref<Control>>;
        /// Returns the index to the item that has the specified realized
        /// container.
        ///
        /// Returns the index to the item that corresponds to the specified
        /// realized container, or -1 if `container` is not found.
        fn index_from_container(this, container: &Ref<Control>) -> i32;
        /// Gets the currently realized containers.
        fn get_realized_containers(this) -> Option<Vec<Ref<Control>>>;
        /// Gets the next control in the specified direction.
        ///
        /// `from` is the control from which movement begins and `wrap`
        /// tells whether to wrap around when the first or last item is
        /// reached.
        fn get_control_in_direction(
            this,
            direction: NavigationDirection,
            from: Option<&Ref<InputElement>>,
            wrap: bool
        ) -> Option<Ref<InputElement>>;
        /// Called when the [`ItemsControl`] that owns the panel changes.
        fn on_items_control_changed(this, old_value: Option<&Ref<ItemsControl>>);
        /// Called when the `items` collection of the owner [`ItemsControl`]
        /// changes.
        ///
        /// This method is called a `collection changed` event is raised by
        /// the items, or when the items property of the items control is
        /// set to a new collection, in which case the action will be a
        /// reset.
        fn on_items_changed(this, items: &Rc<ItemsSourceView>, e: &ItemsChangedEventArgs<'_>);
    }
}

ferro_impl_classes!(
    VirtualizingPanel: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl PanelImpl for VirtualizingPanel {
    fn invalidate_measure_on_children_changed(_this: &Self) {
        // Don't invalidate measure when children are added or removed: the
        // panel is responsible for managing its children.
    }
}

impl VirtualizingPanelImpl for VirtualizingPanel {
    fn scroll_into_view(_this: &Self, _index: i32) -> Option<Ref<Control>> {
        panic!("VirtualizingPanel is abstract.")
    }

    fn container_from_index(_this: &Self, _index: i32) -> Option<Ref<Control>> {
        panic!("VirtualizingPanel is abstract.")
    }

    fn index_from_container(_this: &Self, _container: &Ref<Control>) -> i32 {
        panic!("VirtualizingPanel is abstract.")
    }

    fn get_realized_containers(_this: &Self) -> Option<Vec<Ref<Control>>> {
        panic!("VirtualizingPanel is abstract.")
    }

    fn get_control_in_direction(
        _this: &Self,
        _direction: NavigationDirection,
        _from: Option<&Ref<InputElement>>,
        _wrap: bool,
    ) -> Option<Ref<InputElement>> {
        panic!("VirtualizingPanel is abstract.")
    }

    fn on_items_control_changed(_this: &Self, _old_value: Option<&Ref<ItemsControl>>) {}

    fn on_items_changed(_this: &Self, _items: &Rc<ItemsSourceView>, _e: &ItemsChangedEventArgs<'_>) {}
}

impl INavigableContainer for VirtualizingPanel {
    fn get_control(
        &self,
        direction: NavigationDirection,
        from: Option<&Ref<InputElement>>,
        wrap: bool,
    ) -> Option<Ref<InputElement>> {
        self.get_control_in_direction(direction, from, wrap)
    }
}

impl VirtualizingPanel {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Panel::construct(), items_control: RefCell::new(None), items_changed_token: Cell::new(0) }
    }

    /// Gets the [`ItemContainerGenerator`] for this [`VirtualizingPanel`].
    pub fn item_container_generator(&self) -> Option<Rc<ItemContainerGenerator>> {
        self.items_control().map(|items_control| items_control.item_container_generator())
    }

    /// Gets the items to display.
    pub fn items(&self) -> Rc<ItemsSourceView> {
        match self.items_control() {
            Some(items_control) => items_control.items_view().clone(),
            None => ItemsSourceView::empty(),
        }
    }

    /// Gets the [`ItemsControl`] that the panel is displaying items for.
    pub fn items_control(&self) -> Option<Ref<ItemsControl>> {
        self.items_control.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn set_items_control(&self, value: Option<Ref<ItemsControl>>) {
        let old_value = self.items_control();
        if old_value != value {
            *self.items_control.borrow_mut() = value.as_ref().map(Ref::downgrade);
            self.on_items_control_changed(old_value.as_ref());
        }
    }

    /// Adds the specified [`Control`] to the children collection of a
    /// [`VirtualizingPanel`] element.
    pub fn add_internal_child(&self, control: &Ref<Control>) {
        let items_control = self.ensure_items_control();
        items_control.add_logical_child(control);
        self.children().add(control);
    }

    /// Adds the specified [`Control`] to the children collection of a
    /// [`VirtualizingPanel`] element at the specified index position.
    pub fn insert_internal_child(&self, index: usize, control: &Ref<Control>) {
        let items_control = self.ensure_items_control();
        items_control.add_logical_child(control);
        self.children().insert(index, control);
    }

    /// Removes a child element from the children collection.
    pub fn remove_internal_child(&self, child: &Ref<Control>) {
        let items_control = self.ensure_items_control();
        items_control.remove_logical_child(child);
        self.children().remove(child);
    }

    /// Removes child elements from the children collection.
    pub fn remove_internal_child_range(&self, index: usize, count: usize) {
        let items_control = self.ensure_items_control();

        for i in index..count {
            let c = self.children().get(i);
            items_control.remove_logical_child(&c);
        }

        self.children().remove_range(index, count);
    }

    pub(crate) fn attach(&self, items_control: &Ref<ItemsControl>) {
        if self.items_control().is_some() {
            panic!("The VirtualizingPanel is already attached to an ItemsControl");
        }

        self.set_items_control(Some(items_control.clone()));

        let weak = self.to_ref().downgrade();
        let token = items_control.items_view().add_post_collection_changed(Rc::new(
            move |e: &ItemsChangedEventArgs<'_>| {
                if let Some(this) = weak.upgrade() {
                    this.on_items_control_items_changed(e);
                }
            },
        ));
        self.items_changed_token.set(token);
    }

    pub(crate) fn detach(&self) {
        let items_control = self.ensure_items_control();
        items_control.items_view().remove_post_collection_changed(self.items_changed_token.get());
        self.set_items_control(None);
        self.children().clear();
    }

    pub(crate) fn refresh(&self) {
        self.on_items_control_items_changed(&CollectionUtils::RESET_EVENT_ARGS);
    }

    fn ensure_items_control(&self) -> Ref<ItemsControl> {
        match self.items_control() {
            Some(items_control) => items_control,
            None => panic!("The VirtualizingPanel does not belong to an ItemsControl."),
        }
    }

    fn on_items_control_items_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        self.on_items_changed(&self.items(), e);
    }
}
