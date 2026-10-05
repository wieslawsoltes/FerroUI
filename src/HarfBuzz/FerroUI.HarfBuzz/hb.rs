//! A thin safe wrapper over the HarfBuzz objects the shaper uses.
//!
//! Every `unsafe` block of the crate lives here. The wrappers own one
//! reference to their HarfBuzz object and release it when dropped.

use ferroui_base::utilities::ReadOnlyMemory;
use harfbuzz_sys as sys;
use std::ffi::{c_char, c_void};
use std::os::raw::c_uint;

pub use sys::{hb_feature_t as Feature, hb_glyph_info_t as GlyphInfo, hb_glyph_position_t as GlyphPosition};

/// The direction of a run of text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
}

/// Provides the data of a font table, or `None` when the font has no such
/// table.
pub type TableProvider = Box<dyn Fn(u32) -> Option<ReadOnlyMemory<u8>>>;

/// A HarfBuzz face whose tables come from a [`TableProvider`].
pub struct Face {
    raw: *mut sys::hb_face_t,
}

unsafe extern "C" fn reference_table(
    _face: *mut sys::hb_face_t,
    tag: sys::hb_tag_t,
    user_data: *mut c_void,
) -> *mut sys::hb_blob_t {
    // SAFETY: `user_data` is the `TableProvider` leaked by `Face::new`; it
    // stays alive until HarfBuzz calls `destroy_table_provider`.
    let provider = unsafe { &*(user_data as *const TableProvider) };

    let Some(table) = provider(tag) else {
        return std::ptr::null_mut();
    };

    // The blob keeps the table memory alive: the handle moves into the
    // blob's user data and is dropped by `destroy_table`.
    let table = Box::new(table);
    let (data, length) = {
        let span = table.span();
        (span.as_ptr(), span.len())
    };

    // SAFETY: `data` points to `length` bytes owned by the boxed handle,
    // which is immutable and outlives the blob.
    unsafe {
        sys::hb_blob_create(
            data as *const c_char,
            length as c_uint,
            sys::HB_MEMORY_MODE_READONLY,
            Box::into_raw(table) as *mut c_void,
            Some(destroy_table),
        )
    }
}

unsafe extern "C" fn destroy_table(user_data: *mut c_void) {
    // SAFETY: `user_data` is the box leaked in `reference_table`; HarfBuzz
    // calls the destroy function exactly once.
    drop(unsafe { Box::from_raw(user_data as *mut ReadOnlyMemory<u8>) });
}

unsafe extern "C" fn destroy_table_provider(user_data: *mut c_void) {
    // SAFETY: `user_data` is the box leaked in `Face::new`; HarfBuzz calls
    // the destroy function exactly once, when the face is released.
    drop(unsafe { Box::from_raw(user_data as *mut TableProvider) });
}

impl Face {
    /// Creates a face that reads its tables through `provider`.
    pub fn new(provider: TableProvider, units_per_em: u32) -> Self {
        let user_data = Box::into_raw(Box::new(provider)) as *mut c_void;

        // SAFETY: the callbacks match the signatures HarfBuzz expects and
        // `user_data` stays valid until `destroy_table_provider` runs.
        let raw = unsafe {
            let raw = sys::hb_face_create_for_tables(Some(reference_table), user_data, Some(destroy_table_provider));
            sys::hb_face_set_upem(raw, units_per_em);
            raw
        };

        Self { raw }
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        // SAFETY: releases the reference taken at creation.
        unsafe { sys::hb_face_destroy(self.raw) };
    }
}

/// A HarfBuzz font using the OpenType font functions.
pub struct Font {
    raw: *mut sys::hb_font_t,
}

impl Font {
    /// Creates the font of a face, with the OpenType font functions set.
    pub fn new(face: &Face) -> Self {
        // SAFETY: `face.raw` is a valid face; the font takes its own
        // reference to it.
        let raw = unsafe {
            let raw = sys::hb_font_create(face.raw);
            sys::hb_ot_font_set_funcs(raw);
            raw
        };

        Self { raw }
    }

    /// The horizontal and vertical scale of the font.
    pub fn scale(&self) -> (i32, i32) {
        let (mut x, mut y) = (0, 0);
        // SAFETY: valid font and out-pointers.
        unsafe { sys::hb_font_get_scale(self.raw, &mut x, &mut y) };
        (x, y)
    }

