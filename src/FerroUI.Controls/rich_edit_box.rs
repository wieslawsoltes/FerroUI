use crate::automation::provider::TextEditChangeType;
use crate::documents::{
    BlockCollection, CharacterFormat, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind,
    LineStackingStrategy, ParagraphFormat, RichTextDocument, TextHighlighterCollection,
    TextPosition,
};
use crate::flyouts::PopupFlyoutBase;
use crate::metadata::{PseudoClassesAttribute, TemplatePartAttribute};
use crate::presenters::ContentPresenter;
use crate::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::rich_edit_box_text_input_method_client::RichEditBoxTextInputMethodClient;
use crate::rich_text::{
    EditError, EditKind, EditorSelection, RichEditClipboardFormat, RichEditTextDocument,
    RichTextCompositionChange, RichTextEditor, RichTextTransfer, RtfError, RtfReader, RtfWriter,
};
use crate::rich_text_block::RichTextSelectionChangedEventArgs;
use crate::rich_text_layout::RichTextLineMetrics;
use crate::templates::{IControlTemplate, IDataTemplate};
use crate::utils::ClipboardHelper;
use crate::TextBlock;
use crate::{
    Application, ContentControl, ContextMenuEventArgs, Control, ControlImpl, Decorator, Panel, RichEditBoxTextPresenter,
    RichTextBlock, TextBox, TopLevel,
};
use ferroui_base::animation::TimeSpan;
use ferroui_base::input::platform::{
    ClipboardError, ClipboardErrorKind, IClipboard, PlatformHotkeyConfiguration,
};
use ferroui_base::input::text_input::{
    TextInputMethodClient, TextInputOptions,
};
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, InputScope, Key, KeyEventArgs,
    KeyGesture, KeyboardDevice, LocalBoxFuture, PointerPressedEventArgs, TextInputEventArgs,
};
use ferroui_base::interactivity::{
    IRoutedEventArgs, Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken,
    RoutingStrategies,
};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::{
    subscribe_invalidated, Brushes, FontStyle, FontWeight, IBrush, TextAlignment,
    TextDecorationCollection, TextDecorationLocation, TextDecorations, TextWrapping,
};
use ferroui_base::reactive::{CompositeDisposable, Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_routed_event, ferro_routed_event_args, instantiate,
    BoxedValue, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Point, Rect, Ref, Size, StaticType, StyledElementImpl,
    StyledProperty, StyledPropertyOptions, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;

#[path = "rich_edit_box_format_commands.rs"]
mod format_commands;

#[path = "rich_edit_context_menu.rs"]
mod context_menu;

#[path = "rich_edit_fragment_filter.rs"]
mod fragment_filter;

/// Character casing applied to newly entered text, including clipboard and IME commits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RichEditCharacterCasing {
    #[default]
    Normal = 0,
    Lower = 1,
    Upper = 2,
}
ferroui_base::ferro_markup_enum!(RichEditCharacterCasing { Normal, Lower, Upper }, { namespace: "FerroUI.Controls" });

bitflags::bitflags! {
    /// Built-in formatting accelerators applications can individually suppress.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct DisabledFormattingAccelerators: u32 {
        const NONE = 0;
        const BOLD = 1;
        const ITALIC = 2;
        const UNDERLINE = 4;
        const ALL = u32::MAX;
    }
}
ferroui_base::ferro_markup_enum!(flags DisabledFormattingAccelerators {
    None = DisabledFormattingAccelerators::NONE,
    Bold = DisabledFormattingAccelerators::BOLD,
    Italic = DisabledFormattingAccelerators::ITALIC,
    Underline = DisabledFormattingAccelerators::UNDERLINE,
    All = DisabledFormattingAccelerators::ALL,
}, { namespace: "FerroUI.Controls" });

#[derive(Clone)]
pub struct RichEditCompositionEventArgs {
    base: RoutedEventArgs,
    pub text: String,
    pub selection: EditorSelection,
    pub committed: bool,
    start_index: i32,
    length: i32,
}

impl Default for RichEditCompositionEventArgs {
    fn default() -> Self {
        Self {
            base: RoutedEventArgs::default(),
            text: String::new(),
            selection: EditorSelection::default(),
            committed: false,
            // CTextCompositionEventArgsBase initializes an unavailable range
            // with StartIndex=-1 and Length=0.
            start_index: -1,
            length: 0,
        }
    }
}

impl RichEditCompositionEventArgs {
    /// UTF-16 start of the whole accepted composition, independently of its
    /// current caret. An unavailable or unrepresentable range has index -1.
    pub fn start_index(&self) -> i32 {
        self.start_index
    }

    /// UTF-16 length of the whole accepted composition. Cancellation has an
    /// empty range at its former start in the restored committed document.
    pub fn length(&self) -> i32 {
        self.length
    }
}
ferro_routed_event_args!(RichEditCompositionEventArgs: RoutedEventArgs);

fn typed_rich_edit_event_args<T: IRoutedEventArgs, R>(
    args: &Rc<dyn IRoutedEventArgs>,
    read: impl FnOnce(&T) -> R,
) -> Result<R, String> {
    args.downcast_ref::<T>()
        .map(read)
        .ok_or_else(|| format!("The arguments are not {}.", std::any::type_name::<T>()))
}

ferroui_base::ferro_markup_type!(class RichEditCompositionEventArgs {
    namespace: "FerroUI.Controls",
    handles: [RichEditCompositionEventArgs],
    this: Rc<dyn IRoutedEventArgs>,
    base: Rc<dyn IRoutedEventArgs>,
    properties: [
        Text: String {
            try_get: |args: &Rc<dyn IRoutedEventArgs>| typed_rich_edit_event_args::<RichEditCompositionEventArgs, _>(args, |args| args.text.clone())
        },
        SelectionStart: i32 {
            try_get: |args: &Rc<dyn IRoutedEventArgs>| typed_rich_edit_event_args::<RichEditCompositionEventArgs, _>(args, |args| args.selection.anchor.min(i32::MAX as usize) as i32)
        },
        SelectionEnd: i32 {
            try_get: |args: &Rc<dyn IRoutedEventArgs>| typed_rich_edit_event_args::<RichEditCompositionEventArgs, _>(args, |args| args.selection.active.min(i32::MAX as usize) as i32)
        },
        Committed: bool {
            try_get: |args: &Rc<dyn IRoutedEventArgs>| typed_rich_edit_event_args::<RichEditCompositionEventArgs, _>(args, |args| args.committed)
        },
        StartIndex: i32 {
            try_get: |args: &Rc<dyn IRoutedEventArgs>| typed_rich_edit_event_args(args, RichEditCompositionEventArgs::start_index)
        },
        Length: i32 {
            try_get: |args: &Rc<dyn IRoutedEventArgs>| typed_rich_edit_event_args(args, RichEditCompositionEventArgs::length)
        },
    ],
});

/// Metadata carrier for the closed composition-event routed-event type.
#[doc(hidden)]
pub struct RichEditCompositionRoutedEvent;
ferroui_base::ferro_markup_type!(class RichEditCompositionRoutedEvent as "RoutedEvent`1" {
    namespace: "FerroUI.Interactivity",
    handles: [RoutedEvent<RichEditCompositionEventArgs>, Option<RoutedEvent<RichEditCompositionEventArgs>>],
    base: RoutedEvent,
    generic: "RoutedEvent`1" [RichEditCompositionEventArgs],
});

/// The proposed UTF-16 selection before its change is published.
///
/// A handler may cancel the proposal or select a different range. An explicitly
/// selected replacement wins over cancellation, as in the WinUI control contract.
#[derive(Clone, Default)]
pub struct RichEditBoxSelectionChangingEventArgs {
    base: RoutedEventArgs,
    selection_start: usize,
    selection_length: usize,
    cancel: Rc<Cell<bool>>,
}
ferro_routed_event_args!(RichEditBoxSelectionChangingEventArgs: RoutedEventArgs);

impl RichEditBoxSelectionChangingEventArgs {
    pub fn selection_start(&self) -> usize {
        self.selection_start
    }
    pub fn selection_length(&self) -> usize {
        self.selection_length
    }
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }
}
ferroui_base::ferro_markup_type!(class RichEditBoxSelectionChangingEventArgs {
    namespace: "FerroUI.Controls",
    handles: [RichEditBoxSelectionChangingEventArgs],
    this: Rc<dyn IRoutedEventArgs>,
    base: Rc<dyn IRoutedEventArgs>,
    properties: [
        SelectionStart: i32 {
            try_get: |args: &Rc<dyn IRoutedEventArgs>| typed_rich_edit_event_args::<RichEditBoxSelectionChangingEventArgs, _>(args, |args| args.selection_start().min(i32::MAX as usize) as i32)
        },
        SelectionLength: i32 {
            try_get: |args: &Rc<dyn IRoutedEventArgs>| typed_rich_edit_event_args::<RichEditBoxSelectionChangingEventArgs, _>(args, |args| args.selection_length().min(i32::MAX as usize) as i32)
        },
        Cancel: bool {
            try_get: |args: &Rc<dyn IRoutedEventArgs>| typed_rich_edit_event_args(args, RichEditBoxSelectionChangingEventArgs::cancel),
            try_set: |args: &Rc<dyn IRoutedEventArgs>, value: bool| typed_rich_edit_event_args::<RichEditBoxSelectionChangingEventArgs, _>(args, |args| args.set_cancel(value))
        },
    ],
});

/// Rich text content-change information; formatting alone leaves this flag false.
#[derive(Clone, Default)]
pub struct RichEditBoxTextChangingEventArgs {
    base: RoutedEventArgs,
    is_content_changing: bool,
}
ferro_routed_event_args!(RichEditBoxTextChangingEventArgs: RoutedEventArgs);

impl RichEditBoxTextChangingEventArgs {
    pub fn is_content_changing(&self) -> bool {
        self.is_content_changing
    }
}
ferroui_base::ferro_markup_type!(class RichEditBoxTextChangingEventArgs {
    namespace: "FerroUI.Controls",
    handles: [RichEditBoxTextChangingEventArgs],
    this: Rc<dyn IRoutedEventArgs>,
    base: Rc<dyn IRoutedEventArgs>,
    properties: [
        IsContentChanging: bool {
            try_get: |args: &Rc<dyn IRoutedEventArgs>| typed_rich_edit_event_args(args, RichEditBoxTextChangingEventArgs::is_content_changing)
        },
    ],
});

#[doc(hidden)]
pub struct RichEditBoxSelectionChangingRoutedEvent;
ferroui_base::ferro_markup_type!(class RichEditBoxSelectionChangingRoutedEvent as "RoutedEvent`1" {
    namespace: "FerroUI.Interactivity",
    handles: [RoutedEvent<RichEditBoxSelectionChangingEventArgs>, Option<RoutedEvent<RichEditBoxSelectionChangingEventArgs>>],
    base: RoutedEvent,
    generic: "RoutedEvent`1" [RichEditBoxSelectionChangingEventArgs],
});

#[doc(hidden)]
pub struct RichEditBoxTextChangingRoutedEvent;
ferroui_base::ferro_markup_type!(class RichEditBoxTextChangingRoutedEvent as "RoutedEvent`1" {
    namespace: "FerroUI.Interactivity",
    handles: [RoutedEvent<RichEditBoxTextChangingEventArgs>, Option<RoutedEvent<RichEditBoxTextChangingEventArgs>>],
    base: RoutedEvent,
    generic: "RoutedEvent`1" [RichEditBoxTextChangingEventArgs],
});

/// A structured rich-text editor rendered by FerroUI's text layout, drawing, and input APIs.
///
/// The `Document` is shared with its persistent presenter and remains editable independently of
/// this control. Read-only and length policies apply to user editing, clipboard, and IME;
/// applications can replace the document directly. Every user operation goes through the
/// same transaction engine, including formatting and accessible value changes.
#[repr(C)]
/// Marks a composition operation of the control itself while it lives.
struct CompositionOperation<'a>(&'a RichEditBox);

impl<'a> CompositionOperation<'a> {
    fn enter(owner: &'a RichEditBox) -> Self {
        owner.composition_operation_depth.set(owner.composition_operation_depth.get() + 1);
        Self(owner)
    }
}

