//! Port of `Pages/CompositionPage.xaml.cs`: the class of the document
//! `Pages/CompositionPage.xaml`, its custom visual handler and
//! `CompositionPageColorItem`.

use crate::markup::xaml_class;
use ferroui_base::animation::PlaybackDirection;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, Colors, IBrush, ImmediateDrawingContext, SolidColorBrush};
use ferroui_base::numerics::Vector3;
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::animations::{AnimationIterationBehavior, ImplicitAnimationCollection};
use ferroui_base::rendering::composition::{
    CompositionBrush, CompositionConicGradientBrush, CompositionCustomVisual, CompositionCustomVisualHandler,
    CompositionGradientBrush, CompositionLinearGradientBrush, CompositionRadialGradientBrush,
    CompositionSolidColorBrush, CompositionSolidColorVisual, ElementComposition, ICompositionCustomVisualHandler,
    ICompositionObjectAnimations,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, instantiate, BoxedValue, FerroObjectImpl,
    Point, Rect, Ref, RelativePoint, RelativeUnit, StyledElementImpl, Vector, Vector3D, Visual, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Border, CheckBox, ControlImpl, ItemsControl, ItemsSource, MultiPageImpl, PageImpl, SelectingMultiPageImpl,
    TabbedPage,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

#[repr(C)]
pub struct CompositionPage {
    base: TabbedPage,
    implicit_animations: RefCell<Option<Rc<ImplicitAnimationCollection>>>,
    custom_visual: RefCell<Option<CompositionCustomVisual>>,
    solid_visual: RefCell<Option<CompositionSolidColorVisual>>,
}

ferro_class!(CompositionPage: TabbedPage);
ferro_impl_classes!(
    CompositionPage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl,
    MultiPageImpl,
    SelectingMultiPageImpl
);

/// An event handler of the document: `(sender, e)`.
macro_rules! handler {
    ($method:ident) => {
        |this: &Ref<CompositionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
            this.$method(&sender, e.as_routed_event_args())
        }
    };
}

ferro_class_info!(CompositionPage {
    new: CompositionPage::new,
    markup: {
        methods: [
            static fn SetEnableAnimations(Ref<Border>, bool) =>
                |border: Ref<Border>, value: bool| CompositionPage::set_enable_animations(&border, value),
            fn ButtonThreadSleep(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) => handler!(button_thread_sleep),
            fn ButtonStartCustomVisual(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                handler!(button_start_custom_visual),
            fn ButtonStopCustomVisual(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                handler!(button_stop_custom_visual),
            fn PreciseDirtyRectsCheckboxCustomVisualChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                handler!(precise_dirty_rects_checkbox_custom_visual_changed),
            fn SolidBrushCreateAnimate_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                handler!(solid_brush_create_animate_click),
            fn LinearBrushCreateAnimate_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                handler!(linear_brush_create_animate_click),
            fn RadialBrushCreateAnimate_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                handler!(radial_brush_create_animate_click),
            fn ConicBrushCreateAnimate_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                handler!(conic_brush_create_animate_click),
        ],
    },
});
xaml_class!(CompositionPage, "/Pages/CompositionPage.xaml");

impl VisualImpl for CompositionPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.items().set_items_source(Some(ItemsSource::from_values(Self::create_color_items())));
    }
}

