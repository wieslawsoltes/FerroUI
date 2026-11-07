use crate::metadata::IServiceProvider;
use crate::BoxedValue;
use std::rc::Rc;

/// Represents a deferred content: content that is built on demand, the first
/// time it is requested.
pub trait IDeferredContent {
    /// Builds the deferred content. `service_provider` is the service
    /// provider to build it with, if any; deferred resources are built with
    /// none.
    fn build(&self, service_provider: Option<&Rc<dyn IServiceProvider>>) -> Option<BoxedValue>;

    /// Builds the deferred content, or reports that it could not be built.
    ///
    /// `Err` means that the build failed and that the content reported its error itself: the
    /// deferred content of the markup runtime gives it to the open error scope of the load
    /// (`ferroui_markup_xaml::xaml_error_scope`), which returns it as the error of the load. The
    /// caller then goes on as if the content were not there, and does not keep the failed build:
    /// the next request builds again. Content without an error channel never fails; the default
    /// builds with [`build`](Self::build).
    ///
    /// An addition (#567): where the reference throws out of the build, this reports the error
    /// without unwinding, so that a load can return it with `panic = "abort"`.
    fn try_build(
        &self,
        service_provider: Option<&Rc<dyn IServiceProvider>>,
    ) -> Result<Option<BoxedValue>, DeferredContentFailed> {
        Ok(self.build(service_provider))
    }
}

/// The failure of a build of deferred content
/// ([`IDeferredContent::try_build`]) whose error the content reported itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeferredContentFailed;

impl std::fmt::Display for DeferredContentFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The deferred content could not be built.")
    }
}

impl std::error::Error for DeferredContentFailed {}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn IDeferredContent {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
