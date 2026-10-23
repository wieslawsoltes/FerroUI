//! A structured rich-text view using the framework property system, text
//! formatter, drawing context, input routing and overflow continuations.

use crate::documents::{
    Block, BlockCollection, Hyperlink, IInlineHost, LineStackingStrategy, Paragraph,
    RichTextDocument, Span, TextElement, TextGravity, TextHighlightRegion,
    TextHighlighterCollection, TextPosition, Typography,
};
use crate::rich_text_layout::{RichTextBreak, RichTextLayout, RichTextLineMetrics, RichTextSource};
use crate::{Border, Control, ControlImpl, RichTextBlockOverflow, TextBlock, TopLevel};
use ferroui_base::collections::FerroList;
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    KeyModifiers, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{
    InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutingStrategies,
};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::text_formatting::unicode::GraphemeEnumerator;
use ferroui_base::media::{
    subscribe_invalidated, Brushes, DrawingContext, FontFamily, FontFeatureCollection, FontStretch,
    FontStyle, FontWeight, IBrush, TextAlignment, TextDecorationCollection, TextLineBounds,
    TextTrimming, TextWrapping,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, instantiate, DirectProperty, ElementRef, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, IntoRef, Point, Rect, Ref,
    Size, StyledElementImpl, StyledProperty, StyledPropertyMetadata, StyledPropertyOptions,
    Thickness, Vector, Visual, VisualImpl, VisualImplExt, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;

#[path = "rich_text_context_menu.rs"]
mod context_menu;

#[path = "rich_text_overflow_links.rs"]
mod overflow_links;

/// An atomic selection change. Anchor and active end retain selection direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RichTextSelectionChangedEventArgs {
    pub old_start: usize,
    pub old_end: usize,
    pub start: usize,
    pub end: usize,
}

/// Displays paragraphs, nested spans, hyperlinks and embedded controls. All text
/// indices are UTF-16 code units, matching the shaping and input-method contracts.
#[repr(C)]
pub struct RichTextBlock {
    base: Control,
    document: RefCell<RichTextDocument>,
    document_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    font_features_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    source: RefCell<Option<Rc<RichTextSource>>>,
    layout: RefCell<Option<Rc<RichTextLayout>>>,
    layout_generation: Cell<u64>,
    collapse_paragraph_margins: Cell<bool>,
    constraint: Cell<Size>,
    selection_start: Cell<usize>,
    selection_end: Cell<usize>,
    selection_changing: Cell<bool>,
    navigation_x: Cell<Option<f64>>,
    selection_changed: HandlerList<dyn Fn(&RichTextSelectionChangedEventArgs)>,
    dragging: Cell<bool>,
    context_menu_request_generation: Cell<u64>,
    context_menu_key_request: RefCell<Option<context_menu::ContextMenuKeyRequest>>,
    hovered_link: RefCell<Option<WeakRef<Hyperlink>>>,
    pressed_link: RefCell<Option<WeakRef<Hyperlink>>>,
    focused_link: RefCell<Option<WeakRef<Hyperlink>>>,
    flow_parent: RefCell<Option<WeakRef<RichTextBlock>>>,
    overflow_link: overflow_links::OverflowLinkState,
    updating_document: Cell<bool>,
    reported_overflow: Cell<bool>,
    reported_trimmed: Cell<bool>,
    reported_baseline: Cell<f64>,
    highlighters: TextHighlighterCollection,
    view_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    default_highlight_brush_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

ferro_class! {
    RichTextBlock: Control, virtuals RichTextBlockImpl: ControlImpl {
        /// Paints text content after the background and selection highlights.
        fn render_text(this, context: &mut DrawingContext);
        /// Whether the insertion caret is visible (editable views override this).
        fn caret_visible(this) -> bool;
        /// Whether pointer input activates a hyperlink or edits its text.
        fn allow_hyperlink_navigation(this, modifiers: KeyModifiers) -> bool;
        /// Runs synchronously while the proposed selection is visible to readers.
        /// Returning false cancels it unless a handler selected a different range.
        fn on_selection_changing(this, args: &RichTextSelectionChangedEventArgs) -> bool;
        /// Lets an editable view arbitrate a normalized selection through its editor.
        fn request_selection_change(this, start: usize, end: usize) -> bool;
        /// Editors publish their final transaction selection after document changes.
        fn rebase_selection_on_document_change(this) -> bool;
        /// Space outside the document's formatted text, including editor adornments.
        fn text_insets(this) -> Thickness;
        /// The formatter constraint, independent of a scrollable view's viewport.
        fn text_layout_constraint(this, available_size: Size) -> Size;
        /// Implicit tab interval for a paragraph without an assigned override.
        fn default_incremental_tab(this, font_size: f64) -> f64;
        /// Shared origin for text, inline controls, caret, selection and hit testing.
        fn text_origin(this) -> Point;
        /// Whether selection formatting is currently visible in this view.
        fn selection_is_visible(this) -> bool;
        /// The selection background after the host's focus policy is applied.
        fn selection_render_brush(this) -> Option<Rc<dyn IBrush>>;
        /// Publishes the selection event on the owning control.
        fn raise_selection_changed_event(this);
        /// The keyboard focus target for pointer interaction with text.
        fn text_focus_target(this) -> Ref<InputElement>;
        /// Bounds painted by the document background; editor headers are outside it.
        fn text_background_bounds(this) -> Rect;
        /// Native framework visuals displayed beside the document's inline controls.
        fn additional_visual_children(this) -> Vec<Ref<Visual>>;
    }
}

ferroui_base::ferro_class_info!(RichTextBlock {
    new: RichTextBlock::new,
    markup: {
        content: Blocks,
        properties: [
            Blocks: BlockCollection { get: RichTextBlock::blocks },
            TextHighlighters: TextHighlighterCollection { get: RichTextBlock::text_highlighters },
            HorizontalTextAlignment: TextAlignment { get: RichTextBlock::horizontal_text_alignment, set: RichTextBlock::set_horizontal_text_alignment },
            TextLineBounds: TextLineBounds { get: RichTextBlock::text_line_bounds, set: RichTextBlock::set_text_line_bounds },
        ],
        methods: [
            fn SelectAll() => RichTextBlock::select_all,
            fn ClearSelection() => RichTextBlock::clear_selection,
            fn Copy() => RichTextBlock::copy,
        ],
        fields: [
            ContextMenuOpeningEvent: RoutedEvent<crate::ContextMenuEventArgs> => || *RichTextBlock::context_menu_opening_event(),
            SelectionChangedEvent: RoutedEvent<RoutedEventArgs> => || RichTextBlock::selection_changed_event().clone(),
            IsTextTrimmedChangedEvent: RoutedEvent<RoutedEventArgs> => || RichTextBlock::is_text_trimmed_changed_event().clone(),
        ],
    },
});

ferroui_base::ferro_impl_classes!(RichTextBlock: StyledElementImpl, InteractiveImpl);

impl FerroObjectImpl for RichTextBlock {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.attach_document();
        this.attach_view_observers();
        this.refresh_font_features_subscription();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == TextElement::font_features_property().as_property() {
            this.refresh_font_features_subscription();
        }
        if Typography::is_property(change.property()) {
            this.invalidate_document_layout();
        }
        match change.property().name() {
            "FontFamily"
            | "FontSize"
            | "FontStyle"
            | "FontWeight"
            | "FontStretch"
            | "FontFeatures"
            | "Language"
            | "Foreground"
            | "FlowDirection"
            | "LetterSpacing"
            | "TextAlignment"
            | "LineHeight"
            | "LineStackingStrategy"
            | "TextIndent"
            | "TextDecorations"
            | "SelectionForegroundBrush"
            | "SelectionHighlightColor"
            | "IsTextSelectionEnabled" => {
                this.invalidate_document_layout();
            }
            "TextLineBounds" => {
                this.invalidate_document_layout();
                this.refresh_text_line_bounds_clipping();
            }
            "TextWrapping" | "TextTrimming" | "Padding" | "MaxLines" => this.invalidate_layout(),
            "Background" | "CaretBrush" => this.invalidate_visual(),
            "OverflowContentTarget" => this.reconcile_overflow_content_target(),
            _ => {}
        }
    }
}