impl CompositionPage {
    pub fn construct() -> Self {
        Self {
            base: TabbedPage::construct(),
            implicit_animations: RefCell::new(None),
            custom_visual: RefCell::new(None),
            solid_visual: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.attach_animated_solid_visual(&this.get_control::<Visual>("SolidVisualHost"));
        this.attach_custom_visual(&this.get_control::<Visual>("CustomVisualHost"));
        this
    }

    fn items(&self) -> Ref<ItemsControl> {
        self.get_control::<ItemsControl>("Items")
    }

    fn create_color_items() -> Vec<Rc<CompositionPageColorItem>> {
        const COLORS: [(u8, u8, u8, u8); 48] = [
            (255, 255, 185, 0),
            (255, 231, 72, 86),
            (255, 0, 120, 215),
            (255, 0, 153, 188),
            (255, 122, 117, 116),
            (255, 118, 118, 118),
            (255, 255, 141, 0),
            (255, 232, 17, 35),
            (255, 0, 99, 177),
            (255, 45, 125, 154),
            (255, 93, 90, 88),
            (255, 76, 74, 72),
            (255, 247, 99, 12),
            (255, 234, 0, 94),
            (255, 142, 140, 216),
            (255, 0, 183, 195),
            (255, 104, 118, 138),
            (255, 105, 121, 126),
            (255, 202, 80, 16),
            (255, 195, 0, 82),
            (255, 107, 105, 214),
            (255, 3, 131, 135),
            (255, 81, 92, 107),
            (255, 74, 84, 89),
            (255, 218, 59, 1),
            (255, 227, 0, 140),
            (255, 135, 100, 184),
            (255, 0, 178, 148),
            (255, 86, 124, 115),
            (255, 100, 124, 100),
            (255, 239, 105, 80),
            (255, 191, 0, 119),
            (255, 116, 77, 169),
            (255, 1, 133, 116),
            (255, 72, 104, 96),
            (255, 82, 94, 84),
            (255, 209, 52, 56),
            (255, 194, 57, 179),
            (255, 177, 70, 194),
            (255, 0, 204, 106),
            (255, 73, 130, 5),
            (255, 132, 117, 69),
            (255, 255, 67, 67),
            (255, 154, 0, 137),
            (255, 136, 23, 152),
            (255, 16, 137, 62),
            (255, 16, 124, 16),
            (255, 126, 115, 95),
        ];

        COLORS.iter().map(|&(a, r, g, b)| CompositionPageColorItem::new(Color::from_argb(a, r, g, b))).collect()
    }

    fn ensure_implicit_animations(&self) {
        if self.implicit_animations.borrow().is_some() {
            return;
        }

        let compositor = ElementComposition::get_element_visual(self)
            .expect("the page has a composition visual")
            .compositor()
            .clone();

        let offset_animation = compositor.create_vector3_key_frame_animation();
        offset_animation.set_target(Some("Offset".to_owned()));
        offset_animation.insert_expression_key_frame(1.0, "this.FinalValue", None);
        offset_animation.set_duration(Duration::from_millis(400));

        let rotation_animation = compositor.create_scalar_key_frame_animation();
        rotation_animation.set_target(Some("RotationAngle".to_owned()));
        rotation_animation.insert_key_frame(0.5, 0.160);
        rotation_animation.insert_key_frame(1.0, 0.0);
        rotation_animation.set_duration(Duration::from_millis(400));

        let animation_group = compositor.create_animation_group();
        animation_group.add(offset_animation);
        animation_group.add(rotation_animation);

        let implicit_animations = compositor.create_implicit_animation_collection();
        implicit_animations.set("Offset", animation_group);
        *self.implicit_animations.borrow_mut() = Some(implicit_animations);
    }

    /// The setter of the attached `EnableAnimations` of the document.
    /// The value is not read, as upstream does not read it: setting it
    /// enables the animations.
    pub fn set_enable_animations(border: &Border, _value: bool) {
        let Some(page) = border.find_ancestor_of_type::<CompositionPage>(false) else {
            let weak = border.to_ref().downgrade();
            // The subscription lives as long as the border, as the delegate of upstream does.
            let _subscription: Rc<dyn IDisposable> = border.attached_to_visual_tree(move |_| {
                if let Some(border) = weak.upgrade() {
                    CompositionPage::set_enable_animations(&border, true);
                }
            });
            return;
        };

        if ElementComposition::get_element_visual(&page).is_none() {
            return;
        }

        page.ensure_implicit_animations();
        if let Some(visual_parent) = border.get_visual_parent() {
            if let Some(composition_visual) = ElementComposition::get_element_visual(&visual_parent) {
                composition_visual.set_implicit_animations(page.implicit_animations.borrow().clone());
            }
        }
    }

    fn attach_animated_solid_visual(&self, v: &Visual) {
        // The handlers belong to `v`, a child of the page: they hold the page and `v` weakly.
        let weak = self.to_ref().downgrade();
        let weak_v = v.to_ref().downgrade();
        let update = Rc::new(move || {
            let (Some(this), Some(v)) = (weak.upgrade(), weak_v.upgrade()) else {
                return;
            };
            let solid_visual = this.solid_visual.borrow().clone();
            let Some(solid_visual) = solid_visual else {
                return;
            };
            let props = solid_visual.visual_props();
            props.set_size(&**solid_visual, Vector::new(v.bounds().width / 3.0, v.bounds().height / 3.0));
            props.set_offset(&**solid_visual, Vector3D::new(v.bounds().width / 3.0, v.bounds().height / 3.0, 0.0));
        });

        let weak = self.to_ref().downgrade();
        let weak_v = v.to_ref().downgrade();
        let on_attached = update.clone();
        let _attached = v.attached_to_visual_tree(move |_| {
            let (Some(this), Some(v)) = (weak.upgrade(), weak_v.upgrade()) else {
                return;
            };
            let Some(compositor) = ElementComposition::get_element_visual(&v).map(|v| v.compositor().clone()) else {
                return;
            };
            let same_compositor =
                this.solid_visual.borrow().as_ref().is_some_and(|s| Rc::ptr_eq(s.compositor(), &compositor));
            if same_compositor {
                return;
            }
            let solid_visual = compositor.create_solid_color_visual();
            *this.solid_visual.borrow_mut() = Some(solid_visual.clone());
            ElementComposition::set_element_child_visual(&v, Some((*solid_visual).clone()));
            solid_visual.set_color(Colors::RED);
            let animation = compositor.create_color_key_frame_animation();
            animation.insert_key_frame(0.0, Colors::RED);
            animation.insert_key_frame(0.5, Colors::BLUE);
            animation.insert_key_frame(1.0, Colors::GREEN);
            animation.set_duration(Duration::from_secs(5));
            animation.set_iteration_behavior(AnimationIterationBehavior::Forever);
            animation.set_direction(PlaybackDirection::Alternate);
            solid_visual.start_animation("Color", &*animation);

            solid_visual.visual_props().set_anchor_point(&**solid_visual, Vector::new(0.0, 0.0));

            let scale = compositor.create_vector3_key_frame_animation();
            scale.set_duration(Duration::from_secs(5));
            scale.set_iteration_behavior(AnimationIterationBehavior::Forever);
            scale.insert_key_frame(0.0, Vector3::new(1.0, 1.0, 0.0));
            scale.insert_key_frame(0.5, Vector3::new(1.5, 1.5, 0.0));
            scale.insert_key_frame(1.0, Vector3::new(1.0, 1.0, 0.0));

            solid_visual.start_animation("Scale", &*scale);

            let center = compositor
                .create_expression_animation_with("Vector3(this.Target.Size.X * 0.5, this.Target.Size.Y * 0.5, 1)");
            solid_visual.start_animation("CenterPoint", &*center);
            on_attached();
        });

        let _bounds = v.property_changed(move |a| {
            if a.property() == Visual::bounds_property().as_property() {
                update();
            }
        });
    }

    fn attach_custom_visual(&self, v: &Visual) {
        // The handlers belong to `v`, a child of the page: they hold the page and `v` weakly.
        let weak = self.to_ref().downgrade();
        let weak_v = v.to_ref().downgrade();
        let update = Rc::new(move || {
            let (Some(this), Some(v)) = (weak.upgrade(), weak_v.upgrade()) else {
                return;
            };
            let custom_visual = this.custom_visual.borrow().clone();
            let Some(custom_visual) = custom_visual else {
                return;
            };
            let h = v.bounds().height.min(v.bounds().width / 3.0) as f32;
            let props = custom_visual.visual_props();
            props.set_size(&**custom_visual, Vector::new(v.bounds().width, f64::from(h)));
            props.set_offset(&**custom_visual, Vector3D::new(0.0, (v.bounds().height - f64::from(h)) / 2.0, 0.0));
        });

        let weak = self.to_ref().downgrade();
        let weak_v = v.to_ref().downgrade();
        let on_attached = update.clone();
        let _attached = v.attached_to_visual_tree(move |_| {
            let (Some(this), Some(v)) = (weak.upgrade(), weak_v.upgrade()) else {
                return;
            };
            let Some(compositor) = ElementComposition::get_element_visual(&v).map(|v| v.compositor().clone()) else {
                return;
            };
            let same_compositor =
                this.custom_visual.borrow().as_ref().is_some_and(|c| Rc::ptr_eq(c.compositor(), &compositor));
            if same_compositor {
                return;
            }
            let custom_visual = compositor.create_custom_visual(|| Rc::new(CustomVisualHandler::new()) as Rc<dyn ICompositionCustomVisualHandler>);
            *this.custom_visual.borrow_mut() = Some(custom_visual.clone());
            ElementComposition::set_element_child_visual(&v, Some((*custom_visual).clone()));
            custom_visual.send_handler_message(CustomVisualHandler::start_message());
            this.precise_dirty_rects_checkbox_custom_visual_changed(&None, &RoutedEventArgs::new());
            on_attached();
        });

        let _bounds = v.property_changed(move |a| {
            if a.property() == Visual::bounds_property().as_property() {
                update();
            }
        });
    }

    // Deviation (DEVIATIONS.md, ControlCatalog sample): blocks the UI thread, as upstream's
    // `Thread.Sleep(5000)` does.
    fn button_thread_sleep(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        std::thread::sleep(Duration::from_millis(5000));
    }

    fn button_start_custom_visual(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(custom_visual) = self.custom_visual.borrow().as_ref() {
            custom_visual.send_handler_message(CustomVisualHandler::start_message());
        }
    }

    fn button_stop_custom_visual(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(custom_visual) = self.custom_visual.borrow().as_ref() {
            custom_visual.send_handler_message(CustomVisualHandler::stop_message());
        }
    }

    fn precise_dirty_rects_checkbox_custom_visual_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let precise = self
            .find_control::<CheckBox>("PreciseDirtyRectsCheckboxCustomVisual")
            .is_some_and(|check_box| check_box.is_checked() == Some(true));
        if let Some(custom_visual) = self.custom_visual.borrow().as_ref() {
            custom_visual.send_handler_message(if precise {
                CustomVisualHandler::use_precise_dirty_rects()
            } else {
                CustomVisualHandler::use_non_precise_dirty_rects()
            });
        }
    }

