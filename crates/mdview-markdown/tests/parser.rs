//! Parser integration tests.

use mdview_markdown::{Alignment, BlockKind, Inline, LinkKind, QuoteKind, classify, parse};

#[test]
fn headings_follow_the_commonmark_corpus() {
    let source = "# One\n\n## Two ##\n\n### Three {#custom}\n\n# One\n";
    let document = parse(source);
    assert_eq!(document.headings.len(), 4);
    assert_eq!(document.headings[0].anchor, "one");
    assert_eq!(document.headings[1].anchor, "two");
    assert_eq!(document.headings[2].anchor, "custom");
    assert_eq!(document.headings[3].anchor, "one-1");
    assert!(document.headings.iter().all(|heading| heading.span.start_line > 0));
}

#[test]
fn every_block_has_a_span() {
    let source = "# Title\n\nParagraph.\n\n- one\n- two\n\n> quote\n\n```rust\nfn main() {}\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n";
    let document = parse(source);
    assert!(!document.blocks.is_empty());
    for block in &document.blocks {
        assert!(block.span.start <= block.span.end);
        assert!(block.span.start_line <= block.span.end_line);
        assert!(block.span.end_line <= source.lines().count());
    }
}

#[test]
fn tasks_and_footnotes_are_parsed() {
    let source = "- [x] done\n- [ ] open\n\nText[^1]\n\n[^1]: note\n";
    let document = parse(source);
    let BlockKind::List { items, .. } = &document.blocks[0].kind else {
        panic!("expected a list");
    };
    assert_eq!(items[0].task, Some(true));
    assert_eq!(items[1].task, Some(false));
    assert!(matches!(
        document.blocks.last().map(|block| &block.kind),
        Some(BlockKind::FootnoteDefinition { .. })
    ));
}

#[test]
fn tables_parse_cells_and_alignment() {
    let source = "| left | center | right |\n|:-----|:------:|------:|\n| 1 | 2 | 3 |\n";
    let document = parse(source);
    let BlockKind::Table { alignments, head, rows } = &document.blocks[0].kind else {
        panic!("expected a table");
    };
    assert_eq!(alignments, &[Alignment::Left, Alignment::Center, Alignment::Right]);
    assert_eq!(head.len(), 3);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].len(), 3);
}

#[test]
fn gfm_alerts_are_classified() {
    let source = "> [!WARNING]\n> Careful.\n";
    let document = parse(source);
    let BlockKind::BlockQuote { kind, blocks } = &document.blocks[0].kind else {
        panic!("expected a quote");
    };
    assert_eq!(*kind, QuoteKind::Warning);
    assert_eq!(blocks.len(), 1);
}

#[test]
fn front_matter_is_hidden() {
    let source = "---\ntitle: Test\n---\n\n# Visible\n";
    let document = parse(source);
    let front_matter = document.front_matter.expect("front matter");
    assert!(front_matter.source.contains("title: Test"));
    assert_eq!(document.blocks.len(), 1);
    assert!(matches!(document.blocks[0].kind, BlockKind::Heading { .. }));
}

#[test]
fn math_events_become_inline_and_block_math() {
    let source = "Inline $a^2 + b^2$ math.\n\n$$\n\\int_0^1 x dx\n$$\n\n```math\nE = mc^2\n```\n";
    let document = parse(source);
    let BlockKind::Paragraph(inlines) = &document.blocks[0].kind else {
        panic!("expected a paragraph");
    };
    assert!(
        inlines
            .iter()
            .any(|inline| matches!(inline, Inline::Math { display: false, .. }))
    );
    assert!(
        document
            .blocks
            .iter()
            .any(|block| matches!(&block.kind, BlockKind::Code { math: true, .. }))
    );
}

#[test]
fn classify_destinations() {
    assert_eq!(classify("#section"), LinkKind::Anchor);
    assert_eq!(classify("other.md#section"), LinkKind::LocalFile);
    assert_eq!(classify("/tmp/a.png"), LinkKind::LocalFile);
    assert_eq!(classify("https://example.com/a.png"), LinkKind::External);
    assert_eq!(classify("mailto:a@example.com"), LinkKind::External);
    assert_eq!(classify("file:///tmp/a.png"), LinkKind::FileUrl);
    assert_eq!(classify("data:image/png;base64,AAAA"), LinkKind::Data);
    assert_eq!(classify("javascript:alert(1)"), LinkKind::Invalid);
    assert_eq!(classify("data:text/html,<script>"), LinkKind::Invalid);
}

#[test]
fn large_document_parses_quickly() {
    let mut source = String::with_capacity(1_100_000);
    let mut index = 0;
    while source.len() < 1_000_000 {
        source.push_str(&format!(
            "## Section {index}\n\nA paragraph with **strong** and [a link](other.md#{index}).\n\n| a | b |\n|---|---|\n| one | two |\n\n- item\n- item\n\n```rust\nfn f{index}() {{}}\n```\n\n"
        ));
        index += 1;
    }
    let started = std::time::Instant::now();
    let document = parse(&source);
    let elapsed = started.elapsed();
    assert!(document.blocks.len() > index);
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "1 MB parse took {elapsed:?}"
    );
}
