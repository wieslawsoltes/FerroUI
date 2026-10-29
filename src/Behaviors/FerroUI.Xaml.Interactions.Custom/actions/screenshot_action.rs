//! Port of `Actions/ScreenshotAction.cs`.

use super::control_of;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl, FerroProperty, Nullable, PixelSize, Ref, StyledElementImpl, StyledProperty, Visual};
use ferroui_controls::Control;
use ferroui_base::media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::platform::storage::{FilePickerFileTypes, FilePickerSaveOptions};
use ferroui_base::threading::Dispatcher;
use ferroui_controls::TopLevel;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Renders the target control (or the sender) to a bitmap and saves it as a PNG file the user picks.
#[repr(C)]
pub struct ScreenshotAction {
    base: StyledElementAction,
}

ferro_class!(ScreenshotAction: StyledElementAction);
ferro_class_info!(ScreenshotAction {
    new: ScreenshotAction::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ScreenshotAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ScreenshotAction {
        /// Identifies the `FileName` property.
        pub fn file_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ScreenshotAction, _>("FileName", None)
        }

        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own its target, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<ScreenshotAction, _>("TargetControl", None)
        }
    }
}

impl StyledElementActionImpl for ScreenshotAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let target = this.target_control().or_else(|| control_of(sender));
        let Some(target) = target else {
            return Some(Rc::new(false));
        };

        let target_visual: &Visual = &target;
        let Some(top_level) = TopLevel::get_top_level(Some(target_visual)) else {
            return Some(Rc::new(false));
        };

        this.capture_async(target, top_level);
        Some(Rc::new(true))
    }
}

impl ScreenshotAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the target control. If not set, the sender is used. Markup resolves a name
    /// given for it to the named element.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the file name suggested to the user. This is a property of the object model.
    pub fn file_name(&self) -> Option<String> {
        self.get_value(Self::file_name_property())
    }

    pub fn set_file_name(&self, value: Option<String>) {
        self.set_value(Self::file_name_property(), value)
    }

    fn capture_async(&self, target: Ref<Control>, top_level: Ref<TopLevel>) {
        // Render the control to a bitmap, at 96 DPI.
        let bounds = target.bounds();
        let pixel_size = PixelSize::new(bounds.width as i32, bounds.height as i32);
        let bitmap = RenderTargetBitmap::new(pixel_size);
        bitmap.render(&target.clone().upcast::<Visual>());

        let mut options = FilePickerSaveOptions::new();
        options.set_title(Some("Save Screenshot".to_string()));
        options.set_default_extension(Some("png".to_string()));
        options.set_suggested_file_name(Some(self.file_name().unwrap_or_else(|| "screenshot.png".to_string())));
        options.set_file_type_choices(Some(vec![FilePickerFileTypes::image_png()]));

        let save = top_level.storage_provider().save_file_picker_async(options);
        drop(Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
            // A failure is reported in the debug output, as the reference writes it.
            let result: std::io::Result<()> = async {
                if let Some(file) = save.await? {
                    let mut stream = file.open_write_async().await?;
                    bitmap.save(&mut *stream, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::default()))?;
                }
                Ok::<(), std::io::Error>(())
            }
            .await;
            if let Err(error) = result {
                if cfg!(debug_assertions) {
                    eprintln!("Screenshot failed: {error}");
                }
            }
        }));
    }
}
