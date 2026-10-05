use crate::templates::{FuncTemplate, IDataTemplate, TemplateRef};
use crate::Control;
use ferroui_base::data::BindingPriority;
use ferroui_base::layout::Layoutable;
use ferroui_base::styling::IStyle;
use ferroui_base::{
    ferro_property, AttachedProperty, BoxedValue, FerroObject, FerroObjectExtensions, FerroProperty, Ref, StyledElement, Visual,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// A template that creates the control a design-time preview is hosted in.
pub type PreviewTemplate = TemplateRef<Option<Ref<Control>>>;

/// The object a design-time value is recorded for. The entry keeps the
/// object alive, so its address identifies it for as long as the entry
/// exists.
#[allow(dead_code)]
enum Target {
    Object(Ref<FerroObject>),
    DataTemplate(Rc<dyn IDataTemplate>),
    Style(Rc<dyn IStyle>),
}

fn object_key(target: &FerroObject) -> usize {
    target as *const FerroObject as usize
}

fn data_template_key(target: &Rc<dyn IDataTemplate>) -> usize {
    Rc::as_ptr(target) as *const () as usize
}

/// A style that is an object has the key of that object.
fn style_key(target: &Rc<dyn IStyle>) -> usize {
    match target.as_object() {
        Some(object) => object_key(object),
        None => Rc::as_ptr(target) as *const () as usize,
    }
}

thread_local! {
    static IS_DESIGN_MODE: Cell<bool> = const { Cell::new(false) };
    static PREVIEW_WITH: RefCell<HashMap<usize, (Target, Option<PreviewTemplate>)>> = RefCell::new(HashMap::new());
    static TEMPLATE_DATA_CONTEXT: RefCell<HashMap<usize, (Rc<dyn IDataTemplate>, Option<BoxedValue>)>> =
        RefCell::new(HashMap::new());
}

fn set_preview(key: usize, target: Target, template: Option<PreviewTemplate>) {
    let previous = PREVIEW_WITH.with(|map| map.borrow_mut().insert(key, (target, template)));
    drop(previous);
}

fn get_preview(key: usize) -> Option<Ref<Control>> {
    let template = PREVIEW_WITH.with(|map| map.borrow().get(&key).and_then(|(_, template)| template.clone()));
    template.and_then(|template| template.build_typed())
}

fn control_template(control: Ref<Control>) -> PreviewTemplate {
    FuncTemplate::new(move || Some(control.clone()))
}

fn build_template(template: PreviewTemplate) -> PreviewTemplate {
    FuncTemplate::new(move || template.build_typed())
}

/// Helper class for design-time property support.
pub struct Design;

ferroui_base::ferro_static_type!(Design);

ferroui_base::ferro_properties! { impl Design, also [
    Design::height_property,
    Design::width_property,
    Design::data_context_property,
    Design::preview_with_property,
    Design::design_style_property,
] {} }

impl Design {
    /// Whether the application is running in design mode.
    ///
    /// This property is set by the previewer when it hosts the application;
    /// it is false in a running application.
    pub fn is_design_mode() -> bool {
        IS_DESIGN_MODE.get()
    }

    /// Sets whether the application is running in design mode. For design
    /// hosts.
    #[doc(hidden)]
    pub fn set_is_design_mode(value: bool) {
        IS_DESIGN_MODE.set(value)
    }

    ferro_property!(for Design;
        /// Defines the `Height` attached property: a height for the control
        /// that is applied at design time only.
        pub fn height_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<Design, Control, _>("Height", 0.0)
        }
    );

    /// Sets the design-time height for a control.
    pub fn set_height(control: &Control, value: f64) {
        control.set_value(Self::height_property(), value)
    }

    /// Gets the design-time height for a control.
    pub fn get_height(control: &Control) -> f64 {
        control.get_value(Self::height_property())
    }

    ferro_property!(for Design;
        /// Defines the `Width` attached property: a width for the control
        /// that is applied at design time only.
        pub fn width_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<Design, Control, _>("Width", 0.0)
        }
    );

    /// Sets the design-time width for a control.
    pub fn set_width(control: &Control, value: f64) {
        control.set_value(Self::width_property(), value)
    }

    /// Gets the design-time width for a control.
    pub fn get_width(control: &Control) -> f64 {
        control.get_value(Self::width_property())
    }

    ferro_property!(for Design;
        /// Defines the `DataContext` attached property: a data context for
        /// the control that is applied at design time only.
        pub fn data_context_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<Design, Control, _>("DataContext", None)
        }
    );

    /// Sets the design-time data context for a control.
    pub fn set_data_context(control: &Control, value: Option<BoxedValue>) {
        control.set_value(Self::data_context_property(), value)
    }

    /// Gets the design-time data context for a control.
    pub fn get_data_context(control: &Control) -> Option<BoxedValue> {
        control.get_value(Self::data_context_property())
    }

    /// Sets the design-time data context for a data template.
    pub fn set_data_context_for_data_template(control: &Rc<dyn IDataTemplate>, value: Option<BoxedValue>) {
        let previous = TEMPLATE_DATA_CONTEXT
            .with(|map| map.borrow_mut().insert(data_template_key(control), (control.clone(), value)));
        drop(previous);
    }

    /// Gets the design-time data context for a data template.
    pub fn get_data_context_for_data_template(control: &Rc<dyn IDataTemplate>) -> Option<BoxedValue> {
        TEMPLATE_DATA_CONTEXT
            .with(|map| map.borrow().get(&data_template_key(control)).and_then(|(_, value)| value.clone()))
    }

    ferro_property!(for Design;
        /// Defines the `PreviewWith` attached property: a control that
        /// hosts the object in the previewer.
        pub fn preview_with_property() -> AttachedProperty<Option<Ref<Control>>> {
            FerroProperty::register_attached::<Design, FerroObject, _>("PreviewWith", None)
        }
    );

    /// Sets a preview template for the specified object at design time.
    ///
    /// This method allows you to specify a substitute control template to
    /// be used to preview the given object in the previewer.
    pub fn set_preview_with_template(target: &Ref<FerroObject>, template: Option<PreviewTemplate>) {
        set_preview(object_key(target), Target::Object(target.clone()), template)
    }

    /// Sets a preview control for the specified object at design time.
    ///
    /// This method allows you to specify a substitute control to be used to
    /// preview the given object in the previewer. A visual cannot be
    /// previewed with a control: it needs a template.
    pub fn set_preview_with(target: &Ref<FerroObject>, control: Option<Ref<Control>>) {
        let template = match control {
            // Not a supported scenario without templates; causes stack overflows.
            Some(_) if target.is::<Visual>() => None,
            Some(control) => Some(control_template(control)),
            None => None,
        };
        set_preview(object_key(target), Target::Object(target.clone()), template)
    }

    /// Sets a preview template for the specified data template at design
    /// time.
    pub fn set_preview_with_template_for_data_template(
        target: &Rc<dyn IDataTemplate>,
        template: Option<PreviewTemplate>,
    ) {
        set_preview(data_template_key(target), Target::DataTemplate(target.clone()), template.map(build_template))
    }

    /// Sets a preview control for the specified data template at design
    /// time.
    pub fn set_preview_with_for_data_template(target: &Rc<dyn IDataTemplate>, control: Option<Ref<Control>>) {
        set_preview(data_template_key(target), Target::DataTemplate(target.clone()), control.map(control_template))
    }

    /// Sets a preview template for the specified style at design time.
    pub fn set_preview_with_template_for_style(target: &Rc<dyn IStyle>, template: Option<PreviewTemplate>) {
        set_preview(style_key(target), Target::Style(target.clone()), template.map(build_template))
    }

    /// Sets a preview control for the specified style at design time.
    pub fn set_preview_with_for_style(target: &Rc<dyn IStyle>, control: Option<Ref<Control>>) {
        set_preview(style_key(target), Target::Style(target.clone()), control.map(control_template))
    }

    /// Gets the preview control for the specified object at design time,
    /// if one is set.
    pub fn get_preview_with(target: &FerroObject) -> Option<Ref<Control>> {
        get_preview(object_key(target))
    }

    /// Gets the preview control for the specified data template at design
    /// time, if one is set.
    pub fn get_preview_with_for_data_template(target: &Rc<dyn IDataTemplate>) -> Option<Ref<Control>> {
        get_preview(data_template_key(target))
    }

    /// Gets the preview control for the specified style at design time, if
    /// one is set.
    pub fn get_preview_with_for_style(target: &Rc<dyn IStyle>) -> Option<Ref<Control>> {
        get_preview(style_key(target))
    }

    ferro_property!(for Design;
        /// Defines the `DesignStyle` attached property for design-time use.
        ///
        /// This property allows you to apply a style to a control only at
        /// design-time, enabling custom visualizations or highlighting in
        /// the designer without affecting the runtime appearance.
        pub fn design_style_property() -> AttachedProperty<Option<DesignStyleValue>> {
            FerroProperty::register_attached::<Design, Control, _>("DesignStyle", None)
        }
    );

    /// Sets the design-time style for a control.
    pub fn set_design_style(control: &Control, value: Rc<dyn IStyle>) {
        control.set_value(Self::design_style_property(), Some(DesignStyleValue(value)))
    }

    /// Gets the design-time style for a control.
    pub fn get_design_style(control: &Control) -> Option<Rc<dyn IStyle>> {
        control.get_value(Self::design_style_property()).map(|value| value.0)
    }

    /// Applies the design-time properties set on `source` to `target`.
    pub fn apply_design_mode_properties(target: &Control, source: &Control) {
        let source_object: &FerroObject = source;
        if source.is_set(Self::width_property()) {
            target.bind(
                Layoutable::width_property(),
                FerroObjectExtensions::get_observable(source_object, &**Self::width_property()),
                BindingPriority::LocalValue,
            );
        }
        if source.is_set(Self::height_property()) {
            target.bind(
                Layoutable::height_property(),
                FerroObjectExtensions::get_observable(source_object, &**Self::height_property()),
                BindingPriority::LocalValue,
            );
        }
        if source.is_set(Self::data_context_property()) {
            target.bind(
                StyledElement::data_context_property(),
                FerroObjectExtensions::get_observable(source_object, &**Self::data_context_property()),
                BindingPriority::LocalValue,
            );
        }
        if source.is_set(Self::design_style_property()) {
            if let Some(style) = Self::get_design_style(source) {
                target.styles().add(style);
            }
        }
    }

    /// Creates the control the previewer shows for `target`: the object
    /// itself when it is a control, otherwise the control it is previewed
    /// with, set up to host it, or a message that says why it cannot be
    /// previewed.
    #[doc(hidden)]
    pub fn create_preview_with_control(target: &PreviewTarget) -> Option<Ref<Control>> {
        let target = match target {
            PreviewTarget::Style(style) => return Some(Self::create_preview_for_style(style)),
            PreviewTarget::DataTemplate(template) => return Some(Self::create_preview_for_data_template(template)),
            PreviewTarget::Other => return message_line("This file cannot be previewed in design view"),
            PreviewTarget::Object(target) => target,
        };

        // The styles that are objects.
        if let Some(style) = target.cast::<ferroui_base::styling::StyleBase>() {
            return Some(Self::create_preview_for_style(&style.into()));
        }
        if let Some(styles) = target.cast::<ferroui_base::styling::Styles>() {
            return Some(Self::create_preview_for_style(&styles.into()));
        }

        if let Some(resources) = target.cast::<ferroui_base::controls::ResourceDictionary>() {
            if let Some(substitute) = Self::get_preview_with(target) {
                substitute.resources().merged_dictionaries().add(resources.into());
                return Some(substitute);
            }

            return Some(message(&[
                "ResourceDictionaries can't be previewed without Design.PreviewWith. Add",
                "<Design.PreviewWith>",
                "    <Border Padding=\"20\"><!-- YOUR CONTROL FOR PREVIEW HERE --></Border>",
                "</Design.PreviewWith>",
                "in your resource dictionary",
            ]));
        }

        if target.is::<crate::Application>() {
            return message_line("This file cannot be previewed in design view");
        }

        if !target.is::<crate::Window>() {
            if let Some(preview_with) = Self::get_preview_with(target) {
                return Some(preview_with);
            }
        }

        match target.cast::<Control>() {
            Some(control) => Some(control),
            None => message_line("This file cannot be previewed in design view"),
        }
    }

    fn create_preview_for_style(style: &Rc<dyn IStyle>) -> Ref<Control> {
        if let Some(substitute) = Self::get_preview_with_for_style(style) {
            substitute.styles().add(style.clone());
            return substitute;
        }

        message(&[
            "Styles can't be previewed without Design.PreviewWith. Add",
            "<Design.PreviewWith>",
            "    <Border Padding=\"20\"><!-- YOUR CONTROL FOR PREVIEW HERE --></Border>",
            "</Design.PreviewWith>",
            "before setters in your first Style",
        ])
    }

    fn create_preview_for_data_template(template: &Rc<dyn IDataTemplate>) -> Ref<Control> {
        let substitute =
            Self::get_preview_with_for_data_template(template).and_then(|control| control.cast::<crate::ContentControl>());
        if let Some(substitute) = substitute {
            substitute.set_content_template(Some(template.clone()));
            if !substitute.is_set(Self::data_context_property())
                && substitute.is_set(StyledElement::data_context_property())
            {
                let data_context = substitute.get_value(StyledElement::data_context_property());
                substitute.set_data_context(data_context);
            }
            return substitute.upcast();
        }

        if let Some(data_context) = Self::get_data_context_for_data_template(template) {
            let substitute = crate::ContentControl::new();
            substitute.set_content_template(Some(template.clone()));
            substitute.set_data_context(Some(data_context.clone()));
            substitute.set_content(Some(data_context));
            return substitute.upcast();
        }

        message(&[
            "IDataTemplate can't be previewed without Design.PreviewWith.",
            "Provide ContentControl with your design data as Content. Previewer will set ContentTemplate from this file.",
            "<Design.PreviewWith>",
            "    <ContentControl Content=\"{x:Static YOUR_DATA_OBJECT_HERE}\" />",
            "</Design.PreviewWith>",
        ])
    }
}

/// What a design-time preview is created for; see
/// [`Design::create_preview_with_control`].
pub enum PreviewTarget {
    /// A style that is not an object.
    Style(Rc<dyn IStyle>),
    /// A data template.
    DataTemplate(Rc<dyn IDataTemplate>),
    /// An object: a control, a style, a resource dictionary, the
    /// application, ...
    Object(Ref<FerroObject>),
    /// Anything else.
    Other,
}

/// A line of text of a preview message.
fn message_line(text: &str) -> Option<Ref<Control>> {
    let line = crate::TextBlock::new();
    line.set_text(Some(text));
    Some(line.upcast())
}

/// A preview message: its lines stacked.
fn message(lines: &[&str]) -> Ref<Control> {
    let panel = crate::StackPanel::new();
    for line in lines {
        if let Some(line) = message_line(line) {
            panel.children().add(line);
        }
    }
    panel.upcast()
}

/// The value of the `DesignStyle` property: a style, compared by identity.
#[derive(Clone)]
pub struct DesignStyleValue(pub Rc<dyn IStyle>);

impl PartialEq for DesignStyleValue {
    fn eq(&self, other: &Self) -> bool {
        ferroui_base::styling::style_ptr_eq(&self.0, &other.0)
    }
}
