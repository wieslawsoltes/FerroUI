// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Compiled binding integration with the production reactive docking models.
use crate::tests::{layout, run, scene};
use crate::*;
use ferrodock_model::core::IDockable;
use ferrodock_model_reactiveui::controls::Document;
use ferroui_base::data::core::plugins::PropertyInfoAccessorFactory;
use ferroui_base::data::core::{ClrPropertyInfo, Value};
use ferroui_base::data::{BindingPriority, CompiledBinding, CompiledBindingPathBuilder};
use ferroui_base::layout::Orientation;
use std::rc::Rc;

fn proportion_binding() -> Rc<CompiledBinding> {
    // The same typed accessors a compiled binding emits. No model wrapper,
    // reflection, substitute property notifications or synchronization engine.
    CompiledBinding::new(
        CompiledBindingPathBuilder::new()
            .property(
                Rc::new(ClrPropertyInfo::read_write::<Document, Value<f64>>(
                    "Proportion",
                    IDockable::proportion,
                    IDockable::set_proportion,
                )),
                PropertyInfoAccessorFactory::create_inpc_property_accessor::<Document>(),
            )
            .build(),
    )
}

fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn layout_and_resize_preserve_compiled_two_way_proportion_bindings_to_docking_models() {
    run(|| {
        for (orientation, priority) in [
            (Orientation::Horizontal, BindingPriority::LocalValue),
            (Orientation::Vertical, BindingPriority::LocalValue),
            (Orientation::Horizontal, BindingPriority::Style),
            (Orientation::Vertical, BindingPriority::Style),
        ] {
            let (panel, a, _, b) = scene(orientation, 600.0, f64::NAN, f64::NAN);
            let first = Document::new();
            let second = Document::new();
            first.set_proportion(0.25);
            second.set_proportion(0.25);
            a.set_data_context(Some(first.clone()));
            b.set_data_context(Some(second.clone()));
            let a_binding = a.bind_binding(
                ProportionalStackPanel::proportion_property().as_property(),
                &*proportion_binding().with_priority(priority),
            );
            let b_binding = b.bind_binding(
                ProportionalStackPanel::proportion_property().as_property(),
                &*proportion_binding().with_priority(priority),
            );
            layout(&panel);
            // Attached metadata's default is TwoWay, and current-value writes
            // during normalization must flow through to the source model.
            near(first.proportion(), 0.5);
            near(second.proportion(), 0.5);
            first.set_proportion(0.3);
            second.set_proportion(0.7);
            near(ProportionalStackPanel::get_proportion(&a), 0.3);
            near(ProportionalStackPanel::get_proportion(&b), 0.7);
            layout(&panel);
            panel.resize_proportions(a.clone().upcast(), b.clone().upcast(), 60.0, 75.0);
            near(first.proportion(), 0.4);
            near(second.proportion(), 0.6);
            layout(&panel);
            near(first.proportion(), 0.4);
            near(second.proportion(), 0.6);
            // A later source update proves that the assignment/resize did not
            // replace the live binding with a local value.
            first.set_proportion(0.2);
            second.set_proportion(0.8);
            layout(&panel);
            near(ProportionalStackPanel::get_proportion(&a), 0.2);
            near(ProportionalStackPanel::get_proportion(&b), 0.8);
            a_binding.dispose();
            b_binding.dispose();
        }
    });
}

#[test]
fn compiled_proportion_binding_tracks_data_context_replacement_and_releases_old_model() {
    run(|| {
        use ferroui_base::data::model::INotifyPropertyChanged;
        let (_, a, _, _) = scene(Orientation::Horizontal, 600.0, f64::NAN, f64::NAN);
        let first = Document::new();
        first.set_proportion(0.25);
        let baseline = first.property_changed().handler_count();
        a.set_data_context(Some(first.clone()));
        let binding = a.bind_binding(
            ProportionalStackPanel::proportion_property().as_property(),
            &*proportion_binding(),
        );
        near(ProportionalStackPanel::get_proportion(&a), 0.25);
        assert_eq!(first.property_changed().handler_count(), baseline + 1);
        let second = Document::new();
        second.set_proportion(0.75);
        let second_baseline = second.property_changed().handler_count();
        a.set_data_context(Some(second.clone()));
        near(ProportionalStackPanel::get_proportion(&a), 0.75);
        assert_eq!(first.property_changed().handler_count(), baseline);
        first.set_proportion(0.1);
        near(ProportionalStackPanel::get_proportion(&a), 0.75);
        ProportionalStackPanel::set_proportion(&a, 0.6);
        near(second.proportion(), 0.6);
        near(first.proportion(), 0.1);
        let old = Rc::downgrade(&first);
        drop(first);
        assert!(old.upgrade().is_none());
        binding.dispose();
        assert_eq!(second.property_changed().handler_count(), second_baseline);
    });
}
