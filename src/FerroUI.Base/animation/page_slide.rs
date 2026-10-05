use crate::animation::easings::Easing;
use crate::animation::{
    Animation, Cue, FillMode, IPageTransition, IProgressPageTransition, KeyFrame, PageTransitionItem, TimeSpan,
};
use crate::animation::i_page_transition::{start_async, when_all};
use crate::media::{ITransform, TranslateTransform};
use crate::styling::Setter;
use crate::threading::{CancellationToken, DispatcherTask};
use crate::{Ref, Visual};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The axis on which a [`PageSlide`] should occur.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SlideAxis {
    #[default]
    Horizontal = 0,
    Vertical = 1,
}

/// Transitions between two pages by sliding them horizontally or
/// vertically.
pub struct PageSlide {
    duration: Cell<TimeSpan>,
    orientation: Cell<SlideAxis>,
    slide_in_easing: RefCell<Easing>,
    slide_out_easing: RefCell<Easing>,
    fill_mode: Cell<FillMode>,
}

impl Default for PageSlide {
    fn default() -> Self {
        Self::new()
    }
}

impl PageSlide {
    /// Creates a horizontal slide with a zero duration.
    pub fn new() -> Self {
        Self::with_duration(TimeSpan::ZERO, SlideAxis::Horizontal)
    }

    /// Creates a slide with the given duration and axis.
    pub fn with_duration(duration: TimeSpan, orientation: SlideAxis) -> Self {
        Self {
            duration: Cell::new(duration),
            orientation: Cell::new(orientation),
            slide_in_easing: RefCell::new(Easing::default()),
            slide_out_easing: RefCell::new(Easing::default()),
            fill_mode: Cell::new(FillMode::Forward),
        }
    }

    /// The duration of the animation.
    pub fn duration(&self) -> TimeSpan {
        self.duration.get()
    }

    pub fn set_duration(&self, value: TimeSpan) {
        self.duration.set(value)
    }

    /// The direction of the animation.
    pub fn orientation(&self) -> SlideAxis {
        self.orientation.get()
    }

    pub fn set_orientation(&self, value: SlideAxis) {
        self.orientation.set(value)
    }

    /// The easing used for the page that slides in.
    pub fn slide_in_easing(&self) -> Easing {
        self.slide_in_easing.borrow().clone()
    }

    pub fn set_slide_in_easing(&self, value: impl Into<Easing>) {
        *self.slide_in_easing.borrow_mut() = value.into();
    }

    /// The easing used for the page that slides out.
    pub fn slide_out_easing(&self) -> Easing {
        self.slide_out_easing.borrow().clone()
    }

    pub fn set_slide_out_easing(&self, value: impl Into<Easing>) {
        *self.slide_out_easing.borrow_mut() = value.into();
    }

    /// The fill mode.
    pub fn fill_mode(&self) -> FillMode {
        self.fill_mode.get()
    }

    pub fn set_fill_mode(&self, value: FillMode) {
        self.fill_mode.set(value)
    }

    /// The common visual parent of the two controls. Panics when the
    /// controls have different parents or no parent.
    pub fn get_visual_parent(from: Option<&Ref<Visual>>, to: Option<&Ref<Visual>>) -> Ref<Visual> {
        let p1 = from.or(to).expect("a page").visual_parent();
        let p2 = to.or(from).expect("a page").visual_parent();

        if let (Some(p1), Some(p2)) = (&p1, &p2) {
            if p1 != p2 {
                panic!("Controls for PageSlide must have same parent.");
            }
        }

        match p1 {
            Some(p1) => p1,
            None => panic!("Cannot determine visual parent."),
        }
    }

    fn translate_transform(visual: &Ref<Visual>) -> Ref<TranslateTransform> {
        let existing = visual
            .render_transform()
            .and_then(|transform| transform.as_object().and_then(|o| o.to_ref().cast::<TranslateTransform>()));
        match existing {
            Some(transform) => transform,
            None => {
                let transform = TranslateTransform::new();
                let handle: Rc<dyn ITransform> = (&transform).into();
                visual.set_render_transform(Some(handle));
                transform
            }
        }
    }
}

impl IPageTransition for PageSlide {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        let horizontal = self.orientation.get() == SlideAxis::Horizontal;
        let fill_mode = self.fill_mode.get();
        let duration = self.duration.get();
        let slide_in_easing = self.slide_in_easing();
        let slide_out_easing = self.slide_out_easing();
        let from = from.cloned();
        let to = to.cloned();

        start_async(async move {
            if cancellation_token.is_cancellation_requested() {
                return;
            }

            let mut tasks = Vec::new();
            let parent = Self::get_visual_parent(from.as_ref(), to.as_ref());
            let distance = if horizontal { parent.bounds().width } else { parent.bounds().height };
            let translate_property =
                if horizontal { TranslateTransform::x_property() } else { TranslateTransform::y_property() };

            let slide = |easing: Easing, start: f64, end: f64| {
                let animation = Animation::new();
                animation.set_fill_mode(fill_mode);
                animation.set_easing(easing);
                animation
                    .children()
                    .add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(translate_property, start) as _]));
                animation
                    .children()
                    .add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(translate_property, end) as _]));
                animation.set_duration(duration);
                animation
            };

            if let Some(from) = &from {
                let animation = slide(slide_out_easing, 0.0, if forward { -distance } else { distance });
                tasks.push(animation.run_async_with_clock(from, None, cancellation_token.clone()));
            }

            if let Some(to) = &to {
                to.set_is_visible(true);
                let animation = slide(slide_in_easing, if forward { distance } else { -distance }, 0.0);
                tasks.push(animation.run_async_with_clock(to, None, cancellation_token.clone()));
            }

            when_all(tasks).await;

            if cancellation_token.is_cancellation_requested() {
                return;
            }

            if let Some(from) = &from {
                from.set_is_visible(false);
                if fill_mode != FillMode::None {
                    from.set_render_transform(None);
                }
            }

            if let Some(to) = &to {
                if fill_mode != FillMode::None {
                    to.set_render_transform(None);
                }
            }
        })
    }

    fn as_progress_page_transition(&self) -> Option<&dyn IProgressPageTransition> {
        Some(self)
    }

    fn as_page_slide(&self) -> Option<&PageSlide> {
        Some(self)
    }
}

impl IProgressPageTransition for PageSlide {
    fn update(
        &self,
        progress: f64,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        page_length: f64,
        visible_items: &[PageTransitionItem],
    ) {
        if !visible_items.is_empty() {
            return;
        }

        if from.is_none() && to.is_none() {
            return;
        }

        let parent = Self::get_visual_parent(from, to);
        let horizontal = self.orientation.get() == SlideAxis::Horizontal;
        let distance = if page_length > 0.0 {
            page_length
        } else if horizontal {
            parent.bounds().width
        } else {
            parent.bounds().height
        };
        let offset = distance * progress;

        if let Some(from) = from {
            let ft = Self::translate_transform(from);
            if horizontal {
                ft.set_x(if forward { -offset } else { offset });
            } else {
                ft.set_y(if forward { -offset } else { offset });
            }
        }

        if let Some(to) = to {
            to.set_is_visible(true);
            let tt = Self::translate_transform(to);
            if horizontal {
                tt.set_x(if forward { distance - offset } else { -(distance - offset) });
            } else {
                tt.set_y(if forward { distance - offset } else { -(distance - offset) });
            }
        }
    }

    fn reset(&self, visual: &Ref<Visual>) {
        visual.set_render_transform(None);
    }
}
