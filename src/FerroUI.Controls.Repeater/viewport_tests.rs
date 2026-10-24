// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.
// Behavioral ports of ViewportTests, EffectiveViewportScrollViewerTests and
// EffectiveViewportScrollPresenterTests. Native ScrollContentPresenter and
// LayoutTransformControl provide scrolling and zoom geometry in FerroUI.

use super::*;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::primitives::{Popup, ScrollBarVisibility};
use ferroui_controls::templates::{FuncControlTemplate, FuncDataTemplate};
use crate::test_support::{test_scope, TestRoot};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{
    Border, Control, Grid, ItemsSource, LayoutTransformControl, ScrollViewer, TextBlock, Window,
};
use ferroui_base::data::{BindingPriority, ReflectionBinding};
use ferroui_base::layout::{HorizontalAlignment, ILayoutManager, VerticalAlignment};
use ferroui_base::media::{ScaleTransform, TextWrapping};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObject, FerroObjectExtensions,
    FerroObjectImpl, Point, Rect, Ref, Size, StyledElement, Thickness, Vector,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
struct ViewportProbe {
    base: VirtualizingLayout,
    extent: Cell<Size>,
    measures: RefCell<Vec<Rect>>,
    arranges: RefCell<Vec<Rect>>,
    visible: RefCell<Vec<Rect>>,
}
ferro_class!(ViewportProbe: VirtualizingLayout);
ferro_impl_classes!(ViewportProbe: FerroObjectImpl, LayoutImpl);
impl VirtualizingLayoutImpl for ViewportProbe {
    fn measure_override(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        _available: Size,
    ) -> Size {
        this.measures.borrow_mut().push(context.realization_rect());
        this.visible.borrow_mut().push(context.visible_rect());
        this.extent.get()
    }
    fn arrange_override(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        final_size: Size,
    ) -> Size {
        this.arranges.borrow_mut().push(context.realization_rect());
        final_size
    }
}
impl ViewportProbe {
    fn new(extent: Size) -> Ref<Self> {
        instantiate(Self {
            base: VirtualizingLayout::construct(),
            extent: Cell::new(extent),
            measures: RefCell::new(Vec::new()),
            arranges: RefCell::new(Vec::new()),
            visible: RefCell::new(Vec::new()),
        })
    }
    fn last(&self) -> Rect {
        *self
            .measures
            .borrow()
            .last()
            .expect("The layout was measured.")
    }
}

fn probe(extent: Size) -> (Ref<ItemsRepeater>, Ref<ViewportProbe>) {
    let layout = ViewportProbe::new(extent);
    let repeater = ItemsRepeater::new();
    repeater.set_layout(Some(layout.clone().upcast()));
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    (repeater, layout)
}

fn viewer(content: &Ref<Control>, size: Option<Size>) -> Ref<ScrollViewer> {
    let viewer = ScrollViewer::new();
    viewer.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(
        crate::test_support::create_template,
    )));
    viewer.set_content(Some(Control::boxed(content)));
    viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    if let Some(size) = size {
        viewer.set_width(size.width);
        viewer.set_height(size.height);
    }
    viewer
}

fn settle(root: &TestRoot) {
    // Native cache growth queues background measure work; no wall-clock waits.
    for _ in 0..16 {
        root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(None);
    }
}

fn assert_close(actual: Rect, expected: Rect) {
    for (actual, expected) in [
        (actual.x, expected.x),
        (actual.y, expected.y),
        (actual.width, expected.width),
        (actual.height, expected.height),
    ] {
        assert!(
            (actual - expected).abs() < 0.01,
            "expected {expected}, observed {actual}"
        );
    }
}

#[test]
fn validate_no_scrolling_surface_scenario() {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(500.0, 500.0));
    let root = TestRoot::with_child(&repeater);
    root.set_client_size(Size::new(800.0, 600.0));
    // TestRoot measures its child against ClientSize but otherwise sizes to
    // content. Give it the fixed bounds of the upstream test window too.
    root.set_width(800.0);
    root.set_height(600.0);
    root.execute_initial_layout_pass();
    assert_eq!(layout.measures.borrow()[0], Rect::default());
    assert_close(layout.last(), Rect::new(0.0, 0.0, 800.0, 600.0));
    assert_eq!(*layout.measures.borrow(), *layout.visible.borrow());
    repeater.set_vertical_cache_length(10.0);
    settle(&root);
    assert_close(layout.last(), Rect::new(0.0, 0.0, 800.0, 600.0));
}

#[test]
fn validate_no_scrolling_surface_in_popup_scenario() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (repeater, layout) = probe(Size::new(200.0, 150.0));
    let target = Border::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&target)));
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    let popup = Popup::new();
    popup.set_child(&repeater);
    popup.set_placement_target(Some(target.upcast()));
    popup.set_is_open(true);
    Dispatcher::ui_thread().run_jobs(None);
    let host = popup.host().expect("The native popup opened.").as_control();
    let layout_root = host
        .get_layout_root()
        .expect("The popup has a layout root.");
    layout_root.layout_manager().execute_initial_layout_pass();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!layout.measures.borrow().is_empty());
    let viewport = layout.last();
    assert!(viewport.width.is_finite() && viewport.height.is_finite());
    assert!(viewport.width > 0.0 && viewport.height > 0.0);
    assert_eq!(*layout.measures.borrow(), *layout.visible.borrow());
    popup.set_is_open(false);
    assert!(!repeater.is_attached_to_visual_tree());
    window.close();
}

