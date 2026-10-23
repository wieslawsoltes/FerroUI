//! Characterization of the system MSFTEDIT engine, not WinUI's text engine.
//!
//! This probe prints native HomeKey/EndKey and directional movement destinations,
//! active-end flags, selection types, links, range gravity, formatting affinity,
//! HRESULTs and signed deltas. Its assertions validate setup and unambiguous
//! invariants; the first-run destination/count observations are not Rust parity
//! assertions. Keep the engine version with any fixture derived from its output.
//!
//! ABI and constants verified against Microsoft's Windows SDK projections:
//! https://github.com/microsoft/windows-rs/blob/32c3144490c016fe496a0aed769bce60987a2e9d/crates/libs/windows/src/Windows/Win32/UI/Controls/RichEdit/mod.rs
//! https://learn.microsoft.com/windows/win32/controls/em-getoleinterface
//! https://learn.microsoft.com/windows/win32/controls/text-object-model
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-getflags
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-setflags
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-typetext
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-homekey
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-endkey
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-moveup
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-movedown
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-moveleft
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-moveright
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-gettype
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextrange2-geturl
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextrange2-seturl
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextrange2-getgravity
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextrange2-setgravity
//! https://learn.microsoft.com/windows/windows-app-sdk/api/winrt/microsoft.ui.text.rangegravity
//! https://learn.microsoft.com/windows/win32/controls/em-geteditstyle

use crate::automation::marshalling::{free_bstr, string_to_bstr};
use crate::interop::unmanaged_methods::rtl_get_version;
use ferroui_microcom::{ComPtr, Guid, IUnknown, IUnknownVtbl};
use std::ffi::c_void;
use std::mem::{offset_of, size_of};
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{FreeLibrary, GetLastError, HMODULE, HWND};
use windows_sys::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
};
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleW, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32,
};
use windows_sys::Win32::System::Ole::{OleInitialize, OleUninitialize};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, SendMessageW, SetWindowTextW, ES_AUTOHSCROLL, ES_AUTOVSCROLL,
    ES_MULTILINE, WS_EX_RTLREADING, WS_POPUP,
};

type TestResult<T = ()> = Result<T, String>;
type Getter = unsafe extern "system" fn(*mut c_void, *mut i32) -> i32;
type Setter = unsafe extern "system" fn(*mut c_void, i32) -> i32;
type InterfaceGetter = unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32;
type BoundaryMethod = unsafe extern "system" fn(*mut c_void, i32, i32, *mut i32) -> i32;
type MoveMethod = unsafe extern "system" fn(*mut c_void, i32, i32, i32, *mut i32) -> i32;

const EM_GETOLEINTERFACE: u32 = 1084;
const IID_TEXT_DOCUMENT: Guid = Guid::from_u128(0x8cc497c0_a1df_11ce_8098_00aa0047be5d);
const IID_TEXT_RANGE2: Guid = Guid::from_u128(0xc241f5e2_7206_11d8_a2c7_00a0d1d6c6b3);
const TOM_SEL_START_ACTIVE: i32 = 1;

#[link(name = "oleaut32")]
unsafe extern "system" {
    fn SysStringLen(value: *const u16) -> u32;
}

// Typed prefixes of the SDK's dual interfaces. IDispatch contributes four
// slots after IUnknown; ITextRange contributes 51 more before ITextSelection.
// Uncalled slots retain their ABI width without declaring unused interfaces.
#[repr(C)]
struct TextDocumentVTable {
    _unknown: IUnknownVtbl,
    _dispatch: [*const c_void; 4],
    _get_name: *const c_void,
    get_selection: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
    _get_story_count_through_redo: [*const c_void; 15],
    range: unsafe extern "system" fn(*mut c_void, i32, i32, *mut *mut c_void) -> i32,
}

#[repr(C)]
struct TextSelectionVTable {
    _unknown: IUnknownVtbl,
    _dispatch: [*const c_void; 4],
    get_text: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> i32,
    set_text: unsafe extern "system" fn(*mut c_void, *mut u16) -> i32,
    _get_char_and_set_char: [*const c_void; 2],
    get_duplicate: InterfaceGetter,
    _get_and_set_formatted_text: [*const c_void; 2],
    get_start: Getter,
    set_start: Setter,
    get_end: Getter,
    set_end: Setter,
    get_font: InterfaceGetter,
    _set_font: *const c_void,
    get_para: InterfaceGetter,
    _set_para: *const c_void,
    get_story_length: Getter,
    _get_story_type: *const c_void,
    collapse: Setter,
    _expand_through_set_index: [*const c_void; 3],
    set_range: unsafe extern "system" fn(*mut c_void, i32, i32) -> i32,
    _in_range_through_get_embedded_object: [*const c_void; 29],
    get_flags: Getter,
    set_flags: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    get_type: Getter,
    move_left: MoveMethod,
    move_right: MoveMethod,
    move_up: MoveMethod,
    move_down: MoveMethod,
    home_key: BoundaryMethod,
    end_key: BoundaryMethod,
    type_text: unsafe extern "system" fn(*mut c_void, *mut u16) -> i32,
}

// The SDK ITextRange2 interface inherits ITextSelection, including its ten
// selection slots, even when obtained from an independent document range.
#[repr(C)]
struct TextRange2VTable {
    base: TextSelectionVTable,
    _get_cch_through_set_formatted_text2: [*const c_void; 9],
    get_gravity: Getter,
    set_gravity: Setter,
    _get_para2_through_get_table: [*const c_void; 5],
    get_url: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> i32,
    set_url: unsafe extern "system" fn(*mut c_void, *mut u16) -> i32,
}

#[repr(C)]
struct TextFontVTable {
    _unknown: IUnknownVtbl,
    _dispatch: [*const c_void; 4],
    _get_duplicate_through_set_back_color: [*const c_void; 13],
    get_bold: Getter,
    set_bold: Setter,
    _get_emboss_through_set_engrave: [*const c_void; 8],
    get_italic: Getter,
    set_italic: Setter,
}

#[repr(C)]
struct TextParaVTable {
    _unknown: IUnknownVtbl,
    _dispatch: [*const c_void; 4],
    _get_duplicate_through_set_style: [*const c_void; 7],
    get_alignment: Getter,
    set_alignment: Setter,
}

const _: () = {
    let slot = size_of::<*const c_void>();
    assert!(offset_of!(TextDocumentVTable, get_selection) == 8 * slot);
    assert!(offset_of!(TextDocumentVTable, range) == 24 * slot);
    assert!(offset_of!(TextSelectionVTable, get_text) == 7 * slot);
    assert!(offset_of!(TextSelectionVTable, set_text) == 8 * slot);
    assert!(offset_of!(TextSelectionVTable, get_duplicate) == 11 * slot);
    assert!(offset_of!(TextSelectionVTable, get_start) == 14 * slot);
    assert!(offset_of!(TextSelectionVTable, set_start) == 15 * slot);
    assert!(offset_of!(TextSelectionVTable, get_end) == 16 * slot);
    assert!(offset_of!(TextSelectionVTable, set_end) == 17 * slot);
    assert!(offset_of!(TextSelectionVTable, get_font) == 18 * slot);
    assert!(offset_of!(TextSelectionVTable, get_para) == 20 * slot);
    assert!(offset_of!(TextSelectionVTable, get_story_length) == 22 * slot);
    assert!(offset_of!(TextSelectionVTable, collapse) == 24 * slot);
    assert!(offset_of!(TextSelectionVTable, set_range) == 28 * slot);
    assert!(offset_of!(TextSelectionVTable, get_flags) == 58 * slot);
    assert!(offset_of!(TextSelectionVTable, set_flags) == 59 * slot);
    assert!(offset_of!(TextSelectionVTable, get_type) == 60 * slot);
    assert!(offset_of!(TextSelectionVTable, move_left) == 61 * slot);
    assert!(offset_of!(TextSelectionVTable, move_right) == 62 * slot);
    assert!(offset_of!(TextSelectionVTable, move_up) == 63 * slot);
    assert!(offset_of!(TextSelectionVTable, move_down) == 64 * slot);
    assert!(offset_of!(TextSelectionVTable, home_key) == 65 * slot);
    assert!(offset_of!(TextSelectionVTable, end_key) == 66 * slot);
    assert!(offset_of!(TextSelectionVTable, type_text) == 67 * slot);
    assert!(size_of::<TextSelectionVTable>() == 68 * slot);
    assert!(offset_of!(TextRange2VTable, get_gravity) == 77 * slot);
    assert!(offset_of!(TextRange2VTable, set_gravity) == 78 * slot);
    assert!(offset_of!(TextRange2VTable, get_url) == 84 * slot);
    assert!(offset_of!(TextRange2VTable, set_url) == 85 * slot);
    assert!(offset_of!(TextFontVTable, get_bold) == 20 * slot);
    assert!(offset_of!(TextFontVTable, set_bold) == 21 * slot);
    assert!(offset_of!(TextFontVTable, get_italic) == 30 * slot);
    assert!(offset_of!(TextFontVTable, set_italic) == 31 * slot);
    assert!(offset_of!(TextParaVTable, get_alignment) == 14 * slot);
    assert!(offset_of!(TextParaVTable, set_alignment) == 15 * slot);
};

fn check_hresult(result: i32, operation: &str) -> TestResult {
    if result < 0 {
        Err(format!("{operation}: HRESULT 0x{:08X}", result as u32))
    } else {
        Ok(())
    }
}

