use super::*;
use crate::{
    text_view_margin::Margin, DottedLineMargin, EditorAdornmentError, FoldingMargin,
    LineNumberMargin, TextViewMargin,
};
use ferroui_base::layout::Orientation;
use ferroui_base::reactive::Subscription;
use ferroui_controls::StackPanel;
use std::rc::Weak;

#[derive(Clone, Copy, PartialEq)]
struct MarginViewport {
    offset: Vector,
    size: Size,
    width: f64,
    revision: u64,
}

struct LineNumberGutter {
    numbers: Ref<LineNumberMargin>,
    binding: TextViewMargin,
    separator: Ref<DottedLineMargin>,
}

#[derive(Default)]
pub(super) struct MarginHost {
    panel: OnceCell<Ref<StackPanel>>,
    line_numbers: OnceCell<LineNumberGutter>,
    folding: OnceCell<(Ref<FoldingMargin>, TextViewMargin)>,
    builtin_widths: Cell<(f64, f64)>,
    items: RefCell<Vec<TextViewMargin>>,
    tracked: RefCell<Vec<Weak<Margin>>>,
    width: Cell<f64>,
    previous_viewport: Cell<Option<MarginViewport>>,
    changing: Cell<bool>,
    notifying: Cell<bool>,
    notification_pending: Cell<bool>,
    changed: Rc<HandlerList<dyn Fn()>>,
}

struct ResetFlag<'a>(&'a Cell<bool>);

impl Drop for ResetFlag<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl TextArea {
    pub(crate) fn update_folding_gutter(&self, editor: &TextEditor) {
        let installed = editor.folding_manager().is_some();
        if !installed && self.margins.folding.get().is_none() {
            return;
        }
        let (margin, binding) = self.margins.folding.get_or_init(|| {
            let margin = FoldingMargin::new();
            let binding = margin.as_margin();
            (margin, binding)
        });
        margin.set_current_value(FoldingMargin::font_size_property(), editor.font_size());
        let mounted = margin.visual_parent().is_some();
        let result = if installed && !mounted {
            binding.mount(self, || {
                self.logical_children().add(margin.clone().upcast());
                self.visual_children().add(margin.clone().upcast());
            })
        } else if !installed && mounted {
            binding.unmount(|| {
                self.visual_children().remove(&margin.clone().upcast());
                self.logical_children().remove(&margin.clone().upcast());
            })
        } else {
            Ok(())
        };
        if let Err(error) = result {
            editor.report_error(error.to_string());
        }
    }

    pub(super) fn folding_margin(&self) -> Option<&Ref<FoldingMargin>> {
        self.margins
            .folding
            .get()
            .map(|(margin, _)| margin)
            .filter(|margin| margin.visual_parent().is_some())
    }

    pub(super) fn folding_gutter_width(&self) -> f64 {
        self.folding_margin()
            .map_or(0.0, |margin| margin.preferred_width())
    }

    pub(crate) fn update_line_number_gutter(&self, editor: &TextEditor) {
        let shown = editor.show_line_numbers();
        if !shown && self.margins.line_numbers.get().is_none() {
            return;
        }
        let gutter = self.margins.line_numbers.get_or_init(|| {
            let numbers = LineNumberMargin::new();
            let binding = numbers.as_margin();
            let separator = DottedLineMargin::new();
            separator.set_is_hit_test_visible(false);
            LineNumberGutter {
                numbers,
                binding,
                separator,
            }
        });
        gutter.numbers.set_current_value(
            LineNumberMargin::font_family_property(),
            editor.font_family(),
        );
        gutter
            .numbers
            .set_current_value(LineNumberMargin::font_size_property(), editor.font_size());
        gutter.numbers.set_current_value(LineNumberMargin::font_style_property(), editor.font_style());
        gutter.numbers.set_current_value(LineNumberMargin::font_weight_property(), editor.font_weight());
        gutter.numbers.set_current_value(LineNumberMargin::font_stretch_property(), editor.font_stretch());
        gutter
            .numbers
            .set_min_width_in_digits(editor.line_numbers_min_width_in_digits());
        gutter
            .numbers
            .set_foreground(editor.line_numbers_foreground());
        gutter.separator.set_margin(editor.line_numbers_margin());
        gutter
            .separator
            .set_stroke(editor.line_numbers_foreground());
        let mounted = gutter.numbers.visual_parent().is_some();
        let result = if shown && !mounted {
            gutter.binding.mount(self, || {
                for control in [
                    gutter.numbers.clone().upcast::<Control>(),
                    gutter.separator.clone().upcast(),
                ] {
                    self.logical_children().add(control.clone().upcast());
                    self.visual_children().add(control.upcast());
                }
            })
        } else if !shown && mounted {
            gutter.binding.unmount(|| {
                for control in [
                    gutter.numbers.clone().upcast::<Control>(),
                    gutter.separator.clone().upcast(),
                ] {
                    self.visual_children().remove(&control.clone().upcast());
                    self.logical_children().remove(&control.upcast());
                }
            })
        } else {
            Ok(())
        };
        if let Err(error) = result {
            editor.report_error(error.to_string());
        }
    }

