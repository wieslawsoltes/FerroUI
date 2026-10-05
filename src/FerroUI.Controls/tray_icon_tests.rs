//! Port of the reference `TrayIconTests` (the first three tests), followed
//! by tests written for this code base. The latter are NOT PORTS: they
//! cover what the reference tests leave out (the values forwarded to the
//! platform implementation and to its menu exporter, clicks, disposal at
//! shutdown, a platform without tray icons).

use crate::platform::{
    as_native_menu_exporter_provider, INativeMenuExporter, ITrayIconImpl, ITrayIconWithIsTemplateImpl,
    IWindowIconImpl, MacOSProperties,
};
use crate::test_command::TestCommand;
use crate::testing::{MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Application, NativeMenu, TrayIcon, TrayIcons};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
enum Call {
    SetIcon(bool),
    SetToolTipText(Option<String>),
    SetIsVisible(bool),
    SetIsTemplateIcon(bool),
    Dispose,
}

/// An exporter that records the menus it is given.
#[derive(Default)]
struct RecordingExporter {
    menus: RefCell<Vec<Option<Ref<NativeMenu>>>>,
}

impl INativeMenuExporter for RecordingExporter {
    fn set_native_menu(&self, menu: Option<Ref<NativeMenu>>) {
        self.menus.borrow_mut().push(menu);
    }
}

/// The hand-written counterpart of the mocked implementation of the
/// reference tests: it records the calls.
#[derive(Default)]
struct RecordingTrayIconImpl {
    calls: RefCell<Vec<Call>>,
    on_clicked: RefCell<Option<Rc<dyn Fn()>>>,
    exporter: Option<Rc<RecordingExporter>>,
    is_template_supported: bool,
}

impl RecordingTrayIconImpl {
    fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    fn full() -> Rc<Self> {
        Rc::new(Self { exporter: Some(Rc::new(RecordingExporter::default())), is_template_supported: true, ..Self::default() })
    }

    fn count(&self, call: &Call) -> usize {
        self.calls.borrow().iter().filter(|candidate| *candidate == call).count()
    }

    fn click(&self) {
        let on_clicked = self.on_clicked.borrow().clone();
        on_clicked.expect("a click handler")();
    }
}

impl IDisposable for RecordingTrayIconImpl {
    fn dispose(&self) {
        self.calls.borrow_mut().push(Call::Dispose);
    }
}

impl ITrayIconImpl for RecordingTrayIconImpl {
    fn set_icon(&self, icon: Option<Rc<dyn IWindowIconImpl>>) {
        self.calls.borrow_mut().push(Call::SetIcon(icon.is_some()));
    }

    fn set_tool_tip_text(&self, text: Option<&str>) {
        self.calls.borrow_mut().push(Call::SetToolTipText(text.map(str::to_string)));
    }

    fn set_is_visible(&self, visible: bool) {
        self.calls.borrow_mut().push(Call::SetIsVisible(visible));
    }

    fn menu_exporter(&self) -> Option<Rc<dyn INativeMenuExporter>> {
        self.exporter.clone().map(|exporter| -> Rc<dyn INativeMenuExporter> { exporter })
    }

    fn on_clicked(&self) -> Option<Rc<dyn Fn()>> {
        self.on_clicked.borrow().clone()
    }

    fn set_on_clicked(&self, value: Option<Rc<dyn Fn()>>) {
        *self.on_clicked.borrow_mut() = value;
    }

    fn as_tray_icon_with_is_template_impl(&self) -> Option<&dyn ITrayIconWithIsTemplateImpl> {
        if self.is_template_supported {
            Some(self)
        } else {
            None
        }
    }
}

impl ITrayIconWithIsTemplateImpl for RecordingTrayIconImpl {
    fn set_is_template_icon(&self, is_template_icon: bool) {
        self.calls.borrow_mut().push(Call::SetIsTemplateIcon(is_template_icon));
    }
}

/// The reference `UnitTestApplication.Start(new TestServices(windowingPlatform: platform))`
/// with a platform whose tray icons are created by `tray_icon_impl`.
fn start(tray_icon_impl: impl Fn() -> Option<Rc<dyn ITrayIconImpl>> + 'static) -> UnitTestApplicationScope {
    let platform = MockWindowingPlatform::with(None, None, Some(Rc::new(tray_icon_impl)));
    UnitTestApplication::start(TestServices::new().with_windowing_platform(platform))
}

fn application() -> Ref<Application> {
    UnitTestApplication::current().expect("an application").upcast()
}

fn icons(items: &[&Ref<TrayIcon>]) -> Rc<TrayIcons> {
    let icons = TrayIcons::new();
    for item in items {
        icons.add((*item).clone());
    }
    icons
}