    /// The background of a host as a composition brush, or `None` (`Background as T`).
    fn background_brush(host: &Border) -> Option<Rc<CompositionBrush>> {
        host.background().and_then(|brush| brush.as_composition_brush().map(CompositionBrush::to_rc))
    }

    fn solid_brush_create_animate_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let host = self.get_control::<Border>("SolidBrushHost");
        let Some(visual) = ElementComposition::get_element_visual(&host) else {
            return;
        };

        let compositor = visual.compositor().clone();

        let brush = Self::background_brush(&host)
            .and_then(|brush| CompositionSolidColorBrush::from_brush(&brush))
            .unwrap_or_else(|| compositor.create_solid_color_brush());

        brush.set_color(Color::from_rgb(168, 213, 85));
        host.set_background(Some(brush.as_brush()));

        let animation = compositor.create_color_key_frame_animation();
        animation.insert_key_frame(0.0, Color::from_rgb(168, 213, 85));
        animation.insert_key_frame(0.25, Color::from_rgb(31, 167, 168));
        animation.insert_key_frame(0.75, Color::from_rgb(44, 121, 251));
        animation.insert_key_frame(1.0, Color::from_rgb(168, 213, 85));
        animation.set_duration(Duration::from_secs(4));
        animation.set_iteration_behavior(AnimationIterationBehavior::Forever);
        brush.start_animation("Color", &*animation);
    }

    fn linear_brush_create_animate_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let host = self.get_control::<Border>("LinearBrushHost");
        let Some(visual) = ElementComposition::get_element_visual(&host) else {
            return;
        };

        let compositor = visual.compositor().clone();

        let brush = Self::background_brush(&host)
            .and_then(|brush| CompositionLinearGradientBrush::from_brush(&brush))
            .unwrap_or_else(|| compositor.create_linear_gradient_brush());

        let stops = brush.gradient_stops();
        stops.clear();
        stops.add(compositor.create_gradient_stop_with(0.0, Color::from_rgb(168, 213, 85)));
        stops.add(compositor.create_gradient_stop_with(f64::from(0.33f32), Color::from_rgb(31, 167, 168)));
        stops.add(compositor.create_gradient_stop_with(f64::from(0.66f32), Color::from_rgb(44, 121, 251)));
        stops.add(compositor.create_gradient_stop_with(1.0, Color::from_rgb(168, 213, 85)));
        brush.set_start_point(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative));
        brush.set_end_point(RelativePoint::new(1.0, 0.0, RelativeUnit::Relative));

        host.set_background(Some(brush.as_brush()));

        let start = compositor.create_relative_point_key_frame_animation();
        start.set_duration(Duration::from_secs(4));
        start.set_iteration_behavior(AnimationIterationBehavior::Forever);
        start.insert_key_frame(0.0, RelativePoint::new(-1.0, 0.0, RelativeUnit::Relative));
        start.insert_key_frame(0.5, RelativePoint::new(1.0, 0.0, RelativeUnit::Relative));
        start.insert_key_frame(1.0, RelativePoint::new(-1.0, 0.0, RelativeUnit::Relative));

        let end = compositor.create_relative_point_key_frame_animation();
        end.set_duration(start.duration());
        end.set_iteration_behavior(AnimationIterationBehavior::Forever);
        end.insert_key_frame(0.0, RelativePoint::new(0.0, 0.0, RelativeUnit::Relative));
        end.insert_key_frame(0.5, RelativePoint::new(2.0, 0.0, RelativeUnit::Relative));
        end.insert_key_frame(1.0, RelativePoint::new(0.0, 0.0, RelativeUnit::Relative));

        brush.start_animation("StartPoint", &*start);
        brush.start_animation("EndPoint", &*end);

        Self::animate_gradient_stops(Some(&brush), true);
    }

    fn radial_brush_create_animate_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let host = self.get_control::<Border>("RadialBrushHost");
        let Some(visual) = ElementComposition::get_element_visual(&host) else {
            return;
        };

        let compositor = visual.compositor().clone();

        let brush = Self::background_brush(&host)
            .and_then(|brush| CompositionRadialGradientBrush::from_brush(&brush))
            .unwrap_or_else(|| compositor.create_radial_gradient_brush());

        let stops = brush.gradient_stops();
        stops.clear();
        stops.add(compositor.create_gradient_stop_with(0.0, Colors::YELLOW));
        stops.add(compositor.create_gradient_stop_with(f64::from(0.6f32), Colors::ORANGE));
        stops.add(compositor.create_gradient_stop_with(1.0, Colors::RED));
        brush.set_center(RelativePoint::CENTER);
        brush.set_gradient_origin(RelativePoint::CENTER);

        host.set_background(Some(brush.as_brush()));

        let center_anim = compositor.create_relative_point_key_frame_animation();
        center_anim.set_duration(Duration::from_secs(3));
        center_anim.set_iteration_behavior(AnimationIterationBehavior::Forever);
        center_anim.insert_key_frame(0.0, RelativePoint::new(0.2, 0.5, RelativeUnit::Relative));
        center_anim.insert_key_frame(0.5, RelativePoint::new(0.8, 0.5, RelativeUnit::Relative));
        center_anim.insert_key_frame(1.0, RelativePoint::new(0.2, 0.5, RelativeUnit::Relative));

        brush.start_animation("Center", &*center_anim);
        let radius_x_expression = compositor.create_expression_animation_with(
            "RelativeScalar(Clamp(0.2 + Abs(this.Target.Center.X - 0.5) * 0.9, 0.2, 0.85), RelativeUnit.Relative)",
        );
        let radius_y_expression = compositor.create_expression_animation_with(
            "RelativeScalar(Clamp(0.2 + Abs(this.Target.Center.Y - 0.5) * 0.9, 0.2, 0.85), RelativeUnit.Relative)",
        );
        brush.start_animation("RadiusX", &*radius_x_expression);
        brush.start_animation("RadiusY", &*radius_y_expression);

        Self::animate_gradient_stops(Some(&brush), true);
    }

    fn conic_brush_create_animate_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let host = self.get_control::<Border>("ConicBrushHost");
        let Some(visual) = ElementComposition::get_element_visual(&host) else {
            return;
        };

        let compositor = visual.compositor().clone();

        let brush = Self::background_brush(&host)
            .and_then(|brush| CompositionConicGradientBrush::from_brush(&brush))
            .unwrap_or_else(|| compositor.create_conic_gradient_brush());

        let stops = brush.gradient_stops();
        stops.clear();
        stops.add(compositor.create_gradient_stop_with(0.0, Colors::CYAN));
        stops.add(compositor.create_gradient_stop_with(0.25, Colors::MAGENTA));
        stops.add(compositor.create_gradient_stop_with(0.5, Colors::YELLOW));
        stops.add(compositor.create_gradient_stop_with(0.75, Colors::LIME));
        stops.add(compositor.create_gradient_stop_with(1.0, Colors::CYAN));

        brush.set_center(RelativePoint::CENTER);

        host.set_background(Some(brush.as_brush()));

        let angle_anim = compositor.create_scalar_key_frame_animation();
        angle_anim.set_duration(Duration::from_secs(5));
        angle_anim.set_iteration_behavior(AnimationIterationBehavior::Forever);
        angle_anim.insert_key_frame(0.0, 0.0);
        angle_anim.insert_key_frame(1.0, 360.0);
        brush.start_animation("Angle", &*angle_anim);
        let center_expression = compositor.create_expression_animation_with(
            "RelativePoint(Abs(Cos(ToRadians(this.Target.Angle))), Abs(Sin(ToRadians(this.Target.Angle))), RelativeUnit.Relative)",
        );
        brush.start_animation("Center", &*center_expression);

        Self::animate_gradient_stops(Some(&brush), false);
    }

    fn animate_gradient_stops(brush: Option<&CompositionGradientBrush>, animate_offsets: bool) {
        let Some(brush) = brush else {
            return;
        };
        let compositor = brush.compositor().clone();
        for stop in brush.gradient_stops().to_vec() {
            let Some(gs) = stop.as_composition_gradient_stop() else {
                continue;
            };

            let color_anim = compositor.create_color_key_frame_animation();
            color_anim.set_duration(Duration::from_secs(4));
            color_anim.set_iteration_behavior(AnimationIterationBehavior::Forever);
            color_anim.insert_key_frame(0.0, gs.color());
            color_anim.insert_key_frame(0.5, Self::shift_color(gs.color()));
            color_anim.insert_key_frame(1.0, gs.color());
            gs.start_animation("Color", &*color_anim);

            if !animate_offsets {
                continue;
            }
            let offset_anim = compositor.create_scalar_key_frame_animation();
            offset_anim.set_duration(Duration::from_secs(4));
            offset_anim.set_iteration_behavior(AnimationIterationBehavior::Forever);
            let o = gs.offset() as f32;
            let o2 = (gs.offset() + 0.1).clamp(0.0, 1.0);
            offset_anim.insert_key_frame(0.0, o);
            offset_anim.insert_key_frame(0.5, o2 as f32);
            offset_anim.insert_key_frame(1.0, o);
            gs.start_animation("Offset", &*offset_anim);
        }
    }

    fn shift_color(c: Color) -> Color {
        let r = 255.min((f64::from(c.r) * 1.15) as i32 + 30) as u8;
        let g = 0.max((f64::from(c.g) * 0.75) as i32 - 10) as u8;
        let b = 255.min((f64::from(c.b) * 1.2) as i32 + 20) as u8;
        Color::from_argb(c.a, r, g, b)
    }
}

