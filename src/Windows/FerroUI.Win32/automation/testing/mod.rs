//! A client of UI Automation for runs without a person: it reads the tree
//! of a window and drives its patterns through the client interface of the
//! system (`CUIAutomation`), as a test tool or a screen reader does. The
//! smoke run of the desktop example and the walk of the catalog use it, on
//! a thread of their own.
//!
//! Not from the reference, and no part of what an application uses of the
//! backend. The client interfaces are the ones of `UIAutomationClient.h`;
//! a method is called through its slot of the table of the interface, with
//! the slot numbers of that header (they were compared with the
//! declarations of the `windows` crate, which are generated from the
//! metadata of the Windows SDK).

#![allow(missing_docs)]

use crate::automation::interop::UiaControlTypeId;
use crate::automation::marshalling::{free_bstr, ptr_to_string_bstr, string_to_bstr, ComVariant};
pub use crate::automation::marshalling::VariantValue;
use ferroui_microcom::{ComPtr, Guid, HResult};
use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use windows_sys::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize};

mod text;
pub use text::{TextPattern, TextRange};

#[allow(clippy::all, non_snake_case, non_upper_case_globals, dead_code, unused_unsafe, unsafe_op_in_unsafe_fn)]
mod uia_client {
    include!(concat!(env!("OUT_DIR"), "/uia_client.rs"));
}

use uia_client::{IUIAutomationFocusChangedEventHandler, IUIAutomationFocusChangedEventHandlerImpl};

const CLSID_CUI_AUTOMATION: Guid = Guid::from_u128(0xff48dba4_60ef_4201_aa87_54103eef594e);
const CLSID_CUI_AUTOMATION8: Guid = Guid::from_u128(0xe22ad333_b25f_460c_83d0_0581107395c9);
const IID_IUI_AUTOMATION: Guid = Guid::from_u128(0x30cbe57d_d9d0_452a_ab13_7ac5ac4825ee);
const IID_IUI_AUTOMATION2: Guid = Guid::from_u128(0x34723aff_0c9d_49d0_9896_7ab52df8cd8a);

/// The pattern identifiers the client asks for (`UIA_*PatternId`).
pub const INVOKE_PATTERN: i32 = 10000;
pub const VALUE_PATTERN: i32 = 10002;
pub const RANGE_VALUE_PATTERN: i32 = 10003;
pub const TOGGLE_PATTERN: i32 = 10015;
pub const EXPAND_COLLAPSE_PATTERN: i32 = 10005;
pub const WINDOW_PATTERN: i32 = 10009;
pub const SELECTION_ITEM_PATTERN: i32 = 10010;
pub const SCROLL_ITEM_PATTERN: i32 = 10017;

/// The scopes of a search (`TreeScope`).
pub const TREE_SCOPE_ELEMENT: i32 = 1;
pub const TREE_SCOPE_CHILDREN: i32 = 2;
pub const TREE_SCOPE_DESCENDANTS: i32 = 4;
pub const TREE_SCOPE_SUBTREE: i32 = 7;

const IID_INVOKE_PATTERN: Guid = Guid::from_u128(0xfb377fbe_8ea6_46d5_9c73_6499642d3059);
const IID_TOGGLE_PATTERN: Guid = Guid::from_u128(0x94cf8058_9b8d_4ab9_8bfd_4cd0a33c8c70);
const IID_VALUE_PATTERN: Guid = Guid::from_u128(0xa94cd8b1_0844_4cd6_9d2d_640537ab39e9);
const IID_RANGE_VALUE_PATTERN: Guid = Guid::from_u128(0x59213f4f_7346_49e5_b120_80555987a148);
const IID_EXPAND_COLLAPSE_PATTERN: Guid = Guid::from_u128(0x619be086_1f4e_4ee4_bafa_210128738730);
const IID_SELECTION_ITEM_PATTERN: Guid = Guid::from_u128(0xa8efa66a_0fda_421a_9194_38021f3578ea);
const IID_WINDOW_PATTERN: Guid = Guid::from_u128(0x0faef453_9208_43ef_bbb2_3b485177864f);
const IID_SCROLL_ITEM_PATTERN: Guid = Guid::from_u128(0xb488300f_d015_4f19_9c29_bb595e3645ef);

