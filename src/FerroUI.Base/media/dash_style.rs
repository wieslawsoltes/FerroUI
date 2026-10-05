use crate::animation::Animatable;
use crate::media::immutable::ImmutableDashStyle;
use crate::media::MediaCollection;
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Represents the sequence of dashes and gaps that will be applied by a
/// [`Pen`](crate::media::Pen).
#[repr(C)]
pub struct DashStyle {
    base: Animatable,
    invalidated: HandlerList<dyn Fn()>,
    dashes_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(DashStyle: Animatable);
crate::ferro_class_info!(DashStyle { new: DashStyle::new, interfaces: [std::rc::Rc<dyn crate::media::IDashStyle>] });

impl FerroObjectImpl for DashStyle {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::dashes_property().as_property() {
            let new_value = change.get_new_value::<Option<MediaCollection<f64>>>();

            if let Some(subscription) = this.dashes_subscription.take() {
                subscription.dispose();
            }

            if let Some(new_value) = new_value {
                let weak = this.to_ref().downgrade();
                let subscription = new_value.collection_changed(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.raise_invalidated();
                    }
                });
                *this.dashes_subscription.borrow_mut() = Some(subscription);
            }
        }
    }
}

crate::ferro_properties! { impl DashStyle {
    ferro_property!(pub fn dashes_property() -> StyledProperty<Option<MediaCollection<f64>>> {
        FerroProperty::register::<DashStyle, _>("Dashes", None)
    });

    ferro_property!(pub fn offset_property() -> StyledProperty<f64> {
        FerroProperty::register::<DashStyle, _>("Offset", 0.0)
    });
} }

impl DashStyle {
    fn static_constructor() {
        Self::dashes_property().changed().add_class_handler::<DashStyle>(|x, _| x.raise_invalidated());
        Self::offset_property().changed().add_class_handler::<DashStyle>(|x, _| x.raise_invalidated());
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Animatable::construct(), invalidated: HandlerList::new(), dashes_subscription: RefCell::new(None) }
    }

    /// Creates a dash style without dashes.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a dash style with a copy of the given dashes.
    pub fn with_dashes(dashes: Option<&[f64]>, offset: f64) -> Ref<Self> {
        Self::with_dash_list(MediaCollection::from_items(dashes.unwrap_or(&[]).iter().copied()), offset)
    }

    /// Creates a dash style that shares the given dash list.
    pub fn with_dash_list(dashes: MediaCollection<f64>, offset: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_dashes(Some(dashes));
        result.set_offset(offset);
        result
    }

    /// Represents a dashed dash style.
    pub fn dash() -> Rc<ImmutableDashStyle> {
        thread_local! {
            static DASH: Rc<ImmutableDashStyle> = Rc::new(ImmutableDashStyle::new(Some(&[2.0, 2.0]), 1.0));
        }
        DASH.with(Rc::clone)
    }

    /// Represents a dotted dash style.
    pub fn dot() -> Rc<ImmutableDashStyle> {
        thread_local! {
            static DOT: Rc<ImmutableDashStyle> = Rc::new(ImmutableDashStyle::new(Some(&[0.0, 2.0]), 0.0));
        }
        DOT.with(Rc::clone)
    }

    /// Represents a dashed dotted dash style.
    pub fn dash_dot() -> Rc<ImmutableDashStyle> {
        thread_local! {
            static DASH_DOT: Rc<ImmutableDashStyle> =
                Rc::new(ImmutableDashStyle::new(Some(&[2.0, 2.0, 0.0, 2.0]), 1.0));
        }
        DASH_DOT.with(Rc::clone)
    }

    /// Represents a dashed double dotted dash style.
    pub fn dash_dot_dot() -> Rc<ImmutableDashStyle> {
        thread_local! {
            static DASH_DOT_DOT: Rc<ImmutableDashStyle> =
                Rc::new(ImmutableDashStyle::new(Some(&[2.0, 2.0, 0.0, 2.0, 0.0, 2.0]), 1.0));
        }
        DASH_DOT_DOT.with(Rc::clone)
    }

    /// The length of alternating dashes and gaps.
    pub fn dashes(&self) -> Option<MediaCollection<f64>> {
        self.get_value(Self::dashes_property())
    }

    pub fn set_dashes(&self, value: Option<MediaCollection<f64>>) {
        self.set_value(Self::dashes_property(), value)
    }

    /// How far in the dash sequence the stroke will start.
    pub fn offset(&self) -> f64 {
        self.get_value(Self::offset_property())
    }

    pub fn set_offset(&self, value: f64) {
        self.set_value(Self::offset_property(), value)
    }

    /// Subscribes to invalidation of the dash style: raised when the dashes
    /// or the offset change. Disposing the returned handle unsubscribes.
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

    /// Returns an immutable clone of the dash style.
    pub fn to_immutable(&self) -> ImmutableDashStyle {
        ImmutableDashStyle::new(self.dashes().map(|dashes| dashes.to_vec()).as_deref(), self.offset())
    }
}
