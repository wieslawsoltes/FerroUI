// This source file is adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml)

use crate::automation::peers::ColorSpectrumAutomationPeer;
use crate::helpers::color_picker_helpers::ColorPickerHelpers;
use crate::helpers::hsv::Hsv;
use crate::helpers::increment_amount::IncrementAmount;
use crate::helpers::increment_direction::IncrementDirection;
use crate::helpers::rgb::Rgb;
use crate::primitives::ColorHelper;
use crate::{ColorChangedEventArgs, ColorComponent, ColorSpectrumComponents, ColorSpectrumShape, HsvComponent};
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers, PointerEventArgs,
    PointerPoint, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutHelper, LayoutableImpl};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{Color, FlowDirection, HsvColor, IBrush, IImageBrushSource, ImageBrush};
use ferroui_base::reactive::{Disposable, IDisposable, ObservableExt};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::{HandlerList, MathUtilities};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObject, FerroObjectExtensions, FerroObjectImpl,
    FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::metadata::{PseudoClassesAttribute, TemplatePartAttribute};
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use ferroui_controls::shapes::{Ellipse, Rectangle};
use ferroui_controls::{Canvas, ControlImpl, Panel, ToolTip};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A two dimensional spectrum for color selection.
#[repr(C)]
pub struct ColorSpectrum {
    base: TemplatedControl,

    color_changed: HandlerList<dyn Fn(&ColorChangedEventArgs)>,

    updating_color: Cell<bool>,
    updating_hsv_color: Cell<bool>,
    is_pointer_pressed: Cell<bool>,
    should_show_large_selection: Cell<bool>,
    hsv_values: RefCell<Rc<Vec<Hsv>>>,
    pub(super) third_component: Cell<ColorComponent>,

    layout_root_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    selection_ellipse_panel_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    /// The handlers connected to the input target in `on_apply_template`
    /// (the original removes its handlers one by one with `-=`).
    input_target_disposables: RefCell<Vec<Rc<dyn IDisposable>>>,

    // XAML template parts
    layout_root: RefCell<Option<Ref<Panel>>>,
    sizing_panel: RefCell<Option<Ref<Panel>>>,
    spectrum_rectangle: RefCell<Option<Ref<Rectangle>>>,
    spectrum_ellipse: RefCell<Option<Ref<Ellipse>>>,
    spectrum_overlay_rectangle: RefCell<Option<Ref<Rectangle>>>,
    spectrum_overlay_ellipse: RefCell<Option<Ref<Ellipse>>>,
    input_target: RefCell<Option<Ref<Canvas>>>,
    selection_ellipse_panel: RefCell<Option<Ref<Panel>>>,

    // Put the spectrum images in a bitmap, which is then given to an ImageBrush.
    hue_red_bitmap: RefCell<Option<Rc<Bitmap>>>,
    hue_yellow_bitmap: RefCell<Option<Rc<Bitmap>>>,
    hue_green_bitmap: RefCell<Option<Rc<Bitmap>>>,
    hue_cyan_bitmap: RefCell<Option<Rc<Bitmap>>>,
    hue_blue_bitmap: RefCell<Option<Rc<Bitmap>>>,
    hue_purple_bitmap: RefCell<Option<Rc<Bitmap>>>,

    saturation_minimum_bitmap: RefCell<Option<Rc<Bitmap>>>,
    saturation_maximum_bitmap: RefCell<Option<Rc<Bitmap>>>,

    value_bitmap: RefCell<Option<Rc<Bitmap>>>,
    min_bitmap: RefCell<Option<Rc<Bitmap>>>,
    max_bitmap: RefCell<Option<Rc<Bitmap>>>,

    // Fields used by UpdateEllipse() to ensure that it's using the data
    // associated with the last call to CreateBitmapsAndColorMap(),
    // in order to function properly while the asynchronous bitmap creation
    // is in progress.
    shape_from_last_bitmap_creation: Cell<ColorSpectrumShape>,
    components_from_last_bitmap_creation: Cell<ColorSpectrumComponents>,
    image_width_from_last_bitmap_creation: Cell<f64>,
    image_height_from_last_bitmap_creation: Cell<f64>,
    min_hue_from_last_bitmap_creation: Cell<i32>,
    max_hue_from_last_bitmap_creation: Cell<i32>,
    min_saturation_from_last_bitmap_creation: Cell<i32>,
    max_saturation_from_last_bitmap_creation: Cell<i32>,
    min_value_from_last_bitmap_creation: Cell<i32>,
    max_value_from_last_bitmap_creation: Cell<i32>,

    old_color: Cell<Color>,
    old_hsv_color: Cell<HsvColor>,
}

ferro_class!(ColorSpectrum: TemplatedControl);
ferro_impl_classes!(ColorSpectrum: StyledElementImpl, LayoutableImpl, InteractiveImpl);
ferroui_base::ferro_class_info!(ColorSpectrum { new: ColorSpectrum::new });

/// The pixel data of the spectrum, computed by the task of
/// `create_bitmaps_and_color_map`.
struct SpectrumPixelData {
    pixel_dimension: i32,
    min: Vec<u8>,
    max: Vec<u8>,
    middle1: Vec<u8>,
    middle2: Vec<u8>,
    middle3: Vec<u8>,
    middle4: Vec<u8>,
    hsv_values: Vec<Hsv>,
}

/// The buffers the pixels of the spectrum are written to: the counterpart
/// of the pooled lists the original passes to the pixel functions.
struct SpectrumPixelBuffers<'a> {
    min: &'a mut Vec<u8>,
    middle1: &'a mut Vec<u8>,
    middle2: &'a mut Vec<u8>,
    middle3: &'a mut Vec<u8>,
    middle4: &'a mut Vec<u8>,
    max: &'a mut Vec<u8>,
    hsv_values: &'a mut Vec<Hsv>,
}

/// The ranges of the components of the spectrum.
#[derive(Clone, Copy)]
struct SpectrumRanges {
    min_hue: f64,
    max_hue: f64,
    min_saturation: f64,
    max_saturation: f64,
    min_value: f64,
    max_value: f64,
}

/// Appends the BGRA bytes of `rgb` (opaque) to `data`.
fn add_pixel(data: &mut Vec<u8>, rgb: Rgb) {
    data.push((rgb.b * 255.0).round_ties_even() as u8); // b
    data.push((rgb.g * 255.0).round_ties_even() as u8); // g
    data.push((rgb.r * 255.0).round_ties_even() as u8); // r
    data.push(255); // a - ignored
}

impl FerroObjectImpl for ColorSpectrum {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == Self::color_property().as_property() {
            this.old_color.set(change.get_old_value::<Color>().unwrap_or_default());

