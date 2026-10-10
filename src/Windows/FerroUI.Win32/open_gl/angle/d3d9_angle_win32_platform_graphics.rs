//! The platform graphics of ANGLE on Direct3D 9: one display and one
//! context, shared by everything that renders.

use super::d3d11_angle_win32_platform_graphics::probe_display;
use super::{AngleWin32EglDisplay, Win32AngleEglInterface};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{IPlatformGraphics, IPlatformGraphicsContext};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::ThreadBound;
use ferroui_opengl::egl::{EglContext, EglContextOptions};
use std::cell::RefCell;
use std::rc::Rc;

/// The platform graphics of ANGLE on Direct3D 9.
///
/// The display and its context are objects of the thread that created the
/// graphics, and stay bound to it: the shared context is given on that
/// thread, and asking for it on another panics. The graphics manager does
/// not take these graphics (as in the reference, which goes on to the next
/// rendering mode unless the graphics are the ones of Direct3D 11), so no
/// compositor renders with them.
pub struct D3D9AngleWin32PlatformGraphics {
    shared: ThreadBound<Shared>,
}

struct Shared {
    display: AngleWin32EglDisplay,
    context: RefCell<Option<Rc<EglContext>>>,
}

impl D3D9AngleWin32PlatformGraphics {
    pub fn new(shared_display: AngleWin32EglDisplay) -> D3D9AngleWin32PlatformGraphics {
        D3D9AngleWin32PlatformGraphics {
            shared: ThreadBound::new(Shared { display: shared_display, context: RefCell::new(None) }),
        }
    }

    /// The platform graphics, when the display of Direct3D 9 and a context
    /// of it can be created and the context made current; `None`, with the
    /// failure logged, when not.
    pub fn try_create(egl: &Win32AngleEglInterface) -> Option<D3D9AngleWin32PlatformGraphics> {
        let result = AngleWin32EglDisplay::create_d3d9_display(egl).and_then(|shared_display| {
            match probe_display(&shared_display) {
                Ok(()) => Ok(shared_display),
                Err(error) => {
                    shared_display.dispose();
                    Err(error)
                }
            }
        });

        match result {
            Ok(shared_display) => Some(D3D9AngleWin32PlatformGraphics::new(shared_display)),
            Err(e) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    logger.log_with_values(None, "Unable to initialize ANGLE-based rendering with DirectX9 : {0}", &[&e]);
                }
                None
            }
        }
    }

    /// Disposes the shared context and the display. (The reference leaves
    /// both to the finalizers of the runtime.)
    pub fn dispose(&self) {
        let shared = self.shared.get();
        let context = shared.context.borrow_mut().take();
        if let Some(context) = context {
            IPlatformGraphicsContext::dispose(&*context);
        }
        shared.display.dispose();
    }
}

impl IPlatformGraphics for D3D9AngleWin32PlatformGraphics {
    fn uses_shared_context(&self) -> bool {
        true
    }

    /// # Panics
    /// Panics when the context cannot be created (the exception of the
    /// reference), and on a thread other than the one that created the
    /// graphics.
    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        let shared = self.shared.get();
        let lost = shared.context.borrow().as_ref().is_some_and(|context| IPlatformGraphicsContext::is_lost(&**context));
        if lost {
            let context = shared.context.borrow_mut().take();
            if let Some(context) = context {
                IPlatformGraphicsContext::dispose(&*context);
            }
        }

        let existing = shared.context.borrow().clone();
        match existing {
            Some(context) => context,
            None => match shared.display.create_context(Some(EglContextOptions::default())) {
                Ok(context) => {
                    *shared.context.borrow_mut() = Some(context.clone());
                    context
                }
                Err(error) => panic!("{error}"),
            },
        }
    }

    /// # Panics
    /// Always: the graphics have one context, the shared one (the
    /// `InvalidOperationException` of the reference).
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        panic!("Operation is not valid due to the current state of the object.")
    }
}
