use ferroboard_core::input::ScheduledAction;
use std::time::Duration;
#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::primitives::TouchButton;
use ferroboard_core::input::IKeyboardScheduler;
use ferroui_base::{
    Point, Ref,
    input::{
        IPointer, KeyModifiers, MouseButton, Pointer, PointerPointProperties,
        PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
    },
    media::{Color, TextWrapping},
    threading::Dispatcher,
};
use ferroui_controls::{
    Control, Grid, StackPanel, TextBox, Window, automation::AutomationProperties,
};
use ferroui_headless::HeadlessWindowExtensions;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn automation_press_is_a_named_button_and_preserves_editor_focus() {
    headless::run(|| {
        use ferroui_controls::automation::{
            peers::{AutomationControlType, ControlAutomationPeer},
            provider::IInvokeProvider,
        };
        let root = StackPanel::new();
        let field = TextBox::new();
        let button = TouchButton::new();
        button.set_text("Emoji", 16.0, None, None);
        root.children().add(field.clone());
        root.children().add(button.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(root)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(field.focus());
        let changed = field.clone();
        let count = Rc::new(Cell::new(0));
        let clicks = count.clone();
        let _click = button.click(move |_| {
            clicks.set(clicks.get() + 1);
            changed.set_text(Some("invoked"));
        });
        let peer = ControlAutomationPeer::create_peer_for_element(&button);
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Button
        );
        assert_eq!(peer.get_name(), "Emoji");
        assert!(!peer.get_children().is_empty());
        assert!(!peer.is_keyboard_focusable());
        let provider = peer
            .get_provider::<dyn IInvokeProvider>()
            .expect("native press action");
        provider.invoke().unwrap();
        assert_eq!(count.get(), 1);
        assert_eq!(field.text().as_deref(), Some("invoked"));
        assert!(field.is_focused());
        button.set_text("Symbols", 16.0, None, None);
        assert_eq!(
            peer.get_name(),
            "Symbols",
            "a retained peer reads the live name"
        );
        window.close();
    });
}

#[test]
fn automation_peer_preserves_custom_content_controls() {
    headless::run(|| {
        use ferroui_controls::automation::{
            peers::{AutomationPeer, ControlAutomationPeer},
            provider::IValueProvider,
        };
        fn contains(peer: &Ref<AutomationPeer>, expected: &Ref<AutomationPeer>) -> bool {
            peer.get_children()
                .iter()
                .any(|child| child == expected || contains(child, expected))
        }
        let button = TouchButton::new();
        let field = TextBox::new();
        field.set_text(Some("custom content"));
        button.set_content(field.clone().upcast());
        let peer = ControlAutomationPeer::create_peer_for_element(&button);
        let old_child = ControlAutomationPeer::create_peer_for_element(&field);
        assert!(contains(&peer, &old_child));
        assert_eq!(
            old_child
                .get_provider::<dyn IValueProvider>()
                .unwrap()
                .value()
                .as_deref(),
            Some("custom content")
        );
        let replacement = TextBox::new();
        button.set_content(replacement.clone().upcast());
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(contains(
            &peer,
            &ControlAutomationPeer::create_peer_for_element(&replacement)
        ));
        assert!(!contains(&peer, &old_child));
    });
}

#[test]
fn automation_press_rejects_disabled_button_or_ancestor() {
    headless::run(|| {
        use ferroui_controls::automation::{
            peers::ControlAutomationPeer, provider::IInvokeProvider,
        };
        let root = StackPanel::new();
        let button = TouchButton::new();
        root.children().add(button.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(root.clone())));
        window.show();
        let count = Rc::new(Cell::new(0));
        let clicks = count.clone();
        button.click(move |_| clicks.set(clicks.get() + 1));
        let peer = ControlAutomationPeer::create_peer_for_element(&button);
        let provider = peer.get_provider::<dyn IInvokeProvider>().unwrap();
        root.set_is_enabled(false);
        assert!(!peer.is_enabled());
        assert!(provider.invoke().is_err());
        assert_eq!(count.get(), 0);
        root.set_is_enabled(true);
        provider.invoke().unwrap();
        assert_eq!(count.get(), 1);
        button.set_is_enabled(false);
        assert!(provider.invoke().is_err());
        assert_eq!(count.get(), 1);
        window.close();
    });
}