// The slots of the methods the client calls, after the three of `IUnknown`.
mod slots {
    pub const ADD_REF: usize = 1;
    pub const RELEASE: usize = 2;
    // IUIAutomation
    pub const COMPARE_ELEMENTS: usize = 3;
    pub const GET_ROOT_ELEMENT: usize = 5;
    pub const ELEMENT_FROM_HANDLE: usize = 6;
    pub const ELEMENT_FROM_POINT: usize = 7;
    pub const GET_FOCUSED_ELEMENT: usize = 8;
    pub const CONTROL_VIEW_WALKER: usize = 14;
    pub const RAW_VIEW_WALKER: usize = 16;
    pub const CONTROL_VIEW_CONDITION: usize = 18;
    pub const CREATE_TRUE_CONDITION: usize = 21;
    pub const CREATE_PROPERTY_CONDITION: usize = 23;
    pub const CREATE_AND_CONDITION: usize = 25;
    pub const ADD_FOCUS_CHANGED_EVENT_HANDLER: usize = 39;
    pub const REMOVE_ALL_EVENT_HANDLERS: usize = 41;
    // IUIAutomation2, after the 55 methods of IUIAutomation
    pub const SET_CONNECTION_TIMEOUT: usize = 61;
    pub const SET_TRANSACTION_TIMEOUT: usize = 63;
    // IUIAutomationElement
    pub const SET_FOCUS: usize = 3;
    pub const FIND_ALL: usize = 6;
    pub const GET_CURRENT_PROPERTY_VALUE: usize = 10;
    pub const GET_CURRENT_PATTERN_AS: usize = 14;
    pub const CURRENT_PROCESS_ID: usize = 20;
    pub const CURRENT_CONTROL_TYPE: usize = 21;
    pub const CURRENT_NAME: usize = 23;
    pub const CURRENT_ACCELERATOR_KEY: usize = 24;
    pub const CURRENT_HAS_KEYBOARD_FOCUS: usize = 26;
    pub const CURRENT_IS_KEYBOARD_FOCUSABLE: usize = 27;
    pub const CURRENT_IS_ENABLED: usize = 28;
    pub const CURRENT_AUTOMATION_ID: usize = 29;
    pub const CURRENT_CLASS_NAME: usize = 30;
    pub const CURRENT_NATIVE_WINDOW_HANDLE: usize = 36;
    pub const CURRENT_IS_OFFSCREEN: usize = 38;
    pub const CURRENT_FRAMEWORK_ID: usize = 40;
    pub const CURRENT_BOUNDING_RECTANGLE: usize = 43;
    // IUIAutomationTreeWalker
    pub const GET_PARENT_ELEMENT: usize = 3;
    pub const GET_FIRST_CHILD_ELEMENT: usize = 4;
    pub const GET_NEXT_SIBLING_ELEMENT: usize = 6;
    // IUIAutomationElementArray
    pub const ARRAY_LENGTH: usize = 3;
    pub const ARRAY_GET_ELEMENT: usize = 4;
    // The patterns: the first method, and the first property.
    pub const PATTERN_FIRST: usize = 3;
    pub const PATTERN_SECOND: usize = 4;
    // IUIAutomationExpandCollapsePattern
    pub const EXPAND_COLLAPSE_STATE: usize = 5;
    // IUIAutomationSelectionItemPattern
    pub const SELECTION_ITEM_IS_SELECTED: usize = 6;
}

/// The function in a slot of the table of a COM object.
///
/// # Safety
/// `object` is a live interface pointer whose table has a function of the
/// type `F` in the slot `index`.
unsafe fn slot<F: Copy>(object: *mut c_void, index: usize) -> F {
    debug_assert_eq!(std::mem::size_of::<F>(), std::mem::size_of::<usize>());
    // SAFETY: the contract of the function: an interface pointer points at
    // the pointer of its table, which is an array of function pointers.
    unsafe {
        let table = *object.cast::<*const *const c_void>();
        std::mem::transmute_copy::<*const c_void, F>(&*table.add(index))
    }
}

fn check(result: i32, what: &str) -> Result<(), String> {
    if result >= 0 {
        Ok(())
    } else {
        Err(format!("{what}: {}", HResult(result as u32)))
    }
}

type Method0 = unsafe extern "system" fn(*mut c_void) -> i32;
type Getter<T> = unsafe extern "system" fn(*mut c_void, *mut T) -> i32;

/// An interface pointer the client owns one reference of.
struct Owned(*mut c_void);

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: a live interface pointer with a reference of this value.
        unsafe { slot::<unsafe extern "system" fn(*mut c_void) -> u32>(self.0, slots::RELEASE)(self.0) };
    }
}

impl Clone for Owned {
    fn clone(&self) -> Owned {
        // SAFETY: a live interface pointer; the new value owns the reference
        // `AddRef` takes.
        unsafe { slot::<unsafe extern "system" fn(*mut c_void) -> u32>(self.0, slots::ADD_REF)(self.0) };
        Owned(self.0)
    }
}

impl Owned {
    /// A property of the object that is a number.
    fn number<T: Copy + Default>(&self, index: usize, what: &str) -> Result<T, String> {
        let mut value = T::default();
        // SAFETY: the slot of a property of the type `T` of this interface.
        check(unsafe { slot::<Getter<T>>(self.0, index)(self.0, &mut value) }, what)?;
        Ok(value)
    }

    /// A property of the object that is a string of the system.
    fn text(&self, index: usize, what: &str) -> Result<String, String> {
        let mut value: *mut u16 = std::ptr::null_mut();
        // SAFETY: the slot of a string property of this interface; the
        // string is owned by the caller, who frees it.
        unsafe {
            check(slot::<Getter<*mut u16>>(self.0, index)(self.0, &mut value), what)?;
            let text = ptr_to_string_bstr(value).unwrap_or_default();
            free_bstr(value);
            Ok(text)
        }
    }

    fn call(&self, index: usize, what: &str) -> Result<(), String> {
        // SAFETY: the slot of a method without arguments of this interface.
        check(unsafe { slot::<Method0>(self.0, index)(self.0) }, what)
    }
}

/// An element of the tree, as the client holds it.
#[derive(Clone)]
pub struct Element(Owned);

/// A condition of a search (`IUIAutomationCondition`).
pub struct Condition(Owned);

/// What the client reads of an element.
#[derive(Clone, Debug, PartialEq)]
pub struct ElementInfo {
    pub depth: usize,
    pub control_type: i32,
    pub name: String,
    pub class_name: String,
    pub framework_id: String,
    pub is_enabled: bool,
    pub is_keyboard_focusable: bool,
    /// Left, top, right, bottom, in pixels of the screen.
    pub bounding_rectangle: [i32; 4],
}

