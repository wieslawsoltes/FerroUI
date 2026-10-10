//! Port of `ViewModels/MainWindowViewModel.cs`.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand;
use mini_mvvm::{delay, MiniCommand, ViewModelBase};
use std::cell::Cell;
use std::rc::{Rc, Weak};
use std::time::Duration;

pub struct MainWindowViewModel {
    base: ViewModelBase,
    draw_dirty_rects: Cell<bool>,
    draw_fps: Cell<bool>,
    draw_layout_time_graph: Cell<bool>,
    draw_render_time_graph: Cell<bool>,
    width: Cell<f64>,
    height: Cell<f64>,
    toggle_draw_dirty_rects: Rc<MiniCommand>,
    toggle_draw_fps: Rc<MiniCommand>,
    toggle_draw_layout_time_graph: Rc<MiniCommand>,
    toggle_draw_render_time_graph: Rc<MiniCommand>,
    resize_window: Rc<MiniCommand>,
}

impl PartialEq for MainWindowViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for MainWindowViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl MainWindowViewModel {
    pub fn new() -> Rc<MainWindowViewModel> {
        Rc::new_cyclic(|this: &Weak<MainWindowViewModel>| {
            let toggle_draw_dirty_rects = {
                let this = this.clone();
                MiniCommand::create(move || {
                    if let Some(this) = this.upgrade() {
                        this.set_draw_dirty_rects(!this.draw_dirty_rects());
                    }
                })
            };
            let toggle_draw_fps = {
                let this = this.clone();
                MiniCommand::create(move || {
                    if let Some(this) = this.upgrade() {
                        this.set_draw_fps(!this.draw_fps());
                    }
                })
            };
            let toggle_draw_layout_time_graph = {
                let this = this.clone();
                MiniCommand::create(move || {
                    if let Some(this) = this.upgrade() {
                        this.set_draw_layout_time_graph(!this.draw_layout_time_graph());
                    }
                })
            };
            let toggle_draw_render_time_graph = {
                let this = this.clone();
                MiniCommand::create(move || {
                    if let Some(this) = this.upgrade() {
                        this.set_draw_render_time_graph(!this.draw_render_time_graph());
                    }
                })
            };
            let resize_window = {
                let this = this.clone();
                MiniCommand::create_from_task(move || {
                    let this = this.upgrade();
                    async move {
                        if let Some(this) = this {
                            this.resize_window_async().await;
                        }
                    }
                })
            };

            Self {
                base: ViewModelBase::new(),
                draw_dirty_rects: Cell::new(false),
                draw_fps: Cell::new(true),
                draw_layout_time_graph: Cell::new(false),
                draw_render_time_graph: Cell::new(false),
                width: Cell::new(800.0),
                height: Cell::new(600.0),
                toggle_draw_dirty_rects,
                toggle_draw_fps,
                toggle_draw_layout_time_graph,
                toggle_draw_render_time_graph,
                resize_window,
            }
        })
    }

    pub fn draw_dirty_rects(&self) -> bool {
        self.draw_dirty_rects.get()
    }

