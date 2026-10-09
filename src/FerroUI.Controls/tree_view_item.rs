use crate::generators::RecycleKey;
use crate::i_selectable::{register_selectable, ISelectable};
use crate::items_source::ItemsChangedEventArgs;
use crate::metadata::{PseudoClassesAttribute, TemplatePartAttribute};
use crate::mixins::{PressedMixin, SelectableMixin};
use crate::primitives::{
    HeaderedItemsControl, HeaderedItemsControlImpl, SelectingItemsControl, TemplateAppliedEventArgs,
    TemplatedControlImpl,
};
use crate::templates::{FuncTemplate, ITemplateOf};
use crate::{
    Control, ControlImpl, ItemsControl, ItemsControlImpl, ItemsControlImplExt, Panel, RequestBringIntoViewEventArgs,
    StackPanel, TreeView,
};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::BindingMode;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers, NavigationMethod,
    PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, TappedEventArgs,
};
use ferroui_base::interactivity::{
    InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, BoxedValue, DirectProperty,
    FerroObjectImpl, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, Size, StaticType,
    StyledElement, StyledElementImpl, StyledElementImplExt, StyledProperty, StyledPropertyOptions, VisualImpl,
    WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An item in a [`TreeView`].
#[repr(C)]
pub struct TreeViewItem {
    base: HeaderedItemsControl,
    tree_view: RefCell<Option<WeakRef<TreeView>>>,
    header: RefCell<Option<Ref<Control>>>,
    header_presenter: RefCell<Option<Ref<Control>>>,
    header_double_tapped_token: Cell<Option<RoutedEventHandlerToken>>,
    level: Cell<i32>,
    template_applied: Cell<bool>,
    deferred_bring_into_view_flag: Cell<bool>,
    items_panel_pointer_pressed: Cell<bool>,
}

ferro_class! {
    TreeViewItem: HeaderedItemsControl, virtuals TreeViewItemImpl: HeaderedItemsControlImpl {
        /// Called when a request to bring a control into view reaches the
        /// item.
        fn on_request_bring_into_view(this, e: &RequestBringIntoViewEventArgs);
        /// Invoked when the double tapped event occurs in the header.
        fn on_header_double_tapped(this, e: &TappedEventArgs);
    }
}
ferroui_base::ferro_class_info!(TreeViewItem { new: TreeViewItem::new });

ferro_impl_classes!(
    TreeViewItem: VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    HeaderedItemsControlImpl
);

impl ControlImpl for TreeViewItem {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::TreeViewItemAutomationPeer::new(this).upcast()
    }
}

ferroui_base::ferro_impl_classes!(TreeViewItem: FerroObjectImpl);

impl StyledElementImpl for TreeViewItem {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_logical_tree(this, e);

        let mut tree_view = None;
        let mut ancestor = this.parent();
        while let Some(current) = ancestor {
            if let Some(found) = current.cast::<TreeView>() {
                tree_view = Some(found);
                break;
            }
            ancestor = current.parent();
        }

        *this.tree_view.borrow_mut() = tree_view.as_ref().map(Ref::downgrade);

        this.set_level(Self::calculate_distance_from_logical_parent::<TreeView>(Some(this.to_ref().upcast()), -1) - 1);

        let Some(tree_view) = tree_view else { return };

        if this.item_template().is_none() {
            if let Some(item_template) = tree_view.item_template() {
                this.set_current_value(ItemsControl::item_template_property(), Some(item_template));
            }
        }

        if this.item_container_theme().is_none() {
            if let Some(item_container_theme) = tree_view.item_container_theme() {
                this.set_current_value(ItemsControl::item_container_theme_property(), Some(item_container_theme));
            }
        }
    }
}

impl InputElementImpl for TreeViewItem {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        if !e.handled() {
            let handler = match e.key {
                Key::Left => {
                    Some(apply_to_item_or_recursively_if_ctrl(focus_aware_collapse_item, e.key_modifiers))
                }
                Key::Right => Some(apply_to_item_or_recursively_if_ctrl(expand_item, e.key_modifiers)),
                Key::Enter => Some(apply_to_item_or_recursively_if_ctrl(
                    if this.is_expanded() { collapse_item } else { expand_item },
                    e.key_modifiers,
                )),

                // Do not handle the control modifier with the numpad keys.
                Key::Subtract => Some(KeyHandler::Item(focus_aware_collapse_item)),
                Key::Add => Some(KeyHandler::Item(expand_item)),
                Key::Divide => Some(KeyHandler::Subtree(collapse_item)),
                Key::Multiply => Some(KeyHandler::Subtree(expand_item)),
                _ => None,
            };

