use ferroui_base::input::StandardCursorType;
use ferroui_base::media::imaging::{Bitmap, BitmapEncoderOptions, PngBitmapEncoderOptions};
use ferroui_base::platform::{ICursorFactory, ICursorImpl};
use ferroui_base::PixelPoint;
use std::any::Any;
use std::rc::Rc;

/// A cursor as the value of the CSS `cursor` property.
pub struct CssCursor {
    value: Option<String>,
}

impl CssCursor {
    /// The value that stands for the cursor of the page.
    pub const DEFAULT: &'static str = "default";

    /// The CSS keyword of a standard cursor.
    pub fn new(type_: StandardCursorType) -> Self {
        Self { value: Some(Self::to_keyword(type_).to_string()) }
    }

    /// Create a cursor from base64 image
    pub fn from_base64(base64: &str, format: &str, hotspot: PixelPoint, fallback: StandardCursorType) -> Self {
        Self {
            value: Some(format!(
                "url(\"data:image/{format};base64,{base64}\") {} {}, {}",
                hotspot.x,
                hotspot.y,
                Self::to_keyword(fallback)
            )),
        }
    }

    /// Create a cursor from url to *.cur file.
    pub fn from_url(url: &str, fallback: StandardCursorType) -> Self {
        Self { value: Some(format!("url('{url}'), {}", Self::to_keyword(fallback))) }
    }

    /// Create a cursor from png/svg and hotspot position
    pub fn from_url_with_hot_spot(url: &str, hot_spot: PixelPoint, fallback: StandardCursorType) -> Self {
        Self { value: Some(format!("url('{url}') {} {}, {}", hot_spot.x, hot_spot.y, Self::to_keyword(fallback))) }
    }

    /// The value of the CSS property.
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    /// Sets the value of the CSS property.
    pub fn set_value(&mut self, value: Option<String>) {
        self.value = value;
    }

    fn to_keyword(type_: StandardCursorType) -> &'static str {
        match type_ {
            StandardCursorType::Hand => "pointer",
            StandardCursorType::Cross => "crosshair",
            StandardCursorType::Help => "help",
            StandardCursorType::Ibeam => "text",
            StandardCursorType::No => "not-allowed",
            StandardCursorType::None => "none",
            StandardCursorType::Wait => "progress",
            StandardCursorType::AppStarting => "wait",

            StandardCursorType::DragMove => "move",
            StandardCursorType::DragCopy => "copy",
            StandardCursorType::DragLink => "alias",

            StandardCursorType::UpArrow => "default", /*not found matching one*/
            StandardCursorType::SizeWestEast => "ew-resize",
            StandardCursorType::SizeNorthSouth => "ns-resize",
            StandardCursorType::SizeAll => "move",

            StandardCursorType::TopSide => "n-resize",
            StandardCursorType::BottomSide => "s-resize",
            StandardCursorType::LeftSide => "w-resize",
            StandardCursorType::RightSide => "e-resize",
            StandardCursorType::TopLeftCorner => "nw-resize",
            StandardCursorType::TopRightCorner => "ne-resize",
            StandardCursorType::BottomLeftCorner => "sw-resize",
            StandardCursorType::BottomRightCorner => "se-resize",

            _ => Self::DEFAULT,
        }
    }
}

impl ICursorImpl for CssCursor {
    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Creates CSS cursors.
#[derive(Default)]
pub struct CssCursorFactory;

/// Base64 with the standard alphabet and padding.
fn to_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], chunk.get(1).copied().unwrap_or(0), chunk.get(2).copied().unwrap_or(0)];
        text.push(ALPHABET[(b[0] >> 2) as usize] as char);
        text.push(ALPHABET[(((b[0] & 0x03) << 4) | (b[1] >> 4)) as usize] as char);
        text.push(if chunk.len() > 1 { ALPHABET[(((b[1] & 0x0F) << 2) | (b[2] >> 6)) as usize] as char } else { '=' });
        text.push(if chunk.len() > 2 { ALPHABET[(b[2] & 0x3F) as usize] as char } else { '=' });
    }
    text
}