    pub(super) fn line_number_margin(&self) -> Option<&Ref<LineNumberMargin>> {
        self.margins
            .line_numbers
            .get()
            .map(|gutter| &gutter.numbers)
    }

    pub(super) fn line_number_strip_width(&self, editor: &TextEditor) -> f64 {
        self.line_number_margin().map_or(0.0, |margin| {
            margin.preferred_width(editor.document().line_count())
        })
    }

    pub(super) fn line_number_gutter_contains(&self, editor: &TextEditor, x: f64) -> bool {
        let left = self.custom_margin_width();
        editor.show_line_numbers()
            && (left..left + self.line_number_strip_width(editor)).contains(&x)
    }

    pub(super) fn invalidate_builtin_margins(&self) {
        if let Some(margin) = self.line_number_margin() {
            margin.invalidate_visual();
        }
        if let Some(margin) = self.folding_margin() {
            margin.invalidate_visual();
        }
    }

    pub(super) fn line_number_separator_width(editor: &TextEditor) -> f64 {
        let margin = editor.line_numbers_margin();
        (DottedLineMargin::STROKE_WIDTH + margin.left + margin.right).max(0.0)
    }

    pub(super) fn line_number_separator_contains(&self, x: f64) -> bool {
        self.editor().is_some_and(|editor| {
            if !editor.show_line_numbers() {
                return false;
            }
            let left = self.custom_margin_width() + self.line_number_strip_width(&editor);
            (left..left + Self::line_number_separator_width(&editor)).contains(&x)
        })
    }

    /// Application margins in display order, before line numbers and folding.
    pub fn left_margins(&self) -> Vec<TextViewMargin> {
        self.margins.items.borrow().clone()
    }

    pub fn left_margins_changed(&self, handler: impl Fn() + 'static) -> Subscription {
        let handlers = self.margins.changed.clone();
        let token = handlers.add(Rc::new(handler));
        Disposable::create(move || {
            handlers.remove(token);
        })
        .into()
    }

    /// Adds a native margin before the built-in gutters.
    pub fn insert_left_margin(
        &self,
        index: usize,
        margin: TextViewMargin,
    ) -> Result<(), EditorAdornmentError> {
        let mutation = self.begin_margin_mutation()?;
        if index > self.margins.items.borrow().len() {
            return Err(EditorAdornmentError::InvalidIndex);
        }
        let control = margin.control();
        self.validate_adornment(&control)?;
        margin.mount(self, || {
            let panel = self.margins.panel.get_or_init(|| {
                let panel = StackPanel::new();
                panel.set_orientation(Orientation::Horizontal);
                panel.set_clip_to_bounds(true);
                self.logical_children().add(panel.clone().upcast());
                self.visual_children().add(panel.clone().upcast());
                panel
            });
            self.margins
                .items
                .borrow_mut()
                .insert(index, margin.clone());
            panel.children().insert(index, control);
        })?;
        drop(mutation);
        self.margins_changed();
        Ok(())
    }

    pub fn remove_left_margin(&self, index: usize) -> Result<TextViewMargin, EditorAdornmentError> {
        let mutation = self.begin_margin_mutation()?;
        let margin = self
            .margins
            .items
            .borrow()
            .get(index)
            .cloned()
            .ok_or(EditorAdornmentError::InvalidIndex)?;
        margin.unmount(|| {
            self.margins.items.borrow_mut().remove(index);
            if margin.control().is_keyboard_focus_within() {
                self.focus();
            }
            self.margins
                .panel
                .get()
                .unwrap()
                .children()
                .remove_at(index);
        })?;
        drop(mutation);
        self.margins_changed();
        Ok(margin)
    }

    pub fn move_left_margin(&self, from: usize, to: usize) -> Result<(), EditorAdornmentError> {
        let mutation = self.begin_margin_mutation()?;
        let length = self.margins.items.borrow().len();
        if from >= length || to >= length {
            return Err(EditorAdornmentError::InvalidIndex);
        }
        if from != to {
            {
                let mut margins = self.margins.items.borrow_mut();
                let margin = margins.remove(from);
                margins.insert(to, margin);
            }
            self.margins
                .panel
                .get()
                .unwrap()
                .children()
                .move_item(from, to);
            drop(mutation);
            self.margins_changed();
        }
        Ok(())
    }

