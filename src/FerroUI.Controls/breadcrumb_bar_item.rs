//! WinUI's `BreadcrumbBarItem` (`controls/dev/Breadcrumb/BreadcrumbBarItem.cpp` and `.h` of
//! microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use crate::automation::peers::{AutomationPeer, BreadcrumbBarItemAutomationPeer};
use crate::automation::AutomationProperties;
use crate::flyouts::Flyout;
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::templates::IDataTemplate;
use crate::{BreadcrumbBar, Button, ContentControl, ContentControlImpl, Control, ControlImpl, PlacementMode, StackPanel};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, Key, KeyEventArgs, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl};
use ferroui_base::media::FlowDirection;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::styling::ControlTheme;

use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, Visual, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The key of the presenter theme of the flyout of the ellipsis item (the `FlyoutPresenterStyle`
/// of WinUI's `PART_EllipsisFlyout`).
const ELLIPSIS_FLYOUT_PRESENTER_STYLE: &str = "BreadcrumbBarEllipsisFlyoutPresenterStyle";

/// An item of a [`BreadcrumbBar`] (WinUI's `BreadcrumbBarItem`): a crumb, the ellipsis item that
/// opens a flyout of the hidden crumbs, the last (current) crumb, or a crumb of that flyout.
///
/// The bar creates its items; the content of an item is the item of the bar. The template is the
/// control theme of the WinUI theme (`Controls/BreadcrumbBar.xaml`, from WinUI's
/// `BreadcrumbBar.xaml`), with WinUI's part `PART_ItemButton`. The item sets WinUI's visual states
/// as pseudo-classes: `:ellipsisdropdown` (ItemTypeStates), `:ellipsis`, `:last` and `:rtl`
/// (InlineItemTypeStates: Default, DefaultRTL, LastItem, Ellipsis and EllipsisRTL), and, for a
/// crumb of the flyout, `:pressed` (EllipsisDropDownItemCommonStates, with `:pointerover` and
/// `:disabled`). The states of `PART_ItemButton` are its own pseudo-classes.
#[repr(C)]
pub struct BreadcrumbBarItem {
    base: ContentControl,
    parent_breadcrumb: RefCell<Option<WeakRef<BreadcrumbBar>>>,
    ellipsis_item: RefCell<Option<WeakRef<BreadcrumbBarItem>>>,
    index: Cell<usize>,
    is_ellipsis_item: Cell<bool>,
    is_last_item: Cell<bool>,
    is_ellipsis_drop_down_item: Cell<bool>,
    is_pressed: Cell<bool>,
    tracked_pointer_id: Cell<Option<i32>>,
    button: RefCell<Option<Ref<Button>>>,
    button_subscription: RefCell<Option<Subscription>>,
    ellipsis_flyout: RefCell<Option<Ref<Flyout>>>,
    ellipsis_items_panel: RefCell<Option<Ref<StackPanel>>>,
    ellipsis_drop_down_item_data_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
}

ferro_class!(BreadcrumbBarItem: ContentControl);
ferroui_base::ferro_class_info!(BreadcrumbBarItem {
    new: BreadcrumbBarItem::new,
    markup: {
        namespace: "FerroUI.Controls",
        attributes: [
            TemplatePart("PART_ItemButton", type(Ref<Button>)),
            PseudoClasses(":ellipsisdropdown", ":ellipsis", ":last", ":rtl", ":pressed"),
        ],
    },
});
ferro_impl_classes!(BreadcrumbBarItem: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, ContentControlImpl);

impl FerroObjectImpl for BreadcrumbBarItem {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Visual::flow_direction_property().as_property() && !this.is_ellipsis_drop_down_item.get() {
            // OnFlowDirectionChanged
            this.update_inline_item_type_visual_state();
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
        if !this.is_ellipsis_drop_down_item.get() {
            drop(this.button_subscription.take());
            let button = args.name_scope().find_as::<Button>("PART_ItemButton");
            if let Some(button) = &button {
                // OnLoadedEvent: the click of the button is the click of the item.
                let weak = this.to_ref().downgrade();
                let token = button.click(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        if this.is_ellipsis_item.get() {
                            this.on_ellipsis_item_click();
                        } else {
                            this.on_breadcrumb_bar_item_click();
                        }
                    }
                });
                let weak_button = button.downgrade();
                *this.button_subscription.borrow_mut() = Some(Subscription::new(Disposable::create(move || {
                    if let Some(button) = weak_button.upgrade() {
                        button.remove_handler(Button::click_event(), token);
                    }
                })));
            }
            *this.button.borrow_mut() = button;
            this.update_inline_item_type_visual_state();
        }
        this.update_item_type_visual_state();
    }
}

