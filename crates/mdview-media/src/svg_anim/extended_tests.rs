//! Fixtures for motion paths, timing curves, fill modes, directions, and the wider CSS subset.

use crate::svg_anim::{frame_svg, parse};

fn frame(svg: &str, time: f32) -> String {
    let animation = parse(svg).expect("parse").expect("animation");
    frame_svg(svg, &animation, time)
}

fn attr(svg: &str, name: &str) -> Option<String> {
    let start = svg.find(&format!(" {name}=\""))?;
    let rest = &svg[start + name.len() + 3..];
    Some(rest[..rest.find('"')?].to_owned())
}

fn number(svg: &str, name: &str) -> f32 {
    attr(svg, name).and_then(|value| value.parse().ok()).expect("number")
}

#[test]
fn motion_follows_a_path_in_front_of_the_own_transform() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect transform="scale(2)"><animateMotion path="M0 0 L100 0" dur="2s"/></rect></svg>"##;
    assert_eq!(
        attr(&frame(svg, 1.0), "transform").as_deref(),
        Some("translate(50.000 0.000) scale(2)")
    );
}

#[test]
fn motion_follows_an_mpath_and_turns_with_it() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><path id="track" d="M0 0 L0 40"/><circle r="2"><animateMotion dur="4s" rotate="auto" repeatCount="indefinite"><mpath xlink:href="#track"/></animateMotion></circle></svg>"##;
    assert_eq!(
        attr(&frame(svg, 1.0), "transform").as_deref(),
        Some("translate(0.000 10.000) rotate(90.000)")
    );
}

#[test]
fn stroke_dashes_draw_a_line() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0 L100 0" stroke-dasharray="100"><animate attributeName="stroke-dashoffset" from="100" to="0" dur="2s" fill="freeze"/></path></svg>"##;
    assert_eq!(number(&frame(svg, 1.0), "stroke-dashoffset"), 50.0);
    assert_eq!(number(&frame(svg, 5.0), "stroke-dashoffset"), 0.0, "frozen at the end");
}

#[test]
fn without_freeze_the_value_is_removed_after_the_end() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect><animate attributeName="width" from="0" to="10" dur="1s"/></rect></svg>"##;
    assert!(attr(&frame(svg, 2.0), "width").is_none());
}

#[test]
fn key_splines_ease_each_segment() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect><animate attributeName="x" values="0;100" dur="1s" calcMode="spline" keySplines="0.42 0 1 1"/></rect></svg>"##;
    assert!(number(&frame(svg, 0.25), "x") < 25.0, "ease-in starts slow");
}

#[test]
fn named_and_short_colors_interpolate() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect><animate attributeName="fill" values="black;#fff" dur="2s"/></rect></svg>"##;
    assert_eq!(attr(&frame(svg, 1.0), "fill").as_deref(), Some("#808080"));
}

#[test]
fn css_defaults_to_ease_and_honours_delay_direction_and_count() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><style>@keyframes slide { from { opacity: 0 } to { opacity: 1 } } .a { animation: slide 1s 1s 2 alternate; }</style><rect class="a"/></svg>"##;
    assert!(attr(&frame(svg, 0.5), "opacity").is_none(), "delayed");
    let eased = number(&frame(svg, 1.5), "opacity");
    assert!(eased > 0.7, "ease moves fast in the middle: {eased}");
    let back = number(&frame(svg, 2.25), "opacity");
    assert!(back > 0.8, "the second iteration runs backwards: {back}");
    assert!(
        attr(&frame(svg, 3.5), "opacity").is_none(),
        "no fill mode: removed after two runs"
    );
}

#[test]
fn css_longhands_dashes_and_cubic_bezier() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><style>@keyframes draw { 0% { stroke-dashoffset: 40 } 100% { stroke-dashoffset: 0 } } #p { animation-name: draw; animation-duration: 2s; animation-timing-function: cubic-bezier(0, 0, 1, 1); animation-fill-mode: forwards; }</style><path id="p" d="M0 0 H40"/></svg>"##;
    assert!((number(&frame(svg, 1.0), "stroke-dashoffset") - 20.0).abs() < 0.5);
    assert_eq!(
        number(&frame(svg, 9.0), "stroke-dashoffset"),
        0.0,
        "forwards keeps the end"
    );
}

#[test]
fn css_units_become_svg_user_units() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><style>@keyframes up { from { transform: translate(0, 0px) rotate(0deg); stroke-width: 2px } to { transform: translate(0, -24px) rotate(90deg); stroke-width: 6px } } .u { animation: up 2s linear; }</style><rect class="u"/></svg>"##;
    let frame = frame(svg, 1.0);
    assert_eq!(
        attr(&frame, "transform").as_deref(),
        Some("translate(0.000, -12.000) rotate(45.000)")
    );
    assert_eq!(number(&frame, "stroke-width"), 4.0);
}
