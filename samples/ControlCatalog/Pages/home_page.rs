//! Port of `Pages/HomePage.xaml.cs`: the class of the document
//! `Pages/HomePage.xaml`.

use crate::markup::xaml_class;
use ferroui_base::animation::{Animation, Cue, IterationCount, KeyFrame, TimeSpan};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::TranslateTransform;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::Setter;
use ferroui_base::threading::CancellationToken;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::{TemplatedControlImpl, UniformGrid};
use ferroui_controls::{ContentPage, Control, ControlImpl, PageImpl, SizeChangedEventArgs};
use std::rc::Rc;
use std::time::Duration;

#[repr(C)]
pub struct HomePage {
    base: ContentPage,
}

ferro_class!(HomePage: ContentPage);
ferro_impl_classes!(
    HomePage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);
ferro_class_info!(HomePage {
    new: HomePage::new,
    markup: {
        methods: [
            fn UniformGrid_OnSizeChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<HomePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SizeChangedEventArgs>() {
                        this.uniform_grid_on_size_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(HomePage, "/Pages/HomePage.xaml");

impl VisualImpl for HomePage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        Self::start_floating_animation(&this.banner_logo("BannerLogo1"), 12.0, 8.0, Duration::from_secs(12));
        Self::start_floating_animation(&this.banner_logo("BannerLogo2"), -14.0, 8.0, Duration::from_secs(10));
        Self::start_floating_animation(&this.banner_logo("BannerLogo3"), -12.0, -12.0, Duration::from_secs(14));
        Self::start_floating_animation(&this.banner_logo("BannerLogo4"), 12.0, -8.0, Duration::from_secs(11));
    }
}

impl HomePage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// The named elements `BannerLogo1` to `BannerLogo4`.
    fn banner_logo(&self, name: &str) -> Ref<Control> {
        self.get_control::<Control>(name)
    }

    fn start_floating_animation(target: &Ref<Control>, dx: f64, dy: f64, duration: Duration) {
        let transform = TranslateTransform::new();
        target.set_render_transform(Some(transform.into()));

        let animation = Animation::new();
        animation.set_duration(TimeSpan::from_seconds(duration.as_secs_f64()));
        animation.set_iteration_count(IterationCount::INFINITE);
        for (cue, x, y) in [(0.0, 0.0, 0.0), (0.5, dx, dy), (1.0, 0.0, 0.0)] {
            let key_frame = KeyFrame::new();
            key_frame.set_cue(Cue::new(cue));
            key_frame.setters().add(Setter::new(TranslateTransform::x_property(), x));
            key_frame.setters().add(Setter::new(TranslateTransform::y_property(), y));
            animation.children().add(key_frame);
        }

        drop(animation.run_async(target, CancellationToken::none()));
    }

    fn uniform_grid_on_size_changed(&self, sender: &Option<BoxedValue>, e: &SizeChangedEventArgs) {
        let Some(grid) = from_markup_value::<Ref<UniformGrid>>(sender) else { return };

        const MIN_ITEM_WIDTH: f64 = 248.0;
        grid.set_columns(
            (((e.new_size().width + grid.column_spacing()) / (MIN_ITEM_WIDTH + grid.column_spacing())) as i32).max(1),
        );
    }
}
