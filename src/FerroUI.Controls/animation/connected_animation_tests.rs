//! The connected animation tests of the reference test suite.

use super::{
    BasicConnectedAnimationConfiguration, ConnectedAnimationConfiguration, ConnectedAnimationService,
    DirectConnectedAnimationConfiguration, GravityConnectedAnimationConfiguration,
};
use crate::test_support::test_scope;
use crate::Border;
use ferroui_base::animation::easings::{Easing, LinearEasing};
use ferroui_base::animation::TimeSpan;
use ferroui_base::{Ref, Visual};
use std::cell::Cell;
use std::rc::Rc;

mod connected_animation_configuration_tests {
    use super::*;

    #[test]
    fn gravity_config_is_shadow_enabled_default_is_true() {
        let _scope = test_scope();
        let config = GravityConnectedAnimationConfiguration::new();
        assert!(config.is_shadow_enabled());
    }

    #[test]
    fn gravity_config_is_shadow_enabled_can_be_set_false() {
        let _scope = test_scope();
        let config = GravityConnectedAnimationConfiguration::new().with_is_shadow_enabled(false);
        assert!(!config.is_shadow_enabled());
    }

    #[test]
    fn gravity_config_is_shadow_enabled_round_trips() {
        let _scope = test_scope();
        let config = GravityConnectedAnimationConfiguration::new().with_is_shadow_enabled(true);
        assert!(config.is_shadow_enabled());
        config.set_is_shadow_enabled(false);
        assert!(!config.is_shadow_enabled());
    }

    #[test]
    fn gravity_config_is_connected_animation_configuration() {
        let _scope = test_scope();
        let config = GravityConnectedAnimationConfiguration::new();
        let _: Rc<dyn ConnectedAnimationConfiguration> = config;
    }

    #[test]
    fn direct_config_duration_default_is_null() {
        let _scope = test_scope();
        let config = DirectConnectedAnimationConfiguration::new();
        assert_eq!(None, config.duration());
    }

    #[test]
    fn direct_config_duration_round_trips() {
        let _scope = test_scope();
        let d = TimeSpan::from_milliseconds(200.0);
        let config = DirectConnectedAnimationConfiguration::new().with_duration(Some(d));
        assert_eq!(Some(d), config.duration());
    }

    #[test]
    fn direct_config_duration_can_be_set_to_null() {
        let _scope = test_scope();
        let config =
            DirectConnectedAnimationConfiguration::new().with_duration(Some(TimeSpan::from_milliseconds(100.0)));
        config.set_duration(None);
        assert_eq!(None, config.duration());
    }

    #[test]
    fn direct_config_is_connected_animation_configuration() {
        let _scope = test_scope();
        let config = DirectConnectedAnimationConfiguration::new();
        let _: Rc<dyn ConnectedAnimationConfiguration> = config;
    }

    #[test]
    fn basic_config_is_connected_animation_configuration() {
        let _scope = test_scope();
        let config = BasicConnectedAnimationConfiguration::new();
        let _: Rc<dyn ConnectedAnimationConfiguration> = config;
    }

    #[test]
    fn basic_config_is_instantiable() {
        let _scope = test_scope();
        let config: Rc<dyn ConnectedAnimationConfiguration> = BasicConnectedAnimationConfiguration::new();
        assert!(config.as_any().is::<BasicConnectedAnimationConfiguration>());
    }
}

fn create_service() -> Ref<ConnectedAnimationService> {
    ConnectedAnimationService::new()
}

fn border() -> Ref<Visual> {
    Border::new().upcast()
}

mod connected_animation_service_tests {
    use super::*;

    // `GetForTopLevel_NullTopLevel_ThrowsArgumentNullException`,
    // `PrepareToAnimate_NullKey_ThrowsArgumentNullException` and
    // `PrepareToAnimate_NullSource_ThrowsArgumentNullException` have no
    // counterpart: a top level, a key and a source cannot be null.

    #[test]
    fn default_duration_initially_is_300ms() {
        let _scope = test_scope();
        let service = create_service();
        assert_eq!(TimeSpan::from_milliseconds(300.0), service.default_duration());
    }

    #[test]
    fn default_easing_function_initially_null() {
        let _scope = test_scope();
        let service = create_service();
        assert!(service.default_easing_function().is_none());
    }

