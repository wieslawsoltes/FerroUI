//! WinUI's `BreadcrumbBarItem` (`controls/dev/Breadcrumb/BreadcrumbBarItem.cpp`, `.h` and
//! `BreadcrumbBar.idl` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`;
//! MIT, Copyright (c) Microsoft Corporation, see NOTICE.md).

use super::breadcrumb_bar::create_breadcrumb_element;
use super::BreadcrumbBar;
use crate::automation::peers::{AutomationPeer, BreadcrumbBarItemAutomationPeer};
use crate::automation::AutomationProperties;
use crate::primitives::{Popup, TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::templates::IDataTemplate;
use crate::{Button, ContentControl, ContentControlImpl, ControlImpl, Panel};
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, NavigationMethod,
    PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::FlowDirection;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, Upcast, Visual, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// The visual states of `BreadcrumbBarItem.cpp`, as pseudo-classes (docs/porting/winui-controls.md,
// section 10.11). ItemTypeStates: Inline sets nothing.
const PC_ELLIPSIS_DROP_DOWN: &str = ":ellipsisdropdown";
// InlineItemTypeStates: Default sets nothing.
const PC_DEFAULT_RTL: &str = ":defaultrtl";
const PC_LAST_ITEM: &str = ":lastitem";
const PC_ELLIPSIS: &str = ":ellipsis";
const PC_ELLIPSIS_RTL: &str = ":ellipsisrtl";
const INLINE_ITEM_TYPE_STATES: &[&str] = &[PC_DEFAULT_RTL, PC_LAST_ITEM, PC_ELLIPSIS, PC_ELLIPSIS_RTL];
// EllipsisDropDownItemCommonStates: Normal sets nothing. The names keep the group apart from the
// pointer and enabled pseudo-classes FerroUI sets on every input element.
const PC_DROP_DOWN_POINTER_OVER: &str = ":dropdownpointerover";
const PC_DROP_DOWN_PRESSED: &str = ":dropdownpressed";
const PC_DROP_DOWN_DISABLED: &str = ":dropdowndisabled";
const DROP_DOWN_COMMON_STATES: &[&str] = &[PC_DROP_DOWN_POINTER_OVER, PC_DROP_DOWN_PRESSED, PC_DROP_DOWN_DISABLED];

// Template parts.
const ITEM_BUTTON_PART_NAME: &str = "PART_ItemButton";
const ITEM_ELLIPSIS_FLYOUT_PART_NAME: &str = "PART_EllipsisFlyout";
const ELLIPSIS_ITEMS_REPEATER_PART_NAME: &str = "PART_EllipsisItemsRepeater";

// Automation names.
const ELLIPSIS_FLYOUT_AUTOMATION_NAME: &str = "EllipsisFlyout";
const ELLIPSIS_ITEMS_REPEATER_AUTOMATION_NAME: &str = "EllipsisItemsRepeater";

/// An item of a [`BreadcrumbBar`] (WinUI's `BreadcrumbBarItem`): a content control that shows
/// one crumb of the bar, followed by a chevron. The bar creates one for each item of its items
/// source (an item that is a breadcrumb bar item is its own), and one more as its first child, the
/// ellipsis item, which opens a flyout of the items that do not fit. The items of that flyout are
/// breadcrumb bar items as well, in the drop-down item type.
///
/// Clicking an item, or Enter or Space on it, raises the `ItemClicked` event of the bar with the
/// item and its index; the last item, the current one, shows its content without a button.
///
/// The template is the control theme of the WinUI theme (`Controls/BreadcrumbBar.xaml`, from
/// WinUI's `BreadcrumbBar.xaml`), with WinUI's parts `PART_ItemButton`, `PART_EllipsisFlyout` and
/// the panel `PART_EllipsisItemsRepeater` of the flyout. The item sets WinUI's visual states as
/// pseudo-classes: `:ellipsisdropdown` (the drop-down item type), `:defaultrtl`, `:lastitem`,
/// `:ellipsis` and `:ellipsisrtl` (the inline item types), and `:dropdownpointerover`,
/// `:dropdownpressed` and `:dropdowndisabled` (the common states of a drop-down item). The Fluent
/// and Simple themes have no template for it (docs/porting/winui-controls.md, section 5.1).
#[repr(C)]
pub struct BreadcrumbBarItem {
    base: ContentControl,
    // Common item fields: the index from 1 (0 is the ellipsis item).
    index: Cell<u32>,
    is_ellipsis_drop_down_item: Cell<bool>,
    // Inline item fields.
    is_ellipsis_item: Cell<bool>,
    is_last_item: Cell<bool>,
    parts: RefCell<Parts>,
    // The bar, to ask for the hidden items and to raise the clicks.
    parent_breadcrumb: RefCell<Option<WeakRef<BreadcrumbBar>>>,
    ellipsis_drop_down_item_data_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    // Ellipsis drop-down item fields: the ellipsis item that owns the flyout.
    ellipsis_item: RefCell<Option<WeakRef<BreadcrumbBarItem>>>,
    // Visual state tracking of a drop-down item.
    tracked_pointer_id: Cell<Option<i32>>,
    is_pressed: Cell<bool>,
    is_pointer_over: Cell<bool>,
}

#[derive(Default)]
struct Parts {
    button: Option<Ref<Button>>,
    button_click: Option<ferroui_base::interactivity::RoutedEventHandlerToken>,
    ellipsis_flyout: Option<Ref<Popup>>,
    ellipsis_items_repeater: Option<Ref<Panel>>,
}

ferro_class!(BreadcrumbBarItem: ContentControl);
ferro_class_info!(BreadcrumbBarItem {
    new: BreadcrumbBarItem::new,
    markup: {
        namespace: "FerroUI.Controls",
        attributes: [
            TemplatePart("PART_ItemButton", type(Ref<Button>)),
            TemplatePart("PART_EllipsisFlyout", type(Ref<Popup>)),
            TemplatePart("PART_EllipsisItemsRepeater", type(Ref<Panel>)),
            PseudoClasses(
                ":ellipsisdropdown", ":defaultrtl", ":lastitem", ":ellipsis", ":ellipsisrtl",
                ":dropdownpointerover", ":dropdownpressed", ":dropdowndisabled"
            ),
        ],
    },
});
ferro_impl_classes!(
    BreadcrumbBarItem: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ContentControlImpl
);

impl FerroObjectImpl for BreadcrumbBarItem {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        // HookListeners: the preview key down of the item (a tunnelling handler).
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
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Visual::flow_direction_property().as_property() {
            // OnFlowDirectionChanged
            if !this.is_ellipsis_drop_down_item.get() {
                this.update_inline_item_type_visual_state();
            }
        } else if property == InputElement::is_enabled_property().as_property()
            || property == InputElement::is_effectively_enabled_property().as_property()
        {
            // OnIsEnabledChanged
            if this.is_ellipsis_drop_down_item.get() {
                this.update_ellipsis_drop_down_item_common_visual_state();
            }
        }
    }
}

