//! Video probing and decoding through the system FFmpeg.
//!
//! MD View draws no video codec itself: a pure-Rust decoder for H.264, HEVC, VP9, and AV1 does
//! not exist, and a webview is ruled out. When `ffprobe` and `ffmpeg` are on the PATH, every
//! container and codec they read plays; otherwise the viewer explains how to install them. The
//! programs run without a shell, read only the given file, and write raw frames to a pipe.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::MediaError;

/// Extensions the video viewer opens.
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "mov", "mkv", "webm", "avi", "wmv", "flv", "mpg", "mpeg", "m2ts", "mts", "ts", "3gp", "3g2", "ogv",
    "vob", "asf", "f4v",
];

/// Whether a path opens in the video viewer.
pub fn is_video_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| VIDEO_EXTENSIONS.contains(&extension.as_str()))
}

/// What `ffprobe` reports about a video file.
#[derive(Clone, Debug, PartialEq)]
pub struct VideoInfo {
    /// Frame width in pixels, after rotation metadata.
    pub width: u32,
    /// Frame height in pixels, after rotation metadata.
    pub height: u32,
    /// Duration in seconds, when the container knows it.
    pub duration: Option<f64>,
    /// Frames per second.
    pub fps: f64,
    /// Whether the file has an audio stream.
    pub has_audio: bool,
    /// Video codec name, for the status bar.
    pub codec: String,
}

/// Whether `ffmpeg` and `ffprobe` can be run.
pub fn ffmpeg_available() -> bool {
    ["ffmpeg", "ffprobe"].iter().all(|program| {
        Command::new(program)
            .arg("-version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    })
}

/// Probe a video file with `ffprobe`.
pub fn probe(path: &Path) -> Result<VideoInfo, MediaError> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_streams",
            "-show_format",
            "--",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| MediaError::Io(format!("Could not run ffprobe: {error}")))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(MediaError::Decode(if message.is_empty() {
            "ffprobe could not read the file.".to_owned()
        } else {
            message
        }));
    }
    parse_probe(&output.stdout)
}

/// Parse `ffprobe -print_format json -show_streams -show_format` output.
pub fn parse_probe(json: &[u8]) -> Result<VideoInfo, MediaError> {
    let value: serde_json::Value = serde_json::from_slice(json)
        .map_err(|error| MediaError::Decode(format!("Unreadable ffprobe output: {error}")))?;
    let streams = value["streams"].as_array().cloned().unwrap_or_default();
    let video = streams
        .iter()
        .find(|stream| stream["codec_type"] == "video" && stream["disposition"]["attached_pic"] != 1)
        .ok_or_else(|| MediaError::Decode("The file has no video stream.".to_owned()))?;
    let mut width = video["width"].as_u64().unwrap_or(0) as u32;
    let mut height = video["height"].as_u64().unwrap_or(0) as u32;
    if width == 0 || height == 0 {
        return Err(MediaError::Decode("The video stream has no size.".to_owned()));
    }
    // Phones store portrait video as landscape plus a rotation; ffmpeg applies it when decoding.
    let rotation = video["side_data_list"]
        .as_array()
        .and_then(|list| list.iter().find_map(|entry| entry["rotation"].as_f64()))
        .or_else(|| video["tags"]["rotate"].as_str().and_then(|text| text.parse().ok()))
        .unwrap_or(0.0);
    if (rotation.abs() - 90.0).abs() < 1.0 || (rotation.abs() - 270.0).abs() < 1.0 {
        std::mem::swap(&mut width, &mut height);
    }
    let fps = rate(video["avg_frame_rate"].as_str())
        .or_else(|| rate(video["r_frame_rate"].as_str()))
        .filter(|fps| (1.0..=240.0).contains(fps))
        .unwrap_or(30.0);
    let duration = value["format"]["duration"]
        .as_str()
        .or_else(|| video["duration"].as_str())
        .and_then(|text| text.parse::<f64>().ok())
        .filter(|duration| duration.is_finite() && *duration > 0.0);
    Ok(VideoInfo {
        width,
        height,
        duration,
        fps,
        has_audio: streams.iter().any(|stream| stream["codec_type"] == "audio"),
        codec: video["codec_name"].as_str().unwrap_or("video").to_owned(),
    })
}

fn rate(text: Option<&str>) -> Option<f64> {
    let (numerator, denominator) = text?.split_once('/')?;
    let numerator: f64 = numerator.parse().ok()?;
    let denominator: f64 = denominator.parse().ok()?;
    (denominator > 0.0).then(|| numerator / denominator)
}

/// The decode size for a video shown at most `max_width` pixels wide: the native size, or
/// smaller with the same aspect, rounded to even numbers as video scalers require.
pub fn decode_size(info: &VideoInfo, max_width: u32) -> (u32, u32) {
    let width = info.width.min(max_width.max(16));
    let height = (f64::from(info.height) * f64::from(width) / f64::from(info.width)).round() as u32;
    (width & !1, (height & !1).max(2))
}

