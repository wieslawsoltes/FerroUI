//! Compares the table of `font-language-override` with HarfBuzz. A test of its own, as the crate
//! itself forbids `unsafe` code and this calls HarfBuzz directly.

use ferroui_fonts_web_core::language::language_table;
use harfbuzz_sys::{hb_language_from_string, hb_ot_tags_from_script_and_language, HB_SCRIPT_COMMON};
use std::ffi::CString;

/// HarfBuzz is given the whole language of the culture on the path of the framework.
#[test]
fn harfbuzz_reads_every_entry_of_the_table_as_that_language_system() {
    for (tag, language) in language_table() {
        let name = CString::new(*language).unwrap();
        let mut count = 8;
        let mut found = [0u32; 8];
        // SAFETY: the name is a valid C string, and `found` has room for `count` tags. The script
        // tags are not requested, which the function allows with null pointers.
        unsafe {
            hb_ot_tags_from_script_and_language(
                HB_SCRIPT_COMMON,
                hb_language_from_string(name.as_ptr(), -1),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut count,
                found.as_mut_ptr(),
            );
        }
        let expected = u32::from_be_bytes(format!("{tag:<4}").as_bytes().try_into().unwrap());
        assert!(found[..count as usize].contains(&expected), "{tag} from {language}");
    }
}
