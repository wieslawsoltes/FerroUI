//! WinUI's `TabViewListView` (`controls/dev/TabView/TabViewListView.cpp` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::{TabView, TabViewItem};
use crate::generators::RecycleKey;
use crate::list_view::ListView;
use crate::primitives::{SelectingItemsControl, SelectingItemsControlImpl, TemplatedControlImpl};
use crate::{
    ContentPresenter, Control, ControlImpl, ItemsChangedEventArgs, ItemsControlImpl, ItemsControlImplExt,
};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, VisualImpl,
};

const PC_LEFT_BOTTOM_BORDER_LINE_SHORT: &str = ":left-bottom-border-line-short";
const PC_RIGHT_BOTTOM_BORDER_LINE_SHORT: &str = ":right-bottom-border-line-short";

/// The list of the tabs of a [`TabView`] (WinUI's `TabViewListView`, a primitive of its template):
/// a [`ListView`] whose containers are [`TabViewItem`]s. It tells its tab view about the changes
/// of its items, and sets `:left-bottom-border-line-short` and `:right-bottom-border-line-short`
/// while the first or the last tab is selected (LeftBottomBorderLineShort and
/// RightBottomBorderLineShort).
#[repr(C)]
pub struct TabViewListView {
    base: ListView,
}

ferro_class!(TabViewListView: ListView);
ferroui_base::ferro_class_info!(TabViewListView {
    new: TabViewListView::new,
    markup: {
        namespace: "FerroUI.Controls.Primitives",
        attributes: [PseudoClasses(":left-bottom-border-line-short", ":right-bottom-border-line-short")],
    },
});
ferro_impl_classes!(
    TabViewListView: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    SelectingItemsControlImpl
);

impl FerroObjectImpl for TabViewListView {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == SelectingItemsControl::selected_index_property().as_property() {
            this.update_bottom_border_visual_state();
        }
    }
}

impl ControlImpl for TabViewListView {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ListViewAutomationPeer::new(this).upcast()
    }
}

impl ItemsControlImpl for TabViewListView {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        TabViewItem::new().upcast()
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<TabViewItem>(item)
    }

    fn prepare_container_for_item_override(this: &Self, container: &Ref<Control>, item: &Option<BoxedValue>, index: i32) {
        if let Some(tab) = container.cast::<TabViewItem>() {
            if let Some(tab_view) = this.tab_view() {
                tab.on_tab_view_width_mode_changed(tab_view.tab_width_mode());
                tab.on_close_button_overlay_mode_changed(tab_view.close_button_overlay_mode());
                tab.set_parent_tab_view(&tab_view);
            }
        }
        Self::parent_prepare_container_for_item_override(this, container, item, index);
    }

    fn container_for_item_prepared_override(this: &Self, container: &Ref<Control>, item: &Option<BoxedValue>, index: i32) {
        Self::parent_container_for_item_prepared_override(this, container, item, index);
        // OnContainerContentChanging: the content of the selected tab is shown again.
        if let Some(tab_view) = this.tab_view() {
            tab_view.update_tab_widths(true, true);
            tab_view.update_tab_content();
        }
    }

    fn on_items_view_collection_changed(this: &Self, e: &ItemsChangedEventArgs<'_>) {
        Self::parent_on_items_view_collection_changed(this, e);
        if let Some(tab_view) = this.tab_view() {
            tab_view.on_items_changed(e);
        }
        this.update_bottom_border_visual_state();
    }
}

impl TabViewListView {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ListView::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The tab view of the template the list is in.
    fn tab_view(&self) -> Option<Ref<TabView>> {
        self.templated_parent()?.cast::<TabView>()
    }

    fn update_bottom_border_visual_state(&self) {
        let selected = self.selected_index();
        let last = self.item_count() - 1;
        let classes = self.pseudo_classes();
        classes.set(PC_LEFT_BOTTOM_BORDER_LINE_SHORT, selected == 0);
        classes.set(PC_RIGHT_BOTTOM_BORDER_LINE_SHORT, selected >= 0 && selected == last);
    }

    /// The width of the header of the strip (`ItemsPresenter.Header` in WinUI's template).
    pub(super) fn header_width(&self) -> f64 {
        self.presenter_width("TabStripHeaderPresenter")
    }

    /// The width of the footer of the strip.
    pub(super) fn footer_width(&self) -> f64 {
        self.presenter_width("TabStripFooterPresenter")
    }

    fn presenter_width(&self, name: &str) -> f64 {
        self.find_template_descendant::<ContentPresenter>(name).map_or(0.0, |presenter| presenter.bounds().width)
    }

    /// The width of the tabs (`TabsItemsPresenter`).
    pub(super) fn items_width(&self) -> f64 {
        self.items_panel_root().map_or(0.0, |panel| panel.bounds().width)
    }
}
