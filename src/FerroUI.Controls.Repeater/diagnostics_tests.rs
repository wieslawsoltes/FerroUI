use super::{
    ItemsRepeater, Layout, LayoutsTestHooks, LinedFlowLayout, RepeaterTestHooks, StackLayout,
    StackLayoutState,
};
use ferroui_controls::templates::FuncDataTemplate;
use crate::test_support::test_scope;
use ferroui_controls::testing::TestLogSink;
use ferroui_controls::{Border, ItemsSource};
use ferroui_base::logging::{LogArea, LogEventLevel};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Rect, Ref, Size};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn repeater(layout: Ref<Layout>) -> Ref<ItemsRepeater> {
    let repeater = ItemsRepeater::new();
    repeater.set_layout(Some(layout));
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let element = Border::new();
            element.set_width(40.0);
            element.set_height(20.0);
            Some(element.upcast())
        },
        false,
    )));
    repeater.set_items_source(Some(ItemsSource::from_values(0..3)));
    repeater
}

fn arrange(repeater: &ItemsRepeater) {
    repeater.measure(Size::new(120.0, 60.0));
    repeater.arrange(Rect::new(0.0, 0.0, 120.0, 60.0));
}

#[test]
fn selected_item_diagnostics_use_native_sinks_and_disable_with_negative_indices() {
    let _scope = test_scope();
    RepeaterTestHooks::set_log_item_index(-1);
    let layout = StackLayout::new();
    let repeater = repeater(layout.clone().upcast());
    let events = Rc::new(RefCell::new(Vec::new()));
    let captured = events.clone();
    let changed_source = Rc::new(Cell::new(false));
    let changed_from_sink = changed_source.clone();
    let weak_repeater = repeater.downgrade();
    let weak_layout = layout.downgrade();
    let subscription = TestLogSink::start(move |level, area, source, message, values| {
        if !message.starts_with("Repeater ") {
            return;
        }
        assert_eq!(level, LogEventLevel::Information);
        let repeater = weak_repeater.upgrade().unwrap();
        if area == LogArea::CONTROL {
            assert_eq!(
                source
                    .unwrap()
                    .downcast_ref::<ItemsRepeater>()
                    .unwrap()
                    .to_ref(),
                repeater
            );
            // Sinks may inspect the fully published ownership/index state.
            let _ = repeater.realized_elements();
            if let Some(value) = repeater.layout_state() {
                let state = value.downcast_ref::<Ref<StackLayoutState>>().unwrap();
                assert_eq!(
                    state.flow_algorithm.get_element_if_realized(1),
                    repeater.try_get_element(1),
                );
            }
            RepeaterTestHooks::set_log_item_index(RepeaterTestHooks::get_log_item_index());
        } else {
            assert_eq!(area, LogArea::LAYOUT);
            let layout = weak_layout.upgrade().unwrap();
            assert_eq!(
                source.unwrap().downcast_ref::<Layout>().unwrap().to_ref(),
                layout.clone().upcast::<Layout>()
            );
            assert_eq!(layout.log_item_index(), 1);
            layout.set_layout_id("Observed layout".into());
            // This would be rejected while ItemsRepeater's arrange guard was
            // active. Delivery occurs after the whole host operation finishes.
            if !changed_from_sink.replace(true) {
                repeater.set_items_source(None);
            }
        }
        captured.borrow_mut().push((
            area.to_owned(),
            values
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        ));
    });

    arrange(&repeater);
    assert!(
        events.borrow().is_empty(),
        "Default filters must not emit item traces"
    );

    RepeaterTestHooks::set_log_item_index(1);
    LayoutsTestHooks::set_layout_log_item_index(&layout.clone().upcast(), 1);
    // A size-only measure can reuse StackLayout's cached controls without
    // asking ViewManager to realize anything. Replace the source so this
    // phase actually exercises the selected item's realization boundary.
    repeater.set_items_source(Some(ItemsSource::from_values(3..6)));
    arrange(&repeater);
    assert!(
        events.borrow().is_empty(),
        "Both diagnostic streams must be deferred"
    );
    Dispatcher::ui_thread().run_jobs(None);
    assert!(changed_source.get());
    assert!(repeater.items_source().is_none());
    assert!(events.borrow().iter().any(|(area, values)| {
        area == LogArea::CONTROL && values == &["1", "realized", "Layout"]
    }));
    assert!(events
        .borrow()
        .iter()
        .any(|(area, values)| { area == LogArea::LAYOUT && values[0] == "1" }));
    assert!(events.borrow().iter().all(|(_, values)| values[0] == "1"));

    repeater.set_items_source(None);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(events.borrow().iter().any(|(area, values)| {
        area == LogArea::CONTROL && values == &["1", "recycled", "ElementFactory"]
    }));

    RepeaterTestHooks::set_log_item_index(-1);
    LayoutsTestHooks::set_layout_log_item_index(&layout.clone().upcast(), -1);
    events.borrow_mut().clear();
    repeater.set_items_source(Some(ItemsSource::from_values(0..3)));
    arrange(&repeater);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(events.borrow().is_empty());
    subscription.dispose();
}

