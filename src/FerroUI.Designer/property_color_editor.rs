//! Solid-color authoring reuses the native spectrum, palette and component controls.
//! Popup adjustments are a draft; only Apply (or Enter in the source field) commits.
use ferroui_base::input::{InputElement, Key};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::{Color, FontWeight, SolidColorBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::IStyle;
use ferroui_base::utilities::Uri;
use ferroui_base::{CornerRadius, Ref, Thickness, WeakRef};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Border, Button, ColumnDefinitions, Control, Flyout, Grid, RowDefinitions, StackPanel,
    TextBlock, TextBox, ToolTip,
};
use ferroui_controls_color_picker::{AlphaComponentPosition, ColorView};
use ferroui_markup_xaml::FerroXamlLoader;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type ColorCommit = Rc<dyn Fn(String)>;

struct ColorEditorState {
    text: WeakRef<TextBox>,
    swatch: WeakRef<Border>,
    button: WeakRef<Button>,
    original: RefCell<Option<String>>,
    mixed: Cell<bool>,
    commit: RefCell<Option<ColorCommit>>,
}

impl ColorEditorState {
    fn update_swatch(&self) {
        let Some(text) = self.text.upgrade() else {
            return;
        };
        let value = text.text().unwrap_or_default();
        let color = Color::parse(value.trim()).ok();
        if let Some(swatch) = self.swatch.upgrade() {
            swatch.set_background(
                color.map(|color| SolidColorBrush::from_uint32(color.to_uint32()).into()),
            );
            swatch
                .classes()
                .set("mixed", self.mixed.get() && value.is_empty());
        }
        if let Some(button) = self.button.upgrade() {
            // A binding, gradient or incomplete value remains source. Opening a solid-color
            // picker must never implicitly flatten a richer brush into its first stop.
            button.set_is_enabled(value.trim().is_empty() || color.is_some());
        }
    }

    fn restore(&self) {
        if let Some(text) = self.text.upgrade() {
            text.set_text(self.original.borrow().as_deref());
        }
        self.update_swatch();
    }

    fn request(&self, value: String) {
        if self.mixed.get() || self.original.borrow().as_ref() != Some(&value) {
            let callback = self.commit.borrow().clone();
            if let Some(callback) = callback {
                callback(value);
            }
        }
    }

    fn request_color(&self, color: Color) {
        // Preserve the authored spelling of unchanged named/hex colors. A mixed selection
        // still requires one explicit value even when it happens to equal the picker seed.
        if !self.mixed.get()
            && self
                .original
                .borrow()
                .as_deref()
                .and_then(|text| Color::parse(text).ok())
                == Some(color)
        {
            return;
        }
        let value = format!("#{:08X}", color.to_uint32());
        if let Some(text) = self.text.upgrade() {
            text.set_text(Some(&value));
        }
        self.request(value);
    }
}

/// A native swatch and XAML color field with a staged spectrum/palette flyout.
///
/// `configure` never authors a value. Changes in the native [`ColorView`] remain local
/// until Apply, so a spectrum drag produces one document transaction and one undo entry.
/// Escape and light dismissal discard the popup draft. The owner validates every commit.
pub struct PropertyColorEditor {
    root: Ref<Grid>,
    state: Rc<ColorEditorState>,
    text: Ref<TextBox>,
    button: Ref<Button>,
    view: Ref<ColorView>,
    flyout: Ref<Flyout>,
}

