//! `{SvgImage}`: an SVG asset as the source of an image in markup.

use crate::{SvgDocument, SvgLimits};
use ferroui_base::media::IImage;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::UriRef;
use ferroui_base::BoxedValue;
use ferroui_markup_xaml::{IUriContext, MarkupExtension, XamlLoadError};
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;

/// The scheme of the assets of an application: the only place an image is read from.
const ASSET_SCHEME: &str = "ferres";

/// Provides the image of an SVG asset: `<Image Source="{SvgImage /Assets/logo.svg}" />`.
///
/// The source is an asset of the application, absolute or relative to the document. Any other
/// scheme is an error, so a document never makes the application fetch anything.
pub struct SvgImageExtension {
    source: RefCell<Option<UriRef>>,
}

impl SvgImageExtension {
    /// Creates the extension without a source.
    pub fn new() -> Rc<Self> {
        Rc::new(Self { source: RefCell::new(None) })
    }

    /// Creates the extension with a source.
    pub fn with_source(source: UriRef) -> Rc<Self> {
        let extension = Self::new();
        extension.set_source(Some(source));
        extension
    }

    /// The URI of the asset.
    pub fn source(&self) -> Option<UriRef> {
        self.source.borrow().clone()
    }

    /// Sets the URI of the asset.
    pub fn set_source(&self, source: Option<UriRef>) {
        self.source.replace(source);
    }

    /// The image of the asset; `None` without a source.
    pub fn provide_value(&self, service_provider: &Rc<dyn IServiceProvider>) -> Result<Option<BoxedValue>, XamlLoadError> {
        let Some(source) = self.source() else {
            return Ok(None);
        };
        let context = service_provider.get_service_of::<Rc<dyn IUriContext>>();
        let base_uri = context.and_then(|context| context.base_uri());
        let image = load_image(&source, base_uri.as_ref())?;
        Ok(Some(Rc::new(image)))
    }
}

/// Reads and decodes an SVG asset of the application.
fn load_image(source: &UriRef, base_uri: Option<&url::Url>) -> Result<Rc<dyn IImage>, XamlLoadError> {
    let scheme = match source {
        UriRef::Absolute(url) => Some(url.scheme()),
        UriRef::Relative(_) => base_uri.map(url::Url::scheme),
    };
    if scheme != Some(ASSET_SCHEME) {
        return Err(XamlLoadError::with_message(format!("An SVG image is read from an asset of the application, not '{source:?}'.")));
    }
    let failed = |error: std::io::Error| XamlLoadError::with_message(format!("The SVG image {source:?} was not read: {error}"));
    let mut stream = AssetLoader::open(source, base_uri).map_err(failed)?;
    let limits = SvgLimits::default();
    let mut bytes = Vec::new();
    // One byte over the limit is enough for the decoder to refuse the document.
    let most = u64::try_from(limits.max_bytes).unwrap_or(u64::MAX).saturating_add(1);
    (&mut *stream).take(most).read_to_end(&mut bytes).map_err(failed)?;
    let document = SvgDocument::parse_with_limits(&bytes, &limits)
        .map_err(|error| XamlLoadError::with_message(format!("The SVG image {source:?} was not decoded: {error}")))?;
    Ok(document.to_image().into())
}

impl PartialEq for SvgImageExtension {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl MarkupExtension for SvgImageExtension {
    fn provide_value(&self, service_provider: &Rc<dyn IServiceProvider>) -> Result<Option<BoxedValue>, XamlLoadError> {
        Self::provide_value(self, service_provider)
    }
}

ferroui_base::ferro_markup_type!(class SvgImageExtension {
    namespace: "FerroUI.Svg",
    this: Rc<SvgImageExtension>,
    handles: [SvgImageExtension, Rc<SvgImageExtension>, Option<Rc<SvgImageExtension>>],
    constructors: [() => SvgImageExtension::new, (UriRef) => SvgImageExtension::with_source],
    properties: [Source: Option<UriRef> { get: SvgImageExtension::source, set: SvgImageExtension::set_source } [ConstructorArgument("source")]],
    methods: [try fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
        |this: &Rc<SvgImageExtension>, provider: Rc<dyn IServiceProvider>| { this.provide_value(&provider) }],
});