impl Drop for CompositionOperation<'_> {
    fn drop(&mut self) {
        self.0.composition_operation_depth.set(self.0.composition_operation_depth.get() - 1);
    }
}

pub struct RichEditBox {
    base: TemplatedControl,
    text_presenter: Ref<RichEditBoxTextPresenter>,
    content_element: RefCell<Option<Ref<Control>>>,
    applied_template: RefCell<Option<Rc<dyn IControlTemplate>>>,
    editor: RefCell<Option<Rc<RichTextEditor>>>,
    /// The depth of the control's own composition operations: a composition the editor
    /// ends outside them (a programmatic edit, undo, paste, the text object model) is
    /// reported as ended and the input method is reset (#243 review, S4).
    composition_operation_depth: Cell<u32>,
    im_client: Rc<RichEditBoxTextInputMethodClient>,
    document_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    selection_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    editor_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    input_scope_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    input_scope_owns_options: Cell<bool>,
    projecting_selection: Cell<bool>,
    last_input_selection: Cell<EditorSelection>,
    last_text: RefCell<String>,
    document_generation: Cell<u64>,
    header_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    description_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    placeholder_presenter: RefCell<Option<Ref<TextBlock>>>,
    selection_brush_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    context_flyout_closed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    flyout_focus_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    context_menu_request_generation: Cell<u64>,
    force_focused_visual_state: Cell<bool>,
    can_undo: Cell<bool>,
    can_redo: Cell<bool>,
    can_copy: Cell<bool>,
    can_cut: Cell<bool>,
    can_paste: Cell<bool>,
    can_format: Cell<bool>,
    formatting_state: Cell<format_commands::FormattingState>,
    errors: Rc<HandlerList<dyn Fn(&EditError)>>,
    clipboard_errors: Rc<HandlerList<dyn Fn(&ClipboardError)>>,
    caret_scroll_pending: Cell<bool>,
    caret_timer: RefCell<Option<Rc<DispatcherTimer>>>,
    caret_phase: Cell<bool>,
}

ferro_class!(RichEditBox: TemplatedControl);

impl Drop for RichEditBox {
    fn drop(&mut self) {
        if let Some(timer) = self.caret_timer.get_mut().take() {
            timer.stop();
        }
        for subscription in [
            self.editor_subscription.get_mut().take(),
            self.input_scope_subscription.get_mut().take(),
            self.context_flyout_closed_subscription.get_mut().take(),
            self.flyout_focus_subscription.get_mut().take(),
        ]
        .into_iter()
        .flatten()
        {
            subscription.dispose();
        }
    }
}
ferroui_base::ferro_class_info!(RichEditBox {
    new: RichEditBox::new,
    markup: {
        content: Blocks,
        attributes: [
            TemplatePart("ContentElement", type(Ref<Control>), IsRequired = true),
            TemplatePart("HeaderContentPresenter", type(Ref<ContentPresenter>)),
            TemplatePart("PlaceholderTextContentPresenter", type(Ref<TextBlock>)),
            TemplatePart("DescriptionPresenter", type(Ref<ContentPresenter>)),
        ],
        properties: [
            Blocks: BlockCollection { get: RichEditBox::blocks },
            TextHighlighters: TextHighlighterCollection { get: RichEditBox::text_highlighters },
        ],
        fields: [
            CopyingToClipboardEvent: RoutedEvent<RoutedEventArgs> => || *RichEditBox::copying_to_clipboard_event(),
            CuttingToClipboardEvent: RoutedEvent<RoutedEventArgs> => || *RichEditBox::cutting_to_clipboard_event(),
            PastingFromClipboardEvent: RoutedEvent<RoutedEventArgs> => || *RichEditBox::pasting_from_clipboard_event(),
            TextChangedEvent: RoutedEvent<RoutedEventArgs> => || *RichEditBox::text_changed_event(),
            TextChangingEvent: RoutedEvent<RichEditBoxTextChangingEventArgs> => || *RichEditBox::text_changing_event(),
            SelectionChangingEvent: RoutedEvent<RichEditBoxSelectionChangingEventArgs> => || *RichEditBox::selection_changing_event(),
            SelectionChangedEvent: RoutedEvent<RoutedEventArgs> => || *RichEditBox::selection_changed_event(),
            ContextMenuOpeningEvent: RoutedEvent<ContextMenuEventArgs> => || *RichEditBox::context_menu_opening_event(),
            TextCompositionStartedEvent: RoutedEvent<RichEditCompositionEventArgs> => || *RichEditBox::text_composition_started_event(),
            TextCompositionChangedEvent: RoutedEvent<RichEditCompositionEventArgs> => || *RichEditBox::text_composition_changed_event(),
            TextCompositionEndedEvent: RoutedEvent<RichEditCompositionEventArgs> => || *RichEditBox::text_composition_ended_event(),
        ],
        methods: [
            fn Copy() => RichEditBox::copy,
            fn Cut() => RichEditBox::cut,
            fn Paste() => RichEditBox::paste,
            fn SelectAll() => RichEditBox::select_all,
            fn ToggleBold() => |this: &Ref<RichEditBox>| { this.handle_edit(this.toggle_bold()); },
            fn ToggleItalic() => |this: &Ref<RichEditBox>| { this.handle_edit(this.toggle_italic()); },
            fn ToggleUnderline() => |this: &Ref<RichEditBox>| { this.handle_edit(this.toggle_underline()); },
            fn Undo() => |this: &Ref<RichEditBox>| { this.handle_edit(this.undo()); },
            fn Redo() => |this: &Ref<RichEditBox>| { this.handle_edit(this.redo()); },
        ],
    },
});
ferro_impl_classes!(RichEditBox: StyledElementImpl, InteractiveImpl);

impl LayoutableImpl for RichEditBox {
    fn apply_template(this: &Self) {
        let template = this.template();
        let applied = this.applied_template.borrow().clone();
        if applied != template && (template.is_some() || this.is_attached_to_logical_tree()) {
            // Remove the persistent view before TemplatedControl clears template
            // descendants. Its document, history and composition survive this.
            this.release_template_parts();
            Self::parent_apply_template(this);
            *this.applied_template.borrow_mut() = template;
        } else {
            Self::parent_apply_template(this);
        }
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let result = Self::parent_arrange_override(this, final_size);
        this.on_text_view_geometry_changed();
        result
    }
}

impl TemplatedControlImpl for RichEditBox {
    fn on_apply_template(this: &Self, event: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, event);
        let scope = event.name_scope();
        *this.header_presenter.borrow_mut() =
            scope.find_as::<ContentPresenter>("HeaderContentPresenter");
        *this.description_presenter.borrow_mut() =
            scope.find_as::<ContentPresenter>("DescriptionPresenter");
        *this.placeholder_presenter.borrow_mut() =
            scope.find_as::<TextBlock>("PlaceholderTextContentPresenter");
        if let Some(host) = scope.find_as::<Control>("ContentElement") {
            this.attach_to_content_element(host);
        }
        this.update_presenters();
        this.im_client.raise_text_view_visual_changed();
        this.on_text_view_geometry_changed();
        this.reset_caret_blink();
        this.ensure_caret_visible();
    }
}

impl ControlImpl for RichEditBox {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::RichEditBoxAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for RichEditBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.text_presenter.set_owner(&this.to_ref());
        this.text_presenter.set_inheritance_parent(this.to_ref());
        this.sync_presenter_properties();
        this.refresh_selection_brush_subscriptions();
        this.refresh_context_flyout_subscription();
        this.im_client.attach(Some(&this.to_ref()));
        this.attach_document();
        this.update_presenters();
        let weak = this.to_ref().downgrade();
        let selection_subscription = this.selection_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                // Document notifications can rebase the view while an editor transaction
                // is publishing. The editor emits its final selection when that finishes.
                let _ = this.editor().set_selection(this.selection());
                this.notify_input_method_selection();
                this.im_client.raise_cursor_rectangle_changed();
                this.update_command_states();
                this.reset_caret_blink();
                this.ensure_caret_visible();
            }
        });
        *this.selection_subscription.borrow_mut() = Some(selection_subscription);
        this.update_input_options();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        let property = change.property();
        if property == InputElement::is_focused_property().as_property() {
            if change.get_new_value::<bool>() {
                this.set_force_focused_visual_state(false);
            } else if change.get_old_value::<bool>() == Some(true) {
                // KeyboardDevice has already committed the final destination,
                // including any application redirection. Publish this state
                // before focus styles or observers request selection formatting.
                let focused =
                    KeyboardDevice::instance().and_then(|device| device.focused_element());
                this.set_force_focused_visual_state(
                    this.context_flyout_contains_focus(focused.as_deref()),
                );
            }
        }
        Self::parent_on_property_changed(this, change);
        if property == Control::context_flyout_property().as_property() {
            this.refresh_context_flyout_subscription();
        }
        if property == Self::rich_document_property().as_property() {
            this.attach_document();
        }
        if property == Self::is_read_only_property().as_property() {
            let editor = this.editor();
            if this.is_read_only() {
                this.handle_edit(this.cancel_composition());
            }
            editor.set_is_read_only(this.is_read_only());
            this.im_client.request_reset();
            this.update_command_states();
            this.invalidate_text_view();
        } else if property == Self::max_length_property().as_property() {
            this.editor().set_max_length(this.max_length());
        } else if property == Self::undo_limit_property().as_property()
            || property == Self::is_undo_enabled_property().as_property()
        {
            this.editor().set_undo_limit(if this.is_undo_enabled() {
                this.undo_limit()
            } else {
                0
            });
            this.update_command_states();
        }
        if property == Self::input_scope_property().as_property() {
            this.refresh_input_scope_subscription();
        }
        if property == Self::accepts_return_property().as_property()
            || property == Self::character_casing_property().as_property()
            || property == Self::is_text_prediction_enabled_property().as_property()
        {
            this.update_input_options();
        }
        if property == Self::placeholder_text_property().as_property()
            || property == Self::placeholder_foreground_property().as_property()
            || property == Self::header_property().as_property()
            || property == Self::header_template_property().as_property()
            || property == Self::description_property().as_property()
            || property == Self::text_alignment_property().as_property()
            || property == Self::text_wrapping_property().as_property()
        {
            this.update_presenters();
            this.invalidate_measure();
        }
        if property == Self::caret_blink_interval_property().as_property()
            || property == Self::is_read_only_property().as_property()
            || property == InputElement::is_effectively_enabled_property().as_property()
            || property == Visual::is_visible_property().as_property()
        {
            this.reset_caret_blink();
        }
        if property == InputElement::is_effectively_enabled_property().as_property() {
            this.update_command_states();
        }
        if matches!(property.name(), "FontWeight" | "FontStyle" | "TextDecorations") {
            this.update_format_command_states();
        }
        match property.name() {
            "TextAlignment"
            | "TextWrapping"
            | "LineHeight"
            | "TextIndent"
            | "LineStackingStrategy"
            | "TextDecorations"
            | "LetterSpacing"
            | "IsTextSelectionEnabled"
            | "SelectionHighlightColor"
            | "SelectionForegroundBrush"
            | "CaretBrush" => this.sync_presenter_properties(),
            _ => {}
        }
        if property == Self::selection_highlight_color_property().as_property()
            || property == Self::selection_highlight_color_when_not_focused_property().as_property()
        {
            this.refresh_selection_brush_subscriptions();
            this.text_presenter.invalidate_document_layout();
        }
    }
}

impl VisualImpl for RichEditBox {
    fn on_attached_to_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        this.invalidate_context_menu_request();
        Self::parent_on_attached_to_visual_tree(this, event);
        this.reset_caret_blink();
        this.ensure_caret_visible();
    }

    fn on_detached_from_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        this.invalidate_context_menu_request();
        Self::parent_on_detached_from_visual_tree(this, event);
        this.stop_caret_blink();
    }
}

