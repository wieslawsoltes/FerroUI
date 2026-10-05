use crate::media::immutable::{ImmutableDashStyle, ImmutablePen};
use crate::media::{
    BrushExtensions, DashStyle, IBrush, IDashStyle, IPen, MediaCollection, PenLineCap, PenLineJoin, SolidColorBrush,
};
use crate::media::ref_adapter::RefAdapter;
use crate::reactive::{Disposable, IDisposable};
use crate::rendering::composition::drawing::{
    brush_get_server_resource, CompositorResourceHolder, ICompositionRenderResource, ServerCompositionSimplePen,
};
use crate::rendering::composition::generated::ServerCompositionSimplePenProps;
use crate::rendering::composition::server::{IServerObject, ServerObjectId};
use crate::rendering::composition::transport::BatchStreamWriter;
use crate::rendering::composition::{Compositor, ICompositorSerializable};
use crate::utilities::HandlerList;
use crate::{
    ferro_class, ferro_property, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use std::cell::RefCell;
use std::rc::Rc;

/// The dashes handed to [`Pen::try_modify_or_create`].
pub enum StrokeDashArray<'a> {
    /// A plain list: the created dash style is immutable.
    List(&'a [f64]),
    /// An observable list: the created dash style is mutable and shares the
    /// list.
    Observable(&'a MediaCollection<f64>),
}

type DashStyleSubscription = (Ref<DashStyle>, Rc<dyn IDisposable>);

/// Describes how a stroke is drawn.
#[repr(C)]
pub struct Pen {
    base: FerroObject,
    invalidated: HandlerList<dyn Fn()>,
    subscribed_to_dashes: RefCell<Option<DashStyleSubscription>>,
    resource: CompositorResourceHolder,
}

ferro_class!(Pen: FerroObject);
crate::ferro_class_info!(Pen { new: Pen::new, interfaces: [std::rc::Rc<dyn crate::media::IPen>] });

impl FerroObjectImpl for Pen {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        this.register_for_serialization();
        this.raise_invalidated();

        if change.property() == Self::brush_property().as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<Rc<dyn IBrush>>>();
            this.resource.process_property_change_notification(
                old_value.as_ref().and_then(|b| b.as_composition_render_resource()),
                new_value.as_ref().and_then(|b| b.as_composition_render_resource()),
            );
        }

        if change.property() == Self::dash_style_property().as_property() {
            this.update_dash_style_subscription();
        }
        Self::parent_on_property_changed(this, change);
    }
}

crate::ferro_properties! { impl Pen {
    ferro_property!(pub fn brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<Pen, _>("Brush", None)
    });

    ferro_property!(pub fn thickness_property() -> StyledProperty<f64> {
        FerroProperty::register::<Pen, _>("Thickness", 1.0)
    });

    ferro_property!(pub fn dash_style_property() -> StyledProperty<Option<Rc<dyn IDashStyle>>> {
        FerroProperty::register::<Pen, _>("DashStyle", None)
    });

    ferro_property!(pub fn line_cap_property() -> StyledProperty<PenLineCap> {
        FerroProperty::register::<Pen, _>("LineCap", PenLineCap::Flat)
    });

    ferro_property!(pub fn line_join_property() -> StyledProperty<PenLineJoin> {
        FerroProperty::register::<Pen, _>("LineJoin", PenLineJoin::Bevel)
    });

    ferro_property!(pub fn miter_limit_property() -> StyledProperty<f64> {
        FerroProperty::register::<Pen, _>("MiterLimit", 10.0)
    });
} }

