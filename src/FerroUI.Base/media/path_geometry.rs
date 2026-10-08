use crate::media::media_collection::track_item_property_changed;
use crate::media::{
    FillRule, GeometryImpl, PathFigure, PathFigures, PathMarkupParser, StreamGeometry,
    StreamGeometryContext,
};
use crate::platform::{self, IGeometryContext, IGeometryImpl, PathGeometryContext};
use crate::reactive::IDisposable;
use crate::utilities::FormatError;
use crate::{
    ferro_class, ferro_property, instantiate, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    Ref, StyledProperty,
};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

type FigureSubscriptions = Rc<RefCell<Vec<(Ref<PathFigure>, Rc<dyn IDisposable>)>>>;

/// Represents a complex shape that may be composed of arcs, curves,
/// ellipses, lines, and rectangles.
#[repr(C)]
pub struct PathGeometry {
    base: StreamGeometry,
    figures: RefCell<Option<PathFigures>>,
    figures_observer: RefCell<Option<Rc<dyn IDisposable>>>,
    figures_properties_observer: RefCell<Option<Rc<dyn IDisposable>>>,
    figure_subscriptions: FigureSubscriptions,
}

ferro_class!(PathGeometry: StreamGeometry);
crate::ferro_class_info!(PathGeometry { new: PathGeometry::new });

impl FerroObjectImpl for PathGeometry {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_figures(Some(PathFigures::new()));
    }
}

impl GeometryImpl for PathGeometry {
    fn create_defining_geometry(this: &Self) -> Option<Arc<dyn IGeometryImpl>> {
        let figures = this.figures()?;

        let factory = platform::render_interface();
        let geometry = factory.create_stream_geometry();

        {
            let mut ctx = StreamGeometryContext::new(geometry.open());
            ctx.set_fill_rule(this.fill_rule());
            for f in figures.iter() {
                f.apply_to(&mut ctx);
            }
        }

        Some(geometry)
    }
}

crate::ferro_properties! { impl PathGeometry {
    ferro_property!(pub fn figures_property() -> DirectProperty<PathGeometry, Option<PathFigures>> {
        FerroProperty::register_direct::<PathGeometry, _>(
            "Figures",
            |g| g.figures.borrow().clone(),
            Some(|g, f| g.set_figures(f)),
            None,
        )
    });

    ferro_property!(pub fn fill_rule_property() -> StyledProperty<FillRule> {
        FerroProperty::register::<PathGeometry, _>("FillRule", FillRule::EvenOdd)
    });
} }

