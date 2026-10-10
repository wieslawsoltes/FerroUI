//! A texture of Direct3D 11 as a texture of a context of ANGLE: an image
//! that is shared with another graphics API.

use crate::direct_x::{ID3D11Texture2D, IDXGIKeyedMutex, IDXGIResource, D3D11_TEXTURE2D_DESC};
use ferroui_base::platform::{
    IPlatformGraphicsContext, IPlatformHandle, KnownPlatformGraphicsExternalImageHandleTypes, PlatformGraphicsExternalImageFormat,
    PlatformGraphicsExternalImageProperties, PlatformHandle,
};
use ferroui_base::reactive::IDisposable;
use ferroui_microcom::ComPtr;
use ferroui_opengl::egl::egl_consts::{
    EGL_BACK_BUFFER, EGL_D3D_TEXTURE_ANGLE, EGL_HEIGHT, EGL_NONE, EGL_TEXTURE_2D, EGL_TEXTURE_FORMAT,
    EGL_TEXTURE_INTERNAL_FORMAT_ANGLE, EGL_TEXTURE_RGBA, EGL_TEXTURE_TARGET, EGL_WIDTH,
};
use ferroui_opengl::egl::{EglContext, EglSurface};
use ferroui_opengl::gl_consts::{GL_RGBA, GL_RGBA8, GL_TEXTURE_2D};
use ferroui_opengl::{IGlContext, IGlExportableExternalImageTexture, IGlExternalImageTexture, OpenGlException};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// What a texture holds until it is disposed.
struct Objects {
    egl_surface: Rc<EglSurface>,
    /// Kept for the life of the texture, as the reference keeps it.
    _texture2d: ComPtr<ID3D11Texture2D>,
    mutex: ComPtr<IDXGIKeyedMutex>,
}

pub(crate) struct AngleExternalMemoryD3D11Texture2D {
    context: Rc<EglContext>,
    objects: RefCell<Option<Objects>>,
    texture_id: Cell<i32>,
    internal_format: i32,
    properties: PlatformGraphicsExternalImageProperties,
}

impl AngleExternalMemoryD3D11Texture2D {
    /// The texture, which holds a reference of its own to `texture2d`. The
    /// context is current.
    pub fn new(
        context: &Rc<EglContext>,
        texture2d: &ComPtr<ID3D11Texture2D>,
        props: PlatformGraphicsExternalImageProperties,
    ) -> Result<AngleExternalMemoryD3D11Texture2D, OpenGlException> {
        let mutex = texture2d
            .cast::<IDXGIKeyedMutex>()
            .map_err(|error| OpenGlException::new(format!("The texture has no keyed mutex: {error}")))?;

        let attrs = [
            EGL_WIDTH,
            props.width,
            EGL_HEIGHT,
            props.height,
            EGL_TEXTURE_FORMAT,
            EGL_TEXTURE_RGBA,
            EGL_TEXTURE_TARGET,
            EGL_TEXTURE_2D,
            EGL_TEXTURE_INTERNAL_FORMAT_ANGLE,
            GL_RGBA,
            EGL_NONE,
            EGL_NONE,
            EGL_NONE,
        ];

        let display = context.display();
        let egl_surface =
            display.create_pbuffer_from_client_buffer(EGL_D3D_TEXTURE_ANGLE, texture2d.as_ptr() as isize, &attrs)?;

        let gl = IGlContext::gl_interface(&**context);
        let texture_id = gl.gen_texture();
        gl.bind_texture(GL_TEXTURE_2D, texture_id);

        if display.egl_interface().bind_tex_image(display.handle(), egl_surface.dangerous_get_handle(), EGL_BACK_BUFFER) == 0 {
            let error = OpenGlException::get_formatted_exception_for_egl("eglBindTexImage", display.egl_interface());
            gl.delete_texture(texture_id);
            egl_surface.dispose();
            return Err(error);
        }

        Ok(AngleExternalMemoryD3D11Texture2D {
            context: context.clone(),
            objects: RefCell::new(Some(Objects { egl_surface, _texture2d: texture2d.clone(), mutex })),
            texture_id: Cell::new(texture_id),
            internal_format: GL_RGBA8,
            properties: props,
        })
    }

    /// # Panics
    /// Panics on a texture that was disposed (the
    /// `ObjectDisposedException` of the reference).
    fn with_mutex<R>(&self, f: impl FnOnce(&IDXGIKeyedMutex) -> R) -> R {
        match self.objects.borrow().as_ref() {
            Some(objects) => f(&objects.mutex),
            None => panic!("Cannot access a disposed object.\nObject name: 'AngleExternalMemoryD3D11Texture2D'."),
        }
    }
}

