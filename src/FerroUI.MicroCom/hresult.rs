use std::fmt;

/// `HRESULT` as it crosses the ABI (`typedef unsigned int HRESULT` in `com.h`).
pub type RawHResult = u32;

pub const S_OK: RawHResult = 0;
pub const E_NOTIMPL: RawHResult = 0x8000_4001;
pub const E_NOINTERFACE: RawHResult = 0x8000_4002;
pub const E_POINTER: RawHResult = 0x8000_4003;
pub const E_ABORT: RawHResult = 0x8000_4004;
pub const E_FAIL: RawHResult = 0x8000_4005;
pub const E_UNEXPECTED: RawHResult = 0x8000_FFFF;
pub const E_HANDLE: RawHResult = 0x8007_0006;
pub const E_INVALIDARG: RawHResult = 0x8007_0057;
pub const COR_E_INVALIDOPERATION: RawHResult = 0x8013_1509;
pub const COR_E_OBJECTDISPOSED: RawHResult = 0x8013_1622;

/// A failed (or, rarely, non-`S_OK` success) `HRESULT`; the error type of
/// every generated method whose IDL signature returns `HRESULT`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HResult(pub RawHResult);

impl HResult {
    pub const NOTIMPL: HResult = HResult(E_NOTIMPL);
    pub const NOINTERFACE: HResult = HResult(E_NOINTERFACE);
    pub const POINTER: HResult = HResult(E_POINTER);
    pub const ABORT: HResult = HResult(E_ABORT);
    pub const FAIL: HResult = HResult(E_FAIL);
    pub const UNEXPECTED: HResult = HResult(E_UNEXPECTED);
    pub const HANDLE: HResult = HResult(E_HANDLE);
    pub const INVALIDARG: HResult = HResult(E_INVALIDARG);
    pub const INVALIDOPERATION: HResult = HResult(COR_E_INVALIDOPERATION);
    pub const OBJECTDISPOSED: HResult = HResult(COR_E_OBJECTDISPOSED);

    /// Maps a raw `HRESULT` to `Ok(())` for `S_OK` and `Err` for anything
    /// else. This mirrors the native sources and MicroCom's C# proxies, which
    /// test `hr != S_OK` rather than the sign bit.
    #[inline]
    pub fn check(raw: RawHResult) -> Result<(), HResult> {
        if raw == S_OK {
            Ok(())
        } else {
            Err(HResult(raw))
        }
    }

    /// Converts a callback result back to the raw value handed to native code.
    #[inline]
    pub fn from_result(r: Result<(), HResult>) -> RawHResult {
        match r {
            Ok(()) => S_OK,
            Err(e) => e.0,
        }
    }

    pub fn name(self) -> Option<&'static str> {
        Some(match self.0 {
            S_OK => "S_OK",
            E_NOTIMPL => "E_NOTIMPL",
            E_NOINTERFACE => "E_NOINTERFACE",
            E_POINTER => "E_POINTER",
            E_ABORT => "E_ABORT",
            E_FAIL => "E_FAIL",
            E_UNEXPECTED => "E_UNEXPECTED",
            E_HANDLE => "E_HANDLE",
            E_INVALIDARG => "E_INVALIDARG",
            COR_E_INVALIDOPERATION => "COR_E_INVALIDOPERATION",
            COR_E_OBJECTDISPOSED => "COR_E_OBJECTDISPOSED",
            _ => return None,
        })
    }
}

impl fmt::Debug for HResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name() {
            Some(n) => write!(f, "HResult({n} 0x{:08X})", self.0),
            None => write!(f, "HResult(0x{:08X})", self.0),
        }
    }
}

impl fmt::Display for HResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name() {
            Some(n) => write!(f, "COM call failed: {n} (0x{:08X})", self.0),
            None => write!(f, "COM call failed: HRESULT 0x{:08X}", self.0),
        }
    }
}

impl std::error::Error for HResult {}
