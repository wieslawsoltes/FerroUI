//! The test types declared by the upstream test file
//! `Xaml/FerroIntrinsicsTests.cs`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ferroui_base::animation::TimeSpan;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Color, IBrush, Points, TextDecorationCollection, TextTrimming};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::utilities::Uri;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, CornerRadius, FerroObjectImpl, Matrix, Point, Ref,
    RelativePoint, Size, StyledElementImpl, Thickness, Vector, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl, GridLength, WindowTransparencyLevel};

use crate::support::TypeModule;

/// A control with one plain property per type markup parses itself (the
/// intrinsics of the compiler).
#[repr(C)]
pub struct TestIntrinsicsControl {
    base: Control,
    time_span_property: Cell<TimeSpan>,
    thickness_property: Cell<Thickness>,
    point_property: Cell<Point>,
    vector_property: Cell<Vector>,
    size_property: Cell<Size>,
    matrix_property: Cell<Matrix>,
    corner_radius_property: Cell<CornerRadius>,
    color_property: Cell<Color>,
    relative_point_property: Cell<RelativePoint>,
    grid_length_property: Cell<GridLength>,
    i_brush_property: RefCell<Option<Rc<dyn IBrush>>>,
    text_trimming_property: RefCell<Option<Rc<dyn TextTrimming>>>,
    text_decoration_collection_property: RefCell<Option<TextDecorationCollection>>,
    window_transparency_level_property: Cell<WindowTransparencyLevel>,
    uri_property: RefCell<Option<Uri>>,
    theme_variant_property: RefCell<Option<ThemeVariant>>,
    points_property: RefCell<Option<Points>>,
}

ferro_class!(TestIntrinsicsControl: Control);
ferro_impl_classes!(
    TestIntrinsicsControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(TestIntrinsicsControl {
    new: TestIntrinsicsControl::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
        properties: [
            TimeSpanProperty: TimeSpan {
                get: |this: &Ref<TestIntrinsicsControl>| this.time_span_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: TimeSpan| this.set_time_span_property(value)
            },
            ThicknessProperty: Thickness {
                get: |this: &Ref<TestIntrinsicsControl>| this.thickness_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: Thickness| this.set_thickness_property(value)
            },
            PointProperty: Point {
                get: |this: &Ref<TestIntrinsicsControl>| this.point_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: Point| this.set_point_property(value)
            },
            VectorProperty: Vector {
                get: |this: &Ref<TestIntrinsicsControl>| this.vector_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: Vector| this.set_vector_property(value)
            },
            SizeProperty: Size {
                get: |this: &Ref<TestIntrinsicsControl>| this.size_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: Size| this.set_size_property(value)
            },
            MatrixProperty: Matrix {
                get: |this: &Ref<TestIntrinsicsControl>| this.matrix_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: Matrix| this.set_matrix_property(value)
            },
            CornerRadiusProperty: CornerRadius {
                get: |this: &Ref<TestIntrinsicsControl>| this.corner_radius_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: CornerRadius| this.set_corner_radius_property(value)
            },
            ColorProperty: Color {
                get: |this: &Ref<TestIntrinsicsControl>| this.color_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: Color| this.set_color_property(value)
            },
            RelativePointProperty: RelativePoint {
                get: |this: &Ref<TestIntrinsicsControl>| this.relative_point_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: RelativePoint| this.set_relative_point_property(value)
            },
            GridLengthProperty: GridLength {
                get: |this: &Ref<TestIntrinsicsControl>| this.grid_length_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: GridLength| this.set_grid_length_property(value)
            },
            IBrushProperty: Option<Rc<dyn IBrush>> {
                get: |this: &Ref<TestIntrinsicsControl>| this.i_brush_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: Option<Rc<dyn IBrush>>| this.set_i_brush_property(value)
            },
            TextTrimmingProperty: Option<Rc<dyn TextTrimming>> {
                get: |this: &Ref<TestIntrinsicsControl>| this.text_trimming_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: Option<Rc<dyn TextTrimming>>| {
                    this.set_text_trimming_property(value)
                }
            },
            TextDecorationCollectionProperty: Option<TextDecorationCollection> {
                get: |this: &Ref<TestIntrinsicsControl>| this.text_decoration_collection_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: Option<TextDecorationCollection>| {
                    this.set_text_decoration_collection_property(value)
                }
            },
            WindowTransparencyLevelProperty: WindowTransparencyLevel {
                get: |this: &Ref<TestIntrinsicsControl>| this.window_transparency_level_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: WindowTransparencyLevel| {
                    this.set_window_transparency_level_property(value)
                }
            },
            UriProperty: Option<Uri> {
                get: |this: &Ref<TestIntrinsicsControl>| this.uri_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: Option<Uri>| this.set_uri_property(value)
            },
            ThemeVariantProperty: Option<ThemeVariant> {
                get: |this: &Ref<TestIntrinsicsControl>| this.theme_variant_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: Option<ThemeVariant>| {
                    this.set_theme_variant_property(value)
                }
            },
            PointsProperty: Option<Points> {
                get: |this: &Ref<TestIntrinsicsControl>| this.points_property(),
                set: |this: &Ref<TestIntrinsicsControl>, value: Option<Points>| this.set_points_property(value)
            },
        ],
    },
});

