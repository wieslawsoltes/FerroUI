#![cfg(feature = "headless")]
use ferro_solitaire::{app::App, models::*, view_models::*, views::casino_view::CasinoView};
use ferroui::{
    base::{
        PixelSize, Point,
        animation::{IClock, PlayState, TimeSpan},
        media::imaging::{BitmapEncoderOptions, RenderTargetBitmap},
        reactive::{IDisposable, IObservable, IObserver, LightweightSubject},
        threading::Dispatcher,
    },
    controls::{Border, Button, Canvas},
    headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions},
    prelude::*,
    skia::SkiaApplicationExtensions,
};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};
struct FrameClock {
    time: Cell<f64>,
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}
impl FrameClock {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            time: Cell::new(0.0),
            subject: LightweightSubject::new(),
            state: Cell::new(PlayState::Run),
        })
    }
    fn advance(&self, ms: f64) {
        self.time.set(self.time.get() + ms);
        self.subject
            .on_next(TimeSpan::from_milliseconds(self.time.get()));
    }
}
impl IObservable<TimeSpan> for FrameClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}
impl IClock for FrameClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, state: PlayState) {
        self.state.set(state);
    }
}
fn pump(ms: u64) {
    let frame = ferroui::base::threading::DispatcherFrame::new();
    let stop = frame.clone();
    let timer = ferroui::base::threading::DispatcherTimer::run_once(
        move || stop.set_continue(false),
        Duration::from_millis(ms.max(1)),
        ferroui::base::threading::DispatcherPriority::NORMAL,
    );
    Dispatcher::ui_thread().push_frame(&frame);
    timer.dispose();
}
fn frame(window: &Ref<Window>, name: &str) -> Vec<u8> {
    window.update_layout();
    let bitmap = RenderTargetBitmap::new(PixelSize::new(1200, 860));
    bitmap.render(&window.clone().upcast());
    let mut bytes = vec![];
    bitmap
        .save(&mut bytes, &BitmapEncoderOptions::Png(Default::default()))
        .unwrap();
    if let Ok(dir) = std::env::var("FERRO_SOLITAIRE_CAPTURE_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.png"), &bytes).unwrap();
    }
    bytes
}
fn finish(view: &Rc<CasinoView>, clock: &FrameClock) {
    let deadline = Instant::now() + Duration::from_secs(12);
    while view.is_animating() {
        assert!(Instant::now() < deadline);
        clock.advance(20.0);
        pump(10);
    }
    clock.advance(400.0);
}
#[test]
fn retained_cards_and_native_animation_frames_with_safe_cancellation() {
    AppBuilder::configure::<App>()
        .use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            ..Default::default()
        })
        .use_skia()
        .setup_without_starting();
    let dir = tempfile::tempdir().unwrap();
    let view = CasinoView::new(dir.path().join("state.json"));
    let window = Window::new();
    window.set_width(1200.0);
    window.set_height(860.0);
    let clock = FrameClock::new();
    window.set_clock(Some(clock.clone()));
    window.set_content(Some(Control::boxed(view.root.clone())));
    view.attach_to_window(&window);
    window.show();
    window.update_layout();
    clock.advance(0.0);
    let content = view.main_content.clone();
    let start_opacity = content.opacity();
    clock.advance(1250.0);
    assert!(
        content.opacity() > start_opacity && content.opacity() < 1.0,
        "startup content opacity must interpolate: {} -> {}",
        start_opacity,
        content.opacity()
    );
    let midway = content.opacity();
    view.model.borrow_mut().current_view = Page::Settings;
    view.show_page();
    window.update_layout();
    clock.advance(0.0);
    assert!(std::ptr::eq(&*content, &*view.main_content));
    assert_eq!(
        content.opacity(),
        midway,
        "navigation must not restart the startup fade"
    );
    let page = view.page.borrow().as_ref().unwrap().clone();
    assert_eq!(
        page.opacity(),
        1.0,
        "replacement pages have no independent fade"
    );
    let tray = page.get_control::<Border>("Tray");
    let brush = tray.border_brush().unwrap();
    let start_point = brush.as_linear_gradient_brush().unwrap().start_point();
    clock.advance(30_000.0);
    let brush = tray.border_brush().unwrap();
    let mid_point = brush.as_linear_gradient_brush().unwrap().start_point();
    assert_ne!(
        start_point, mid_point,
        "the gold brush must interpolate on the native clock"
    );
    // The 300ms tray transition starts only after the page is attached.
    pump(25);
    clock.advance(0.0);
    let start_y = tray.render_transform().unwrap().value().m32;
    clock.advance(150.0);
    let mid_y = tray.render_transform().unwrap().value().m32;
    assert!(
        mid_y < start_y && mid_y > 0.0,
        "tray translation must interpolate: {start_y} -> {mid_y}"
    );
    clock.advance(200.0);
    assert!(tray.render_transform().unwrap().value().m32.abs() < 0.01);

    view.model.borrow_mut().new_game(GameKind::FreeCell, 42);
    view.show_page();
    window.update_layout();
    clock.advance(0.0);
    assert_eq!(view.main_content.opacity(), 1.0);
    assert_eq!(
        view.page.borrow().as_ref().unwrap().opacity(),
        1.0,
        "entering a game must not fade it in again"
    );
    pump(25);
    clock.advance(0.0);
    clock.advance(3000.0);
    let field = view.field.borrow().as_ref().unwrap().clone();
    let from = CardPosition {
        pile: Pile::Tableau(0),
        index: 6,
    };
    let id = view.model.borrow().game().unwrap().board.tableau[0][6].id;
    let control = field.control_by_id(id).unwrap();
    assert_eq!(field.cards.borrow().len(), 52);
    let child_count = field.canvas.children().count();
    let old_x = Canvas::get_left(&control);
    let old_y = Canvas::get_top(&control);
    assert!(view.move_card(from, Pile::Cell(0)));
    assert!(std::ptr::eq(&*control, &*field.control_by_id(id).unwrap()));
    assert_eq!(field.canvas.children().count(), child_count);
    clock.advance(0.0);
    let initial = frame(&window, "move-start");
    clock.advance(175.0);
    let mid_y = Canvas::get_top(&control);
    assert!(
        mid_y < old_y && mid_y > 30.0,
        "card must interpolate from {old_y} to 30, got {mid_y}"
    );
    let middle = frame(&window, "move-middle");
    clock.advance(200.0);
    assert!((Canvas::get_top(&control) - 30.0).abs() < 0.01);
    let end = frame(&window, "move-end");
    assert_ne!(initial, middle);
    assert_ne!(middle, end);
    finish(&view, &clock);
    view.undo();
    clock.advance(0.0);
    clock.advance(400.0);
    finish(&view, &clock);
    assert!(std::ptr::eq(&*control, &*field.control_by_id(id).unwrap()));
    assert!((Canvas::get_left(&control) - old_x).abs() < 0.01);
    assert!((Canvas::get_top(&control) - old_y).abs() < 0.01);
    field.begin_drag(from, Point::new(old_x + 20.0, old_y + 20.0));
    assert!(
        control.transitions().is_none(),
        "dragging must not have easing latency"
    );
    field.move_drag(Point::new(old_x + 80.0, old_y + 80.0));
    assert!((Canvas::get_top(&control) - (old_y + 60.0)).abs() < 0.01);
    field.restore_positions();
    assert!(control.transitions().is_some());
    clock.advance(0.0);
    clock.advance(175.0);
    assert!(
        Canvas::get_top(&control) > old_y && Canvas::get_top(&control) < old_y + 60.0,
        "invalid drop must animate home"
    );
    clock.advance(200.0);

    // Re-dealing on the same view keeps every card instance, including covered cards.
    let controls: Vec<_> = (0..52).map(|id| field.control_by_id(id).unwrap()).collect();
    view.ask_restart();
    view.confirm_dialog();
    assert!(view.is_animating());
    for (id, old) in controls.iter().enumerate() {
        assert!(std::ptr::eq(
            &**old,
            &*field.control_by_id(id as u8).unwrap()
        ));
    }
    assert!(
        field
            .displayed_board
            .borrow()
            .as_ref()
            .unwrap()
            .tableau
            .iter()
            .all(Vec::is_empty)
    );
    pump(100);
    assert!(
        field
            .displayed_board
            .borrow()
            .as_ref()
            .unwrap()
            .tableau
            .iter()
            .all(Vec::is_empty)
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    while field
        .displayed_board
        .borrow()
        .as_ref()
        .unwrap()
        .tableau
        .iter()
        .all(Vec::is_empty)
    {
        assert!(
            Instant::now() < deadline,
            "the initial deal step must run after its delay"
        );
        pump(20);
    }
    let committed = view.model.borrow().game().unwrap().board.clone();
    view.model.borrow_mut().save();
    assert_eq!(
        PersistentState::load(&dir.path().join("state.json"))
            .unwrap()
            .games[1]
            .as_ref()
            .unwrap()
            .board,
        committed
    );
    let displayed = field.displayed_board.borrow().clone();
    view.home();
    assert!(!view.is_animating());
    pump(400);
    assert_eq!(
        *field.displayed_board.borrow(),
        displayed,
        "a cancelled callback must not touch the old field"
    );
    // Exercise the actual title commands with and without a stored game. Every
    // entry must stage a fresh deck rather than silently resuming a final layout.
    for kind in GameKind::ALL {
        for has_saved_game in [false, true] {
            view.home();
            if has_saved_game {
                view.model.borrow_mut().new_game(kind, 42);
                view.home();
            } else {
                view.model.borrow_mut().state.games[kind.index()] = None;
            }
            let played = view.model.borrow().state.statistics[kind.index()].played;
            let button = view
                .page
                .borrow()
                .as_ref()
                .unwrap()
                .get_control::<Button>(kind.name());
            button.raise_event(&ferroui::base::interactivity::RoutedEventArgs::with_event(
                Button::click_event(),
            ));
            assert!(
                view.is_animating(),
                "opening {} must start its deal",
                kind.name()
            );
            assert_eq!(view.model.borrow().current_view, Page::Game(kind));
            assert_eq!(
                view.model.borrow().state.statistics[kind.index()].played,
                played + 1
            );
            let field = view.field.borrow().as_ref().unwrap().clone();
            let displayed = field.displayed_board.borrow();
            let board = displayed.as_ref().unwrap();
            assert!(board.tableau.iter().all(Vec::is_empty));
            assert!(board.cards().all(|c| c.is_face_down));
            assert_eq!(
                board.cards().count(),
                if kind == GameKind::Spider { 104 } else { 52 }
            );
            assert_eq!(view.model.borrow().game().unwrap().elapsed_seconds, 0);
        }
    }
    view.home();
    view.model.borrow_mut().new_game(GameKind::FreeCell, 42);
    view.show_page();
    let active_field = view.field.borrow().as_ref().unwrap().clone();
    let old = active_field.control_by_id(0).unwrap();
    view.ask_new_game();
    view.confirm_dialog();
    assert!(view.is_animating());
    assert!(Rc::ptr_eq(
        &active_field,
        view.field.borrow().as_ref().unwrap()
    ));
    assert!(std::ptr::eq(
        &*old,
        &*active_field.control_by_id(0).unwrap()
    ));
    window.close();
    assert!(!view.is_animating());
    PersistentState::load(&dir.path().join("state.json"))
        .unwrap()
        .validate()
        .unwrap();
}