#[test]
fn platform_impl_should_be_created_only_after_icon_is_attached_to_application() {
    let impl_ = RecordingTrayIconImpl::new();
    let create_count = Rc::new(Cell::new(0));
    let _app = start({
        let (impl_, create_count) = (impl_.clone(), create_count.clone());
        move || {
            create_count.set(create_count.get() + 1);
            Some(impl_.clone() as Rc<dyn ITrayIconImpl>)
        }
    });

    let target = TrayIcon::new();
    target.set_tool_tip_text(Some("Test icon".to_string()));
    let icons = icons(&[&target]);

    assert_eq!(create_count.get(), 0);

    TrayIcon::set_icons(&application(), Some(icons));

    assert_eq!(create_count.get(), 1);
    assert_eq!(impl_.count(&Call::SetToolTipText(Some("Test icon".to_string()))), 1);
    assert_eq!(impl_.count(&Call::SetIsVisible(true)), 1);

    TrayIcon::set_icons(&application(), None);

    assert_eq!(impl_.count(&Call::Dispose), 1);
}

#[test]
fn collection_changes_should_attach_and_detach_icons() {
    let implementations: Rc<RefCell<Vec<Rc<RecordingTrayIconImpl>>>> = Rc::new(RefCell::new(Vec::new()));
    let _app = start({
        let implementations = implementations.clone();
        move || {
            let impl_ = RecordingTrayIconImpl::new();
            implementations.borrow_mut().push(impl_.clone());
            Some(impl_ as Rc<dyn ITrayIconImpl>)
        }
    });

    let target = TrayIcon::new();
    let icons = TrayIcons::new();
    TrayIcon::set_icons(&application(), Some(icons.clone()));

    icons.add(target.clone());

    assert_eq!(implementations.borrow().len(), 1);

    icons.clear();

    assert_eq!(implementations.borrow()[0].count(&Call::Dispose), 1);

    icons.add(target.clone());

    assert_eq!(implementations.borrow().len(), 2);
}

#[test]
fn replaced_collection_should_no_longer_attach_icons() {
    let create_count = Rc::new(Cell::new(0));
    let _app = start({
        let create_count = create_count.clone();
        move || {
            create_count.set(create_count.get() + 1);
            Some(RecordingTrayIconImpl::new() as Rc<dyn ITrayIconImpl>)
        }
    });

    let old_icons = TrayIcons::new();
    let new_icons = TrayIcons::new();

    TrayIcon::set_icons(&application(), Some(old_icons.clone()));
    TrayIcon::set_icons(&application(), Some(new_icons.clone()));
    old_icons.add(TrayIcon::new());

    assert_eq!(create_count.get(), 0);

    new_icons.add(TrayIcon::new());

    assert_eq!(create_count.get(), 1);
}

// The tests below are not ports.

#[test]
fn values_are_forwarded_to_the_implementation_and_to_its_menu_exporter() {
    let impl_ = RecordingTrayIconImpl::full();
    let _app = start({
        let impl_ = impl_.clone();
        move || Some(impl_.clone() as Rc<dyn ITrayIconImpl>)
    });
    let exporter = impl_.exporter.clone().expect("an exporter");

    let menu = NativeMenu::new();
    let target = TrayIcon::new();
    target.set_menu(Some(menu.clone()));
    target.set_is_visible(false);
    MacOSProperties::set_is_template_icon(&target, true);
    assert!(target.native_menu_exporter().is_none());

    TrayIcon::set_icons(&application(), Some(icons(&[&target])));

    // Hidden first, the values, then the visibility of the icon.
    assert_eq!(
        *impl_.calls.borrow(),
        vec![
            Call::SetIsVisible(false),
            Call::SetIcon(false),
            Call::SetToolTipText(None),
            Call::SetIsTemplateIcon(true),
            Call::SetIsVisible(false),
        ]
    );
    assert_eq!(*exporter.menus.borrow(), vec![Some(menu.clone())]);
    assert!(target.native_menu_exporter().is_some());
    assert!(as_native_menu_exporter_provider(&target).is_some_and(|provider| provider.native_menu_exporter().is_some()));

    impl_.calls.borrow_mut().clear();
    target.set_is_visible(true);
    target.set_tool_tip_text(Some("Tip".to_string()));
    target.set_icon(None);
    MacOSProperties::set_is_template_icon(&target, false);
    let other = NativeMenu::new();
    target.set_menu(Some(other.clone()));
    target.set_menu(None);

    assert_eq!(
        *impl_.calls.borrow(),
        vec![Call::SetIsVisible(true), Call::SetToolTipText(Some("Tip".to_string())), Call::SetIsTemplateIcon(false)]
    );
    assert_eq!(*exporter.menus.borrow(), vec![Some(menu), Some(other), None]);
}

