//! One wallpaper and independently reserved panels per output. The application,
//! session services, shell state and command endpoint are shared by all windows.
use super::{
    appearance::Appearance, clock, create, desktop, root, OutputMetrics, Scene, Shell,
    ThemeController,
};
use crate::{services::ApplicationCatalog, session::Session};
use ferroui_base::media::{immutable::ImmutableSolidColorBrush, Color};
use ferroui_base::threading::{CancellationTokenSource, Dispatcher, DispatcherPriority};
use ferroui_base::{FerroLocator, LocatorExtensions, Ref};
use ferroui_controls::{
    platform::IScreenImpl, Application, Control, Screens, Window, WindowBaseImpl,
    WindowTransparencyLevel, WindowTransparencyLevelCollection,
};
use ferroui_view::prelude::*;
use ferroui_wayland::layer_shell::{LayerConfig, LayerWindow};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};
use time::OffsetDateTime;

pub(super) fn transparent(window: &Window) {
    window.set_transparency_level_hint(WindowTransparencyLevelCollection::new(vec![
        WindowTransparencyLevel::transparent(),
    ]));
    window.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(
        Color::from_argb(0, 0, 0, 0),
    ))));
}

struct Surface {
    window: Ref<Window>,
    owner: Owner,
    config: LayerConfig,
}

impl Surface {
    fn new(
        shell: &Rc<Shell>,
        output: &str,
        namespace: &str,
        config: LayerConfig,
        width: i32,
        content: impl FnOnce() -> Element,
    ) -> Result<Self, String> {
        let window = LayerWindow::new(namespace, Some(output), config)?;
        transparent(&window);
        if config.keyboard == ferroui_wayland::layer_shell::KeyboardInteractivity::Exclusive {
            // A newly activated layer may have its Window as the focus root until
            // the first control is loaded. Route accelerators from that root too.
            let state = Rc::downgrade(shell);
            window.add_handler_with(
                ferroui_base::input::InputElement::key_down_event(),
                move |_, event| {
                    if let Some(shell) = state.upgrade() {
                        super::shortcut(&shell, event);
                    }
                },
                ferroui_base::interactivity::RoutingStrategies::TUNNEL,
                false,
            );
        }
        let owner = Owner::new_root();
        let tree = owner.with(|| {
            provide_context(shell.appearance);
            let metrics = OutputMetrics {
                width: signal(f64::from(width)),
                pins: signal(pin_capacity(f64::from(width))),
            };
            provide_context(metrics);
            let state = Rc::downgrade(shell);
            let name = output.to_owned();
            grid()
                .on_size_changed(move |e| {
                    let width = e.new_size().width;
                    metrics.width.set_if_changed(width);
                    metrics.pins.set_if_changed(pin_capacity(width));
                })
                .with(move |grid| {
                    let name = name.clone();
                    let state = state.clone();
                    let event = ferroui_base::input::InputElement::pointer_pressed_event();
                    let token = grid.add_handler_with(
                        event,
                        move |_, _| {
                            if let Some(shell) = state.upgrade() {
                                shell.clicked_output.replace(Some(name.clone()));
                            }
                        },
                        ferroui_base::interactivity::RoutingStrategies::TUNNEL,
                        true,
                    );
                    let weak = grid.downgrade();
                    on_cleanup(move || {
                        if let Some(grid) = weak.upgrade() {
                            grid.remove_handler(event, token);
                        }
                    });
                })
                .child(content())
                .into_element()
                .mount()
        });
        window.set_content(Some(Control::boxed(tree)));
        WindowBaseImpl::show(&*window);
        Ok(Self {
            window,
            owner,
            config,
        })
    }

    fn configure(&mut self, config: LayerConfig) -> Result<(), String> {
        if config != self.config {
            LayerWindow::configure(&self.window, config)?;
            self.config = config;
        }
        Ok(())
    }
}

fn pin_capacity(width: f64) -> usize {
    if width >= 1024.0 {
        4
    } else if width >= 800.0 {
        2
    } else {
        0
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.window.close();
        self.owner.dispose();
    }
}

struct Output {
    _wallpaper: Surface,
    top: Option<Surface>,
    bottom: Option<Surface>,
}

struct Host {
    shell: Rc<Shell>,
    clock: Signal<OffsetDateTime>,
    outputs: RefCell<BTreeMap<String, Output>>,
    widths: RefCell<BTreeMap<String, i32>>,
    overlay: RefCell<Option<(String, Surface)>>,
    output_dirty: Cell<bool>,
    queued: Cell<bool>,
    cancel: CancellationTokenSource,
    error: RefCell<Option<String>>,
}

impl Host {
    fn panel(
        &self,
        slot: &mut Option<Surface>,
        output: &str,
        width: i32,
        top: bool,
        height: f64,
    ) -> Result<(), String> {
        if height <= 0.0 {
            slot.take();
        } else {
            let config = LayerConfig::panel(top, height.round().max(1.0) as u32);
            if let Some(panel) = slot {
                panel.configure(config)?;
            } else {
                let shell = self.shell.clone();
                let clock = self.clock;
                let namespace = if top {
                    "ferro-desktop-top"
                } else {
                    "ferro-desktop-bottom"
                };
                *slot = Some(Surface::new(
                    &self.shell,
                    output,
                    namespace,
                    config,
                    width,
                    move || {
                        dynamic(move || desktop::panel(&shell, clock, shell.appearance.get(), top))
                            .into_element()
                    },
                )?);
            }
        }
        Ok(())
    }

