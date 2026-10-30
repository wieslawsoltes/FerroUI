//! The items panel of the strip of TabView in its default width mode: the sizing of the tabs of
//! `TabView::UpdateTabWidths` of microsoft-ui-xaml (`controls/dev/TabView/TabView.cpp`, `.h`,
//! `TabView.idl`, commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0; Copyright (c) Microsoft
//! Corporation, licensed under the MIT License, see NOTICE.md of the crate) for
//! `TabWidthMode="Equal"`, which gives every tab the width the strip has for its tabs divided by
//! their number, between `TabViewItemMinWidth` and `TabViewItemMaxWidth`; tabs that do not fit at
//! the minimum width overflow the strip, which scrolls them.
//!
//! WinUI computes the width in the code of TabView, when TabView is measured or its tabs change,
//! and sets it as the `Width` of every TabViewItem; the panel of its list view stacks them. FerroUI's
//! TabControl does not size its tabs, so the theme gives its strip this panel
//! (`Controls/TabControl.xaml`), which measures and arranges its children with the computed width.
//! It is a class of the theme's primitives as `InfoBarPanel` is; the limits are styled
//! properties, which the theme sets from the two resources.

use ferroui_base::input::{INavigableContainer, InputElement, InputElementImpl, NavigationDirection};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, FerroObject,
    FerroObjectExtensions, FerroObjectImpl, FerroProperty, Rect, Ref, Size, StyledElementImpl, StyledProperty,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::{Control, ControlImpl, Grid, ItemsControl, Panel, PanelImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The minimum width of a tab when the theme sets none (`c_tabMinimumWidth` of TabView, the
/// default of `TabViewItemMinWidth`).
const DEFAULT_MIN_ITEM_WIDTH: f64 = 48.0;

/// The maximum width of a tab when the theme sets none (`c_tabMaximumWidth` of TabView, the
/// default of `TabViewItemMaxWidth`).
const DEFAULT_MAX_ITEM_WIDTH: f64 = 200.0;

/// Lays out its children side by side, every one as wide as the others: the width the panel has
/// for them divided by their number, but not less than `MinItemWidth` and not more than
/// `MaxItemWidth` (TabView's `TabWidthMode="Equal"`). The children shrink with the width of the
/// panel down to the minimum; more children than fit at the minimum make the panel as wide as
/// they are, so that the scroll viewer around it scrolls them.
///
/// The width the panel has is the width it is measured with. In a scroll viewer that scrolls
/// horizontally, which measures its content with an infinite width, it is the width the viewport
/// offers the panel: the viewport less the padding of the scroll content presenter and less the
/// width the other content of the scroll viewer takes beside the panel (the header and the footer
/// of the strip, its padding), with the room the scroll buttons take while the content overflows
/// given back: TabView computes the width of its tabs from the width of its strip without the
/// scroll buttons, and hides them when the tabs fit. A new width of the viewport measures the
/// panel again. Without a width (an infinite one, or a viewport that has not been laid out yet)
/// every child has the maximum width, as in TabView. Hidden children are left out.
///
/// The children are arranged from the left; with `FlowDirection` right to left the framework
/// mirrors the panel, so that the first child is at the right, as the tabs of TabView are.
#[repr(C)]
pub struct TabViewEqualWidthPanel {
    base: Panel,
    /// The width of every child, computed by the last measure.
    item_width: Cell<f64>,
    /// The widths of the viewport the last measure used, when the panel was measured in a scroll
    /// viewer (and not with a width).
    viewport_widths: Cell<Option<ViewportWidths>>,
    /// The subscription to the viewport of the scroll content presenter around the panel.
    viewport_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

/// The widths the viewport of a scroll viewer offers the children of the panel.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ViewportWidths {
    /// Without the scroll buttons: the width the children share.
    available: f64,
    /// Beside the scroll buttons that are shown.
    current: f64,
}

impl ViewportWidths {
    fn same(&self, other: &ViewportWidths) -> bool {
        same_width(self.available, other.available) && same_width(self.current, other.current)
    }
}

ferro_class!(TabViewEqualWidthPanel: Panel);
ferro_class_info!(TabViewEqualWidthPanel { new: TabViewEqualWidthPanel::new });
ferro_impl_classes!(
    TabViewEqualWidthPanel: FerroObjectImpl,
    StyledElementImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

ferro_properties! { impl TabViewEqualWidthPanel {
    ferro_property!(
        /// Defines the `MinItemWidth` property.
        pub fn min_item_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<TabViewEqualWidthPanel, _>("MinItemWidth", DEFAULT_MIN_ITEM_WIDTH)
        }
    );

    ferro_property!(
        /// Defines the `MaxItemWidth` property.
        pub fn max_item_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<TabViewEqualWidthPanel, _>("MaxItemWidth", DEFAULT_MAX_ITEM_WIDTH)
        }
    );
} }

impl LayoutableImpl for TabViewEqualWidthPanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let children = this.children().snapshot();
        let visible: Vec<&Ref<Control>> = children.iter().filter(|child| child.is_visible()).collect();

        let viewport = if available_size.width.is_finite() { None } else { this.viewport_widths_for_items() };
        this.viewport_widths.set(viewport);
        let available_width = match viewport {
            Some(viewport) => viewport.available,
            None => available_size.width,
        };

        if visible.is_empty() {
            this.item_width.set(0.0);
            return Size::default();
        }

        // TabView::UpdateTabWidths: the width for the tabs divided by their number, clamped to
        // the limits (an infinite width gives the maximum).
        let count = visible.len() as f64;
        let item_width = this.clamp_item_width(available_width / count);
        this.item_width.set(item_width);

        let child_constraint = Size::new(item_width, available_size.height);
        let mut height = 0.0_f64;
        for child in &visible {
            child.measure(child_constraint);
            height = height.max(child.desired_size().height);
        }

        // Children that fill the width exactly report that width, not a width rounded up by a
        // floating point error (which would make the strip scroll by a pixel).
        let mut width = item_width * count;
        if available_width.is_finite() && (width - available_width).abs() < 1e-6 {
            width = available_width;
        }

        // Children that fit the viewport without the scroll buttons but not beside the buttons
        // that are still shown (the content overflowed before): the panel asks for no more than
        // the viewport, so that the scroll viewer stops scrolling and hides its buttons, and the
        // next measure, in the same layout pass, has the whole viewport for the children (TabView
        // hides the scroll buttons when its tabs fit the width of its strip).
        if let Some(viewport) = viewport {
            if width <= viewport.available + 1e-6 && width > viewport.current + 1e-6 {
                width = viewport.current;
            }
        }

        Size::new(width, height)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        // The widths of the viewport the last measure used came from the desired sizes of the
        // previous layout pass (the panel and its siblings); when they have changed since (a new
        // panel, new content beside it), the panel is measured again with the current ones.
        if let Some(used) = this.viewport_widths.get() {
            if this.viewport_widths_for_items().is_some_and(|current| !current.same(&used)) {
                this.invalidate_measure();
            }
        }

        let item_width = this.item_width.get();
        let mut x = 0.0;
        for child in this.children().snapshot().iter() {
            if !child.is_visible() {
                continue;
            }
            let height = final_size.height.max(child.desired_size().height);
            child.arrange(Rect::new(x, 0.0, item_width, height));
            x += item_width;
        }

        final_size
    }
}

