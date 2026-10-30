//! The port of `SafeArrayRefTests` of the reference's integration tests of
//! Windows: they test types that are internal to the backend, which a test
//! crate of the port cannot reach, so they are tests of the crate. On
//! Windows they run against the automation library of the system; on any
//! other host against the stand-in of `ole_aut.rs`.

use super::safe_array_ref::{SafeArrayRef, SAFEARRAY};
use super::SafeArrayMarshaller;
use crate::automation::interop::{IToggleProvider, IToggleProviderImpl};
use ferroui_microcom::{ComPtr, HResult};
use std::ffi::c_void;

#[test]
fn convert_to_unmanaged_sizes_string_array_to_input() {
    let values = ["foo".to_owned(), "bar".to_owned(), "baz".to_owned()];
    let safe_array = SafeArrayMarshaller::<String>::convert_to_unmanaged(Some(&values)).unwrap();

    assert_eq!(3, get_length(safe_array));

    // SAFETY: the array of the line above, used no more.
    unsafe { SafeArrayMarshaller::<String>::free(safe_array) };
}

#[test]
fn convert_to_unmanaged_sizes_provider_array_to_input() {
    let providers = [test_provider(1), test_provider(2)];
    let safe_array = SafeArrayMarshaller::<ComPtr<IToggleProvider>>::convert_to_unmanaged(Some(&providers)).unwrap();

    assert_eq!(providers.len(), get_length(safe_array));

    // SAFETY: the array of the line above, used no more.
    unsafe { SafeArrayMarshaller::<ComPtr<IToggleProvider>>::free(safe_array) };
}

#[test]
fn convert_to_unmanaged_fills_provider_array_with_com_wrappers() {
    let providers = [test_provider(1), test_provider(2)];
    let safe_array = SafeArrayMarshaller::<ComPtr<IToggleProvider>>::convert_to_unmanaged(Some(&providers)).unwrap();

    // Check the length before reading the entries. A wrongly sized array holds slots that were
    // never written, and dereferencing those below would take down the test host.
    assert_eq!(providers.len(), get_length(safe_array));
    assert!(get_entries(safe_array).iter().all(|entry| !entry.is_null()));

    // SAFETY: the array holds pointers of the interface it was made of.
    let round_tripped = unsafe { SafeArrayMarshaller::<ComPtr<IToggleProvider>>::convert_to_managed(safe_array) }.unwrap();
    let round_tripped = round_tripped.expect("an array");
    assert_eq!(vec![1, 2], round_tripped.iter().map(|x| x.get_toggle_state().unwrap()).collect::<Vec<_>>());

    // SAFETY: the array of this test, used no more.
    unsafe { SafeArrayMarshaller::<ComPtr<IToggleProvider>>::free(safe_array) };

    // Not in the reference: the objects are the ones that went in.
    for (provider, back) in providers.iter().zip(&round_tripped) {
        assert_eq!(provider.as_ptr(), back.as_ptr());
    }
}

// Not from the reference.

#[test]
fn no_values_are_no_array() {
    let safe_array = SafeArrayMarshaller::<i32>::convert_to_unmanaged(None).unwrap();
    assert!(safe_array.is_null());
    // SAFETY: no array.
    assert_eq!(None, unsafe { SafeArrayMarshaller::<i32>::convert_to_managed(safe_array) }.unwrap());
    // SAFETY: no array: nothing is destroyed.
    unsafe { SafeArrayMarshaller::<i32>::free(safe_array) };
}

#[test]
fn an_empty_array_is_an_array_without_elements() {
    let safe_array = SafeArrayMarshaller::<f64>::convert_to_unmanaged(Some(&[])).unwrap();
    assert!(!safe_array.is_null());
    assert_eq!(0, get_length(safe_array));
    // SAFETY: an array of doubles.
    assert_eq!(Some(Vec::<f64>::new()), unsafe { SafeArrayMarshaller::<f64>::convert_to_managed(safe_array) }.unwrap());
    // SAFETY: used no more.
    unsafe { SafeArrayMarshaller::<f64>::free(safe_array) };
}

