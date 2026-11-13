//! Depth-limited flattening for hostile nesting.

use super::*;

impl EventParser<'_> {
    pub(super) fn nested_inlines(&mut self, end: InlineEnd) -> Vec<Inline> {
        if self.depth >= MAX_NESTING {
            return self.collect_inline_text();
        }
        self.depth += 1;
        let (children, _) = self.parse_inlines(end);
        self.depth -= 1;
        children
    }

    /// Consume a nested inline container, flattening its text instead of recursing.
    pub(super) fn collect_inline_text(&mut self) -> Vec<Inline> {
        let mut level = 1usize;
        let mut text = String::new();
        while let Some((event, _)) = self.peek().cloned() {
            self.cursor += 1;
            match event {
                Event::Start(_) => level += 1,
                Event::End(_) => {
                    level -= 1;
                    if level == 0 {
                        break;
                    }
                }
                Event::Text(value) | Event::Code(value) | Event::InlineMath(value) | Event::DisplayMath(value) => {
                    text.push_str(&value)
                }
                Event::SoftBreak | Event::HardBreak => text.push(' '),
                _ => {}
            }
        }
        if text.is_empty() {
            Vec::new()
        } else {
            vec![Inline::Text(text)]
        }
    }

    /// Consume a block container, flattening its text into one paragraph.
    pub(super) fn collect_block_text(&mut self) -> Vec<Block> {
        let mut level = 1usize;
        let mut text = String::new();
        while let Some((event, _)) = self.peek().cloned() {
            self.cursor += 1;
            match event {
                Event::Start(_) => level += 1,
                Event::End(_) => {
                    level -= 1;
                    if level == 0 {
                        break;
                    }
                }
                Event::Text(value) | Event::Code(value) => text.push_str(&value),
                Event::SoftBreak | Event::HardBreak => text.push('\n'),
                _ => {}
            }
        }
        let mut blocks = Vec::new();
        if !text.is_empty() {
            self.push_block(
                &mut blocks,
                Span::default(),
                BlockKind::Paragraph(vec![Inline::Text(text)]),
            );
        }
        blocks
    }

    pub(super) fn collect_until_block_end(&mut self, end: BlockEnd) -> Vec<Block> {
        let mut level = 1usize;
        let mut text = String::new();
        while let Some((event, _)) = self.peek().cloned() {
            self.cursor += 1;
            match event {
                Event::Start(_) => level += 1,
                Event::End(_) => {
                    level -= 1;
                    if level == 0 {
                        break;
                    }
                }
                Event::Text(value) => text.push_str(&value),
                _ => {}
            }
        }
        let _ = end;
        let mut blocks = Vec::new();
        if !text.is_empty() {
            self.push_block(
                &mut blocks,
                Span::default(),
                BlockKind::Paragraph(vec![Inline::Text(text)]),
            );
        }
        blocks
    }
}