            // If we're in the process of internally updating the color,
            // then we don't want to respond to the Color property changing.
            if !this.updating_color.get() {
                let color = this.color();

                this.updating_hsv_color.set(true);
                let new_hsv = Rgb::from_color(color).to_hsv();
                this.set_current_value(Self::hsv_color_property(), new_hsv.to_hsv_color(color.a as f64 / 255.0));
                this.updating_hsv_color.set(false);

                this.update_ellipse();
                this.update_bitmap_sources();
                this.raise_color_changed();
            }
        } else if change.property() == Self::hsv_color_property().as_property() {
            // If we're in the process of internally updating the HSV color,
            // then we don't want to respond to the HsvColor property changing.
            if !this.updating_hsv_color.get() {
                this.set_color_from_hsv_color();
            }

            this.old_hsv_color.set(change.get_old_value::<HsvColor>().unwrap_or_default());
        } else if change.property() == Self::min_hue_property().as_property()
            || change.property() == Self::max_hue_property().as_property()
        {
            let min_hue = this.min_hue();
            let max_hue = this.max_hue();

            if !(0..=359).contains(&min_hue) {
                panic!("MinHue must be between 0 and 359.");
            } else if !(0..=359).contains(&max_hue) {
                panic!("MaxHue must be between 0 and 359.");
            }

            let components = this.components();

            // If hue is one of the axes in the spectrum bitmap, then we'll need to regenerate it
            // if the maximum or minimum value has changed.
            if components != ColorSpectrumComponents::SaturationValue
                && components != ColorSpectrumComponents::ValueSaturation
            {
                this.create_bitmaps_and_color_map();
            }
        } else if change.property() == Self::min_saturation_property().as_property()
            || change.property() == Self::max_saturation_property().as_property()
        {
            let min_saturation = this.min_saturation();
            let max_saturation = this.max_saturation();

            if !(0..=100).contains(&min_saturation) {
                panic!("MinSaturation must be between 0 and 100.");
            } else if !(0..=100).contains(&max_saturation) {
                panic!("MaxSaturation must be between 0 and 100.");
            }

            let components = this.components();

            // If value is one of the axes in the spectrum bitmap, then we'll need to regenerate it
            // if the maximum or minimum value has changed.
            if components != ColorSpectrumComponents::HueValue && components != ColorSpectrumComponents::ValueHue {
                this.create_bitmaps_and_color_map();
            }
        } else if change.property() == Self::min_value_property().as_property()
            || change.property() == Self::max_value_property().as_property()
        {
            let min_value = this.min_value();
            let max_value = this.max_value();

            if !(0..=100).contains(&min_value) {
                panic!("MinValue must be between 0 and 100.");
            } else if !(0..=100).contains(&max_value) {
                panic!("MaxValue must be between 0 and 100.");
            }

            let components = this.components();

            // If value is one of the axes in the spectrum bitmap, then we'll need to regenerate it
            // if the maximum or minimum value has changed.
            if components != ColorSpectrumComponents::HueSaturation
                && components != ColorSpectrumComponents::SaturationHue
            {
                this.create_bitmaps_and_color_map();
            }
        } else if change.property() == Self::shape_property().as_property() {
            this.create_bitmaps_and_color_map();
        } else if change.property() == Self::components_property().as_property() {
            // Calculate and update the ThirdComponent value
            match this.components() {
                ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::SaturationHue => {
                    this.set_third_component(ColorComponent::Component3); // HsvComponent.Value
                }
                ColorSpectrumComponents::HueValue | ColorSpectrumComponents::ValueHue => {
                    this.set_third_component(ColorComponent::Component2); // HsvComponent.Saturation
                }
                ColorSpectrumComponents::SaturationValue | ColorSpectrumComponents::ValueSaturation => {
                    this.set_third_component(ColorComponent::Component1); // HsvComponent.Hue
                }
            }

            this.create_bitmaps_and_color_map();
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl VisualImpl for ColorSpectrum {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        // If the color was updated while this ColorSpectrum was not part of the visual tree,
        // the selection ellipse may be in an incorrect position. This is because the spectrum
        // renders based on layout scaling to avoid color banding; however, layout scale is only
        // available when the control is attached to the visual tree. The ColorSpectrum's color
        // may be updated from code-behind or from binding with another control when it's not
        // part of the visual tree.
        //
        //  See the discussion 9077 of the upstream project.
        //
        // To work-around this issue the selection ellipse is refreshed here.
        this.update_ellipse();

        // OnAttachedToVisualTree is called after OnApplyTemplate so events cannot be connected here
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl ControlImpl for ColorSpectrum {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ColorSpectrumAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for ColorSpectrum {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        this.unregister_events(); // Failsafe

        let input_target = e.name_scope().find_as::<Canvas>("PART_InputTarget");
        let layout_root = e.name_scope().find_as::<Panel>("PART_LayoutRoot");
        let selection_ellipse_panel = e.name_scope().find_as::<Panel>("PART_SelectionEllipsePanel");
        *this.input_target.borrow_mut() = input_target.clone();
        *this.layout_root.borrow_mut() = layout_root.clone();
        *this.selection_ellipse_panel.borrow_mut() = selection_ellipse_panel.clone();
        *this.sizing_panel.borrow_mut() = e.name_scope().find_as::<Panel>("PART_SizingPanel");
        *this.spectrum_ellipse.borrow_mut() = e.name_scope().find_as::<Ellipse>("PART_SpectrumEllipse");
        *this.spectrum_rectangle.borrow_mut() = e.name_scope().find_as::<Rectangle>("PART_SpectrumRectangle");
        *this.spectrum_overlay_ellipse.borrow_mut() = e.name_scope().find_as::<Ellipse>("PART_SpectrumOverlayEllipse");
        *this.spectrum_overlay_rectangle.borrow_mut() =
            e.name_scope().find_as::<Rectangle>("PART_SpectrumOverlayRectangle");

        if let Some(input_target) = &input_target {
            let weak = this.to_ref().downgrade();
            let mut disposables = Vec::new();

            let w = weak.clone();
            disposables.push(input_target.add_disposable_handler(
                InputElement::pointer_entered_event(),
                move |_, args: &PointerEventArgs| {
                    if let Some(this) = w.upgrade() {
                        this.input_target_pointer_entered(args);
                    }
                },
            ));
            let w = weak.clone();
            disposables.push(input_target.add_disposable_handler(
                InputElement::pointer_exited_event(),
                move |_, args: &PointerEventArgs| {
                    if let Some(this) = w.upgrade() {
                        this.input_target_pointer_exited(args);
                    }
                },
            ));
            let w = weak.clone();
            disposables.push(input_target.add_disposable_handler(
                InputElement::pointer_pressed_event(),
                move |_, args: &PointerPressedEventArgs| {
                    if let Some(this) = w.upgrade() {
                        this.input_target_pointer_pressed(args);
                    }
                },
            ));
            let w = weak.clone();
            disposables.push(input_target.add_disposable_handler(
                InputElement::pointer_moved_event(),
                move |_, args: &PointerEventArgs| {
                    if let Some(this) = w.upgrade() {
                        this.input_target_pointer_moved(args);
                    }
                },
            ));
            let w = weak;
            disposables.push(input_target.add_disposable_handler(
                InputElement::pointer_released_event(),
                move |_, args: &PointerReleasedEventArgs| {
                    if let Some(this) = w.upgrade() {
                        this.input_target_pointer_released(args);
                    }
                },
            ));

            *this.input_target_disposables.borrow_mut() = disposables;
        }

        if let Some(layout_root) = &layout_root {
            let weak = this.to_ref().downgrade();
            let object: &FerroObject = layout_root;
            let subscription = FerroObjectExtensions::get_observable(object, Visual::bounds_property()).subscribe_fn(
                move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.create_bitmaps_and_color_map();
                    }
                },
            );
            *this.layout_root_disposable.borrow_mut() = Some(subscription);
        }

        if let Some(selection_ellipse_panel) = &selection_ellipse_panel {
            let weak = this.to_ref().downgrade();
            let object: &FerroObject = selection_ellipse_panel;
            let subscription = FerroObjectExtensions::get_observable(object, Visual::flow_direction_property())
                .subscribe_fn(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.update_ellipse();
                    }
                });
            *this.selection_ellipse_panel_disposable.borrow_mut() = Some(subscription);
        }

        if let Some(selection_ellipse_panel) = &selection_ellipse_panel {
            if ColorHelper::to_display_name_exists() {
                ToolTip::set_tip(selection_ellipse_panel, Some(Rc::new(ColorHelper::to_display_name(this.color()))));
            }
        }

        // If we haven't yet created our bitmaps, do so now.
        if this.hsv_values.borrow().is_empty() {
            this.create_bitmaps_and_color_map();
        }

        this.update_ellipse();
        this.update_pseudo_classes();
    }
}

impl InputElementImpl for ColorSpectrum {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        let key = e.key;

        if key != Key::Left && key != Key::Right && key != Key::Up && key != Key::Down {
            Self::parent_on_key_down(this, e);
            return;
        }

        let is_control_down = e.key_modifiers.contains(KeyModifiers::CONTROL);

        let mut increment_component = HsvComponent::Hue;

        let mut is_saturation_value = false;

        if key == Key::Left || key == Key::Right {
            match this.components() {
                ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::HueValue => {
                    increment_component = HsvComponent::Hue;
                }
                ColorSpectrumComponents::SaturationValue => {
                    is_saturation_value = true;
                    increment_component = HsvComponent::Saturation;
                }
                ColorSpectrumComponents::SaturationHue => {
                    increment_component = HsvComponent::Saturation;
                }
                ColorSpectrumComponents::ValueHue | ColorSpectrumComponents::ValueSaturation => {
                    increment_component = HsvComponent::Value;
                }
            }
        } else if key == Key::Up || key == Key::Down {
            match this.components() {
                ColorSpectrumComponents::SaturationHue | ColorSpectrumComponents::ValueHue => {
                    increment_component = HsvComponent::Hue;
                }
                ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::ValueSaturation => {
                    increment_component = HsvComponent::Saturation;
                }
                ColorSpectrumComponents::SaturationValue => {
                    is_saturation_value = true;
                    increment_component = HsvComponent::Value;
                }
                ColorSpectrumComponents::HueValue => {
                    increment_component = HsvComponent::Value;
                }
            }
        }

        let mut min_bound = 0.0;
        let mut max_bound = 0.0;

        match increment_component {
            HsvComponent::Hue => {
                min_bound = this.min_hue() as f64;
                max_bound = this.max_hue() as f64;
            }
            HsvComponent::Saturation => {
                min_bound = this.min_saturation() as f64;
                max_bound = this.max_saturation() as f64;
            }
            HsvComponent::Value => {
                min_bound = this.min_value() as f64;
                max_bound = this.max_value() as f64;
            }
            HsvComponent::Alpha => {}
        }