impl InputElementImpl for BreadcrumbBarItem {
    // OnChildPreviewKeyDown: Enter and Space click the item.
    fn on_key_down(this: &Self, args: &KeyEventArgs) {
        Self::parent_on_key_down(this, args);
        if args.handled() || !matches!(args.key, Key::Enter | Key::Space) {
            return;
        }
        this.on_click_event();
        args.set_handled(true);
    }

    fn on_pointer_entered(this: &Self, args: &PointerEventArgs) {
        Self::parent_on_pointer_entered(this, args);
        if this.is_ellipsis_drop_down_item.get() {
            this.ignore_pointer_id(args.pointer().id());
        }
    }

    fn on_pointer_exited(this: &Self, args: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, args);
        if this.is_ellipsis_drop_down_item.get() {
            this.process_pointer_canceled(args.pointer().id());
        }
    }

    fn on_pointer_pressed(this: &Self, args: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, args);
        if !this.is_ellipsis_drop_down_item.get() || this.ignore_pointer_id(args.pointer().id()) {
            return;
        }
        let pressed = if args.pointer().type_() == PointerType::Mouse {
            args.get_current_point(Some(this)).properties.is_left_button_pressed
        } else {
            true
        };
        this.is_pressed.set(pressed);
        if pressed {
            this.update_ellipsis_drop_down_item_common_visual_state();
        }
    }

    fn on_pointer_released(this: &Self, args: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, args);
        if !this.is_ellipsis_drop_down_item.get() || this.ignore_pointer_id(args.pointer().id()) {
            return;
        }
        if this.is_pressed.get() {
            this.is_pressed.set(false);
            this.update_ellipsis_drop_down_item_common_visual_state();
            this.on_click_event();
        }
    }

    fn on_pointer_capture_lost(this: &Self, args: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, args);
        if this.is_ellipsis_drop_down_item.get() {
            this.process_pointer_canceled(args.pointer().id());
        }
    }
}