impl Pen {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            invalidated: HandlerList::new(),
            subscribed_to_dashes: RefCell::new(None),
            resource: CompositorResourceHolder::new(),
        }
    }

    /// Creates a pen with default values.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a solid pen with the given brush and thickness: no dashes,
    /// flat caps, miter joins and a miter limit of 10.
    pub fn with_brush(brush: Option<Rc<dyn IBrush>>, thickness: f64) -> Ref<Self> {
        Self::with_all(brush, thickness, None, PenLineCap::Flat, PenLineJoin::Miter, 10.0)
    }

    /// Creates a solid pen with the color given as an `0xAARRGGBB` value.
    pub fn from_uint32(color: u32, thickness: f64) -> Ref<Self> {
        Self::with_brush(Some(SolidColorBrush::from_uint32(color).into()), thickness)
    }

    /// Creates a pen.
    pub fn with_all(
        brush: Option<Rc<dyn IBrush>>,
        thickness: f64,
        dash_style: Option<Rc<dyn IDashStyle>>,
        line_cap: PenLineCap,
        line_join: PenLineJoin,
        miter_limit: f64,
    ) -> Ref<Self> {
        let result = Self::new();
        result.set_brush(brush);
        result.set_thickness(thickness);
        result.set_line_cap(line_cap);
        result.set_line_join(line_join);
        result.set_miter_limit(miter_limit);
        result.set_dash_style(dash_style);
        result
    }

    /// The brush used to draw the stroke.
    pub fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::brush_property())
    }

    pub fn set_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::brush_property(), value)
    }

    /// The stroke thickness.
    pub fn thickness(&self) -> f64 {
        self.get_value(Self::thickness_property())
    }

    pub fn set_thickness(&self, value: f64) {
        self.set_value(Self::thickness_property(), value)
    }

    /// The style of dashed lines drawn with the pen.
    pub fn dash_style(&self) -> Option<Rc<dyn IDashStyle>> {
        self.get_value(Self::dash_style_property())
    }

    pub fn set_dash_style(&self, value: Option<Rc<dyn IDashStyle>>) {
        self.set_value(Self::dash_style_property(), value)
    }

    /// The type of shape to use on both ends of a line.
    pub fn line_cap(&self) -> PenLineCap {
        self.get_value(Self::line_cap_property())
    }

    pub fn set_line_cap(&self, value: PenLineCap) {
        self.set_value(Self::line_cap_property(), value)
    }

    /// A value describing how to join consecutive line or curve segments.
    pub fn line_join(&self) -> PenLineJoin {
        self.get_value(Self::line_join_property())
    }

    pub fn set_line_join(&self, value: PenLineJoin) {
        self.set_value(Self::line_join_property(), value)
    }

    /// The limit of the ratio of the miter length to half this pen's
    /// thickness.
    pub fn miter_limit(&self) -> f64 {
        self.get_value(Self::miter_limit_property())
    }

    pub fn set_miter_limit(&self, value: f64) {
        self.set_value(Self::miter_limit_property(), value)
    }

    /// Creates an immutable clone of the pen.
    pub fn to_immutable(&self) -> ImmutablePen {
        ImmutablePen::new(
            self.brush().map(|brush| BrushExtensions::to_immutable(&brush)),
            self.thickness(),
            self.dash_style().map(|style| BrushExtensions::dash_style_to_immutable(&style)),
            self.line_cap(),
            self.line_join(),
            self.miter_limit(),
        )
    }

    /// Smart reuse and update of a pen.
    ///
    /// If `brush` is `None`, `pen` is cleared. If the brush and the dashes
    /// are immutable, a new immutable pen is created. Otherwise the existing
    /// mutable pen is updated, or a new mutable pen is created.
    ///
    /// Returns true if `pen` was created or cleared, false if the existing
    /// pen was modified in place.
    #[allow(clippy::too_many_arguments)]
    pub fn try_modify_or_create(
        pen: &mut Option<Rc<dyn IPen>>,
        brush: Option<Rc<dyn IBrush>>,
        thickness: f64,
        stroke_dash_array: Option<StrokeDashArray<'_>>,
        stroke_dash_offset: f64,
        line_cap: PenLineCap,
        line_join: PenLineJoin,
        miter_limit: f64,
    ) -> bool {
        let previous_pen = pen.clone();

        let Some(brush) = brush else {
            *pen = None;
            return previous_pen.is_some();
        };

        let mut dash_style: Option<Rc<dyn IDashStyle>> = None;
        let mut immutable_dash_style: Option<Rc<ImmutableDashStyle>> = None;
        match stroke_dash_array {
            // The list supports notification: create a mutable dash style.
            Some(StrokeDashArray::Observable(list)) if !list.is_empty() => {
                dash_style = Some(DashStyle::with_dash_list(list.clone(), stroke_dash_offset).into());
            }
            Some(StrokeDashArray::List(list)) if !list.is_empty() => {
                let style = Rc::new(ImmutableDashStyle::new(Some(list), stroke_dash_offset));
                immutable_dash_style = Some(style.clone());
                dash_style = Some(style);
            }
            _ => {}
        }

        if let Some(immutable_brush) = brush.clone().into_immutable_brush() {
            if dash_style.is_none() || immutable_dash_style.is_some() {
                *pen = Some(Rc::new(ImmutablePen::new(
                    Some(immutable_brush),
                    thickness,
                    immutable_dash_style,
                    line_cap,
                    line_join,
                    miter_limit,
                )));
                return true;
            }
        }

        let mutable_pen = previous_pen
            .as_ref()
            .and_then(|p| p.as_object())
            .and_then(|o| o.downcast_ref::<Pen>())
            .map(Pen::to_ref)
            .unwrap_or_else(Pen::new);
        mutable_pen.set_brush(Some(brush));
        mutable_pen.set_thickness(thickness);
        mutable_pen.set_line_cap(line_cap);
        mutable_pen.set_line_join(line_join);
        mutable_pen.set_dash_style(dash_style);
        mutable_pen.set_miter_limit(miter_limit);

        let new_pen: Rc<dyn IPen> = mutable_pen.into();
        let changed = match &previous_pen {
            Some(previous) => !previous.equals(&*new_pen),
            None => true,
        };
        *pen = Some(new_pen);
        changed
    }

    /// Subscribes to invalidation of the pen: raised whenever a change
    /// requires strokes drawn with the pen to be redrawn. Disposing the
    /// returned handle unsubscribes.
    pub fn invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.invalidated.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidated.remove(token);
            }
        })
    }

    fn raise_invalidated(&self) {
        if self.invalidated.is_empty() {
            return;
        }
        for (_, handler) in self.invalidated.snapshot().iter() {
            handler();
        }
    }

    /// The object as something a compositor serializes: the adapter keeps
    /// the object alive while it is queued.
    pub(crate) fn as_compositor_serializable(&self) -> Rc<dyn ICompositorSerializable> {
        Rc::new(RefAdapter(self.to_ref()))
    }

    fn register_for_serialization(&self) {
        if self.resource.is_attached() {
            let serializable = self.as_compositor_serializable();
            self.resource.register_for_invalidation_on_all_compositors(&serializable);
        }
    }

    fn update_dash_style_subscription(&self) {
        let new_value: Option<Ref<DashStyle>> = self
            .dash_style()
            .as_ref()
            .and_then(|style| style.as_object())
            .and_then(|o| o.downcast_ref::<DashStyle>())
            .map(DashStyle::to_ref);

        let same = match (&*self.subscribed_to_dashes.borrow(), &new_value) {
            (Some((old, _)), Some(new)) => old.ptr_eq(new),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }

        if let Some((_, subscription)) = self.subscribed_to_dashes.take() {
            subscription.dispose();
        }

        if let Some(new_value) = new_value {
            let weak = self.to_ref().downgrade();
            let subscription = new_value.invalidated(move || {
                if let Some(target) = weak.upgrade() {
                    target.register_for_serialization();
                    target.raise_invalidated();
                }
            });
            *self.subscribed_to_dashes.borrow_mut() = Some((new_value, subscription));
        }
    }
}

