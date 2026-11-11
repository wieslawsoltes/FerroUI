use super::{CameraPreview, TexturePresenter};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{immutable::ImmutablePen, DrawingContext, IPen, StreamGeometry},
    platform::IGeometryContext,
    reactive::Subscription,
    rendering::composition::frames::{FrameIdentity, FrameMetadata},
    threading::{DispatcherPriority, DispatcherTimer},
    FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Point, Rect, Ref,
    StyledElementImpl, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl, PanelImpl};
use ferroui_media::MediaSubscription;
use ferroui_media_controls::frame_layout;
use ferroui_media_qr::{QrCandidate, QrFrameGeometry};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
    time::Duration,
};

/// A camera preview that outlines QR detections supplied by an analysis engine.
/// Creating this control does not start capture or recognition.
#[repr(C)]
pub struct QrPreview {
    base: CameraPreview,
    outline: OnceCell<Ref<QrOutline>>,
}
ferro_class!(QrPreview:CameraPreview);
ferro_class_info!(QrPreview {
    new: QrPreview::new
});
ferro_impl_classes!(QrPreview:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);

impl FerroObjectImpl for QrPreview {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let presenter = this.base.presenter.borrow().as_ref().unwrap().clone();
        let outline = QrOutline::new(presenter);
        this.children().add(&outline);
        this.outline.get_or_init(|| outline);
    }

    fn on_property_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, args);
        if args.property() == CameraPreview::stretch_property().as_property() {
            if let Some(outline) = this.outline.get() {
                outline.invalidate_visual();
            }
        }
    }
}

impl VisualImpl for QrPreview {
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        this.set_detections(&[]);
        Self::parent_on_detached_from_visual_tree(this, args);
    }
}

impl QrPreview {
    pub fn construct() -> Self {
        Self {
            base: CameraPreview::construct(),
            outline: OnceCell::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Replaces the outlines with a single frame's detections. Pass an empty
    /// slice when no codes are found. An outline expires if analysis stops
    /// refreshing it. The preview retains only corner and frame metadata.
    pub fn set_detections(&self, candidates: &[QrCandidate]) {
        if let Some(outline) = self.outline.get() {
            outline.set_detections(candidates);
        }
    }
}

struct Detections {
    frame: FrameIdentity,
    geometry: QrFrameGeometry,
    corners: Vec<[(f64, f64); 4]>,
}

impl Detections {
    fn matches(&self, frame: &FrameMetadata) -> bool {
        // Analysis runs less often than presentation and may complete after
        // the displayed sequence has advanced.
        self.frame.source == frame.identity.source
            && self.frame.generation == frame.identity.generation
            && self.geometry == QrFrameGeometry::from(frame)
    }
}

#[repr(C)]
struct QrOutline {
    base: Control,
    presenter: Ref<TexturePresenter>,
    detections: RefCell<Option<Detections>>,
    matched: Cell<bool>,
    timeout: RefCell<Option<Subscription>>,
    subscriptions: OnceCell<[MediaSubscription; 2]>,
    edge: Rc<dyn IPen>,
    stroke: Rc<dyn IPen>,
}
ferro_class!(QrOutline:Control);
ferro_impl_classes!(QrOutline:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);

impl FerroObjectImpl for QrOutline {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_is_hit_test_visible(false);
        this.subscriptions.get_or_init(|| {
            let presented = this.to_ref().downgrade();
            let cleared = presented.clone();
            [
                this.presenter.frame_presented().subscribe(move |frame| {
                    if let Some(outline) = presented.upgrade() {
                        outline.update_match(Some(frame));
                    }
                }),
                this.presenter.frame_cleared().subscribe(move |_| {
                    if let Some(outline) = cleared.upgrade() {
                        outline.set_detections(&[]);
                    }
                }),
            ]
        });
    }

    fn on_property_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, args);
        if this.matched.get() && args.property() == Visual::bounds_property().as_property() {
            this.invalidate_visual();
        }
    }
}

impl VisualImpl for QrOutline {
    fn render(this: &Self, context: &mut DrawingContext) {
        if !this.matched.get() {
            return;
        }
        let detections = this.detections.borrow();
        let Some(detections) = detections.as_ref() else {
            return;
        };
        let Some(frame) = this.presenter.presented_frame() else {
            return;
        };
        let bounds = this.bounds().size();
        let Some(layout) = frame_layout(&frame, bounds, this.presenter.stretch()) else {
            return;
        };
        let geometry = StreamGeometry::new();
        {
            let mut path = geometry.open();
            for corners in &detections.corners {
                let [first, rest @ ..] = corners.map(|(x, y)| {
                    Point::new(x * f64::from(frame.width), y * f64::from(frame.height))
                        * layout.transform
                });
                path.begin_figure(first, false);
                for corner in rest {
                    path.line_to(corner, true);
                }
                path.end_figure(true);
            }
        }
        let clip = context.push_clip(layout.content.intersect(Rect::from_size(bounds)));
        let geometry = geometry.upcast();
        context.draw_geometry(None, Some(&this.edge), &geometry);
        context.draw_geometry(None, Some(&this.stroke), &geometry);
        context.pop(clip);
    }
}

impl QrOutline {
    fn new(presenter: Ref<TexturePresenter>) -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            presenter,
            detections: RefCell::new(None),
            matched: Cell::new(false),
            timeout: RefCell::new(None),
            subscriptions: OnceCell::new(),
            edge: Rc::new(ImmutablePen::from_uint32(0xff000000, 5.)),
            stroke: Rc::new(ImmutablePen::from_uint32(0xff39ff7b, 3.)),
        })
    }

    fn set_detections(&self, candidates: &[QrCandidate]) {
        let next = candidates.first().and_then(|first| {
            first.frame_geometry.validate().ok()?;
            let corners: Vec<_> = candidates
                .iter()
                .filter(|candidate| {
                    candidate.frame == first.frame
                        && candidate.frame_geometry == first.frame_geometry
                        && candidate
                            .corners
                            .iter()
                            .all(|(x, y)| (0. ..=1.).contains(x) && (0. ..=1.).contains(y))
                })
                .map(|candidate| candidate.corners)
                .collect();
            (!corners.is_empty()).then_some(Detections {
                frame: first.frame,
                geometry: first.frame_geometry,
                corners,
            })
        });
        self.timeout.borrow_mut().take();
        if next.is_some() {
            let weak = self.to_ref().downgrade();
            self.timeout.replace(Some(DispatcherTimer::run_once(
                move || {
                    if let Some(outline) = weak.upgrade() {
                        outline.set_detections(&[]);
                    }
                },
                Duration::from_millis(250),
                DispatcherPriority::NORMAL,
            )));
        }
        let previous = self.detections.replace(next);
        let frame = self.presenter.presented_frame();
        let matched_before = self.matched.get();
        self.update_match(frame.as_deref());
        if matched_before && self.matched.get() {
            let current = self.detections.borrow();
            let same_outlines = previous.zip(current.as_ref()).is_some_and(|(old, new)| {
                old.geometry == new.geometry && old.corners == new.corners
            });
            if !same_outlines {
                self.invalidate_visual();
            }
        }
    }

    fn update_match(&self, frame: Option<&FrameMetadata>) {
        let matched = frame.is_some_and(|frame| {
            self.detections
                .borrow()
                .as_ref()
                .is_some_and(|detections| detections.matches(frame))
        });
        if self.matched.replace(matched) != matched {
            self.invalidate_visual();
        }
    }
}
