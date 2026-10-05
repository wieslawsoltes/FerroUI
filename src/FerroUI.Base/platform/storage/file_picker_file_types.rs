use super::FilePickerFileType;
use std::rc::Rc;

/// Dictionary of well known file types.
pub struct FilePickerFileTypes;

struct WellKnown {
    all: Rc<FilePickerFileType>,
    text_plain: Rc<FilePickerFileType>,
    image_all: Rc<FilePickerFileType>,
    image_jpg: Rc<FilePickerFileType>,
    image_png: Rc<FilePickerFileType>,
    image_webp: Rc<FilePickerFileType>,
    pdf: Rc<FilePickerFileType>,
    json: Rc<FilePickerFileType>,
    xml: Rc<FilePickerFileType>,
}

fn file_type(name: &str, patterns: &[&str], uniform_type_identifiers: &[&str], mime_types: &[&str]) -> Rc<FilePickerFileType> {
    FilePickerFileType::new(Some(name))
        .with_patterns(patterns)
        .with_apple_uniform_type_identifiers(uniform_type_identifiers)
        .with_mime_types(mime_types)
}

thread_local! {
    // One instance of each file type per thread, as the static properties
    // upstream: a file type is compared by identity.
    static WELL_KNOWN: WellKnown = WellKnown {
        all: file_type("All", &["*.*"], &["public.item"], &["*/*"]),
        text_plain: file_type("Plain Text", &["*.txt"], &["public.plain-text"], &["text/plain"]),
        image_all: file_type(
            "All Images",
            &["*.png", "*.jpg", "*.jpeg", "*.gif", "*.bmp", "*.webp"],
            &["public.image"],
            &["image/*"],
        ),
        image_jpg: file_type("JPEG image", &["*.jpg", "*.jpeg"], &["public.jpeg"], &["image/jpeg"]),
        image_png: file_type("PNG image", &["*.png"], &["public.png"], &["image/png"]),
        image_webp: file_type("WebP image", &["*.webp"], &["org.webmproject.webp"], &["image/webp"]),
        pdf: file_type("PDF document", &["*.pdf"], &["com.adobe.pdf"], &["application/pdf"]),
        json: file_type("JSON document", &["*.json"], &["public.json"], &["application/json"]),
        xml: file_type("XML document", &["*.xml"], &["public.xml"], &["application/xml", "text/xml"]),
    };
}

impl FilePickerFileTypes {
    /// Any file.
    pub fn all() -> Rc<FilePickerFileType> {
        WELL_KNOWN.with(|types| types.all.clone())
    }

    /// Plain text files.
    pub fn text_plain() -> Rc<FilePickerFileType> {
        WELL_KNOWN.with(|types| types.text_plain.clone())
    }

    /// Images of any of the common formats.
    pub fn image_all() -> Rc<FilePickerFileType> {
        WELL_KNOWN.with(|types| types.image_all.clone())
    }

    /// JPEG images.
    pub fn image_jpg() -> Rc<FilePickerFileType> {
        WELL_KNOWN.with(|types| types.image_jpg.clone())
    }

    /// PNG images.
    pub fn image_png() -> Rc<FilePickerFileType> {
        WELL_KNOWN.with(|types| types.image_png.clone())
    }

    /// WebP images.
    pub fn image_webp() -> Rc<FilePickerFileType> {
        WELL_KNOWN.with(|types| types.image_webp.clone())
    }

    /// PDF documents.
    pub fn pdf() -> Rc<FilePickerFileType> {
        WELL_KNOWN.with(|types| types.pdf.clone())
    }

    /// JSON documents.
    pub fn json() -> Rc<FilePickerFileType> {
        WELL_KNOWN.with(|types| types.json.clone())
    }

    /// XML documents.
    pub fn xml() -> Rc<FilePickerFileType> {
        WELL_KNOWN.with(|types| types.xml.clone())
    }
}
