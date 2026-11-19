use std::path::PathBuf;

use super::*;

fn clip(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/v1-workspace/media/video")
        .join(name)
}

fn info(width: u32, height: u32) -> VideoInfo {
    VideoInfo {
        width,
        height,
        duration: None,
        fps: 30.0,
        has_audio: false,
        codec: "h264".to_owned(),
    }
}

#[test]
fn decode_sizes_keep_aspect_and_are_even() {
    let portrait = info(1080, 1920);
    assert_eq!(decode_size(&portrait, 4000), (1080, 1920));
    let (width, height) = decode_size(&portrait, 541);
    assert_eq!((width % 2, height % 2), (0, 0));
    assert!((f64::from(height) / f64::from(width) - 1920.0 / 1080.0).abs() < 0.01);
}

#[test]
fn fixtures_probe_with_their_codecs_sizes_and_sound() {
    let cases = [
        ("clip.mp4", "h264", (480, 270), true),
        ("clip.webm", "vp9", (480, 270), true),
        ("portrait.mov", "h264", (270, 480), false),
        ("silent.mkv", "h264", (480, 270), false),
    ];
    for (name, codec, size, has_audio) in cases {
        let info = probe(&clip(name)).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(info.codec, codec, "{name}");
        assert_eq!((info.width, info.height), size, "{name}: rotation applied");
        assert_eq!(info.has_audio, has_audio, "{name}");
        assert!((info.fps - 25.0).abs() < 0.5, "{name}: {} fps", info.fps);
        let duration = info.duration.unwrap_or_default();
        assert!(duration > 1.0 && duration < 10.0, "{name}: {duration} s");
    }
}

/// Mean of the RGB channels, to tell a decoded picture from a blank buffer.
fn brightness(frame: &VideoFrame) -> f64 {
    let sum: u64 = frame
        .rgba
        .chunks_exact(4)
        .map(|pixel| u64::from(pixel[0]) + u64::from(pixel[1]) + u64::from(pixel[2]))
        .sum();
    sum as f64 / (frame.rgba.len() / 4 * 3) as f64
}

#[test]
fn every_fixture_decodes_frames_in_order_and_seeks() {
    for name in ["clip.mp4", "clip.webm", "portrait.mov", "silent.mkv"] {
        let (mut reader, info) = VideoReader::open(&clip(name)).unwrap_or_else(|error| panic!("{name}: {error}"));
        let size = decode_size(&info, 160);
        let mut last = -1.0;
        for _ in 0..10 {
            let frame = reader.next_frame(size).expect("decode").expect("a frame");
            assert_eq!(frame.rgba.len(), (size.0 * size.1 * 4) as usize, "{name}");
            assert!(frame.at > last, "{name}: frames come in display order");
            assert!(brightness(&frame) > 5.0, "{name}: the picture is not blank");
            last = frame.at;
        }
        assert!(last.abs() < 0.5, "{name}: the first frames start near zero");
        reader.seek(2.0).expect("seek");
        let frame = reader.next_frame(size).expect("decode").expect("a frame");
        assert!((frame.at - 2.0).abs() < 0.05, "{name}: seek landed at {}", frame.at);
        let mut count = 1;
        while reader.next_frame(size).expect("decode").is_some() {
            count += 1;
        }
        let expected = (info.duration.unwrap_or_default() - 2.0) * info.fps;
        assert!(
            (f64::from(count) - expected).abs() <= 1.0,
            "{name}: {count} frames after the seek"
        );
    }
}

#[test]
fn a_seek_past_the_end_shows_the_last_frame() {
    for name in ["clip.mp4", "clip.webm"] {
        let (mut reader, info) = VideoReader::open(&clip(name)).expect("open");
        let duration = info.duration.unwrap_or_default();
        reader.seek(duration + 5.0).expect("seek");
        let frame = reader.next_frame((64, 36)).expect("decode").expect("the last frame");
        assert!(duration - frame.at < 0.1, "{name}: {} of {duration}", frame.at);
        assert!(
            reader.next_frame((64, 36)).expect("decode").is_none(),
            "{name}: then the end"
        );
    }
}

#[test]
fn soundtracks_decode_to_device_pcm_from_any_start() {
    for name in ["clip.mp4", "clip.webm"] {
        let mut reader = AudioReader::open(&clip(name), 1.0, 48_000, 2)
            .expect("open")
            .expect("a soundtrack");
        let mut samples = Vec::new();
        while let Some(chunk) = reader.next_chunk().expect("decode") {
            samples.extend(chunk);
        }
        let seconds = samples.len() as f64 / (48_000.0 * 2.0);
        assert!(
            (seconds - 3.0).abs() < 0.1,
            "{name}: {seconds:.3} s after one second of four"
        );
        let peak = samples.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert!(peak > 0.05 && peak <= 1.0, "{name}: peak {peak}");
    }
    assert!(
        AudioReader::open(&clip("silent.mkv"), 0.0, 48_000, 2)
            .expect("open")
            .is_none()
    );
}

#[test]
fn unsupported_containers_and_garbage_are_errors() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("clip.wmv");
    let cases: [(&[u8], &str); 4] = [
        (
            &[0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9],
            "Windows Media",
        ),
        (b"OggS\0\x02\0\0\0\0\0\0", "Ogg video"),
        (b"\x89PNG\r\n\x1a\n\0\0", "this container is not supported"),
        (b"RIFF\0\0\0\0AVI LIST", "AVI"),
    ];
    for (bytes, message) in cases {
        std::fs::write(&path, bytes).expect("write");
        let error = probe(&path).expect_err("not playable");
        assert!(error.to_string().contains(message), "{error}");
    }
    std::fs::write(&path, b"").expect("write");
    assert!(probe(&path).is_err());
}