fn last_error(operation: &str) -> String {
    // SAFETY: GetLastError reads this thread's last-error value.
    let error = unsafe { GetLastError() };
    format!("{operation}: Win32 error {error}")
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

struct OleApartment;

impl OleApartment {
    fn new() -> TestResult<Self> {
        // SAFETY: the reserved argument is null; the probe owns its thread.
        check_hresult(unsafe { OleInitialize(null()) }, "OleInitialize")?;
        Ok(Self)
    }
}

impl Drop for OleApartment {
    fn drop(&mut self) {
        // SAFETY: exactly balances this thread's successful OleInitialize.
        unsafe { OleUninitialize() };
    }
}

struct SystemRichEditLibrary(HMODULE);

impl SystemRichEditLibrary {
    fn load() -> TestResult<Self> {
        // SAFETY: a terminated name, null reserved handle, documented search flag.
        let module = unsafe {
            LoadLibraryExW(
                wide("Msftedit.dll").as_ptr(),
                null_mut(),
                LOAD_LIBRARY_SEARCH_SYSTEM32,
            )
        };
        if module.is_null() {
            Err(last_error("LoadLibraryExW(Msftedit.dll, System32)"))
        } else {
            Ok(Self(module))
        }
    }

    fn describe(&self) -> TestResult {
        let mut path = vec![0u16; 32768];
        // SAFETY: a loaded module and a writable UTF-16 buffer of the given size.
        let count = unsafe { GetModuleFileNameW(self.0, path.as_mut_ptr(), path.len() as u32) };
        if count == 0 {
            return Err(last_error("GetModuleFileNameW(Msftedit.dll)"));
        }
        if count as usize >= path.len() {
            return Err("Msftedit.dll module path was truncated".into());
        }
        path.truncate(count as usize + 1);
        let version = file_version(&path)?;
        println!(
            "SYSTEM_MSFTEDIT_ENGINE windows={:?} dll={:?} file_version={}.{}.{}.{}",
            rtl_get_version(),
            String::from_utf16_lossy(&path[..count as usize]),
            version.0,
            version.1,
            version.2,
            version.3,
        );
        Ok(())
    }
}

impl Drop for SystemRichEditLibrary {
    fn drop(&mut self) {
        // SAFETY: this owns one LoadLibraryExW reference; all controls are gone.
        unsafe { FreeLibrary(self.0) };
    }
}

fn file_version(path: &[u16]) -> TestResult<(u32, u32, u32, u32)> {
    let mut ignored = 0;
    // SAFETY: path is the terminated module filename returned above.
    let size = unsafe { GetFileVersionInfoSizeW(path.as_ptr(), &mut ignored) };
    if size == 0 {
        return Err(last_error("GetFileVersionInfoSizeW(Msftedit.dll)"));
    }
    // A u32 buffer supplies the alignment of VS_FIXEDFILEINFO as well as its
    // storage. The API's byte count remains the exact requested size.
    let mut data = vec![0u32; (size as usize).div_ceil(size_of::<u32>())];
    // SAFETY: a terminated path and writable storage for at least size bytes.
    if unsafe { GetFileVersionInfoW(path.as_ptr(), 0, size, data.as_mut_ptr().cast()) } == 0 {
        return Err(last_error("GetFileVersionInfoW(Msftedit.dll)"));
    }
    let mut root = null_mut();
    let mut length = 0;
    // SAFETY: a successful version-info block and valid output parameters.
    if unsafe {
        VerQueryValueW(
            data.as_ptr().cast(),
            wide("\\").as_ptr(),
            &mut root,
            &mut length,
        )
    } == 0
        || root.is_null()
        || length < size_of::<VS_FIXEDFILEINFO>() as u32
    {
        return Err("Msftedit.dll has no complete fixed version information".into());
    }
    // SAFETY: VerQueryValueW returned this structure inside the still-live block.
    let info = unsafe { root.cast::<VS_FIXEDFILEINFO>().read_unaligned() };
    if info.dwSignature != 0xfeef04bd {
        return Err("Msftedit.dll fixed version information has an invalid signature".into());
    }
    Ok((
        info.dwFileVersionMS >> 16,
        info.dwFileVersionMS & 0xffff,
        info.dwFileVersionLS >> 16,
        info.dwFileVersionLS & 0xffff,
    ))
}

struct HiddenRichEdit(HWND);

impl HiddenRichEdit {
    fn new() -> TestResult<Self> {
        Self::with_extended_style(0)
    }

    fn with_extended_style(extended_style: u32) -> TestResult<Self> {
        // SAFETY: null names the current executable module and does not AddRef it.
        let instance = unsafe { GetModuleHandleW(null()) };
        if instance.is_null() {
            return Err(last_error("GetModuleHandleW"));
        }
        // SAFETY: MSFTEDIT registered its class when the DLL was loaded. All
        // strings remain live through this synchronous call. The unshown popup
        // has no parent, menu or creation data. AutoHScroll excludes soft wraps.
        let hwnd = unsafe {
            CreateWindowExW(
                extended_style,
                wide("RICHEDIT50W").as_ptr(),
                wide("").as_ptr(),
                WS_POPUP | ES_MULTILINE as u32 | ES_AUTOHSCROLL as u32 | ES_AUTOVSCROLL as u32,
                0,
                0,
                1000,
                500,
                null_mut(),
                null_mut(),
                instance,
                null(),
            )
        };
        if hwnd.is_null() {
            Err(last_error("CreateWindowExW(RICHEDIT50W)"))
        } else {
            Ok(Self(hwnd))
        }
    }

    fn set_text(&self, text: &str) -> TestResult {
        // SAFETY: a live window and a terminated string for this synchronous call.
        if unsafe { SetWindowTextW(self.0, wide(text).as_ptr()) } == 0 {
            Err(last_error("SetWindowTextW(RICHEDIT50W)"))
        } else {
            Ok(())
        }
    }

    fn document(&self) -> TestResult<NativeDocument> {
        let mut ole = null_mut::<c_void>();
        // SAFETY: EM_GETOLEINTERFACE writes one owned interface reference to ole.
        if unsafe {
            SendMessageW(
                self.0,
                EM_GETOLEINTERFACE,
                0,
                (&mut ole as *mut *mut c_void) as isize,
            )
        } == 0
        {
            return Err("EM_GETOLEINTERFACE failed".into());
        }
        // SAFETY: the successful message returns one AddRef'd IRichEditOle,
        // whose first interface slots are IUnknown.
        let ole = unsafe { ComPtr::<IUnknown>::from_raw(ole.cast()) }
            .ok_or("EM_GETOLEINTERFACE returned null")?;
        let mut document = null_mut();
        // SAFETY: an exact SDK IID and a writable interface-pointer output.
        let result = unsafe { ole.query_interface_raw(&IID_TEXT_DOCUMENT, &mut document) };
        check_hresult(result as i32, "QueryInterface(ITextDocument)")?;
        // SAFETY: successful QueryInterface transfers one owned reference.
        unsafe { ComPtr::<IUnknown>::from_raw(document.cast()) }
            .map(NativeDocument)
            .ok_or_else(|| "QueryInterface(ITextDocument) returned null".into())
    }

    fn selection(&self) -> TestResult<NativeSelection> {
        self.document()?.selection()
    }
}

struct NativeDocument(ComPtr<IUnknown>);

impl NativeDocument {
    fn table(&self) -> &TextDocumentVTable {
        // SAFETY: QueryInterface established this owned ITextDocument reference.
        unsafe { &**self.0.as_ptr().cast::<*const TextDocumentVTable>() }
    }

    fn selection(&self) -> TestResult<NativeSelection> {
        let mut selection = null_mut();
        // SAFETY: GetSelection is slot 8 of ITextDocument and writes an owned
        // ITextSelection reference. The document stays alive through the call.
        check_hresult(
            unsafe { (self.table().get_selection)(self.0.as_ptr().cast(), &mut selection) },
            "ITextDocument::GetSelection",
        )?;
        // SAFETY: successful GetSelection transfers one owned reference. The
        // returned selection and the window keep the underlying story alive.
        unsafe { ComPtr::<IUnknown>::from_raw(selection.cast()) }
            .map(NativeSelection)
            .ok_or_else(|| "ITextDocument::GetSelection returned null".into())
    }

    fn range(&self, start: i32, end: i32) -> TestResult<NativeRange2> {
        let mut range = null_mut::<c_void>();
        // SAFETY: the SDK Range slot takes two signed UTF-16 positions and
        // writes an owned ITextRange reference, independent of the selection.
        check_hresult(
            unsafe { (self.table().range)(self.0.as_ptr().cast(), start, end, &mut range) },
            "ITextDocument::Range",
        )?;
        // SAFETY: successful Range transfers one owned interface reference.
        let range = unsafe { ComPtr::<IUnknown>::from_raw(range.cast()) }
            .ok_or("ITextDocument::Range returned null")?;
        let mut range2 = null_mut();
        // SAFETY: the exact SDK IID and writable interface-pointer output.
        check_hresult(
            unsafe { range.query_interface_raw(&IID_TEXT_RANGE2, &mut range2) } as i32,
            "QueryInterface(ITextRange2)",
        )?;
        // SAFETY: successful QueryInterface transfers one owned reference.
        unsafe { ComPtr::<IUnknown>::from_raw(range2.cast()) }
            .map(NativeRange2)
            .ok_or_else(|| "QueryInterface(ITextRange2) returned null".into())
    }
}

impl Drop for HiddenRichEdit {
    fn drop(&mut self) {
        // SAFETY: the window was created and is destroyed on the probe thread.
        unsafe { DestroyWindow(self.0) };
    }
}

struct Bstr(*mut u16);

impl Drop for Bstr {
    fn drop(&mut self) {
        // SAFETY: this owns an allocated or returned BSTR; null is also allowed.
        unsafe { free_bstr(self.0) };
    }
}

impl Bstr {
    fn text(&self) -> TestResult<String> {
        if self.0.is_null() {
            return Ok(String::new());
        }
        // SAFETY: this guard owns the live BSTR through explicit-length decoding.
        // SysStringLen preserves embedded NULs, unlike a terminated-string read.
        let units = unsafe { std::slice::from_raw_parts(self.0, SysStringLen(self.0) as usize) };
        String::from_utf16(units).map_err(|_| "native TOM returned malformed UTF-16".into())
    }
}

#[derive(Clone)]
struct NativeRange2(ComPtr<IUnknown>);

impl NativeRange2 {
    fn table(&self) -> &TextRange2VTable {
        // SAFETY: QueryInterface established this owned SDK ITextRange2.
        unsafe { &**self.0.as_ptr().cast::<*const TextRange2VTable>() }
    }

    fn number(&self, getter: Getter, name: &str) -> TestResult<i32> {
        let mut value = 0;
        // SAFETY: callers use a typed integer getter from this interface.
        check_hresult(unsafe { getter(self.0.as_ptr().cast(), &mut value) }, name)?;
        Ok(value)
    }

    fn state(&self) -> TestResult<RangeState> {
        Ok(RangeState {
            start: self.number(self.table().base.get_start, "ITextRange2::GetStart")?,
            end: self.number(self.table().base.get_end, "ITextRange2::GetEnd")?,
        })
    }

    fn text(&self) -> TestResult<String> {
        let mut text = Bstr(null_mut());
        // SAFETY: GetText writes an owned BSTR, including on an empty range.
        check_hresult(
            unsafe { (self.table().base.get_text)(self.0.as_ptr().cast(), &mut text.0) },
            "ITextRange2::GetText",
        )?;
        text.text()
    }

    fn get_url(&self) -> TestResult<(i32, Option<String>)> {
        let mut url = Bstr(null_mut());
        // SAFETY: the verified SDK slot writes an owned BSTR. Null versus empty
        // and failing HRESULTs are observations, not assumptions of this probe.
        let result = unsafe { (self.table().get_url)(self.0.as_ptr().cast(), &mut url.0) };
        let value = if url.0.is_null() {
            None
        } else {
            Some(url.text()?)
        };
        Ok((result, value))
    }

    fn set_url(&self, value: Option<&str>) -> TestResult<i32> {
        let url = Bstr(value.map_or(null_mut(), string_to_bstr));
        if value.is_some() && url.0.is_null() {
            return Err("SysAllocStringLen for ITextRange2::SetURL failed".into());
        }
        // SAFETY: the SDK slot borrows this possibly-null BSTR synchronously.
        // Invalid syntax/collapsed ranges may fail; retain the exact HRESULT.
        Ok(unsafe { (self.table().set_url)(self.0.as_ptr().cast(), url.0) })
    }

    fn snapshot(
        &self,
        document: &NativeDocument,
        selection: &NativeSelection,
    ) -> TestResult<LinkSnapshot> {
        let range = self.state()?;
        let selection_before = selection.state()?;
        let range_text = self.text()?;
        let story_length = self.number(
            self.table().base.get_story_length,
            "ITextRange2::GetStoryLength",
        )?;
        let story_text = document.range(0, story_length)?.text()?;
        // Reading story metadata through independent ranges must not hide a
        // GetURL endpoint expansion or disturb the actual native selection.
        assert_eq!(
            self.state()?,
            range,
            "snapshot reads retain the observed range"
        );
        assert_eq!(
            selection.state()?,
            selection_before,
            "snapshot reads retain the selection"
        );
        assert!(0 <= range.start && range.start <= range.end && range.end <= story_length);
        let range_text_utf16_length = range_text.encode_utf16().count();
        let story_text_utf16_length = story_text.encode_utf16().count();
        assert!(story_text_utf16_length <= story_length as usize);
        Ok(LinkSnapshot {
            range,
            range_text,
            range_text_utf16_length,
            story_text,
            story_text_utf16_length,
            story_length,
            selection: selection_before,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RangeState {
    start: i32,
    end: i32,
}

struct LinkSnapshot {
    range: RangeState,
    range_text: String,
    range_text_utf16_length: usize,
    story_text: String,
    story_text_utf16_length: usize,
    story_length: i32,
    selection: SelectionState,
}

impl std::fmt::Debug for LinkSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LinkSnapshot")
            .field("range", &self.range)
            .field("range_text", &self.range_text)
            .field("range_text_utf16_length", &self.range_text_utf16_length)
            .field("story_text", &self.story_text)
            .field("story_text_utf16_length", &self.story_text_utf16_length)
            .field("story_length", &self.story_length)
            .field("selection", &self.selection)
            .finish()
    }
}

struct NativeSelection(ComPtr<IUnknown>);

impl NativeSelection {
    fn table(&self) -> &TextSelectionVTable {
        // SAFETY: GetSelection returned this owned ITextSelection reference,
        // whose SDK vtable prefix is checked above. It outlives this borrow.
        unsafe { &**self.0.as_ptr().cast::<*const TextSelectionVTable>() }
    }

    fn number(&self, getter: Getter, name: &str) -> TestResult<i32> {
        let mut value = 0;
        // SAFETY: callers pass a typed integer getter from this interface's table.
        check_hresult(unsafe { getter(self.0.as_ptr().cast(), &mut value) }, name)?;
        Ok(value)
    }

    fn state(&self) -> TestResult<SelectionState> {
        let table = self.table();
        Ok(SelectionState {
            start: self.number(table.get_start, "ITextRange::GetStart")?,
            end: self.number(table.get_end, "ITextRange::GetEnd")?,
            flags: self.number(table.get_flags, "ITextSelection::GetFlags")?,
        })
    }

    fn set_range(&self, anchor: i32, active: i32) -> TestResult {
        self.set_range_result(anchor, active).map(|_| ())
    }

    fn set_range_result(&self, anchor: i32, active: i32) -> TestResult<i32> {
        // SAFETY: the SDK SetRange ABI accepts two signed character positions.
        let result = unsafe { (self.table().set_range)(self.0.as_ptr().cast(), anchor, active) };
        check_hresult(result, "ITextRange::SetRange")?;
        Ok(result)
    }

    fn set_flags(&self, flags: i32) -> TestResult<i32> {
        // SAFETY: slot 59 has the SDK's LONG flags input and HRESULT result.
        let result = unsafe { (self.table().set_flags)(self.0.as_ptr().cast(), flags) };
        check_hresult(result, "ITextSelection::SetFlags")?;
        Ok(result)
    }

    fn type_text(&self, text: &str) -> TestResult<i32> {
        let text = Bstr(string_to_bstr(text));
        if text.0.is_null() {
            return Err("SysAllocStringLen for ITextSelection::TypeText failed".into());
        }
        // SAFETY: slot 67 consumes a borrowed BSTR. The guard retains the exact
        // UTF-16 string through the synchronous call and frees it on all exits.
        let result = unsafe { (self.table().type_text)(self.0.as_ptr().cast(), text.0) };
        check_hresult(result, "ITextSelection::TypeText")?;
        Ok(result)
    }

    fn text(&self) -> TestResult<String> {
        let mut text = Bstr(null_mut());
        // SAFETY: GetText writes an owned BSTR; the guard frees it on all exits.
        check_hresult(
            unsafe { (self.table().get_text)(self.0.as_ptr().cast(), &mut text.0) },
            "ITextRange::GetText",
        )?;
        if text.0.is_null() {
            return Ok(String::new());
        }
        // SAFETY: the successful getter returned a live BSTR. Its explicit
        // length includes embedded NULs, and the guard owns it through decoding.
        let units = unsafe { std::slice::from_raw_parts(text.0, SysStringLen(text.0) as usize) };
        String::from_utf16(units).map_err(|_| "native TOM returned malformed UTF-16".into())
    }

    fn navigate(&self, unit: Unit, extend: bool, end: bool) -> TestResult<(i32, i32)> {
        let method = if end {
            self.table().end_key
        } else {
            self.table().home_key
        };
        let mut delta = i32::MIN;
        // SAFETY: both SDK methods take Unit, Extend and a writable LONG delta.
        let result = unsafe {
            method(
                self.0.as_ptr().cast(),
                unit as i32,
                i32::from(extend),
                &mut delta,
            )
        };
        check_hresult(result, if end { "EndKey" } else { "HomeKey" })?;
        Ok((result, delta))
    }

    fn move_vertical(
        &self,
        unit: Unit,
        count: i32,
        extend: bool,
        down: bool,
    ) -> TestResult<(i32, i32)> {
        let method = if down {
            self.table().move_down
        } else {
            self.table().move_up
        };
        let mut delta = i32::MIN;
        // SAFETY: the SDK methods take Unit, Count, Extend and a writable LONG.
        let result = unsafe {
            method(
                self.0.as_ptr().cast(),
                unit as i32,
                count,
                i32::from(extend),
                &mut delta,
            )
        };
        check_hresult(result, if down { "MoveDown" } else { "MoveUp" })?;
        Ok((result, delta))
    }

    fn move_horizontal(
        &self,
        unit: Unit,
        count: i32,
        extend: bool,
        right: bool,
    ) -> TestResult<(i32, i32)> {
        let method = if right {
            self.table().move_right
        } else {
            self.table().move_left
        };
        let mut delta = i32::MIN;
        // SAFETY: SDK slots 61 and 62 take Unit, Count, Extend and a writable LONG.
        let result = unsafe {
            method(
                self.0.as_ptr().cast(),
                unit as i32,
                count,
                i32::from(extend),
                &mut delta,
            )
        };
        check_hresult(result, if right { "MoveRight" } else { "MoveLeft" })?;
        Ok((result, delta))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelectionState {
    start: i32,
    end: i32,
    flags: i32,
}

impl SelectionState {
    fn directed(self) -> (i32, i32) {
        if self.flags & TOM_SEL_START_ACTIVE != 0 {
            (self.end, self.start)
        } else {
            (self.start, self.end)
        }
    }
}

// These are Win32 TOM constants, not the differently numbered WinUI enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
enum Unit {
    Character = 1,
    Word = 2,
    Paragraph = 4,
    Line = 5,
    Story = 6,
}

fn observe(
    selection: &NativeSelection,
    fixture: &str,
    text_length: i32,
    story_length: i32,
    anchor: i32,
    active: i32,
    unit: Unit,
    extend: bool,
    end: bool,
) -> TestResult {
    selection.set_range(anchor, active)?;
    let before = selection.state()?;
    assert_eq!(
        before.directed(),
        (anchor, active),
        "{fixture}: native fixture direction"
    );
    let (result, delta) = selection.navigate(unit, extend, end)?;
    let after = selection.state()?;
    let (accepted_anchor, accepted_active) = after.directed();
    println!(
        "SYSTEM_MSFTEDIT_HOME_END fixture={fixture} supplied_utf16_length={text_length} native_story_length={story_length} unit={unit:?} extend={extend} operation={} before={before:?} after={after:?} anchor={accepted_anchor} active={accepted_active} delta={delta} active_displacement={} hresult=0x{:08X}",
        if end { "End" } else { "Home" },
        accepted_active - active,
        result as u32,
    );
    assert!(0 <= after.start && after.start <= after.end && after.end <= story_length);
    if extend {
        assert_eq!(
            accepted_anchor, anchor,
            "extension retains the native anchor"
        );
    } else {
        assert_eq!(after.start, after.end, "collapse leaves a native caret");
    }
    if unit == Unit::Story && !end {
        assert_eq!(accepted_active, 0);
    }
    // A second identical command is an unambiguous no-op in these unwrapped
    // fixtures. Record S_OK/S_FALSE and flags as well as the expected zero delta.
    let (repeat_result, repeat_delta) = selection.navigate(unit, extend, end)?;
    let repeated = selection.state()?;
    println!(
        "SYSTEM_MSFTEDIT_HOME_END_REPEAT fixture={fixture} unit={unit:?} extend={extend} operation={} after={repeated:?} delta={repeat_delta} hresult=0x{:08X}",
        if end { "End" } else { "Home" },
        repeat_result as u32,
    );
    assert_eq!(repeated.directed(), after.directed());
    assert_eq!(repeat_delta, 0);
    Ok(())
}

fn characterize() -> TestResult {
    // Locals drop in reverse declaration order: selection -> HWND -> DLL -> OLE,
    // including any early return or assertion failure.
    let _apartment = OleApartment::new()?;
    let library = SystemRichEditLibrary::load()?;
    library.describe()?;
    let window = HiddenRichEdit::new()?;
    let selection = window.selection()?;
    for (name, text, ranges) in [
        ("multiline", "abc\rdefg\rhij", &[(1, 6), (6, 1)][..]),
        ("utf16", "A😀B\rxy", &[(1, 6), (6, 1)][..]),
        ("empty_lines", "ab\r\rxy\r", &[(3, 3), (7, 7)][..]),
        ("empty_story", "", &[(0, 0)][..]),
    ] {
        window.set_text(text)?;
        let text_length = text.encode_utf16().count() as i32;
        selection.set_range(0, text_length)?;
        assert_eq!(
            selection.text()?,
            text,
            "{name}: native text was installed exactly"
        );
        let story_length =
            selection.number(selection.table().get_story_length, "GetStoryLength")?;
        println!(
            "SYSTEM_MSFTEDIT_FIXTURE name={name} text={text:?} utf16_length={text_length} native_story_length={story_length}"
        );
        assert!(story_length >= text_length);
        for &(anchor, active) in ranges {
            for unit in [Unit::Line, Unit::Story] {
                for extend in [false, true] {
                    for end in [false, true] {
                        observe(
                            &selection,
                            name,
                            text_length,
                            story_length,
                            anchor,
                            active,
                            unit,
                            extend,
                            end,
                        )?;
                    }
                }
            }
        }
    }
    // Explicit boundary inputs supplement the idempotence checks above.
    window.set_text("abc\rdefg\rhij")?;
    let story_length = selection.number(selection.table().get_story_length, "GetStoryLength")?;
    println!(
        "SYSTEM_MSFTEDIT_FIXTURE name=boundary text=\"abc\\rdefg\\rhij\" utf16_length=12 native_story_length={story_length}"
    );
    for (unit, end, position) in [
        (Unit::Line, false, 0),
        (Unit::Line, true, 3),
        (Unit::Story, false, 0),
        (Unit::Story, true, 12),
    ] {
        observe(
            &selection,
            "boundary",
            12,
            story_length,
            position,
            position,
            unit,
            false,
            end,
        )?;
    }
    println!("SYSTEM_MSFTEDIT_PROBE_COMPLETE commands=60 repeated_noops=60");
    Ok(())
}

#[test]
fn rich_text_tom_oracle_system_msftedit_home_end_characterization() {
    // A dedicated thread makes apartment ownership independent of other tests
    // on a reused harness worker. No window is shown and no focus is requested.
    std::thread::Builder::new()
        .name("system-msftedit-tom".into())
        .spawn(characterize)
        .expect("start the native TOM probe thread")
        .join()
        .expect("the native TOM probe thread panicked")
        .expect("the native TOM probe failed");
}

fn observe_vertical(
    selection: &NativeSelection,
    fixture: &str,
    story_length: i32,
    unit: Unit,
    count: i32,
    extend: bool,
    down: bool,
) -> TestResult {
    let before = selection.state()?;
    let (anchor, active) = before.directed();
    let (result, delta) = selection.move_vertical(unit, count, extend, down)?;
    let after = selection.state()?;
    let (accepted_anchor, accepted_active) = after.directed();
    println!(
        "SYSTEM_MSFTEDIT_VERTICAL fixture={fixture} unit={unit:?} count={count} extend={extend} operation={} before={before:?} after={after:?} anchor={accepted_anchor} active={accepted_active} delta={delta} active_displacement={} hresult=0x{:08X}",
        if down { "Down" } else { "Up" },
        accepted_active - active,
        result as u32,
    );
    assert!(0 <= after.start && after.start <= after.end && after.end <= story_length);
    if extend {
        assert_eq!(
            accepted_anchor, anchor,
            "extension retains the native anchor"
        );
    } else if count != 0 {
        assert_eq!(after.start, after.end, "movement leaves a native caret");
    }
    // Count=0, collapse-edge choice, signed unit counts, paragraph transitions,
    // boundary no-ops and desired-column retention are observations. Do not
    // turn the initial run into assertions derived from the Rust implementation.
    Ok(())
}

/// Setup diagnostics are retained in the raw native log, outside the strict
/// SYSTEM_MSFTEDIT_ movement-record schema. Reads must not move the selection.
/// Keep their individual failures visible without replacing the exact text
/// assertion immediately following this diagnostic.
fn describe_vertical_reset(
    window: &HiddenRichEdit,
    selection: &NativeSelection,
    fixture: &str,
    case: usize,
    supplied_length: i32,
    set_range_result: i32,
) {
    let before = selection.state();
    let selected_text = selection.text();
    let story_length = selection.number(selection.table().get_story_length, "GetStoryLength");
    let independent = window.document().and_then(|document| {
        let range = document.range(0, supplied_length)?;
        Ok((range.state()?, range.text()?))
    });
    let fresh_selection = window
        .selection()
        .and_then(|fresh| Ok((fresh.state()?, fresh.text()?)));
    let after = selection.state();
    println!(
        "MSFTEDIT_VERTICAL_SETUP_DIAGNOSTIC fixture={fixture} case={case} supplied_utf16_length={supplied_length} set_range_hresult=0x{:08X} selection={before:?} selected_text={selected_text:?} native_story_length={story_length:?} independent_range={independent:?} fresh_selection={fresh_selection:?} selection_after_reads={after:?}",
        set_range_result as u32,
    );
    if let (Ok(before), Ok(after)) = (before, after) {
        assert_eq!(
            before, after,
            "{fixture}: setup diagnostics retain the native selection"
        );
    }
}

fn characterize_vertical() -> TestResult {
    // Same ownership order as the Home/End probe; all COM work stays on this
    // thread and no HWND is shown or focused.
    let _apartment = OleApartment::new()?;
    let library = SystemRichEditLibrary::load()?;
    library.describe()?;
    let mut commands = 0;
    for (name, text, ranges) in [
        (
            "vertical_multiline",
            "abcdef\rxy\rabcdef\rpqrs",
            &[(2, 12), (12, 2), (5, 5)][..],
        ),
        ("vertical_utf16", "A😀B\rxy\rA😀B", &[(1, 6), (6, 1)][..]),
        ("vertical_empty_lines", "ab\r\rxy\r", &[(3, 3), (7, 7)][..]),
        ("vertical_empty_story", "", &[(0, 0)][..]),
    ] {
        // Independent fixtures must not inherit navigation or selection state
        // from a prior WM_SETTEXT on another story. The command ordering within
        // this fixture remains intentional and unchanged.
        let window = HiddenRichEdit::new()?;
        window.set_text(text)?;
        let selection = window.selection()?;
        let length = text.encode_utf16().count() as i32;
        let reset = selection.set_range_result(0, length)?;
        describe_vertical_reset(&window, &selection, name, 0, length, reset);
        assert_eq!(selection.text()?, text, "{name}: exact native setup");
        let story_length =
            selection.number(selection.table().get_story_length, "GetStoryLength")?;
        println!(
            "SYSTEM_MSFTEDIT_VERTICAL_FIXTURE name={name} text={text:?} supplied_utf16_length={length} native_story_length={story_length}"
        );
        for &(anchor, active) in ranges {
            for unit in [Unit::Line, Unit::Paragraph] {
                for extend in [false, true] {
                    for down in [false, true] {
                        for count in [0, 1, 2, -1] {
                            selection.set_range(anchor, active)?;
                            assert_eq!(selection.state()?.directed(), (anchor, active));
                            observe_vertical(
                                &selection,
                                name,
                                story_length,
                                unit,
                                count,
                                extend,
                                down,
                            )?;
                            commands += 1;
                        }
                    }
                }
            }
        }
    }
    {
        let text = "abcdef\rx\rabcdef";
        let window = HiddenRichEdit::new()?;
        window.set_text(text)?;
        let selection = window.selection()?;
        let length = text.encode_utf16().count() as i32;
        let reset = selection.set_range_result(0, length)?;
        describe_vertical_reset(&window, &selection, "retained_column", 0, length, reset);
        assert_eq!(
            selection.text()?,
            text,
            "column fixture: exact native setup"
        );
        let story_length = selection.number(selection.table().get_story_length, "GetStoryLength")?;
        println!(
            "SYSTEM_MSFTEDIT_VERTICAL_FIXTURE name=retained_column text={text:?} supplied_utf16_length={} native_story_length={story_length}",
            text.encode_utf16().count()
        );
        // Unlike matrix rows, these calls deliberately retain native navigation state.
        selection.set_range(5, 5)?;
        for down in [true, true, false, false] {
            observe_vertical(
                &selection,
                "retained_column",
                story_length,
                Unit::Line,
                1,
                false,
                down,
            )?;
            commands += 1;
        }
    }
    {
        let text = "abcdef\rx\rabcdef";
        let window = HiddenRichEdit::new()?;
        window.set_text(text)?;
        let selection = window.selection()?;
        let length = text.encode_utf16().count() as i32;
        let reset = selection.set_range_result(0, length)?;
        describe_vertical_reset(&window, &selection, "vertical_boundary", 0, length, reset);
        assert_eq!(selection.text()?, text, "boundary fixture: exact native setup");
        let story_length = selection.number(selection.table().get_story_length, "GetStoryLength")?;
        println!(
            "SYSTEM_MSFTEDIT_VERTICAL_FIXTURE name=vertical_boundary text={text:?} supplied_utf16_length={} native_story_length={story_length}",
            text.encode_utf16().count()
        );
        for (position, down) in [(0, false), (text.encode_utf16().count() as i32, true)] {
            for unit in [Unit::Line, Unit::Paragraph] {
                selection.set_range(position, position)?;
                observe_vertical(
                    &selection,
                    "vertical_boundary",
                    story_length,
                    unit,
                    1,
                    false,
                    down,
                )?;
                commands += 1;
            }
        }
    }
    commands += characterize_isolated_vertical()?;
    println!("SYSTEM_MSFTEDIT_VERTICAL_PROBE_COMPLETE commands={commands}");
    Ok(())
}

/// SetRange resets the endpoints, but the first matrix exposed column/EOL
/// observations which may retain other native navigation state. Give each of
/// those commands a fresh HWND, then separately observe intentional sequences.
/// The caller keeps the OLE apartment and System32 library alive throughout.
fn characterize_isolated_vertical() -> TestResult<usize> {
    let mut commands = 0;
    for (name, text, cases) in [
        (
            "vertical_isolated_multiline",
            "abcdef\rxy\rabcdef\rpqrs",
            &[
                (5, 5, true, 2, false),
                (5, 5, true, 2, true),
                (5, 5, false, -2, false),
                (5, 5, false, -2, true),
                (2, 12, true, 1, false),
                (12, 2, false, 1, false),
            ][..],
        ),
        (
            "vertical_isolated_empty_lines",
            "ab\r\rxy\r",
            &[
                (7, 7, false, 1, true),
                (7, 7, true, -1, true),
                (7, 7, false, 2, true),
                (7, 7, true, -2, true),
            ][..],
        ),
    ] {
        for (index, &(anchor, active, down, count, extend)) in cases.iter().enumerate() {
            let window = HiddenRichEdit::new()?;
            window.set_text(text)?;
            let selection = window.selection()?;
            let length = text.encode_utf16().count() as i32;
            let reset = selection.set_range_result(0, length)?;
            describe_vertical_reset(&window, &selection, name, index, length, reset);
            assert_eq!(selection.text()?, text, "{name}: exact isolated setup");
            let story_length =
                selection.number(selection.table().get_story_length, "GetStoryLength")?;
            if index == 0 {
                println!(
                    "SYSTEM_MSFTEDIT_VERTICAL_FIXTURE name={name} text={text:?} supplied_utf16_length={length} native_story_length={story_length}"
                );
            }
            selection.set_range(anchor, active)?;
            assert_eq!(selection.state()?.directed(), (anchor, active));
            observe_vertical(
                &selection,
                name,
                story_length,
                Unit::Line,
                count,
                extend,
                down,
            )?;
            commands += 1;
        }
    }
    let text = "abcdef\rx\rabcdef";
    for (name, extend) in [
        ("vertical_isolated_column_collapse", false),
        ("vertical_isolated_column_extend", true),
    ] {
        let window = HiddenRichEdit::new()?;
        window.set_text(text)?;
        let selection = window.selection()?;
        let length = text.encode_utf16().count() as i32;
        let reset = selection.set_range_result(0, length)?;
        describe_vertical_reset(&window, &selection, name, 0, length, reset);
        assert_eq!(selection.text()?, text, "{name}: exact isolated setup");
        let story_length =
            selection.number(selection.table().get_story_length, "GetStoryLength")?;
        println!(
            "SYSTEM_MSFTEDIT_VERTICAL_FIXTURE name={name} text={text:?} supplied_utf16_length={length} native_story_length={story_length}"
        );
        selection.set_range(5, 5)?;
        for down in [true, true, false, false] {
            observe_vertical(&selection, name, story_length, Unit::Line, 1, extend, down)?;
            commands += 1;
        }
    }
    // Switching between collapsed and extended movement distinguishes shared
    // column state from the per-command column used by continuous extension.
    for (name, steps) in [
        (
            "vertical_isolated_column_collapse_then_extend",
            [(true, false), (true, true), (false, true), (false, false)],
        ),
        (
            "vertical_isolated_column_extend_then_collapse",
            [(true, true), (true, false), (false, false), (false, true)],
        ),
    ] {
        let window = HiddenRichEdit::new()?;
        window.set_text(text)?;
        let selection = window.selection()?;
        let length = text.encode_utf16().count() as i32;
        let reset = selection.set_range_result(0, length)?;
        describe_vertical_reset(&window, &selection, name, 0, length, reset);
        assert_eq!(selection.text()?, text, "{name}: exact mixed-column setup");
        let story_length =
            selection.number(selection.table().get_story_length, "GetStoryLength")?;
        println!(
            "SYSTEM_MSFTEDIT_VERTICAL_FIXTURE name={name} text={text:?} supplied_utf16_length={length} native_story_length={story_length}"
        );
        selection.set_range(5, 5)?;
        for (down, extend) in steps {
            observe_vertical(&selection, name, story_length, Unit::Line, 1, extend, down)?;
            commands += 1;
        }
    }
    Ok(commands)
}

#[test]
fn rich_text_tom_oracle_system_msftedit_vertical_characterization() {
    std::thread::Builder::new()
        .name("system-msftedit-vertical".into())
        .spawn(characterize_vertical)
        .expect("start the native vertical TOM probe thread")
        .join()
        .expect("the native vertical TOM probe thread panicked")
        .expect("the native vertical TOM probe failed");
}

#[derive(Clone, Copy)]
struct HorizontalCase {
    anchor: i32,
    active: i32,
    unit: Unit,
    count: i32,
    extend: bool,
    right: bool,
}

fn horizontal_cases(ranges: &[(i32, i32)], units: &[Unit], counts: &[i32]) -> Vec<HorizontalCase> {
    let mut cases = Vec::new();
    for &(anchor, active) in ranges {
        for &unit in units {
            for extend in [false, true] {
                for right in [false, true] {
                    for &count in counts {
                        cases.push(HorizontalCase {
                            anchor,
                            active,
                            unit,
                            count,
                            extend,
                            right,
                        });
                    }
                }
            }
        }
    }
    cases
}

fn characterize_horizontal_fixture(
    name: &str,
    text: &str,
    window_ex_style: u32,
    cases: &[HorizontalCase],
) -> TestResult<usize> {
    let length = text.encode_utf16().count() as i32;
    for (index, case) in cases.iter().enumerate() {
        // Every command starts with a fresh control. SetRange alone does not
        // reset every native caret/column flag, as the vertical probe showed.
        let window = HiddenRichEdit::with_extended_style(window_ex_style)?;
        window.set_text(text)?;
        let selection = window.selection()?;
        selection.set_range(0, length)?;
        assert_eq!(selection.text()?, text, "{name}: exact native setup");
        let story_length =
            selection.number(selection.table().get_story_length, "GetStoryLength")?;
        assert!(story_length >= length);
        if index == 0 {
            println!(
                "SYSTEM_MSFTEDIT_HORIZONTAL_FIXTURE name={name} text={text:?} supplied_utf16_length={length} native_story_length={story_length} window_ex_style={window_ex_style}"
            );
        }
        selection.set_range(case.anchor, case.active)?;
        let before = selection.state()?;
        assert_eq!(
            before.directed(),
            (case.anchor, case.active),
            "{name}: native fixture direction"
        );
        let before_type =
            selection.number(selection.table().get_type, "ITextSelection::GetType")?;
        let (result, delta) =
            selection.move_horizontal(case.unit, case.count, case.extend, case.right)?;
        let after = selection.state()?;
        let after_type = selection.number(selection.table().get_type, "ITextSelection::GetType")?;
        let (accepted_anchor, accepted_active) = after.directed();
        println!(
            "SYSTEM_MSFTEDIT_HORIZONTAL fixture={name} unit={:?} count={} extend={} operation={} before={before:?} before_type={before_type} after={after:?} after_type={after_type} anchor={accepted_anchor} active={accepted_active} delta={delta} active_displacement={} hresult=0x{:08X}",
            case.unit,
            case.count,
            case.extend,
            if case.right { "Right" } else { "Left" },
            accepted_active - case.active,
            result as u32,
        );
        assert!(0 <= after.start && after.start <= after.end && after.end <= story_length);
        if case.extend {
            assert_eq!(
                accepted_anchor, case.anchor,
                "extension retains the native anchor"
            );
        } else if case.count != 0 {
            assert_eq!(after.start, after.end, "movement leaves a native caret");
        }
        // Capture all observable navigation state before this independent
        // verification changes the selection. Movement must not edit the story.
        selection.set_range(0, length)?;
        assert_eq!(
            selection.text()?,
            text,
            "{name}: navigation preserves native text"
        );
        assert_eq!(
            selection.number(selection.table().get_story_length, "GetStoryLength")?,
            story_length
        );
    }
    Ok(cases.len())
}

fn characterize_horizontal() -> TestResult {
    // Selection and HWND guards are local to each case; the SDK apartment and
    // System32 library outlive all of them, including early failure/unwind.
    let _apartment = OleApartment::new()?;
    let library = SystemRichEditLibrary::load()?;
    library.describe()?;
    let mut commands = 0;
    let units = [Unit::Character, Unit::Word];

    // Forward/reversed ranges distinguish active-end from directional collapse;
    // 0, 1, 2 and negative counts expose collapse accounting and signed results.
    let mut cases = horizontal_cases(&[(1, 6), (6, 1)], &units, &[0, 1, 2, -1]);
    // Aligned word endpoints distinguish directional movement from a collapse
    // which consumes the first unit even when no boundary adjustment is needed.
    cases.extend(horizontal_cases(
        &[(0, 4), (4, 8), (4, 0), (8, 4)],
        &[Unit::Word],
        &[-2, -1, 1, 2],
    ));
    cases.extend(horizontal_cases(&[(5, 5)], &units, &[1, -1]));
    for (position, right) in [(0, false), (13, true)] {
        for unit in units {
            for extend in [false, true] {
                cases.push(HorizontalCase {
                    anchor: position,
                    active: position,
                    unit,
                    count: 1,
                    extend,
                    right,
                });
            }
        }
    }
    commands += characterize_horizontal_fixture("horizontal_ascii", "one two three", 0, &cases)?;

    // UTF-16 scalar boundaries, a position inside a combining sequence, and an
    // emoji ZWJ sequence distinguish Character keyboard movement from scalar,
    // code-unit and grapheme movement. Their destinations remain observations.
    let mut grapheme_cases = horizontal_cases(
        &[(1, 1), (3, 3), (4, 4), (5, 5), (13, 13)],
        &[Unit::Character],
        &[1],
    );
    grapheme_cases.extend(horizontal_cases(
        &[(1, 4), (4, 1)],
        &[Unit::Character],
        &[1, 2],
    ));
    commands += characterize_horizontal_fixture(
        "horizontal_graphemes",
        "A😀e\u{301}👩\u{200d}👩\u{200d}👦B",
        0,
        &grapheme_cases,
    )?;
    commands += characterize_horizontal_fixture(
        "horizontal_word_boundaries",
        "one, two\tthree\r四五 six",
        0,
        &horizontal_cases(
            &[(3, 3), (4, 4), (8, 8), (14, 14), (15, 15), (17, 17)],
            &[Unit::Word],
            &[1],
        ),
    )?;

    // The SDK documents WS_EX_RTLREADING as enabling Rich Edit bidi processing.
    // Record the window style, not an inferred paragraph-direction property.
    // Mixed runs and reversed cross-run selections expose logical/visual order.
    let bidi = horizontal_cases(&[(4, 4), (5, 5), (7, 7), (2, 6), (6, 2)], &units, &[1]);
    for (name, window_ex_style) in [
        ("horizontal_bidi_default", 0),
        ("horizontal_bidi_rtl_reading", WS_EX_RTLREADING),
    ] {
        commands += characterize_horizontal_fixture(name, "abc אבג xyz", window_ex_style, &bidi)?;
    }
    commands += characterize_horizontal_fixture(
        "horizontal_empty_story",
        "",
        0,
        &horizontal_cases(&[(0, 0)], &units, &[1]),
    )?;
    assert_eq!(
        commands, 300,
        "keep the deterministic characterization matrix complete"
    );
    println!("SYSTEM_MSFTEDIT_HORIZONTAL_PROBE_COMPLETE commands={commands}");
    Ok(())
}

#[test]
fn rich_text_tom_oracle_system_msftedit_horizontal_characterization() {
    std::thread::Builder::new()
        .name("system-msftedit-horizontal".into())
        .spawn(characterize_horizontal)
        .expect("start the native horizontal TOM probe thread")
        .join()
        .expect("the native horizontal TOM probe thread panicked")
        .expect("the native horizontal TOM probe failed");
}

/// Flags are observed on fresh, unshown windows. In particular, requesting
/// tomSelActive here is not evidence of a focused WinUI input session. The
/// TypeText result is captured before reading the complete native story, which
/// itself moves the selection and can include the synthetic terminal EOP.
fn characterize_options_fixture(
    name: &str,
    text: &str,
    ranges: &[(i32, i32)],
    flags: &[i32],
    typed: &str,
) -> TestResult<usize> {
    let mut commands = 0;
    let length = text.encode_utf16().count() as i32;
    for &(anchor, active) in ranges {
        for &requested_flags in flags {
            let window = HiddenRichEdit::new()?;
            window.set_text(text)?;
            let selection = window.selection()?;
            selection.set_range(0, length)?;
            assert_eq!(selection.text()?, text, "{name}: exact options setup");
            let story_length =
                selection.number(selection.table().get_story_length, "GetStoryLength")?;
            if commands == 0 {
                println!(
                    "SYSTEM_MSFTEDIT_OPTIONS_FIXTURE name={name} text={text:?} supplied_utf16_length={length} native_story_length={story_length} window_ex_style=0"
                );
            }
            selection.set_range(anchor, active)?;
            let before = selection.state()?;
            assert_eq!(before.directed(), (anchor, active));
            let before_type =
                selection.number(selection.table().get_type, "ITextSelection::GetType")?;
            let set_result = selection.set_flags(requested_flags)?;
            let after_set = selection.state()?;
            let after_set_type =
                selection.number(selection.table().get_type, "ITextSelection::GetType")?;
            assert_eq!(
                selection.number(selection.table().get_story_length, "GetStoryLength")?,
                story_length,
                "setting flags preserves the story length"
            );
            let type_result = selection.type_text(typed)?;
            let after = selection.state()?;
            let after_type =
                selection.number(selection.table().get_type, "ITextSelection::GetType")?;
            let story_length_after =
                selection.number(selection.table().get_story_length, "GetStoryLength")?;
            assert!(after_set.end <= story_length && after.end <= story_length_after);

            selection.set_range(0, story_length_after)?;
            let story_text = selection.text()?;
            let story_text_length = story_text.encode_utf16().count();
            assert!(story_text_length <= story_length_after as usize);
            println!(
                "SYSTEM_MSFTEDIT_OPTIONS fixture={name} requested_flags={requested_flags} before={before:?} before_type={before_type} after_set={after_set:?} after_set_type={after_set_type} set_hresult=0x{:08X} typed={typed:?} after={after:?} after_type={after_type} type_hresult=0x{:08X} native_story_length_after={story_length_after} story_text={story_text:?} story_text_utf16_length={story_text_length}",
                set_result as u32, type_result as u32,
            );
            commands += 1;
        }
    }
    Ok(commands)
}

fn characterize_options() -> TestResult {
    let _apartment = OleApartment::new()?;
    let library = SystemRichEditLibrary::load()?;
    library.describe()?;
    // All combinations of the five documented bits (1, 2, 4, 8, 16), without
    // presuming the native getter accepts every requested writable flag.
    let all_flags: Vec<_> = (0..32).collect();
    let mut commands = characterize_options_fixture(
        "options_ascii",
        "abcdef",
        &[(2, 2), (1, 4), (4, 1)],
        &all_flags,
        "XY",
    )?;
    commands += characterize_options_fixture(
        "options_unicode",
        "A😀e\u{301}👩\u{200d}👩\u{200d}👦B",
        &[(1, 1), (3, 3), (4, 4), (5, 5), (13, 13)],
        &[0, 4, 16, 20],
        "Z",
    )?;
    commands += characterize_options_fixture(
        "options_paragraph",
        "ab\rcd",
        &[(2, 2), (3, 3)],
        &[0, 2, 4, 6, 16, 18, 20, 22],
        "Z",
    )?;
    commands += characterize_options_fixture(
        "options_empty_input",
        "abcdef",
        &[(1, 4), (4, 1)],
        &[0, 4, 16, 20],
        "",
    )?;
    // Keep inserted UTF-16 scalars, grapheme clusters and paragraph breaks
    // distinct. The original ASCII matrix does not establish how a multi-unit
    // TypeText call consumes following text in overtype mode.
    for (name, typed) in [
        ("options_typed_emoji", "😀"),
        ("options_typed_combining", "e\u{301}"),
        ("options_typed_zwj", "👩\u{200d}👩\u{200d}👦"),
        ("options_typed_cr", "\r"),
        ("options_typed_crlf", "\r\n"),
        ("options_typed_multiline", "X\rY"),
        ("options_typed_leading_mark", "\u{301}Z"),
    ] {
        commands += characterize_options_fixture(
            name,
            "abcdef",
            &[(1, 1), (2, 2), (1, 4), (4, 1)],
            &[0, 4, 20],
            typed,
        )?;
    }
    for (name, typed) in [
        ("options_paragraph_typed_ascii", "XY"),
        ("options_paragraph_typed_multiline", "X\rY"),
        ("options_paragraph_typed_cr", "\r"),
        ("options_paragraph_typed_crlf", "\r\n"),
        ("options_paragraph_typed_unicode", "😀e\u{301}"),
    ] {
        commands += characterize_options_fixture(
            name,
            "ab\rcd",
            &[(1, 1), (2, 2), (3, 3), (1, 4), (4, 1)],
            &[4, 20],
            typed,
        )?;
    }
    for (name, typed) in [
        ("options_unicode_typed_ascii", "XY"),
        ("options_unicode_typed_emoji", "😀"),
        ("options_unicode_typed_combining", "e\u{301}"),
    ] {
        commands += characterize_options_fixture(
            name,
            "A😀e\u{301}👩\u{200d}👩\u{200d}👦B",
            &[(1, 1), (3, 3), (4, 4), (5, 5), (13, 13), (1, 5), (5, 1)],
            &[4, 20],
            typed,
        )?;
    }
    assert_eq!(commands, 316, "keep the flags/typing matrix complete");
    println!("SYSTEM_MSFTEDIT_OPTIONS_PROBE_COMPLETE commands={commands}");
    Ok(())
}

#[test]
fn rich_text_tom_oracle_system_msftedit_options_characterization() {
    std::thread::Builder::new()
        .name("system-msftedit-options".into())
        .spawn(characterize_options)
        .expect("start the native options TOM probe thread")
        .join()
        .expect("the native options TOM probe thread panicked")
        .expect("the native options TOM probe failed");
}

#[derive(Clone, Copy)]
enum LinkOperation<'a> {
    Get,
    Set(Option<&'a str>),
}

#[derive(Clone, Copy)]
struct LinkSetup<'a> {
    range: (i32, i32),
    url: &'a str,
}

fn observe_link_step(
    name: &str,
    step: usize,
    role: &str,
    initial_range: (i32, i32),
    operation: LinkOperation<'_>,
    range: &NativeRange2,
    document: &NativeDocument,
    selection: &NativeSelection,
) -> TestResult<i32> {
    let before = range.snapshot(document, selection)?;
    let (method, argument, result, value) = match operation {
        LinkOperation::Get => {
            let (result, value) = range.get_url()?;
            ("Get", None, result, value)
        }
        LinkOperation::Set(argument) => ("Set", argument, range.set_url(argument)?, None),
    };
    let after = range.snapshot(document, selection)?;
    let argument_null = argument.is_none();
    let argument = argument.unwrap_or_default();
    let argument_utf16_length = argument.encode_utf16().count();
    let result_null = value.is_none();
    let value = value.unwrap_or_default();
    let result_utf16_length = value.encode_utf16().count();
    println!(
        "SYSTEM_MSFTEDIT_LINK fixture={name} step={step} role={role} operation={method} initial_start={} initial_end={} argument_null={argument_null} argument={argument:?} argument_utf16_length={argument_utf16_length} before={before:?} after={after:?} hresult=0x{:08X} result_null={result_null} result={value:?} result_utf16_length={result_utf16_length}",
        initial_range.0, initial_range.1, result as u32,
    );
    Ok(result)
}

/// Every fixture owns a fresh HWND and independent ITextDocument::Range objects.
/// All target/setup ranges are created before any SetURL, so native range
/// tracking, including any hidden address positions, is preserved and observed.
/// A getter runs on its own range; no preliminary GetURL changes its endpoints.
fn characterize_link_fixture(
    name: &str,
    text: &str,
    setup: &[LinkSetup<'_>],
    initial_range: (i32, i32),
    operation: LinkOperation<'_>,
) -> TestResult<usize> {
    let window = HiddenRichEdit::new()?;
    window.set_text(text)?;
    let document = window.document()?;
    let selection = document.selection()?;
    let length = text.encode_utf16().count() as i32;
    let initial = document.range(0, length)?;
    assert_eq!(initial.text()?, text, "{name}: exact Link fixture setup");
    let story_length = initial.number(initial.table().base.get_story_length, "GetStoryLength")?;
    println!(
        "SYSTEM_MSFTEDIT_LINK_FIXTURE name={name} text={text:?} supplied_utf16_length={length} native_story_length={story_length} window_ex_style=0"
    );
    let mut setup_ranges = Vec::with_capacity(setup.len());
    for item in setup {
        let range = document.range(item.range.0, item.range.1)?;
        assert_eq!(
            range.state()?,
            RangeState {
                start: item.range.0,
                end: item.range.1
            }
        );
        setup_ranges.push(range);
    }
    let target = document.range(initial_range.0, initial_range.1)?;
    assert_eq!(
        target.state()?,
        RangeState {
            start: initial_range.0,
            end: initial_range.1
        }
    );
    let mut step = 0;
    for (item, range) in setup.iter().zip(&setup_ranges) {
        let result = observe_link_step(
            name,
            step,
            "Setup",
            item.range,
            LinkOperation::Set(Some(item.url)),
            range,
            &document,
            &selection,
        )?;
        // These are documented, nonempty, quoted setup links. The target call
        // deliberately records failures for malformed/empty/collapsed inputs.
        check_hresult(result, "ITextRange2::SetURL fixture setup")?;
        step += 1;
    }
    observe_link_step(
        name,
        step,
        "Target",
        initial_range,
        operation,
        &target,
        &document,
        &selection,
    )?;
    step += 1;
    if matches!(operation, LinkOperation::Set(_)) {
        observe_link_step(
            name,
            step,
            "Readback",
            initial_range,
            LinkOperation::Get,
            &target,
            &document,
            &selection,
        )?;
        step += 1;
    }
    Ok(step)
}

fn characterize_links() -> TestResult {
    let _apartment = OleApartment::new()?;
    let library = SystemRichEditLibrary::load()?;
    library.describe()?;
    const A: &str = "\"https://a.example/path\"";
    const B: &str = "\"https://b.example/other\"";
    let mut commands = 0;
    let mut fixtures = 0;
    let mut observe = |name: &str, text: &str, setup: &[LinkSetup<'_>], range, operation| {
        let observed = characterize_link_fixture(name, text, setup, range, operation)?;
        fixtures += 1;
        commands += observed;
        Ok::<_, String>(())
    };

    for (name, range) in [
        ("caret_start", (0, 0)),
        ("character", (0, 1)),
        ("word", (0, 5)),
        ("whole", (0, 10)),
        ("caret_inside", (2, 2)),
        ("terminal_eop", (10, 11)),
    ] {
        observe(
            &format!("link_plain_get_{name}"),
            "alpha beta",
            &[],
            range,
            LinkOperation::Get,
        )?;
    }
    for (name, url) in [
        ("quoted", Some(A)),
        (
            "sentinel_quoted",
            Some("\u{fddf}\"https://a.example/path\""),
        ),
        ("unquoted", Some("https://a.example/path")),
        ("quoted_non_uri", Some("\"not a URI\"")),
        ("empty", Some("")),
        ("quoted_empty", Some("\"\"")),
        ("null", None),
        ("interior_quote", Some("\"https://a.example/a\"b\"")),
        ("embedded_nul", Some("\"https://a.example/a\0b\"")),
    ] {
        observe(
            &format!("link_plain_set_{name}"),
            "alpha beta",
            &[],
            (0, 5),
            LinkOperation::Set(url),
        )?;
    }
    observe(
        "link_collapsed_set",
        "alpha beta",
        &[],
        (2, 2),
        LinkOperation::Set(Some(A)),
    )?;
    observe(
        "link_empty_story_set",
        "",
        &[],
        (0, 0),
        LinkOperation::Set(Some(A)),
    )?;
    for (name, range) in [
        ("emoji", (1, 3)),
        ("combining", (3, 5)),
        ("combining_tail", (4, 5)),
    ] {
        observe(
            &format!("link_unicode_set_{name}"),
            "A😀e\u{301}B",
            &[],
            range,
            LinkOperation::Set(Some("\"https://例子.example/😀\"")),
        )?;
    }
    for (name, range) in [("break", (2, 3)), ("crossing", (1, 4)), ("whole", (0, 5))] {
        observe(
            &format!("link_paragraph_set_{name}"),
            "ab\rcd",
            &[],
            range,
            LinkOperation::Set(Some(A)),
        )?;
    }
    let regular_url = "https://a.example/path";
    let regular_range = (0, regular_url.encode_utf16().count() as i32);
    observe(
        "link_regular_url_get",
        regular_url,
        &[],
        regular_range,
        LinkOperation::Get,
    )?;
    observe(
        "link_regular_url_set",
        regular_url,
        &[],
        regular_range,
        LinkOperation::Set(Some(B)),
    )?;

    let first = [LinkSetup {
        range: (0, 5),
        url: A,
    }];
    for (name, range) in [
        ("whole", (0, 5)),
        ("partial", (1, 3)),
        ("character", (1, 2)),
        ("caret_start", (0, 0)),
        ("caret_inside", (2, 2)),
        ("caret_end", (5, 5)),
        ("after", (6, 10)),
        ("crossing", (3, 8)),
        ("whole_story", (0, 10)),
        ("caret_story_end", (10, 10)),
    ] {
        observe(
            &format!("link_friendly_get_{name}"),
            "alpha beta",
            &first,
            range,
            LinkOperation::Get,
        )?;
    }
    for (name, range, url) in [
        ("partial_update", (1, 3), Some(B)),
        ("whole_update", (0, 5), Some(B)),
        ("caret_update", (2, 2), Some(B)),
        ("empty_clear", (1, 3), Some("")),
        ("quoted_empty", (1, 3), Some("\"\"")),
        ("null_clear", (1, 3), None),
        ("append_same", (5, 10), Some(A)),
        ("append_different", (5, 10), Some(B)),
        ("cross_same", (3, 8), Some(A)),
        ("cross_different", (3, 8), Some(B)),
    ] {
        observe(
            &format!("link_friendly_set_{name}"),
            "alpha beta",
            &first,
            range,
            LinkOperation::Set(url),
        )?;
    }
    for (kind, url) in [("same", A), ("different", B)] {
        let adjacent = [first[0], LinkSetup { range: (5, 9), url }];
        for (name, range) in [
            ("first", (0, 5)),
            ("second", (5, 9)),
            ("crossing", (2, 7)),
            ("caret_boundary", (5, 5)),
            ("both", (0, 9)),
            ("after", (9, 10)),
        ] {
            observe(
                &format!("link_adjacent_{kind}_get_{name}"),
                "alphabeta!",
                &adjacent,
                range,
                LinkOperation::Get,
            )?;
        }
        observe(
            &format!("link_adjacent_{kind}_set_crossing"),
            "alphabeta!",
            &adjacent,
            (3, 7),
            LinkOperation::Set(Some("\"https://c.example/replacement\"")),
        )?;
    }
    assert_eq!(
        (commands, fixtures),
        (137, 59),
        "keep the Link matrix complete"
    );
    println!("SYSTEM_MSFTEDIT_LINK_PROBE_COMPLETE commands={commands} fixtures={fixtures}");
    Ok(())
}

#[test]
fn rich_text_tom_oracle_system_msftedit_link_characterization() {
    std::thread::Builder::new()
        .name("system-msftedit-link".into())
        .spawn(characterize_links)
        .expect("start the native Link TOM probe thread")
        .join()
        .expect("the native Link TOM probe thread panicked")
        .expect("the native Link TOM probe failed");
}

/// A failed getter retains its HRESULT and the initialized output sentinel.
/// Mixed formatting and unsupported independent-range flags are observations.
#[derive(Clone, Copy)]
struct NativeNumber {
    hresult: i32,
    value: i32,
}

impl std::fmt::Debug for NativeNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "NativeNumber {{ hresult: 0x{:08X}, value: {} }}",
            self.hresult as u32, self.value
        )
    }
}

impl NativeRange2 {
    fn raw_number(&self, getter: Getter) -> NativeNumber {
        let mut value = i32::MIN;
        // SAFETY: the caller supplies a verified LONG getter of this interface.
        let hresult = unsafe { getter(self.0.as_ptr().cast(), &mut value) };
        NativeNumber { hresult, value }
    }

    fn format_interface(
        &self,
        getter: InterfaceGetter,
        name: &str,
    ) -> TestResult<ComPtr<IUnknown>> {
        let mut format = null_mut();
        // SAFETY: GetFont/GetPara return an owned SDK interface reference.
        check_hresult(unsafe { getter(self.0.as_ptr().cast(), &mut format) }, name)?;
        // SAFETY: successful GetFont/GetPara transfers this owned reference.
        unsafe { ComPtr::<IUnknown>::from_raw(format.cast()) }
            .ok_or_else(|| format!("{name} returned null"))
    }

    fn font(&self) -> TestResult<ComPtr<IUnknown>> {
        self.format_interface(self.table().base.get_font, "ITextRange2::GetFont")
    }

    fn paragraph(&self) -> TestResult<ComPtr<IUnknown>> {
        self.format_interface(self.table().base.get_para, "ITextRange2::GetPara")
    }

    fn duplicate(&self) -> TestResult<(i32, Option<Self>)> {
        let mut duplicate = null_mut();
        // SAFETY: GetDuplicate is the SDK's owned ITextRange output at slot 11.
        let result =
            unsafe { (self.table().base.get_duplicate)(self.0.as_ptr().cast(), &mut duplicate) };
        // SAFETY: a non-null output is an owned ITextRange reference.
        let duplicate = unsafe { ComPtr::<IUnknown>::from_raw(duplicate.cast()) };
        if result < 0 {
            return Ok((result, None));
        }
        let duplicate = duplicate.ok_or("successful GetDuplicate returned null")?;
        let mut range2 = null_mut();
        // SAFETY: query the verified SDK IID before using its extended table.
        check_hresult(
            unsafe { duplicate.query_interface_raw(&IID_TEXT_RANGE2, &mut range2) } as i32,
            "QueryInterface(ITextRange2 duplicate)",
        )?;
        // SAFETY: successful QueryInterface transfers an owned reference.
        let duplicate = unsafe { ComPtr::<IUnknown>::from_raw(range2.cast()) }
            .map(Self)
            .ok_or("QueryInterface(ITextRange2 duplicate) returned null")?;
        Ok((result, Some(duplicate)))
    }

    fn gravity_state(&self) -> TestResult<GravityRangeState> {
        let range = self.state()?;
        let gravity = self.raw_number(self.table().get_gravity);
        let flags = self.raw_number(self.table().base.get_flags);
        let font = self.font()?;
        let paragraph = self.paragraph()?;
        // SAFETY: the format getters returned the verified SDK interfaces.
        let font_table = unsafe { &**font.as_ptr().cast::<*const TextFontVTable>() };
        // SAFETY: GetPara returned an owned ITextPara with this verified prefix.
        let paragraph_table = unsafe { &**paragraph.as_ptr().cast::<*const TextParaVTable>() };
        let read = |object: &ComPtr<IUnknown>, getter: Getter| {
            let mut value = i32::MIN;
            // SAFETY: callers pair a typed SDK getter with its live interface.
            let hresult = unsafe { getter(object.as_ptr().cast(), &mut value) };
            NativeNumber { hresult, value }
        };
        let bold = read(&font, font_table.get_bold);
        let italic = read(&font, font_table.get_italic);
        let alignment = read(&paragraph, paragraph_table.get_alignment);
        let text = self.text()?;
        let text_utf16_length = text.encode_utf16().count();
        assert_eq!(
            self.state()?,
            range,
            "format observation must not move its range"
        );
        Ok(GravityRangeState {
            range,
            gravity,
            flags,
            bold,
            italic,
            alignment,
            text,
            text_utf16_length,
        })
    }
}

struct GravityRangeState {
    range: RangeState,
    gravity: NativeNumber,
    flags: NativeNumber,
    bold: NativeNumber,
    italic: NativeNumber,
    alignment: NativeNumber,
    text: String,
    text_utf16_length: usize,
}

impl std::fmt::Debug for GravityRangeState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GravityRangeState")
            .field("range", &self.range)
            .field("gravity", &self.gravity)
            .field("flags", &self.flags)
            .field("bold", &self.bold)
            .field("italic", &self.italic)
            .field("alignment", &self.alignment)
            .field("text", &self.text)
            .field("text_utf16_length", &self.text_utf16_length)
            .finish()
    }
}

