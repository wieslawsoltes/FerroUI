// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.
// Behavioral ports of the nested-template/cache API scenarios and the three
// Repeater interaction scenarios at the pinned WinUI revision. Native test
// roots, templates, collection notifications and ScrollViewer layout replace
// the Windows test application and UI Automation input driver.

use super::*;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::templates::{FuncControlTemplate, FuncDataTemplate, IDataTemplate};
use crate::test_support::{test_scope, TestRoot};
use ferroui_controls::{
    Control, Grid, IItemsList, ItemsChangedEventArgs, ItemsSource, ScrollViewer, StackPanel,
    TextBlock,
};
use ferroui_base::collections::FerroList;
use ferroui_base::data::{BindingPriority, ReflectionBinding};
use ferroui_base::layout::Orientation;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectExtensions, Point, Ref, Size, StyledElement, Vector,
};
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn settle(root: &TestRoot) {
    // Cache growth and effective-viewport delivery schedule native layout work.
    // Drain deterministic dispatcher work rather than waiting for wall time.
    for _ in 0..16 {
        root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(None);
    }
}

fn viewer(content: &Ref<Control>) -> Ref<ScrollViewer> {
    let scroll = ScrollViewer::new();
    scroll.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(
        crate::test_support::create_template,
    )));
    scroll.set_content(Some(Control::boxed(content)));
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    scroll
}

fn mount(repeater: &Ref<ItemsRepeater>, size: Size) -> (Ref<TestRoot>, Ref<ScrollViewer>) {
    let scroll = viewer(&repeater.clone().upcast());
    let host = ItemsRepeaterScrollHost::new();
    host.set_width(size.width);
    host.set_height(size.height);
    host.set_scroll_viewer(Some(scroll.clone()));
    let root = TestRoot::with_child(&host);
    root.execute_initial_layout_pass();
    settle(&root);
    (root, scroll)
}

fn bound_text(width: Option<f64>, height: f64) -> Ref<TextBlock> {
    let text = TextBlock::new();
    if let Some(width) = width {
        text.set_width(width);
    }
    // A fixed native line height keeps the test independent of the platform
    // font metrics; the source scenario depends on rows, not a particular font.
    text.set_height(height);
    text.bind_binding(
        TextBlock::text_property().as_property(),
        &*ReflectionBinding::empty(),
    );
    text
}

fn text_template(width: Option<f64>, height: f64) -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(
        |_| true,
        move |_, _| Some(bound_text(width, height).upcast()),
        true,
    )
}

fn verify_text_and_element_indices(repeater: &ItemsRepeater) {
    for index in 0..10 {
        let element = repeater.try_get_element(index).expect("realized item");
        assert_eq!(
            element.cast::<TextBlock>().unwrap().text(),
            Some(format!("Item #{index}")),
        );
        assert_eq!(repeater.get_element_index(&element), index);
    }
    assert!(repeater.try_get_element(20).is_none());
}

#[test]
fn validate_element_to_index_mapping() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    let factory = RecyclingElementFactory::new();
    factory.set_recycle_pool(Some(RecyclePool::new()));
    factory
        .templates()
        .add("Item".into(), text_template(None, 50.0));
    repeater.set_element_factory(Some(factory.as_element_factory()));
    repeater.set_items_source(Some(ItemsSource::from_values(
        (0..10).map(|index| format!("Item #{index}")),
    )));
    // As in the source, do not assign Layout: this must use the default stack.
    let (_root, _scroll) = mount(&repeater, Size::new(400.0, 800.0));
    verify_text_and_element_indices(&repeater);
}

#[test]
fn validate_repeater_defaults() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_items_source(Some(ItemsSource::from_values(
        (0..10).map(|index| format!("Item #{index}")),
    )));
    // The source scenario leaves both Layout and ItemTemplate unspecified.
    let (_root, _scroll) = mount(&repeater, Size::new(400.0, 800.0));
    verify_text_and_element_indices(&repeater);
}

#[test]
fn validate_null_items_source() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    let error = catch_unwind(AssertUnwindSafe(|| {
        repeater.get_or_create_element(0);
    }))
    .expect_err("requesting an element without a source must reject the call");
    let message = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .expect("the native precondition exposes a diagnostic");
    // The ignored C# assertion spells this ItemSource, but its pinned C++
    // implementation emits ItemsSource. Keep the actual source diagnostic;
    // Rust's programmer-error panic replaces the E_FAIL COM exception.
    assert!(message.contains("ItemsSource doesn't have a value"));
    assert!(repeater.items_source_view().is_none());
    assert!(repeater.try_get_element(0).is_none());
}

