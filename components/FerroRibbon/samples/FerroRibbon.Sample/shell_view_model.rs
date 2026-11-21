// Copyright (c) 2026 Wiesław Šoltés. MIT.
//! ShellPage navigation translated through an application-specific native surface.
//! No controls, visual trees, dispatchers or global application state live here.
use ferroribbon_core::theming::{RibbonChromeStyle, RibbonThemePalette, RibbonThemeStyle};
use ferroui_reactiveui::{ICommand, ReactiveCommand};
use std::{
    cell::{Cell, OnceCell, RefCell},
    fmt,
    rc::Rc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellError {
    NullKey,
    UnknownPage(String),
    InvalidTag,
    ChildrenChanged,
    Native(String),
}
impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullKey => f.write_str("A demo page key is required"),
            Self::UnknownPage(key) => write!(f, "Unknown demo page {key:?}"),
            Self::InvalidTag => f.write_str("The rail Tag must be a string or null"),
            Self::ChildrenChanged => {
                f.write_str("The shell page children changed during traversal")
            }
            Self::Native(error) => f.write_str(error),
        }
    }
}
impl std::error::Error for ShellError {}

pub trait ShellSurface: 'static {
    type Page: Clone;
    fn surface_style(&self) -> RibbonThemeStyle;
    fn theme_style(&self) -> RibbonThemeStyle;
    fn chrome(&self) -> RibbonChromeStyle;
    fn palette(&self) -> Rc<RibbonThemePalette>;
    fn palette_override(&self) -> Option<Rc<RibbonThemePalette>>;
    fn apply_theme(
        &self,
        palette: Rc<RibbonThemePalette>,
        chrome: RibbonChromeStyle,
        style: RibbonThemeStyle,
    ) -> Result<(), ShellError>;
    fn apply_palette(&self, palette: Rc<RibbonThemePalette>) -> Result<(), ShellError>;
    fn create_page(&self, key: &str) -> Result<Self::Page, ShellError>;
    fn add_page(&self, page: &Self::Page);
    fn remove_page(&self, page: &Self::Page);
    fn show_only(&self, page: &Self::Page) -> Result<(), ShellError>;
    fn check_rail(&self, key: Option<&str>) -> Result<(), ShellError>;
    fn suspend(&self, page: &Self::Page);
    fn resume(&self, page: &Self::Page);
    fn is_cad(&self, page: &Self::Page) -> bool;
    fn follow_cad_theme(&self, page: &Self::Page);
    fn reset_rail_theme(&self);
}

// This is the shell's six-page cache, not a general dictionary implementation.
// Slots retain the source runtime's observable stale-page removal order when a
// removed page is recreated. See the unchanged-class reference corpus.
struct PageCache<P> {
    slots: Vec<Option<(String, P)>>,
    vacant: Vec<usize>,
}
impl<P: Clone> PageCache<P> {
    fn get(&self, key: Option<&str>) -> Result<Option<P>, ShellError> {
        let key = key.ok_or(ShellError::NullKey)?;
        Ok(self
            .slots
            .iter()
            .flatten()
            .find(|(k, _)| k == key)
            .map(|(_, p)| p.clone()))
    }
    fn insert(&mut self, key: String, page: P) {
        if let Some((_, value)) = self.slots.iter_mut().flatten().find(|(k, _)| *k == key) {
            *value = page;
        } else if let Some(index) = self.vacant.pop() {
            self.slots[index] = Some((key, page));
        } else {
            self.slots.push(Some((key, page)));
        }
    }
    fn remove(&mut self, key: &str) {
        if let Some(index) = self
            .slots
            .iter()
            .position(|p| p.as_ref().is_some_and(|(k, _)| k == key))
        {
            self.slots[index] = None;
            self.vacant.push(index);
        }
    }
}

