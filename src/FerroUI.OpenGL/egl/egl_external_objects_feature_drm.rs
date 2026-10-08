//! The second file of the partial class `EglExternalObjectsFeature`: the import of dma-buf
//! images through `EGL_EXT_image_dma_buf_import` and `GL_OES_EGL_image`.

use super::egl_consts::*;
use super::egl_external_objects_feature::EglExternalObjectsFeature;
use super::{EglContext, EglImage};
use crate::gl_consts::*;
use crate::{IGlContext, IGlExternalImageTexture, OpenGlException};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{
    IPlatformGraphicsContext, IPlatformHandle, PlatformGraphicsDrmFormat, PlatformGraphicsExternalImageDmaBufProperties,
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties,
};
use ferroui_base::reactive::IDisposable;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::ptr;
use std::rc::Rc;

/// The fields this file of the partial class declares.
pub(super) struct DmaBufImport {
    dma_buf_supported: bool,
    has_modifiers: bool,
    dma_buf_formats_queried: Cell<bool>,
    dma_buf_formats: RefCell<Option<Vec<PlatformGraphicsDrmFormat>>>,
}

impl DmaBufImport {
    fn new(dma_buf_supported: bool, has_modifiers: bool) -> Self {
        Self {
            dma_buf_supported,
            has_modifiers,
            dma_buf_formats_queried: Cell::new(false),
            dma_buf_formats: RefCell::new(None),
        }
    }

    /// Whether the context imports dma-buf images (what `TryInitializeDrm` returns).
    pub(super) fn dma_buf_supported(&self) -> bool {
        self.dma_buf_supported
    }
}

/// The failure of an attribute of a plane the extension has no name for
/// (`ArgumentOutOfRangeException(nameof(plane))`).
fn plane_out_of_range() -> OpenGlException {
    OpenGlException::new("Specified argument was out of the range of valid values. (Parameter 'plane')")
}

fn log_error(message: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
        logger.log(None, message);
    }
}

impl EglExternalObjectsFeature {
    /// Finds out whether the context imports dma-buf images, and with modifiers. The
    /// context is current.
    // The original catches and logs an exception of the queries ("Unable to initialize EGL
    // dma-buf import feature: ..."); the queries used here answer without failing, so there
    // is nothing to catch.
    pub(super) fn try_initialize_drm(context: &Rc<EglContext>) -> DmaBufImport {
        let egl = context.display().egl_interface();
        if !egl.is_create_image_khr_available() || !egl.is_destroy_image_khr_available() {
            return DmaBufImport::new(false, false);
        }

        let egl_extensions = match egl.query_string(context.display().handle(), EGL_EXTENSIONS) {
            Some(egl_extensions) if egl_extensions.contains("EGL_EXT_image_dma_buf_import") => egl_extensions,
            _ => return DmaBufImport::new(false, false),
        };

        if !context.gl_interface().get_extensions().iter().any(|extension| extension == "GL_OES_EGL_image") {
            return DmaBufImport::new(false, false);
        }

        let has_modifiers = egl_extensions.contains("EGL_EXT_image_dma_buf_import_modifiers");
        DmaBufImport::new(true, has_modifiers)
    }

    pub(super) fn get_supported_dma_buf_image_formats() -> Vec<PlatformGraphicsExternalImageFormat> {
        vec![PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm, PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm]
    }

    /// The DRM formats and modifiers the display imports (`SupportedDmaBufFormats`), asked
    /// for once; `None` when they cannot be enumerated.
    pub(super) fn get_supported_dma_buf_formats(&self) -> Option<Vec<PlatformGraphicsDrmFormat>> {
        if !self.drm.dma_buf_formats_queried.get() {
            // A feature that outlived its context has nothing to ask.
            let context = self.context.upgrade()?;
            let lock = context.display().lock();
            if !self.drm.dma_buf_formats_queried.get() {
                let formats = self.try_query_dma_buf_formats(&context);
                *self.drm.dma_buf_formats.borrow_mut() = formats;
                self.drm.dma_buf_formats_queried.set(true);
            }
            lock.dispose();
        }
        self.drm.dma_buf_formats.borrow().clone()
    }

