//! WinUI's `BreadcrumbBar` (`controls/dev/Breadcrumb/BreadcrumbBar.cpp`, `.h`, `.idl`,
//! `BreadcrumbBarElementFactory.cpp` and `BreadcrumbIterable.cpp` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::{BreadcrumbBarItem, BreadcrumbBarItemClickedEventArgs};
use crate::automation::{AccessibilityView, AutomationProperties};
use crate::items_source::{items_equal, ItemsChangedEventArgs, ItemsSource};
use crate::items_source_view::ItemsSourceView;
use crate::primitives::{
    BreadcrumbBarPanel, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::templates::IDataTemplate;
use crate::{Control, ControlImpl};
use ferroui_base::input::{
    FocusChangingEventArgs, InputElement, InputElementImpl, Key, KeyEventArgs, KeyModifiers, NavigationMethod,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::FlowDirection;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, Upcast, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The automation name of the ellipsis item (`SR_AutomationNameEllipsisBreadcrumbBarItem`, the
/// English text of WinUI's resources).
const AUTOMATION_NAME_ELLIPSIS_BREADCRUMB_BAR_ITEM: &str = "More";

/// The template part of the items (`s_itemsRepeaterPartName`).
const ITEMS_REPEATER_PART_NAME: &str = "PART_ItemsRepeater";

/// The element of an item (WinUI's `BreadcrumbElementFactory::GetElementCore`): an item that is a
/// breadcrumb bar item is its own element; otherwise the element is the root of the item template
/// built for the item when that root is a breadcrumb bar item, or a new breadcrumb bar item whose
/// content is the item, shown with the item template. The element's data context is the item.
pub(crate) fn create_breadcrumb_element(
    item: Option<BoxedValue>,
    template: Option<&Rc<dyn IDataTemplate>>,
) -> Ref<BreadcrumbBarItem> {
    if let Some(own) =
        item.as_ref().and_then(Control::from_boxed).and_then(|control| control.cast::<BreadcrumbBarItem>())
    {
        // An item shown in the bar is shown in the flyout by a new item with its content: one
        // element has one parent.
        if own.parent().is_none() {
            return own;
        }
        let element = BreadcrumbBarItem::new();
        element.set_content(own.content());
        element.set_content_template(own.content_template());
        return element;
    }
    if let Some(template) = template {
        if let Some(built) = template.build(&item).and_then(|control| control.cast::<BreadcrumbBarItem>()) {
            // "When the list has not changed the returned item is still a BreadcrumbBarItem but the
            // item is not reset, so we set the content here"
            built.set_data_context(item.clone());
            built.set_content(item);
            return built;
        }
    }
    let element = BreadcrumbBarItem::new();
    element.set_data_context(item.clone());
    element.set_content(item);
    // "If a user provided item template exists, we pass the template down to the ContentPresenter
    // of the BreadcrumbBarItem."
    if let Some(template) = template {
        element.set_content_template(Some(template.clone()));
    }
    element
}

/// A navigation aid that shows the path to the current location as a row of crumbs (WinUI's
/// `BreadcrumbBar`). `ItemsSource` is the path, from the root to the current location;
/// `ItemTemplate` shows each item. Each item is shown by a [`BreadcrumbBarItem`] followed by a
/// chevron, and the last one, the current location, without a button. When the items do not fit,
/// the first ones are hidden behind an ellipsis item, which lists them in a flyout. Clicking an
/// item, in the bar or in the flyout, raises `ItemClicked` with the item and its index in the items
/// source.
///
/// Keyboard: Left and Right move the focus between the shown items (mirrored right to left), from
/// the ellipsis item to the first shown item and back; Enter and Space click the focused item. The
/// focus enters the bar at the ellipsis item when it is shown, and at the first item otherwise.
/// Automation: the bar is a navigation landmark (its control theme); each shown item has its
/// position and the number of shown items, and the ellipsis item is in the raw view while it is
/// hidden.
///
/// The template is the control theme of the WinUI theme (`Controls/BreadcrumbBar.xaml`, from
/// WinUI's `BreadcrumbBar.xaml`) with the part `PART_ItemsRepeater`, a
/// [`BreadcrumbBarPanel`] whose children the bar realizes: the ellipsis item first, then an item
/// for each item of the source. The Fluent and Simple themes have no template for it
/// (docs/porting/winui-controls.md, section 5.1).
///
/// Differences (docs/porting/winui-controls.md, section 10.11): the items panel is a panel the bar
/// fills, where WinUI has an `ItemsRepeater` with a `BreadcrumbLayout` (`ferroui-controls` cannot
/// depend on the repeater crate); `ItemTemplate` is a data template (WinUI also takes an element
/// factory); access keys are left out.
#[repr(C)]
pub struct BreadcrumbBar {
    base: TemplatedControl,
    items_repeater: RefCell<Option<Ref<BreadcrumbBarPanel>>>,
    // "This collection is only composed of the consumer defined objects, it doesn't include the
    // extra ellipsis/nullptr element."
    breadcrumb_items_source_view: RefCell<Option<Rc<ItemsSourceView>>>,
    items_source_changed: Cell<Option<u64>>,
    // The elements of the panel: the ellipsis item (index 0) and an item for each item of the
    // source, with the item each one shows.
    elements: RefCell<Vec<(Option<BoxedValue>, Ref<BreadcrumbBarItem>)>>,
    ellipsis_breadcrumb_bar_item: RefCell<Option<Ref<BreadcrumbBarItem>>>,
    last_breadcrumb_bar_item: RefCell<Option<Ref<BreadcrumbBarItem>>>,
    // "Index of the last focused item when breadcrumb lost focus"
    focused_index: Cell<i32>,
    item_clicked: HandlerList<dyn Fn(&BreadcrumbBar, &BreadcrumbBarItemClickedEventArgs)>,
}

ferro_class!(BreadcrumbBar: TemplatedControl);
ferroui_base::ferro_class_info!(BreadcrumbBar {
    new: BreadcrumbBar::new,
    markup: {
        namespace: "FerroUI.Controls",
        events: [
            ItemClicked(Option<BoxedValue>, BreadcrumbBarItemClickedEventArgs) =>
                |this: &Ref<BreadcrumbBar>, handler: MarkupDelegate| {
                this.item_clicked(move |sender: &BreadcrumbBar, args: &BreadcrumbBarItemClickedEventArgs| {
                    handler.invoke(&[into_markup_value(Some(sender.to_ref())), into_markup_value(args.clone())]);
                })
            },
        ],
        attributes: [
            TemplatePart("PART_ItemsRepeater", type(Ref<BreadcrumbBarPanel>)),
        ],
    },
});
ferro_impl_classes!(BreadcrumbBar: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl FerroObjectImpl for BreadcrumbBar {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        this.add_handler_with(
            InputElement::key_down_event(),
            move |_, args| {
                if let Some(this) = weak.upgrade() {
                    this.on_child_preview_key_down(args);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak = this.to_ref().downgrade();
        this.add_handler(InputElement::getting_focus_event(), move |_, args| {
            if let Some(this) = weak.upgrade() {
                this.on_getting_focus(args);
            }
        });
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::items_source_property().as_property() {
            this.update_items_repeater_items_source();
        } else if property == Self::item_template_property().as_property() {
            // UpdateItemTemplate: the elements are made again with the new template.
            this.update_elements(true);
            this.update_ellipsis_breadcrumb_bar_item_drop_down_item_template();
        }
    }
}

impl ControlImpl for BreadcrumbBar {}

impl InputElementImpl for BreadcrumbBar {}

impl TemplatedControlImpl for BreadcrumbBar {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        if let Some(old) = this.items_repeater.take() {
            old.children().clear();
        }
        this.elements.borrow_mut().clear();
        *this.ellipsis_breadcrumb_bar_item.borrow_mut() = None;
        *this.last_breadcrumb_bar_item.borrow_mut() = None;
        *this.items_repeater.borrow_mut() = args.name_scope().find_as::<BreadcrumbBarPanel>(ITEMS_REPEATER_PART_NAME);
        this.update_items_repeater_items_source();
    }
}

ferroui_base::ferro_properties! { impl BreadcrumbBar {
    ferro_property!(
        /// Defines the `ItemsSource` property: the items of the path, from the root to the current
        /// location.
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
            items_repeater: RefCell::new(None),
            breadcrumb_items_source_view: RefCell::new(None),
            items_source_changed: Cell::new(None),
            elements: RefCell::new(Vec::new()),
            ellipsis_breadcrumb_bar_item: RefCell::new(None),
            last_breadcrumb_bar_item: RefCell::new(None),
            focused_index: Cell::new(1),
            item_clicked: HandlerList::new(),
        }
    }

    /// Creates a breadcrumb bar without items.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the items of the path.
    pub fn items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::items_source_property())
    }

    /// Sets the items of the path, from the root to the current location.
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

    /// Adds a handler of the `ItemClicked` event, raised when an item of the bar or of the flyout
    /// of the ellipsis item is clicked. Dropping the returned guard removes the handler;
    /// [`Subscription::detach`] keeps it for the life of the bar.
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

    /// The panel of the items, once the template is applied.
    pub(crate) fn items_repeater(&self) -> Option<Ref<BreadcrumbBarPanel>> {
        self.items_repeater.borrow().clone()
    }

    /// The element at `index` of the panel (`TryGetElement`): 0 is the ellipsis item.
    pub(crate) fn try_get_element(&self, index: i32) -> Option<Ref<BreadcrumbBarItem>> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.elements.borrow().get(index).map(|(_, element)| element.clone()))
    }

    /// The index of `element` in the panel (`GetElementIndex`), -1 for an element that is not one.
    fn get_element_index(&self, element: &InputElement) -> i32 {
        self.elements
            .borrow()
            .iter()
            .position(|(_, candidate)| std::ptr::eq::<InputElement>((**candidate).upcast(), element))
            .map_or(-1, |index| index as i32)
    }

    fn element_count(&self) -> i32 {
        self.elements.borrow().len() as i32
    }

    fn update_ellipsis_breadcrumb_bar_item_drop_down_item_template(&self) {
        // "Copy the item template to the ellipsis item too"
        if let Some(ellipsis) = self.ellipsis_breadcrumb_bar_item.borrow().as_ref() {
            ellipsis.set_ellipsis_drop_down_item_data_template(self.item_template());
        }
    }

    fn update_items_repeater_items_source(&self) {
        if let (Some(view), Some(token)) = (self.breadcrumb_items_source_view.take(), self.items_source_changed.take())
        {
            view.remove_collection_changed(token);
        }
        if let Some(source) = self.items_source() {
            let view = ItemsSourceView::new(source);
            let weak = self.to_ref().downgrade();
            let token = view.add_collection_changed(Rc::new(move |_: &ItemsChangedEventArgs<'_>| {
                if let Some(this) = weak.upgrade() {
                    this.on_breadcrumb_bar_items_source_collection_changed();
                }
            }));
            *self.breadcrumb_items_source_view.borrow_mut() = Some(view);
            self.items_source_changed.set(Some(token));
        }
        self.update_elements(false);
        self.force_update_last_element();
    }

    fn on_breadcrumb_bar_items_source_collection_changed(&self) {
        self.update_elements(false);
        // "For some reason, when interacting with keyboard, the last element doesn't raise the
        // OnPrepared event"
        self.force_update_last_element();
    }

    /// The items of the panel (`BreadcrumbIterable`): the ellipsis item, then the items of the
    /// source. An element whose item is unchanged at its position is kept (as the repeater keeps
    /// the realized elements), so the focus stays on it; the elements after the first change are
    /// made again. `rebuild` makes all of them again (a new item template).
    fn update_elements(&self, rebuild: bool) {
        let Some(panel) = self.items_repeater() else { return };
        // Without a source the panel has no element, not even the ellipsis item (the empty items
        // source of the repeater).
        let items: Option<Vec<Option<BoxedValue>>> =
            self.breadcrumb_items_source_view.borrow().as_ref().map(|view| view.to_vec());
        let has_source = items.is_some();
        let items = items.unwrap_or_default();
        let template = self.item_template();
        let mut elements = self.elements.borrow_mut();
        // The common prefix: the ellipsis item and the items that did not change.
        let mut keep = if rebuild || !has_source { 0 } else { elements.len().min(1) };
        if keep > 0 {
            while keep < elements.len() && keep <= items.len() && items_equal(&elements[keep].0, &items[keep - 1]) {
                keep += 1;
            }
        }
        let children = panel.children();
        for (_, element) in elements.drain(keep..).rev() {
            // OnElementClearingEvent
            element.reset_visual_properties();
            children.remove(element);
        }
        if !has_source {
            *self.ellipsis_breadcrumb_bar_item.borrow_mut() = None;
            return;
        }
        let mut prepared = Vec::new();
        if elements.is_empty() {
            let ellipsis = create_breadcrumb_element(None, template.as_ref());
            children.add(ellipsis.clone());
            elements.push((None, ellipsis.clone()));
            prepared.push((0, ellipsis));
        }
        for (index, item) in items.iter().enumerate().skip(elements.len() - 1) {
            let element = create_breadcrumb_element(item.clone(), template.as_ref());
            children.add(element.clone());
            elements.push((item.clone(), element.clone()));
            prepared.push((index + 1, element));
        }
        drop(elements);
        for (index, element) in prepared {
            self.on_element_prepared(&element, index as u32);
        }
    }

    /// OnElementPreparedEvent: the ellipsis item, the last item and the other items get their
    /// properties.
    fn on_element_prepared(&self, item: &BreadcrumbBarItem, item_index: u32) {
        item.set_is_ellipsis_drop_down_item(false);
        // "Set the parent breadcrumb reference for raising click events"
        item.set_parent_breadcrumb(self);
        // "Set the item index to fill the Index parameter in the ClickedEventArgs"
        item.set_index(item_index);
        // "The first element is always the ellipsis item"
        if item_index == 0 {
            item.set_properties_for_ellipsis_item();
            *self.ellipsis_breadcrumb_bar_item.borrow_mut() = Some(item.to_ref());
            self.update_ellipsis_breadcrumb_bar_item_drop_down_item_template();
            AutomationProperties::set_name(item, Some(AUTOMATION_NAME_ELLIPSIS_BREADCRUMB_BAR_ITEM));
        } else if let Some(view) = self.breadcrumb_items_source_view.borrow().as_ref() {
            if item_index as usize == view.count() {
                self.update_last_element(item);
            } else {
                // "Any other element just resets the visual properties"
                item.reset_visual_properties();
            }
        }
    }

    fn reset_last_breadcrumb_bar_item(&self) {
        if let Some(last) = self.last_breadcrumb_bar_item.borrow().as_ref() {
            last.reset_visual_properties();
        }
    }

    fn force_update_last_element(&self) {
        let count = self.breadcrumb_items_source_view.borrow().as_ref().map(|view| view.count());
        match count {
            Some(item_count) => {
                if let Some(new_last) = self.try_get_element(item_count as i32).filter(|_| item_count > 0) {
                    self.update_last_element(&new_last);
                }
                // "If the given collection is empty, then reset the last element visual properties"
                if item_count == 0 {
                    self.reset_last_breadcrumb_bar_item();
                }
            }
            // "Or if the ItemsSource was null, also reset the last breadcrumb Item"
            None => self.reset_last_breadcrumb_bar_item(),
        }
    }

    fn update_last_element(&self, new_last: &BreadcrumbBarItem) {
        // "If the element is the last element in the array, then we reset the visual properties
        // for the previous last element"
        self.reset_last_breadcrumb_bar_item();
        new_last.set_properties_for_last_item();
        *self.last_breadcrumb_bar_item.borrow_mut() = Some(new_last.to_ref());
    }

    pub(crate) fn raise_item_clicked_event(&self, content: Option<BoxedValue>, index: u32) {
        let args = BreadcrumbBarItemClickedEventArgs::new(index as i32, content);
        for (_, handler) in self.item_clicked.snapshot().iter() {
            handler(self, &args);
        }
    }

    /// The items hidden behind the ellipsis, from the first (`GetHiddenElementsList`).
    fn get_hidden_elements_list(&self, first_shown_element: usize) -> Vec<Option<BoxedValue>> {
        let view = self.breadcrumb_items_source_view.borrow();
        let Some(view) = view.as_ref() else { return Vec::new() };
        (0..first_shown_element.saturating_sub(1)).map(|i| view.get_at(i)).collect()
    }

    /// The items the ellipsis item lists: empty while it is not shown (`HiddenElements`; "The
    /// hidden element list is generated in the BreadcrumbLayout during the arrange method").
    pub(crate) fn hidden_elements(&self) -> Vec<Option<BoxedValue>> {
        match self.items_repeater() {
            Some(panel) if panel.ellipsis_is_rendered() => {
                self.get_hidden_elements_list(panel.first_rendered_item_index_after_ellipsis())
            }
            _ => Vec::new(),
        }
    }

    /// After an arrange of the panel: the shown items get their position, from 1, and the number of
    /// shown items, and the ellipsis item is in the raw view while it is hidden
    /// (`ReIndexVisibleElementsForAccessibility`).
    pub(crate) fn re_index_visible_elements_for_accessibility(&self) {
        let Some(panel) = self.items_repeater() else { return };
        let visible_items_count = panel.visible_items_count();
        let is_ellipsis_rendered = panel.ellipsis_is_rendered();
        let first_item_to_index =
            if is_ellipsis_rendered { panel.first_rendered_item_index_after_ellipsis() } else { 1 };
        // "In order to make the ellipsis inaccessible to accessbility tools when it's hidden, we set
        // the accessibilityView to raw and restore it to content when it becomes visible."
        if let Some(ellipsis) = self.ellipsis_breadcrumb_bar_item.borrow().as_ref() {
            let view = if is_ellipsis_rendered { AccessibilityView::Content } else { AccessibilityView::Raw };
            AutomationProperties::set_accessibility_view(ellipsis, view);
        }
        for accessibility_index in 1..=visible_items_count {
            let item_to_index = first_item_to_index + accessibility_index - 1;
            if let Some(element) = self.try_get_element(item_to_index as i32) {
                AutomationProperties::set_position_in_set(&element, accessibility_index as i32);
                AutomationProperties::set_size_of_set(&element, visible_items_count as i32);
            }
        }
    }

    fn ellipsis_is_rendered(&self) -> bool {
        self.items_repeater().is_some_and(|panel| panel.ellipsis_is_rendered())
    }

    fn first_rendered_item_index_after_ellipsis(&self) -> i32 {
        self.items_repeater().map_or(0, |panel| panel.first_rendered_item_index_after_ellipsis() as i32)
    }

    /// "When focus comes from outside the BreadcrumbBar control we will put focus on the selected
    /// item."
    fn on_getting_focus(&self, args: &FocusChangingEventArgs) {
        if self.items_repeater().is_none()
            || !matches!(args.navigation_method, NavigationMethod::Tab | NavigationMethod::Directional)
        {
            return;
        }
        let from_outside = args.old_focused_element.as_ref().is_none_or(|old| self.get_element_index(old) < 0);
        if from_outside {
            // "Reset the focused index"
            self.focus_element_at(if self.ellipsis_is_rendered() { 0 } else { 1 });
            if let Some(selected) = self.try_get_element(self.focused_index.get()) {
                let selected: Ref<InputElement> = selected.upcast();
                if args.try_set_new_focused_element(Some(&selected)) {
                    args.set_handled(true);
                }
            }
        } else if !args.key_modifiers.contains(KeyModifiers::CONTROL) {
            // "Focus was already in the repeater: in RS3+ Selection follows focus unless control is
            // held down."
            if let Some(new_focused) = args.new_focused_element() {
                self.focus_element_at(self.get_element_index(&new_focused));
            }
        }
    }

    fn focus_element_at(&self, index: i32) {
        if index >= 0 {
            self.focused_index.set(index);
        }
    }

    fn move_focus(&self, index_increment: i32, source: Option<&InputElement>) -> bool {
        let Some(focused) = source else { return false };
        let mut focused_index = self.get_element_index(focused);
        if focused_index < 0 || index_increment == 0 {
            return false;
        }
        focused_index += index_increment;
        let item_count = self.element_count();
        while focused_index >= 0 && focused_index < item_count {
            if let Some(item) = self.try_get_element(focused_index) {
                if item.focus_with(NavigationMethod::Directional, KeyModifiers::empty()) {
                    self.focus_element_at(focused_index);
                    return true;
                }
            }
            focused_index += index_increment;
        }
        false
    }

    fn move_focus_previous(&self, source: Option<&InputElement>) -> bool {
        let mut movement_previous = -1;
        // "If the focus is in the first visible item, then move to the ellipsis"
        let focused_index = self.focused_index.get();
        if focused_index == 1 {
            movement_previous = 0;
        } else if self.ellipsis_is_rendered() && focused_index == self.first_rendered_item_index_after_ellipsis() {
            movement_previous = -focused_index;
        }
        self.move_focus(movement_previous, source)
    }

    fn move_focus_next(&self, source: Option<&InputElement>) -> bool {
        let mut movement_next = 1;
        // "If the focus is in the ellipsis, then move to the first visible item"
        if self.focused_index.get() == 0 {
            movement_next = self.first_rendered_item_index_after_ellipsis();
        }
        self.move_focus(movement_next, source)
    }

    /// The preview key down of the bar: Left and Right (mirrored right to left) move the focus
    /// (`OnChildPreviewKeyDown`).
    fn on_child_preview_key_down(&self, args: &KeyEventArgs) {
        let flow_direction_is_ltr = self.flow_direction() == FlowDirection::LeftToRight;
        let key_is_left = args.key == Key::Left;
        let key_is_right = args.key == Key::Right;
        let source = args.source().and_then(|source| source.cast::<InputElement>());
        // The focus is on an item of the bar, not of the flyout of the ellipsis item.
        if source.as_ref().is_none_or(|source| self.get_element_index(source) < 0) {
            return;
        }
        if (flow_direction_is_ltr && key_is_right) || (!flow_direction_is_ltr && key_is_left) {
            // "Moving to the next element"
            if self.move_focus_next(source.as_deref()) {
                args.set_handled(true);
            }
        } else if ((flow_direction_is_ltr && key_is_left) || (!flow_direction_is_ltr && key_is_right))
            && self.move_focus_previous(source.as_deref())
        {
            // "Moving to previous element"
            args.set_handled(true);
        }
    }
}
