//! WinUI's `ItemsStackPanel` (`dxaml/xcp/dxaml/lib/ItemsStackPanel_Partial.cpp` and the metadata
//! of `Microsoft.UI.Xaml.Controls.cs` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md), on FerroUI's `VirtualizingStackPanel` (docs/porting/winui-calculator-gaps.md, LV-1).

use crate::{ControlImpl, PanelImpl, VirtualizingPanelImpl, VirtualizingStackPanel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_enum, ferro_property, instantiate, FerroObjectImpl, FerroProperty,
    Ref, StyledElementImpl, StyledProperty, Thickness, VisualImpl,
};

/// What stays in view when items are added or removed (WinUI's `ItemsUpdatingScrollMode`).
/// Accepted; the panel keeps the scroll offset of the virtualizing stack panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ItemsUpdatingScrollMode {
    #[default]
    KeepItemsInView = 0,
    KeepScrollOffset = 1,
    KeepLastItemInView = 2,
}

ferro_markup_enum!(
    ItemsUpdatingScrollMode { KeepItemsInView, KeepScrollOffset, KeepLastItemInView },
    { namespace: "FerroUI.Controls" }
);

/// Where the headers of the groups are (WinUI's `GroupHeaderPlacement`). Accepted; grouping is
/// not ported.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GroupHeaderPlacement {
    #[default]
    Top = 0,
    Left = 1,
}

ferro_markup_enum!(GroupHeaderPlacement { Top, Left }, { namespace: "FerroUI.Controls.Primitives" });

/// The virtualizing items panel of a list view (WinUI's `ItemsStackPanel`): FerroUI's
/// [`VirtualizingStackPanel`] (orientation, cache length, recycling) with WinUI's visible and
/// cache indexes. Grouping (`GroupPadding`, `GroupHeaderPlacement`, `AreStickyGroupHeadersEnabled`)
/// is accepted and has no effect: FerroUI's items controls have no groups.
#[repr(C)]
pub struct ItemsStackPanel {
    base: VirtualizingStackPanel,
}

ferro_class!(ItemsStackPanel: VirtualizingStackPanel);
ferroui_base::ferro_class_info!(ItemsStackPanel {
    new: ItemsStackPanel::new,
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(
    ItemsStackPanel: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    VirtualizingPanelImpl,
    FerroObjectImpl
);

ferroui_base::ferro_properties! { impl ItemsStackPanel {
    ferro_property!(
        /// Defines the `ItemsUpdatingScrollMode` property.
        pub fn items_updating_scroll_mode_property() -> StyledProperty<ItemsUpdatingScrollMode> {
            FerroProperty::register::<ItemsStackPanel, _>(
                "ItemsUpdatingScrollMode",
                ItemsUpdatingScrollMode::KeepItemsInView,
            )
        }
    );

    ferro_property!(
        /// Defines the `GroupPadding` property. Accepted.
        pub fn group_padding_property() -> StyledProperty<Thickness> {
            FerroProperty::register::<ItemsStackPanel, _>("GroupPadding", Thickness::default())
        }
    );

    ferro_property!(
        /// Defines the `GroupHeaderPlacement` property. Accepted.
        pub fn group_header_placement_property() -> StyledProperty<GroupHeaderPlacement> {
            FerroProperty::register::<ItemsStackPanel, _>("GroupHeaderPlacement", GroupHeaderPlacement::Top)
        }
    );

    ferro_property!(
        /// Defines the `AreStickyGroupHeadersEnabled` property. Accepted.
        pub fn are_sticky_group_headers_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<ItemsStackPanel, _>("AreStickyGroupHeadersEnabled", true)
        }
    );
} }

impl ItemsStackPanel {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: VirtualizingStackPanel::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn items_updating_scroll_mode(&self) -> ItemsUpdatingScrollMode {
        self.get_value(Self::items_updating_scroll_mode_property())
    }

    pub fn set_items_updating_scroll_mode(&self, value: ItemsUpdatingScrollMode) {
        self.set_value(Self::items_updating_scroll_mode_property(), value)
    }

    pub fn group_padding(&self) -> Thickness {
        self.get_value(Self::group_padding_property())
    }

    pub fn set_group_padding(&self, value: Thickness) {
        self.set_value(Self::group_padding_property(), value)
    }

    pub fn group_header_placement(&self) -> GroupHeaderPlacement {
        self.get_value(Self::group_header_placement_property())
    }

    pub fn set_group_header_placement(&self, value: GroupHeaderPlacement) {
        self.set_value(Self::group_header_placement_property(), value)
    }

    pub fn are_sticky_group_headers_enabled(&self) -> bool {
        self.get_value(Self::are_sticky_group_headers_enabled_property())
    }

    pub fn set_are_sticky_group_headers_enabled(&self, value: bool) {
        self.set_value(Self::are_sticky_group_headers_enabled_property(), value)
    }

    /// The first realized container, including the cache (WinUI's `FirstCacheIndex`).
    pub fn first_cache_index(&self) -> i32 {
        self.first_realized_index()
    }

    /// The last realized container, including the cache (WinUI's `LastCacheIndex`).
    pub fn last_cache_index(&self) -> i32 {
        self.last_realized_index()
    }

    /// The realized indexes whose containers intersect the viewport.
    fn visible_indexes(&self) -> impl Iterator<Item = i32> + '_ {
        let viewport = self.view_port();
        let horizontal = self.orientation() == Orientation::Horizontal;
        let (first, last) = (self.first_realized_index(), self.last_realized_index());
        (first.max(0)..=last).filter(move |index| {
            if *index < 0 {
                return false;
            }
            self.container_from_index(*index).is_some_and(|container| {
                let bounds = container.bounds();
                if horizontal {
                    bounds.right() > viewport.x && bounds.x < viewport.right()
                } else {
                    bounds.bottom() > viewport.y && bounds.y < viewport.bottom()
                }
            })
        })
    }

    /// The first item at least partly in the viewport (WinUI's `FirstVisibleIndex`), -1 without.
    pub fn first_visible_index(&self) -> i32 {
        self.visible_indexes().next().unwrap_or(-1)
    }

    /// The last item at least partly in the viewport (WinUI's `LastVisibleIndex`), -1 without.
    pub fn last_visible_index(&self) -> i32 {
        self.visible_indexes().last().unwrap_or(-1)
    }
}