        // The order of saturation and value in the spectrum is reversed - the max value is at the bottom while the min value is at the top -
        // so we want left and up to be lower for hue, but higher for saturation and value.
        // This will ensure that the icon always moves in the direction of the key press.
        let mut direction = if (increment_component == HsvComponent::Hue && (key == Key::Left || key == Key::Up))
            || (increment_component != HsvComponent::Hue && (key == Key::Right || key == Key::Down))
        {
            IncrementDirection::Lower
        } else {
            IncrementDirection::Higher
        };

        // Image is flipped in RightToLeft, so we need to invert direction in that case.
        // The combination saturation and value is also flipped, so we need to invert in that case too.
        // If both are false, we don't need to invert.
        // If both are true, we would invert twice, so not invert at all.
        if (this.flow_direction() == FlowDirection::RightToLeft) != is_saturation_value
            && (key == Key::Left || key == Key::Right)
        {
            if direction == IncrementDirection::Higher {
                direction = IncrementDirection::Lower;
            } else {
                direction = IncrementDirection::Higher;
            }
        }

        let amount = if is_control_down { IncrementAmount::Large } else { IncrementAmount::Small };

        let hsv_color = this.hsv_color();
        this.update_color(ColorPickerHelpers::increment_color_component(
            Hsv::from_hsv_color(hsv_color),
            increment_component,
            direction,
            amount,
            true, // shouldWrap
            min_bound,
            max_bound,
        ));

        e.set_handled(true);
    }

    fn on_got_focus(this: &Self, e: &ferroui_base::input::FocusChangedEventArgs) {
        // We only want to bother with the color name tool tip if we can provide color names.
        if let Some(selection_ellipse_panel) = this.selection_ellipse_panel() {
            if ColorHelper::to_display_name_exists() {
                ToolTip::set_is_open(&selection_ellipse_panel, true);
            }
        }

        this.update_pseudo_classes();

        Self::parent_on_got_focus(this, e);
    }

    fn on_lost_focus(this: &Self, e: &ferroui_base::input::FocusChangedEventArgs) {
        // We only want to bother with the color name tool tip if we can provide color names.
        if let Some(selection_ellipse_panel) = this.selection_ellipse_panel() {
            if ColorHelper::to_display_name_exists() {
                ToolTip::set_is_open(&selection_ellipse_panel, false);
            }
        }

        this.update_pseudo_classes();

        Self::parent_on_lost_focus(this, e);
    }

    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        // We only want to bother with the color name tool tip if we can provide color names.
        if let Some(selection_ellipse_panel) = this.selection_ellipse_panel() {
            if ColorHelper::to_display_name_exists() {
                ToolTip::set_is_open(&selection_ellipse_panel, false);
            }
        }

        this.update_pseudo_classes();

        Self::parent_on_pointer_exited(this, e);
    }
}