fn bind_group_source(repeater: &ItemsRepeater) {
    let object: &FerroObject = repeater;
    repeater.bind(
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
}

fn groups(items_per_group: i32) -> ItemsSource {
    // Preserve notifying inner and outer collections, including their identity
    // when recycled templates inherit a different group's data context.
    let groups = FerroList::from_items(
        (0..100).map(|_| ItemsSource::new(Rc::new(FerroList::from_items(0..items_per_group)))),
    );
    ItemsSource::new(Rc::new(groups))
}

fn nested_template(horizontal_scrollers: bool) -> Rc<dyn IDataTemplate> {
    let item_template = if horizontal_scrollers {
        text_template(Some(100.0), 100.0)
    } else {
        text_template(None, 20.0)
    };
    FuncDataTemplate::new(
        |_| true,
        move |_, _| {
            let panel = StackPanel::new();
            panel.children().add(bound_text(None, 20.0));
            let inner = ItemsRepeater::new();
            inner.set_data_template(Some(item_template.clone()));
            bind_group_source(&inner);
            if horizontal_scrollers {
                let layout = StackLayout::new();
                layout.set_orientation(Orientation::Horizontal);
                inner.set_layout(Some(layout.upcast()));
                let scroll = viewer(&inner.upcast());
                scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
                scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled);
                let host = ItemsRepeaterScrollHost::new();
                host.set_scroll_viewer(Some(scroll));
                panel.children().add(host);
            } else {
                inner.set_vertical_cache_length(0.0);
                panel.children().add(inner);
            }
            Some(panel.upcast())
        },
        true,
    )
}

#[test]
fn nested_repeater_with_data_template_scenario() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_vertical_cache_length(0.0);
    repeater.set_data_template(Some(nested_template(false)));
    repeater.set_items_source(Some(groups(5)));
    let (root, scroll) = mount(&repeater, Size::new(400.0, 600.0));

    // The pinned source returns early for its animated variant (#779). Its
    // executed branch scrolls a zero-cache, 100-by-five nested source through
    // these nine exact destinations, causing group-template recycling.
    for step in 1..10 {
        let offset = (step * 200) as f64;
        scroll.set_offset(Vector::new(0.0, offset));
        settle(&root);
        assert_eq!(scroll.offset().y, offset, "nested scroll step {step}");
    }
}

#[test]
fn verify_current_anchor() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_vertical_cache_length(0.0);
    repeater.set_data_template(Some(text_template(None, 50.0)));
    repeater.set_items_source(Some(ItemsSource::from_values(0..500)));
    let (root, scroll) = mount(&repeater, Size::new(400.0, 600.0));
    // This declaration is commented out in the pinned source. Exercise its
    // exact offset and anchor-index assertions through the native viewer.
    for step in 1..10 {
        scroll.set_offset(Vector::new(0.0, (step * 200) as f64));
        settle(&root);
        assert_eq!(scroll.offset().y, (step * 200) as f64);
        let anchor = scroll.current_anchor().expect("a visible anchor candidate");
        assert_eq!(repeater.get_element_index(&anchor), step * 4);
    }
}

#[test]
fn verify_corrections_in_non_scrollable_direction() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let panel = StackPanel::new();
            let inner = ItemsRepeater::new();
            let layout = StackLayout::new();
            layout.set_orientation(Orientation::Horizontal);
            inner.set_layout(Some(layout.upcast()));
            bind_group_source(&inner);
            panel.children().add(inner);
            Some(panel.upcast())
        },
        true,
    )));
    repeater.set_items_source(Some(groups(4)));
    let scroll = viewer(&repeater.clone().upcast());
    scroll.set_width(400.0);
    scroll.set_height(400.0);
    let host = ItemsRepeaterScrollHost::new();
    host.set_width(400.0);
    host.set_height(600.0);
    host.set_scroll_viewer(Some(scroll.clone()));
    let root = TestRoot::with_child(&host);
    root.execute_initial_layout_pass();
    settle(&root);
    // The commented source scenario scrolls this nested orthogonal layout
    // four times and asserts completion without a crash. Native set_offset
    // completes synchronously; draining layout observes queued corrections.
    for step in 1..5 {
        scroll.set_offset(Vector::new(0.0, (step * 200) as f64));
        settle(&root);
        assert_eq!(scroll.offset().x, 0.0);
        assert!(scroll.offset().y > 0.0);
    }
}

#[test]
fn verify_store_scenario_cache() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_data_template(Some(nested_template(true)));
    repeater.set_items_source(Some(groups(4)));
    let (_root, scroll) = mount(&repeater, Size::new(400.0, 200.0));

    // Upstream specifically checks outer group two, which is outside the
    // visible area but within the default cache, and its first inner item.
    let group = repeater
        .try_get_element(2)
        .expect("the outer cache must realize group two")
        .cast::<StackPanel>()
        .unwrap();
    assert!(group.bounds().y >= scroll.viewport().height);
    let host = group
        .children()
        .get(1)
        .cast::<ItemsRepeaterScrollHost>()
        .unwrap();
    let inner_scroll = host.scroll_viewer().unwrap();
    let inner = Control::from_boxed(&inner_scroll.content().unwrap())
        .unwrap()
        .cast::<ItemsRepeater>()
        .unwrap();
    assert!(inner.try_get_element(0).is_some());
}