impl ControlImpl for BreadcrumbBarItem {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        BreadcrumbBarItemAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for BreadcrumbBarItem {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        this.revoke_parts_listeners();
        let scope = args.name_scope();
        if this.is_ellipsis_drop_down_item.get() {
            this.update_ellipsis_drop_down_item_common_visual_state();
        } else {
            let mut parts = Parts {
                button: scope.find_as::<Button>(ITEM_BUTTON_PART_NAME),
                ellipsis_flyout: scope.find_as::<Popup>(ITEM_ELLIPSIS_FLYOUT_PART_NAME),
                ellipsis_items_repeater: scope.find_as::<Panel>(ELLIPSIS_ITEMS_REPEATER_PART_NAME),
                button_click: None,
            };
            // OnLoadedEvent: the click of the button is the click of the item (the ellipsis item
            // or a crumb, decided when it happens).
            if let Some(button) = &parts.button {
                let weak = this.to_ref().downgrade();
                parts.button_click = Some(button.click(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        if this.is_ellipsis_item.get() {
                            this.on_ellipsis_item_click();
                        } else {
                            this.on_breadcrumb_bar_item_click();
                        }
                    }
                }));
            }
            *this.parts.borrow_mut() = parts;
            if this.is_ellipsis_item.get() {
                this.set_properties_for_ellipsis_item();
            } else if this.is_last_item.get() {
                this.set_properties_for_last_item();
            } else {
                this.update_inline_item_type_visual_state();
            }
        }
        this.update_item_type_visual_state();
    }
}