#[test]
fn the_values_of_every_plain_element_type_round_trip() {
    fn round_trip<T: super::SafeArrayElement + PartialEq + std::fmt::Debug + Clone>(values: &[T], element_size: u32) {
        let safe_array = SafeArrayMarshaller::<T>::convert_to_unmanaged(Some(values)).unwrap();
        assert_eq!(values.len(), get_length(safe_array));
        // SAFETY: a live array.
        assert_eq!(element_size, unsafe { (*safe_array.as_ptr()).cb_elements });
        // SAFETY: an array of the element type it was made of.
        let back = unsafe { SafeArrayMarshaller::<T>::convert_to_managed(safe_array) }.unwrap();
        assert_eq!(Some(values.to_vec()), back);
        // SAFETY: used no more.
        unsafe { SafeArrayMarshaller::<T>::free(safe_array) };
    }

    round_trip(&[i8::MIN, 0, i8::MAX], 1);
    round_trip(&[i16::MIN, 0, i16::MAX], 2);
    round_trip(&[i32::MIN, 3, i32::MAX], 4);
    round_trip(&[i64::MIN, 0, i64::MAX], 8);
    round_trip(&[0u8, 1, u8::MAX], 1);
    round_trip(&[0u16, u16::MAX], 2);
    round_trip(&[0u32, u32::MAX], 4);
    round_trip(&[0u64, u64::MAX], 8);
    round_trip(&[1.5f32, -2.25], 4);
    round_trip(&[1.5f64, -2.25, 1e300], 8);
    round_trip(&[true, false, true], 2);
    round_trip(&[String::new(), "text of several words".to_owned()], std::mem::size_of::<usize>() as u32);
}

#[test]
#[cfg(target_pointer_width = "64")]
fn the_descriptor_of_an_array_has_the_layout_of_the_system() {
    assert_eq!(32, std::mem::size_of::<SAFEARRAY>());
    assert_eq!(2, std::mem::offset_of!(SAFEARRAY, f_features));
    assert_eq!(4, std::mem::offset_of!(SAFEARRAY, cb_elements));
    assert_eq!(8, std::mem::offset_of!(SAFEARRAY, c_locks));
    assert_eq!(16, std::mem::offset_of!(SAFEARRAY, pv_data));
    assert_eq!(24, std::mem::offset_of!(SAFEARRAY, rgsabound));
    assert_eq!(std::mem::size_of::<usize>(), std::mem::size_of::<SafeArrayRef>());
}

fn get_length(safe_array: SafeArrayRef) -> usize {
    // SAFETY: a live array of one dimension.
    unsafe { (*safe_array.as_ptr()).rgsabound[0].c_elements as usize }
}

fn get_entries(safe_array: SafeArrayRef) -> Vec<*mut c_void> {
    let length = get_length(safe_array);
    // SAFETY: a live array of `length` pointers.
    unsafe {
        let data = (*safe_array.as_ptr()).pv_data.cast::<*mut c_void>();
        (0..length).map(|index| *data.add(index)).collect()
    }
}

/// The provider of the reference's test has one method that answers a
/// number; here it is a toggle provider whose state is the number.
struct TestProvider {
    value: i32,
}

impl IToggleProviderImpl for TestProvider {
    fn toggle(&self) -> Result<(), HResult> {
        Ok(())
    }

    fn get_toggle_state(&self) -> Result<i32, HResult> {
        Ok(self.value)
    }
}

fn test_provider(value: i32) -> ComPtr<IToggleProvider> {
    IToggleProvider::from_impl(TestProvider { value })
}

// Regression coverage for allocation/element-conversion failure paths added
// while porting the native TextEdit event's SAFEARRAY(BSTR) payload.

