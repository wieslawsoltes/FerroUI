//! Native bitmap branch of RibbonIconPresenter.CreateImageSource (9755e281). Copyright Wiesław
//! Šoltés, MIT.
use super::ribbon_image_provider::RibbonUriImage;
use super::ribbon_image_scale::{
    ImageAsset, QualifiedImageSource, ScaleBitmap, qualified_assets, selected_asset,
};
use super::{RibbonDeferredImage, RibbonIconPresenter, RibbonImageKind, RibbonImageProvider};
use ferroribbon_core::{RibbonString, text::trim};
use ferroui_base::{
    FerroLocator, LocatorExtensions, WeakRef,
    data::core::ValueTypes,
    media::{IImage, ImageVariantContext, imaging::Bitmap},
    platform::{AssetLoader, IPlatformSettings, storage::file_io::StorageProviderHelpers},
    utilities::{Uri, UriExtensions, UriKind, UriRef},
};
use ferroui_controls::{Application, Image};
use std::{
    fmt,
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
    rc::Rc,
};

/// Failure from the native bitmap-loading API. Unsupported paths are not blank successes.
#[derive(Debug)]
pub enum RibbonImageLoadError {
    UnpairedUtf16,
    InvalidUri,
    MissingApplicationBaseUri,
    InvalidApplicationBaseUri,
    MissingApplicationDataDirectory(&'static str),
    InvalidApplicationDataPath,
    UnsupportedScheme(String),
    UnsupportedPathForm,
    SvgNotImplemented,
    Load(std::io::Error),
}
impl fmt::Display for RibbonImageLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnpairedUtf16 => f.write_str("native image URI input cannot represent unpaired UTF-16"),
            Self::InvalidUri => f.write_str("invalid native image URI"),
            Self::MissingApplicationBaseUri => f.write_str("set Application.Resources[\"RibbonImageBaseUri\"] to the application's absolute ferres resource root before loading relative or ms-appx images"),
            Self::InvalidApplicationBaseUri => f.write_str("RibbonImageBaseUri must be an absolute ferres URI with a trailing slash"),
            Self::MissingApplicationDataDirectory(key) => write!(f, "set Application.Resources[{key:?}] to an absolute application data directory"),
            Self::InvalidApplicationDataPath => f.write_str("invalid ms-appdata image path or package"),
            Self::UnsupportedScheme(scheme) => write!(f, "image URI scheme {scheme:?} is not implemented"),
            Self::UnsupportedPathForm => f.write_str("the image path cannot be mapped to a file on this platform"),
            Self::SvgNotImplemented => f.write_str("the bitmap-only helper cannot decode SVG"),
            Self::Load(error) => write!(f, "native image loading failed: {error}"),
        }
    }
}
impl std::error::Error for RibbonImageLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Load(error) => Some(error),
            _ => None,
        }
    }
}

impl RibbonIconPresenter {
    /// Application resource key mapping the source's implicit packaged-app root.
    pub const IMAGE_BASE_URI_RESOURCE: &'static str = "RibbonImageBaseUri";
    /// Application resource key containing the current package's identity name as a string.
    ///
    /// A qualified `ms-appdata` URI must name this package. An unqualified URI
    /// uses the configured application data directories directly.
    pub const IMAGE_APPLICATION_DATA_PACKAGE_RESOURCE: &'static str =
        "RibbonImageApplicationDataPackage";
    /// Absolute directory for `ms-appdata:///local/` images.
    pub const IMAGE_LOCAL_DATA_DIRECTORY_RESOURCE: &'static str = "RibbonImageLocalDataDirectory";
    /// Absolute directory for `ms-appdata:///roaming/` images.
    pub const IMAGE_ROAMING_DATA_DIRECTORY_RESOURCE: &'static str =
        "RibbonImageRoamingDataDirectory";
    /// Absolute directory for `ms-appdata:///temp/` images.
    pub const IMAGE_TEMP_DATA_DIRECTORY_RESOURCE: &'static str = "RibbonImageTempDataDirectory";

