use crate::internal::*;
use crate::*;
use ferroui_base::input::{InputElementImpl, MouseButton, RawInputModifiers};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::threading::CancellationToken;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Rect, Ref, Size,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::templates::FuncControlTemplate;
use ferroui_controls::{AppBuilder, Application, Border, Control, ControlImpl, PanelImpl, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use std::cell::{Cell, RefCell};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-proportional-headless",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
pub(crate) fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .expect("headless case completed");
}
pub(crate) fn layout(panel: &ProportionalStackPanel) {
    panel.invalidate_measure();
    panel.measure(Size::INFINITY);
    panel.arrange(Rect::from_size(panel.desired_size()));
}
pub(crate) fn scene(
    orientation: Orientation,
    length: f64,
    first: f64,
    second: f64,
) -> (
    Ref<ProportionalStackPanel>,
    Ref<Border>,
    Ref<ProportionalStackPanelSplitter>,
    Ref<Border>,
) {
    let panel = ProportionalStackPanel::new();
    panel.set_orientation(orientation);
    panel.set_width(if orientation == Orientation::Horizontal {
        length
    } else {
        100.0
    });
    panel.set_height(if orientation == Orientation::Vertical {
        length
    } else {
        100.0
    });
    let a = Border::new();
    let splitter = ProportionalStackPanelSplitter::new();
    let b = Border::new();
    ProportionalStackPanel::set_proportion(&a, first);
    ProportionalStackPanel::set_proportion(&b, second);
    panel.children().add(a.clone());
    panel.children().add(splitter.clone());
    panel.children().add(b.clone());
    (panel, a, splitter, b)
}
fn extent(control: &Control, orientation: Orientation) -> f64 {
    if orientation == Orientation::Horizontal {
        control.bounds().width
    } else {
        control.bounds().height
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "expected {b}, got {a}");
}
pub(crate) fn update(window: &Window) {
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();
}
fn host(control: Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(600.0);
    window.set_height(500.0);
    window.set_content(Some(Control::boxed(control)));
    window.show();
    update(&window);
    window
}
#[test]
fn proportional_panel_constructor_and_attached_defaults() {
    run(|| {
        let panel = ProportionalStackPanel::new();
        assert_eq!(panel.orientation(), Orientation::Vertical);
        assert!(ProportionalStackPanel::get_proportion(&panel).is_nan());
        assert!(!ProportionalStackPanel::get_is_collapsed(&panel));
        assert!(ProportionalStackPanel::get_collapsed_proportion(&panel).is_nan());
        let splitter = ProportionalStackPanelSplitter::new();
        near(splitter.thickness(), 4.0);
        assert!(splitter.is_resizing_enabled());
        assert!(!splitter.preview_resize());
        assert!(splitter.preview_background().is_none());
        near(splitter.preview_opacity(), 1.0);
        near(
            ProportionalStackPanelSplitter::get_minimum_proportion_size(&splitter),
            75.0,
        );
    });
}
#[test]
fn lays_out_children_horizontal() {
    run(|| {
        let (panel, a, splitter, b) = scene(Orientation::Horizontal, 300.0, f64::NAN, f64::NAN);
        layout(&panel);
        assert_eq!(panel.bounds().size(), Size::new(300.0, 100.0));
        assert_eq!(a.bounds(), Rect::new(0.0, 0.0, 148.0, 100.0));
        assert_eq!(splitter.bounds(), Rect::new(148.0, 0.0, 4.0, 100.0));
        assert_eq!(b.bounds(), Rect::new(152.0, 0.0, 148.0, 100.0));
    });
}
#[test]
fn lays_out_children_vertical() {
    run(|| {
        let (panel, a, splitter, b) = scene(Orientation::Vertical, 300.0, f64::NAN, f64::NAN);
        layout(&panel);
        assert_eq!(panel.bounds().size(), Size::new(100.0, 300.0));
        assert_eq!(a.bounds(), Rect::new(0.0, 0.0, 100.0, 148.0));
        assert_eq!(splitter.bounds(), Rect::new(0.0, 148.0, 100.0, 4.0));
        assert_eq!(b.bounds(), Rect::new(0.0, 152.0, 100.0, 148.0));
    });
}
#[test]
fn collapsed_children_do_not_reset_other_collapsed_proportions() {
    run(|| {
        let (panel, a, _, b) = scene(Orientation::Horizontal, 300.0, 0.4, 0.6);
        layout(&panel);
        let stored = ProportionalStackPanel::get_collapsed_proportion(&b);
        assert!(!stored.is_nan());
        ProportionalStackPanel::set_is_collapsed(&a, true);
        layout(&panel);
        near(ProportionalStackPanel::get_collapsed_proportion(&b), stored);
        near(ProportionalStackPanel::get_proportion(&b), 1.0);
        ProportionalStackPanel::set_is_collapsed(&a, false);
        layout(&panel);
        near(ProportionalStackPanel::get_proportion(&a), 0.4);
        near(ProportionalStackPanel::get_proportion(&b), 0.6);
    });
}
#[test]
fn should_not_trim_borders_in_either_orientation() {
    run(|| {
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            for (proportion, first, second) in [
                (0.5, 300.0, 300.0),
                (0.25, 150.0, 450.0),
                (0.628_318_530_717_958_6, 377.0, 223.0),
                (0.314_159_265_358_979_3, 188.0, 412.0),
            ] {
                let (panel, a, splitter, b) =
                    scene(orientation, 604.0, proportion, 1.0 - proportion);
                layout(&panel);
                near(extent(&a, orientation), first);
                near(extent(&b, orientation), second);
                near(
                    extent(&a, orientation)
                        + extent(&splitter, orientation)
                        + extent(&b, orientation),
                    604.0,
                );
            }
        }
    });
}
#[test]
fn lays_out_nested_default_panels() {
    run(|| {
        let panel = ProportionalStackPanel::new();
        panel.set_orientation(Orientation::Horizontal);
        panel.set_width(1000.0);
        panel.set_height(500.0);
        for column in 0..3 {
            if column > 0 {
                panel.children().add(ProportionalStackPanelSplitter::new());
            }
            let inner = ProportionalStackPanel::new();
            for row in 0..3 {
                if row > 0 {
                    inner.children().add(ProportionalStackPanelSplitter::new());
                }
                let child = Border::new();
                if (column == 0 && row == 0) || (column == 2 && row == 2) {
                    ProportionalStackPanel::set_proportion(&child, 0.5);
                }
                inner.children().add(child);
            }
            panel.children().add(inner);
        }
        layout(&panel);
        assert_eq!(panel.bounds().size(), Size::new(1000.0, 500.0));
        let expected = [
            (0.0, 330.0),
            (330.0, 4.0),
            (334.0, 331.0),
            (665.0, 4.0),
            (669.0, 331.0),
        ];
        for (index, (x, width)) in expected.into_iter().enumerate() {
            assert_eq!(
                panel.children().get(index).bounds(),
                Rect::new(x, 0.0, width, 500.0)
            );
        }
    });
}
#[test]
fn assigning_proportions_respects_original_minimum_and_maximum_cases() {
    run(|| {
        let (panel, a, _, _) = scene(Orientation::Horizontal, 500.0, 0.1, 0.5);
        a.set_min_width(150.0);
        layout(&panel);
        assert!(a.bounds().width >= 150.0);
        let (panel, a, _, _) = scene(Orientation::Horizontal, 300.0, 0.9, 0.7);
        a.set_max_width(100.0);
        layout(&panel);
        assert!(a.bounds().width <= 100.0);
        let (panel, a, _, _) = scene(Orientation::Vertical, 500.0, 0.1, 0.5);
        a.set_min_height(200.0);
        layout(&panel);
        assert!(a.bounds().height >= 200.0);
    });
}
#[test]
fn thickness_changes_remeasure_splitter_and_adjacent_panes() {
    run(|| {
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            let panel = ProportionalStackPanel::new();
            panel.set_orientation(orientation);
            let a = Border::new();
            let b = Border::new();
            let splitter = ProportionalStackPanelSplitter::new();
            panel.children().add(a.clone());
            panel.children().add(splitter.clone());
            panel.children().add(b);
            let window = host(panel.upcast());
            near(extent(&splitter, orientation), 4.0);
            let before = extent(&a, orientation);
            splitter.set_thickness(24.0);
            window.update_layout();
            near(extent(&splitter, orientation), 24.0);
            near(extent(&a, orientation), before - 10.0);
            splitter.set_thickness(0.0);
            window.update_layout();
            near(extent(&splitter, orientation), 0.0);
            window.close();
        }
    });
}
#[test]
fn orientation_changes_update_same_splitter_and_keep_source_pseudo_class_accumulation() {
    run(|| {
        let panel = ProportionalStackPanel::new();
        panel.set_orientation(Orientation::Horizontal);
        let splitter = ProportionalStackPanelSplitter::new();
        splitter.set_thickness(10.0);
        panel.children().add(Border::new());
        panel.children().add(splitter.clone());
        panel.children().add(Border::new());
        let window = host(panel.clone().upcast());
        near(splitter.bounds().width, 10.0);
        near(splitter.bounds().height, panel.bounds().height);
        panel.set_orientation(Orientation::Vertical);
        window.update_layout();
        near(splitter.bounds().height, 10.0);
        near(splitter.bounds().width, panel.bounds().width);
        assert!(splitter.width().is_nan());
        panel.set_orientation(Orientation::Horizontal);
        window.update_layout();
        near(splitter.bounds().width, 10.0);
        assert!(splitter.height().is_nan());
        assert!(splitter.classes().contains(":horizontal"));
        assert!(splitter.classes().contains(":vertical"));
        window.close();
    });
}
#[repr(C)]
struct PolicyPanel {
    base: ProportionalStackPanel,
    assignments: Cell<usize>,
    resizes: Cell<usize>,
    target: RefCell<Option<Ref<Control>>>,
    neighbor: RefCell<Option<Ref<Control>>>,
    delta: Cell<f64>,
    minimum: Cell<f64>,
}
ferro_class!(PolicyPanel:ProportionalStackPanel);
ferro_impl_classes!(PolicyPanel:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl PolicyPanel {
    fn new(orientation: Orientation) -> Ref<Self> {
        let panel = instantiate(Self {
            base: ProportionalStackPanel::construct(),
            assignments: Cell::new(0),
            resizes: Cell::new(0),
            target: RefCell::new(None),
            neighbor: RefCell::new(None),
            delta: Cell::new(0.0),
            minimum: Cell::new(0.0),
        });
        panel.set_orientation(orientation);
        panel.children().add(Border::new());
        let splitter = ProportionalStackPanelSplitter::new();
        splitter.set_thickness(10.0);
        splitter.set_template(Some(FuncControlTemplate::new(|_, _| {
            let border = Border::new();
            border.set_background(Some(ferroui_base::media::Brushes::gray()));
            border.upcast()
        })));
        panel.children().add(splitter);
        panel.children().add(Border::new());
        panel
    }
}
impl ProportionalStackPanelImpl for PolicyPanel {
    fn get_proportional_dimension(this: &Self, size: Size, splitter: f64) -> f64 {
        max(
            1.0,
            Self::parent_get_proportional_dimension(this, size, splitter),
        )
    }
    fn on_assign_proportions(this: &Self, size: Size, splitter: f64) {
        this.assignments.set(this.assignments.get() + 1);
        let dimension = this.get_proportional_dimension(size, splitter);
        ProportionalStackPanel::set_proportion(&this.children().get(0), 120.0 / dimension);
        ProportionalStackPanel::set_proportion(&this.children().get(2), 180.0 / dimension);
    }
    fn resize_proportions(
        this: &Self,
        target: Ref<Control>,
        neighbor: Ref<Control>,
        delta: f64,
        minimum: f64,
    ) {
        this.resizes.set(this.resizes.get() + 1);
        this.target.replace(Some(target));
        this.neighbor.replace(Some(neighbor));
        this.delta.set(delta);
        this.minimum.set(minimum);
    }
}
#[test]
fn assignment_and_dimension_overrides_preserve_explicit_space_after_window_resize() {
    run(|| {
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            let panel = PolicyPanel::new(orientation);
            let window = host(panel.clone().upcast());
            assert!(panel.assignments.get() >= 2);
            assert!(panel.is_measure_valid());
            near(extent(&panel.children().get(0), orientation), 120.0);
            near(extent(&panel.children().get(2), orientation), 180.0);
            window.set_width(window.width() + 100.0);
            window.set_height(window.height() + 100.0);
            update(&window);
            near(extent(&panel.children().get(0), orientation), 120.0);
            near(extent(&panel.children().get(2), orientation), 180.0);
            window.close();
            let defaults = ProportionalStackPanel::new();
            defaults.set_orientation(orientation);
            for _ in 0..2 {
                let child = Border::new();
                ProportionalStackPanel::set_proportion(&child, 0.25);
                defaults.children().add(child);
            }
            defaults.measure(Size::new(400.0, 400.0));
            defaults.arrange(Rect::new(0.0, 0.0, 400.0, 400.0));
            near(extent(&defaults.children().get(0), orientation), 200.0);
            near(
                ProportionalStackPanel::get_proportion(&defaults.children().get(0)),
                0.5,
            );
        }
    });
}
#[test]
fn desired_extent_uses_override_when_gaps_exceed_viewport() {
    run(|| {
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            let panel = PolicyPanel::new(orientation);
            let size = ProportionalStackPanel::measure_override(&panel, Size::new(4.0, 4.0));
            near(
                ProportionUtils::get_relevant_dimension(size, orientation),
                300.0,
            );
        }
    });
}
#[test]
fn native_pointer_and_preview_input_invoke_resize_override() {
    run(|| {
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            for preview in [false, true] {
                let panel = PolicyPanel::new(orientation);
                let splitter = panel
                    .children()
                    .get(1)
                    .cast::<ProportionalStackPanelSplitter>()
                    .unwrap();
                splitter.set_preview_resize(preview);
                ProportionalStackPanelSplitter::set_minimum_proportion_size(&splitter, 25.0);
                let window = host(panel.clone().upcast());
                let point = splitter
                    .translate_point(
                        Point::new(
                            splitter.bounds().width / 2.0,
                            splitter.bounds().height / 2.0,
                        ),
                        &window,
                    )
                    .unwrap();
                let end = if orientation == Orientation::Horizontal {
                    Point::new(point.x + 30.0, point.y)
                } else {
                    Point::new(point.x, point.y + 30.0)
                };
                window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
                window.mouse_move(end, RawInputModifiers::NONE);
                assert_eq!(panel.resizes.get(), if preview { 0 } else { 1 });
                window.mouse_up(end, MouseButton::Left, RawInputModifiers::NONE);
                assert_eq!(panel.resizes.get(), 1);
                assert_eq!(
                    panel.target.borrow().as_ref(),
                    Some(&panel.children().get(0))
                );
                assert_eq!(
                    panel.neighbor.borrow().as_ref(),
                    Some(&panel.children().get(2))
                );
                near(panel.delta.get(), 30.0);
                near(panel.minimum.get(), 25.0);
                window.close();
            }
        }
    });
}