    fn try_query_dma_buf_formats(&self, context: &EglContext) -> Option<Vec<PlatformGraphicsDrmFormat>> {
        let egl = context.display().egl_interface();
        let display = context.display().handle();
        if !self.drm.dma_buf_supported
            || !self.drm.has_modifiers
            || !egl.is_query_dma_buf_formats_ext_available()
            || !egl.is_query_dma_buf_modifiers_ext_available()
        {
            return None;
        }

        // The original reads the arrays at the counts the second query of each kind
        // returns, and catches and logs the exception of an index beyond them.
        let index_out_of_range = || {
            log_error("Unable to enumerate EGL dma-buf import formats: Index was outside the bounds of the array.");
            None
        };

        let mut format_count = 0;
        // SAFETY: no formats are asked for, so the null buffer is not written to.
        if !unsafe { egl.query_dma_buf_formats_ext(display, 0, ptr::null_mut(), &mut format_count) } || format_count <= 0 {
            return None;
        }

        let mut formats = vec![0i32; format_count as usize];
        // SAFETY: the buffer holds `format_count` values.
        if !unsafe { egl.query_dma_buf_formats_ext(display, format_count, formats.as_mut_ptr(), &mut format_count) } {
            return None;
        }

        let mut result = Vec::new();
        for f in 0..format_count.max(0) as usize {
            let Some(&format) = formats.get(f) else {
                return index_out_of_range();
            };
            let mut modifier_count = 0;
            // SAFETY: no modifiers are asked for, so the null buffers are not written to.
            if !unsafe {
                egl.query_dma_buf_modifiers_ext(display, format, 0, ptr::null_mut(), ptr::null_mut(), &mut modifier_count)
            } {
                continue;
            }
            if modifier_count <= 0 {
                // No explicit modifiers reported: the format imports with the implicit layout.
                result.push(PlatformGraphicsDrmFormat::new(
                    format as u32,
                    PlatformGraphicsExternalImageDmaBufProperties::DRM_MODIFIER_INVALID,
                ));
                continue;
            }

            let mut modifiers = vec![0u64; modifier_count as usize];
            // EGLBoolean is 4 bytes; back the external-only buffer with ints.
            let mut external_only = vec![0u32; modifier_count as usize];
            // SAFETY: both buffers hold `modifier_count` values.
            if !unsafe {
                egl.query_dma_buf_modifiers_ext(
                    display,
                    format,
                    modifier_count,
                    modifiers.as_mut_ptr(),
                    external_only.as_mut_ptr(),
                    &mut modifier_count,
                )
            } {
                continue;
            }

            for m in 0..modifier_count.max(0) as usize {
                let (Some(&modifier), Some(&external)) = (modifiers.get(m), external_only.get(m)) else {
                    return index_out_of_range();
                };
                // External-only modifiers can only be sampled via GL_TEXTURE_EXTERNAL_OES;
                // the import path binds GL_TEXTURE_2D, so they are not usable here.
                if external == 0 {
                    result.push(PlatformGraphicsDrmFormat::new(format as u32, modifier));
                }
            }
        }

        Some(result)
    }

    /// Imports a dma-buf image as a texture of the context, which is current.
    ///
    /// # Panics
    /// Panics when the properties name fewer file descriptors, offsets or strides than
    /// planes, or when the handle is not a file descriptor (the index and overflow
    /// exceptions of the original).
    pub(super) fn import_dma_buf_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Result<Rc<dyn IGlExternalImageTexture>, OpenGlException> {
        if !self.drm.dma_buf_supported {
            return Err(Self::not_supported(&*handle));
        }

        let Some(dma_buf) = properties.dma_buf_properties.clone() else {
            return Err(OpenGlException::new("DmaBufProperties must be set for dma-buf imports"));
        };

        let Some(context) = self.context.upgrade() else {
            return Err(OpenGlException::new("Cannot access a disposed object. Object name: 'EglContext'."));
        };

        let plane_count = if dma_buf.plane_count > 0 { dma_buf.plane_count } else { 1 };

        let mut attribs = vec![
            EGL_WIDTH,
            properties.width,
            EGL_HEIGHT,
            properties.height,
            EGL_LINUX_DRM_FOURCC_EXT,
            dma_buf.drm_format as i32,
        ];

        for p in 0..plane_count {
            let plane = p as usize;
            let fd = match &dma_buf.plane_fds {
                Some(fds) => fds[plane],
                None => i32::try_from(handle.handle()).expect("the handle is a file descriptor"),
            };
            let offset = match &dma_buf.plane_offsets {
                Some(offsets) => offsets[plane] as i32,
                None => properties.memory_offset as i32,
            };
            let pitch = match &dma_buf.plane_strides {
                Some(strides) => strides[plane] as i32,
                None => 0,
            };

            attribs.push(Self::plane_fd_attrib(p)?);
            attribs.push(fd);
            attribs.push(Self::plane_offset_attrib(p)?);
            attribs.push(offset);
            attribs.push(Self::plane_pitch_attrib(p)?);
            attribs.push(pitch);

            if self.drm.has_modifiers
                && dma_buf.drm_modifier != PlatformGraphicsExternalImageDmaBufProperties::DRM_MODIFIER_INVALID
            {
                attribs.push(Self::plane_modifier_lo_attrib(p)?);
                attribs.push((dma_buf.drm_modifier & 0xFFFF_FFFF) as i32);
                attribs.push(Self::plane_modifier_hi_attrib(p)?);
                attribs.push((dma_buf.drm_modifier >> 32) as i32);
            }
        }

        attribs.push(EGL_NONE);

        let display = context.display();
        let lock = display.lock();
        let image_handle = display.egl_interface().create_image_khr(display.handle(), 0, EGL_LINUX_DMA_BUF_EXT, 0, &attribs);
        lock.dispose();

        if image_handle == 0 {
            return Err(OpenGlException::new("eglCreateImageKHR failed to import the dma-buf"));
        }

        let egl_image = EglImage::new(display, image_handle);

        let gl = context.gl_interface();
        let old_texture = gl.get_integerv(GL_TEXTURE_BINDING_2D);
        let texture = gl.gen_texture();
        let bound = (|| {
            gl.bind_texture(GL_TEXTURE_2D, texture);
            // Calling an entry point the context lacks is an exception of the original,
            // which is caught below like any other.
            if !gl.is_egl_image_target_texture_2d_oes_available() {
                return Err(OpenGlException::new("Unable to find an entry point named 'glEGLImageTargetTexture2DOES'."));
            }
            // SAFETY: the image is one `eglCreateImageKHR` has just returned for the display
            // of the context, and it is alive: it is destroyed below or by the texture.
            unsafe { gl.egl_image_target_texture_2d_oes(GL_TEXTURE_2D, egl_image.handle() as *mut c_void) };
            let err = gl.get_error();
            if err != 0 {
                return Err(OpenGlException::get_formatted_exception_for_code("glEGLImageTargetTexture2DOES", err));
            }

            // The imported texture has no mip levels; ensure it is sampling-complete.
            gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
            Ok(())
        })();
        if let Err(error) = bound {
            gl.bind_texture(GL_TEXTURE_2D, old_texture);
            gl.delete_texture(texture);
            egl_image.dispose();
            return Err(error);
        }

        gl.bind_texture(GL_TEXTURE_2D, old_texture);
        Ok(Rc::new(DmaBufImageTexture {
            context,
            image: RefCell::new(Some(egl_image)),
            texture: Cell::new(texture),
            properties,
        }))
    }