fn single_viewer(with_host: bool) {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(500.0, 600.0));
    let scroll = viewer(&repeater.clone().upcast(), Some(Size::new(200.0, 300.0)));
    let host = with_host.then(|| {
        let host = ItemsRepeaterScrollHost::new();
        host.set_scroll_viewer(Some(scroll.clone()));
        host
    });
    let root = TestRoot::with_child(
        host.as_ref()
            .map(|host| host.clone().upcast::<Control>())
            .unwrap_or_else(|| scroll.clone().upcast()),
    );
    root.execute_initial_layout_pass();
    assert_eq!(layout.measures.borrow()[0], Rect::default());
    assert_close(layout.last(), Rect::new(0.0, 0.0, 200.0, 300.0));
    scroll.set_offset(Vector::new(0.0, 100.0));
    settle(&root);
    assert_close(layout.last(), Rect::new(0.0, 100.0, 200.0, 300.0));
    scroll.set_offset(Vector::new(400.0, 100.0));
    settle(&root);
    assert_eq!(scroll.offset(), Vector::new(300.0, 100.0));
    assert_close(layout.last(), Rect::new(300.0, 100.0, 200.0, 300.0));
    assert_eq!(*layout.measures.borrow(), *layout.visible.borrow());
}

#[test]
fn validate_basic_scroll_viewer_scenario() {
    single_viewer(false);
}

#[test]
fn validate_items_repeater_scroll_host_scenario() {
    single_viewer(true);
}

#[test]
fn validate_one_scroll_presenter_scenario() {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(500.0, 600.0));
    let presenter = ScrollContentPresenter::new();
    presenter.set_can_horizontally_scroll(true);
    presenter.set_can_vertically_scroll(true);
    presenter.set_content(Some(Control::boxed(&repeater)));
    presenter.set_width(200.0);
    presenter.set_height(300.0);
    let root = TestRoot::with_child(&presenter);
    root.execute_initial_layout_pass();
    assert_eq!(layout.measures.borrow()[0], Rect::default());
    assert_close(layout.last(), Rect::new(0.0, 0.0, 200.0, 300.0));
    presenter.set_offset(Vector::new(0.0, 100.0));
    settle(&root);
    assert_close(layout.last(), Rect::new(0.0, 100.0, 200.0, 300.0));
    presenter.set_offset(Vector::new(400.0, 100.0));
    settle(&root);
    assert_eq!(presenter.offset(), Vector::new(300.0, 100.0));
    assert_close(layout.last(), Rect::new(300.0, 100.0, 200.0, 300.0));
}

fn two_scrollers(presenters: bool) {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(500.0, 500.0));
    if presenters {
        let horizontal = ScrollContentPresenter::new();
        horizontal.set_can_horizontally_scroll(true);
        horizontal.set_can_vertically_scroll(false);
        horizontal.set_content(Some(Control::boxed(&repeater)));
        let grid = Grid::new();
        grid.children().add(&horizontal);
        let vertical = ScrollContentPresenter::new();
        vertical.set_can_horizontally_scroll(false);
        vertical.set_can_vertically_scroll(true);
        vertical.set_content(Some(Control::boxed(&grid)));
        vertical.set_width(200.0);
        vertical.set_height(200.0);
        let root = TestRoot::with_child(&vertical);
        root.execute_initial_layout_pass();
        assert_close(layout.last(), Rect::new(0.0, 0.0, 200.0, 200.0));
        vertical.set_offset(Vector::new(0.0, 100.0));
        settle(&root);
        assert_close(layout.last(), Rect::new(0.0, 100.0, 200.0, 200.0));
        horizontal.set_offset(Vector::new(400.0, 0.0));
        settle(&root);
        assert_close(layout.last(), Rect::new(300.0, 100.0, 200.0, 200.0));
    } else {
        let horizontal = viewer(&repeater.clone().upcast(), None);
        horizontal.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        let vertical = viewer(&horizontal.clone().upcast(), Some(Size::new(200.0, 200.0)));
        vertical.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        let root = TestRoot::with_child(&vertical);
        root.execute_initial_layout_pass();
        assert_close(layout.last(), Rect::new(0.0, 0.0, 200.0, 200.0));
        vertical.set_offset(Vector::new(0.0, 100.0));
        settle(&root);
        assert_close(layout.last(), Rect::new(0.0, 100.0, 200.0, 200.0));
        horizontal.set_offset(Vector::new(400.0, 0.0));
        settle(&root);
        assert_close(layout.last(), Rect::new(300.0, 100.0, 200.0, 200.0));
    }
}

#[test]
fn validate_two_scroll_viewer_scenario() {
    two_scrollers(false);
}

