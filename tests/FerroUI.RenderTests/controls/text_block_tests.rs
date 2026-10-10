//! Port of upstream's `Controls/TextBlockTests.cs`.
//!
//! Every test of the class runs only on Windows upstream (`Win32Fact`,
//! `Win32Theory`): the expected images are the text of that platform, and
//! most of the tests name a font of it. The bodies are complete and the
//! tests are ignored elsewhere, with upstream's message.

use crate::test_base::TestBase;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{
    Brushes, Colors, FontFamily, FontStyle, SolidColorBrush, TextAlignment, TextDecoration, TextDecorationCollection,
    TextDecorationLocation, TextDecorationUnit, TextHintingMode, TextOptions, TextRenderingMode, TextWrapping,
};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::documents::{InlineCollection, Run};
use ferroui_controls::{Border, Control, Decorator, StackPanel, TextBlock};

fn base() -> TestBase {
    TestBase::new(r"Controls\TextBlock")
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn should_draw_text_decorations() {
    let t = base();
    let target = Border::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(30.0);
    target.set_background(Some(Brushes::white()));
    let child = TextBlock::new();
    child.set_font_family(crate::test_base::test_font_family());
    child.set_font_size(12.0);
    child.set_foreground(Some(Brushes::black()));
    child.set_text(Some("Neque porro quisquam est qui dolorem"));
    child.set_vertical_alignment(VerticalAlignment::Top);
    child.set_text_wrapping(TextWrapping::NoWrap);
    let overline = TextDecoration::new();
    overline.set_location(TextDecorationLocation::Overline);
    overline.set_stroke_thickness(1.5);
    overline.set_stroke_thickness_unit(TextDecorationUnit::Pixel);
    overline.set_stroke(Some(SolidColorBrush::with_color(Colors::RED).into()));
    let baseline = TextDecoration::new();
    baseline.set_location(TextDecorationLocation::Baseline);
    baseline.set_stroke_thickness(1.5);
    baseline.set_stroke_thickness_unit(TextDecorationUnit::Pixel);
    baseline.set_stroke(Some(SolidColorBrush::with_color(Colors::GREEN).into()));
    let underline = TextDecoration::new();
    underline.set_location(TextDecorationLocation::Underline);
    underline.set_stroke_thickness(1.5);
    underline.set_stroke_thickness_unit(TextDecorationUnit::Pixel);
    underline.set_stroke(Some(SolidColorBrush::with_color(Colors::BLUE).into()));
    underline.set_stroke_offset(2.0);
    underline.set_stroke_offset_unit(TextDecorationUnit::Pixel);
    child.set_text_decorations(Some(TextDecorationCollection::from_items([overline, baseline, underline])));
    target.set_child(child);

    t.render_to_file(&target, "Should_Draw_TextDecorations");
    t.compare_images("Should_Draw_TextDecorations");
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn wrapping_no_wrap() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = TextBlock::new();
    child.set_font_family(FontFamily::new("Courier New"));
    child.set_background(Some(Brushes::red()));
    child.set_font_size(12.0);
    child.set_foreground(Some(Brushes::black()));
    child.set_text(Some("Neque porro quisquam est qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit"));
    child.set_vertical_alignment(VerticalAlignment::Top);
    child.set_text_wrapping(TextWrapping::NoWrap);
    target.set_child(child);

    t.render_to_file(&target, "Wrapping_NoWrap");
    t.compare_images("Wrapping_NoWrap");
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn restricted_height_vertical_align() {
    let t = base();

    // Upstream's local function has the defaults `clip = true` and
    // `restrictHeight = true`.
    fn text(vertical_alignment: VerticalAlignment, clip: bool, restrict_height: bool) -> Ref<Control> {
        let border = Border::new();
        border.set_border_brush(Some(Brushes::blue()));
        border.set_border_thickness(Thickness::uniform(1.0));
        border.set_vertical_alignment(VerticalAlignment::Center);
        border.set_horizontal_alignment(HorizontalAlignment::Center);
        border.set_height(if restrict_height { 20.0 } else { f64::NAN });
        border.set_margin(Thickness::uniform(1.0));
        let child = TextBlock::new();
        child.set_font_family(FontFamily::new("Courier New"));
        child.set_background(Some(Brushes::red()));
        child.set_font_size(24.0);
        child.set_foreground(Some(Brushes::black()));
        child.set_text(Some("L"));
        child.set_vertical_alignment(vertical_alignment);
        child.set_clip_to_bounds(clip);
        border.set_child(child);
        border.upcast()
    }

    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(190.0);
    target.set_height(80.0);

    let child = StackPanel::new();
    child.set_orientation(Orientation::Horizontal);
    child.children().add(text(VerticalAlignment::Stretch, true, false));
    child.children().add(text(VerticalAlignment::Center, true, true));
    child.children().add(text(VerticalAlignment::Stretch, true, true));
    child.children().add(text(VerticalAlignment::Top, true, true));
    child.children().add(text(VerticalAlignment::Bottom, true, true));
    child.children().add(text(VerticalAlignment::Center, false, true));
    child.children().add(text(VerticalAlignment::Stretch, false, true));
    child.children().add(text(VerticalAlignment::Top, false, true));
    child.children().add(text(VerticalAlignment::Bottom, false, true));
    target.set_child(child);

    t.render_to_file(&target, "RestrictedHeight_VerticalAlign");
    t.compare_images("RestrictedHeight_VerticalAlign");
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn should_draw_run_with_background() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(50.0);
    let child = TextBlock::new();
    child.set_font_family(FontFamily::new("Courier New"));
    child.set_font_size(12.0);
    child.set_foreground(Some(Brushes::black()));
    child.set_vertical_alignment(VerticalAlignment::Top);
    child.set_text_wrapping(TextWrapping::NoWrap);
    let inlines = InlineCollection::new();
    let run = Run::new();
    run.set_text(Some("Neque porro quisquam"));
    run.set_background(Some(Brushes::red()));
    inlines.add(run);
    child.set_inlines(Some(inlines));
    target.set_child(child);

    t.render_to_file(&target, "Should_Draw_Run_With_Background");
    t.compare_images("Should_Draw_Run_With_Background");
}

fn should_measure_arrange_text_block(width: f64, height: f64, text_wrapping: TextWrapping) {
    let t = base();
    let text = "Hello World";

    let target = StackPanel::new();
    target.set_width(200.0);
    target.set_height(height);

    let child = TextBlock::new();
    child.set_text(Some(text));
    child.set_background(Some(Brushes::red()));
    child.set_horizontal_alignment(HorizontalAlignment::Left);
    child.set_text_alignment(TextAlignment::Left);
    child.set_width(width);
    child.set_text_wrapping(text_wrapping);
    target.children().add(child);
    let child = TextBlock::new();
    child.set_text(Some(text));
    child.set_background(Some(Brushes::red()));
    child.set_horizontal_alignment(HorizontalAlignment::Left);
    child.set_text_alignment(TextAlignment::Center);
    child.set_width(width);
    child.set_text_wrapping(text_wrapping);
    target.children().add(child);
    let child = TextBlock::new();
    child.set_text(Some(text));
    child.set_background(Some(Brushes::red()));
    child.set_horizontal_alignment(HorizontalAlignment::Left);
    child.set_text_alignment(TextAlignment::Right);
    child.set_width(width);
    child.set_text_wrapping(text_wrapping);
    target.children().add(child);

    let child = TextBlock::new();
    child.set_text(Some(text));
    child.set_background(Some(Brushes::red()));
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_text_alignment(TextAlignment::Left);
    child.set_width(width);
    child.set_text_wrapping(text_wrapping);
    target.children().add(child);
    let child = TextBlock::new();
    child.set_text(Some(text));
    child.set_background(Some(Brushes::red()));
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_text_alignment(TextAlignment::Center);
    child.set_width(width);
    child.set_text_wrapping(text_wrapping);
    target.children().add(child);
    let child = TextBlock::new();
    child.set_text(Some(text));
    child.set_background(Some(Brushes::red()));
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_text_alignment(TextAlignment::Right);
    child.set_width(width);
    child.set_text_wrapping(text_wrapping);
    target.children().add(child);

    let child = TextBlock::new();
    child.set_text(Some(text));
    child.set_background(Some(Brushes::red()));
    child.set_horizontal_alignment(HorizontalAlignment::Right);
    child.set_text_alignment(TextAlignment::Left);
    child.set_width(width);
    child.set_text_wrapping(text_wrapping);
    target.children().add(child);
    let child = TextBlock::new();
    child.set_text(Some(text));
    child.set_background(Some(Brushes::red()));
    child.set_horizontal_alignment(HorizontalAlignment::Right);
    child.set_text_alignment(TextAlignment::Center);
    child.set_width(width);
    child.set_text_wrapping(text_wrapping);
    target.children().add(child);
    let child = TextBlock::new();
    child.set_text(Some(text));
    child.set_background(Some(Brushes::red()));
    child.set_horizontal_alignment(HorizontalAlignment::Right);
    child.set_text_alignment(TextAlignment::Right);
    child.set_width(width);
    child.set_text_wrapping(text_wrapping);
    target.children().add(child);

    let test_name = format!("Should_Measure_Arrange_TextBlock_{width}_{text_wrapping:?}");

    t.render_to_file(&target, &test_name);
    t.compare_images(&test_name);
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn should_measure_arrange_text_block_150_200_no_wrap() {
    should_measure_arrange_text_block(150.0, 200.0, TextWrapping::NoWrap);
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn should_measure_arrange_text_block_44_200_no_wrap() {
    should_measure_arrange_text_block(44.0, 200.0, TextWrapping::NoWrap);
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn should_measure_arrange_text_block_44_400_wrap() {
    should_measure_arrange_text_block(44.0, 400.0, TextWrapping::Wrap);
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn should_keep_trailing_white_space() {
    let t = base();

    // <StackPanel VerticalAlignment="Center" HorizontalAlignment="Center">
    //   <TextBlock Margin="0 10 0 0" HorizontalAlignment="Center" VerticalAlignment="Center" Background="Magenta" FontSize="44" Text="aaaa" FontFamily="Courier New"/>
    //   <TextBlock Margin="0 10 0 0" HorizontalAlignment="Center" VerticalAlignment="Center" Background="Magenta" FontSize="44" Text="a a " FontFamily="Courier New"/>
    //   <TextBlock Margin="0 10 0 0" HorizontalAlignment="Center" VerticalAlignment="Center" Background="Magenta" FontSize="44" Text="    " FontFamily="Courier New"/>
    //   <TextBlock Margin="0 10 0 0" HorizontalAlignment="Center" VerticalAlignment="Center" Background="Magenta" FontSize="44" Text="LLLL" FontFamily="Courier New"/>
    // </StackPanel>

    fn create_text(text: &str) -> Ref<TextBlock> {
        let result = TextBlock::new();
        result.set_margin(Thickness::new(0.0, 10.0, 0.0, 0.0));
        result.set_horizontal_alignment(HorizontalAlignment::Center);
        result.set_vertical_alignment(VerticalAlignment::Center);
        result.set_background(Some(Brushes::magenta()));
        result.set_font_size(44.0);
        result.set_text(Some(text));
        result.set_font_family(FontFamily::new("Courier New"));
        result
    }

    let target = StackPanel::new();
    target.set_vertical_alignment(VerticalAlignment::Center);
    target.set_horizontal_alignment(HorizontalAlignment::Center);
    target.set_width(300.0);
    target.set_height(300.0);
    target.children().add(create_text("aaaa"));
    target.children().add(create_text("a a "));
    target.children().add(create_text("    "));
    target.children().add(create_text("LLLL"));

    let test_name = "Should_Keep_TrailingWhiteSpace";
    t.render_to_file(&target, test_name);
    t.compare_images(test_name);
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn should_account_for_overhang_leading_and_trailing() {
    let t = base();

    const SYMBOLS_FONT: &str =
        "resm:FerroUI.Skia.RenderTests.Assets?assembly=ferroui-render-tests#Source Serif 4 36pt";
    fn create_text(text: &str) -> Ref<TextBlock> {
        let result = TextBlock::new();
        result.set_clip_to_bounds(false);
        result.set_margin(Thickness::uniform(4.0));
        result.set_horizontal_alignment(HorizontalAlignment::Center);
        result.set_vertical_alignment(VerticalAlignment::Center);
        result.set_background(Some(Brushes::magenta()));
        result.set_font_style(FontStyle::Italic);
        result.set_font_size(44.0);
        result.set_text(Some(text));
        result.set_font_family(FontFamily::new(SYMBOLS_FONT));
        result
    }

    let target = StackPanel::new();
    target.set_vertical_alignment(VerticalAlignment::Center);
    target.set_horizontal_alignment(HorizontalAlignment::Center);
    target.set_width(300.0);
    target.set_height(600.0);
    // Required antialiasing to work for Overhang
    target.set_background(Some(SolidColorBrush::with_color(Colors::WHITE).into()));

    target.children().add(create_text("f"));
    target.children().add(create_text("y"));
    target.children().add(create_text("ff"));
    target.children().add(create_text("yy"));
    target.children().add(create_text("faaf"));
    target.children().add(create_text("yaay"));
    target.children().add(create_text("y y "));
    target.children().add(create_text("f f "));

    let test_name = "Should_Account_For_Overhang_Leading_And_Trailing";
    t.render_to_file(&target, test_name);
    t.compare_images(test_name);
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn should_draw_multi_line_text_with_over_hand_leading_trailing() {
    let t = base();

    const SYMBOLS_FONT: &str =
        "resm:FerroUI.Skia.RenderTests.Assets?assembly=ferroui-render-tests#Source Serif 4 36pt";
    fn create_text(text: &str) -> Ref<TextBlock> {
        let result = TextBlock::new();
        result.set_horizontal_alignment(HorizontalAlignment::Center);
        result.set_vertical_alignment(VerticalAlignment::Center);
        result.set_text_alignment(TextAlignment::Center);
        result.set_background(Some(Brushes::magenta()));
        result.set_font_style(FontStyle::Italic);
        result.set_font_size(44.0);
        result.set_text(Some(text));
        result.set_font_family(FontFamily::new(SYMBOLS_FONT));
        result
    }

    let target = StackPanel::new();
    target.set_vertical_alignment(VerticalAlignment::Center);
    target.set_horizontal_alignment(HorizontalAlignment::Center);
    target.set_width(600.0);
    target.set_height(200.0);
    // Required antialiasing to work for Overhang
    target.set_background(Some(SolidColorBrush::with_color(Colors::WHITE).into()));

    target.children().add(create_text("fff Why this is a\nbig text yyy\nyyy with multiple lines fff"));

    let test_name = "Should_Draw_MultiLineText_WithOverHandLeadingTrailing";
    t.render_to_file(&target, test_name);
    t.compare_images(test_name);
}

fn should_render_text_block_with_text_options(
    text_rendering_mode: TextRenderingMode,
    text_hinting_mode: TextHintingMode,
) {
    let t = base();
    let text_block = TextBlock::new();
    text_block.set_font_family(crate::test_base::test_font_family());
    text_block.set_font_size(24.0);
    text_block.set_foreground(Some(Brushes::black()));
    text_block.set_text(Some("TextOptions"));
    text_block.set_background(Some(Brushes::light_gray()));
    text_block.set_padding(Thickness::uniform(10.0));

    TextOptions::set_text_options(
        &text_block,
        TextOptions { text_rendering_mode, text_hinting_mode, ..TextOptions::default() },
    );

    let target = Border::new();
    target.set_width(300.0);
    target.set_height(100.0);
    target.set_background(Some(Brushes::white()));
    target.set_child(text_block);

    let test_name =
        format!("Should_Render_TextBlock_With_TextOptions_{text_rendering_mode:?}_{text_hinting_mode:?}");
    t.render_to_file(&target, &test_name);
    t.compare_images(&test_name);
}

#[test]
#[cfg_attr(not(windows), ignore = "Depends on the backend")]
fn should_render_text_block_with_text_options_antialias_none() {
    should_render_text_block_with_text_options(TextRenderingMode::Antialias, TextHintingMode::None);
}

#[test]
#[cfg_attr(not(windows), ignore = "Depends on the backend")]
fn should_render_text_block_with_text_options_alias_none() {
    should_render_text_block_with_text_options(TextRenderingMode::Alias, TextHintingMode::None);
}

#[test]
#[cfg_attr(not(windows), ignore = "Depends on the backend")]
fn should_render_text_block_with_text_options_antialias_light() {
    should_render_text_block_with_text_options(TextRenderingMode::Antialias, TextHintingMode::Light);
}

#[test]
#[cfg_attr(not(windows), ignore = "Depends on the backend")]
fn should_render_text_block_with_text_options_alias_light() {
    should_render_text_block_with_text_options(TextRenderingMode::Alias, TextHintingMode::Light);
}
