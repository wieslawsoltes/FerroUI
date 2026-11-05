//! WinUI's `BreadcrumbBar` (`controls/dev/Breadcrumb/BreadcrumbBar.cpp`, `.h`, `.idl` and
//! `BreadcrumbBarItemClickedEventArgs.cpp` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use crate::automation::{AccessibilityView, AutomationProperties};
use crate::primitives::{BreadcrumbLayoutPanel, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::templates::IDataTemplate;
use crate::{BreadcrumbBarItem, Control, ControlImpl, ItemsSource, ItemsSourceView};
use ferroui_base::input::{FocusManager, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, NavigationMethod};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::FlowDirection;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::HandlerList;

use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_type, ferro_property, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use ferroui_base::Upcast;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The automation name of the ellipsis item (`AutomationNameEllipsisBreadcrumbBarItem` of the
/// English resources).
const ELLIPSIS_AUTOMATION_NAME: &str = "More";

/// The arguments of the `ItemClicked` event of a [`BreadcrumbBar`] (WinUI's
/// `BreadcrumbBarItemClickedEventArgs`): the index of the clicked item in the items of the bar,
/// and the item.
#[derive(Clone)]
pub struct BreadcrumbBarItemClickedEventArgs {
    /// The index of the item in the items source.
    pub index: i32,
    /// The item.
    pub item: Option<BoxedValue>,
}

impl BreadcrumbBarItemClickedEventArgs {
    /// The index of the item in the items source.
    pub fn index(&self) -> i32 {
        self.index
    }

    /// The item.
    pub fn item(&self) -> Option<BoxedValue> {
        self.item.clone()
    }
}

impl PartialEq for BreadcrumbBarItemClickedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && crate::items_source::items_equal(&self.item, &other.item)
    }
}

impl std::fmt::Debug for BreadcrumbBarItemClickedEventArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BreadcrumbBarItemClickedEventArgs").field("index", &self.index).finish_non_exhaustive()
    }
}

ferro_markup_type!(class BreadcrumbBarItemClickedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [BreadcrumbBarItemClickedEventArgs],
    properties: [
        Index: i32 { get: BreadcrumbBarItemClickedEventArgs::index },
        Item: Option<BoxedValue> { get: BreadcrumbBarItemClickedEventArgs::item },
    ],
});

/// A navigation trail (WinUI's `BreadcrumbBar`): an item for each level of its `ItemsSource`,
/// separated by chevrons, the last one the current level. When the items do not all fit, the
/// first ones are replaced by an ellipsis item whose flyout lists them. Clicking an item, or a
/// crumb of the flyout, raises `ItemClicked` with its index.
///
/// The template is the control theme of the WinUI theme (`Controls/BreadcrumbBar.xaml`, from
/// WinUI's `BreadcrumbBar.xaml`), with WinUI's part `PART_ItemsRepeater`, a
/// [`BreadcrumbLayoutPanel`]. The Fluent and Simple themes have no template for it
/// (docs/porting/winui-controls.md, section 5.1).
///
/// Differences (docs/porting/winui-controls.md, section 10.11): the items are realized in a
/// panel, not an `ItemsRepeater`, all of them, again when the items source changes; the focus that
/// comes from outside the bar goes to the first focusable item (WinUI moves it to the item that
/// had it last).
#[repr(C)]
pub struct BreadcrumbBar {
    base: TemplatedControl,
    items_panel: RefCell<Option<Ref<BreadcrumbLayoutPanel>>>,
    items_view: RefCell<Option<(Rc<ItemsSourceView>, u64)>>,
    ellipsis_item: RefCell<Option<Ref<BreadcrumbBarItem>>>,
    last_item: RefCell<Option<Ref<BreadcrumbBarItem>>>,
    focused_index: Cell<usize>,
    item_clicked: HandlerList<dyn Fn(&BreadcrumbBar, &BreadcrumbBarItemClickedEventArgs)>,
}