pub struct ShellViewModel<H: ShellSurface> {
    surface: H,
    current_page: RefCell<Option<String>>,
    office_chrome: Cell<RibbonChromeStyle>,
    pages: RefCell<PageCache<H::Page>>,
    navigation: OnceCell<Rc<dyn ICommand>>,
}
impl<H: ShellSurface> ShellViewModel<H> {
    pub fn new(surface: H) -> Rc<Self> {
        let this = Rc::new(Self {
            surface,
            current_page: RefCell::new(Some("word".into())),
            office_chrome: Cell::new(RibbonChromeStyle::Neutral),
            pages: RefCell::new(PageCache {
                slots: Vec::new(),
                vacant: Vec::new(),
            }),
            navigation: OnceCell::new(),
        });
        let weak = Rc::downgrade(&this);
        assert!(
            this.navigation
                .set(Rc::new(ReactiveCommand::create_param(
                    move |key: Option<String>| {
                        if let Some(this) = weak.upgrade() {
                            this.show_optional(key.as_deref())
                                .expect("Shell navigation");
                        }
                    }
                )))
                .is_ok()
        );
        this
    }
    pub fn navigation(&self) -> Rc<dyn ICommand> {
        self.navigation
            .get()
            .expect("Shell command initialized")
            .clone()
    }
    pub fn current_page(&self) -> Option<String> {
        self.current_page.borrow().clone()
    }
    pub fn current_content(&self) -> Result<Option<H::Page>, ShellError> {
        self.pages
            .borrow()
            .get(self.current_page.borrow().as_deref())
    }
    pub fn show(&self, key: &str) -> Result<(), ShellError> {
        self.show_optional(Some(key))
    }
    pub fn show_optional(&self, key: Option<&str>) -> Result<(), ShellError> {
        if self.current_page.borrow().as_deref() != key {
            let previous = self.current_content()?;
            if let Some(previous) = previous {
                self.surface.suspend(&previous);
            }
        }
        self.current_page.replace(key.map(str::to_owned));
        let style = if key == Some("cad") {
            RibbonThemeStyle::Cad
        } else {
            self.surface.surface_style()
        };
        if key == Some("cad") && self.surface.theme_style() != RibbonThemeStyle::Cad {
            self.office_chrome.set(self.surface.chrome());
        }
        if self.surface.theme_style() != style || key == Some("cad") {
            self.surface.apply_theme(
                if key == Some("cad") {
                    RibbonThemePalette::cad()
                } else {
                    self.surface.palette()
                },
                if key == Some("cad") {
                    RibbonChromeStyle::Neutral
                } else {
                    self.office_chrome.get()
                },
                style,
            )?;
        }
        let cached = self.pages.borrow().get(key)?;
        let page = match cached {
            Some(page) => page,
            None => {
                let key = key.ok_or(ShellError::NullKey)?;
                let page = self.surface.create_page(key)?;
                self.pages.borrow_mut().insert(key.to_owned(), page.clone());
                self.surface.add_page(&page);
                page
            }
        };
        self.surface.show_only(&page)?;
        self.surface.check_rail(key)?;
        self.surface.resume(&page);
        if self.surface.is_cad(&page) {
            self.surface.follow_cad_theme(&page);
            return Ok(());
        }
        self.surface.reset_rail_theme();
        self.surface.apply_palette(
            self.surface
                .palette_override()
                .unwrap_or_else(|| match key {
                    Some("excel") => RibbonThemePalette::excel(),
                    Some("powerpoint") => RibbonThemePalette::power_point(),
                    Some("tools") => RibbonThemePalette::graphite(),
                    _ => RibbonThemePalette::word(),
                }),
        )
    }
    pub fn invalidate_office_pages(&self) {
        let current = self.current_page();
        let stale = self
            .pages
            .borrow()
            .slots
            .iter()
            .flatten()
            .filter(|(key, _)| key != "settings" && key != "cad" && Some(key) != current.as_ref())
            .cloned()
            .collect::<Vec<_>>();
        for (key, page) in &stale {
            self.surface.remove_page(page);
            self.pages.borrow_mut().remove(key);
        }
    }
}

#[cfg(test)]
#[path = "shell_view_model_tests.rs"]
mod tests;