struct GravitySnapshot {
    primary: GravityRangeState,
    duplicate: Option<GravityRangeState>,
    story_text: String,
    story_text_utf16_length: usize,
    story_length: i32,
    selection: SelectionState,
}

impl std::fmt::Debug for GravitySnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GravitySnapshot")
            .field("primary", &self.primary)
            .field("duplicate", &self.duplicate)
            .field("story_text", &self.story_text)
            .field("story_text_utf16_length", &self.story_text_utf16_length)
            .field("story_length", &self.story_length)
            .field("selection", &self.selection)
            .finish()
    }
}

impl GravitySnapshot {
    fn capture(
        primary: &NativeRange2,
        duplicate: Option<&NativeRange2>,
        document: &NativeDocument,
        selection: &NativeSelection,
    ) -> TestResult<Self> {
        let selected = selection.state()?;
        let state = primary.gravity_state()?;
        let duplicate = duplicate.map(NativeRange2::gravity_state).transpose()?;
        let story_length =
            primary.number(primary.table().base.get_story_length, "GetStoryLength")?;
        let story_text = document.range(0, story_length)?.text()?;
        let story_text_utf16_length = story_text.encode_utf16().count();
        assert!(story_text_utf16_length <= story_length as usize);
        assert_eq!(
            selection.state()?,
            selected,
            "gravity observation retains selection"
        );
        Ok(Self {
            primary: state,
            duplicate,
            story_text,
            story_text_utf16_length,
            story_length,
            selection: selected,
        })
    }
}