#[test]
fn selected_item_diagnostics_do_not_retain_released_repeaters() {
    let _scope = test_scope();
    RepeaterTestHooks::set_log_item_index(1);
    let events = Rc::new(RefCell::new(Vec::new()));
    let captured = events.clone();
    let subscription = TestLogSink::start(move |_, _, _, message, _| {
        if message.starts_with("Repeater ") {
            captured.borrow_mut().push(message.to_owned());
        }
    });
    // A shareable layout can outlive its old host. Its queued arrangement
    // snapshot still must not be delivered after that host has been released.
    let layout = StackLayout::new();
    LayoutsTestHooks::set_layout_log_item_index(&layout.clone().upcast(), 1);
    let weak = {
        let repeater = repeater(layout.clone().upcast());
        arrange(&repeater);
        assert!(
            events.borrow().is_empty(),
            "Lifecycle diagnostics must be deferred"
        );
        repeater.downgrade()
    };
    assert!(weak.upgrade().is_none());
    Dispatcher::ui_thread().run_jobs(None);
    assert!(events.borrow().is_empty());
    RepeaterTestHooks::set_log_item_index(-1);
    subscription.dispose();
}

#[test]
fn lined_flow_item_diagnostics_report_the_arranged_bounds_without_realizing_more_items() {
    let _scope = test_scope();
    RepeaterTestHooks::set_log_item_index(-1);
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    let repeater = repeater(layout.clone().upcast());
    let events = Rc::new(RefCell::new(Vec::new()));
    let captured = events.clone();
    let subscription = TestLogSink::start(move |_, area, source, message, values| {
        if message.starts_with("Repeater arranged item ") {
            assert_eq!(area, LogArea::LAYOUT);
            assert!(source.unwrap().downcast_ref::<Layout>().is_some());
            captured.borrow_mut().push(
                values
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>(),
            );
        }
    });
    LayoutsTestHooks::set_layout_log_item_index(&layout.clone().upcast(), 1);
    arrange(&repeater);
    Dispatcher::ui_thread().run_jobs(None);
    let target = repeater.try_get_element(1).unwrap();
    assert_eq!(events.borrow().last().unwrap()[0], "1");
    assert_eq!(
        events.borrow().last().unwrap()[1],
        target.bounds().to_string()
    );
    let realized = repeater.realized_elements();
    events.borrow_mut().clear();
    LayoutsTestHooks::set_layout_log_item_index(&layout.clone().upcast(), 100);
    repeater.invalidate_arrange();
    arrange(&repeater);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(events.borrow().is_empty());
    assert_eq!(repeater.realized_elements(), realized);
    subscription.dispose();
}
