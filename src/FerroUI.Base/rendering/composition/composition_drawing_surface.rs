use super::server::{IServerObject, ServerCompositionDrawingSurface, ServerObjectId};
use super::{
    CompositionSurface, Compositor, ICompositionImportedGpuImage, ICompositionImportedGpuSemaphore, ServerJobTask,
};
use std::ops::Deref;
use std::rc::Rc;

/// A composition surface whose content is set from imported GPU images.
#[derive(Clone)]
pub struct CompositionDrawingSurface(Rc<CompositionSurface>);

impl Deref for CompositionDrawingSurface {
    type Target = Rc<CompositionSurface>;

    fn deref(&self) -> &Rc<CompositionSurface> {
        &self.0
    }
}

fn imported_image(image: &dyn ICompositionImportedGpuImage) -> Rc<super::CompositionImportedGpuImage> {
    match image.as_imported_gpu_image() {
        Some(image) => image.rc(),
        None => panic!("the image was not imported by a composition interop"),
    }
}

fn imported_semaphore(semaphore: &dyn ICompositionImportedGpuSemaphore) -> Rc<super::CompositionImportedGpuSemaphore> {
    match semaphore.as_imported_gpu_semaphore() {
        Some(semaphore) => semaphore.rc(),
        None => panic!("the semaphore was not imported by a composition interop"),
    }
}

impl CompositionDrawingSurface {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionDrawingSurface {
        let server = compositor.create_server_object(|compositor, _| ServerCompositionDrawingSurface::new(compositor));
        CompositionDrawingSurface(CompositionSurface::new(compositor, server))
    }

    fn server_id(&self) -> ServerObjectId {
        super::ICompositionObject::server(&*self.0)
    }

    /// Runs `update` on the server surface on the render thread.
    ///
    /// The job names the surface by id. The id stays bound to the server
    /// surface while this object is alive, disposed or not (see
    /// [`CompositionObject`](super::CompositionObject)), so the job reaches
    /// the surface as upstream, where it holds the server object itself; a
    /// disposed surface disposes the new snapshot.
    fn invoke(
        &self,
        update: impl FnOnce(&ServerCompositionDrawingSurface) -> Result<(), Rc<dyn std::error::Error>> + 'static,
    ) -> ServerJobTask<()> {
        self.compositor().invoke_server_object_job_async(
            self.server_id(),
            move |_, server: Option<Rc<dyn IServerObject>>| {
                let server = server.and_then(|server| server.into_any_rc().downcast::<ServerCompositionDrawingSurface>().ok());
                match server {
                    Some(server) => update(&server),
                    None => panic!("the server object of a live drawing surface is not a drawing surface"),
                }
            },
            false,
        )
    }

    /// Updates the surface contents using an imported memory image using a keyed mutex as the means of synchronization
    ///
    /// `acquire_index` is the mutex key to wait for before accessing the
    /// image, `release_index` the mutex key to release for after accessing
    /// the image. Returns a task that completes when update operation is
    /// completed and user code is free to destroy or dispose the image.
    pub fn update_with_keyed_mutex_async(
        &self,
        image: &dyn ICompositionImportedGpuImage,
        acquire_index: u32,
        release_index: u32,
    ) -> ServerJobTask<()> {
        let img = imported_image(image);
        self.invoke(move |server| server.update_with_keyed_mutex(&img, acquire_index, release_index))
    }

    /// Updates the surface contents using an imported memory image using a semaphore pair as the means of synchronization
    ///
    /// `wait_for_semaphore` is the semaphore to wait for before accessing
    /// the image, `signal_semaphore` the semaphore to signal after
    /// accessing the image.
    pub fn update_with_semaphores_async(
        &self,
        image: &dyn ICompositionImportedGpuImage,
        wait_for_semaphore: &dyn ICompositionImportedGpuSemaphore,
        signal_semaphore: &dyn ICompositionImportedGpuSemaphore,
    ) -> ServerJobTask<()> {
        let img = imported_image(image);
        let wait = imported_semaphore(wait_for_semaphore);
        let signal = imported_semaphore(signal_semaphore);
        self.invoke(move |server| server.update_with_semaphores(&img, &wait, &signal))
    }

    /// Updates the surface contents using an imported memory image using a semaphore pair as the means of synchronization
    ///
    /// The semaphores are timeline semaphores: `wait_for_value` is the value
    /// to wait for before accessing the image, `signal_value` the value to
    /// signal after accessing it.
    pub fn update_with_timeline_semaphores_async(
        &self,
        image: &dyn ICompositionImportedGpuImage,
        wait_for_semaphore: &dyn ICompositionImportedGpuSemaphore,
        wait_for_value: u64,
        signal_semaphore: &dyn ICompositionImportedGpuSemaphore,
        signal_value: u64,
    ) -> ServerJobTask<()> {
        let img = imported_image(image);
        let wait = imported_semaphore(wait_for_semaphore);
        let signal = imported_semaphore(signal_semaphore);
        self.invoke(move |server| {
            server.update_with_timeline_semaphores(&img, &wait, wait_for_value, &signal, signal_value)
        })
    }

    /// Updates the surface contents using an unspecified automatic means of synchronization
    /// provided by the underlying platform
    pub fn update_async(&self, image: &dyn ICompositionImportedGpuImage) -> ServerJobTask<()> {
        let img = imported_image(image);
        self.invoke(move |server| server.update_with_automatic_sync(&img))
    }

    pub fn dispose(&self) {
        self.0.dispose()
    }
}
