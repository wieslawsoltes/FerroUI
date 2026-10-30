use crate::timeline_tests::run;
use crate::{
    DeferredContentControl as Host, DeferredContentPresentationSettings as Settings,
    DeferredContentPresentationTimeline as Timeline, DeferredContentPresenter as Presenter,
    DeferredContentScheduling as Scheduling, IDeferredContentPresentation,
};
use ferroui_base::animation::TimeSpan;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Ref, Size};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::templates::{
    FuncControlTemplate, IDataTemplate, IRecyclingDataTemplate, ITemplateWithParam,
};
use ferroui_controls::{Control, Panel, TextBlock, Window};
use std::cell::Cell;
use std::rc::Rc;
struct CountingTemplate {
    builds: Cell<usize>,
    delay: Cell<std::time::Duration>,
}
impl CountingTemplate {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            builds: Cell::new(0),
            delay: Cell::new(std::time::Duration::ZERO),
        })
    }
}
impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for CountingTemplate {
    fn build(&self, data: &Option<BoxedValue>) -> Option<Ref<Control>> {
        self.build_with_existing(data.as_ref(), None)
    }
}
impl IDataTemplate for CountingTemplate {
    fn match_(&self, _: Option<&BoxedValue>) -> bool {
        true
    }
    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        Some(self)
    }
}
impl IRecyclingDataTemplate for CountingTemplate {
    fn build_with_existing(
        &self,
        _: Option<&BoxedValue>,
        existing: Option<Ref<Control>>,
    ) -> Option<Ref<Control>> {
        if let Some(existing) = existing {
            return Some(existing);
        }
        if !self.delay.get().is_zero() {
            std::thread::sleep(self.delay.get());
        }
        self.builds.set(self.builds.get() + 1);
        Some(TextBlock::new().upcast())
    }
}
struct Manual(Rc<Timeline>);
impl Manual {
    fn new() -> Self {
        let t = Settings::default_timeline();
        assert_eq!(t.pending_count(), 0);
        t.set_auto_schedule(false);
        t.set_reveal_duration(TimeSpan::ZERO);
        Self(t)
    }
}
impl Drop for Manual {
    fn drop(&mut self) {
        self.0.set_auto_schedule(true);
        self.0.set_max_presentations_per_pass(8);
        self.0
            .set_reveal_duration(TimeSpan::from_milliseconds(90.0));
    }
}
struct Scene(Ref<Window>);
impl Scene {
    fn new(control: Ref<Control>) -> Self {
        let w = Window::new();
        w.set_width(800.0);
        w.set_height(600.0);
        w.set_content(Some(Control::boxed(control)));
        w.show();
        w.update_layout();
        Self(w)
    }
    fn drain(&self, timeline: &Timeline) {
        Dispatcher::ui_thread().run_jobs(None);
        self.0.update_layout();
        timeline.flush_pending_batch_for_testing();
        Dispatcher::ui_thread().run_jobs(None);
        self.0.update_layout();
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.0.close();
    }
}
fn boxed(text: &str) -> BoxedValue {
    Rc::new(text.to_owned())
}
fn child_context(control: &Host) -> String {
    let value = control
        .presenter()
        .unwrap()
        .child()
        .unwrap()
        .data_context()
        .unwrap();
    value.downcast_ref::<String>().unwrap().clone()
}
fn standard(control: &Host) {
    control.set_template(Some(FuncControlTemplate::for_type::<Host>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_owned()));
        scope.register("PART_ContentPresenter", presenter.clone().upcast());
        presenter.upcast()
    })));
}
#[test]
fn original_host_defers_materialization_until_dispatcher_batch() {
    run(|| {
        let manual = Manual::new();
        let template = CountingTemplate::new();
        let control = Host::new();
        control.set_content(Some(boxed("Deferred")));
        control.set_content_template(Some(template.clone()));
        let scene = Scene::new(control.clone().upcast());
        control.apply_template();
        scene.0.update_layout();
        assert!(control.presenter().is_some());
        assert!(control.presenter().unwrap().child().is_none());
        assert_eq!(template.builds.get(), 0);
        scene.drain(&manual.0);
        assert!(control.presenter().unwrap().child().is_some());
        assert_eq!(template.builds.get(), 1);
    });
}
#[test]
fn original_host_batches_changes_into_one_build_of_latest_context() {
    run(|| {
        let manual = Manual::new();
        let template = CountingTemplate::new();
        let control = Host::new();
        control.set_content_template(Some(template.clone()));
        let scene = Scene::new(control.clone().upcast());
        control.apply_template();
        control.set_content(Some(boxed("First")));
        control.set_content(Some(boxed("Second")));
        scene.0.update_layout();
        assert_eq!(template.builds.get(), 0);
        assert!(control.presenter().unwrap().child().is_none());
        scene.drain(&manual.0);
        assert_eq!(template.builds.get(), 1);
        assert_eq!(child_context(&control), "Second");
    });
}
#[test]
fn original_host_detaches_and_reattaches_with_deferred_context() {
    run(|| {
        let manual = Manual::new();
        let template = CountingTemplate::new();
        let control = Host::new();
        control.set_content(Some(boxed("First")));
        control.set_content_template(Some(template.clone()));
        let first = Scene::new(control.clone().upcast());
        control.apply_template();
        first.drain(&manual.0);
        assert_eq!(template.builds.get(), 1);
        first.0.set_content(None);
        first.0.close();
        control.set_content(Some(boxed("Second")));
        assert_eq!(template.builds.get(), 1);
        let second = Scene::new(control.clone().upcast());
        second.drain(&manual.0);
        assert_eq!(template.builds.get(), 2);
        assert_eq!(child_context(&control), "Second");
    });
}
fn clear_case(standard_presenter: bool, materialize: bool) {
    run(move || {
        let manual = Manual::new();
        let template = CountingTemplate::new();
        let control = Host::new();
        let content = boxed("Initial");
        control.set_content(Some(content.clone()));
        control.set_content_template(Some(template.clone()));
        if standard_presenter {
            standard(&control);
        }
        control.clear_presentation();
        let scene = Scene::new(control.clone().upcast());
        control.apply_template();
        if materialize {
            scene.drain(&manual.0);
        }
        let child = control.presenter().unwrap().child();
        assert_eq!(child.is_some(), materialize);
        control.set_content(Some(boxed("Replacement")));
        control.clear_presentation();
        control.clear_presentation();
        assert!(control.presenter().unwrap().child().is_none());
        if let Some(child) = child {
            assert!(child.parent().is_none());
        }
        assert!(Rc::ptr_eq(
            &control.content_template().unwrap(),
            &(template.clone() as Rc<dyn IDataTemplate>)
        ));
        let builds = template.builds.get();
        scene.drain(&manual.0);
        assert_eq!(template.builds.get(), builds);
        assert!(control.presenter().unwrap().child().is_none());
        let preserved = control.content().unwrap();
        scene.0.set_content(None);
        scene.0.set_content(Some(Control::boxed(control.clone())));
        scene.drain(&manual.0);
        assert_eq!(template.builds.get(), builds + 1);
        assert!(Rc::ptr_eq(
            &preserved,
            &control
                .presenter()
                .unwrap()
                .child()
                .unwrap()
                .data_context()
                .unwrap()
        ));
        control.clear_presentation();
        control.set_content(Some(content.clone()));
        scene.drain(&manual.0);
        assert!(Rc::ptr_eq(
            &content,
            &control
                .presenter()
                .unwrap()
                .child()
                .unwrap()
                .data_context()
                .unwrap()
        ));
    });
}
#[test]
fn original_clear_default_not_materialized() {
    clear_case(false, false);
}
#[test]
fn original_clear_default_materialized() {
    clear_case(false, true);
}
#[test]
fn original_clear_standard_not_materialized() {
    clear_case(true, false);
}
#[test]
fn original_clear_standard_materialized() {
    clear_case(true, true);
}
#[test]
fn original_host_standard_presenter_fallback_defers_materialization() {
    run(|| {
        let manual = Manual::new();
        let template = CountingTemplate::new();
        let control = Host::new();
        standard(&control);
        control.set_content(Some(boxed("FallbackPresenter")));
        control.set_content_template(Some(template.clone()));
        let scene = Scene::new(control.clone().upcast());
        control.apply_template();
        scene.0.update_layout();
        assert!(control.presenter().is_some());
        assert!(control.presenter().unwrap().child().is_none());
        assert_eq!(template.builds.get(), 0);
        scene.drain(&manual.0);
        assert!(control.presenter().unwrap().child().is_some());
        assert_eq!(template.builds.get(), 1);
    });
}
#[test]
fn standalone_presenter_retains_old_child_and_measure_until_next_batch() {
    run(|| {
        let manual = Manual::new();
        let template = CountingTemplate::new();
        let presenter = Presenter::new();
        presenter.set_content_template(Some(template.clone()));
        presenter.set_content(Some(boxed("First")));
        let scene = Scene::new(presenter.clone().upcast());
        assert!(presenter.child().is_none());
        assert_eq!(presenter.desired_size(), Size::default());
        scene.drain(&manual.0);
        let child = presenter.child().unwrap();
        let size = presenter.desired_size();
        assert_eq!(template.builds.get(), 1);
        presenter.set_content(Some(boxed("Second")));
        scene.0.update_layout();
        assert_eq!(presenter.child(), Some(child.clone()));
        assert_eq!(presenter.desired_size(), size);
        assert_eq!(
            child
                .data_context()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "First"
        );
        scene.drain(&manual.0);
        assert_eq!(presenter.child(), Some(child));
        assert_eq!(template.builds.get(), 1);
        assert_eq!(
            presenter
                .child()
                .unwrap()
                .data_context()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "Second"
        );
    });
}
#[test]
fn inherited_scope_order_delay_and_independent_timelines_use_source_rules() {
    run(|| {
        let _manual = Manual::new();
        let scope = Panel::new();
        let first = Host::new();
        let second = Host::new();
        let a = Timeline::new();
        a.set_auto_schedule(false);
        a.set_max_presentations_per_pass(1);
        a.set_reveal_duration(TimeSpan::ZERO);
        let b = Timeline::new();
        b.set_auto_schedule(false);
        b.set_reveal_duration(TimeSpan::ZERO);
        Scheduling::set_timeline(&scope, Some(a.clone()));
        Scheduling::set_order(&scope, 5);
        Scheduling::set_delay(&scope, TimeSpan::from_ticks(-1));
        scope.children().add(first.clone().upcast::<Control>());
        scope.children().add(second.clone().upcast::<Control>());
        let template = CountingTemplate::new();
        for host in [&first, &second] {
            host.set_content_template(Some(template.clone()));
            host.set_content(Some(boxed("Scoped")));
        }
        Scheduling::set_order(&second, -1);
        let scene = Scene::new(scope.clone().upcast());
        assert_eq!(Scheduling::get_order(&first), 5);
        assert_eq!(Scheduling::get_delay(&first), TimeSpan::from_ticks(-1));
        assert_eq!(a.pending_count(), 2);
        scene.drain(&a);
        assert!(first.presenter().unwrap().child().is_none());
        assert!(second.presenter().unwrap().child().is_some());
        Scheduling::set_timeline(&first, Some(b.clone()));
        assert_eq!(a.pending_count(), 0);
        assert_eq!(b.pending_count(), 1);
        scene.drain(&b);
        assert!(first.presenter().unwrap().child().is_some());
        assert_eq!(Settings::default_timeline().pending_count(), 0);
    });
}
#[derive(PartialEq)]
struct Immediate;
impl IDeferredContentPresentation for Immediate {
    fn defer_content_presentation(&self) -> bool {
        false
    }
}
#[test]
fn original_content_opt_out_realizes_immediately_without_opacity_staging() {
    run(|| {
        let _manual = Manual::new();
        let template = CountingTemplate::new();
        let control = Host::new();
        let content: Rc<dyn IDeferredContentPresentation> = Rc::new(Immediate);
        control.set_content_template(Some(template.clone()));
        control.set_content(Some(Rc::new(content.clone())));
        let scene = Scene::new(control.clone().upcast());
        assert_eq!(template.builds.get(), 1);
        assert_eq!(control.presenter().unwrap().opacity(), 1.0);
        assert_eq!(Settings::default_timeline().pending_count(), 0);
        let presenter = Presenter::new();
        presenter.set_content_template(Some(template.clone()));
        presenter.set_content(Some(Rc::new(content)));
        scene.0.set_content(Some(Control::boxed(presenter.clone())));
        scene.0.update_layout();
        assert!(presenter.child().is_some());
        assert_eq!(presenter.opacity(), 1.0);
    });
}
#[test]
fn default_template_binds_source_visual_properties_and_cancels_nested_presenter_queue() {
    run(|| {
        use ferroui_base::Thickness;
        let manual = Manual::new();
        let host = Host::new();
        host.set_padding(Thickness::uniform(7.0));
        host.set_content(Some(boxed("Content")));
        let scene = Scene::new(host.clone().upcast());
        let presenter = host.presenter().unwrap();
        assert!(presenter.cast::<Presenter>().is_some());
        assert_eq!(presenter.padding(), Thickness::uniform(7.0));
        assert_eq!(manual.0.pending_count(), 1);
        host.set_padding(Thickness::uniform(11.0));
        assert_eq!(presenter.padding(), Thickness::uniform(11.0));
        scene.drain(&manual.0);
        assert_eq!(manual.0.pending_count(), 0);
    });
}