impl InputElementImpl for RichEditBox {
    fn on_got_focus(this: &Self, event: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, event);
        if !this.is_focused() {
            return;
        }
        this.set_force_focused_visual_state(false);
        // Focus does not change the persistent presenter's identity. Actual
        // template attachment changes publish their own view notification;
        // this transition only needs to refresh the current caret geometry.
        this.im_client.raise_cursor_rectangle_changed();
        this.reset_caret_blink();
        this.ensure_caret_visible();
        this.text_presenter.invalidate_document_layout();
    }
    fn on_lost_focus(this: &Self, event: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, event);
        // Popup and embedded-control focus events can pass through their owner.
        // Only the editor's own focus transition changes its editing state.
        if !event.is_source(this) || this.is_focused() {
            return;
        }
        this.text_presenter.focus_hyperlink(None);
        this.stop_caret_blink();
        let retain = this.context_flyout_contains_focus(event.new_focused_element.as_deref());
        this.set_force_focused_visual_state(retain);
        if !retain {
            this.complete_focus_loss();
        }
    }
    fn on_text_input(this: &Self, event: &TextInputEventArgs) {
        if event.handled() {
            return;
        }
        if let Some(commit) = event.input_method_commit() {
            if !commit.is_current() {
                commit.complete(Err(
                    ferroui_base::input::text_input::TextInputMethodError::StateChanged,
                ));
                event.set_handled(true);
                return;
            }
            if let Some(text) = &event.text {
                let result = if let Some(edit) = commit.edit() {
                    this.apply_input_method_edit(edit, false)
                } else if commit.completes_composition() {
                    this.complete_input_method_composition()
                } else {
                    this.insert_text(text)
                };
                commit.complete(
                    result
                        .as_ref()
                        .copied()
                        .map_err(crate::rich_edit_box_text_input_method_client::input_method_error),
                );
                this.handle_edit(result);
            }
        } else if !this.is_read_only() {
            if let Some(text) = &event.text {
                this.handle_edit(this.insert_text(text));
            }
        }
        event.set_handled(true);
    }
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        if event.handled() {
            return;
        }
        if this.editor().is_composing() {
            if event.key == Key::Escape {
                this.handle_edit(this.cancel_composition());
                event.set_handled(true);
            }
            return;
        }
        let keymap = Application::current()
            .and_then(|app| app.platform_settings())
            .map(|settings| settings.hotkey_configuration())
            .unwrap_or_else(|| Rc::new(PlatformHotkeyConfiguration::default()));
        let matches =
            |gestures: &[KeyGesture]| gestures.iter().any(|gesture| gesture.matches(Some(event)));
        if matches(&keymap.copy) {
            this.copy();
        } else if matches(&keymap.cut) {
            this.cut();
        } else if matches(&keymap.paste) {
            this.paste();
        } else if matches(&keymap.undo) {
            if !this.is_read_only() {
                this.handle_edit(this.undo());
            }
        } else if matches(&keymap.redo) {
            if !this.is_read_only() {
                this.handle_edit(this.redo());
            }
        } else if !this.is_read_only()
            && event.key_modifiers == keymap.command_modifiers
            && this.format_accelerator(event.key)
        {
        } else {
            let whole_word = event
                .key_modifiers
                .contains(keymap.whole_word_text_action_modifiers);
            match event.key {
                Key::Back => {
                    if !this.is_read_only() {
                        this.handle_edit(this.delete_backward(whole_word));
                    }
                }
                Key::Delete => {
                    if !this.is_read_only() {
                        this.handle_edit(this.delete_forward(whole_word));
                    }
                }
                Key::Enter if this.accepts_return() => {
                    if !this.is_read_only() {
                        this.handle_edit(this.insert_text("\n"));
                    }
                }
                Key::Tab if this.accepts_tab() => {
                    if !this.is_read_only() {
                        this.handle_edit(this.insert_text("\t"));
                    }
                }
                _ => {
                    Self::parent_on_key_down(this, event);
                    this.text_presenter.on_key_down(event);
                    return;
                }
            }
        }
        event.set_handled(true);
    }

    fn on_key_up(this: &Self, event: &KeyEventArgs) {
        Self::parent_on_key_up(this, event);
        if !event.handled() {
            this.text_presenter.on_key_up(event);
        }
    }

    fn on_pointer_pressed(this: &Self, event: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, event);
        if !event.handled()
            && event
                .get_current_point(Some(this))
                .properties
                .is_left_button_pressed
        {
            this.complete_composition_keeping_text();
            // Template chrome can take focus without moving the document caret.
            this.focus_with(
                ferroui_base::input::NavigationMethod::Pointer,
                event.key_modifiers(),
            );
        }
    }
}

ferroui_base::ferro_properties! { impl RichEditBox {
    /// The shared document the editor edits (`RichDocument`, a FerroUI addition, D3).
    /// WinUI's `Document` is the text object model: [`RichEditBox::document`].
    pub fn rich_document_property() -> DirectProperty<RichEditBox, RichTextDocument> {
        RichTextBlock::rich_document_property().add_owner::<RichEditBox>(Self::rich_document, Some(Self::set_rich_document), None)
    }
    pub fn selection_start_property() -> DirectProperty<RichEditBox, i32> {
        RichTextBlock::selection_start_property().add_owner::<RichEditBox>(
            |this| this.selection_start().min(i32::MAX as usize) as i32,
            Some(|this, value| this.set_selection_start(value.max(0) as usize)), None)
    }
    pub fn selection_end_property() -> DirectProperty<RichEditBox, i32> {
        RichTextBlock::selection_end_property().add_owner::<RichEditBox>(
            |this| this.selection_end().min(i32::MAX as usize) as i32,
            Some(|this, value| this.set_selection_end(value.max(0) as usize)), None)
    }
    pub fn selected_text_property() -> DirectProperty<RichEditBox, String> {
        RichTextBlock::selected_text_property().add_owner::<RichEditBox>(Self::selected_text, None, None)
    }
    pub fn is_text_selection_enabled_property() -> StyledProperty<bool> { RichTextBlock::is_text_selection_enabled_property().add_owner::<RichEditBox>() }
    pub fn selection_highlight_color_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { RichTextBlock::selection_highlight_color_property().add_owner::<RichEditBox>() }
    pub fn selection_highlight_color_when_not_focused_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<RichEditBox, _>("SelectionHighlightColorWhenNotFocused", Some(Brushes::transparent()))
    }
    pub fn selection_foreground_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { RichTextBlock::selection_foreground_brush_property().add_owner::<RichEditBox>() }
    pub fn caret_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { RichTextBlock::caret_brush_property().add_owner::<RichEditBox>() }
    pub fn letter_spacing_property() -> StyledProperty<f64> { RichTextBlock::letter_spacing_property().add_owner::<RichEditBox>() }
    pub fn language_property() -> StyledProperty<ferroui_base::utilities::CultureInfo> { RichTextBlock::language_property().add_owner::<RichEditBox>() }
    pub fn text_alignment_property() -> StyledProperty<TextAlignment> { RichTextBlock::text_alignment_property().add_owner::<RichEditBox>() }
    pub fn text_wrapping_property() -> StyledProperty<TextWrapping> { RichTextBlock::text_wrapping_property().add_owner::<RichEditBox>() }
    pub fn line_height_property() -> StyledProperty<f64> { RichTextBlock::line_height_property().add_owner::<RichEditBox>() }
    pub fn text_indent_property() -> StyledProperty<f64> { RichTextBlock::text_indent_property().add_owner::<RichEditBox>() }
    pub fn line_stacking_strategy_property() -> StyledProperty<LineStackingStrategy> { RichTextBlock::line_stacking_strategy_property().add_owner::<RichEditBox>() }
    pub fn text_decorations_property() -> StyledProperty<Option<TextDecorationCollection>> { RichTextBlock::text_decorations_property().add_owner::<RichEditBox>() }
    pub fn is_read_only_property() -> StyledProperty<bool> { FerroProperty::register::<RichEditBox, _>("IsReadOnly", false) }
    pub fn accepts_return_property() -> StyledProperty<bool> { FerroProperty::register::<RichEditBox, _>("AcceptsReturn", true) }
    pub fn accepts_tab_property() -> StyledProperty<bool> { FerroProperty::register::<RichEditBox, _>("AcceptsTab", false) }
    pub fn caret_blink_interval_property() -> StyledProperty<TimeSpan> { TextBox::caret_blink_interval_property().add_owner::<RichEditBox>() }
    pub fn max_length_property() -> StyledProperty<i32> { FerroProperty::register_with::<RichEditBox, _>("MaxLength", StyledPropertyOptions::new(0).validate(|value| *value >= 0)) }
    pub fn clipboard_copy_format_property() -> StyledProperty<RichEditClipboardFormat> { FerroProperty::register::<RichEditBox, _>("ClipboardCopyFormat", RichEditClipboardFormat::AllFormats) }
    pub fn disabled_formatting_accelerators_property() -> StyledProperty<DisabledFormattingAccelerators> { FerroProperty::register::<RichEditBox, _>("DisabledFormattingAccelerators", DisabledFormattingAccelerators::NONE) }
    pub fn character_casing_property() -> StyledProperty<RichEditCharacterCasing> { FerroProperty::register::<RichEditBox, _>("CharacterCasing", RichEditCharacterCasing::Normal) }
    pub fn is_text_prediction_enabled_property() -> StyledProperty<bool> { FerroProperty::register::<RichEditBox, _>("IsTextPredictionEnabled", true) }
    pub fn input_scope_property() -> StyledProperty<Option<Ref<InputScope>>> { FerroProperty::register::<RichEditBox, _>("InputScope", None) }
    pub fn placeholder_text_property() -> StyledProperty<String> { FerroProperty::register::<RichEditBox, _>("PlaceholderText", String::new()) }
    pub fn placeholder_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { FerroProperty::register::<RichEditBox, _>("PlaceholderForeground", None) }
    pub fn header_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<RichEditBox, _>("Header", None) }
    pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> { FerroProperty::register::<RichEditBox, _>("HeaderTemplate", None) }
    pub fn description_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<RichEditBox, _>("Description", None) }
    pub fn is_undo_enabled_property() -> StyledProperty<bool> { FerroProperty::register::<RichEditBox, _>("IsUndoEnabled", true) }
    pub fn undo_limit_property() -> StyledProperty<i32> { FerroProperty::register_with::<RichEditBox, _>("UndoLimit", StyledPropertyOptions::new(100).validate(|value| *value >= 0)) }
    pub fn can_undo_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("CanUndo", |this| this.can_undo.get(), None, false) }
    pub fn can_redo_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("CanRedo", |this| this.can_redo.get(), None, false) }
    pub fn can_copy_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("CanCopy", |this| this.can_copy.get(), None, false) }
    pub fn can_cut_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("CanCut", |this| this.can_cut.get(), None, false) }
    pub fn can_paste_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("CanPaste", |this| this.can_paste.get(), None, false) }
    pub fn can_format_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("CanFormat", Self::can_format, None, true) }
    pub fn bold_state_property() -> DirectProperty<RichEditBox, Option<bool>> { FerroProperty::register_direct::<RichEditBox, _>("BoldState", Self::bold_state, None, Some(false)) }
    pub fn italic_state_property() -> DirectProperty<RichEditBox, Option<bool>> { FerroProperty::register_direct::<RichEditBox, _>("ItalicState", Self::italic_state, None, Some(false)) }
    pub fn underline_state_property() -> DirectProperty<RichEditBox, Option<bool>> { FerroProperty::register_direct::<RichEditBox, _>("UnderlineState", Self::underline_state, None, Some(false)) }
    pub fn is_bold_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("IsBold", Self::is_bold, None, false) }
    pub fn is_italic_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("IsItalic", Self::is_italic, None, false) }
    pub fn is_underlined_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("IsUnderlined", Self::is_underlined, None, false) }
    pub fn text_property() -> DirectProperty<RichEditBox, String> {
        FerroProperty::register_direct::<RichEditBox, String>("Text", |this| this.text(), Some(|this, value| {
            // A TwoWay binding echoes a document change through this setter.
            // Equal text must not flatten formatting, reset selection, or clear
            // history while an edit or undo is publishing its new content.
            if this.text() != value {
                this.handle_edit(this.set_text(&value));
            }
        }), String::new())
    }
} }