// --- CustomVisualHandler -----------------------------------------------------------

/// A message of the custom visual handler: the identity of the object is the message.
struct HandlerMessage;

// The messages are sent from the UI thread and compared on the render thread by identity, as
// upstream's `static readonly object` instances are.
type HandlerMessageRef = std::sync::Arc<dyn Any + Send + Sync>;

static STOP_MESSAGE: std::sync::LazyLock<HandlerMessageRef> = std::sync::LazyLock::new(|| std::sync::Arc::new(HandlerMessage));
static START_MESSAGE: std::sync::LazyLock<HandlerMessageRef> = std::sync::LazyLock::new(|| std::sync::Arc::new(HandlerMessage));
static USE_PRECISE_DIRTY_RECTS: std::sync::LazyLock<HandlerMessageRef> =
    std::sync::LazyLock::new(|| std::sync::Arc::new(HandlerMessage));
static USE_NON_PRECISE_DIRTY_RECTS: std::sync::LazyLock<HandlerMessageRef> =
    std::sync::LazyLock::new(|| std::sync::Arc::new(HandlerMessage));

struct CustomVisualHandler {
    base: CompositionCustomVisualHandler,
    animation_elapsed: Cell<Duration>,
    last_server_time: Cell<Option<Duration>>,
    running: Cell<bool>,
    precise_dirty_rects: Cell<bool>,
    ellipses: RefCell<Vec<(Point, f64, ImmutableSolidColorBrush)>>,
}

