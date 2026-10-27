//! A portable prepared-text flow example. The deterministic monospace measurer
//! illustrates geometry; applications use the native text adapter for typography.

use std::error::Error;
use std::rc::Rc;

use ferroui_pretext::{
    DelegateTextMeasurerFactory, PrepareOptions, PretextLayout, RichInlineBreakMode, RichInlineItem,
};
use ferroui_pretext_layout::{CircleObstacle, ColumnFlowLayout, PreparedTextMetrics, RectObstacle};

fn main() -> Result<(), Box<dyn Error>> {
    let engine = PretextLayout::new(Rc::new(DelegateTextMeasurerFactory::new(
        |text: &str, _: &str| text.chars().count() as f64 * 8.0,
    )));
    let prepared = engine.prepare_with_segments(
        "Prepare this paragraph once. Reflow it through columns as the exclusion \
         shape moves. Width changes reuse the prepared measurements, and only \
         the visible line ranges need their text materialized. This is useful \
         for editorial pages, canvas labels and virtualized cards.",
        "16px Monospace",
        PrepareOptions::default(),
    )?;
    let columns = [
        RectObstacle {
            x: 0.0,
            y: 0.0,
            width: 240.0,
            height: 220.0,
        },
        RectObstacle {
            x: 264.0,
            y: 0.0,
            width: 240.0,
            height: 220.0,
        },
    ];
    let rectangles = [RectObstacle {
        x: 0.0,
        y: 0.0,
        width: 80.0,
        height: 40.0,
    }];

    println!("Illustrative monospace geometry (8 units per Unicode scalar)");
    println!("Prepared segments: {}", prepared.segments().len());
    for width in [120.0, 180.0, 240.0] {
        let metrics = PreparedTextMetrics::collect_wrap_metrics(&prepared, width, 20.0);
        println!(
            "Width {width:>3.0}: {} lines, height {:.0}, widest {:.0}",
            metrics.line_count, metrics.height, metrics.max_line_width,
        );
    }

    for circle_y in [70.0, 150.0] {
        let circles = [CircleObstacle {
            x: 150.0,
            y: circle_y,
            radius: 45.0,
        }];
        let lines = ColumnFlowLayout::layout_into_columns(
            &prepared,
            &columns,
            &rectangles,
            &circles,
            20.0,
            48.0,
        )?;
        println!(
            "\nCircle centered at (150, {circle_y:.0}); {} positioned lines:",
            lines.len()
        );
        for line in lines {
            println!(
                "  x={:>3.0} y={:>3.0} w={:>3.0}  {}",
                line.x, line.y, line.width, line.text
            );
        }
    }

    let inline = engine.prepare_rich_inline(&[
        RichInlineItem::new("Assigned to ", "16px Monospace"),
        RichInlineItem::new("@maya", "16px Monospace")
            .with_break_mode(RichInlineBreakMode::Never)
            .with_extra_width(16.0),
        RichInlineItem::new(" for the next editorial review.", "16px Monospace"),
    ])?;
    println!("\nRich inline flow with an atomic badge:");
    PretextLayout::walk_rich_inline_line_ranges(&inline, 160.0, |range| {
        let line = PretextLayout::materialize_rich_inline_line_range(&inline, &range);
        print!("  width={:>3.0}", line.width);
        for fragment in line.fragments {
            print!(
                "  [item {} gap {:.0}: {}]",
                fragment.item_index, fragment.gap_before, fragment.text
            );
        }
        println!();
    });
    Ok(())
}