#[test]
fn validate_two_scroll_presenters_scenario() {
    two_scrollers(true);
}

#[test]
fn scale_transform_supplies_the_zoomed_effective_viewport() {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(500.0, 600.0));
    let zoom = LayoutTransformControl::new();
    zoom.set_child(&repeater);
    let scale = ScaleTransform::with_scale(1.0, 1.0);
    zoom.set_layout_transform(Some(scale.clone().into()));
    let scroll = viewer(&zoom.clone().upcast(), Some(Size::new(200.0, 300.0)));
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    scroll.set_offset(Vector::new(300.0, 100.0));
    settle(&root);
    assert_close(layout.last(), Rect::new(300.0, 100.0, 200.0, 300.0));
    scale.set_scale_x(2.0);
    scale.set_scale_y(2.0);
    settle(&root);
    assert_close(layout.last(), Rect::new(150.0, 50.0, 100.0, 150.0));
    assert_eq!(scroll.extent(), Size::new(1000.0, 1200.0));
    scale.set_scale_x(1.0);
    scale.set_scale_y(1.0);
    settle(&root);
    assert_close(layout.last(), Rect::new(300.0, 100.0, 200.0, 300.0));
}

#[test]
fn relative_render_transform_origin_refreshes_when_the_repeater_resizes() {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(500.0, 600.0));
    repeater.set_render_transform_origin(ferroui_base::RelativePoint::CENTER);
    repeater.set_render_transform(Some(ScaleTransform::with_scale(2.0, 2.0).into()));
    let scroll = viewer(&repeater.clone().upcast(), Some(Size::new(200.0, 300.0)));
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    settle(&root);
    assert_eq!(repeater.bounds().size(), Size::new(500.0, 600.0));
    assert_close(layout.last(), Rect::new(125.0, 150.0, 100.0, 150.0));

    // The scroller still exposes the same viewport at the same offset. The
    // new bounds move the center about which the repeater is scaled.
    layout.extent.set(Size::new(700.0, 1000.0));
    layout.invalidate_measure();
    settle(&root);
    assert_eq!(scroll.offset(), Vector::default());
    assert_eq!(scroll.viewport(), Size::new(200.0, 300.0));
    assert_eq!(repeater.bounds().size(), Size::new(700.0, 1000.0));
    assert_close(layout.last(), Rect::new(175.0, 250.0, 100.0, 150.0));
}

fn cache_growth(presenter: bool) {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(1000.0, 2000.0));
    repeater.set_horizontal_cache_length(2.0);
    repeater.set_vertical_cache_length(2.0);
    let content = if presenter {
        let presenter = ScrollContentPresenter::new();
        presenter.set_can_horizontally_scroll(true);
        presenter.set_can_vertically_scroll(true);
        presenter.set_width(400.0);
        presenter.set_height(400.0);
        presenter.set_content(Some(Control::boxed(&repeater)));
        presenter.upcast::<Control>()
    } else {
        viewer(&repeater.clone().upcast(), Some(Size::new(400.0, 400.0))).upcast()
    };
    let root = TestRoot::with_child(content);
    root.execute_initial_layout_pass();
    settle(&root);
    let expected = Rect::new(-400.0, -400.0, 1200.0, 1200.0);
    assert_close(layout.last(), expected);
    assert_close(*layout.arranges.borrow().last().unwrap(), expected);
    for windows in [&layout.measures, &layout.arranges] {
        let mut windows = windows.borrow().clone();
        windows.dedup();
        let windows: Vec<_> = windows
            .into_iter()
            .filter(|rect| rect.width > 0.0)
            .collect();
        for pair in windows.windows(2) {
            assert_eq!(pair[1].x - pair[0].x, -40.0);
            assert_eq!(pair[1].y - pair[0].y, -40.0);
            assert_eq!(pair[1].width - pair[0].width, 80.0);
            assert_eq!(pair[1].height - pair[0].height, 80.0);
        }
    }
}

#[test]
fn can_grow_cache_buffer_with_scroll_viewer() {
    cache_growth(false);
}

#[test]
fn can_grow_cache_buffer_with_scroll_presenter() {
    cache_growth(true);
}

fn aligned_destination(vertical_scroll: bool) {
    let _scope = test_scope();
    for alignment in 0..3 {
        let (repeater, layout) = probe(if vertical_scroll {
            Size::new(100.0, 3000.0)
        } else {
            Size::new(3000.0, 100.0)
        });
        if vertical_scroll {
            repeater.set_horizontal_alignment(
                [
                    HorizontalAlignment::Left,
                    HorizontalAlignment::Center,
                    HorizontalAlignment::Right,
                ][alignment],
            );
        } else {
            repeater.set_vertical_alignment(
                [
                    VerticalAlignment::Top,
                    VerticalAlignment::Center,
                    VerticalAlignment::Bottom,
                ][alignment],
            );
        }
        let scroll = viewer(&repeater.clone().upcast(), Some(Size::new(400.0, 300.0)));
        let root = TestRoot::with_child(&scroll);
        root.execute_initial_layout_pass();
        scroll.set_offset(if vertical_scroll {
            Vector::new(0.0, 1000.0)
        } else {
            Vector::new(1000.0, 0.0)
        });
        // FerroUI commits native offsets synchronously; its next layout has
        // the destination window before the frame is rendered.
        root.layout_manager().execute_layout_pass();
        let expected = if vertical_scroll {
            Rect::new(-(alignment as f64) * 150.0, 1000.0, 400.0, 300.0)
        } else {
            Rect::new(1000.0, -(alignment as f64) * 100.0, 400.0, 300.0)
        };
        assert_close(layout.last(), expected);
        repeater.invalidate_measure();
        settle(&root);
        assert_close(layout.last(), expected);
        root.set_child(None);
    }
}