            match handler {
                Some(handler) => e.set_handled(handler.invoke(this)),
                None => {
                    if let Some(owner) = this.tree_view_owner() {
                        owner.update_selection_from_event(&this.to_ref().upcast(), e);
                    }
                }
            }
        }

        // Don't call the base implementation: let events bubble up to the
        // containing tree view.
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);

        this.items_panel_pointer_pressed.set(this.is_items_panel_event(e));

        // Don't select if the pointer was pressed over the items panel.
        if !this.items_panel_pointer_pressed.get() {
            if let Some(owner) = this.tree_view_owner() {
                owner.update_selection_from_event(&this.to_ref().upcast(), e);
            }
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);

        // Don't select if the gesture either started on the items panel, or
        // ended on it.
        if !this.items_panel_pointer_pressed.get() && !this.is_items_panel_event(e) {
            if let Some(owner) = this.tree_view_owner() {
                owner.update_selection_from_event(&this.to_ref().upcast(), e);
            }
        }

        this.items_panel_pointer_pressed.set(false);
    }
}

impl TemplatedControlImpl for TreeViewItem {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let previous = this.header_presenter.take();
        if let (Some(previous), Some(token)) = (previous, this.header_double_tapped_token.take()) {
            previous.remove_handler(InputElement::double_tapped_event(), token);
        }

        *this.header.borrow_mut() = e.name_scope().find_as::<Control>("PART_Header");
        let header_presenter = e.name_scope().find_as::<Control>("PART_HeaderPresenter");
        *this.header_presenter.borrow_mut() = header_presenter.clone();
        this.template_applied.set(true);

        if let Some(header_presenter) = header_presenter {
            let weak = this.to_ref().downgrade();
            let token = header_presenter.add_handler(InputElement::double_tapped_event(), move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.header_double_tapped(e);
                }
            });
            this.header_double_tapped_token.set(Some(token));
        }

        if this.deferred_bring_into_view_flag.get() {
            this.deferred_bring_into_view_flag.set(false);
            this.bring_into_view();
        }
    }
}

impl ItemsControlImpl for TreeViewItem {
    fn create_container_for_item_override(
        this: &Self,
        item: &Option<BoxedValue>,
        index: i32,
        recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        this.ensure_tree_view().create_container_for_item_override(item, index, recycle_key)
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, index: i32) -> (bool, Option<RecycleKey>) {
        this.ensure_tree_view().needs_container_override(item, index)
    }

    fn prepare_container_for_item_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32,
    ) {
        this.ensure_tree_view().prepare_container_for_item_override(container, item, index)
    }

    fn container_for_item_prepared_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32,
    ) {
        this.ensure_tree_view().container_for_item_prepared_override(container, item, index)
    }

    fn on_items_view_collection_changed(this: &Self, e: &ItemsChangedEventArgs<'_>) {
        Self::parent_on_items_view_collection_changed(this, e);

        let Some(tree_view) = this.tree_view_owner() else { return };

        match e.action {
            NotifyCollectionChangedAction::Remove | NotifyCollectionChangedAction::Replace => {
                for i in e.old_items {
                    tree_view.selected_items().remove(&i);
                }
            }
            NotifyCollectionChangedAction::Reset => {
                for i in this.get_realized_containers() {
                    if i.downcast_ref::<TreeViewItem>().is_some_and(|tvi| tvi.is_selected()) {
                        tree_view.selected_items().remove(&i.data_context());
                    }
                }
            }
            _ => {}
        }
    }
}

impl TreeViewItemImpl for TreeViewItem {
    fn on_request_bring_into_view(this: &Self, e: &RequestBringIntoViewEventArgs) {
        let this_visual: Ref<ferroui_base::Visual> = this.to_ref().upcast();
        let is_target = e.target_object.as_ref() == Some(&this_visual);
        if !is_target {
            return;
        }

        if !this.template_applied.get() {
            this.deferred_bring_into_view_flag.set(true);
            return;
        }

        let header = this.header.borrow().clone();
        if let Some(header) = header {
            if let Some(m) = header.transform_to_visual(this) {
                let bounds = Rect::from_size(Self::get_header_target_size(&header));
                let rect = bounds.transform_to_aabb(m);
                e.set_target_rect(rect);
            }
        }
    }

    fn on_header_double_tapped(this: &Self, e: &TappedEventArgs) {
        if this.item_count() > 0 {
            this.set_current_value(Self::is_expanded_property(), !this.is_expanded());
            e.set_handled(true);
        }
    }
}

impl ISelectable for TreeViewItem {
    fn is_selected(&self) -> bool {
        TreeViewItem::is_selected(self)
    }

