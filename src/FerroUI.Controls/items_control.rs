use crate::assigned_binding::AssignedBinding;
use crate::generators::{ItemContainerGenerator, RecycleKey};
use crate::items_source::{ItemsChangedEventArgs, ItemsSource, ItemsView};
use crate::metadata::PseudoClassesAttribute;
use crate::navigable_containers::{as_navigable_container, register_navigable_container};
use crate::presenters::{ContentPresenter, ItemsPresenter};
use crate::primitives::{
    HeaderedContentControl, HeaderedItemsControl, HeaderedSelectingItemsControl, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl,
    TemplatedControlImplExt,
};
use crate::templates::{FuncDataTemplate, FuncTemplate, IDataTemplate, ITemplateOf};
use crate::{
    Canvas, ContainerClearingEventArgs, ContainerIndexChangedEventArgs, ContainerPreparedEventArgs, ContentControl,
    Control, ControlImpl, ItemCollection, ItemsSourceView, Panel, StackPanel, TextBlock, VirtualizingPanel, WrapPanel,
};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::input::{
    FocusChangedEventArgs, FocusManager, INavigableContainer, InputElement, InputElementImpl, InputElementImplExt,
    KeyEventArgs,
    KeyModifiers, KeyboardNavigation, NavigationDirection, NavigationMethod,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::{ChildIndexChangedEventArgs, IChildIndexProvider};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::ControlTheme;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AnyValue, AttachedProperty, BoxedValue,
    DirectProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    ObjectType, PropertyValue, Ref, StyledElement, StyledElementImpl, StyledProperty, Upcast, Visual, VisualImpl,
    WeakRef,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

type ChildIndexChangedHandlers = HandlerList<dyn Fn(&ChildIndexChangedEventArgs)>;

/// Displays a collection of items.
#[repr(C)]
pub struct ItemsControl {
    base: TemplatedControl,
    items: ItemCollection,
    item_count: Cell<i32>,
    item_container_generator: OnceCell<Rc<ItemContainerGenerator>>,
    child_index_changed: Rc<ChildIndexChangedHandlers>,
    child_index_provider: OnceCell<Rc<ItemsControlChildIndexProvider>>,
    display_member_item_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    items_presenter: RefCell<Option<Ref<ItemsPresenter>>>,
    presenter: RefCell<Option<Ref<ItemsPresenter>>>,
    wrap_focus: Cell<bool>,
    preparing_container: HandlerList<dyn Fn(&ContainerPreparedEventArgs)>,
    container_prepared: HandlerList<dyn Fn(&ContainerPreparedEventArgs)>,
    container_index_changed: HandlerList<dyn Fn(&ContainerIndexChangedEventArgs)>,
    container_clearing: HandlerList<dyn Fn(&ContainerClearingEventArgs)>,
}

ferro_class! {
    ItemsControl: TemplatedControl, virtuals ItemsControlImpl: TemplatedControlImpl {
        /// Creates or a container that can be used to display an item.
        fn create_container_for_item_override(
            this,
            item: &Option<BoxedValue>,
            index: i32,
            recycle_key: Option<RecycleKey>
        ) -> Ref<Control>;
        /// Prepares the specified element to display the specified item.
        fn prepare_container_for_item_override(this, container: &Ref<Control>, item: &Option<BoxedValue>, index: i32);
        /// Called when a container has been fully prepared to display an
        /// item.
        ///
        /// This method will be called when a container has been fully
        /// prepared and added to the logical and visual trees, but may be
        /// called before a layout pass has completed. It is called
        /// immediately before the `container_prepared` event is raised.
        fn container_for_item_prepared_override(this, container: &Ref<Control>, item: &Option<BoxedValue>, index: i32);
        /// Called when the index for a container changes due to an
        /// insertion or removal in the items collection.
        fn container_index_changed_override(this, container: &Ref<Control>, old_index: i32, new_index: i32);
        /// Undoes the effects of the `prepare_container_for_item_override`
        /// method.
        fn clear_container_for_item_override(this, container: &Ref<Control>);
        /// Determines whether the specified item can be its own container.
        ///
        /// Returns true if the item needs a container; otherwise false if
        /// the item can itself be used as a container. The second value is
        /// a key that can be used to locate a previously recycled container
        /// of the correct type, or `None` if the item cannot be recycled.
        /// If the item is its own container then by definition it cannot be
        /// recycled, so the key should be `None`.
        fn needs_container_override(this, item: &Option<BoxedValue>, index: i32) -> (bool, Option<RecycleKey>);
        /// Called when the collection changed event is raised on the items
        /// view.
        fn on_items_view_collection_changed(this, e: &ItemsChangedEventArgs<'_>);
    }
}
ferroui_base::ferro_class_info!(ItemsControl { new: ItemsControl::new });

ferro_impl_classes!(ItemsControl: VisualImpl, LayoutableImpl, InteractiveImpl);

impl ControlImpl for ItemsControl {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ItemsControlAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for ItemsControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.update_pseudo_classes();

        let weak = this.to_ref().downgrade();
        this.items.add_collection_changed(Rc::new(move |e: &ItemsChangedEventArgs<'_>| {
            if let Some(this) = weak.upgrade() {
                this.on_items_view_collection_changed(e);
            }
        }));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::item_container_theme_property().as_property()
            && this.item_container_generator.get().is_some()
        {
            this.refresh_containers();
        } else if change.property() == Self::items_source_property().as_property() {
            this.items.set_items_source(change.get_new_value::<Option<ItemsSource>>());
        } else if change.property() == Self::item_template_property().as_property() {
            if change.get_new_value::<Option<Rc<dyn IDataTemplate>>>().is_some()
                && this.display_member_binding().is_some()
            {
                panic!("Cannot set both DisplayMemberBinding and ItemTemplate.");
            }
            this.refresh_containers();
        } else if change.property() == Self::display_member_binding_property().as_property() {
            if change.get_new_value::<Option<AssignedBinding>>().is_some() && this.item_template().is_some() {
                panic!("Cannot set both DisplayMemberBinding and ItemTemplate.");
            }
            *this.display_member_item_template.borrow_mut() = None;
            this.refresh_containers();
        }
    }
}