impl ElementInfo {
    /// The name of the control type (`Button` for 50000).
    pub fn control_type_name(&self) -> String {
        control_type_name(self.control_type)
    }
}

/// The name of a control type of the system.
pub fn control_type_name(control_type: i32) -> String {
    format!("{:?}", UiaControlTypeId(control_type))
}

impl Element {
    pub fn name(&self) -> Result<String, String> {
        self.0.text(slots::CURRENT_NAME, "IUIAutomationElement::get_CurrentName")
    }

    pub fn class_name(&self) -> Result<String, String> {
        self.0.text(slots::CURRENT_CLASS_NAME, "IUIAutomationElement::get_CurrentClassName")
    }

    pub fn framework_id(&self) -> Result<String, String> {
        self.0.text(slots::CURRENT_FRAMEWORK_ID, "IUIAutomationElement::get_CurrentFrameworkId")
    }

    pub fn control_type(&self) -> Result<i32, String> {
        self.0.number(slots::CURRENT_CONTROL_TYPE, "IUIAutomationElement::get_CurrentControlType")
    }

    pub fn is_enabled(&self) -> Result<bool, String> {
        Ok(self.0.number::<i32>(slots::CURRENT_IS_ENABLED, "IUIAutomationElement::get_CurrentIsEnabled")? != 0)
    }

    pub fn is_keyboard_focusable(&self) -> Result<bool, String> {
        let value: i32 =
            self.0.number(slots::CURRENT_IS_KEYBOARD_FOCUSABLE, "IUIAutomationElement::get_CurrentIsKeyboardFocusable")?;
        Ok(value != 0)
    }

    pub fn has_keyboard_focus(&self) -> Result<bool, String> {
        let value: i32 =
            self.0.number(slots::CURRENT_HAS_KEYBOARD_FOCUS, "IUIAutomationElement::get_CurrentHasKeyboardFocus")?;
        Ok(value != 0)
    }

    /// Left, top, right, bottom.
    pub fn bounding_rectangle(&self) -> Result<[i32; 4], String> {
        self.0.number(slots::CURRENT_BOUNDING_RECTANGLE, "IUIAutomationElement::get_CurrentBoundingRectangle")
    }

    pub fn set_focus(&self) -> Result<(), String> {
        self.0.call(slots::SET_FOCUS, "IUIAutomationElement::SetFocus")
    }

    pub fn info(&self, depth: usize) -> Result<ElementInfo, String> {
        Ok(ElementInfo {
            depth,
            control_type: self.control_type()?,
            name: self.name()?,
            class_name: self.class_name()?,
            framework_id: self.framework_id()?,
            is_enabled: self.is_enabled()?,
            is_keyboard_focusable: self.is_keyboard_focusable()?,
            bounding_rectangle: self.bounding_rectangle()?,
        })
    }

    /// The pattern of the element as its client interface; `None` when the
    /// element does not have the pattern.
    fn pattern(&self, pattern_id: i32, iid: &Guid, what: &str) -> Result<Option<Owned>, String> {
        type GetCurrentPatternAs = unsafe extern "system" fn(*mut c_void, i32, *const Guid, *mut *mut c_void) -> i32;
        let mut pattern = std::ptr::null_mut();
        // SAFETY: the slot of `GetCurrentPatternAs`, an identifier that
        // lives through the call and a place for the result.
        check(
            unsafe {
                slot::<GetCurrentPatternAs>(self.0 .0, slots::GET_CURRENT_PATTERN_AS)(self.0 .0, pattern_id, iid, &mut pattern)
            },
            what,
        )?;
        Ok((!pattern.is_null()).then(|| Owned(pattern)))
    }

    fn required_pattern(&self, pattern_id: i32, iid: &Guid, what: &str) -> Result<Owned, String> {
        self.pattern(pattern_id, iid, what)?.ok_or_else(|| format!("{what}: the element does not have the pattern"))
    }

    /// Whether the element has the pattern (one of the patterns of this
    /// module).
    pub fn has_pattern(&self, pattern_id: i32) -> Result<bool, String> {
        let iid = match pattern_id {
            INVOKE_PATTERN => &IID_INVOKE_PATTERN,
            VALUE_PATTERN => &IID_VALUE_PATTERN,
            RANGE_VALUE_PATTERN => &IID_RANGE_VALUE_PATTERN,
            TOGGLE_PATTERN => &IID_TOGGLE_PATTERN,
            EXPAND_COLLAPSE_PATTERN => &IID_EXPAND_COLLAPSE_PATTERN,
            SELECTION_ITEM_PATTERN => &IID_SELECTION_ITEM_PATTERN,
            WINDOW_PATTERN => &IID_WINDOW_PATTERN,
            SCROLL_ITEM_PATTERN => &IID_SCROLL_ITEM_PATTERN,
            other => return Err(format!("the pattern {other} is not one the client asks for")),
        };
        Ok(self.pattern(pattern_id, iid, "IUIAutomationElement::GetCurrentPatternAs")?.is_some())
    }

    /// `IUIAutomationInvokePattern::Invoke`.
    pub fn invoke(&self) -> Result<(), String> {
        self.required_pattern(INVOKE_PATTERN, &IID_INVOKE_PATTERN, "the invoke pattern")?
            .call(slots::PATTERN_FIRST, "IUIAutomationInvokePattern::Invoke")
    }