    /// Load a bitmap through native rendering/asset services. Framework setup is required.
    ///
    /// Fully qualified ferres/resm/file URIs are native inputs. Relative and ms-appx
    /// paths use IMAGE_BASE_URI_RESOURCE. A qualified ms-appx authority names a
    /// registered asset assembly. Application data URIs use the corresponding
    /// directory resource. This API uses the native bitmap decoder directly.
    /// This fallible Rust API is additional to the source's private loading helper.
    pub fn try_load_bitmap_uri(value: &RibbonString) -> Result<Rc<Bitmap>, RibbonImageLoadError> {
        let reference = native_image_uri(value, false)?;
        if is_svg(&reference) {
            return Err(RibbonImageLoadError::SvgNotImplemented);
        }
        let assets = qualified_assets(&reference);
        load_bitmap(
            selected_asset(&assets, &application_image_context())
                .map_or(&reference, |index| &assets[index].uri),
        )
    }

    pub(crate) fn create_display_uri_source(
        value: &RibbonString,
        svg_target: Option<WeakRef<Image>>,
    ) -> Option<Rc<dyn IImage>> {
        match Self::create_uri_image(value, svg_target) {
            Ok(source) => Some(source),
            Err(RibbonImageLoadError::InvalidUri | RibbonImageLoadError::UnpairedUtf16) => None,
            Err(error) => Some(RibbonDeferredImage::failed(error)),
        }
    }

    // The source menu helper always constructs BitmapImage, even for an SVG URI.
    // Keep that distinction from the presenter's SVG dispatch.
    pub(crate) fn create_uri_image(
        value: &RibbonString,
        svg_target: Option<WeakRef<Image>>,
    ) -> Result<Rc<dyn IImage>, RibbonImageLoadError> {
        let reference = native_image_uri(value, svg_target.is_none())?;
        let svg_target = svg_target.filter(|_| is_svg(&reference));
        let provider = FerroLocator::current().get_service::<dyn RibbonImageProvider>();
        let assets = qualified_assets(&reference);
        if !assets.is_empty() {
            return Ok(Rc::new(QualifiedImageSource::new(
                assets,
                application_image_context(),
                move |asset| {
                    load_display_source(&reference, asset, svg_target.clone(), provider.clone())
                        .unwrap_or_else(RibbonDeferredImage::failed)
                },
            )));
        }
        load_display_source(&reference, None, svg_target, provider)
    }
}

fn application_image_context() -> ImageVariantContext {
    ImageVariantContext {
        scaling: 1.,
        theme: Application::current().and_then(|app| app.actual_theme_variant()),
        platform_colors: FerroLocator::current()
            .get_service::<dyn IPlatformSettings>()
            .map(|settings| settings.get_color_values())
            .unwrap_or_default(),
    }
}

fn load_display_source(
    reference: &UriRef,
    asset: Option<&ImageAsset>,
    svg_target: Option<WeakRef<Image>>,
    provider: Option<Rc<dyn RibbonImageProvider>>,
) -> Result<Rc<dyn IImage>, RibbonImageLoadError> {
    let physical = asset.map_or(reference, |asset| &asset.uri).clone();
    let percent = asset.and_then(ImageAsset::percent);
    let kind = if svg_target.is_some() {
        RibbonImageKind::Svg
    } else {
        RibbonImageKind::Bitmap
    };
    let remote = cfg!(any(not(target_arch = "wasm32"), target_os = "emscripten"))
        && reference
            .as_url()
            .is_some_and(|url| matches!(url.scheme(), "http" | "https"));
    if remote || svg_target.is_some() || provider.is_some() {
        let logical = reference.clone();
        let request = async move {
            if let Some(open) = provider.and_then(|provider| provider.open(&logical, kind)) {
                return open.await;
            }
            #[cfg(any(not(target_arch = "wasm32"), target_os = "emscripten"))]
            if remote {
                let bytes = super::ribbon_http_images::read_http_image(&physical)
                    .await
                    .map_err(RibbonImageLoadError::Load)?;
                if let Some(target) = svg_target {
                    return super::ribbon_svg_images::decode_svg(&bytes, target)
                        .map_err(RibbonImageLoadError::Load);
                }
                let bitmap = Bitmap::from_stream(&mut std::io::Cursor::new(bytes))
                    .map_err(RibbonImageLoadError::Load)?;
                return Ok(Rc::new(bitmap) as Rc<dyn IImage>);
            }
            if let Some(target) = svg_target {
                let bytes = read_svg(&physical)?;
                return super::ribbon_svg_images::decode_svg(&bytes, target)
                    .map_err(RibbonImageLoadError::Load);
            }
            load_display_bitmap(&physical, percent)
        };
        return Ok(RibbonDeferredImage::start(Box::pin(request)));
    }
    Ok(RibbonUriImage::decoded(load_display_bitmap(
        &physical, percent,
    )?))
}