#[test]
fn validate_scroll_starting_anticipation_for_horizontally_aligned_repeater() {
    aligned_destination(true);
}

#[test]
fn validate_scroll_starting_anticipation_for_vertically_aligned_repeater() {
    aligned_destination(false);
}

#[test]
fn can_bring_into_view_elements_with_native_scroll_viewer() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let item = Border::new();
            item.set_height(100.0);
            Some(item.upcast())
        },
        true,
    )));
    repeater.set_items_source(Some(ItemsSource::from_values(0..400)));
    let scroll = viewer(&repeater.clone().upcast(), Some(Size::new(400.0, 600.0)));
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    for index in [100, 103, 399, 0] {
        let element = repeater.get_or_create_element(index);
        element.bring_into_view();
        settle(&root);
        let position = element.translate_point(Point::default(), &scroll).unwrap();
        assert!(
            position.y >= -0.01 && position.y + element.bounds().height <= 600.01,
            "item {index} at {position:?}, scroll {:?}",
            scroll.offset()
        );
        assert_eq!(repeater.get_element_index(&element), index);
        assert!(repeater.viewport_manager.made_anchor().is_none());
        assert!(repeater.realized_elements().len() <= 10);
    }
}

#[test]
fn verify_uniform_grid_layout_doesnt_crash_when_trying_to_scroll_to_end() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    let grid = UniformGridLayout::new();
    grid.set_min_item_width(100.0);
    grid.set_min_item_height(40.0);
    grid.set_min_row_spacing(10.0);
    grid.set_min_column_spacing(10.0);
    repeater.set_layout(Some(grid.upcast()));
    repeater.set_items_source(Some(ItemsSource::from_items((0..1000).map(|index| {
        let text = TextBlock::new();
        text.set_text(Some(&format!("#{index}")));
        let border = Border::new();
        border.set_child(&text);
        Some(Control::boxed(border))
    }))));
    let scroll = viewer(&repeater.clone().upcast(), Some(Size::new(400.0, 600.0)));
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    settle(&root);
    scroll.set_offset(Vector::new(0.0, repeater.bounds().height));
    settle(&root);
    assert!(repeater.try_get_element(999).is_some());
    scroll.set_offset(Vector::default());
    settle(&root);
    assert!(repeater.try_get_element(0).is_some());
    assert_eq!(scroll.offset(), Vector::default());
}

#[test]
fn verify_uniform_grid_layout_doesnt_hang_when_trying_to_scroll_to_start() {
    let _scope = test_scope();
    let outer = ItemsRepeater::new();
    outer.set_items_source(Some(ItemsSource::from_values(
        (0..10).map(|_| ItemsSource::from_values(1..7)),
    )));
    outer.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let inner = ItemsRepeater::new();
            let grid = UniformGridLayout::new();
            grid.set_orientation(ferroui_base::layout::Orientation::Horizontal);
            grid.set_min_item_width(150.0);
            grid.set_min_item_height(150.0);
            grid.set_maximum_rows_or_columns(3);
            grid.set_min_row_spacing(4.0);
            grid.set_min_column_spacing(4.0);
            grid.set_items_stretch(UniformGridLayoutItemsStretch::Fill);
            inner.set_layout(Some(grid.upcast()));
            let object: &FerroObject = &inner;
            inner.bind(
                ItemsRepeater::items_source_property(),
                FerroObjectExtensions::get_observable_with(
                    object,
                    StyledElement::data_context_property(),
                    |data: Option<BoxedValue>| {
                        data.and_then(|data| data.downcast_ref::<ItemsSource>().cloned())
                    },
                ),
                BindingPriority::LocalValue,
            );
            Some(inner.upcast())
        },
        true,
    )));
    let scroll = viewer(&outer.clone().upcast(), Some(Size::new(500.0, 400.0)));
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    settle(&root);
    for step in (1..=16).chain((0..16).rev()) {
        let end = (scroll.extent().height - scroll.viewport().height).max(0.0);
        scroll.set_offset(Vector::new(0.0, end * step as f64 / 16.0));
        settle(&root);
        assert!(scroll.offset().y.is_finite());
        assert!(!outer.realized_elements().is_empty());
    }
    assert_eq!(scroll.offset().y, 0.0);
    let first = outer
        .try_get_element(0)
        .unwrap()
        .cast::<ItemsRepeater>()
        .unwrap();
    assert!(first.try_get_element(0).is_some());
}

