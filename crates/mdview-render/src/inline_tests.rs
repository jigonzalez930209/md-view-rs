//! Inline segment tests.

use mdview_markdown::Inline;

#[test]
fn ins_and_u_tags_underline_their_text() {
    let inlines = vec![
        Inline::Text("a ".to_owned()),
        Inline::Html("<ins>".to_owned()),
        Inline::Text("b".to_owned()),
        Inline::Html("</ins>".to_owned()),
        Inline::Html("<u>".to_owned()),
        Inline::Text("c".to_owned()),
        Inline::Html("</u>".to_owned()),
        Inline::Text(" d".to_owned()),
    ];
    let underlined: Vec<(String, bool)> = super::segments(&inlines)
        .into_iter()
        .map(|segment| (segment.text, segment.underline))
        .collect();
    assert_eq!(
        underlined,
        vec![
            ("a ".to_owned(), false),
            ("b".to_owned(), true),
            ("c".to_owned(), true),
            (" d".to_owned(), false)
        ]
    );
}
