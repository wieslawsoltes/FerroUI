//! The sources of animated visuals (`IAnimatedVisualSource`, `IAnimatedVisualSource2`,
//! `IDynamicAnimatedVisualSource` and `LottieDocumentSource`).

use crate::{AnimatedVisual, AnimatedVisualError};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::media::Color;
use ferroui_base::platform::IAssetLoader;
use ferroui_base::utilities::UriRef;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroLocator, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, LocatorExtensions, Ref, StyledProperty,
};
use ferroui_markup_xaml::ServiceProviderExtensions;
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;
use url::Url;

/// Makes the animated visual a player or an icon shows.
pub trait IAnimatedVisualSource {
    fn try_create_animated_visual(&self) -> Result<Rc<AnimatedVisual>, AnimatedVisualError>;

    /// The colours set on the source by name (`SetColorProperty`), which the paints bound to
    /// those names take.
    fn color_properties(&self) -> Vec<(String, Color)> {
        Vec::new()
    }

    /// The source as one whose visual can change.
    fn as_dynamic(&self) -> Option<&dyn IDynamicAnimatedVisualSource> {
        None
    }
}

/// A source with named positions, as progress from 0 to 1.
pub trait IAnimatedVisualSource2: IAnimatedVisualSource {
    fn markers(&self) -> Vec<(String, f64)> {
        self.try_create_animated_visual().map_or_else(|_| Vec::new(), |visual| visual.progress_markers())
    }
}

/// A source whose visual can change: a player makes it again when it says so.
pub trait IDynamicAnimatedVisualSource: IAnimatedVisualSource {
    fn add_animated_visual_invalidated(&self, handler: Rc<dyn Fn()>);
}

impl AnimatedVisual {
    /// The markers as progress from 0 to 1, in the order of the document.
    pub fn progress_markers(&self) -> Vec<(String, f64)> {
        let length = self.out_point - self.in_point;
        self.markers
            .iter()
            .map(|marker| {
                let progress = if length > 0.0 { (marker.frame - self.in_point) / length } else { 0.0 };
                (marker.name.clone(), progress.clamp(0.0, 1.0))
            })
            .collect()
    }
}

/// Gives the bytes of an image of a document that is in a file, by its path in the document.
pub type ImageLoader = dyn Fn(&str) -> Option<Vec<u8>>;

/// A source of a Lottie document. It is read once, when it is first asked for.
#[derive(Default)]
pub struct LottieDocumentSource {
    json: RefCell<Option<Rc<str>>>,
    visual: RefCell<Option<Rc<AnimatedVisual>>>,
    image_loader: RefCell<Option<Rc<ImageLoader>>>,
    colors: RefCell<Vec<(String, Color)>>,
    invalidated: RefCell<Vec<Rc<dyn Fn()>>>,
}

impl LottieDocumentSource {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// A source of the document `json`.
    pub fn create_from_string(json: &str) -> Rc<Self> {
        let source = Self::new();
        source.set_source(json);
        source
    }

    /// Reads the document from `stream` (`SetSourceAsync` of a stream).
    pub fn set_source_stream(&self, mut stream: impl Read) -> std::io::Result<()> {
        let mut json = String::new();
        stream.read_to_string(&mut json)?;
        self.set_source(&json);
        Ok(())
    }

    /// Sets what loads the images a document keeps in files. Without it those images are not
    /// drawn. Players of this source make their visual again.
    pub fn set_image_loader(&self, loader: Option<Rc<ImageLoader>>) {
        *self.image_loader.borrow_mut() = loader;
        self.invalidate();
    }

    /// Replaces the document; players of this source make their visual again.
    pub fn set_source(&self, json: &str) {
        *self.json.borrow_mut() = Some(Rc::from(json));
        self.invalidate();
    }

    /// Sets the colour of the paints bound to `name` (`SetColorProperty`). The document is not
    /// read again.
    pub fn set_color_property(&self, name: &str, color: Color) {
        {
            let mut colors = self.colors.borrow_mut();
            match colors.iter_mut().find(|(existing, _)| existing == name) {
                Some(entry) => entry.1 = color,
                None => colors.push((name.to_owned(), color)),
            }
        }
        self.notify();
    }

    fn invalidate(&self) {
        self.visual.borrow_mut().take();
        self.notify();
    }

