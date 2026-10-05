use crate::media::media_collection::track_item_property_changed;
use crate::media::{PathSegments, StreamGeometryContext};
use crate::platform::IGeometryContext;
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{
    ferro_class, ferro_property, instantiate, DirectProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Point, Ref, StyledProperty,
};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// Represents a subsection of a geometry: a single connected series of
/// segments.
#[repr(C)]
pub struct PathFigure {
    base: FerroObject,
    segments_invalidated: HandlerList<dyn Fn()>,
    segments: RefCell<Option<PathSegments>>,
    segments_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    segments_properties_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(PathFigure: FerroObject);
crate::ferro_class_info!(PathFigure { new: PathFigure::new });

impl FerroObjectImpl for PathFigure {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_segments(Some(PathSegments::new()));
    }
}

crate::ferro_properties! { impl PathFigure {
    ferro_property!(pub fn is_closed_property() -> StyledProperty<bool> {
        FerroProperty::register::<PathFigure, _>("IsClosed", true)
    });

    ferro_property!(pub fn is_filled_property() -> StyledProperty<bool> {
        FerroProperty::register::<PathFigure, _>("IsFilled", true)
    });

    ferro_property!(pub fn segments_property() -> DirectProperty<PathFigure, Option<PathSegments>> {
        FerroProperty::register_direct::<PathFigure, _>(
            "Segments",
            |f| f.segments.borrow().clone(),
            Some(|f, s| f.set_segments(s)),
            None,
        )
    });

    ferro_property!(pub fn start_point_property() -> StyledProperty<Point> {
        FerroProperty::register::<PathFigure, _>("StartPoint", Point::default())
    });
} }

impl PathFigure {
    fn static_constructor() {
        Self::segments_property().changed().add_class_handler::<PathFigure>(|s, _| s.on_segments_changed());
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            segments_invalidated: HandlerList::new(),
            segments: RefCell::new(None),
            segments_disposable: RefCell::new(None),
            segments_properties_disposable: RefCell::new(None),
        }
    }

    /// Creates a figure with an empty segments collection.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Subscribes to invalidation of the figure's segments. Disposing the
    /// returned handle unsubscribes.
    pub(crate) fn segments_invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.segments_invalidated.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.segments_invalidated.remove(token);
            }
        })
    }

    fn on_segments_changed(&self) {
        if let Some(disposable) = self.segments_disposable.take() {
            disposable.dispose();
        }
        if let Some(disposable) = self.segments_properties_disposable.take() {
            disposable.dispose();
        }

        let segments = self.segments.borrow().clone();
        if let Some(segments) = segments {
            let weak = self.to_ref().downgrade();
            let invalidate = move || {
                if let Some(this) = weak.upgrade() {
                    this.invalidate_segments();
                }
            };

            let (added, removed, reset) = (invalidate.clone(), invalidate.clone(), invalidate.clone());
            let disposable = segments.for_each_item(move |_, _| added(), move |_, _| removed(), reset);
            *self.segments_disposable.borrow_mut() = Some(disposable);

            let disposable = track_item_property_changed(&segments, invalidate);
            *self.segments_properties_disposable.borrow_mut() = Some(disposable);
        }
    }

    fn invalidate_segments(&self) {
        if self.segments_invalidated.is_empty() {
            return;
        }
        for (_, handler) in self.segments_invalidated.snapshot().iter() {
            handler();
        }
    }

    /// Whether the figure is closed.
    pub fn is_closed(&self) -> bool {
        self.get_value(Self::is_closed_property())
    }

    pub fn set_is_closed(&self, value: bool) {
        self.set_value(Self::is_closed_property(), value)
    }

    /// Whether the figure is filled.
    pub fn is_filled(&self) -> bool {
        self.get_value(Self::is_filled_property())
    }

    pub fn set_is_filled(&self, value: bool) {
        self.set_value(Self::is_filled_property(), value)
    }

    /// The segments of the figure.
    pub fn segments(&self) -> Option<PathSegments> {
        self.segments.borrow().clone()
    }

    pub fn set_segments(&self, value: Option<PathSegments>) {
        self.set_and_raise(Self::segments_property(), &self.segments, value);
    }

    /// The start point of the figure.
    pub fn start_point(&self) -> Point {
        self.get_value(Self::start_point_property())
    }

    pub fn set_start_point(&self, value: Point) {
        self.set_value(Self::start_point_property(), value)
    }

    /// Draws the figure into `ctx`.
    pub(crate) fn apply_to(&self, ctx: &mut StreamGeometryContext) {
        ctx.begin_figure(self.start_point(), self.is_filled());

        if let Some(segments) = self.segments() {
            for segment in segments.iter() {
                segment.apply_to(ctx);
            }
        }

        ctx.end_figure(self.is_closed());
    }
}

/// Writes the path markup of the figure.
impl fmt::Display for PathFigure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let segments: Vec<String> = match self.segments() {
            Some(segments) => segments.iter().map(|segment| segment.to_string()).collect(),
            None => Vec::new(),
        };
        write!(f, "M {} {}{}", self.start_point(), segments.join(" "), if self.is_closed() { "Z" } else { "" })
    }
}