#[test]
fn reveal_skips_first_materialization_reuses_own_transition_and_preserves_user_transition() {
    run(|| {
        use ferroui_base::animation::{DoubleTransition, Transitions};
        use ferroui_base::Visual;
        let manual = Manual::new();
        manual
            .0
            .set_reveal_duration(TimeSpan::from_milliseconds(90.0));
        let template = CountingTemplate::new();
        let host = Host::new();
        host.set_content_template(Some(template));
        host.set_content(Some(boxed("First")));
        let scene = Scene::new(host.clone().upcast());
        scene.drain(&manual.0);
        let presenter = host.presenter().unwrap();
        assert_eq!(presenter.opacity(), 1.0);
        assert!(presenter.transitions().is_none());
        host.set_content(Some(boxed("Second")));
        manual.0.flush_pending_batch_for_testing();
        let transitions = presenter.transitions().unwrap();
        assert_eq!(transitions.count(), 1);
        let transition = transitions
            .get(0)
            .as_object()
            .unwrap()
            .cast::<DoubleTransition>()
            .unwrap();
        assert_eq!(transition.duration(), TimeSpan::from_milliseconds(90.0));
        assert_eq!(
            transition.property(),
            Some(Visual::opacity_property().as_property())
        );
        manual
            .0
            .set_reveal_duration(TimeSpan::from_milliseconds(120.0));
        host.set_content(Some(boxed("Third")));
        manual.0.flush_pending_batch_for_testing();
        assert_eq!(transitions.count(), 1);
        assert_eq!(transition.duration(), TimeSpan::from_milliseconds(120.0));
        let user = DoubleTransition::with_property(
            Visual::opacity_property().as_property(),
            TimeSpan::from_milliseconds(700.0),
        );
        let user_list = Transitions::from_items([user.clone().into()]);
        presenter.set_transitions(Some(user_list.clone()));
        host.set_content(Some(boxed("Fourth")));
        manual.0.flush_pending_batch_for_testing();
        assert_eq!(user_list.count(), 1);
        assert_eq!(user.duration(), TimeSpan::from_milliseconds(700.0));
        assert!(presenter.transitions().unwrap().ptr_eq(&user_list));
        scene.drain(&manual.0);
    });
}
#[test]
fn retemplate_and_detach_cancel_old_pending_targets_without_realizing_stale_content() {
    run(|| {
        let manual = Manual::new();
        let host = Host::new();
        let template = CountingTemplate::new();
        host.set_content_template(Some(template.clone()));
        host.set_content(Some(boxed("Latest")));
        let scene = Scene::new(host.clone().upcast());
        let old = host.presenter().unwrap();
        assert_eq!(manual.0.pending_count(), 1);
        standard(&host);
        scene.0.update_layout();
        assert_ne!(host.presenter(), Some(old.clone()));
        assert_eq!(manual.0.pending_count(), 1);
        scene.drain(&manual.0);
        assert!(old.child().is_none());
        assert_eq!(template.builds.get(), 1);
        host.set_content(Some(boxed("Detached")));
        scene.0.set_content(None);
        assert_eq!(manual.0.pending_count(), 0);
        manual.0.flush_pending_batch_for_testing();
        assert_eq!(template.builds.get(), 1);
    });
}
#[test]
fn inherited_template_scope_is_used_unless_an_explicit_null_timeline_is_set() {
    run(|| {
        use crate::scheduling::{resolve_delay, resolve_order, resolve_timeline};
        let _manual = Manual::new();
        let parent = Host::new();
        let t = Timeline::new();
        t.set_auto_schedule(false);
        Scheduling::set_timeline(&parent, Some(t.clone()));
        Scheduling::set_delay(&parent, TimeSpan::from_milliseconds(50.0));
        Scheduling::set_order(&parent, -7);
        let presenter = Presenter::new();
        presenter.set_templated_parent(Some(parent.clone().upcast()));
        assert!(Rc::ptr_eq(&resolve_timeline(&presenter), &t));
        assert_eq!(resolve_delay(&presenter), TimeSpan::from_milliseconds(50.0));
        assert_eq!(resolve_order(&presenter), -7);
        Scheduling::set_timeline(&presenter, None);
        assert!(Rc::ptr_eq(
            &resolve_timeline(&presenter),
            &Settings::default_timeline()
        ));
        Scheduling::set_delay(&presenter, TimeSpan::ZERO);
        Scheduling::set_order(&presenter, 0);
        assert_eq!(resolve_delay(&presenter), TimeSpan::ZERO);
        assert_eq!(resolve_order(&presenter), 0);
    });
}