#[repr(C)]
struct MeasuringControl {
    base: Control,
    constraint: Cell<Size>,
}
ferro_class!(MeasuringControl:Control);
ferro_impl_classes!(MeasuringControl:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl LayoutableImpl for MeasuringControl {
    fn measure_override(this: &Self, size: Size) -> Size {
        this.constraint.set(size);
        Size::new(100.0, 100.0)
    }
}
impl MeasuringControl {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            constraint: Cell::new(Size::default()),
        })
    }
}
#[repr(C)]
struct MeasurementPanel {
    base: ProportionalStackPanel,
    offered: Cell<Size>,
    gaps: Cell<f64>,
}
ferro_class!(MeasurementPanel:ProportionalStackPanel);
ferro_impl_classes!(MeasurementPanel:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl ProportionalStackPanelImpl for MeasurementPanel {
    fn get_measure_constraint(this: &Self, size: Size, gaps: f64) -> Size {
        this.offered.set(size);
        this.gaps.set(gaps);
        if this.orientation() == Orientation::Horizontal {
            Size::new(300.0 + gaps, size.height)
        } else {
            Size::new(size.width, 300.0 + gaps)
        }
    }
    fn get_child_measure_constraint(this: &Self, child: Ref<Control>, constraint: Size) -> Size {
        if child == this.children().get(0) {
            if this.orientation() == Orientation::Horizontal {
                Size::new(f64::INFINITY, constraint.height)
            } else {
                Size::new(constraint.width, f64::INFINITY)
            }
        } else {
            constraint
        }
    }
}
#[test]
fn measurement_policies_resolve_infinite_space_and_preserve_intrinsic_child_constraints() {
    run(|| {
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            let panel = instantiate(MeasurementPanel {
                base: ProportionalStackPanel::construct(),
                offered: Cell::new(Size::default()),
                gaps: Cell::new(0.0),
            });
            panel.set_orientation(orientation);
            let intrinsic = MeasuringControl::new();
            let proportional = MeasuringControl::new();
            let splitter = ProportionalStackPanelSplitter::new();
            splitter.set_thickness(10.0);
            panel.children().add(intrinsic.clone());
            panel.children().add(splitter);
            panel.children().add(proportional.clone());
            let offered = if orientation == Orientation::Horizontal {
                Size::new(f64::INFINITY, 400.0)
            } else {
                Size::new(400.0, f64::INFINITY)
            };
            panel.measure(offered);
            panel.arrange(Rect::from_size(panel.desired_size()));
            assert_eq!(panel.offered.get(), offered);
            near(panel.gaps.get(), 10.0);
            near(
                ProportionUtils::get_relevant_dimension(panel.desired_size(), orientation),
                310.0,
            );
            assert!(ProportionUtils::get_relevant_dimension(
                intrinsic.constraint.get(),
                orientation
            )
            .is_infinite());
            near(
                ProportionUtils::get_relevant_dimension(proportional.constraint.get(), orientation),
                150.0,
            );
            near(extent(&intrinsic, orientation), 150.0);
            near(extent(&proportional, orientation), 150.0);
            let defaults = ProportionalStackPanel::new();
            defaults.set_orientation(orientation);
            defaults.children().add(Border::new());
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || defaults.measure(offered)
            ))
            .is_err());
        }
    });
}
#[test]
fn dimensions_keep_source_ties_to_even_fraction_carry_nan_and_constraint_rules() {
    run(|| {
        let control = Border::new();
        control.set_min_width(20.0);
        control.set_max_width(40.0);
        near(
            DimensionCalculator::calculate_dimension_with_constraints(
                &control,
                Orientation::Horizontal,
                100.0,
                0.1,
                &mut 0.0,
            ),
            20.0,
        );
        near(
            DimensionCalculator::calculate_dimension_with_constraints(
                &control,
                Orientation::Horizontal,
                100.0,
                0.8,
                &mut 0.0,
            ),
            40.0,
        );
        let mut fractions = 0.5;
        near(
            DimensionCalculator::calculate_dimension(3.0, 0.5, &mut fractions),
            2.0,
        );
        near(fractions, 0.0);
        assert!(max(f64::NAN, 1.0).is_nan());
        assert!(max(1.0, f64::NAN).is_nan());
        assert_eq!(max(-0.0, 0.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(min(-0.0, 0.0).to_bits(), (-0.0f64).to_bits());
        assert!(ProportionUtils::is_valid_proportion(f64::INFINITY));
        assert!(!ProportionUtils::is_valid_proportion(-1.0));
        assert!(ProportionUtils::clamp(f64::NAN, 0.0, 1.0).is_nan());
        near(ProportionUtils::dimension_to_proportion(5.0, 0.0), 0.0);
    });
}
#[test]
fn source_non_splitter_enumeration_invalidates_without_an_extra_collection_subscription() {
    run(|| {
        let (panel, _, _, _) = scene(Orientation::Horizontal, 300.0, 0.5, 0.5);
        let children = panel.children();
        let mut values = ProportionUtils::get_non_splitter_children(&children);
        assert!(values.next().is_some());
        children.add(Border::new());
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| values.next())).is_err());
    });
}
#[test]
fn compiled_splitter_theme_builds_and_template_bindings_follow_the_control() {
    run(|| {
        use ferroui_base::controls::{ResourceDictionary, ResourceKey};
        use ferroui_base::styling::ControlTheme;
        let value=crate::compiled_xaml::try_load(None,"ferres://FerroDock.Controls.ProportionalStackPanel/ProportionalStackPanelSplitter.xaml").unwrap().unwrap();
        let dictionary = value.downcast_ref::<Ref<ResourceDictionary>>().unwrap();
        let resource = dictionary
            .get(&ResourceKey::from(ProportionalStackPanelSplitter::TYPE))
            .unwrap();
        let theme = resource.downcast_ref::<Ref<ControlTheme>>().unwrap();
        let splitter = ProportionalStackPanelSplitter::new();
        splitter.set_theme(theme.clone());
        let window = host(splitter.clone().upcast());
        splitter.apply_template();
        let border = splitter.visual_children().get(0).cast::<Border>().unwrap();
        assert!(border.background().is_some());
        splitter.set_background(Some(ferroui_base::media::Brushes::red()));
        assert_eq!(border.background(), splitter.background());
        window.close();
    });
}