impl ControlImpl for RichTextBlock {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::RichTextBlockAutomationPeer::new(this).upcast()
    }
}

impl VisualImpl for RichTextBlock {
    fn on_attached_to_visual_tree(this: &Self, event: &ferroui_base::VisualTreeAttachmentEventArgs) {
        this.invalidate_context_menu_request();
        Self::parent_on_attached_to_visual_tree(this, event);
    }

    fn on_detached_from_visual_tree(this: &Self, event: &ferroui_base::VisualTreeAttachmentEventArgs) {
        this.invalidate_context_menu_request();
        Self::parent_on_detached_from_visual_tree(this, event);
    }

    fn render(this: &Self, context: &mut DrawingContext) {
        if let Some(background) = this.background() {
            context.fill_rectangle(&background, this.text_background_bounds(), 0.0);
        }
        let layout = this.page_layout();
        for highlight in &layout.source.highlights {
            if let Some(brush) = &highlight.background {
                for rect in this.get_rects_for_range(highlight.range.clone()) {
                    context.fill_rectangle(brush, rect, 0.0);
                }
            }
        }
        if this.selection_is_visible() {
            if let Some(brush) = this.selection_render_brush() {
                for rect in this.selection_rects() {
                    context.fill_rectangle(&brush, rect, 0.0);
                }
            }
        }
        this.render_text(context);
        if this.caret_visible() && this.selection_start() == this.selection_end() {
            if let Some(brush) = this.caret_brush().or_else(|| this.foreground()) {
                context.fill_rectangle(&brush, this.cursor_rectangle(), 0.0);
            }
        }
    }

    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }
}

impl RichTextBlockImpl for RichTextBlock {
    fn render_text(this: &Self, context: &mut DrawingContext) {
        this.page_layout().draw(context, this.text_origin());
    }

    fn caret_visible(_this: &Self) -> bool {
        false
    }

    fn allow_hyperlink_navigation(_this: &Self, _modifiers: KeyModifiers) -> bool {
        true
    }

    fn on_selection_changing(_this: &Self, _args: &RichTextSelectionChangedEventArgs) -> bool {
        true
    }

    fn request_selection_change(_this: &Self, _start: usize, _end: usize) -> bool {
        false
    }

    fn rebase_selection_on_document_change(_this: &Self) -> bool {
        true
    }

    fn text_insets(this: &Self) -> Thickness {
        this.padding()
    }

    fn text_layout_constraint(this: &Self, available_size: Size) -> Size {
        available_size.deflate(this.text_insets())
    }

    fn default_incremental_tab(_this: &Self, font_size: f64) -> f64 {
        // RichTextServices::TextParagraphProperties::GetDefaultIncrementalTab.
        4.0 * font_size
    }

    fn text_origin(this: &Self) -> Point {
        let insets = this.text_insets();
        Point::new(insets.left, insets.top)
    }

    fn selection_is_visible(this: &Self) -> bool {
        this.is_text_selection_enabled()
    }

    fn selection_render_brush(this: &Self) -> Option<Rc<dyn IBrush>> {
        this.selection_highlight_color()
    }

    fn raise_selection_changed_event(this: &Self) {
        this.raise_event(&RoutedEventArgs::with_event(Self::selection_changed_event()));
    }

    fn text_focus_target(this: &Self) -> Ref<InputElement> {
        this.to_ref().upcast()
    }

    fn text_background_bounds(this: &Self) -> Rect {
        Rect::from_size(this.bounds().size())
    }

    fn additional_visual_children(_this: &Self) -> Vec<Ref<Visual>> {
        Vec::new()
    }
}

impl LayoutableImpl for RichTextBlock {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let inner = this.text_layout_constraint(available_size);
        if this.constraint.replace(inner) != inner {
            this.clear_page_layout();
        }
        let cached = this.cached_page_layout();
        if cached
            .as_ref()
            .is_some_and(|layout| !layout.hosted_objects_are_current())
        {
            this.clear_page_layout();
        }
        // Native SetupLinkedBlockLayout resolves only this overflow's direct
        // predecessor. Geometry queries and recursive parent formatting do not.
        this.setup_linked_overflow_master();
        let layout = this.page_layout_core(true);
        if this
            .cached_page_layout()
            .as_ref()
            .is_some_and(|cached| Rc::ptr_eq(cached, &layout))
        {
            this.synchronize_hosted_visuals(&layout);
        }
        layout.size.inflate(this.text_insets())
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let inner = this.text_layout_constraint(final_size);
        if this.constraint.replace(inner) != inner {
            this.clear_page_layout();
        }
        let layout = this.page_layout();
        if !this.synchronize_hosted_visuals(&layout) {
            return final_size;
        }
        let generation = this.page_layout_generation();
        let origin = this.text_origin();
        if !this.owns_page_layout(&layout, generation) {
            return final_size;
        }
        let children = layout.embedded_controls(origin);
        let owner = this.to_ref().upcast::<Visual>();
        for (child, rect) in children {
            if !this.owns_page_layout(&layout, generation) {
                break;
            }
            if child.visual_parent().as_ref() != Some(&owner) {
                this.clear_page_layout();
                break;
            }
            child.arrange(rect);
            if this.owns_page_layout(&layout, generation)
                && child.visual_parent().as_ref() != Some(&owner)
            {
                this.clear_page_layout();
                break;
            }
        }
        final_size
    }

    fn on_measure_invalidated(this: &Self) {
        this.clear_page_layout();
        Self::parent_on_measure_invalidated(this);
    }
}

impl RichTextBlock {
    fn owns_page_layout(&self, layout: &Rc<RichTextLayout>, generation: u64) -> bool {
        self.page_layout_generation() == generation
            && self
                .cached_page_layout()
                .as_ref()
                .is_some_and(|cached| Rc::ptr_eq(cached, layout))
    }

    fn synchronize_hosted_visuals(&self, layout: &Rc<RichTextLayout>) -> bool {
        let generation = self.page_layout_generation();
        if !self.owns_page_layout(layout, generation) {
            return false;
        }
        let children = layout.embedded_controls(Point::default());
        let mut desired: Vec<Ref<Visual>> = children
            .iter()
            .map(|(child, _)| child.clone().upcast())
            .collect();
        desired.extend(self.additional_visual_children());
        if !self.owns_page_layout(layout, generation) {
            return false;
        }
        let owner = self.to_ref().upcast::<Visual>();
        let current = self.visual_children().snapshot();
        if current.as_slice() != desired.as_slice() {
            // Remove only absent objects. Retained children keep their visual
            // attachment and do not receive a false detach/attach lifecycle.
            for child in current.iter().filter(|child| !desired.contains(child)) {
                self.visual_children().remove(child);
                if !self.owns_page_layout(layout, generation) {
                    return false;
                }
                if child.visual_parent().as_ref() == Some(&owner) {
                    self.clear_page_layout();
                    return false;
                }
            }
            for (index, child) in desired.into_iter().enumerate() {
                if index < self.visual_children().count()
                    && self.visual_children().get(index) == child
                {
                    continue;
                }
                if let Some(previous_index) = self.visual_children().index_of(&child) {
                    if previous_index != index {
                        self.visual_children().move_item(previous_index, index);
                        if !self.owns_page_layout(layout, generation) {
                            return false;
                        }
                        if child.visual_parent().as_ref() != Some(&owner) {
                            self.clear_page_layout();
                            return false;
                        }
                    }
                    continue;
                }
                // A control crossing a column boundary moves to the view that
                // currently owns its formatted run.
                if let Some(parent) = child.visual_parent() {
                    parent.visual_children().remove(&child);
                }
                if !self.owns_page_layout(layout, generation) {
                    return false;
                }
                if child.visual_parent().is_some() {
                    self.clear_page_layout();
                    return false;
                }
                if index > self.visual_children().count() {
                    self.clear_page_layout();
                    return false;
                }
                self.visual_children().insert(index, child.clone());
                if !self.owns_page_layout(layout, generation) {
                    return false;
                }
                if child.visual_parent().as_ref() != Some(&owner) {
                    self.clear_page_layout();
                    return false;
                }
            }
        }
        true
    }