#[test]
fn can_bring_into_view_elements_with_alignment_and_animation() {
    let _scope = test_scope();
    let clock = super::services_tests::TestClock::install();
    let repeater = ItemsRepeater::new();
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    let lorem = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam laoreet erat vel massa rutrum, eget mollis massa vulputate. Vivamus semper augue leo, eget faucibus nulla mattis nec. Donec scelerisque lacus at dui ultricies, eget auctor ipsum placerat. Integer aliquet libero sed nisi eleifend, nec rutrum arcu lacinia.";
    repeater.set_items_source(Some(ItemsSource::from_values(
        (0..400).map(|index| format!("{index}: {}", &lorem[..250])),
    )));
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let border = Border::new();
            border.set_height(94.0);
            border.set_margin(Thickness::uniform(5.0));
            let text = TextBlock::new();
            text.set_text_wrapping(TextWrapping::WrapWithOverflow);
            text.bind_binding(
                TextBlock::text_property().as_property(),
                &*ReflectionBinding::empty(),
            );
            border.set_child(&text);
            Some(border.upcast())
        },
        true,
    )));
    let scroll = viewer(&repeater.clone().upcast(), None);
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    let host = ItemsRepeaterScrollHost::new();
    host.set_width(400.0);
    host.set_height(600.0);
    host.set_scroll_viewer(Some(scroll.clone()));
    let root = TestRoot::with_child(&host);
    root.execute_initial_layout_pass();
    settle(&root);

    // Preserve the source sequence and outer dimensions. Native requests use
    // the fully arranged target. WinUI captures RenderSize before the first
    // disconnected target is arranged, so its zero-height request realizes
    // 96..=103. Centering the complete 94-DIP native target instead realizes
    // 96..=104 at offset 10152. API-SHAPES.md records the source comparison.
    // The native clock replaces CompositionTarget ticks without wall sleeps.
    for (step, (index, alignment, animate, expected_first, expected_last, expected_offset)) in [
        (100, 0.5, false, 96, 104, 10152.0),
        (103, 0.5, true, 99, 107, 10464.0),
        (0, 0.5, true, 0, 6, 0.0),
        (20, 0.0, false, 19, 26, 2085.0),
    ]
    .into_iter()
    .enumerate()
    {
        let target = if index == 103 {
            repeater
                .try_get_element(index)
                .expect("already realized target")
        } else {
            repeater.get_or_create_element(index)
        };
        host.start_bring_into_view(&target, 0.0, alignment, 0.0, 0.0, animate);
        settle(&root);
        if animate {
            let time = step as f64 * 1000.0;
            clock.pulse(time);
            settle(&root);
            assert_eq!(
                repeater.try_get_element(index),
                Some(target.clone()),
                "target {index} at animation start"
            );
            clock.pulse(time + 125.0);
            settle(&root);
            assert_eq!(
                repeater.try_get_element(index),
                Some(target.clone()),
                "target {index} during animation"
            );
            clock.pulse(time + 251.0);
            settle(&root);
        }
        assert!(!host.has_pending_bring_into_view());
        assert_eq!(
            repeater.try_get_element(index),
            Some(target.clone()),
            "target {index} after completion"
        );
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&target).pin_count(),
            0
        );
        assert_eq!(target.bounds().size(), Size::new(390.0, 94.0));
        assert_eq!(target.desired_size(), Size::new(400.0, 104.0));
        assert!(
            (scroll.offset().y - expected_offset).abs() < 0.01,
            "target {index}: expected offset {expected_offset}, observed {:?}",
            scroll.offset(),
        );
        let mut realized: Vec<_> = repeater
            .realized_elements()
            .iter()
            .map(|element| repeater.get_element_index(element))
            .collect();
        realized.sort_unstable();
        assert_eq!(
            realized,
            (expected_first..=expected_last).collect::<Vec<_>>(),
            "index {index}, offset {:?}, viewport {:?}, target bounds {:?}, target desired {:?}, layout extent {:?}",
            scroll.offset(),
            repeater.visible_window(),
            target.bounds(),
            target.desired_size(),
            repeater.viewport_manager.layout_extent(),
        );
        if index == 0 {
            assert_eq!(scroll.offset().y, 0.0);
        }
        let before = scroll.offset();
        host.start_bring_into_view(&target, 0.0, alignment, 0.0, 0.0, false);
        settle(&root);
        assert_eq!(
            scroll.offset(),
            before,
            "repeating the same aligned request is a no-op"
        );
    }
}

fn hosted_repeater(
    source: ItemsSource,
) -> (
    Ref<TestRoot>,
    Ref<ItemsRepeaterScrollHost>,
    Ref<ScrollViewer>,
    Ref<ItemsRepeater>,
) {
    let repeater = ItemsRepeater::new();
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let item = Border::new();
            item.set_height(20.0);
            Some(item.upcast())
        },
        true,
    )));
    repeater.set_items_source(Some(source));
    let scroll = viewer(&repeater.clone().upcast(), None);
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    let host = ItemsRepeaterScrollHost::new();
    host.set_width(200.0);
    host.set_height(200.0);
    host.set_scroll_viewer(Some(scroll.clone()));
    let root = TestRoot::with_child(&host);
    root.execute_initial_layout_pass();
    settle(&root);
    (root, host, scroll, repeater)
}