    #[test]
    fn default_duration_can_be_set() {
        let _scope = test_scope();
        let service = create_service();
        let d = TimeSpan::from_milliseconds(500.0);
        service.set_default_duration(d);
        assert_eq!(d, service.default_duration());
    }

    #[test]
    fn default_easing_function_can_be_set() {
        let _scope = test_scope();
        let service = create_service();
        let easing = Easing::new(LinearEasing::new());
        service.set_default_easing_function(Some(easing.clone()));
        assert!(service.default_easing_function() == Some(easing));
    }

    #[test]
    fn get_animation_unknown_key_returns_null() {
        let _scope = test_scope();
        let service = create_service();
        assert!(service.get_animation("nonexistent").is_none());
    }

    #[test]
    #[should_panic(expected = "The value cannot be an empty string. (Parameter 'key')")]
    fn prepare_to_animate_empty_key_throws_argument_exception() {
        let _scope = test_scope();
        let service = create_service();
        let source = border();
        service.prepare_to_animate("", &source);
    }

    #[test]
    fn prepare_to_animate_returns_animation_with_matching_key() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        assert_eq!("hero", animation.key());
    }

    #[test]
    fn prepare_to_animate_animation_initially_not_consumed() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        assert!(!animation.is_consumed());
    }

    #[test]
    fn get_animation_after_prepare_returns_animation() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        assert!(service.get_animation("hero").is_some_and(|a| Rc::ptr_eq(&a, &animation)));
    }

    #[test]
    fn get_animation_returns_same_instance_for_same_key() {
        let _scope = test_scope();
        let service = create_service();
        service.prepare_to_animate("hero", &border());
        let a1 = service.get_animation("hero").expect("an animation");
        let a2 = service.get_animation("hero").expect("an animation");
        assert!(Rc::ptr_eq(&a1, &a2));
    }

    #[test]
    fn prepare_to_animate_same_key_replaces_old_animation() {
        let _scope = test_scope();
        let service = create_service();
        let first = service.prepare_to_animate("hero", &border());
        let second = service.prepare_to_animate("hero", &border());

        assert!(!Rc::ptr_eq(&first, &second));
        assert!(first.is_consumed() || first.is_disposed());
        assert!(service.get_animation("hero").is_some_and(|a| Rc::ptr_eq(&a, &second)));
    }

    #[test]
    fn prepare_to_animate_different_keys_both_in_service() {
        let _scope = test_scope();
        let service = create_service();
        let a1 = service.prepare_to_animate("key1", &border());
        let a2 = service.prepare_to_animate("key2", &border());

        assert!(service.get_animation("key1").is_some_and(|a| Rc::ptr_eq(&a, &a1)));
        assert!(service.get_animation("key2").is_some_and(|a| Rc::ptr_eq(&a, &a2)));
    }
}

mod connected_animation_tests {
    use super::*;

