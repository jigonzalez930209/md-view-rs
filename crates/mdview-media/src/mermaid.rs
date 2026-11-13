//! Mermaid diagram rendering with the native `mermaid-rs-renderer`.

use mermaid_rs_renderer::{LayoutConfig, Theme, compute_layout, parse_mermaid, render_svg};

use crate::{MediaError, MediaKind, MediaRequest, Raster, rasterize_svg};

/// Render a Mermaid diagram to an RGBA raster.
pub fn render_mermaid(source: &str, dark: bool, max_width: u32) -> Result<Raster, MediaError> {
    let source = source.trim();
    if source.is_empty() {
        return Err(MediaError::Empty);
    }
    let parsed = parse_mermaid(source).map_err(|error| MediaError::Mermaid(error.to_string()))?;
    let theme = if dark { Theme::dark() } else { Theme::modern() };
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let svg = render_svg(&layout, &theme, &config);
    let mut request = MediaRequest::new(MediaKind::Svg, source);
    request.max_width = max_width.max(64);
    request.max_height = max_width.saturating_mul(2).max(64);
    request.dark = dark;
    rasterize_svg(svg.as_bytes(), &request).map_err(|error| match error {
        MediaError::Svg(message) => MediaError::Mermaid(message),
        other => other,
    })
}

#[cfg(test)]
mod tests {
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
            ("gantt", "gantt\n title Plan\n section Work\n Task :a1, 2024-01-01, 3d"),
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
        ];
        for (name, source) in cases {
            let raster = render_mermaid(source, false, 640).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert!(raster.is_consistent(), "{name}");
        }
    }

    #[test]
    fn unsupported_diagrams_report_an_error() {
        let error = render_mermaid("unknownDiagram\n    A --> B", false, 400).expect_err("error");
        assert!(matches!(error, MediaError::Mermaid(_)));
    }
}
