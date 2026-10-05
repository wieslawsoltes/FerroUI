//! Port of `DecoratedWindow.xaml.cs`: the class of the document
//! `DecoratedWindow.xaml`.

use crate::markup::xaml_class;
use ferroui_base::input::{Cursor, InputElement, InputElementImpl, StandardCursorType};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, ContentControlImpl, Control, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowEdge, WindowImpl,
    WindowState,
};

/// A window that draws its own title bar and resize borders.
#[repr(C)]
pub struct DecoratedWindow {
    base: Window,
}

ferro_class!(DecoratedWindow: Window);
ferro_impl_classes!(
    DecoratedWindow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(DecoratedWindow { new: DecoratedWindow::new });
xaml_class!(DecoratedWindow, "/DecoratedWindow.xaml");

impl DecoratedWindow {
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        {
            let window = this.downgrade();
            this.get_control::<Control>("TitleBar").add_handler(InputElement::pointer_pressed_event(), move |_, e| {
                if let Some(window) = window.upgrade() {
                    window.begin_move_drag(e);
                }
            });
        }
        this.setup_side("Left", StandardCursorType::LeftSide, WindowEdge::West);
        this.setup_side("Right", StandardCursorType::RightSide, WindowEdge::East);
        this.setup_side("Top", StandardCursorType::TopSide, WindowEdge::North);
        this.setup_side("Bottom", StandardCursorType::BottomSide, WindowEdge::South);
        this.setup_side("TopLeft", StandardCursorType::TopLeftCorner, WindowEdge::NorthWest);
        this.setup_side("TopRight", StandardCursorType::TopRightCorner, WindowEdge::NorthEast);
        this.setup_side("BottomLeft", StandardCursorType::BottomLeftCorner, WindowEdge::SouthWest);
        this.setup_side("BottomRight", StandardCursorType::BottomRightCorner, WindowEdge::SouthEast);
        {
            let window = this.downgrade();
            this.get_control::<Button>("MinimizeButton").click(move |_, _| {
                if let Some(window) = window.upgrade() {
                    window.set_window_state(WindowState::Minimized);
                }
            });
        }
        {
            let window = this.downgrade();
            this.get_control::<Button>("MaximizeButton").click(move |_, _| {
                if let Some(window) = window.upgrade() {
                    window.set_window_state(if window.window_state() == WindowState::Maximized {
                        WindowState::Normal
                    } else {
                        WindowState::Maximized
                    });
                }
            });
        }
        {
            let window = this.downgrade();
            this.get_control::<Button>("CloseButton").click(move |_, _| {
                if let Some(window) = window.upgrade() {
                    window.close();
                }
            });
        }
        this
    }

    /// `SetupSide(ctl, cursor, edge)`, for the named element `name`.
    fn setup_side(&self, name: &str, cursor: StandardCursorType, edge: WindowEdge) {
        let ctl = self.get_control::<Control>(name);
        ctl.set_cursor(Some(Cursor::new(cursor)));
        let window = self.to_ref().downgrade();
        ctl.add_handler(InputElement::pointer_pressed_event(), move |_, e| {
            if let Some(window) = window.upgrade() {
                if window.window_state() == WindowState::Normal {
                    window.begin_resize_drag(edge, e);
                }
            }
        });
    }
}