impl CustomVisualHandler {
    fn new() -> Self {
        Self {
            base: CompositionCustomVisualHandler::new(),
            animation_elapsed: Cell::new(Duration::ZERO),
            last_server_time: Cell::new(None),
            running: Cell::new(false),
            precise_dirty_rects: Cell::new(false),
            ellipses: RefCell::new(Vec::new()),
        }
    }

    fn stop_message() -> HandlerMessageRef {
        STOP_MESSAGE.clone()
    }

    fn start_message() -> HandlerMessageRef {
        START_MESSAGE.clone()
    }

    fn use_precise_dirty_rects() -> HandlerMessageRef {
        USE_PRECISE_DIRTY_RECTS.clone()
    }

    fn use_non_precise_dirty_rects() -> HandlerMessageRef {
        USE_NON_PRECISE_DIRTY_RECTS.clone()
    }

    fn update_rects(&self) {
        if self.running.get() {
            let now = self.base.composition_now();
            if let Some(last_server_time) = self.last_server_time.get() {
                self.animation_elapsed.set(self.animation_elapsed.get() + now.saturating_sub(last_server_time));
            }
            self.last_server_time.set(Some(now));
        }

        let mut ellipses = self.ellipses.borrow_mut();
        ellipses.clear();

        const CNT: i32 = 20;
        let effective_size = self.base.effective_size();
        let max_point_size_x = effective_size.x / (f64::from(CNT) * 1.6);
        let max_point_size_y = effective_size.y / 4.0;
        let point_size = max_point_size_x.min(max_point_size_y);
        let animation_length = Duration::from_secs(4);
        let elapsed = self.animation_elapsed.get().as_secs_f64();
        let animation_stage = elapsed / animation_length.as_secs_f64();

        let sin_offset = elapsed.cos() * 1.5;

        for c in 0..CNT {
            let stage = (animation_stage + f64::from(c) / f64::from(CNT)) % 1.0;
            let color_stage =
                (animation_stage + ((elapsed * 2.0).sin() + 1.0) / 2.0 + f64::from(c) / f64::from(CNT)) % 1.0;
            let pos_x = (effective_size.x + point_size * 3.0) * stage - point_size;
            let pos_y = (effective_size.y - point_size) * (1.0 + (stage * 3.14 * 3.0 + sin_offset).sin()) / 2.0
                + point_size / 2.0;
            let opacity = (stage * 3.14).sin();

            ellipses.push((
                Point::new(pos_x, pos_y),
                point_size / 2.0,
                ImmutableSolidColorBrush::with_opacity(
                    Color::from_argb(
                        255,
                        (255.0 - 255.0 * color_stage) as u8,
                        (255.0 * (0.5 - color_stage).abs() * 2.0) as u8,
                        (255.0 * color_stage) as u8,
                    ),
                    opacity,
                ),
            ));
        }
    }

