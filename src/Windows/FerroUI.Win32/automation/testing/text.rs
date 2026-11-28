//! Text pattern calls on the Windows UI Automation client interfaces.

use super::{check, slot, Element, Getter, Owned};
use crate::automation::marshalling::{free_bstr, ptr_to_string_bstr, string_to_bstr, SafeArrayRef};
use ferroui_microcom::Guid;
use std::ffi::c_void;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::POINT;

const TEXT_PATTERN2: i32 = 10024;
const IID_TEXT_PATTERN2: Guid = Guid::from_u128(0x506a921a_fcc9_409f_b23b_37eb74106872);

// The SDK's UIAutomationClient.h defines the vtable layout, including IUnknown.
const RANGE_FROM_POINT: usize = 3;
const RANGE_FROM_CHILD: usize = 4;
const GET_SELECTION: usize = 5;
const GET_DOCUMENT_RANGE: usize = 7;
const GET_CARET_RANGE: usize = 10;
const RANGE_ARRAY_LENGTH: usize = 3;
const RANGE_ARRAY_GET_ELEMENT: usize = 4;
const RANGE_COMPARE_ENDPOINTS: usize = 5;
const RANGE_FIND_TEXT: usize = 8;
const RANGE_GET_BOUNDING_RECTANGLES: usize = 10;
const RANGE_GET_ENCLOSING_ELEMENT: usize = 11;
const RANGE_GET_TEXT: usize = 12;
const RANGE_SELECT: usize = 16;
const RANGE_GET_CHILDREN: usize = 20;

pub struct TextPattern(Owned);
pub struct TextRange(Owned);

fn owned_result(value: *mut c_void, method: &str) -> Result<Owned, String> {
    if value.is_null() {
        Err(format!("{method}: no result"))
    } else {
        Ok(Owned(value))
    }
}

impl Element {
    pub fn text_pattern(&self) -> Result<TextPattern, String> {
        self.required_pattern(
            TEXT_PATTERN2,
            &IID_TEXT_PATTERN2,
            "the TextPattern2 pattern",
        )
        .map(TextPattern)
    }
}

impl TextPattern {
    pub fn range_from_point(&self, x: i32, y: i32) -> Result<TextRange, String> {
        type RangeFromPoint =
            unsafe extern "system" fn(*mut c_void, POINT, *mut *mut c_void) -> i32;
        let method = "IUIAutomationTextPattern::RangeFromPoint";
        let mut range = null_mut();
        let pattern = self.0 .0;
        // SAFETY: the interface takes screen pixels by value and a writable pointer output.
        check(
            unsafe {
                slot::<RangeFromPoint>(pattern, RANGE_FROM_POINT)(
                    pattern,
                    POINT { x, y },
                    &mut range,
                )
            },
            method,
        )?;
        owned_result(range, method).map(TextRange)
    }

    pub fn range_from_child(&self, child: &Element) -> Result<TextRange, String> {
        type RangeFromChild =
            unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void) -> i32;
        let method = "IUIAutomationTextPattern::RangeFromChild";
        let mut range = null_mut();
        let pattern = self.0 .0;
        // SAFETY: child holds a live element interface and the pointer output is writable.
        check(
            unsafe {
                slot::<RangeFromChild>(pattern, RANGE_FROM_CHILD)(pattern, child.0 .0, &mut range)
            },
            method,
        )?;
        owned_result(range, method).map(TextRange)
    }

    pub fn document_range(&self) -> Result<TextRange, String> {
        let method = "IUIAutomationTextPattern::get_DocumentRange";
        let range = self.0.number(GET_DOCUMENT_RANGE, method)?;
        owned_result(range, method).map(TextRange)
    }

    pub fn caret_range(&self) -> Result<(bool, TextRange), String> {
        type GetCaretRange =
            unsafe extern "system" fn(*mut c_void, *mut i32, *mut *mut c_void) -> i32;
        let method = "IUIAutomationTextPattern2::GetCaretRange";
        let mut active = 0;
        let mut range = null_mut();
        let pattern = self.0 .0;
        // SAFETY: the method accepts writable BOOL and interface pointer outputs.
        check(
            unsafe {
                slot::<GetCaretRange>(pattern, GET_CARET_RANGE)(pattern, &mut active, &mut range)
            },
            method,
        )?;
        Ok((active != 0, TextRange(owned_result(range, method)?)))
    }

    pub fn selection(&self) -> Result<Vec<TextRange>, String> {
        type GetElement = unsafe extern "system" fn(*mut c_void, i32, *mut *mut c_void) -> i32;
        let method = "IUIAutomationTextPattern::GetSelection";
        let ranges = owned_result(self.0.number(GET_SELECTION, method)?, method)?;
        let length = ranges.number::<i32>(
            RANGE_ARRAY_LENGTH,
            "IUIAutomationTextRangeArray::get_Length",
        )?;
        (0..length)
            .map(|index| {
                let method = "IUIAutomationTextRangeArray::GetElement";
                let mut range = null_mut();
                // SAFETY: the array is live, index is in range, and the output is writable.
                check(
                    unsafe {
                        slot::<GetElement>(ranges.0, RANGE_ARRAY_GET_ELEMENT)(
                            ranges.0, index, &mut range,
                        )
                    },
                    method,
                )?;
                owned_result(range, method).map(TextRange)
            })
            .collect()
    }
}

