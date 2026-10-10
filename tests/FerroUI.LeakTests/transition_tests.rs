//! Port of the transition tests of the reference leak tests: the instance of
//! a transition that ran, and the controls a shared collection of
//! transitions was applied to, are freed.

use crate::leak::Tracked;
use crate::services::{collect_garbage_loaded, styled_window, MockGlobalClock};
use ferroui_base::animation::{
    Animatable, DoubleTransition, ITransition, TimeSpan, TransformOperationsTransition, Transitions,
};
use ferroui_base::controls::ResourceKey;
use ferroui_base::styling::{ControlTheme, Setter};
use ferroui_base::{AnyValue, Ref, Visual};
use ferroui_controls::testing::UnitTestApplication;
use ferroui_controls::{Application, Border, Button, Control, UserControl, Window};
use std::rc::Rc;

/// A collection with a transition of the render transform, as the themes
/// share one between the controls of a class, and a control theme for
/// buttons that sets it.
fn theme_with_shared_transitions() -> Ref<ControlTheme> {
    let transition: Rc<dyn ITransition> = TransformOperationsTransition::with_property(
        Visual::render_transform_property().as_property(),
        TimeSpan::from_seconds(0.750),
    )
    .into();
    let shared_transitions = Transitions::from_items([transition]);

    // What the resources of the application hold for the class, if
    // anything: the themes of the application are among its styles.
    let based_on = Application::current()
        .and_then(|application| application.resources().get(&ResourceKey::Type(Button::TYPE)))
        .and_then(|value| {
            let value: &dyn AnyValue = &*value;
            value.downcast_ref::<Ref<ControlTheme>>().cloned()
        });

    let control_theme =
        ControlTheme::with_setters(Button::TYPE, [Setter::new(Animatable::transitions_property(), Some(shared_transitions))]);
    control_theme.set_based_on(based_on);
    control_theme
}

#[test]
fn transition_on_styled_property_is_freed() {
    let clock = MockGlobalClock::new();
    let _app = UnitTestApplication::start(styled_window(clock.clone()));

    let transition_instance = {
        let opacity_transition: Rc<dyn ITransition> =
            DoubleTransition::with_property(Visual::opacity_property().as_property(), TimeSpan::from_seconds(1.0)).into();

        let border = Border::new();
        border.set_transitions(Some(Transitions::from_items([opacity_transition.clone()])));
        let window = Window::new();
        window.set_content(Some(Control::boxed(&border)));
        window.show();

        border.set_opacity(0.0);

        clock.pulse(TimeSpan::from_seconds(0.0));
        clock.pulse(TimeSpan::from_seconds(0.5));

        assert_eq!(0.5, border.opacity());

        let transition_instance =
            border.try_get_transition_instance(&opacity_transition).expect("the transition has an instance while it runs");

        clock.pulse(TimeSpan::from_seconds(1.0));

        assert_eq!(0.0, border.opacity());

        window.close();

        let tracked = Tracked::shared("the instance of the transition", &transition_instance);
        tracked.assert_alive();
        tracked
    };

    collect_garbage_loaded();

    transition_instance.assert_freed();
}

#[test]
fn shared_transition_collection_is_not_leaking() {
    let clock = MockGlobalClock::new();
    let _app = UnitTestApplication::start(styled_window(clock.clone()));

    // The themes share collections of transitions, so the scenario matters.
    let control_theme = theme_with_shared_transitions();

    let button = {
        let button = Button::new();
        button.set_theme(&control_theme);
        let window = Window::new();
        window.set_content(Some(Control::boxed(&button)));
        window.show();
        window.set_content(None);
        window.close();

        Tracked::object("the button", &button)
    };

    collect_garbage_loaded();

    button.assert_freed();
}

#[test]
fn lazily_created_control_should_not_leak_transitions() {
    let clock = MockGlobalClock::new();
    let _app = UnitTestApplication::start(styled_window(clock.clone()));

    let control_theme = theme_with_shared_transitions();

    let button = {
        let window = Window::new();
        window.show();
        let button = Button::new();
        button.set_theme(&control_theme);
        let user_control = UserControl::new();
        user_control.set_content(Some(Control::boxed(&button)));
        // A control that is not visible does not attach its content to the
        // visual tree.
        user_control.set_is_visible(false);
        window.set_content(Some(Control::boxed(&user_control)));
        window.set_content(None);
        window.close();

        Tracked::object("the button", &button)
    };

    collect_garbage_loaded();

    button.assert_freed();
}
