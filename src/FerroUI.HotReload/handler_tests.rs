use super::*;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, Ref,
};
use ferroui_controls::{Button, ContentControl, StackPanel};
use ferroui_markup_xaml::{
    FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
};
use std::cell::Cell;

#[repr(C)]
struct HandlerView {
    base: ContentControl,
    clicks: Cell<usize>,
    roots: Cell<usize>,
}
ferro_class!(HandlerView: ContentControl);
ferro_impl_classes!(HandlerView: ferroui_base::FerroObjectImpl, ferroui_base::StyledElementImpl,
    ferroui_base::VisualImpl, ferroui_base::layout::LayoutableImpl,
    ferroui_base::interactivity::InteractiveImpl, ferroui_base::input::InputElementImpl,
    ferroui_controls::ControlImpl, ferroui_controls::primitives::TemplatedControlImpl,
    ferroui_controls::ContentControlImpl);
ferro_class_info!(HandlerView {
    new: HandlerView::new,
    markup: { methods: [
        fn OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
            |this: &Ref<HandlerView>, _: Option<BoxedValue>, _: Rc<dyn IRoutedEventArgs>| this.clicks.set(this.clicks.get() + 1),
        fn OnRootClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
            |this: &Ref<HandlerView>, _: Option<BoxedValue>, _: Rc<dyn IRoutedEventArgs>| this.roots.set(this.roots.get() + 1),
    ] },
});
impl HandlerView {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: ContentControl::construct(),
            clicks: Cell::new(0),
            roots: Cell::new(0),
        })
    }
}
fn source(width: usize) -> String {
    format!(
        r#"<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="{}" Button.Click="OnRootClick">
<ContentControl.Resources><ControlTemplate x:Key="Lazy"><Button Click="OnClick"/></ControlTemplate></ContentControl.Resources>
<Button Name="action" Click="OnClick" Width="{width}"/>
</ContentControl>"#,
        HandlerView::TYPE.full_name()
    )
}
fn load(root: &Ref<HandlerView>, source: &str) {
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    ferroui_base::TypeInfo::register(HandlerView::TYPE);
    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
        RuntimeXamlLoaderDocument::with_root_instance(Some(Rc::new(root.clone())), source),
        None,
    )
    .unwrap();
}
fn button(root: &Ref<HandlerView>) -> Ref<Button> {
    Ref::<Button>::from_xaml_object(&root.content().unwrap()).unwrap()
}
fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}
// A headless ContentControl has no presenter/visual parent route. Exercise the
// root attachment directly as well as the child attachment.
fn click_view(root: &Ref<HandlerView>) {
    click(&button(root));
    root.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}
fn lazy(root: &Ref<HandlerView>) -> Rc<dyn ferroui_controls::templates::IControlTemplate> {
    Rc::<ferroui_markup_xaml::templates::ControlTemplate>::from_xaml_object(
        &root.resources().get(&"Lazy".into()).unwrap(),
    )
    .unwrap()
    .as_control_template()
}
#[test]
fn handlers_call_preserved_root_and_retire_only_xaml_subscriptions() {
    let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
    let root = HandlerView::new();
    let baseline = source(10);
    load(&root, &baseline);
    let uri = "ferres://handler-test/View.xaml";
    let target: Rc<dyn ReloadTarget> = Rc::new(
        ComponentReloadTarget::content_control(
            &root.clone().upcast(),
            uri.into(),
            baseline.clone(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .with_owned_resources()
        .with_owned_handlers(),
    );
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), baseline).unwrap();
    session.register_target(&target);
    click_view(&root);
    assert_eq!((root.clicks.get(), root.roots.get()), (1, 1));
    let application = Rc::new(Cell::new(0));
    for width in [20, 30, 40] {
        let old = button(&root);
        let weak = old.downgrade();
        let app = application.clone();
        old.click(move |_, _| app.set(app.get() + 1));
        let previous = (root.clicks.get(), root.roots.get());
        session.submit(uri.into(), source(width));
        let reached = Cell::new(false);
        let failed = session
            .apply_pending_with(|| {
                reached.set(true);
                click_view(&root);
                assert_eq!(
                    (root.clicks.get(), root.roots.get()),
                    previous,
                    "candidate callbacks ran before publication"
                );
                Err(ReloadError::Failed("rollback".into()))
            })
            .unwrap();
        assert!(reached.get(), "{failed:?}");
        assert_eq!(failed.status, ReloadStatus::Failed);
        assert!(button(&root).ptr_eq(&old));
        click_view(&root);
        assert_eq!(
            (root.clicks.get(), root.roots.get()),
            (previous.0 + 1, previous.1 + 1)
        );
        session.submit(uri.into(), source(width));
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
        let before = (root.clicks.get(), root.roots.get());
        click(&old);
        assert_eq!(
            (root.clicks.get(), root.roots.get()),
            before,
            "retired child retained its XAML handler"
        );
        click_view(&root);
        assert_eq!(
            (root.clicks.get(), root.roots.get()),
            (before.0 + 1, before.1 + 1)
        );
        drop(old);
        assert!(weak.upgrade().is_none());
    }
    assert_eq!(
        application.get(),
        6,
        "application handlers were removed or duplicated"
    );
    let weak = root.downgrade();
    drop(root);
    assert!(weak.upgrade().is_none());
}
#[test]
fn deferred_handlers_keep_their_generation_and_preserved_code_behind() {
    let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
    let root = HandlerView::new();
    let baseline = source(10);
    load(&root, &baseline);
    let old_template = lazy(&root);
    let uri = "ferres://handler-test/Lazy.xaml";
    let target: Rc<dyn ReloadTarget> = Rc::new(
        ComponentReloadTarget::content_control(
            &root.clone().upcast(),
            uri.into(),
            baseline.clone(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .with_owned_resources()
        .with_owned_handlers(),
    );
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), baseline).unwrap();
    session.register_target(&target);
    session.submit(uri.into(), source(20));
    let result = session.apply_pending().unwrap();
    assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
    let old = old_template
        .build(&root.clone().upcast())
        .unwrap()
        .result()
        .cast::<Button>()
        .unwrap();
    click(&old);
    assert_eq!(
        root.clicks.get(),
        0,
        "retired lazy template installed a handler"
    );
    let new = lazy(&root)
        .build(&root.clone().upcast())
        .unwrap()
        .result()
        .cast::<Button>()
        .unwrap();
    click(&new);
    assert_eq!(
        root.clicks.get(),
        1,
        "new deferred handler lost code-behind context"
    );
}