impl StyledElementImpl for ItemsControl {
    fn child_index_provider(this: &Self) -> Option<Rc<dyn IChildIndexProvider>> {
        let provider = this
            .child_index_provider
            .get_or_init(|| Rc::new(ItemsControlChildIndexProvider { owner: this.to_ref().downgrade() }))
            .clone();
        Some(provider)
    }
}

impl InputElementImpl for ItemsControl {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);

        // If the focus is coming from a child control, set the tab once
        // active element to the focused control. This ensures that tabbing
        // back into the control will focus the last focused control when
        // the tab navigation mode is `Once`.
        if !e.is_source(this) {
            if let Some(ie) = e.source().and_then(|source| source.cast::<InputElement>()) {
                KeyboardNavigation::set_tab_once_active_element(this, Some(ie));
            }
        }
    }

    /// Handles directional navigation within the [`ItemsControl`].
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        if !e.handled() {
            let focus = FocusManager::get_focus_manager(this);
            let direction = e.key.to_navigation_direction(KeyModifiers::NONE);
            let panel = this.presenter().and_then(|presenter| presenter.panel());
            let container = panel.as_ref().and_then(|panel| as_navigable_container(panel));
            let focused = focus.as_ref().and_then(|focus| focus.get_focused_element());

            let (Some(container), Some(focused), Some(direction), Some(panel)) =
                (container, focused, direction, panel.as_ref())
            else {
                return;
            };

            if direction.is_tab() {
                return;
            }

            let panel_visual: Ref<Visual> = panel.clone().upcast();
            let mut current: Option<Ref<Visual>> = Some(focused.upcast());

            while let Some(visual) = current {
                let parent = visual.visual_parent();

                if parent.as_ref() == Some(&panel_visual) {
                    if let Some(input_element) = visual.cast::<InputElement>() {
                        let next = Self::get_next_control(container, direction, Some(&input_element), this.wrap_focus());

                        if let Some(next) = next {
                            next.focus_with(NavigationMethod::Directional, e.key_modifiers);
                            e.set_handled(true);
                        }

                        break;
                    }
                }

                current = parent;
            }
        }

        Self::parent_on_key_down(this, e);
    }
}

impl TemplatedControlImpl for ItemsControl {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        *this.items_presenter.borrow_mut() = e.name_scope().find_as::<ItemsPresenter>("PART_ItemsPresenter");
    }
}