#[path = "recording_drawing.rs"]
mod recording_drawing;
#[test]
fn preview_adorner_uses_source_geometry_brush_precedence_live_opacity_and_capture_cleanup() {
    run(|| {
        use crate::splitter_preview_adorner::SplitterPreviewAdorner;
        use ferroui_base::input::{Pointer, PointerCaptureLostEventArgs, PointerType};
        use ferroui_base::media::{Brushes, Colors, DrawingContext, IBrush};
        use ferroui_controls::primitives::{AdornerLayer, VisualLayerManager};
        use std::rc::Rc;
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            let panel = PolicyPanel::new(orientation);
            let splitter = panel
                .children()
                .get(1)
                .cast::<ProportionalStackPanelSplitter>()
                .unwrap();
            splitter.set_preview_resize(true);
            let manager = VisualLayerManager::new();
            manager.set_child(panel.clone());
            let window = host(manager.clone().upcast());
            let layer = manager.adorner_layer().unwrap();
            let point = splitter
                .translate_point(
                    Point::new(
                        splitter.bounds().width / 2.0,
                        splitter.bounds().height / 2.0,
                    ),
                    &window,
                )
                .unwrap();
            window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
            assert_eq!(layer.children().count(), 1);
            let preview = layer
                .children()
                .get(0)
                .cast::<SplitterPreviewAdorner>()
                .unwrap();
            assert_eq!(preview.orientation(), orientation);
            near(preview.thickness(), 10.0);
            assert_eq!(
                preview.parent().unwrap(),
                panel.clone().upcast::<ferroui_base::StyledElement>()
            );
            assert_eq!(
                AdornerLayer::get_adorned_element(&preview),
                Some(panel.clone().upcast::<ferroui_base::Visual>())
            );
            let start = preview.offset();
            let end = if orientation == Orientation::Horizontal {
                Point::new(point.x + 30.0, point.y)
            } else {
                Point::new(point.x, point.y + 30.0)
            };
            window.mouse_move(end, RawInputModifiers::NONE);
            near(preview.offset(), start + 30.0);
            assert!(splitter.classes().contains(":preview"));
            let mut drawing = recording_drawing::RecordingDrawing::new();
            preview.render(&mut DrawingContext::new(&mut drawing));
            let (brush, rect) = drawing.draws.last().unwrap();
            assert_eq!(brush.as_solid_color_brush().unwrap().color(), Colors::GRAY);
            let expected = if orientation == Orientation::Horizontal {
                Rect::new(start + 30.0, 0.0, 10.0, panel.bounds().height)
            } else {
                Rect::new(0.0, start + 30.0, panel.bounds().width, 10.0)
            };
            assert_eq!(rect.rect, expected);
            panel.resources().set(
                "DockApplicationAccentBrushIndicator",
                Some(Rc::new(Brushes::red() as Rc<dyn IBrush>)),
            );
            drawing.draws.clear();
            preview.render(&mut DrawingContext::new(&mut drawing));
            assert_eq!(
                drawing.draws[0].0.as_solid_color_brush().unwrap().color(),
                Colors::RED
            );
            splitter.set_preview_background(Some(Brushes::blue()));
            splitter.set_preview_opacity(0.5);
            near(preview.opacity(), 0.5);
            drawing.draws.clear();
            preview.render(&mut DrawingContext::new(&mut drawing));
            assert_eq!(
                drawing.draws[0].0.as_solid_color_brush().unwrap().color(),
                Colors::BLUE
            );
            // Raw mouse release also loses capture; the source capture-loss
            // handler cleans up even when disabled release skips committing.
            splitter.set_is_resizing_enabled(false);
            window.mouse_up(end, MouseButton::Left, RawInputModifiers::NONE);
            assert_eq!(layer.children().count(), 0);
            assert_eq!(panel.resizes.get(), 0);
            assert!(!splitter.classes().contains(":preview"));
            splitter.raise_event(&PointerCaptureLostEventArgs::new(
                splitter.clone(),
                Pointer::new(1, PointerType::Mouse, true),
            ));
            assert_eq!(layer.children().count(), 0);
            assert!(preview.parent().is_none());
            assert!(!splitter.classes().contains(":preview"));
            window.close();
        }
    });
}

