//! An image of the shared memory extension over a System V segment (the
//! port of `XShm/X11ShmImage.cs`).

use crate::lib_c::{shmat, shmctl, shmdt, shmget, IPC_CREAT, IPC_PRIVATE, IPC_RMID};
use crate::xlib::{self, VisualPointer, XDisplay, XID};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::PixelSize;
use std::cell::Cell;
use std::ffi::c_ulong;
use x11_dl::xlib::XImage;
use x11_dl::xshm::XShmSegmentInfo;

pub struct X11ShmImage {
    deferred_display: XDisplay,
    shm_image: Cell<*mut XImage>,
    shm_segment_info: Cell<*mut XShmSegmentInfo>,
    size: PixelSize,
    disposed: Cell<bool>,
}

impl X11ShmImage {
    pub const BYTE_SIZE_OF_PIXEL: i32 = 4;

    /// # Panics
    /// Panics when the image cannot be created or the segment cannot be
    /// attached (`Failed to shmat`, the exception of the reference).
    pub fn new(size: PixelSize, deferred_display: XDisplay, visual: VisualPointer, depth: i32) -> X11ShmImage {
        // The XShmSegmentInfo struct will be stored in XImage, and it must pin the address.
        let shm_segment_info: *mut XShmSegmentInfo = Box::into_raw(Box::new(XShmSegmentInfo {
            shmseg: 0,
            shmid: 0,
            shmaddr: std::ptr::null_mut(),
            readOnly: 0,
        }));

        let width = size.width;
        let height = size.height;

        debug_assert!(depth == 32, "The PixelFormat must be Bgra8888, so that the depth should be 32.");

        // SAFETY: the segment information is a live allocation that outlives the image (it is
        // freed after the image is destroyed, in `dispose`).
        let shm_image =
            unsafe { xlib::x_shm_create_image(deferred_display, visual, depth as u32, shm_segment_info, width as u32, height as u32) };
        if shm_image.is_null() {
            // SAFETY: the allocation was made above and nothing else holds it.
            drop(unsafe { Box::from_raw(shm_segment_info) });
            // The reference dereferences the null image.
            panic!("XShmCreateImage failed");
        }

        let map_length = (width * Self::BYTE_SIZE_OF_PIXEL * height) as usize;
        let shmid = shmget(IPC_PRIVATE, map_length, IPC_CREAT | 0o777);

        let Some(shmaddr) = shmat(shmid, 0) else {
            shmctl(shmid, IPC_RMID);
            // SAFETY: the image was created above and has no data yet; the segment information
            // is freed after the image that refers to it.
            unsafe {
                xlib::x_destroy_image(shm_image);
                drop(Box::from_raw(shm_segment_info));
            }
            panic!("Failed to shmat");
        };

        // SAFETY: both are live allocations made above; the image is given the attached segment
        // as its data, which stays attached until `dispose` detaches it after clearing the
        // pointer here.
        unsafe {
            (*shm_segment_info).shmid = shmid;
            (*shm_segment_info).shmaddr = shmaddr.cast();
            (*shm_image).data = shmaddr.cast();

            xlib::x_shm_attach(deferred_display, shm_segment_info);
        }

        if let Some(logger) = Logger::try_get(LogEventLevel::Debug, LogArea::X11_PLATFORM) {
            logger.log_with_values(
                None,
                "[X11ShmImage] CreateX11ShmImage Size={Size} shmid={Shmid:X} shmaddr={ShmAddr}",
                &[&format!("{}x{}", size.width, size.height), &shmid, &(shmaddr as usize)],
            );
        }

        X11ShmImage {
            deferred_display,
            shm_image: Cell::new(shm_image),
            shm_segment_info: Cell::new(shm_segment_info),
            size,
            disposed: Cell::new(false),
        }
    }

    /// The address of the pixels: null once the image is disposed.
    pub fn shm_addr(&self) -> *mut u8 {
        let info = self.shm_segment_info.get();
        if info.is_null() {
            return std::ptr::null_mut();
        }
        // SAFETY: the segment information is a live allocation until `dispose` clears the pointer.
        unsafe { (*info).shmaddr.cast() }
    }

    pub fn size(&self) -> PixelSize {
        self.size
    }

    /// The identifier of the segment on the server: zero once the image is
    /// disposed.
    pub fn shm_seg(&self) -> c_ulong {
        let info = self.shm_segment_info.get();
        if info.is_null() {
            return 0;
        }
        // SAFETY: as in `shm_addr`.
        unsafe { (*info).shmseg }
    }

    /// Submits this image to the given window via XShmPutImage and requests a completion event. Returns
    /// false if the server rejected the request, in which case no completion event will be delivered.
    pub fn put(&self, window_x_id: XID) -> bool {
        let image = self.shm_image.get();
        if image.is_null() {
            return false;
        }
        let gc = xlib::x_create_gc(self.deferred_display, window_x_id);
        // SAFETY: the image is live (checked above) and its data is the attached segment.
        let status = unsafe {
            xlib::x_shm_put_image(
                self.deferred_display,
                window_x_id,
                gc,
                image,
                0,
                0,
                0,
                0,
                self.size.width as u32,
                self.size.height as u32,
                true,
            )
        };
        xlib::x_flush(self.deferred_display);
        xlib::x_free_gc(self.deferred_display, gc);
        status
    }

    pub fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }

        // Teardown order per the MIT-SHM spec: detach the server, destroy the image, then drop the segment.
        // https://xorg.freedesktop.org/archive/X11R7.7/doc/xextproto/shm.html

        let info = self.shm_segment_info.replace(std::ptr::null_mut());
        let image = self.shm_image.replace(std::ptr::null_mut());
        // SAFETY: both pointers are the live allocations of the constructor, used here for the
        // last time: the pointers of the object were cleared above, so nothing reads the pixels
        // after the segment is detached.
        unsafe {
            xlib::x_shm_detach(self.deferred_display, info);

            // Clear data first so XDestroyImage frees only the XImage structure - the shm segment is ours to drop
            // below; otherwise XDestroyImage would call free() on the shmat() pointer.
            (*image).data = std::ptr::null_mut();
            xlib::x_destroy_image(image);

            shmdt((*info).shmaddr.cast());
            shmctl((*info).shmid, IPC_RMID);

            drop(Box::from_raw(info));
        }

        if let Some(logger) = Logger::try_get(LogEventLevel::Debug, LogArea::X11_PLATFORM) {
            logger.log(None, "[X11ShmImage] Dispose");
        }
    }
}

impl Drop for X11ShmImage {
    /// The reference leaves an image that nobody disposed to the server
    /// and the system until the process ends; here it is released with its
    /// last owner.
    fn drop(&mut self) {
        self.dispose();
    }
}