#[derive(Clone, Copy)]
enum GravityTarget {
    Primary,
    Duplicate,
    External(i32, i32),
}

#[derive(Clone, Copy)]
enum GravityOperation<'a> {
    GetGravity,
    SetGravity(i32),
    SetRange(i32, i32),
    SetStart(i32),
    SetEnd(i32),
    Collapse(i32),
    Duplicate,
    SetText(&'a str),
    SetBold(i32),
    SetItalic(i32),
    SetAlignment(i32),
}

#[derive(Clone, Copy)]
struct GravityAction<'a> {
    target: GravityTarget,
    operation: GravityOperation<'a>,
}

impl<'a> GravityAction<'a> {
    fn primary(operation: GravityOperation<'a>) -> Self {
        Self {
            target: GravityTarget::Primary,
            operation,
        }
    }

    fn duplicate(operation: GravityOperation<'a>) -> Self {
        Self {
            target: GravityTarget::Duplicate,
            operation,
        }
    }

    fn external(start: i32, end: i32, operation: GravityOperation<'a>) -> Self {
        Self {
            target: GravityTarget::External(start, end),
            operation,
        }
    }
}

fn observe_gravity_step(
    name: &str,
    step: usize,
    role: &str,
    action: GravityAction<'_>,
    primary: &NativeRange2,
    duplicate: &mut Option<NativeRange2>,
    document: &NativeDocument,
    selection: &NativeSelection,
) -> TestResult<i32> {
    use GravityOperation::*;
    let (target, range) = match action.target {
        GravityTarget::Primary => ("Primary", primary.clone()),
        GravityTarget::Duplicate => (
            "Duplicate",
            duplicate
                .as_ref()
                .ok_or("duplicate command without a duplicate")?
                .clone(),
        ),
        GravityTarget::External(start, end) => ("External", document.range(start, end)?),
    };
    let (method, mut argument1, mut argument2, value, text) = match action.operation {
        GetGravity => ("GetGravity", 0, 0, 0, ""),
        SetGravity(value) => ("SetGravity", 0, 0, value, ""),
        SetRange(start, end) => ("SetRange", start, end, 0, ""),
        SetStart(start) => ("SetStart", start, 0, 0, ""),
        SetEnd(end) => ("SetEnd", 0, end, 0, ""),
        Collapse(value) => ("Collapse", 0, 0, value, ""),
        Duplicate => ("Duplicate", 0, 0, 0, ""),
        SetText(text) => ("SetText", 0, 0, 0, text),
        SetBold(value) => ("SetBold", 0, 0, value, ""),
        SetItalic(value) => ("SetItalic", 0, 0, value, ""),
        SetAlignment(value) => ("SetAlignment", 0, 0, value, ""),
    };
    if let GravityTarget::External(start, end) = action.target {
        assert!(matches!(
            action.operation,
            SetText(_) | SetBold(_) | SetItalic(_) | SetAlignment(_)
        ));
        argument1 = start;
        argument2 = end;
    }
    let before = GravitySnapshot::capture(primary, duplicate.as_ref(), document, selection)?;
    let action_before = range.state()?;
    let mut output = None;
    let table = range.table();
    let result = match action.operation {
        GetGravity => {
            let number = range.raw_number(table.get_gravity);
            output = Some(number.value);
            number.hresult
        }
        SetGravity(value) => {
            // SAFETY: SDK SetGravity takes a signed LONG; invalid values are measured.
            unsafe { (table.set_gravity)(range.0.as_ptr().cast(), value) }
        }
        SetRange(start, end) => {
            // SAFETY: both signed LONG positions, including extreme values, are SDK inputs.
            unsafe { (table.base.set_range)(range.0.as_ptr().cast(), start, end) }
        }
        SetStart(start) => {
            // SAFETY: the verified SDK slot accepts one signed LONG position.
            unsafe { (table.base.set_start)(range.0.as_ptr().cast(), start) }
        }
        SetEnd(end) => {
            // SAFETY: the verified SDK slot accepts one signed LONG position.
            unsafe { (table.base.set_end)(range.0.as_ptr().cast(), end) }
        }
        Collapse(value) => {
            // SAFETY: the SDK takes tomTrue (-1) for start and tomFalse (0) for end.
            unsafe { (table.base.collapse)(range.0.as_ptr().cast(), value) }
        }
        Duplicate => {
            assert!(matches!(action.target, GravityTarget::Primary));
            assert!(duplicate.is_none(), "one independent duplicate per fixture");
            let (result, copy) = range.duplicate()?;
            *duplicate = copy;
            result
        }
        SetText(text) => {
            let text = Bstr(string_to_bstr(text));
            if text.0.is_null() {
                return Err("SysAllocStringLen for gravity SetText failed".into());
            }
            // SAFETY: SetText borrows the allocated explicit-length BSTR synchronously.
            unsafe { (table.base.set_text)(range.0.as_ptr().cast(), text.0) }
        }
        SetBold(value) | SetItalic(value) => {
            let font = range.font()?;
            // SAFETY: GetFont returned the SDK ITextFont interface.
            let table = unsafe { &**font.as_ptr().cast::<*const TextFontVTable>() };
            let setter = if matches!(action.operation, SetBold(_)) {
                table.set_bold
            } else {
                table.set_italic
            };
            // SAFETY: the typed setter borrows its live ITextFont interface.
            unsafe { setter(font.as_ptr().cast(), value) }
        }
        SetAlignment(value) => {
            let paragraph = range.paragraph()?;
            // SAFETY: GetPara returned the SDK ITextPara interface.
            let table = unsafe { &**paragraph.as_ptr().cast::<*const TextParaVTable>() };
            // SAFETY: the typed setter borrows its live ITextPara interface.
            unsafe { (table.set_alignment)(paragraph.as_ptr().cast(), value) }
        }
    };
    let action_after = range.state()?;
    let after = GravitySnapshot::capture(primary, duplicate.as_ref(), document, selection)?;
    let text_utf16_length = text.encode_utf16().count();
    println!(
        "SYSTEM_MSFTEDIT_GRAVITY fixture={name} step={step} role={role} target={target} operation={method} argument1={argument1} argument2={argument2} value={value} text={text:?} text_utf16_length={text_utf16_length} action_before={action_before:?} action_after={action_after:?} before={before:?} after={after:?} hresult=0x{:08X} output={output:?}",
        result as u32,
    );
    Ok(result)
}

