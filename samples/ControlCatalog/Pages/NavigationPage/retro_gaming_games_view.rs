//! Port of `Pages/NavigationPage/RetroGamingGamesView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/RetroGamingGamesView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::Interactive;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Border, Button, Control, SizeChangedEventArgs, UserControl, WrapPanel};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct RetroGamingGamesView {
    base: UserControl,
    game_selected: RefCell<Option<Rc<dyn Fn(&str)>>>,
}

user_control_class!(RetroGamingGamesView);
ferro_class_info!(RetroGamingGamesView { new: RetroGamingGamesView::new });
xaml_class!(RetroGamingGamesView, "/Pages/NavigationPage/RetroGamingGamesView.xaml");

impl RetroGamingGamesView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            game_selected: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        this.on_click("GameCyberNinjaBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Cyber Ninja 2084");
            }
        });
        this.on_click("GameNeonRacerBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Neon Racer");
            }
        });
        this.on_click("GameDungeonBitBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Dungeon Bit");
            }
        });
        this.on_click("GameForestSpiritBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Forest Spirit");
            }
        });
        this.on_click("GamePixelQuestBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Pixel Quest");
            }
        });
        this.on_click("GameSpaceVoidsBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Space Voids");
            }
        });
        this.on_click("GameCyberCityBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Cyber City");
            }
        });

        let weak = this.downgrade();
        this.games_grid().size_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_grid_size_changed(sender, e);
            }
        });
        this
    }

    /// `<button>.Click += (_, _) => ..`: the handler belongs to a child of
    /// the view, so it holds the view weakly.
    fn on_click(&self, button: &str, handler: impl Fn(&RetroGamingGamesView) + 'static) {
        let weak = self.to_ref().downgrade();
        self.get_control::<Button>(button).click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                handler(&this);
            }
        });
    }

    pub fn game_selected(&self) -> Option<Rc<dyn Fn(&str)>> {
        self.game_selected.borrow().clone()
    }

    pub fn set_game_selected(&self, value: Option<Rc<dyn Fn(&str)>>) {
        *self.game_selected.borrow_mut() = value;
    }

    fn games_grid(&self) -> Ref<WrapPanel> {
        self.get_control::<WrapPanel>("GamesGrid")
    }

    fn on_grid_size_changed(&self, _sender: &Interactive, _e: &SizeChangedEventArgs) {
        const DEFAULT_WIDTH: f64 = 145.0;
        let games_grid = self.games_grid();
        let available = games_grid.bounds().width;
        if available <= 0.0 {
            return;
        }

        let single_column = available < DEFAULT_WIDTH * 2.0;
        for child in games_grid.children().snapshot().iter() {
            let card = child
                .cast::<Button>()
                .and_then(|btn| btn.content())
                .and_then(|content| Control::from_boxed(&content))
                .and_then(|content| content.cast::<Border>());
            if let Some(card) = card {
                card.set_width(if single_column { available } else { DEFAULT_WIDTH });
            }
        }
    }
}
