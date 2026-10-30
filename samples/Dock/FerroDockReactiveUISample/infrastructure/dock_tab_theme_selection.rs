// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Application-owned Browser overlay on the existing Fluent styles.
use ferrodock_ferro_themes_browser::BrowserTabTheme;
use ferroui_base::{styling::Styles, Ref};
use std::{cell::RefCell, rc::Rc};

pub(crate) struct DockTabThemeSelection {
    styles: Ref<Styles>,
    browser: RefCell<Option<Ref<BrowserTabTheme>>>,
}
impl DockTabThemeSelection {
    pub(crate) fn new(styles: Ref<Styles>) -> Rc<Self> {
        Rc::new(Self {
            styles,
            browser: RefCell::new(None),
        })
    }
    pub(crate) fn selected(&self) -> i32 {
        i32::from(self.browser.borrow().is_some())
    }
    pub(crate) fn select(&self, index: i32) {
        match index {
            0 => {
                let browser = self.browser.borrow_mut().take();
                if let Some(browser) = browser {
                    self.styles.remove(browser.as_style());
                }
            }
            1 if self.browser.borrow().is_none() => {
                let browser = BrowserTabTheme::new();
                *self.browser.borrow_mut() = Some(browser.clone());
                self.styles.add(browser.as_style());
            }
            _ => {}
        }
    }
}