fn uniform_grid_fixture() -> (
    Ref<TestRoot>,
    Ref<ScrollViewer>,
    Ref<ItemsRepeater>,
    Ref<UniformGridLayout>,
) {
    // Match UniformGridLayoutDemo: forty 100x100 items, eight-pixel spacing,
    // at most four columns and a viewport no taller than 500 pixels.
    let layout = UniformGridLayout::new();
    layout.set_min_row_spacing(8.0);
    layout.set_min_column_spacing(8.0);
    layout.set_maximum_rows_or_columns(4);
    let repeater = ItemsRepeater::new();
    repeater.set_layout(Some(layout.clone().upcast()));
    repeater.set_items_source(Some(ItemsSource::from_values(0..40)));
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let grid = Grid::new();
            grid.set_width(100.0);
            grid.set_height(100.0);
            grid.children().add(bound_text(None, 20.0));
            Some(grid.upcast())
        },
        true,
    )));
    let scroll = viewer(&repeater.clone().upcast());
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
    scroll.set_width(500.0);
    scroll.set_height(500.0);
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    settle(&root);
    (root, scroll, repeater, layout)
}

#[test]
fn verify_uniform_grid_layout_does_not_create_holes() {
    let _scope = test_scope();
    let (root, scroll, repeater, _layout) = uniform_grid_fixture();
    let old_height = repeater.bounds().height;
    // Native offset changes exercise the same end-and-small-reverse-scroll
    // layout sequence without claiming to test a Windows wheel driver.
    scroll.set_offset(Vector::new(0.0, 2000.0));
    settle(&root);
    for _ in 0..5 {
        scroll.set_offset(Vector::new(0.0, (scroll.offset().y - 50.0).max(0.0)));
        settle(&root);
    }
    assert!(
        (old_height - repeater.bounds().height).abs() < 1.0,
        "reversing scroll direction must not add/remove layout rows: {old_height} -> {}",
        repeater.bounds().height,
    );
}

#[test]
fn verify_uniform_grid_layout_scrolling() {
    let _scope = test_scope();
    let (root, scroll, _repeater, layout) = uniform_grid_fixture();
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    scroll.set_max_width(500.0);
    layout.set_items_stretch(UniformGridLayoutItemsStretch::Fill);
    layout.set_min_column_spacing(50.0);
    settle(&root);
    scroll.set_offset(Vector::new(0.0, 1000.0));
    settle(&root);
    assert_eq!(scroll.offset().y, 1000.0);
}

#[test]
fn insert_at_start_behavior() {
    let _scope = test_scope();
    let source = Rc::new(FerroList::<String>::new());
    let repeater = ItemsRepeater::new();
    repeater.set_min_height(100.0);
    repeater.set_items_source(Some(source.clone().into()));
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let grid = Grid::new();
            grid.children().add(bound_text(None, 20.0));
            Some(grid.upcast())
        },
        true,
    )));
    let (root, _scroll) = mount(&repeater, Size::new(400.0, 300.0));
    let count_label = TextBlock::new();
    for previous_count in 0..10 {
        // BasicDemo's button handler records the count BEFORE inserting. Its
        // interaction test checks this previous-frame count after each click.
        count_label.set_text(Some(&repeater.visual_children_count().to_string()));
        source.insert(0, format!("Item{previous_count}"));
        settle(&root);
        assert_eq!(count_label.text(), Some(previous_count.to_string()));
        assert_eq!(repeater.visual_children_count(), previous_count + 1);
    }
}

struct StableResetSource {
    items: Vec<BoxedValue>,
    reads: Cell<usize>,
    changes: HandlerList<ferroui_controls::ItemsChangedHandler>,
}

impl StableResetSource {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            items: (0..3).map(|value| Rc::new(value) as BoxedValue).collect(),
            reads: Cell::new(0),
            changes: HandlerList::new(),
        })
    }

    fn reset(&self) {
        for (_, handler) in self.changes.snapshot().iter() {
            handler(&ItemsChangedEventArgs::RESET);
        }
    }
}

impl IItemsList for StableResetSource {
    fn count(&self) -> usize {
        self.items.len()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        self.reads.set(self.reads.get() + 1);
        Some(self.items[index].clone())
    }