#[test]
fn cancelling_a_host_bring_preserves_independent_pins_and_releases_its_target() {
    let _scope = test_scope();
    let clock = super::services_tests::TestClock::install();
    let (root, host, scroll, repeater) = hosted_repeater(ItemsSource::from_values(0..200));
    let target = repeater.get_or_create_element(100);
    repeater.pin_element(&target);
    let info = ItemsRepeater::get_virtualization_info(&target);
    host.start_bring_into_view(&target, 0.0, 0.0, 0.0, 0.0, true);
    assert_eq!(info.pin_count(), 2);
    settle(&root);
    clock.pulse(0.0);
    clock.pulse(100.0);
    settle(&root);
    assert_eq!(repeater.try_get_element(100), Some(target.clone()));
    scroll.set_offset(Vector::new(0.0, 20.0));
    assert!(!host.has_pending_bring_into_view());
    assert_eq!(
        info.pin_count(),
        1,
        "the caller's pin remains owned by the caller"
    );
    clock.pulse(251.0);
    settle(&root);
    assert_eq!(scroll.offset().y, 20.0);
    assert_eq!(repeater.try_get_element(100), Some(target.clone()));
    repeater.unpin_element(&target);
    settle(&root);
    assert!(repeater.try_get_element(100).is_none());
}

#[test]
fn replacing_and_detaching_host_bring_releases_each_operation_pin() {
    let _scope = test_scope();
    let clock = super::services_tests::TestClock::install();
    let (root, host, _, repeater) = hosted_repeater(ItemsSource::from_values(0..200));
    let first = repeater.get_or_create_element(80);
    host.start_bring_into_view(&first, 0.0, 0.0, 0.0, 0.0, true);
    let first_info = ItemsRepeater::get_virtualization_info(&first);
    assert_eq!(first_info.pin_count(), 1);
    let second = repeater.get_or_create_element(100);
    host.start_bring_into_view(&second, 0.0, 0.0, 0.0, 0.0, true);
    assert_eq!(first_info.pin_count(), 0);
    let second_info = ItemsRepeater::get_virtualization_info(&second);
    assert_eq!(second_info.pin_count(), 1);
    settle(&root);
    clock.pulse(0.0);
    assert!(host.has_pending_bring_into_view());
    root.set_child(None::<Ref<Control>>);
    assert!(!host.has_pending_bring_into_view());
    assert_eq!(
        second_info.pin_count(),
        0,
        "a retained detached host releases its lease"
    );
    clock.pulse(500.0);
    assert_eq!(second_info.pin_count(), 0);
}

#[test]
fn replacing_or_detaching_host_content_cancels_an_obsolete_target() {
    let _scope = test_scope();
    let clock = super::services_tests::TestClock::install();
    for replace_content in [false, true] {
        let (root, host, scroll, repeater) = hosted_repeater(ItemsSource::from_values(0..200));
        let wrapper = Border::new();
        scroll.set_content(None);
        wrapper.set_child(&repeater);
        scroll.set_content(Some(Control::boxed(&wrapper)));
        settle(&root);
        let target = repeater.get_or_create_element(100);
        host.start_bring_into_view(&target, 0.0, 0.0, 0.0, 0.0, true);
        settle(&root);
        clock.pulse(0.0);
        let info = ItemsRepeater::get_virtualization_info(&target);
        assert_eq!(info.pin_count(), 1);
        if replace_content {
            scroll.set_content(Some(Control::boxed(Border::new())));
        } else {
            // The host/viewer stay attached, and the target still belongs to
            // its original repeater. The full ancestry nevertheless changed.
            wrapper.set_child(None::<Ref<Control>>);
        }
        let before = scroll.offset();
        clock.pulse(251.0);
        assert_eq!(scroll.offset(), before);
        assert!(!host.has_pending_bring_into_view());
        assert_eq!(info.pin_count(), 0);
    }
}

#[test]
fn source_mutations_clear_offscreen_pins_released_by_layout() {
    use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction};
    for action in [
        NotifyCollectionChangedAction::Remove,
        NotifyCollectionChangedAction::Replace,
        NotifyCollectionChangedAction::Reset,
    ] {
        let _scope = test_scope();
        let clock = super::services_tests::TestClock::install();
        let source = Rc::new(FerroList::from_items(0..200));
        let (root, host, scroll, repeater) = hosted_repeater(source.clone().into());
        let target = repeater.get_or_create_element(100);
        host.start_bring_into_view(&target, 0.0, 0.0, 0.0, 0.0, true);
        settle(&root);
        clock.pulse(0.0);
        let before = scroll.offset();
        let info = ItemsRepeater::get_virtualization_info(&target);
        assert_eq!(info.owner(), super::ElementOwner::PinnedPool);
        assert!(!info.auto_recycle_candidate());
        match action {
            NotifyCollectionChangedAction::Remove => source.remove_at(100),
            NotifyCollectionChangedAction::Replace => source.set(100, 999),
            NotifyCollectionChangedAction::Reset => source.clear(),
            _ => unreachable!(),
        }
        assert_eq!(
            info.owner(),
            super::ElementOwner::ElementFactory,
            "{action:?}"
        );
        assert_eq!(info.pin_count(), 0, "{action:?}");
        assert_eq!(repeater.get_element_index(&target), -1, "{action:?}");
        clock.pulse(125.0);
        assert!(!host.has_pending_bring_into_view(), "{action:?}");
        assert_eq!(scroll.offset(), before, "{action:?}");
    }
}