impl ICursorFactory for CssCursorFactory {
    fn get_cursor(&self, cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        Rc::new(CssCursor::new(cursor_type))
    }

    /// # Panics
    /// Panics when the bitmap cannot be encoded.
    fn create_cursor(&self, cursor: &Bitmap, hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        let mut image_stream = Vec::new();
        if let Err(error) = cursor.save(&mut image_stream, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT)) {
            panic!("Unable to save the cursor bitmap: {error}");
        }

        let base64_string = to_base64(&image_stream);
        Rc::new(CssCursor::from_base64(&base64_string, "png", hot_spot, StandardCursorType::Arrow))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_cursors_map_to_css_keywords() {
        let expected = [
            (StandardCursorType::Arrow, "default"),
            (StandardCursorType::Ibeam, "text"),
            (StandardCursorType::Wait, "progress"),
            (StandardCursorType::Cross, "crosshair"),
            (StandardCursorType::UpArrow, "default"),
            (StandardCursorType::SizeWestEast, "ew-resize"),
            (StandardCursorType::SizeNorthSouth, "ns-resize"),
            (StandardCursorType::SizeAll, "move"),
            (StandardCursorType::No, "not-allowed"),
            (StandardCursorType::Hand, "pointer"),
            (StandardCursorType::AppStarting, "wait"),
            (StandardCursorType::Help, "help"),
            (StandardCursorType::TopSide, "n-resize"),
            (StandardCursorType::BottomSide, "s-resize"),
            (StandardCursorType::LeftSide, "w-resize"),
            (StandardCursorType::RightSide, "e-resize"),
            (StandardCursorType::TopLeftCorner, "nw-resize"),
            (StandardCursorType::TopRightCorner, "ne-resize"),
            (StandardCursorType::BottomLeftCorner, "sw-resize"),
            (StandardCursorType::BottomRightCorner, "se-resize"),
            (StandardCursorType::DragMove, "move"),
            (StandardCursorType::DragCopy, "copy"),
            (StandardCursorType::DragLink, "alias"),
            (StandardCursorType::None, "none"),
        ];

        for (type_, keyword) in expected {
            assert_eq!(Some(keyword), CssCursor::new(type_).value(), "{type_:?}");
        }
    }

    #[test]
    fn the_factory_creates_keyword_cursors() {
        let cursor = CssCursorFactory.get_cursor(StandardCursorType::Hand);

        assert_eq!(Some("pointer"), cursor.as_any().downcast_ref::<CssCursor>().unwrap().value());
    }

    #[test]
    fn an_embedded_image_carries_its_hot_spot_and_a_fallback() {
        let cursor = CssCursor::from_base64("QUJD", "png", PixelPoint::new(3, 7), StandardCursorType::Arrow);

        assert_eq!(Some("url(\"data:image/png;base64,QUJD\") 3 7, default"), cursor.value());
    }

    #[test]
    fn a_cursor_file_is_referenced_by_its_url() {
        let cursor = CssCursor::from_url("cursors/busy.cur", StandardCursorType::Wait);

        assert_eq!(Some("url('cursors/busy.cur'), progress"), cursor.value());
    }

    #[test]
    fn an_image_url_carries_its_hot_spot() {
        let cursor = CssCursor::from_url_with_hot_spot("pen.svg", PixelPoint::new(1, 15), StandardCursorType::Cross);

        assert_eq!(Some("url('pen.svg') 1 15, crosshair"), cursor.value());
    }

    #[test]
    fn base64_pads_incomplete_groups() {
        assert_eq!("", to_base64(b""));
        assert_eq!("Zg==", to_base64(b"f"));
        assert_eq!("Zm8=", to_base64(b"fo"));
        assert_eq!("Zm9v", to_base64(b"foo"));
        assert_eq!("Zm9vYg==", to_base64(b"foob"));
        assert_eq!("Zm9vYmE=", to_base64(b"fooba"));
        assert_eq!("Zm9vYmFy", to_base64(b"foobar"));
        assert_eq!("+/8=", to_base64(&[0xFB, 0xFF]));
    }
}
