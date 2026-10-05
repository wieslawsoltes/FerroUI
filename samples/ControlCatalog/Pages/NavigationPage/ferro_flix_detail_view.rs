//! Port of the code-behind of the document
//! `Pages/NavigationPage/FerroFlixDetailView.xaml`: the class of the document.

use crate::markup::{user_control_class, xaml_class};
use crate::view_models::random::Random;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{Color, IBrush, IImageBrushSource, ImageBrush, SolidColorBrush, Stretch};
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::Uri;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Border, TextBlock, UserControl};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

const MOVIE_ASSETS: [&str; 12] = [
    "ferres://ControlCatalog/Assets/Movies/trending1.jpg",
    "ferres://ControlCatalog/Assets/Movies/trending2.jpg",
    "ferres://ControlCatalog/Assets/Movies/toprated1.jpg",
    "ferres://ControlCatalog/Assets/Movies/toprated2.jpg",
    "ferres://ControlCatalog/Assets/Movies/toprated3.jpg",
    "ferres://ControlCatalog/Assets/Movies/toprated4.jpg",
    "ferres://ControlCatalog/Assets/Movies/continue1.jpg",
    "ferres://ControlCatalog/Assets/Movies/morelike1.jpg",
    "ferres://ControlCatalog/Assets/Movies/search1.jpg",
    "ferres://ControlCatalog/Assets/Movies/hero.jpg",
    "ferres://ControlCatalog/Assets/Movies/cast1.jpg",
    "ferres://ControlCatalog/Assets/Movies/cast2.jpg",
];

#[repr(C)]
pub struct FerroFlixDetailView {
    base: UserControl,
}

user_control_class!(FerroFlixDetailView);
ferro_class_info!(FerroFlixDetailView { new: FerroFlixDetailView::new });
xaml_class!(FerroFlixDetailView, "/Pages/NavigationPage/FerroFlixDetailView.xaml");

/// `text.GetHashCode()`: a 32 bit hash of the text. The hash of the
/// managed runtime is keyed per process; this one is the same in every run.
fn hash_code(text: &str) -> i32 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    let hash = hasher.finish();
    (hash ^ (hash >> 32)) as u32 as i32
}

/// The year, the rating and the duration shown for the movie `movie_title`.
fn movie_facts(movie_title: &str) -> (String, String, String) {
    let mut rng = Random::with_seed(hash_code(movie_title));

    let year = (2020 + rng.next_max(6)).to_string();
    // `NextDouble()`: the sample of the generator in `0.0..1.0`.
    let next_double = f64::from(rng.next()) * (1.0 / f64::from(i32::MAX));
    let rating = format!("{:.1}/10", 6.5 + next_double * 3.0);
    let mins = 90 + rng.next_max(60);
    let duration = format!("{}h {}m", mins / 60, mins % 60);
    (year, rating, duration)
}

/// The brush of the image of the asset `uri`; `None` when the asset cannot
/// be opened or decoded (the `catch` of the original).
fn image_brush(uri: &str) -> Option<Rc<dyn IBrush>> {
    let uri = Uri::absolute(uri).ok()?;
    let mut stream = AssetLoader::open(&uri, None).ok()?;
    let bitmap: Rc<dyn IImageBrushSource> = Rc::new(Bitmap::from_stream(&mut stream).ok()?);
    let brush = ImageBrush::with_source(Some(bitmap));
    brush.set_stretch(Stretch::UniformToFill);
    Some(brush.into())
}

impl FerroFlixDetailView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// `new FerroFlixDetailView(movieTitle)`.
    ///
    /// # Panics
    /// Panics if the hash of the title is `i32::MIN` (the overflow of
    /// `Math.Abs` in the managed original).
    pub fn with_movie_title(movie_title: &str) -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        this.text_block("HeroTitleLabel").set_text(Some(movie_title));

        let img_idx = hash_code(movie_title).checked_abs().expect("Negating the minimum value is invalid.") as usize
            % MOVIE_ASSETS.len();
        let (year, rating, duration) = movie_facts(movie_title);

        this.text_block("YearLabel").set_text(Some(&year));
        this.text_block("RatingLabel").set_text(Some(&rating));
        this.text_block("DurationLabel").set_text(Some(&duration));

        let background = image_brush(MOVIE_ASSETS[img_idx])
            .unwrap_or_else(|| SolidColorBrush::with_color(Color::parse("#111111").expect("a color")).into());
        this.hero_bg().set_background(Some(background));
        this
    }

    fn hero_bg(&self) -> Ref<Border> {
        self.get_control::<Border>("HeroBg")
    }

    /// The named text blocks `HeroTitleLabel`, `YearLabel`, `RatingLabel` and `DurationLabel`.
    fn text_block(&self, name: &str) -> Ref<TextBlock> {
        self.get_control::<TextBlock>(name)
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn movie_facts_are_in_the_ranges_of_the_original_and_stable() {
        for title in ["Cyber Dune", "Neon Horizon", "Void Runners", ""] {
            let (year, rating, duration) = movie_facts(title);
            let year: i32 = year.parse().expect("a year");
            assert!((2020..2026).contains(&year));
            let rating: f64 = rating.strip_suffix("/10").expect("the suffix").parse().expect("a rating");
            assert!((6.5..=9.5).contains(&rating));
            assert!(duration.starts_with("1h ") || duration.starts_with("2h "));
            assert_eq!((year.to_string(), format!("{rating:.1}/10"), duration), movie_facts(title));
        }
    }
}
