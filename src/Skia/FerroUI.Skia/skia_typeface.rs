use crate::skia_sharp_extensions::slant_to_font_style;
use ferroui_base::media::fonts::OpenTypeTag;
use ferroui_base::media::{FontSimulations, FontStretch, FontStyle, FontWeight, IFontMemory, IPlatformTypeface};
use ferroui_base::utilities::ReadOnlyMemory;
use skia_safe::{Font, Typeface};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::rc::{Rc, Weak};

thread_local! {
    /// The live typefaces of this backend by address.
    ///
    /// The platform typeface contract has no hook to recover the backend's
    /// type from a `dyn IPlatformTypeface`, so the typefaces the backend
    /// creates are tracked here and looked up by the address of the object
    /// behind the handle. Entries are removed when the typeface is dropped.
    static REGISTRY: RefCell<HashMap<usize, Weak<SkiaTypeface>>> = RefCell::new(HashMap::new());
}

/// A platform typeface over a Skia typeface.
pub struct SkiaTypeface {
    sk_typeface: Typeface,
    font_simulations: FontSimulations,
    weight: FontWeight,
    style: FontStyle,
    stretch: FontStretch,
}

impl SkiaTypeface {
    /// Wraps a Skia typeface, applying the given style simulations when
    /// fonts are created from it.
    pub fn new(typeface: Typeface, font_simulations: FontSimulations) -> Rc<Self> {
        let font_style = typeface.font_style();

        let typeface = Rc::new(Self {
            weight: FontWeight(*font_style.weight()),
            style: slant_to_font_style(font_style.slant()),
            stretch: FontStretch::from_i32(*font_style.width()).unwrap_or(FontStretch::Normal),
            sk_typeface: typeface,
            font_simulations,
        });

        let key = Rc::as_ptr(&typeface) as *const () as usize;
        REGISTRY.with(|registry| registry.borrow_mut().insert(key, Rc::downgrade(&typeface)));

        typeface
    }

    /// The Skia typeface.
    pub fn sk_typeface(&self) -> &Typeface {
        &self.sk_typeface
    }

    /// Creates a Skia font of the given size with the simulations of the
    /// typeface applied.
    pub fn create_sk_font(&self, size: f32) -> Font {
        let skew_x = if self.font_simulations.contains(FontSimulations::Oblique) { -0.3 } else { 0.0 };

        let mut font = Font::from_typeface_with_params(self.sk_typeface.clone(), size, 1.0, skew_x);
        font.set_linear_metrics(true);
        font.set_embolden(self.font_simulations.contains(FontSimulations::Bold));
        font
    }

    /// Recovers the backend typeface behind a platform typeface.
    ///
    /// Returns `None` for typefaces created by another backend.
    pub fn try_get(platform_typeface: &dyn IPlatformTypeface) -> Option<Rc<SkiaTypeface>> {
        let key = platform_typeface as *const dyn IPlatformTypeface as *const () as usize;
        REGISTRY.with(|registry| registry.borrow().get(&key).and_then(Weak::upgrade))
    }
}

impl Drop for SkiaTypeface {
    fn drop(&mut self) {
        let key = self as *const Self as usize;
        // The registry may already be gone during thread teardown.
        let _ = REGISTRY.try_with(|registry| {
            registry.borrow_mut().remove(&key);
        });
    }
}

impl IFontMemory for SkiaTypeface {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        let data = self.sk_typeface.copy_table_data(tag.value())?;

        Some(ReadOnlyMemory::from_slice(data.as_bytes()))
    }

    fn dispose(&self) {
        // The Skia typeface is reference counted and released with the last
        // handle to this object.
    }
}

impl IPlatformTypeface for SkiaTypeface {
    fn family_name(&self) -> String {
        self.sk_typeface.family_name()
    }

    fn weight(&self) -> FontWeight {
        self.weight
    }

    fn style(&self) -> FontStyle {
        self.style
    }

    fn stretch(&self) -> FontStretch {
        self.stretch
    }

    fn font_simulations(&self) -> FontSimulations {
        self.font_simulations
    }

    fn try_get_stream(&self) -> Option<Box<dyn Read>> {
        let (bytes, _) = self.sk_typeface.to_font_bytes()?;

        Some(Box::new(Cursor::new(bytes)))
    }
}