impl RichEditBox {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<RichEditBox>(true);
        InputElement::context_requested_event().add_class_handler_with::<RichEditBox>(
            Self::on_context_requested,
            RoutingStrategies::BUBBLE,
            false,
        );
        InputElement::text_input_method_client_requested_event().add_class_handler::<RichEditBox>(
            |this, event| {
                if !this.is_read_only() && event.source().as_ref() == Some(&this.to_ref().upcast())
                {
                    event.set_client(Some(this.im_client.clone()));
                }
            },
        );
    }
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            text_presenter: RichEditBoxTextPresenter::new(),
            content_element: RefCell::new(None),
            applied_template: RefCell::new(None),
            editor: RefCell::new(None),
            composition_operation_depth: Cell::new(0),
            im_client: RichEditBoxTextInputMethodClient::new(),
            document_subscription: RefCell::new(None),
            selection_subscription: RefCell::new(None),
            editor_subscription: RefCell::new(None),
            input_scope_subscription: RefCell::new(None),
            input_scope_owns_options: Cell::new(false),
            projecting_selection: Cell::new(false),
            last_input_selection: Cell::new(EditorSelection::default()),
            last_text: RefCell::new(String::new()),
            document_generation: Cell::new(0),
            header_presenter: RefCell::new(None),
            description_presenter: RefCell::new(None),
            placeholder_presenter: RefCell::new(None),
            selection_brush_subscriptions: RefCell::new(Vec::new()),
            context_flyout_closed_subscription: RefCell::new(None),
            flyout_focus_subscription: RefCell::new(None),
            context_menu_request_generation: Cell::new(0),
            force_focused_visual_state: Cell::new(false),
            can_undo: Cell::new(false),
            can_redo: Cell::new(false),
            can_copy: Cell::new(false),
            can_cut: Cell::new(false),
            can_paste: Cell::new(true),
            can_format: Cell::new(true),
            formatting_state: Cell::new(format_commands::FormattingState::default()),
            errors: Rc::new(HandlerList::new()),
            clipboard_errors: Rc::new(HandlerList::new()),
            caret_scroll_pending: Cell::new(false),
            caret_timer: RefCell::new(None),
            caret_phase: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] = &[
        TemplatePartAttribute::new("ContentElement", <Control as StaticType>::TYPE),
        TemplatePartAttribute::new(
            "HeaderContentPresenter",
            <ContentPresenter as StaticType>::TYPE,
        ),
        TemplatePartAttribute::new(
            "PlaceholderTextContentPresenter",
            <TextBlock as StaticType>::TYPE,
        ),
        TemplatePartAttribute::new(
            "DescriptionPresenter",
            <ContentPresenter as StaticType>::TYPE,
        ),
    ];
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[
        ":header",
        ":description",
        ":empty",
        ":placeholder",
        ":flyout-focus",
    ]);

    /// The persistent rich document view used by the current control template.
    pub fn text_presenter(&self) -> Ref<RichEditBoxTextPresenter> {
        self.text_presenter.clone()
    }
    /// The supported content host found in the applied template, if present.
    pub fn content_element(&self) -> Option<Ref<Control>> {
        self.content_element.borrow().clone()
    }
    /// The shared document the editor edits (the property `RichDocument`, D3).
    pub fn rich_document(&self) -> RichTextDocument {
        self.text_presenter.rich_document()
    }
    /// Sets the shared document the editor edits.
    pub fn set_rich_document(&self, document: RichTextDocument) {
        self.text_presenter.set_rich_document(document);
    }
    /// Attaches a shared document whose blocks no other view owns.
    pub fn try_set_rich_document(&self, document: RichTextDocument) -> bool {
        self.text_presenter.try_set_rich_document(document)
    }
    /// WinUI's `RichEditBox.Document`: the text object model of the content, the same
    /// object [`text_document`](Self::text_document) returns (`docs/porting/winui-rich-text.md`, D3).
    pub fn document(&self) -> RichEditTextDocument {
        self.text_document()
    }
    pub fn blocks(&self) -> BlockCollection {
        self.text_presenter.blocks()
    }
    pub fn text_highlighters(&self) -> TextHighlighterCollection {
        self.text_presenter.text_highlighters()
    }
    /// The selection of the editor while one exists, so that handlers of TextChanging
    /// and the input method see the selection of the edit being made, not the view's
    /// earlier one (#243 review, N4).
    pub fn selection_start(&self) -> usize {
        match self.editor.borrow().as_ref() {
            Some(editor) => editor.selection().anchor,
            None => self.text_presenter.selection_start(),
        }
    }
    pub fn set_selection_start(&self, value: usize) {
        self.select(value, self.selection_end());
    }
    pub fn selection_end(&self) -> usize {
        match self.editor.borrow().as_ref() {
            Some(editor) => editor.selection().active,
            None => self.text_presenter.selection_end(),
        }
    }
    pub fn set_selection_end(&self, value: usize) {
        self.select(self.selection_start(), value);
    }
    pub fn caret_index(&self) -> usize {
        self.selection_end()
    }
    pub fn select(&self, start: usize, end: usize) {
        self.text_presenter.select(start, end);
    }
    pub fn clear_selection(&self) {
        self.text_presenter.clear_selection();
    }
    pub fn selected_text(&self) -> String {
        self.text_presenter.selected_text()
    }
    pub fn selection_changed(
        &self,
        handler: impl Fn(&RichTextSelectionChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.text_presenter.selection_changed(handler)
    }
    pub fn is_text_selection_enabled(&self) -> bool {
        self.get_value(Self::is_text_selection_enabled_property())
    }
    pub fn set_is_text_selection_enabled(&self, value: bool) {
        self.set_value(Self::is_text_selection_enabled_property(), value);
    }
    pub fn selection_highlight_color(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::selection_highlight_color_property())
    }
    pub fn set_selection_highlight_color(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::selection_highlight_color_property(), value);
    }
    pub fn selection_highlight_color_when_not_focused(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::selection_highlight_color_when_not_focused_property())
    }
    pub fn set_selection_highlight_color_when_not_focused(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(
            Self::selection_highlight_color_when_not_focused_property(),
            value,
        );
    }
    pub fn selection_foreground_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::selection_foreground_brush_property())
    }
    pub fn set_selection_foreground_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::selection_foreground_brush_property(), value);
    }
    pub fn caret_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::caret_brush_property())
    }
    pub fn set_caret_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::caret_brush_property(), value);
    }
    pub fn letter_spacing(&self) -> f64 {
        self.get_value(Self::letter_spacing_property())
    }
    pub fn language(&self) -> ferroui_base::utilities::CultureInfo {
        self.get_value(Self::language_property())
    }
    pub fn set_language(&self, value: ferroui_base::utilities::CultureInfo) {
        self.set_value(Self::language_property(), value);
    }
    pub fn set_letter_spacing(&self, value: f64) {
        self.set_value(Self::letter_spacing_property(), value);
    }
    pub fn text_alignment(&self) -> TextAlignment {
        self.get_value(Self::text_alignment_property())
    }
    pub fn set_text_alignment(&self, value: TextAlignment) {
        self.set_value(Self::text_alignment_property(), value);
    }
    pub fn text_wrapping(&self) -> TextWrapping {
        self.get_value(Self::text_wrapping_property())
    }
    pub fn set_text_wrapping(&self, value: TextWrapping) {
        self.set_value(Self::text_wrapping_property(), value);
    }
    pub fn line_height(&self) -> f64 {
        self.get_value(Self::line_height_property())
    }
    pub fn set_line_height(&self, value: f64) {
        self.set_value(Self::line_height_property(), value);
    }
    pub fn text_indent(&self) -> f64 {
        self.get_value(Self::text_indent_property())
    }
    pub fn set_text_indent(&self, value: f64) {
        self.set_value(Self::text_indent_property(), value);
    }
    pub fn line_stacking_strategy(&self) -> LineStackingStrategy {
        self.get_value(Self::line_stacking_strategy_property())
    }
    pub fn set_line_stacking_strategy(&self, value: LineStackingStrategy) {
        self.set_value(Self::line_stacking_strategy_property(), value);
    }
    pub fn text_decorations(&self) -> Option<TextDecorationCollection> {
        self.get_value(Self::text_decorations_property())
    }
    pub fn set_text_decorations(&self, value: Option<TextDecorationCollection>) {
        self.set_value(Self::text_decorations_property(), value);
    }

    /// The caret rectangle in the owning control's coordinate system.
    pub fn cursor_rectangle(&self) -> Rect {
        self.rect_from_presenter(self.text_presenter.cursor_rectangle())
    }
    pub fn get_rect_from_character_index(&self, index: usize) -> Rect {
        self.rect_from_presenter(self.text_presenter.get_rect_from_character_index(index))
    }
    pub fn get_rect_from_text_position(&self, position: TextPosition) -> Rect {
        self.rect_from_presenter(self.text_presenter.get_rect_from_text_position(position))
    }
    pub fn selection_rects(&self) -> Vec<Rect> {
        self.get_rects_for_range(
            self.selection_start().min(self.selection_end())
                ..self.selection_start().max(self.selection_end()),
        )
    }
    pub fn get_rects_for_range(&self, range: Range<usize>) -> Vec<Rect> {
        self.text_presenter
            .get_rects_for_range(range)
            .into_iter()
            .map(|rect| self.rect_from_presenter(rect))
            .collect()
    }
    pub fn hit_test_text_position(&self, point: Point) -> usize {
        let point = self
            .transform_to_visual(&self.text_presenter)
            .map_or(point, |transform| point.transform(transform));
        self.text_presenter.hit_test_text_position(point)
    }
    pub fn line_metrics(&self) -> Vec<RichTextLineMetrics> {
        self.text_presenter
            .line_metrics()
            .into_iter()
            .map(|mut line| {
                let baseline = self.rect_from_presenter(Rect::new(0.0, line.baseline, 0.0, 0.0));
                line.bounds = self.rect_from_presenter(line.bounds);
                line.baseline = baseline.y;
                line
            })
            .collect()
    }
    fn rect_from_presenter(&self, rect: Rect) -> Rect {
        self.text_presenter
            .transform_to_visual(self)
            .map_or(rect, |transform| rect.transform_to_aabb(transform))
    }

    fn sync_presenter_properties(&self) {
        let presenter = &self.text_presenter;
        presenter.set_is_text_selection_enabled(self.is_text_selection_enabled());
        presenter.set_selection_highlight_color(self.selection_highlight_color());
        presenter.set_selection_foreground_brush(self.selection_foreground_brush());
        presenter.set_caret_brush(self.caret_brush());
        presenter.set_text_alignment(self.text_alignment());
        presenter.set_text_wrapping(self.text_wrapping());
        presenter.set_line_height(self.line_height());
        presenter.set_text_indent(self.text_indent());
        presenter.set_line_stacking_strategy(self.line_stacking_strategy());
        presenter.set_text_decorations(self.text_decorations());
        presenter.set_letter_spacing(self.letter_spacing());
    }

    pub(crate) fn on_text_view_property_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        let property = change.property();
        if property == RichTextBlock::rich_document_property().as_property() {
            let (old, new) = change.get_old_and_new_value::<RichTextDocument>();
            self.raise_direct_property_changed(Self::rich_document_property(), &old, &new);
        } else if property == RichTextBlock::selection_start_property().as_property() {
            let (old, new) = change.get_old_and_new_value::<i32>();
            self.raise_direct_property_changed(Self::selection_start_property(), &old, &new);
        } else if property == RichTextBlock::selection_end_property().as_property() {
            let (old, new) = change.get_old_and_new_value::<i32>();
            self.raise_direct_property_changed(Self::selection_end_property(), &old, &new);
        } else if property == RichTextBlock::selected_text_property().as_property() {
            let (old, new) = change.get_old_and_new_value::<String>();
            self.raise_direct_property_changed(Self::selected_text_property(), &old, &new);
        }
    }
    pub(crate) fn request_presenter_selection_change(&self, start: usize, end: usize) -> bool {
        if self.projecting_selection.get() {
            return false;
        }
        let editor = self.editor();
        let selection = EditorSelection::new(start, end);
        if editor.is_in_operation() || editor.selection() == selection {
            return false;
        }
        self.handle_edit(editor.set_selection(selection));
        true
    }
    pub(crate) fn presenter_selection_changing(
        &self,
        args: &RichTextSelectionChangedEventArgs,
    ) -> bool {
        let args = RichEditBoxSelectionChangingEventArgs {
            base: RoutedEventArgs::with_event(Self::selection_changing_event()),
            selection_start: args.start.min(args.end),
            selection_length: args.start.abs_diff(args.end),
            cancel: Rc::new(Cell::new(false)),
        };
        self.raise_event(&args);
        !args.cancel()
    }
    pub(crate) fn rebase_presenter_selection_on_document_change(&self) -> bool {
        let editor = self.editor.borrow().clone();
        !editor.is_some_and(|editor| editor.is_in_operation())
    }
    pub(crate) fn raise_presenter_selection_changed(&self) {
        self.raise_event(&RoutedEventArgs::with_event(Self::selection_changed_event()));
    }
    pub(crate) fn active_selection_brush(&self) -> Option<Rc<dyn IBrush>> {
        if self.is_focused() || self.force_focused_visual_state.get() {
            self.selection_highlight_color()
        } else {
            self.selection_highlight_color_when_not_focused()
        }
    }
    pub(crate) fn context_flyout_contains_focus(&self, focused: Option<&InputElement>) -> bool {
        let Some(flyout) = self.context_flyout().filter(|flyout| {
            flyout.is_open() && flyout.target() == Some(self.to_ref().upcast::<Control>())
        }) else {
            return false;
        };
        let Some(presenter) = flyout
            .cast::<PopupFlyoutBase>()
            .and_then(|flyout| flyout.popup().child())
        else {
            return false;
        };
        let presenter: Ref<Visual> = presenter.upcast();
        let mut current = focused.map(|focused| focused.to_ref().upcast::<Visual>());
        while let Some(visual) = current {
            if visual == presenter {
                return true;
            }
            current = visual.visual_parent();
        }
        false
    }

    fn set_force_focused_visual_state(&self, value: bool) {
        if self.force_focused_visual_state.replace(value) == value {
            return;
        }
        self.pseudo_classes().set(":flyout-focus", value);
        self.text_presenter.invalidate_document_layout();
        let previous = self.flyout_focus_subscription.take();
        if let Some(previous) = previous {
            previous.dispose();
        }
        if value {
            if let Some(device) = KeyboardDevice::instance() {
                let weak = self.to_ref().downgrade();
                *self.flyout_focus_subscription.borrow_mut() =
                    Some(device.property_changed(move |property| {
                        if property == "FocusedElement" {
                            if let Some(owner) = weak.upgrade() {
                                owner.refresh_flyout_focus_state();
                            }
                        }
                    }));
            }
        }
    }

    fn refresh_flyout_focus_state(&self) {
        if !self.force_focused_visual_state.get() {
            return;
        }
        if self.is_focused() {
            self.set_force_focused_visual_state(false);
        } else {
            let focused = KeyboardDevice::instance().and_then(|device| device.focused_element());
            if !self.context_flyout_contains_focus(focused.as_deref()) {
                self.complete_focus_loss();
            }
        }
    }

    fn refresh_context_flyout_subscription(&self) {
        let previous = self.context_flyout_closed_subscription.take();
        if let Some(previous) = previous {
            previous.dispose();
        }
        if let Some(flyout) = self.context_flyout() {
            let weak = self.to_ref().downgrade();
            *self.context_flyout_closed_subscription.borrow_mut() =
                Some(flyout.closed(move || {
                    if let Some(owner) = weak.upgrade() {
                        owner.refresh_flyout_focus_state();
                    }
                }));
        }
        self.refresh_flyout_focus_state();
    }

    fn complete_focus_loss(&self) {
        self.set_force_focused_visual_state(false);
        self.complete_composition_keeping_text();
        self.editor().checkpoint();
        self.im_client.request_reset();
        self.stop_caret_blink();
        self.text_presenter.invalidate_document_layout();
    }

    pub(crate) fn selection_is_visible(&self) -> bool {
        self.active_selection_brush().is_some_and(|brush| {
            brush.opacity() > 0.0
                && brush
                    .as_solid_color_brush()
                    .is_none_or(|solid| solid.color().a != 0)
        })
    }
    fn refresh_selection_brush_subscriptions(&self) {
        let old = std::mem::take(&mut *self.selection_brush_subscriptions.borrow_mut());
        for subscription in old {
            subscription.dispose();
        }
        let mut subscriptions = Vec::new();
        for brush in [
            self.selection_highlight_color(),
            self.selection_highlight_color_when_not_focused(),
        ] {
            let weak = self.to_ref().downgrade();
            if let Some(subscription) = subscribe_invalidated(
                &brush,
                Rc::new(move || {
                    if let Some(owner) = weak.upgrade() {
                        owner.text_presenter.invalidate_document_layout();
                    }
                }),
            ) {
                subscriptions.push(subscription);
            }
        }
        *self.selection_brush_subscriptions.borrow_mut() = subscriptions;
    }
    pub(crate) fn on_text_view_geometry_changed(&self) {
        if self.text_presenter.has_measured_layout() {
            // Automatic association starts only once a real view constraint is
            // available. It survives later invalidation and offscreen reflow.
            self.editor()
                .set_text_view(Some(self.text_presenter.clone().upcast()))
                .expect("The presenter and editor share one document.");
        }
        self.im_client.raise_cursor_rectangle_changed();
    }
    fn invalidate_text_view(&self) {
        self.text_presenter.invalidate_visual();
    }
    pub fn is_read_only(&self) -> bool {
        self.get_value(Self::is_read_only_property())
    }
    pub fn set_is_read_only(&self, value: bool) {
        self.set_value(Self::is_read_only_property(), value);
    }
    /// The framework caret period. Zero or a negative interval keeps the caret steady.
    pub fn caret_blink_interval(&self) -> TimeSpan {
        self.get_value(Self::caret_blink_interval_property())
    }
    pub fn set_caret_blink_interval(&self, value: TimeSpan) {
        self.set_value(Self::caret_blink_interval_property(), value);
    }
    pub fn accepts_return(&self) -> bool {
        self.get_value(Self::accepts_return_property())
    }
    pub fn set_accepts_return(&self, value: bool) {
        self.set_value(Self::accepts_return_property(), value);
    }
    pub fn accepts_tab(&self) -> bool {
        self.get_value(Self::accepts_tab_property())
    }
    pub fn set_accepts_tab(&self, value: bool) {
        self.set_value(Self::accepts_tab_property(), value);
    }
    pub fn max_length(&self) -> usize {
        self.get_value(Self::max_length_property()) as usize
    }
    pub fn set_max_length(&self, value: usize) {
        self.set_value(
            Self::max_length_property(),
            value.min(i32::MAX as usize) as i32,
        );
    }
    pub fn clipboard_copy_format(&self) -> RichEditClipboardFormat {
        self.get_value(Self::clipboard_copy_format_property())
    }
    pub fn set_clipboard_copy_format(&self, value: RichEditClipboardFormat) {
        self.set_value(Self::clipboard_copy_format_property(), value);
    }
    pub fn disabled_formatting_accelerators(&self) -> DisabledFormattingAccelerators {
        self.get_value(Self::disabled_formatting_accelerators_property())
    }
    pub fn set_disabled_formatting_accelerators(&self, value: DisabledFormattingAccelerators) {
        self.set_value(Self::disabled_formatting_accelerators_property(), value);
    }
    pub fn character_casing(&self) -> RichEditCharacterCasing {
        self.get_value(Self::character_casing_property())
    }
    pub fn set_character_casing(&self, value: RichEditCharacterCasing) {
        self.set_value(Self::character_casing_property(), value);
    }
    pub fn is_text_prediction_enabled(&self) -> bool {
        self.get_value(Self::is_text_prediction_enabled_property())
    }
    pub fn set_is_text_prediction_enabled(&self, value: bool) {
        self.set_value(Self::is_text_prediction_enabled_property(), value);
    }
    pub fn input_scope(&self) -> Option<Ref<InputScope>> {
        self.get_value(Self::input_scope_property())
    }
    pub fn set_input_scope(&self, value: impl Into<Option<Ref<InputScope>>>) {
        self.set_value(Self::input_scope_property(), value.into());
    }
    pub fn placeholder_text(&self) -> String {
        self.get_value(Self::placeholder_text_property())
    }
    pub fn set_placeholder_text(&self, value: impl Into<String>) {
        self.set_value(Self::placeholder_text_property(), value.into());
    }
    pub fn placeholder_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::placeholder_foreground_property())
    }
    pub fn set_placeholder_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::placeholder_foreground_property(), value);
    }
    pub fn header(&self) -> Option<BoxedValue> {
        self.get_value(Self::header_property())
    }
    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::header_property(), value);
    }
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }
    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value);
    }
    pub fn description(&self) -> Option<BoxedValue> {
        self.get_value(Self::description_property())
    }
    pub fn set_description(&self, value: Option<BoxedValue>) {
        self.set_value(Self::description_property(), value);
    }
    /// The framework presenter for header data or a HeaderTemplate.
    pub fn header_presenter(&self) -> Option<Ref<ContentPresenter>> {
        self.header_presenter.borrow().clone()
    }
    pub fn description_presenter(&self) -> Option<Ref<ContentPresenter>> {
        self.description_presenter.borrow().clone()
    }
    pub fn placeholder_presenter(&self) -> Option<Ref<TextBlock>> {
        self.placeholder_presenter.borrow().clone()
    }

    fn attach_to_content_element(&self, host: Ref<Control>) {
        let presenter: Ref<Control> = self.text_presenter.clone().upcast();
        // A detached presenter inherits owner font defaults. The real content
        // host becomes its inheritance parent once it enters the template.
        self.text_presenter
            .set_inheritance_parent(None::<Ref<ferroui_base::FerroObject>>);
        let attached = if let Some(content) = host.cast::<ContentControl>() {
            content.set_content(Some(Control::boxed(presenter.clone())));
            true
        } else if let Some(content) = host.cast::<ContentPresenter>() {
            content.set_content(Some(Control::boxed(presenter.clone())));
            content.update_child();
            true
        } else if let Some(decorator) = host.cast::<Decorator>() {
            decorator.set_child(presenter.clone());
            true
        } else if let Some(panel) = host.cast::<Panel>() {
            panel.children().add(presenter);
            true
        } else {
            false
        };
        if attached {
            self.text_presenter.set_templated_parent(self.to_ref());
            *self.content_element.borrow_mut() = Some(host);
        } else {
            self.text_presenter.set_inheritance_parent(self.to_ref());
        }
    }

    fn release_template_parts(&self) {
        let old = self.content_element.borrow_mut().take();
        let presenter: Ref<Control> = self.text_presenter.clone().upcast();
        if let Some(host) = old {
            if let Some(content) = host.cast::<ContentControl>() {
                if content
                    .content()
                    .as_ref()
                    .and_then(Control::from_boxed)
                    .as_ref()
                    == Some(&presenter)
                {
                    content.set_content(None);
                    if let Some(view) = content.presenter() {
                        view.update_child();
                    }
                }
            } else if let Some(content) = host.cast::<ContentPresenter>() {
                if content
                    .content()
                    .as_ref()
                    .and_then(Control::from_boxed)
                    .as_ref()
                    == Some(&presenter)
                {
                    content.set_content(None);
                    content.update_child();
                }
            } else if let Some(decorator) = host.cast::<Decorator>() {
                if decorator.child().as_ref() == Some(&presenter) {
                    decorator.set_child(None::<Ref<Control>>);
                }
            } else if let Some(panel) = host.cast::<Panel>() {
                panel.children().remove(&presenter);
            }
        }
        if let Some(parent) = self.text_presenter.visual_parent() {
            parent
                .visual_children()
                .remove(&self.text_presenter.clone().upcast());
        }
        self.text_presenter
            .set_parent(None::<Ref<ferroui_base::StyledElement>>);
        self.text_presenter
            .set_templated_parent(None::<Ref<ferroui_base::FerroObject>>);
        self.text_presenter.set_inheritance_parent(self.to_ref());
        self.header_presenter.borrow_mut().take();
        self.description_presenter.borrow_mut().take();
        self.placeholder_presenter.borrow_mut().take();
        self.stop_caret_blink();
        self.im_client.raise_text_view_visual_changed();
    }

    fn update_presenters(&self) {
        let show_header = self.header().is_some() || self.header_template().is_some();
        if let Some(presenter) = self.header_presenter() {
            presenter.set_is_visible(show_header);
        }
        let show_description = self.description().is_some();
        if let Some(presenter) = self.description_presenter() {
            presenter.set_is_visible(show_description);
        }
        self.pseudo_classes().set(":header", show_header);
        self.pseudo_classes().set(":description", show_description);
        self.update_placeholder_visibility();
    }

    fn update_placeholder_visibility(&self) {
        let empty = self.rich_document().text_length() == 0;
        self.pseudo_classes().set(":empty", empty);
        self.pseudo_classes()
            .set(":placeholder", empty && !self.placeholder_text().is_empty());
        if let Some(placeholder) = self.placeholder_presenter() {
            placeholder.set_is_visible(empty && !self.placeholder_text().is_empty());
        }
    }
    pub fn is_undo_enabled(&self) -> bool {
        self.get_value(Self::is_undo_enabled_property())
    }
    pub fn set_is_undo_enabled(&self, value: bool) {
        self.set_value(Self::is_undo_enabled_property(), value);
    }
    pub fn undo_limit(&self) -> usize {
        self.get_value(Self::undo_limit_property()) as usize
    }
    pub fn set_undo_limit(&self, value: usize) {
        self.set_value(
            Self::undo_limit_property(),
            value.min(i32::MAX as usize) as i32,
        );
    }
    pub fn can_undo(&self) -> bool {
        self.editor().can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.editor().can_redo()
    }
    pub fn can_copy(&self) -> bool {
        self.selection_start() != self.selection_end()
    }
    pub fn can_cut(&self) -> bool {
        self.can_copy() && !self.is_read_only()
    }
    pub fn can_paste(&self) -> bool {
        !self.is_read_only() && self.is_effectively_enabled()
    }
    pub fn text(&self) -> String {
        self.rich_document().text()
    }
    /// Text-object editing services sharing the view's document, selection, and undo history.
    pub fn text_document(&self) -> RichEditTextDocument {
        RichEditTextDocument::from_editor(self.editor())
    }

    /// Replaces all content programmatically. The public document is not limited by the UI's
    /// input policies. Selection and user undo history are reset after replacement.
    pub fn set_text(&self, text: &str) -> Result<(), EditError> {
        self.rich_document()
            .replace_text(0..self.rich_document().text_length(), text)?;
        self.editor().clear_history();
        self.select(0, 0);
        self.update_command_states();
        Ok(())
    }

    pub fn editor(&self) -> Rc<RichTextEditor> {
        let existing = self.editor.borrow().clone();
        if let Some(editor) = existing {
            if editor.rich_document() == self.rich_document() {
                return editor;
            }
            editor.set_selection_projection(None);
            editor
                .set_text_view(None)
                .expect("Removing a text view cannot change its document.");
        }
        let editor = Rc::new(RichTextEditor::new(self.rich_document()));
        if self.text_presenter.has_measured_layout() {
            editor
                .set_text_view(Some(self.text_presenter.clone().upcast()))
                .expect("The presenter and editor share one document.");
        }
        editor.set_is_read_only(self.is_read_only());
        editor.set_max_length(self.max_length());
        editor.set_undo_limit(if self.is_undo_enabled() {
            self.undo_limit()
        } else {
            0
        });
        *self.editor.borrow_mut() = Some(editor.clone());
        let weak = self.to_ref().downgrade();
        editor.set_selection_projection(Some(Rc::new(move |selection| {
            if let Some(this) = weak.upgrade() {
                struct Projection<'a>(&'a Cell<bool>, bool);
                impl Drop for Projection<'_> {
                    fn drop(&mut self) {
                        self.0.set(self.1);
                    }
                }
                let guard = Projection(
                    &this.projecting_selection,
                    this.projecting_selection.replace(true),
                );
                this.select(selection.anchor, selection.active);
                let accepted = this.selection();
                drop(guard);
                accepted
            } else {
                selection
            }
        })));
        let old = self.editor_subscription.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        let weak = self.to_ref().downgrade();
        let subscription = editor.changed(move || {
            if let Some(this) = weak.upgrade() {
                this.after_edit(&this.editor());
            }
        });
        // The control observes accepted causes before a client ever requests
        // its peer. Creating a peer while handling the first cause must not
        // install another forwarding subscription for that same model.
        let weak = self.to_ref().downgrade();
        let weak_editor = Rc::downgrade(&editor);
        let composition_subscription = editor.composition_changed(move |change| {
            let (Some(this), Some(editor)) = (weak.upgrade(), weak_editor.upgrade()) else {
                return;
            };
            let current = this
                .editor
                .borrow()
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, &editor));
            if current && editor.rich_document() == this.rich_document() {
                this.raise_automation_composition_changed(change);
                if let RichTextCompositionChange::Finalized { text } = change {
                    if this.composition_operation_depth.get() == 0 {
                        this.on_implicit_composition_end(text);
                    }
                }
            }
        });
        *self.editor_subscription.borrow_mut() =
            Some(Rc::new(CompositeDisposable::from_disposables([
                subscription,
                composition_subscription,
            ])));
        editor
    }

    /// A composition the editor ended outside the control's own composition operations
    /// (TextBoxBase.cpp raises TextCompositionEnded and resets the input method for it).
    fn on_implicit_composition_end(&self, text: &str) {
        let start = self.editor().selection().range().start;
        self.raise_composition(Self::text_composition_ended_event(), text, !text.is_empty(), start..start);
        self.im_client.request_reset();
    }

    /// Ends a composition keeping its text, as RichEdit does when the user clicks or
    /// focus leaves (TextBoxBase.cpp terminates the composition on left button down and
    /// WM_KILLFOCUS; #243 review, S5), then resets the input method.
    fn complete_composition_keeping_text(&self) {
        let editor = self.editor();
        if !editor.is_composing() {
            return;
        }
        if self.complete_input_method_composition().is_err() && editor.is_composing() {
            let text = editor
                .active_composition_range()
                .and_then(|range| editor.rich_document().get_text(range).ok())
                .unwrap_or_default();
            self.handle_edit(self.commit_composition(&text));
        }
        self.im_client.request_reset();
    }

    fn raise_automation_composition_changed(&self, change: &RichTextCompositionChange) {
        let (cause, text) = match change {
            RichTextCompositionChange::Preview { changed_text } => {
                (TextEditChangeType::Composition, changed_text)
            }
            RichTextCompositionChange::Finalized { text } => {
                (TextEditChangeType::CompositionFinalized, text)
            }
        };
        self.get_or_create_automation_peer()
            .raise_text_edit_text_changed_event(cause, vec![text.clone()]);
    }

    fn attach_document(&self) {
        self.document_generation
            .set(self.document_generation.get().wrapping_add(1));
        let old = self.document_subscription.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        let old_editor = self.editor.borrow_mut().take();
        if let Some(editor) = &old_editor {
            editor.set_selection_projection(None);
            editor
                .set_text_view(None)
                .expect("Removing a text view cannot change its document.");
        }
        drop(old_editor);
        let weak = self.to_ref().downgrade();
        let subscription = self.rich_document().changed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.on_document_changed();
            }
        });
        *self.document_subscription.borrow_mut() = Some(subscription);
        if self.document_generation.get() > 1 {
            self.on_document_changed();
        } else {
            *self.last_text.borrow_mut() = self.text();
            self.update_command_states();
        }
    }

    fn on_document_changed(&self) {
        let text = self.text();
        let previous = self.last_text.replace(text.clone());
        if previous != text {
            self.raise_direct_property_changed(Self::text_property(), &previous, &text);
        }
        // CRichEditBox::OnContentChanged publishes TextChanging synchronously
        // after the backing store is updated. TextChanged is a deferred routed
        // event, allowing normal UI changes after the text has been rendered.
        let generation = self.document_generation.get();
        self.raise_event(&RichEditBoxTextChangingEventArgs {
            base: RoutedEventArgs::with_event(Self::text_changing_event()),
            is_content_changing: previous != text,
        });
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    if this.document_generation.get() == generation {
                        this.raise_event(&RoutedEventArgs::with_event(Self::text_changed_event()));
                    }
                }
            },
            DispatcherPriority::LOADED,
        );
        self.im_client.raise_surrounding_text_changed();
        self.im_client.raise_cursor_rectangle_changed();
        self.update_placeholder_visibility();
        self.update_command_states();
    }

    fn refresh_input_scope_subscription(&self) {
        let previous = self.input_scope_subscription.borrow_mut().take();
        if let Some(previous) = previous {
            previous.dispose();
        }
        if let Some(scope) = self.input_scope() {
            let weak = self.to_ref().downgrade();
            *self.input_scope_subscription.borrow_mut() = Some(scope.changed(move || {
                if let Some(this) = weak.upgrade() {
                    this.update_input_scope_snapshot();
                }
            }));
        }
        self.update_input_scope_snapshot();
    }

    fn update_input_scope_snapshot(&self) {
        if let Some(scope) = self.input_scope() {
            self.input_scope_owns_options.set(true);
            TextInputOptions::set_input_scope_names(self, Some(scope.snapshot()));
        } else if self.input_scope_owns_options.replace(false) {
            // Expose inherited options again; never overwrite legacy ContentType.
            self.clear_value(TextInputOptions::input_scope_names_property());
        }
    }

    fn update_input_options(&self) {
        TextInputOptions::set_multiline(self, self.accepts_return());
        TextInputOptions::set_lowercase(
            self,
            self.character_casing() == RichEditCharacterCasing::Lower,
        );
        TextInputOptions::set_uppercase(
            self,
            self.character_casing() == RichEditCharacterCasing::Upper,
        );
        TextInputOptions::set_show_suggestions(self, Some(self.is_text_prediction_enabled()));
    }

    fn update_command_states(&self) {
        self.set_and_raise_cell(Self::can_undo_property(), &self.can_undo, self.can_undo());
        self.set_and_raise_cell(Self::can_redo_property(), &self.can_redo, self.can_redo());
        self.set_and_raise_cell(Self::can_copy_property(), &self.can_copy, self.can_copy());
        self.set_and_raise_cell(Self::can_cut_property(), &self.can_cut, self.can_cut());
        self.set_and_raise_cell(
            Self::can_paste_property(),
            &self.can_paste,
            self.can_paste(),
        );
        self.update_format_command_states();
    }

    fn selection(&self) -> EditorSelection {
        EditorSelection::new(self.selection_start(), self.selection_end())
    }
    fn synchronize_editor_selection(&self, editor: &RichTextEditor) -> Result<(), EditError> {
        editor.set_selection(self.selection())
    }
    fn after_edit(&self, editor: &RichTextEditor) {
        let selection = editor.selection();
        self.select(selection.anchor, selection.active);
        self.notify_input_method_selection();
        self.im_client.raise_cursor_rectangle_changed();
        self.update_command_states();
        self.reset_caret_blink();
        self.invalidate_text_view();
        self.ensure_caret_visible();
    }

    fn notify_input_method_selection(&self) {
        let editor = self.editor();
        let selection = editor.input_method_selection();
        let committed_changed = self.last_input_selection.replace(selection) != selection;
        // Observe every accepted visible change, including an external caret move
        // away and back between native callbacks. The intrinsic revision fences
        // captured sessions independently of the legacy event, whose selection
        // getter intentionally stays in committed coordinates during preedit.
        // Preview cursor movement must not announce a committed selection change.
        self.im_client.input_method_state_revision();
        if committed_changed {
            self.im_client.raise_selection_changed();
        }
    }
    fn ensure_caret_visible(&self) {
        if self.text_presenter.visual_parent().is_none() {
            return;
        }
        if self.text_presenter.is_measure_valid() && self.text_presenter.is_arrange_valid() {
            self.text_presenter.bring_caret_into_view();
        } else if !self.caret_scroll_pending.replace(true) {
            let weak = self.to_ref().downgrade();
            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(this) = weak.upgrade() {
                        this.caret_scroll_pending.set(false);
                        if this.text_presenter.visual_parent().is_some() {
                            this.text_presenter.bring_caret_into_view();
                        }
                    }
                },
                DispatcherPriority::LOADED,
            );
        }
    }
    fn caret_is_active(&self) -> bool {
        self.is_focused()
            && self.is_effectively_enabled()
            && self.is_effectively_visible()
            && !self.is_read_only()
            && self.selection_start() == self.selection_end()
            && self.visual_parent().is_some()
            && self.text_presenter.visual_parent().is_some()
    }

    pub fn caret_visible(&self) -> bool {
        self.caret_is_active() && self.caret_phase.get()
    }

    fn stop_caret_blink(&self) {
        let timer = self.caret_timer.borrow().clone();
        if let Some(timer) = timer {
            timer.stop();
        }
        if self.caret_phase.replace(false) {
            self.invalidate_text_view();
        }
    }

    fn reset_caret_blink(&self) {
        let timer = self.caret_timer.borrow().clone();
        if let Some(timer) = &timer {
            timer.stop();
        }
        let active = self.caret_is_active();
        self.caret_phase.set(active);
        self.invalidate_text_view();
        let interval = self.caret_blink_interval();
        if !active || interval <= TimeSpan::ZERO || self.editor().is_composing() {
            return;
        }
        let timer = timer.unwrap_or_else(|| {
            let timer = DispatcherTimer::with_priority(DispatcherPriority::BACKGROUND);
            let weak = self.to_ref().downgrade();
            timer.tick(move |timer| {
                let Some(this) = weak.upgrade() else {
                    timer.stop();
                    return;
                };
                if this.caret_is_active() {
                    this.caret_phase.set(!this.caret_phase.get());
                    this.invalidate_text_view();
                } else {
                    this.caret_phase.set(false);
                    timer.stop();
                }
            });
            *self.caret_timer.borrow_mut() = Some(timer.clone());
            timer
        });
        // The dispatcher uses signed millisecond due times. Keep submillisecond
        // values from spinning the UI queue and bound unusually long intervals.
        let duration = interval.to_duration().unwrap_or_default();
        timer.set_interval(duration.clamp(
            std::time::Duration::from_millis(1),
            std::time::Duration::from_millis(i32::MAX as u64),
        ));
        timer.start();
    }

    fn filter_input(&self, text: &str) -> String {
        let text: String = text
            .chars()
            .filter(|ch| {
                *ch != '\u{7f}'
                    && (*ch != '\r' && *ch != '\n' || self.accepts_return())
                    && (*ch != '\t' || self.accepts_tab())
            })
            .collect();
        match self.character_casing() {
            RichEditCharacterCasing::Normal => text,
            RichEditCharacterCasing::Lower => text.to_lowercase(),
            RichEditCharacterCasing::Upper => text.to_uppercase(),
        }
    }
    fn filter_fragment(
        &self,
        fragment: &DocumentSnapshot,
    ) -> Result<DocumentSnapshot, crate::documents::DocumentError> {
        fragment_filter::filter(self, fragment)
    }
    pub fn insert_text(&self, text: &str) -> Result<bool, EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        if editor.is_composing() {
            self.commit_composition(text)
        } else {
            editor.insert_text(&self.filter_input(text), EditKind::Typing)
        }
    }
    pub fn replace_selection(&self, fragment: &DocumentSnapshot) -> Result<bool, EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        editor.insert_fragment(fragment, EditKind::Paste)
    }
    pub fn delete_backward(&self, whole_word: bool) -> Result<bool, EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        editor.delete_backward(whole_word)
    }
    pub fn delete_forward(&self, whole_word: bool) -> Result<bool, EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        editor.delete_forward(whole_word)
    }
    pub fn apply_format(&self, format: &CharacterFormat) -> Result<(), EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        editor.apply_format(format)
    }
    pub fn apply_paragraph_format(&self, format: &ParagraphFormat) -> Result<(), EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        editor.apply_paragraph_format(format)
    }
    pub fn undo(&self) -> Result<bool, EditError> {
        let editor = self.editor();
        editor.undo()
    }
    pub fn redo(&self) -> Result<bool, EditError> {
        let editor = self.editor();
        editor.redo()
    }
    pub fn select_all(&self) {
        self.text_presenter.select_all();
    }

    fn format_accelerator(&self, key: Key) -> bool {
        let disabled = self.disabled_formatting_accelerators();
        let result = match key {
            Key::B if !disabled.contains(DisabledFormattingAccelerators::BOLD) => self.toggle_bold(),
            Key::I if !disabled.contains(DisabledFormattingAccelerators::ITALIC) => self.toggle_italic(),
            Key::U if !disabled.contains(DisabledFormattingAccelerators::UNDERLINE) => self.toggle_underline(),
            _ => return false,
        };
        self.handle_edit(result);
        true
    }

    pub(crate) fn begin_input_method_composition(&self) -> Result<(), EditError> {
        let _operation = CompositionOperation::enter(self);
        let editor = self.editor();
        if editor.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        if editor.is_in_operation() {
            return Err(EditError::ReentrantEdit);
        }
        // Superseding a session closes its lifecycle before opening the next.
        // The cancellation fence preserves a newer session started by a handler.
        self.cancel_composition()?;
        if !Rc::ptr_eq(&editor, &self.editor()) {
            return Err(EditError::ReentrantEdit);
        }
        self.synchronize_editor_selection(&editor)?;
        editor.begin_input_method_composition()?;
        let version = editor.rich_document().mutation_version();
        let selection = editor.selection();
        let input_version = self.im_client.input_method_state_revision();
        // EN_STARTCOMPOSITION precedes the first text replacement. WinUI maps
        // its unavailable native composition string to StartIndex=0, Length=0,
        // independently of the committed selection that may later be replaced.
        self.raise_composition(Self::text_composition_started_event(), "", false, 0..0);
        if Rc::ptr_eq(&editor, &self.editor())
            && editor.is_composing()
            && editor.rich_document().mutation_version() == version
            && editor.selection() == selection
            && self.im_client.input_method_state_revision() == input_version
        {
            Ok(())
        } else {
            Err(EditError::ReentrantEdit)
        }
    }

    pub fn set_preedit_text(
        &self,
        text: Option<&str>,
        cursor: Option<usize>,
    ) -> Result<(), EditError> {
        let _operation = CompositionOperation::enter(self);
        let Some(text) = text else {
            self.cancel_composition()?;
            return Ok(());
        };
        let editor = self.editor();
        if editor.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        if editor.is_in_operation() {
            return Err(EditError::ReentrantEdit);
        }
        self.synchronize_editor_selection(&editor)?;
        let started = !editor.is_composing();
        if started {
            // TextBoxBase.cpp raises TextCompositionStarted before the first preview
            // reaches the text, so before TextChanging (#243 review, N1): at the caret,
            // with no composed text yet.
            let caret = editor.selection().range().start;
            let version = editor.rich_document().mutation_version();
            self.raise_composition(Self::text_composition_started_event(), "", false, caret..caret);
            if !Rc::ptr_eq(&editor, &self.editor())
                || editor.rich_document().mutation_version() != version
                || editor.is_composing()
            {
                return Err(EditError::ReentrantEdit);
            }
        }
        let previous_text = editor.rich_document().text();
        let cursor = cursor.map(|offset| {
            self.filter_input(crate::rich_text::utf16_prefix(text, offset))
                .encode_utf16()
                .count()
        });
        editor.set_preedit(&self.filter_input(text), cursor)?;
        // Public composition notifications expose the accepted preview. A handler
        // may replace the document, selection, or preview; the original platform
        // operation must not adopt that newer state as its own composition.
        let accepted_range = editor
            .active_composition_range()
            .ok_or(EditError::ReentrantEdit)?;
        let accepted_text = editor.rich_document().get_text(accepted_range.clone())?;
        let changed = previous_text != editor.rich_document().text();
        let version = editor.rich_document().mutation_version();
        let selection = editor.selection();
        let input_version = self.im_client.input_method_state_revision();
        let current = || {
            Rc::ptr_eq(&editor, &self.editor())
                && editor.is_composing()
                && editor.rich_document().mutation_version() == version
                && editor.selection() == selection
                && self.im_client.input_method_state_revision() == input_version
        };
        if changed {
            self.raise_composition(
                Self::text_composition_changed_event(),
                &accepted_text,
                false,
                accepted_range,
            );
        }
        if current() {
            Ok(())
        } else {
            Err(EditError::ReentrantEdit)
        }
    }
    pub(crate) fn apply_input_method_edit(
        &self,
        edit: &ferroui_base::input::text_input::TextInputMethodEdit,
        composing: bool,
    ) -> Result<bool, EditError> {
        let _operation = CompositionOperation::enter(self);
        if self.filter_input(&edit.text) != edit.text {
            return Err(crate::documents::DocumentError::InvalidFormat(
                "Input filtering would change an exact input-method edit.",
            ).into());
        }
        let editor = self.editor();
        let started = composing && !editor.is_composing();
        let changed = editor.apply_exact_input_edit(edit, composing)?;
        let version = editor.rich_document().mutation_version();
        let selection = editor.selection();
        let input_version = self.im_client.input_method_state_revision();
        let current = || Rc::ptr_eq(&editor, &self.editor())
            && editor.rich_document().mutation_version() == version
            && editor.selection() == selection
            && editor.is_composing() == composing
            && self.im_client.input_method_state_revision() == input_version;
        if composing {
            let range = editor.active_composition_range()
                .ok_or(EditError::ReentrantEdit)?;
            let text = editor.rich_document().get_text(range.clone())?;
            if started {
                self.raise_composition(Self::text_composition_started_event(), &text, false, range.clone());
                if !current() { return Err(EditError::ReentrantEdit); }
            }
            if changed {
                self.raise_composition(Self::text_composition_changed_event(), &text, false, range);
            }
        }
        if current() { Ok(changed) } else { Err(EditError::ReentrantEdit) }
    }

    pub(crate) fn complete_input_method_composition(&self) -> Result<bool, EditError> {
        let _operation = CompositionOperation::enter(self);
        let editor = self.editor();
        let range = editor.active_composition_range();
        let text = range.as_ref().and_then(|range| editor.rich_document().get_text(range.clone()).ok())
            .unwrap_or_default();
        let composing = editor.is_composing();
        let changed = editor.complete_exact_composition()?;
        if composing {
            let version = editor.rich_document().mutation_version();
            let selection = editor.selection();
            let input_version = self.im_client.input_method_state_revision();
            // A successful completion with no active range closes an explicit
            // native start that never received text. It uses the native null
            // event extent without fabricating an automation composition range.
            self.raise_composition(Self::text_composition_ended_event(), &text, true, range.unwrap_or(0..0));
            if !Rc::ptr_eq(&editor, &self.editor()) || version != editor.rich_document().mutation_version()
                || selection != editor.selection() || editor.is_composing()
                || self.im_client.input_method_state_revision() != input_version {
                return Err(EditError::ReentrantEdit);
            }
        }
        Ok(changed)
    }

    pub fn cancel_composition(&self) -> Result<bool, EditError> {
        let _operation = CompositionOperation::enter(self);
        let editor = self.editor();
        let start = editor.active_composition_range().map_or(0, |range| range.start);
        let changed = editor.cancel_composition()?;
        if changed {
            let version = editor.rich_document().mutation_version();
            let selection = editor.selection();
            let input_version = self.im_client.input_method_state_revision();
            self.raise_composition(Self::text_composition_ended_event(), "", false, start..start);
            if !Rc::ptr_eq(&editor, &self.editor()) || editor.is_composing()
                || editor.rich_document().mutation_version() != version
                || editor.selection() != selection
                || self.im_client.input_method_state_revision() != input_version {
                return Err(EditError::ReentrantEdit);
            }
        }
        Ok(changed)
    }
    pub fn commit_composition(&self, text: &str) -> Result<bool, EditError> {
        let _operation = CompositionOperation::enter(self);
        let editor = self.editor();
        let (changed, composition) = editor.commit_composition_result(&self.filter_input(text))?;
        if let Some(composition) = composition {
            let input_version = self.im_client.input_method_state_revision();
            let current = || {
                Rc::ptr_eq(&editor, &self.editor())
                    && editor.rich_document().mutation_version() == composition.mutation_version
                    && editor.selection() == composition.selection
                    && !editor.is_composing()
                    && self.im_client.input_method_state_revision() == input_version
            };
            if !current() {
                return Err(EditError::ReentrantEdit);
            }
            self.raise_composition(
                Self::text_composition_ended_event(),
                &composition.text,
                true,
                composition.range.clone(),
            );
            if !current() {
                return Err(EditError::ReentrantEdit);
            }
        }
        Ok(changed)
    }
    fn raise_composition(
        &self,
        event: &RoutedEvent<RichEditCompositionEventArgs>,
        text: &str,
        committed: bool,
        range: Range<usize>,
    ) {
        let extent = i32::try_from(range.start).ok().zip(
            i32::try_from(range.end.saturating_sub(range.start)).ok(),
        );
        let (start_index, length) = extent.unwrap_or((-1, 0));
        self.raise_event(&RichEditCompositionEventArgs {
            base: RoutedEventArgs::with_event(event),
            text: text.to_owned(),
            selection: self.selection(),
            committed,
            start_index,
            length,
        });
    }

    pub fn export_rtf(&self) -> Result<String, RtfError> {
        RtfWriter::write_with_default_tab_stop(
            &self.rich_document().snapshot(),
            self.editor()
                .default_paragraph_format()
                .default_tab_stop
                .unwrap_or(48.0),
        )
    }
    pub fn import_rtf(&self, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let document = RtfReader::read(bytes)?;
        self.text_document().replace_document(&document, false)?;
        self.editor().clear_history();
        self.select(0, 0);
        self.update_command_states();
        Ok(())
    }

    pub fn copy(&self) {
        self.start_copy(false);
    }
    pub fn cut(&self) {
        self.start_copy(true);
    }
    fn start_copy(&self, cut: bool) {
        if !self.can_copy() || cut && !self.can_cut() {
            return;
        }
        if let Some(clipboard) = TopLevel::get_top_level(Some(self)).and_then(|top| top.clipboard())
        {
            let operation = self.copy_to_clipboard_async(clipboard, cut);
            self.start_clipboard_operation(operation);
        }
    }
    pub fn paste(&self) {
        if !self.can_paste() {
            return;
        }
        if let Some(clipboard) = TopLevel::get_top_level(Some(self)).and_then(|top| top.clipboard())
        {
            let operation = self.paste_from_clipboard_async(clipboard, false);
            self.start_clipboard_operation(operation);
        }
    }
    fn start_clipboard_operation(&self, operation: LocalBoxFuture<Result<(), ClipboardError>>) {
        let weak = self.to_ref().downgrade();
        ClipboardHelper::start(async move {
            if let Err(error) = operation.await {
                if let Some(this) = weak.upgrade() {
                    for (_, callback) in this.clipboard_errors.snapshot().iter() {
                        callback(&error);
                    }
                }
            }
            Ok(())
        });
    }
    pub fn copy_to_clipboard_async(
        &self,
        clipboard: Rc<dyn IClipboard>,
        cut: bool,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let this = self.to_ref();
        let document = self.rich_document();
        let mutation = document.mutation_version();
        let selection = self.selection();
        let allowed = !selection.is_empty() && (!cut || !self.is_read_only());
        let event = RoutedEventArgs::with_event(if cut {
            Self::cutting_to_clipboard_event()
        } else {
            Self::copying_to_clipboard_event()
        });
        if allowed {
            self.raise_event(&event);
        }
        let format = self.clipboard_copy_format();
        let default_tab_stop = self
            .editor()
            .default_paragraph_format()
            .default_tab_stop
            .unwrap_or(48.0);
        let fragment = if allowed && !event.handled() {
            Some(
                document
                    .slice(selection.range())
                    .map_err(|error| ClipboardError::other(error.to_string())),
            )
        } else {
            None
        };
        Box::pin(async move {
            if !allowed || event.handled() {
                return Ok(());
            }
            let Some(fragment) = fragment else {
                return Ok(());
            };
            let fragment = fragment?;
            clipboard
                .set_data_async(Some(RichTextTransfer::create_with_default_tab_stop(
                    &fragment,
                    format,
                    default_tab_stop,
                )))
                .await?;
            // The async clipboard may complete after typing or moving the selection. In that
            // case it has still copied the requested fragment, but must not delete new content.
            // Public revisions may be deferred by an application's document batch;
            // the mutation identity also observes changes before that batch closes.
            if cut
                && this.rich_document() == document
                && document.mutation_version() == mutation
                && this.selection() == selection
                && !this.is_read_only()
            {
                let editor = this.editor();
                this.synchronize_editor_selection(&editor)
                    .map_err(|error| ClipboardError::other(error.to_string()))?;
                editor
                    .delete_selection()
                    .map_err(|error| ClipboardError::other(error.to_string()))?;
            }
            Ok(())
        })
    }
    pub fn paste_from_clipboard_async(
        &self,
        clipboard: Rc<dyn IClipboard>,
        plain_text_only: bool,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let this = self.to_ref();
        let document = self.rich_document();
        let mutation = document.mutation_version();
        let selection = self.selection();
        let event = RoutedEventArgs::with_event(Self::pasting_from_clipboard_event());
        if self.can_paste() {
            self.raise_event(&event);
        }
        let allowed = self.can_paste() && !event.handled();
        Box::pin(async move {
            if !allowed {
                return Ok(());
            }
            let Some(data) = clipboard.try_get_data_async().await? else {
                return Ok(());
            };
            let result = RichTextTransfer::read_for_insertion(
                data.as_ref(),
                plain_text_only,
                this.editor()
                    .default_paragraph_format()
                    .default_tab_stop
                    .unwrap_or(48.0),
            )
            .await;
            data.dispose();
            let Some(fragment) = result? else {
                return Ok(());
            };
            if this.rich_document() != document
                || document.mutation_version() != mutation
                || this.selection() != selection
                || this.is_read_only()
            {
                return Err(ClipboardError::from_kind(ClipboardErrorKind::Canceled));
            }
            // Copying a custom character-format service may execute application
            // code. Observe accepted editor changes while filtering, then reject
            // the older paste without rolling back the callback's newer state.
            let editor = this.editor();
            let changed = Rc::new(Cell::new(false));
            let observed = changed.clone();
            let subscription = editor.changed(move || observed.set(true));
            let fragment = this.filter_fragment(&fragment);
            subscription.dispose();
            let fragment = fragment.map_err(|error| ClipboardError::other(error.to_string()))?;
            if changed.get()
                || !Rc::ptr_eq(&editor, &this.editor())
                || this.rich_document() != document
                || document.mutation_version() != mutation
                || this.selection() != selection
                || this.is_read_only()
            {
                return Err(ClipboardError::from_kind(ClipboardErrorKind::Canceled));
            }
            this.replace_selection(&fragment)
                .map_err(|error| ClipboardError::other(error.to_string()))?;
            Ok(())
        })
    }

    pub(crate) fn handle_edit<T>(&self, result: Result<T, EditError>) {
        if let Err(error) = result {
            for (_, callback) in self.errors.snapshot().iter() {
                callback(&error);
            }
        }
    }
    pub fn edit_failed(&self, handler: impl Fn(&EditError) + 'static) -> Rc<dyn IDisposable> {
        let token = self.errors.add(Rc::new(handler));
        let events = self.errors.clone();
        Disposable::create(move || {
            events.remove(token);
        })
    }
    pub fn clipboard_failed(
        &self,
        handler: impl Fn(&ClipboardError) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.clipboard_errors.add(Rc::new(handler));
        let events = self.clipboard_errors.clone();
        Disposable::create(move || {
            events.remove(token);
        })
    }
    pub fn text_input_method_client(&self) -> Rc<dyn TextInputMethodClient> {
        self.im_client.clone()
    }

    ferro_routed_event!(
        pub fn selection_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<RichEditBox, RoutedEventArgs>(
                "SelectionChanged",
                RoutingStrategies::BUBBLE,
            )
        }
    );

    ferro_routed_event!(
        pub fn text_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("TextChanged", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn text_changing_event() -> RoutedEvent<RichEditBoxTextChangingEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("TextChanging", RoutingStrategies::DIRECT)
        }
    );
    ferro_routed_event!(
        pub fn selection_changing_event() -> RoutedEvent<RichEditBoxSelectionChangingEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("SelectionChanging", RoutingStrategies::DIRECT)
        }
    );
    ferro_routed_event!(
        pub fn copying_to_clipboard_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("CopyingToClipboard", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn cutting_to_clipboard_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("CuttingToClipboard", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn pasting_from_clipboard_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<RichEditBox, _>(
                "PastingFromClipboard",
                RoutingStrategies::BUBBLE,
            )
        }
    );
    ferro_routed_event!(
        pub fn text_composition_started_event() -> RoutedEvent<RichEditCompositionEventArgs> {
            RoutedEvent::register::<RichEditBox, _>(
                "TextCompositionStarted",
                RoutingStrategies::BUBBLE,
            )
        }
    );
    ferro_routed_event!(
        pub fn text_composition_changed_event() -> RoutedEvent<RichEditCompositionEventArgs> {
            RoutedEvent::register::<RichEditBox, _>(
                "TextCompositionChanged",
                RoutingStrategies::BUBBLE,
            )
        }
    );
    ferro_routed_event!(
        pub fn text_composition_ended_event() -> RoutedEvent<RichEditCompositionEventArgs> {
            RoutedEvent::register::<RichEditBox, _>(
                "TextCompositionEnded",
                RoutingStrategies::BUBBLE,
            )
        }
    );
    pub fn text_changed(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::text_changed_event(), handler)
    }
    pub fn text_changing(
        &self,
        handler: impl Fn(&Interactive, &RichEditBoxTextChangingEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::text_changing_event(), handler)
    }
    pub fn selection_changing(
        &self,
        handler: impl Fn(&Interactive, &RichEditBoxSelectionChangingEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::selection_changing_event(), handler)
    }
}