    /// `IUIAutomationTogglePattern::Toggle`.
    pub fn toggle(&self) -> Result<(), String> {
        self.required_pattern(TOGGLE_PATTERN, &IID_TOGGLE_PATTERN, "the toggle pattern")?
            .call(slots::PATTERN_FIRST, "IUIAutomationTogglePattern::Toggle")
    }

    /// The toggle state: 0 off, 1 on, 2 indeterminate.
    pub fn toggle_state(&self) -> Result<i32, String> {
        self.required_pattern(TOGGLE_PATTERN, &IID_TOGGLE_PATTERN, "the toggle pattern")?
            .number(slots::PATTERN_SECOND, "IUIAutomationTogglePattern::get_CurrentToggleState")
    }

    /// `IUIAutomationValuePattern::SetValue`.
    pub fn set_value(&self, value: &str) -> Result<(), String> {
        let pattern = self.required_pattern(VALUE_PATTERN, &IID_VALUE_PATTERN, "the value pattern")?;
        let bstr = string_to_bstr(value);
        // SAFETY: the slot of `SetValue`, which takes a string of the
        // system; the string is freed after the call.
        unsafe {
            let result =
                slot::<unsafe extern "system" fn(*mut c_void, *mut u16) -> i32>(pattern.0, slots::PATTERN_FIRST)(pattern.0, bstr);
            free_bstr(bstr);
            check(result, "IUIAutomationValuePattern::SetValue")
        }
    }

    pub fn value(&self) -> Result<String, String> {
        self.required_pattern(VALUE_PATTERN, &IID_VALUE_PATTERN, "the value pattern")?
            .text(slots::PATTERN_SECOND, "IUIAutomationValuePattern::get_CurrentValue")
    }

    /// `IUIAutomationRangeValuePattern::SetValue`.
    pub fn set_range_value(&self, value: f64) -> Result<(), String> {
        let pattern = self.required_pattern(RANGE_VALUE_PATTERN, &IID_RANGE_VALUE_PATTERN, "the range value pattern")?;
        // SAFETY: the slot of `SetValue`, which takes a double.
        check(
            unsafe {
                slot::<unsafe extern "system" fn(*mut c_void, f64) -> i32>(pattern.0, slots::PATTERN_FIRST)(pattern.0, value)
            },
            "IUIAutomationRangeValuePattern::SetValue",
        )
    }

    pub fn range_value(&self) -> Result<f64, String> {
        self.required_pattern(RANGE_VALUE_PATTERN, &IID_RANGE_VALUE_PATTERN, "the range value pattern")?
            .number(slots::PATTERN_SECOND, "IUIAutomationRangeValuePattern::get_CurrentValue")
    }

    pub fn automation_id(&self) -> Result<String, String> {
        self.0.text(slots::CURRENT_AUTOMATION_ID, "IUIAutomationElement::get_CurrentAutomationId")
    }

    pub fn accelerator_key(&self) -> Result<String, String> {
        self.0.text(slots::CURRENT_ACCELERATOR_KEY, "IUIAutomationElement::get_CurrentAcceleratorKey")
    }

    pub fn process_id(&self) -> Result<i32, String> {
        self.0.number(slots::CURRENT_PROCESS_ID, "IUIAutomationElement::get_CurrentProcessId")
    }

    /// The window handle of the element, 0 for an element that is no window.
    pub fn native_window_handle(&self) -> Result<isize, String> {
        self.0.number(slots::CURRENT_NATIVE_WINDOW_HANDLE, "IUIAutomationElement::get_CurrentNativeWindowHandle")
    }

    pub fn is_offscreen(&self) -> Result<bool, String> {
        Ok(self.0.number::<i32>(slots::CURRENT_IS_OFFSCREEN, "IUIAutomationElement::get_CurrentIsOffscreen")? != 0)
    }

    /// `GetCurrentPropertyValue`: the value of any property, `None` for an
    /// empty value.
    pub fn property_value(&self, property_id: i32) -> Result<Option<VariantValue>, String> {
        let mut variant = ComVariant::null();
        // SAFETY: the slot of `GetCurrentPropertyValue`, an identifier and a
        // place for a variant the caller owns and frees.
        unsafe {
            check(
                slot::<unsafe extern "system" fn(*mut c_void, i32, *mut ComVariant) -> i32>(
                    self.0 .0,
                    slots::GET_CURRENT_PROPERTY_VALUE,
                )(self.0 .0, property_id, &mut variant),
                "IUIAutomationElement::GetCurrentPropertyValue",
            )?;
            let value = variant.as_object().map_err(|error| format!("the property {property_id}: {error}"));
            variant.dispose();
            value
        }
    }

    /// `FindAll`: the elements of the scope (`TREE_SCOPE_*`) that meet the
    /// condition, in the order of the tree.
    pub fn find_all(&self, scope: i32, condition: &Condition) -> Result<Vec<Element>, String> {
        let mut array = std::ptr::null_mut();
        // SAFETY: the slot of `FindAll`, a scope, a live condition and a
        // place for the array the caller owns.
        check(
            unsafe {
                slot::<unsafe extern "system" fn(*mut c_void, i32, *mut c_void, *mut *mut c_void) -> i32>(
                    self.0 .0,
                    slots::FIND_ALL,
                )(self.0 .0, scope, condition.0 .0, &mut array)
            },
            "IUIAutomationElement::FindAll",
        )?;
        Self::collect_array(array)
    }

