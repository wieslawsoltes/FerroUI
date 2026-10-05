//! Port of `ViewModels/TransitioningContentControlPageViewModel.cs`.

use ferroui_base::animation::{
    Animation, AnimationTask, CompositePageTransition, CrossFade, Cue, IPageTransition, KeyFrame, PageSlide, SlideAxis,
    TimeSpan,
};
use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{IImage, ScaleTransform};
use ferroui_base::platform::AssetLoader;
use ferroui_base::styling::Setter;
use ferroui_base::threading::{CancellationToken, DispatcherTask};
use ferroui_base::utilities::Uri;
use ferroui_base::{Ref, Visual};
use ferroui_controls::ItemsSource;
use mini_mvvm::{start_async, ViewModelBase};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// The view model of the page of the transitioning content control.
pub struct TransitioningContentControlPageViewModel {
    base: ViewModelBase,
    page_transitions: Rc<BindableList<Rc<PageTransition>>>,
    images: Vec<Rc<Bitmap>>,
    selected_image: RefCell<Rc<Bitmap>>,
    reversed: Cell<bool>,
    selected_transition: RefCell<Rc<PageTransition>>,
    clip_to_bounds: Cell<bool>,
    duration: Cell<i32>,
}

impl PartialEq for TransitioningContentControlPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for TransitioningContentControlPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl TransitioningContentControlPageViewModel {
    /// # Panics
    /// Panics if an image of the page cannot be opened or decoded (an
    /// exception of the constructor in the managed original).
    pub fn new() -> Rc<TransitioningContentControlPageViewModel> {
        let images = ["delicate-arch-896885_640.jpg", "hirsch-899118_640.jpg", "maple-leaf-888807_640.jpg"];

        let images: Vec<Rc<Bitmap>> = images
            .into_iter()
            .map(|image| {
                let path = format!("ferres://ControlCatalog/Assets/{image}");
                let uri = Uri::absolute(&path).unwrap_or_else(|e| panic!("{e}"));
                let mut stream = AssetLoader::open(&uri, None).unwrap_or_else(|e| panic!("{e}"));
                Rc::new(Bitmap::from_stream(&mut stream).unwrap_or_else(|e| panic!("{e}")))
            })
            .collect();

        Self::with_images(images)
    }

    /// The rest of the constructor, for the images it has loaded.
    fn with_images(images: Vec<Rc<Bitmap>>) -> Rc<TransitioningContentControlPageViewModel> {
        let page_transitions = BindableList::new([]);
        let duration = 500;
        Self::setup_transitions_of(&page_transitions, duration);

        let selected_transition = page_transitions.items().get(1);
        let selected_image = images[0].clone();

        Rc::new(Self {
            base: ViewModelBase::new(),
            page_transitions,
            images,
            selected_image: RefCell::new(selected_image),
            reversed: Cell::new(false),
            selected_transition: RefCell::new(selected_transition),
            clip_to_bounds: Cell::new(false),
            duration: Cell::new(duration),
        })
    }

    pub fn page_transitions(&self) -> Rc<BindableList<Rc<PageTransition>>> {
        self.page_transitions.clone()
    }

    pub fn images(&self) -> Vec<Rc<Bitmap>> {
        self.images.clone()
    }

    /// The selected image.
    pub fn selected_image(&self) -> Rc<Bitmap> {
        self.selected_image.borrow().clone()
    }

    pub fn set_selected_image(&self, value: Rc<Bitmap>) {
        if !Rc::ptr_eq(&self.selected_image.borrow(), &value) {
            *self.selected_image.borrow_mut() = value;
            self.base.raise_property_changed("SelectedImage");
        }
    }

    /// The transition to play.
    pub fn selected_transition(&self) -> Rc<PageTransition> {
        self.selected_transition.borrow().clone()
    }

    pub fn set_selected_transition(&self, value: Rc<PageTransition>) {
        self.base.raise_and_set_if_changed(&self.selected_transition, value, "SelectedTransition");
    }