fn characterize_gravity_fixture(
    name: &str,
    text: &str,
    setup: &[GravityAction<'_>],
    initial: (i32, i32),
    gravity: i32,
    actions: &[GravityAction<'_>],
) -> TestResult<usize> {
    let window = HiddenRichEdit::new()?;
    window.set_text(text)?;
    let document = window.document()?;
    let selection = document.selection()?;
    let length = text.encode_utf16().count() as i32;
    let primary = document.range(0, 0)?;
    let story_length = primary.number(primary.table().base.get_story_length, "GetStoryLength")?;
    assert_eq!(
        document.range(0, length)?.text()?,
        text,
        "exact gravity fixture setup"
    );
    let commands = setup.len() + 2 + actions.len();
    println!(
        "SYSTEM_MSFTEDIT_GRAVITY_FIXTURE name={name} text={text:?} supplied_utf16_length={length} native_story_length={story_length} window_ex_style=0 expected_commands={commands}"
    );
    let mut duplicate = None;
    let prefix = [
        GravityAction::primary(GravityOperation::SetRange(initial.0, initial.1)),
        GravityAction::primary(GravityOperation::SetGravity(gravity)),
    ];
    for (step, action) in setup
        .iter()
        .chain(&prefix)
        .chain(actions)
        .copied()
        .enumerate()
    {
        let role = if step < setup.len() {
            "Setup"
        } else {
            "Target"
        };
        let result = observe_gravity_step(
            name,
            step,
            role,
            action,
            &primary,
            &mut duplicate,
            &document,
            &selection,
        )?;
        if role == "Setup" {
            check_hresult(result, "gravity fixture format setup")?;
        }
    }
    Ok(commands)
}

fn characterize_gravity() -> TestResult {
    use GravityAction as A;
    use GravityOperation::*;
    let _apartment = OleApartment::new()?;
    let library = SystemRichEditLibrary::load()?;
    library.describe()?;
    let mut commands = 0;
    let mut fixtures = 0;
    let mut observe =
        |name: &str, text: &str, setup: &[A<'_>], initial, gravity, actions: &[A<'_>]| {
            commands += characterize_gravity_fixture(name, text, setup, initial, gravity, actions)?;
            fixtures += 1;
            Ok::<_, String>(())
        };
    for gravity in 0..=4 {
        for (name, text, split, position, paragraphs) in [
            ("ascii_start", "abCD", 2, 0, false),
            ("ascii_boundary", "abCD", 2, 2, false),
            ("ascii_end", "abCD", 2, 4, false),
            ("unicode_boundary", "a😀e\u{301}Z", 3, 3, false),
            ("unicode_combining_inside", "a😀e\u{301}Z", 3, 4, false),
            ("paragraph_before_mark", "ab\rcd", 3, 2, true),
            ("paragraph_after_mark", "ab\rcd", 3, 3, true),
            ("space_boundary", "ab CD", 3, 3, false),
            ("empty", "", 0, 0, false),
        ] {
            let length = text.encode_utf16().count() as i32;
            let mut setup = vec![
                A::external(0, length, SetBold(0)),
                A::external(0, length, SetItalic(0)),
                A::external(0, length, SetAlignment(0)),
            ];
            if length > 0 {
                setup.push(A::external(0, split, SetBold(-1)));
                setup.push(A::external(split, length, SetItalic(-1)));
            }
            if paragraphs {
                setup.push(A::external(split, length, SetAlignment(2)));
            }
            observe(
                &format!("gravity_{gravity}_affinity_{name}"),
                text,
                &setup,
                (position, position),
                gravity,
                &[
                    A::primary(GetGravity),
                    A::primary(SetText("X")),
                    A::primary(GetGravity),
                ],
            )?;
        }
        // Fresh controls distinguish insertion at each boundary from edits
        // earlier/later in the story. Native duplicates are tracked in parallel.
        for (direction, initial) in [("forward", (1, 4)), ("reverse", (4, 1)), ("caret", (2, 2))] {
            for position in [0, 1, 2, 4, 6] {
                observe(
                    &format!("gravity_{gravity}_insert_{direction}_{position}"),
                    "abcdef",
                    &[],
                    initial,
                    gravity,
                    &[
                        A::primary(Duplicate),
                        A::external(position, position, SetText("X")),
                        A::primary(GetGravity),
                        A::duplicate(GetGravity),
                    ],
                )?;
            }
        }
        for (direction, initial) in [("forward", (1, 4)), ("reverse", (4, 1))] {
            for (name, start, end, text) in [
                ("left_overlap", 0, 2, "X"),
                ("inside", 2, 3, "😀"),
                ("right_delete", 3, 6, ""),
                ("whole", 0, 6, "Z"),
            ] {
                observe(
                    &format!("gravity_{gravity}_replace_{direction}_{name}"),
                    "abcdef",
                    &[],
                    initial,
                    gravity,
                    &[
                        A::primary(Duplicate),
                        A::external(start, end, SetText(text)),
                        A::primary(GetGravity),
                        A::duplicate(GetGravity),
                    ],
                )?;
            }
            observe(
                &format!("gravity_{gravity}_retention_{direction}"),
                "abcdef",
                &[],
                initial,
                gravity,
                &[
                    A::primary(GetGravity),
                    A::primary(Duplicate),
                    A::duplicate(SetGravity((gravity + 1) % 5)),
                    A::primary(SetRange(initial.0, initial.1)),
                    A::primary(SetRange(2, 5)),
                    A::primary(SetRange(5, 2)),
                    A::primary(SetStart(6)),
                    A::primary(SetEnd(0)),
                    A::primary(SetRange(1, 4)),
                    A::primary(Collapse(-1)),
                    A::primary(Collapse(-1)),
                    A::primary(SetRange(4, 1)),
                    A::primary(Collapse(0)),
                    A::primary(SetRange(2, 2)),
                    A::primary(SetText("Q")),
                    A::duplicate(GetGravity),
                    A::primary(GetGravity),
                ],
            )?;
        }
        observe(
            &format!("gravity_{gravity}_extreme_endpoints"),
            "abcdef",
            &[],
            (1, 4),
            gravity,
            &[
                A::primary(SetStart(i32::MIN)),
                A::primary(SetEnd(i32::MAX)),
                A::primary(SetRange(i32::MAX, i32::MIN)),
                A::primary(Collapse(-1)),
                A::primary(SetRange(-5, 2)),
                A::primary(SetRange(2, -5)),
                A::primary(SetStart(i32::MAX)),
                A::primary(SetEnd(i32::MIN)),
                A::primary(GetGravity),
            ],
        )?;
    }
    for value in [i32::MIN, -9_999_999, -2, -1, 5, 6, 32, i32::MAX] {
        for (direction, initial) in [("caret", (2, 2)), ("forward", (1, 4)), ("reverse", (4, 1))] {
            observe(
                &format!("gravity_invalid_{value}_{direction}"),
                "abcdef",
                &[],
                initial,
                2,
                &[
                    A::primary(SetGravity(value)),
                    A::primary(GetGravity),
                    A::primary(Duplicate),
                    A::duplicate(GetGravity),
                    A::primary(SetRange(2, 2)),
                    A::primary(GetGravity),
                ],
            )?;
        }
    }
    assert_eq!(
        (commands, fixtures),
        (1577, 199),
        "keep the gravity matrix complete"
    );
    println!("SYSTEM_MSFTEDIT_GRAVITY_PROBE_COMPLETE commands={commands} fixtures={fixtures}");
    Ok(())
}

#[test]
fn rich_text_tom_oracle_system_msftedit_gravity_characterization() {
    std::thread::Builder::new()
        .name("system-msftedit-gravity".into())
        .spawn(characterize_gravity)
        .expect("start the native gravity TOM probe thread")
        .join()
        .expect("the native gravity TOM probe thread panicked")
        .expect("the native gravity TOM probe failed");
}

// Actual selection Gravity is a separate measurement from document.Range.
// ITextSelection2 has the ITextRange2 vtable without additional methods:
// https://learn.microsoft.com/windows/win32/api/tom/nn-tom-itextselection2
// The SDK pin above defines IID c241f5e1-7206-11d8-a2c7-00a0d1d6c6b3.
// Both extended interfaces must resolve to the GetSelection object's canonical
// IUnknown before this fixture is permitted to call its Gravity slots.
const IID_TEXT_SELECTION2: Guid = Guid::from_u128(0xc241f5e1_7206_11d8_a2c7_00a0d1d6c6b3);
const IID_NATIVE_UNKNOWN: Guid = Guid::from_u128(0x00000000_0000_0000_c000_000000000046);

struct NativeInterfaceObservation {
    hresult: i32,
    is_null: bool,
    identity: Option<(i32, bool, bool)>,
}

impl std::fmt::Debug for NativeInterfaceObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NativeInterface {{ hresult: 0x{:08X}, is_null: {}, identity: ",
            self.hresult as u32, self.is_null
        )?;
        if let Some((hresult, is_null, same_identity)) = self.identity {
            write!(f, "Some(NativeIdentity {{ hresult: 0x{:08X}, is_null: {is_null}, same_identity: {same_identity} }})", hresult as u32)?;
        } else {
            write!(f, "None")?;
        }
        write!(f, " }}")
    }
}

fn native_query(object: &ComPtr<IUnknown>, iid: &Guid) -> (i32, Option<ComPtr<IUnknown>>) {
    let mut result = null_mut();
    // SAFETY: an exact SDK IID and an owned interface-pointer output. Retain the
    // HRESULT and null distinction before inspecting or using the interface.
    let hresult = unsafe { object.query_interface_raw(iid, &mut result) } as i32;
    // SAFETY: any returned non-null interface is one owned COM reference.
    (hresult, unsafe {
        ComPtr::<IUnknown>::from_raw(result.cast())
    })
}

fn observed_query(
    object: &ComPtr<IUnknown>,
    iid: &Guid,
    source_identity: &ComPtr<IUnknown>,
) -> (NativeInterfaceObservation, Option<ComPtr<IUnknown>>) {
    let (hresult, result) = native_query(object, iid);
    let identity = if hresult >= 0 {
        result.as_ref().map(|result| {
            let (hresult, identity) = native_query(result, &IID_NATIVE_UNKNOWN);
            let same = identity
                .as_ref()
                .is_some_and(|identity| identity.as_ptr() == source_identity.as_ptr());
            (hresult, identity.is_none(), same)
        })
    } else {
        None
    };
    (
        NativeInterfaceObservation {
            hresult,
            is_null: result.is_none(),
            identity,
        },
        result,
    )
}

struct NativeTextObservation {
    hresult: i32,
    is_null: bool,
    text: String,
    text_utf16_length: usize,
}

impl std::fmt::Debug for NativeTextObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NativeText {{ hresult: 0x{:08X}, is_null: {}, text: {:?}, text_utf16_length: {} }}",
            self.hresult as u32, self.is_null, self.text, self.text_utf16_length
        )
    }
}