impl BreadcrumbBarItem {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            parent_breadcrumb: RefCell::new(None),
            ellipsis_item: RefCell::new(None),
            index: Cell::new(0),
            is_ellipsis_item: Cell::new(false),
            is_last_item: Cell::new(false),
            is_ellipsis_drop_down_item: Cell::new(false),
            is_pressed: Cell::new(false),
            tracked_pointer_id: Cell::new(None),
            button: RefCell::new(None),
            button_subscription: RefCell::new(None),
            ellipsis_flyout: RefCell::new(None),
            ellipsis_items_panel: RefCell::new(None),
            ellipsis_drop_down_item_data_template: RefCell::new(None),
        }
    }

    /// Creates an item.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the item is the ellipsis item of its bar.
    pub fn is_ellipsis_item(&self) -> bool {
        self.is_ellipsis_item.get()
    }

    /// Whether the item is the last (current) item of its bar.
    pub fn is_last_item(&self) -> bool {
        self.is_last_item.get()
    }

    /// Whether the item is a crumb of the flyout of the ellipsis item.
    pub fn is_ellipsis_drop_down_item(&self) -> bool {
        self.is_ellipsis_drop_down_item.get()
    }

    /// The flyout of the hidden crumbs, while the item is the ellipsis item.
    pub fn ellipsis_flyout(&self) -> Option<Ref<Flyout>> {
        self.ellipsis_flyout.borrow().clone()
    }

    /// The crumbs of the flyout of the ellipsis item, the last hidden one first.
    pub fn ellipsis_drop_down_items(&self) -> Vec<Ref<BreadcrumbBarItem>> {
        let panel = self.ellipsis_items_panel.borrow().clone();
        panel
            .map(|panel| panel.children().snapshot().iter().filter_map(|child| child.cast::<BreadcrumbBarItem>()).collect())
            .unwrap_or_default()
    }

    pub(crate) fn set_parent_breadcrumb(&self, parent: &BreadcrumbBar) {
        *self.parent_breadcrumb.borrow_mut() = Some(parent.to_ref().downgrade());
    }

    pub(crate) fn set_ellipsis_drop_down_item_data_template(&self, template: Option<Rc<dyn IDataTemplate>>) {
        *self.ellipsis_drop_down_item_data_template.borrow_mut() = template;
    }

    pub(crate) fn set_index(&self, index: usize) {
        self.index.set(index);
    }

    pub(crate) fn set_is_ellipsis_drop_down_item(&self, value: bool) {
        self.is_ellipsis_drop_down_item.set(value);
        self.update_item_type_visual_state();
    }

    fn set_ellipsis_item(&self, ellipsis_item: &BreadcrumbBarItem) {
        *self.ellipsis_item.borrow_mut() = Some(ellipsis_item.to_ref().downgrade());
    }

    fn raise_item_clicked_event(&self, content: Option<BoxedValue>, index: usize) {
        let parent = self.parent_breadcrumb.borrow().as_ref().and_then(WeakRef::upgrade);
        if let Some(parent) = parent {
            parent.raise_item_clicked_event(content, index);
        }
    }

    fn on_breadcrumb_bar_item_click(&self) {
        self.raise_item_clicked_event(self.content(), self.index.get().saturating_sub(1));
    }

    fn open_flyout(&self) {
        let flyout = self.ellipsis_flyout.borrow().clone();
        if let Some(flyout) = flyout {
            flyout.show_at(self);
        }
    }

    pub(crate) fn close_flyout(&self) {
        let flyout = self.ellipsis_flyout.borrow().clone();
        if let Some(flyout) = flyout {
            flyout.hide();
        }
    }

    fn update_item_type_visual_state(&self) {
        self.pseudo_classes().set(":ellipsisdropdown", self.is_ellipsis_drop_down_item.get());
    }

    fn update_ellipsis_drop_down_item_common_visual_state(&self) {
        self.pseudo_classes().set(":pressed", self.is_ellipsis_drop_down_item.get() && self.is_pressed.get());
    }

    fn update_inline_item_type_visual_state(&self) {
        let right_to_left = self.flow_direction() == FlowDirection::RightToLeft;
        let ellipsis = self.is_ellipsis_item.get();
        let last = !ellipsis && self.is_last_item.get();
        let pseudo_classes = self.pseudo_classes();
        pseudo_classes.set(":ellipsis", ellipsis);
        pseudo_classes.set(":last", last);
        pseudo_classes.set(":rtl", right_to_left && !last);
    }

    /// Opens the flyout with the hidden crumbs (WinUI's `OnEllipsisItemClick`): the hidden items of
    /// the bar in reverse order, each a crumb of the flyout.
    fn on_ellipsis_item_click(&self) {
        let parent = self.parent_breadcrumb.borrow().as_ref().and_then(WeakRef::upgrade);
        let Some(parent) = parent else { return };
        let panel = self.ellipsis_items_panel.borrow().clone();
        if let Some(panel) = panel {
            panel.children().clear();
            // CloneEllipsisItemSource: "The new list contains all the elements in reverse order"
            let hidden: Vec<_> = parent.hidden_elements().into_iter().rev().collect();
            let count = hidden.len();
            let template = self.ellipsis_drop_down_item_data_template.borrow().clone();
            for (index, item) in hidden.into_iter().enumerate() {
                let drop_down_item = BreadcrumbBarItem::new();
                drop_down_item.set_is_ellipsis_drop_down_item(true);
                drop_down_item.set_content(item);
                if let Some(template) = &template {
                    drop_down_item.set_content_template(Some(template.clone()));
                }
                // UpdateFlyoutIndex
                drop_down_item.set_ellipsis_item(self);
                drop_down_item.set_index(count - index);
                AutomationProperties::set_position_in_set(&drop_down_item, index as i32 + 1);
                AutomationProperties::set_size_of_set(&drop_down_item, count as i32);
                panel.children().add(&drop_down_item);
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
            return;
        }
        self.is_ellipsis_item.set(false);
        self.is_last_item.set(false);
        self.ellipsis_flyout.borrow_mut().take();
        self.ellipsis_items_panel.borrow_mut().take();
        self.update_inline_item_type_visual_state();
    }

    /// Creates the flyout of the hidden crumbs (WinUI's `InstantiateFlyout`): a stack of crumbs,
    /// below the ellipsis item, in the presenter theme `BreadcrumbBarEllipsisFlyoutPresenterStyle`.
    fn instantiate_flyout(&self) {
        if self.ellipsis_flyout.borrow().is_some() {
            return;
        }
        let panel = StackPanel::new();
        panel.set_name(Some("PART_EllipsisItemsRepeater".to_string()));
        AutomationProperties::set_name(&panel, Some("EllipsisItemsRepeater"));
        panel.set_horizontal_alignment(HorizontalAlignment::Stretch);
        let flyout = Flyout::new();
        flyout.set_content(Some(Control::boxed(panel.clone())));
        flyout.set_placement(PlacementMode::Bottom);
        let key = ResourceKey::from(ELLIPSIS_FLYOUT_PRESENTER_STYLE);
        let theme = self
            .try_find_resource(&key, self.actual_theme_variant().as_ref())
            .flatten()
            .and_then(|value| ValueTypes::try_cast(&value, ValueType::of::<Option<Ref<ControlTheme>>>()))
            .and_then(|value| value.downcast_ref::<Option<Ref<ControlTheme>>>().cloned().flatten());
        if let Some(theme) = theme {
            flyout.set_flyout_presenter_theme(theme);
        }
        *self.ellipsis_items_panel.borrow_mut() = Some(panel);
        *self.ellipsis_flyout.borrow_mut() = Some(flyout);
    }

    pub(crate) fn set_properties_for_ellipsis_item(&self) {
        self.is_ellipsis_item.set(true);
        self.is_last_item.set(false);
        self.instantiate_flyout();
        self.update_inline_item_type_visual_state();
    }

    fn process_pointer_canceled(&self, pointer_id: i32) {
        if self.ignore_pointer_id(pointer_id) {
            return;
        }
        self.is_pressed.set(false);
        self.tracked_pointer_id.set(None);
        self.update_ellipsis_drop_down_item_common_visual_state();
    }

    /// Whether a pointer is not the one the item tracks; the first pointer becomes the tracked one
    /// (WinUI's `IgnorePointerId`).
    fn ignore_pointer_id(&self, pointer_id: i32) -> bool {
        match self.tracked_pointer_id.get() {
            None => {
                self.tracked_pointer_id.set(Some(pointer_id));
                false
            }
            Some(tracked) => tracked != pointer_id,
        }
    }

    /// The click of the item (WinUI's `OnClickEvent`): a crumb of the flyout closes it and raises
    /// `ItemClicked` of the bar; the ellipsis item opens the flyout; another crumb raises
    /// `ItemClicked`.
    pub(crate) fn on_click_event(&self) {
        if self.is_ellipsis_drop_down_item.get() {
            let ellipsis_item = self.ellipsis_item.borrow().as_ref().and_then(WeakRef::upgrade);
            if let Some(ellipsis_item) = ellipsis_item {
                // "Once an element has been clicked, close the flyout"
                ellipsis_item.close_flyout();
                ellipsis_item.raise_item_clicked_event(self.content(), self.index.get().saturating_sub(1));
            }
        } else if self.is_ellipsis_item.get() {
            self.on_ellipsis_item_click();
        } else {
            self.on_breadcrumb_bar_item_click();
        }
    }
}