    fn reconcile(&self) -> Result<(), String> {
        let mut output_set_changed = false;
        if self.output_dirty.replace(false) {
            let updated: BTreeMap<_, _> = LayerWindow::outputs()?
                .into_iter()
                .filter_map(|o| Some((o.name?, o.logical_size.width)))
                .collect();
            let mut previous = self.widths.borrow_mut();
            output_set_changed = previous.keys().ne(updated.keys());
            *previous = updated;
        }
        let widths = self.widths.borrow();
        let mut outputs = self.outputs.borrow_mut();
        outputs.retain(|name, _| widths.contains_key(name));
        let appearance: Appearance = self.shell.appearance.get_untracked();
        let (top, bottom) = appearance.insets();
        for (name, width) in widths.iter() {
            if !outputs.contains_key(name) {
                let shell = self.shell.clone();
                let clock = self.clock;
                let wallpaper = Surface::new(
                    &self.shell,
                    name,
                    "ferro-desktop-background",
                    LayerConfig::background(),
                    *width,
                    move || {
                        dynamic(move || desktop::background(&shell, clock, shell.appearance.get()))
                            .into_element()
                    },
                )?;
                outputs.insert(
                    name.clone(),
                    Output {
                        _wallpaper: wallpaper,
                        top: None,
                        bottom: None,
                    },
                );
            }
            let output = outputs.get_mut(name).unwrap();
            self.panel(&mut output.top, name, *width, true, top)?;
            self.panel(&mut output.bottom, name, *width, false, bottom)?;
        }
        drop(outputs);
        let mut overlay = self.overlay.borrow_mut();
        if overlay
            .as_ref()
            .is_some_and(|(name, _)| !widths.contains_key(name))
        {
            overlay.take();
        }
        if self.shell.scene.get_untracked() == Scene::Desktop || widths.is_empty() {
            if let Some((_, cached)) = overlay.as_ref() {
                // Unmap the layer to release keyboard focus and stop its render
                // loop. Keep at most one mounted control tree across invocations.
                WindowBaseImpl::hide(&*cached.window);
            }
            self.shell.clicked_output.take();
            return Ok(());
        }
        if let Some((_, surface)) = overlay
            .as_ref()
            .filter(|(_, surface)| surface.window.is_visible())
        {
            if output_set_changed {
                // Sway may clear layer keyboard focus when any head disappears, even
                // when the focused overlay stays on a surviving head. Remap its role
                // on the same output without rebuilding the retained control tree.
                WindowBaseImpl::hide(&*surface.window);
                WindowBaseImpl::show(&*surface.window);
            }
            return Ok(());
        }
        let requested = self
            .shell
            .clicked_output
            .take()
            .or_else(|| self.shell.session.as_ref()?.focused_output());
        let output = requested
            .as_ref()
            .filter(|n| widths.contains_key(*n))
            .unwrap_or_else(|| widths.keys().next().unwrap());
        if let Some((_, cached)) = overlay.as_ref().filter(|(name, _)| name == output) {
            WindowBaseImpl::show(&*cached.window);
        } else {
            overlay.take();
            let shell = self.shell.clone();
            let window = Surface::new(
                &shell,
                output,
                "ferro-desktop-overlay",
                LayerConfig::overlay(),
                widths[output],
                || root(shell.clone(), false),
            )?;
            *overlay = Some((output.clone(), window));
        }
        Ok(())
    }

    fn schedule(self: &Rc<Self>) {
        if self.queued.replace(true) {
            return;
        }
        let weak = Rc::downgrade(self);
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(host) = weak.upgrade() {
                    host.queued.set(false);
                    if let Err(e) = host.reconcile() {
                        host.error.replace(Some(e));
                        host.cancel.cancel();
                    }
                }
            },
            DispatcherPriority::NORMAL,
        );
    }
}

pub fn run(
    app: &Ref<Application>,
    scene: Scene,
    catalog: Rc<ApplicationCatalog>,
    session: Rc<Session>,
    themes: Rc<ThemeController>,
) -> Result<(), String> {
    let shell = create(scene, catalog, Some(session), themes, true);
    let host = Rc::new(Host {
        shell: shell.clone(),
        clock: clock(),
        outputs: Default::default(),
        widths: Default::default(),
        overlay: Default::default(),
        output_dirty: Cell::new(true),
        queued: Cell::new(false),
        cancel: CancellationTokenSource::new(),
        error: Default::default(),
    });
    let impl_ = FerroLocator::current()
        .get_service::<dyn IScreenImpl>()
        .ok_or("Wayland screens unavailable")?;
    let screens = Screens::new(impl_);
    let weak = Rc::downgrade(&host);
    let changed = screens.changed(move || {
        if let Some(host) = weak.upgrade() {
            host.output_dirty.set(true);
            host.schedule();
        }
    });
    let weak = Rc::downgrade(&host);
    effect(move || {
        shell.appearance.get();
        shell.scene.get();
        if let Some(host) = weak.upgrade() {
            host.schedule();
        }
    });
    host.reconcile()?;
    app.run_with_token(&host.cancel.token());
    changed.dispose();
    let error = host.error.take();
    error.map_or(Ok(()), Err)
}