impl NativeRange2 {
    fn observed_text(&self) -> TestResult<NativeTextObservation> {
        let mut text = Bstr(null_mut());
        // SAFETY: the verified getter writes an owned, possibly-null BSTR.
        let hresult = unsafe { (self.table().base.get_text)(self.0.as_ptr().cast(), &mut text.0) };
        let is_null = text.0.is_null();
        let text = text.text()?;
        let text_utf16_length = text.encode_utf16().count();
        Ok(NativeTextObservation {
            hresult,
            is_null,
            text,
            text_utf16_length,
        })
    }

    fn selection_gravity_state(&self) -> TestResult<SelectionGravityRangeState> {
        let range = self.state()?;
        let font = self.font()?;
        let paragraph = self.paragraph()?;
        // SAFETY: the SDK format getters return owned ITextFont/ITextPara.
        let font_table = unsafe { &**font.as_ptr().cast::<*const TextFontVTable>() };
        // SAFETY: GetPara returned an owned ITextPara with this verified prefix.
        let para_table = unsafe { &**paragraph.as_ptr().cast::<*const TextParaVTable>() };
        let read = |object: &ComPtr<IUnknown>, getter: Getter| {
            let mut value = i32::MIN;
            // SAFETY: each caller supplies the getter for its live interface.
            let hresult = unsafe { getter(object.as_ptr().cast(), &mut value) };
            NativeNumber { hresult, value }
        };
        let result = SelectionGravityRangeState {
            range,
            gravity: self.raw_number(self.table().get_gravity),
            flags: self.raw_number(self.table().base.get_flags),
            selection_type: self.raw_number(self.table().base.get_type),
            bold: read(&font, font_table.get_bold),
            italic: read(&font, font_table.get_italic),
            alignment: read(&paragraph, para_table.get_alignment),
            text: self.observed_text()?,
        };
        assert_eq!(
            self.state()?,
            range,
            "selection gravity observation retains endpoints"
        );
        Ok(result)
    }
}

struct SelectionGravityRangeState {
    range: RangeState,
    gravity: NativeNumber,
    flags: NativeNumber,
    selection_type: NativeNumber,
    bold: NativeNumber,
    italic: NativeNumber,
    alignment: NativeNumber,
    text: NativeTextObservation,
}

impl std::fmt::Debug for SelectionGravityRangeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectionGravityRangeState")
            .field("range", &self.range)
            .field("gravity", &self.gravity)
            .field("flags", &self.flags)
            .field("selection_type", &self.selection_type)
            .field("bold", &self.bold)
            .field("italic", &self.italic)
            .field("alignment", &self.alignment)
            .field("text", &self.text)
            .finish()
    }
}

struct SelectionGravitySnapshot {
    primary: SelectionGravityRangeState,
    duplicate: Option<SelectionGravityRangeState>,
    story_text: NativeTextObservation,
    story_length: NativeNumber,
    selection: SelectionState,
}

impl std::fmt::Debug for SelectionGravitySnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectionGravitySnapshot")
            .field("primary", &self.primary)
            .field("duplicate", &self.duplicate)
            .field("story_text", &self.story_text)
            .field("story_length", &self.story_length)
            .field("selection", &self.selection)
            .finish()
    }
}