    fn begin_margin_mutation(&self) -> Result<ResetFlag<'_>, EditorAdornmentError> {
        if self.rendering.get()
            || self.margins.notifying.get()
            || self.margins.changing.replace(true)
        {
            Err(EditorAdornmentError::Busy)
        } else {
            Ok(ResetFlag(&self.margins.changing))
        }
    }

    fn margins_changed(&self) {
        self.margins.previous_viewport.set(None);
        self.invalidate_line_measure();
        for (_, handler) in self.margins.changed.snapshot().iter() {
            handler();
        }
    }

    pub(super) fn measure_margins(&self, available: Size) {
        if let Some(panel) = self.margins.panel.get() {
            if self.margins.changing.replace(true) {
                return;
            }
            let _layout = ResetFlag(&self.margins.changing);
            panel.measure(available);
        }
    }

    pub(super) fn arrange_margins(&self, size: Size) -> bool {
        if self.margins.changing.replace(true) {
            return false;
        }
        let _layout = ResetFlag(&self.margins.changing);
        let changed = self.margins.panel.get().is_some_and(|panel| {
            panel.measure(size);
            let width = panel.desired_size().width.min(size.width);
            let changed = self.margins.width.replace(width) != width;
            panel.arrange(Rect::new(0.0, 0.0, width, size.height));
            changed
        });
        let mut number_width = 0.0;
        if let (Some(gutter), Some(editor)) = (
            self.margins.line_numbers.get(),
            self.editor().filter(|editor| editor.show_line_numbers()),
        ) {
            let left = self.custom_margin_width();
            number_width = self.line_number_strip_width(&editor);
            gutter.numbers.measure(Size::new(number_width, size.height));
            gutter
                .numbers
                .arrange(Rect::new(left, 0.0, number_width, size.height));
            let width = Self::line_number_separator_width(&editor);
            gutter.separator.measure(Size::new(width, size.height));
            gutter
                .separator
                .arrange(Rect::new(left + number_width, 0.0, width, size.height));
        }
        let mut folding_width = 0.0;
        if let Some(margin) = self.folding_margin() {
            folding_width = margin.preferred_width();
            margin.measure(Size::new(folding_width, size.height));
            margin.arrange(Rect::new(
                self.gutter_width() - folding_width,
                0.0,
                folding_width,
                size.height,
            ));
        }
        let widths = (number_width, folding_width);
        let builtin_width_changed = self.margins.builtin_widths.replace(widths) != widths;
        changed || builtin_width_changed
    }

    pub(super) fn custom_margin_width(&self) -> f64 {
        self.margins.width.get()
    }

    pub(super) fn is_margin_visual(&self, source: &Visual) -> bool {
        self.margins.panel.get().is_some_and(|panel| {
            panel.is_visual_ancestor_of(source) || panel.ptr_eq(&source.to_ref())
        })
    }

    pub(super) fn refresh_margin_viewport(&self) {
        let key = MarginViewport {
            offset: self.offset.get(),
            size: self.viewport.get(),
            width: self.custom_margin_width(),
            revision: self.visual_lines.revision.get(),
        };
        if self.margins.tracked.borrow().is_empty()
            || self.margins.previous_viewport.get() == Some(key)
            || self.margins.notification_pending.replace(true)
        {
            return;
        }
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(view) = weak.upgrade() {
                    view.margins.notification_pending.set(false);
                    view.notify_margin_viewport();
                }
            },
            DispatcherPriority::NORMAL,
        );
    }

    pub(crate) fn track_margin(&self, margin: Weak<Margin>) {
        self.margins.tracked.borrow_mut().push(margin);
        self.invalidate_margin_viewport();
    }

    pub(crate) fn untrack_margin(&self, margin: *const Margin) {
        self.margins
            .tracked
            .borrow_mut()
            .retain(|tracked| tracked.as_ptr() != margin);
    }

    pub(crate) fn invalidate_margin_viewport(&self) {
        self.margins.previous_viewport.set(None);
        self.refresh_margin_viewport();
    }

    pub(crate) fn notify_margin_document_changed(&self) {
        let tracked = self.margins.tracked.borrow().clone();
        for margin in tracked.into_iter().filter_map(|margin| margin.upgrade()) {
            margin.view_document_changed(self);
        }
    }

    fn notify_margin_viewport(&self) {
        self.margins.previous_viewport.set(Some(MarginViewport {
            offset: self.offset.get(),
            size: self.viewport.get(),
            width: self.custom_margin_width(),
            revision: self.visual_lines.revision.get(),
        }));
        self.margins.notifying.set(true);
        let _notice = ResetFlag(&self.margins.notifying);
        let tracked = self.margins.tracked.borrow().clone();
        for item in tracked.into_iter().filter_map(|margin| margin.upgrade()) {
            item.update_viewport(self);
        }
    }

    pub(super) fn disconnect_margins(&self) {
        for item in self
            .margins
            .tracked
            .take()
            .into_iter()
            .filter_map(|margin| margin.upgrade())
        {
            item.view_dropped(self);
        }
    }
}