#[test]
fn the_attached_native_menu_of_a_tray_icon_reaches_its_exporter() {
    let impl_ = RecordingTrayIconImpl::full();
    let _app = start({
        let impl_ = impl_.clone();
        move || Some(impl_.clone() as Rc<dyn ITrayIconImpl>)
    });
    let exporter = impl_.exporter.clone().expect("an exporter");
    let target = TrayIcon::new();
    TrayIcon::set_icons(&application(), Some(icons(&[&target])));
    exporter.menus.borrow_mut().clear();

    let menu = NativeMenu::new();
    NativeMenu::set_menu(&target, Some(menu.clone()));

    assert_eq!(*exporter.menus.borrow(), vec![Some(menu)]);
}

#[test]
fn a_click_raises_clicked_and_executes_the_command_that_can_execute() {
    let impl_ = RecordingTrayIconImpl::new();
    let _app = start({
        let impl_ = impl_.clone();
        move || Some(impl_.clone() as Rc<dyn ITrayIconImpl>)
    });

    let target = TrayIcon::new();
    let clicks = Rc::new(Cell::new(0));
    let subscription = target.clicked({
        let clicks = clicks.clone();
        move |_| clicks.set(clicks.get() + 1)
    });
    let can_execute = Rc::new(Cell::new(true));
    let executed = Rc::new(Cell::new(0));
    let command = TestCommand::with_can_execute_and_execute(
        {
            let can_execute = can_execute.clone();
            move |_| can_execute.get()
        },
        {
            let executed = executed.clone();
            move |_| executed.set(executed.get() + 1)
        },
    );
    target.set_command(command.as_command());
    TrayIcon::set_icons(&application(), Some(icons(&[&target])));

    impl_.click();
    assert_eq!((clicks.get(), executed.get()), (1, 1));

    can_execute.set(false);
    impl_.click();
    assert_eq!((clicks.get(), executed.get()), (2, 1));

    subscription.dispose();
    impl_.click();
    assert_eq!((clicks.get(), executed.get()), (2, 1));
}

#[test]
fn icons_are_disposed_when_the_dispatcher_shuts_down() {
    let implementations: Rc<RefCell<Vec<Rc<RecordingTrayIconImpl>>>> = Rc::new(RefCell::new(Vec::new()));
    let _app = start({
        let implementations = implementations.clone();
        move || {
            let impl_ = RecordingTrayIconImpl::new();
            implementations.borrow_mut().push(impl_.clone());
            Some(impl_ as Rc<dyn ITrayIconImpl>)
        }
    });

    let (first, second) = (TrayIcon::new(), TrayIcon::new());
    let icons = icons(&[&first, &second]);
    TrayIcon::set_icons(&application(), Some(icons.clone()));
    assert_eq!(implementations.borrow().len(), 2);

    Dispatcher::ui_thread().invoke_shutdown();

    assert!(implementations.borrow().iter().all(|impl_| impl_.count(&Call::Dispose) == 1));
    assert!(first.native_menu_exporter().is_none());

    // A disposed icon is not attached again.
    icons.clear();
    icons.add(first.clone());
    assert_eq!(implementations.borrow().len(), 2);
}

#[test]
fn dispose_removes_the_icon_once() {
    let impl_ = RecordingTrayIconImpl::new();
    let _app = start({
        let impl_ = impl_.clone();
        move || Some(impl_.clone() as Rc<dyn ITrayIconImpl>)
    });
    let target = TrayIcon::new();
    TrayIcon::set_icons(&application(), Some(icons(&[&target])));

    target.to_disposable().dispose();
    target.dispose();

    assert_eq!(impl_.count(&Call::Dispose), 1);

    // Changes after the disposal reach nothing.
    target.set_tool_tip_text(Some("late".to_string()));
    assert_eq!(impl_.count(&Call::SetToolTipText(Some("late".to_string()))), 0);
}

#[test]
fn a_platform_without_tray_icons_is_tolerated() {
    let create_count = Rc::new(Cell::new(0));
    let _app = start({
        let create_count = create_count.clone();
        move || {
            create_count.set(create_count.get() + 1);
            None
        }
    });
    let target = TrayIcon::new();
    let icons = icons(&[&target]);
    TrayIcon::set_icons(&application(), Some(icons.clone()));

    target.set_tool_tip_text(Some("Tip".to_string()));
    target.set_menu(Some(NativeMenu::new()));
    target.set_is_visible(false);
    assert!(target.native_menu_exporter().is_none());
    assert_eq!(create_count.get(), 1);

    // The icon counts as attached: the platform is asked again only after
    // a detach.
    icons.clear();
    icons.add(target.clone());
    assert_eq!(create_count.get(), 2);
}

#[test]
fn icons_must_be_set_on_the_application() {
    let _app = start(|| None);
    let target = TrayIcon::new();

    let error = catch_unwind(AssertUnwindSafe(|| target.set_value(TrayIcon::icons_property(), Some(TrayIcons::new()))))
        .expect_err("the call panics");

    assert_eq!(error.downcast_ref::<&str>().copied(), Some("TrayIcon.Icons must be set on the Application."));
}
