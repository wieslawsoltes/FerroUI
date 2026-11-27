//! WinUI's `SelectorBar` and `SelectorBarItem` (`controls/dev/SelectorBar` of microsoft-ui-xaml
//! commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).
//!
//! The strip is a [`TabStrip`] and its items are [`TabStripItem`]s: their control themes are the
//! themes of these classes, which in the WinUI theme are `DefaultSelectorBarStyle` and
//! `DefaultSelectorBarItemStyle`. Unlike a tab strip, a selector bar may have no selection until it
//! gets the focus.

use crate::automation::AutomationProperties;
use crate::generators::RecycleKey;
use crate::presenters::ContentPresenter;
use crate::primitives::{
    SelectingItemsControl, SelectingItemsControlImpl, TabStrip, TabStripItem, TemplatedControlImpl,
};
use crate::{
    ContentControl, ContentControlImpl, Control, ControlImpl, IconElement, ItemsControlImpl, SelectionMode, StackPanel,
    TextBlock,
};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{FocusChangedEventArgs, InputElementImpl, InputElementImplExt};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::media::{ScaleTransform, TextWrapping};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, RelativePoint, StaticType,
    StyledElementImpl, StyledProperty, Thickness, TypeInfo, Visual, VisualImpl,
};
use std::cell::RefCell;

/// `SelectorBarItemSpacing` of the WinUI theme.
const ITEM_SPACING: f64 = 8.0;
/// `SelectorBarItemIconVisualMargin` of the WinUI theme.
const ICON_MARGIN: f64 = -2.0;
/// `SelectorBarItemIconScale` of the WinUI theme.
const ICON_SCALE: f64 = 0.8;

/// The `PART_IconVisual` and `PART_TextVisual` of the item, in their stack panel.
struct ItemParts {
    panel: Ref<StackPanel>,
    icon: Ref<ContentPresenter>,
    text: Ref<TextBlock>,
}

/// An item of a [`SelectorBar`]: its `Text` beside its optional `Icon`.
#[repr(C)]
pub struct SelectorBarItem {
    base: TabStripItem,
    parts: RefCell<Option<ItemParts>>,
}

ferro_class!(SelectorBarItem: TabStripItem);
ferro_class_info!(SelectorBarItem {
    new: SelectorBarItem::new,
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(
    SelectorBarItem: VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl StyledElementImpl for SelectorBarItem {
    // The themes of the tab strip item are the themes of the selector bar item.
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <TabStripItem as StaticType>::TYPE
    }
}

impl FerroObjectImpl for SelectorBarItem {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::text_property().as_property()
            || change.property() == Self::icon_property().as_property()
        {
            this.update_parts();
        }
    }
}

ferro_properties! {
    impl SelectorBarItem {
        /// Defines the `Text` property.
        pub fn text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<SelectorBarItem, _>("Text", None)
        }

        /// Defines the `Icon` property.
        pub fn icon_property() -> StyledProperty<Option<Ref<IconElement>>> {
            FerroProperty::register::<SelectorBarItem, _>("Icon", None)
        }
    }
}

impl SelectorBarItem {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TabStripItem::construct(), parts: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The text of the item.
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<&str>) {
        self.set_value(Self::text_property(), value.map(str::to_owned));
    }

    /// The icon of the item.
    pub fn icon(&self) -> Option<Ref<IconElement>> {
        self.get_value(Self::icon_property())
    }

    pub fn set_icon(&self, value: Option<Ref<IconElement>>) {
        self.set_value(Self::icon_property(), value);
    }

    /// `UpdatePartsVisibility`: a part without a value is collapsed, and so is the panel when the
    /// item has neither.
    fn update_parts(&self) {
        let mut parts = self.parts.borrow_mut();
        let created = parts.is_none();
        let parts = parts.get_or_insert_with(Self::create_parts);
        let icon = self.icon();
        let text = self.text().filter(|text| !text.is_empty());

        parts.icon.set_is_visible(icon.is_some());
        parts.icon.set_content(icon.as_ref().map(Control::boxed));
        parts.text.set_is_visible(text.is_some());
        parts.text.set_text(text.as_deref());
        parts.panel.set_is_visible(icon.is_some() || text.is_some());
        // The name of the peer of WinUI's item is its text; a name the application sets wins.
        self.set_value_with_priority(AutomationProperties::name_property(), text.clone(), BindingPriority::Style);

        if created {
            let panel = parts.panel.clone();
            ContentControl::set_content(self, Some(Control::boxed(panel)));
        }
    }

    fn create_parts() -> ItemParts {
        let panel = StackPanel::new();
        panel.set_orientation(Orientation::Horizontal);
        panel.set_spacing(ITEM_SPACING);
        let icon = ContentPresenter::new();
        icon.set_margin(Thickness::new(ICON_MARGIN, 0.0, ICON_MARGIN, 0.0));
        icon.set_render_transform_origin(RelativePoint::CENTER);
        let scale = ScaleTransform::new();
        scale.set_scale_x(ICON_SCALE);
        scale.set_scale_y(ICON_SCALE);
        icon.set_render_transform(Some((&scale).into()));
        let text = TextBlock::new();
        text.set_text_wrapping(TextWrapping::Wrap);
        panel.children().add(icon.clone());
        panel.children().add(text.clone());
        ItemParts { panel, icon, text }
    }
}

/// A strip of [`SelectorBarItem`]s of which one can be selected (WinUI's `SelectorBar`).
#[repr(C)]
pub struct SelectorBar {
    base: TabStrip,
}

ferro_class!(SelectorBar: TabStrip);
ferro_class_info!(SelectorBar {
    new: SelectorBar::new,
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(
    SelectorBar: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    TemplatedControlImpl,
    SelectingItemsControlImpl
);

impl StyledElementImpl for SelectorBar {
    // The themes of the tab strip are the themes of the selector bar.
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <TabStrip as StaticType>::TYPE
    }
}

impl ItemsControlImpl for SelectorBar {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        SelectorBarItem::new().upcast()
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<SelectorBarItem>(item)
    }
}

impl InputElementImpl for SelectorBar {
    /// `OnGotFocus`: a bar that gets the focus without a selection selects the focused item, or
    /// else its first focusable item.
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);

        if this.selected_item().is_some() {
            return;
        }
        let source = e.source().and_then(|source| source.cast::<Visual>());
        let focused = this
            .get_container_from_event_source(source.as_ref())
            .map(|container| this.index_from_container(&container));
        match focused {
            Some(index) if index >= 0 => this.set_selected_index(index),
            _ => this.select_first_focusable_item(),
        }
    }
}

impl SelectorBar {
    fn static_constructor() {
        SelectingItemsControl::selection_mode_property().override_default_value::<SelectorBar>(SelectionMode::SINGLE);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TabStrip::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// `SelectFirstFocusableItem`.
    fn select_first_focusable_item(&self) {
        for index in 0..self.item_count() {
            // The items of a bar are their own containers, realized or not.
            let item = self.items().get_at(index as usize).and_then(|item| Control::from_boxed(&item));
            let focusable = item
                .or_else(|| self.container_from_index(index))
                .is_some_and(|item| item.focusable() && item.is_enabled());
            if focusable {
                self.set_selected_index(index);
                return;
            }
        }
    }
}
