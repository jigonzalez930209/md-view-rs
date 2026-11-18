#!/usr/bin/env python3
"""Regenerate the media fixtures in examples/v1-workspace/media.

Every viewer added after 1.0.0-rc.5 has sample files there: still images in each supported
format, animated rasters and SVGs, a multi-page PDF, videos, and delimited tables. The files
are committed; this script documents how they were made and rebuilds them.

Requires Pillow, FFmpeg (with libx264, libvpx-vp9, libopus, libjxl, libwebp, exr, hdr, qoi),
The PDF is written by hand. Run from the repository root:

    python3 scripts/fixtures/make_media.py
"""

from __future__ import annotations

import math
import random
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2] / "examples" / "v1-workspace" / "media"
W, H = 240, 150
ACCENT = (9, 105, 218)


def font(size: int) -> ImageFont.ImageFont:
    for name in ("DejaVuSans-Bold.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default()


def card(label: str, phase: float = 0.0) -> Image.Image:
    """A labelled test card: gradient, color bars, a moving disc, and the format name."""
    image = Image.new("RGBA", (W, H))
    pixels = image.load()
    for y in range(H):
        for x in range(W):
            pixels[x, y] = (int(255 * x / W), int(160 * y / H) + 40, 200 - int(120 * x / W), 255)
    draw = ImageDraw.Draw(image)
    bars = [(255, 255, 255), (255, 214, 0), (0, 200, 220), (0, 190, 80), (230, 0, 150), (220, 30, 30), ACCENT]
    for index, color in enumerate(bars):
        draw.rectangle([index * W // 7, H - 26, (index + 1) * W // 7, H], fill=color)
    cx = 30 + (W - 60) * (0.5 + 0.5 * math.sin(phase * 2 * math.pi))
    draw.ellipse([cx - 14, 34, cx + 14, 62], fill=(255, 255, 255, 255), outline=(0, 0, 0, 255), width=3)
    draw.rounded_rectangle([10, 72, W - 10, 108], radius=8, fill=(0, 0, 0, 170))
    text_font = font(20)
    box = draw.textbbox((0, 0), label, font=text_font)
    draw.text(((W - (box[2] - box[0])) / 2, 78), label, font=text_font, fill=(255, 255, 255, 255))
    draw.text((10, 8), "MD View", font=font(14), fill=(255, 255, 255, 255))
    return image


def run(*args: str | Path) -> None:
    subprocess.run([str(arg) for arg in args], check=True, stdin=subprocess.DEVNULL)


def ffmpeg(*args: str | Path) -> None:
    run("ffmpeg", "-v", "error", "-y", *args)


def images(tmp: Path) -> None:
    out = ROOT / "images"
    out.mkdir(parents=True, exist_ok=True)
    pillow = {
        "png": {}, "jpg": {"quality": 88}, "jfif": {"format": "JPEG", "quality": 88}, "webp": {"quality": 90},
        "bmp": {}, "dib": {"format": "BMP"}, "tiff": {"compression": "tiff_lzw"}, "gif": {},
        "ico": {"sizes": [(64, 64), (32, 32), (16, 16)]}, "tga": {}, "ppm": {}, "pgm": {}, "pbm": {},
    }
    for extension, options in pillow.items():
        image = card(extension.upper())
        if extension in ("jpg", "jfif", "bmp", "dib", "ppm", "tga"):
            image = image.convert("RGB")
        elif extension == "pgm":
            image = image.convert("L")
        elif extension == "pbm":
            image = image.convert("1")
        image.save(out / f"sample.{extension}", **options)
    source = tmp / "source.png"
    card("JPEG XL").save(source)
    ffmpeg("-i", source, "-c:v", "libjxl", "-distance", "1.0", out / "sample.jxl")
    card("QOI").save(source)
    ffmpeg("-i", source, "-c:v", "qoi", out / "sample.qoi")
    card("PAM").save(source)
    ffmpeg("-i", source, "-c:v", "pam", "-f", "image2", out / "sample.pam")
    card("OpenEXR").save(source)
    ffmpeg("-i", source, "-pix_fmt", "gbrpf32le", "-c:v", "exr", "-compression", "zip1", out / "sample.exr")
    card("Radiance HDR").save(source)
    ffmpeg("-i", source, "-c:v", "hdr", "-f", "image2", out / "sample.hdr")
    farbfeld = card("farbfeld")
    data = bytearray(b"farbfeld" + struct.pack(">II", W, H))
    data += b"".join(struct.pack(">H", channel * 257) for channel in farbfeld.tobytes())
    (out / "sample.ff").write_bytes(bytes(data))


def animated_rasters() -> None:
    out = ROOT / "animated"
    out.mkdir(parents=True, exist_ok=True)
    frames = [card("GIF", index / 24) for index in range(24)]
    frames[0].save(out / "bounce.gif", save_all=True, append_images=frames[1:], duration=50, loop=0, disposal=2)
    frames = [card("WebP", index / 24) for index in range(24)]
    frames[0].save(out / "bounce.webp", save_all=True, append_images=frames[1:], duration=50, loop=0, quality=80)
    frames = [card("APNG", index / 24) for index in range(24)]
    frames[0].save(out / "bounce.apng", format="PNG", save_all=True, append_images=frames[1:], duration=50, loop=0)


def pdf_text(x: float, y: float, size: float, text: str, bold: bool = False) -> str:
    escaped = text.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")
    escaped = escaped.encode("cp1252").decode("latin-1")
    return f"\nBT /{'F2' if bold else 'F1'} {size} Tf {x} {y} Td ({escaped}) Tj ET\n"


def pdf(tmp: Path) -> None:
    """A hand-built PDF: vector text in two fonts, a table, shapes, a JPEG, and mixed page sizes."""
    out = ROOT / "documents"
    out.mkdir(parents=True, exist_ok=True)
    photo = tmp / "photo.jpg"
    card("Embedded JPEG").convert("RGB").save(photo, quality=85)
    jpeg = photo.read_bytes()
    pages: list[tuple[float, float, str]] = []
    a4, letter = (595.0, 842.0), (612.0, 792.0)
    # Page 1: title, text, image, shapes.
    body = "0.035 0.41 0.85 rg 40 770 515 40 re f\n1 1 1 rg" + pdf_text(56, 783, 22, "PDF viewer fixture", True)
    body += "0 0 0 rg"
    lines = [
        "Use the toolbar arrows to move between pages, - and + to zoom,",
        "Fit width to fill the view, and 100% for actual size.",
        "Accents render through WinAnsi: áéíóú ñ ç ü - \u201cquotes\u201d \u2013 dashes \u2022 bullets.",
    ]
    for index, line in enumerate(lines):
        body += pdf_text(56, 730 - index * 18, 12, line)
    body += "q 320 0 0 200 56 440 cm /Im1 Do Q\n"
    body += "0.9 0.2 0.2 RG 4 w 420 600 m 520 480 l S\n"
    body += "0.1 0.7 0.3 rg 400 420 120 40 re f\n0.95 0.75 0 rg 460 520 m 500 450 l 420 450 l f\n"
    pages.append((*a4, body))
    # Page 2: a table drawn with rules.
    body = "0 0 0 rg" + pdf_text(56, 790, 18, "Table", True)
    top, row_height, columns = 760, 16, [56, 96, 260, 360, 460, 539]
    headers = ["#", "Name", "Value", "Ratio", "Flag"]
    for col, header in enumerate(headers):
        body += pdf_text(columns[col] + 4, top - 12, 10, header, True)
    for row in range(1, 41):
        y = top - 12 - row * row_height
        cells = [str(row), f"Item {row}", f"{row * 3.5:.2f}", f"{row / 41:.3f}", "yes" if row % 2 else "no"]
        for col, cell in enumerate(cells):
            body += pdf_text(columns[col] + 4, y, 10, cell)
    body += "0.5 G 0.5 w\n"
    for row in range(42):
        y = top - row * row_height
        body += f"{columns[0]} {y} m {columns[-1]} {y} l S\n"
    for x in columns:
        body += f"{x} {top} m {x} {top - 41 * row_height} l S\n"
    pages.append((*a4, body))
    # Pages 3-5: running text on US Letter.
    paragraph = (
        "MD View renders PDF pages off the UI thread and keeps textures only for pages near the "
        "viewport, so long documents stay light. "
    )
    for number in range(3, 6):
        body = "0 0 0 rg" + pdf_text(56, 740, 18, f"Section {number}", True)
        words, line, y = (paragraph * 12).split(), "", 710
        for word in words:
            if len(line) + len(word) > 88:
                body += pdf_text(56, y, 11, line)
                line, y = "", y - 15
            line = f"{line} {word}".strip()
        body += pdf_text(56, y, 11, line)
        body += pdf_text(290, 40, 9, f"Page {number}")
        pages.append((*letter, body))
    # Page 6: landscape.
    body = "0.035 0.41 0.85 rg 0 0 842 595 re f 1 1 1 rg" + pdf_text(260, 290, 32, "Landscape page", True)
    pages.append((842.0, 595.0, body))

    objects: list[bytes] = []
    def add(data: bytes) -> int:
        objects.append(data)
        return len(objects)
    font_regular = add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")
    font_bold = add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>")
    image = add(
        f"<< /Type /XObject /Subtype /Image /Width {W} /Height {H} /ColorSpace /DeviceRGB "
        f"/BitsPerComponent 8 /Filter /DCTDecode /Length {len(jpeg)} >>\nstream\n".encode() + jpeg + b"\nendstream"
    )
    pages_id = len(objects) + 1 + 2 * len(pages)
    kids = []
    for width, height, content in pages:
        stream = content.encode("latin-1")
        content_id = add(b"<< /Length %d >>\nstream\n" % len(stream) + stream + b"\nendstream")
        kids.append(add(
            f"<< /Type /Page /Parent {pages_id} 0 R /MediaBox [0 0 {width} {height}] /Contents {content_id} 0 R "
            f"/Resources << /Font << /F1 {font_regular} 0 R /F2 {font_bold} 0 R >> /XObject << /Im1 {image} 0 R >> >> >>"
            .encode()
        ))
    kid_refs = " ".join(f"{kid} 0 R" for kid in kids)
    assert add(f"<< /Type /Pages /Kids [{kid_refs}] /Count {len(kids)} >>".encode()) == pages_id
    catalog = add(f"<< /Type /Catalog /Pages {pages_id} 0 R >>".encode())
    output = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for number, data in enumerate(objects, start=1):
        offsets.append(len(output))
        output += f"{number} 0 obj\n".encode() + data + b"\nendobj\n"
    xref = len(output)
    output += f"xref\n0 {len(objects) + 1}\n0000000000 65535 f \n".encode()
    output += b"".join(f"{offset:010d} 00000 n \n".encode() for offset in offsets)
    output += f"trailer\n<< /Size {len(objects) + 1} /Root {catalog} 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    (out / "guide.pdf").write_bytes(bytes(output))


def videos() -> None:
    out = ROOT / "video"
    out.mkdir(parents=True, exist_ok=True)
    picture = ["-f", "lavfi", "-i", "testsrc2=size=480x270:rate=25:duration=4"]
    tone = ["-f", "lavfi", "-i", "sine=frequency=440:beep_factor=4:duration=4"]
    ffmpeg(*picture, *tone, "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "30", "-c:a", "aac",
           "-b:a", "64k", "-shortest", "-movflags", "+faststart", out / "clip.mp4")
    ffmpeg(*picture, *tone, "-c:v", "libvpx-vp9", "-b:v", "250k", "-c:a", "libopus", "-b:a", "48k",
           "-shortest", out / "clip.webm")
    ffmpeg("-f", "lavfi", "-i", "smptebars=size=480x270:rate=25:duration=3", "-c:v", "libx264",
           "-pix_fmt", "yuv420p", "-crf", "30", out / "silent.mkv")
    # Stored landscape with a rotation, the way phones record portrait video.
    landscape = out / "landscape.tmp.mov"
    ffmpeg("-f", "lavfi", "-i", "testsrc=size=480x270:rate=25:duration=3", "-c:v", "libx264",
           "-pix_fmt", "yuv420p", "-crf", "30", landscape)
    ffmpeg("-display_rotation", "90", "-i", landscape, "-c", "copy", out / "portrait.mov")
    landscape.unlink()


def tables() -> None:
    out = ROOT / "tables"
    out.mkdir(parents=True, exist_ok=True)
    (out / "people.csv").write_text(
        "id,name,city,notes,score\n"
        '1,Ada Lovelace,London,"Wrote the first program, ""Note G""",98.5\n'
        '2,José Núñez,Málaga,"Two lines:\nsecond line",87\n'
        "3,李小龍,香港,Unicode names and cities,91.25\n"
        '4,  Spaces  ,Oslo," leading and trailing ",70\n'
        "5,Empty notes,Lima,,64\n"
        "6,Grace Hopper,Arlington,Compilers,99\n",
        encoding="utf-8",
    )
    (out / "metrics.tsv").write_text(
        "metric\tp50\tp95\tunit\n"
        "frame time\t5.4\t9.8\tms\n"
        "startup\t180\t260\tms\n"
        "peak memory\t245\t310\tMiB\n",
        encoding="utf-8",
    )
    (out / "inventory.psv").write_text(
        "sku|product|stock|price\nA-100|Keyboard|42|39.90\nA-101|Mouse|17|19.50\nA-102|Monitor|5|229.00\n",
        encoding="utf-8",
    )
    (out / "european.csv").write_text(
        "fecha;producto;cantidad;importe\n2026-01-03;Café;12;3,50\n2026-01-04;Té verde;4;2,75\n"
        '2026-01-05;"Agua; con gas";30;1,10\n',
        encoding="utf-8",
    )
    random.seed(7)
    lines = ["row,group,value,ratio,label"]
    for index in range(1, 5001):
        lines.append(f"{index},G{index % 12},{random.randint(0, 99999)},{random.random():.4f},item-{index:05d}")
    (out / "large.csv").write_text("\r\n".join(lines) + "\r\n", encoding="utf-8")


def main() -> int:
    for program in ("ffmpeg",):
        if shutil.which(program) is None:
            print(f"{program} is required", file=sys.stderr)
            return 1
    with tempfile.TemporaryDirectory() as directory:
        tmp = Path(directory)
        images(tmp)
        animated_rasters()
        pdf(tmp)
    videos()
    tables()
    print(f"fixtures written to {ROOT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