impl VisualImpl for TabViewEqualWidthPanel {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.subscribe_to_viewport();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        if let Some(subscription) = this.viewport_subscription.borrow_mut().take() {
            subscription.dispose();
        }
    }
}

impl INavigableContainer for TabViewEqualWidthPanel {
    fn get_control(
        &self,
        direction: NavigationDirection,
        from: Option<&Ref<InputElement>>,
        wrap: bool,
    ) -> Option<Ref<InputElement>> {
        let from = from.and_then(|from| from.cast::<Control>());
        let mut result = self.control_in_direction(direction, from.as_ref());

        if result.is_none() && wrap {
            use NavigationDirection::*;
            match direction {
                Left | Previous | PageUp => result = self.control_in_direction(Last, None),
                Right | Next | PageDown => result = self.control_in_direction(First, None),
                _ => {}
            }
        }

        result
    }
}

impl TabViewEqualWidthPanel {
    fn static_constructor() {
        Layoutable::affects_measure::<TabViewEqualWidthPanel>(&[
            Self::min_item_width_property().as_property(),
            Self::max_item_width_property().as_property(),
        ]);
        ferroui_controls::register_navigable_container::<TabViewEqualWidthPanel>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            item_width: Cell::new(0.0),
            viewport_widths: Cell::new(None),
            viewport_subscription: RefCell::new(None),
        }
    }

    /// Creates a panel of tabs of equal width.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The minimum width of a child (`TabViewItemMinWidth`).
    pub fn min_item_width(&self) -> f64 {
        self.get_value(Self::min_item_width_property())
    }

    pub fn set_min_item_width(&self, value: f64) {
        self.set_value(Self::min_item_width_property(), value)
    }

    /// The maximum width of a child (`TabViewItemMaxWidth`).
    pub fn max_item_width(&self) -> f64 {
        self.get_value(Self::max_item_width_property())
    }

    pub fn set_max_item_width(&self, value: f64) {
        self.set_value(Self::max_item_width_property(), value)
    }

    /// The width of every child, as the last measure computed it.
    pub fn item_width(&self) -> f64 {
        self.item_width.get()
    }

    /// `std::clamp(width, minTabWidth, maxTabWidth)` of TabView, the minimum winning over a
    /// smaller maximum.
    fn clamp_item_width(&self, width: f64) -> f64 {
        width.min(self.max_item_width()).max(self.min_item_width().max(0.0))
    }

    /// The scroll content presenter of the scroll viewer around the panel that scrolls it
    /// horizontally: the first one among the ancestors of the panel up to the items control that
    /// owns it.
    fn scroll_content_presenter(&self) -> Option<Ref<ScrollContentPresenter>> {
        for ancestor in self.get_visual_ancestors() {
            if let Some(presenter) = ancestor.cast::<ScrollContentPresenter>() {
                return presenter.can_horizontally_scroll().then_some(presenter);
            }
            if ancestor.cast::<ItemsControl>().is_some() {
                break;
            }
        }
        None
    }

    /// The widths the viewport of the scroll viewer around the panel offers its children, or
    /// `None` without such a scroll viewer or before its viewport has been laid out.
    fn viewport_widths_for_items(&self) -> Option<ViewportWidths> {
        let presenter = self.scroll_content_presenter()?;
        let viewport = presenter.viewport().width;
        if viewport <= 0.0 {
            return None;
        }

        // The content of the scroll viewer less the panel: the header and the footer of the strip
        // and the margins around the panel, from their desired sizes.
        let content = presenter.child()?;
        let beside = (content.desired_size().width - self.desired_size().width).max(0.0);
        let inset = presenter.padding() + presenter.border_thickness();
        let margin = self.margin();
        let current = (viewport - inset.left - inset.right - beside - margin.left - margin.right).max(0.0);

        Some(ViewportWidths { available: current + Self::scroll_button_room(&presenter), current })
    }

    /// The room the scroll buttons beside the scroll content presenter take while the content
    /// overflows: in the grid of the template of the scroll viewer, the width of the auto columns
    /// beside the presenter beyond their minimum width (the columns of `ScrollDecreaseButton` and
    /// `ScrollIncreaseButton`, as wide as their minimum while the buttons are hidden).
    fn scroll_button_room(presenter: &ScrollContentPresenter) -> f64 {
        let Some(grid) = presenter.visual_parent().and_then(|parent| parent.cast::<Grid>()) else {
            return 0.0;
        };
        let columns = grid.column_definitions();
        let count = columns.count();
        if count == 0 {
            return 0.0;
        }
        let first = (Grid::get_column(presenter).max(0) as usize).min(count - 1);
        let last = (first + Grid::get_column_span(presenter).max(1) as usize).min(count);

        (0..count)
            .filter(|index| *index < first || *index >= last)
            .map(|index| columns.get(index))
            .filter(|column| column.width().is_auto())
            .map(|column| (column.actual_width() - column.min_width()).max(0.0))
            .sum()
    }

    /// Measures the panel again when the viewport of the scroll viewer around it changes to
    /// widths that give its children another width or the panel another desired width.
    fn subscribe_to_viewport(&self) {
        if let Some(subscription) = self.viewport_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        let Some(presenter) = self.scroll_content_presenter() else {
            return;
        };

        let weak = self.to_ref().downgrade();
        let object: &FerroObject = &presenter;
        let subscription = FerroObjectExtensions::get_observable(object, ScrollContentPresenter::viewport_property())
            .subscribe_fn(move |_| {
                let Some(this) = weak.upgrade() else { return };
                let changed = match (this.viewport_widths.get(), this.viewport_widths_for_items()) {
                    (Some(used), Some(current)) => !used.same(&current),
                    (None, None) => false,
                    _ => true,
                };
                if changed {
                    this.invalidate_measure();
                }
            });
        *self.viewport_subscription.borrow_mut() = Some(subscription);
    }

    /// The child in `direction` from `from` (the children are in one row).
    fn control_in_direction(&self, direction: NavigationDirection, from: Option<&Ref<Control>>) -> Option<Ref<InputElement>> {
        let children = self.children();
        let mut index: i64 = from.and_then(|from| children.index_of(from)).map_or(-1, |index| index as i64);

        match direction {
            NavigationDirection::First => index = 0,
            NavigationDirection::Last => index = children.count() as i64 - 1,
            NavigationDirection::Next => index += 1,
            NavigationDirection::Previous | NavigationDirection::Left => {
                if index != -1 {
                    index -= 1;
                }
            }
            NavigationDirection::Right => {
                if index != -1 {
                    index += 1;
                }
            }
            _ => index = -1,
        }

        if index >= 0 && (index as usize) < children.count() {
            Some(children.get(index as usize).upcast())
        } else {
            None
        }
    }
}