    /// A hosted object is attached before measurement, matching the paragraph
    /// host's capability. Geometry readers never enter this path.
    pub(crate) fn measure_hosted_control(
        &self,
        child: &Ref<Control>,
        size: Size,
        generation: u64,
    ) -> bool {
        if self.page_layout_generation() != generation {
            return false;
        }
        let visual = child.clone().upcast::<Visual>();
        let owner = self.to_ref().upcast::<Visual>();
        if visual.visual_parent().as_ref() != Some(&owner) {
            if let Some(parent) = visual.visual_parent() {
                parent.visual_children().remove(&visual);
            }
            if self.page_layout_generation() != generation {
                return false;
            }
            if visual.visual_parent().is_some() {
                self.clear_page_layout();
                return false;
            }
            self.visual_children().add(visual);
        }
        if self.page_layout_generation() == generation {
            if child.visual_parent().as_ref() != Some(&owner) {
                self.clear_page_layout();
                return false;
            }
            child.measure(size);
            if child.visual_parent().as_ref() != Some(&owner) {
                // A callback transferred ownership. The outer page cannot
                // reclaim it during its final synchronization pass.
                self.clear_page_layout();
            }
        }
        self.page_layout_generation() == generation
    }
}

impl InputElementImpl for RichTextBlock {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        this.invalidate_visual();
    }

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);
        this.focus_hyperlink(None);
        this.invalidate_visual();
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        if !e.handled()
            && e.key == Key::Tab
            && this.move_hyperlink_focus(e.key_modifiers.contains(KeyModifiers::SHIFT))
        {
            e.set_handled(true);
            return;
        }
        let link = this
            .focused_link
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade);
        if !e.handled() && link.is_some_and(|link| link.key_down(e.key)) {
            e.set_handled(true);
            return;
        }
        if e.handled() || !this.is_text_selection_enabled() {
            return;
        }
        let command = e
            .key_modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::META);
        if command && e.key == Key::A {
            this.select_all();
            e.set_handled(true);
            return;
        }
        if command && e.key == Key::C {
            this.copy();
            e.set_handled(true);
            return;
        }
        let active = this.selection_end();
        let text = this.document().text();
        let text: Vec<u16> = text.encode_utf16().collect();
        let extend = e.key_modifiers.contains(KeyModifiers::SHIFT);
        let mut navigation_x = None;
        let next = match e.key {
            Key::Left if !extend && this.selection_start() != active => {
                this.selection_start().min(active)
            }
            Key::Right if !extend && this.selection_start() != active => {
                this.selection_start().max(active)
            }
            Key::Left if command => {
                crate::utils::StringUtils::previous_word(&text, active as i32).max(0) as usize
            }
            Key::Right if command => {
                crate::utils::StringUtils::next_word(&text, active as i32).max(0) as usize
            }
            Key::Left => grapheme_before(&text, active),
            Key::Right => grapheme_after(&text, active),
            Key::Home if command => 0,
            Key::End if command => text.len(),
            Key::Home | Key::End => {
                let layout = this.page_layout();
                let line = layout
                    .lines
                    .iter()
                    .enumerate()
                    .find(|(index, line)| {
                        active >= line.start()
                            && (active < line.end()
                                || active == line.end()
                                    && layout
                                        .lines
                                        .get(index + 1)
                                        .is_none_or(|next| next.start() > active))
                    })
                    .map(|(_, line)| line)
                    .or_else(|| layout.lines.last());
                line.map_or(active, |line| {
                    if e.key == Key::Home {
                        line.start()
                    } else {
                        // Like TextBox::move_end, stop before a real line break.
                        // Paragraph terminators are outside the stored paragraph text.
                        (line.start()
                            + (line.line.length() - line.line.new_line_length()).max(0) as usize)
                            .min(line.paragraph_offset + line.paragraph_length)
                    }
                })
            }
            Key::Up | Key::Down | Key::PageUp | Key::PageDown => {
                let rect = this.cursor_rectangle();
                let page = matches!(e.key, Key::PageUp | Key::PageDown);
                let distance = if page {
                    this.bounds().height.max(rect.height)
                } else {
                    rect.height.max(1.0)
                };
                let down = matches!(e.key, Key::Down | Key::PageDown);
                let origin = this.text_origin();
                let x = this.navigation_x.get().unwrap_or(rect.x - origin.x);
                navigation_x = Some(x);
                this.hit_test_text_position(Point::new(
                    x + origin.x,
                    rect.y + rect.height * 0.5 + if down { distance } else { -distance },
                ))
            }
            _ => return,
        };
        this.select(if extend { this.selection_start() } else { next }, next);
        // Crossing a short line must not discard the original desired column.
        // Any nonvertical selection request resets it in select().
        this.navigation_x.set(navigation_x);
        e.set_handled(true);
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if !e.handled()
            && e.get_current_point(Some(this))
                .properties
                .is_left_button_pressed
            && this.allow_hyperlink_navigation(e.key_modifiers())
        {
            if let Some(link) = this.hyperlink_at(e.get_position(Some(this))) {
                this.focus_hyperlink(Some(link.clone()));
                if link.pointer_pressed() {
                    this.text_focus_target().focus_with(
                        ferroui_base::input::NavigationMethod::Pointer,
                        e.key_modifiers(),
                    );
                    *this.pressed_link.borrow_mut() = Some(link.downgrade());
                    let input: Ref<InputElement> = this.to_ref().upcast();
                    e.pointer().capture(Some(&input));
                    e.set_handled(true);
                    return;
                }
            }
        }
        if e.handled()
            || !this.is_text_selection_enabled()
            || !e
                .get_current_point(Some(this))
                .properties
                .is_left_button_pressed
        {
            return;
        }
        this.text_focus_target().focus_with(
            ferroui_base::input::NavigationMethod::Pointer,
            e.key_modifiers(),
        );
        this.focus_hyperlink(None);
        let index = this.hit_test_text_position(e.get_position(Some(this)));
        let text: Vec<u16> = this.document().text().encode_utf16().collect();
        match e.click_count() {
            2 => this.select(
                crate::utils::StringUtils::previous_word(&text, index as i32).max(0) as usize,
                crate::utils::StringUtils::next_word(&text, index as i32).max(0) as usize,
            ),
            3 => {
                let start = text[..index.min(text.len())]
                    .iter()
                    .rposition(|c| *c == b'\n' as u16)
                    .map_or(0, |i| i + 1);
                let end = text[index.min(text.len())..]
                    .iter()
                    .position(|c| *c == b'\n' as u16)
                    .map_or(text.len(), |i| index + i);
                this.select(start, end);
            }
            _ => this.select(
                if e.key_modifiers().contains(KeyModifiers::SHIFT) {
                    this.selection_start()
                } else {
                    index
                },
                index,
            ),
        }
        this.dragging.set(true);
        let input: Ref<InputElement> = this.to_ref().upcast();
        e.pointer().capture(Some(&input));
        e.set_handled(true);
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        let hovered = this.hyperlink_at(e.get_position(Some(this)));
        let old = this
            .hovered_link
            .borrow_mut()
            .take()
            .and_then(|link| link.upgrade());
        if old != hovered {
            if let Some(old) = old {
                old.set_is_pointer_over(false);
            }
            if let Some(link) = &hovered {
                link.set_is_pointer_over(true);
            }
        }
        *this.hovered_link.borrow_mut() = hovered.map(|link| link.downgrade());
        if this.dragging.get()
            && e.get_current_point(Some(this))
                .properties
                .is_left_button_pressed
        {
            let end = this.hit_test_text_position(e.get_position(Some(this)));
            this.select(this.selection_start(), end);
            e.set_handled(true);
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        let pressed = this
            .pressed_link
            .borrow_mut()
            .take()
            .and_then(|link| link.upgrade());
        if let Some(link) = pressed {
            let over = this.hyperlink_at(e.get_position(Some(this))).as_ref() == Some(&link);
            link.pointer_released(over);
            e.pointer().capture(None);
            e.set_handled(true);
            return;
        }
        if this.dragging.replace(false) {
            e.pointer().capture(None);
            e.set_handled(true);
        }
    }

    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        if this.try_handle_context_menu_key_up(e) {
            return;
        }
        Self::parent_on_key_up(this, e);
        let link = this
            .focused_link
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade);
        if !e.handled() && link.is_some_and(|link| link.key_up(e.key)) {
            e.set_handled(true);
        }
    }

    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);
        let link = this
            .hovered_link
            .borrow_mut()
            .take()
            .and_then(|link| link.upgrade());
        if let Some(link) = link {
            link.set_is_pointer_over(false);
        }
    }

    fn on_pointer_capture_lost(this: &Self, e: &ferroui_base::input::PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);
        this.dragging.set(false);
        let link = this
            .pressed_link
            .borrow_mut()
            .take()
            .and_then(|link| link.upgrade());
        if let Some(link) = link {
            link.cancel_pointer();
        }
    }
}