    fn key_index_mapping(&self) -> Option<&dyn ferroui_controls::IKeyIndexMapping> {
        Some(self)
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(
        &self,
        handler: Rc<ferroui_controls::ItemsChangedHandler>,
    ) -> Option<u64> {
        Some(self.changes.add(handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.changes.remove(token);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ferroui_controls::IKeyIndexMapping for StableResetSource {
    fn key_from_index(&self, index: i32) -> String {
        self.items[index as usize]
            .downcast_ref::<i32>()
            .unwrap()
            .to_string()
    }

    fn index_from_key(&self, key: &str) -> i32 {
        // Key lookup must not be routed through get_at, whose absence is the
        // observable data-virtualization contract under test.
        self.items
            .iter()
            .position(|item| item.downcast_ref::<i32>().unwrap().to_string() == key)
            .map_or(-1, |index| index as i32)
    }
}

fn recycling_text_factory(width: Option<f64>) -> Ref<RecyclingElementFactory> {
    let factory = RecyclingElementFactory::new();
    factory.set_recycle_pool(Some(RecyclePool::new()));
    factory
        .templates()
        .add("Item".into(), text_template(width, 100.0));
    factory
}

#[test]
fn validate_stable_resets() {
    let _scope = test_scope();
    let source = StableResetSource::new();
    let repeater = ItemsRepeater::new();
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    repeater.set_items_source(Some(ItemsSource::new(source.clone())));
    repeater.set_element_factory(Some(recycling_text_factory(None).as_element_factory()));
    let (root, _scroll) = mount(&repeater, Size::new(200.0, 200.0));
    let before: Vec<_> = (0..3)
        .map(|index| {
            let element = repeater
                .try_get_element(index)
                .expect("initial realized item");
            let data = element.data_context().unwrap();
            (element, data)
        })
        .collect();

    source.reads.set(0);
    source.reset();
    settle(&root);
    // The upstream assertion is zero source reads, not merely stable visual
    // identity: a stable reset retains the already-bound data as well.
    assert_eq!(source.reads.get(), 0);
    for (index, (old_element, old_data)) in before.iter().enumerate() {
        let element = repeater.try_get_element(index as i32).unwrap();
        assert!(element.ptr_eq(old_element));
        assert!(Rc::ptr_eq(&element.data_context().unwrap(), old_data));
        assert_eq!(
            element.cast::<TextBlock>().unwrap().text(),
            Some(index.to_string())
        );
    }
    assert_eq!(
        (0..3)
            .filter(|&index| repeater.try_get_element(index).is_some())
            .count(),
        3
    );
    assert_eq!(source.reads.get(), 0);
}

fn verify_grid_placement(repeater: &ItemsRepeater, source: &FerroList<i32>) -> usize {
    let mut realized = Vec::new();
    for index in 0..source.count() {
        let Some(element) = repeater.try_get_element(index as i32) else {
            continue;
        };
        let text = element.cast::<TextBlock>().unwrap();
        assert_eq!(
            text.text(),
            Some(source.get(index).to_string()),
            "item {index} data"
        );
        let position = element.translate_point(Point::default(), repeater).unwrap();
        assert_eq!(position.x, (index % 3) as f64 * 100.0, "item {index} X");
        assert_eq!(position.y, (index / 3) as f64 * 100.0, "item {index} Y");
        realized.push(index);
    }
    let first = *realized.first().expect("some grid cells must be realized");
    let last = *realized.last().unwrap();
    assert_eq!(
        last - first + 1,
        realized.len(),
        "contiguous realized range"
    );
    realized.len()
}

#[test]
fn validate_grid_placement_after_insert_before_scrolled_anchor() {
    let _scope = test_scope();
    // This desired-behavior regression is ignored in the pinned source under
    // PC-GRID-ANCHOR-INSERT. Keep all cell/data/contiguity assertions in Rust.
    let source = Rc::new(FerroList::from_items(0..60));
    let layout = UniformGridLayout::new();
    layout.set_min_item_width(100.0);
    layout.set_min_item_height(100.0);
    layout.set_maximum_rows_or_columns(3);
    let repeater = ItemsRepeater::new();
    repeater.set_layout(Some(layout.upcast()));
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    repeater.set_element_factory(Some(
        recycling_text_factory(Some(100.0)).as_element_factory(),
    ));
    repeater.set_items_source(Some(source.clone().into()));
    let scroll = viewer(&repeater.clone().upcast());
    scroll.set_width(300.0);
    scroll.set_height(300.0);
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    settle(&root);
    scroll.set_offset(Vector::new(0.0, 1050.0));
    settle(&root);
    assert_eq!(scroll.offset().y, 1050.0);
    assert!(repeater.try_get_element(0).is_none());
    verify_grid_placement(&repeater, &source);

    source.insert(0, 1000);
    settle(&root);
    let realized = verify_grid_placement(&repeater, &source);
    assert!(
        realized >= 9,
        "at least the three visible rows are realized"
    );
}
