//! Tests of the Windows.UI.Composition mode against the system. They run
//! where the tests of the crate run on Windows (the Windows jobs of CI);
//! each prints what the system answered. Not from upstream: the reference
//! has no tests of the mode.

use super::d2d_effects::D2DEffects;
use super::win_ui_composited_window::WinUiCompositedWindow;
use super::win_ui_composition_shared::WinUiCompositionShared;
use super::win_ui_effect_base::{ColorSourceEffect, OpacityEffect, WinUIEffectBase, WinUIGaussianBlurEffect};
use crate::interop::unmanaged_methods::co_task_mem_free;
use crate::simple_window::SimpleWindow;
use crate::win32_platform::Win32Platform;
use crate::win_rt::{
    DispatcherQueueOptions, HStringInterop, ICompositor, IGraphicsEffect, IGraphicsEffectD2D1Interop, IGraphicsEffectSource,
    IInspectable, IPropertyValue, NativeWinRTMethods, PropertyType, DISPATCHERQUEUE_THREAD_APARTMENTTYPE,
    DISPATCHERQUEUE_THREAD_TYPE,
};
use ferroui_base::platform::PlatformThemeVariant;
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::PixelSize;
use ferroui_microcom::{Guid, HResult, IUnknown, Interface};
use ferroui_opengl::egl::IEglWindowGlPlatformSurfaceInfo;
use std::sync::Arc;

/// An effect is one object behind three vtables: the calls below go
/// through the vtables and the thunks, as the calls of the compositor do.
#[test]
fn an_effect_answers_through_its_three_interfaces() {
    let color = WinUIEffectBase::new(ColorSourceEffect::new(vec![0.25, 0.5, 0.75, 1.0]), &[]);
    let color_source = color.cast::<IGraphicsEffectSource>().expect("the source interface of an effect");
    let effect = WinUIEffectBase::new(OpacityEffect::new(0.5), &[&color_source]);

    // One for the other, in every direction, and one identity.
    let source = effect.cast::<IGraphicsEffectSource>().expect("IGraphicsEffectSource");
    let interop = source.cast::<IGraphicsEffectD2D1Interop>().expect("IGraphicsEffectD2D1Interop");
    let back = interop.cast::<IGraphicsEffect>().expect("IGraphicsEffect");
    assert_eq!(back.as_ptr(), effect.as_ptr());
    assert!(interop.cast::<IInspectable>().is_ok());
    let identity = effect.cast::<IUnknown>().unwrap().as_ptr();
    assert_eq!(identity, source.cast::<IUnknown>().unwrap().as_ptr());
    assert_eq!(identity, interop.cast::<IUnknown>().unwrap().as_ptr());
    assert!(effect.cast::<IPropertyValue>().is_err());

    // The effect of Direct2D and its properties.
    assert_eq!(interop.get_effect_id().unwrap(), D2DEffects::CLSID_D2D1_OPACITY);
    assert_eq!(interop.get_property_count().unwrap(), 1);
    let opacity = interop.get_property(0).unwrap().expect("the opacity");
    assert!(opacity.type_().unwrap() == PropertyType::Single);
    assert_eq!(opacity.get_single().unwrap(), 0.5);
    assert!(interop.get_property(1).unwrap().is_none());
    let mut index = 0;
    // SAFETY: the method fails without writing to its out parameters.
    let mapping = unsafe { interop.get_named_property_mapping(0, &mut index, std::ptr::null_mut()) };
    assert_eq!(mapping.unwrap_err().0, HResult::NOTIMPL.0);

    // The sources: the object that was given, and a failure past the end.
    assert_eq!(interop.get_source_count().unwrap(), 1);
    let first = interop.get_source(0).unwrap().expect("the source");
    assert_eq!(first.as_ptr(), color_source.as_ptr());
    assert_eq!(interop.get_source(1).unwrap_err().0, HResult::INVALIDARG.0);
    let color_interop = color.cast::<IGraphicsEffectD2D1Interop>().unwrap();
    assert_eq!(color_interop.get_source_count().unwrap(), 0);
    assert_eq!(color_interop.get_effect_id().unwrap(), D2DEffects::CLSID_D2D1_FLOOD);
    let flood = color_interop.get_property(0).unwrap().expect("the colour");
    assert!(flood.type_().unwrap() == PropertyType::SingleArray);

    // The name is the null string, and setting one is accepted.
    assert_eq!(effect.name().unwrap(), 0);
    assert!(effect.set_name(0).is_ok());

    // As an object of the Windows Runtime.
    let class_name = HStringInterop::from_handle(effect.get_runtime_class_name().unwrap(), true);
    assert_eq!(class_name.value().as_deref(), Some("FerroUI.Win32.WinRT.Composition.OpacityEffect"));
    let mut count = 0u64;
    let mut iids: *mut Guid = std::ptr::null_mut();
    // SAFETY: out parameters of this frame; the block is freed below.
    unsafe { effect.get_iids(&mut count, &mut iids) }.unwrap();
    assert_eq!(count, 3);
    // SAFETY: the call succeeded, so the block holds three identifiers.
    assert_eq!(
        unsafe { std::slice::from_raw_parts(iids, 3) },
        &[IGraphicsEffect::IID, IGraphicsEffectSource::IID, IGraphicsEffectD2D1Interop::IID]
    );
    // SAFETY: the block of the call above, not used again.
    unsafe { co_task_mem_free(iids.cast()) };

    // An effect keeps its sources: the colour effect lives while the
    // opacity effect does, whoever else lets go of it.
    drop((color, color_source, color_interop, flood, first));
    let kept = interop.get_source(0).unwrap().expect("the source is kept");
    assert_eq!(kept.cast::<IGraphicsEffectD2D1Interop>().unwrap().get_effect_id().unwrap(), D2DEffects::CLSID_D2D1_FLOOD);

    let blur = WinUIEffectBase::new(WinUIGaussianBlurEffect, &[&kept]);
    let blur_interop = blur.cast::<IGraphicsEffectD2D1Interop>().unwrap();
    assert_eq!(blur_interop.get_property_count().unwrap(), 3);
    assert_eq!(blur_interop.get_property(0).unwrap().unwrap().get_single().unwrap(), 30.0);
    assert_eq!(blur_interop.get_property(1).unwrap().unwrap().get_u_int32().unwrap(), 1);
    assert_eq!(blur_interop.get_property(2).unwrap().unwrap().get_u_int32().unwrap(), 1);
}