#[test]
fn stale_host_bring_cannot_scroll_or_unpin_a_recycled_target() {
    use ferroui_base::collections::FerroList;
    let _scope = test_scope();
    let clock = super::services_tests::TestClock::install();
    let source = Rc::new(FerroList::from_items((0..200).map(|_| {
        let item = Border::new();
        item.set_height(20.0);
        item.upcast::<Control>()
    })));
    let (root, host, scroll, repeater) = hosted_repeater(source.clone().into());
    repeater.set_data_template(None);
    settle(&root);
    let target = repeater.get_or_create_element(100);
    host.start_bring_into_view(&target, 0.0, 0.0, 0.0, 0.0, true);
    settle(&root);
    clock.pulse(0.0);
    let before = scroll.offset();
    source.remove_at(100);
    source.insert(0, target.clone());
    let replacement = repeater.get_or_create_element(0);
    assert_eq!(
        replacement, target,
        "reuse the exact native control for a different item position"
    );
    repeater.pin_element(&replacement);
    let info = ItemsRepeater::get_virtualization_info(&replacement);
    assert_eq!(info.pin_count(), 1);
    clock.pulse(125.0);
    assert!(!host.has_pending_bring_into_view());
    assert_eq!(
        scroll.offset(),
        before,
        "the obsolete animation cannot commit its destination"
    );
    assert_eq!(
        info.pin_count(),
        1,
        "the obsolete lease cannot release the new realization's pin"
    );
    assert_eq!(
        repeater.viewport_manager.made_anchor(),
        Some(replacement.clone())
    );
    clock.pulse(251.0);
    assert_eq!(scroll.offset(), before);
    assert_eq!(info.pin_count(), 1);
    assert_eq!(
        repeater.viewport_manager.made_anchor(),
        Some(replacement.clone())
    );
    repeater.unpin_element(&replacement);
}

#[test]
fn detaching_a_nested_host_bring_releases_inner_and_outer_realization_pins() {
    let _scope = test_scope();
    let _clock = super::services_tests::TestClock::install();
    let (root, host, scroll, inner) = hosted_repeater(ItemsSource::from_values(0..200));
    let outer = ItemsRepeater::new();
    scroll.set_content(None);
    outer.set_items_source(Some(ItemsSource::from_items([Some(Control::boxed(
        &inner,
    ))])));
    scroll.set_content(Some(Control::boxed(&outer)));
    settle(&root);
    let target = inner.get_or_create_element(100);
    host.start_bring_into_view(&target, 0.0, 0.0, 0.0, 0.0, true);
    let inner_info = ItemsRepeater::get_virtualization_info(&target);
    let outer_info = ItemsRepeater::get_virtualization_info(&inner);
    assert_eq!(inner_info.pin_count(), 1);
    assert_eq!(outer_info.pin_count(), 1);
    root.set_child(None::<Ref<Control>>);
    assert_eq!(inner_info.pin_count(), 0);
    assert_eq!(outer_info.pin_count(), 0);
    assert!(!host.has_pending_bring_into_view());
}

#[test]
fn detach_and_reparent_reset_viewport_cache_and_anchor_lifetimes() {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(1000.0, 2000.0));
    repeater.set_vertical_cache_length(2.0);
    let first = viewer(&repeater.clone().upcast(), Some(Size::new(200.0, 300.0)));
    let second = viewer(&Border::new().upcast(), Some(Size::new(400.0, 500.0)));
    let panel = Grid::new();
    panel.children().add(&first);
    panel.children().add(&second);
    let root = TestRoot::with_child(&panel);
    root.execute_initial_layout_pass();
    settle(&root);
    first.set_offset(Vector::new(0.0, 700.0));
    settle(&root);
    assert_eq!(repeater.visible_window().y, 700.0);
    first.set_content(None);
    assert!(!repeater.is_attached_to_visual_tree());
    second.set_content(Some(Control::boxed(&repeater)));
    settle(&root);
    assert_close(
        *layout.visible.borrow().last().unwrap(),
        Rect::new(0.0, 0.0, 400.0, 500.0),
    );
    let weak = repeater.downgrade();
    second.set_content(None);
    drop(repeater);
    settle(&root);
    assert!(weak.upgrade().is_none());
}

