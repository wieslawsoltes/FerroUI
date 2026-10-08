//! Port of `Pages/OpenGlPage.xaml.cs`: the class of the document
//! `Pages/OpenGlPage.xaml` and the control the page draws with.

use super::open_gl::{GlPageKnobs, OpenGlContent};
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::IImage;
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl,
    FerroPropertyChangedEventArgs, PixelSize, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{Button, ContentPage, Control, ControlImpl, Image, ScrollViewer, TopLevel, Window};
use ferroui_opengl::controls::{OpenGlControlBase, OpenGlControlBaseImpl};
use ferroui_opengl::{GlInterface, GlProfileType, GlVersion};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct OpenGlPage {
    base: ContentPage,
}

content_page_class!(OpenGlPage);
ferro_class_info!(OpenGlPage {
    new: OpenGlPage::new,
    markup: {
        methods: [
            fn SnapshotClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<OpenGlPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.snapshot_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(OpenGlPage, "/Pages/OpenGlPage.xaml");

impl OpenGlPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    /// # Panics
    /// Panics if the document does not name the elements the page uses (a
    /// null reference in the managed original).
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let gl = this.find_control::<OpenGlPageControl>("GL").expect("the element GL");
        let knobs = this.find_control::<GlPageKnobs>("Knobs").expect("the element Knobs");
        gl.init(&knobs);

        // The handler belongs to the page: it holds the page weakly.
        let weak = this.downgrade();
        let _attached: Rc<dyn IDisposable> = this.attached_to_visual_tree(move |_| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            if this.get_window().is_some() {
                this.find_control::<Button>("Snapshot").expect("the element Snapshot").set_is_visible(true);
            }
        });
        this
    }

    /// `TopLevel.GetTopLevel(this)` as a window.
    fn get_window(&self) -> Option<Ref<Window>> {
        TopLevel::get_top_level(Some(self)).and_then(|top_level| top_level.cast::<Window>())
    }

    /// # Panics
    /// Panics if the page has no composition visual or is not in a window (a
    /// null reference and a failed cast in the managed original), and if the
    /// snapshot fails (an exception of the asynchronous handler).
    fn snapshot_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let v = ElementComposition::get_element_visual(self).expect("the composition visual of the page");
        let snap = v.compositor().create_composition_visual_snapshot(&v, 1.5);
        // What follows the `await` of the original: the continuation of the snapshot. It
        // keeps the page alive, as the asynchronous method does.
        let this = self.to_ref();
        let task = snap.clone();
        snap.on_completed(move || {
            let snap = match task.take_result() {
                Some(Ok(snap)) => snap,
                Some(Err(error)) => panic!("{error}"),
                None => return,
            };
            let image = Image::new();
            let source: Rc<dyn IImage> = Rc::new(snap);
            image.set_source(Some(source));

            let scroll_viewer = ScrollViewer::new();
            scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
            scroll_viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
            scroll_viewer.set_content(Some(Control::boxed(image)));

            let window = Window::new();
            window.set_content(Some(Control::boxed(scroll_viewer)));
            let owner = this.get_window().expect("the page is in a window");
            drop(window.show_dialog(&owner));
        });
    }
}

#[repr(C)]
pub struct OpenGlPageControl {
    base: OpenGlControlBase,
    content: RefCell<OpenGlContent>,
    knobs: RefCell<Option<Ref<GlPageKnobs>>>,
}

ferro_class!(OpenGlPageControl: OpenGlControlBase);
ferro_impl_classes!(
    OpenGlPageControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(OpenGlPageControl { new: OpenGlPageControl::new });

impl OpenGlControlBaseImpl for OpenGlPageControl {
    fn on_open_gl_init(this: &Self, gl: &GlInterface) {
        // A control without a context has the default value of the version in the original.
        let version = this.gl_version().unwrap_or(GlVersion::new(GlProfileType::OpenGL, 0, 0));
        this.content.borrow_mut().init(gl, version);
    }

    fn on_open_gl_deinit(this: &Self, gl: &GlInterface) {
        this.content.borrow().deinit(gl);
    }

    fn on_open_gl_render(this: &Self, gl: &GlInterface, fb: i32) {
        let Some(knobs) = this.knobs.borrow().clone() else {
            return;
        };
        let bounds = this.bounds();
        this.content.borrow().on_open_gl_render(
            gl,
            fb,
            PixelSize::new(bounds.width as i32, bounds.height as i32),
            knobs.yaw(),
            knobs.pitch(),
            knobs.roll(),
            knobs.disco(),
        );
        if f64::from(knobs.disco()) > 0.01 {
            this.request_next_frame_rendering();
        }
        let info = this.content.borrow().info().to_string();
        knobs.set_info(info);
    }
}

impl OpenGlPageControl {
    /// # Panics
    /// Panics if the model of the content cannot be read (an exception of
    /// the field initializer in the managed original).
    pub fn construct() -> Self {
        Self { base: OpenGlControlBase::construct(), content: RefCell::new(OpenGlContent::new()), knobs: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn init(&self, knobs: &Ref<GlPageKnobs>) {
        *self.knobs.borrow_mut() = Some(knobs.clone());
        // The handler belongs to the knobs, a sibling of the control: it holds the control weakly.
        let weak = self.to_ref().downgrade();
        let _subscription: Rc<dyn IDisposable> = knobs.property_changed(move |change: &FerroPropertyChangedEventArgs<'_>| {
            if let Some(this) = weak.upgrade() {
                this.knobs_property_changed(change);
            }
        });
    }

    fn knobs_property_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == GlPageKnobs::yaw_property().as_property()
            || change.property() == GlPageKnobs::roll_property().as_property()
            || change.property() == GlPageKnobs::pitch_property().as_property()
            || change.property() == GlPageKnobs::disco_property().as_property()
        {
            self.request_next_frame_rendering();
        }
    }
}