impl PropertyColorEditor {
    pub fn new() -> Result<Self, String> {
        ferroui_controls_color_picker::register_types();
        let root = Grid::new();
        root.classes().add("property-color-editor");
        root.set_column_definitions(ColumnDefinitions::parse("30,*").unwrap());
        let button = Button::new();
        button.classes().add("property-color-swatch");
        button.set_width(27.0);
        button.set_height(26.0);
        button.set_min_width(0.0);
        button.set_min_height(26.0);
        button.set_padding(Thickness::uniform(3.0));
        button.set_horizontal_alignment(HorizontalAlignment::Left);
        AutomationProperties::set_name(&button, Some("Choose a solid color"));
        ToolTip::set_tip(&button, Some(Rc::new("Choose a solid color".to_owned())));
        // A native checker beneath the color keeps transparent/alpha values legible.
        let checker = Grid::new();
        checker.set_width(18.0);
        checker.set_height(18.0);
        checker.set_column_definitions(ColumnDefinitions::parse("*,*,*,*").unwrap());
        checker.set_row_definitions(RowDefinitions::parse("*,*,*,*").unwrap());
        for row in 0..4 {
            for column in 0..4 {
                let square = Border::new();
                square.set_background(Some(
                    SolidColorBrush::from_uint32(if (row + column) % 2 == 0 {
                        0xffbfc3cd
                    } else {
                        0xff777e8e
                    })
                    .into(),
                ));
                Grid::set_row(&square, row);
                Grid::set_column(&square, column);
                checker.children().add(square.upcast::<Control>());
            }
        }
        let swatch = Border::new();
        swatch.set_border_thickness(Thickness::uniform(1.0));
        swatch.set_border_brush(Some(SolidColorBrush::from_uint32(0x40777e8e).into()));
        swatch.set_corner_radius(CornerRadius::uniform(2.0));
        Grid::set_column_span(&swatch, 4);
        Grid::set_row_span(&swatch, 4);
        checker.children().add(swatch.clone().upcast::<Control>());
        button.set_content(Some(Control::boxed(&checker)));
        root.children().add(button.clone().upcast::<Control>());
        let text = TextBox::new();
        text.classes().add("designer-property-field");
        text.classes().add("property-color-code");
        text.set_min_height(26.0);
        text.set_font_size(11.0);
        text.set_placeholder_text(Some("Inherited"));
        text.set_padding(Thickness::new(7.0, 3.0, 5.0, 3.0));
        Grid::set_column(&text, 1);
        AutomationProperties::set_name(&text, Some("XAML color value"));
        ToolTip::set_tip(
            &text,
            Some(Rc::new(
                "Named color or #AARRGGBB · Enter applies · Escape restores".to_owned(),
            )),
        );
        root.children().add(text.clone().upcast::<Control>());
        let state = Rc::new(ColorEditorState {
            text: text.downgrade(),
            swatch: swatch.downgrade(),
            button: button.downgrade(),
            original: RefCell::new(None),
            mixed: Cell::new(false),
            commit: RefCell::new(None),
        });
        let draft = state.clone();
        text.text_changing(move |_, _| draft.update_swatch());
        let draft = state.clone();
        text.add_handler(InputElement::key_down_event(), move |_, event| {
            match event.key {
                Key::Enter => {
                    if let Some(text) = draft.text.upgrade() {
                        draft.request(text.text().unwrap_or_default());
                    }
                }
                Key::Escape => draft.restore(),
                _ => return,
            }
            event.set_handled(true);
        });

        let body = StackPanel::new();
        body.classes().add("designer-chrome");
        body.classes().add("designer-color-popup");
        body.set_spacing(8.0);
        body.set_width(350.0);
        // Library control themes are scoped to this popup, leaving authored colors and
        // ColorView controls on the artboard under their application's own theme.
        let uri = Uri::absolute("ferres://FerroUI.Controls.ColorPicker/Themes/Fluent/Fluent.xaml")
            .map_err(|error| error.to_string())?;
        let loaded = FerroXamlLoader::load(&uri, None).map_err(|error| error.to_string())?;
        let style = from_markup_value::<Rc<dyn IStyle>>(&Some(loaded))
            .ok_or("The native color editor theme could not be loaded")?;
        body.styles().add(style);
        let title = TextBlock::new();
        title.set_text(Some("Solid color"));
        title.set_font_size(12.0);
        title.set_font_weight(FontWeight::SemiBold);
        body.children().add(title.upcast::<Control>());
        let view = ColorView::new();
        view.set_hex_input_alpha_position(AlphaComponentPosition::Leading);
        view.set_is_accent_colors_visible(false);
        body.children().add(view.clone().upcast::<Control>());
        let actions = StackPanel::new();
        actions.set_orientation(Orientation::Horizontal);
        actions.set_horizontal_alignment(HorizontalAlignment::Right);
        actions.set_spacing(8.0);
        let cancel = Button::new();
        cancel.set_content(Some(Rc::new("Cancel".to_owned())));
        let apply = Button::new();
        apply.classes().add("accent");
        apply.set_content(Some(Rc::new("Apply color".to_owned())));
        actions.children().add(cancel.clone().upcast::<Control>());
        actions.children().add(apply.clone().upcast::<Control>());
        body.children().add(actions.upcast::<Control>());
        let flyout = Flyout::new();
        flyout.flyout_presenter_classes().add("designer-chrome");
        flyout
            .flyout_presenter_classes()
            .add("designer-color-flyout");
        flyout.set_content(Some(Control::boxed(&body)));
        button.set_flyout(Some(flyout.clone().upcast()));
        let seed = state.clone();
        let weak_view = view.downgrade();
        flyout.opening(move |event| {
            let current = seed
                .text
                .upgrade()
                .and_then(|text| text.text())
                .unwrap_or_default();
            let color = if current.trim().is_empty() {
                Some(Color::from_argb(255, 255, 255, 255))
            } else {
                Color::parse(current.trim()).ok()
            };
            if let Some((view, color)) = weak_view.upgrade().zip(color) {
                view.set_color(color);
            } else {
                event.set_cancel(true);
            }
        });
        let weak_flyout = flyout.downgrade();
        cancel.click(move |_, _| {
            if let Some(flyout) = weak_flyout.upgrade() {
                flyout.hide();
            }
        });
        let weak_flyout = flyout.downgrade();
        let weak_view = view.downgrade();
        let accepted = state.clone();
        apply.click(move |_, _| {
            if let Some((flyout, view)) = weak_flyout.upgrade().zip(weak_view.upgrade()) {
                let color = view.color();
                flyout.hide();
                accepted.request_color(color);
            }
        });
        Ok(Self {
            root,
            state,
            text,
            button,
            view,
            flyout,
        })
    }

    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast()
    }
    pub fn text_editor(&self) -> Ref<TextBox> {
        self.text.clone()
    }
    pub fn swatch_button(&self) -> Ref<Button> {
        self.button.clone()
    }
    pub fn color_view(&self) -> Ref<ColorView> {
        self.view.clone()
    }
    pub fn flyout(&self) -> Ref<Flyout> {
        self.flyout.clone()
    }

    pub fn configure(&self, value: Option<&str>, mixed: bool) -> Result<(), String> {
        if value.is_some_and(|value| Color::parse(value.trim()).is_err()) {
            return Err("Use the source editor for an expression or complex brush".into());
        }
        self.flyout.hide();
        *self.state.original.borrow_mut() = value.map(str::to_owned);
        self.state.mixed.set(mixed);
        self.text
            .set_placeholder_text(Some(if mixed { "Mixed" } else { "Inherited" }));
        self.state.restore();
        Ok(())
    }

    pub fn on_commit(&self, callback: impl Fn(String) + 'static) {
        *self.state.commit.borrow_mut() = Some(Rc::new(callback));
    }
}

#[cfg(test)]
#[path = "property_color_editor_tests.rs"]
mod tests;