impl ICompositionRenderResource for Pen {
    fn add_ref_on_compositor(&self, c: &Rc<Compositor>) {
        let owner = self.as_compositor_serializable();
        let (_, created) = self.resource.create_or_add_ref(c, Some(owner), |c| {
            c.create_server_object(|server, _| ServerCompositionSimplePen::new(server) as Rc<dyn IServerObject>)
        });
        if created {
            if let Some(brush) = self.brush() {
                if let Some(brush) = brush.as_composition_render_resource() {
                    brush.add_ref_on_compositor(c);
                }
            }
            self.update_dash_style_subscription();
        }
    }

    fn release_on_compositor(&self, c: &Rc<Compositor>) {
        if self.resource.release(c) {
            if let Some(brush) = self.brush() {
                if let Some(brush) = brush.as_composition_render_resource() {
                    brush.release_on_compositor(c);
                }
            }
            self.update_dash_style_subscription();
        }
    }

    fn get_for_compositor(&self, c: &Compositor) -> ServerObjectId {
        self.resource.get_for_compositor(c)
    }
}

impl ICompositorSerializable for RefAdapter<Pen> {
    fn try_get_server(&self, c: &Compositor) -> Option<ServerObjectId> {
        self.0.resource.try_get_for_compositor(c)
    }

    fn serialization_key(&self) -> *const () {
        RefAdapter::reference_id(self)
    }

