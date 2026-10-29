#![cfg(feature = "headless")]
use ferro_solitaire::{
    app::App, controls::tray_container::brush, models::*, view_models::*,
    views::casino_view::CasinoView,
};
use ferroui::{
    base::{
        PixelSize, Point,
        media::imaging::{BitmapEncoderOptions, RenderTargetBitmap},
        threading::Dispatcher,
    },
    controls::{Button, Image},
    headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessWindowExtensions,
    },
    prelude::*,
    skia::SkiaApplicationExtensions,
};

fn click(view: &std::rc::Rc<CasinoView>, name: &str) {
    let button = view
        .page
        .borrow()
        .as_ref()
        .unwrap()
        .get_control::<Button>(name);
    button.raise_event(&ferroui::base::interactivity::RoutedEventArgs::with_event(
        Button::click_event(),
    ));
    wait_for_presentation(view);
}
fn wait_for_presentation(view: &std::rc::Rc<CasinoView>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(12);
    while view.is_animating() {
        assert!(
            std::time::Instant::now() < deadline,
            "animation sequence did not finish"
        );
        Dispatcher::ui_thread().run_jobs(None);
        ferroui::headless::FerroHeadlessPlatform::force_render_timer_tick(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
fn settle(window: &Ref<Window>) {
    window.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();
}
fn capture(window: &Ref<Window>, name: &str) {
    settle(window);
    if let Ok(dir) = std::env::var("FERRO_SOLITAIRE_CAPTURE_DIR") {
        // Export settled pages; animation frames are captured separately with an IClock.
        let wait = ferroui::base::threading::DispatcherFrame::new();
        let done = wait.clone();
        let timer = ferroui::base::threading::DispatcherTimer::run_once(
            move || done.set_continue(false),
            std::time::Duration::from_millis(2600),
            ferroui::base::threading::DispatcherPriority::NORMAL,
        );
        Dispatcher::ui_thread().push_frame(&wait);
        timer.dispose();
        settle(window);
        std::fs::create_dir_all(&dir).unwrap();
        let bitmap = RenderTargetBitmap::new(PixelSize::new(1200, 860));
        bitmap.render(&window.clone().upcast());
        bitmap
            .save_to_file(
                &format!("{dir}/{name}.png"),
                &BitmapEncoderOptions::Png(Default::default()),
            )
            .unwrap();
    }
}
#[test]
fn native_xaml_pages_card_artwork_commands_drag_and_persistence() {
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
    window.set_background(Some(brush("#116336")));
    window.set_content(Some(Control::boxed(view.root.clone())));
    view.attach_to_window(&window);
    window.show();
    capture(&window, "title");
    assert!(
        view.page
            .borrow()
            .as_ref()
            .unwrap()
            .get_control::<Image>("SolitaireGlyph")
            .source()
            .is_some()
    );
    click(&view, "Settings");
    settle(&window);
    click(&view, "DrawMode");
    click(&view, "Difficulty");
    assert_eq!(
        view.model.borrow().state.settings.draw_mode,
        DrawMode::DrawThree
    );
    assert_eq!(
        view.model.borrow().state.settings.difficulty,
        Difficulty::Medium
    );
    capture(&window, "settings");
    click(&view, "Back");
    for kind in GameKind::ALL {
        view.model.borrow_mut().new_game(kind, 42);
        view.show_page();
        settle(&window);
        capture(&window, kind.name());
        let before = view.model.borrow().game().unwrap().board.clone();
        if kind != GameKind::FreeCell {
            click(&view, "Deal");
            assert_ne!(view.model.borrow().game().unwrap().board, before);
            click(&view, "Undo");
            assert_eq!(view.model.borrow().game().unwrap().board, before);
        }
        click(&view, "Restart");
        assert!(view.dialog_open());
        click(&view, "Cancel");
        assert!(!view.dialog_open());
        click(&view, "NewGame");
        assert!(view.dialog_open());
        click(&view, "Confirm");
        assert!(!view.dialog_open());
        assert_eq!(view.model.borrow().state.statistics[kind.index()].played, 2);
        click(&view, "Home");
    }
    // Exercise routed mouse input through the actual Viewbox coordinate transform.
    view.model.borrow_mut().new_game(GameKind::FreeCell, 42);
    view.show_page();
    settle(&window);
    let field = view.field.borrow().as_ref().unwrap().clone();
    let from = CardPosition {
        pile: Pile::Tableau(0),
        index: view.model.borrow().game().unwrap().board.tableau[0].len() - 1,
    };
    let card_rect = field
        .cards
        .borrow()
        .iter()
        .find(|c| c.position == from)
        .unwrap()
        .rect;
    let cell_rect = field
        .piles
        .borrow()
        .iter()
        .find(|(p, _)| *p == Pile::Cell(0))
        .unwrap()
        .1;
    let source = field
        .canvas
        .translate_point(Point::new(card_rect.x + 30.0, card_rect.y + 60.0), &window)
        .unwrap();
    let dest = field
        .canvas
        .translate_point(Point::new(cell_rect.x + 30.0, cell_rect.y + 60.0), &window)
        .unwrap();
    use ferroui::base::input::{MouseButton, RawInputModifiers};
    window.mouse_down(source, MouseButton::Left, RawInputModifiers::NONE);
    assert_eq!(
        field.selected.get(),
        Some(from),
        "source point {source:?}; source card {card_rect:?}; canvas {:?}",
        field.canvas.bounds()
    );
    assert!(field.drag.get().is_some(), "pointer must capture on press");
    window.mouse_move(dest, RawInputModifiers::LEFT_MOUSE_BUTTON);
    window.mouse_up(dest, MouseButton::Left, RawInputModifiers::NONE);
    assert_eq!(view.model.borrow().game().unwrap().board.cells[0].len(), 1);
    assert!(field.drag.get().is_none());
    wait_for_presentation(&view);
    view.undo();
    wait_for_presentation(&view);
    assert!(view.model.borrow().game().unwrap().board.cells[0].is_empty());
    // Native click-to-move uses the same rule path as dragging.
    window.mouse_down(source, MouseButton::Left, RawInputModifiers::NONE);
    window.mouse_up(source, MouseButton::Left, RawInputModifiers::NONE);
    window.mouse_down(dest, MouseButton::Left, RawInputModifiers::NONE);
    window.mouse_up(dest, MouseButton::Left, RawInputModifiers::NONE);
    assert_eq!(view.model.borrow().game().unwrap().board.cells[0].len(), 1);
    wait_for_presentation(&view);
    // Keyboard commands are routed from the focused native field to the window.
    view.model.borrow_mut().new_game(GameKind::Klondike, 42);
    view.show_page();
    settle(&window);
    view.field.borrow().as_ref().unwrap().canvas.focus();
    use ferroui::base::input::{Key, PhysicalKey};
    window.key_press(
        Key::Space,
        RawInputModifiers::NONE,
        PhysicalKey::Space,
        None,
    );
    window.key_release(
        Key::Space,
        RawInputModifiers::NONE,
        PhysicalKey::Space,
        None,
    );
    assert_eq!(view.model.borrow().game().unwrap().board.waste.len(), 3);
    wait_for_presentation(&view);
    window.key_press(
        Key::Z,
        RawInputModifiers::CONTROL,
        PhysicalKey::Z,
        Some("z"),
    );
    window.key_release(
        Key::Z,
        RawInputModifiers::CONTROL,
        PhysicalKey::Z,
        Some("z"),
    );
    assert!(view.model.borrow().game().unwrap().board.waste.is_empty());
    wait_for_presentation(&view);
    // Completing the last foundation opens the win dialog and records one victory.
    {
        let mut model = view.model.borrow_mut();
        let g = model.game_mut().unwrap();
        let cards: Vec<_> = g.board.cards().copied().collect();
        for pile in &mut g.board.tableau {
            pile.clear();
        }
        g.board.stock.clear();
        for (i, suit) in CardSuit::ALL.into_iter().enumerate() {
            g.board.foundations[i] = cards
                .iter()
                .copied()
                .filter(|c| c.suit == suit)
                .map(|mut c| {
                    c.is_face_down = false;
                    c
                })
                .collect();
            g.board.foundations[i].sort_by_key(|c| c.rank);
        }
        let king = g.board.foundations[3].pop().unwrap();
        g.board.tableau[0].push(king);
    }
    view.show_page();
    assert!(view.move_card(
        CardPosition {
            pile: Pile::Tableau(0),
            index: 0
        },
        Pile::Foundation(3)
    ));
    wait_for_presentation(&view);
    assert!(view.dialog_open());
    assert_eq!(view.model.borrow().state.statistics[0].won, 1);
    capture(&window, "victory");
    click(&view, "Cancel");
    wait_for_presentation(&view);
    view.undo();
    wait_for_presentation(&view);
    assert!(!view.model.borrow().game().unwrap().board.is_game_won);
    assert_eq!(view.model.borrow().state.statistics[0].won, 1);
    view.home();
    click(&view, "Statistics");
    capture(&window, "statistics");
    assert!(
        !view.page.borrow().as_ref().unwrap().is_visible()
            || view.page.borrow().as_ref().unwrap().bounds().width > 0.0
    );
    window.close();
    let restored = PersistentState::load(&dir.path().join("state.json")).unwrap();
    restored.validate().unwrap();
}
