// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::internal::PanelNavigationHelper;
use crate::splitter_preview_adorner::SplitterPreviewAdorner;
use crate::ProportionalStackPanel;
use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl, Orientation};
use ferroui_base::media::IBrush;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    AttachedProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Point, Ref, Size, StyledElement, StyledElementImpl,
    StyledProperty, StyledPropertyOptions, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::metadata::PseudoClassesAttribute;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{AdornerLayer, TemplatedControlImpl, Thumb, ThumbImpl};
use ferroui_controls::{Control, ControlImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
/// Native Thumb control for resizing adjacent panes of a proportional panel.
#[repr(C)]
pub struct ProportionalStackPanelSplitter {
    base: Thumb,
    start_point: Cell<Point>,
    is_moving: Cell<bool>,
    preview_adorner: RefCell<Option<Ref<SplitterPreviewAdorner>>>,
    adorner_layer: RefCell<Option<Ref<AdornerLayer>>>,
    start_offset: Cell<f64>,
}
ferro_class!(ProportionalStackPanelSplitter:Thumb);
ferro_class_info!(ProportionalStackPanelSplitter {
    new: ProportionalStackPanelSplitter::new
});
ferro_impl_classes!(ProportionalStackPanelSplitter:StyledElementImpl,InteractiveImpl,ControlImpl,TemplatedControlImpl,ThumbImpl);
ferro_properties! {
    impl ProportionalStackPanelSplitter {
        /// Identifies the splitter thickness, default four.
        pub fn thickness_property()->StyledProperty<f64> {FerroProperty::register::<ProportionalStackPanelSplitter,_>("Thickness",4.0)}
        /// Identifies whether resizing is enabled, default true.
        pub fn is_resizing_enabled_property()->StyledProperty<bool> {FerroProperty::register::<ProportionalStackPanelSplitter,_>("IsResizingEnabled",true)}
        /// Identifies whether dragging previews before committing, default false.
        pub fn preview_resize_property()->StyledProperty<bool> {FerroProperty::register::<ProportionalStackPanelSplitter,_>("PreviewResize",false)}
        /// Identifies the optional preview brush override.
        pub fn preview_background_property()->StyledProperty<Option<Rc<dyn IBrush>>> {FerroProperty::register::<ProportionalStackPanelSplitter,_>("PreviewBackground",None)}
        /// Identifies preview opacity, default one.
        pub fn preview_opacity_property()->StyledProperty<f64> {FerroProperty::register::<ProportionalStackPanelSplitter,_>("PreviewOpacity",1.0)}
        /// Identifies the inherited minimum pane length, default 75.
        pub fn minimum_proportion_size_property()->AttachedProperty<f64> {FerroProperty::register_attached_with::<ProportionalStackPanelSplitter,Control,_>("MinimumProportionSize",StyledPropertyOptions::new(75.0).inherits(true))}
    }
}
impl ProportionalStackPanelSplitter {
    /// Source pseudoclass metadata; orientation classes accumulate as in source.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute =
        PseudoClassesAttribute::new(&[":horizontal", ":vertical", ":preview"]);
    fn static_constructor() {
        Layoutable::affects_measure::<Self>(&[Self::thickness_property().as_property()]);
    }
    /// Initializes fields for this splitter or a deriving control.
    pub fn construct() -> Self {
        Self {
            base: Thumb::construct(),
            start_point: Cell::new(Point::default()),
            is_moving: Cell::new(false),
            preview_adorner: RefCell::new(None),
            adorner_layer: RefCell::new(None),
            start_offset: Cell::new(0.0),
        }
    }
    /// Creates a splitter with source defaults.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets the splitter's thickness along the split axis.
    pub fn thickness(&self) -> f64 {
        self.get_value(Self::thickness_property())
    }
    /// Sets splitter thickness.
    pub fn set_thickness(&self, value: f64) {
        self.set_value(Self::thickness_property(), value);
    }
    /// Gets whether native resizing input is enabled.
    pub fn is_resizing_enabled(&self) -> bool {
        self.get_value(Self::is_resizing_enabled_property())
    }
    /// Sets whether native resizing input is enabled.
    pub fn set_is_resizing_enabled(&self, value: bool) {
        self.set_value(Self::is_resizing_enabled_property(), value);
    }
    /// Gets whether changes preview until release.
    pub fn preview_resize(&self) -> bool {
        self.get_value(Self::preview_resize_property())
    }
    /// Sets whether changes preview until release.
    pub fn set_preview_resize(&self, value: bool) {
        self.set_value(Self::preview_resize_property(), value);
    }
    /// Gets the optional preview brush.
    pub fn preview_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::preview_background_property())
    }
    /// Sets the optional preview brush.
    pub fn set_preview_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::preview_background_property(), value);
    }
    /// Gets the preview opacity.
    pub fn preview_opacity(&self) -> f64 {
        self.get_value(Self::preview_opacity_property())
    }
    /// Sets the preview opacity.
    pub fn set_preview_opacity(&self, value: f64) {
        self.set_value(Self::preview_opacity_property(), value);
    }
    /// Gets the attached inherited minimum pane length.
    pub fn get_minimum_proportion_size(control: &FerroObject) -> f64 {
        control.get_value(Self::minimum_proportion_size_property())
    }
    /// Sets the attached inherited minimum pane length.
    pub fn set_minimum_proportion_size(control: &FerroObject, value: f64) {
        control.set_value(Self::minimum_proportion_size_property(), value);
    }
    fn current_preview(&self) -> Ref<SplitterPreviewAdorner> {
        self.preview_adorner
            .borrow()
            .clone()
            .expect("live preview adorner")
    }
    fn update_preview_pseudo_class(&self) {
        self.pseudo_classes()
            .set(":preview", self.preview_resize() && self.is_moving.get());
    }
    pub(crate) fn is_splitter(control: Option<&Ref<Control>>) -> Option<Ref<Self>> {
        let control = control?;
        if let Some(presenter) = control.cast::<ContentPresenter>() {
            if presenter.child().is_none() {
                presenter.update_child();
            }
            if let Some(splitter) = presenter.child().and_then(|child| child.cast::<Self>()) {
                return Some(splitter);
            }
        }
        control.cast()
    }
    pub(crate) fn update_visual_state(&self) {
        if let Some(panel) = self.get_panel() {
            PanelNavigationHelper::update_control_visual_state(
                self,
                &panel,
                self.thickness(),
                self.is_resizing_enabled(),
            );
            self.pseudo_classes()
                .add_pseudo(if panel.orientation() == Orientation::Vertical {
                    ":vertical"
                } else {
                    ":horizontal"
                });
        }
    }
    fn get_panel(&self) -> Option<Ref<ProportionalStackPanel>> {
        PanelNavigationHelper::get_parent_panel(self)
    }
    fn find_next_child(&self, panel: &ProportionalStackPanel) -> Option<Ref<Control>> {
        PanelNavigationHelper::get_sibling_in_direction(self, panel, 1)
    }
    fn get_target_element(&self, panel: &ProportionalStackPanel) -> Option<Ref<Control>> {
        PanelNavigationHelper::get_sibling_in_direction(self, panel, -1)
    }
    fn set_target_proportion(&self, delta: f64) {
        let Some(panel) = self.get_panel() else {
            return;
        };
        let Some(target) = self.get_target_element(&panel) else {
            return;
        };
        let Some(neighbor) = self.find_next_child(&panel) else {
            return;
        };
        panel.resize_proportions(
            target,
            neighbor,
            delta,
            self.get_value(Self::minimum_proportion_size_property()),
        );
    }
}
impl FerroObjectImpl for ProportionalStackPanelSplitter {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::is_resizing_enabled_property().as_property()
            || property == Self::thickness_property().as_property()
        {
            this.update_visual_state();
        }
        if property == Self::preview_resize_property().as_property() {
            this.update_preview_pseudo_class();
        }
        let has_preview = this.preview_adorner.borrow().is_some();
        if has_preview
            && (property == Self::preview_background_property().as_property()
                || property == Self::preview_opacity_property().as_property())
        {
            this.current_preview()
                .set_background(this.preview_background());
            this.current_preview().set_opacity(this.preview_opacity());
            this.current_preview().invalidate_visual();
        }
    }
}
impl InputElementImpl for ProportionalStackPanelSplitter {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if this.is_resizing_enabled() {
            if let Some(panel) = this.get_panel() {
                this.start_point.set(e.get_position(Some(&panel)));
                this.is_moving.set(true);
                this.update_preview_pseudo_class();
                if this.preview_resize() {
                    if let Some(pos) = this.translate_point(Point::default(), &panel) {
                        let layer = AdornerLayer::get_adorner_layer(&panel);
                        this.adorner_layer.replace(layer.clone());
                        if layer.is_some() {
                            this.start_offset.set(
                                if panel.orientation() == Orientation::Vertical {
                                    pos.y
                                } else {
                                    pos.x
                                },
                            );
                            let adorner = SplitterPreviewAdorner::new();
                            adorner.set_orientation(panel.orientation());
                            adorner.set_thickness(this.thickness());
                            adorner.set_offset(this.start_offset.get());
                            adorner.set_background(this.preview_background());
                            adorner.set_opacity(this.preview_opacity());
                            AdornerLayer::set_adorned_element(&adorner, panel.clone());
                            this.preview_adorner.replace(Some(adorner));
                            this.current_preview().set_parent(panel.clone());
                            let layer = this
                                .adorner_layer
                                .borrow()
                                .clone()
                                .expect("live preview layer");
                            layer.children().add(this.current_preview());
                        }
                    }
                }
            }
        }
    }
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if this.is_moving.get() && this.is_resizing_enabled() {
            if let Some(panel) = this.get_panel() {
                let point = e.get_position(Some(&panel));
                if this.preview_resize() {
                    let delta = point - this.start_point.get();
                    this.set_target_proportion(if panel.orientation() == Orientation::Vertical {
                        delta.y
                    } else {
                        delta.x
                    });
                    let adorner = this.preview_adorner.borrow().clone();
                    let layer = this.adorner_layer.borrow().clone();
                    if let (Some(adorner), Some(layer)) = (adorner, layer) {
                        layer.children().remove(adorner.clone());
                        this.current_preview()
                            .set_parent(None::<Ref<StyledElement>>);
                        this.preview_adorner.replace(None);
                        this.adorner_layer.replace(None);
                    }
                }
            }
        }
        this.is_moving.set(false);
        this.update_preview_pseudo_class();
    }
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        if this.is_moving.get() && this.is_resizing_enabled() {
            if let Some(panel) = this.get_panel() {
                let point = e.get_position(Some(&panel));
                let delta = point - this.start_point.get();
                if this.preview_resize() {
                    let adorner = this.preview_adorner.borrow().clone();
                    if let Some(adorner) = adorner {
                        adorner.set_offset(
                            this.start_offset.get()
                                + if panel.orientation() == Orientation::Vertical {
                                    delta.y
                                } else {
                                    delta.x
                                },
                        );
                        adorner.invalidate_visual();
                    }
                } else {
                    this.start_point.set(point);
                    this.set_target_proportion(if panel.orientation() == Orientation::Vertical {
                        delta.y
                    } else {
                        delta.x
                    });
                }
            }
        }
    }
    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);
        let adorner = this.preview_adorner.borrow().clone();
        let layer = this.adorner_layer.borrow().clone();
        if let (Some(adorner), Some(layer)) = (adorner, layer) {
            layer.children().remove(adorner.clone());
            this.current_preview()
                .set_parent(None::<Ref<StyledElement>>);
            this.preview_adorner.replace(None);
            this.adorner_layer.replace(None);
        }
        this.is_moving.set(false);
        this.update_preview_pseudo_class();
    }
}
impl LayoutableImpl for ProportionalStackPanelSplitter {
    fn measure_override(this: &Self, _available: Size) -> Size {
        match this.get_panel() {
            Some(panel) => {
                if panel.orientation() == Orientation::Vertical {
                    Size::new(0.0, this.thickness())
                } else {
                    Size::new(this.thickness(), 0.0)
                }
            }
            None => Size::default(),
        }
    }
}
impl VisualImpl for ProportionalStackPanelSplitter {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        if this.get_panel().is_none() {
            return;
        }
        this.update_visual_state();
        this.update_preview_pseudo_class();
    }
}

#[cfg(test)]
#[path = "splitter_constraints_tests.rs"]
mod constraints_tests;

#[cfg(test)]
#[path = "splitter_headless_tests.rs"]
mod headless_tests;
