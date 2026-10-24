//! Port of `System/ClipboardMonitorBehavior.cs`.

use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::input::DataFormat;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, DirectProperty, FerroObjectImpl, FerroProperty, Ref,
    StaticType, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::{Control, TopLevel};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const TEXT_FORMAT: &str = "Text";
const FILES_FORMAT: &str = "Files";

/// A behavior that monitors the system clipboard for specific data formats.
///
/// While the associated control is in the visual tree, the clipboard of its top level is read
/// every second; [`has_data`](Self::has_data) tells whether it holds data in one of the
/// [`formats`](Self::formats).
#[repr(C)]
pub struct ClipboardMonitorBehavior {
    base: StyledElementBehaviorOfT,
    timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
    has_data: Cell<bool>,
    // Deviation (DEVIATIONS.md, Behaviors): the `ContentChanged` event is a list of Rust handlers,
    // not declared to markup.
    content_changed: HandlerList<dyn Fn()>,
    // The subscriptions of the attached to and detached from visual tree events of the control.
    tree_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

ferro_class!(ClipboardMonitorBehavior: StyledElementBehaviorOfT);
ferro_class_info!(ClipboardMonitorBehavior { new: ClipboardMonitorBehavior::new });
ferroui_base::ferro_impl_classes!(ClipboardMonitorBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ClipboardMonitorBehavior {
        /// Identifies the `Formats` property.
        pub fn formats_property() -> StyledProperty<String> {
            FerroProperty::register::<ClipboardMonitorBehavior, _>("Formats", TEXT_FORMAT.to_string())
        }

        /// Identifies the `HasData` property.
        pub fn has_data_property() -> DirectProperty<ClipboardMonitorBehavior, bool> {
            FerroProperty::register_direct::<ClipboardMonitorBehavior, _>(
                "HasData",
                |behavior| behavior.has_data(),
                None,
                false,
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ClipboardMonitorBehavior {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);
            if let Some(associated_object) = this.associated_object_as::<Control>() {
                let behavior = this.to_ref().downgrade();
                let attached = associated_object.attached_to_visual_tree(move |_e| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.start_monitoring();
                    }
                });
                let behavior = this.to_ref().downgrade();
                let detached = associated_object.detached_from_visual_tree(move |_e| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.stop_monitoring();
                    }
                });
                this.tree_subscriptions.borrow_mut().extend([attached, detached]);

                // If already attached to visual tree, start monitoring.
                if associated_object.is_loaded() {
                    this.start_monitoring();
                }
            }
        }

        fn on_detaching(this: &Self) {
            let subscriptions = std::mem::take(&mut *this.tree_subscriptions.borrow_mut());
            for subscription in subscriptions {
                subscription.dispose();
            }
            this.stop_monitoring();
            Self::parent_on_detaching(this);
        }
    }
}

impl ClipboardMonitorBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            timer: RefCell::new(None),
            has_data: Cell::new(false),
            content_changed: HandlerList::new(),
            tree_subscriptions: RefCell::new(Vec::new()),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the comma-separated list of clipboard formats to listen for (e.g.,
    /// "Text,FileNames").
    pub fn formats(&self) -> String {
        self.get_value(Self::formats_property())
    }

    pub fn set_formats(&self, value: String) {
        self.set_value(Self::formats_property(), value)
    }

    /// Gets a value indicating whether the clipboard contains data in any of the specified
    /// [`formats`](Self::formats).
    pub fn has_data(&self) -> bool {
        self.has_data.get()
    }

    fn set_has_data(&self, value: bool) {
        self.set_and_raise_cell(Self::has_data_property(), &self.has_data, value);
    }

    /// Occurs when the clipboard content availability changes. The subscription ends when the
    /// returned handle is disposed.
    pub fn content_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.content_changed.add(Rc::new(handler));
        let behavior = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(behavior) = behavior.upgrade() {
                behavior.content_changed.remove(token);
            }
        })
    }

    fn start_monitoring(&self) {
        if self.timer.borrow().is_some() {
            return;
        }

        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_secs(1));
        let behavior = self.to_ref().downgrade();
        let tick = timer.tick(move |_timer| {
            if let Some(behavior) = behavior.upgrade() {
                behavior.check_clipboard();
            }
        });
        timer.start();
        *self.timer.borrow_mut() = Some((timer, tick));
        self.check_clipboard();
    }

    fn stop_monitoring(&self) {
        let timer = self.timer.borrow_mut().take();
        if let Some((timer, tick)) = timer {
            timer.stop();
            tick.dispose();
        }
    }

    fn check_clipboard(&self) {
        let Some(associated_object) = self.associated_object_as::<Control>() else {
            return;
        };

        let visual: &Visual = &associated_object;
        let Some(clipboard) = TopLevel::get_top_level(Some(visual)).and_then(|top_level| top_level.clipboard()) else {
            return;
        };

        let behavior = self.to_ref().downgrade();
        let formats = clipboard.get_data_formats_async();
        drop(Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
            // Ignore clipboard access errors.
            let Ok(formats) = formats.await else {
                return;
            };
            let Some(behavior) = behavior.upgrade() else {
                return;
            };

            let requested_formats = behavior.formats();
            let requested_formats: Vec<&str> =
                requested_formats.split(',').map(str::trim).filter(|format| !format.is_empty()).collect();
            let normalized_formats: Vec<String> = formats.iter().map(Self::convert_data_format_identifier).collect();

            let has_data = if requested_formats.is_empty() {
                // If no formats specified, check if any format exists.
                !normalized_formats.is_empty()
            } else {
                requested_formats.iter().any(|requested_format| {
                    let requested_format = Self::normalize_requested_format(requested_format);
                    normalized_formats.iter().any(|format| format.eq_ignore_ascii_case(requested_format))
                })
            };

            if behavior.has_data.get() != has_data {
                behavior.set_has_data(has_data);
                for (_, handler) in behavior.content_changed.snapshot().iter() {
                    handler();
                }
            }
        }));
    }

    fn convert_data_format_identifier(format: &DataFormat) -> String {
        if DataFormat::text().as_data_format() == format {
            return TEXT_FORMAT.to_string();
        }

        if DataFormat::file().as_data_format() == format {
            return FILES_FORMAT.to_string();
        }

        format.identifier().to_string()
    }

    fn normalize_requested_format(format: &str) -> &str {
        if format.eq_ignore_ascii_case("FileNames") {
            FILES_FORMAT
        } else {
            format
        }
    }
}