ferro_class!(BreadcrumbBar: TemplatedControl);
ferroui_base::ferro_class_info!(BreadcrumbBar {
    new: BreadcrumbBar::new,
    markup: {
        namespace: "FerroUI.Controls",
        events: [
            ItemClicked(Option<BoxedValue>, BreadcrumbBarItemClickedEventArgs) => |this: &Ref<BreadcrumbBar>, handler: MarkupDelegate| {
                this.item_clicked(move |sender: &BreadcrumbBar, args: &BreadcrumbBarItemClickedEventArgs| {
                    handler.invoke(&[into_markup_value(Some(sender.to_ref())), into_markup_value(args.clone())]);
                })
            },
        ],
        attributes: [
            TemplatePart("PART_ItemsRepeater", type(Ref<BreadcrumbLayoutPanel>)),
        ],
    },
});
ferro_impl_classes!(BreadcrumbBar: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl FerroObjectImpl for BreadcrumbBar {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::items_source_property().as_property() {
            this.update_items_repeater_items_source();
        } else if property == Self::item_template_property().as_property() {
            // UpdateItemTemplate and UpdateEllipsisBreadcrumbBarItemDropDownItemTemplate: the items
            // are realized again with the template.
            this.realize_items();
        } else if property == Visual::flow_direction_property().as_property() {
            this.update_breadcrumb_bar_items_flow_direction();
        }
    }
}

impl ControlImpl for BreadcrumbBar {}

impl TemplatedControlImpl for BreadcrumbBar {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        let previous = this.items_panel.borrow_mut().take();
        if let Some(previous) = previous {
            previous.set_arranged(None);
            previous.children().clear();
        }
        let panel = args.name_scope().find_as::<BreadcrumbLayoutPanel>("PART_ItemsRepeater");
        if let Some(panel) = &panel {
            let weak = this.to_ref().downgrade();
            panel.set_arranged(Some(Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.re_index_visible_elements_for_accessibility();
                }
            })));
        }
        *this.items_panel.borrow_mut() = panel;
        this.update_items_repeater_items_source();
    }
}

impl InputElementImpl for BreadcrumbBar {
    // OnChildPreviewKeyDown: the keys of the flow direction move the focus through the items.
    fn on_key_down(this: &Self, args: &KeyEventArgs) {
        Self::parent_on_key_down(this, args);
        if args.handled() {
            return;
        }
        let left_to_right = this.flow_direction() == FlowDirection::LeftToRight;
        let next = (left_to_right && args.key == Key::Right) || (!left_to_right && args.key == Key::Left);
        let previous = (left_to_right && args.key == Key::Left) || (!left_to_right && args.key == Key::Right);
        if (next && this.move_focus_next()) || (previous && this.move_focus_previous()) {
            args.set_handled(true);
        }
    }
}

ferroui_base::ferro_properties! { impl BreadcrumbBar {
    ferro_property!(
        /// Defines the `ItemsSource` property: the levels of the trail, the first the root.
        pub fn items_source_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<BreadcrumbBar, _>("ItemsSource", None)
        }
    );

    ferro_property!(
        /// Defines the `ItemTemplate` property: the template of the content of the items.
        pub fn item_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<BreadcrumbBar, _>("ItemTemplate", None)
        }
    );
} }