/// One clip per further container and codec: name, codec name, and size.
const FORMATS: [(&str, &str, (u32, u32)); 10] = [
    ("hevc.mp4", "hevc", (320, 180)),
    ("av1.mkv", "av1", (320, 180)),
    ("vp8.webm", "vp8", (320, 180)),
    ("xvid.avi", "mpeg4", (320, 180)),
    ("mjpeg.avi", "mjpeg", (320, 180)),
    ("h264.ts", "h264", (320, 180)),
    ("mpeg2.ts", "mpeg", (320, 180)),
    ("mpeg1.mpg", "mpeg", (320, 180)),
    ("h264.flv", "h264", (320, 180)),
    ("h263.3gp", "h263", (176, 144)),
];

#[test]
fn every_format_probes_decodes_and_plays_its_soundtrack() {
    for (name, codec, size) in FORMATS {
        let path = clip(&format!("formats/{name}"));
        let (mut reader, info) = VideoReader::open(&path).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            (info.codec.as_str(), (info.width, info.height)),
            (codec, size),
            "{name}"
        );
        assert!(info.has_audio, "{name}: the soundtrack is playable");
        assert!((info.fps - 25.0).abs() < 0.5, "{name}: {} fps", info.fps);
        let duration = info.duration.unwrap_or_default();
        assert!((duration - 2.0).abs() < 0.1, "{name}: {duration} s");
        let mut last = -1.0;
        let mut count = 0;
        while let Some(frame) = reader.next_frame((64, 36)).expect("decode") {
            assert!(frame.at > last, "{name}: frames come in display order");
            assert!(count > 0 || frame.at < 0.1, "{name}: starts at {}", frame.at);
            assert!(brightness(&frame) > 5.0, "{name}: the picture is not blank");
            (last, count) = (frame.at, count + 1);
        }
        assert_eq!(count, 50, "{name}: two seconds at 25 fps");
        let mut audio = AudioReader::open(&path, 0.5, 48_000, 2)
            .expect("open")
            .expect("a soundtrack");
        let mut samples = Vec::new();
        while let Some(chunk) = audio.next_chunk().expect("decode") {
            samples.extend(chunk);
        }
        let seconds = samples.len() as f64 / 96_000.0;
        assert!(
            (1.4..1.65).contains(&seconds),
            "{name}: {seconds:.3} s after half a second of two"
        );
        let peak = samples.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert!(peak > 0.05 && peak <= 1.0, "{name}: peak {peak}");
    }
}

#[test]
fn frames_after_a_seek_match_the_same_frames_from_the_start() {
    let names = ["clip.mp4", "clip.webm", "silent.mkv"]
        .into_iter()
        .map(str::to_owned)
        .chain(FORMATS.iter().map(|(name, ..)| format!("formats/{name}")));
    for name in names {
        let (mut reader, info) = VideoReader::open(&clip(&name)).expect("open");
        let size = decode_size(&info, 48);
        let mut all = Vec::new();
        while let Some(frame) = reader.next_frame(size).expect("decode") {
            all.push(frame);
        }
        let target = all[all.len() / 2].at;
        reader.seek(target).expect("seek");
        let mut after = 0;
        while let Some(frame) = reader.next_frame(size).expect("decode") {
            let same = all
                .iter()
                .find(|known| (known.at - frame.at).abs() < 1e-3)
                .unwrap_or_else(|| panic!("{name}: no frame at {:.3} from the start", frame.at));
            assert!(
                same.rgba == frame.rgba,
                "{name}: the frame at {:.3} differs after a seek",
                frame.at
            );
            after += 1;
        }
        assert_eq!(
            after,
            all.len() - all.len() / 2,
            "{name}: every frame from {target:.3} on"
        );
    }
}

#[test]
fn a_panicking_decoder_becomes_an_error() {
    let error = guarded("video decoder", || -> u32 { panic!("damaged data") }).expect_err("caught");
    assert!(error.to_string().contains("video decoder failed"), "{error}");
    assert_eq!(guarded("video decoder", || 7).expect("no panic"), 7);
}

#[test]
fn damaged_files_end_in_errors_not_panics() {
    // Every sample cut short and with bytes flipped in the middle: opening and playing may
    // fail, but must not panic.
    let directory = tempfile::tempdir().expect("tempdir");
    for (name, ..) in FORMATS {
        let mut bytes = std::fs::read(clip(&format!("formats/{name}"))).expect("read");
        bytes.truncate(bytes.len() * 2 / 3);
        let middle = bytes.len() / 2;
        for byte in &mut bytes[middle..middle + 4096] {
            *byte ^= 0x5A;
        }
        let path = directory.path().join(name);
        std::fs::write(&path, &bytes).expect("write");
        if let Ok((mut reader, info)) = VideoReader::open(&path) {
            let size = decode_size(&info, 32);
            while let Ok(Some(_)) = reader.next_frame(size) {}
        }
        if let Ok(Some(mut audio)) = AudioReader::open(&path, 0.0, 48_000, 2) {
            while let Ok(Some(_)) = audio.next_chunk() {}
        }
    }
}