    /// Whether the content is clipped to its bounds.
    pub fn clip_to_bounds(&self) -> bool {
        self.clip_to_bounds.get()
    }

    pub fn set_clip_to_bounds(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.clip_to_bounds, value, "ClipToBounds");
    }

    /// The duration of the transitions, in milliseconds.
    pub fn duration(&self) -> i32 {
        self.duration.get()
    }

    pub fn set_duration(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.duration, value, "Duration");
        self.setup_transitions();
    }

    /// Whether the animation is reversed.
    pub fn reversed(&self) -> bool {
        self.reversed.get()
    }

    pub fn set_reversed(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.reversed, value, "Reversed");
    }

    fn setup_transitions(&self) {
        Self::setup_transitions_of(&self.page_transitions, self.duration());
    }

    fn setup_transitions_of(page_transitions: &Rc<BindableList<Rc<PageTransition>>>, duration: i32) {
        let page_transitions = page_transitions.items();
        if page_transitions.count() == 0 {
            page_transitions.add_range([
                PageTransition::new("None"),
                PageTransition::new("CrossFade"),
                PageTransition::new("Slide horizontally"),
                PageTransition::new("Slide vertically"),
                PageTransition::new("Composite"),
                PageTransition::new("Custom"),
            ]);
        }

        let duration = TimeSpan::from_milliseconds(f64::from(duration));
        let cross_fade: Rc<dyn IPageTransition> = Rc::new(CrossFade::with_duration(duration));
        let slide_horizontally: Rc<dyn IPageTransition> =
            Rc::new(PageSlide::with_duration(duration, SlideAxis::Horizontal));
        let slide_vertically: Rc<dyn IPageTransition> = Rc::new(PageSlide::with_duration(duration, SlideAxis::Vertical));
        page_transitions.get(1).set_transition(Some(cross_fade.clone()));
        page_transitions.get(2).set_transition(Some(slide_horizontally.clone()));
        page_transitions.get(3).set_transition(Some(slide_vertically.clone()));

        let composite_transition = CompositePageTransition::new();
        composite_transition.add(cross_fade);
        composite_transition.add(slide_horizontally);
        composite_transition.add(slide_vertically);
        page_transitions.get(4).set_transition(Some(Rc::new(composite_transition)));

        page_transitions.get(5).set_transition(Some(Rc::new(CustomTransition::with_duration(duration))));
    }

    fn index_of_selected_image(&self) -> i32 {
        let selected = self.selected_image();
        self.images.iter().position(|image| Rc::ptr_eq(image, &selected)).map_or(-1, |index| index as i32)
    }

    pub fn next_image(&self) {
        self.set_reversed(false);
        let mut index = self.index_of_selected_image() + 1;

        if index >= self.images.len() as i32 {
            index = 0;
        }

        self.set_selected_image(self.images[index as usize].clone());
    }

    pub fn prev_image(&self) {
        self.set_reversed(true);
        let mut index = self.index_of_selected_image() - 1;

        if index < 0 {
            index = self.images.len() as i32 - 1;
        }

        self.set_selected_image(self.images[index as usize].clone());
    }
}

