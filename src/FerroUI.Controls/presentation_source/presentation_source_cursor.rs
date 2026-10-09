use super::PresentationSource;
use ferroui_base::input::text_input::ITextInputMethodImpl;
use ferroui_base::input::{Cursor, FocusManager, IInputRoot, InputElement};
use ferroui_base::platform::IOptionalFeatureProvider;
use ferroui_base::{Point, Rect, Ref, Size};
use std::rc::Rc;

impl PresentationSource {
    fn update_cursor(&self) {
        let cursor = self.cursor_override.borrow().clone().or_else(|| self.cursor.borrow().clone());
        if let Some(platform_impl) = self.platform_impl() {
            platform_impl.set_cursor(cursor.map(|cursor| cursor.platform_impl().clone()));
        }
    }

    fn set_cursor(&self, cursor: Option<Rc<Cursor>>) {
        *self.cursor.borrow_mut() = cursor;
        self.update_cursor();
    }

    /// This should only be used by the in-process drag source.
    pub fn set_cursor_override(&self, cursor: Option<Rc<Cursor>>) {
        *self.cursor_override.borrow_mut() = cursor;
        self.update_cursor();
    }
}

impl IInputRoot for PresentationSource {
    fn hit_test_chrome_element(&self, point: Point) -> Option<ferroui_base::input::WindowDecorationsElementRole> {
        let visual = self.root_visual()?.get_visual_at_filtered(point, &PresentationSource::chrome_hit_test_filter);
        PresentationSource::get_chrome_role_from_visual(visual)
    }

    fn focus_manager(&self) -> Option<Rc<FocusManager>> {
        Some(PresentationSource::focus_manager(self).clone())
    }

    fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
        self.pointer_over_element.borrow().clone()
    }

    fn set_pointer_over_element(&self, value: Option<Ref<InputElement>>) {
        let old = self.pointer_over_element.replace(value);
        drop(old);
    }

    fn cursor_element(&self) -> Option<Ref<InputElement>> {
        self.cursor_element.borrow().clone()
    }

    fn set_cursor_element(&self, value: Option<Ref<InputElement>>) {
        if *self.cursor_element.borrow() == value {
            return;
        }

        if let Some(subscription) = self.cursor_element_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        let old = self.cursor_element.replace(value.clone());
        drop(old);
        if let Some(new) = &value {
            let weak = self.this.clone();
            let element = new.downgrade();
            let subscription = new.property_changed(move |e| {
                if e.property() == InputElement::cursor_property().as_property() {
                    if let Some(source) = weak.upgrade() {
                        source.set_cursor(element.upgrade().and_then(|element| element.cursor()));
                    }
                }
            });
            *self.cursor_element_subscription.borrow_mut() = Some(subscription);
        }
        self.set_cursor(value.and_then(|element| element.cursor()));
    }

    fn input_method(&self) -> Option<Rc<dyn ITextInputMethodImpl>> {
        let platform_impl = self.platform_impl()?;
        let provider: &dyn IOptionalFeatureProvider = &*platform_impl;
        provider.try_get::<dyn ITextInputMethodImpl>()
    }

    fn root_element(&self) -> Ref<InputElement> {
        PresentationSource::root_element(self)
    }

    fn try_root_element(&self) -> Option<Ref<InputElement>> {
        self.root_visual()
    }

    fn focus_root(&self) -> Ref<InputElement> {
        PresentationSource::focus_root(self)
    }

    fn pointer_over_invalidated(&self) {
        let pre_processor = self.pointer_over_pre_processor.borrow().clone();
        if let Some(pre_processor) = pre_processor {
            pre_processor.scene_invalidated(Rect::from_position_size(Point::new(0.0, 0.0), Size::INFINITY));
        }
    }
}