#[test]
fn preview_resize_commits_after_layout_and_render_jobs_between_pointer_events() {
    run(|| {
        use ferroui_controls::primitives::{AdornerLayer, VisualLayerManager};
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            let panel = PolicyPanel::new(orientation);
            let splitter = panel
                .children()
                .get(1)
                .cast::<ProportionalStackPanelSplitter>()
                .unwrap();
            splitter.set_preview_resize(true);
            let manager = VisualLayerManager::new();
            manager.set_child(panel.clone());
            let window = host(manager.upcast());
            let point = splitter
                .translate_point(
                    Point::new(
                        splitter.bounds().width / 2.0,
                        splitter.bounds().height / 2.0,
                    ),
                    &window,
                )
                .unwrap();
            let end = if orientation == Orientation::Horizontal {
                Point::new(point.x + 30.0, point.y)
            } else {
                Point::new(point.x, point.y + 30.0)
            };
            window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
            update(&window);
            window.mouse_move(end, RawInputModifiers::NONE);
            update(&window);
            assert_eq!(panel.resizes.get(), 0);
            window.mouse_up(end, MouseButton::Left, RawInputModifiers::NONE);
            update(&window);
            assert_eq!(panel.resizes.get(), 1);
            near(panel.delta.get(), 30.0);
            assert_eq!(
                AdornerLayer::get_adorner_layer(&panel)
                    .unwrap()
                    .children()
                    .count(),
                0
            );
            window.close();
        }
    });
}