impl PathGeometry {
    fn static_constructor() {
        Self::figures_property().changed().add_class_handler::<PathGeometry>(|s, e| {
            s.on_figures_changed(e.get_new_value::<Option<PathFigures>>())
        });
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: StreamGeometry::construct(),
            figures: RefCell::new(None),
            figures_observer: RefCell::new(None),
            figures_properties_observer: RefCell::new(None),
            figure_subscriptions: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Creates a path geometry with an empty figures collection.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Parses the specified path data to a path geometry.
    pub fn parse(path_data: &str) -> Result<Ref<PathGeometry>, FormatError> {
        let path_geometry = PathGeometry::new();

        let mut context = PathGeometryContext::new(path_geometry.clone());
        let result = PathMarkupParser::new(&mut context).parse(path_data);
        context.dispose();
        result?;

        Ok(path_geometry)
    }

    /// The figures.
    pub fn figures(&self) -> Option<PathFigures> {
        self.figures.borrow().clone()
    }

    pub fn set_figures(&self, value: Option<PathFigures>) {
        self.set_and_raise(Self::figures_property(), &self.figures, value);
    }

    /// How the intersecting areas contained in the path geometry are
    /// combined. The default value is [`FillRule::EvenOdd`].
    pub fn fill_rule(&self) -> FillRule {
        self.get_value(Self::fill_rule_property())
    }

    pub fn set_fill_rule(&self, value: FillRule) {
        self.set_value(Self::fill_rule_property(), value)
    }

    fn on_figures_changed(&self, figures: Option<PathFigures>) {
        if let Some(observer) = self.figures_observer.take() {
            observer.dispose();
        }
        if let Some(observer) = self.figures_properties_observer.take() {
            observer.dispose();
        }

        let Some(figures) = figures else {
            return;
        };

        let weak = self.to_ref().downgrade();
        let invalidate = move || {
            if let Some(this) = weak.upgrade() {
                this.invalidate_geometry();
            }
        };

        let subscriptions = self.figure_subscriptions.clone();
        let subscriptions_added = subscriptions.clone();
        let (invalidate_added, invalidate_removed, invalidate_reset) =
            (invalidate.clone(), invalidate.clone(), invalidate.clone());
        let observer = figures.for_each_item(
            move |_, s: &Ref<PathFigure>| {
                let from_segments = invalidate_added.clone();
                let subscription = s.segments_invalidated(from_segments);
                subscriptions_added.borrow_mut().push((s.clone(), subscription));
                invalidate_added();
            },
            move |_, s: &Ref<PathFigure>| {
                let position = subscriptions.borrow().iter().position(|(f, _)| f.ptr_eq(s));
                if let Some(position) = position {
                    let (_, subscription) = subscriptions.borrow_mut().remove(position);
                    subscription.dispose();
                }
                invalidate_removed();
            },
            invalidate_reset,
        );
        *self.figures_observer.borrow_mut() = Some(observer);

        let observer = track_item_property_changed(&figures, invalidate);
        *self.figures_properties_observer.borrow_mut() = Some(observer);
    }
}

/// Writes the path markup of the geometry.
impl fmt::Display for PathGeometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let figures: Vec<String> = match self.figures() {
            Some(figures) => figures.iter().map(|figure| figure.to_string()).collect(),
            None => Vec::new(),
        };
        write!(f, "{}{}", if self.fill_rule() != FillRule::EvenOdd { "F1 " } else { "" }, figures.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use crate::media::{ArcSegment, PathSegments, PolyLineSegment};
    use crate::{Point, Size};

    #[test]
    fn path_geometry_triggers_invalidation_on_figures_add() {
        let segment = PolyLineSegment::with_points([Point::new(1.0, 1.0), Point::new(2.0, 2.0)]);

        let figure = PathFigure::new();
        figure.set_segments(Some(PathSegments::from_items([segment.upcast()])));
        figure.set_is_closed(false);
        figure.set_is_filled(false);

        let target = PathGeometry::new();
        let changed = Rc::new(Cell::new(false));
        let c = changed.clone();
        target.changed(move || c.set(true));

        target.figures().unwrap().add(figure);
        assert!(changed.get());
    }

    #[test]
    fn path_segment_triggers_invalidation_on_property_change() {
        let target_segment = ArcSegment::new();
        target_segment.set_size(Size::new(10.0, 10.0));
        target_segment.set_point(Point::new(5.0, 5.0));

        let figure = PathFigure::new();
        figure.set_is_closed(false);
        figure.set_segments(Some(PathSegments::from_items([target_segment.clone().upcast()])));

        let target = PathGeometry::new();
        target.set_figures(Some(PathFigures::from_items([figure])));

        let changed = Rc::new(Cell::new(false));
        let c = changed.clone();
        target.changed(move || c.set(true));

        target_segment.set_size(Size::new(20.0, 20.0));

        assert!(changed.get());
    }

    #[test]
    fn adding_a_segment_and_changing_a_figure_invalidate() {
        let target = PathGeometry::parse("M 0,0 L 10,10").unwrap();
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        target.changed(move || c.set(c.get() + 1));

        let figure = target.figures().unwrap().get(0);
        figure.segments().unwrap().add(crate::media::LineSegment::new().upcast());
        assert_eq!(1, count.get());

        figure.set_is_closed(true);
        assert_eq!(2, count.get());

        target.figures().unwrap().remove_at(0);
        assert_eq!(3, count.get());

        // A removed figure no longer invalidates the geometry.
        figure.set_is_closed(false);
        figure.segments().unwrap().add(crate::media::LineSegment::new().upcast());
        assert_eq!(3, count.get());
    }

    #[test]
    fn to_string_writes_path_markup() {
        let target = PathGeometry::parse("F1 M10,20 L30,40 C1,2 3,4 5,6 Q1,2 3,4 A10,20 30 1 0 40,50 Z").unwrap();
        assert_eq!(
            "F1 M 10, 20 L 30, 40 C 1, 2 3, 4 5, 6 Q 1, 2 3, 4 A 10, 20 30 1 0 40, 50Z",
            target.to_string()
        );
        assert_eq!("", PathGeometry::new().to_string());
    }
}
