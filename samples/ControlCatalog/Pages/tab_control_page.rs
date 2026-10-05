//! Port of `Pages/TabControlPage.xaml.cs`: the class of the document
//! `Pages/TabControlPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::{TabControlPageViewModel, TabControlPageViewModelItem};
use ferroui_base::data::model::BindableList;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::Uri;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{ContentPage, Dock};
use std::rc::Rc;

#[repr(C)]
pub struct TabControlPage {
    base: ContentPage,
}

content_page_class!(TabControlPage);
ferro_class_info!(TabControlPage { new: TabControlPage::new });
xaml_class!(TabControlPage, "/Pages/TabControlPage.xaml");

impl TabControlPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let data_context = TabControlPageViewModel::new();
        data_context.set_tabs(Some(BindableList::new([
            TabControlPageViewModelItem::new()
                .with_header("Arch")
                .with_text("This is the first templated tab page.")
                .with_image(Self::load_bitmap("ferres://ControlCatalog/Assets/delicate-arch-896885_640.jpg")),
            TabControlPageViewModelItem::new()
                .with_header("Leaf")
                .with_text("This is the second templated tab page.")
                .with_image(Self::load_bitmap("ferres://ControlCatalog/Assets/maple-leaf-888807_640.jpg")),
            TabControlPageViewModelItem::new()
                .with_header("Disabled")
                .with_text("You should not see this.")
                .with_is_enabled(false),
        ])));
        data_context.set_tab_placement(Dock::Top);
        this.set_data_context(Some(data_context as BoxedValue));
        this
    }

    /// # Panics
    /// Panics if the asset cannot be opened or decoded.
    fn load_bitmap(uri: &str) -> Rc<Bitmap> {
        let uri = Uri::absolute(uri).unwrap_or_else(|e| panic!("{e}"));
        let mut stream = AssetLoader::open(&uri, None).unwrap_or_else(|e| panic!("{e}"));
        Rc::new(Bitmap::from_stream(&mut stream).unwrap_or_else(|e| panic!("{e}")))
    }
}