/// The markup types of RichEditBox declared next to their types, registered with the
/// other types of the crate.
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <RichEditCharacterCasing as MarkupTyped>::MARKUP,
    <DisabledFormattingAccelerators as MarkupTyped>::MARKUP,
    <RichEditCompositionEventArgs as MarkupTyped>::MARKUP,
    <RichEditCompositionRoutedEvent as MarkupTyped>::MARKUP,
    <RichEditBoxSelectionChangingEventArgs as MarkupTyped>::MARKUP,
    <RichEditBoxSelectionChangingRoutedEvent as MarkupTyped>::MARKUP,
    <RichEditBoxTextChangingEventArgs as MarkupTyped>::MARKUP,
    <RichEditBoxTextChangingRoutedEvent as MarkupTyped>::MARKUP,
    <crate::automation::provider::TextEditChangeType as MarkupTyped>::MARKUP,
];

/// Registers the nullable forms of the value types of [`MARKUP_TYPES`].
pub(crate) fn register_value_types() {
    use ferroui_base::data::core::ValueTypes;
    ValueTypes::register_nullable::<RichEditCharacterCasing>();
    ValueTypes::register_nullable::<DisabledFormattingAccelerators>();
    ValueTypes::register_nullable::<crate::automation::provider::TextEditChangeType>();
    ValueTypes::register_nullable::<RoutedEvent<RichEditCompositionEventArgs>>();
    ValueTypes::register_nullable::<RoutedEvent<RichEditBoxSelectionChangingEventArgs>>();
    ValueTypes::register_nullable::<RoutedEvent<RichEditBoxTextChangingEventArgs>>();
}