    #[test]
    fn key_matches_prepared_key() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("myKey", &border());
        assert_eq!("myKey", animation.key());
    }

    #[test]
    fn is_consumed_initially_false() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        assert!(!animation.is_consumed());
    }

    #[test]
    fn configuration_initially_null() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        assert!(animation.configuration().is_none());
    }

    #[test]
    fn configuration_gravity_round_trips() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        let config: Rc<dyn ConnectedAnimationConfiguration> =
            GravityConnectedAnimationConfiguration::new().with_is_shadow_enabled(false);
        animation.set_configuration(Some(config.clone()));
        assert!(animation.configuration().is_some_and(|c| Rc::ptr_eq(&c, &config)));
    }

    #[test]
    fn configuration_direct_round_trips() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        let config: Rc<dyn ConnectedAnimationConfiguration> =
            DirectConnectedAnimationConfiguration::new().with_duration(Some(TimeSpan::from_milliseconds(100.0)));
        animation.set_configuration(Some(config.clone()));
        assert!(animation.configuration().is_some_and(|c| Rc::ptr_eq(&c, &config)));
    }

    #[test]
    fn configuration_basic_round_trips() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        let config: Rc<dyn ConnectedAnimationConfiguration> = BasicConnectedAnimationConfiguration::new();
        animation.set_configuration(Some(config.clone()));
        assert!(animation.configuration().is_some_and(|c| Rc::ptr_eq(&c, &config)));
    }

    #[test]
    fn try_start_returns_true_when_not_consumed() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        let result = animation.try_start(&border());
        assert!(result);
    }

    #[test]
    fn try_start_consumes_animation() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        animation.try_start(&border());
        assert!(animation.is_consumed());
    }

    #[test]
    fn try_start_second_call_returns_false() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        animation.try_start(&border());
        let second = animation.try_start(&border());
        assert!(!second);
    }

    #[test]
    fn try_start_returns_false_when_disposed() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        animation.dispose();
        let result = animation.try_start(&border());
        assert!(!result);
    }

    #[test]
    fn try_start_with_empty_coordinated_elements_returns_true() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        let result = animation.try_start_with_coordinated_elements(&border(), &[]);
        assert!(result);
    }

    #[test]
    fn try_start_when_no_top_level_fires_completed() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());

        let received = Rc::new(Cell::new(None));
        let _ = animation.completed({
            let received = received.clone();
            move |e| received.set(Some(*e))
        });

        animation.try_start(&border()); // no top level ancestor

        assert!(received.get().is_some());
    }

    #[test]
    fn try_start_when_no_top_level_completed_not_cancelled() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());

        let cancelled = Rc::new(Cell::new(None));
        let _ = animation.completed({
            let cancelled = cancelled.clone();
            move |e| cancelled.set(Some(e.cancelled()))
        });

        animation.try_start(&border());

        assert_eq!(Some(false), cancelled.get());
    }

    #[test]
    fn try_start_when_no_top_level_removes_from_service() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());

        animation.try_start(&border());

        assert!(service.get_animation("hero").is_none());
    }

    #[test]
    fn try_start_with_coordinated_elements_when_no_top_level_fires_completed() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        let coordinated = [border(), border()];

        let fired = Rc::new(Cell::new(false));
        let _ = animation.completed({
            let fired = fired.clone();
            move |_| fired.set(true)
        });

        animation.try_start_with_coordinated_elements(&border(), &coordinated);

        assert!(fired.get());
    }

    #[test]
    fn dispose_removes_from_service() {
        let _scope = test_scope();
        let service = create_service();
        service.prepare_to_animate("hero", &border());
        assert!(service.get_animation("hero").is_some());

        service.get_animation("hero").expect("an animation").dispose();

        assert!(service.get_animation("hero").is_none());
    }

    #[test]
    fn dispose_called_twice_is_no_op() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());

        animation.dispose();
        animation.dispose(); // must not panic
    }

    #[test]
    fn dispose_when_not_mid_flight_does_not_fire_completed() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());

        let fired = Rc::new(Cell::new(false));
        let _ = animation.completed({
            let fired = fired.clone();
            move |_| fired.set(true)
        });

        animation.dispose();

        assert!(!fired.get());
    }

    #[test]
    fn get_animation_returns_null_after_try_start() {
        let _scope = test_scope();
        let service = create_service();
        service.prepare_to_animate("hero", &border());
        service.get_animation("hero").expect("an animation").try_start(&border());
        assert!(service.get_animation("hero").is_none());
    }

    #[test]
    fn get_animation_returns_null_after_dispose() {
        let _scope = test_scope();
        let service = create_service();
        service.prepare_to_animate("hero", &border());
        service.get_animation("hero").expect("an animation").dispose();
        assert!(service.get_animation("hero").is_none());
    }

    #[test]
    fn get_animation_returns_null_when_animation_is_consumed() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        animation.try_start(&border());
        assert!(service.get_animation("hero").is_none());
    }

    #[test]
    fn multiple_animations_independent_lifecycles() {
        let _scope = test_scope();
        let service = create_service();
        let a1 = service.prepare_to_animate("anim1", &border());
        let a2 = service.prepare_to_animate("anim2", &border());

        a1.try_start(&border());

        assert!(a1.is_consumed());
        assert!(!a2.is_consumed());
        assert!(service.get_animation("anim1").is_none());
        assert!(service.get_animation("anim2").is_some());
    }

    #[test]
    fn prepare_to_animate_after_dispose_can_prepare_again() {
        let _scope = test_scope();
        let service = create_service();
        let first = service.prepare_to_animate("hero", &border());
        first.dispose();

        let second = service.prepare_to_animate("hero", &border());
        assert!(!Rc::ptr_eq(&first, &second));
        assert!(service.get_animation("hero").is_some_and(|a| Rc::ptr_eq(&a, &second)));
    }

    // The timing and easing resolution of the configurations.

    #[test]
    fn configuration_direct_with_duration_uses_provided_duration() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        animation.set_configuration(Some(
            DirectConnectedAnimationConfiguration::new().with_duration(Some(TimeSpan::from_milliseconds(250.0))),
        ));

        let resolved = animation.resolve_timing_and_easing(&service);

        assert_eq!(TimeSpan::from_milliseconds(250.0), resolved.duration);
    }

    #[test]
    fn configuration_direct_with_null_duration_uses_service_default_duration() {
        let _scope = test_scope();
        let service = create_service();
        service.set_default_duration(TimeSpan::from_milliseconds(400.0));
        let animation = service.prepare_to_animate("hero", &border());
        animation.set_configuration(Some(DirectConnectedAnimationConfiguration::new().with_duration(None)));

        let resolved = animation.resolve_timing_and_easing(&service);

        assert_eq!(TimeSpan::from_milliseconds(400.0), resolved.duration);
    }

    #[test]
    fn configuration_basic_uses_service_default_duration() {
        let _scope = test_scope();
        let service = create_service();
        service.set_default_duration(TimeSpan::from_milliseconds(400.0));
        let animation = service.prepare_to_animate("hero", &border());
        animation.set_configuration(Some(BasicConnectedAnimationConfiguration::new()));

        let resolved = animation.resolve_timing_and_easing(&service);

        assert_eq!(TimeSpan::from_milliseconds(400.0), resolved.duration);
    }

    #[test]
    fn configuration_gravity_uses_service_default_duration() {
        let _scope = test_scope();
        let service = create_service();
        service.set_default_duration(TimeSpan::from_milliseconds(350.0));
        let animation = service.prepare_to_animate("hero", &border());
        animation.set_configuration(Some(GravityConnectedAnimationConfiguration::new()));

        let resolved = animation.resolve_timing_and_easing(&service);

        assert_eq!(TimeSpan::from_milliseconds(350.0), resolved.duration);
    }

    #[test]
    fn configuration_gravity_use_gravity_dip_is_true() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        animation.set_configuration(Some(GravityConnectedAnimationConfiguration::new()));

        let resolved = animation.resolve_timing_and_easing(&service);

        assert!(resolved.use_gravity_dip);
    }

    #[test]
    fn configuration_direct_use_gravity_dip_is_false() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        animation.set_configuration(Some(DirectConnectedAnimationConfiguration::new()));

        let resolved = animation.resolve_timing_and_easing(&service);

        assert!(!resolved.use_gravity_dip);
    }

    #[test]
    fn configuration_basic_use_gravity_dip_is_false() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        animation.set_configuration(Some(BasicConnectedAnimationConfiguration::new()));

        let resolved = animation.resolve_timing_and_easing(&service);

        assert!(!resolved.use_gravity_dip);
    }

    #[test]
    fn configuration_gravity_is_shadow_enabled_false_use_shadow_is_false() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        animation.set_configuration(Some(GravityConnectedAnimationConfiguration::new().with_is_shadow_enabled(false)));

        let resolved = animation.resolve_timing_and_easing(&service);

        assert!(!resolved.use_shadow);
    }

    #[test]
    fn configuration_gravity_is_shadow_enabled_true_use_shadow_is_true() {
        let _scope = test_scope();
        let service = create_service();
        let animation = service.prepare_to_animate("hero", &border());
        animation.set_configuration(Some(GravityConnectedAnimationConfiguration::new().with_is_shadow_enabled(true)));

        let resolved = animation.resolve_timing_and_easing(&service);

        assert!(resolved.use_shadow);
    }
}