    fn plane_fd_attrib(plane: i32) -> Result<i32, OpenGlException> {
        match plane {
            0 => Ok(EGL_DMA_BUF_PLANE0_FD_EXT),
            1 => Ok(EGL_DMA_BUF_PLANE1_FD_EXT),
            2 => Ok(EGL_DMA_BUF_PLANE2_FD_EXT),
            3 => Ok(EGL_DMA_BUF_PLANE3_FD_EXT),
            _ => Err(plane_out_of_range()),
        }
    }

    fn plane_offset_attrib(plane: i32) -> Result<i32, OpenGlException> {
        match plane {
            0 => Ok(EGL_DMA_BUF_PLANE0_OFFSET_EXT),
            1 => Ok(EGL_DMA_BUF_PLANE1_OFFSET_EXT),
            2 => Ok(EGL_DMA_BUF_PLANE2_OFFSET_EXT),
            3 => Ok(EGL_DMA_BUF_PLANE3_OFFSET_EXT),
            _ => Err(plane_out_of_range()),
        }
    }

    fn plane_pitch_attrib(plane: i32) -> Result<i32, OpenGlException> {
        match plane {
            0 => Ok(EGL_DMA_BUF_PLANE0_PITCH_EXT),
            1 => Ok(EGL_DMA_BUF_PLANE1_PITCH_EXT),
            2 => Ok(EGL_DMA_BUF_PLANE2_PITCH_EXT),
            3 => Ok(EGL_DMA_BUF_PLANE3_PITCH_EXT),
            _ => Err(plane_out_of_range()),
        }
    }

    fn plane_modifier_lo_attrib(plane: i32) -> Result<i32, OpenGlException> {
        match plane {
            0 => Ok(EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT),
            1 => Ok(EGL_DMA_BUF_PLANE1_MODIFIER_LO_EXT),
            2 => Ok(EGL_DMA_BUF_PLANE2_MODIFIER_LO_EXT),
            3 => Ok(EGL_DMA_BUF_PLANE3_MODIFIER_LO_EXT),
            _ => Err(plane_out_of_range()),
        }
    }

    fn plane_modifier_hi_attrib(plane: i32) -> Result<i32, OpenGlException> {
        match plane {
            0 => Ok(EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT),
            1 => Ok(EGL_DMA_BUF_PLANE1_MODIFIER_HI_EXT),
            2 => Ok(EGL_DMA_BUF_PLANE2_MODIFIER_HI_EXT),
            3 => Ok(EGL_DMA_BUF_PLANE3_MODIFIER_HI_EXT),
            _ => Err(plane_out_of_range()),
        }
    }
}

/// A texture of the context that samples an imported dma-buf image.
struct DmaBufImageTexture {
    context: Rc<EglContext>,
    image: RefCell<Option<EglImage>>,
    texture: Cell<i32>,
    properties: PlatformGraphicsExternalImageProperties,
}

impl IGlExternalImageTexture for DmaBufImageTexture {
    fn dispose(&self) {
        if self.context.is_lost() {
            return;
        }
        let current = self.context.ensure_current();
        if self.texture.get() != 0 {
            self.context.gl_interface().delete_texture(self.texture.get());
            self.texture.set(0);
        }
        let image = self.image.borrow_mut().take();
        if let Some(image) = image {
            image.dispose();
        }
        current.dispose();
    }