impl ColorSpectrum {
    /// The pseudo-class of a pressed spectrum.
    pub const PC_PRESSED: &'static str = ":pressed";
    /// The pseudo-class of a dark selection ellipse.
    pub const PC_DARK_SELECTOR: &'static str = ":dark-selector";
    /// The pseudo-class of a large selection ellipse.
    pub const PC_LARGE_SELECTOR: &'static str = ":large-selector";
    /// The pseudo-class of a light selection ellipse.
    pub const PC_LIGHT_SELECTOR: &'static str = ":light-selector";

    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] = &[
        TemplatePartAttribute::new("PART_InputTarget", <Canvas as ferroui_base::StaticType>::TYPE),
        TemplatePartAttribute::new("PART_LayoutRoot", <Panel as ferroui_base::StaticType>::TYPE),
        TemplatePartAttribute::new("PART_SelectionEllipsePanel", <Panel as ferroui_base::StaticType>::TYPE),
        TemplatePartAttribute::new("PART_SizingPanel", <Panel as ferroui_base::StaticType>::TYPE),
        TemplatePartAttribute::new("PART_SpectrumEllipse", <Ellipse as ferroui_base::StaticType>::TYPE),
        TemplatePartAttribute::new("PART_SpectrumRectangle", <Rectangle as ferroui_base::StaticType>::TYPE),
        TemplatePartAttribute::new("PART_SpectrumOverlayEllipse", <Ellipse as ferroui_base::StaticType>::TYPE),
        TemplatePartAttribute::new("PART_SpectrumOverlayRectangle", <Rectangle as ferroui_base::StaticType>::TYPE),
    ];

    /// The pseudo-classes set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[
        Self::PC_PRESSED,
        Self::PC_LARGE_SELECTOR,
        Self::PC_DARK_SELECTOR,
        Self::PC_LIGHT_SELECTOR,
    ]);

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            color_changed: HandlerList::new(),
            updating_color: Cell::new(false),
            updating_hsv_color: Cell::new(false),
            is_pointer_pressed: Cell::new(false),
            should_show_large_selection: Cell::new(false),
            hsv_values: RefCell::new(Rc::new(Vec::new())),
            third_component: Cell::new(ColorComponent::Component3), // HsvComponent.Value
            layout_root_disposable: RefCell::new(None),
            selection_ellipse_panel_disposable: RefCell::new(None),
            input_target_disposables: RefCell::new(Vec::new()),
            layout_root: RefCell::new(None),
            sizing_panel: RefCell::new(None),
            spectrum_rectangle: RefCell::new(None),
            spectrum_ellipse: RefCell::new(None),
            spectrum_overlay_rectangle: RefCell::new(None),
            spectrum_overlay_ellipse: RefCell::new(None),
            input_target: RefCell::new(None),
            selection_ellipse_panel: RefCell::new(None),
            hue_red_bitmap: RefCell::new(None),
            hue_yellow_bitmap: RefCell::new(None),
            hue_green_bitmap: RefCell::new(None),
            hue_cyan_bitmap: RefCell::new(None),
            hue_blue_bitmap: RefCell::new(None),
            hue_purple_bitmap: RefCell::new(None),
            saturation_minimum_bitmap: RefCell::new(None),
            saturation_maximum_bitmap: RefCell::new(None),
            value_bitmap: RefCell::new(None),
            min_bitmap: RefCell::new(None),
            max_bitmap: RefCell::new(None),
            shape_from_last_bitmap_creation: Cell::new(ColorSpectrumShape::Box),
            components_from_last_bitmap_creation: Cell::new(ColorSpectrumComponents::HueSaturation),
            image_width_from_last_bitmap_creation: Cell::new(0.0),
            image_height_from_last_bitmap_creation: Cell::new(0.0),
            min_hue_from_last_bitmap_creation: Cell::new(0),
            max_hue_from_last_bitmap_creation: Cell::new(0),
            min_saturation_from_last_bitmap_creation: Cell::new(0),
            max_saturation_from_last_bitmap_creation: Cell::new(0),
            min_value_from_last_bitmap_creation: Cell::new(0),
            max_value_from_last_bitmap_creation: Cell::new(0),
            old_color: Cell::new(Color::from_argb(255, 255, 255, 255)),
            old_hsv_color: Cell::new(HsvColor::from_ahsv(0.0, 0.0, 1.0, 1.0)),
        }
    }

    /// Initializes a new instance of the [`ColorSpectrum`] class.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.shape_from_last_bitmap_creation.set(this.shape());
        this.components_from_last_bitmap_creation.set(this.components());
        this.image_width_from_last_bitmap_creation.set(0.0);
        this.image_height_from_last_bitmap_creation.set(0.0);
        this.min_hue_from_last_bitmap_creation.set(this.min_hue());
        this.max_hue_from_last_bitmap_creation.set(this.max_hue());
        this.min_saturation_from_last_bitmap_creation.set(this.min_saturation());
        this.max_saturation_from_last_bitmap_creation.set(this.max_saturation());
        this.min_value_from_last_bitmap_creation.set(this.min_value());
        this.max_value_from_last_bitmap_creation.set(this.max_value());
        this
    }

    /// Event for when the selected color changes within the spectrum.
    /// Disposing the returned handle unsubscribes.
    pub fn color_changed(&self, handler: impl Fn(&ColorChangedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.color_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.color_changed.remove(token);
            }
        })
    }

    fn selection_ellipse_panel(&self) -> Option<Ref<Panel>> {
        self.selection_ellipse_panel.borrow().clone()
    }

    /// Explicitly unregisters all events connected in OnApplyTemplate().
    fn unregister_events(&self) {
        if let Some(disposable) = self.layout_root_disposable.borrow_mut().take() {
            disposable.dispose();
        }

        if let Some(disposable) = self.selection_ellipse_panel_disposable.borrow_mut().take() {
            disposable.dispose();
        }

        let disposables = std::mem::take(&mut *self.input_target_disposables.borrow_mut());
        for disposable in disposables {
            disposable.dispose();
        }
    }

    fn set_color_from_hsv_color(&self) {
        let hsv_color = self.hsv_color();

        self.updating_color.set(true);
        let new_rgb = Hsv::from_hsv_color(hsv_color).to_rgb();

        self.set_current_value(Self::color_property(), new_rgb.to_color(hsv_color.a));

        self.updating_color.set(false);

        self.update_ellipse();
        self.update_bitmap_sources();
        self.raise_color_changed();
    }

    /// Raises the color changed event when the color differs from the last
    /// one reported.
    pub fn raise_color_changed(&self) {
        let new_color = self.color();
        let old_color = self.old_color.get();

        let color_changed =
            old_color.a != new_color.a || old_color.r != new_color.r || old_color.g != new_color.g || old_color.b != new_color.b;

        let are_both_colors_black = (old_color.r == new_color.r && new_color.r == 0)
            || (old_color.g == new_color.g && new_color.g == 0)
            || (old_color.b == new_color.b && new_color.b == 0);

        if color_changed || are_both_colors_black {
            let color_changed_event_args = ColorChangedEventArgs::new(old_color, new_color);
            for (_, handler) in self.color_changed.snapshot().iter() {
                handler(&color_changed_event_args);
            }

            if let Some(selection_ellipse_panel) = self.selection_ellipse_panel() {
                if ColorHelper::to_display_name_exists() {
                    ToolTip::set_tip(&selection_ellipse_panel, Some(Rc::new(ColorHelper::to_display_name(self.color()))));
                }
            }
        }
    }

    /// Updates the visual state of the control by applying latest PseudoClasses.
    fn update_pseudo_classes(&self) {
        self.pseudo_classes().set(Self::PC_PRESSED, self.is_pointer_pressed.get());
        // Note: The ":pointerover" pseudo class is set in the base Control

        if self.is_pointer_pressed.get() {
            self.pseudo_classes().set(Self::PC_LARGE_SELECTOR, self.should_show_large_selection.get());
        } else {
            self.pseudo_classes().set(Self::PC_LARGE_SELECTOR, false);
        }

        if self.selection_ellipse_should_be_light() {
            self.pseudo_classes().set(Self::PC_DARK_SELECTOR, false);
            self.pseudo_classes().set(Self::PC_LIGHT_SELECTOR, true);
        } else {
            self.pseudo_classes().set(Self::PC_DARK_SELECTOR, true);
            self.pseudo_classes().set(Self::PC_LIGHT_SELECTOR, false);
        }
    }

    /// Changes the currently selected color (always in HSV) and applies all necessary updates.
    ///
    /// Some additional logic is applied in certain situations to coerce and sync color values.
    /// Use this method instead of update the [`color`](Self::color) or
    /// [`hsv_color`](Self::hsv_color) directly.
    fn update_color(&self, mut new_hsv: Hsv) {
        self.updating_color.set(true);
        self.updating_hsv_color.set(true);

        let mut alpha = self.hsv_color().a;

        // It is common for the ColorPicker (and therefore the Spectrum) to be initialized
        // with a #00000000 color value in some use cases. This is usually used to indicate
        // that no color has been selected by the user. Note that #00000000 is different than
        // #00FFFFFF (Transparent).
        //
        // In this situation, whenever the user clicks on the spectrum, the third
        // component and alpha component will remain zero. This is because the spectrum only
        // controls two components at any given time.
        //
        // This is very unintuitive from a user-standpoint as after the user clicks on the
        // spectrum they must then increase the alpha and then the third component sliders
        // to the desired value. In fact, until they increase these slider values no color
        // will show at all since it is fully transparent and black. In almost all cases
        // though the desired value is simply full color.
        //
        // To work around this usability issue with an initial #00000000 color, the selected
        // color is coerced into a color with maximum third component value and maximum alpha.
        // This can only happen here in the spectrum if those two components are already zero.
        //
        // In the past this coercion was restricted to occur only one time. However, when
        // ColorPicker controls are re-used or recycled #00000000 can be set multiple times.
        // Each time needs this special logic for usability so now anytime the color is
        // changed on the spectrum this logic will run.
        //
        // Also note this is NOT currently done for #00FFFFFF (Transparent) but based on
        // further usability study that case may need to be handled here as well. Right now
        // Transparent is treated as a normal color value with the alpha intentionally set
        // to zero so the alpha slider must still be adjusted after the spectrum.
        if self.is_loaded() {
            let is_alpha_component_zero = alpha == 0.0;
            let is_third_component_zero = match self.components() {
                ColorSpectrumComponents::HueValue | ColorSpectrumComponents::ValueHue => new_hsv.s == 0.0,
                ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::SaturationHue => new_hsv.v == 0.0,
                ColorSpectrumComponents::ValueSaturation | ColorSpectrumComponents::SaturationValue => new_hsv.h == 0.0,
            };

            if is_alpha_component_zero && is_third_component_zero {
                alpha = 1.0;

                match self.components() {
                    ColorSpectrumComponents::HueValue | ColorSpectrumComponents::ValueHue => {
                        new_hsv.s = 1.0;
                    }
                    ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::SaturationHue => {
                        new_hsv.v = 1.0;
                    }
                    ColorSpectrumComponents::ValueSaturation | ColorSpectrumComponents::SaturationValue => {
                        // Hue is mathematically NOT a special case; however, is one conceptually.
                        // It doesn't make sense to change the selected Hue value, so why is it set here?
                        // Setting to 360.0 is equivalent to the max set for other components and is
                        // internally wrapped back to 0.0 (since 360 degrees = 0 degrees).
                        // This means effectively there is no change to the hue component value.
                        new_hsv.h = 360.0;
                    }
                }
            }
        }

        let new_rgb = new_hsv.to_rgb();

        self.set_current_value(Self::color_property(), new_rgb.to_color(alpha));
        self.set_current_value(Self::hsv_color_property(), new_hsv.to_hsv_color(alpha));

        self.update_ellipse();
        self.update_pseudo_classes();

        self.updating_hsv_color.set(false);
        self.updating_color.set(false);

        self.raise_color_changed();
    }

    /// Updates the selected [`hsv_color`](Self::hsv_color) and
    /// [`color`](Self::color) based on a point within the color spectrum.
    fn update_color_from_point(&self, point: PointerPoint) {
        let hsv_values = self.hsv_values.borrow().clone();

        // If we haven't initialized our HSV value array yet, then we should just ignore any user input -
        // we don't yet know what to do with it.
        if hsv_values.is_empty() {
            return;
        }

        // Remember the bitmap size follows physical device pixels
        let scale = LayoutHelper::get_layout_scale(self);
        let mut x_position = point.position.x * scale;
        let mut y_position = point.position.y * scale;
        let image_width = self.image_width_from_last_bitmap_creation.get();
        let image_height = self.image_height_from_last_bitmap_creation.get();
        let radius = f64::min(image_width, image_height) / 2.0;
        let distance_from_radius = ((x_position - radius).powf(2.0) + (y_position - radius).powf(2.0)).sqrt();

        let shape = self.shape();

        // If the point is outside the circle, we should bring it back into the circle.
        if distance_from_radius > radius && shape == ColorSpectrumShape::Ring {
            x_position = (radius / distance_from_radius) * (x_position - radius) + radius;
            y_position = (radius / distance_from_radius) * (y_position - radius) + radius;
        }

        // Now we need to find the index into the array of HSL values at each point in the spectrum m_image.
        let mut x = x_position.round_ties_even() as i32;
        let mut y = y_position.round_ties_even() as i32;
        let width = image_width.round_ties_even() as i32;

        if x < 0 {
            x = 0;
        } else if x as f64 >= image_width {
            x = image_width.round_ties_even() as i32 - 1;
        }

        if y < 0 {
            y = 0;
        } else if y as f64 >= image_height {
            y = image_height.round_ties_even() as i32 - 1;
        }

        // The gradient image contains two dimensions of HSL information, but not the third.
        // We should keep the third where it already was.
        // Note: This can sometimes cause a crash -- possibly due to differences in c# rounding. Therefore, index is now clamped.
        let index = MathUtilities::clamp_i32(y * width + x, 0, hsv_values.len() as i32 - 1);
        let mut hsv_at_point = hsv_values[index as usize];

        let hsv_color = self.hsv_color();

        match self.components() {
            ColorSpectrumComponents::HueValue | ColorSpectrumComponents::ValueHue => {
                hsv_at_point.s = hsv_color.s;
            }
            ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::SaturationHue => {
                hsv_at_point.v = hsv_color.v;
            }
            ColorSpectrumComponents::ValueSaturation | ColorSpectrumComponents::SaturationValue => {
                hsv_at_point.h = hsv_color.h;
            }
        }

        self.update_color(hsv_at_point);
    }

    /// Updates the position of the selection ellipse on the spectrum which indicates the selected color.
    fn update_ellipse(&self) {
        let Some(selection_ellipse_panel) = self.selection_ellipse_panel() else {
            return;
        };

        let image_width = self.image_width_from_last_bitmap_creation.get();
        let image_height = self.image_height_from_last_bitmap_creation.get();

        // If we don't have an image size yet, we shouldn't be showing the ellipse.
        if image_width == 0.0 || image_height == 0.0 {
            selection_ellipse_panel.set_is_visible(false);
            return;
        } else {
            selection_ellipse_panel.set_is_visible(true);
        }

        let x_position: f64;
        let y_position: f64;

        let min_hue = self.min_hue_from_last_bitmap_creation.get() as f64;
        let max_hue = self.max_hue_from_last_bitmap_creation.get() as f64;
        let min_saturation = self.min_saturation_from_last_bitmap_creation.get() as f64;
        let max_saturation = self.max_saturation_from_last_bitmap_creation.get() as f64;
        let min_value = self.min_value_from_last_bitmap_creation.get() as f64;
        let max_value = self.max_value_from_last_bitmap_creation.get() as f64;
        let components = self.components_from_last_bitmap_creation.get();

        let mut hsv_color = Hsv::from_hsv_color(self.hsv_color());

        hsv_color.h = MathUtilities::clamp(hsv_color.h, min_hue, max_hue);
        hsv_color.s = MathUtilities::clamp(hsv_color.s, min_saturation / 100.0, max_saturation / 100.0);
        hsv_color.v = MathUtilities::clamp(hsv_color.v, min_value / 100.0, max_value / 100.0);

        if self.shape_from_last_bitmap_creation.get() == ColorSpectrumShape::Box {
            let x_percent: f64;
            let y_percent: f64;

            let h_percent = (hsv_color.h - min_hue) / (max_hue - min_hue);
            let mut s_percent = (hsv_color.s * 100.0 - min_saturation) / (max_saturation - min_saturation);
            let mut v_percent = (hsv_color.v * 100.0 - min_value) / (max_value - min_value);

            // In the case where saturation was an axis in the spectrum with hue, or value is an axis, full stop,
            // we inverted the direction of that axis in order to put more hue on the outside of the ring,
            // so we need to do similarly here when positioning the ellipse.
            if components == ColorSpectrumComponents::HueSaturation || components == ColorSpectrumComponents::SaturationHue {
                s_percent = 1.0 - s_percent;
            } else {
                v_percent = 1.0 - v_percent;
            }

            match components {
                ColorSpectrumComponents::HueValue => {
                    x_percent = h_percent;
                    y_percent = v_percent;
                }
                ColorSpectrumComponents::HueSaturation => {
                    x_percent = h_percent;
                    y_percent = s_percent;
                }
                ColorSpectrumComponents::ValueHue => {
                    x_percent = v_percent;
                    y_percent = h_percent;
                }
                ColorSpectrumComponents::ValueSaturation => {
                    x_percent = v_percent;
                    y_percent = s_percent;
                }
                ColorSpectrumComponents::SaturationHue => {
                    x_percent = s_percent;
                    y_percent = h_percent;
                }
                ColorSpectrumComponents::SaturationValue => {
                    x_percent = s_percent;
                    y_percent = v_percent;
                }
            }

            x_position = image_width * x_percent;
            y_position = image_height * y_percent;
        } else {
            let theta_value: f64;
            let r_value: f64;

            let h_theta_value =
                if max_hue != min_hue { 360.0 * (hsv_color.h - min_hue) / (max_hue - min_hue) } else { 0.0 };
            let mut s_theta_value = if max_saturation != min_saturation {
                360.0 * (hsv_color.s * 100.0 - min_saturation) / (max_saturation - min_saturation)
            } else {
                0.0
            };
            let mut v_theta_value = if max_value != min_value {
                360.0 * (hsv_color.v * 100.0 - min_value) / (max_value - min_value)
            } else {
                0.0
            };
            let h_r_value = if max_hue != min_hue { (hsv_color.h - min_hue) / (max_hue - min_hue) - 1.0 } else { 0.0 };
            let mut s_r_value = if max_saturation != min_saturation {
                (hsv_color.s * 100.0 - min_saturation) / (max_saturation - min_saturation) - 1.0
            } else {
                0.0
            };
            let mut v_r_value = if max_value != min_value {
                (hsv_color.v * 100.0 - min_value) / (max_value - min_value) - 1.0
            } else {
                0.0
            };

            // In the case where saturation was an axis in the spectrum with hue, or value is an axis, full stop,
            // we inverted the direction of that axis in order to put more hue on the outside of the ring,
            // so we need to do similarly here when positioning the ellipse.
            if components == ColorSpectrumComponents::HueSaturation || components == ColorSpectrumComponents::SaturationHue {
                s_theta_value = 360.0 - s_theta_value;
                s_r_value = -s_r_value - 1.0;
            } else {
                v_theta_value = 360.0 - v_theta_value;
                v_r_value = -v_r_value - 1.0;
            }

            match components {
                ColorSpectrumComponents::HueValue => {
                    theta_value = h_theta_value;
                    r_value = v_r_value;
                }
                ColorSpectrumComponents::HueSaturation => {
                    theta_value = h_theta_value;
                    r_value = s_r_value;
                }
                ColorSpectrumComponents::ValueHue => {
                    theta_value = v_theta_value;
                    r_value = h_r_value;
                }
                ColorSpectrumComponents::ValueSaturation => {
                    theta_value = v_theta_value;
                    r_value = s_r_value;
                }
                ColorSpectrumComponents::SaturationHue => {
                    theta_value = s_theta_value;
                    r_value = h_r_value;
                }
                ColorSpectrumComponents::SaturationValue => {
                    theta_value = s_theta_value;
                    r_value = v_r_value;
                }
            }

            let radius = f64::min(image_width, image_height) / 2.0;

            x_position = (((theta_value * std::f64::consts::PI / 180.0) + std::f64::consts::PI).cos() * radius * r_value)
                + radius;
            y_position = (((theta_value * std::f64::consts::PI / 180.0) + std::f64::consts::PI).sin() * radius * r_value)
                + radius;
        }

        // Remember the bitmap size follows physical device pixels
        // Warning: LayoutHelper.GetLayoutScale() doesn't work unless the control is visible
        // This will not be true in all cases if the color is updated from another control or code-behind
        let scale = LayoutHelper::get_layout_scale(self);
        Canvas::set_left(&selection_ellipse_panel, (x_position / scale) - (selection_ellipse_panel.width() / 2.0));
        Canvas::set_top(&selection_ellipse_panel, (y_position / scale) - (selection_ellipse_panel.height() / 2.0));

        // We only want to bother with the color name tool tip if we can provide color names.
        if self.is_focused() && ColorHelper::to_display_name_exists() {
            ToolTip::set_is_open(&selection_ellipse_panel, true);
        }

        self.update_pseudo_classes();
    }

    /// The handler of `PointerEntered` of the input target.
    fn input_target_pointer_entered(&self, args: &PointerEventArgs) {
        self.update_pseudo_classes();
        args.set_handled(true);
    }

    /// The handler of `PointerExited` of the input target.
    fn input_target_pointer_exited(&self, args: &PointerEventArgs) {
        self.update_pseudo_classes();
        args.set_handled(true);
    }

    /// The handler of `PointerPressed` of the input target.
    fn input_target_pointer_pressed(&self, args: &PointerPressedEventArgs) {
        let input_target: Option<Ref<Visual>> = self.input_target.borrow().clone().map(Ref::upcast);

        self.focus();

        self.is_pointer_pressed.set(true);
        // When the pen pointer type is available, `args.pointer().type_() == PointerType::Pen` is
        // also a reason for a large selection (as in the original).
        self.should_show_large_selection.set(args.pointer().type_() == PointerType::Touch);

        let capture: Option<Ref<InputElement>> = self.input_target.borrow().clone().map(Ref::upcast);
        args.pointer().capture(capture.as_ref());
        self.update_color_from_point(args.get_current_point(input_target.as_deref()));
        self.update_pseudo_classes();
        self.update_ellipse();

        args.set_handled(true);
    }

    /// The handler of `PointerMoved` of the input target.
    fn input_target_pointer_moved(&self, args: &PointerEventArgs) {
        if !self.is_pointer_pressed.get() {
            return;
        }

        let input_target: Option<Ref<Visual>> = self.input_target.borrow().clone().map(Ref::upcast);
        self.update_color_from_point(args.get_current_point(input_target.as_deref()));
        args.set_handled(true);
    }

    /// The handler of `PointerReleased` of the input target.
    fn input_target_pointer_released(&self, args: &PointerReleasedEventArgs) {
        self.is_pointer_pressed.set(false);
        self.should_show_large_selection.set(false);

        args.pointer().capture(None);
        self.update_pseudo_classes();
        self.update_ellipse();

        args.set_handled(true);
    }

    /// C# `private async void CreateBitmapsAndColorMap()`: starts the
    /// asynchronous creation of the spectrum bitmaps on the dispatcher of the
    /// thread of the control; it runs up to its first await before this
    /// returns.
    fn create_bitmaps_and_color_map(&self) {
        let this = self.to_ref();
        Dispatcher::current_dispatcher().to_task_scheduler().start_local(async move {
            Self::create_bitmaps_and_color_map_async(this).await;
        });
    }

    async fn create_bitmaps_and_color_map_async(this: Ref<Self>) {
        let (
            Some(layout_root),
            Some(sizing_panel),
            Some(input_target),
            Some(spectrum_rectangle),
            Some(spectrum_ellipse),
            Some(spectrum_overlay_rectangle),
            Some(spectrum_overlay_ellipse),
        ) = (
            this.layout_root.borrow().clone(),
            this.sizing_panel.borrow().clone(),
            this.input_target.borrow().clone(),
            this.spectrum_rectangle.borrow().clone(),
            this.spectrum_ellipse.borrow().clone(),
            this.spectrum_overlay_rectangle.borrow().clone(),
            this.spectrum_overlay_ellipse.borrow().clone(),
        )
        else {
            return;
        };

        // We want ColorSpectrum to always be a square, so we'll take the smaller of the dimensions
        // and size the sizing panel to that.
        let min_dimension = f64::min(layout_root.bounds().width, layout_root.bounds().height);

        if min_dimension == 0.0 {
            return;
        }

        sizing_panel.set_width(min_dimension);
        sizing_panel.set_height(min_dimension);
        input_target.set_width(min_dimension);
        input_target.set_height(min_dimension);
        spectrum_rectangle.set_width(min_dimension);
        spectrum_rectangle.set_height(min_dimension);
        spectrum_ellipse.set_width(min_dimension);
        spectrum_ellipse.set_height(min_dimension);
        spectrum_overlay_rectangle.set_width(min_dimension);
        spectrum_overlay_rectangle.set_height(min_dimension);
        spectrum_overlay_ellipse.set_width(min_dimension);
        spectrum_overlay_ellipse.set_height(min_dimension);

        let hsv_color = this.hsv_color();
        let min_hue = this.min_hue();
        let mut max_hue = this.max_hue();
        let min_saturation = this.min_saturation();
        let mut max_saturation = this.max_saturation();
        let min_value = this.min_value();
        let mut max_value = this.max_value();
        let shape = this.shape();
        let components = this.components();

        // If min >= max, then by convention, min is the only number that a property can have.
        if min_hue >= max_hue {
            max_hue = min_hue;
        }

        if min_saturation >= max_saturation {
            max_saturation = min_saturation;
        }

        if min_value >= max_value {
            max_value = min_value;
        }

        let hsv = Hsv::from_hsv_color(hsv_color);

        // In FerroUI, Bounds returns the actual device-independent pixel size of a control.
        // However, this is not necessarily the size of the control rendered on a display.
        // A desktop or application scaling factor may be applied which must be accounted for here.
        // Remember bitmaps in FerroUI are rendered mapping to actual device pixels, not the device-
        // independent pixels of controls.
        let scale = LayoutHelper::get_layout_scale(&this);
        let pixel_dimension = (min_dimension * scale).round_ties_even() as i32;
        let pixel_count = (pixel_dimension * pixel_dimension) as usize;
        let pixel_data_size = pixel_count * 4;
        // We'll only save pixel data for the middle bitmaps if our third dimension is hue.
        let middle_bitmaps_size = if matches!(
            components,
            ColorSpectrumComponents::ValueSaturation | ColorSpectrumComponents::SaturationValue
        ) {
            pixel_data_size
        } else {
            0
        };

        let ranges = SpectrumRanges {
            min_hue: min_hue as f64,
            max_hue: max_hue as f64,
            min_saturation: min_saturation as f64,
            max_saturation: max_saturation as f64,
            min_value: min_value as f64,
            max_value: max_value as f64,
        };

        // Deviation (DEVIATIONS.md, Colour picker): the original computes the pixels on a
        // thread-pool thread (`Task.Run`); the port has no thread pool (the browser has no
        // threads), so they are computed in a job of the UI thread at background priority.
        let task = Dispatcher::current_dispatcher().invoke_async_local_with_priority(
            move || {
                let mut data = SpectrumPixelData {
                    pixel_dimension,
                    min: Vec::with_capacity(pixel_data_size),
                    max: Vec::with_capacity(pixel_data_size),
                    // The middle 4 are only needed and used in the case of hue as the third dimension.
                    // Saturation and luminosity need only a min and max.
                    middle1: Vec::with_capacity(middle_bitmaps_size),
                    middle2: Vec::with_capacity(middle_bitmaps_size),
                    middle3: Vec::with_capacity(middle_bitmaps_size),
                    middle4: Vec::with_capacity(middle_bitmaps_size),
                    hsv_values: Vec::with_capacity(pixel_count),
                };
                let mut buffers = SpectrumPixelBuffers {
                    min: &mut data.min,
                    middle1: &mut data.middle1,
                    middle2: &mut data.middle2,
                    middle3: &mut data.middle3,
                    middle4: &mut data.middle4,
                    max: &mut data.max,
                    hsv_values: &mut data.hsv_values,
                };

                // As the user perceives it, every time the third dimension not represented in the ColorSpectrum changes,
                // the ColorSpectrum will visually change to accommodate that value.  For example, if the ColorSpectrum handles hue and luminosity,
                // and the saturation externally goes from 1.0 to 0.5, then the ColorSpectrum will visually change to look more washed out
                // to represent that third dimension's new value.
                // Internally, however, we don't want to regenerate the ColorSpectrum bitmap every single time this happens, since that's very expensive.
                // In order to make it so that we don't have to, we implement an optimization where, rather than having only one bitmap,
                // we instead have multiple that we blend together using opacity to create the effect that we want.
                // In the case where the third dimension is saturation or luminosity, we only need two: one bitmap at the minimum value
                // of the third dimension, and one bitmap at the maximum.  Then we set the second's opacity at whatever the value of
                // the third dimension is - e.g., a saturation of 0.5 implies an opacity of 50%.
                // In the case where the third dimension is hue, we need six: one bitmap corresponding to red, yellow, green, cyan, blue, and purple.
                // We'll then blend between whichever colors our hue exists between - e.g., an orange color would use red and yellow with an opacity of 50%.
                // This optimization does incur slightly more startup time initially since we have to generate multiple bitmaps at once instead of only one,
                // but the running time savings after that are *huge* when we can just set an opacity instead of generating a brand new bitmap.
                if shape == ColorSpectrumShape::Box {
                    for x in (0..pixel_dimension).rev() {
                        for y in (0..pixel_dimension).rev() {
                            Self::fill_pixel_for_box(
                                x as f64,
                                y as f64,
                                hsv,
                                pixel_dimension,
                                components,
                                ranges,
                                &mut buffers,
                            );
                        }
                    }
                } else {
                    for y in 0..pixel_dimension {
                        for x in 0..pixel_dimension {
                            Self::fill_pixel_for_ring(
                                x as f64,
                                y as f64,
                                pixel_dimension as f64 / 2.0,
                                hsv,
                                components,
                                ranges,
                                &mut buffers,
                            );
                        }
                    }
                }

                data
            },
            DispatcherPriority::BACKGROUND,
        );

        // A canceled job (the dispatcher shut down) ends the method, as an abandoned task.
        let Ok(data) = task.await else {
            return;
        };

        // `Dispatcher.UIThread.InvokeAsync` of the original: the dispatcher of the thread of the
        // control, which is the UI thread.
        let apply = Dispatcher::current_dispatcher().invoke_async_local(move || this.apply_bitmaps(data));
        let _ = apply.await;
    }

    /// The body of the `Dispatcher.UIThread.InvokeAsync` call of
    /// `CreateBitmapsAndColorMap`: creates the bitmaps from the pixel data
    /// and applies them.
    fn apply_bitmaps(&self, data: SpectrumPixelData) {
        let pixel_width = data.pixel_dimension;
        let pixel_height = data.pixel_dimension;

        let components2 = self.components();

        let create = |pixels: &[u8]| Rc::new(ColorPickerHelpers::create_bitmap_from_pixel_data(pixels, pixel_width, pixel_height));
        let replace = |field: &RefCell<Option<Rc<Bitmap>>>, bitmap: Rc<Bitmap>| {
            let previous = field.replace(Some(bitmap));
            if let Some(previous) = previous {
                previous.dispose();
            }
        };

        let min_bitmap = create(&data.min);
        let max_bitmap = create(&data.max);
        *self.min_bitmap.borrow_mut() = Some(min_bitmap.clone());
        *self.max_bitmap.borrow_mut() = Some(max_bitmap.clone());

        match components2 {
            ColorSpectrumComponents::HueValue | ColorSpectrumComponents::ValueHue => {
                replace(&self.saturation_minimum_bitmap, min_bitmap);
                replace(&self.saturation_maximum_bitmap, max_bitmap);
            }
            ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::SaturationHue => {
                replace(&self.value_bitmap, max_bitmap);
            }
            ColorSpectrumComponents::ValueSaturation | ColorSpectrumComponents::SaturationValue => {
                replace(&self.hue_red_bitmap, min_bitmap);
                replace(&self.hue_yellow_bitmap, create(&data.middle1));
                replace(&self.hue_green_bitmap, create(&data.middle2));
                replace(&self.hue_cyan_bitmap, create(&data.middle3));
                replace(&self.hue_blue_bitmap, create(&data.middle4));
                replace(&self.hue_purple_bitmap, max_bitmap);
            }
        }

        self.shape_from_last_bitmap_creation.set(self.shape());
        self.components_from_last_bitmap_creation.set(self.components());
        self.image_width_from_last_bitmap_creation.set(data.pixel_dimension as f64);
        self.image_height_from_last_bitmap_creation.set(data.pixel_dimension as f64);
        self.min_hue_from_last_bitmap_creation.set(self.min_hue());
        self.max_hue_from_last_bitmap_creation.set(self.max_hue());
        self.min_saturation_from_last_bitmap_creation.set(self.min_saturation());
        self.max_saturation_from_last_bitmap_creation.set(self.max_saturation());
        self.min_value_from_last_bitmap_creation.set(self.min_value());
        self.max_value_from_last_bitmap_creation.set(self.max_value());

        *self.hsv_values.borrow_mut() = Rc::new(data.hsv_values);

        self.update_bitmap_sources();
        self.update_ellipse();
    }

    /// Writes the pixel at (`x`, `y`) of every bitmap of a box spectrum.
    fn fill_pixel_for_box(
        x: f64,
        y: f64,
        base_hsv: Hsv,
        min_dimension: i32,
        components: ColorSpectrumComponents,
        ranges: SpectrumRanges,
        buffers: &mut SpectrumPixelBuffers<'_>,
    ) {
        let h_min = ranges.min_hue;
        let h_max = ranges.max_hue;
        let s_min = ranges.min_saturation / 100.0;
        let s_max = ranges.max_saturation / 100.0;
        let v_min = ranges.min_value / 100.0;
        let v_max = ranges.max_value / 100.0;

        let mut hsv_min = base_hsv;
        let mut hsv_middle1 = base_hsv;
        let mut hsv_middle2 = base_hsv;
        let mut hsv_middle3 = base_hsv;
        let mut hsv_middle4 = base_hsv;
        let mut hsv_max = base_hsv;

        let x_percent = (min_dimension as f64 - 1.0 - x) / (min_dimension as f64 - 1.0);
        let y_percent = (min_dimension as f64 - 1.0 - y) / (min_dimension as f64 - 1.0);

        Self::assign_components(
            components,
            x_percent,
            y_percent,
            (h_min, h_max, s_min, s_max, v_min, v_max),
            [&mut hsv_min, &mut hsv_middle1, &mut hsv_middle2, &mut hsv_middle3, &mut hsv_middle4, &mut hsv_max],
        );

        Self::finish_pixel(
            components,
            (s_min, s_max, v_min, v_max),
            [hsv_min, hsv_middle1, hsv_middle2, hsv_middle3, hsv_middle4, hsv_max],
            buffers,
        );
    }

    /// The `switch (components)` the two pixel functions of the original
    /// share: assigns the two components of the spectrum and the minimum and
    /// maximum of the third to the six colors of a pixel. The first
    /// component of `components` is taken at `y_fraction` of its range and
    /// the second at `x_fraction`: the box passes its `xPercent` and
    /// `yPercent`, the ring its radius `r` and its `thetaPercent`.
    fn assign_components(
        components: ColorSpectrumComponents,
        x_fraction: f64,
        y_fraction: f64,
        (h_min, h_max, s_min, s_max, v_min, v_max): (f64, f64, f64, f64, f64, f64),
        hsvs: [&mut Hsv; 6],
    ) {
        let [hsv_min, hsv_middle1, hsv_middle2, hsv_middle3, hsv_middle4, hsv_max] = hsvs;
        let mut all = |set: &dyn Fn(&mut Hsv)| {
            set(hsv_min);
            set(hsv_middle1);
            set(hsv_middle2);
            set(hsv_middle3);
            set(hsv_middle4);
            set(hsv_max);
        };

        match components {
            ColorSpectrumComponents::HueValue => {
                all(&|hsv| hsv.h = h_min + y_fraction * (h_max - h_min));
                all(&|hsv| hsv.v = v_min + x_fraction * (v_max - v_min));
            }
            ColorSpectrumComponents::HueSaturation => {
                all(&|hsv| hsv.h = h_min + y_fraction * (h_max - h_min));
                all(&|hsv| hsv.s = s_min + x_fraction * (s_max - s_min));
            }
            ColorSpectrumComponents::ValueHue => {
                all(&|hsv| hsv.v = v_min + y_fraction * (v_max - v_min));
                all(&|hsv| hsv.h = h_min + x_fraction * (h_max - h_min));
            }
            ColorSpectrumComponents::ValueSaturation => {
                all(&|hsv| hsv.v = v_min + y_fraction * (v_max - v_min));
                all(&|hsv| hsv.s = s_min + x_fraction * (s_max - s_min));
            }
            ColorSpectrumComponents::SaturationHue => {
                all(&|hsv| hsv.s = s_min + y_fraction * (s_max - s_min));
                all(&|hsv| hsv.h = h_min + x_fraction * (h_max - h_min));
            }
            ColorSpectrumComponents::SaturationValue => {
                all(&|hsv| hsv.s = s_min + y_fraction * (s_max - s_min));
                all(&|hsv| hsv.v = v_min + x_fraction * (v_max - v_min));
            }
        }

        match components {
            ColorSpectrumComponents::HueValue | ColorSpectrumComponents::ValueHue => {
                hsv_min.s = 0.0;
                hsv_max.s = 1.0;
            }
            ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::SaturationHue => {
                hsv_min.v = 0.0;
                hsv_max.v = 1.0;
            }
            ColorSpectrumComponents::ValueSaturation | ColorSpectrumComponents::SaturationValue => {
                hsv_min.h = 0.0;
                hsv_middle1.h = 60.0;
                hsv_middle2.h = 120.0;
                hsv_middle3.h = 180.0;
                hsv_middle4.h = 240.0;
                hsv_max.h = 300.0;
            }
        }
    }

    /// The end of the two pixel functions of the original: inverts the
    /// saturation or value axis and appends the pixel to the buffers.
    fn finish_pixel(
        components: ColorSpectrumComponents,
        (s_min, s_max, v_min, v_max): (f64, f64, f64, f64),
        mut hsvs: [Hsv; 6],
        buffers: &mut SpectrumPixelBuffers<'_>,
    ) {
        // If saturation is an axis in the spectrum with hue, or value is an axis, then we want
        // that axis to go from maximum at the top to minimum at the bottom,
        // or maximum at the outside to minimum at the inside in the case of the ring configuration,
        // so we'll invert the number before assigning the HSL value to the array.
        // Otherwise, we'll have a very narrow section in the middle that actually has meaningful hue
        // in the case of the ring configuration.
        if components == ColorSpectrumComponents::HueSaturation || components == ColorSpectrumComponents::SaturationHue {
            for hsv in &mut hsvs {
                hsv.s = s_max - hsv.s + s_min;
            }
        } else {
            for hsv in &mut hsvs {
                hsv.v = v_max - hsv.v + v_min;
            }
        }

        let [hsv_min, hsv_middle1, hsv_middle2, hsv_middle3, hsv_middle4, hsv_max] = hsvs;

        buffers.hsv_values.push(hsv_min);

        add_pixel(buffers.min, hsv_min.to_rgb());

        // We'll only save pixel data for the middle bitmaps if our third dimension is hue.
        if components == ColorSpectrumComponents::ValueSaturation || components == ColorSpectrumComponents::SaturationValue {
            add_pixel(buffers.middle1, hsv_middle1.to_rgb());
            add_pixel(buffers.middle2, hsv_middle2.to_rgb());
            add_pixel(buffers.middle3, hsv_middle3.to_rgb());
            add_pixel(buffers.middle4, hsv_middle4.to_rgb());
        }

        add_pixel(buffers.max, hsv_max.to_rgb());
    }

    /// Writes the pixel at (`x`, `y`) of every bitmap of a ring spectrum.
    fn fill_pixel_for_ring(
        x: f64,
        y: f64,
        radius: f64,
        base_hsv: Hsv,
        components: ColorSpectrumComponents,
        ranges: SpectrumRanges,
        buffers: &mut SpectrumPixelBuffers<'_>,
    ) {
        let h_min = ranges.min_hue;
        let h_max = ranges.max_hue;
        let s_min = ranges.min_saturation / 100.0;
        let s_max = ranges.max_saturation / 100.0;
        let v_min = ranges.min_value / 100.0;
        let v_max = ranges.max_value / 100.0;

        let mut distance_from_radius = ((x - radius).powf(2.0) + (y - radius).powf(2.0)).sqrt();

        let mut x_to_use = x;
        let mut y_to_use = y;

        // If we're outside the ring, then we want the pixel to appear as blank.
        // However, to avoid issues with rounding errors, we'll act as though this point
        // is on the edge of the ring for the purposes of returning an HSL value.
        // That way, hit testing on the edges will always return the correct value.
        if distance_from_radius > radius {
            x_to_use = (radius / distance_from_radius) * (x - radius) + radius;
            y_to_use = (radius / distance_from_radius) * (y - radius) + radius;
            distance_from_radius = radius;
        }

        let mut hsv_min = base_hsv;
        let mut hsv_middle1 = base_hsv;
        let mut hsv_middle2 = base_hsv;
        let mut hsv_middle3 = base_hsv;
        let mut hsv_middle4 = base_hsv;
        let mut hsv_max = base_hsv;

        let r = 1.0 - distance_from_radius / radius;

        let mut theta = (radius - y_to_use).atan2(radius - x_to_use) * 180.0 / std::f64::consts::PI;
        theta += 180.0;
        theta = theta.floor();

        while theta > 360.0 {
            theta -= 360.0;
        }

        let theta_percent = theta / 360.0;

        Self::assign_components(
            components,
            r,
            theta_percent,
            (h_min, h_max, s_min, s_max, v_min, v_max),
            [&mut hsv_min, &mut hsv_middle1, &mut hsv_middle2, &mut hsv_middle3, &mut hsv_middle4, &mut hsv_max],
        );

        Self::finish_pixel(
            components,
            (s_min, s_max, v_min, v_max),
            [hsv_min, hsv_middle1, hsv_middle2, hsv_middle3, hsv_middle4, hsv_max],
            buffers,
        );
    }

    fn update_bitmap_sources(&self) {
        let (
            Some(spectrum_overlay_rectangle),
            Some(spectrum_overlay_ellipse),
            Some(spectrum_rectangle),
            Some(spectrum_ellipse),
        ) = (
            self.spectrum_overlay_rectangle.borrow().clone(),
            self.spectrum_overlay_ellipse.borrow().clone(),
            self.spectrum_rectangle.borrow().clone(),
            self.spectrum_ellipse.borrow().clone(),
        )
        else {
            return;
        };

        let hsv_color = self.hsv_color();
        let components = self.components();

        let image_brush = |bitmap: &Rc<Bitmap>| -> Rc<dyn IBrush> {
            let source: Rc<dyn IImageBrushSource> = bitmap.clone();
            ImageBrush::with_source(Some(source)).into()
        };

        // We'll set the base image and the overlay image based on which component is our third dimension.
        // If it's saturation or luminosity, then the base image is that dimension at its minimum value,
        // while the overlay image is that dimension at its maximum value.
        // If it's hue, then we'll figure out where in the color wheel we are, and then use the two
        // colors on either side of our position as our base image and overlay image.
        // For example, if our hue is orange, then the base image would be red and the overlay image yellow.
        //
        // As in the original, the overlay brush is assigned to the overlay rectangle twice and
        // never to the overlay ellipse.
        match components {
            ColorSpectrumComponents::HueValue | ColorSpectrumComponents::ValueHue => {
                let (Some(saturation_minimum_bitmap), Some(saturation_maximum_bitmap)) =
                    (self.saturation_minimum_bitmap.borrow().clone(), self.saturation_maximum_bitmap.borrow().clone())
                else {
                    return;
                };

                let spectrum_brush = image_brush(&saturation_minimum_bitmap);
                let spectrum_overlay_brush = image_brush(&saturation_maximum_bitmap);

                spectrum_overlay_rectangle.set_opacity(hsv_color.s);
                spectrum_overlay_ellipse.set_opacity(hsv_color.s);
                spectrum_rectangle.set_fill(Some(spectrum_brush.clone()));
                spectrum_ellipse.set_fill(Some(spectrum_brush));
                spectrum_overlay_rectangle.set_fill(Some(spectrum_overlay_brush.clone()));
                spectrum_overlay_rectangle.set_fill(Some(spectrum_overlay_brush));
            }
            ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::SaturationHue => {
                let Some(value_bitmap) = self.value_bitmap.borrow().clone() else {
                    return;
                };

                let spectrum_brush = image_brush(&value_bitmap);
                let spectrum_overlay_brush = image_brush(&value_bitmap);

                spectrum_overlay_rectangle.set_opacity(1.0);
                spectrum_overlay_ellipse.set_opacity(1.0);
                spectrum_rectangle.set_fill(Some(spectrum_brush.clone()));
                spectrum_ellipse.set_fill(Some(spectrum_brush));
                spectrum_overlay_rectangle.set_fill(Some(spectrum_overlay_brush.clone()));
                spectrum_overlay_rectangle.set_fill(Some(spectrum_overlay_brush));
            }
            ColorSpectrumComponents::ValueSaturation | ColorSpectrumComponents::SaturationValue => {
                let (Some(red), Some(yellow), Some(green), Some(cyan), Some(blue), Some(purple)) = (
                    self.hue_red_bitmap.borrow().clone(),
                    self.hue_yellow_bitmap.borrow().clone(),
                    self.hue_green_bitmap.borrow().clone(),
                    self.hue_cyan_bitmap.borrow().clone(),
                    self.hue_blue_bitmap.borrow().clone(),
                    self.hue_purple_bitmap.borrow().clone(),
                ) else {
                    return;
                };

                let spectrum_brush: Rc<dyn IBrush>;
                let spectrum_overlay_brush: Rc<dyn IBrush>;

                let sextant = hsv_color.h / 60.0;

                if sextant < 1.0 {
                    spectrum_brush = image_brush(&red);
                    spectrum_overlay_brush = image_brush(&yellow);
                } else if (1.0..2.0).contains(&sextant) {
                    spectrum_brush = image_brush(&yellow);
                    spectrum_overlay_brush = image_brush(&green);
                } else if (2.0..3.0).contains(&sextant) {
                    spectrum_brush = image_brush(&green);
                    spectrum_overlay_brush = image_brush(&cyan);
                } else if (3.0..4.0).contains(&sextant) {
                    spectrum_brush = image_brush(&cyan);
                    spectrum_overlay_brush = image_brush(&blue);
                } else if (4.0..5.0).contains(&sextant) {
                    spectrum_brush = image_brush(&blue);
                    spectrum_overlay_brush = image_brush(&purple);
                } else {
                    spectrum_brush = image_brush(&purple);
                    spectrum_overlay_brush = image_brush(&red);
                }

                spectrum_overlay_rectangle.set_opacity(sextant - sextant.trunc());
                spectrum_overlay_ellipse.set_opacity(sextant - sextant.trunc());
                spectrum_rectangle.set_fill(Some(spectrum_brush.clone()));
                spectrum_ellipse.set_fill(Some(spectrum_brush));
                spectrum_overlay_rectangle.set_fill(Some(spectrum_overlay_brush.clone()));
                spectrum_overlay_rectangle.set_fill(Some(spectrum_overlay_brush));
            }
        }
    }

    /// Determines whether the selection ellipse should be light based on the relative
    /// luminance of the selected color.
    fn selection_ellipse_should_be_light(&self) -> bool {
        // The selection ellipse should be light if and only if the chosen color
        // contrasts more with black than it does with white.
        // To find how much something contrasts with white, we use the equation
        // for relative luminance.
        //
        // If the third component is value, then we won't be updating the spectrum's displayed colors,
        // so in that case we should use a value of 1 when considering the backdrop
        // for the selection ellipse.
        let displayed_color: Color;

        if self.components() == ColorSpectrumComponents::HueSaturation
            || self.components() == ColorSpectrumComponents::SaturationHue
        {
            let hsv_color = self.hsv_color();
            let color = Hsv::new(hsv_color.h, hsv_color.s, 1.0).to_rgb();
            displayed_color = color.to_color(hsv_color.a);
        } else {
            displayed_color = self.color();
        }

        let lum = ColorHelper::get_relative_luminance(displayed_color);

        lum <= 0.5
    }
}
