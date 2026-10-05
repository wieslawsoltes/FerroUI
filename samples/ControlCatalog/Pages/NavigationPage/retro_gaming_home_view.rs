//! Port of `Pages/NavigationPage/RetroGamingHomeView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/RetroGamingHomeView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Button, UserControl};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct RetroGamingHomeView {
    base: UserControl,
    game_selected: RefCell<Option<Rc<dyn Fn(&str)>>>,
}

user_control_class!(RetroGamingHomeView);
ferro_class_info!(RetroGamingHomeView { new: RetroGamingHomeView::new });
xaml_class!(RetroGamingHomeView, "/Pages/NavigationPage/RetroGamingHomeView.xaml");

impl RetroGamingHomeView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            game_selected: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        this.on_click("HeroPlayBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Cyber Ninja 2084");
            }
        });
        this.on_click("ContinuePixelQuestBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Pixel Quest");
            }
        });
        this.on_click("ContinueSpaceVoidsBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Space Voids");
            }
        });
        this.on_click("NewReleaseNeonRacerBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Neon Racer");
            }
        });
        this.on_click("NewReleaseDungeonBitBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Dungeon Bit");
            }
        });
        this.on_click("NewReleaseForestSpiritBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Forest Spirit");
            }
        });
        this.on_click("NewReleaseCyberCityBtn", |this| {
            if let Some(game_selected) = this.game_selected() {
                game_selected("Cyber City");
            }
        });
        this
    }

    /// `<button>.Click += (_, _) => ..`: the handler belongs to a child of
    /// the view, so it holds the view weakly.
    fn on_click(&self, button: &str, handler: impl Fn(&RetroGamingHomeView) + 'static) {
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
}