    fn acquire_keyed_mutex(&self, _key: u32) {
        panic!("Specified method is not supported.");
    }

    fn release_keyed_mutex(&self, _key: u32) {
        panic!("Specified method is not supported.");
    }

    fn texture_id(&self) -> i32 {
        self.texture.get()
    }

    fn internal_format(&self) -> i32 {
        GL_RGBA8
    }

    fn texture_type(&self) -> i32 {
        GL_TEXTURE_2D
    }

    fn properties(&self) -> PlatformGraphicsExternalImageProperties {
        self.properties.clone()
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream project has no tests of the feature. They cover both files of
    // the class, on an EGL library and an OpenGL implementation that are scripted.
    use super::*;
    use crate::egl::{EglDisplay, EglDisplayOptions, EglInterface};
    use crate::testing::FakeGl;
    use crate::{GetProcAddress, GlProfileType, GlVersion, IGlContextExternalObjectsFeature};
    use ferroui_base::platform::{
        IOptionalFeatureProvider, KnownPlatformGraphicsExternalImageHandleTypes, PlatformHandle,
    };
    use ferroui_base::rendering::composition::CompositionGpuImportedImageSynchronizationCapabilities;
    use ferroui_base::PixelSize;
    use std::any::TypeId;
    use std::collections::{HashMap, HashSet};
    use std::ffi::{c_char, CStr, CString};

    const DISPLAY: isize = 5;
    const CONTEXT: isize = 99;
    const DMA_BUF: &str = KnownPlatformGraphicsExternalImageHandleTypes::DMA_BUF_FILE_DESCRIPTOR;
    const SURFACELESS: &str = "EGL_KHR_surfaceless_context";
    const IMPORT: &str = "EGL_KHR_surfaceless_context EGL_EXT_image_dma_buf_import";
    const IMPORT_WITH_MODIFIERS: &str =
        "EGL_KHR_surfaceless_context EGL_EXT_image_dma_buf_import EGL_EXT_image_dma_buf_import_modifiers";

    /// The script and the record of the EGL library of the test thread.
    #[derive(Default)]
    struct FakeEgl {
        extensions: Option<CString>,
        /// The optional entry points the library lacks.
        missing: HashSet<&'static str>,
        gl_loader: Option<GetProcAddress>,
        /// What `eglCreateImageKHR` returns.
        image_handle: isize,
        /// The attribute lists `eglCreateImageKHR` was called with, without the terminator.
        image_attribs: Vec<Vec<i32>>,
        destroyed_images: Vec<isize>,
        formats: Vec<i32>,
        /// The modifiers of a format with their external-only flags; a format without an
        /// entry fails the query.
        modifiers: HashMap<i32, Vec<(u64, u32)>>,
        format_queries: i32,
    }

    thread_local! {
        static EGL: RefCell<FakeEgl> = RefCell::new(FakeEgl::default());
    }

    fn with<R>(f: impl FnOnce(&mut FakeEgl) -> R) -> R {
        EGL.with(|egl| f(&mut egl.borrow_mut()))
    }

    unsafe extern "system" fn get_error() -> i32 {
        EGL_SUCCESS
    }

    unsafe extern "system" fn initialize(_display: isize, major: *mut i32, minor: *mut i32) -> u32 {
        // SAFETY: the interface passes valid pointers.
        unsafe {
            *major = 1;
            *minor = 5;
        }
        1
    }

    unsafe extern "system" fn bind_api(_api: i32) -> u32 {
        1
    }

    unsafe extern "system" fn query_api() -> i32 {
        EGL_OPENGL_ES_API
    }

    unsafe extern "system" fn choose_config(
        _display: isize,
        _attribs: *const i32,
        configs: *mut isize,
        config_size: i32,
        num_configs: *mut i32,
    ) -> u32 {
        // SAFETY: the interface passes a valid count pointer and a buffer of `config_size`
        // configurations or null.
        unsafe {
            *num_configs = 1;
            if !configs.is_null() && config_size >= 1 {
                *configs = 11;
            }
        }
        1
    }

    unsafe extern "system" fn get_config_attrib(_display: isize, _config: isize, _attr: i32, rv: *mut i32) -> u32 {
        // SAFETY: the interface passes a valid pointer.
        unsafe { *rv = 8 };
        1
    }

    unsafe extern "system" fn query_string(_display: isize, _name: i32) -> *const c_char {
        // The string is owned by the script of the thread, which outlives the call.
        with(|egl| egl.extensions.as_ref().map_or(ptr::null(), |extensions| extensions.as_ptr()))
    }

    unsafe extern "system" fn create_context(_display: isize, _config: isize, _share: isize, _attrs: *const i32) -> isize {
        CONTEXT
    }

    unsafe extern "system" fn destroy_context(_display: isize, _context: isize) -> u32 {
        1
    }

    unsafe extern "system" fn make_current(_display: isize, _draw: isize, _read: isize, _context: isize) -> u32 {
        1
    }

    unsafe extern "system" fn get_current_context() -> isize {
        0
    }

    unsafe extern "system" fn get_current_display() -> isize {
        0
    }

    unsafe extern "system" fn get_current_surface(_read_draw: i32) -> isize {
        0
    }

    unsafe extern "system" fn terminate(_display: isize) {}

    unsafe extern "system" fn get_proc_address(name: *const c_char) -> *const c_void {
        // SAFETY: the context passes a terminated name.
        let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().into_owned();
        let loader = with(|egl| egl.gl_loader.clone());
        loader.map_or(ptr::null(), |loader| loader(&name))
    }

    unsafe extern "system" fn create_image_khr(
        _display: isize,
        _context: isize,
        _target: i32,
        _client_buffer: isize,
        attribs: *const i32,
    ) -> isize {
        let mut list = Vec::new();
        // SAFETY: the feature passes a list that is terminated with `EGL_NONE`.
        unsafe {
            let mut i = 0;
            while *attribs.add(i) != EGL_NONE {
                list.push(*attribs.add(i));
                i += 1;
            }
        }
        with(|egl| {
            egl.image_attribs.push(list);
            egl.image_handle
        })
    }

    unsafe extern "system" fn destroy_image_khr(_display: isize, image: isize) -> u32 {
        with(|egl| egl.destroyed_images.push(image));
        1
    }

    unsafe extern "system" fn query_dma_buf_formats_ext(
        _display: isize,
        max_formats: i32,
        formats: *mut i32,
        num_formats: *mut i32,
    ) -> u32 {
        with(|egl| {
            egl.format_queries += 1;
            // SAFETY: the feature passes a valid count pointer and a buffer of `max_formats`
            // values or null.
            unsafe {
                *num_formats = egl.formats.len() as i32;
                if !formats.is_null() {
                    for (i, format) in egl.formats.iter().take(max_formats.max(0) as usize).enumerate() {
                        *formats.add(i) = *format;
                    }
                }
            }
            1
        })
    }

    unsafe extern "system" fn query_dma_buf_modifiers_ext(
        _display: isize,
        format: i32,
        max_modifiers: i32,
        modifiers: *mut u64,
        external_only: *mut u32,
        num_modifiers: *mut i32,
    ) -> u32 {
        with(|egl| {
            let Some(list) = egl.modifiers.get(&format) else {
                return 0;
            };
            // SAFETY: the feature passes a valid count pointer and buffers of `max_modifiers`
            // values or null.
            unsafe {
                *num_modifiers = list.len() as i32;
                if !modifiers.is_null() && !external_only.is_null() {
                    for (i, (modifier, external)) in list.iter().take(max_modifiers.max(0) as usize).enumerate() {
                        *modifiers.add(i) = *modifier;
                        *external_only.add(i) = *external;
                    }
                }
            }
            1
        })
    }

    unsafe extern "system" fn other() {}

    fn egl() -> Rc<EglInterface> {
        let loader: GetProcAddress = Rc::new(|name: &str| {
            if with(|egl| egl.missing.contains(name)) {
                return ptr::null();
            }
            match name {
                "eglGetError" => get_error as *const c_void,
                "eglInitialize" => initialize as *const c_void,
                "eglBindAPI" => bind_api as *const c_void,
                "eglQueryAPI" => query_api as *const c_void,
                "eglChooseConfig" => choose_config as *const c_void,
                "eglGetConfigAttrib" => get_config_attrib as *const c_void,
                "eglQueryString" => query_string as *const c_void,
                "eglCreateContext" => create_context as *const c_void,
                "eglDestroyContext" => destroy_context as *const c_void,
                "eglMakeCurrent" => make_current as *const c_void,
                "eglGetCurrentContext" => get_current_context as *const c_void,
                "eglGetCurrentDisplay" => get_current_display as *const c_void,
                "eglGetCurrentSurface" => get_current_surface as *const c_void,
                "eglTerminate" => terminate as *const c_void,
                "eglGetProcAddress" => get_proc_address as *const c_void,
                "eglCreateImageKHR" => create_image_khr as *const c_void,
                "eglDestroyImageKHR" => destroy_image_khr as *const c_void,
                "eglQueryDmaBufFormatsEXT" => query_dma_buf_formats_ext as *const c_void,
                "eglQueryDmaBufModifiersEXT" => query_dma_buf_modifiers_ext as *const c_void,
                name if name.ends_with("EXT") || name.ends_with("KHR") => ptr::null(),
                _ => other as *const c_void,
            }
        });
        // SAFETY: the loader returns functions of the declared signatures for the entry
        // points the tests call.
        Rc::new(unsafe { EglInterface::new(&loader) })
    }

    /// A context of a scripted display with the given extensions of EGL and of OpenGL. The
    /// script of the EGL library is reset first, except for the entry points in `missing`.
    struct Setup {
        gl: FakeGl,
        // Kept alive for the context, which the display owns.
        _display: Rc<EglDisplay>,
        context: Rc<EglContext>,
    }

    fn setup_without(egl_extensions: &str, gl_extensions: &str, missing: &[&'static str]) -> Setup {
        let gl = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        gl.set_string(GL_EXTENSIONS, Some(gl_extensions));
        let gl_loader = gl.loader();
        with(|egl| {
            *egl = FakeEgl {
                extensions: Some(CString::new(egl_extensions).unwrap()),
                missing: missing.iter().copied().collect(),
                gl_loader: Some(gl_loader),
                image_handle: 77,
                ..Default::default()
            }
        });

        let display =
            EglDisplay::new(DISPLAY, EglDisplayOptions { egl: Some(egl()), ..Default::default() }).expect("a display");
        let context = display.create_context(None).expect("a context");
        gl.clear_calls();
        Setup { gl, _display: display, context }
    }

    fn setup(egl_extensions: &str, gl_extensions: &str) -> Setup {
        setup_without(egl_extensions, gl_extensions, &[])
    }

    impl Setup {
        /// The feature as the context offers it.
        fn feature(&self) -> Option<Rc<dyn IGlContextExternalObjectsFeature>> {
            let feature = self.context.try_get_feature(TypeId::of::<dyn IGlContextExternalObjectsFeature>())?;
            Some(feature.downcast_ref::<Rc<dyn IGlContextExternalObjectsFeature>>().expect("the contract").clone())
        }
    }

    fn dma_buf_handle(fd: isize) -> Rc<dyn IPlatformHandle> {
        Rc::new(PlatformHandle::new(fd, Some(DMA_BUF)))
    }

    fn properties(dma_buf: Option<PlatformGraphicsExternalImageDmaBufProperties>) -> PlatformGraphicsExternalImageProperties {
        PlatformGraphicsExternalImageProperties {
            width: 640,
            height: 480,
            memory_offset: 16,
            dma_buf_properties: dma_buf,
            ..Default::default()
        }
    }

    fn message<T>(result: Result<T, OpenGlException>) -> String {
        match result {
            Ok(_) => panic!("the call was expected to fail"),
            Err(error) => error.message().to_string(),
        }
    }

    #[test]
    fn a_context_without_the_extensions_has_no_feature() {
        assert!(setup(SURFACELESS, "GL_OES_EGL_image").feature().is_none());
        // The extension of OpenGL that binds the image to a texture is needed as well.
        assert!(setup(IMPORT, "GL_OES_vertex_array_object").feature().is_none());
    }

    #[test]
    fn a_context_without_the_image_entry_points_has_no_feature() {
        assert!(setup_without(IMPORT, "GL_OES_EGL_image", &["eglCreateImageKHR"]).feature().is_none());
        assert!(setup_without(IMPORT, "GL_OES_EGL_image", &["eglDestroyImageKHR"]).feature().is_none());
    }

    #[test]
    fn a_context_with_the_extensions_offers_the_import_of_dma_buf_images() {
        let setup = setup(IMPORT, "GL_OES_EGL_image");
        let feature = setup.feature().expect("the feature");

        assert_eq!(vec![DMA_BUF.to_string()], feature.supported_importable_external_image_types());
        assert!(feature.supported_exportable_external_image_types().is_empty());
        assert!(feature.supported_importable_external_semaphore_types().is_empty());
        assert!(feature.supported_exportable_external_semaphore_types().is_empty());
        assert_eq!(
            vec![PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm, PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm],
            feature.get_supported_formats_for_external_memory_type(DMA_BUF)
        );
        assert!(feature.get_supported_formats_for_external_memory_type("Other").is_empty());
        assert_eq!(
            CompositionGpuImportedImageSynchronizationCapabilities::AUTOMATIC,
            feature.get_synchronization_capabilities(DMA_BUF)
        );
        assert_eq!(
            CompositionGpuImportedImageSynchronizationCapabilities::default(),
            feature.get_synchronization_capabilities("Other")
        );
        assert_eq!(None, feature.device_luid());
        assert_eq!(None, feature.device_uuid());
    }

    #[test]
    fn what_only_the_extensions_of_open_gl_offer_is_refused_without_them() {
        let setup = setup(IMPORT, "GL_OES_EGL_image");
        let feature = setup.feature().expect("the feature");
        let format = PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm;

        assert_eq!("Specified method is not supported.", message(feature.create_image(DMA_BUF, PixelSize::new(1, 1), format)));
        assert_eq!("Specified method is not supported.", message(feature.create_image("Other", PixelSize::new(1, 1), format)));
        assert_eq!("Specified method is not supported.", message(feature.create_semaphore("Other")));

        let other: Rc<dyn IPlatformHandle> = Rc::new(PlatformHandle::new(3, Some("Other")));
        assert_eq!("Other is not supported", message(feature.import_image(other.clone(), properties(None))));
        assert_eq!("Other is not supported", message(feature.import_semaphore(other)));
        let unnamed: Rc<dyn IPlatformHandle> = Rc::new(PlatformHandle::new(3, None));
        assert_eq!(" is not supported", message(feature.import_semaphore(unnamed)));
    }

    #[test]
    fn the_formats_are_not_enumerated_without_the_modifiers_extension() {
        let setup = setup(IMPORT, "GL_OES_EGL_image");
        with(|egl| egl.formats = vec![1]);

        assert_eq!(None, setup.feature().expect("the feature").supported_dma_buf_formats());
        assert_eq!(0, with(|egl| egl.format_queries));
    }

    #[test]
    fn the_formats_are_not_enumerated_without_the_query_entry_points() {
        for missing in ["eglQueryDmaBufFormatsEXT", "eglQueryDmaBufModifiersEXT"] {
            let setup = setup_without(IMPORT_WITH_MODIFIERS, "GL_OES_EGL_image", &[missing]);
            with(|egl| egl.formats = vec![1]);

            assert_eq!(None, setup.feature().expect("the feature").supported_dma_buf_formats());
        }
    }

    #[test]
    fn a_display_without_formats_enumerates_none() {
        let setup = setup(IMPORT_WITH_MODIFIERS, "GL_OES_EGL_image");

        assert_eq!(None, setup.feature().expect("the feature").supported_dma_buf_formats());
    }

    #[test]
    fn the_formats_are_enumerated_once_with_the_modifiers_a_texture_can_sample() {
        let setup = setup(IMPORT_WITH_MODIFIERS, "GL_OES_EGL_image");
        with(|egl| {
            // The format 3 has no entry: the query of its modifiers fails.
            egl.formats = vec![1, 2, 3, 4];
            // The second modifier is external only.
            egl.modifiers.insert(1, vec![(10, 0), (11, 1), (0x1_0000_0002, 0)]);
            egl.modifiers.insert(2, Vec::new());
            egl.modifiers.insert(4, vec![(40, 1)]);
        });
        let feature = setup.feature().expect("the feature");

        let expected = vec![
            PlatformGraphicsDrmFormat::new(1, 10),
            PlatformGraphicsDrmFormat::new(1, 0x1_0000_0002),
            PlatformGraphicsDrmFormat::new(2, PlatformGraphicsExternalImageDmaBufProperties::DRM_MODIFIER_INVALID),
        ];
        assert_eq!(Some(expected.clone()), feature.supported_dma_buf_formats());
        // The count and the formats: two queries, which are not repeated.
        assert_eq!(2, with(|egl| egl.format_queries));
        assert_eq!(Some(expected), feature.supported_dma_buf_formats());
        assert_eq!(2, with(|egl| egl.format_queries));
    }

    #[test]
    fn an_import_without_dma_buf_properties_or_with_too_many_planes_is_refused() {
        let setup = setup(IMPORT, "GL_OES_EGL_image");
        let feature = setup.feature().expect("the feature");

        assert_eq!(
            "DmaBufProperties must be set for dma-buf imports",
            message(feature.import_image(dma_buf_handle(7), properties(None)))
        );

        let five_planes = PlatformGraphicsExternalImageDmaBufProperties { plane_count: 5, ..Default::default() };
        assert_eq!(
            "Specified argument was out of the range of valid values. (Parameter 'plane')",
            message(feature.import_image(dma_buf_handle(7), properties(Some(five_planes))))
        );
        assert!(with(|egl| egl.image_attribs.is_empty()));
        assert!(setup.gl.calls().is_empty());
    }

    #[test]
    fn an_image_egl_does_not_create_is_refused() {
        let setup = setup(IMPORT, "GL_OES_EGL_image");
        with(|egl| egl.image_handle = 0);
        let dma_buf = PlatformGraphicsExternalImageDmaBufProperties::default();

        assert_eq!(
            "eglCreateImageKHR failed to import the dma-buf",
            message(setup.feature().expect("the feature").import_image(dma_buf_handle(7), properties(Some(dma_buf))))
        );
        assert!(setup.gl.calls().is_empty());
    }

    #[test]
    fn a_single_plane_is_described_by_the_handle_and_bound_to_a_texture() {
        let setup = setup(IMPORT_WITH_MODIFIERS, "GL_OES_EGL_image");
        setup.gl.set_integer(GL_TEXTURE_BINDING_2D, 5);
        let dma_buf = PlatformGraphicsExternalImageDmaBufProperties {
            drm_format: 0x3432_5241,
            drm_modifier: PlatformGraphicsExternalImageDmaBufProperties::DRM_MODIFIER_INVALID,
            ..Default::default()
        };
        let properties = properties(Some(dma_buf));

        let texture =
            setup.feature().expect("the feature").import_image(dma_buf_handle(7), properties.clone()).expect("a texture");

        // The invalid modifier stands for the implicit layout: no modifier attributes.
        let expected = vec![
            EGL_WIDTH,
            640,
            EGL_HEIGHT,
            480,
            EGL_LINUX_DRM_FOURCC_EXT,
            0x3432_5241,
            EGL_DMA_BUF_PLANE0_FD_EXT,
            7,
            EGL_DMA_BUF_PLANE0_OFFSET_EXT,
            16,
            EGL_DMA_BUF_PLANE0_PITCH_EXT,
            0,
        ];
        assert_eq!(vec![expected], with(|egl| egl.image_attribs.clone()));

        let id = texture.texture_id();
        assert_ne!(0, id);
        assert_eq!(
            vec![
                "glGenTextures(1)".to_string(),
                format!("glBindTexture({GL_TEXTURE_2D},{id})"),
                format!("glEGLImageTargetTexture2DOES({GL_TEXTURE_2D},77)"),
                format!("glTexParameteri({GL_TEXTURE_2D},{GL_TEXTURE_MIN_FILTER},{GL_LINEAR})"),
                format!("glTexParameteri({GL_TEXTURE_2D},{GL_TEXTURE_MAG_FILTER},{GL_LINEAR})"),
                format!("glBindTexture({GL_TEXTURE_2D},5)"),
            ],
            setup.gl.calls()
        );
        assert_eq!(GL_RGBA8, texture.internal_format());
        assert_eq!(GL_TEXTURE_2D, texture.texture_type());
        assert_eq!(properties, texture.properties());

        // Disposing deletes the texture and destroys the image, once.
        setup.gl.clear_calls();
        texture.dispose();
        texture.dispose();
        assert_eq!(vec![format!("glDeleteTextures({id})")], setup.gl.calls());
        assert_eq!(vec![77], with(|egl| egl.destroyed_images.clone()));
        assert_eq!(0, texture.texture_id());
    }

    #[test]
    fn the_planes_are_described_by_the_properties_with_the_modifier_in_two_halves() {
        let setup = setup(IMPORT_WITH_MODIFIERS, "GL_OES_EGL_image");
        let dma_buf = PlatformGraphicsExternalImageDmaBufProperties {
            drm_format: 0x3231_564E,
            drm_modifier: 0x0100_0000_0000_0002,
            plane_count: 2,
            plane_fds: Some(vec![8, 9]),
            plane_strides: Some(vec![640, 320]),
            plane_offsets: Some(vec![0, 4096]),
        };

        setup.feature().expect("the feature").import_image(dma_buf_handle(7), properties(Some(dma_buf))).expect("a texture");

        let expected = vec![
            EGL_WIDTH,
            640,
            EGL_HEIGHT,
            480,
            EGL_LINUX_DRM_FOURCC_EXT,
            0x3231_564E,
            EGL_DMA_BUF_PLANE0_FD_EXT,
            8,
            EGL_DMA_BUF_PLANE0_OFFSET_EXT,
            0,
            EGL_DMA_BUF_PLANE0_PITCH_EXT,
            640,
            EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT,
            2,
            EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT,
            0x0100_0000,
            EGL_DMA_BUF_PLANE1_FD_EXT,
            9,
            EGL_DMA_BUF_PLANE1_OFFSET_EXT,
            4096,
            EGL_DMA_BUF_PLANE1_PITCH_EXT,
            320,
            EGL_DMA_BUF_PLANE1_MODIFIER_LO_EXT,
            2,
            EGL_DMA_BUF_PLANE1_MODIFIER_HI_EXT,
            0x0100_0000,
        ];
        assert_eq!(vec![expected], with(|egl| egl.image_attribs.clone()));
    }

    #[test]
    fn a_modifier_is_not_passed_without_the_modifiers_extension() {
        let setup = setup(IMPORT, "GL_OES_EGL_image");
        let dma_buf = PlatformGraphicsExternalImageDmaBufProperties { drm_modifier: 2, ..Default::default() };

        setup.feature().expect("the feature").import_image(dma_buf_handle(7), properties(Some(dma_buf))).expect("a texture");

        let attribs = with(|egl| egl.image_attribs[0].clone());
        assert!(!attribs.contains(&EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT));
        assert!(!attribs.contains(&EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT));
    }

    #[test]
    fn an_image_open_gl_does_not_bind_is_destroyed_with_its_texture() {
        let setup = setup(IMPORT, "GL_OES_EGL_image");
        setup.gl.set_integer(GL_TEXTURE_BINDING_2D, 5);
        setup.gl.set_error(GL_INVALID_OPERATION);
        let dma_buf = PlatformGraphicsExternalImageDmaBufProperties::default();

        let error = setup
            .feature()
            .expect("the feature")
            .import_image(dma_buf_handle(7), properties(Some(dma_buf)))
            .err()
            .expect("an error");

        assert_eq!(Some(GL_INVALID_OPERATION), error.error_code());
        assert!(error.message().starts_with("glEGLImageTargetTexture2DOES failed with error"));
        let calls = setup.gl.calls();
        assert_eq!(Some(&format!("glBindTexture({GL_TEXTURE_2D},5)")), calls.iter().rev().nth(1));
        assert!(calls.last().expect("a call").starts_with("glDeleteTextures("));
        assert!(!calls.iter().any(|call| call.starts_with("glTexParameteri")));
        assert_eq!(vec![77], with(|egl| egl.destroyed_images.clone()));
    }
}