#[test]
fn text_safe_array_failed_element_conversion_releases_initialized_com_references() {
    use super::{SafeArrayElement, SafeArrayLockError, VarEnum};
    use ferroui_microcom::ComObject;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[derive(Clone, Copy)]
    enum Failure { Never, Error, Panic }
    struct Element { provider: ComPtr<IToggleProvider>, failure: Failure }
    impl SafeArrayElement for Element {
        const VAR_TYPE: VarEnum = VarEnum::VT_UNKNOWN;
        type Raw = *mut c_void;
        fn to_raw(&self) -> Self::Raw {
            self.provider.clone().into_raw().cast()
        }
        fn try_to_raw(&self) -> Result<Self::Raw, SafeArrayLockError> {
            match self.failure {
                Failure::Never => Ok(self.to_raw()),
                Failure::Error => Err(SafeArrayLockError(ferroui_microcom::E_FAIL as i32)),
                Failure::Panic => panic!("injected element conversion failure"),
            }
        }
        unsafe fn from_raw(raw: Self::Raw) -> Self {
            // SAFETY: this test's array contains live toggle interfaces.
            Self { provider: unsafe { ComPtr::from_raw_add_ref(raw.cast()) }.unwrap(), failure: Failure::Never }
        }
    }
    for failure in [Failure::Error, Failure::Panic] {
        let provider = test_provider(7);
        let elements = [
            Element { provider: provider.clone(), failure: Failure::Never },
            Element { provider: provider.clone(), failure },
        ];
        // SAFETY: the factory above creates a ComObject<TestProvider>.
        let count = || unsafe { ComObject::<TestProvider>::ref_count(provider.as_ptr().cast()) };
        assert_eq!(count(), 3);
        let result = catch_unwind(AssertUnwindSafe(|| SafeArrayMarshaller::<Element>::convert_to_unmanaged(Some(&elements))));
        match failure {
            Failure::Error => assert_eq!(result.unwrap().unwrap_err(), SafeArrayLockError(ferroui_microcom::E_FAIL as i32)),
            Failure::Panic => assert!(result.is_err()),
            Failure::Never => unreachable!("the loop injects a failing conversion"),
        }
        // The first slot acquired a COM reference before the second failed.
        // The zeroed remainder and initialized slot have both been freed.
        assert_eq!(count(), 3);
        drop(elements);
        assert_eq!(count(), 1);
    }
}

#[test]
fn text_safe_array_rejects_invalid_shape_before_converting_elements() {
    use super::{SafeArrayElement, SafeArrayLockError, VarEnum};
    use std::cell::Cell;
    use std::rc::Rc;

    struct WrongSize(Rc<Cell<usize>>);
    impl SafeArrayElement for WrongSize {
        const VAR_TYPE: VarEnum = VarEnum::VT_UI8;
        type Raw = u16;
        fn to_raw(&self) -> Self::Raw { self.0.set(self.0.get() + 1); 0 }
        unsafe fn from_raw(_: Self::Raw) -> Self { Self(Rc::new(Cell::new(0))) }
    }
    struct UnsupportedType(Rc<Cell<usize>>);
    impl SafeArrayElement for UnsupportedType {
        const VAR_TYPE: VarEnum = VarEnum::VT_EMPTY;
        type Raw = u8;
        fn to_raw(&self) -> Self::Raw { self.0.set(self.0.get() + 1); 0 }
        unsafe fn from_raw(_: Self::Raw) -> Self { Self(Rc::new(Cell::new(0))) }
    }
    let calls = Rc::new(Cell::new(0));
    assert_eq!(SafeArrayMarshaller::<WrongSize>::convert_to_unmanaged(Some(&[WrongSize(calls.clone())])).unwrap_err(), SafeArrayLockError(ferroui_microcom::E_INVALIDARG as i32));
    assert_eq!(calls.get(), 0);
    assert_eq!(SafeArrayMarshaller::<UnsupportedType>::convert_to_unmanaged(Some(&[UnsupportedType(calls.clone())])).unwrap_err(), SafeArrayLockError(0x8007_000Eu32 as i32));
    assert_eq!(calls.get(), 0);
}

#[test]
fn text_safe_array_reader_unwind_releases_the_native_lock() {
    use super::{SafeArrayElement, VarEnum};
    use std::panic::{catch_unwind, AssertUnwindSafe};

    struct PanickingRead;
    impl SafeArrayElement for PanickingRead {
        const VAR_TYPE: VarEnum = VarEnum::VT_UI1;
        type Raw = u8;
        fn to_raw(&self) -> Self::Raw { 0 }
        unsafe fn from_raw(_: Self::Raw) -> Self { panic!("injected element read failure") }
    }
    let array = SafeArrayMarshaller::<u8>::convert_to_unmanaged(Some(&[1])).unwrap();
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: the decoder's raw element type and VAR_TYPE match this
        // array. Only its deliberate conversion failure differs.
        unsafe { SafeArrayMarshaller::<PanickingRead>::convert_to_managed(array) }
    }));
    assert!(result.is_err());
    // SAFETY: this test still owns the same live array after decoding failed.
    unsafe {
        assert_eq!((*array.as_ptr()).c_locks, 0);
        SafeArrayMarshaller::<u8>::free(array);
    }
}
