//! The platform typeface of the backend: a face of the engine at one style.

use crate::font::{FaceStyle, FontFace};
use ferroui_base::media::fonts::OpenTypeTag;
use ferroui_base::media::{FontSimulations, FontStretch, FontStyle, FontWeight, IFontMemory, IPlatformTypeface};
use ferroui_base::utilities::ReadOnlyMemory;
use std::any::Any;
use std::io::{Cursor, Read};
use std::rc::Rc;
use std::sync::Arc;

/// A face of the engine at one style: the variation instance of a variable face, and the style
/// simulations the face cannot give by itself.
pub struct ProGpuTypeface {
    face: Arc<FontFace>,
    coordinates: Arc<[i16]>,
    family_name: String,
    weight: FontWeight,
    style: FontStyle,
    stretch: FontStretch,
    font_simulations: FontSimulations,
}

impl ProGpuTypeface {
    /// The typeface of `face` asked for at `requested` (or as the face is designed, for `None`),
    /// with `font_simulations` on top.
    pub(crate) fn new(face: Arc<FontFace>, family_name: Option<String>, requested: Option<FaceStyle>, font_simulations: FontSimulations) -> Rc<Self> {
        let designed = FaceStyle { weight: i32::from(face.weight()), width: i32::from(face.width()), italic: face.italic() };
        let (coordinates, applied) = match requested {
            Some(style) => face.resolve_style(style),
            None => Default::default(),
        };
        let requested = requested.unwrap_or(designed);
        let weight = if applied.weight() { requested.weight } else { designed.weight };
        let italic = designed.italic || (applied.slant() && requested.italic);
        let family_name = family_name.or_else(|| face.name(16)).or_else(|| face.name(1)).unwrap_or_default();
        Rc::new(ProGpuTypeface {
            face,
            coordinates: coordinates.into(),
            family_name,
            weight: FontWeight(weight),
            style: if italic { FontStyle::Italic } else { FontStyle::Normal },
            stretch: FontStretch::from_i32(designed.width).unwrap_or(FontStretch::Normal),
            font_simulations,
        })
    }

    /// The typeface behind a platform typeface, when it is one of this backend.
    pub fn try_get(typeface: &dyn IPlatformTypeface) -> Option<&ProGpuTypeface> {
        typeface.as_any().downcast_ref::<ProGpuTypeface>()
    }

    /// The face of the engine.
    pub(crate) fn face(&self) -> &Arc<FontFace> {
        &self.face
    }

    /// The normalized variation coordinates of the instance (empty for a static face).
    pub(crate) fn coordinates(&self) -> &Arc<[i16]> {
        &self.coordinates
    }
}

impl IFontMemory for ProGpuTypeface {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        // The table is copied: the readers of the base library own what they keep.
        self.face.table(tag.value()).map(ReadOnlyMemory::from_slice)
    }

    fn dispose(&self) {
        // The face is shared and released with its last holder.
    }
}

impl IPlatformTypeface for ProGpuTypeface {
    fn family_name(&self) -> String {
        self.family_name.clone()
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
        let (bytes, _) = self.face.data();
        (!bytes.is_empty()).then(|| Box::new(Cursor::new(bytes.to_vec())) as Box<dyn Read>)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
