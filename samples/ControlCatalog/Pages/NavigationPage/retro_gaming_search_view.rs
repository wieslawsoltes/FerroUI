//! Port of `Pages/NavigationPage/RetroGamingSearchView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/RetroGamingSearchView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Button, UserControl};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct RetroGamingSearchView {
    base: UserControl,
    close_requested: RefCell<Option<Rc<dyn Fn()>>>,
    game_selected: RefCell<Option<Rc<dyn Fn(&str)>>>,
}

user_control_class!(RetroGamingSearchView);
ferro_class_info!(RetroGamingSearchView { new: RetroGamingSearchView::new });
xaml_class!(RetroGamingSearchView, "/Pages/NavigationPage/RetroGamingSearchView.xaml");

impl RetroGamingSearchView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            close_requested: RefCell::new(None),
            game_selected: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        this.on_click("CloseBtn", |this| {
            if let Some(close_requested) = this.close_requested() {
                close_requested();
            }
        });
        this.on_click("SearchCyberNinjaBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Cyber Ninja 2084");
            }
        });
        this.on_click("SearchNeonRacerBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Neon Racer");
            }
        });
        this.on_click("SearchDungeonBitBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Dungeon Bit");
            }
        });
        this.on_click("SearchForestSpiritBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Forest Spirit");
            }
        });
        this.on_click("SearchPixelQuestBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Pixel Quest");
            }
        });
        this.on_click("SearchSpaceVoidsBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Space Voids");
            }
        });
        this.on_click("SearchCyberCityBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Cyber City");
            }
        });
        this
    }

    /// `<button>.Click += (_, _) => ..`: the handler belongs to a child of
    /// the view, so it holds the view weakly.
    fn on_click(&self, button: &str, handler: impl Fn(&RetroGamingSearchView) + 'static) {
        let weak = self.to_ref().downgrade();
        self.get_control::<Button>(button).click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                handler(&this);
            }
        });
    }

    pub fn close_requested(&self) -> Option<Rc<dyn Fn()>> {
        self.close_requested.borrow().clone()
    }

    pub fn set_close_requested(&self, value: Option<Rc<dyn Fn()>>) {
        *self.close_requested.borrow_mut() = value;
    }

    pub fn game_selected(&self) -> Option<Rc<dyn Fn(&str)>> {
        self.game_selected.borrow().clone()
    }

    pub fn set_game_selected(&self, value: Option<Rc<dyn Fn(&str)>>) {
        *self.game_selected.borrow_mut() = value;
    }
}