    pub fn set_draw_dirty_rects(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.draw_dirty_rects, value, "DrawDirtyRects");
    }

    pub fn draw_fps(&self) -> bool {
        self.draw_fps.get()
    }

    pub fn set_draw_fps(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.draw_fps, value, "DrawFps");
    }

    pub fn draw_layout_time_graph(&self) -> bool {
        self.draw_layout_time_graph.get()
    }

    pub fn set_draw_layout_time_graph(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.draw_layout_time_graph, value, "DrawLayoutTimeGraph");
    }

    pub fn draw_render_time_graph(&self) -> bool {
        self.draw_render_time_graph.get()
    }

    pub fn set_draw_render_time_graph(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.draw_render_time_graph, value, "DrawRenderTimeGraph");
    }

    pub fn width(&self) -> f64 {
        self.width.get()
    }

    pub fn set_width(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.width, value, "Width");
    }

    pub fn height(&self) -> f64 {
        self.height.get()
    }

    pub fn set_height(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.height, value, "Height");
    }

    pub fn toggle_draw_dirty_rects(&self) -> Rc<MiniCommand> {
        self.toggle_draw_dirty_rects.clone()
    }

    pub fn toggle_draw_fps(&self) -> Rc<MiniCommand> {
        self.toggle_draw_fps.clone()
    }

    pub fn toggle_draw_layout_time_graph(&self) -> Rc<MiniCommand> {
        self.toggle_draw_layout_time_graph.clone()
    }

    pub fn toggle_draw_render_time_graph(&self) -> Rc<MiniCommand> {
        self.toggle_draw_render_time_graph.clone()
    }

    pub fn resize_window(&self) -> Rc<MiniCommand> {
        self.resize_window.clone()
    }

    async fn resize_window_async(&self) {
        for _ in 0..30 {
            self.set_width(self.width() + 10.0);
            self.set_height(self.height() + 5.0);
            delay(Duration::from_millis(10)).await;
        }

        delay(Duration::from_millis(10)).await;

        for _ in 0..30 {
            self.set_width(self.width() - 10.0);
            self.set_height(self.height() - 5.0);
            delay(Duration::from_millis(10)).await;
        }
    }
}

ferro_markup_type!(class MainWindowViewModel {
    this: Rc<MainWindowViewModel>,
    handles: [MainWindowViewModel, Rc<MainWindowViewModel>, Option<Rc<MainWindowViewModel>>],
    constructors: [() => MainWindowViewModel::new],
    properties: [
        DrawDirtyRects: bool {
            get: |this: &Rc<MainWindowViewModel>| this.draw_dirty_rects(),
            set: |this: &Rc<MainWindowViewModel>, value: bool| this.set_draw_dirty_rects(value)
        },
        DrawFps: bool {
            get: |this: &Rc<MainWindowViewModel>| this.draw_fps(),
            set: |this: &Rc<MainWindowViewModel>, value: bool| this.set_draw_fps(value)
        },
        DrawLayoutTimeGraph: bool {
            get: |this: &Rc<MainWindowViewModel>| this.draw_layout_time_graph(),
            set: |this: &Rc<MainWindowViewModel>, value: bool| this.set_draw_layout_time_graph(value)
        },
        DrawRenderTimeGraph: bool {
            get: |this: &Rc<MainWindowViewModel>| this.draw_render_time_graph(),
            set: |this: &Rc<MainWindowViewModel>, value: bool| this.set_draw_render_time_graph(value)
        },
        Width: f64 {
            get: |this: &Rc<MainWindowViewModel>| this.width(),
            set: |this: &Rc<MainWindowViewModel>, value: f64| this.set_width(value)
        },
        Height: f64 {
            get: |this: &Rc<MainWindowViewModel>| this.height(),
            set: |this: &Rc<MainWindowViewModel>, value: f64| this.set_height(value)
        },
        ToggleDrawDirtyRects: Rc<dyn ICommand> {
            get: |this: &Rc<MainWindowViewModel>| this.toggle_draw_dirty_rects().as_command()
        },
        ToggleDrawFps: Rc<dyn ICommand> { get: |this: &Rc<MainWindowViewModel>| this.toggle_draw_fps().as_command() },
        ToggleDrawLayoutTimeGraph: Rc<dyn ICommand> {
            get: |this: &Rc<MainWindowViewModel>| this.toggle_draw_layout_time_graph().as_command()
        },
        ToggleDrawRenderTimeGraph: Rc<dyn ICommand> {
            get: |this: &Rc<MainWindowViewModel>| this.toggle_draw_render_time_graph().as_command()
        },
        ResizeWindow: Rc<dyn ICommand> { get: |this: &Rc<MainWindowViewModel>| this.resize_window().as_command() },
    ],
    notify_property_changed: MainWindowViewModel,
});