impl ItemsControlImpl for ItemsControl {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        ContentPresenter::new().upcast()
    }

    fn prepare_container_for_item_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        _index: i32,
    ) {
        let item_control = item.as_ref().and_then(Control::from_boxed);

        if item_control.as_ref() == Some(container) {
            return;
        }

        let item_template = this.get_effective_item_template();

        if let Some(hcc) = container.downcast_ref::<HeaderedContentControl>() {
            Self::set_if_unset(hcc, ContentControl::content_property(), item.clone());

            // An item with a header: a headered content control.
            let headered = item_control.as_ref().and_then(|c| crate::i_headered::as_headered(c));
            if let Some(headered) = headered {
                Self::set_if_unset(hcc, HeaderedContentControl::header_property(), headered.header());
            } else if !Self::is_visual(item) {
                Self::set_if_unset(hcc, HeaderedContentControl::header_property(), item.clone());
            }

            if let Some(item_template) = &item_template {
                Self::set_if_unset(
                    hcc,
                    HeaderedContentControl::header_template_property(),
                    Some(item_template.clone()),
                );
            }
        } else if let Some(cc) = container.downcast_ref::<ContentControl>() {
            Self::set_if_unset(cc, ContentControl::content_property(), item.clone());
            if let Some(item_template) = &item_template {
                Self::set_if_unset(cc, ContentControl::content_template_property(), Some(item_template.clone()));
            }
        } else if let Some(p) = container.downcast_ref::<ContentPresenter>() {
            Self::set_if_unset(p, ContentPresenter::content_property(), item.clone());
            if let Some(item_template) = &item_template {
                Self::set_if_unset(p, ContentPresenter::content_template_property(), Some(item_template.clone()));
            }
        } else if let Some(ic) = container.downcast_ref::<ItemsControl>() {
            if let Some(item_template) = &item_template {
                Self::set_if_unset(ic, Self::item_template_property(), Some(item_template.clone()));
            }
            if let Some(ict) = this.item_container_theme() {
                Self::set_if_unset(ic, Self::item_container_theme_property(), Some(ict));
            }
        }

        // These conditions are separate because the headered items controls
        // also need to run the items control preparation.
        if let Some(hic) = container.downcast_ref::<HeaderedItemsControl>() {
            Self::set_if_unset(hic, HeaderedItemsControl::header_property(), item.clone());
            Self::set_if_unset(hic, HeaderedItemsControl::header_template_property(), item_template.clone());
            hic.prepare_item_container(this);
        } else if let Some(hsic) = container.downcast_ref::<HeaderedSelectingItemsControl>() {
            Self::set_if_unset(hsic, HeaderedSelectingItemsControl::header_property(), item.clone());
            Self::set_if_unset(hsic, HeaderedSelectingItemsControl::header_template_property(), item_template.clone());
            hsic.prepare_item_container(this);
        }
    }

    fn container_for_item_prepared_override(
        _this: &Self,
        _container: &Ref<Control>,
        _item: &Option<BoxedValue>,
        _index: i32,
    ) {
    }

    fn container_index_changed_override(_this: &Self, _container: &Ref<Control>, _old_index: i32, _new_index: i32) {}

    fn clear_container_for_item_override(_this: &Self, container: &Ref<Control>) {
        if let Some(hcc) = container.downcast_ref::<HeaderedContentControl>() {
            hcc.clear_value(ContentControl::content_property());
            hcc.clear_value(HeaderedContentControl::header_property());
            hcc.clear_value(HeaderedContentControl::header_template_property());
        } else if let Some(cc) = container.downcast_ref::<ContentControl>() {
            cc.clear_value(ContentControl::content_property());
            cc.clear_value(ContentControl::content_template_property());
        } else if let Some(p) = container.downcast_ref::<ContentPresenter>() {
            p.clear_value(ContentPresenter::content_property());
            p.clear_value(ContentPresenter::content_template_property());
        } else if let Some(ic) = container.downcast_ref::<ItemsControl>() {
            ic.clear_value(Self::item_template_property());
            ic.clear_value(Self::item_container_theme_property());
        }

        if let Some(hic) = container.downcast_ref::<HeaderedItemsControl>() {
            hic.clear_value(HeaderedItemsControl::header_property());
            hic.clear_value(HeaderedItemsControl::header_template_property());
        } else if let Some(hsic) = container.downcast_ref::<HeaderedSelectingItemsControl>() {
            hsic.clear_value(HeaderedSelectingItemsControl::header_property());
            hsic.clear_value(HeaderedSelectingItemsControl::header_template_property());
        }

        // Feels like we should be clearing the headered items control's
        // items binding here, but looking at the WPF source it seems that
        // this isn't done there.
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<Control>(item)
    }

    fn on_items_view_collection_changed(this: &Self, e: &ItemsChangedEventArgs<'_>) {
        if !this.items.is_read_only() {
            match e.action {
                NotifyCollectionChangedAction::Add => this.add_control_items_to_logical_children(e.new_items),
                NotifyCollectionChangedAction::Remove => this.remove_control_items_from_logical_children(e.old_items),
                _ => {}
            }
        }

        this.set_item_count(this.items_view().count() as i32);
    }
}