fn load_display_bitmap(
    reference: &UriRef,
    percent: Option<u16>,
) -> Result<Rc<dyn IImage>, RibbonImageLoadError> {
    let bitmap = load_bitmap(reference)?;
    Ok(match percent {
        Some(percent) => Rc::new(ScaleBitmap { bitmap, percent }),
        None => bitmap,
    })
}

fn native_image_uri(
    value: &RibbonString,
    bitmap_only: bool,
) -> Result<UriRef, RibbonImageLoadError> {
    let units = value.as_utf16();
    let text = String::from_utf16(if bitmap_only { units } else { trim(units) })
        .map_err(|_| RibbonImageLoadError::UnpairedUtf16)?;
    let uri = resolve(&text)?;
    if uri.scheme() == "ms-appdata" {
        // Inspect the source authority before the URL parser drops empty
        // user info or port delimiters.
        let authority = uri
            .original_string()
            .split_once(':')
            .and_then(|(_, rest)| rest.strip_prefix("//"))
            .and_then(|rest| rest.split(['/', '?', '#']).next())
            .unwrap_or_default();
        if !authority.is_empty() && !is_package_name(authority) {
            return Err(RibbonImageLoadError::InvalidApplicationDataPath);
        }
    }
    let text = uri.absolute_uri();
    // URL parsing strips some literal ASCII control characters. Escape them
    // before parsing so packaged filenames retain their exact source spelling.
    let escaped;
    let text = if text.bytes().any(|byte| byte.is_ascii_control()) {
        use std::fmt::Write;
        let mut encoded = String::with_capacity(text.len());
        for character in text.chars() {
            if character.is_ascii_control() {
                write!(encoded, "%{:02X}", character as u8).expect("write to String");
            } else {
                encoded.push(character);
            }
        }
        escaped = encoded;
        &escaped
    } else {
        text
    };
    UriRef::parse_absolute(text)
        .map(UriRef::from)
        .map_err(|_| RibbonImageLoadError::InvalidUri)
}

fn is_svg(reference: &UriRef) -> bool {
    reference.as_url().is_some_and(|url| {
        UriExtensions::get_unescape_absolute_path(url)
            .to_ascii_lowercase()
            .ends_with(".svg")
    })
}

fn load_bitmap(reference: &UriRef) -> Result<Rc<Bitmap>, RibbonImageLoadError> {
    let url = reference.as_url().ok_or(RibbonImageLoadError::InvalidUri)?;
    let bitmap = match url.scheme() {
        "ferres" | "resm" => {
            let mut stream =
                AssetLoader::open(reference, None).map_err(RibbonImageLoadError::Load)?;
            Bitmap::from_stream(&mut *stream)
        }
        "file" | "ms-appdata" => {
            let file = image_file(reference)?;
            Bitmap::from_file(file.to_str().ok_or(RibbonImageLoadError::InvalidUri)?)
        }
        scheme => return Err(RibbonImageLoadError::UnsupportedScheme(scheme.to_owned())),
    }
    .map_err(RibbonImageLoadError::Load)?;
    Ok(Rc::new(bitmap))
}