    fn invalidate_current_ellipse_rects(&self) {
        for (center, size, _) in self.ellipses.borrow().iter() {
            self.base.invalidate_rect(Rect::new(center.x - size, center.y - size, size * 2.0, size * 2.0));
        }
    }
}

impl ICompositionCustomVisualHandler for CustomVisualHandler {
    fn handler_base(&self) -> &CompositionCustomVisualHandler {
        &self.base
    }

    fn on_render(&self, drawing_context: &mut ImmediateDrawingContext<'_>) {
        if self.ellipses.borrow().is_empty() {
            self.update_rects();
        }

        for (center, size, brush) in self.ellipses.borrow().iter() {
            drawing_context.draw_ellipse(Some(brush), None, *center, *size, *size);
        }
    }

    fn on_message(&self, message: HandlerMessageRef) {
        if std::sync::Arc::ptr_eq(&message, &Self::start_message()) {
            self.running.set(true);
            self.last_server_time.set(None);
            self.base.register_for_next_animation_frame_update();
        } else if std::sync::Arc::ptr_eq(&message, &Self::stop_message()) {
            self.running.set(false);
        } else if std::sync::Arc::ptr_eq(&message, &Self::use_precise_dirty_rects()) {
            self.precise_dirty_rects.set(true);
        } else if std::sync::Arc::ptr_eq(&message, &Self::use_non_precise_dirty_rects()) {
            self.precise_dirty_rects.set(false);
        }
    }