// --- added tests (not in the reference suite) ---------------------------------

mod overlay_flight {
    use super::*;
    use crate::primitives::OverlayLayer;
    use crate::testing::{TestServices, UnitTestApplication};
    use crate::{Control, StackPanel, Window};
    use ferroui_base::animation::{IClock, IGlobalClock, PlayState};
    use ferroui_base::layout::ILayoutManager;
    use ferroui_base::media::Brushes;
    use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::{FerroLocator, LocatorExtensions};

    /// The global clock of the test: pulsed by the test.
    struct TestClock {
        subject: LightweightSubject<TimeSpan>,
        play_state: Cell<PlayState>,
    }

    impl IObservable<TimeSpan> for TestClock {
        fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
            self.subject.subscribe(observer)
        }
    }

    impl IClock for TestClock {
        fn play_state(&self) -> PlayState {
            self.play_state.get()
        }

        fn set_play_state(&self, value: PlayState) {
            self.play_state.set(value)
        }
    }

    impl IGlobalClock for TestClock {}

    fn sized_border(width: f64, height: f64) -> Ref<Border> {
        let border = Border::new();
        border.set_width(width);
        border.set_height(height);
        border
    }

    #[test]
    fn try_start_in_a_window_flies_a_proxy_over_the_overlay_layer() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let clock = Rc::new(TestClock { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) });
        let global_clock: Rc<dyn IGlobalClock> = clock.clone();
        FerroLocator::current_mutable().bind::<dyn IGlobalClock>().to_constant(global_clock);

        let source = sized_border(50.0, 50.0);
        source.set_background(Some(Brushes::red()));
        let destination = sized_border(100.0, 80.0);
        destination.set_background(Some(Brushes::blue()));
        let coordinated = sized_border(10.0, 10.0);
        let panel = StackPanel::new();
        panel.children().add(source.clone());
        panel.children().add(destination.clone());
        panel.children().add(coordinated.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(&panel)));
        window.show();
        window.layout_manager().execute_initial_layout_pass();
        let overlay_layer = OverlayLayer::get_overlay_layer(&window).expect("the window has an overlay layer");

        let service = ConnectedAnimationService::get_for_top_level(&window);
        assert!(service == ConnectedAnimationService::get_for_top_level(&window));
        let animation = service.prepare_to_animate("hero", &source);
        let completed = Rc::new(Cell::new(None));
        let _ = animation.completed({
            let completed = completed.clone();
            move |e| completed.set(Some(e.cancelled()))
        });

        assert!(animation.try_start_with_coordinated_elements(&destination, &[coordinated.clone().upcast()]));
        Dispatcher::ui_thread().run_jobs(None);

        // In flight: the proxy is over the overlay layer, the destination and
        // the coordinated element are hidden.
        assert_eq!(1, overlay_layer.children().count());
        assert_eq!(0.0, destination.opacity());
        assert_eq!(0.0, coordinated.opacity());

        for milliseconds in [0.0, 100.0, 200.0, 300.0, 400.0] {
            clock.subject.on_next(TimeSpan::from_milliseconds(milliseconds));
            Dispatcher::ui_thread().run_jobs(None);
        }

        assert_eq!(Some(false), completed.get());
        assert_eq!(0, overlay_layer.children().count());
        assert_eq!(1.0, destination.opacity());
        assert_eq!(1.0, coordinated.opacity());
        assert!(service.get_animation("hero").is_none());
    }

    #[test]
    fn dispose_in_flight_restores_the_destination_and_reports_cancellation() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let clock = Rc::new(TestClock { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) });
        let global_clock: Rc<dyn IGlobalClock> = clock.clone();
        FerroLocator::current_mutable().bind::<dyn IGlobalClock>().to_constant(global_clock);

        let source = sized_border(50.0, 50.0);
        let destination = sized_border(100.0, 80.0);
        let panel = StackPanel::new();
        panel.children().add(source.clone());
        panel.children().add(destination.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(&panel)));
        window.show();
        window.layout_manager().execute_initial_layout_pass();
        let overlay_layer = OverlayLayer::get_overlay_layer(&window).expect("the window has an overlay layer");

        let service = ConnectedAnimationService::get_for_top_level(&window);
        let animation = service.prepare_to_animate("hero", &source);
        let cancelled = Rc::new(Cell::new(None));
        let _ = animation.completed({
            let cancelled = cancelled.clone();
            move |e| {
                if cancelled.get().is_none() {
                    cancelled.set(Some(e.cancelled()));
                }
            }
        });

        assert!(animation.try_start(&destination));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(1, overlay_layer.children().count());

        animation.dispose();

        assert_eq!(Some(true), cancelled.get());
        assert_eq!(0, overlay_layer.children().count());
        assert_eq!(1.0, destination.opacity());
    }
}