ferroui_base::ferro_properties! {
    impl RichTextBlock {
        pub fn document_property() -> DirectProperty<RichTextBlock, RichTextDocument> {
            FerroProperty::register_direct::<RichTextBlock, _>("Document", Self::document,
                Some(|this: &RichTextBlock, value| this.set_document(value)), RichTextDocument::new())
        }
        pub fn selection_start_property() -> DirectProperty<RichTextBlock, i32> {
            FerroProperty::register_direct::<RichTextBlock, _>("SelectionStart", |this| this.selection_start().min(i32::MAX as usize) as i32,
                Some(|this: &RichTextBlock, value: i32| this.set_selection_start(value.max(0) as usize)), 0)
        }
        pub fn selection_end_property() -> DirectProperty<RichTextBlock, i32> {
            FerroProperty::register_direct::<RichTextBlock, _>("SelectionEnd", |this| this.selection_end().min(i32::MAX as usize) as i32,
                Some(|this: &RichTextBlock, value: i32| this.set_selection_end(value.max(0) as usize)), 0)
        }
        pub fn selected_text_property() -> DirectProperty<RichTextBlock, String> {
            FerroProperty::register_direct::<RichTextBlock, _>("SelectedText", Self::selected_text, None, String::new())
        }
        pub fn is_text_selection_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<RichTextBlock, _>("IsTextSelectionEnabled", true)
        }
        pub fn selection_highlight_color_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<RichTextBlock, _>("SelectionHighlightColor", Some(Brushes::light_blue()))
        }
        pub fn selection_foreground_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            crate::SelectableTextBlock::selection_foreground_brush_property().add_owner::<RichTextBlock>()
        }
        pub fn caret_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<RichTextBlock, _>("CaretBrush", None)
        }
        pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { Border::background_property().add_owner::<RichTextBlock>() }
        pub fn padding_property() -> StyledProperty<Thickness> { crate::Decorator::padding_property().add_owner::<RichTextBlock>() }
        pub fn font_family_property() -> StyledProperty<FontFamily> { TextElement::font_family_property().add_owner::<RichTextBlock>() }
        pub fn font_size_property() -> StyledProperty<f64> { TextElement::font_size_property().add_owner::<RichTextBlock>() }
        pub fn font_style_property() -> StyledProperty<FontStyle> { TextElement::font_style_property().add_owner::<RichTextBlock>() }
        pub fn font_weight_property() -> StyledProperty<FontWeight> { TextElement::font_weight_property().add_owner::<RichTextBlock>() }
        pub fn font_stretch_property() -> StyledProperty<FontStretch> { TextElement::font_stretch_property().add_owner::<RichTextBlock>() }
        pub fn font_features_property() -> StyledProperty<Option<FontFeatureCollection>> { TextElement::font_features_property().add_owner::<RichTextBlock>() }
        pub fn language_property() -> StyledProperty<ferroui_base::utilities::CultureInfo> { TextElement::language_property().add_owner::<RichTextBlock>() }
        pub fn foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { TextElement::foreground_property().add_owner::<RichTextBlock>() }
        pub fn letter_spacing_property() -> StyledProperty<f64> { TextElement::letter_spacing_property().add_owner::<RichTextBlock>() }
        pub fn text_alignment_property() -> StyledProperty<TextAlignment> { TextBlock::text_alignment_property().add_owner::<RichTextBlock>() }
        pub fn line_height_property() -> StyledProperty<f64> { TextBlock::line_height_property().add_owner::<RichTextBlock>() }
        pub fn text_indent_property() -> StyledProperty<f64> { Paragraph::text_indent_property().add_owner::<RichTextBlock>() }
        pub fn line_stacking_strategy_property() -> StyledProperty<LineStackingStrategy> { Block::line_stacking_strategy_property().add_owner::<RichTextBlock>() }
        pub fn text_line_bounds_property() -> StyledProperty<TextLineBounds> { TextBlock::text_line_bounds_property().add_owner::<RichTextBlock>() }
        pub fn text_decorations_property() -> StyledProperty<Option<TextDecorationCollection>> { TextBlock::text_decorations_property().add_owner::<RichTextBlock>() }
        pub fn text_trimming_property() -> StyledProperty<Rc<dyn TextTrimming>> { TextBlock::text_trimming_property().add_owner::<RichTextBlock>() }
        pub fn text_wrapping_property() -> StyledProperty<TextWrapping> {
            FerroProperty::register::<RichTextBlock, _>("TextWrapping", TextWrapping::Wrap)
        }
        pub fn max_lines_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<RichTextBlock, _>("MaxLines", StyledPropertyOptions::new(0).validate(|value| *value >= 0))
        }
        pub fn overflow_content_target_property() -> StyledProperty<Option<ElementRef<RichTextBlockOverflow>>> {
            ferroui_base::data::core::ValueTypes::register_element_ref::<RichTextBlockOverflow>();
            FerroProperty::register_with::<RichTextBlock, _>("OverflowContentTarget", StyledPropertyOptions::new(None))
        }
        pub fn has_overflow_content_property() -> DirectProperty<RichTextBlock, bool> {
            FerroProperty::register_direct::<RichTextBlock, _>("HasOverflowContent", Self::has_overflow_content, None, false)
        }
        pub fn baseline_offset_property() -> DirectProperty<RichTextBlock, f64> {
            FerroProperty::register_direct::<RichTextBlock, _>("BaselineOffset", Self::baseline_offset, None, 0.0)
        }
        pub fn is_text_trimmed_property() -> DirectProperty<RichTextBlock, bool> {
            FerroProperty::register_direct::<RichTextBlock, _>("IsTextTrimmed", Self::is_text_trimmed, None, false)
        }
    }
}

