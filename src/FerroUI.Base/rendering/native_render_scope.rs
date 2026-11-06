//! A lexical native frame scope, including jobs, snapshots and retirement.
//!
//! A borrowed graphics-context token need not be disposed in stack order, so
//! it cannot own an Objective-C autorelease pool. A synchronous compositor
//! callback provides the required thread-local, strictly nested lifetime.
pub(crate) fn run<T>(render: impl FnOnce() -> T) -> T {
    #[cfg(target_vendor = "apple")]
    let _pool = apple::Pool::new();
    render()
}

#[cfg(target_vendor = "apple")]
mod apple {
    use std::{ffi::c_void, marker::PhantomData, rc::Rc};
    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_autoreleasePoolPush() -> *mut c_void;
        fn objc_autoreleasePoolPop(pool: *mut c_void);
    }
    pub(super) struct Pool(*mut c_void, PhantomData<Rc<()>>);
    impl Pool {
        pub(super) fn new() -> Self {
            // SAFETY: pushes a pool on the calling thread. The guard cannot
            // move to another thread and remains local to synchronous run().
            Self(unsafe { objc_autoreleasePoolPush() }, PhantomData)
        }
    }
    impl Drop for Pool {
        fn drop(&mut self) {
            // SAFETY: lexical nesting in run() balances this thread's push,
            // including unwinding. No context/disposable token owns the pool.
            unsafe { objc_autoreleasePoolPop(self.0) }
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::run;
    use std::ffi::{c_char, c_void};
    #[link(name = "Foundation", kind = "framework")]
    unsafe extern "C" {}
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend();
        fn objc_retain(object: *mut c_void) -> *mut c_void;
        fn objc_release(object: *mut c_void);
    }
    #[test]
    fn frame_scope_drains_autoreleases_in_nested_calls_and_on_unwind() {
        // SAFETY: known NSObject methods through their exact message ABI. Each
        // object has a separate +1 reference until its post-pool count is read.
        unsafe {
            let send: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let count: unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            for unwind in [false, true] {
                let mut object = std::ptr::null_mut();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run(|| {
                        object = send(
                            objc_getClass(c"NSObject".as_ptr()),
                            sel_registerName(c"new".as_ptr()),
                        );
                        objc_retain(object);
                        send(object, sel_registerName(c"autorelease".as_ptr()));
                        run(|| {
                            assert_eq!(count(object, sel_registerName(c"retainCount".as_ptr())), 2)
                        });
                        if unwind {
                            panic!("test frame unwind");
                        }
                    })
                }));
                assert_eq!(result.is_err(), unwind);
                assert_eq!(count(object, sel_registerName(c"retainCount".as_ptr())), 1);
                objc_release(object);
            }
        }
    }
}
