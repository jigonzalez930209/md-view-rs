//! Video probing and decoding through FFmpeg.
//!
//! MD View draws no video codec itself and a webview is ruled out, so `ffprobe` reads the
//! stream layout and `ffmpeg` decodes frames and sound into pipes. Packaged builds ship their
//! own LGPL FFmpeg (built by `scripts/ffmpeg/build.sh`) next to the executable; a build
//! without one falls back to the programs on the PATH. The programs run without a shell,
//! read only the given file, and write raw frames or PCM to a pipe.

use std::path::{Path, PathBuf};
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

/// Where packaged builds keep FFmpeg, in lookup order: an explicit `MDVIEW_FFMPEG_DIR`,
/// beside the executable (Windows, the macOS bundle), and `../lib/mdview` from it (the Linux
/// packages and the AppImage). The bare name, resolved through the PATH, comes last.
pub fn tool_candidates(name: &str, executable: Option<&Path>, override_dir: Option<&Path>) -> Vec<PathBuf> {
    let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    let mut candidates = Vec::new();
    if let Some(directory) = override_dir {
        candidates.push(directory.join(&file));
    }
    if let Some(directory) = executable.and_then(Path::parent) {
        candidates.push(directory.join(&file));
        candidates.push(directory.join("../lib/mdview").join(&file));
    }
    candidates.push(PathBuf::from(file));
    candidates
}

/// The FFmpeg program to run for `name` (`ffmpeg` or `ffprobe`): the bundled copy when there
/// is one, otherwise the name for a PATH lookup.
pub fn tool_path(name: &str) -> PathBuf {
    let executable = std::env::current_exe().ok();
    let override_dir = std::env::var_os("MDVIEW_FFMPEG_DIR").map(PathBuf::from);
    let mut candidates = tool_candidates(name, executable.as_deref(), override_dir.as_deref());
    let fallback = candidates.pop().unwrap_or_else(|| PathBuf::from(name));
    candidates.into_iter().find(|path| path.is_file()).unwrap_or(fallback)
}

/// A command for the bundled or system `ffmpeg`.
pub fn ffmpeg_command() -> Command {
    Command::new(tool_path("ffmpeg"))
}

/// Whether `ffmpeg` and `ffprobe` can be run.
pub fn ffmpeg_available() -> bool {
    ["ffmpeg", "ffprobe"].iter().all(|program| {
        Command::new(tool_path(program))
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
    let output = Command::new(tool_path("ffprobe"))
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

/// Arguments for `ffmpeg` that decode the first audio stream of `path` from `start` seconds
/// into interleaved signed 16-bit PCM at `rate` Hz with `channels` channels on stdout.
pub fn audio_args(path: &Path, start: f64, rate: u32, channels: u16) -> Vec<String> {
    let mut args: Vec<String> = ["-v", "error", "-nostdin", "-ss"].map(str::to_owned).to_vec();
    args.push(format!("{:.3}", start.max(0.0)));
    args.push("-i".to_owned());
    args.push(path.to_string_lossy().into_owned());
    args.extend(["-map", "0:a:0", "-vn", "-sn", "-ac"].map(str::to_owned));
    args.push(channels.max(1).to_string());
    args.push("-ar".to_owned());
    args.push(rate.max(8000).to_string());
    args.extend(["-f", "s16le", "-"].map(str::to_owned));
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
        let audio = audio_args(path, 1.5, 48_000, 2);
        assert!(
            audio.contains(&"/tmp/a b; rm -rf ~.mp4".to_owned()),
            "the path is one argument"
        );
        assert!(audio.windows(2).any(|pair| pair == ["-ar", "48000"]));
        assert!(audio.windows(2).any(|pair| pair == ["-ac", "2"]));
        assert!(audio.ends_with(&["-f".to_owned(), "s16le".to_owned(), "-".to_owned()]));
    }

    #[test]
    fn bundled_ffmpeg_is_found_before_the_path() {
        let exe = std::env::consts::EXE_SUFFIX;
        let executable = Path::new("/opt/mdview/usr/bin/mdview");
        let candidates = tool_candidates("ffprobe", Some(executable), Some(Path::new("/dev/ff")));
        assert_eq!(
            candidates,
            vec![
                PathBuf::from(format!("/dev/ff/ffprobe{exe}")),
                PathBuf::from(format!("/opt/mdview/usr/bin/ffprobe{exe}")),
                PathBuf::from(format!("/opt/mdview/usr/bin/../lib/mdview/ffprobe{exe}")),
                PathBuf::from(format!("ffprobe{exe}")),
            ]
        );
        assert_eq!(
            tool_candidates("ffmpeg", None, None),
            vec![PathBuf::from(format!("ffmpeg{exe}"))]
        );
    }

    #[test]
    fn real_files_probe_and_decode_when_ffmpeg_is_installed() {
        if !ffmpeg_available() {
            return;
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/v1-workspace/media/video/clip.mp4");
        let info = probe(&path).expect("probe");
        assert_eq!((info.width, info.height), (480, 270));
        assert!((info.fps - 25.0).abs() < 0.01);
        assert!(info.has_audio);
        let size = decode_size(&info, 160);
        let output = ffmpeg_command()
            .args(decode_args(&path, 0.5, size, Some(1)))
            .output()
            .expect("decode");
        assert_eq!(output.stdout.len(), (size.0 * size.1 * 4) as usize, "one RGBA frame");
        let audio = ffmpeg_command()
            .args(audio_args(&path, 3.0, 48_000, 2))
            .output()
            .expect("audio");
        let seconds = audio.stdout.len() as f64 / (48_000.0 * 2.0 * 2.0);
        assert!(
            (seconds - 1.0).abs() < 0.1,
            "one second of stereo PCM, got {seconds:.2} s"
        );
    }
}