    fn serialize_changes(&self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        let pen = &self.0;
        ServerCompositionSimplePenProps::serialize_all_changes(
            writer,
            brush_get_server_resource(pen.brush().as_ref(), Some(c)),
            pen.dash_style().map(|style| style.into_immutable_dash_style()),
            pen.line_cap(),
            pen.line_join(),
            pen.miter_limit(),
            pen.thickness(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::immutable::ImmutableSolidColorBrush;
    use crate::media::Colors;
    use std::cell::Cell;

    fn assert_invalidated(target: &Ref<Pen>, action: impl FnOnce()) {
        let raised = Rc::new(Cell::new(false));
        let r = raised.clone();
        let subscription = target.invalidated(move || r.set(true));
        action();
        assert!(raised.get());
        subscription.dispose();
    }

    fn immutable_test_pen() -> Rc<dyn IPen> {
        Rc::new(ImmutablePen::new(
            Some(Rc::new(ImmutableSolidColorBrush::new(Colors::RED))),
            2.0,
            Some(Rc::new(ImmutableDashStyle::new(Some(&[0.1, 0.2]), 5.0))),
            PenLineCap::Round,
            PenLineJoin::Round,
            21.0,
        ))
    }

    fn modify(pen: &mut Option<Rc<dyn IPen>>, brush: Option<Rc<dyn IBrush>>) -> bool {
        Pen::try_modify_or_create(pen, brush, 2.0, None, 0.0, PenLineCap::Flat, PenLineJoin::Miter, 10.0)
    }

    #[test]
    fn changing_thickness_raises_invalidated() {
        let target = Pen::new();
        assert_invalidated(&target, || target.set_thickness(18.0));
    }

    #[test]
    fn changing_dash_style_dashes_raises_invalidated() {
        let dashes = DashStyle::new();
        let target = Pen::new();
        target.set_dash_style(Some((&dashes).into()));
        assert_invalidated(&target, || dashes.set_dashes(Some(MediaCollection::from_items([0.1, 0.2]))));
    }

    #[test]
    fn adding_dash_style_dashes_raises_invalidated() {
        let dashes = DashStyle::new();
        let target = Pen::new();
        target.set_dash_style(Some((&dashes).into()));
        assert_invalidated(&target, || dashes.set_dashes(Some(MediaCollection::from_items([0.3]))));
    }

    #[test]
    fn adding_dash_style_dash_raises_invalidated() {
        let dashes = DashStyle::new();
        let target = Pen::new();
        target.set_dash_style(Some((&dashes).into()));
        dashes.set_dashes(Some(MediaCollection::from_items([0.3])));
        assert_invalidated(&target, || dashes.dashes().unwrap().add_range([1.0, 2.0]));
    }

    #[test]
    fn replaced_dash_style_no_longer_invalidates() {
        let dashes = DashStyle::new();
        let target = Pen::new();
        target.set_dash_style(Some((&dashes).into()));
        target.set_dash_style(None);
        let raised = Rc::new(Cell::new(false));
        let r = raised.clone();
        target.invalidated(move || r.set(true));
        dashes.set_offset(3.0);
        assert!(!raised.get());
    }

    #[test]
    fn equality_is_implemented_between_immutable_and_mutable_pens() {
        let brush: Rc<ImmutableSolidColorBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
        let target1: Rc<dyn IPen> = Rc::new(ImmutablePen::new(
            Some(brush.clone()),
            2.0,
            Some(DashStyle::dash()),
            PenLineCap::Round,
            PenLineJoin::Round,
            21.0,
        ));
        let target2: Rc<dyn IPen> = Pen::with_all(
            Some(brush),
            2.0,
            Some(DashStyle::dash()),
            PenLineCap::Round,
            PenLineJoin::Round,
            21.0,
        )
        .into();

        assert!(target1 == target2);
    }

    #[test]
    fn equality_is_implemented_between_mutable_and_immutable_dash_styles() {
        let brush: Rc<ImmutableSolidColorBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
        let target1: Rc<dyn IPen> = Rc::new(ImmutablePen::new(
            Some(brush.clone()),
            2.0,
            Some(Rc::new(ImmutableDashStyle::new(Some(&[0.1, 0.2]), 5.0))),
            PenLineCap::Round,
            PenLineJoin::Round,
            21.0,
        ));
        let target2: Rc<dyn IPen> = Pen::with_all(
            Some(brush),
            2.0,
            Some(DashStyle::with_dashes(Some(&[0.1, 0.2]), 5.0).into()),
            PenLineCap::Round,
            PenLineJoin::Round,
            21.0,
        )
        .into();

        assert!(target1 == target2);
    }

    #[test]
    fn try_modify_or_create_should_return_true_when_previous_exists_and_assign_null_when_brush_is_null() {
        let mut target = Some(immutable_test_pen());
        let result = modify(&mut target, None);
        assert!(result);
        assert!(target.is_none());
    }

    #[test]
    fn try_modify_or_create_should_return_false_when_previous_not_exists_and_assign_null_when_brush_is_null() {
        let mut target: Option<Rc<dyn IPen>> = None;
        let result = modify(&mut target, None);
        assert!(!result);
        assert!(target.is_none());
    }

    #[test]
    fn try_modify_or_create_should_return_true_when_previous_immutable_and_assign_mutable_when_brush_is_mutable() {
        let mut target = Some(immutable_test_pen());
        let result = modify(&mut target, Some(SolidColorBrush::with_color(Colors::BLUE).into()));
        assert!(result);
        assert!(target.unwrap().as_object().unwrap().is::<Pen>());
    }

    #[test]
    fn try_modify_or_create_should_return_true_when_previous_immutable_and_assign_immutable_when_brush_is_immutable()
    {
        let mut target = Some(immutable_test_pen());
        let result = modify(&mut target, Some(Rc::new(ImmutableSolidColorBrush::new(Colors::BLUE))));
        assert!(result);
        assert!(target.unwrap().as_any().is::<ImmutablePen>());
    }

    #[test]
    fn try_modify_or_create_should_return_false_when_previous_mutable_and_modify_mutable_when_brush_is_mutable() {
        let old_pen = Pen::with_all(
            Some(SolidColorBrush::with_color(Colors::RED).into()),
            2.0,
            Some(Rc::new(ImmutableDashStyle::new(Some(&[0.1, 0.2]), 5.0))),
            PenLineCap::Round,
            PenLineJoin::Round,
            21.0,
        );
        let mut target: Option<Rc<dyn IPen>> = Some((&old_pen).into());
        let result = modify(&mut target, Some(SolidColorBrush::with_color(Colors::BLUE).into()));
        assert!(!result);
        assert!(target.unwrap().as_object().unwrap().downcast_ref::<Pen>().unwrap().to_ref().ptr_eq(&old_pen));
    }

    #[test]
    fn try_modify_or_create_uses_dash_array_kind() {
        let mut target: Option<Rc<dyn IPen>> = None;
        let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::BLUE));
        assert!(Pen::try_modify_or_create(
            &mut target,
            Some(brush.clone()),
            2.0,
            Some(StrokeDashArray::List(&[1.0, 2.0])),
            3.0,
            PenLineCap::Flat,
            PenLineJoin::Miter,
            10.0
        ));
        let pen = target.clone().unwrap();
        assert!(pen.as_any().is::<ImmutablePen>());
        assert_eq!(Some(vec![1.0, 2.0]), pen.dash_style().unwrap().dashes());
        assert_eq!(3.0, pen.dash_style().unwrap().offset());

        // The immutable pen is replaced by a mutable one. The result is false
        // because the previous (immutable) pen compares structurally equal
        // to the new pen.
        let list = MediaCollection::from_items([1.0, 2.0]);
        assert!(!Pen::try_modify_or_create(
            &mut target,
            Some(brush),
            2.0,
            Some(StrokeDashArray::Observable(&list)),
            3.0,
            PenLineCap::Flat,
            PenLineJoin::Miter,
            10.0
        ));
        let pen = target.unwrap();
        assert!(pen.as_object().unwrap().is::<Pen>());
        assert!(pen.dash_style().unwrap().as_object().unwrap().is::<DashStyle>());
    }

    #[test]
    fn to_immutable_copies_values() {
        let pen = Pen::with_all(
            Some(SolidColorBrush::with_color(Colors::RED).into()),
            3.0,
            Some(DashStyle::with_dashes(Some(&[1.0, 2.0]), 4.0).into()),
            PenLineCap::Square,
            PenLineJoin::Round,
            7.0,
        );
        let immutable = pen.to_immutable();
        assert_eq!(3.0, immutable.thickness());
        assert_eq!(PenLineCap::Square, immutable.line_cap());
        assert_eq!(PenLineJoin::Round, immutable.line_join());
        assert_eq!(7.0, immutable.miter_limit());
        assert_eq!(Some(vec![1.0, 2.0]), immutable.dash_style().unwrap().dashes());
        assert_eq!(Some(Colors::RED), immutable.brush().unwrap().as_solid_color_brush().map(|b| b.color()));
        let as_pen: Rc<dyn IPen> = pen.into();
        assert_eq!(3.0, BrushExtensions::pen_to_immutable(&as_pen).thickness());
        let immutable: Rc<dyn IPen> = Rc::new(immutable);
        assert!(Rc::ptr_eq(&BrushExtensions::pen_to_immutable(&immutable), &immutable.clone().into_immutable_pen()));
    }
}
