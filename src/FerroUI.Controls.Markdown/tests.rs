//! The document the control builds, and what it does with hostile text.

use crate::builder::{is_safe_link, MAX_INLINE_DEPTH, MAX_TABLE_COLUMNS, MAX_TABLE_ROWS};
use crate::{register_types, MarkdownTextBlock};
use ferroui_base::Ref;
use ferroui_controls::documents::{Hyperlink, Inline, InlineUIContainer, Paragraph, Span};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::Grid;
use std::cell::Cell;
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

fn control(markdown: &str) -> Ref<MarkdownTextBlock> {
    let target = MarkdownTextBlock::new();
    target.set_markdown(Some(markdown));
    target
}

fn paragraphs(target: &MarkdownTextBlock) -> Vec<Ref<Paragraph>> {
    target.blocks().snapshot().iter().filter_map(|block| block.cast::<Paragraph>()).collect()
}

fn text(paragraph: &Paragraph) -> String {
    paragraph.inlines().text().unwrap_or_default()
}

/// The inlines of `inlines` and of the spans in them, without recursion.
fn all_inlines(paragraph: &Paragraph) -> Vec<Ref<Inline>> {
    let mut found = Vec::new();
    let mut pending: Vec<Ref<Inline>> = paragraph.inlines().snapshot().iter().cloned().collect();
    while let Some(inline) = pending.pop() {
        if let Some(span) = inline.cast::<Span>() {
            pending.extend(span.inlines().snapshot().iter().cloned());
        }
        found.push(inline);
    }
    found
}

fn span_depth(paragraph: &Paragraph) -> usize {
    let mut depth = 0;
    let mut level: Vec<Ref<Inline>> = paragraph.inlines().snapshot().iter().cloned().collect();
    loop {
        let next: Vec<Ref<Inline>> = level
            .iter()
            .filter_map(|inline| inline.cast::<Span>())
            .flat_map(|span| span.inlines().snapshot().iter().cloned().collect::<Vec<_>>())
            .collect();
        if level.iter().all(|inline| inline.cast::<Span>().is_none()) {
            return depth;
        }
        depth += 1;
        level = next;
    }
}

#[test]
fn paragraphs_headings_and_emphasis_become_rich_text() {
    let _application = start();
    let target = control("# Title\n\nSome *emphasis* and **strong** text.\n\nSecond ~~gone~~ one.");
    let blocks = paragraphs(&target);

    assert_eq!(3, blocks.len());
    assert_eq!("Title", text(&blocks[0]));
    assert_eq!(28.0, blocks[0].font_size());
    assert_eq!("Some emphasis and strong text.", text(&blocks[1]));
    assert_eq!("Second gone one.", text(&blocks[2]));
}

#[test]
fn lists_number_their_items_and_indent_by_depth() {
    let _application = start();
    let target = control("3. three\n4. four\n   - inner\n\nafter");
    let blocks = paragraphs(&target);
    let lines: Vec<String> = blocks.iter().map(|block| text(block)).collect();

    assert_eq!(vec!["3. three", "4. four", "\u{2022} inner", "after"], lines);
    assert!(blocks[2].left_indent() > blocks[1].left_indent());
    assert_eq!(0.0, blocks[3].left_indent());
}

#[test]
fn task_lists_show_their_state() {
    let _application = start();
    let target = control("- [x] done\n- [ ] open");
    let lines: Vec<String> = paragraphs(&target).iter().map(|block| text(block)).collect();

    assert_eq!(vec!["\u{2022} \u{2611} done", "\u{2022} \u{2610} open"], lines);
}

#[test]
fn a_code_block_keeps_its_lines() {
    let _application = start();
    let target = control("```rust\nfn a() {}\n\n  b\n```");
    let blocks = paragraphs(&target);

    assert_eq!(1, blocks.len());
    assert_eq!("fn a() {}\n\n  b", text(&blocks[0]).replace("\r\n", "\n"));
}

#[test]
fn a_table_is_a_grid_with_its_cells() {
    let _application = start();
    let target = control("| a | b |\n|---|--:|\n| 1 | 2 |\n| 3 | 4 |");
    let blocks = paragraphs(&target);

    assert_eq!(1, blocks.len());
    let container = all_inlines(&blocks[0]).into_iter().find_map(|inline| inline.cast::<InlineUIContainer>());
    let grid = container.and_then(|container| container.child()).and_then(|child| child.cast::<Grid>());
    let grid = grid.expect("the table is a grid");
    assert_eq!(2, grid.column_definitions().count());
    assert_eq!(3, grid.row_definitions().count());
    assert_eq!(6, grid.children().count());
}