impl SelectionGravitySnapshot {
    fn capture(
        primary: &NativeRange2,
        duplicate: Option<&NativeRange2>,
        document: &NativeDocument,
        selection: &NativeSelection,
    ) -> TestResult<Self> {
        let selected = selection.state()?;
        let primary = primary.selection_gravity_state()?;
        assert_eq!(
            primary.range,
            RangeState {
                start: selected.start,
                end: selected.end
            },
            "extended interface is the actual selection"
        );
        check_hresult(primary.flags.hresult, "actual selection GetFlags")?;
        assert_eq!(primary.flags.value, selected.flags);
        let duplicate = duplicate
            .map(NativeRange2::selection_gravity_state)
            .transpose()?;
        let caret = document.range(0, 0)?;
        let story_length = caret.raw_number(caret.table().base.get_story_length);
        check_hresult(story_length.hresult, "selection gravity story length")?;
        assert!(story_length.value >= 0);
        let story_text = document.range(0, story_length.value)?.observed_text()?;
        assert_eq!(
            selection.state()?,
            selected,
            "snapshot metadata must not move the selection"
        );
        Ok(Self {
            primary,
            duplicate,
            story_text,
            story_length,
            selection: selected,
        })
    }
}

#[derive(Clone, Copy)]
enum SelectionGravityOperation<'a> {
    GetGravity,
    SetGravity(i32),
    SetRange(i32, i32),
    Duplicate,
    TypeText(Option<&'a str>),
    MoveLeft(Unit, i32, bool),
    MoveRight(Unit, i32, bool),
    MoveUp(Unit, i32, bool),
    MoveDown(Unit, i32, bool),
    EndKey(Unit, bool),
    SetBold(i32),
    SetItalic(i32),
    SetAlignment(i32),
}

#[derive(Clone, Copy)]
struct SelectionGravityAction<'a> {
    target: GravityTarget,
    operation: SelectionGravityOperation<'a>,
}

impl<'a> SelectionGravityAction<'a> {
    fn primary(operation: SelectionGravityOperation<'a>) -> Self {
        Self {
            target: GravityTarget::Primary,
            operation,
        }
    }
    fn duplicate(operation: SelectionGravityOperation<'a>) -> Self {
        Self {
            target: GravityTarget::Duplicate,
            operation,
        }
    }
    fn external(start: i32, end: i32, operation: SelectionGravityOperation<'a>) -> Self {
        Self {
            target: GravityTarget::External(start, end),
            operation,
        }
    }
}

fn observe_selection_gravity_step(
    name: &str,
    step: usize,
    role: &str,
    action: SelectionGravityAction<'_>,
    primary: &NativeRange2,
    duplicate: &mut Option<NativeRange2>,
    document: &NativeDocument,
    selection: &NativeSelection,
    source_identity: &ComPtr<IUnknown>,
) -> TestResult<i32> {
    use SelectionGravityOperation::*;
    let (target, range) = match action.target {
        GravityTarget::Primary => ("Primary", primary.clone()),
        GravityTarget::Duplicate => (
            "Duplicate",
            duplicate
                .as_ref()
                .ok_or("selection gravity duplicate is unavailable")?
                .clone(),
        ),
        GravityTarget::External(start, end) => ("External", document.range(start, end)?),
    };
    let (method, mut argument1, mut argument2, value, extend, text) = match action.operation {
        GetGravity => ("GetGravity", 0, 0, 0, false, None),
        SetGravity(value) => ("SetGravity", 0, 0, value, false, None),
        SetRange(start, end) => ("SetRange", start, end, 0, false, None),
        Duplicate => ("Duplicate", 0, 0, 0, false, None),
        TypeText(text) => ("TypeText", 0, 0, 0, false, text),
        MoveLeft(unit, count, extend) => ("MoveLeft", unit as i32, count, 0, extend, None),
        MoveRight(unit, count, extend) => ("MoveRight", unit as i32, count, 0, extend, None),
        MoveUp(unit, count, extend) => ("MoveUp", unit as i32, count, 0, extend, None),
        MoveDown(unit, count, extend) => ("MoveDown", unit as i32, count, 0, extend, None),
        EndKey(unit, extend) => ("EndKey", unit as i32, 0, 0, extend, None),
        SetBold(value) => ("SetBold", 0, 0, value, false, None),
        SetItalic(value) => ("SetItalic", 0, 0, value, false, None),
        SetAlignment(value) => ("SetAlignment", 0, 0, value, false, None),
    };
    if let GravityTarget::External(start, end) = action.target {
        assert!(matches!(
            action.operation,
            GetGravity | SetBold(_) | SetItalic(_) | SetAlignment(_)
        ));
        argument1 = start;
        argument2 = end;
    }
    let before =
        SelectionGravitySnapshot::capture(primary, duplicate.as_ref(), document, selection)?;
    let action_before = range.selection_gravity_state()?;
    let mut output = None;
    let mut returned_null = None;
    let mut duplicate_interface = None;
    let table = range.table();
    let hresult = match action.operation {
        GetGravity => {
            let number = range.raw_number(table.get_gravity);
            output = Some(number.value);
            number.hresult
        }
        SetGravity(value) => {
            // SAFETY: exact signed LONG input; invalid values are observations.
            unsafe { (table.set_gravity)(range.0.as_ptr().cast(), value) }
        }
        SetRange(start, end) => {
            // SAFETY: verified ITextRange prefix and signed LONG positions.
            unsafe { (table.base.set_range)(range.0.as_ptr().cast(), start, end) }
        }
        Duplicate => {
            assert!(matches!(action.target, GravityTarget::Primary));
            assert!(
                duplicate.is_none(),
                "one duplicate per fresh selection fixture"
            );
            let mut raw = null_mut();
            // SAFETY: GetDuplicate returns an owned ITextRange reference.
            let hresult = unsafe { (table.base.get_duplicate)(range.0.as_ptr().cast(), &mut raw) };
            returned_null = Some(raw.is_null());
            // SAFETY: any non-null output is one owned interface reference.
            let raw = unsafe { ComPtr::<IUnknown>::from_raw(raw.cast()) };
            if hresult >= 0 {
                if let Some(raw) = raw {
                    let (observation, extended) =
                        observed_query(&raw, &IID_TEXT_RANGE2, source_identity);
                    if observation.hresult >= 0 {
                        *duplicate = extended.map(NativeRange2);
                    }
                    duplicate_interface = Some(observation);
                }
            }
            hresult
        }
        TypeText(text) => {
            assert!(matches!(action.target, GravityTarget::Primary));
            let nonnull = text.is_some();
            let text = Bstr(text.map_or(null_mut(), string_to_bstr));
            if nonnull && text.0.is_null() {
                return Err("SysAllocStringLen for selection gravity TypeText failed".into());
            }
            // SAFETY: call the original GetSelection interface, retaining its
            // exact nullable BSTR and raw HRESULT rather than asserting success.
            unsafe { (selection.table().type_text)(selection.0.as_ptr().cast(), text.0) }
        }
        MoveLeft(unit, count, extend)
        | MoveRight(unit, count, extend)
        | MoveUp(unit, count, extend)
        | MoveDown(unit, count, extend) => {
            assert!(matches!(action.target, GravityTarget::Primary));
            let table = selection.table();
            let method = match action.operation {
                MoveLeft(..) => table.move_left,
                MoveRight(..) => table.move_right,
                MoveUp(..) => table.move_up,
                MoveDown(..) => table.move_down,
                _ => unreachable!(),
            };
            let mut delta = i32::MIN;
            // SAFETY: exact SDK movement signature and initialized LONG output.
            let hresult = unsafe {
                method(
                    selection.0.as_ptr().cast(),
                    unit as i32,
                    count,
                    i32::from(extend),
                    &mut delta,
                )
            };
            output = Some(delta);
            hresult
        }
        EndKey(unit, extend) => {
            assert!(matches!(action.target, GravityTarget::Primary));
            let mut delta = i32::MIN;
            // SAFETY: exact SDK boundary signature and initialized LONG output.
            let hresult = unsafe {
                (selection.table().end_key)(
                    selection.0.as_ptr().cast(),
                    unit as i32,
                    i32::from(extend),
                    &mut delta,
                )
            };
            output = Some(delta);
            hresult
        }
        SetBold(value) | SetItalic(value) => {
            let font = range.font()?;
            // SAFETY: GetFont returns the verified SDK ITextFont prefix.
            let table = unsafe { &**font.as_ptr().cast::<*const TextFontVTable>() };
            let setter = if matches!(action.operation, SetBold(_)) {
                table.set_bold
            } else {
                table.set_italic
            };
            // SAFETY: the typed setter is paired with its live SDK interface.
            unsafe { setter(font.as_ptr().cast(), value) }
        }
        SetAlignment(value) => {
            let paragraph = range.paragraph()?;
            // SAFETY: GetPara returns the verified SDK ITextPara prefix.
            let table = unsafe { &**paragraph.as_ptr().cast::<*const TextParaVTable>() };
            // SAFETY: the typed setter is paired with its live SDK interface.
            unsafe { (table.set_alignment)(paragraph.as_ptr().cast(), value) }
        }
    };
    let action_after = range.selection_gravity_state()?;
    let after =
        SelectionGravitySnapshot::capture(primary, duplicate.as_ref(), document, selection)?;
    let text_null = text.is_none();
    let text = text.unwrap_or("");
    let text_utf16_length = text.encode_utf16().count();
    println!("SYSTEM_MSFTEDIT_SELECTION_GRAVITY fixture={name} step={step} role={role} target={target} operation={method} argument1={argument1} argument2={argument2} value={value} extend={extend} text_null={text_null} text={text:?} text_utf16_length={text_utf16_length} action_before={action_before:?} action_after={action_after:?} before={before:?} after={after:?} hresult=0x{:08X} output={output:?} returned_null={returned_null:?} duplicate_interface={duplicate_interface:?}", hresult as u32);
    Ok(hresult)
}

