//! Icon generation for packages.

use std::fs;
use std::path::Path;

/// Sizes written as PNG and bundled into ICO/ICNS.
pub const SIZES: [u32; 8] = [16, 32, 48, 64, 128, 256, 512, 1024];

/// Generate PNG, ICO, and ICNS icons from `assets/icon.svg`.
pub fn generate() -> Result<(), String> {
    let svg = fs::read_to_string("assets/icon.svg").map_err(|error| error.to_string())?;
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).map_err(|error| error.to_string())?;
    fs::create_dir_all("assets/icons").map_err(|error| error.to_string())?;
    let mut pngs = Vec::new();
    for size in SIZES {
        let png = render_png(&tree, size)?;
        fs::write(format!("assets/icons/{size}.png"), &png).map_err(|error| error.to_string())?;
        pngs.push((size, png));
    }
    fs::write("assets/icons/mdview.ico", ico(&pngs)?).map_err(|error| error.to_string())?;
    fs::write("assets/icons/mdview.icns", icns(&pngs)?).map_err(|error| error.to_string())?;
    println!("wrote {} icon sizes to assets/icons/", pngs.len());
    Ok(())
}

fn render_png(tree: &usvg::Tree, size: u32) -> Result<Vec<u8>, String> {
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(size, size).ok_or_else(|| format!("cannot allocate {size}x{size}"))?;
    let scale = size as f32 / tree.size().width().max(1.0);
    resvg::render(
        tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|error| error.to_string())
}

fn ico(pngs: &[(u32, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let entries = pngs.iter().filter(|(size, _)| *size <= 256).collect::<Vec<_>>();
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    let mut offset = 6 + entries.len() * 16;
    for (size, data) in &entries {
        let dimension = if *size >= 256 { 0 } else { *size as u8 };
        out.push(dimension);
        out.push(dimension);
        out.push(0);
        out.push(0);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += data.len();
    }
    for (_, data) in &entries {
        out.extend_from_slice(data);
    }
    Ok(out)
}

fn icns(pngs: &[(u32, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let kinds = [
        (16u32, *b"icp4"),
        (32, *b"icp5"),
        (64, *b"icp6"),
        (128, *b"ic07"),
        (256, *b"ic08"),
        (512, *b"ic09"),
        (1024, *b"ic10"),
    ];
    let mut body = Vec::new();
    for (size, kind) in kinds {
        let Some((_, data)) = pngs.iter().find(|(candidate, _)| *candidate == size) else {
            continue;
        };
        body.extend_from_slice(&kind);
        body.extend_from_slice(&((data.len() + 8) as u32).to_be_bytes());
        body.extend_from_slice(data);
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// Copy the generated PNGs into a package icon tree.
pub fn install(stage: &Path) -> Result<(), String> {
    for size in [16u32, 32, 48, 64, 128, 256, 512] {
        let directory = stage.join(format!("usr/share/icons/hicolor/{size}x{size}/apps"));
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        fs::copy(format!("assets/icons/{size}.png"), directory.join("mdview.png"))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}
