//! Port of `Pages/NavigationPage/LAvenirDishDetailView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/LAvenirDishDetailView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{Color, IBrush, IImageBrushSource, ImageBrush, SolidColorBrush, Stretch};
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::Uri;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Border, TextBlock, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct LAvenirDishDetailView {
    base: UserControl,
}

user_control_class!(LAvenirDishDetailView);
ferro_class_info!(LAvenirDishDetailView { new: LAvenirDishDetailView::new });
xaml_class!(LAvenirDishDetailView, "/Pages/NavigationPage/LAvenirDishDetailView.xaml");

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

impl LAvenirDishDetailView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// `new LAvenirDishDetailView(name, price, description, imageFile)`.
    pub fn with_dish(name: &str, price: &str, description: &str, image_file: &str) -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        this.title_label().set_text(Some(name));
        this.price_label().set_text(Some(price));
        this.description_label().set_text(Some(description));

        let background = image_brush(&format!("ferres://ControlCatalog/Assets/Restaurant/{image_file}"))
            .unwrap_or_else(|| SolidColorBrush::with_color(Color::parse("#1a1836").expect("a color")).into());
        this.hero_bg().set_background(Some(background));
        this
    }

    fn hero_bg(&self) -> Ref<Border> {
        self.get_control::<Border>("HeroBg")
    }

    fn title_label(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("TitleLabel")
    }

    fn price_label(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("PriceLabel")
    }

    fn description_label(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DescriptionLabel")
    }
}