#[test]
fn reparenting_registers_surviving_layout_children_with_the_new_anchor_provider() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let item = Border::new();
            item.set_width(100.0);
            item.set_height(50.0);
            Some(item.upcast())
        },
        true,
    )));
    repeater.set_items_source(Some(ItemsSource::from_values(0..100)));
    let first_item_preparations = Rc::new(Cell::new(0));
    let preparations = first_item_preparations.clone();
    let _prepared_subscription = repeater.element_prepared(move |args| {
        if args.index == 0 {
            preparations.set(preparations.get() + 1);
        }
    });
    let size = Some(Size::new(200.0, 300.0));
    let first = viewer(&repeater.clone().upcast(), size);
    let second = viewer(&Border::new().upcast(), size);
    let panel = Grid::new();
    panel.children().add(&first);
    panel.children().add(&second);
    let root = TestRoot::with_child(&panel);
    root.execute_initial_layout_pass();
    settle(&root);
    let retained = repeater
        .try_get_element(0)
        .expect("The first item is visible.");
    let preparation_count = first_item_preparations.get();
    assert_eq!(first.current_anchor(), Some(retained.clone()));

    // The first layout-owned row survives each move without ElementPrepared.
    // It still needs to be registered after arrange with the new provider,
    // even when no cache growth schedules an additional measure pass.
    for (previous, next) in [(&first, &second), (&second, &first)] {
        previous.set_content(Some(Control::boxed(Border::new())));
        next.set_content(Some(Control::boxed(&repeater)));
        settle(&root);
        assert_eq!(repeater.try_get_element(0), Some(retained.clone()));
        assert_eq!(first_item_preparations.get(), preparation_count);
        assert_eq!(next.current_anchor(), Some(retained.clone()));
        assert!(previous.current_anchor().is_none());
        assert_eq!(next.offset(), Vector::default());
    }
}

#[test]
fn reattaching_after_scrolling_keeps_the_realized_items() {
    // WinUI's OnUnloaded resets the scrollers and the cache buffer alone; the visible window
    // and the extent survive, so the first measure after reattaching realizes the same items.
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let item = Border::new();
            item.set_width(100.0);
            item.set_height(50.0);
            Some(item.upcast())
        },
        true,
    )));
    repeater.set_items_source(Some(ItemsSource::from_values(0..100)));
    let host = Border::new();
    host.set_height(5000.0);
    host.set_child(Some(repeater.clone()));
    let first = viewer(&host.clone().upcast(), Some(Size::new(200.0, 300.0)));
    let root = TestRoot::with_child(&first);
    root.execute_initial_layout_pass();
    settle(&root);
    first.set_offset(Vector::new(0.0, 1000.0));
    settle(&root);
    let visible = repeater
        .try_get_element(21)
        .expect("Item 21 is in the scrolled viewport.");
    let cleared = Rc::new(Cell::new(0));
    let count = cleared.clone();
    let _subscription = repeater.element_clearing(move |_| count.set(count.get() + 1));
    host.set_child(None::<Ref<Control>>);
    assert!(!repeater.is_attached_to_visual_tree());
    host.set_child(Some(repeater.clone()));
    settle(&root);
    assert_eq!(cleared.get(), 0);
    assert_eq!(repeater.try_get_element(21), Some(visible));
}

/// One notch of the mouse wheel over `target`, routed as the platform routes it.
fn wheel_notch(target: &Ref<Control>, delta_y: f64) -> bool {
    use ferroui_base::input::{
        IPointer, KeyModifiers, Pointer, PointerPointProperties, PointerType, PointerUpdateKind,
        PointerWheelEventArgs, RawInputModifiers,
    };
    let pointer: Rc<dyn IPointer> =
        Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let e = PointerWheelEventArgs::new(
        target.clone(),
        pointer,
        target,
        Point::default(),
        0,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
        KeyModifiers::NONE,
        Vector::new(0.0, delta_y),
    );
    target.raise_event(&e);
    e.handled()
}

#[test]
fn the_mouse_wheel_scrolls_a_virtualizing_repeater() {
    // Wheel input, not offsets set directly: the offset the presenter sets for each notch
    // must survive the repeater's realization and the scroll anchoring that follows it.
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let item = Border::new();
            item.set_width(100.0);
            item.set_height(50.0);
            Some(item.upcast())
        },
        true,
    )));
    repeater.set_items_source(Some(ItemsSource::from_values(0..1000)));
    let first = viewer(&repeater.clone().upcast(), Some(Size::new(200.0, 300.0)));
    let root = TestRoot::with_child(&first);
    root.execute_initial_layout_pass();
    settle(&root);
    assert_eq!(first.offset().y, 0.0);
    for notch in 1..=10 {
        let target = repeater
            .realized_elements()
            .into_iter()
            .next()
            .expect("A realized item is under the pointer.");
        assert!(wheel_notch(&target, -1.0), "the wheel is handled");
        settle(&root);
        assert!(
            first.offset().y >= notch as f64 * 50.0 - 1.0,
            "after {notch} notches the offset is {}",
            first.offset().y
        );
    }
}
