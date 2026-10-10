//! Port of `Pages/HitTestingPage.cs`.

use ferroui_base::VisualImplExt;
use ferroui_base::animation::easings::{IEasing, SineEaseInOut};
use ferroui_base::animation::PlaybackDirection;
use ferroui_base::input::{InputElement, InputElementImpl, PointerEventArgs, PointerPressedEventArgs};
use ferroui_base::interactivity::{Interactive, InteractiveImpl};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, EllipseGeometry, IBrush, SolidColorBrush};
use ferroui_base::numerics::Vector3;
use ferroui_base::rendering::composition::animations::AnimationIterationBehavior;
use ferroui_base::rendering::composition::{Compositor, ElementComposition, ICompositionObjectAnimations};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::Decimal;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Point, Rect, Ref,
    RelativePoint, RelativeUnit, Size, StyledElementImpl, Thickness, Vector3D, VisualImpl,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::shapes::Ellipse;
use ferroui_controls::{
    Border, Canvas, CheckBox, ContentControlImpl, Control, ControlImpl, Grid, GridLength, NumericUpDown,
    RowDefinition, StackPanel, TextBlock, UserControl,
};
use std::cell::{Cell as ValueCell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const REGION_SIZE: i32 = 80;
const GROUP_COLUMNS: i32 = 8;
const GROUP_ROWS: i32 = 5;
const CELLS_PER_GROUP_SIDE: i32 = 10;
const CELL_STRIDE: i32 = 10;
const CELL_SIZE: i32 = 8;
const ANIMATION_TRAVEL: i32 = 64;

/// `_cells.Length`.
const CELL_COUNT: i32 = GROUP_COLUMNS * GROUP_ROWS * CELLS_PER_GROUP_SIDE * CELLS_PER_GROUP_SIDE;

#[repr(C)]
pub struct HitTestingPage {
    base: UserControl,
    scene: Ref<Canvas>,
    stats: Ref<TextBlock>,
    cells: RefCell<Vec<Ref<Cell>>>,
    /// The time the stopwatch of the page was (re)started at, in the
    /// milliseconds of the clock of the dispatcher.
    stopwatch: ValueCell<i64>,
    compositor: RefCell<Option<Rc<Compositor>>>,
    hit_tests_per_frame: ValueCell<i32>,
    update_count: ValueCell<i32>,
    hit_test_count: ValueCell<i32>,
    hit_count: ValueCell<i32>,
    last_second_update_count: ValueCell<i32>,
    last_second_hit_test_count: ValueCell<i32>,
    last_second_hit_count: ValueCell<i32>,
    /// The elapsed time of the stopwatch at the last update of the statistics, in milliseconds.
    last_second_time: ValueCell<i64>,
    last_second_updates_per_second: ValueCell<f64>,
    last_second_hit_tests_per_second: ValueCell<f64>,
    last_second_hits_per_second: ValueCell<f64>,
    click_count: ValueCell<i32>,
    is_attached: ValueCell<bool>,
    update_queued: ValueCell<bool>,
    animations_started: ValueCell<bool>,
    last_hit_cell: RefCell<Option<Ref<Cell>>>,
    last_clicked_cell: RefCell<Option<Ref<Cell>>>,
    region_selection: ValueCell<bool>,
    region: Ref<Ellipse>,
    region_hits: RefCell<Vec<Ref<Cell>>>,
}

ferro_class!(HitTestingPage: UserControl);
ferro_impl_classes!(
    HitTestingPage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(HitTestingPage { new: HitTestingPage::new });

impl VisualImpl for HitTestingPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.reset_state();
        *this.compositor.borrow_mut() =
            ElementComposition::get_element_visual(this).map(|visual| visual.compositor().clone());
        this.is_attached.set(true);
        this.request_next_update();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.is_attached.set(false);
        this.update_queued.set(false);
        *this.compositor.borrow_mut() = None;
        this.reset_state();
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl HitTestingPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            scene: Canvas::new(),
            stats: TextBlock::new(),
            cells: RefCell::new(Vec::with_capacity(CELL_COUNT as usize)),
            stopwatch: ValueCell::new(Dispatcher::ui_thread().now()),
            compositor: RefCell::new(None),
            hit_tests_per_frame: ValueCell::new(256),
            update_count: ValueCell::new(0),
            hit_test_count: ValueCell::new(0),
            hit_count: ValueCell::new(0),
            last_second_update_count: ValueCell::new(0),
            last_second_hit_test_count: ValueCell::new(0),
            last_second_hit_count: ValueCell::new(0),
            last_second_time: ValueCell::new(0),
            last_second_updates_per_second: ValueCell::new(0.0),
            last_second_hit_tests_per_second: ValueCell::new(0.0),
            last_second_hits_per_second: ValueCell::new(0.0),
            click_count: ValueCell::new(0),
            is_attached: ValueCell::new(false),
            update_queued: ValueCell::new(false),
            animations_started: ValueCell::new(false),
            last_hit_cell: RefCell::new(None),
            last_clicked_cell: RefCell::new(None),
            region_selection: ValueCell::new(false),
            region: Ellipse::new(),
            region_hits: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());

        this.scene.set_width(f64::from(GROUP_COLUMNS * CELLS_PER_GROUP_SIDE * CELL_STRIDE));
        this.scene.set_height(f64::from(GROUP_ROWS * CELLS_PER_GROUP_SIDE * CELL_STRIDE));
        this.scene.set_background(Some(Brushes::transparent() as Rc<dyn IBrush>));

        // The handlers belong to children of the page: they hold the page weakly.
        let weak = this.downgrade();
        this.scene.add_handler(InputElement::pointer_moved_event(), move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.scene_pointer_moved(sender, e);
            }
        });
        let weak = this.downgrade();
        this.scene.add_handler(InputElement::pointer_pressed_event(), move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.scene_pointer_pressed(sender, e);
            }
        });

        this.stats.set_horizontal_alignment(HorizontalAlignment::Center);
        this.stats.set_vertical_alignment(VerticalAlignment::Top);
        this.stats.set_margin(Thickness::uniform(12.0));
        this.stats.set_padding(Thickness::symmetric(8.0, 4.0));
        this.stats.set_background(Some(SolidColorBrush::with_color(Color::from_argb(220, 255, 255, 255)).into()));
        this.stats.set_foreground(Some(Brushes::black() as Rc<dyn IBrush>));
        this.stats.set_is_hit_test_visible(false);

        let region_selection = CheckBox::new();
        region_selection.set_content(Some(Rc::new(String::from("Region Selection")) as BoxedValue));

        this.region.set_width(f64::from(REGION_SIZE * 2));
        this.region.set_height(f64::from(REGION_SIZE));
        this.region.set_fill(Some(Brushes::transparent() as Rc<dyn IBrush>));
        this.region.set_stroke_thickness(2.0);
        this.region.set_stroke(Some(Brushes::green() as Rc<dyn IBrush>));
        this.region.set_is_visible(false);
        this.region.set_is_hit_test_visible(false);

        let weak = this.downgrade();
        region_selection.is_checked_changed(move |_s, _e| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            this.region_selection.set(!this.region_selection.get());

            this.region.set_is_visible(this.region_selection.get());

            this.clear_region_highlights();
        });

        let number_of_hit_tests = NumericUpDown::new();
        number_of_hit_tests.set_minimum(Decimal::from(0));
        number_of_hit_tests.set_numeric_value(Some(Decimal::from(this.hit_tests_per_frame.get())));
        number_of_hit_tests.set_width(200.0);
        number_of_hit_tests.set_horizontal_alignment(HorizontalAlignment::Left);
        number_of_hit_tests.set_vertical_alignment(VerticalAlignment::Center);

        let weak = this.downgrade();
        // The handler of the control holds the control weakly.
        let weak_number_of_hit_tests = number_of_hit_tests.downgrade();
        number_of_hit_tests.value_changed(move |_s, _e| {
            let (Some(this), Some(number_of_hit_tests)) = (weak.upgrade(), weak_number_of_hit_tests.upgrade()) else {
                return;
            };
            if let Some(value) = number_of_hit_tests.value() {
                // `(int)value`: the fraction is cut off.
                this.hit_tests_per_frame.set(value.to_f64().trunc() as i32);
            }
        });

        let param = StackPanel::new();
        param.set_orientation(Orientation::Horizontal);
        param.set_horizontal_alignment(HorizontalAlignment::Left);
        param.set_vertical_alignment(VerticalAlignment::Bottom);
        param.set_margin(Thickness::uniform(12.0));
        param.set_spacing(8.0);
        let label = TextBlock::new();
        label.set_text(Some("Hit tests per update:"));
        param.children().add(label);
        param.children().add(number_of_hit_tests);
        param.children().add(region_selection);

        let root = Grid::new();
        root.set_clip_to_bounds(true);
        root.row_definitions().add(RowDefinition::with_height(GridLength::AUTO));
        root.row_definitions().add(RowDefinition::with_height(GridLength::AUTO));
        root.row_definitions().add(RowDefinition::with_height(GridLength::STAR));

        Grid::set_row(&param, 0);
        root.children().add(param);
        Grid::set_row(&this.stats, 1);
        root.children().add(this.stats.clone());
        Grid::set_row(&this.scene, 2);
        root.children().add(this.scene.clone());

        this.set_content(Some(Control::boxed(root)));
        this.build_scene();
        this.reset_state();

        this
    }

    fn clear_region_highlights(&self) {
        let last_clicked_cell = self.last_clicked_cell.borrow_mut().take();
        if let Some(last_clicked_cell) = last_clicked_cell {
            last_clicked_cell.clear_highlight();
        }
        let region_hits = std::mem::take(&mut *self.region_hits.borrow_mut());
        for cell in region_hits {
            cell.clear_highlight();
        }
    }

    fn scene_pointer_pressed(&self, _sender: &Interactive, _e: &PointerPressedEventArgs) {
        if self.region_selection.get() {
            let bounds = self.region.bounds();

            self.clear_region_highlights();

            let geometry = EllipseGeometry::with_rect(bounds);

            let cells = self.scene.get_visuals_at_geometry(&geometry);

            for cell in cells {
                if let Some(c) = cell.visual_hit.cast::<Cell>() {
                    c.set_is_latest_click(true);
                    c.update_highlight();
                    self.region_hits.borrow_mut().push(c);
                }
            }
        }
    }

    fn scene_pointer_moved(&self, _sender: &Interactive, e: &PointerEventArgs) {
        if self.region_selection.get() {
            let point = e.get_position(Some(&self.scene));

            let rect = Rect::from_position_size(point, Size::default()).inflate_thickness(Thickness::symmetric(
                self.region.bounds().width / 2.0,
                self.region.bounds().height / 2.0,
            ));

            Canvas::set_left(&self.region, rect.x);
            Canvas::set_top(&self.region, rect.y);
        }
    }

    fn build_scene(&self) {
        let mut index = 0;
        let group_size = CELLS_PER_GROUP_SIDE * CELL_STRIDE;

        for group_y in 0..GROUP_ROWS {
            for group_x in 0..GROUP_COLUMNS {
                let group = Canvas::new();
                group.set_width(f64::from(group_size));
                group.set_height(f64::from(group_size));
                group.set_background(Some(Brushes::transparent() as Rc<dyn IBrush>));
                Canvas::set_left(&group, f64::from(group_x * group_size));
                Canvas::set_top(&group, f64::from(group_y * group_size));
                self.scene.children().add(group.clone());

                for y in 0..CELLS_PER_GROUP_SIDE {
                    for x in 0..CELLS_PER_GROUP_SIDE {
                        let cell = Cell::new(index);
                        cell.set_width(f64::from(CELL_SIZE));
                        cell.set_height(f64::from(CELL_SIZE));
                        cell.set_background(Some(Self::create_brush(index)));
                        cell.set_render_transform_origin(RelativePoint::new(0.5, 0.5, RelativeUnit::Relative));
                        // The handler belongs to a descendant of the page: it holds the page weakly.
                        let weak = self.to_ref().downgrade();
                        cell.add_handler(InputElement::pointer_pressed_event(), move |sender, e| {
                            if let Some(this) = weak.upgrade() {
                                this.on_cell_pointer_pressed(sender, e);
                            }
                        });

                        Canvas::set_left(&cell, f64::from(x * CELL_STRIDE));
                        Canvas::set_top(&cell, f64::from(y * CELL_STRIDE));
                        group.children().add(cell.clone());
                        self.cells.borrow_mut().push(cell);
                        index += 1;
                    }
                }
            }
        }

        self.scene.children().add(self.region.clone());
    }

    fn on_composition_update(&self) {
        self.update_queued.set(false);

        if !self.is_attached.get() {
            return;
        }

        if !self.animations_started.get() {
            self.start_animations();
        }

        self.run_hit_tests();

        self.update_count.set(self.update_count.get() + 1);
        if self.stopwatch_elapsed() - self.last_second_time.get() >= 1000 {
            self.update_stats();
        }

        self.request_next_update();
    }

    fn request_next_update(&self) {
        let compositor = self.compositor.borrow().clone();
        let Some(compositor) = compositor else {
            return;
        };
        if self.update_queued.get() {
            return;
        }

        self.update_queued.set(true);
        // The compositor keeps the callback until the next commit: it holds the page weakly.
        let weak = self.to_ref().downgrade();
        compositor.request_composition_update(move || {
            if let Some(this) = weak.upgrade() {
                this.on_composition_update();
            }
        });
    }

    fn start_animations(&self) {
        let mut started = 0;
        let easing: Rc<dyn IEasing> = Rc::new(SineEaseInOut::new());

        let cells = self.cells.borrow().clone();
        for (i, cell) in cells.iter().enumerate() {
            let i = i as i32;
            if i % 5 != 0 {
                continue;
            }

            let Some(visual) = ElementComposition::get_element_visual(cell) else {
                continue;
            };

            let translation = visual.compositor().create_vector3_key_frame_animation();
            translation.set_target(Some("Translation".to_owned()));
            translation.set_duration(Duration::from_millis((900 + (i % 700)) as u64));
            translation.set_direction(PlaybackDirection::Alternate);
            translation.set_iteration_behavior(AnimationIterationBehavior::Forever);
            translation.insert_key_frame_with_easing(0.0, Vector3::new(0.0, 0.0, 0.0), easing.clone());
            translation.insert_key_frame_with_easing(1.0, Self::get_animation_offset(i), easing.clone());
            visual.start_animation("Translation", &*translation);

            started += 1;
        }

        self.animations_started.set(started > 0);
    }

    fn stop_animations(&self) {
        let cells = self.cells.borrow().clone();
        for cell in &cells {
            let Some(visual) = ElementComposition::get_element_visual(cell) else {
                continue;
            };

            visual.stop_animation("Translation");
            visual.set_translation(Vector3D::default());
        }

        self.animations_started.set(false);
    }

    fn run_hit_tests(&self) {
        let width = f64::max(1.0, self.scene.bounds().width);
        let height = f64::max(1.0, self.scene.bounds().height);
        let base_index = self.update_count.get().wrapping_mul(37);

        for i in 0..self.hit_tests_per_frame.get() {
            self.hit_test_count.set(self.hit_test_count.get() + 1);
            let sample = base_index.wrapping_add(i.wrapping_mul(97));
            let point =
                Point::new(f64::from(sample.wrapping_mul(17)) % width, f64::from(sample.wrapping_mul(29)) % height);
            let hit = self.scene.get_visual_at(point);

            if let Some(cell) = hit.and_then(|hit| hit.cast::<Cell>()) {
                self.set_last_hit_cell(&cell);
                self.hit_count.set(self.hit_count.get() + 1);
            }
        }
    }

    fn get_animation_offset(index: i32) -> Vector3 {
        let x = match index % 4 {
            0 => -ANIMATION_TRAVEL,
            1 => ANIMATION_TRAVEL,
            2 => -ANIMATION_TRAVEL / 2,
            _ => ANIMATION_TRAVEL / 2,
        };
        let y = match index / 4 % 4 {
            0 => -ANIMATION_TRAVEL,
            1 => ANIMATION_TRAVEL,
            2 => ANIMATION_TRAVEL / 2,
            _ => -ANIMATION_TRAVEL / 2,
        };

        Vector3::new(x as f32, y as f32, 0.0)
    }

    fn reset_state(&self) {
        self.stop_animations();

        let last_clicked_cell = self.last_clicked_cell.borrow().clone();
        if let Some(last_clicked_cell) = last_clicked_cell {
            last_clicked_cell.clear_highlight();
        }
        *self.last_hit_cell.borrow_mut() = None;
        *self.last_clicked_cell.borrow_mut() = None;

        self.update_count.set(0);
        self.hit_test_count.set(0);
        self.hit_count.set(0);
        self.last_second_update_count.set(0);
        self.last_second_hit_test_count.set(0);
        self.last_second_hit_count.set(0);
        self.last_second_time.set(0);
        self.last_second_updates_per_second.set(0.0);
        self.last_second_hit_tests_per_second.set(0.0);
        self.last_second_hits_per_second.set(0.0);
        self.click_count.set(0);
        self.stopwatch.set(Dispatcher::ui_thread().now());
        self.update_stats();
    }

    /// `_stopwatch.Elapsed`, in milliseconds.
    fn stopwatch_elapsed(&self) -> i64 {
        Dispatcher::ui_thread().now() - self.stopwatch.get()
    }

    fn update_stats(&self) {
        let elapsed = self.stopwatch_elapsed();
        let seconds = f64::max(0.001, (elapsed - self.last_second_time.get()) as f64 / 1000.0);
        self.last_second_updates_per_second
            .set(f64::from(self.update_count.get() - self.last_second_update_count.get()) / seconds);
        self.last_second_hit_tests_per_second
            .set(f64::from(self.hit_test_count.get() - self.last_second_hit_test_count.get()) / seconds);
        self.last_second_hits_per_second
            .set(f64::from(self.hit_count.get() - self.last_second_hit_count.get()) / seconds);
        self.last_second_update_count.set(self.update_count.get());
        self.last_second_hit_test_count.set(self.hit_test_count.get());
        self.last_second_hit_count.set(self.hit_count.get());
        self.last_second_time.set(elapsed);

        self.stats.set_text(Some(&format!(
            "Visuals: {} ({} animated), Hit tests/frame: {}, Composition updates/s: {:.1}, Hit tests/s: {:.0}, \
             Hits/s: {:.0}, Misses/s: {:.0}, Clicks: {}",
            CELL_COUNT,
            CELL_COUNT / 5,
            self.hit_tests_per_frame.get(),
            self.last_second_updates_per_second.get(),
            self.last_second_hit_tests_per_second.get(),
            self.last_second_hits_per_second.get(),
            self.last_second_hit_tests_per_second.get() - self.last_second_hits_per_second.get(),
            self.click_count.get(),
        )));
    }

    fn on_cell_pointer_pressed(&self, sender: &Interactive, e: &PointerPressedEventArgs) {
        let Some(cell) = sender.to_ref().cast::<Cell>() else {
            return;
        };
        if self.region_selection.get() {
            return;
        }

        self.set_last_clicked_cell(&cell);
        self.click_count.set(self.click_count.get() + 1);
        e.set_handled(true);
    }

    fn set_last_hit_cell(&self, cell: &Ref<Cell>) {
        if self.last_hit_cell.borrow().as_ref() == Some(cell) {
            return;
        }

        *self.last_hit_cell.borrow_mut() = Some(cell.clone());
    }

    fn set_last_clicked_cell(&self, cell: &Ref<Cell>) {
        let last_clicked_cell = self.last_clicked_cell.borrow().clone();
        if let Some(last_clicked_cell) = last_clicked_cell {
            if &last_clicked_cell != cell {
                last_clicked_cell.set_is_latest_click(false);
                last_clicked_cell.update_highlight();
            }
        }

        *self.last_clicked_cell.borrow_mut() = Some(cell.clone());
        cell.set_is_latest_click(true);
        cell.update_highlight();
    }

    fn create_brush(index: i32) -> Rc<dyn IBrush> {
        let r = (80 + (index * 47 % 160)) as u8;
        let g = (80 + (index * 91 % 160)) as u8;
        let b = (80 + (index * 137 % 160)) as u8;
        SolidColorBrush::with_color(Color::from_rgb(r, g, b)).into()
    }
}

#[repr(C)]
struct Cell {
    base: Border,
    index: i32,
    is_latest_click: ValueCell<bool>,
}

ferro_class!(Cell: Border);
ferro_impl_classes!(
    Cell: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(Cell {});

impl Cell {
    fn new(index: i32) -> Ref<Self> {
        let this = instantiate(Self { base: Border::construct(), index, is_latest_click: ValueCell::new(false) });
        this.set_border_thickness(Thickness::uniform(1.0));
        this
    }

    #[allow(dead_code)]
    fn index(&self) -> i32 {
        self.index
    }

    #[allow(dead_code)]
    fn is_latest_click(&self) -> bool {
        self.is_latest_click.get()
    }

    fn set_is_latest_click(&self, value: bool) {
        self.is_latest_click.set(value);
    }

    fn clear_highlight(&self) {
        self.set_is_latest_click(false);
        self.update_highlight();
    }

    fn update_highlight(&self) {
        self.set_border_brush(Some(if self.is_latest_click.get() {
            Brushes::white() as Rc<dyn IBrush>
        } else {
            Brushes::transparent() as Rc<dyn IBrush>
        }));
        self.set_z_index(if self.is_latest_click.get() { 1 } else { 0 });
    }
}