impl RichTextBlock {
    ferroui_base::ferro_routed_event!(
        pub fn selection_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<RichTextBlock, RoutedEventArgs>(
                "SelectionChanged",
                RoutingStrategies::BUBBLE,
            )
        }
    );
    ferroui_base::ferro_routed_event!(
        pub fn is_text_trimmed_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<RichTextBlock, RoutedEventArgs>(
                "IsTextTrimmedChanged",
                RoutingStrategies::BUBBLE,
            )
        }
    );

    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<RichTextBlock>(true);
        InputElement::context_requested_event().add_class_handler_with::<RichTextBlock>(
            Self::on_context_requested,
            RoutingStrategies::BUBBLE,
            false,
        );
        InputElement::context_canceled_event().add_class_handler_with::<RichTextBlock>(
            Self::on_context_canceled,
            RoutingStrategies::BUBBLE,
            false,
        );
        // Trimmed line boxes deliberately exclude some glyph ink, just as the
        // shared TextBlock formatter does. Overflow uses the document owner's
        // bounds policy even when it is not in the owner's logical tree.
        Visual::clip_to_bounds_property().override_metadata::<RichTextBlock>(
            StyledPropertyMetadata::new(Some(true)).with_coerce(
                |object: &FerroObject, clip: bool| {
                    clip && object.downcast_ref::<RichTextBlock>().is_none_or(|view| {
                        view.effective_text_line_bounds() == TextLineBounds::Full
                    })
                },
            ),
        );
        Visual::affects_render::<RichTextBlock>(&[
            Self::background_property().as_property(),
            Self::foreground_property().as_property(),
            Self::selection_foreground_brush_property().as_property(),
            Self::selection_highlight_color_property().as_property(),
            Self::caret_brush_property().as_property(),
        ]);
    }

    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            document: RefCell::new(RichTextDocument::new()),
            document_subscription: RefCell::new(None),
            font_features_subscription: RefCell::new(None),
            source: RefCell::new(None),
            layout: RefCell::new(None),
            layout_generation: Cell::new(0),
            collapse_paragraph_margins: Cell::new(true),
            constraint: Cell::new(Size::new(f64::INFINITY, f64::INFINITY)),
            selection_start: Cell::new(0),
            selection_end: Cell::new(0),
            selection_changing: Cell::new(false),
            navigation_x: Cell::new(None),
            selection_changed: HandlerList::new(),
            dragging: Cell::new(false),
            context_menu_request_generation: Cell::new(0),
            context_menu_key_request: RefCell::new(None),
            hovered_link: RefCell::new(None),
            pressed_link: RefCell::new(None),
            focused_link: RefCell::new(None),
            flow_parent: RefCell::new(None),
            overflow_link: overflow_links::OverflowLinkState::new(),
            updating_document: Cell::new(false),
            reported_overflow: Cell::new(false),
            reported_trimmed: Cell::new(false),
            reported_baseline: Cell::new(0.0),
            highlighters: TextHighlighterCollection::new(),
            view_subscriptions: RefCell::new(Vec::new()),
            default_highlight_brush_subscriptions: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn refresh_font_features_subscription(&self) {
        let previous = self.font_features_subscription.borrow_mut().take();
        if let Some(previous) = previous {
            previous.dispose();
        }
        let subscription = self.font_features().map(|features| {
            let weak = self.to_ref().downgrade();
            features.collection_changed(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.invalidate_document_layout();
                }
            })
        });
        *self.font_features_subscription.borrow_mut() = subscription;
    }

    pub fn document(&self) -> RichTextDocument {
        let parent = self
            .flow_parent
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade);
        parent.map_or_else(
            || self.document.borrow().clone(),
            |parent| parent.document(),
        )
    }

    pub fn set_document(&self, value: RichTextDocument) {
        self.try_set_document(value);
    }

    /// Attaches a document whose block elements are not owned by another view.
    /// Use an overflow target to share a view's document, or restore a snapshot
    /// into another document to make an independent view.
    pub fn try_set_document(&self, value: RichTextDocument) -> bool {
        if self
            .flow_parent
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
            .is_some()
            || value.blocks().logical_parent().is_some_and(|parent| {
                parent != self.to_ref().upcast::<ferroui_base::StyledElement>()
            })
        {
            return false;
        }
        let old = self.document.borrow().clone();
        if old == value {
            return true;
        }
        self.focus_hyperlink(None);
        let subscription = self.document_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        old.set_inline_host(None);
        old.set_logical_parent(None);
        *self.document.borrow_mut() = value.clone();
        self.attach_document();
        self.select(0, 0);
        self.invalidate_document_layout();
        self.raise_direct_property_changed(Self::document_property(), &old, &value);
        true
    }

    fn attach_document(&self) {
        if self.updating_document.replace(true) {
            return;
        }
        let document = self.document.borrow().clone();
        document.set_logical_parent(Some(self));
        let host: Rc<dyn IInlineHost> = Rc::new(RichBlockInlineHost(self.to_ref().downgrade()));
        document.set_inline_host(Some(host));
        let weak = self.to_ref().downgrade();
        let subscription = document.changed(move |change| {
            if let Some(this) = weak.upgrade() {
                this.invalidate_document_layout();
                let start = change
                    .map_position(TextPosition::new(
                        this.selection_start(),
                        TextGravity::Forward,
                    ))
                    .offset;
                let end = change
                    .map_position(TextPosition::new(
                        this.selection_end(),
                        TextGravity::Forward,
                    ))
                    .offset;
                if this.rebase_selection_on_document_change() {
                    this.select(start, end);
                }
                this.raise_direct_property_changed(
                    Self::selected_text_property(),
                    &String::new(),
                    &this.selected_text(),
                );
            }
        });
        if let Some(old) = self
            .document_subscription
            .borrow_mut()
            .replace(subscription)
        {
            old.dispose();
        }
        self.updating_document.set(false);
    }

    pub fn blocks(&self) -> BlockCollection {
        self.document().blocks()
    }
    pub fn text_highlighters(&self) -> TextHighlighterCollection {
        self.previous_flow_parent().map_or_else(
            || self.highlighters.clone(),
            |parent| parent.text_highlighters(),
        )
    }

    fn attach_view_observers(&self) {
        let weak = self.to_ref().downgrade();
        let highlighters = self.highlighters.invalidated(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidate_document_layout();
            }
        });
        let weak = self.to_ref().downgrade();
        let resources = self.resources_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.refresh_default_highlight_brush_subscriptions();
                this.invalidate_document_layout();
            }
        });
        let weak = self.to_ref().downgrade();
        let theme = self.actual_theme_variant_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.refresh_default_highlight_brush_subscriptions();
                this.invalidate_document_layout();
            }
        });
        *self.view_subscriptions.borrow_mut() = vec![highlighters, resources, theme];
        self.refresh_default_highlight_brush_subscriptions();
    }

    fn default_highlight_brush(&self, foreground: bool) -> Option<Rc<dyn IBrush>> {
        let key = ResourceKey::from(if foreground {
            "TextControlHighlighterForeground"
        } else {
            "TextControlHighlighterBackground"
        });
        self.try_find_resource(&key, self.actual_theme_variant().as_ref())
            .flatten()
            .and_then(|value| {
                let converted =
                    ValueTypes::try_cast(&value, ValueType::of::<Option<Rc<dyn IBrush>>>())?;
                converted
                    .downcast_ref::<Option<Rc<dyn IBrush>>>()
                    .cloned()
                    .flatten()
            })
            .filter(|brush| brush.as_solid_color_brush().is_some())
            .or_else(|| {
                Some(if foreground {
                    Brushes::black()
                } else {
                    Brushes::yellow()
                })
            })
    }

    fn refresh_default_highlight_brush_subscriptions(&self) {
        let old = std::mem::take(&mut *self.default_highlight_brush_subscriptions.borrow_mut());
        for subscription in old {
            subscription.dispose();
        }
        let mut subscriptions = Vec::new();
        for foreground in [true, false] {
            let brush = self.default_highlight_brush(foreground);
            let weak = self.to_ref().downgrade();
            let handler: Rc<dyn Fn()> = Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.invalidate_document_layout();
                }
            });
            if let Some(subscription) = subscribe_invalidated(&brush, handler) {
                subscriptions.push(subscription);
            }
        }
        *self.default_highlight_brush_subscriptions.borrow_mut() = subscriptions;
    }

    pub(crate) fn highlight_regions(&self) -> Vec<TextHighlightRegion> {
        let text: Vec<u16> = self.document().text().encode_utf16().collect();
        self.text_highlighters().merged_regions_with(
            text.len(),
            self.default_highlight_brush(true),
            self.default_highlight_brush(false),
            |range| {
                let start = scalar_boundary(&text, range.start);
                let end = if scalar_boundary(&text, range.end) < range.end {
                    range.end + 1
                } else {
                    range.end
                };
                start..end
            },
        )
    }
    pub fn add_child(&self, block: impl IntoRef<Block>) {
        self.blocks().add(block);
    }
    pub fn text(&self) -> String {
        self.document().text()
    }
    pub fn selection_start(&self) -> usize {
        let parent = self
            .flow_parent
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade);
        parent.map_or_else(
            || self.selection_start.get(),
            |parent| parent.selection_start(),
        )
    }
    pub fn selection_end(&self) -> usize {
        let parent = self
            .flow_parent
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade);
        parent.map_or_else(|| self.selection_end.get(), |parent| parent.selection_end())
    }
    pub fn caret_index(&self) -> usize {
        self.selection_end()
    }
    pub fn set_selection_start(&self, value: usize) {
        self.select(value, self.selection_end());
    }
    pub fn set_selection_end(&self, value: usize) {
        self.select(self.selection_start(), value);
    }

    pub fn select(&self, start: usize, end: usize) {
        self.navigation_x.set(None);
        let parent = self
            .flow_parent
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade);
        if let Some(parent) = parent {
            parent.select(start, end);
            return;
        }
        let text: Vec<u16> = self.document().text().encode_utf16().collect();
        let start = scalar_boundary(&text, start);
        let end = scalar_boundary(&text, end);
        if !self.selection_changing.get() && self.request_selection_change(start, end) {
            return;
        }
        let old_start = self.selection_start.replace(start);
        let old_end = self.selection_end.replace(end);
        if start == old_start && end == old_end {
            return;
        }
        // WinUI CTextBoxBase::ProcessSelectionChangingEvent suppresses nested
        // notifications, but lets an application's new selection supersede Cancel.
        // Keep proposed values observable without publishing transient property values.
        if self.selection_changing.replace(true) {
            return;
        }
        struct SelectionChange<'a>(&'a Cell<bool>);
        impl Drop for SelectionChange<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let guard = SelectionChange(&self.selection_changing);
        let args = RichTextSelectionChangedEventArgs {
            old_start,
            old_end,
            start,
            end,
        };
        let accepted = self.on_selection_changing(&args);
        let changed_by_handler =
            self.selection_start.get() != start || self.selection_end.get() != end;
        if !accepted && !changed_by_handler {
            // A handler may have edited the public document as well. Its current
            // scalar boundaries govern restoration of the previous range.
            let text: Vec<u16> = self.document().text().encode_utf16().collect();
            self.selection_start.set(scalar_boundary(&text, old_start));
            self.selection_end.set(scalar_boundary(&text, old_end));
            return;
        }
        let args = RichTextSelectionChangedEventArgs {
            old_start,
            old_end,
            start: self.selection_start.get(),
            end: self.selection_end.get(),
        };
        drop(guard);
        if args.start == old_start && args.end == old_end {
            return;
        }
        self.notify_selection_changed(&args);
    }

    fn notify_selection_changed(&self, args: &RichTextSelectionChangedEventArgs) {
        self.raise_direct_property_changed(
            Self::selection_start_property(),
            &(args.old_start.min(i32::MAX as usize) as i32),
            &(args.start.min(i32::MAX as usize) as i32),
        );
        self.raise_direct_property_changed(
            Self::selection_end_property(),
            &(args.old_end.min(i32::MAX as usize) as i32),
            &(args.end.min(i32::MAX as usize) as i32),
        );
        self.raise_direct_property_changed(
            Self::selected_text_property(),
            &String::new(),
            &self.selected_text(),
        );
        if self.selection_foreground_brush().is_some()
            || args.start != args.end
            || args.old_start != args.old_end
        {
            self.invalidate_document_layout();
        } else {
            self.invalidate_visual();
        }
        for (_, handler) in self.selection_changed.snapshot().iter() {
            handler(args);
        }
        self.raise_selection_changed_event();
        if let Some(target) = self.overflow_content_target() {
            target.notify_selection_changed(args);
        }
    }

    pub fn selection_changed(
        &self,
        handler: impl Fn(&RichTextSelectionChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.selection_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.selection_changed.remove(token);
            }
        })
    }

    pub fn selected_text(&self) -> String {
        let start = self.selection_start().min(self.selection_end());
        let end = self.selection_start().max(self.selection_end());
        self.document().get_text(start..end).unwrap_or_default()
    }
    pub fn select_all(&self) {
        self.select(0, self.document().text_length());
    }
    pub fn clear_selection(&self) {
        self.select(self.selection_end(), self.selection_end());
    }
    pub fn can_copy(&self) -> bool {
        self.selection_start() != self.selection_end()
    }

    /// Finds a hyperlink inside rendered content, excluding blank padding.
    pub fn hyperlink_at(&self, point: Point) -> Option<Ref<Hyperlink>> {
        if !self
            .line_metrics()
            .iter()
            .any(|line| line.bounds.contains(point))
        {
            return None;
        }
        let index = self.hit_test_text_position(point);
        let mut element = self
            .document()
            .element_at(index, TextGravity::Forward)
            .map(|element| element.upcast::<ferroui_base::StyledElement>());
        while let Some(current) = element {
            if let Some(link) = current.clone().cast::<Hyperlink>() {
                return Some(link);
            }
            element = current.parent();
        }
        None
    }

    pub(crate) fn focus_hyperlink(&self, next: Option<Ref<Hyperlink>>) {
        let previous = self
            .focused_link
            .borrow_mut()
            .take()
            .and_then(|link| link.upgrade());
        if previous != next {
            if let Some(previous) = previous {
                previous.set_is_focused(false);
                previous.cancel_pointer();
            }
        }
        if let Some(next) = next {
            next.set_is_focused(true);
            *self.focused_link.borrow_mut() = Some(next.downgrade());
        }
    }

    fn move_hyperlink_focus(&self, backwards: bool) -> bool {
        fn collect(inlines: &crate::documents::InlineCollection, result: &mut Vec<Ref<Hyperlink>>) {
            for inline in inlines.snapshot().iter() {
                if let Some(link) = inline.clone().cast::<Hyperlink>() {
                    if link.is_enabled() && link.is_tab_stop() {
                        result.push(link);
                    }
                } else if let Some(span) = inline.clone().cast::<Span>() {
                    collect(&span.inlines(), result);
                }
            }
        }
        let mut links = Vec::new();
        for paragraph in self.document().paragraphs() {
            collect(&paragraph.inlines(), &mut links);
        }
        links.sort_by_key(|link| link.tab_index());
        let current = self
            .focused_link
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade);
        let index = current.and_then(|current| links.iter().position(|link| *link == current));
        let next = match (backwards, index) {
            (false, Some(index)) => links.get(index + 1),
            (true, Some(index)) => index.checked_sub(1).and_then(|index| links.get(index)),
            (false, None) => links.first(),
            (true, None) => links.last(),
        }
        .cloned();
        let handled = next.is_some();
        self.focus_hyperlink(next);
        handled
    }

    pub fn copy(&self) {
        let text = self.selected_text();
        if text.is_empty() {
            return;
        }
        let Some(clipboard) = TopLevel::get_top_level(Some(self)).and_then(|top| top.clipboard())
        else {
            return;
        };
        crate::utils::ClipboardHelper::start(async move {
            match clipboard.set_text_async(Some(&text)).await {
                Err(error)
                    if crate::utils::ClipboardHelper::is_expected_clipboard_exception(&error) =>
                {
                    Ok(())
                }
                result => result,
            }
        });
    }

    /// Exports using the same resolved tab intervals as this view and its overflow
    /// chain. RTF has one document-wide default interval, so paragraphs resolving
    /// to different intervals return an unsupported-feature error.
    pub fn export_rtf(&self) -> Result<String, crate::rich_text::RtfError> {
        let mut snapshot = self.document().snapshot();
        let source = self.page_layout().source.clone();
        for (paragraph, resolved) in snapshot.paragraphs.iter_mut().zip(&source.paragraphs) {
            paragraph.format.default_tab_stop = Some(resolved.default_tab_stop);
        }
        crate::rich_text::RtfWriter::write(&snapshot)
    }

    pub fn hit_test_text_position(&self, point: Point) -> usize {
        let origin = self.text_origin();
        self.page_layout()
            .hit_test(Point::new(point.x - origin.x, point.y - origin.y))
    }
    pub fn get_rect_from_character_index(&self, index: usize) -> Rect {
        self.get_rect_from_text_position(TextPosition::new(index, TextGravity::Forward))
    }

    /// Gets a caret rectangle at the leading or trailing character edge of the
    /// same UTF-16 insertion position. Affinity matters at a bidi run boundary.
    pub fn get_rect_from_text_position(&self, position: TextPosition) -> Rect {
        let origin = self.text_origin();
        self.page_layout()
            .caret_rect_with_gravity(position.offset, position.gravity)
            .translate(Vector::new(origin.x, origin.y))
    }

    /// Finds the view containing a document insertion position. A terminal
    /// insertion point belongs to the final column; an off-page position has no
    /// view until enough overflow columns are linked and laid out.
    pub fn text_view_for_position(&self, index: usize) -> Option<Ref<RichTextBlock>> {
        if self.document().validate_range(index..index).is_err() {
            return None;
        }
        let mut view = Some(self.to_ref());
        while let Some(current) = view {
            let layout = current.layout.borrow().clone()?;
            let start = layout
                .lines
                .first()
                .map_or(layout.next.position, |line| line.start());
            let end = layout.next.position;
            if index >= start
                && !layout.lines.is_empty()
                && (index < end || index == end && !layout.has_overflow)
                && layout
                    .visible_text_ranges()
                    .iter()
                    .any(|range| index >= range.start && index <= range.end)
            {
                return Some(current);
            }
            view = current.overflow_content_target().map(Ref::upcast);
        }
        None
    }
    pub fn cursor_rectangle(&self) -> Rect {
        self.get_rect_from_character_index(self.caret_index())
    }
    pub fn selection_rects(&self) -> Vec<Rect> {
        self.get_rects_for_range(
            self.selection_start().min(self.selection_end())
                ..self.selection_start().max(self.selection_end()),
        )
    }

    /// Gets the visible bounds of a UTF-16 range without changing selection.
    /// Invalid or empty ranges have no rectangles. Overflow views return only
    /// the rectangles within their own formatted page.
    pub fn get_rects_for_range(&self, range: Range<usize>) -> Vec<Rect> {
        if range.is_empty() || self.document().validate_range(range.clone()).is_err() {
            return Vec::new();
        }
        let origin = self.text_origin();
        self.page_layout()
            .selection_rects(range.start, range.end)
            .into_iter()
            .map(|rect| rect.translate(Vector::new(origin.x, origin.y)))
            .collect()
    }
    pub fn line_metrics(&self) -> Vec<RichTextLineMetrics> {
        self.page_layout().metrics(self.text_origin())
    }
    /// Source ranges retained in this formatted page, excluding collapsed text
    /// represented by an ellipsis. Ancestor viewports may clip these further.
    pub fn visible_text_ranges(&self) -> Vec<Range<usize>> {
        self.page_layout().visible_text_ranges()
    }
    pub fn is_text_trimmed(&self) -> bool {
        self.page_layout().is_trimmed
    }
    pub fn baseline_offset(&self) -> f64 {
        self.line_metrics()
            .first()
            .map_or(0.0, |line| line.baseline)
    }
    pub fn has_overflow_content(&self) -> bool {
        self.page_layout().has_overflow
    }
    pub fn content_start(&self) -> usize {
        let layout = self.page_layout();
        layout
            .lines
            .first()
            .map_or(layout.next.position, |line| line.start())
    }
    pub fn content_end(&self) -> usize {
        self.page_layout().next.position
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
    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }
    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_property(), value);
    }
    pub fn padding(&self) -> Thickness {
        self.get_value(Self::padding_property())
    }
    pub fn set_padding(&self, value: Thickness) {
        self.set_value(Self::padding_property(), value);
    }
    pub fn font_family(&self) -> FontFamily {
        self.get_value(Self::font_family_property())
    }
    pub fn set_font_family(&self, value: FontFamily) {
        self.set_value(Self::font_family_property(), value);
    }
    pub fn font_size(&self) -> f64 {
        self.get_value(Self::font_size_property())
    }
    pub fn set_font_size(&self, value: f64) {
        self.set_value(Self::font_size_property(), value);
    }
    pub fn font_style(&self) -> FontStyle {
        self.get_value(Self::font_style_property())
    }
    pub fn set_font_style(&self, value: FontStyle) {
        self.set_value(Self::font_style_property(), value);
    }
    pub fn font_weight(&self) -> FontWeight {
        self.get_value(Self::font_weight_property())
    }
    pub fn set_font_weight(&self, value: FontWeight) {
        self.set_value(Self::font_weight_property(), value);
    }
    pub fn font_stretch(&self) -> FontStretch {
        self.get_value(Self::font_stretch_property())
    }
    pub fn set_font_stretch(&self, value: FontStretch) {
        self.set_value(Self::font_stretch_property(), value);
    }
    pub fn font_features(&self) -> Option<FontFeatureCollection> {
        self.get_value(Self::font_features_property())
    }
    pub fn language(&self) -> ferroui_base::utilities::CultureInfo {
        self.get_value(Self::language_property())
    }
    pub fn set_language(&self, value: ferroui_base::utilities::CultureInfo) {
        self.set_value(Self::language_property(), value);
    }
    pub fn set_font_features(&self, value: Option<FontFeatureCollection>) {
        self.set_value(Self::font_features_property(), value);
    }
    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }
    pub fn set_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::foreground_property(), value);
    }
    pub fn letter_spacing(&self) -> f64 {
        self.get_value(Self::letter_spacing_property())
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
    pub fn horizontal_text_alignment(&self) -> TextAlignment {
        self.text_alignment()
    }
    pub fn set_horizontal_text_alignment(&self, value: TextAlignment) {
        self.set_text_alignment(value);
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
    /// The shared formatter's bounds policy for every line in this document.
    /// The value is inherited from the framework's TextBlock attached property.
    pub fn text_line_bounds(&self) -> TextLineBounds {
        self.get_value(Self::text_line_bounds_property())
    }
    pub fn set_text_line_bounds(&self, value: TextLineBounds) {
        self.set_value(Self::text_line_bounds_property(), value);
    }
    fn effective_text_line_bounds(&self) -> TextLineBounds {
        self.previous_flow_parent().map_or_else(
            || self.text_line_bounds(),
            |source| source.effective_text_line_bounds(),
        )
    }
    fn refresh_text_line_bounds_clipping(&self) {
        self.coerce_value(Visual::clip_to_bounds_property().as_property());
        if let Some(target) = self.installed_overflow_target() {
            target.refresh_text_line_bounds_clipping();
        }
    }
    pub fn text_decorations(&self) -> Option<TextDecorationCollection> {
        self.get_value(Self::text_decorations_property())
    }
    pub fn set_text_decorations(&self, value: Option<TextDecorationCollection>) {
        self.set_value(Self::text_decorations_property(), value);
    }
    pub fn text_trimming(&self) -> Rc<dyn TextTrimming> {
        self.get_value(Self::text_trimming_property())
    }
    pub fn set_text_trimming(&self, value: Rc<dyn TextTrimming>) {
        self.set_value(Self::text_trimming_property(), value);
    }
    pub fn line_height(&self) -> f64 {
        self.get_value(Self::line_height_property())
    }
    pub fn set_line_height(&self, value: f64) {
        self.set_value(Self::line_height_property(), value);
    }
    pub fn text_wrapping(&self) -> TextWrapping {
        self.get_value(Self::text_wrapping_property())
    }
    pub fn set_text_wrapping(&self, value: TextWrapping) {
        self.set_value(Self::text_wrapping_property(), value);
    }
    pub fn max_lines(&self) -> usize {
        self.get_value(Self::max_lines_property()).max(0) as usize
    }
    pub fn set_max_lines(&self, value: usize) {
        self.set_value(
            Self::max_lines_property(),
            value.min(i32::MAX as usize) as i32,
        );
    }

    pub fn overflow_content_target(&self) -> Option<Ref<RichTextBlockOverflow>> {
        self.get_value(Self::overflow_content_target_property())
            .and_then(|target| target.get())
    }
    /// Links an overflow view. Cycles and a target already owned by another view
    /// are rejected without changing either chain.
    pub fn try_set_overflow_content_target(
        &self,
        target: Option<Ref<RichTextBlockOverflow>>,
    ) -> bool {
        if target
            .as_ref()
            .is_some_and(|target| !self.can_link_target(target))
        {
            return false;
        }
        self.set_value(
            Self::overflow_content_target_property(),
            target.map(|target| ElementRef::new(&target)),
        );
        true
    }
    pub fn set_overflow_content_target(&self, target: Option<Ref<RichTextBlockOverflow>>) {
        self.try_set_overflow_content_target(target);
    }
    fn can_link_target(&self, target: &RichTextBlockOverflow) -> bool {
        let this = self.to_ref();
        let target_base: Ref<RichTextBlock> = target.to_ref().upcast();
        if this == target_base {
            return false;
        }
        if target
            .flow_parent
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
            .is_some_and(|parent| parent != this)
        {
            return false;
        }
        let mut cursor = Some(target_base);
        let mut visited = std::collections::HashSet::new();
        while let Some(current) = cursor {
            if current == this || !visited.insert(&*current as *const RichTextBlock as usize) {
                return false;
            }
            cursor = current.overflow_content_target().map(|next| next.upcast());
        }
        true
    }
    /// Immediate predecessor, retained separately from the public cached
    /// ContentSource master so continuation breaks remain sequential.
    pub(crate) fn previous_flow_parent(&self) -> Option<Ref<RichTextBlock>> {
        self.flow_parent
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
    }
    pub(crate) fn invalidate_document_layout(&self) {
        self.source.borrow_mut().take();
        self.invalidate_layout();
    }
    fn invalidate_layout(&self) {
        self.clear_page_layout();
        self.invalidate_measure();
        self.invalidate_visual();
        if let Some(target) = self.installed_overflow_target() {
            target.invalidate_layout();
        }
    }
    fn clear_page_layout(&self) {
        self.layout_generation
            .set(self.layout_generation.get().wrapping_add(1));
        self.layout.borrow_mut().take();
    }

    /// Detects invalidation even while an outer formatter call is constructing
    /// a layout that has not yet been published in the cache.
    pub(crate) fn page_layout_generation(&self) -> u64 {
        self.layout_generation.get()
    }

    pub(crate) fn collapses_paragraph_margins(&self) -> bool {
        self.collapse_paragraph_margins.get()
    }

    /// Editable paragraph spacing is additive; read-only block layout uses
    /// the pinned BlockNode margin-collapsing contract.
    pub(crate) fn set_paragraph_margin_collapsing(&self, value: bool) {
        if self.collapse_paragraph_margins.replace(value) != value {
            self.invalidate_document_layout();
        }
    }

    /// Observes the current layout identity without formatting or callbacks.
    pub(crate) fn cached_page_layout(&self) -> Option<Rc<RichTextLayout>> {
        self.layout.borrow().clone()
    }

    /// The baseline published with the current page cache. Reading it does
    /// not format, invoke a virtual origin, or enter nested layout.
    pub(crate) fn cached_baseline_offset(&self) -> Option<f64> {
        self.layout
            .borrow()
            .as_ref()
            .map(|_| self.reported_baseline.get())
    }

    pub(crate) fn page_layout(&self) -> Rc<RichTextLayout> {
        self.page_layout_core(false)
    }

    fn page_layout_core(&self, measure_objects: bool) -> Rc<RichTextLayout> {
        if let Some(layout) = self.layout.borrow().as_ref() {
            return layout.clone();
        }
        let generation = self.layout_generation.get();
        let parent = self
            .flow_parent
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade);
        let (source, start) = if let Some(parent) = parent {
            let layout = parent.page_layout();
            (layout.source.clone(), layout.next.clone())
        } else {
            // Building runs can call custom text elements. Release the cache
            // borrow before doing so and preserve any newer reentrant source.
            let cached_source = self.source.borrow().clone();
            let source =
                cached_source.unwrap_or_else(|| RichTextSource::new(&self.document(), self));
            if self.layout_generation.get() == generation {
                *self.source.borrow_mut() = Some(source.clone());
            }
            (source, RichTextBreak::default())
        };
        let constraint = self.constraint.get();
        let layout = RichTextLayout::new(
            source,
            constraint,
            self.text_wrapping(),
            self.max_lines(),
            &self.text_trimming(),
            self.overflow_content_target().is_none(),
            &start,
            measure_objects.then_some((self, generation)),
        );
        let baseline = layout
            .metrics(self.text_origin())
            .first()
            .map_or(0.0, |line| line.baseline);
        if self.layout_generation.get() != generation {
            // A formatter, embedded control or virtual origin may invalidate
            // this view or publish a newer nested layout. Do not install the
            // preceding source or report its metrics as the current cache.
            return layout;
        }
        let old_overflow = self.reported_overflow.replace(layout.has_overflow);
        let old_trimmed = self.reported_trimmed.replace(layout.is_trimmed);
        let old_baseline = self.reported_baseline.replace(baseline);
        // Publish the complete cache before invoking property or routed-event
        // handlers; they may immediately ask for layout or trimming state.
        *self.layout.borrow_mut() = Some(layout.clone());
        if let Some(target) = self.installed_overflow_target() {
            target.invalidate_layout();
        }
        if old_overflow != layout.has_overflow {
            self.raise_direct_property_changed(
                Self::has_overflow_content_property(),
                &old_overflow,
                &layout.has_overflow,
            );
        }
        if old_baseline != baseline {
            self.raise_direct_property_changed(
                Self::baseline_offset_property(),
                &old_baseline,
                &baseline,
            );
        }
        if old_trimmed != layout.is_trimmed {
            self.raise_direct_property_changed(
                Self::is_text_trimmed_property(),
                &old_trimmed,
                &layout.is_trimmed,
            );
            self.raise_event(&RoutedEventArgs::with_event(
                Self::is_text_trimmed_changed_event(),
            ));
        }
        layout
    }
}