ferro_markup_type!(class TransitioningContentControlPageViewModel {
    this: Rc<TransitioningContentControlPageViewModel>,
    handles: [
        TransitioningContentControlPageViewModel,
        Rc<TransitioningContentControlPageViewModel>,
        Option<Rc<TransitioningContentControlPageViewModel>>
    ],
    constructors: [() => TransitioningContentControlPageViewModel::new],
    properties: [
        // A list a binding delivers to an items source property.
        PageTransitions: ItemsSource {
            get: |this: &Rc<TransitioningContentControlPageViewModel>| ItemsSource::from(this.page_transitions())
        },
        // Declared with the contract of the bitmap: a bitmap handle has no equality and
        // cannot be the type of a declared property.
        SelectedImage: Rc<dyn IImage> {
            get: |this: &Rc<TransitioningContentControlPageViewModel>| this.selected_image() as Rc<dyn IImage>
        },
        SelectedTransition: Rc<PageTransition> {
            get: |this: &Rc<TransitioningContentControlPageViewModel>| this.selected_transition(),
            set: |this: &Rc<TransitioningContentControlPageViewModel>, value: Rc<PageTransition>| {
                this.set_selected_transition(value)
            }
        },
        ClipToBounds: bool {
            get: |this: &Rc<TransitioningContentControlPageViewModel>| this.clip_to_bounds(),
            set: |this: &Rc<TransitioningContentControlPageViewModel>, value: bool| this.set_clip_to_bounds(value)
        },
        Duration: i32 {
            get: |this: &Rc<TransitioningContentControlPageViewModel>| this.duration(),
            set: |this: &Rc<TransitioningContentControlPageViewModel>, value: i32| this.set_duration(value)
        },
        Reversed: bool {
            get: |this: &Rc<TransitioningContentControlPageViewModel>| this.reversed(),
            set: |this: &Rc<TransitioningContentControlPageViewModel>, value: bool| this.set_reversed(value)
        },
    ],
    methods: [
        fn NextImage() => |this: &Rc<TransitioningContentControlPageViewModel>| this.next_image(),
        fn PrevImage() => |this: &Rc<TransitioningContentControlPageViewModel>| this.prev_image(),
    ],
    notify_property_changed: TransitioningContentControlPageViewModel,
});

/// A transition of the page, with the title it is listed with.
pub struct PageTransition {
    base: ViewModelBase,
    display_title: String,
    transition: RefCell<Option<Rc<dyn IPageTransition>>>,
}

impl PartialEq for PageTransition {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for PageTransition {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl PageTransition {
    pub fn new(display_title: &str) -> Rc<PageTransition> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            display_title: display_title.to_string(),
            transition: RefCell::new(None),
        })
    }

    pub fn display_title(&self) -> String {
        self.display_title.clone()
    }

    /// The transition.
    pub fn transition(&self) -> Option<Rc<dyn IPageTransition>> {
        self.transition.borrow().clone()
    }

    pub fn set_transition(&self, value: Option<Rc<dyn IPageTransition>>) {
        self.base.raise_and_set_if_changed(&self.transition, value, "Transition");
    }
}

/// `ToString()`: the title.
impl fmt::Display for PageTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display_title)
    }
}

ferro_markup_type!(class PageTransition {
    this: Rc<PageTransition>,
    handles: [PageTransition, Rc<PageTransition>, Option<Rc<PageTransition>>],
    constructors: [(String) => |display_title: String| PageTransition::new(&display_title)],
    properties: [
        DisplayTitle: String { get: |this: &Rc<PageTransition>| this.display_title() },
        Transition: Option<Rc<dyn IPageTransition>> {
            get: |this: &Rc<PageTransition>| this.transition(),
            set: |this: &Rc<PageTransition>, value: Option<Rc<dyn IPageTransition>>| this.set_transition(value)
        },
    ],
    methods: [fn ToString() -> String => |this: &Rc<PageTransition>| this.to_string()],
    notify_property_changed: PageTransition,
});

/// A transition that scales the pages vertically: the old one to nothing,
/// the new one from nothing.
#[derive(Default)]
pub struct CustomTransition {
    duration: Cell<TimeSpan>,
}

impl PartialEq for CustomTransition {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl CustomTransition {
    /// Initializes a new instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initializes a new instance with the duration of the animation.
    pub fn with_duration(duration: TimeSpan) -> Self {
        Self { duration: Cell::new(duration) }
    }

    /// The duration of the animation.
    pub fn duration(&self) -> TimeSpan {
        self.duration.get()
    }

    pub fn set_duration(&self, value: TimeSpan) {
        self.duration.set(value);
    }

    /// The common visual parent of the two controls; either may be `None`,
    /// but not both.
    ///
    /// # Panics
    /// Panics if the two controls do not share a common parent, or have
    /// none.
    fn get_visual_parent(from: Option<&Ref<Visual>>, to: Option<&Ref<Visual>>) -> Ref<Visual> {
        let p1 = from.or(to).expect("a control").visual_parent();
        let p2 = to.or(from).expect("a control").visual_parent();

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
}

impl IPageTransition for CustomTransition {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        _forward: bool,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        let duration = self.duration();
        let from = from.cloned();
        let to = to.cloned();