#[test]
fn automation_press_does_not_start_pointer_repeat_or_long_press_timers() {
    headless::run(|| {
        use ferroui_controls::automation::{
            peers::ControlAutomationPeer, provider::IInvokeProvider,
        };
        let button = TouchButton::new();
        let clock = Rc::new(Clock::default());
        button.set_scheduler(clock.clone());
        button.set_repeat_delay(Some(Duration::from_nanos(1000)));
        let count = Rc::new(Cell::new(0));
        let clicks = count.clone();
        button.click(move |_| clicks.set(clicks.get() + 1));
        button.long_press(|_| panic!("automation press is not a held pointer"));
        let peer = ControlAutomationPeer::create_peer_for_element(&button);
        let provider = peer.get_provider::<dyn IInvokeProvider>().unwrap();
        provider.invoke().unwrap();
        assert_eq!(count.get(), 1);
        assert!(clock.pending.borrow().is_empty());
        clock.advance(100_000);
        assert_eq!(count.get(), 1);
    });
}

#[test]
fn retained_automation_provider_does_not_keep_button_alive_and_rejects_expired_owner() {
    headless::run(|| {
        use ferroui_controls::automation::{
            peers::ControlAutomationPeer, provider::IInvokeProvider,
        };
        let button = TouchButton::new();
        let peer = ControlAutomationPeer::create_peer_for_element(&button);
        let provider = peer.get_provider::<dyn IInvokeProvider>().unwrap();
        let weak = button.downgrade();
        drop(button);
        assert!(weak.upgrade().is_none());
        assert!(provider.invoke().is_err());
    });
}

