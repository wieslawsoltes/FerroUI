//! Port of `Pages/ScreenPage.cs`.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{
    BoxShadows, Brushes, DrawingContext, FlowDirection, FormattedText, IBrush, IPen, Pen, SolidColorBrush, Typeface,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, PixelSize, Point,
    Rect, Ref, StyledElementImpl, Thickness, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Button, ContentPage, Control, ControlImpl, Dock, DockPanel, PageImpl, TopLevel, Window};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct ScreenPage {
    base: ContentPage,
    presenter: Ref<ScreensPagePresenter>,
}

ferro_class!(ScreenPage: ContentPage);
ferro_impl_classes!(
    ScreenPage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);
ferro_class_info!(ScreenPage { new: ScreenPage::new });

impl VisualImpl for ScreenPage {
    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }

    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        let top_level = TopLevel::get_top_level(Some(this));
        if let Some(w) = top_level.as_ref().and_then(|top_level| top_level.cast::<Window>()) {
            let presenter = this.presenter.clone();
            w.position_changed(move |_| presenter.invalidate_visual());
        }

        if let Some(screens) = top_level.and_then(|top_level| top_level.screens()) {
            let presenter = this.presenter.clone();
            screens.changed(move || {
                presenter.invalidate_visual();
            });
        }
    }
}

impl ScreenPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), presenter: ScreensPagePresenter::new() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());

        let button = Button::new();
        button.set_content(Some(Rc::new(String::from("Request ScreenDetails")) as BoxedValue));
        button.set_vertical_alignment(VerticalAlignment::Top);
        button.set_value(DockPanel::dock_property(), Dock::Top);
        {
            let weak = this.downgrade();
            let weak_button = button.downgrade();
            button.click(move |_, _| {
                let (Some(this), Some(button)) = (weak.upgrade(), weak_button.upgrade()) else { return };
                mini_mvvm::start_async(async move {
                    let screens = TopLevel::get_top_level(Some(&this)).expect("the top level of the page").screens();
                    let success = match screens {
                        Some(screens) => screens.request_screen_details().await,
                        None => false,
                    };
                    let text = format!("Request ScreenDetails: {}", if success { "Granted" } else { "Denied" });
                    button.set_content(Some(Rc::new(text) as BoxedValue));
                });
            });
        }

        let panel = DockPanel::new();
        panel.children().add(&button);
        panel.children().add(&this.presenter);
        panel.set_vertical_spacing(10.0);
        panel.set_margin(Thickness::uniform(10.0));
        this.set_content(Some(Control::boxed(&panel)));
        this
    }
}

/// `ScreenPage.ScreensPagePresenter`: draws the screens and their details.
#[repr(C)]
struct ScreensPagePresenter {
    base: Control,
    primary_brush: Rc<dyn IBrush>,
    default_brush: Rc<dyn IBrush>,
    active_pen: Rc<dyn IPen>,
    default_pen: Rc<dyn IPen>,
    left_most: Cell<f64>,
    top_most: Cell<f64>,
}

ferro_class!(ScreensPagePresenter: Control);
ferro_impl_classes!(
    ScreensPagePresenter: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

/// A tenth of a pixel coordinate, computed in single precision as the
/// managed original does.
fn tenth(value: i32) -> f64 {
    f64::from(value as f32 / 10.0)
}

/// The text of a flag as the managed original writes it.
fn flag(value: bool) -> &'static str {
    if value {
        "True"
    } else {
        "False"
    }
}

