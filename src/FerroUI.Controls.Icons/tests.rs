//! The placement, the properties and the markup of the icons.

use crate::placement::{image_rect, measure};
use crate::{register_types, BitmapIcon, BitmapIconSource, ImageIcon, ImageIconSource};
use ferroui_base::media::{DrawingContext, IImage};
use ferroui_base::platform::{AssetAssembly, AssetStream, IAssetLoader};
use ferroui_base::utilities::{Uri, UriKind, UriRef};
use ferroui_base::{Rect, Size, StaticType};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use std::any::Any;
use std::cell::RefCell;
use std::io::{self, Cursor};
use std::rc::Rc;

/// An asset loader that records the URIs opened; each is an empty stream.
#[derive(Default)]
struct RecordingAssets {
    opened: RefCell<Vec<String>>,
}

impl IAssetLoader for RecordingAssets {
    fn set_default_assembly(&self, _assembly: &AssetAssembly) {}
    fn exists(&self, _uri: &UriRef, _base_uri: Option<&url::Url>) -> bool {
        true
    }
    fn open(&self, uri: &UriRef, _base_uri: Option<&url::Url>) -> io::Result<Box<dyn AssetStream>> {
        self.opened.borrow_mut().push(uri.to_string());
        Ok(Box::new(Cursor::new(Vec::new())))
    }
    fn open_and_get_assembly(
        &self,
        _uri: &UriRef,
        _base_uri: Option<&url::Url>,
    ) -> io::Result<(Box<dyn AssetStream>, AssetAssembly)> {
        Err(io::Error::from(io::ErrorKind::NotFound))
    }
    fn get_assembly(&self, _uri: &UriRef, _base_uri: Option<&url::Url>) -> Option<AssetAssembly> {
        None
    }
    fn get_assets(&self, _uri: &UriRef, _base_uri: Option<&url::Url>) -> Vec<url::Url> {
        Vec::new()
    }
    fn invalidate_assembly_cache(&self, _name: &str) {}
    fn invalidate_assembly_cache_all(&self) {}
}

/// An image that draws nothing.
struct EmptyImage;

impl IImage for EmptyImage {
    fn size(&self) -> Size {
        Size::new(16.0, 16.0)
    }
    fn draw(&self, _context: &mut DrawingContext<'_>, _source_rect: Rect, _dest_rect: Rect) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn uri(text: &str) -> Uri {
    Uri::new(text, UriKind::Absolute).expect("an absolute URI")
}

#[test]
fn an_image_is_scaled_uniformly_and_centered() {
    let rect = image_rect(Size::new(100.0, 40.0), Size::new(20.0, 20.0)).expect("a rectangle");
    assert_eq!(Rect::new(30.0, 0.0, 40.0, 40.0), rect);
}

#[test]
fn nothing_is_drawn_for_an_empty_icon_or_image() {
    assert!(image_rect(Size::new(0.0, 40.0), Size::new(20.0, 20.0)).is_none());
    assert!(image_rect(Size::new(40.0, 40.0), Size::new(0.0, 20.0)).is_none());
}

#[test]
fn an_unconstrained_icon_takes_the_size_of_its_image() {
    let size = measure(Size::new(f64::INFINITY, f64::INFINITY), Size::new(32.0, 16.0));
    assert_eq!(Size::new(32.0, 16.0), size);
}

#[test]
fn a_bitmap_icon_is_monochrome_by_default() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let icon = BitmapIcon::new();
    assert!(icon.show_as_monochrome());
    assert!(icon.source().is_none());

    icon.set_show_as_monochrome(false);
    assert!(!icon.show_as_monochrome());
}

#[test]
fn an_icon_without_an_image_takes_no_space() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let bitmap = BitmapIcon::new();
    let image = ImageIcon::new();
    bitmap.measure(Size::new(50.0, 50.0));
    image.measure(Size::new(50.0, 50.0));

    assert_eq!(Size::default(), bitmap.desired_size());
    assert_eq!(Size::default(), image.desired_size());
}

#[test]
fn the_icons_are_registered_for_markup() {
    register_types();
    assert!(<BitmapIcon as StaticType>::TYPE.name().ends_with("BitmapIcon"));
    assert!(<ImageIcon as StaticType>::TYPE.name().ends_with("ImageIcon"));
    assert!(<BitmapIconSource as StaticType>::TYPE.name().ends_with("BitmapIconSource"));
    assert!(<ImageIconSource as StaticType>::TYPE.name().ends_with("ImageIconSource"));
}

#[test]
fn a_uri_source_is_loaded_when_first_needed_and_once() {
    let assets = Rc::new(RecordingAssets::default());
    let services = TestServices::styled_window().with_asset_loader(assets.clone());
    let _application = UnitTestApplication::start(services);
    let icon = BitmapIcon::new();

    icon.set_uri_source(Some(uri("ferres://App/one.png")));
    assert!(assets.opened.borrow().is_empty(), "setting the property loads nothing");

    icon.measure(Size::new(50.0, 50.0));
    icon.invalidate_measure();
    icon.measure(Size::new(60.0, 60.0));
    // The authority of a URI is canonically lower case; the asset loader matches it ignoring case.
    assert_eq!(vec!["ferres://app/one.png".to_owned()], *assets.opened.borrow());

    icon.set_uri_source(Some(uri("ferres://App/two.png")));
    icon.measure(Size::new(50.0, 50.0));
    assert_eq!(2, assets.opened.borrow().len());
    assert_eq!("ferres://app/two.png", assets.opened.borrow()[1]);
}

#[test]
fn a_uri_that_cannot_be_loaded_shows_nothing() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let icon = BitmapIcon::new();
    icon.set_uri_source(Some(uri("ferres://Missing/none.png")));

    icon.measure(Size::new(50.0, 50.0));

    assert!(icon.displayed_bitmap().is_none());
    assert_eq!(Size::default(), icon.desired_size());
}

#[test]
fn a_bitmap_icon_source_creates_and_updates_its_icons() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let source = BitmapIconSource::new();
    assert!(source.show_as_monochrome());
    source.set_uri_source(Some(uri("ferres://App/one.png")));
    source.set_show_as_monochrome(false);

    let element = source.create_icon_element().expect("an icon");
    let icon = element.cast::<BitmapIcon>().expect("a BitmapIcon");
    assert_eq!(Some("ferres://App/one.png"), icon.uri_source().as_ref().map(Uri::original_string));
    assert!(!icon.show_as_monochrome());

    source.set_uri_source(Some(uri("ferres://App/two.png")));
    source.set_show_as_monochrome(true);
    assert_eq!(Some("ferres://App/two.png"), icon.uri_source().as_ref().map(Uri::original_string));
    assert!(icon.show_as_monochrome());
}

#[test]
fn an_image_icon_source_creates_and_updates_its_icons() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let source = ImageIconSource::new();
    assert!(source.image_source().is_none());

    let element = source.create_icon_element().expect("an icon");
    let icon = element.cast::<ImageIcon>().expect("an ImageIcon");
    assert!(icon.source().is_none());

    let image: Rc<dyn IImage> = Rc::new(EmptyImage);
    source.set_image_source(Some(image.clone()));
    assert!(icon.source().is_some_and(|shown| Rc::ptr_eq(&shown, &image)));
    source.set_image_source(None);
    assert!(icon.source().is_none());
}