    fn collect_array(array: *mut c_void) -> Result<Vec<Element>, String> {
        if array.is_null() {
            return Ok(Vec::new());
        }
        let array = Owned(array);
        let length: i32 = array.number(slots::ARRAY_LENGTH, "IUIAutomationElementArray::get_Length")?;
        let mut elements = Vec::with_capacity(length.max(0) as usize);
        for index in 0..length {
            let mut element = std::ptr::null_mut();
            // SAFETY: the slot of `GetElement`, an index below the length and
            // a place for an element the caller owns.
            check(
                unsafe {
                    slot::<unsafe extern "system" fn(*mut c_void, i32, *mut *mut c_void) -> i32>(
                        array.0,
                        slots::ARRAY_GET_ELEMENT,
                    )(array.0, index, &mut element)
                },
                "IUIAutomationElementArray::GetElement",
            )?;
            if !element.is_null() {
                elements.push(Element(Owned(element)));
            }
        }
        Ok(elements)
    }

    /// `IUIAutomationExpandCollapsePattern::Expand`.
    pub fn expand(&self) -> Result<(), String> {
        self.required_pattern(EXPAND_COLLAPSE_PATTERN, &IID_EXPAND_COLLAPSE_PATTERN, "the expand collapse pattern")?
            .call(slots::PATTERN_FIRST, "IUIAutomationExpandCollapsePattern::Expand")
    }

    /// `IUIAutomationExpandCollapsePattern::Collapse`.
    pub fn collapse(&self) -> Result<(), String> {
        self.required_pattern(EXPAND_COLLAPSE_PATTERN, &IID_EXPAND_COLLAPSE_PATTERN, "the expand collapse pattern")?
            .call(slots::PATTERN_SECOND, "IUIAutomationExpandCollapsePattern::Collapse")
    }

    /// The state of the expand collapse pattern: 0 collapsed, 1 expanded,
    /// 2 partially expanded, 3 a leaf.
    pub fn expand_collapse_state(&self) -> Result<i32, String> {
        self.required_pattern(EXPAND_COLLAPSE_PATTERN, &IID_EXPAND_COLLAPSE_PATTERN, "the expand collapse pattern")?
            .number(slots::EXPAND_COLLAPSE_STATE, "IUIAutomationExpandCollapsePattern::get_CurrentExpandCollapseState")
    }

    /// `IUIAutomationSelectionItemPattern::Select`.
    pub fn select(&self) -> Result<(), String> {
        self.required_pattern(SELECTION_ITEM_PATTERN, &IID_SELECTION_ITEM_PATTERN, "the selection item pattern")?
            .call(slots::PATTERN_FIRST, "IUIAutomationSelectionItemPattern::Select")
    }

    /// Whether the element is selected; false for an element without the
    /// selection item pattern.
    pub fn is_selected(&self) -> Result<bool, String> {
        match self.pattern(SELECTION_ITEM_PATTERN, &IID_SELECTION_ITEM_PATTERN, "the selection item pattern")? {
            Some(pattern) => Ok(pattern.number::<i32>(
                slots::SELECTION_ITEM_IS_SELECTED,
                "IUIAutomationSelectionItemPattern::get_CurrentIsSelected",
            )? != 0),
            None => Ok(false),
        }
    }

    /// `IUIAutomationScrollItemPattern::ScrollIntoView`.
    pub fn scroll_into_view(&self) -> Result<(), String> {
        self.required_pattern(SCROLL_ITEM_PATTERN, &IID_SCROLL_ITEM_PATTERN, "the scroll item pattern")?
            .call(slots::PATTERN_FIRST, "IUIAutomationScrollItemPattern::ScrollIntoView")
    }

    /// `IUIAutomationWindowPattern::Close`.
    pub fn close(&self) -> Result<(), String> {
        self.required_pattern(WINDOW_PATTERN, &IID_WINDOW_PATTERN, "the window pattern")?
            .call(slots::PATTERN_FIRST, "IUIAutomationWindowPattern::Close")
    }
}

/// The elements that got the focus, as the handler was told of them: interface pointers the
/// log holds a reference of, until the client reads them.
///
/// The handler does not ask an element anything. It is called by the automation core while
/// the provider that raised the event may still be inside the call that raised it, on its UI
/// thread; a question to the element is a call of the provider, which waits for that thread
/// (run 38101124412 did not end after the first event of a provider of the process of the
/// client). The client asks later, on its own thread ([`UiaClient::take_focus_events`]).
#[derive(Clone, Default)]
pub struct FocusLog {
    elements: Arc<Mutex<Vec<usize>>>,
}

impl FocusLog {
    pub fn new() -> FocusLog {
        FocusLog::default()
    }
}

struct FocusChangedHandler {
    log: FocusLog,
}

impl IUIAutomationFocusChangedEventHandlerImpl for FocusChangedHandler {
    fn handle_focus_changed_event(&self, sender: *mut c_void) -> Result<(), HResult> {
        if sender.is_null() {
            return Ok(());
        }
        // SAFETY: a live interface pointer of the caller; the log takes a reference of it
        // (`AddRef`, the second slot), which the client gives back when it has read it.
        unsafe { slot::<unsafe extern "system" fn(*mut c_void) -> u32>(sender, 1)(sender) };
        self.log.elements.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(sender as usize);
        Ok(())
    }
}