impl Drop for RichTextBlock {
    fn drop(&mut self) {
        self.detach_installed_overflow_on_drop();
        if let Some(subscription) = self.font_features_subscription.get_mut().take() {
            subscription.dispose();
        }
        for subscription in self.view_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        for subscription in self
            .default_highlight_brush_subscriptions
            .get_mut()
            .drain(..)
        {
            subscription.dispose();
        }
        if let Some(subscription) = self.document_subscription.get_mut().take() {
            subscription.dispose();
        }
        self.document.get_mut().set_inline_host(None);
        self.document.get_mut().set_logical_parent(None);
    }
}

struct RichBlockInlineHost(WeakRef<RichTextBlock>);
impl IInlineHost for RichBlockInlineHost {
    fn invalidate(&self) {
        if let Some(this) = self.0.upgrade() {
            this.invalidate_document_layout();
        }
    }
    fn invalidate_render(&self) {
        if let Some(this) = self.0.upgrade() {
            this.invalidate_visual();
        }
    }
    fn with_visual_children(&self, action: &mut dyn FnMut(&FerroList<Ref<Visual>>)) {
        if let Some(this) = self.0.upgrade() {
            action(&this.visual_children());
        }
    }
}

fn scalar_boundary(text: &[u16], position: usize) -> usize {
    let position = position.min(text.len());
    if position > 0
        && position < text.len()
        && (0xD800..=0xDBFF).contains(&text[position - 1])
        && (0xDC00..=0xDFFF).contains(&text[position])
    {
        position - 1
    } else {
        position
    }
}
fn grapheme_before(text: &[u16], position: usize) -> usize {
    let mut result = 0;
    let mut enumerator = GraphemeEnumerator::new(text);
    while let Some(grapheme) = enumerator.move_next() {
        if grapheme.offset() >= position {
            break;
        }
        result = grapheme.offset();
    }
    result
}
fn grapheme_after(text: &[u16], position: usize) -> usize {
    let mut enumerator = GraphemeEnumerator::new(text);
    while let Some(grapheme) = enumerator.move_next() {
        let end = grapheme.offset() + grapheme.length();
        if end > position {
            return end;
        }
    }
    text.len()
}
