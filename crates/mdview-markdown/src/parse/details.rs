//! Block post-passes: group `<details>` HTML with its Markdown, drop comment-only
//! HTML blocks, and move footnote definitions to the end, as GitHub renders them.
//!
//! GitHub READMEs commonly write a collapsible section as an opening HTML block
//! (`<details><summary>…</summary>`), ordinary Markdown, and a closing `</details>`
//! block. CommonMark sees three unrelated blocks; this pass turns them into one
//! [`BlockKind::Details`] so the renderer can collapse the Markdown like GitHub.

use crate::{Block, BlockKind, Span};

fn count(html: &str, needle: &str) -> usize {
    html.to_ascii_lowercase().matches(needle).count()
}

/// Net `<details>` nesting an HTML block opens (positive) or closes (negative).
fn balance(html: &str) -> isize {
    count(html, "<details").cast_signed() - count(html, "</details").cast_signed()
}

/// Whether an HTML block renders nothing on GitHub: only comments and whitespace.
fn is_comment_only(html: &str) -> bool {
    let mut rest = html;
    while let Some(start) = rest.find("<!--") {
        if !rest[..start].trim().is_empty() {
            return false;
        }
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => return true,
        }
    }
    rest.trim().is_empty()
}

/// Group details elements at this level and inside container blocks, and drop HTML
/// blocks that hold only comments (GitHub renders nothing for them).
pub(super) fn group(blocks: Vec<Block>) -> Vec<Block> {
    let mut out = Vec::with_capacity(blocks.len());
    let blocks = blocks
        .into_iter()
        .filter(|block| !matches!(&block.kind, BlockKind::Html(html) if is_comment_only(html)));
    let mut iter = blocks.peekable();
    while let Some(block) = iter.next() {
        let block = recurse(block);
        let opens = matches!(&block.kind, BlockKind::Html(html) if balance(html) > 0);
        if !opens {
            out.push(block);
            continue;
        }
        let BlockKind::Html(summary) = &block.kind else {
            out.push(block);
            continue;
        };
        let mut depth = balance(summary);
        let mut inner = Vec::new();
        let mut closed_at = None;
        let mut pending = Vec::new();
        for next in iter.by_ref() {
            if let BlockKind::Html(html) = &next.kind {
                depth += balance(html);
                if depth <= 0 {
                    closed_at = Some(next.span);
                    break;
                }
            }
            pending.push(next);
        }
        match closed_at {
            Some(end) => {
                inner.extend(pending);
                let open = summary.to_ascii_lowercase().contains(" open");
                out.push(Block {
                    id: block.id,
                    span: Span {
                        start: block.span.start,
                        end: end.end,
                        start_line: block.span.start_line,
                        end_line: end.end_line,
                    },
                    kind: BlockKind::Details {
                        summary: summary.clone(),
                        open,
                        blocks: group(inner),
                    },
                });
            }
            None => {
                // Never closed: keep the blocks as they are.
                out.push(block);
                out.extend(pending.into_iter().map(recurse));
            }
        }
    }
    out
}

/// Move footnote definitions after the other blocks, keeping their order: GitHub
/// renders every note in one section at the end of the document.
pub(super) fn footnotes_last(blocks: Vec<Block>) -> Vec<Block> {
    let (notes, mut rest): (Vec<Block>, Vec<Block>) = blocks
        .into_iter()
        .partition(|block| matches!(block.kind, BlockKind::FootnoteDefinition { .. }));
    rest.extend(notes);
    rest
}

fn recurse(mut block: Block) -> Block {
    match &mut block.kind {
        BlockKind::BlockQuote { blocks, .. } | BlockKind::FootnoteDefinition { blocks, .. } => {
            *blocks = group(std::mem::take(blocks));
        }
        BlockKind::List { items, .. } => {
            for item in items {
                item.blocks = group(std::mem::take(&mut item.blocks));
            }
        }
        _ => {}
    }
    block
}

#[cfg(test)]
mod tests {
    use crate::{BlockKind, parse};

    #[test]
    fn details_wrap_the_markdown_between_their_html_blocks() {
        let document =
            parse("<details>\n<summary>More</summary>\n\nHidden **text**.\n\n- item\n\n</details>\n\nAfter.\n");
        assert_eq!(document.blocks.len(), 2);
        let BlockKind::Details { summary, open, blocks } = &document.blocks[0].kind else {
            panic!("expected details, got {:?}", document.blocks[0].kind);
        };
        assert!(summary.contains("<summary>More</summary>"));
        assert!(!open);
        assert_eq!(blocks.len(), 2);
        assert!(matches!(document.blocks[1].kind, BlockKind::Paragraph(_)));
    }

    #[test]
    fn footnote_definitions_move_to_the_end() {
        let document = parse("Text[^a].\n\n[^a]: Note.\n\nMore text.\n");
        assert!(matches!(document.blocks[1].kind, BlockKind::Paragraph(_)));
        assert!(matches!(document.blocks[2].kind, BlockKind::FootnoteDefinition { .. }));
    }

    #[test]
    fn comment_only_html_blocks_disappear() {
        let document = parse("Before.\n\n<!-- hidden -->\n\nAfter.\n");
        assert_eq!(document.blocks.len(), 2);
    }

    #[test]
    fn single_block_and_unclosed_details_stay_html() {
        let document = parse("<details><summary>A</summary>Body</details>\n\n<details open>\n\nText\n");
        assert!(matches!(document.blocks[0].kind, BlockKind::Html(_)));
        assert!(matches!(document.blocks[1].kind, BlockKind::Html(_)));
        assert!(matches!(document.blocks[2].kind, BlockKind::Paragraph(_)));
    }
}
