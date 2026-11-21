// Copyright (c) 2026 Wiesław Šoltés. MIT.
//! Shared demo state from ShellPage.xaml.cs::DemoSettings. No UI tree belongs here.
use ferroribbon::{controls::Ribbon, theming::RibbonTheme};
use ferroribbon_core::{
    RibbonDensity, RibbonDisplayMode,
    theming::{RibbonThemePalette, RibbonThemeStyle},
};
use ferroui_base::{
    Ref,
    reactive::{AnonymousObserver, IDisposable, IObservable, IObserver, LightweightSubject},
    utilities::DateTimeFormatInfo,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub type DemoClock = Rc<dyn Fn() -> Result<String, time::error::IndeterminateOffset>>;

/// One UI-thread service is shared by the native demo pages. Independent instances
/// allow an application composition root to supply a clock without changing views.
pub struct DemoSettings {
    palette_override: RefCell<Option<Rc<RibbonThemePalette>>>,
    word_ribbon: RefCell<Option<Ref<Ribbon>>>,
    surface_style: Cell<RibbonThemeStyle>,
    density: Cell<RibbonDensity>,
    display_mode: Cell<Option<RibbonDisplayMode>>,
    log: Rc<RefCell<Vec<String>>>,
    clock: DemoClock,
    surface_style_changed: LightweightSubject<()>,
    changed: LightweightSubject<()>,
    logged: LightweightSubject<Option<String>>,
}

/// A concrete demo subscription. Removal is idempotent and also occurs on Drop.
pub struct DemoSubscription(Rc<dyn IDisposable>);
impl DemoSubscription {
    pub fn unsubscribe(&self) {
        self.0.dispose();
    }
}
impl Drop for DemoSubscription {
    fn drop(&mut self) {
        self.unsubscribe();
    }
}

ferroui_base::ferro_thread_local! {
    static CURRENT: Rc<DemoSettings> = DemoSettings::new(Rc::new(local_timestamp));
}

fn local_timestamp() -> Result<String, time::error::IndeterminateOffset> {
    local_timestamp_at(time::OffsetDateTime::now_utc())
}
fn local_timestamp_at(
    utc: time::OffsetDateTime,
) -> Result<String, time::error::IndeterminateOffset> {
    let time = utc.to_offset(time::UtcOffset::local_offset_at(utc)?).time();
    // In the source custom HH:mm:ss format, ':' is the culture's time separator.
    // The selected framework already exposes these conventions; no formatter or
    // timezone runtime is recreated in the port.
    let info = DateTimeFormatInfo::current_info();
    Ok(timestamp(time, info.time_separator()))
}
fn timestamp(time: time::Time, separator: &str) -> String {
    let (hour, minute, second) = time.as_hms();
    format!("{hour:02}{separator}{minute:02}{separator}{second:02}")
}

impl DemoSettings {
    pub fn new(clock: DemoClock) -> Rc<Self> {
        Rc::new(Self {
            palette_override: RefCell::new(None),
            word_ribbon: RefCell::new(None),
            surface_style: Cell::new(RibbonThemeStyle::Office),
            density: Cell::new(RibbonDensity::default()),
            display_mode: Cell::new(None),
            log: Rc::new(RefCell::new(Vec::new())),
            clock,
            surface_style_changed: LightweightSubject::new(),
            changed: LightweightSubject::new(),
            logged: LightweightSubject::new(),
        })
    }
    pub fn current() -> Rc<Self> {
        CURRENT.with(Rc::clone)
    }
    pub fn palette_override(&self) -> Option<Rc<RibbonThemePalette>> {
        self.palette_override.borrow().clone()
    }
    pub fn set_palette_override(&self, value: Option<Rc<RibbonThemePalette>>) {
        self.palette_override.replace(value);
    }
    pub fn word_ribbon(&self) -> Option<Ref<Ribbon>> {
        self.word_ribbon.borrow().clone()
    }
    pub fn set_word_ribbon(&self, value: Option<Ref<Ribbon>>) {
        self.word_ribbon.replace(value);
    }
    pub fn surface_style(&self) -> RibbonThemeStyle {
        self.surface_style.get()
    }
    pub fn set_surface_style(
        &self,
        style: RibbonThemeStyle,
    ) -> Result<(), ferroribbon::theming::RibbonThemeApplyError> {
        if self.surface_style.get() == style {
            return Ok(());
        }
        self.surface_style.set(style);
        RibbonTheme::apply_style(style)?;
        self.surface_style_changed.on_next(());
        Ok(())
    }
    pub fn density(&self) -> RibbonDensity {
        self.density.get()
    }
    pub fn display_mode(&self) -> Option<RibbonDisplayMode> {
        self.display_mode.get()
    }
    pub fn apply(&self, density: RibbonDensity, display_mode: Option<RibbonDisplayMode>) {
        self.density.set(density);
        self.display_mode.set(display_mode);
        self.changed.on_next(());
    }
    /// The original public list retains shared identity and external mutations.
    pub fn log(&self) -> Rc<RefCell<Vec<String>>> {
        self.log.clone()
    }
    pub fn write(&self, message: Option<&str>) -> Result<(), time::error::IndeterminateOffset> {
        let line = format!("{}  {}", (self.clock)()?, message.unwrap_or_default());
        {
            let mut log = self.log.borrow_mut();
            log.insert(0, line);
            // Exactly one tail removal, including an externally oversized list.
            if log.len() > 200 {
                log.pop();
            }
        }
        self.logged.on_next(message.map(str::to_owned));
        Ok(())
    }
    pub fn surface_style_changed(&self, callback: impl Fn() + 'static) -> DemoSubscription {
        DemoSubscription(
            self.surface_style_changed
                .subscribe(Rc::new(AnonymousObserver::new(move |()| callback()))),
        )
    }
    pub fn changed(&self, callback: impl Fn() + 'static) -> DemoSubscription {
        DemoSubscription(
            self.changed
                .subscribe(Rc::new(AnonymousObserver::new(move |()| callback()))),
        )
    }
    pub fn logged(&self, callback: impl Fn(Option<String>) + 'static) -> DemoSubscription {
        DemoSubscription(
            self.logged
                .subscribe(Rc::new(AnonymousObserver::new(callback))),
        )
    }
}

impl crate::cad::cad_view_model::CadLog for DemoSettings {
    fn write(&self, line: &str) {
        self.write(Some(line))
            .expect("Native local time is unavailable for the demo log");
    }
}

#[cfg(test)]
#[path = "demo_settings_tests.rs"]
mod tests;