/// Whether two widths are the same up to a floating point error.
fn same_width(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() < 1e-6
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::support::start_themed_application;
    use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
    use ferroui_base::media::FlowDirection;
    use ferroui_base::Point;
    use ferroui_controls::primitives::ScrollBarVisibility;
    use ferroui_controls::testing::MockWindowingPlatform;
    use ferroui_controls::{Border, Dock, DockPanel, ScrollViewer, Window};

    fn tab(width: f64, height: f64) -> Ref<Border> {
        let border = Border::new();
        // The content of a tab, which the width of the panel stretches or narrows.
        let content = Border::new();
        content.set_width(width);
        content.set_height(height);
        content.set_horizontal_alignment(HorizontalAlignment::Left);
        border.set_child(&content);
        border
    }

    /// The panel of the theme: the limits of TabViewItemMinWidth (100) and TabViewItemMaxWidth
    /// (240) of TabView_themeresources.xaml.
    fn panel(tabs: &[Ref<Border>]) -> Ref<TabViewEqualWidthPanel> {
        let panel = TabViewEqualWidthPanel::new();
        panel.set_min_item_width(100.0);
        panel.set_max_item_width(240.0);
        for tab in tabs {
            panel.children().add(tab.clone());
        }
        panel
    }

    fn tabs(count: usize) -> Vec<Ref<Border>> {
        (0..count).map(|index| tab(40.0 + 30.0 * index as f64, 32.0)).collect()
    }

    #[test]
    fn every_child_has_the_same_width() {
        let tabs = tabs(3);
        let panel = panel(&tabs);

        panel.measure(Size::new(600.0, 40.0));
        assert_eq!(Size::new(600.0, 32.0), panel.desired_size());
        assert_eq!(200.0, panel.item_width());
        panel.arrange(Rect::new(0.0, 0.0, 600.0, 40.0));
        assert_eq!(Rect::new(0.0, 0.0, 200.0, 40.0), tabs[0].bounds());
        assert_eq!(Rect::new(200.0, 0.0, 200.0, 40.0), tabs[1].bounds());
        assert_eq!(Rect::new(400.0, 0.0, 200.0, 40.0), tabs[2].bounds());
    }

    #[test]
    fn the_width_is_not_more_than_the_maximum() {
        let tabs = tabs(2);
        let panel = panel(&tabs);

        panel.measure(Size::new(1000.0, 40.0));
        assert_eq!(Size::new(480.0, 32.0), panel.desired_size());
        panel.arrange(Rect::new(0.0, 0.0, 1000.0, 40.0));
        assert_eq!(Rect::new(0.0, 0.0, 240.0, 40.0), tabs[0].bounds());
        assert_eq!(Rect::new(240.0, 0.0, 240.0, 40.0), tabs[1].bounds());
    }

    #[test]
    fn an_infinite_width_gives_the_maximum() {
        let tabs = tabs(3);
        let panel = panel(&tabs);

        panel.measure(Size::new(f64::INFINITY, f64::INFINITY));
        assert_eq!(Size::new(720.0, 32.0), panel.desired_size());
        assert_eq!(240.0, panel.item_width());
    }

    #[test]
    fn the_children_shrink_with_the_panel_down_to_the_minimum() {
        let tabs = tabs(4);
        let panel = panel(&tabs);

        panel.measure(Size::new(500.0, 40.0));
        assert_eq!(125.0, panel.item_width());
        panel.measure(Size::new(420.0, 40.0));
        assert_eq!(105.0, panel.item_width());
        assert_eq!(Size::new(420.0, 32.0), panel.desired_size());
        panel.arrange(Rect::new(0.0, 0.0, 420.0, 40.0));
        assert_eq!(Rect::new(315.0, 0.0, 105.0, 40.0), tabs[3].bounds());
    }

    #[test]
    fn children_that_do_not_fit_at_the_minimum_overflow() {
        let tabs = tabs(5);
        let panel = panel(&tabs);

        // The panel asks for the width of its children at the minimum width; measured with a
        // finite width, the framework constrains its desired size to that width (a scroll viewer
        // measures it with an infinite width, which keeps the full width).
        panel.measure(Size::new(300.0, 40.0));
        assert_eq!(100.0, panel.item_width());
        assert_eq!(Size::new(300.0, 32.0), panel.desired_size());
        panel.arrange(Rect::new(0.0, 0.0, 500.0, 40.0));
        assert_eq!(Rect::new(400.0, 0.0, 100.0, 40.0), tabs[4].bounds());
    }

    #[test]
    fn the_minimum_wins_over_a_smaller_maximum() {
        let tabs = tabs(2);
        let panel = panel(&tabs);
        panel.set_max_item_width(80.0);

        panel.measure(Size::new(1000.0, 40.0));
        assert_eq!(100.0, panel.item_width());
    }

    #[test]
    fn hidden_children_are_left_out() {
        let tabs = tabs(3);
        tabs[1].set_is_visible(false);
        let panel = panel(&tabs);

        panel.measure(Size::new(400.0, 40.0));
        assert_eq!(200.0, panel.item_width());
        assert_eq!(Size::new(400.0, 32.0), panel.desired_size());
        panel.arrange(Rect::new(0.0, 0.0, 400.0, 40.0));
        assert_eq!(Rect::new(0.0, 0.0, 200.0, 40.0), tabs[0].bounds());
        assert_eq!(Rect::new(200.0, 0.0, 200.0, 40.0), tabs[2].bounds());
    }

    #[test]
    fn a_new_limit_measures_the_panel_again() {
        let tabs = tabs(2);
        let panel = panel(&tabs);
        panel.measure(Size::new(1000.0, 40.0));
        assert!(panel.is_measure_valid());

        panel.set_max_item_width(300.0);
        assert!(!panel.is_measure_valid());
        panel.measure(Size::new(1000.0, 40.0));
        assert_eq!(300.0, panel.item_width());
    }

    /// A window with `content` at the top left, in a frame of the given width.
    fn window_with(content: &Ref<Control>, width: f64) -> (Ref<Window>, Ref<Border>) {
        let frame = Border::new();
        frame.set_width(width);
        frame.set_horizontal_alignment(HorizontalAlignment::Left);
        frame.set_vertical_alignment(VerticalAlignment::Top);
        frame.set_child(content.clone());
        let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(1000.0, 400.0));
        window.set_content(Some(Control::boxed(&frame)));
        window.show();
        window.update_layout();
        (window, frame)
    }

    /// The strip of the theme: a scroll viewer that scrolls horizontally around a dock panel of a
    /// 4 px header, the panel and the footer that fills the rest.
    fn strip(panel: &Ref<TabViewEqualWidthPanel>) -> Ref<ScrollViewer> {
        let header = Border::new();
        header.set_width(4.0);
        DockPanel::set_dock(&header, Dock::Left);
        DockPanel::set_dock(panel, Dock::Left);
        let footer = Border::new();
        footer.set_min_width(4.0);
        let content = DockPanel::new();
        content.children().add(header);
        content.children().add(panel.clone());
        content.children().add(footer);

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroll_viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        scroll_viewer.set_content(Some(Control::boxed(&content)));
        scroll_viewer
    }

    #[test]
    fn in_a_scroll_viewer_the_children_share_its_viewport() {
        let _app = start_themed_application();
        let tabs = tabs(4);
        let panel = panel(&tabs);
        let scroll_viewer = strip(&panel);
        let (window, frame) = window_with(&scroll_viewer.clone().upcast(), 508.0);

        // The viewport less the header and the footer, 4 each.
        assert_eq!(508.0, scroll_viewer.viewport().width);
        assert_eq!(125.0, panel.item_width());
        assert_eq!(508.0, scroll_viewer.extent().width);
        for (index, tab) in tabs.iter().enumerate() {
            assert_eq!(125.0 * index as f64, tab.bounds().x);
            assert_eq!(125.0, tab.bounds().width);
        }

        // A narrower viewport narrows the children.
        frame.set_width(448.0);
        window.update_layout();
        assert_eq!(110.0, panel.item_width());
        assert_eq!(448.0, scroll_viewer.extent().width);

        // A viewport too narrow for the minimum width scrolls the children.
        frame.set_width(300.0);
        window.update_layout();
        assert_eq!(100.0, panel.item_width());
        assert_eq!(300.0, scroll_viewer.viewport().width);
        assert_eq!(408.0, scroll_viewer.extent().width);

        // A wider one again shares its width, up to the maximum.
        frame.set_width(808.0);
        window.update_layout();
        assert_eq!(200.0, panel.item_width());
        frame.set_width(1000.0);
        window.update_layout();
        assert_eq!(240.0, panel.item_width());
        assert_eq!(1000.0, scroll_viewer.viewport().width);

        // Another tab shares the same width.
        frame.set_width(508.0);
        window.update_layout();
        panel.children().add(tab(50.0, 32.0));
        window.update_layout();
        assert_eq!(100.0, panel.item_width());
        window.close();
    }

    #[test]
    fn right_to_left_the_first_child_is_at_the_right() {
        let _app = start_themed_application();
        let tabs = tabs(2);
        let panel = panel(&tabs);
        let host = Border::new();
        host.set_flow_direction(FlowDirection::RightToLeft);
        host.set_child(&panel);
        let (window, frame) = window_with(&host.clone().upcast(), 400.0);

        assert_eq!(200.0, panel.item_width());
        // In the panel the first child is at the left; the framework mirrors the panel.
        assert_eq!(Rect::new(0.0, 0.0, 200.0, 32.0), tabs[0].bounds());
        let origin = |tab: &Ref<Border>| tab.translate_point(Point::default(), &frame).expect("in the frame");
        assert_eq!(Point::new(400.0, 0.0), origin(&tabs[0]));
        assert_eq!(Point::new(200.0, 0.0), origin(&tabs[1]));
        window.close();
    }
}