impl IGlExternalImageTexture for AngleExternalMemoryD3D11Texture2D {
    /// # Panics
    /// Panics when the mutex cannot be acquired (the exception of the
    /// reference).
    fn acquire_keyed_mutex(&self, key: u32) {
        if let Err(error) = self.with_mutex(|mutex| mutex.acquire_sync(u64::from(key), i32::MAX as u32)) {
            panic!("IDXGIKeyedMutex::AcquireSync: {error}");
        }
    }

    /// # Panics
    /// Panics when the mutex cannot be released (the exception of the
    /// reference).
    fn release_keyed_mutex(&self, key: u32) {
        if let Err(error) = self.with_mutex(|mutex| mutex.release_sync(u64::from(key))) {
            panic!("IDXGIKeyedMutex::ReleaseSync: {error}");
        }
    }

    fn texture_id(&self) -> i32 {
        self.texture_id.get()
    }

    fn internal_format(&self) -> i32 {
        self.internal_format
    }

    fn texture_type(&self) -> i32 {
        GL_TEXTURE_2D
    }

    fn properties(&self) -> PlatformGraphicsExternalImageProperties {
        self.properties.clone()
    }

    fn dispose(&self) {
        if !IPlatformGraphicsContext::is_lost(&*self.context) && self.texture_id.get() != 0 {
            let current = IPlatformGraphicsContext::ensure_current(&*self.context);
            IGlContext::gl_interface(&*self.context).delete_texture(self.texture_id.get());
            current.dispose();
        }
        self.texture_id.set(0);
        if let Some(objects) = self.objects.borrow_mut().take() {
            objects.egl_surface.dispose();
        }
    }
}

pub(crate) struct AngleExternalMemoryD3D11ExportedTexture2D {
    base: AngleExternalMemoryD3D11Texture2D,
    handle: Rc<dyn IPlatformHandle>,
}

impl AngleExternalMemoryD3D11ExportedTexture2D {
    fn get_handle(texture2d: &ComPtr<ID3D11Texture2D>) -> Result<Rc<dyn IPlatformHandle>, OpenGlException> {
        let shared_handle = texture2d
            .cast::<IDXGIResource>()
            .and_then(|resource| resource.get_shared_handle())
            .map_err(|error| OpenGlException::new(format!("The texture has no shared handle: {error}")))?;
        Ok(Rc::new(PlatformHandle::new(
            shared_handle,
            Some(KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_GLOBAL_SHARED_HANDLE),
        )))
    }

    pub fn new(
        context: &Rc<EglContext>,
        texture2d: &ComPtr<ID3D11Texture2D>,
        desc: &D3D11_TEXTURE2D_DESC,
        format: PlatformGraphicsExternalImageFormat,
    ) -> Result<AngleExternalMemoryD3D11ExportedTexture2D, OpenGlException> {
        let handle = Self::get_handle(texture2d)?;
        let properties = PlatformGraphicsExternalImageProperties {
            width: desc.width as i32,
            height: desc.height as i32,
            format,
            ..Default::default()
        };
        Ok(AngleExternalMemoryD3D11ExportedTexture2D {
            base: AngleExternalMemoryD3D11Texture2D::new(context, texture2d, properties)?,
            handle,
        })
    }
}

impl IGlExternalImageTexture for AngleExternalMemoryD3D11ExportedTexture2D {
    fn acquire_keyed_mutex(&self, key: u32) {
        self.base.acquire_keyed_mutex(key);
    }

    fn release_keyed_mutex(&self, key: u32) {
        self.base.release_keyed_mutex(key);
    }

    fn texture_id(&self) -> i32 {
        self.base.texture_id()
    }

    fn internal_format(&self) -> i32 {
        self.base.internal_format()
    }

    fn texture_type(&self) -> i32 {
        self.base.texture_type()
    }

    fn properties(&self) -> PlatformGraphicsExternalImageProperties {
        self.base.properties()
    }

    fn dispose(&self) {
        self.base.dispose();
    }
}

impl IGlExportableExternalImageTexture for AngleExternalMemoryD3D11ExportedTexture2D {
    fn get_handle(&self) -> Rc<dyn IPlatformHandle> {
        self.handle.clone()
    }
}
