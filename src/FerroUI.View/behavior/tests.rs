use super::*;
use crate::prelude::*;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::Dispatcher;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Border, Button, Control, Window};

#[test]
fn native_behavior_resolves_named_event_sources_and_orders_actions() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let owner = Owner::new_root();
    let view = owner.with(|| {
        rsx! {
            StackPanel {
                behaviors: [
                    event("Click", Some("Approve"), [
                        change_property(Some("Card"), "Width", "140"),
                        change_property(Some("Card"), "Width", "220"),
                        add_class(Some("Card"), "approved"),
                    ]),
                ],
                Border { name: "Card", width: 100.0, height: 48.0 }
                Button { name: "Approve", content: "Approve" }
            }
        }
        .mount()
    });
    let window = Window::new();
    window.set_content(Some(Control::boxed(&view)));
    window.show();
    let card = view
        .get_logical_descendants()
        .find_map(|control| control.cast::<Border>())
        .unwrap();
    let button = view
        .get_logical_descendants()
        .find_map(|control| control.cast::<Button>())
        .unwrap();
    assert_eq!(card.width(), 100.0);
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(card.width(), 220.0);
    assert!(card.classes().contains("approved"));
    window.close();
    owner.dispose();
}

#[test]
fn native_behavior_data_condition_uses_context_and_restores_the_property() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let owner = Owner::new_root();
    let view = owner.with(|| {
        rsx! {
            Border {
                width: 120.0,
                height: 40.0,
                behaviors: [
                    data(".", Comparison::Equal, "True", true, [
                        change_property(None, "Width", "240"),
                    ]),
                ],
            }
        }
        .mount()
    });
    view.set_data_context(Some(Rc::new(false)));
    let window = Window::new();
    window.set_content(Some(Control::boxed(&view)));
    window.show();
    let card = view.cast::<Border>().unwrap();
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(card.width(), 120.0);
    card.set_data_context(Some(Rc::new(true)));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(card.width(), 240.0);
    card.set_data_context(Some(Rc::new(false)));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(card.width(), 120.0);
    window.close();
    owner.dispose();
}