/// The client: COM on the calling thread (the multithreaded apartment, so
/// that the handlers of events are called without a message loop), the
/// automation object and its walker of the raw view. To be made, used and
/// dropped on one thread that is not the UI thread of the windows it reads:
/// a call waits for that thread to answer.
pub struct UiaClient {
    automation: Owned,
    walker: Owned,
    handlers: Vec<ComPtr<IUIAutomationFocusChangedEventHandler>>,
}

impl UiaClient {
    pub fn new() -> Result<UiaClient, String> {
        Self::create(&CLSID_CUI_AUTOMATION)
    }

    /// Creates the Windows 8 client, which exposes IUIAutomation2 for timeouts.
    pub fn new_v8() -> Result<UiaClient, String> {
        Self::create(&CLSID_CUI_AUTOMATION8)
    }

    fn create(class: &Guid) -> Result<UiaClient, String> {
        // SAFETY: the reserved argument is null; the flag is
        // `COINIT_MULTITHREADED`. The thread keeps COM until it ends.
        check(unsafe { CoInitializeEx(std::ptr::null(), 0) }, "CoInitializeEx")?;
        let mut automation = std::ptr::null_mut();
        // SAFETY: two identifiers that live through the call, no outer
        // object, an in-process server (`CLSCTX_INPROC_SERVER`), and a
        // place for the result.
        check(
            unsafe {
                CoCreateInstance(
                    (class as *const Guid).cast(),
                    std::ptr::null_mut(),
                    1,
                    (&IID_IUI_AUTOMATION as *const Guid).cast(),
                    &mut automation,
                )
            },
            "CoCreateInstance(CUIAutomation)",
        )?;
        let automation = Owned(automation);
        let mut walker = std::ptr::null_mut();
        // SAFETY: the slot of `get_RawViewWalker`.
        check(
            unsafe { slot::<Getter<*mut c_void>>(automation.0, slots::RAW_VIEW_WALKER)(automation.0, &mut walker) },
            "IUIAutomation::get_RawViewWalker",
        )?;
        Ok(UiaClient { automation, walker: Owned(walker), handlers: Vec::new() })
    }

    /// `IUIAutomation2::put_ConnectionTimeout` and `put_TransactionTimeout`:
    /// how long a call of the client waits for a provider that does not
    /// answer (the defaults are 2 and 20 seconds), so that an application
    /// that stopped responding fails the calls of a test instead of holding
    /// each of them.
    pub fn set_timeouts(&self, connection_ms: u32, transaction_ms: u32) -> Result<(), String> {
        type QueryInterface = unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> i32;
        let mut automation2 = std::ptr::null_mut();
        // SAFETY: `QueryInterface` of a live object, an identifier that lives
        // through the call and a place for the result, which is owned.
        check(
            unsafe { slot::<QueryInterface>(self.automation.0, 0)(self.automation.0, &IID_IUI_AUTOMATION2, &mut automation2) },
            "QueryInterface(IUIAutomation2)",
        )?;
        let automation2 = Owned(automation2);
        type SetTimeout = unsafe extern "system" fn(*mut c_void, u32) -> i32;
        // SAFETY: the slots of the two setters of `IUIAutomation2`.
        unsafe {
            check(
                slot::<SetTimeout>(automation2.0, slots::SET_CONNECTION_TIMEOUT)(automation2.0, connection_ms),
                "IUIAutomation2::put_ConnectionTimeout",
            )?;
            check(
                slot::<SetTimeout>(automation2.0, slots::SET_TRANSACTION_TIMEOUT)(automation2.0, transaction_ms),
                "IUIAutomation2::put_TransactionTimeout",
            )
        }
    }

    /// `IUIAutomation::ElementFromHandle`.
    pub fn element_from_handle(&self, hwnd: isize) -> Result<Element, String> {
        let mut element = std::ptr::null_mut();
        // SAFETY: the slot of `ElementFromHandle`, a window handle and a
        // place for the result.
        check(
            unsafe {
                slot::<unsafe extern "system" fn(*mut c_void, isize, *mut *mut c_void) -> i32>(
                    self.automation.0,
                    slots::ELEMENT_FROM_HANDLE,
                )(self.automation.0, hwnd, &mut element)
            },
            "IUIAutomation::ElementFromHandle",
        )?;
        if element.is_null() {
            return Err("IUIAutomation::ElementFromHandle: no element".to_owned());
        }
        Ok(Element(Owned(element)))
    }

    fn element_out(&self, index: usize, what: &str) -> Result<Option<Element>, String> {
        let mut element = std::ptr::null_mut();
        // SAFETY: a slot of the automation object that returns an element.
        check(unsafe { slot::<Getter<*mut c_void>>(self.automation.0, index)(self.automation.0, &mut element) }, what)?;
        Ok((!element.is_null()).then(|| Element(Owned(element))))
    }

    /// `IUIAutomation::GetRootElement`: the desktop.
    pub fn root_element(&self) -> Result<Element, String> {
        self.element_out(slots::GET_ROOT_ELEMENT, "IUIAutomation::GetRootElement")?
            .ok_or_else(|| "IUIAutomation::GetRootElement: no element".to_owned())
    }

