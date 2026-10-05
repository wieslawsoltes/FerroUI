//! Port of `Pages/NavigationPage/RetroGamingDetailView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/RetroGamingDetailView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{Color, GradientStop, IImageBrushSource, ImageBrush, LinearGradientBrush, Stretch};
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::Uri;
use ferroui_base::{ferro_class_info, instantiate, Ref, RelativePoint, RelativeUnit};
use ferroui_controls::{Border, TextBlock, UserControl};
use std::rc::Rc;

const GAME_ASSETS: [(&str, &str); 8] = [
    ("Cyber Ninja 2084", "hero.jpg"),
    ("Pixel Quest", "pixel_quest.jpg"),
    ("Neon Racer", "neon_racer.jpg"),
    ("Dungeon Bit", "dungeon_bit.jpg"),
    ("Forest Spirit", "forest_spirit.jpg"),
    ("Cyber City", "cyber_city.jpg"),
    ("Neon Ninja", "neon_ninja.jpg"),
    ("Space Voids", "space_voids.jpg"),
];

/// `GameAssets.TryGetValue(title, ..)`.
fn game_asset(title: &str) -> Option<&'static str> {
    GAME_ASSETS.iter().find(|(game, _)| *game == title).map(|(_, file)| *file)
}

#[repr(C)]
pub struct RetroGamingDetailView {
    base: UserControl,
}

user_control_class!(RetroGamingDetailView);
ferro_class_info!(RetroGamingDetailView { new: RetroGamingDetailView::new });
xaml_class!(RetroGamingDetailView, "/Pages/NavigationPage/RetroGamingDetailView.xaml");

/// The bitmap of the asset `uri`; `None` when the asset cannot be opened or
/// decoded (the `catch` of the original).
fn load_bitmap(uri: &str) -> Option<Rc<dyn IImageBrushSource>> {
    let uri = Uri::absolute(uri).ok()?;
    let mut stream = AssetLoader::open(&uri, None).ok()?;
    Some(Rc::new(Bitmap::from_stream(&mut stream).ok()?))
}

impl RetroGamingDetailView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// `new RetroGamingDetailView(gameTitle)`.
    pub fn with_game_title(game_title: &str) -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        this.detail_title_text().set_text(Some(&game_title.to_uppercase()));

        let filename = game_asset(game_title).or_else(|| game_asset("Neon Ninja"));

        if let Some(filename) = filename {
            match load_bitmap(&format!("ferres://ControlCatalog/Assets/RetroGaming/{filename}")) {
                Some(bmp) => {
                    let brush = ImageBrush::with_source(Some(bmp));
                    brush.set_stretch(Stretch::UniformToFill);
                    this.detail_hero_image_border().set_background(Some(brush.into()));
                }
                None => this.set_fallback_background(),
            }
        } else {
            this.set_fallback_background();
        }
        this
    }

    fn detail_hero_image_border(&self) -> Ref<Border> {
        self.get_control::<Border>("DetailHeroImageBorder")
    }

    fn detail_title_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DetailTitleText")
    }

    fn set_fallback_background(&self) {
        let grad = LinearGradientBrush::new();
        grad.set_start_point(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative));
        grad.set_end_point(RelativePoint::new(1.0, 1.0, RelativeUnit::Relative));
        grad.gradient_stops().add(GradientStop::with_color_and_offset(Color::parse("#3d2060").expect("a color"), 0.0));
        grad.gradient_stops().add(GradientStop::with_color_and_offset(Color::parse("#120a1f").expect("a color"), 1.0));
        self.detail_hero_image_border().set_background(Some(grad.into()));
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn game_assets_are_found_by_title() {
        assert_eq!(Some("hero.jpg"), game_asset("Cyber Ninja 2084"));
        assert_eq!(Some("space_voids.jpg"), game_asset("Space Voids"));
        assert_eq!(None, game_asset("cyber city"));
    }
}
