//! WinUI's `GridView`, `GridViewItem` and `GridViewItemTemplateSettings`
//! (`dxaml/xcp/dxaml/lib/GridView_Partial.cpp` and the item classes of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md; docs/porting/winui-calculator-gaps.md, LV-2).

use super::{ItemsWrapGrid, ListViewBase, ListViewBaseItem};
use crate::generators::RecycleKey;
use crate::primitives::{SelectingItemsControlImpl, TemplatedControlImpl};
use crate::templates::{FuncTemplate, ITemplateOf};
use crate::{ContentControlImpl, Control, ControlImpl, ItemsControl, ItemsControlImpl, Panel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_type, instantiate, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use std::rc::Rc;

/// A grid of items (WinUI's `GridView`): a [`ListViewBase`] whose items panel is an
/// [`ItemsWrapGrid`] in rows and whose containers are [`GridViewItem`]s.
#[repr(C)]
pub struct GridView {
    base: ListViewBase,
}

ferro_class!(GridView: ListViewBase);
ferroui_base::ferro_class_info!(GridView { new: GridView::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(
    GridView: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    SelectingItemsControlImpl,
    FerroObjectImpl
);

impl ControlImpl for GridView {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::GridViewAutomationPeer::new(this).upcast()
    }
}

impl ItemsControlImpl for GridView {
    fn create_container_for_item_override(
        this: &Self,
        item: &Option<BoxedValue>,
        index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        this.choose_item_container(item, index).unwrap_or_else(|| GridViewItem::new().upcast())
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<GridViewItem>(item)
    }
}

impl GridView {
    fn static_constructor() {
        // WinUI's style: an ItemsWrapGrid in rows.
        let default_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> = FuncTemplate::new(|| {
            let panel = ItemsWrapGrid::new();
            panel.set_orientation(Orientation::Horizontal);
            Some(panel.upcast::<Panel>())
        });
        ItemsControl::items_panel_property().override_default_value::<GridView>(default_panel);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ListViewBase::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// The values the template of a grid item binds to (WinUI's `GridViewItemTemplateSettings`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GridViewItemTemplateSettings {
    drag_items_count: i32,
}

impl GridViewItemTemplateSettings {
    /// The number of items dragged with this one: 0 (dragging is not ported).
    pub fn drag_items_count(&self) -> i32 {
        self.drag_items_count
    }
}

ferro_markup_type!(class GridViewItemTemplateSettings {
    namespace: "FerroUI.Controls.Primitives",
    handles: [GridViewItemTemplateSettings],
    properties: [
        DragItemsCount: i32 { get: |settings: &GridViewItemTemplateSettings| settings.drag_items_count() },
    ],
});

/// An item of a [`GridView`] (WinUI's `GridViewItem`).
#[repr(C)]
pub struct GridViewItem {
    base: ListViewBaseItem,
}

ferro_class!(GridViewItem: ListViewBaseItem);
ferroui_base::ferro_class_info!(GridViewItem {
    new: GridViewItem::new,
    markup: { namespace: "FerroUI.Controls", content: Content },
});
ferro_impl_classes!(
    GridViewItem: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    FerroObjectImpl
);

impl ControlImpl for GridViewItem {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::GridViewItemAutomationPeer::new(this).upcast()
    }
}

impl GridViewItem {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ListViewBaseItem::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// WinUI's `TemplateSettings`.
    pub fn template_settings(&self) -> GridViewItemTemplateSettings {
        GridViewItemTemplateSettings::default()
    }
}