impl ItemsControl {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[":empty", ":singleitem"]);
}

ferroui_base::ferro_properties! { impl ItemsControl {
    ferro_property!(
        /// Defines the `ItemContainerTheme` property.
        pub fn item_container_theme_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<ItemsControl, _>("ItemContainerTheme", None)
        }
    );

    ferro_property!(
        /// Defines the `ItemCount` property.
        pub fn item_count_property() -> DirectProperty<ItemsControl, i32> {
            FerroProperty::register_direct::<ItemsControl, _>("ItemCount", |o| o.item_count(), None, 0)
        }
    );

    ferro_property!(
        /// Defines the `ItemsPanel` property.
        pub fn items_panel_property() -> StyledProperty<Rc<dyn ITemplateOf<Option<Ref<Panel>>>>> {
            // The default value for the `ItemsPanel` property.
            let default_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
                FuncTemplate::new(|| Some(StackPanel::new().upcast::<Panel>()));
            FerroProperty::register::<ItemsControl, _>("ItemsPanel", default_panel)
        }
    );

    ferro_property!(
        /// Defines the `ItemsSource` property.
        pub fn items_source_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<ItemsControl, _>("ItemsSource", None)
        }
    );

    ferro_property!(
        /// Defines the `ItemTemplate` property.
        pub fn item_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<ItemsControl, _>("ItemTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `DisplayMemberBinding` property.
        pub fn display_member_binding_property() -> StyledProperty<Option<AssignedBinding>> {
            FerroProperty::register::<ItemsControl, _>("DisplayMemberBinding", None)
        }
    );

    ferro_property!(
        fn applied_item_container_theme_property() -> AttachedProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register_attached::<ItemsControl, Control, _>("AppliedItemContainerTheme", None)
        }
    );
} }

