use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{RectangleGeometry, Stretch},
    rendering::composition::{frames::*, *},
    threading::Dispatcher,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Matrix,
    Rect, Ref, Size, StyledElementImpl, StyledProperty, Vector, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_media::{MediaError, MediaErrorKind, MediaEvent};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

pub struct ImportedTextureFrame {
    pub image: Rc<dyn ICompositionImportedGpuImage>,
    pub producer_ready: GpuCompletion,
}
pub trait TextureFrameImporter {
    fn import(
        &self,
        interop: &dyn ICompositionGpuInterop,
        frame: &FrameLease,
    ) -> Result<ImportedTextureFrame, MediaError>;
}
#[repr(C)]
pub struct TexturePresenter {
    base: Control,
    source: RefCell<Option<Arc<FrameMailbox>>>,
    importer: RefCell<Option<Rc<dyn TextureFrameImporter>>>,
    interop: RefCell<Option<Rc<dyn ICompositionGpuInterop>>>,
    surface: RefCell<Option<CompositionDrawingSurface>>,
    visual: RefCell<Option<CompositionSurfaceVisual>>,
    generation: Cell<u64>,
    busy: Cell<bool>,
    current: RefCell<Option<FrameMetadata>>,
    last_error: RefCell<Option<MediaError>>,
    errors: MediaEvent<MediaError>,
    presented: MediaEvent<FrameMetadata>,
    cleared: MediaEvent<()>,
}
ferro_class!(TexturePresenter:Control);
ferro_class_info!(TexturePresenter {
    new: TexturePresenter::new
});
ferro_impl_classes!(TexturePresenter:LayoutableImpl,StyledElementImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl FerroObjectImpl for TexturePresenter {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_clip_to_bounds(true);
    }
    fn on_property_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, args);
        if args.property() == Visual::bounds_property().as_property()
            || args.property() == Self::stretch_property().as_property()
        {
            this.layout_frame();
        }
    }
}
ferro_properties! {impl TexturePresenter {
    pub fn stretch_property()->StyledProperty<Stretch> {FerroProperty::register::<Self,_>("Stretch",Stretch::Uniform)}
}}
impl VisualImpl for TexturePresenter {
    fn on_attached_to_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, args);
        this.attach();
    }
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        this.release();
        Self::parent_on_detached_from_visual_tree(this, args);
    }
}
impl TexturePresenter {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            source: RefCell::new(None),
            importer: RefCell::new(None),
            interop: RefCell::new(None),
            surface: RefCell::new(None),
            visual: RefCell::new(None),
            generation: Cell::new(0),
            busy: Cell::new(false),
            current: RefCell::new(None),
            last_error: RefCell::new(None),
            errors: MediaEvent::default(),
            presented: MediaEvent::default(),
            cleared: MediaEvent::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn stretch(&self) -> Stretch {
        self.get_value(Self::stretch_property())
    }
    pub fn set_stretch(&self, value: Stretch) {
        self.set_value(Self::stretch_property(), value);
    }
    pub fn errors(&self) -> &MediaEvent<MediaError> {
        &self.errors
    }
    /// Raised after the compositor accepts a current shared frame. This is a
    /// presentation-state notification, not a GPU-completion fence.
    pub fn frame_presented(&self) -> &MediaEvent<FrameMetadata> {
        &self.presented
    }
    /// Raised when presentation is reset or the current frame is removed.
    pub fn frame_cleared(&self) -> &MediaEvent<()> {
        &self.cleared
    }
    /// Metadata for the currently presented frame, with no pixel storage.
    pub fn presented_frame(&self) -> Option<std::cell::Ref<'_, FrameMetadata>> {
        std::cell::Ref::filter_map(self.current.borrow(), Option::as_ref).ok()
    }
    pub fn last_error(&self) -> Option<MediaError> {
        self.last_error.borrow().clone()
    }
    pub fn set_frame_source(
        &self,
        source: Option<Arc<FrameMailbox>>,
        importer: Option<Rc<dyn TextureFrameImporter>>,
    ) {
        self.release();
        self.last_error.borrow_mut().take();
        *self.source.borrow_mut() = source;
        *self.importer.borrow_mut() = importer;
        if self.is_attached_to_visual_tree() {
            self.attach();
        }
    }
    fn error(&self, error: MediaError) {
        *self.last_error.borrow_mut() = Some(error.clone());
        self.errors.emit(&error);
    }
    fn attach(&self) {
        if self.source.borrow().is_none() || self.interop.borrow().is_some() {
            return;
        }
        let Some(element) = ElementComposition::get_element_visual(&self.to_ref()) else {
            return;
        };
        let compositor = element.compositor().clone();
        let generation = self.generation.get();
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().spawn_local(async move {
            let interop = compositor.try_get_composition_gpu_interop().await;
            let Some(view) = weak
                .upgrade()
                .filter(|v| v.generation.get() == generation && v.is_attached_to_visual_tree())
            else {
                return;
            };
            let Some(interop) = interop else {
                view.error(MediaError::new(
                    MediaErrorKind::Unsupported,
                    "Renderer has no shared texture importer",
                ));
                return;
            };
            let surface = compositor.create_drawing_surface();
            let visual = compositor.create_surface_visual();
            visual.set_surface(Some((*surface).clone()));
            ElementComposition::set_element_child_visual(&view, Some((*visual).clone()));
            *view.surface.borrow_mut() = Some(surface);
            *view.visual.borrow_mut() = Some(visual);
            *view.interop.borrow_mut() = Some(interop);
            view.present_latest();
        });
    }
    /// Retire presentation on context loss while retaining the selected source.
    /// A fresh provider/importer must be attached after the renderer recreates it.
    pub fn invalidate_device(&self) {
        self.release();
    }
    fn release(&self) {
        self.generation.set(
            self.generation
                .get()
                .checked_add(1)
                .expect("presenter generation exhausted"),
        );
        self.current.borrow_mut().take();
        self.interop.borrow_mut().take();
        self.busy.set(false);
        ElementComposition::set_element_child_visual(&self.to_ref(), None);
        if let Some(visual) = self.visual.borrow_mut().take() {
            visual.set_surface(None);
            visual.dispose();
        }
        if let Some(surface) = self.surface.borrow_mut().take() {
            surface.dispose();
        }
        self.cleared.emit(&());
    }
    /// Called on a source frame/invalidation opportunity; unchanged paused
    /// frames do not submit another import or render job.
    pub fn present_latest(&self) {
        if self.busy.get() {
            return;
        }
        let frame = self.source.borrow().as_ref().and_then(|s| s.latest());
        let Some(frame) = frame else {
            let had_frame = self.current.borrow_mut().take().is_some();
            if had_frame {
                if let Some(surface) = self.surface.borrow().clone() {
                    surface.clear_async();
                }
                self.cleared.emit(&());
            }
            return;
        };
        if self
            .current
            .borrow()
            .as_ref()
            .is_some_and(|m| m.identity == frame.metadata().identity)
        {
            return;
        }
        let (Some(interop), Some(surface), Some(importer)) = (
            self.interop.borrow().clone(),
            self.surface.borrow().clone(),
            self.importer.borrow().clone(),
        ) else {
            return;
        };
        if interop.is_lost() {
            self.release();
            self.error(MediaError::new(
                MediaErrorKind::DeviceLost,
                "Graphics device was lost",
            ));
            return;
        }
        let imported = match importer.import(&*interop, &frame) {
            Ok(imported) => imported,
            Err(error) => {
                self.release();
                self.error(error);
                return;
            }
        };
        let metadata = frame.metadata().clone();
        let validity = frame.clone();
        let task =
            surface.update_shared_frame_async(&*imported.image, frame, imported.producer_ready);
        let complete = task.clone();
        let weak = self.to_ref().downgrade();
        let generation = self.generation.get();
        self.busy.set(true);
        task.on_completed(move || {
            imported.image.dispose_async();
            let Some(view) = weak.upgrade().filter(|v| v.generation.get() == generation) else {
                return;
            };
            view.busy.set(false);
            if !validity.is_current() {
                view.present_latest();
                return;
            }
            if let Some(error) = complete.exception() {
                view.release();
                view.error(MediaError::new(
                    MediaErrorKind::Unsupported,
                    error.to_string(),
                ));
            } else {
                view.last_error.borrow_mut().take();
                let previous = view.current.replace(Some(metadata.clone()));
                // Rebuilding an identical clip for every video frame queues
                // extra composition work. Bounds and stretch changes already
                // relayout the current frame through on_property_changed.
                if previous.as_ref().is_none_or(|frame| {
                    frame.width != metadata.width
                        || frame.height != metadata.height
                        || frame.visible != metadata.visible
                        || frame.rotation != metadata.rotation
                        || frame.mirrored != metadata.mirrored
                }) {
                    view.layout_frame();
                }
                view.presented.emit(&metadata);
                // A frame published while this one was imported was skipped (the presenter was
                // busy): present it now. Nothing is submitted when the latest is this frame.
                view.present_latest();
            }
        });
    }
    fn layout_frame(&self) {
        let Some(visual) = self.visual.borrow().clone() else {
            return;
        };
        let current = self.current.borrow();
        let Some(frame) = current.as_ref() else {
            return;
        };
        if let Some(layout) = frame_layout(frame, self.bounds().size(), self.stretch()) {
            visual.set_size(Vector::new(frame.width as f64, frame.height as f64));
            visual.set_transform_matrix(layout.transform);
            let clip = RectangleGeometry::new();
            clip.set_rect(Rect::new(
                frame.visible.x as f64,
                frame.visible.y as f64,
                frame.visible.width as f64,
                frame.visible.height as f64,
            ));
            visual.set_clip(clip.platform_impl());
        } else {
            visual.set_size(Vector::ZERO);
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct FrameLayout {
    pub content: Rect,
    pub transform: Matrix,
}
/// One affine mapping shared by texture sampling and QR overlays. Crop-to-fill
/// content may extend outside the viewport; clipping never replaces this rect.
pub fn frame_layout(frame: &FrameMetadata, bounds: Size, stretch: Stretch) -> Option<FrameLayout> {
    if !bounds.width.is_finite()
        || !bounds.height.is_finite()
        || bounds.width <= 0.
        || bounds.height <= 0.
    {
        return None;
    }
    let r = frame.visible;
    if r.width == 0 || r.height == 0 {
        return None;
    }
    let rotated = matches!(
        frame.rotation,
        FrameRotation::Clockwise90 | FrameRotation::Clockwise270
    );
    let (w, h) = if rotated {
        (r.height as f64, r.width as f64)
    } else {
        (r.width as f64, r.height as f64)
    };
    let (sx, sy) = match stretch {
        Stretch::None => (1., 1.),
        Stretch::Fill => (bounds.width / w, bounds.height / h),
        Stretch::Uniform => {
            let s = (bounds.width / w).min(bounds.height / h);
            (s, s)
        }
        Stretch::UniformToFill => {
            let s = (bounds.width / w).max(bounds.height / h);
            (s, s)
        }
    };
    let content = Rect::new(
        (bounds.width - w * sx) / 2.,
        (bounds.height - h * sy) / 2.,
        w * sx,
        h * sy,
    );
    let map = |x: f64, y: f64| {
        let (u, v) = frame
            .orient(
                (x - r.x as f64) / r.width as f64,
                (y - r.y as f64) / r.height as f64,
            )
            .unwrap();
        (
            content.x + u * content.width,
            content.y + v * content.height,
        )
    };
    let a = map(0., 0.);
    let b = map(1., 0.);
    let c = map(0., 1.);
    Some(FrameLayout {
        content,
        transform: Matrix::new(b.0 - a.0, b.1 - a.1, c.0 - a.0, c.1 - a.1, a.0, a.1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn metadata() -> FrameMetadata {
        FrameMetadata {
            identity: FrameIdentity {
                source: 1,
                generation: 1,
                sequence: 1,
            },
            width: 400,
            height: 200,
            visible: FrameRegion {
                x: 100,
                y: 50,
                width: 200,
                height: 100,
            },
            format: FrameFormat::Bgra8,
            color: FrameColor::Srgb,
            alpha: FrameAlpha::Opaque,
            rotation: FrameRotation::None,
            mirrored: false,
            top_left_origin: true,
            timestamp: std::time::Duration::ZERO,
            device: None,
            device_generation: 1,
            planes: vec![FramePlane {
                offset: 0,
                stride: 1600,
                length: 320000,
            }],
            retained_bytes: 320000,
        }
    }
    #[test]
    fn crop_to_fill_preserves_content_beyond_the_viewport_and_maps_true_corners() {
        let frame = metadata();
        let layout = frame_layout(&frame, Size::new(200., 200.), Stretch::UniformToFill).unwrap();
        assert_eq!(layout.content, Rect::new(-100., 0., 400., 200.));
        let mapped = ferroui_base::Point::new(100., 50.) * layout.transform;
        assert!((mapped.x + 100.).abs() < 1e-10 && mapped.y.abs() < 1e-10);
    }
    #[test]
    fn quarter_rotation_mirror_and_zero_size_are_not_applied_twice() {
        let mut frame = metadata();
        frame.rotation = FrameRotation::Clockwise90;
        frame.mirrored = true;
        let layout = frame_layout(&frame, Size::new(100., 200.), Stretch::Uniform).unwrap();
        let first = ferroui_base::Point::new(100., 50.) * layout.transform;
        let second = ferroui_base::Point::new(300., 150.) * layout.transform;
        assert!(first.x.abs() < 1e-10 && first.y.abs() < 1e-10);
        assert!((second.x - 100.).abs() < 1e-10 && (second.y - 200.).abs() < 1e-10);
        assert!(frame_layout(&frame, Size::new(0., 200.), Stretch::Uniform).is_none());
    }
}
