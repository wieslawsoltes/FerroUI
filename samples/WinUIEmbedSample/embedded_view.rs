//! Port of the code of the document `EmbeddedView.xaml` of the upstream sample (the class of
//! the document): the content of the swap chain panel. A button and a slider that count
//! and show their value, the keys pressed while it has the focus, the clipboard, a drag source
//! and a drop target, hover, and an ink canvas that draws the points of the pointer, the
//! intermediate points among them.
//!
//! The handlers the document names (`Click`, `PointerPressed`, ...) are the methods of the
//! markup of the class; the others are added in the constructor, as the reference adds them. A
//! handler of an element of the view holds the view, or the element it changes, weakly: the
//! element holds its handlers.

use crate::markup::xaml_class;
use ferroui_base::input::platform::{ClipboardError, IClipboard};
use ferroui_base::input::{
    AsyncDataTransferExtensions, DataFormat, DataTransfer, DataTransferExtensions, DataTransferItem, DragDrop,
    DragDropEffects, DragEventArgs, IAsyncDataTransfer, IDataTransfer, InputElement, InputElementImpl, KeyEventArgs,
    KeyModifiers, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, JoinHandle};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Point, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::{RangeBase, TemplatedControlImpl};
use ferroui_controls::shapes::Ellipse;
use ferroui_controls::{
    Border, Canvas, ContentControlImpl, ControlImpl, Slider, TextBlock, TextBox, TopLevel, UserControl,
};
use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;

#[repr(C)]
pub struct EmbeddedView {
    base: UserControl,
    clicks: Cell<i32>,
    point_count: Cell<i32>,
    intermediate_count: Cell<i32>,
}