    /// Shapes the contents of `buffer` with this font.
    pub fn shape(&self, buffer: &mut Buffer, features: &[Feature]) {
        // SAFETY: valid font and buffer; `features` is a slice of
        // `features.len()` initialized features (or null when empty).
        unsafe {
            sys::hb_shape(
                self.raw,
                buffer.raw,
                if features.is_empty() { std::ptr::null() } else { features.as_ptr() },
                features.len() as c_uint,
            )
        };
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        // SAFETY: releases the reference taken at creation.
        unsafe { sys::hb_font_destroy(self.raw) };
    }
}

/// A HarfBuzz buffer: the text going into the shaper and the glyphs coming
/// out of it.
pub struct Buffer {
    raw: *mut sys::hb_buffer_t,
}

impl Buffer {
    pub fn new() -> Self {
        // SAFETY: creating a buffer has no preconditions.
        Self { raw: unsafe { sys::hb_buffer_create() } }
    }

    /// Clears the contents and the properties of the buffer.
    pub fn reset(&mut self) {
        // SAFETY: valid buffer.
        unsafe { sys::hb_buffer_reset(self.raw) };
    }

    /// Adds `item_length` UTF-16 code units of `text` starting at
    /// `item_offset`; the rest of `text` is context. Clusters are code unit
    /// offsets into `text`.
    pub fn add_utf16(&mut self, text: &[u16], item_offset: usize, item_length: usize) {
        assert!(item_offset + item_length <= text.len());

        // SAFETY: the item range lies inside `text`, as asserted; HarfBuzz
        // copies what it needs before returning.
        unsafe {
            sys::hb_buffer_add_utf16(
                self.raw,
                text.as_ptr(),
                text.len() as i32,
                item_offset as c_uint,
                item_length as i32,
            )
        };
    }

    /// The number of items in the buffer.
    pub fn len(&self) -> usize {
        // SAFETY: valid buffer.
        unsafe { sys::hb_buffer_get_length(self.raw) as usize }
    }

    /// Sets unset segment properties from the contents of the buffer.
    pub fn guess_segment_properties(&mut self) {
        // SAFETY: valid buffer.
        unsafe { sys::hb_buffer_guess_segment_properties(self.raw) };
    }

    pub fn set_direction(&mut self, direction: Direction) {
        let direction = match direction {
            Direction::LeftToRight => sys::HB_DIRECTION_LTR,
            Direction::RightToLeft => sys::HB_DIRECTION_RTL,
        };
        // SAFETY: valid buffer.
        unsafe { sys::hb_buffer_set_direction(self.raw, direction) };
    }

    /// Sets the language from a BCP 47 language tag.
    pub fn set_language(&mut self, language: &str) {
        // SAFETY: the tag is passed with its length; HarfBuzz interns the
        // language and does not keep the pointer.
        unsafe {
            let language = sys::hb_language_from_string(language.as_ptr() as *const c_char, language.len() as i32);
            sys::hb_buffer_set_language(self.raw, language);
        }
    }

    /// The items of the buffer: codepoints before shaping, glyphs after.
    pub fn glyph_infos(&self) -> &[GlyphInfo] {
        let mut length = 0;
        // SAFETY: HarfBuzz returns a pointer to `length` items that stays
        // valid until the buffer is modified, which needs `&mut self`.
        unsafe {
            let infos = sys::hb_buffer_get_glyph_infos(self.raw, &mut length);
            if infos.is_null() || length == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(infos, length as usize)
            }
        }
    }

    /// The items of the buffer, for modification before shaping.
    pub fn glyph_infos_mut(&mut self) -> &mut [GlyphInfo] {
        let mut length = 0;
        // SAFETY: as in `glyph_infos`; the exclusive borrow of the buffer
        // guarantees unique access to the items.
        unsafe {
            let infos = sys::hb_buffer_get_glyph_infos(self.raw, &mut length);
            if infos.is_null() || length == 0 {
                &mut []
            } else {
                std::slice::from_raw_parts_mut(infos, length as usize)
            }
        }
    }

    /// The positions of the glyphs after shaping.
    pub fn glyph_positions(&self) -> &[GlyphPosition] {
        let mut length = 0;
        // SAFETY: as in `glyph_infos`.
        unsafe {
            let positions = sys::hb_buffer_get_glyph_positions(self.raw, &mut length);
            if positions.is_null() || length == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(positions, length as usize)
            }
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: releases the reference taken at creation.
        unsafe { sys::hb_buffer_destroy(self.raw) };
    }
}
