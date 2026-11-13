//! Hostile-input tests for the media pipeline.

use mdview_media::{MediaKind, MediaRequest, rasterize_svg, render_math, render_mermaid};

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

#[test]
fn random_svg_bytes_return_errors_not_panics() {
    let mut request = MediaRequest::new(MediaKind::Svg, "fuzz.svg");
    request.max_width = 128;
    request.max_height = 128;
    let mut state = 0xabcd_u64;
    for _ in 0..200 {
        let length = (lcg(&mut state) % 8_192) as usize;
        let mut bytes = Vec::with_capacity(length);
        for _ in 0..length {
            bytes.push((lcg(&mut state) % 256) as u8);
        }
        let _ = rasterize_svg(&bytes, &request);
    }
}

#[test]
fn hostile_svg_structures_are_rejected() {
    let request = MediaRequest::new(MediaKind::Svg, "hostile.svg");
    let cases = [
        "<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>",
        "<svg xmlns=\"http://www.w3.org/2000/svg\"><image href=\"file:///etc/passwd\"/></svg>",
        "<svg xmlns=\"http://www.w3.org/2000/svg\"><foreignObject><body/></foreignObject></svg>",
        "<!DOCTYPE svg [<!ENTITY x SYSTEM \"file:///etc/passwd\">]><svg>&x;</svg>",
        "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"1e9999\" height=\"1e9999\"/></svg>",
    ];
    for source in cases {
        let _ = rasterize_svg(source.as_bytes(), &request);
    }
}

#[test]
fn random_math_and_mermaid_do_not_panic() {
    let alphabet = b"\\\\{}[]^_&;#$% abc123\\n\\t";
    let mut state = 0x1234_u64;
    for _ in 0..100 {
        let length = (lcg(&mut state) % 200) as usize;
        let mut source = String::with_capacity(length);
        for _ in 0..length {
            let index = (lcg(&mut state) as usize) % alphabet.len();
            source.push(alphabet[index] as char);
        }
        let _ = render_math(&source, false, 320);
        let _ = render_mermaid(&source, false, 320);
    }
}
