//! Deterministic fuzz-style hardening tests.

use mdview_markdown::parse;

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

#[test]
fn random_markdown_never_panics() {
    let alphabet = b"#*_-[]()<>!`~|:\\n\\t abcXYZ0123$\\\"'";
    let mut state = 0x5eed_u64;
    for _ in 0..300 {
        let length = (lcg(&mut state) % 4_000) as usize;
        let mut source = String::with_capacity(length);
        for _ in 0..length {
            let index = (lcg(&mut state) as usize) % alphabet.len();
            source.push(alphabet[index] as char);
        }
        let document = parse(&source);
        for block in &document.blocks {
            assert!(block.span.start <= block.span.end);
            assert!(block.span.start_line <= block.span.end_line);
        }
    }
}

#[test]
fn deeply_nested_containers_are_bounded() {
    let mut source = String::new();
    for _ in 0..2_000 {
        source.push_str("> ");
    }
    source.push_str("deep\\n");
    let document = parse(&source);
    assert!(!document.blocks.is_empty());

    let mut emphasis = String::new();
    for _ in 0..1_000 {
        emphasis.push('*');
    }
    emphasis.push_str("text");
    for _ in 0..1_000 {
        emphasis.push('*');
    }
    let document = parse(&emphasis);
    assert!(!document.blocks.is_empty());
}