    fn set_is_selected(&self, value: bool) {
        TreeViewItem::set_is_selected(self, value)
    }
}

/// A key handler of an item: applied to the item alone or to every item of
/// its subtree.
enum KeyHandler {
    Item(fn(&TreeViewItem) -> bool),
    Subtree(fn(&TreeViewItem) -> bool),
}

impl KeyHandler {
    // NOTE: the subtree handlers do not use the expand/collapse subtree
    // functions of the tree view because we want to know if any items were
    // in fact expanded to set the event handled status. Also the handling
    // here avoids a potential infinite recursion/stack overflow.
    fn invoke(&self, tree_view_item: &TreeViewItem) -> bool {
        match self {
            KeyHandler::Item(f) => f(tree_view_item),
            KeyHandler::Subtree(f) => {
                // All the items are enumerated before applying the function.
                // This avoids a potential infinite loop if there is an
                // infinite tree (a lazily infinite tree). But it also means
                // a lazily loaded tree will not be expanded completely.
                let mut items = Vec::new();
                sub_tree(&tree_view_item.to_ref(), &mut items);

                let mut result = false;
                for item in &items {
                    let applied = f(item);
                    result = result || applied;
                }
                result
            }
        }
    }
}

fn apply_to_item_or_recursively_if_ctrl(f: fn(&TreeViewItem) -> bool, key_modifiers: KeyModifiers) -> KeyHandler {
    if key_modifiers.contains(KeyModifiers::CONTROL) {
        return KeyHandler::Subtree(f);
    }

    KeyHandler::Item(f)
}

fn expand_item(tree_view_item: &TreeViewItem) -> bool {
    if tree_view_item.item_count() > 0 && !tree_view_item.is_expanded() {
        tree_view_item.set_current_value(TreeViewItem::is_expanded_property(), true);
        return true;
    }

    false
}

fn collapse_item(tree_view_item: &TreeViewItem) -> bool {
    if tree_view_item.item_count() > 0 && tree_view_item.is_expanded() {
        tree_view_item.set_current_value(TreeViewItem::is_expanded_property(), false);
        return true;
    }

    false
}

fn focus_aware_collapse_item(tree_view_item: &TreeViewItem) -> bool {
    if tree_view_item.item_count() > 0 && tree_view_item.is_expanded() {
        if tree_view_item.is_focused() {
            tree_view_item.set_current_value(TreeViewItem::is_expanded_property(), false);
        } else {
            tree_view_item.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
        }

        return true;
    }

    false
}

fn sub_tree(tree_view_item: &Ref<TreeViewItem>, result: &mut Vec<Ref<TreeViewItem>>) {
    result.push(tree_view_item.clone());

    for child in StyledElement::logical_children(tree_view_item).snapshot().iter() {
        if let Some(child) = child.clone().cast::<TreeViewItem>() {
            sub_tree(&child, result);
        }
    }
}

impl TreeViewItem {
    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] =
        &[TemplatePartAttribute::new("PART_Header", <Control as StaticType>::TYPE)];

    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[":pressed", ":selected"]);
}

