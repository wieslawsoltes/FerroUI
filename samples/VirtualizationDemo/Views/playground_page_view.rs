//! Port of `Views/PlaygroundPageView.xaml.cs`: the class of the document
//! `Views/PlaygroundPageView.xaml`.

use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, ListBox, TextBlock, UserControl};
use std::rc::Rc;
use std::time::Duration;

#[repr(C)]
pub struct PlaygroundPageView {
    base: UserControl,
    timer: Rc<DispatcherTimer>,
}

ferro_class!(PlaygroundPageView: UserControl);
ferro_impl_classes!(
    PlaygroundPageView: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(PlaygroundPageView { new: PlaygroundPageView::new });
xaml_class!(PlaygroundPageView, "/Views/PlaygroundPageView.xaml");

impl VisualImpl for PlaygroundPageView {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.timer.start();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.timer.stop();
    }
}

impl PlaygroundPageView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), timer: DispatcherTimer::new() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        this.timer.set_interval(Duration::from_millis(500));

        // The managed original subscribes a method of the view to the timer the view owns: a
        // cycle its collector frees. Here the handler holds the view weakly.
        let weak = this.downgrade();
        this.timer.tick(move |_| {
            if let Some(this) = weak.upgrade() {
                this.timer_tick();
            }
        });

        this
    }

    fn timer_tick(&self) {
        // The fields the build of the managed original generates for the named elements.
        let list = self.get_control::<ListBox>("list");
        let item_count = self.get_control::<TextBlock>("itemCount");

        let message = format!(
            "Realized {} of {}",
            list.get_realized_containers().len(),
            list.items_panel_root().map(|panel| panel.children().count().to_string()).unwrap_or_default()
        );
        item_count.set_text(Some(&message));
    }
}