impl InputElementImpl for BreadcrumbBarItem {
    fn on_pointer_entered(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_entered(this, e);
        if this.is_ellipsis_drop_down_item.get() {
            this.process_pointer_over(e.pointer().id());
        }
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        if this.is_ellipsis_drop_down_item.get() {
            this.process_pointer_over(e.pointer().id());
        }
    }

    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);
        if this.is_ellipsis_drop_down_item.get() {
            this.process_pointer_canceled(e.pointer().id());
        }
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if !this.is_ellipsis_drop_down_item.get() || this.ignore_pointer_id(e.pointer().id()) {
            return;
        }
        let is_pressed = if e.pointer().type_() == PointerType::Mouse {
            e.get_current_point(Some(this.upcast())).properties.is_left_button_pressed
        } else {
            true
        };
        this.is_pressed.set(is_pressed);
        if is_pressed {
            this.update_ellipsis_drop_down_item_common_visual_state();
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if !this.is_ellipsis_drop_down_item.get() || this.ignore_pointer_id(e.pointer().id()) {
            return;
        }
        if this.is_pressed.get() {
            this.is_pressed.set(false);
            this.update_ellipsis_drop_down_item_common_visual_state();
            this.on_click_event();
        }
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);
        if this.is_ellipsis_drop_down_item.get() {
            this.process_pointer_canceled(e.pointer().id());
        }
    }
}