#[test]
fn multiple_instances_keep_independent_code_behind_and_closed_targets_are_weak() {
    let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
    let roots = [HandlerView::new(), HandlerView::new()];
    let baseline = source(10);
    let uri = "ferres://handler-test/Multiple.xaml";
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), baseline.clone())
        .unwrap();
    let targets: Vec<Rc<dyn ReloadTarget>> = roots
        .iter()
        .map(|root| {
            load(root, &baseline);
            let target: Rc<dyn ReloadTarget> = Rc::new(
                ComponentReloadTarget::content_control(
                    &root.clone().upcast(),
                    uri.into(),
                    baseline.clone(),
                    RuntimeXamlLoaderConfiguration::default(),
                )
                .with_owned_resources()
                .with_owned_handlers(),
            );
            session.register_target(&target);
            target
        })
        .collect();
    session.submit(uri.into(), source(20));
    let result = session.apply_pending().unwrap();
    assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
    click_view(&roots[0]);
    assert_eq!((roots[0].clicks.get(), roots[0].roots.get()), (1, 1));
    assert_eq!((roots[1].clicks.get(), roots[1].roots.get()), (0, 0));
    click_view(&roots[1]);
    assert_eq!((roots[1].clicks.get(), roots[1].roots.get()), (1, 1));
    let weak = roots[0].downgrade();
    drop(roots);
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    assert!(weak.upgrade().is_none());
    assert_eq!(targets.len(), 2);
}

#[repr(C)]
struct HandlerPanel {
    base: ferroui_controls::StackPanel,
    clicks: Cell<usize>,
}
ferro_class!(HandlerPanel: StackPanel);
ferro_impl_classes!(HandlerPanel: ferroui_base::FerroObjectImpl, ferroui_base::StyledElementImpl,
    ferroui_base::VisualImpl, ferroui_base::layout::LayoutableImpl,
    ferroui_base::interactivity::InteractiveImpl, ferroui_base::input::InputElementImpl,
    ferroui_controls::ControlImpl,
    ferroui_controls::PanelImpl, ferroui_controls::StackPanelImpl);
ferro_class_info!(HandlerPanel {
    new: HandlerPanel::new,
    markup: { methods: [
        fn OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
            |this: &Ref<HandlerPanel>, _: Option<BoxedValue>, _: Rc<dyn IRoutedEventArgs>| this.clicks.set(this.clicks.get() + 1),
    ] },
});
impl HandlerPanel {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: ferroui_controls::StackPanel::construct(),
            clicks: Cell::new(0),
        })
    }
}
#[test]
fn panel_handlers_replace_owned_children_and_keep_later_application_subscriptions() {
    let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    ferroui_base::TypeInfo::register(HandlerPanel::TYPE);
    let root = HandlerPanel::new();
    let baseline = r#"<StackPanel xmlns="https://github.com/ferroui"><Button Click="OnClick" Width="10"/></StackPanel>"#;
    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
        RuntimeXamlLoaderDocument::with_root_instance(Some(Rc::new(root.clone())), baseline),
        None,
    )
    .unwrap();
    let uri = "ferres://handler-test/Panel.xaml";
    let make_target = |range| {
        PanelReloadTarget::new(
            &root.clone().upcast(),
            range,
            uri.into(),
            baseline.into(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .unwrap()
    };
    assert!(matches!(
        make_target(0..0).with_owned_handlers(),
        Err(ReloadError::RestartRequired(_))
    ));
    let target: Rc<dyn ReloadTarget> = Rc::new(make_target(0..1).with_owned_handlers().unwrap());
    let application = Rc::new(Cell::new(0));
    let button = Button::new();
    let app = application.clone();
    button.click(move |_, _| app.set(app.get() + 1));
    root.children()
        .add(button.clone().upcast::<ferroui_controls::Control>());
    let mut session = HotReloadSession::new();
    session
        .register_document(uri.into(), baseline.into())
        .unwrap();
    session.register_target(&target);
    for width in [20, 30] {
        let old = root.children().get(0).cast::<Button>().unwrap();
        click(&old);
        let before = root.clicks.get();
        session.submit(uri.into(), baseline.replace("10", &width.to_string()));
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
        click(&old);
        assert_eq!(root.clicks.get(), before);
        click(&root.children().get(0).cast::<Button>().unwrap());
        assert_eq!(root.clicks.get(), before + 1);
        assert!(root
            .children()
            .get(1)
            .ptr_eq(&button.clone().upcast::<ferroui_controls::Control>()));
        click(&button);
    }
    assert_eq!(application.get(), 2);
}
