//! WinUI's `ListView`, `ListViewBaseItem`, `ListViewItem` and `ListViewItemTemplateSettings`
//! (`dxaml/xcp/dxaml/lib/ListView_Partial.cpp`, `ListViewBaseItem_Partial.cpp` of microsoft-ui-xaml
//! commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::{ItemsStackPanel, ListViewBase};
use crate::generators::RecycleKey;
use crate::primitives::{SelectingItemsControlImpl, TemplatedControlImpl};
use crate::templates::{FuncTemplate, ITemplateOf};
use crate::{ContentControlImpl, Control, ControlImpl, ItemsControl, ItemsControlImpl, ListBoxItem, Panel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_type, instantiate, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use std::rc::Rc;

/// A vertical list of items (WinUI's `ListView`): a [`ListViewBase`] whose items panel is an
/// [`ItemsStackPanel`] and whose containers are [`ListViewItem`]s.
#[repr(C)]
pub struct ListView {
    base: ListViewBase,
}

ferro_class!(ListView: ListViewBase);
ferroui_base::ferro_class_info!(ListView { new: ListView::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(
    ListView: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    SelectingItemsControlImpl,
    FerroObjectImpl
);

impl ControlImpl for ListView {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ListViewAutomationPeer::new(this).upcast()
    }
}

impl ItemsControlImpl for ListView {
    fn create_container_for_item_override(
        this: &Self,
        item: &Option<BoxedValue>,
        index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        this.choose_item_container(item, index).unwrap_or_else(|| ListViewItem::new().upcast())
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<ListViewItem>(item)
    }
}

impl ListView {
    fn static_constructor() {
        // WinUI's style: an ItemsStackPanel.
        let default_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
            FuncTemplate::new(|| Some(ItemsStackPanel::new().upcast::<Panel>()));
        ItemsControl::items_panel_property().override_default_value::<ListView>(default_panel);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ListViewBase::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// The values the template of a list item binds to (WinUI's `ListViewItemTemplateSettings`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ListViewItemTemplateSettings {
    drag_items_count: i32,
}

impl ListViewItemTemplateSettings {
    /// The number of items dragged with this one: 0 (dragging is not ported).
    pub fn drag_items_count(&self) -> i32 {
        self.drag_items_count
    }
}

ferro_markup_type!(class ListViewItemTemplateSettings {
    namespace: "FerroUI.Controls.Primitives",
    handles: [ListViewItemTemplateSettings],
    properties: [
        DragItemsCount: i32 { get: |settings: &ListViewItemTemplateSettings| settings.drag_items_count() },
    ],
});

/// The base of the items of a list view (WinUI's `ListViewBaseItem : SelectorItem`). FerroUI's
/// `SelectorItem` is [`ListBoxItem`]; deriving from it keeps the keyed list item themes of the
/// WinUI theme applicable and reuses its selection (decision D4).
#[repr(C)]
pub struct ListViewBaseItem {
    base: ListBoxItem,
}

ferro_class!(ListViewBaseItem: ListBoxItem);
ferroui_base::ferro_class_info!(ListViewBaseItem {
    // No markup constructor: WinUI's constructor is protected (the base of the items).
    markup: { namespace: "FerroUI.Controls.Primitives", content: Content },
});
ferro_impl_classes!(
    ListViewBaseItem: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    FerroObjectImpl
);

impl ControlImpl for ListViewBaseItem {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ListViewBaseItemAutomationPeer::new(this).upcast()
    }
}

impl ListViewBaseItem {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ListBoxItem::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// An item of a [`ListView`] (WinUI's `ListViewItem`).
#[repr(C)]
pub struct ListViewItem {
    base: ListViewBaseItem,
}

ferro_class!(ListViewItem: ListViewBaseItem);
ferroui_base::ferro_class_info!(ListViewItem {
    new: ListViewItem::new,
    markup: { namespace: "FerroUI.Controls", content: Content },
});
ferro_impl_classes!(
    ListViewItem: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    FerroObjectImpl
);

impl ControlImpl for ListViewItem {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ListViewItemAutomationPeer::new(this).upcast()
    }
}

impl ListViewItem {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ListViewBaseItem::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// WinUI's `TemplateSettings`.
    pub fn template_settings(&self) -> ListViewItemTemplateSettings {
        ListViewItemTemplateSettings::default()
    }
}