fn characterize_selection_gravity_fixture(
    name: &str,
    text: &str,
    setup: &[SelectionGravityAction<'_>],
    initial: Option<((i32, i32), i32)>,
    actions: &[SelectionGravityAction<'_>],
) -> TestResult<usize> {
    characterize_selection_gravity_fixture_with_paragraph_inspection(
        name, text, setup, initial, actions, false,
    )
}

fn characterize_selection_gravity_fixture_with_paragraph_inspection(
    name: &str,
    text: &str,
    setup: &[SelectionGravityAction<'_>],
    initial: Option<((i32, i32), i32)>,
    actions: &[SelectionGravityAction<'_>],
    inspect_paragraphs: bool,
) -> TestResult<usize> {
    let window = HiddenRichEdit::new()?;
    window.set_text(text)?;
    let document = window.document()?;
    let selection = document.selection()?;
    let length = text.encode_utf16().count() as i32;
    let story_length = selection.number(selection.table().get_story_length, "GetStoryLength")?;
    assert_eq!(
        document.range(0, length)?.text()?,
        text,
        "exact selection gravity fixture"
    );
    let mut prefix = Vec::new();
    if let Some(((start, end), gravity)) = initial {
        prefix.push(SelectionGravityAction::primary(
            SelectionGravityOperation::SetRange(start, end),
        ));
        prefix.push(SelectionGravityAction::primary(
            SelectionGravityOperation::SetGravity(gravity),
        ));
    }
    let initialization = if initial.is_some() {
        "Configured"
    } else {
        "Default"
    };
    let initial_commands = setup.len() + prefix.len() + actions.len();
    let commands = initial_commands + if inspect_paragraphs { 5 } else { 0 };
    println!("SYSTEM_MSFTEDIT_SELECTION_GRAVITY_FIXTURE name={name} text={text:?} supplied_utf16_length={length} native_story_length={story_length} window_ex_style=0 expected_commands={commands} initialization={initialization}");
    let (source_hresult, source_identity) = native_query(&selection.0, &IID_NATIVE_UNKNOWN);
    let source_null = source_identity.is_none();
    check_hresult(source_hresult, "selection QueryInterface(IUnknown)")?;
    let source_identity = source_identity.ok_or("selection identity is null")?;
    let (range2, primary) = observed_query(&selection.0, &IID_TEXT_RANGE2, &source_identity);
    let (selection2, selection2_object) =
        observed_query(&selection.0, &IID_TEXT_SELECTION2, &source_identity);
    println!("SYSTEM_MSFTEDIT_SELECTION_GRAVITY_INTERFACE fixture={name} source_hresult=0x{:08X} source_null={source_null} range2={range2:?} selection2={selection2:?}", source_hresult as u32);
    for observation in [&range2, &selection2] {
        check_hresult(observation.hresult, "actual selection extended interface")?;
        let (hresult, is_null, same_identity) = observation
            .identity
            .ok_or("selection extension has no identity")?;
        check_hresult(hresult, "selection extension QueryInterface(IUnknown)")?;
        assert!(
            !observation.is_null && !is_null && same_identity,
            "extended interface must identify the original selection"
        );
    }
    let primary = NativeRange2(primary.ok_or("selection ITextRange2 is null")?);
    drop(selection2_object);
    let mut duplicate = None;
    for (step, action) in setup
        .iter()
        .chain(&prefix)
        .chain(actions)
        .copied()
        .enumerate()
    {
        let role = if step < setup.len() {
            "Setup"
        } else {
            "Target"
        };
        let hresult = observe_selection_gravity_step(
            name,
            step,
            role,
            action,
            &primary,
            &mut duplicate,
            &document,
            &selection,
            &source_identity,
        )?;
        if role == "Setup" {
            check_hresult(hresult, "selection gravity formatting setup")?;
        }
    }
    if inspect_paragraphs {
        let ((start, end), _) =
            initial.ok_or("paragraph inspection requires a configured range")?;
        let observations = selection_paragraph_inspections(
            start.min(end),
            &document,
            &selection,
            duplicate.as_ref(),
        )?;
        for (offset, action) in observations.into_iter().enumerate() {
            observe_selection_gravity_step(
                name,
                initial_commands + offset,
                "Target",
                action,
                &primary,
                &mut duplicate,
                &document,
                &selection,
                &source_identity,
            )?;
        }
    }
    Ok(commands)
}

// Reuse the existing action snapshots: each independent GetGravity also records
// text, Bold, Italic, paragraph alignment, null BSTRs and each getter's HRESULT.
// Derive inspection ranges from the observed story, never an expected TypeText
// result. The first query covers the old replacement start to the resulting
// caret when collapsed; an unchanged nonempty selection is sampled at its old
// start. The remaining queries cover the first, second and final paragraphs,
// including their actual CR marks. Missing paragraphs repeat the final span.
fn selection_paragraph_inspections(
    original_start: i32,
    document: &NativeDocument,
    selection: &NativeSelection,
    duplicate: Option<&NativeRange2>,
) -> TestResult<[SelectionGravityAction<'static>; 5]> {
    use SelectionGravityAction as A;
    use SelectionGravityOperation::GetGravity;
    let selected = selection.state()?;
    let duplicate_range = duplicate.map(NativeRange2::state).transpose()?;
    let length = selection.number(selection.table().get_story_length, "GetStoryLength")?;
    let story = document.range(0, length)?.observed_text()?;
    check_hresult(story.hresult, "paragraph inspection story text")?;
    let units: Vec<_> = story.text.encode_utf16().collect();
    assert_eq!(
        units.len(),
        length as usize,
        "inspect the exact native story"
    );
    let mut paragraphs = Vec::new();
    let mut paragraph_start = 0;
    for (index, unit) in units.iter().enumerate() {
        if *unit == u16::from(b'\r') {
            let end = index as i32 + 1;
            paragraphs.push((paragraph_start, end));
            paragraph_start = end;
        }
    }
    if paragraph_start < length {
        paragraphs.push((paragraph_start, length));
    }
    let last = paragraphs.last().copied().unwrap_or((0, 0));
    let first = paragraphs.first().copied().unwrap_or(last);
    let second = paragraphs.get(1).copied().unwrap_or(last);
    let start = original_start.clamp(0, length);
    let end = if selected.start == selected.end {
        selected.start
    } else {
        start
    };
    assert_eq!(
        selection.state()?,
        selected,
        "inspection planning retains selection"
    );
    assert_eq!(
        duplicate.map(NativeRange2::state).transpose()?,
        duplicate_range,
        "inspection planning retains duplicate endpoints"
    );
    Ok([
        A::external(start.min(end), start.max(end), GetGravity),
        A::external(first.0, first.1, GetGravity),
        A::external(second.0, second.1, GetGravity),
        A::external(last.0, last.1, GetGravity),
        A::primary(GetGravity),
    ])
}

fn characterize_selection_paragraph_typing() -> TestResult<(usize, usize)> {
    use SelectionGravityAction as A;
    use SelectionGravityOperation::*;
    // Gravity 0 is explicit in each new fixture. The original 250-command
    // matrix already compares all five modes, including reversed emoji typing.
    // These cases vary only the missing replacement boundaries/input shapes.
    let cases = [
        ("inner_forward", "ab\rcd", (1, 4), "X", 0, 2, 3),
        ("inner_reverse", "ab\rcd", (4, 1), "X", 0, 2, 3),
        ("mark_forward", "ab\rcd", (2, 3), "X", 0, 2, 3),
        ("mark_reverse", "ab\rcd", (3, 2), "X", 0, 2, 3),
        ("end_at_next_start", "ab\rcd", (1, 3), "X", 0, 2, 3),
        ("whole_first", "ab\rcd", (0, 3), "X", 0, 2, 3),
        ("end_at_visible_end", "ab\rcd", (1, 5), "X", 0, 2, 3),
        ("whole_visible", "ab\rcd", (0, 5), "X", 0, 2, 3),
        ("empty_forward", "ab\rcd", (1, 4), "", 0, 2, 3),
        ("empty_reverse", "ab\rcd", (4, 1), "", 0, 2, 3),
        ("emoji_forward", "ab\rcd", (1, 4), "😀", 0, 2, 3),
        ("cr_forward", "ab\rcd", (1, 4), "\r", 0, 2, 3),
        ("lf_forward", "ab\rcd", (1, 4), "\n", 0, 2, 3),
        ("crlf_reverse", "ab\rcd", (4, 1), "\r\n", 0, 2, 3),
        ("opposite_alignment", "ab\rcd", (1, 4), "X", 2, 0, 3),
        ("three_paragraphs", "ab\rcd\ref", (1, 7), "X", 0, 2, 6),
    ];
    let mut commands = 0;
    for (name, text, initial, typed, leading, trailing, last_start) in cases {
        let length = text.encode_utf16().count() as i32;
        let setup = [
            A::external(0, length, SetBold(0)),
            A::external(0, length, SetItalic(0)),
            A::external(0, length, SetAlignment(leading)),
            A::external(0, 3, SetBold(-1)),
            A::external(last_start, length, SetItalic(-1)),
            A::external(last_start, length, SetAlignment(trailing)),
        ];
        commands += characterize_selection_gravity_fixture_with_paragraph_inspection(
            &format!("selection_paragraph_typing_{name}"),
            text,
            &setup,
            Some((initial, 0)),
            &[A::primary(Duplicate), A::primary(TypeText(Some(typed)))],
            true,
        )?;
    }
    assert_eq!(
        (commands, cases.len()),
        (240, 16),
        "keep paragraph typing coverage complete"
    );
    Ok((commands, cases.len()))
}

fn characterize_selection_gravity() -> TestResult {
    use SelectionGravityAction as A;
    use SelectionGravityOperation::*;
    let _apartment = OleApartment::new()?;
    let library = SystemRichEditLibrary::load()?;
    library.describe()?;
    let mut commands = 0;
    let mut fixtures = 0;
    let mut observe = |name: &str, text: &str, setup: &[A<'_>], initial, actions: &[A<'_>]| {
        commands += characterize_selection_gravity_fixture(name, text, setup, initial, actions)?;
        fixtures += 1;
        Ok::<_, String>(())
    };
    let ordinary = [
        A::external(0, 4, SetBold(0)),
        A::external(0, 4, SetItalic(0)),
        A::external(0, 2, SetBold(-1)),
        A::external(2, 4, SetItalic(-1)),
    ];
    let paragraphs = [
        A::external(0, 5, SetBold(0)),
        A::external(0, 5, SetItalic(0)),
        A::external(0, 5, SetAlignment(0)),
        A::external(0, 3, SetBold(-1)),
        A::external(3, 5, SetItalic(-1)),
        A::external(3, 5, SetAlignment(2)),
    ];
    for gravity in 0..=4 {
        observe(
            &format!("selection_gravity_{gravity}_ordinary"),
            "abCD",
            &ordinary,
            Some(((2, 2), gravity)),
            &[
                A::primary(Duplicate),
                A::duplicate(SetGravity((gravity + 1) % 5)),
                A::primary(TypeText(Some("X"))),
                A::external(2, 3, GetGravity),
                A::primary(GetGravity),
            ],
        )?;
        observe(
            &format!("selection_gravity_{gravity}_paragraph"),
            "ab\rcd",
            &paragraphs,
            Some(((2, 2), gravity)),
            &[
                A::primary(GetGravity),
                A::primary(SetRange(3, 3)),
                // Selection SetRange may itself change gravity; measure both
                // its retention and the requested affinity at the new boundary.
                A::primary(SetGravity(gravity)),
                A::primary(TypeText(Some("X"))),
                A::external(3, 4, GetGravity),
                A::primary(GetGravity),
            ],
        )?;
        observe(
            &format!("selection_gravity_{gravity}_navigation"),
            "ab\rcd",
            &paragraphs,
            Some(((4, 1), gravity)),
            &[
                A::primary(GetGravity),
                A::primary(Duplicate),
                A::primary(MoveRight(Unit::Character, 1, false)),
                A::primary(MoveLeft(Unit::Character, 1, true)),
                A::primary(SetRange(1, 1)),
                A::primary(EndKey(Unit::Line, false)),
                A::primary(MoveRight(Unit::Character, 1, false)),
                A::primary(MoveUp(Unit::Line, 1, false)),
                A::primary(MoveDown(Unit::Line, 1, true)),
                A::primary(SetRange(4, 1)),
                A::primary(TypeText(Some("😀"))),
                A::external(1, 3, GetGravity),
                A::primary(GetGravity),
            ],
        )?;
    }
    for (name, positions) in [("caret", (2, 2)), ("reverse", (4, 1))] {
        observe(
            &format!("selection_gravity_invalid_{name}"),
            "abcdef",
            &[],
            Some((positions, 2)),
            &[
                A::primary(SetGravity(-1)),
                A::primary(GetGravity),
                A::primary(SetGravity(5)),
                A::primary(GetGravity),
                A::primary(TypeText(None)),
                A::primary(TypeText(Some(""))),
                A::primary(GetGravity),
            ],
        )?;
    }
    // No SetRange or SetGravity masks the fresh empty selection's defaults.
    observe(
        "selection_gravity_empty_default",
        "",
        &[],
        None,
        &[A::primary(TypeText(Some("X"))), A::primary(GetGravity)],
    )?;
    assert_eq!(
        (commands, fixtures),
        (250, 18),
        "keep the actual selection matrix bounded and complete"
    );
    let (paragraph_commands, paragraph_fixtures) = characterize_selection_paragraph_typing()?;
    commands += paragraph_commands;
    fixtures += paragraph_fixtures;
    assert_eq!(
        (commands, fixtures),
        (490, 34),
        "keep the extended selection matrix complete"
    );
    println!(
        "SYSTEM_MSFTEDIT_SELECTION_GRAVITY_PROBE_COMPLETE commands={commands} fixtures={fixtures}"
    );
    Ok(())
}

#[test]
fn rich_text_tom_oracle_system_msftedit_selection_gravity_characterization() {
    std::thread::Builder::new()
        .name("system-msftedit-selection-gravity-probe".into())
        .spawn(characterize_selection_gravity)
        .expect("start the actual selection Gravity probe thread")
        .join()
        .expect("the actual selection Gravity probe thread panicked")
        .expect("the actual selection Gravity probe failed");
}


// Additional endpoint relationships are a separate observation family. This
// stage deliberately reuses GravitySnapshot (including its format getters) to
// compare with the older endpoint cases. It is not a cache-cold/GetFont-free
// experiment and must not be used to infer independent duplicate font caches.
fn observe_range_tracking_step(
    name: &str,
    step: usize,
    action: GravityAction<'_>,
    primary: &NativeRange2,
    duplicate: &mut Option<NativeRange2>,
    document: &NativeDocument,
    selection: &NativeSelection,
) -> TestResult<i32> {
    use GravityOperation::*;
    let (target, range) = match action.target {
        GravityTarget::Primary => ("Primary", primary.clone()),
        GravityTarget::Duplicate => (
            "Duplicate",
            duplicate
                .as_ref()
                .ok_or("range tracking getter has no duplicate")?
                .clone(),
        ),
        GravityTarget::External(start, end) => ("External", document.range(start, end)?),
    };
    let (method, mut argument1, mut argument2, value, text) = match action.operation {
        SetRange(start, end) => ("SetRange", start, end, 0, ""),
        SetGravity(value) => ("SetGravity", 0, 0, value, ""),
        Duplicate => ("Duplicate", 0, 0, 0, ""),
        SetText(text) => ("SetText", 0, 0, 0, text),
        GetGravity => ("GetGravity", 0, 0, 0, ""),
        _ => return Err("unexpected operation in range tracking matrix".into()),
    };
    if let GravityTarget::External(start, end) = action.target {
        assert!(matches!(action.operation, SetText(_)));
        argument1 = start;
        argument2 = end;
    }
    let before = GravitySnapshot::capture(primary, duplicate.as_ref(), document, selection)?;
    let action_before = range.state()?;
    let table = range.table();
    let mut output = None;
    let hresult = match action.operation {
        SetRange(start, end) => {
            // SAFETY: use the verified SDK signed-LONG endpoint signature.
            unsafe { (table.base.set_range)(range.0.as_ptr().cast(), start, end) }
        }
        SetGravity(value) => {
            // SAFETY: preserve the requested signed enum and raw native result.
            unsafe { (table.set_gravity)(range.0.as_ptr().cast(), value) }
        }
        Duplicate => {
            assert!(matches!(action.target, GravityTarget::Primary));
            assert!(
                duplicate.is_none(),
                "one passive duplicate per fresh control"
            );
            let (result, copy) = range.duplicate()?;
            *duplicate = copy;
            result
        }
        SetText(text) => {
            let text = Bstr(string_to_bstr(text));
            if text.0.is_null() {
                return Err("SysAllocStringLen for range tracking SetText failed".into());
            }
            // SAFETY: SetText borrows this allocated explicit-length BSTR.
            // Do not assert success or predict the surviving passive endpoints.
            unsafe { (table.base.set_text)(range.0.as_ptr().cast(), text.0) }
        }
        GetGravity => {
            let number = range.raw_number(table.get_gravity);
            output = Some(number.value);
            number.hresult
        }
        _ => unreachable!("the operation was validated before snapshot capture"),
    };
    let action_after = range.state()?;
    let after = GravitySnapshot::capture(primary, duplicate.as_ref(), document, selection)?;
    let text_utf16_length = text.encode_utf16().count();
    println!("SYSTEM_MSFTEDIT_RANGE_TRACKING fixture={name} step={step} role=Target target={target} operation={method} argument1={argument1} argument2={argument2} value={value} text={text:?} text_utf16_length={text_utf16_length} action_before={action_before:?} action_after={action_after:?} before={before:?} after={after:?} hresult=0x{:08X} output={output:?}", hresult as u32);
    Ok(hresult)
}

fn characterize_range_tracking_fixture(
    name: &str,
    initial: (i32, i32),
    gravity: i32,
    edited: (i32, i32),
    replacement: &str,
) -> TestResult<usize> {
    use GravityAction as A;
    use GravityOperation::*;
    let window = HiddenRichEdit::new()?;
    let text = "abcdef";
    window.set_text(text)?;
    let document = window.document()?;
    let selection = document.selection()?;
    let primary = document.range(0, 0)?;
    let length = text.encode_utf16().count() as i32;
    let story_length = primary.number(primary.table().base.get_story_length, "GetStoryLength")?;
    assert_eq!(
        document.range(0, length)?.text()?,
        text,
        "exact endpoint fixture"
    );
    let actions = [
        A::primary(SetRange(initial.0, initial.1)),
        A::primary(SetGravity(gravity)),
        A::primary(Duplicate),
        A::external(edited.0, edited.1, SetText(replacement)),
        A::primary(GetGravity),
        A::duplicate(GetGravity),
    ];
    let commands = actions.len();
    println!("SYSTEM_MSFTEDIT_RANGE_TRACKING_FIXTURE name={name} text={text:?} supplied_utf16_length={length} native_story_length={story_length} window_ex_style=0 expected_commands={commands}");
    let mut duplicate = None;
    for (step, action) in actions.into_iter().enumerate() {
        observe_range_tracking_step(
            name,
            step,
            action,
            &primary,
            &mut duplicate,
            &document,
            &selection,
        )?;
    }
    Ok(commands)
}

fn characterize_range_tracking() -> TestResult {
    let _apartment = OleApartment::new()?;
    let library = SystemRichEditLibrary::load()?;
    library.describe()?;
    // Twelve relationships at UIBehavior (0); three representative cases also
    // request Forward (2). Preserve raw mode readback rather than predicting it.
    let cases = [
        (
            "passive_tracking_0_right_overlap_text",
            (1, 4),
            0,
            (3, 6),
            "XY",
        ),
        ("passive_tracking_0_exact_span", (1, 4), 0, (1, 4), "XY"),
        ("passive_tracking_0_shared_start", (1, 4), 0, (1, 3), "XY"),
        ("passive_tracking_0_shared_end", (1, 4), 0, (2, 4), "XY"),
        ("passive_tracking_0_caret_inside", (2, 2), 0, (0, 4), "XY"),
        ("passive_tracking_0_caret_at_start", (2, 2), 0, (2, 4), "XY"),
        ("passive_tracking_0_caret_at_end", (2, 2), 0, (0, 2), "XY"),
        ("passive_tracking_0_same_text", (1, 4), 0, (1, 4), "bcd"),
        (
            "passive_tracking_0_retained_prefix",
            (1, 4),
            0,
            (0, 2),
            "abX",
        ),
        (
            "passive_tracking_0_retained_suffix",
            (1, 4),
            0,
            (0, 2),
            "Xab",
        ),
        (
            "passive_tracking_0_covered_shared_start",
            (1, 4),
            0,
            (1, 6),
            "Z",
        ),
        (
            "passive_tracking_0_covered_shared_end",
            (1, 4),
            0,
            (0, 4),
            "Z",
        ),
        (
            "passive_tracking_2_right_overlap_text",
            (1, 4),
            2,
            (3, 6),
            "XY",
        ),
        ("passive_tracking_2_exact_span", (1, 4), 2, (1, 4), "XY"),
        ("passive_tracking_2_caret_inside", (2, 2), 2, (0, 4), "XY"),
    ];
    let mut commands = 0;
    for (name, initial, gravity, edited, replacement) in cases {
        commands +=
            characterize_range_tracking_fixture(name, initial, gravity, edited, replacement)?;
    }
    let fixtures = cases.len();
    assert_eq!(
        (commands, fixtures),
        (90, 15),
        "keep endpoint follow-up complete"
    );
    println!(
        "SYSTEM_MSFTEDIT_RANGE_TRACKING_PROBE_COMPLETE commands={commands} fixtures={fixtures}"
    );
    Ok(())
}

#[test]
fn rich_text_tom_oracle_system_msftedit_range_tracking_characterization() {
    std::thread::Builder::new()
        .name("system-msftedit-range-tracking".into())
        .spawn(characterize_range_tracking)
        .expect("start the native passive range tracking probe thread")
        .join()
        .expect("the native passive range tracking probe thread panicked")
        .expect("the native passive range tracking probe failed");
}