/// Arguments for `ffmpeg` that decode `path` from `start` seconds into raw RGBA frames of
/// `size` on stdout; `frames` limits the count (one for a still while paused).
pub fn decode_args(path: &Path, start: f64, size: (u32, u32), frames: Option<u32>) -> Vec<String> {
    let mut args: Vec<String> = ["-v", "error", "-nostdin", "-ss"]
        .iter()
        .map(|arg| (*arg).to_owned())
        .collect();
    args.push(format!("{:.3}", start.max(0.0)));
    args.push("-i".to_owned());
    args.push(path.to_string_lossy().into_owned());
    args.extend(["-an", "-sn", "-vf"].map(str::to_owned));
    args.push(format!("scale={}:{}:flags=bilinear", size.0, size.1));
    if let Some(frames) = frames {
        args.push("-frames:v".to_owned());
        args.push(frames.to_string());
    }
    args.extend(["-f", "rawvideo", "-pix_fmt", "rgba", "-"].map(str::to_owned));
    args
}

/// Arguments for `ffplay` that play only the audio of `path` from `start`, with no window.
pub fn audio_args(path: &Path, start: f64) -> Vec<String> {
    let mut args: Vec<String> = ["-nodisp", "-autoexit", "-loglevel", "quiet", "-ss"]
        .map(str::to_owned)
        .to_vec();
    args.push(format!("{:.3}", start.max(0.0)));
    args.push(path.to_string_lossy().into_owned());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROBE: &str = r#"{
      "streams": [
        {"codec_type": "video", "codec_name": "h264", "width": 1920, "height": 1080,
         "avg_frame_rate": "30000/1001", "r_frame_rate": "30000/1001", "disposition": {"attached_pic": 0},
         "side_data_list": [{"rotation": -90}]},
        {"codec_type": "audio", "codec_name": "aac"}
      ],
      "format": {"duration": "12.500000"}
    }"#;

    #[test]
    fn probe_output_is_parsed_with_rotation() {
        let info = parse_probe(PROBE.as_bytes()).expect("parse");
        assert_eq!((info.width, info.height), (1080, 1920), "rotated to portrait");
        assert!((info.fps - 29.97).abs() < 0.01);
        assert_eq!(info.duration, Some(12.5));
        assert!(info.has_audio);
        assert_eq!(info.codec, "h264");
    }

    #[test]
    fn audio_only_or_broken_probes_are_errors() {
        assert!(parse_probe(br#"{"streams":[{"codec_type":"audio"}],"format":{}}"#).is_err());
        assert!(parse_probe(b"not json").is_err());
    }

    #[test]
    fn decode_sizes_keep_aspect_and_are_even() {
        let info = parse_probe(PROBE.as_bytes()).expect("parse");
        assert_eq!(decode_size(&info, 4000), (1080, 1920));
        let (width, height) = decode_size(&info, 541);
        assert_eq!(width % 2, 0);
        assert_eq!(height % 2, 0);
        assert!((f64::from(height) / f64::from(width) - 1920.0 / 1080.0).abs() < 0.01);
    }

    #[test]
    fn arguments_never_go_through_a_shell() {
        let path = Path::new("/tmp/a b; rm -rf ~.mp4");
        let args = decode_args(path, 3.25, (640, 360), Some(1));
        assert!(
            args.contains(&"/tmp/a b; rm -rf ~.mp4".to_owned()),
            "the path is one argument"
        );
        assert!(args.windows(2).any(|pair| pair == ["-ss", "3.250"]));
        assert!(args.windows(2).any(|pair| pair == ["-frames:v", "1"]));
        assert_eq!(
            audio_args(path, 0.0).last().map(String::as_str),
            Some("/tmp/a b; rm -rf ~.mp4")
        );
    }

    #[test]
    fn real_files_probe_and_decode_when_ffmpeg_is_installed() {
        if !ffmpeg_available() {
            return;
        }
        let directory = std::env::temp_dir().join(format!("mdview-video-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("temp dir");
        let path = directory.join("clip.mp4");
        let made = Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=320x240:rate=25:duration=1",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&path)
            .status()
            .is_ok_and(|status| status.success());
        assert!(made, "ffmpeg made a test clip");
        let info = probe(&path).expect("probe");
        assert_eq!((info.width, info.height), (320, 240));
        assert!((info.fps - 25.0).abs() < 0.01);
        let size = decode_size(&info, 160);
        let output = Command::new("ffmpeg")
            .args(decode_args(&path, 0.5, size, Some(1)))
            .output()
            .expect("decode");
        assert_eq!(output.stdout.len(), (size.0 * size.1 * 4) as usize, "one RGBA frame");
        let _ = std::fs::remove_dir_all(&directory);
    }
}