impl VisualImpl for ScreensPagePresenter {
    fn render(this: &Self, context: &mut DrawingContext) {
        Self::parent_render(this, context);

        let top_level = TopLevel::get_top_level(Some(this)).expect("the top level of the presenter");
        let Some(screens) = top_level.screens() else {
            let formatted_text = Self::create_formatted_text("Current platform doesn't support Screens API.", 12.0);
            context.draw_text(&formatted_text, Point::new(15.0, 15.0));
            return;
        };

        let active_screen = screens.screen_from_top_level(&top_level);
        let mut max_bottom: f64 = 0.0;
        let all = screens.all();

        for i in 0..screens.screen_count() {
            let screen = &all[i as usize];
            let bounds = screen.bounds();
            let working_area = screen.working_area();

            if tenth(bounds.x) < this.left_most.get() {
                this.left_most.set(tenth(bounds.x));
                this.post_invalidate_visual();
                return;
            }
            if tenth(bounds.y) < this.top_most.get() {
                this.top_most.set(tenth(bounds.y));
                this.post_invalidate_visual();
                return;
            }
            let primary = screen.is_primary();
            let active = active_screen.as_ref().is_some_and(|active_screen| **screen == **active_screen);

            let left_most = this.left_most.get().abs();
            let top_most = this.top_most.get().abs();
            let bounds_rect = Rect::new(
                tenth(bounds.x) + left_most,
                tenth(bounds.y) + top_most,
                tenth(bounds.width),
                tenth(bounds.height),
            );
            let working_area_rect = Rect::new(
                tenth(working_area.x) + left_most,
                tenth(working_area.y) + top_most,
                tenth(working_area.width),
                tenth(working_area.height),
            );

            let brush = if primary { &this.primary_brush } else { &this.default_brush };
            let pen = if active { &this.active_pen } else { &this.default_pen };
            context.draw_rectangle(Some(brush), Some(pen), bounds_rect, 0.0, 0.0, &BoxShadows::default());
            context.draw_rectangle(Some(brush), Some(pen), working_area_rect, 0.0, 0.0, &BoxShadows::default());

            let identifier = Self::create_screen_identifier(&(i + 1).to_string(), primary);
            let center = bounds_rect.center() - Point::new(identifier.width() / 2.0, identifier.height() / 2.0);

            context.draw_text(&identifier, center);
            max_bottom = max_bottom.max(bounds_rect.bottom());
        }

        let mut current_height = max_bottom;

        for i in 0..screens.screen_count() {
            let screen = &all[i as usize];

            let formatted_text = Self::create_formatted_text(&format!("Screen {}", i + 1), 18.0);
            context.draw_text(&formatted_text, Point::new(0.0, current_height));
            current_height += 25.0;

            let lines = [
                format!("DisplayName: {}", screen.display_name().unwrap_or_default()),
                format!(
                    "Handle: {}",
                    screen
                        .try_get_platform_handle()
                        .and_then(|handle| handle.handle_descriptor().map(str::to_string))
                        .unwrap_or_default()
                ),
                format!("Bounds: {}:{}", screen.bounds().width, screen.bounds().height),
                format!("WorkArea: {}:{}", screen.working_area().width, screen.working_area().height),
                format!("Scaling: {}%", screen.scaling() * 100.0),
                format!("IsPrimary: {}", flag(screen.is_primary())),
                format!("CurrentOrientation: {:?}", screen.current_orientation()),
            ];
            for line in &lines {
                let formatted_text = Self::create_formatted_text(line, 12.0);
                context.draw_text(&formatted_text, Point::new(15.0, current_height));
                current_height += 20.0;
            }

            let current = active_screen.as_ref().is_some_and(|active_screen| **screen == **active_screen);
            let formatted_text = Self::create_formatted_text(&format!("Current: {}", flag(current)), 12.0);
            context.draw_text(&formatted_text, Point::new(15.0, current_height));
            current_height += 30.0;
        }

        if let Some(w) = top_level.cast::<Window>() {
            let w_pos = w.position();
            let w_size = PixelSize::from_size(w.frame_size().unwrap_or(w.client_size()), w.desktop_scaling());
            context.draw_rectangle_outline(
                &this.active_pen,
                Rect::new(
                    tenth(w_pos.x) + this.left_most.get().abs(),
                    tenth(w_pos.y) + this.top_most.get().abs(),
                    f64::from(w_size.width) / 10.0,
                    f64::from(w_size.height) / 10.0,
                ),
                0.0,
            );
        }
    }
}

impl ScreensPagePresenter {
    fn construct() -> Self {
        let primary_brush: Rc<dyn IBrush> =
            SolidColorBrush::parse("#FF0078D7").expect("a colour literal").into();
        let black: Rc<dyn IBrush> = Brushes::black();
        let dark_gray: Rc<dyn IBrush> = Brushes::dark_gray();
        Self {
            base: Control::construct(),
            primary_brush,
            default_brush: Brushes::light_gray(),
            active_pen: Pen::with_brush(Some(black), 1.0).into(),
            default_pen: Pen::with_brush(Some(dark_gray), 1.0).into(),
            left_most: Cell::new(0.0),
            top_most: Cell::new(0.0),
        }
    }

    fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn post_invalidate_visual(&self) {
        let this = self.to_ref();
        Dispatcher::ui_thread().post_local(move || this.invalidate_visual(), DispatcherPriority::BACKGROUND);
    }

    fn create_formatted_text(text_to_format: &str, size: f64) -> FormattedText {
        let green: Rc<dyn IBrush> = Brushes::green();
        FormattedText::new(
            text_to_format,
            CultureInfo::current_culture(),
            FlowDirection::LeftToRight,
            Typeface::default(),
            size,
            Some(green),
        )
    }

    fn create_screen_identifier(text_to_format: &str, primary: bool) -> FormattedText {
        let foreground: Rc<dyn IBrush> = if primary { Brushes::white() } else { Brushes::black() };
        FormattedText::new(
            text_to_format,
            CultureInfo::current_culture(),
            FlowDirection::LeftToRight,
            Typeface::default(),
            20.0,
            Some(foreground),
        )
    }
}