fn image_file(reference: &UriRef) -> Result<PathBuf, RibbonImageLoadError> {
    let url = reference.as_url().ok_or(RibbonImageLoadError::InvalidUri)?;
    if url.scheme() == "ms-appdata" {
        let path = UriExtensions::get_unescape_absolute_path(url);
        application_data_file(UriExtensions::authority(url), &path)
    } else {
        url.to_file_path()
            .map_err(|_| RibbonImageLoadError::UnsupportedPathForm)
    }
}

fn read_svg(reference: &UriRef) -> Result<Vec<u8>, RibbonImageLoadError> {
    let url = reference.as_url().ok_or(RibbonImageLoadError::InvalidUri)?;
    let mut bytes = Vec::new();
    match url.scheme() {
        "ferres" | "resm" => {
            let mut stream =
                AssetLoader::open(reference, None).map_err(RibbonImageLoadError::Load)?;
            stream.read_to_end(&mut bytes)
        }
        "file" | "ms-appdata" => {
            File::open(image_file(reference)?).and_then(|mut stream| stream.read_to_end(&mut bytes))
        }
        scheme => return Err(RibbonImageLoadError::UnsupportedScheme(scheme.to_owned())),
    }
    .map_err(RibbonImageLoadError::Load)?;
    Ok(bytes)
}

fn application_resource(key: &str) -> Option<ferroui_base::BoxedValue> {
    Application::current()
        .and_then(|app| {
            app.resources()
                .try_get_resource(&key.into(), None)
                .flatten()
        })
        .and_then(ValueTypes::normalize)
}

fn application_data_file(authority: &str, path: &str) -> Result<PathBuf, RibbonImageLoadError> {
    use RibbonIconPresenter as Presenter;
    if !authority.is_empty() {
        let package = application_resource(Presenter::IMAGE_APPLICATION_DATA_PACKAGE_RESOURCE);
        let package = package
            .as_ref()
            .and_then(|value| value.downcast_ref::<String>());
        if !is_package_name(authority)
            || !package.is_some_and(|name| name.eq_ignore_ascii_case(authority))
        {
            return Err(RibbonImageLoadError::InvalidApplicationDataPath);
        }
    }
    let (folder, name) = path
        .strip_prefix('/')
        .and_then(|path| path.split_once('/'))
        .ok_or(RibbonImageLoadError::InvalidApplicationDataPath)?;
    let key = match folder {
        "local" => Presenter::IMAGE_LOCAL_DATA_DIRECTORY_RESOURCE,
        "roaming" => Presenter::IMAGE_ROAMING_DATA_DIRECTORY_RESOURCE,
        "temp" => Presenter::IMAGE_TEMP_DATA_DIRECTORY_RESOURCE,
        _ => return Err(RibbonImageLoadError::InvalidApplicationDataPath),
    };
    let relative = Path::new(name);
    // A decoded separator must not turn the name into a drive, UNC or parent path.
    if name.is_empty()
        || name.contains(['\\', '\0'])
        || (cfg!(windows) && name.contains(':'))
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(RibbonImageLoadError::InvalidApplicationDataPath);
    }
    let value = application_resource(key)
        .ok_or(RibbonImageLoadError::MissingApplicationDataDirectory(key))?;
    let root = value
        .downcast_ref::<PathBuf>()
        .map(PathBuf::as_path)
        .or_else(|| value.downcast_ref::<String>().map(Path::new))
        .filter(|path| path.is_absolute())
        .ok_or(RibbonImageLoadError::MissingApplicationDataDirectory(key))?;
    let root = root.canonicalize().map_err(RibbonImageLoadError::Load)?;
    let file = root
        .join(relative)
        .canonicalize()
        .map_err(RibbonImageLoadError::Load)?;
    // Canonicalization accounts for directory links inside the application root.
    if !file.starts_with(&root) {
        return Err(RibbonImageLoadError::InvalidApplicationDataPath);
    }
    Ok(file)
}

