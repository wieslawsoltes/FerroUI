//! Complete pinned window theme and source invoke sequence across a compiled boundary.
use ferrodock_ferro::controls::PinnedDockWindow;
use ferroui_base::{
    controls::ResourceKey,
    styling::ControlTheme,
    threading::{CancellationToken, Dispatcher},
    Ref,
};
use ferroui_controls::{
    automation::{
        peers::{AutomationControlType, ControlAutomationPeer},
        provider::IInvokeProvider,
        AutomationProperties,
    },
    AppBuilder, Application, SizeToContent, Window, WindowDecorations, WindowState,
    WindowTransparencyLevel,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
#[repr(C)]
struct DerivedPinnedWindow {
    base: PinnedDockWindow,
}
ferroui_base::ferro_class!(DerivedPinnedWindow:PinnedDockWindow);
ferroui_base::ferro_class_info!(DerivedPinnedWindow {});
ferroui_base::ferro_impl_classes!(DerivedPinnedWindow:ferroui_base::FerroObjectImpl,ferroui_base::StyledElementImpl,ferroui_base::VisualImpl,ferroui_base::layout::LayoutableImpl,ferroui_base::interactivity::InteractiveImpl,ferroui_base::input::InputElementImpl,ferroui_controls::ControlImpl,ferroui_controls::primitives::TemplatedControlImpl,ferroui_controls::ContentControlImpl,ferroui_controls::TopLevelImpl,ferroui_controls::WindowBaseImpl,ferroui_controls::WindowImpl);
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .after_setup(|_| {
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style())
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "pinned-window-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn original_pinned_window_theme_keeps_all_setters_and_peer_invokes_hidden_and_visible_windows() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro_themes_fluent::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/PinnedWindowConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let window = value.downcast_ref::<Ref<PinnedDockWindow>>().unwrap();
                let theme = ferroui_base::metadata::from_markup_value::<Ref<ControlTheme>>(
                    &window.find_resource(&ResourceKey::from(PinnedDockWindow::TYPE)),
                )
                .unwrap();
                window.set_theme(Some(theme));
                window.apply_styling();
                assert_eq!(window.style_key(), PinnedDockWindow::TYPE);
                assert_eq!(window.window_decorations(), WindowDecorations::None);
                assert!(!window.show_in_taskbar());
                assert!(!window.can_resize());
                assert!(!window.show_activated());
                assert!(window.background().is_none());
                assert_eq!(
                    &*window.transparency_level_hint(),
                    &[WindowTransparencyLevel::transparent()]
                );
                assert_eq!(window.size_to_content(), SizeToContent::MANUAL);
                assert!(!window.topmost());
                assert!(window.is_hit_test_visible());
                assert_eq!(window.width(), 320.0);
                assert_eq!(window.height(), 200.0);
                let peer = ControlAutomationPeer::create_peer_for_element(window);
                assert_eq!(peer.get_class_name(), "PinnedDockWindow");
                assert_eq!(
                    peer.get_automation_control_type(),
                    AutomationControlType::Window
                );
                assert_eq!(peer.get_name(), "Pinned tools");
                assert_eq!(
                    peer.get_help_text(),
                    "WindowState=Normal; Visible=false; ShowInTaskbar=false"
                );
                window.set_title(Some("".into()));
                assert_eq!(peer.get_name(), "Pinned dock window");
                AutomationProperties::set_name(window, Some("Explicit pinned tools"));
                assert_eq!(peer.get_name(), "Explicit pinned tools");
                // Focus() may fail on a nonfocusable Window; enable focus for this sequence.
                window.set_focusable(true);
                let invoke = peer.get_provider::<dyn IInvokeProvider>().unwrap();
                invoke.invoke().unwrap();
                Dispatcher::ui_thread().run_jobs(None);
                assert!(window.is_visible());
                assert!(window.is_active());
                assert!(window.is_focused());
                assert_eq!(
                    peer.get_help_text(),
                    "WindowState=Normal; Visible=true; ShowInTaskbar=false"
                );
                invoke.invoke().unwrap();
                assert!(window.is_visible());
                window.set_window_state(WindowState::Maximized);
                assert!(peer.get_help_text().starts_with("WindowState=Maximized;"));
                window.hide();
                invoke.invoke().unwrap();
                assert!(window.is_visible());
                window.close();
                // Source style-key override is stable even when a subclass inherits it.
                let derived: Ref<DerivedPinnedWindow> =
                    ferroui_base::instantiate(DerivedPinnedWindow {
                        base: PinnedDockWindow::construct(),
                    });
                assert_eq!(derived.style_key(), PinnedDockWindow::TYPE);
                assert_eq!(
                    ControlAutomationPeer::create_peer_for_element(&derived).get_class_name(),
                    "PinnedDockWindow"
                );
                derived.close();
                let base = Window::new();
                assert_ne!(base.style_key(), PinnedDockWindow::TYPE);
                base.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