impl TestIntrinsicsControl {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            time_span_property: Cell::new(TimeSpan::default()),
            thickness_property: Cell::new(Thickness::default()),
            point_property: Cell::new(Point::default()),
            vector_property: Cell::new(Vector::default()),
            size_property: Cell::new(Size::default()),
            matrix_property: Cell::new(Matrix::default()),
            corner_radius_property: Cell::new(CornerRadius::default()),
            color_property: Cell::new(Color::default()),
            relative_point_property: Cell::new(RelativePoint::default()),
            grid_length_property: Cell::new(GridLength::default()),
            i_brush_property: RefCell::new(None),
            text_trimming_property: RefCell::new(None),
            text_decoration_collection_property: RefCell::new(None),
            window_transparency_level_property: Cell::new(WindowTransparencyLevel::none()),
            uri_property: RefCell::new(None),
            theme_variant_property: RefCell::new(None),
            points_property: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn time_span_property(&self) -> TimeSpan {
        self.time_span_property.get()
    }

    pub fn set_time_span_property(&self, value: TimeSpan) {
        self.time_span_property.set(value);
    }

    pub fn thickness_property(&self) -> Thickness {
        self.thickness_property.get()
    }

    pub fn set_thickness_property(&self, value: Thickness) {
        self.thickness_property.set(value);
    }

    pub fn point_property(&self) -> Point {
        self.point_property.get()
    }

    pub fn set_point_property(&self, value: Point) {
        self.point_property.set(value);
    }

    pub fn vector_property(&self) -> Vector {
        self.vector_property.get()
    }

    pub fn set_vector_property(&self, value: Vector) {
        self.vector_property.set(value);
    }

    pub fn size_property(&self) -> Size {
        self.size_property.get()
    }

    pub fn set_size_property(&self, value: Size) {
        self.size_property.set(value);
    }

    pub fn matrix_property(&self) -> Matrix {
        self.matrix_property.get()
    }

    pub fn set_matrix_property(&self, value: Matrix) {
        self.matrix_property.set(value);
    }

    pub fn corner_radius_property(&self) -> CornerRadius {
        self.corner_radius_property.get()
    }

    pub fn set_corner_radius_property(&self, value: CornerRadius) {
        self.corner_radius_property.set(value);
    }

    pub fn color_property(&self) -> Color {
        self.color_property.get()
    }

    pub fn set_color_property(&self, value: Color) {
        self.color_property.set(value);
    }

    pub fn relative_point_property(&self) -> RelativePoint {
        self.relative_point_property.get()
    }

    pub fn set_relative_point_property(&self, value: RelativePoint) {
        self.relative_point_property.set(value);
    }

    pub fn grid_length_property(&self) -> GridLength {
        self.grid_length_property.get()
    }

    pub fn set_grid_length_property(&self, value: GridLength) {
        self.grid_length_property.set(value);
    }

    pub fn i_brush_property(&self) -> Option<Rc<dyn IBrush>> {
        self.i_brush_property.borrow().clone()
    }

    pub fn set_i_brush_property(&self, value: Option<Rc<dyn IBrush>>) {
        *self.i_brush_property.borrow_mut() = value;
    }

    pub fn text_trimming_property(&self) -> Option<Rc<dyn TextTrimming>> {
        self.text_trimming_property.borrow().clone()
    }

    pub fn set_text_trimming_property(&self, value: Option<Rc<dyn TextTrimming>>) {
        *self.text_trimming_property.borrow_mut() = value;
    }

    pub fn text_decoration_collection_property(&self) -> Option<TextDecorationCollection> {
        self.text_decoration_collection_property.borrow().clone()
    }

    pub fn set_text_decoration_collection_property(&self, value: Option<TextDecorationCollection>) {
        *self.text_decoration_collection_property.borrow_mut() = value;
    }

    pub fn window_transparency_level_property(&self) -> WindowTransparencyLevel {
        self.window_transparency_level_property.get()
    }

    pub fn set_window_transparency_level_property(&self, value: WindowTransparencyLevel) {
        self.window_transparency_level_property.set(value);
    }

    pub fn uri_property(&self) -> Option<Uri> {
        self.uri_property.borrow().clone()
    }

    pub fn set_uri_property(&self, value: Option<Uri>) {
        *self.uri_property.borrow_mut() = value;
    }

    pub fn theme_variant_property(&self) -> Option<ThemeVariant> {
        self.theme_variant_property.borrow().clone()
    }

    pub fn set_theme_variant_property(&self, value: Option<ThemeVariant>) {
        *self.theme_variant_property.borrow_mut() = value;
    }

    pub fn points_property(&self) -> Option<Points> {
        self.points_property.borrow().clone()
    }

    pub fn set_points_property(&self, value: Option<Points>) {
        *self.points_property.borrow_mut() = value;
    }
}

pub(crate) const MODULE: TypeModule =
    TypeModule { types: &[TestIntrinsicsControl::TYPE], markup_types: &[], value_types: || {} };