fn is_package_name(name: &str) -> bool {
    (3..=50).contains(&name.len())
        && !name.ends_with('.')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
}

fn application_base() -> Result<Uri, RibbonImageLoadError> {
    let value = application_resource(RibbonIconPresenter::IMAGE_BASE_URI_RESOURCE)
        .ok_or(RibbonImageLoadError::MissingApplicationBaseUri)?;
    let base = value
        .downcast_ref::<Uri>()
        .cloned()
        .or_else(|| {
            value
                .downcast_ref::<String>()
                .and_then(|s| Uri::try_create(s, UriKind::Absolute))
        })
        .ok_or(RibbonImageLoadError::InvalidApplicationBaseUri)?;
    if !base.is_absolute_uri()
        || base.scheme() != "ferres"
        || !base.absolute_path().ends_with('/')
        || !has_authority(&base)?
        || !base.query().is_empty()
        || base.absolute_uri().contains('#')
    {
        return Err(RibbonImageLoadError::InvalidApplicationBaseUri);
    }
    Ok(base)
}

fn resolve(value: &str) -> Result<Uri, RibbonImageLoadError> {
    // The source URI constructor removes these four ASCII whitespace characters.
    // Probe absolute URIs with both ends trimmed; packaged relative paths retain
    // leading characters because the source adds its ms-appx prefix first.
    // VT, FF and non-ASCII whitespace are filename characters here (source corpus).
    let uri_whitespace = |c| matches!(c, ' ' | '\t' | '\r' | '\n');
    let absolute_input = value.trim_matches(uri_whitespace);
    let drive = matches!(absolute_input.as_bytes(), [letter, b':', b'/' | b'\\', ..] if letter.is_ascii_alphabetic());
    let unc = absolute_input.starts_with("//") || absolute_input.starts_with("\\\\");
    if drive || unc || (!cfg!(windows) && absolute_input.starts_with('/')) {
        return StorageProviderHelpers::try_get_uri_from_file_path(absolute_input, false)
            .map(Uri::from)
            .ok_or(RibbonImageLoadError::UnsupportedPathForm);
    }
    let absolute = Uri::try_create(absolute_input, UriKind::Absolute);
    let value = value.trim_end_matches(uri_whitespace);
    let uri = match absolute {
        Some(uri) => uri,
        None => Uri::new(value, UriKind::Relative).map_err(|_| RibbonImageLoadError::InvalidUri)?,
    };
    if !uri.is_absolute_uri()
        && matches!(value.as_bytes(), [letter, b':', ..] if letter.is_ascii_alphabetic())
    {
        return Err(RibbonImageLoadError::UnsupportedPathForm);
    }
    if uri.is_absolute_uri() && uri.scheme() != "ms-appx" {
        return Ok(uri);
    }
    if uri.is_absolute_uri() && has_authority(&uri)? {
        let package_path = &uri.absolute_uri()["ms-appx:".len()..];
        return Uri::absolute(&format!("ferres:{package_path}"))
            .map_err(|_| RibbonImageLoadError::InvalidUri);
    }
    let path = if uri.is_absolute_uri() {
        // Keep the escaped path/query/fragment; the native loader performs its own unescape.
        &uri.absolute_uri()["ms-appx:".len()..]
    } else {
        value
    };
    let relative = Uri::new(
        &path.trim_start_matches(['/', '\\']).replace('\\', "/"),
        UriKind::Relative,
    )
    .map_err(|_| RibbonImageLoadError::InvalidUri)?;
    Ok(Uri::combine(&application_base()?, &relative))
}

fn has_authority(uri: &Uri) -> Result<bool, RibbonImageLoadError> {
    let url =
        UriRef::parse_absolute(uri.absolute_uri()).map_err(|_| RibbonImageLoadError::InvalidUri)?;
    Ok(!UriExtensions::authority(&url).is_empty())
}