        start_async(async move {
            if cancellation_token.is_cancellation_requested() {
                return;
            }

            let mut tasks = Vec::new();
            let _parent = Self::get_visual_parent(from.as_ref(), to.as_ref());
            let scale_property = ScaleTransform::scale_y_property();

            let scale = |start: f64, end: f64| {
                let animation = Animation::new();
                animation.children().add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(scale_property, start) as _]));
                animation.children().add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(scale_property, end) as _]));
                animation.set_duration(duration);
                animation
            };

            if let Some(from) = &from {
                let animation = scale(1.0, 0.0);
                tasks.push(animation.run_async(from, cancellation_token.clone()));
            }

            if let Some(to) = &to {
                to.set_is_visible(true);
                let animation = scale(0.0, 1.0);
                tasks.push(animation.run_async(to, cancellation_token.clone()));
            }

            if let Err(error) = AnimationTask::when_all(tasks).await {
                panic!("{error}");
            }

            if let Some(from) = &from {
                if !cancellation_token.is_cancellation_requested() {
                    from.set_is_visible(false);
                }
            }
        })
    }
}

ferro_markup_type!(class CustomTransition {
    this: Rc<CustomTransition>,
    handles: [CustomTransition, Rc<CustomTransition>, Option<Rc<CustomTransition>>],
    interfaces: [Rc<dyn IPageTransition>],
    constructors: [
        () => || Rc::new(CustomTransition::new()),
        (TimeSpan) => |duration: TimeSpan| Rc::new(CustomTransition::with_duration(duration)),
    ],
    properties: [
        Duration: TimeSpan {
            get: |this: &Rc<CustomTransition>| this.duration(),
            set: |this: &Rc<CustomTransition>, value: TimeSpan| this.set_duration(value)
        },
    ],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_transitions_are_listed_and_rebuilt_for_a_duration() {
        let page_transitions = BindableList::new([]);
        TransitioningContentControlPageViewModel::setup_transitions_of(&page_transitions, 500);
        let titles: Vec<_> = page_transitions.items().to_vec().iter().map(|t| t.to_string()).collect();
        assert_eq!(
            vec!["None", "CrossFade", "Slide horizontally", "Slide vertically", "Composite", "Custom"],
            titles
        );
        assert!(page_transitions.items().get(0).transition().is_none());
        let slide = page_transitions.items().get(2).transition().expect("a transition");
        assert_eq!(TimeSpan::from_milliseconds(500.0), slide.as_page_slide().expect("a slide").duration());
        assert_eq!(SlideAxis::Horizontal, slide.as_page_slide().expect("a slide").orientation());
        let composite = page_transitions.items().get(4).transition().expect("a transition");
        assert_eq!(3, composite.as_composite_page_transition().expect("a composite").page_transitions().len());

        // The same six entries get new transitions, and notify.
        let changed = Rc::new(Cell::new(0));
        let sink = changed.clone();
        page_transitions.items().get(3).property_changed().add(Rc::new(move |_: &str| sink.set(sink.get() + 1)));
        TransitioningContentControlPageViewModel::setup_transitions_of(&page_transitions, 750);
        assert_eq!(6, page_transitions.items().count());
        assert_eq!(1, changed.get());
        let slide = page_transitions.items().get(3).transition().expect("a transition");
        assert_eq!(TimeSpan::from_milliseconds(750.0), slide.as_page_slide().expect("a slide").duration());
    }

    #[test]
    fn a_custom_transition_has_no_duration_until_it_is_given_one() {
        let transition = CustomTransition::new();
        assert_eq!(TimeSpan::ZERO, transition.duration());
        transition.set_duration(TimeSpan::from_milliseconds(250.0));
        assert_eq!(TimeSpan::from_milliseconds(250.0), transition.duration());
    }
}