#[test]
fn original_host_limits_real_template_builds_per_batch() {
    run(|| {
        let manual = Manual::new();
        manual.0.set_max_presentations_per_pass(2);
        let panel = Panel::new();
        let mut hosts = Vec::new();
        let mut templates = Vec::new();
        for _ in 0..5 {
            let host = Host::new();
            let template = CountingTemplate::new();
            host.set_content_template(Some(template.clone()));
            panel.children().add(host.clone().upcast::<Control>());
            hosts.push(host);
            templates.push(template);
        }
        let scene = Scene::new(panel.upcast());
        assert_eq!(templates.iter().map(|t| t.builds.get()).sum::<usize>(), 0);
        for host in &hosts {
            host.set_content(Some(boxed("Batch")));
        }
        assert_eq!(manual.0.pending_count(), 5);
        for expected in [2, 4, 5] {
            scene.drain(&manual.0);
            assert_eq!(
                templates.iter().map(|t| t.builds.get()).sum::<usize>(),
                expected
            );
        }
        for host in &hosts {
            assert!(host.presenter().unwrap().child().is_some());
        }
    });
}
#[test]
fn items_control_presenters_use_scoped_time_budget_and_native_containers() {
    run(|| {
        use crate::DeferredContentPresentationBudgetMode as Mode;
        use ferroui_controls::templates::FuncDataTemplate;
        use ferroui_controls::ItemsControl;
        use std::cell::RefCell;
        let _manual = Manual::new();
        let t = Timeline::new();
        t.set_auto_schedule(false);
        t.set_budget_mode(Mode::RealizationTime);
        t.set_max_realization_time_per_pass(TimeSpan::from_milliseconds(10.0));
        t.set_reveal_duration(TimeSpan::ZERO);
        let templates = Rc::new(vec![
            CountingTemplate::new(),
            CountingTemplate::new(),
            CountingTemplate::new(),
        ]);
        templates[0].delay.set(std::time::Duration::from_millis(25));
        let presenters = Rc::new(RefCell::new(Vec::new()));
        let items = ItemsControl::new();
        Scheduling::set_timeline(&items, Some(t.clone()));
        let list = presenters.clone();
        let builds = templates.clone();
        items.set_item_template(Some(FuncDataTemplate::new(
            |_| true,
            move |data, _| {
                let index = *data.as_ref().unwrap().downcast_ref::<usize>().unwrap();
                let presenter = Presenter::new();
                presenter.set_content_template(Some(builds[index].clone()));
                presenter.set_content(Some(boxed(&format!("Slot{index}"))));
                list.borrow_mut().push(presenter.clone());
                Some(presenter.upcast())
            },
            false,
        )));
        for i in 0..3 {
            items.items().add(Some(Rc::new(i as usize)));
        }
        let scene = Scene::new(items.upcast());
        assert_eq!(presenters.borrow().len(), 3);
        assert_eq!(t.pending_count(), 3);
        assert_eq!(templates.iter().map(|t| t.builds.get()).sum::<usize>(), 0);
        scene.drain(&t);
        assert_eq!(templates[0].builds.get(), 1);
        assert_eq!(templates[1].builds.get(), 0);
        assert_eq!(templates[2].builds.get(), 0);
        scene.drain(&t);
        assert_eq!(templates.iter().map(|t| t.builds.get()).sum::<usize>(), 3);
        for presenter in presenters.borrow().iter() {
            assert!(presenter.child().is_some());
        }
    });
}
#[test]
fn original_reveal_updates_content_and_completes_on_native_render_ticks() {
    use crate::timeline_tests::{dispatcher_delay, ASSEMBLY};
    use ferroui_base::threading::CancellationToken;
    use ferroui_headless::{FerroHeadlessPlatform, HeadlessUnitTestSession};
    use std::time::{Duration, Instant};
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch_task(
            || async {
                crate::register_types();
                let manual = Manual::new();
                manual
                    .0
                    .set_reveal_duration(TimeSpan::from_milliseconds(50.0));
                let host = Host::new();
                host.set_content_template(Some(CountingTemplate::new()));
                host.set_content(Some(boxed("First")));
                let scene = Scene::new(host.clone().upcast());
                scene.drain(&manual.0);
                assert_eq!(child_context(&host), "First");
                host.set_content(Some(boxed("Second")));
                scene.0.update_layout();
                Dispatcher::ui_thread().run_jobs(None);
                scene.0.update_layout();
                manual.0.flush_pending_batch_for_testing();
                let presenter = host.presenter().unwrap();
                assert_eq!(presenter.transitions().unwrap().count(), 1);
                assert!(presenter.opacity() > 0.5);
                Dispatcher::ui_thread().run_jobs(None);
                scene.0.update_layout();
                assert_eq!(child_context(&host), "Second");
                let until = Instant::now() + Duration::from_secs(2);
                loop {
                    Dispatcher::ui_thread().run_jobs(None);
                    scene.0.update_layout();
                    FerroHeadlessPlatform::force_render_timer_tick(1);
                    if presenter.opacity() == 1.0 {
                        break;
                    }
                    assert!(Instant::now() < until, "reveal did not complete");
                    dispatcher_delay(Duration::from_millis(10)).await;
                }
                assert_eq!(presenter.opacity(), 1.0);
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
