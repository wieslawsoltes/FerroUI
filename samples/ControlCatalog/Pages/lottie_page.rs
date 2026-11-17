use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::media::Stretch;
use ferroui_base::{BoxedValue, Ref, ferro_class_info, instantiate};
use ferroui_controls::{ComboBox, ContentPage};
use ferroui_controls_lottie::Lottie;
use std::rc::Rc;

#[repr(C)]
pub struct LottiePage {
    base: ContentPage,
}

content_page_class!(LottiePage);
ferro_class_info!(LottiePage {
    new: LottiePage::new,
    markup: {
        methods: [
            fn OnAnimationChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<LottiePage>, _, _| this.change_animation(),
            fn OnStretchChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<LottiePage>, _, _| this.change_stretch(),
            fn OnRepeatChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<LottiePage>, _, _| this.change_repeat(),
            fn OnPlay(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<LottiePage>, _, _| { if let Some(player) = this.player() { player.play(); } },
            fn OnPause(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<LottiePage>, _, _| { if let Some(player) = this.player() { player.pause(); } },
            fn OnResume(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<LottiePage>, _, _| { if let Some(player) = this.player() { player.resume(); } },
            fn OnStop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<LottiePage>, _, _| { if let Some(player) = this.player() { player.stop(); } },
        ],
    },
});
xaml_class!(LottiePage, "/Pages/LottiePage.xaml");

impl LottiePage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn player(&self) -> Option<Ref<Lottie>> {
        self.find_control::<Lottie>("Player")
    }

    fn selection(&self, name: &str) -> Option<i32> {
        self.find_control::<ComboBox>(name)
            .map(|combo| combo.selected_index())
    }

    fn change_animation(&self) {
        let Some(player) = self.player() else { return };
        let path = match self.selection("Animation") {
            Some(0) => "/Assets/Lottie/Orbit.json",
            Some(1) => "/Assets/Lottie/Signal.json",
            _ => return,
        };
        player.set_path(Some(path.into()));
    }

    fn change_stretch(&self) {
        let Some(player) = self.player() else { return };
        let stretch = match self.selection("Fit") {
            Some(0) => Stretch::Uniform,
            Some(1) => Stretch::Fill,
            Some(2) => Stretch::UniformToFill,
            Some(3) => Stretch::None,
            _ => return,
        };
        player.set_stretch(stretch);
    }

    fn change_repeat(&self) {
        let Some(player) = self.player() else { return };
        let repeat = match self.selection("Repeats") {
            Some(0) => Lottie::INFINITY,
            Some(1) => 1,
            Some(2) => 2,
            _ => return,
        };
        player.set_repeat_count(repeat);
    }
}
