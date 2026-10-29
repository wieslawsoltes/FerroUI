//! CasinoView code-behind. The source pages stay as XAML; Rust wires their named
//! controls to the CasinoViewModel commands, without a second UI framework.
use crate::{
    behaviors::card_field_behavior,
    controls::{CardStackPlacementControl, tray_container::brush},
    models::*,
    view_models::presentation::{CARD_TRANSITION_MS, Presentation, PresentationFrame},
    view_models::*,
    views::pages,
};
use ferroui::{
    base::{
        controls::ResourceDictionary,
        input::{Key, KeyModifiers},
        media::{DrawingImage, IImage},
        reactive::IDisposable,
        threading::{DispatcherPriority, DispatcherTimer},
    },
    controls::{
        Border, Canvas, ContentControl, Dock, DockPanel, Image, Panel, StackPanel, UserControl,
    },
    markup_xaml_loader::FerroRuntimeXamlLoader,
    prelude::*,
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DialogAction {
    None,
    NewGame,
    Restart,
    Won,
}
pub struct CasinoView {
    pub model: RefCell<CasinoViewModel>,
    pub root: Ref<DockPanel>,
    pub host: Ref<ContentControl>,
    pub main_content: Ref<Panel>,
    pub status: Ref<TextBlock>,
    pub field: RefCell<Option<Rc<CardStackPlacementControl>>>,
    pub page: RefCell<Option<Ref<UserControl>>>,
    artwork: Ref<ResourceDictionary>,
    dialog: Cell<DialogAction>,
    timer: RefCell<Option<Rc<dyn IDisposable>>>,
    presentation: RefCell<Option<VecDeque<PresentationFrame>>>,
    presentation_timer: RefCell<Option<Rc<dyn IDisposable>>>,
    presentation_generation: Cell<u64>,
}
impl CasinoView {
    pub fn new(path: PathBuf) -> Rc<Self> {
        let artwork: Ref<ResourceDictionary> =
            FerroRuntimeXamlLoader::parse_as(include_str!("../styles/CardsStyle.xaml"), None)
                .expect("valid card artwork XAML");
        let root: Ref<DockPanel> =
            FerroRuntimeXamlLoader::parse_as(include_str!("CasinoView.xaml"), None)
                .expect("valid casino background XAML");
        ferroui::controls::documents::TextElement::set_font_family_on(
            &root,
            ferroui::base::media::FontFamily::parse("fonts:Solitaire#Nunito").expect("font URI"),
        );
        let status = TextBlock::new();
        status.set_foreground(Some(brush("#DDEBDD")));
        status.set_font_size(14.0);
        status.set_margin(Thickness::new(20.0, 4.0, 20.0, 12.0));
        status.set_horizontal_alignment(HorizontalAlignment::Center);
        DockPanel::set_dock(&status, Dock::Bottom);
        root.children().add(status.clone());
        let host = ContentControl::new();
        // The source fades this persistent container once when the casino
        // attaches, rather than fading each replacement navigation page.
        let main_content = Panel::new();
        main_content.classes().add("mainContent");
        main_content.children().add(host.clone());
        root.children().add(main_content.clone());
        let this = Rc::new(Self {
            model: RefCell::new(CasinoViewModel::load(path)),
            root,
            host,
            main_content,
            status,
            artwork,
            field: RefCell::new(None),
            page: RefCell::new(None),
            dialog: Cell::new(DialogAction::None),
            timer: RefCell::new(None),
            presentation: RefCell::new(None),
            presentation_timer: RefCell::new(None),
            presentation_generation: Cell::new(0),
        });
        this.show_page();
        this
    }
    pub fn attach_to_window(self: &Rc<Self>, window: &Ref<Window>) {
        let weak = Rc::downgrade(self);
        window.key_down(move |_, event| {
            let Some(this) = weak.upgrade() else { return };
            if this.input_blocked() {
                if event.key == Key::Escape {
                    this.dismiss_dialog();
                    event.set_handled(true);
                }
                return;
            }
            let cmd = event
                .key_modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::META);
            match event.key {
                Key::Escape => {
                    if let Some(field) = this.field.borrow().as_ref() {
                        field.select(None);
                        field.restore_positions();
                    }
                }
                Key::Z if cmd => this.undo(),
                Key::N if cmd => this.ask_new_game(),
                Key::Space => this.draw(),
                Key::H => this.hint(),
                Key::A => this.auto_move(),
                _ => return,
            }
            event.set_handled(true);
        });
        // The close handler owns the view until the window closes; all other
        // handlers point back weakly, avoiding control/view reference cycles.
        let this = self.clone();
        window.closed(move || {
            this.cancel_presentation();
            this.model.borrow_mut().save();
            if let Some(timer) = this.timer.borrow_mut().take() {
                timer.dispose();
            }
        });
        let weak = Rc::downgrade(self);
        let weak_window = window.downgrade();
        let last = Instant::now();
        // A Cell keeps the Fn timer's monotonic baseline without touching core models.
        let last_tick = Cell::new(last);
        let autosave_ticks = Cell::new(0u32);
        *self.timer.borrow_mut() = Some(DispatcherTimer::run(
            move || {
                let (Some(this), Some(window)) = (weak.upgrade(), weak_window.upgrade()) else {
                    return false;
                };
                let last = last_tick.get();
                let now = Instant::now();
                let seconds = now.duration_since(last).as_secs();
                if seconds > 0 {
                    last_tick.set(last + Duration::from_secs(seconds));
                    if window.is_active() && !this.input_blocked() {
                        this.model.borrow_mut().tick(seconds);
                    }
                    this.update_score();
                    autosave_ticks.set(autosave_ticks.get() + 1);
                    if autosave_ticks.get() >= 30 {
                        autosave_ticks.set(0);
                        this.model.borrow_mut().save();
                    }
                }
                true
            },
            Duration::from_millis(250),
            DispatcherPriority::BACKGROUND,
        ));
    }
    pub fn show_page(self: &Rc<Self>) {
        self.cancel_presentation();
        self.dialog.set(DialogAction::None);
        self.field.borrow_mut().take();
        let current = self.model.borrow().current_view;
        let view = match current {
            Page::Title => {
                let view = pages::title_view();
                let image = view.get_control::<Image>("SolitaireGlyph");
                let value = self.artwork.get(&"SolitaireGlyph".into()).unwrap();
                let image_source: Rc<dyn IImage> = value
                    .downcast_ref::<Ref<DrawingImage>>()
                    .unwrap()
                    .clone()
                    .into();
                image.set_source(Some(image_source));
                for kind in GameKind::ALL {
                    self.bind(&view, kind.name(), move |this| {
                        this.open_game(kind);
                    });
                }
                self.bind(&view, "Settings", |this| {
                    this.model.borrow_mut().current_view = Page::Settings;
                    this.show_page();
                });
                self.bind(&view, "Statistics", |this| {
                    this.model.borrow_mut().current_view = Page::Statistics;
                    this.show_page();
                });
                view
            }
            Page::Settings => {
                let view = pages::settings_view();
                let settings = self.model.borrow().state.settings.clone();
                view.get_control::<Button>("DrawMode")
                    .set_content(Some(Rc::new(format!(
                        "{} card{}",
                        settings.draw_mode.count(),
                        if settings.draw_mode == DrawMode::DrawThree {
                            "s"
                        } else {
                            ""
                        }
                    ))));
                view.get_control::<Button>("Difficulty")
                    .set_content(Some(Rc::new(settings.difficulty.label().to_string())));
                self.bind(&view, "DrawMode", |this| {
                    let mut model = this.model.borrow_mut();
                    model.state.settings.draw_mode =
                        if model.state.settings.draw_mode == DrawMode::DrawOne {
                            DrawMode::DrawThree
                        } else {
                            DrawMode::DrawOne
                        };
                    model.save();
                    drop(model);
                    this.show_page();
                });
                self.bind(&view, "Difficulty", |this| {
                    let mut model = this.model.borrow_mut();
                    model.state.settings.difficulty = match model.state.settings.difficulty {
                        Difficulty::Easy => Difficulty::Medium,
                        Difficulty::Medium => Difficulty::Hard,
                        Difficulty::Hard => Difficulty::Easy,
                    };
                    model.save();
                    drop(model);
                    this.show_page();
                });
                self.bind(&view, "Back", Self::home);
                view
            }
            Page::Statistics => {
                let view = pages::statistics_view();
                let panel = view.get_control::<StackPanel>("GameStatistics");
                for kind in GameKind::ALL {
                    let stats = self.model.borrow().state.statistics[kind.index()].clone();
                    let column = StackPanel::new();
                    column.set_width(250.0);
                    column.set_spacing(18.0);
                    for (text, size, color) in [
                        (kind.name().to_string(), 28.0, "#FCFC80"),
                        (format!("Games played     {}", stats.played), 19.0, "White"),
                        (format!("Games won         {}", stats.won), 19.0, "White"),
                        (
                            format!(
                                "Win rate               {:.0}%",
                                if stats.played == 0 {
                                    0.0
                                } else {
                                    f64::from(stats.won) * 100.0 / f64::from(stats.played)
                                }
                            ),
                            19.0,
                            "White",
                        ),
                        (
                            format!(
                                "Best score          {}",
                                stats.best_score.map_or("—".into(), |v| v.to_string())
                            ),
                            19.0,
                            "White",
                        ),
                        (
                            format!(
                                "Best time            {}",
                                stats.best_seconds.map_or("—".into(), time_text)
                            ),
                            19.0,
                            "White",
                        ),
                        (
                            format!(
                                "Fewest moves    {}",
                                stats.fewest_moves.map_or("—".into(), |v| v.to_string())
                            ),
                            19.0,
                            "White",
                        ),
                        (
                            format!("Time played       {}", time_text(stats.total_seconds)),
                            19.0,
                            "#C5DACB",
                        ),
                    ] {
                        let label = TextBlock::new();
                        label.set_text(Some(&text));
                        label.set_font_size(size);
                        label.set_foreground(Some(brush(color)));
                        column.children().add(label);
                    }
                    panel.children().add(column);
                }
                self.bind(&view, "Back", Self::home);
                view
            }
            Page::Game(kind) => {
                let view = pages::card_game_view();
                view.get_control::<TextBlock>("GameName")
                    .set_text(Some(kind.name()));
                let field = CardStackPlacementControl::new(
                    view.get_control::<Canvas>("CardField"),
                    self.artwork.clone(),
                );
                field.rebuild(self.model.borrow().game().unwrap());
                card_field_behavior::attach(&field, self);
                *self.field.borrow_mut() = Some(field);
                self.bind(&view, "Home", Self::home);
                self.bind(&view, "NewGame", Self::ask_new_game);
                self.bind(&view, "Restart", Self::ask_restart);
                self.bind(&view, "Undo", Self::undo);
                self.bind(&view, "Hint", Self::hint);
                self.bind(&view, "AutoMove", Self::auto_move);
                self.bind(&view, "Deal", Self::draw);
                self.bind(&view, "Confirm", Self::confirm_dialog);
                self.bind(&view, "Cancel", Self::dismiss_dialog);
                view.get_control::<Button>("AutoMove")
                    .set_is_visible(kind != GameKind::Spider);
                view.get_control::<Button>("Deal")
                    .set_is_visible(kind != GameKind::FreeCell);
                view.get_control::<Button>("Deal").set_content(Some(Rc::new(
                    if kind == GameKind::Spider {
                        "Deal row"
                    } else {
                        "Draw"
                    }
                    .to_string(),
                )));
                view
            }
        };
        self.host.set_content(Some(Control::boxed(view.clone())));
        let weak_view = view.downgrade();
        // Run after attachment/styles have established the tray's starting transform.
        DispatcherTimer::run_once(
            move || {
                if let Some(view) = weak_view.upgrade()
                    && view.is_attached_to_visual_tree()
                    && let Some(tray) = view
                        .find_control::<Border>("Tray")
                        .or_else(|| view.find_control::<Border>("DeckControls"))
                {
                    tray.classes().add("animateUp");
                }
            },
            Duration::from_millis(16),
            DispatcherPriority::NORMAL,
        );
        *self.page.borrow_mut() = Some(view);
        self.update_score();
        self.refresh_message();
        if self
            .model
            .borrow()
            .game()
            .is_some_and(|g| g.board.is_game_won)
        {
            self.show_dialog(
                DialogAction::Won,
                "You won!",
                "Every card is home. Play another game?",
                "New game",
            );
        }
    }
    fn bind(
        self: &Rc<Self>,
        view: &Ref<UserControl>,
        name: &str,
        action: impl Fn(&Rc<Self>) + 'static,
    ) {
        let weak = Rc::downgrade(self);
        view.get_control::<Button>(name).click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                action(&this);
            }
        });
    }
    /// Title buttons and desktop game arguments share the source's fresh-deal path.
    pub fn open_game(self: &Rc<Self>, kind: GameKind) {
        self.model.borrow_mut().navigate_to_game(kind);
        self.model.borrow_mut().save();
        self.show_page();
        self.start_deal();
    }
    pub fn home(self: &Rc<Self>) {
        self.model.borrow_mut().current_view = Page::Title;
        self.model.borrow_mut().save();
        self.show_page();
    }
    pub fn set_message(&self, text: &str) {
        self.model.borrow_mut().message = text.to_string();
        self.refresh_message();
    }
    fn refresh_message(&self) {
        let model = self.model.borrow();
        let text = if !model.message.is_empty() {
            model.message.as_str()
        } else if matches!(model.current_view, Page::Game(_)) {
            "Drag a card or run · Click, then click a destination · Double-click to send to a foundation"
        } else {
            "Klondike · FreeCell · Spider"
        };
        self.status.set_text(Some(text));
    }
    pub fn update_score(&self) {
        let model = self.model.borrow();
        if let (Some(game), Some(view)) = (model.game(), self.page.borrow().as_ref()) {
            view.get_control::<TextBlock>("Score")
                .set_text(Some(&format!(
                    "Score  {}     ·     Moves  {}     ·     Time  {}     ·     {}",
                    game.board.score,
                    game.board.moves,
                    time_text(game.elapsed_seconds),
                    match game.kind {
                        GameKind::Klondike => format!(
                            "Draw {} · {} in stock",
                            game.draw_mode.count(),
                            game.board.stock.len()
                        ),
                        GameKind::Spider => format!(
                            "{} · {} deals left",
                            game.difficulty.label(),
                            game.board.stock.len() / 10
                        ),
                        GameKind::FreeCell => format!(
                            "{} free cells",
                            game.board.cells.iter().filter(|p| p.is_empty()).count()
                        ),
                    }
                )));
            let busy = self.is_animating();
            view.get_control::<Button>("Undo")
                .set_is_enabled(!busy && game.can_undo());
            for name in ["NewGame", "Restart", "Hint", "AutoMove", "Deal"] {
                view.get_control::<Button>(name).set_is_enabled(!busy);
            }
        }
    }
    fn changed(self: &Rc<Self>) {
        self.model.borrow_mut().message.clear();
        self.model.borrow_mut().save();
        let board = self.model.borrow().game().map(|g| g.board.clone());
        if let Some(board) = board {
            self.play_presentation(
                Presentation {
                    initial: board.clone(),
                    frames: VecDeque::from([PresentationFrame {
                        delay_ms: CARD_TRANSITION_MS,
                        board,
                    }]),
                },
                true,
            );
        }
    }
    pub fn is_animating(&self) -> bool {
        self.presentation.borrow().is_some()
    }
    pub fn input_blocked(&self) -> bool {
        self.dialog_open() || self.is_animating()
    }
    fn cancel_presentation(&self) {
        self.presentation_generation
            .set(self.presentation_generation.get().wrapping_add(1));
        if let Some(timer) = self.presentation_timer.borrow_mut().take() {
            timer.dispose();
        }
        self.presentation.borrow_mut().take();
    }
    /// Stage the already committed initial board using the source deal timings.
    pub fn start_deal(self: &Rc<Self>) {
        let sequence = self.model.borrow().game().map(Presentation::deal);
        if let Some(sequence) = sequence {
            self.play_presentation(sequence, false);
        }
    }
    fn play_presentation(self: &Rc<Self>, sequence: Presentation, animate_initial: bool) {
        self.cancel_presentation();
        *self.presentation.borrow_mut() = Some(sequence.frames);
        if let (Some(field), Some(game)) =
            (self.field.borrow().as_ref(), self.model.borrow().game())
        {
            field.present(
                game.kind,
                game.draw_mode,
                &sequence.initial,
                animate_initial,
            );
        }
        self.update_score();
        self.refresh_message();
        self.advance_presentation();
    }
    fn advance_presentation(self: &Rc<Self>) {
        let next = self
            .presentation
            .borrow_mut()
            .as_mut()
            .and_then(VecDeque::pop_front);
        if let Some(frame) = next {
            let weak = Rc::downgrade(self);
            let generation = self.presentation_generation.get();
            *self.presentation_timer.borrow_mut() = Some(DispatcherTimer::run_once(
                move || {
                    let Some(this) = weak.upgrade() else { return };
                    if this.presentation_generation.get() != generation {
                        return;
                    }
                    if let (Some(field), Some(game)) =
                        (this.field.borrow().as_ref(), this.model.borrow().game())
                    {
                        field.present(game.kind, game.draw_mode, &frame.board, true);
                    }
                    this.advance_presentation();
                },
                Duration::from_millis(frame.delay_ms),
                DispatcherPriority::NORMAL,
            ));
        } else {
            self.presentation.borrow_mut().take();
            self.presentation_timer.borrow_mut().take();
            self.update_score();
            self.refresh_message();
            if self
                .model
                .borrow()
                .game()
                .is_some_and(|g| g.board.is_game_won)
            {
                self.show_dialog(
                    DialogAction::Won,
                    "You won!",
                    "Every card is home. Play another game?",
                    "New game",
                );
            }
        }
    }
    pub fn move_card(self: &Rc<Self>, from: CardPosition, to: Pile) -> bool {
        if self.input_blocked() {
            return false;
        }
        let moved = self
            .model
            .borrow_mut()
            .game_mut()
            .is_some_and(|g| g.check_and_move_card(from, to));
        if moved {
            self.changed();
        }
        moved
    }
    pub fn move_to_foundation(self: &Rc<Self>, from: CardPosition) {
        if self.input_blocked() {
            return;
        }
        let moved = self
            .model
            .borrow_mut()
            .game_mut()
            .is_some_and(|g| g.try_move_card_to_appropriate_foundation(from));
        if moved {
            self.changed();
        } else {
            self.set_message("This card cannot move to a foundation yet.");
        }
    }
    pub fn draw(self: &Rc<Self>) {
        if self.input_blocked() {
            return;
        }
        let sequence = {
            let mut model = self.model.borrow_mut();
            model.game_mut().and_then(|g| {
                let before = g.board.clone();
                if g.turn_stock() {
                    Some(Presentation::stock(g, before))
                } else {
                    None
                }
            })
        };
        if let Some(sequence) = sequence {
            self.model.borrow_mut().message.clear();
            self.model.borrow_mut().save();
            self.play_presentation(sequence, true);
        } else {
            self.set_message(
                "No cards to draw. In Spider, fill every empty column before dealing.",
            );
        }
    }
    pub fn undo(self: &Rc<Self>) {
        if self.input_blocked() {
            return;
        }
        let moved = self
            .model
            .borrow_mut()
            .game_mut()
            .is_some_and(|g| g.undo_move());
        if moved {
            self.changed();
        }
    }
    pub fn auto_move(self: &Rc<Self>) {
        if self.input_blocked() {
            return;
        }
        let sequence = self
            .model
            .borrow_mut()
            .game_mut()
            .and_then(Presentation::auto_move);
        if let Some(sequence) = sequence {
            self.model.borrow_mut().message.clear();
            self.model.borrow_mut().save();
            self.play_presentation(sequence, true);
        } else {
            self.set_message("No cards can move to a foundation yet.");
        }
    }
    pub fn hint(self: &Rc<Self>) {
        if self.input_blocked() {
            return;
        }
        let hint = self.model.borrow().game().and_then(|g| g.hint());
        if let (Some(hint), Some(field)) = (hint, self.field.borrow().as_ref()) {
            field.select(Some(hint.from));
            field.highlight_target(hint.to);
            self.set_message(&format!(
                "Move the highlighted card to {}.",
                pile_name(hint.to)
            ));
        } else {
            self.set_message("No tableau move found. Try the stock, a free cell, or Undo.");
        }
    }
    pub fn dialog_open(&self) -> bool {
        self.dialog.get() != DialogAction::None
    }
    fn show_dialog(&self, action: DialogAction, title: &str, message: &str, confirm: &str) {
        if let Some(field) = self.field.borrow().as_ref() {
            field.restore_positions();
            field.select(None);
        }
        if let Some(view) = self.page.borrow().as_ref() {
            self.dialog.set(action);
            view.get_control::<TextBlock>("DialogTitle")
                .set_text(Some(title));
            view.get_control::<TextBlock>("DialogMessage")
                .set_text(Some(message));
            view.get_control::<Button>("Confirm")
                .set_content(Some(Rc::new(confirm.to_string())));
            if action == DialogAction::Won {
                view.get_control::<Border>("DeckControls")
                    .classes()
                    .remove("animateUp");
            }
            view.get_control::<Border>("Dialog").set_is_visible(true);
            view.get_control::<Canvas>("CardField")
                .set_is_enabled(false);
        }
    }
    pub fn dismiss_dialog(self: &Rc<Self>) {
        self.dialog.set(DialogAction::None);
        if let Some(view) = self.page.borrow().as_ref() {
            view.get_control::<Border>("DeckControls")
                .classes()
                .add("animateUp");
            view.get_control::<Border>("Dialog").set_is_visible(false);
            view.get_control::<Canvas>("CardField").set_is_enabled(true);
        }
    }
    pub fn ask_new_game(self: &Rc<Self>) {
        if !self.input_blocked() && self.model.borrow().game().is_some() {
            self.show_dialog(
                DialogAction::NewGame,
                "New game?",
                "Replace this deal with a freshly shuffled deck?",
                "New game",
            );
        }
    }
    pub fn ask_restart(self: &Rc<Self>) {
        if !self.input_blocked() && self.model.borrow().game().is_some() {
            self.show_dialog(
                DialogAction::Restart,
                "Restart this deal?",
                "Return every card to its starting position and reset the clock?",
                "Restart",
            );
        }
    }
    pub fn confirm_dialog(self: &Rc<Self>) {
        let action = self.dialog.replace(DialogAction::None);
        if action == DialogAction::None {
            return;
        }
        let kind = self.model.borrow().game().map(|g| g.kind);
        if let Some(kind) = kind {
            if action == DialogAction::Restart {
                self.model.borrow_mut().game_mut().unwrap().restart();
            } else {
                self.model.borrow_mut().new_game(kind, rand::random());
            }
            self.model.borrow_mut().save();
            self.dismiss_dialog();
            self.start_deal();
        }
    }
}
impl Drop for CasinoView {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.get_mut().take() {
            timer.dispose();
        }
        if let Some(timer) = self.presentation_timer.get_mut().take() {
            timer.dispose();
        }
    }
}
pub fn time_text(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
fn pile_name(pile: Pile) -> String {
    match pile {
        Pile::Tableau(i) => format!("column {}", i + 1),
        Pile::Foundation(i) => format!(
            "the {} foundation",
            CardSuit::ALL.get(i).map_or("completed run", |s| s.symbol())
        ),
        Pile::Cell(i) => format!("free cell {}", i + 1),
        Pile::Stock => "the stock".into(),
        Pile::Waste => "the waste".into(),
    }
}
