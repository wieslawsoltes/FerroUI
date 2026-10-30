// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    data::core::ValueTypes,
    input::ICommand,
    reactive::{Disposable, IDisposable},
    BoxedValue, Ref,
};
use ferroui_controls::ScrollViewer;
use std::{cell::Cell, rc::Rc};
/// Original supported line scroll directions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollViewerLineDirection {
    /// Scroll left by one line.
    #[default]
    Left = 0,
    /// Scroll right by one line.
    Right = 1,
    /// Scroll up by one line.
    Up = 2,
    /// Scroll down by one line.
    Down = 3,
}
ferroui_base::ferro_markup_enum!(ScrollViewerLineDirection {
    Left,
    Right,
    Up,
    Down
});
/// Original command that scrolls the supplied native viewer by one line.
pub struct ScrollViewerLineCommand {
    direction: Cell<ScrollViewerLineDirection>,
}
impl PartialEq for ScrollViewerLineCommand {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ScrollViewerLineCommand {
    /// Creates a command with the source default Left direction.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            direction: Cell::new(ScrollViewerLineDirection::Left),
        })
    }
    /// Gets Direction.
    pub fn direction(&self) -> ScrollViewerLineDirection {
        self.direction.get()
    }
    /// Sets Direction without raising the source no-op event.
    pub fn set_direction(&self, value: ScrollViewerLineDirection) {
        self.direction.set(value);
    }
    fn viewer(parameter: Option<&BoxedValue>) -> Option<Ref<ScrollViewer>> {
        let value = parameter?;
        if let Some(viewer) = value.downcast_ref::<Ref<ScrollViewer>>() {
            return Some(viewer.clone());
        }
        ValueTypes::as_object(&**value)?.cast::<ScrollViewer>()
    }
}
impl ICommand for ScrollViewerLineCommand {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        Self::viewer(parameter).is_some()
    }
    fn execute(&self, parameter: Option<&BoxedValue>) {
        let Some(viewer) = Self::viewer(parameter) else {
            return;
        };
        match self.direction() {
            ScrollViewerLineDirection::Left => viewer.line_left(),
            ScrollViewerLineDirection::Right => viewer.line_right(),
            ScrollViewerLineDirection::Up => viewer.line_up(),
            ScrollViewerLineDirection::Down => viewer.line_down(),
        }
    }
    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
ferroui_base::ferro_markup_type!(class ScrollViewerLineCommand {
    namespace:"FerroDock.Ferro.Commands",this:Rc<ScrollViewerLineCommand>,handles:[Rc<ScrollViewerLineCommand>,Option<Rc<ScrollViewerLineCommand>>],interfaces:[Rc<dyn ICommand>],constructors:[()=>ScrollViewerLineCommand::new],
    properties:[Direction:ScrollViewerLineDirection{get:|this:&Rc<ScrollViewerLineCommand>|this.direction(),set:|this:&Rc<ScrollViewerLineCommand>,value|this.set_direction(value)}],
});

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::{threading::CancellationToken, Vector};
    use ferroui_controls::{
        primitives::ScrollBarVisibility, AppBuilder, Application, Border, Control, Window,
    };
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
        FerroTestApplicationAttribute, FerroTestAssembly, HeadlessUnitTestSession,
    };
    fn build() -> AppBuilder {
        AppBuilder::configure::<Application>()
            .use_headless(FerroHeadlessPlatformOptions::default())
            .after_setup(|_| {
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_fluent::FluentTheme::new().as_style())
            })
    }
    const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
        name: "source-scroll-command",
        test_application: Some(FerroTestApplicationAttribute::new(build)),
        test_isolation: None,
    };
    #[test]
    fn original_command_moves_actual_viewer_in_all_directions_and_ignores_nonviewers_and_events() {
        HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
            .dispatch(
                || {
                    crate::register_types();
                    let viewer = ScrollViewer::new();
                    viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Visible);
                    viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Visible);
                    let content = Border::new();
                    content.set_width(1000.0);
                    content.set_height(1000.0);
                    viewer.set_content(Some(Rc::new(content)));
                    let window = Window::new();
                    window.set_width(200.0);
                    window.set_height(200.0);
                    window.set_content(Some(Rc::new(viewer.clone())));
                    window.show();
                    window.update_layout();
                    assert!(
                        viewer.extent().width > viewer.viewport().width
                            && viewer.extent().height > viewer.viewport().height
                    );
                    let command = ScrollViewerLineCommand::new();
                    assert_eq!(command.direction(), ScrollViewerLineDirection::Left);
                    let subscription = command
                        .can_execute_changed(Rc::new(|| panic!("source event never raises")));
                    for (direction, x_sign, y_sign) in [
                        (ScrollViewerLineDirection::Left, -1.0, 0.0),
                        (ScrollViewerLineDirection::Right, 1.0, 0.0),
                        (ScrollViewerLineDirection::Up, 0.0, -1.0),
                        (ScrollViewerLineDirection::Down, 0.0, 1.0),
                    ] {
                        viewer.set_offset(Vector::new(100.0, 100.0));
                        command.set_direction(direction);
                        // A base-typed native handle must recover the actual receiver too.
                        let parameter: BoxedValue = Rc::new(viewer.clone().upcast::<Control>());
                        assert!(command.can_execute(Some(&parameter)));
                        command.execute(Some(&parameter));
                        let offset = viewer.offset();
                        if x_sign == 0.0 {
                            assert_eq!(offset.x, 100.0);
                        } else {
                            assert_eq!((offset.x - 100.0).signum(), x_sign);
                        }
                        if y_sign == 0.0 {
                            assert_eq!(offset.y, 100.0);
                        } else {
                            assert_eq!((offset.y - 100.0).signum(), y_sign);
                        }
                    }
                    let wrong: BoxedValue = Rc::new(true);
                    assert!(!command.can_execute(Some(&wrong)));
                    assert!(!command.can_execute(None));
                    let offset = viewer.offset();
                    command.execute(Some(&wrong));
                    command.execute(None);
                    assert_eq!(viewer.offset(), offset);
                    subscription.dispose();
                    window.close();
                },
                CancellationToken::none(),
            )
            .wait()
            .unwrap();
    }
}
