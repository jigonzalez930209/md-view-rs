use super::*;

#[test]
fn flowcharts_render() {
    let source = "flowchart LR\n    A[Start] --> B{Decision}\n    B -->|Yes| C[OK]\n    B -->|No| D[Cancel]";
    let raster = render_mermaid(source, false, 640).expect("render");
    assert!(raster.width > 0 && raster.height > 0);
    assert!(raster.is_consistent());
}

#[test]
fn sequence_diagrams_render() {
    let source = "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi";
    assert!(render_mermaid(source, true, 640).is_ok());
}

#[test]
fn twenty_tier_a_diagrams_render() {
    let cases = [
        ("flowchart LR", "flowchart LR\n A --> B --> C"),
        (
            "flowchart TD",
            "flowchart TD\n A[Start] --> B{Choice}\n B -->|Yes| C[End]",
        ),
        (
            "flowchart subgraph",
            "flowchart LR\n subgraph One\n A --> B\n end\n B --> C",
        ),
        (
            "flowchart styled",
            "flowchart LR\n A:::hot --> B\n classDef hot fill:#f00",
        ),
        ("sequence", "sequenceDiagram\n Alice->>Bob: Hello\n Bob-->>Alice: Hi"),
        (
            "sequence loop",
            "sequenceDiagram\n loop Every minute\n Alice->>Bob: Ping\n end",
        ),
        (
            "sequence alt",
            "sequenceDiagram\n alt ok\n Alice->>Bob: Yes\n else no\n Alice->>Bob: No\n end",
        ),
        (
            "class",
            "classDiagram\n class Animal { +String name }\n Animal <|-- Dog",
        ),
        ("class relation", "classDiagram\n A --> B : uses"),
        (
            "state",
            "stateDiagram-v2\n [*] --> Idle\n Idle --> Running\n Running --> [*]",
        ),
        (
            "state note",
            "stateDiagram-v2\n state Check { [*] --> Ready }\n [*] --> Check",
        ),
        ("er", "erDiagram\n CUSTOMER ||--o{ ORDER : places"),
        (
            "er fields",
            "erDiagram\n USER { string name }\n USER ||--|| PROFILE : has",
        ),
        ("pie", "pie title Pets\n \"Dogs\" : 60\n \"Cats\" : 40"),
        ("pie show", "pie showData\n \"A\" : 1\n \"B\" : 2"),
        (
            "gantt",
            "gantt\n title Plan\n dateFormat YYYY-MM-DD\n section Work\n Task :a1, 2024-01-01, 3d",
        ),
        (
            "journey",
            "journey\n title Day\n section Work\n Wake: 5: Me\n Code: 3: Me",
        ),
        ("timeline", "timeline\n title History\n 2020 : Start\n 2021 : Next"),
        ("mindmap", "mindmap\n root((MD View))\n  Editor\n  Preview"),
        (
            "gitGraph",
            "gitGraph\n commit\n branch dev\n checkout dev\n commit\n checkout main\n merge dev",
        ),
        (
            "quadrant",
            "quadrantChart\n title Reach\n x-axis Low --> High\n y-axis Low --> High\n A: [0.3, 0.6]",
        ),
        (
            "xychart",
            "xychart-beta\n x-axis [jan, feb, mar]\n y-axis 0 --> 10\n bar [3, 7, 5]",
        ),
    ];
    for (name, source) in cases {
        let raster = render_mermaid(source, false, 640).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(raster.is_consistent(), "{name}");
    }
}

/// A sequence diagram with nested fragments: the `loop` after an `alt` must start
/// below it (the previous renderer drew it over the `alt` frame).
#[test]
fn sequence_fragments_do_not_overlap() {
    let source = "sequenceDiagram\n participant U\n participant A\n U->>A: login\n alt ok\n A-->>U: token\n else bad\n A-->>U: 401\n end\n loop refresh\n U->>A: renew\n end";
    let svg = mermaid_svg(source, false).expect("svg");
    let frames = frame_tops(&svg);
    assert_eq!(frames.len(), 2, "{frames:?}");
    let (alt_top, alt_bottom) = frames[0];
    let (loop_top, _) = frames[1];
    assert!(alt_bottom > alt_top);
    assert!(
        loop_top >= alt_bottom - 0.5,
        "loop {loop_top} starts inside alt {alt_top}..{alt_bottom}"
    );
}

/// The vertical extent of each `loop`/`alt` frame: Mermaid draws each as four solid
/// `loopLine`s; the dashed `else` separators are skipped.
fn frame_tops(svg: &str) -> Vec<(f32, f32)> {
    let mut lines = Vec::new();
    for line in svg.split("<line").skip(1) {
        let element = line.split('>').next().unwrap_or("");
        if !element.contains("loopLine") || element.contains("dasharray") {
            continue;
        }
        let value = |name: &str| -> Option<f32> {
            let start = element.find(&format!(" {name}=\""))? + name.len() + 3;
            element[start..].split('"').next()?.parse().ok()
        };
        if let (Some(y1), Some(y2)) = (value("y1"), value("y2")) {
            lines.push((y1, y2));
        }
    }
    lines
        .chunks(4)
        .filter(|chunk| chunk.len() == 4)
        .map(|chunk| {
            let ys = chunk.iter().flat_map(|(y1, y2)| [*y1, *y2]);
            let top = ys.clone().fold(f32::INFINITY, f32::min);
            let bottom = ys.fold(f32::NEG_INFINITY, f32::max);
            (top, bottom)
        })
        .collect()
}

/// A tall diagram keeps its natural size instead of shrinking to twice the width.
#[test]
fn tall_diagrams_are_not_squeezed() {
    let mut source = String::from("flowchart TD\n");
    for index in 0..14 {
        source.push_str(&format!(
            " N{index}[Step {index}] --> N{}[Step {}]\n",
            index + 1,
            index + 1
        ));
    }
    let raster = render_mermaid(&source, false, 400).expect("render");
    assert!(raster.height > 800, "{}x{}", raster.width, raster.height);
}

#[test]
fn diagrams_carry_no_html_labels() {
    let source = "flowchart TD\n A[\"**Bold** label\"] -->|edge| B(Round)\n subgraph S[Group]\n B\n end";
    let svg = mermaid_svg(source, true).expect("svg");
    assert!(!svg.to_ascii_lowercase().contains("<foreignobject"));
}

#[test]
fn unsupported_diagrams_report_an_error() {
    let error = render_mermaid("unknownDiagram\n    A --> B", false, 400).expect_err("error");
    assert!(matches!(error, MediaError::Mermaid(_)));
}

#[test]
fn empty_sources_are_refused() {
    assert_eq!(
        render_mermaid("  \n", false, 400).expect_err("empty"),
        MediaError::Empty
    );
}
