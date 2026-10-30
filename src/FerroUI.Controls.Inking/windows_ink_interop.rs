//! The interfaces of the Windows Runtime the ink services of Windows call (`Windows.UI.Input.Inking`,
//! the in-memory stream of `Windows.Storage.Streams` and the collections and asynchronous
//! operations of `Windows.Foundation`), and what the services need around them.
//!
//! The bindings are generated from `windows_ink.idl` by the MicroCom generator in the build script
//! of the crate. They are values and vtable calls, so they compile, and their identifiers are
//! tested, on every host; what calls the system is Windows only. Beside them are the pieces a
//! projection of the Windows Runtime provides: activation and strings, awaiting an operation, and
//! the `IIterable<InkPoint>` the stroke builder reads. WinUI has none of this: its ink control
//! calls the ink engine in C++/WinRT.

#![allow(non_camel_case_types)]

#[allow(
    clippy::all,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    missing_docs,
    unused_unsafe,
    unsafe_op_in_unsafe_fn
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/windows_ink.rs"));
}

#[allow(unused_imports)]
pub(crate) use bindings::*;

#[cfg(windows)]
pub(crate) use imp::*;

#[cfg(windows)]
mod imp {
    use super::*;
    use ferroui_microcom::{make_com_with, ComPtr, Guid, HResult, Interface, InterfaceEntry, E_ABORT, E_FAIL};
    use ferroui_win32::win_rt::{HStringInterop, NativeWinRTMethods};
    use std::cell::Cell;
    use std::future::Future;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex, PoisonError};
    use std::task::{Context, Poll, Waker};

    /// `E_BOUNDS`: an index past the end of a collection.
    const E_BOUNDS: u32 = 0x8000_000B;

    /// A new object of the runtime class of the name (its constructor without arguments), as the
    /// interface.
    pub(crate) fn activate<T: Interface>(class_name: &str) -> Result<ComPtr<T>, HResult> {
        NativeWinRTMethods::create_instance::<T>(class_name)
    }

    /// The activation factory, or the interface of the static members, of the runtime class of
    /// the name.
    pub(crate) fn factory<T: Interface>(class_name: &str) -> Result<ComPtr<T>, HResult> {
        NativeWinRTMethods::create_activation_factory::<T>(class_name)
    }

    /// The object a method returned: a null reference, which the methods the services call do
    /// not return on success, is `E_POINTER`.
    pub(crate) fn object<T: Interface>(result: Result<Option<ComPtr<T>>, HResult>) -> Result<ComPtr<T>, HResult> {
        result?.ok_or(HResult::POINTER)
    }

    /// The text of a string handle a method returned, whose ownership is the caller's: the handle
    /// is deleted. The null handle is the empty string.
    pub(crate) fn take_string(handle: isize) -> String {
        HStringInterop::from_handle(handle, true).value().unwrap_or_default()
    }

    /// The `boolean` of the ABI (one byte, zero or one).
    pub(crate) fn to_abi(value: bool) -> u8 {
        u8::from(value)
    }

    /// The value of a `boolean` of the ABI: anything but zero is true.
    pub(crate) fn from_abi(value: u8) -> bool {
        value != 0
    }

    // Awaiting an operation.

    /// The status of an operation as its completion handler reports it, and the waker of the task
    /// that waits for it. The handler is agile: the operation calls it on the thread it completes
    /// on, and all it does there is record the status and wake the task.
    #[derive(Default)]
    pub(crate) struct Completion {
        status: Option<AsyncStatus>,
        waker: Option<Waker>,
    }

    type SharedCompletion = Arc<Mutex<Completion>>;

    fn complete(completion: &SharedCompletion, status: AsyncStatus) {
        let waker = {
            let mut completion = completion.lock().unwrap_or_else(PoisonError::into_inner);
            completion.status = Some(status);
            completion.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    /// An asynchronous operation of the bindings.
    pub(crate) trait AsyncOperation: Interface {
        type Output;

        /// Gives the operation a completion handler that records the status.
        fn set_completion(&self, completion: SharedCompletion) -> Result<(), HResult>;

        /// `GetResults`.
        fn results(&self) -> Result<Self::Output, HResult>;
    }

    macro_rules! async_operation {
        ($operation:ident, $handler:ident, $handler_impl:ident, $output:ty) => {
            impl AsyncOperation for $operation {
                type Output = $output;

                fn set_completion(&self, completion: SharedCompletion) -> Result<(), HResult> {
                    struct Handler(SharedCompletion);

                    impl $handler_impl for Handler {
                        fn invoke(&self, _async_info: Option<&$operation>, async_status: AsyncStatus) -> Result<(), HResult> {
                            complete(&self.0, async_status);
                            Ok(())
                        }
                    }

                    impl IAgileObjectImpl for Handler {}

                    let handler: ComPtr<$handler> =
                        make_com_with(Handler(completion), [InterfaceEntry::of::<IAgileObject>()]);
                    self.set_completed(Some(&handler))
                }

                fn results(&self) -> Result<$output, HResult> {
                    self.get_results()
                }
            }
        };
    }

    async_operation!(
        IAsyncOperationUInt32,
        AsyncOperationCompletedHandlerUInt32,
        AsyncOperationCompletedHandlerUInt32Impl,
        u32
    );
    async_operation!(
        IAsyncOperationWithProgressUInt32UInt32,
        AsyncOperationWithProgressCompletedHandlerUInt32UInt32,
        AsyncOperationWithProgressCompletedHandlerUInt32UInt32Impl,
        u32
    );
    async_operation!(
        IAsyncActionWithProgressUInt64,
        AsyncActionWithProgressCompletedHandlerUInt64,
        AsyncActionWithProgressCompletedHandlerUInt64Impl,
        ()
    );
    async_operation!(
        IAsyncOperationVectorViewInkRecognitionResult,
        AsyncOperationCompletedHandlerVectorViewInkRecognitionResult,
        AsyncOperationCompletedHandlerVectorViewInkRecognitionResultImpl,
        Option<ComPtr<IVectorViewInkRecognitionResult>>
    );

    /// Awaits an operation: its results, or the error it ended with. Nothing blocks: the task is
    /// woken by the completion handler.
    pub(crate) fn wait<O: AsyncOperation>(operation: ComPtr<O>) -> AsyncOperationFuture<O> {
        AsyncOperationFuture { operation, completion: None }
    }

    pub(crate) struct AsyncOperationFuture<O: AsyncOperation> {
        operation: ComPtr<O>,
        completion: Option<SharedCompletion>,
    }

    impl<O: AsyncOperation> Unpin for AsyncOperationFuture<O> {}

    impl<O: AsyncOperation> Future for AsyncOperationFuture<O> {
        type Output = Result<O::Output, HResult>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let completion = match &self.completion {
                Some(completion) => completion.clone(),
                None => {
                    let completion: SharedCompletion = Arc::default();
                    completion.lock().unwrap_or_else(PoisonError::into_inner).waker = Some(cx.waker().clone());
                    self.completion = Some(completion.clone());
                    // An operation that has completed already calls the handler before this
                    // returns.
                    if let Err(error) = self.operation.set_completion(completion.clone()) {
                        return Poll::Ready(Err(error));
                    }
                    completion
                }
            };

            let status = {
                let mut state = completion.lock().unwrap_or_else(PoisonError::into_inner);
                if state.status.is_none() {
                    state.waker = Some(cx.waker().clone());
                }
                state.status
            };
            match status {
                None => Poll::Pending,
                Some(AsyncStatus::Completed) => Poll::Ready(self.operation.results()),
                Some(AsyncStatus::Canceled) => Poll::Ready(Err(HResult(E_ABORT))),
                Some(_) => {
                    let error = self
                        .operation
                        .cast::<IAsyncInfo>()
                        .and_then(|info| info.error_code())
                        .map(|code| if code < 0 { HResult(code as u32) } else { HResult(E_FAIL) })
                        .unwrap_or(HResult(E_FAIL));
                    Poll::Ready(Err(error))
                }
            }
        }
    }

    // The points of a stroke the stroke builder reads.

    /// What an object of the services answers to `IInspectable::GetIids`: the identifiers of its
    /// interfaces, in memory of the COM allocator, which the caller frees.
    ///
    /// # Safety
    /// `iid_count` must be the out parameter of the count of the call: valid for a write.
    unsafe fn iids(interfaces: &[Guid], iid_count: *mut u32) -> Result<*mut Guid, HResult> {
        use windows_sys::Win32::System::Com::CoTaskMemAlloc;
        let size = std::mem::size_of_val(interfaces);
        // SAFETY: an allocation of the size of the identifiers.
        let memory = unsafe { CoTaskMemAlloc(size.max(1)) }.cast::<Guid>();
        if memory.is_null() {
            return Err(HResult(0x8007_000E)); // E_OUTOFMEMORY
        }
        // SAFETY: the block has room for every identifier; the count is valid for a write by the
        // contract of this function.
        unsafe {
            for (index, interface) in interfaces.iter().enumerate() {
                memory.add(index).write(*interface);
            }
            iid_count.write(interfaces.len() as u32);
        }
        Ok(memory)
    }

    macro_rules! inspectable {
        ($type:ty, $class_name:expr, $interface:ty) => {
            impl IInspectableImpl for $type {
                fn get_iids(&self, iid_count: *mut u32) -> Result<*mut Guid, HResult> {
                    // SAFETY: the out parameter of the call.
                    unsafe { iids(&[<$interface as Interface>::IID], iid_count) }
                }

                fn get_runtime_class_name(&self) -> Result<isize, HResult> {
                    NativeWinRTMethods::windows_create_string($class_name)
                }

                fn get_trust_level(&self) -> Result<TrustLevel, HResult> {
                    Ok(TrustLevel::BaseTrust)
                }
            }
        };
    }

    /// `IIterable<InkPoint>` over the points of a stroke, which
    /// `InkStrokeBuilder.CreateStrokeFromInkPoints` takes (a projection of the Windows Runtime
    /// hands it a collection of its own). The builder reads it during the call, on the thread of
    /// the call.
    pub(crate) struct InkPointIterable {
        points: Rc<Vec<ComPtr<IInkPoint>>>,
    }

    impl InkPointIterable {
        pub(crate) fn new(points: Vec<ComPtr<IInkPoint>>) -> ComPtr<IIterableInkPoint> {
            make_com_with(InkPointIterable { points: Rc::new(points) }, [])
        }
    }

    inspectable!(
        InkPointIterable,
        "Windows.Foundation.Collections.IIterable`1<Windows.UI.Input.Inking.InkPoint>",
        IIterableInkPoint
    );

    impl IIterableInkPointImpl for InkPointIterable {
        fn first(&self) -> Result<Option<ComPtr<IIteratorInkPoint>>, HResult> {
            Ok(Some(make_com_with(InkPointIterator { points: self.points.clone(), index: Cell::new(0) }, [])))
        }
    }

    /// `IIterator<InkPoint>` of [`InkPointIterable`].
    struct InkPointIterator {
        points: Rc<Vec<ComPtr<IInkPoint>>>,
        index: Cell<usize>,
    }

    inspectable!(
        InkPointIterator,
        "Windows.Foundation.Collections.IIterator`1<Windows.UI.Input.Inking.InkPoint>",
        IIteratorInkPoint
    );

    impl IIteratorInkPointImpl for InkPointIterator {
        fn current(&self) -> Result<Option<ComPtr<IInkPoint>>, HResult> {
            self.points.get(self.index.get()).cloned().map(Some).ok_or(HResult(E_BOUNDS))
        }

        fn has_current(&self) -> Result<u8, HResult> {
            Ok(to_abi(self.index.get() < self.points.len()))
        }

        fn move_next(&self) -> Result<u8, HResult> {
            let next = (self.index.get() + 1).min(self.points.len());
            self.index.set(next);
            Ok(to_abi(next < self.points.len()))
        }

        fn get_many(&self, items_size: u32, items: *mut *mut IInkPoint) -> Result<u32, HResult> {
            let start = self.index.get();
            let mut count = 0u32;
            for point in self.points.iter().skip(start).take(items_size as usize) {
                // SAFETY: the array of the caller, of `items_size` places; the caller owns the
                // reference written to it.
                unsafe { items.add(count as usize).write(point.clone().into_raw()) };
                count += 1;
            }
            self.index.set(start + count as usize);
            Ok(count)
        }
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the bindings on every host.
    //!
    //! The identifier of an instance of a parameterized interface or delegate is derived by the
    //! Windows Runtime from the signature of the instance (a version 5 UUID of the namespace
    //! `11f47ad5-7b73-42c0-abae-878b1e16adee` and the signature as UTF-8), as the tests of the
    //! bindings of the WinUI integration derive theirs. The interface definition file declares
    //! each instance with the identifier and, in the comment above it, the signature: the test
    //! derives every identifier again from its signature.

    use super::*;
    use ferroui_microcom::{Guid, Interface};

    /// SHA-1 (FIPS 180-4), which the derivation of the identifiers uses.
    fn sha1(data: &[u8]) -> [u8; 20] {
        let mut h: [u32; 5] = [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476, 0xC3D2_E1F0];
        let mut message = data.to_vec();
        let bit_length = (data.len() as u64) * 8;
        message.push(0x80);
        while message.len() % 64 != 56 {
            message.push(0);
        }
        message.extend_from_slice(&bit_length.to_be_bytes());
        for chunk in message.chunks(64) {
            let mut w = [0u32; 80];
            for (i, word) in chunk.chunks(4).enumerate() {
                w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
            }
            for i in 16..80 {
                w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
            }
            let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
            for (i, word) in w.iter().enumerate() {
                let (f, k) = match i {
                    0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
                    20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                    40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                    _ => (b ^ c ^ d, 0xCA62_C1D6),
                };
                let temp = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*word);
                e = d;
                d = c;
                c = b.rotate_left(30);
                b = a;
                a = temp;
            }
            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
        }
        let mut digest = [0u8; 20];
        for (i, word) in h.iter().enumerate() {
            digest[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        digest
    }

    /// The identifier of a parameterized instance with the signature.
    fn parameterized_iid(signature: &str) -> Guid {
        let namespace = 0x11f47ad5_7b73_42c0_abae_878b1e16adee_u128.to_be_bytes();
        let mut data = namespace.to_vec();
        data.extend_from_slice(signature.as_bytes());
        let hash = sha1(&data);
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&hash[..16]);
        bytes[6] = (bytes[6] & 0x0F) | 0x50;
        bytes[8] = (bytes[8] & 0x3F) | 0x80;
        Guid::from_u128(u128::from_be_bytes(bytes))
    }

    #[test]
    fn the_identifiers_of_known_instances() {
        // IVectorView<String> and IAsyncOperation<UInt32> of the Windows SDK headers
        // (__FIVectorView_1_HSTRING, __FIAsyncOperation_1_UINT32).
        assert_eq!(
            Guid::parse("2f13c006-a03a-5f69-b090-75a43e33423e").unwrap(),
            parameterized_iid("pinterface({bbe1fa4c-b0e3-4583-baef-1f1b2e483e56};string)")
        );
        assert_eq!(Guid::parse("ef60385f-be78-584b-aaef-7829ada2b0de").unwrap(), IAsyncOperationUInt32::IID);
    }

    #[test]
    fn every_parameterized_instance_has_the_identifier_of_its_signature() {
        let idl = include_str!("windows_ink.idl");
        let lines: Vec<&str> = idl.lines().collect();
        let mut checked = 0;
        for (index, line) in lines.iter().enumerate() {
            let Some(comment) = line.strip_prefix("// ") else { continue };
            let Some((_, signature)) = comment.split_once(": ") else { continue };
            if !signature.starts_with("pinterface(") {
                continue;
            }
            let uuid_line = lines[index + 1];
            let uuid = uuid_line.trim().strip_prefix("[uuid(").and_then(|rest| rest.strip_suffix(")]")).unwrap();
            let declared = Guid::parse(&uuid.to_lowercase()).unwrap();
            assert_eq!(declared, parameterized_iid(signature), "{}", lines[index + 2]);
            checked += 1;
        }
        // Every instance the services declare.
        assert_eq!(15, checked);
    }

    #[test]
    fn the_bindings_have_the_identifiers_of_the_metadata() {
        assert_eq!(Guid::parse("af86e2e0-b12d-4c6a-9c5a-d7aa65101e90").unwrap(), IInspectable::IID);
        assert_eq!(Guid::parse("22accbc6-faa9-4f14-b68c-f6cee670ae16").unwrap(), IInkStrokeContainer::IID);
        assert_eq!(Guid::parse("3d07bea5-baea-4c82-a719-7b83da1067d2").unwrap(), IInkStrokeContainer3::IID);
        assert_eq!(Guid::parse("15144d60-cce3-4fcf-9d52-11518ab6afd4").unwrap(), IInkStroke::IID);
        assert_eq!(Guid::parse("905a0fe1-bc53-11df-8c49-001e4fc686da").unwrap(), IRandomAccessStream::IID);
    }

    #[test]
    fn the_structures_have_the_layout_of_the_abi() {
        use std::mem::size_of;
        assert_eq!(8, size_of::<Point>());
        assert_eq!(8, size_of::<Size>());
        assert_eq!(16, size_of::<Rect>());
        assert_eq!(4, size_of::<Color>());
        assert_eq!(24, size_of::<Matrix3x2>());
        assert_eq!(4, size_of::<PenTipShape>());
        assert_eq!(4, size_of::<InkPersistenceFormat>());
    }
}