impl BreadcrumbBar {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            items_panel: RefCell::new(None),
            items_view: RefCell::new(None),
            ellipsis_item: RefCell::new(None),
            last_item: RefCell::new(None),
            focused_index: Cell::new(0),
            item_clicked: HandlerList::new(),
        }
    }

    /// Creates an empty bar.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the levels of the trail.
    pub fn items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::items_source_property())
    }

    /// Sets the levels of the trail.
    pub fn set_items_source(&self, value: Option<ItemsSource>) {
        self.set_value(Self::items_source_property(), value);
    }

    /// Gets the template of the content of the items.
    pub fn item_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::item_template_property())
    }

    /// Sets the template of the content of the items.
    pub fn set_item_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::item_template_property(), value);
    }

    /// Adds a handler of the `ItemClicked` event.
    ///
    /// Dropping the returned guard removes the handler; [`Subscription::detach`] keeps it
    /// for the life of the control.
    pub fn item_clicked(
        &self,
        handler: impl Fn(&BreadcrumbBar, &BreadcrumbBarItemClickedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.item_clicked.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.item_clicked.remove(token);
            }
        }))
    }

    /// The realized items: the ellipsis item first, then an item for each item of the source.
    pub fn realized_items(&self) -> Vec<Ref<BreadcrumbBarItem>> {
        let panel = self.items_panel.borrow().clone();
        panel
            .map(|panel| panel.children().snapshot().iter().filter_map(|child| child.cast::<BreadcrumbBarItem>()).collect())
            .unwrap_or_default()
    }

    pub(crate) fn raise_item_clicked_event(&self, content: Option<BoxedValue>, index: usize) {
        let args = BreadcrumbBarItemClickedEventArgs { index: index as i32, item: content };
        for (_, handler) in self.item_clicked.snapshot().iter() {
            handler(self, &args);
        }
    }

    fn update_items_repeater_items_source(&self) {
        let previous = self.items_view.borrow_mut().take();
        if let Some((view, token)) = previous {
            view.remove_collection_changed(token);
        }
        if let Some(source) = self.items_source() {
            let view = ItemsSourceView::new(source);
            let weak = self.to_ref().downgrade();
            let token = view.add_collection_changed(Rc::new(move |_| {
                if let Some(this) = weak.upgrade() {
                    // OnBreadcrumbBarItemsSourceCollectionChanged
                    this.realize_items();
                }
            }));
            *self.items_view.borrow_mut() = Some((view, token));
        }
        self.realize_items();
    }

    /// Realizes the items in the panel (WinUI's repeater with the `BreadcrumbIterable`, whose first
    /// element is the ellipsis, and `OnElementPreparedEvent`).
    fn realize_items(&self) {
        let Some(panel) = self.items_panel.borrow().clone() else { return };
        panel.children().clear();
        self.ellipsis_item.borrow_mut().take();
        self.last_item.borrow_mut().take();
        let template = self.item_template();
        let items: Vec<Option<BoxedValue>> =
            self.items_view.borrow().as_ref().map(|(view, _)| (0..view.count()).map(|index| view.get_at(index)).collect()).unwrap_or_default();
        let item_count = items.len();
        for index in 0..=item_count {
            let item = if index == 0 { self.ellipsis_element(&template) } else { self.element_for(&items[index - 1], &template) };
            item.set_is_ellipsis_drop_down_item(false);
            // "Set the parent breadcrumb reference for raising click events"
            item.set_parent_breadcrumb(self);
            item.set_index(index);
            item.set_flow_direction(self.flow_direction());
            if index == 0 {
                // "The first element is always the ellipsis item"
                item.set_properties_for_ellipsis_item();
                item.set_ellipsis_drop_down_item_data_template(template.clone());
                AutomationProperties::set_name(&item, Some(ELLIPSIS_AUTOMATION_NAME));
                *self.ellipsis_item.borrow_mut() = Some(item.clone());
            } else {
                if index == item_count {
                    item.set_properties_for_last_item();
                    *self.last_item.borrow_mut() = Some(item.clone());
                } else {
                    item.reset_visual_properties();
                }
            }
            panel.children().add(&item);
        }
    }

    /// The element of an item of the source (WinUI's `BreadcrumbElementFactory::GetElementCore`):
    /// the item itself when it is a breadcrumb item, the element of the template when that is a
    /// breadcrumb item (whose content is the item), else a new breadcrumb item whose content is
    /// the item and whose content template is the template.
    fn element_for(&self, data: &Option<BoxedValue>, template: &Option<Rc<dyn IDataTemplate>>) -> Ref<BreadcrumbBarItem> {
        if let Some(item) = data.as_ref().and_then(Control::from_boxed).and_then(|control| control.cast::<BreadcrumbBarItem>()) {
            return item;
        }
        if let Some(template) = template {
            if let Some(item) = template.build(data).and_then(|control| control.cast::<BreadcrumbBarItem>()) {
                // "When the list has not changed the returned item is still a BreadcrumbBarItem but
                // the item is not reset, so we set the content here"
                item.set_data_context(data.clone());
                item.set_content(data.clone());
                return item;
            }
        }
        let item = BreadcrumbBarItem::new();
        item.set_content(data.clone());
        // "If a user provided item template exists, we pass the template down to the
        // ContentPresenter of the BreadcrumbBarItem."
        if let Some(template) = template {
            item.set_content_template(Some(template.clone()));
        }
        item
    }

    /// The ellipsis item: a breadcrumb item without content, with the item template.
    fn ellipsis_element(&self, template: &Option<Rc<dyn IDataTemplate>>) -> Ref<BreadcrumbBarItem> {
        let item = BreadcrumbBarItem::new();
        if let Some(template) = template {
            item.set_content_template(Some(template.clone()));
        }
        item
    }

    fn update_breadcrumb_bar_items_flow_direction(&self) {
        let flow_direction = self.flow_direction();
        for item in self.realized_items() {
            item.set_flow_direction(flow_direction);
        }
    }

    /// The items of the source the ellipsis hides (WinUI's `HiddenElements`).
    pub(crate) fn hidden_elements(&self) -> Vec<Option<BoxedValue>> {
        let Some(panel) = self.items_panel.borrow().clone() else { return Vec::new() };
        if !panel.ellipsis_is_rendered() {
            return Vec::new();
        }
        // GetHiddenElementsList
        let first_shown = panel.first_rendered_item_index_after_ellipsis();
        let view = self.items_view.borrow();
        let Some((view, _)) = view.as_ref() else { return Vec::new() };
        (0..first_shown.saturating_sub(1)).map(|index| view.get_at(index)).collect()
    }

    /// Indexes the visible items for automation after each arrangement (WinUI's
    /// `ReIndexVisibleElementsForAccessibility`): their positions from 1, and the ellipsis item
    /// raw while it is hidden.
    fn re_index_visible_elements_for_accessibility(&self) {
        let Some(panel) = self.items_panel.borrow().clone() else { return };
        let visible_items_count = panel.visible_items_count();
        let ellipsis_is_rendered = panel.ellipsis_is_rendered();
        let first_item_to_index = if ellipsis_is_rendered { panel.first_rendered_item_index_after_ellipsis() } else { 1 };
        let ellipsis = self.ellipsis_item.borrow().clone();
        if let Some(ellipsis) = ellipsis {
            AutomationProperties::set_accessibility_view(
                &ellipsis,
                if ellipsis_is_rendered { AccessibilityView::Content } else { AccessibilityView::Raw },
            );
        }
        let items = self.realized_items();
        for (position, index) in (1..=visible_items_count).zip(first_item_to_index..) {
            if let Some(item) = items.get(index) {
                AutomationProperties::set_position_in_set(item, position as i32);
                AutomationProperties::set_size_of_set(item, visible_items_count as i32);
            }
        }
    }

    /// The index of the focused element among the realized items.
    fn focused_item_index(&self) -> Option<usize> {
        let focused = FocusManager::get_focus_manager(self).and_then(|manager| manager.get_focused_element())?;
        let focused: &Visual = (*focused).upcast();
        self.realized_items().iter().position(|item| {
            let item: &Visual = (**item).upcast();
            std::ptr::eq(item, focused) || item.is_visual_ancestor_of(focused)
        })
    }

    fn move_focus(&self, index_increment: isize) -> bool {
        let Some(focused_index) = self.focused_item_index() else { return false };
        if index_increment == 0 {
            return false;
        }
        let items = self.realized_items();
        let mut index = focused_index as isize + index_increment;
        while index >= 0 && (index as usize) < items.len() {
            let item = &items[index as usize];
            if item.is_effectively_visible() && item.bounds().width > 0.0 && item.focus_with(NavigationMethod::Directional, Default::default()) {
                self.focused_index.set(index as usize);
                return true;
            }
            index += index_increment;
        }
        false
    }

    fn move_focus_previous(&self) -> bool {
        let mut movement: isize = -1;
        if let Some(panel) = self.items_panel.borrow().clone() {
            let focused = self.focused_item_index().unwrap_or(self.focused_index.get());
            self.focused_index.set(focused);
            // "If the focus is in the first visible item, then move to the ellipsis"
            if focused == 1 {
                movement = 0;
            } else if panel.ellipsis_is_rendered() && focused == panel.first_rendered_item_index_after_ellipsis() {
                movement = -(focused as isize);
            }
        }
        self.move_focus(movement)
    }

    fn move_focus_next(&self) -> bool {
        let mut movement: isize = 1;
        let focused = self.focused_item_index().unwrap_or(self.focused_index.get());
        self.focused_index.set(focused);
        // "If the focus is in the ellipsis, then move to the first visible item"
        if focused == 0 {
            if let Some(panel) = self.items_panel.borrow().clone() {
                movement = panel.first_rendered_item_index_after_ellipsis() as isize;
            }
        }
        self.move_focus(movement)
    }
}
