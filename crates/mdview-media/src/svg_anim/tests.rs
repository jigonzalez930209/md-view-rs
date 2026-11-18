//! Animation fixtures.

use crate::svg_anim::{frame_svg, parse};
use crate::{MediaKind, MediaRequest, rasterize_svg};

fn frame(svg: &str, time: f32) -> String {
    let animation = parse(svg).expect("parse").expect("animation");
    frame_svg(svg, &animation, time)
}

fn attr<'a>(svg: &'a str, name: &str) -> &'a str {
    let start = svg.find(&format!("{name}=\"")).expect("attribute");
    let rest = &svg[start + name.len() + 2..];
    &rest[..rest.find('"').expect("end")]
}

#[test]
fn smil_opacity_linear() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect width="10" height="10"><animate attributeName="opacity" values="0;1" dur="2s"/></rect></svg>"##;
    assert_eq!(attr(&frame(svg, 0.0), "opacity"), "0.000");
    assert_eq!(attr(&frame(svg, 1.0), "opacity"), "0.500");
    assert_eq!(attr(&frame(svg, 2.0), "opacity"), "1.000");
}

#[test]
fn smil_opacity_discrete() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect><animate attributeName="opacity" values="0;1" dur="2s" calcMode="discrete"/></rect></svg>"##;
    assert_eq!(attr(&frame(svg, 0.5), "opacity"), "0");
    assert_eq!(attr(&frame(svg, 1.5), "opacity"), "1");
}

#[test]
fn key_times_control_progress() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect><animate attributeName="opacity" values="0;1;0" keyTimes="0;0.25;1" dur="2s"/></rect></svg>"##;
    assert_eq!(attr(&frame(svg, 0.5), "opacity"), "1.000");
    assert_eq!(attr(&frame(svg, 1.0), "opacity"), "0.667");
}

#[test]
fn indefinite_repeat_loops() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect><animate attributeName="opacity" values="0;1" dur="1s" repeatCount="indefinite"/></rect></svg>"##;
    assert_eq!(attr(&frame(svg, 1.5), "opacity"), "0.500");
}

#[test]
fn begin_delays_the_clip() {
    // Before `begin` the element keeps its own value, as browsers show it (SMIL).
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect><animate attributeName="opacity" values="0;1" begin="1s" dur="1s"/></rect></svg>"##;
    assert!(!frame(svg, 0.5).contains("opacity="));
    assert_eq!(attr(&frame(svg, 1.5), "opacity"), "0.500");
}

#[test]
fn set_changes_fill_after_begin() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect fill="#000000"><set attributeName="fill" to="#ff0000" begin="1s"/></rect></svg>"##;
    assert_eq!(attr(&frame(svg, 0.5), "fill"), "#000000");
    assert_eq!(attr(&frame(svg, 1.5), "fill"), "#ff0000");
}

#[test]
fn fill_colors_interpolate() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect><animate attributeName="fill" values="#000000;#ffffff" dur="2s"/></rect></svg>"##;
    assert_eq!(attr(&frame(svg, 1.0), "fill"), "#808080");
}

#[test]
fn stroke_colors_interpolate() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect><animate attributeName="stroke" values="#ff0000;#0000ff" dur="2s"/></rect></svg>"##;
    assert_eq!(attr(&frame(svg, 1.0), "stroke"), "#800080");
}

#[test]
fn translate_transform() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect><animateTransform attributeName="transform" type="translate" values="0 0;10 20" dur="2s"/></rect></svg>"##;
    assert_eq!(attr(&frame(svg, 1.0), "transform"), "translate(5.000 10.000)");
}

#[test]
fn rotate_transform() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect><animateTransform type="rotate" values="0 5 5;180 5 5" dur="2s"/></rect></svg>"##;
    assert_eq!(attr(&frame(svg, 1.0), "transform"), "rotate(90.000 5.000 5.000)");
}

#[test]
fn scale_transform() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect><animateTransform type="scale" values="1;2" dur="1s"/></rect></svg>"##;
    assert_eq!(attr(&frame(svg, 0.5), "transform"), "scale(1.500)");
}

#[test]
fn geometry_animation() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><circle cx="10"><animate attributeName="cx" values="10;30" dur="2s"/></circle></svg>"##;
    assert_eq!(attr(&frame(svg, 1.0), "cx"), "20.000");
}

#[test]
fn css_keyframes_opacity() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><style>@keyframes fade { from { opacity: 0; } to { opacity: 1; } } .fade { animation: fade 2s linear infinite; }</style><rect class="fade"/></svg>"##;
    assert_eq!(attr(&frame(svg, 1.0), "opacity"), "0.500");
}

#[test]
fn css_keyframes_transform() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><style>@keyframes spin { 0% { transform: rotate(0); } 100% { transform: rotate(90); } } #spin { animation: spin 2s infinite linear; }</style><rect id="spin"/></svg>"##;
    assert_eq!(attr(&frame(svg, 1.0), "transform"), "rotate(45.000)");
}

#[test]
fn css_inline_animation() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><style>@keyframes glow { from { fill: #000000; } to { fill: #ffffff; } }</style><rect style="animation: glow 1s infinite linear"/></svg>"##;
    assert_eq!(attr(&frame(svg, 0.5), "fill"), "#808080");
}

#[test]
fn static_svg_has_no_animation() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect fill="#000"/></svg>"##;
    assert!(parse(svg).expect("parse").is_none());
}

#[test]
fn unsupported_scripts_do_not_create_clips() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><script>animate</script><rect/></svg>"##;
    assert!(parse(svg).expect("parse").is_none());
}

#[test]
fn raster_frames_differ_over_time() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"><animate attributeName="opacity" values="0;1" dur="1s"/></rect></svg>"##;
    let animation = parse(svg).expect("parse").expect("animation");
    let mut request = MediaRequest::new(MediaKind::Svg, "pulse.svg");
    request.max_width = 10;
    request.max_height = 10;
    let first = rasterize_svg(frame_svg(svg, &animation, 0.0).as_bytes(), &request).expect("first");
    let last = rasterize_svg(frame_svg(svg, &animation, 1.0).as_bytes(), &request).expect("last");
    assert_eq!(first.rgba[3], 0);
    assert!(last.rgba[3] > 200);
}

#[test]
fn partial_motion_features_are_reported() {
    // keyPoints is not supported: the motion still plays, and the document is flagged.
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect><animateMotion path="M0 0 L10 10" dur="1s" keyPoints="0;1" keyTimes="0;1"/></rect></svg>"##;
    let animation = parse(svg).expect("parse").expect("animation");
    assert!(animation.unsupported);
}