    fn notify(&self) {
        let handlers = self.invalidated.borrow().clone();
        for handler in handlers {
            handler();
        }
    }
}

impl IAnimatedVisualSource for LottieDocumentSource {
    fn try_create_animated_visual(&self) -> Result<Rc<AnimatedVisual>, AnimatedVisualError> {
        if let Some(visual) = self.visual.borrow().as_ref() {
            return Ok(visual.clone());
        }
        let json = self.json.borrow().clone().ok_or(AnimatedVisualError::Invalid("source"))?;
        let mut visual = AnimatedVisual::parse(&json)?;
        let loader = self.image_loader.borrow().clone();
        if let Some(loader) = loader {
            for (id, path) in visual.image_files() {
                if let Some(bytes) = loader(&path) {
                    visual.set_image_bytes(&id, bytes);
                }
            }
        }
        let visual = Rc::new(visual);
        *self.visual.borrow_mut() = Some(visual.clone());
        Ok(visual)
    }

    fn color_properties(&self) -> Vec<(String, Color)> {
        self.colors.borrow().clone()
    }

    fn as_dynamic(&self) -> Option<&dyn IDynamicAnimatedVisualSource> {
        Some(self)
    }
}

impl IAnimatedVisualSource2 for LottieDocumentSource {}

impl IDynamicAnimatedVisualSource for LottieDocumentSource {
    fn add_animated_visual_invalidated(&self, handler: Rc<dyn Fn()>) {
        self.invalidated.borrow_mut().push(handler);
    }
}

/// A source as an object of markup: what the `Source` of a player or an icon holds.
#[repr(C)]
pub struct AnimatedVisualSource {
    base: FerroObject,
    source: RefCell<Option<Rc<dyn IAnimatedVisualSource>>>,
}

ferro_class!(AnimatedVisualSource: FerroObject);
ferro_class_info!(AnimatedVisualSource {
    new: AnimatedVisualSource::new,
    markup: { namespace: "FerroUI.AnimatedVisuals" },
});

impl FerroObjectImpl for AnimatedVisualSource {}

impl AnimatedVisualSource {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), source: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// An object for a source implemented in code.
    pub fn from_source(source: Rc<dyn IAnimatedVisualSource>) -> Ref<Self> {
        let this = Self::new();
        *this.source.borrow_mut() = Some(source);
        this
    }

    /// The source this object stands for.
    pub fn source(&self) -> Option<Rc<dyn IAnimatedVisualSource>> {
        self.source.borrow().clone()
    }

    pub(crate) fn set_inner(&self, source: Rc<dyn IAnimatedVisualSource>) {
        *self.source.borrow_mut() = Some(source);
    }
}

/// A source of a Lottie document, from text, a stream or the URI of an asset.
#[repr(C)]
pub struct LottieVisualSource {
    base: AnimatedVisualSource,
    document: Rc<LottieDocumentSource>,
    base_uri: Option<Url>,
    load_error: RefCell<Option<String>>,
}

ferro_class!(LottieVisualSource: AnimatedVisualSource);
ferro_class_info!(LottieVisualSource {
    new: LottieVisualSource::new,
    markup: {
        namespace: "FerroUI.AnimatedVisuals",
        constructors: [
            (Rc<dyn IServiceProvider>) => LottieVisualSource::with_service_provider,
        ],
        content: UriSource,
    },
});

ferro_properties! {
    impl LottieVisualSource {
        pub fn uri_source_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<Self, _>("UriSource", None)
        }
    }
}

impl FerroObjectImpl for LottieVisualSource {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::uri_source_property().as_property() {
            this.load_uri();
        }
    }
}

impl LottieVisualSource {
    pub fn construct_with(base_uri: Option<Url>) -> Self {
        let document = LottieDocumentSource::new();
        let base = AnimatedVisualSource::construct();
        base.set_inner(document.clone());
        Self { base, document, base_uri, load_error: RefCell::new(None) }
    }

