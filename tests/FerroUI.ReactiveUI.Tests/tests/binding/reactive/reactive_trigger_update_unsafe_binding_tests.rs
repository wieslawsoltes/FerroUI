//! `ReactiveTriggerUpdateUnsafeBindingTests` (`ReactiveUI.Binding.Tests/Reactive/ReactiveTriggerUpdateUnsafeBindingTests.cs`).
//!
//! `BindUnsafe` with an update stream is
//! [`bind_with_trigger`](ferroui_reactiveui::PropertyBindingMixins::bind_with_trigger) and
//! [`bind_select_with_trigger`](ferroui_reactiveui::PropertyBindingMixins::bind_select_with_trigger);
//! the matrix of upstream's data source is a loop.

use crate::tests::binding::test_models::{DispatchStubView, DispatchStubViewModel};
use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactiveui::*;
use ferroui_system::{IObservable, IObserver};
use std::rc::Rc;

#[test]
fn bind_unsafe_all_wirings_propagates_and_disposes() {
    const INITIAL: &str = "model";
    const EDITED: &str = "edited";
    for direction in [TriggerUpdate::ViewToViewModel, TriggerUpdate::ViewModelToView] {
        for has_stream in [false, true] {
            for converters in [false, true] {
                let model = DispatchStubViewModel::new();
                model.set_caption(INITIAL.to_string());
                let view = DispatchStubView::with_caption("view");
                let signal: Rc<Subject<i32>> = Subject::new();
                let updates: Option<Rc<dyn IObservable<i32>>> = has_stream.then(|| signal.as_observable());
                let binding = if converters {
                    view.bind_select_with_trigger(
                        model.clone(),
                        settable_prop!(DispatchStubViewModel => caption),
                        settable_prop!(DispatchStubView => caption),
                        |value: String| value,
                        |value: String| value,
                        updates,
                        direction,
                    )
                } else {
                    view.bind_with_trigger(model.clone(), settable_prop!(DispatchStubViewModel => caption), settable_prop!(DispatchStubView => caption), updates, direction)
                }
                .expect("no hook refuses the binding");

                assert_eq!(view.caption(), INITIAL);
                if direction == TriggerUpdate::ViewToViewModel {
                    view.set_caption(EDITED.to_string());
                } else {
                    model.set_caption(EDITED.to_string());
                }

                signal.on_next(0);
                assert_eq!(view.caption(), EDITED, "{direction:?}, stream {has_stream}, converters {converters}");
                assert_eq!(model.caption(), EDITED, "{direction:?}, stream {has_stream}, converters {converters}");
                binding.dispose();

                model.set_caption(INITIAL.to_string());
                signal.on_next(0);
                assert_eq!(view.caption(), EDITED);
                assert!(!signal.has_observers());
            }
        }
    }
}