impl BreadcrumbBarItem {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            index: Cell::new(0),
            is_ellipsis_drop_down_item: Cell::new(false),
            is_ellipsis_item: Cell::new(false),
            is_last_item: Cell::new(false),
            parts: RefCell::new(Parts::default()),
            parent_breadcrumb: RefCell::new(None),
            ellipsis_drop_down_item_data_template: RefCell::new(None),
            ellipsis_item: RefCell::new(None),
            tracked_pointer_id: Cell::new(None),
            is_pressed: Cell::new(false),
            is_pointer_over: Cell::new(false),
        }
    }

    /// Creates a breadcrumb bar item.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    #[cfg(test)]
    /// Whether the item is an item of the flyout of the ellipsis item.
    pub(crate) fn is_ellipsis_drop_down_item(&self) -> bool {
        self.is_ellipsis_drop_down_item.get()
    }

    #[cfg(test)]
    /// Whether the item is the ellipsis item of its bar.
    pub(crate) fn is_ellipsis_item(&self) -> bool {
        self.is_ellipsis_item.get()
    }

    #[cfg(test)]
    /// Whether the item is the last item, the current one, of its bar.
    pub(crate) fn is_last_item(&self) -> bool {
        self.is_last_item.get()
    }

    #[cfg(test)]
    /// The index of the item from 1 (0 for the ellipsis item).
    pub(crate) fn index(&self) -> u32 {
        self.index.get()
    }

    /// The button of the crumb, once the template is applied.
    pub(crate) fn item_button(&self) -> Option<Ref<Button>> {
        self.parts.borrow().button.clone()
    }

    /// The items panel of the flyout of the ellipsis item, once the template is applied.
    pub(crate) fn ellipsis_items_repeater(&self) -> Option<Ref<Panel>> {
        self.parts.borrow().ellipsis_items_repeater.clone()
    }

    /// The flyout of the ellipsis item, once the template is applied.
    pub(crate) fn ellipsis_flyout(&self) -> Option<Ref<Popup>> {
        self.parts.borrow().ellipsis_flyout.clone()
    }

    fn revoke_parts_listeners(&self) {
        let parts = std::mem::take(&mut *self.parts.borrow_mut());
        if let (Some(button), Some(token)) = (parts.button, parts.button_click) {
            button.remove_handler(Button::click_event(), token);
        }
    }

    pub(crate) fn set_parent_breadcrumb(&self, parent: &BreadcrumbBar) {
        *self.parent_breadcrumb.borrow_mut() = Some(parent.to_ref().downgrade());
    }

    fn parent_breadcrumb(&self) -> Option<Ref<BreadcrumbBar>> {
        self.parent_breadcrumb.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn set_ellipsis_drop_down_item_data_template(&self, template: Option<Rc<dyn IDataTemplate>>) {
        *self.ellipsis_drop_down_item_data_template.borrow_mut() = template;
    }

    pub(crate) fn set_index(&self, index: u32) {
        self.index.set(index);
    }

    pub(crate) fn set_is_ellipsis_drop_down_item(&self, is_ellipsis_drop_down_item: bool) {
        self.is_ellipsis_drop_down_item.set(is_ellipsis_drop_down_item);
        self.update_item_type_visual_state();
        if is_ellipsis_drop_down_item {
            self.update_ellipsis_drop_down_item_common_visual_state();
        }
    }

    pub(crate) fn set_ellipsis_item(&self, ellipsis_item: &BreadcrumbBarItem) {
        *self.ellipsis_item.borrow_mut() = Some(ellipsis_item.to_ref().downgrade());
    }

    fn raise_item_clicked_event(&self, content: Option<ferroui_base::BoxedValue>, index: u32) {
        if let Some(breadcrumb) = self.parent_breadcrumb() {
            breadcrumb.raise_item_clicked_event(content, index);
        }
    }

    fn on_breadcrumb_bar_item_click(&self) {
        self.raise_item_clicked_event(self.content(), self.index.get().wrapping_sub(1));
    }

    /// The preview key down of the item: Enter and Space click it.
    fn on_child_preview_key_down(&self, args: &KeyEventArgs) {
        if !matches!(args.key, Key::Enter | Key::Space) || !self.is_key_source(args) {
            return;
        }
        if self.is_ellipsis_drop_down_item.get() {
            self.on_click_event();
        } else if self.is_ellipsis_item.get() {
            self.on_ellipsis_item_click();
        } else {
            self.on_breadcrumb_bar_item_click();
        }
        args.set_handled(true);
    }

    /// Whether the key event comes from this item, and not from an item of the flyout of the
    /// ellipsis item, whose events tunnel through the ellipsis item here (the flyout of WinUI is
    /// outside the tree of the item).
    fn is_key_source(&self, args: &KeyEventArgs) -> bool {
        let mut visual = args.source().and_then(|source| source.cast::<Visual>());
        while let Some(current) = visual {
            if let Some(item) = current.cast::<BreadcrumbBarItem>() {
                return std::ptr::eq::<BreadcrumbBarItem>(&*item, self);
            }
            visual = current.visual_parent();
        }
        false
    }

    /// The prepared drop-down item at `index` of the flyout (`OnFlyoutElementPreparedEvent` with
    /// `UpdateFlyoutIndex`): its index counts from the end, as the flyout lists the hidden items
    /// from the last to the first.
    fn prepare_flyout_element(&self, element: &BreadcrumbBarItem, index: usize, item_count: usize) {
        element.set_is_ellipsis_drop_down_item(true);
        element.set_ellipsis_item(self);
        element.set_index((item_count - index) as u32);
        AutomationProperties::set_position_in_set(element, index as i32 + 1);
        AutomationProperties::set_size_of_set(element, item_count as i32);
    }

    /// A copy of the hidden items in the reverse order (`CloneEllipsisItemSource`).
    fn clone_ellipsis_item_source(
        hidden: &[Option<ferroui_base::BoxedValue>],
    ) -> Vec<Option<ferroui_base::BoxedValue>> {
        hidden.iter().rev().cloned().collect()
    }

    fn open_flyout(&self) {
        if let Some(flyout) = self.ellipsis_flyout() {
            // ShowAt(*this)
            flyout.set_placement_target(self.to_ref().upcast::<crate::Control>());
            flyout.set_is_open(true);
            // The flyout gives the focus to its content.
            if let Some(first) =
                self.ellipsis_items_repeater().and_then(|panel| panel.children().snapshot().first().cloned())
            {
                first.focus_with(NavigationMethod::Unspecified, Default::default());
            }
        }
    }

    pub(crate) fn close_flyout(&self) {
        if let Some(flyout) = self.ellipsis_flyout() {
            flyout.set_is_open(false);
        }
    }

    fn go_to_state(&self, group: &[&str], state: Option<&str>) {
        let pseudo_classes = self.pseudo_classes();
        for name in group {
            pseudo_classes.set(name, state == Some(*name));
        }
    }

    fn update_item_type_visual_state(&self) {
        self.pseudo_classes().set(PC_ELLIPSIS_DROP_DOWN, self.is_ellipsis_drop_down_item.get());
    }

    fn update_ellipsis_drop_down_item_common_visual_state(&self) {
        let state = if !self.is_effectively_enabled() {
            Some(PC_DROP_DOWN_DISABLED)
        } else if self.is_pressed.get() {
            Some(PC_DROP_DOWN_PRESSED)
        } else if self.is_pointer_over.get() {
            Some(PC_DROP_DOWN_POINTER_OVER)
        } else {
            None
        };
        self.go_to_state(DROP_DOWN_COMMON_STATES, state);
    }

    fn update_inline_item_type_visual_state(&self) {
        let is_left_to_right = self.flow_direction() == FlowDirection::LeftToRight;
        let state = if self.is_ellipsis_item.get() {
            Some(if is_left_to_right { PC_ELLIPSIS } else { PC_ELLIPSIS_RTL })
        } else if self.is_last_item.get() {
            Some(PC_LAST_ITEM)
        } else if is_left_to_right {
            None
        } else {
            Some(PC_DEFAULT_RTL)
        };
        self.go_to_state(INLINE_ITEM_TYPE_STATES, state);
    }

    /// The click of the ellipsis item: fills the flyout with the hidden items, the last first,
    /// and opens it (`OnEllipsisItemClick`).
    fn on_ellipsis_item_click(&self) {
        let Some(breadcrumb) = self.parent_breadcrumb() else { return };
        let hidden_elements = Self::clone_ellipsis_item_source(&breadcrumb.hidden_elements());
        if let Some(panel) = self.ellipsis_items_repeater() {
            let template = self.ellipsis_drop_down_item_data_template.borrow().clone();
            let children = panel.children();
            children.clear();
            let item_count = hidden_elements.len();
            for (index, item) in hidden_elements.into_iter().enumerate() {
                let element = create_breadcrumb_element(item, template.as_ref());
                self.prepare_flyout_element(&element, index, item_count);
                children.add(element);
            }
        }
        self.open_flyout();
    }

    pub(crate) fn set_properties_for_last_item(&self) {
        self.is_ellipsis_item.set(false);
        self.is_last_item.set(true);
        self.update_inline_item_type_visual_state();
    }

    pub(crate) fn reset_visual_properties(&self) {
        if self.is_ellipsis_drop_down_item.get() {
            self.update_ellipsis_drop_down_item_common_visual_state();
        } else {
            self.is_ellipsis_item.set(false);
            self.is_last_item.set(false);
            self.update_inline_item_type_visual_state();
        }
    }

    /// InstantiateFlyout: the automation names of the flyout and of its items panel.
    fn instantiate_flyout(&self) {
        let parts = self.parts.borrow();
        if let (Some(_), Some(flyout)) = (&parts.button, &parts.ellipsis_flyout) {
            AutomationProperties::set_name(flyout, Some(ELLIPSIS_FLYOUT_AUTOMATION_NAME));
            if let Some(panel) = &parts.ellipsis_items_repeater {
                AutomationProperties::set_name(panel, Some(ELLIPSIS_ITEMS_REPEATER_AUTOMATION_NAME));
            }
        }
    }

    pub(crate) fn set_properties_for_ellipsis_item(&self) {
        self.is_ellipsis_item.set(true);
        self.is_last_item.set(false);
        self.instantiate_flyout();
        self.update_inline_item_type_visual_state();
    }

    fn process_pointer_over(&self, pointer_id: i32) {
        if self.ignore_pointer_id(pointer_id) {
            return;
        }
        if !self.is_pointer_over.get() {
            self.is_pointer_over.set(true);
            self.update_ellipsis_drop_down_item_common_visual_state();
        }
    }

    fn process_pointer_canceled(&self, pointer_id: i32) {
        if self.ignore_pointer_id(pointer_id) {
            return;
        }
        self.is_pressed.set(false);
        self.is_pointer_over.set(false);
        self.tracked_pointer_id.set(None);
        self.update_ellipsis_drop_down_item_common_visual_state();
    }

    /// False when `pointer_id` is the tracked pointer, or when no pointer is tracked (it becomes
    /// the tracked one); true for another pointer (`IgnorePointerId`).
    fn ignore_pointer_id(&self, pointer_id: i32) -> bool {
        match self.tracked_pointer_id.get() {
            None => {
                self.tracked_pointer_id.set(Some(pointer_id));
                false
            }
            Some(tracked) => tracked != pointer_id,
        }
    }

    /// The click of the item, by the pointer, the keyboard or automation (`OnClickEvent`): a
    /// drop-down item closes the flyout and raises the click of its item, the ellipsis item opens
    /// the flyout, and a crumb raises its click.
    pub(crate) fn on_click_event(&self) {
        if self.is_ellipsis_drop_down_item.get() {
            let ellipsis_item = self.ellipsis_item.borrow().as_ref().and_then(WeakRef::upgrade);
            if let Some(ellipsis_item) = ellipsis_item {
                // "Once an element has been clicked, close the flyout"
                ellipsis_item.close_flyout();
                ellipsis_item.raise_item_clicked_event(self.content(), self.index.get().wrapping_sub(1));
            }
        } else if self.is_ellipsis_item.get() {
            self.on_ellipsis_item_click();
        } else {
            self.on_breadcrumb_bar_item_click();
        }
    }
}