ferroui_base::ferro_properties! { impl TreeViewItem {
    ferro_property!(
        /// Defines the `IsExpanded` property.
        pub fn is_expanded_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<TreeViewItem, _>(
                "IsExpanded",
                StyledPropertyOptions::new(false).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );

    ferro_property!(
        /// Defines the `IsSelected` property.
        pub fn is_selected_property() -> StyledProperty<bool> {
            SelectingItemsControl::is_selected_property().add_owner::<TreeViewItem>()
        }
    );

    ferro_property!(
        /// Defines the `Level` property.
        pub fn level_property() -> DirectProperty<TreeViewItem, i32> {
            FerroProperty::register_direct::<TreeViewItem, _>("Level", |o| o.level(), None, 0)
        }
    );
} }

impl TreeViewItem {
    ferro_routed_event!(
        /// Defines the `Expanded` event.
        pub fn expanded_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<TreeViewItem, _>("Expanded", RoutingStrategies::BUBBLE | RoutingStrategies::TUNNEL)
        }
    );

    ferro_routed_event!(
        /// Defines the `Collapsed` event.
        pub fn collapsed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<TreeViewItem, _>(
                "Collapsed",
                RoutingStrategies::BUBBLE | RoutingStrategies::TUNNEL,
            )
        }
    );

    pub(crate) fn static_constructor() {
        register_selectable::<TreeViewItem>();
        SelectableMixin::attach::<TreeViewItem>(Self::is_selected_property());
        PressedMixin::attach::<TreeViewItem>();
        InputElement::focusable_property().override_default_value::<TreeViewItem>(true);
        let default_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
            FuncTemplate::new(|| Some(StackPanel::new().upcast::<Panel>()));
        ItemsControl::items_panel_property().override_default_value::<TreeViewItem>(default_panel);
        crate::platform::PlatformFeedback::feedback_type_property()
            .override_default_value::<TreeViewItem>(crate::platform::FeedbackType::Auto);
        crate::automation::AutomationProperties::is_offscreen_behavior_property()
            .override_default_value::<TreeViewItem>(crate::automation::IsOffscreenBehavior::FromClip);
        Control::request_bring_into_view_event()
            .add_class_handler::<TreeViewItem>(|x, e| x.on_request_bring_into_view(e));
        Self::is_expanded_property().changed().add_class_handler::<TreeViewItem>(|x, e| x.on_is_expanded_changed(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: HeaderedItemsControl::construct(),
            tree_view: RefCell::new(None),
            header: RefCell::new(None),
            header_presenter: RefCell::new(None),
            header_double_tapped_token: Cell::new(None),
            level: Cell::new(0),
            template_applied: Cell::new(false),
            deferred_bring_into_view_flag: Cell::new(false),
            items_panel_pointer_pressed: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn on_is_expanded_changed(&self, args: &FerroPropertyChangedEventArgs<'_>) {
        let routed_event = if args.get_new_value::<bool>() { Self::expanded_event() } else { Self::collapsed_event() };
        let event_args = RoutedEventArgs::with_event_and_source(routed_event, self.to_ref());
        self.raise_event(&event_args);
    }

    /// Gets or sets a value indicating whether the item is expanded to show
    /// its children.
    pub fn is_expanded(&self) -> bool {
        self.get_value(Self::is_expanded_property())
    }

    pub fn set_is_expanded(&self, value: bool) {
        self.set_value(Self::is_expanded_property(), value)
    }

    /// Gets or sets the selection state of the item.
    pub fn is_selected(&self) -> bool {
        self.get_value(Self::is_selected_property())
    }

    pub fn set_is_selected(&self, value: bool) {
        self.set_value(Self::is_selected_property(), value)
    }

    /// Gets the level/indentation of the item.
    pub fn level(&self) -> i32 {
        self.level.get()
    }

    fn set_level(&self, value: i32) {
        self.set_and_raise_cell(Self::level_property(), &self.level, value);
    }

    /// Occurs after the [`TreeViewItem`] has expanded to show its children.
    pub fn expanded(
        &self,
        handler: impl Fn(&ferroui_base::interactivity::Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::expanded_event(), handler)
    }

    /// Occurs after the [`TreeViewItem`] has collapsed to hide its children.
    pub fn collapsed(
        &self,
        handler: impl Fn(&ferroui_base::interactivity::Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::collapsed_event(), handler)
    }

    pub(crate) fn tree_view_owner(&self) -> Option<Ref<TreeView>> {
        self.tree_view.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn get_header_target_size(header: &Control) -> Size {
        // Use the desired width, not the width of the bounds: the latter is
        // stretched to the full width of the tree view's extent, preventing
        // scrolling, whereas the desired size is what the header actually
        // needs.
        let margin = header.margin();
        let desired_width = header.desired_size().width - margin.left - margin.right;
        let bounds_size = header.bounds().size();

        bounds_size.with_width(desired_width.clamp(0.0, bounds_size.width))
    }

    /// Due to the implicit pointer capture, the items panel receives no
    /// events if swiped onto. So we need to perform our own hit test to
    /// reliably determine whether the mouse event occurred over it.
    fn is_items_panel_event(&self, e: &PointerEventArgs) -> bool {
        match self.items_panel_root() {
            Some(items_panel_root) => {
                items_panel_root.input_hit_test(e.get_position(Some(&items_panel_root))).is_some()
            }
            None => false,
        }
    }

    fn calculate_distance_from_logical_parent<T: ferroui_base::ObjectType>(
        logical: Option<Ref<StyledElement>>,
        default: i32,
    ) -> i32 {
        let mut result = 0;
        let mut logical = logical;

        while let Some(current) = logical.take() {
            if current.is::<T>() {
                return result;
            }

            result += 1;
            logical = current.parent();
        }

        default
    }

    fn ensure_tree_view(&self) -> Ref<TreeView> {
        match self.tree_view_owner() {
            Some(tree_view) => tree_view,
            None => panic!("The TreeViewItem is not part of a TreeView."),
        }
    }

    fn header_double_tapped(&self, e: &TappedEventArgs) {
        self.on_header_double_tapped(e);
    }
}