struct WindowInfo(isize);

impl IEglWindowGlPlatformSurfaceInfo for WindowInfo {
    fn handle(&self) -> isize {
        self.0
    }

    fn size(&self) -> PixelSize {
        PixelSize::new(320, 200)
    }

    fn scaling(&self) -> f64 {
        1.0
    }
}

/// The compositor takes the effects of the backend: the brushes of the
/// acrylic and of the mica effect are made from them, and the tree of a
/// window is built over them with its blur visuals. On a thread of its
/// own with a dispatcher queue, as the connection of the mode has.
#[test]
fn the_compositor_makes_the_brushes_and_the_tree_of_a_window() {
    std::thread::spawn(|| {
        let controller = NativeWinRTMethods::create_dispatcher_queue_controller(DispatcherQueueOptions {
            dw_size: std::mem::size_of::<DispatcherQueueOptions>() as i32,
            thread_type: DISPATCHERQUEUE_THREAD_TYPE::DQTYPE_THREAD_CURRENT,
            apartment_type: DISPATCHERQUEUE_THREAD_APARTMENTTYPE::DQTAT_COM_NONE,
        })
        .expect("a dispatcher queue for the thread");

        let compositor = match NativeWinRTMethods::create_instance::<ICompositor>("Windows.UI.Composition.Compositor") {
            Ok(compositor) => compositor,
            Err(error) => {
                // A session that cannot create a compositor says so; the
                // mode is then passed over (session 0 of a virtual
                // machine).
                println!("skipped: this session does not activate Windows.UI.Composition.Compositor: {error}");
                return;
            }
        };

        let version = Win32Platform::windows_version();
        let shared = WinUiCompositionShared::new(&compositor);
        println!(
            "Windows {version}: the shared state of the mode: {:?}",
            shared.as_ref().map(|shared| format!(
                "the acrylic brush; mica light {}, mica dark {}",
                shared.mica_brush_light().is_some(),
                shared.mica_brush_dark().is_some()
            ))
        );
        let shared = shared.expect("the brushes of the blur effects");
        // Mica is of Windows 11.
        assert_eq!(shared.mica_brush_light().is_some(), version.build >= 22000);
        assert_eq!(shared.mica_brush_dark().is_some(), version.build >= 22000);

        // The tree of a window, with the corners of the backdrop rounded.
        let window = SimpleWindow::new(None);
        let info: Arc<dyn IEglWindowGlPlatformSurfaceInfo> = Arc::new(WindowInfo(window.handle()));
        let composited = WinUiCompositedWindow::new(info, shared.clone(), Some(8.0));
        println!("the composition tree of a window: {:?}", composited.as_ref().map(|_| "created"));
        let composited = composited.expect("the composition tree of a window");
        {
            let _transaction = composited.begin_transaction();
            composited.resize_if_needed(PixelSize::new(320, 200)).expect("the size of the visual");
            for level in [
                CompositionTransparencyLevel::None,
                CompositionTransparencyLevel::Transparent,
                CompositionTransparencyLevel::AcrylicBlur,
                CompositionTransparencyLevel::Mica,
                CompositionTransparencyLevel::None,
            ] {
                for theme in [PlatformThemeVariant::Light, PlatformThemeVariant::Dark] {
                    composited.apply_effects(level, theme).expect("the visibility of the blur visuals");
                }
            }
        }
        assert!(!shared.sync_root().is_entered());
        composited.dispose();
        window.dispose();
        drop(controller);
    })
    .join()
    .unwrap();
}