struct Pending {
    due: i64,
    disposed: Cell<bool>,
    action: RefCell<Option<Rc<dyn Fn()>>>,
}
impl Pending {
    fn cancel(&self) {
        self.disposed.set(true);
    }
}
#[derive(Default)]
struct Clock {
    now: Cell<i64>,
    pending: RefCell<Vec<Rc<Pending>>>,
}
impl IKeyboardScheduler for Clock {
    fn schedule(&self, delay: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
        let timer = Rc::new(Pending {
            due: self.now.get() + i64::try_from(delay.as_nanos() / 100).unwrap(),
            disposed: Cell::new(false),
            action: RefCell::new(Some(action)),
        });
        self.pending.borrow_mut().push(timer.clone());
        ScheduledAction::new(move || timer.cancel())
    }
}
impl Clock {
    fn advance(&self, now: i64) {
        loop {
            let index = self
                .pending
                .borrow()
                .iter()
                .enumerate()
                .filter(|(_, timer)| timer.due <= now)
                .min_by_key(|(_, timer)| timer.due)
                .map(|(i, _)| i);
            let Some(index) = index else {
                break;
            };
            let timer = self.pending.borrow_mut().remove(index);
            self.now.set(timer.due);
            let action = timer.action.borrow_mut().take();
            if !timer.disposed.get() {
                action.unwrap()();
            }
        }
        self.now.set(now);
    }
    fn clear(&self) {
        self.pending.borrow_mut().clear();
    }
}
fn show(button: &Ref<TouchButton>) -> Ref<Window> {
    button.set_width(100.0);
    button.set_height(40.0);
    let window = Window::new();
    window.set_width(100.0);
    window.set_height(40.0);
    window.set_content(Some(Control::boxed(button.clone())));
    window.show();
    Dispatcher::current_dispatcher().run_jobs(None);
    assert_eq!(button.bounds().width, 100.0);
    window
}
fn down(button: &Ref<TouchButton>, window: &Ref<Window>, pointer: &Rc<Pointer>) {
    let position = button
        .translate_point(Point::new(10.0, 10.0), window)
        .unwrap();
    let args = PointerPressedEventArgs::new(
        button,
        pointer.clone(),
        window,
        position,
        0,
        PointerPointProperties {
            is_left_button_pressed: true,
            ..Default::default()
        },
        KeyModifiers::empty(),
        1,
    );
    button.raise_event(&args);
    assert!(args.handled());
}
fn up(button: &Ref<TouchButton>, window: &Ref<Window>, pointer: &Rc<Pointer>, x: f64, y: f64) {
    let position = button.translate_point(Point::new(x, y), window).unwrap();
    let args = PointerReleasedEventArgs::new(
        button,
        pointer.clone(),
        window,
        position,
        1,
        PointerPointProperties::default(),
        KeyModifiers::empty(),
        MouseButton::Left,
    );
    button.raise_event(&args);
    assert!(args.handled());
}
#[test]
fn native_grid_content_identity_brush_sharing_text_and_release_margin() {
    headless::run(|| {
        let button = TouchButton::new();
        assert!(
            button
                .to_ref()
                .upcast::<Control>()
                .downcast::<Grid>()
                .is_ok()
        );
        assert!(!button.focusable());
        assert!(!button.is_tab_stop());
        assert_eq!(button.content_grid(), button.content_grid());
        let text = button.set_text("Copy", 17.0, None, None);
        assert_eq!(text.text().as_deref(), Some("Copy"));
        assert_eq!(text.text_wrapping(), TextWrapping::NoWrap);
        assert!(!text.is_hit_test_visible());
        assert_eq!(
            AutomationProperties::get_name(&button).as_deref(),
            Some("Copy")
        );
        button.set_content(Grid::new().upcast());
        assert_eq!(button.content_grid().children().count(), 1);
        let normal = Color::from_uint32(0xff112233);
        let pressed = Color::from_uint32(0xff445566);
        button.set_colors(normal, pressed);
        let other = TouchButton::new();
        other.set_colors(normal, pressed);
        assert!(
            button
                .background()
                .unwrap()
                .equals(other.background().as_ref().unwrap().as_ref())
        );
        let window = show(&button);
        let clicks = Rc::new(Cell::new(0));
        let count = clicks.clone();
        let subscription = button.click(move |_| count.set(count.get() + 1));
        let pointer = Pointer::new(1, PointerType::Mouse, true);
        for (x, y, inside) in [
            (-8.0, -8.0, true),
            (108.0, 48.0, true),
            (-8.0001, 0.0, false),
            (108.0001, 0.0, false),
            (0.0, 48.0001, false),
        ] {
            let before = clicks.get();
            down(&button, &window, &pointer);
            assert!(pointer.captured().is_some());
            up(&button, &window, &pointer, x, y);
            assert!(pointer.captured().is_none());
            assert_eq!(clicks.get(), before + i32::from(inside));
        }
        button.remove_click(subscription);
        down(&button, &window, &pointer);
        up(&button, &window, &pointer, 5.0, 5.0);
        assert_eq!(clicks.get(), 2);
        window.close();
    });
}
#[test]
fn native_long_press_repeat_precedence_thresholds_and_capture_loss() {
    headless::run(|| {
        let clock = Rc::new(Clock::default());
        let button = TouchButton::new();
        button.set_scheduler(clock.clone());
        let window = show(&button);
        let clicks = Rc::new(Cell::new(0));
        let count = clicks.clone();
        button.click(move |_| count.set(count.get() + 1));
        let longs = Rc::new(Cell::new(0));
        let count = longs.clone();
        let long_subscription = button.long_press(move |_| count.set(count.get() + 1));
        let pointer = Pointer::new(1, PointerType::Touch, true);
        down(&button, &window, &pointer);
        clock.advance(4_499_999);
        assert_eq!(longs.get(), 0);
        clock.advance(4_500_000);
        assert_eq!(longs.get(), 1);
        up(&button, &window, &pointer, 5.0, 5.0);
        assert_eq!(clicks.get(), 0);
        button.set_repeat_delay(Some(Duration::from_millis(200)));
        down(&button, &window, &pointer);
        clock.advance(6_499_999);
        assert_eq!(clicks.get(), 0);
        clock.advance(6_500_000);
        assert_eq!(clicks.get(), 1);
        clock.advance(7_000_000);
        assert_eq!(clicks.get(), 2);
        assert_eq!(longs.get(), 1);
        up(&button, &window, &pointer, 5.0, 5.0);
        clock.advance(8_000_000);
        assert_eq!(clicks.get(), 2);
        down(&button, &window, &pointer);
        pointer.platform_capture_lost();
        clock.advance(20_000_000);
        up(&button, &window, &pointer, 5.0, 5.0);
        assert_eq!(clicks.get(), 2);
        button.set_repeat_delay(None);
        button.remove_long_press(long_subscription);
        down(&button, &window, &pointer);
        assert!(clock.pending.borrow().is_empty());
        up(&button, &window, &pointer, 5.0, 5.0);
        assert_eq!(clicks.get(), 3);
        clock.clear();
        window.close();
    });
}
#[test]
fn native_capture_transfer_and_source_multi_pointer_reset_quirk() {
    headless::run(|| {
        let button = TouchButton::new();
        let window = show(&button);
        let pointer = Pointer::new(1, PointerType::Pen, true);
        down(&button, &window, &pointer);
        let new_owner = TextBox::new();
        pointer.capture(Some(&new_owner.clone().upcast()));
        // Reset from capture loss must not release the NEW owner's capture.
        assert_eq!(pointer.captured(), Some(new_owner.clone().upcast()));
        pointer.capture(None);
        let count = Rc::new(Cell::new(0));
        let clicks = count.clone();
        button.click(move |_| clicks.set(clicks.get() + 1));
        down(&button, &window, &pointer);
        let second = Pointer::new(2, PointerType::Touch, false);
        down(&button, &window, &second);
        // Source doesn't verify pointer identity on release: first up consumes the shared press.
        up(&button, &window, &pointer, 0.0, 0.0);
        up(&button, &window, &second, 0.0, 0.0);
        assert_eq!(count.get(), 1);
        window.close();
    });
}
#[test]
fn native_pointer_click_keeps_editing_focus_and_repeat_reads_live_interval() {
    headless::run(|| {
        let box_ = TextBox::new();
        let button = TouchButton::new();
        button.set_width(100.0);
        button.set_height(40.0);
        let stack = StackPanel::new();
        stack.children().add(box_.clone());
        stack.children().add(button.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(stack)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(box_.focus());
        let clock = Rc::new(Clock::default());
        button.set_scheduler(clock.clone());
        button.set_repeat_delay(Some(Duration::from_nanos(10000)));
        let clicks = Rc::new(Cell::new(0));
        let count = clicks.clone();
        button.click(move |button| {
            count.set(count.get() + 1);
            button.set_repeat_interval(Duration::from_nanos(7000));
        });
        let pointer = Pointer::new(1, PointerType::Mouse, true);
        down(&button, &window, &pointer);
        assert!(box_.is_focused());
        clock.advance(100);
        assert_eq!(clicks.get(), 1);
        clock.advance(169);
        assert_eq!(clicks.get(), 1);
        clock.advance(170);
        assert_eq!(clicks.get(), 2);
        up(&button, &window, &pointer, 5.0, 5.0);
        assert!(box_.is_focused());
        clock.clear();
        window.close();
    });
}

#[test]
fn headless_raw_mouse_touch_cancel_and_overlapping_contacts() {
    headless::run(|| {
        let button = TouchButton::new();
        let window = show(&button);
        let clock = Rc::new(Clock::default());
        button.set_scheduler(clock.clone());
        let clicks = Rc::new(Cell::new(0));
        let count = clicks.clone();
        button.click(move |_| count.set(count.get() + 1));
        let point = Point::new(20.0, 20.0);
        let modifiers = ferroui_base::input::RawInputModifiers::empty();
        // These helpers send raw platform input through hit testing, the mouse/touch devices,
        // capture and routed events, rather than calling the control handlers directly.
        window.mouse_down(point, MouseButton::Left, modifiers);
        window.mouse_up(point, MouseButton::Left, modifiers);
        assert_eq!(clicks.get(), 1);
        let touch = window.touch_begin(point, modifiers);
        window.touch_end(touch.as_ref(), point, modifiers);
        assert_eq!(clicks.get(), 2);

        button.set_repeat_delay(Some(Duration::from_nanos(10000)));
        let touch = window.touch_begin(point, modifiers);
        assert_eq!(clock.pending.borrow().len(), 1);
        touch.dispose(); // RawTouchCancel -> TouchDevice -> Pointer.dispose -> capture lost.
        assert!(clock.pending.borrow()[0].disposed.get());
        clock.advance(1000);
        assert_eq!(clicks.get(), 2);

        button.set_repeat_delay(None);
        let first = window.touch_begin(point, modifiers);
        let second = window.touch_begin(point, modifiers);
        window.touch_end(first.as_ref(), point, modifiers);
        window.touch_end(second.as_ref(), point, modifiers);
        assert_eq!(clicks.get(), 3); // Shared pressed state consumes the first release only.
        clock.clear();
        window.close();
    });
}

#[test]
fn native_icon_content_styling_accessibility_and_failed_custom_registration() {
    headless::run(|| {
        use ferroboard_controls::icons::{IconError, IconFactory};
        use ferroboard_core::keys::KeyIcon;
        use ferroui_base::media::{IBrush, SolidColorBrush, Stretch};
        use ferroui_controls::shapes::Path;
        let button = TouchButton::new();
        let foreground: Rc<dyn IBrush> = SolidColorBrush::from_uint32(0xff123456).into();
        button
            .set_icon(KeyIcon::Shift, 23.25, Some(foreground.clone()), None)
            .unwrap();
        assert_eq!(
            AutomationProperties::get_name(&button).as_deref(),
            Some("Shift")
        );
        let children = button.content_grid().children();
        let child = children.get(0);
        let path = child.downcast::<Path>().unwrap();
        assert_eq!(path.width(), 23.25);
        assert_eq!(path.height(), 23.25);
        assert_eq!(path.stretch(), Stretch::None);
        assert!(!path.is_hit_test_visible());
        assert!(path.fill().unwrap().equals(foreground.as_ref()));
        assert!(Rc::ptr_eq(&path.fill().unwrap(), &foreground));
        assert!(path.data().is_some());
        button
            .set_icon(KeyIcon::Shift, 20.0, None, Some(""))
            .unwrap();
        assert_eq!(AutomationProperties::get_name(&button).as_deref(), Some(""));
        let before = button.content_grid().children().get(0);
        IconFactory::register(KeyIcon::Emoticon, "M invalid 0");
        assert!(matches!(
            button.set_icon(KeyIcon::Emoticon, 20.0, None, None),
            Err(IconError::InvalidNumber { .. })
        ));
        assert_eq!(button.content_grid().children().get(0), before);
        assert_eq!(AutomationProperties::get_name(&button).as_deref(), Some(""));
        IconFactory::register_value(i32::MAX, Some("M 0 0 L 1 1".into()));
        button.set_icon_value(i32::MAX, 10.0, None, None).unwrap();
        assert_eq!(
            AutomationProperties::get_name(&button).as_deref(),
            Some("2147483647")
        );
    });
}

#[test]
fn native_grid_decoration_preserves_background_identity_padding_and_rounding_properties() {
    headless::run(|| {
        use ferroui_base::{CornerRadius, Thickness};
        use ferroui_controls::{Border, Panel};
        let button = TouchButton::new();
        button.set_use_layout_rounding(false);
        button.set_corner_radius(CornerRadius::uniform(11.0));
        button.set_padding(Thickness::new(4.0, 5.0, 6.0, 7.0));
        button.set_border_thickness(Thickness::uniform(2.0));
        button.set_colors(Color::from_rgb(20, 40, 60), Color::from_rgb(60, 40, 20));
        let native = button.children().get(0);
        let decoration = native.downcast_ref::<Border>().unwrap();
        assert_eq!(decoration.corner_radius(), button.corner_radius());
        assert_eq!(decoration.padding(), button.padding());
        assert_eq!(decoration.border_thickness(), button.border_thickness());
        assert!(Rc::ptr_eq(
            decoration.background().as_ref().unwrap(),
            button.background().as_ref().unwrap()
        ));
        let brush = ferroui_base::media::SolidColorBrush::with_color(Color::from_rgb(1, 2, 3));
        let inherited = button.clone().upcast::<Panel>();
        inherited.set_background(Some(brush.clone().into()));
        assert!(Rc::ptr_eq(
            decoration.background().as_ref().unwrap(),
            button.background().as_ref().unwrap()
        ));
        assert_eq!(
            decoration
                .background()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            brush.color()
        );
        button.set_border_brush(Some(brush.into()));
        let window = show(&button);
        let content = button.content_grid().bounds();
        assert_eq!(content.x, 6.0);
        assert_eq!(content.y, 7.0);
        assert_eq!(content.width, 86.0);
        assert_eq!(content.height, 24.0);
        window.close();
    });
}