    fn on_animation_frame_update(&self) {
        if self.running.get() {
            if self.precise_dirty_rects.get() {
                self.invalidate_current_ellipse_rects();
            } else {
                self.base.invalidate();
            }
            self.update_rects();
            if self.precise_dirty_rects.get() {
                self.invalidate_current_ellipse_rects();
            }
            self.base.register_for_next_animation_frame_update();
        }
    }
}

// --- CompositionPageColorItem ------------------------------------------------------

/// An item of the color list of the page.
pub struct CompositionPageColorItem {
    color: Color,
}

impl PartialEq for CompositionPageColorItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl CompositionPageColorItem {
    pub fn new(color: Color) -> Rc<CompositionPageColorItem> {
        Rc::new(Self { color })
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn color_brush(&self) -> Rc<dyn IBrush> {
        SolidColorBrush::with_color(self.color).into()
    }

    pub fn color_hex_value(&self) -> String {
        self.color.to_string().chars().skip(3).collect::<String>().to_uppercase()
    }
}

ferro_markup_type!(class CompositionPageColorItem {
    this: Rc<CompositionPageColorItem>,
    handles: [CompositionPageColorItem, Rc<CompositionPageColorItem>, Option<Rc<CompositionPageColorItem>>],
    constructors: [(Color) => CompositionPageColorItem::new],
    properties: [
        Color: Color { get: |this: &Rc<CompositionPageColorItem>| this.color() },
        ColorBrush: Rc<dyn IBrush> { get: |this: &Rc<CompositionPageColorItem>| this.color_brush() },
        ColorHexValue: String { get: |this: &Rc<CompositionPageColorItem>| this.color_hex_value() },
    ],
});