impl ItemsControl {
    fn static_constructor() {
        register_navigable_container::<StackPanel>();
        register_navigable_container::<WrapPanel>();
        register_navigable_container::<Canvas>();
        register_navigable_container::<VirtualizingPanel>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            items: ItemCollection::new(),
            item_count: Cell::new(0),
            item_container_generator: OnceCell::new(),
            child_index_changed: Rc::new(HandlerList::new()),
            child_index_provider: OnceCell::new(),
            display_member_item_template: RefCell::new(None),
            items_presenter: RefCell::new(None),
            presenter: RefCell::new(None),
            wrap_focus: Cell::new(false),
            preparing_container: HandlerList::new(),
            container_prepared: HandlerList::new(),
            container_index_changed: HandlerList::new(),
            container_clearing: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the binding to use for binding to the display member of
    /// each item.
    pub fn display_member_binding(&self) -> Option<AssignedBinding> {
        self.get_value(Self::display_member_binding_property())
    }

    pub fn set_display_member_binding(&self, value: Option<AssignedBinding>) {
        self.set_value(Self::display_member_binding_property(), value)
    }

    /// Gets the [`ItemContainerGenerator`] for the control.
    pub fn item_container_generator(&self) -> Rc<ItemContainerGenerator> {
        self.item_container_generator.get_or_init(|| Rc::new(ItemContainerGenerator::new(self))).clone()
    }

    /// Gets the items to display.
    ///
    /// You use either the `items` or the `items_source` property to specify
    /// the collection that should be used to generate the content of your
    /// [`ItemsControl`]. When the `items_source` property is set, the
    /// `items` collection is made read-only and fixed-size.
    ///
    /// When `items_source` is in use, setting the `items_source` property
    /// to `None` removes the collection and restores usage to `items`,
    /// which will be an empty [`ItemCollection`].
    pub fn items(&self) -> ItemCollection {
        self.items.clone()
    }

    /// Gets or sets the [`ControlTheme`] that is applied to the container
    /// element generated for each item.
    pub fn item_container_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::item_container_theme_property())
    }

    pub fn set_item_container_theme(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::item_container_theme_property(), value)
    }

    /// Gets the number of items being displayed by the [`ItemsControl`].
    pub fn item_count(&self) -> i32 {
        self.item_count.get()
    }

    fn set_item_count(&self, value: i32) {
        if self.set_and_raise_cell(Self::item_count_property(), &self.item_count, value) {
            self.update_pseudo_classes();
            self.raise_child_index_changed(&ChildIndexChangedEventArgs::total_count_changed());
        }
    }

    /// Gets or sets the panel used to display the items.
    pub fn items_panel(&self) -> Rc<dyn ITemplateOf<Option<Ref<Panel>>>> {
        self.get_value(Self::items_panel_property())
    }

    pub fn set_items_panel(&self, value: Rc<dyn ITemplateOf<Option<Ref<Panel>>>>) {
        self.set_value(Self::items_panel_property(), value)
    }

    /// Gets or sets a collection used to generate the content of the
    /// [`ItemsControl`].
    ///
    /// A common scenario is to use an [`ItemsControl`] such as a list box
    /// to display a data collection, or to bind an [`ItemsControl`] to a
    /// collection object. To bind an [`ItemsControl`] to a collection
    /// object, use the `items_source` property.
    ///
    /// When the `items_source` property is set, the `items` collection is
    /// made read-only and fixed-size.
    ///
    /// When `items_source` is in use, setting the property to `None`
    /// removes the collection and restores usage to `items`, which will be
    /// an empty [`ItemCollection`].
    pub fn items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::items_source_property())
    }

    pub fn set_items_source(&self, value: Option<ItemsSource>) {
        self.set_value(Self::items_source_property(), value)
    }

    /// Gets or sets the data template used to display the items in the
    /// control.
    pub fn item_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::item_template_property())
    }

    pub fn set_item_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::item_template_property(), value)
    }

    /// Gets the items presenter control.
    pub fn presenter(&self) -> Option<Ref<ItemsPresenter>> {
        self.presenter.borrow().clone()
    }

    /// Gets the [`Panel`] specified by `items_panel`.
    pub fn items_panel_root(&self) -> Option<Ref<Panel>> {
        self.presenter().and_then(|presenter| presenter.panel())
    }

    /// Gets a read-only view of the items in the [`ItemsControl`].
    pub fn items_view(&self) -> &Rc<ItemsSourceView> {
        self.items.view()
    }

    /// Whether directional navigation wraps around at the first and last
    /// items.
    pub fn wrap_focus(&self) -> bool {
        self.wrap_focus.get()
    }

    pub fn set_wrap_focus(&self, value: bool) {
        self.wrap_focus.set(value)
    }

    /// Occurs immediately before a container is prepared for use.
    ///
    /// The prepared element might be newly created or an existing container
    /// that is being re-used.
    pub fn preparing_container(&self, handler: impl Fn(&ContainerPreparedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.preparing_container.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.preparing_container.remove(token);
            }
        })
    }

    /// Occurs each time a container is prepared for use.
    ///
    /// The prepared element might be newly created or an existing container
    /// that is being re-used.
    pub fn container_prepared(&self, handler: impl Fn(&ContainerPreparedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.container_prepared.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.container_prepared.remove(token);
            }
        })
    }

    /// Occurs for each realized container when the index for the item it
    /// represents has changed.
    ///
    /// This event is raised for each realized container where the index for
    /// the item it represents has changed. For example, when another item
    /// is added or removed in the data source, the index for items that
    /// come after in the ordering will be impacted.
    pub fn container_index_changed(
        &self,
        handler: impl Fn(&ContainerIndexChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.container_index_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.container_index_changed.remove(token);
            }
        })
    }

    /// Occurs each time a container is cleared.
    ///
    /// This event is raised immediately each time an container is cleared,
    /// such as when it falls outside the range of realized items or the
    /// corresponding item is removed.
    pub fn container_clearing(&self, handler: impl Fn(&ContainerClearingEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.container_clearing.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.container_clearing.remove(token);
            }
        })
    }

    /// Gets a default recycle key that can be used when an [`ItemsControl`]
    /// supports a single container type.
    pub const DEFAULT_RECYCLE_KEY: RecycleKey = RecycleKey::Default;

    /// Returns the container for the item at the specified index.
    ///
    /// Returns the container for the item at the specified index within the
    /// item collection, if the item has a container; otherwise, `None`.
    pub fn container_from_index(&self, index: i32) -> Option<Ref<Control>> {
        self.presenter().and_then(|presenter| presenter.container_from_index(index))
    }

    /// Returns the container corresponding to the specified item.
    ///
    /// Returns a container that corresponds to the specified item, if the
    /// item has a container and exists in the collection; otherwise,
    /// `None`.
    pub fn container_from_item(&self, item: &Option<BoxedValue>) -> Option<Ref<Control>> {
        let index = self.items.index_of(item);
        if index >= 0 {
            self.container_from_index(index)
        } else {
            None
        }
    }

    /// Returns the index to the item that has the specified, generated
    /// container.
    ///
    /// Returns the index to the item that corresponds to the specified
    /// generated container, or -1 if `container` is not found.
    pub fn index_from_container(&self, container: &Ref<Control>) -> i32 {
        self.presenter().map_or(-1, |presenter| presenter.index_from_container(container))
    }

    /// Returns the item that corresponds to the specified, generated
    /// container.
    ///
    /// Returns the contained item, or `None` if the container does not
    /// contain an item.
    pub fn item_from_container(&self, container: &Ref<Control>) -> Option<BoxedValue> {
        let index = self.index_from_container(container);
        if index >= 0 && (index as usize) < self.items.count() {
            self.items.get_at(index as usize)
        } else {
            None
        }
    }

    /// Gets the currently realized containers.
    pub fn get_realized_containers(&self) -> Vec<Ref<Control>> {
        self.presenter().and_then(|presenter| presenter.get_realized_containers()).unwrap_or_default()
    }

    /// Scrolls the specified item into view.
    pub fn scroll_into_view(&self, index: i32) {
        if let Some(presenter) = self.presenter() {
            presenter.scroll_into_view(index);
        }
    }

    /// Scrolls the specified item into view.
    pub fn scroll_into_view_item(&self, item: &Option<BoxedValue>) {
        self.scroll_into_view(self.items_view().index_of(item))
    }

    /// Returns the [`ItemsControl`] that owns the specified container
    /// control.
    ///
    /// Returns the owning [`ItemsControl`] or `None` if the control is not
    /// an items container.
    pub fn items_control_from_item_container(container: &Ref<Control>) -> Option<Ref<ItemsControl>> {
        let mut c = container.parent().and_then(|parent| parent.cast::<Control>());

        while let Some(control) = c {
            if let Some(items_control) = control.cast::<ItemsControl>() {
                return if items_control.index_from_container(container) >= 0 { Some(items_control) } else { None };
            }

            c = control.parent().and_then(|parent| parent.cast::<Control>());
        }

        None
    }

    /// A default implementation of `needs_container_override` that returns
    /// true and sets the recycle key to
    /// [`DEFAULT_RECYCLE_KEY`](Self::DEFAULT_RECYCLE_KEY) if the item is
    /// not a `T`.
    pub fn needs_container<T: ObjectType + Upcast<Control>>(
        &self,
        item: &Option<BoxedValue>,
    ) -> (bool, Option<RecycleKey>) {
        let is_t = item.as_ref().and_then(Control::from_boxed).is_some_and(|control| control.is::<T>());

        if is_t {
            (false, None)
        } else {
            (true, Some(Self::DEFAULT_RECYCLE_KEY))
        }
    }

    /// Refreshes the containers displayed by the control.
    ///
    /// Causes all containers to be unrealized and re-realized.
    pub fn refresh_containers(&self) {
        if let Some(presenter) = self.presenter() {
            presenter.refresh();
        }
    }

    pub(crate) fn add_logical_child(&self, c: &Ref<Control>) {
        let child: Ref<StyledElement> = c.clone().upcast();
        if !self.logical_children().contains(&child) {
            self.logical_children().add(child);
        }
    }

    pub(crate) fn remove_logical_child(&self, c: &Ref<Control>) {
        let child: Ref<StyledElement> = c.clone().upcast();
        self.logical_children().remove(&child);
    }

    /// Called by [`ItemsPresenter`] to register with the [`ItemsControl`].
    ///
    /// Items presenters can be within nested templates or in popups and so
    /// are not necessarily created immediately when the items control's
    /// template is instantiated. Instead they register themselves using
    /// this method.
    pub(crate) fn register_items_presenter(&self, presenter: &ItemsPresenter) {
        *self.presenter.borrow_mut() = Some(presenter.to_ref());
        self.raise_child_index_changed(&ChildIndexChangedEventArgs::child_indexes_reset());
    }

    pub(crate) fn prepare_item_container(&self, container: &Ref<Control>, item: &Option<BoxedValue>, index: i32) {
        if !self.preparing_container.is_empty() {
            let e = ContainerPreparedEventArgs::new(container.clone(), index);
            for (_, handler) in self.preparing_container.snapshot().iter() {
                handler(&e);
            }
        }

        // If the container has no theme set, or we've already applied our
        // item container theme (and it hasn't changed since) then we're in
        // control of the container's theme and may need to update it.
        if !container.is_set(StyledElement::theme_property().as_property())
            || container.get_value(Self::applied_item_container_theme_property()) == container.theme()
        {
            let item_container_theme = self.item_container_theme();
            let matches = item_container_theme
                .as_ref()
                .and_then(|theme| theme.target_type())
                .is_some_and(|target_type| target_type.is_assignable_from(container.style_key()));

            if matches {
                // We have an item container theme and it matches the
                // container. Set the theme property, and mark the container
                // as having had the item container theme applied.
                container.set_current_value(StyledElement::theme_property(), item_container_theme.clone());
                container.set_value(Self::applied_item_container_theme_property(), item_container_theme);
            } else {
                // Otherwise clear the theme and the applied item container
                // theme property.
                container.clear_value(StyledElement::theme_property());
                container.clear_value(Self::applied_item_container_theme_property());
            }
        }

        if item.as_ref().and_then(Control::from_boxed).is_none() {
            container.set_data_context(item.clone());
        }

        self.prepare_container_for_item_override(container, item, index);
    }

    pub(crate) fn item_container_prepared(&self, container: &Ref<Control>, item: &Option<BoxedValue>, index: i32) {
        self.container_for_item_prepared_override(container, item, index);
        if !self.child_index_changed.is_empty() {
            self.raise_child_index_changed(&ChildIndexChangedEventArgs::new(container.clone().upcast(), index));
        }
        if !self.container_prepared.is_empty() {
            let e = ContainerPreparedEventArgs::new(container.clone(), index);
            for (_, handler) in self.container_prepared.snapshot().iter() {
                handler(&e);
            }
        }
    }

    pub(crate) fn item_container_index_changed(&self, container: &Ref<Control>, old_index: i32, new_index: i32) {
        self.container_index_changed_override(container, old_index, new_index);
        if !self.child_index_changed.is_empty() {
            self.raise_child_index_changed(&ChildIndexChangedEventArgs::new(container.clone().upcast(), new_index));
        }
        if !self.container_index_changed.is_empty() {
            let e = ContainerIndexChangedEventArgs::new(container.clone(), old_index, new_index);
            for (_, handler) in self.container_index_changed.snapshot().iter() {
                handler(&e);
            }
        }
    }

    pub(crate) fn clear_item_container(&self, container: &Ref<Control>) {
        self.clear_container_for_item_override(container);
        if !self.container_clearing.is_empty() {
            let e = ContainerClearingEventArgs::new(container.clone());
            for (_, handler) in self.container_clearing.snapshot().iter() {
                handler(&e);
            }
        }
    }

    fn add_control_items_to_logical_children(&self, items: ItemsView<'_>) {
        let mut to_add: Option<Vec<Ref<StyledElement>>> = None;

        for i in items {
            if let Some(control) = i.as_ref().and_then(Control::from_boxed) {
                let control: Ref<StyledElement> = control.upcast();
                if !self.logical_children().contains(&control) {
                    to_add.get_or_insert_with(Vec::new).push(control);
                }
            }
        }

        if let Some(to_add) = to_add {
            self.logical_children().add_range(to_add);
        }
    }

    pub(crate) fn set_if_unset<T: PropertyValue>(target: &FerroObject, property: &'static StyledProperty<T>, value: T) {
        if !target.is_set(property.as_property()) {
            target.set_current_value(property, value);
        }
    }

    fn remove_control_items_from_logical_children(&self, items: ItemsView<'_>) {
        let mut to_remove: Option<Vec<Ref<StyledElement>>> = None;

        for i in items {
            if let Some(control) = i.as_ref().and_then(Control::from_boxed) {
                to_remove.get_or_insert_with(Vec::new).push(control.upcast());
            }
        }

        if let Some(to_remove) = to_remove {
            self.logical_children().remove_all(to_remove);
        }
    }

    /// The template used to display the items: the item template, or the
    /// template created for the display member binding.
    pub(crate) fn get_effective_item_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        if let Some(item_template) = self.item_template() {
            return Some(item_template);
        }

        if self.display_member_item_template.borrow().is_none() {
            if let Some(binding) = self.display_member_binding() {
                let template: Rc<dyn IDataTemplate> = FuncDataTemplate::new(
                    |_| true,
                    move |_, _| {
                        let text_block = TextBlock::new();
                        text_block.bind_binding(TextBlock::text_property().as_property(), &*binding);
                        Some(text_block.upcast())
                    },
                    false,
                );
                *self.display_member_item_template.borrow_mut() = Some(template);
            }
        }

        self.display_member_item_template.borrow().clone()
    }

    fn update_pseudo_classes(&self) {
        self.pseudo_classes().set(":empty", self.item_count() == 0);
        self.pseudo_classes().set(":singleitem", self.item_count() == 1);
    }

    /// Whether an item is a visual: a control (see `Control::boxed`) or a
    /// visual handle.
    fn is_visual(item: &Option<BoxedValue>) -> bool {
        match item {
            Some(item) => {
                let value: &dyn AnyValue = &**item;
                value.is::<Ref<Control>>() || value.is::<Ref<Visual>>()
            }
            None => false,
        }
    }

    /// Gets the next focusable, enabled and visible control of a container
    /// in the specified direction.
    pub fn get_next_control(
        container: &dyn INavigableContainer,
        mut direction: NavigationDirection,
        from: Option<&Ref<InputElement>>,
        wrap: bool,
    ) -> Option<Ref<InputElement>> {
        let mut from: Option<Ref<InputElement>> = from.cloned();
        let mut current = from.clone();

        loop {
            let result = container.get_control(direction, current.as_ref(), wrap)?;

            if result.focusable() && result.is_effectively_enabled() && result.is_effectively_visible() {
                return Some(result);
            }

            current = Some(result.clone());

            if current == from {
                return None;
            }

            match direction {
                // We did not find an enabled first item. Move downwards
                // until we find one.
                NavigationDirection::First => {
                    direction = NavigationDirection::Down;
                    from = Some(result);
                }
                // We did not find an enabled last item. Move upwards until
                // we find one.
                NavigationDirection::Last => {
                    direction = NavigationDirection::Up;
                    from = Some(result);
                }
                _ => {}
            }
        }
    }

    fn raise_child_index_changed(&self, e: &ChildIndexChangedEventArgs) {
        if !self.child_index_changed.is_empty() {
            for (_, handler) in self.child_index_changed.snapshot().iter() {
                handler(e);
            }
        }
    }
}

/// The child index provider of an items control.
struct ItemsControlChildIndexProvider {
    owner: WeakRef<ItemsControl>,
}

impl IChildIndexProvider for ItemsControlChildIndexProvider {
    fn get_child_index(&self, child: &StyledElement) -> i32 {
        let Some(owner) = self.owner.upgrade() else { return -1 };
        match child.downcast_ref::<Control>() {
            Some(container) => owner.index_from_container(&container.to_ref()),
            None => -1,
        }
    }

    fn try_get_total_count(&self) -> Option<i32> {
        let owner = self.owner.upgrade()?;
        Some(owner.items_view().count() as i32)
    }

    fn child_index_changed(&self, handler: Rc<dyn Fn(&ChildIndexChangedEventArgs)>) -> Rc<dyn IDisposable> {
        let Some(owner) = self.owner.upgrade() else { return Disposable::empty() };
        let token = owner.child_index_changed.add(handler);
        let handlers = Rc::downgrade(&owner.child_index_changed);
        Disposable::create(move || {
            if let Some(handlers) = handlers.upgrade() {
                handlers.remove(token);
            }
        })
    }
}