#[test]
fn a_table_past_its_limits_is_cut_and_says_so() {
    let _application = start();
    let columns = MAX_TABLE_COLUMNS + 10;
    let head = format!("|{}\n|{}\n", "a|".repeat(columns), "-|".repeat(columns));
    let rows = format!("|{}\n", "1|".repeat(columns)).repeat(MAX_TABLE_ROWS + 5);
    let target = control(&(head + &rows));
    let blocks = paragraphs(&target);

    let grid = all_inlines(&blocks[0])
        .into_iter()
        .find_map(|inline| inline.cast::<InlineUIContainer>())
        .and_then(|container| container.child())
        .and_then(|child| child.cast::<Grid>())
        .expect("the table is a grid");
    assert_eq!(MAX_TABLE_COLUMNS, grid.column_definitions().count());
    assert_eq!(MAX_TABLE_ROWS, grid.row_definitions().count());
    // The head is one of the rows kept, so six rows are left out.
    assert_eq!("6 more rows", text(&blocks[1]));
}

#[test]
fn only_safe_addresses_open() {
    for address in ["https://example.com", "HTTP://example.com", "mailto:a@example.com", "page.md", "#part", "a/b:c"] {
        assert!(is_safe_link(address), "{address}");
    }
    for address in ["javascript:alert(1)", "file:///etc/passwd", " JavaScript:x", "ms-settings:", "data:text/html,x"] {
        assert!(!is_safe_link(address), "{address}");
    }
}

#[test]
fn a_link_with_an_unsafe_address_is_text() {
    let _application = start();
    let target = control("[safe](https://example.com) and [bad](javascript:alert(1))");
    let blocks = paragraphs(&target);
    let links: Vec<Ref<Hyperlink>> =
        all_inlines(&blocks[0]).into_iter().filter_map(|inline| inline.cast::<Hyperlink>()).collect();

    assert_eq!(1, links.len());
    assert_eq!(Some("https://example.com".to_string()), links[0].navigate_uri());
    assert_eq!("safe and bad", text(&blocks[0]));
}

#[test]
fn a_handled_link_click_reaches_the_control() {
    let _application = start();
    let target = control("[safe](https://example.com)");
    let clicked = Rc::new(Cell::new(0));
    let count = clicked.clone();
    let _subscription = target.link_clicked(move |args| {
        count.set(count.get() + 1);
        args.set_handled(true);
    });
    let link = all_inlines(&paragraphs(&target)[0])
        .into_iter()
        .find_map(|inline| inline.cast::<Hyperlink>())
        .expect("the link");

    assert!(link.navigate());
    assert_eq!(1, clicked.get());
}

#[test]
fn html_is_shown_as_text() {
    let _application = start();
    let target = control("before <script>alert(1)</script> after");

    assert_eq!("before <script>alert(1)</script> after", text(&paragraphs(&target)[0]));
}

#[test]
fn an_image_without_a_resolver_is_its_alternative_text() {
    let _application = start();
    let target = control("![a *fine* picture](https://example.com/a.png)");
    let blocks = paragraphs(&target);

    assert_eq!("a fine picture", text(&blocks[0]));
    assert!(all_inlines(&blocks[0]).iter().all(|inline| inline.cast::<InlineUIContainer>().is_none()));
}

#[test]
fn the_resolver_is_asked_for_the_address_of_an_image() {
    let _application = start();
    let asked = Rc::new(std::cell::RefCell::new(Vec::new()));
    let seen = asked.clone();
    let target = MarkdownTextBlock::new();
    target.set_image_resolver(Some(Rc::new(move |address: &str| {
        seen.borrow_mut().push(address.to_string());
        None
    })));
    target.set_markdown(Some("![alt](pictures/a.png)"));

    assert_eq!(vec!["pictures/a.png".to_string()], *asked.borrow());
    assert_eq!("alt", text(&paragraphs(&target)[0]));
}

#[test]
fn deeply_nested_input_is_bounded() {
    let _application = start();
    // Quotes, lists and emphasis nested far past anything a person writes.
    let quotes = ">".repeat(20_000) + " deep";
    let lists: String = (0..2_000).map(|depth| " ".repeat(depth * 2) + "- x\n").collect();
    let emphasis = "*a **b ".repeat(5_000);
    let links = "[".repeat(20_000) + "x" + &"](u)".repeat(20_000);

    for markdown in [quotes, lists, emphasis, links] {
        let target = control(&markdown);
        for block in paragraphs(&target) {
            assert!(span_depth(&block) <= MAX_INLINE_DEPTH);
        }
    }
}

#[test]
fn a_new_text_replaces_the_document() {
    let _application = start();
    let target = control("one\n\ntwo");
    assert_eq!(2, paragraphs(&target).len());

    target.set_markdown(Some("three"));
    assert_eq!(1, paragraphs(&target).len());

    target.set_markdown(None);
    assert!(paragraphs(&target).is_empty());
}