    /// `IUIAutomation::ElementFromPoint`, in pixels of the screen.
    pub fn element_from_point(&self, x: i32, y: i32) -> Result<Option<Element>, String> {
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct Point {
            x: i32,
            y: i32,
        }
        let mut element = std::ptr::null_mut();
        // SAFETY: the slot of `ElementFromPoint`, a point by value and a
        // place for the result.
        check(
            unsafe {
                slot::<unsafe extern "system" fn(*mut c_void, Point, *mut *mut c_void) -> i32>(
                    self.automation.0,
                    slots::ELEMENT_FROM_POINT,
                )(self.automation.0, Point { x, y }, &mut element)
            },
            "IUIAutomation::ElementFromPoint",
        )?;
        Ok((!element.is_null()).then(|| Element(Owned(element))))
    }

    /// `IUIAutomation::CompareElements`: whether the two are the same
    /// element of the user interface.
    pub fn compare_elements(&self, a: &Element, b: &Element) -> Result<bool, String> {
        let mut same = 0i32;
        // SAFETY: the slot of `CompareElements`, two live elements and a
        // place for the result.
        check(
            unsafe {
                slot::<unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void, *mut i32) -> i32>(
                    self.automation.0,
                    slots::COMPARE_ELEMENTS,
                )(self.automation.0, a.0 .0, b.0 .0, &mut same)
            },
            "IUIAutomation::CompareElements",
        )?;
        Ok(same != 0)
    }

    fn condition_out(&self, result: i32, condition: *mut c_void, what: &str) -> Result<Condition, String> {
        check(result, what)?;
        if condition.is_null() {
            return Err(format!("{what}: no condition"));
        }
        Ok(Condition(Owned(condition)))
    }

    /// `IUIAutomation::CreateTrueCondition`.
    pub fn true_condition(&self) -> Result<Condition, String> {
        let mut condition = std::ptr::null_mut();
        // SAFETY: the slot of `CreateTrueCondition`.
        let result = unsafe {
            slot::<Getter<*mut c_void>>(self.automation.0, slots::CREATE_TRUE_CONDITION)(self.automation.0, &mut condition)
        };
        self.condition_out(result, condition, "IUIAutomation::CreateTrueCondition")
    }

    /// `IUIAutomation::get_ControlViewCondition`: the elements of the control
    /// view.
    pub fn control_view_condition(&self) -> Result<Condition, String> {
        let mut condition = std::ptr::null_mut();
        // SAFETY: the slot of `get_ControlViewCondition`.
        let result = unsafe {
            slot::<Getter<*mut c_void>>(self.automation.0, slots::CONTROL_VIEW_CONDITION)(self.automation.0, &mut condition)
        };
        self.condition_out(result, condition, "IUIAutomation::get_ControlViewCondition")
    }

    /// `IUIAutomation::CreatePropertyCondition`.
    pub fn property_condition(&self, property_id: i32, value: &VariantValue) -> Result<Condition, String> {
        let mut variant = ComVariant::create(Some(value)).map_err(|error| error.to_string())?;
        let mut condition = std::ptr::null_mut();
        // SAFETY: the slot of `CreatePropertyCondition`, which takes the
        // variant by value and copies it; the variant is freed after the call.
        let result = unsafe {
            let result = slot::<unsafe extern "system" fn(*mut c_void, i32, ComVariant, *mut *mut c_void) -> i32>(
                self.automation.0,
                slots::CREATE_PROPERTY_CONDITION,
            )(self.automation.0, property_id, variant, &mut condition);
            variant.dispose();
            result
        };
        self.condition_out(result, condition, "IUIAutomation::CreatePropertyCondition")
    }

    /// `IUIAutomation::CreateAndCondition`.
    pub fn and_condition(&self, a: &Condition, b: &Condition) -> Result<Condition, String> {
        let mut condition = std::ptr::null_mut();
        // SAFETY: the slot of `CreateAndCondition`, two live conditions and a
        // place for the result.
        let result = unsafe {
            slot::<unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void, *mut *mut c_void) -> i32>(
                self.automation.0,
                slots::CREATE_AND_CONDITION,
            )(self.automation.0, a.0 .0, b.0 .0, &mut condition)
        };
        self.condition_out(result, condition, "IUIAutomation::CreateAndCondition")
    }

    fn walker_step(&self, walker: &Owned, element: &Element, index: usize, what: &str) -> Result<Option<Element>, String> {
        let mut found = std::ptr::null_mut();
        // SAFETY: a slot of the walker that takes an element and a place for
        // the element it leads to.
        check(
            unsafe {
                slot::<unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void) -> i32>(walker.0, index)(
                    walker.0,
                    element.0 .0,
                    &mut found,
                )
            },
            what,
        )?;
        Ok((!found.is_null()).then(|| Element(Owned(found))))
    }

    fn control_view_walker(&self) -> Result<Owned, String> {
        let mut walker = std::ptr::null_mut();
        // SAFETY: the slot of `get_ControlViewWalker`.
        check(
            unsafe {
                slot::<Getter<*mut c_void>>(self.automation.0, slots::CONTROL_VIEW_WALKER)(self.automation.0, &mut walker)
            },
            "IUIAutomation::get_ControlViewWalker",
        )?;
        Ok(Owned(walker))
    }

    /// The children of an element in the control view, the view of a test
    /// tool.
    pub fn control_view_children(&self, element: &Element) -> Result<Vec<Element>, String> {
        let walker = self.control_view_walker()?;
        let mut result = Vec::new();
        let mut child =
            self.walker_step(&walker, element, slots::GET_FIRST_CHILD_ELEMENT, "IUIAutomationTreeWalker::GetFirstChildElement")?;
        while let Some(current) = child {
            child = self.walker_step(
                &walker,
                &current,
                slots::GET_NEXT_SIBLING_ELEMENT,
                "IUIAutomationTreeWalker::GetNextSiblingElement",
            )?;
            result.push(current);
        }
        Ok(result)
    }

    /// The parent of an element in the control view.
    pub fn control_view_parent(&self, element: &Element) -> Result<Option<Element>, String> {
        let walker = self.control_view_walker()?;
        self.walker_step(&walker, element, slots::GET_PARENT_ELEMENT, "IUIAutomationTreeWalker::GetParentElement")
    }

    /// `IUIAutomation::GetFocusedElement`.
    pub fn focused_element(&self) -> Result<Option<Element>, String> {
        let mut element = std::ptr::null_mut();
        // SAFETY: the slot of `GetFocusedElement`.
        check(
            unsafe {
                slot::<Getter<*mut c_void>>(self.automation.0, slots::GET_FOCUSED_ELEMENT)(self.automation.0, &mut element)
            },
            "IUIAutomation::GetFocusedElement",
        )?;
        Ok((!element.is_null()).then(|| Element(Owned(element))))
    }

    fn step(&self, element: &Element, index: usize, what: &str) -> Result<Option<Element>, String> {
        let mut found = std::ptr::null_mut();
        // SAFETY: a slot of the walker that takes an element and a place
        // for the element it leads to.
        check(
            unsafe {
                slot::<unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void) -> i32>(self.walker.0, index)(
                    self.walker.0,
                    element.0 .0,
                    &mut found,
                )
            },
            what,
        )?;
        Ok((!found.is_null()).then(|| Element(Owned(found))))
    }

    /// The children of an element in the raw view.
    pub fn children(&self, element: &Element) -> Result<Vec<Element>, String> {
        let mut result = Vec::new();
        let mut child = self.step(element, slots::GET_FIRST_CHILD_ELEMENT, "IUIAutomationTreeWalker::GetFirstChildElement")?;
        while let Some(current) = child {
            child = self.step(&current, slots::GET_NEXT_SIBLING_ELEMENT, "IUIAutomationTreeWalker::GetNextSiblingElement")?;
            result.push(current);
        }
        Ok(result)
    }

    /// The elements below `root`, depth first, each with its depth (the
    /// children of the root are at depth one).
    pub fn descendants(&self, root: &Element) -> Result<Vec<(usize, Element)>, String> {
        fn walk(client: &UiaClient, element: &Element, depth: usize, result: &mut Vec<(usize, Element)>) -> Result<(), String> {
            for child in client.children(element)? {
                let index = result.len();
                walk(client, &child, depth + 1, result)?;
                result.insert(index, (depth, child));
            }
            Ok(())
        }
        let mut result = Vec::new();
        walk(self, root, 1, &mut result)?;
        Ok(result)
    }

    /// The control type and the name of every element the log was told of since the last call,
    /// read now, on the thread of the client.
    pub fn take_focus_events(&self, log: &FocusLog) -> Vec<String> {
        let elements = std::mem::take(&mut *log.elements.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
        elements
            .into_iter()
            .map(|element| {
                // The reference the handler took becomes the element's.
                let element = Element(Owned(element as *mut c_void));
                let name = element.name().unwrap_or_else(|error| format!("<{error}>"));
                let control_type = element.control_type().map(control_type_name).unwrap_or_default();
                format!("{control_type} {name:?}")
            })
            .collect()
    }

    /// Listens for changes of the focus: the log gets every element the
    /// system reports.
    pub fn add_focus_changed_event_handler(&mut self, log: FocusLog) -> Result<(), String> {
        let handler = IUIAutomationFocusChangedEventHandler::from_impl(FocusChangedHandler { log });
        // SAFETY: the slot of `AddFocusChangedEventHandler`, no cache
        // request and a live handler, of which the system takes a
        // reference.
        check(
            unsafe {
                slot::<unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void) -> i32>(
                    self.automation.0,
                    slots::ADD_FOCUS_CHANGED_EVENT_HANDLER,
                )(self.automation.0, std::ptr::null_mut(), handler.as_ptr().cast())
            },
            "IUIAutomation::AddFocusChangedEventHandler",
        )?;
        self.handlers.push(handler);
        Ok(())
    }
}

impl Drop for UiaClient {
    fn drop(&mut self) {
        if !self.handlers.is_empty() {
            let _ = self.automation.call(slots::REMOVE_ALL_EVENT_HANDLERS, "IUIAutomation::RemoveAllEventHandlers");
        }
    }
}

/// The window of the desktop that has the foreground; 0 when none has.
pub fn foreground_window() -> isize {
    // SAFETY: no arguments.
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() as isize }
}

/// Ends COM on the calling thread, after every value of this module of the
/// thread was dropped.
pub fn uninitialize_com() {
    // SAFETY: pairs the initialisation of `UiaClient::new` on this thread;
    // the caller has dropped the client and its elements.
    unsafe { CoUninitialize() }
}

/// The count of the elements of each control type, most frequent first.
pub fn count_by_control_type(elements: &[ElementInfo]) -> Vec<(String, usize)> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for element in elements {
        *counts.entry(element.control_type_name()).or_default() += 1;
    }
    let mut counts: Vec<(String, usize)> = counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    counts
}