    pub fn construct() -> Self {
        Self::construct_with(None)
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn with_service_provider(services: Rc<dyn IServiceProvider>) -> Ref<Self> {
        instantiate(Self::construct_with(services.get_context_base_uri()))
    }

    /// A source of the document `json` (`CreateFromString`).
    pub fn create_from_string(json: &str) -> Ref<Self> {
        let this = Self::new();
        this.set_source(json);
        this
    }

    pub fn uri_source(&self) -> Option<String> {
        self.get_value(Self::uri_source_property())
    }
    pub fn set_uri_source(&self, value: Option<String>) {
        self.set_value(Self::uri_source_property(), value);
    }

    /// Why the asset of `UriSource` could not be read.
    pub fn load_error(&self) -> Option<String> {
        self.load_error.borrow().clone()
    }

    /// The document behind this object.
    pub fn document(&self) -> &Rc<LottieDocumentSource> {
        &self.document
    }

    pub fn set_source(&self, json: &str) {
        self.document.set_source(json);
    }

    /// Reads the document from `stream` (`SetSourceAsync` of a stream).
    pub fn set_source_stream(&self, stream: impl Read) -> std::io::Result<()> {
        self.document.set_source_stream(stream)
    }

    pub fn set_image_loader(&self, loader: Option<Rc<ImageLoader>>) {
        self.document.set_image_loader(loader);
    }

    pub fn set_color_property(&self, name: &str, color: Color) {
        self.document.set_color_property(name, color);
    }

    fn load_uri(&self) {
        self.load_error.borrow_mut().take();
        let Some(uri) = self.uri_source().filter(|uri| !uri.is_empty()) else {
            return;
        };
        match self.read_asset(&uri) {
            Ok(text) => self.document.set_source(&text),
            Err(error) => {
                *self.load_error.borrow_mut() = Some(error.to_string());
                self.document.set_source("");
            }
        }
    }

    fn read_asset(&self, uri: &str) -> std::io::Result<String> {
        let invalid = |error: String| std::io::Error::new(std::io::ErrorKind::InvalidInput, error);
        let uri = UriRef::parse(uri).map_err(|error| invalid(error.to_string()))?;
        let loader = FerroLocator::current()
            .get_service::<dyn IAssetLoader>()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No asset loader is available"))?;
        let mut stream = loader.open(&uri, self.base_uri.as_ref())?;
        let mut text = String::new();
        stream.read_to_string(&mut text)?;
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    const DOCUMENT: &str = r#"{"fr":30,"ip":0,"op":100,"w":10,"h":10,"layers":[],
        "markers":[{"cm":"Normal","tm":25,"dr":0},{"cm":"Pressed","tm":100,"dr":0}]}"#;

    #[test]
    fn a_source_reads_its_document_once_and_gives_markers_as_progress() {
        let source = LottieDocumentSource::create_from_string(DOCUMENT);
        let first = source.try_create_animated_visual().unwrap();
        let second = source.try_create_animated_visual().unwrap();
        assert!(Rc::ptr_eq(&first, &second));
        assert_eq!(source.markers(), vec![("Normal".to_owned(), 0.25), ("Pressed".to_owned(), 1.0)]);
    }

    #[test]
    fn a_new_document_invalidates_the_visual() {
        let source = LottieDocumentSource::create_from_string(DOCUMENT);
        let first = source.try_create_animated_visual().unwrap();
        let told = Rc::new(Cell::new(0));
        let seen = told.clone();
        source.add_animated_visual_invalidated(Rc::new(move || seen.set(seen.get() + 1)));
        source.set_source(DOCUMENT);
        assert_eq!(told.get(), 1);
        assert!(!Rc::ptr_eq(&first, &source.try_create_animated_visual().unwrap()));
    }

    #[test]
    fn a_color_property_reaches_the_handlers_and_keeps_the_visual() {
        let source = LottieDocumentSource::create_from_string(DOCUMENT);
        let first = source.try_create_animated_visual().unwrap();
        let told = Rc::new(Cell::new(0));
        let seen = told.clone();
        source.add_animated_visual_invalidated(Rc::new(move || seen.set(seen.get() + 1)));
        source.set_color_property("Foreground", Color::from_rgb(1, 2, 3));
        source.set_color_property("Foreground", Color::from_rgb(4, 5, 6));
        assert_eq!(told.get(), 2);
        assert_eq!(source.color_properties(), vec![("Foreground".to_owned(), Color::from_rgb(4, 5, 6))]);
        assert!(Rc::ptr_eq(&first, &source.try_create_animated_visual().unwrap()));
    }

    #[test]
    fn an_empty_source_or_a_bad_document_is_an_error() {
        assert!(LottieDocumentSource::new().try_create_animated_visual().is_err());
        assert!(LottieDocumentSource::create_from_string("not json").try_create_animated_visual().is_err());
    }
}
