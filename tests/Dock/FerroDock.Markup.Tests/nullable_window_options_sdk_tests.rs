//! Native options application retains the original null rejection and live setter behavior.
use crate::markup_extension_tests::run;
use ferrodock_model::core::*;
use ferrodock_model_ferro::DockWindowOptionsMetadata;
use ferrodock_model_reactiveui::core::DockWindow;
use ferroui_base::{data::core::ValueType, metadata::MarkupTyped, BoxedValue};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::{cell::RefCell, rc::Rc};
#[test]
fn native_apply_to_null_retains_original_argument_error_and_options_identity() {
    run(|| {
        let options = DockWindowOptions::new();
        let parent = DockWindow::new();
        options.set_parent_window(Some(parent.clone()));
        options.set_is_modal(true);
        let value = Rc::new(options.clone()) as BoxedValue;
        let metadata = <dyn DockWindowOptionsMetadata as MarkupTyped>::MARKUP;
        let method = metadata.find_methods("ApplyTo").next().unwrap();
        assert_eq!(
            (method.parameters[0])(),
            ValueType::of::<Option<Rc<dyn IDockWindow>>>()
        );
        let error = (method.invoke)(&[Some(value), None]).expect_err("source null argument error");
        assert!(error.to_string().contains("Parameter 'window'"));
        assert!(options.is_modal());
        assert!(Rc::ptr_eq(
            &options.parent_window().unwrap(),
            &(parent as Rc<dyn IDockWindow>)
        ));
    });
}
#[test]
fn native_raw_and_optional_window_arguments_apply_live_callback_changes_to_the_same_models() {
    run(|| {
        for raw in [false, true] {
            for remove in [false, true] {
                let options = DockWindowOptions::new();
                let first = DockWindow::new();
                let second = DockWindow::new();
                let window = DockWindow::new();
                options.set_owner_mode(DockWindowOwnerMode::DockableWindow);
                options.set_parent_window(Some(first));
                options.set_is_modal(true);
                options.set_show_in_taskbar(Some(false));
                window.set_show_in_taskbar(Some(true));
                let changes = Rc::new(RefCell::new(Vec::new()));
                let log = changes.clone();
                let live = options.clone();
                let replacement = second.clone();
                window.property_changed().add(Rc::new(move |name| {
                    log.borrow_mut().push(name.to_owned());
                    match name {
                        "OwnerMode" => {
                            live.set_owner_mode(DockWindowOwnerMode::ParentWindow);
                            live.set_parent_window(Some(replacement.clone()));
                            live.set_is_modal(false);
                        }
                        "ParentWindow" => live.set_is_modal(true),
                        "IsModal" => {
                            live.set_show_in_taskbar(if remove { None } else { Some(false) })
                        }
                        _ => {}
                    }
                }));
                let window_arg = if raw {
                    Rc::new(window.clone() as Rc<dyn IDockWindow>) as BoxedValue
                } else {
                    Rc::new(Some(window.clone() as Rc<dyn IDockWindow>)) as BoxedValue
                };
                let method = <dyn DockWindowOptionsMetadata as MarkupTyped>::MARKUP
                    .find_methods("ApplyTo")
                    .next()
                    .unwrap();
                (method.invoke)(&[Some(Rc::new(options.clone())), Some(window_arg)]).unwrap();
                let mut expected = vec!["OwnerMode", "ParentWindow", "IsModal"];
                if !remove {
                    expected.push("ShowInTaskbar");
                }
                assert_eq!(&*changes.borrow(), &expected);
                assert_eq!(window.owner_mode(), DockWindowOwnerMode::DockableWindow);
                assert_eq!(options.owner_mode(), DockWindowOwnerMode::ParentWindow);
                assert!(window.is_modal());
                assert_eq!(window.show_in_taskbar(), Some(remove));
                assert!(Rc::ptr_eq(
                    &window.parent_window().unwrap(),
                    &(second as Rc<dyn IDockWindow>)
                ));
                assert!(Rc::ptr_eq(
                    &options.parent_window().unwrap(),
                    &window.parent_window().unwrap()
                ));
            }
        }
    });
}