impl TextRange {
    pub fn enclosing_element(&self) -> Result<Element, String> {
        let method = "IUIAutomationTextRange::GetEnclosingElement";
        let element = self.0.number(RANGE_GET_ENCLOSING_ELEMENT, method)?;
        owned_result(element, method).map(Element)
    }

    pub fn children(&self) -> Result<Vec<Element>, String> {
        Element::collect_array(
            self.0
                .number(RANGE_GET_CHILDREN, "IUIAutomationTextRange::GetChildren")?,
        )
    }

    pub fn text(&self) -> Result<String, String> {
        type GetText = unsafe extern "system" fn(*mut c_void, i32, *mut *mut u16) -> i32;
        let range = self.0 .0;
        let mut value = null_mut();
        // SAFETY: -1 requests the whole range and GetText returns an owned BSTR.
        unsafe {
            check(
                slot::<GetText>(range, RANGE_GET_TEXT)(range, -1, &mut value),
                "IUIAutomationTextRange::GetText",
            )?;
            let text = ptr_to_string_bstr(value).unwrap_or_default();
            free_bstr(value);
            Ok(text)
        }
    }

    pub fn find_text(&self, text: &str) -> Result<Option<TextRange>, String> {
        type FindText =
            unsafe extern "system" fn(*mut c_void, *mut u16, i32, i32, *mut *mut c_void) -> i32;
        let range = self.0 .0;
        let mut found = null_mut();
        let text = string_to_bstr(text);
        // SAFETY: the search string is a live BSTR and the interface pointer output is writable.
        let result = unsafe {
            let result = slot::<FindText>(range, RANGE_FIND_TEXT)(range, text, 0, 0, &mut found);
            free_bstr(text);
            result
        };
        check(result, "IUIAutomationTextRange::FindText")?;
        Ok((!found.is_null()).then(|| TextRange(Owned(found))))
    }

    pub fn select(&self) -> Result<(), String> {
        self.0.call(RANGE_SELECT, "IUIAutomationTextRange::Select")
    }

    pub fn starts_at_end_of(&self, other: &TextRange) -> Result<bool, String> {
        type CompareEndpoints =
            unsafe extern "system" fn(*mut c_void, i32, *mut c_void, i32, *mut i32) -> i32;
        let range = self.0 .0;
        let mut comparison = 0;
        // SAFETY: both ranges are live. The endpoint discriminants are Start (0) and End (1).
        check(
            unsafe {
                slot::<CompareEndpoints>(range, RANGE_COMPARE_ENDPOINTS)(
                    range,
                    0,
                    other.0 .0,
                    1,
                    &mut comparison,
                )
            },
            "IUIAutomationTextRange::CompareEndpoints",
        )?;
        Ok(comparison == 0)
    }

    pub fn bounding_rectangles(&self) -> Result<Vec<[f64; 4]>, String> {
        let range = self.0 .0;
        let mut bounds = SafeArrayRef::null();
        // SAFETY: this method returns an owned one-dimensional SAFEARRAY of doubles.
        let coords = unsafe {
            check(
                slot::<Getter<SafeArrayRef>>(range, RANGE_GET_BOUNDING_RECTANGLES)(
                    range,
                    &mut bounds,
                ),
                "IUIAutomationTextRange::GetBoundingRectangles",
            )?;
            let coords = SafeArrayRef::to_array::<f64>(Some(bounds));
            bounds.destroy();
            coords
                .map_err(|error| error.to_string())?
                .unwrap_or_default()
        };
        let rectangles = coords.chunks_exact(4);
        if !rectangles.remainder().is_empty() {
            return Err("UI Automation returned incomplete rectangle coordinates".into());
        }
        Ok(rectangles
            .map(|rect| [rect[0], rect[1], rect[2], rect[3]])
            .collect())
    }
}