ferro_class!(EmbeddedView: UserControl);
ferro_impl_classes!(
    EmbeddedView: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(EmbeddedView {
    new: EmbeddedView::new,
    markup: {
        methods: [
            fn OnAvButtonClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EmbeddedView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_av_button_click(&sender, e.as_routed_event_args())
                },
            fn OnClipboardCopy(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EmbeddedView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clipboard_copy(&sender, e.as_routed_event_args())
                },
            fn OnClipboardPaste(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EmbeddedView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clipboard_paste(&sender, e.as_routed_event_args())
                },
            fn OnInkPointerPressed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EmbeddedView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        this.on_ink_pointer_pressed(&sender, e)
                    }
                },
            fn OnInkPointerMoved(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EmbeddedView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerEventArgs>() {
                        this.on_ink_pointer_moved(&sender, e)
                    }
                },
            fn OnInkPointerReleased(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EmbeddedView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerReleasedEventArgs>() {
                        this.on_ink_pointer_released(&sender, e)
                    }
                },
            fn OnClearInk(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EmbeddedView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_ink(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(EmbeddedView, "/EmbeddedView.xaml");

/// Runs the rest of an `async void` handler on the dispatcher of the thread, where the await of
/// the reference resumes.
fn start_async(future: impl Future<Output = ()> + 'static) -> JoinHandle<()> {
    let priority = Dispatcher::current_priority().unwrap_or(DispatcherPriority::NORMAL);
    Dispatcher::current_dispatcher().start_local_with_priority(future, priority)
}

/// The result of a clipboard operation awaited by an `async void` handler: an exception of
/// such a method is raised on the dispatcher.
///
/// # Panics
/// Panics if the operation failed.
fn raise<T>(result: Result<T, ClipboardError>) -> T {
    result.unwrap_or_else(|error| panic!("{error}"))
}

/// `{e.KeyModifiers}`: the names of the flags, as the text of a flags enumeration of the
/// reference (`None` when there is none).
pub fn key_modifiers_text(modifiers: KeyModifiers) -> String {
    let names: Vec<&str> = [
        (KeyModifiers::ALT, "Alt"),
        (KeyModifiers::CONTROL, "Control"),
        (KeyModifiers::SHIFT, "Shift"),
        (KeyModifiers::META, "Meta"),
    ]
    .into_iter()
    .filter(|(flag, _)| modifiers.contains(*flag))
    .map(|(_, name)| name)
    .collect();
    if names.is_empty() {
        "None".to_string()
    } else {
        names.join(", ")
    }
}

/// `{value:F0}`: the number rounded to an integer, halves away from zero.
pub fn fixed_0(value: f64) -> String {
    format!("{:.0}", value.round())
}

impl EmbeddedView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            clicks: Cell::new(0),
            point_count: Cell::new(0),
            intermediate_count: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let slider_value = this.av_slider_value().downgrade();
        let slider = this.av_slider().downgrade();
        this.av_slider().property_changed(move |e| {
            if e.property() == RangeBase::value_property().as_property() {
                if let (Some(slider_value), Some(slider)) = (slider_value.upgrade(), slider.upgrade()) {
                    slider_value.set_text(Some(&format!("Slider: {}", fixed_0(slider.value()))));
                }
            }
        });
        let hover_state = this.hover_state().downgrade();
        this.hover_panel().add_handler(&InputElement::pointer_entered_event(), move |_, _| {
            if let Some(hover_state) = hover_state.upgrade() {
                hover_state.set_text(Some("over"));
            }
        });
        let hover_state = this.hover_state().downgrade();
        this.hover_panel().add_handler(&InputElement::pointer_exited_event(), move |_, _| {
            if let Some(hover_state) = hover_state.upgrade() {
                hover_state.set_text(Some("out"));
            }
        });
        let key_readout = this.key_readout().downgrade();
        this.add_handler_with(
            &InputElement::key_down_event(),
            move |_, e: &KeyEventArgs| {
                if let Some(key_readout) = key_readout.upgrade() {
                    Self::on_key_readout(&key_readout, e);
                }
            },
            Interactive::DEFAULT_ROUTES,
            true,
        );

        let drop_target = this.drop_target_border();
        DragDrop::set_allow_drop(&drop_target, true);
        drop_target.add_handler(&DragDrop::drag_over_event(), |_, e| Self::on_drop_drag_over(e));
        let drop_status = this.drop_status().downgrade();
        drop_target.add_handler(&DragDrop::drop_event(), move |_, e| {
            if let Some(drop_status) = drop_status.upgrade() {
                Self::on_drop(&drop_status, e);
            }
        });

        this.drag_source_border()
            .add_handler(&InputElement::pointer_pressed_event(), |_, e| Self::on_drag_source_pointer_pressed(e));

        this
    }

    fn av_click_count(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("AvClickCount")
    }

    pub fn av_slider(&self) -> Ref<Slider> {
        self.get_control::<Slider>("AvSlider")
    }

    pub fn av_slider_value(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("AvSliderValue")
    }

    fn key_readout(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("KeyReadout")
    }

    fn clipboard_text(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("ClipboardText")
    }

    fn clipboard_status(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("ClipboardStatus")
    }

    fn drag_source_border(&self) -> Ref<Border> {
        self.get_control::<Border>("DragSourceBorder")
    }

    fn drop_target_border(&self) -> Ref<Border> {
        self.get_control::<Border>("DropTargetBorder")
    }

    fn drop_status(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DropStatus")
    }

    fn hover_panel(&self) -> Ref<Border> {
        self.get_control::<Border>("HoverPanel")
    }

    fn hover_state(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("HoverState")
    }

    fn ink_canvas(&self) -> Ref<Canvas> {
        self.get_control::<Canvas>("InkCanvas")
    }

    fn ink_stats(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("InkStats")
    }

    /// `TopLevel.GetTopLevel(this)?.Clipboard`.
    fn clipboard(&self) -> Option<Rc<dyn IClipboard>> {
        TopLevel::get_top_level(Some(self)).and_then(|top_level| top_level.clipboard())
    }

    /// `async void`.
    fn on_clipboard_copy(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(clipboard) = self.clipboard() else {
            self.clipboard_status().set_text(Some("no clipboard"));
            return;
        };
        let text = self.clipboard_text().text().unwrap_or_default();
        let data = DataTransfer::new();
        data.add(DataTransferItem::create_text(Some(&text)));
        let data: Rc<dyn IAsyncDataTransfer> = data;
        let pending = clipboard.set_data_async(Some(data));
        let this = self.to_ref();
        drop(start_async(async move {
            raise(pending.await);
            let length = this.clipboard_text().text().unwrap_or_default().encode_utf16().count();
            this.clipboard_status().set_text(Some(&format!("copied {length} chars")));
        }));
    }

    /// `async void`.
    fn on_clipboard_paste(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(clipboard) = self.clipboard() else {
            self.clipboard_status().set_text(Some("no clipboard"));
            return;
        };
        let pending = clipboard.try_get_data_async();
        let this = self.to_ref();
        drop(start_async(async move {
            // `using var data`: the data transfer is disposed however the read ends.
            let data = raise(pending.await);
            let text = match &data {
                Some(data) => {
                    let text = data.try_get_text_async().await;
                    data.dispose();
                    raise(text)
                }
                None => None,
            };
            match text {
                Some(text) => {
                    this.clipboard_text().set_text(Some(&text));
                    this.clipboard_status().set_text(Some(&format!("pasted {} chars", text.encode_utf16().count())));
                }
                None => this.clipboard_status().set_text(Some("no text on clipboard")),
            }
        }));
    }

    /// `async void`: a drag of the text, which the drop target of WinUI, a text box or another
    /// application takes.
    fn on_drag_source_pointer_pressed(e: &PointerPressedEventArgs) {
        let data = DataTransfer::new();
        data.add(DataTransferItem::create_text(Some("Hello from FerroUI")));
        let data: Rc<dyn IDataTransfer> = data;
        let pending = DragDrop::do_drag_drop_async(e, data, DragDropEffects::COPY | DragDropEffects::MOVE);
        // A drag that is cancelled or has no source ends with no effect; the reference ignores
        // the exception such a drag may end with.
        drop(start_async(async move {
            let _ = pending.await;
        }));
    }

    fn on_drop_drag_over(e: &DragEventArgs) {
        e.set_drag_effects(e.drag_effects() & (DragDropEffects::COPY | DragDropEffects::LINK | DragDropEffects::MOVE));
        if e.drag_effects() == DragDropEffects::NONE {
            e.set_drag_effects(DragDropEffects::COPY);
        }
        e.set_handled(true);
    }

    fn on_drop(drop_status: &TextBlock, e: &DragEventArgs) {
        let dt = e.data_transfer();
        let files = dt.try_get_values(&DataFormat::file()).unwrap_or_default();
        let text = dt.try_get_value(&DataFormat::text());

        if !files.is_empty() {
            let paths: Vec<String> = files.iter().map(|file| file.path().local_path()).collect();
            drop_status.set_text(Some(&format!("{} file(s):\n{}", files.len(), paths.join("\n"))));
        } else if let Some(text) = text.filter(|text| !text.is_empty()) {
            drop_status.set_text(Some(&format!("text: {text}")));
        } else {
            drop_status.set_text(Some("(unknown payload)"));
        }

        e.set_handled(true);
    }

    fn on_key_readout(key_readout: &TextBlock, e: &KeyEventArgs) {
        let symbol = match &e.key_symbol {
            Some(symbol) if !symbol.is_empty() => format!("\"{symbol}\""),
            _ => "null".to_string(),
        };
        key_readout.set_text(Some(&format!(
            "Key={}  Physical={}  Symbol={symbol}\nModifiers={}",
            e.key,
            e.physical_key,
            key_modifiers_text(e.key_modifiers)
        )));
    }

    pub fn on_av_button_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.clicks.set(self.clicks.get() + 1);
        self.av_click_count().set_text(Some(&format!("Clicked {} times", self.clicks.get())));
    }

    fn on_ink_pointer_pressed(&self, _sender: &Option<BoxedValue>, e: &PointerPressedEventArgs) {
        let ink_canvas = self.ink_canvas();
        self.add_dot(e.get_position(Some(&ink_canvas)), Brushes::red());
        self.point_count.set(self.point_count.get() + 1);
        let capture: Ref<InputElement> = ink_canvas.upcast();
        e.pointer().capture(Some(&capture));
        self.update_stats();
        e.set_handled(true);
    }

    fn on_ink_pointer_moved(&self, _sender: &Option<BoxedValue>, e: &PointerEventArgs) {
        let ink_canvas = self.ink_canvas();
        let intermediates = e.get_intermediate_points(Some(&ink_canvas));
        for intermediate in &intermediates {
            self.add_dot(intermediate.position, Brushes::orange());
        }
        self.intermediate_count.set(self.intermediate_count.get() + intermediates.len() as i32);

        self.add_dot(e.get_position(Some(&ink_canvas)), Brushes::green());
        self.point_count.set(self.point_count.get() + 1);
        self.update_stats();
    }

    fn on_ink_pointer_released(&self, _sender: &Option<BoxedValue>, e: &PointerReleasedEventArgs) {
        self.add_dot(e.get_position(Some(&self.ink_canvas())), Brushes::blue());
        self.point_count.set(self.point_count.get() + 1);
        self.update_stats();
        e.set_handled(true);
    }

    fn add_dot(&self, position: Point, brush: Rc<dyn IBrush>) {
        const SIZE: f64 = 6.0;
        let dot = Ellipse::new();
        dot.set_width(SIZE);
        dot.set_height(SIZE);
        dot.set_fill(Some(brush));
        Canvas::set_left(&dot, position.x - SIZE / 2.0);
        Canvas::set_top(&dot, position.y - SIZE / 2.0);
        self.ink_canvas().children().add(dot);
    }

    pub fn on_clear_ink(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.ink_canvas().children().clear();
        self.point_count.set(0);
        self.intermediate_count.set(0);
        self.update_stats();
    }

    fn update_stats(&self) {
        self.ink_stats().set_text(Some(&format!(
            "{} points / {} intermediate",
            self.point_count.get(),
            self.intermediate_count.get()
        )));
    }
}
