//! Local and remote image loading plus raster decoding.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::{MAX_IMAGE_SIDE, MAX_SOURCE_BYTES, MediaError, MediaRequest, Raster, svg::rasterize_svg};

/// Read the bytes for an image/SVG request from a local path or an HTTP(S) URL.
pub(crate) fn read_source(request: &MediaRequest) -> Result<Vec<u8>, MediaError> {
    let source = request.source.trim();
    if source.is_empty() {
        return Err(MediaError::Empty);
    }
    let lower = source.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return read_remote(request, source);
    }
    if lower.starts_with("data:") {
        return Err(MediaError::RefusedPath);
    }
    let document = request.base.as_ref().map(|directory| directory.join("document.md"));
    let path = craft_docpath::resolve(source, document.as_deref()).map_err(|_| MediaError::RefusedPath)?;
    read_local(&path)
}

fn read_local(path: &Path) -> Result<Vec<u8>, MediaError> {
    let metadata = std::fs::metadata(path).map_err(|error| MediaError::Io(error.to_string()))?;
    if !metadata.is_file() {
        return Err(MediaError::RefusedPath);
    }
    if metadata.len() > MAX_SOURCE_BYTES as u64 {
        return Err(MediaError::TooLarge);
    }
    let file = std::fs::File::open(path).map_err(|error| MediaError::Io(error.to_string()))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_SOURCE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| MediaError::Io(error.to_string()))?;
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(MediaError::TooLarge);
    }
    Ok(bytes)
}

fn read_remote(request: &MediaRequest, url: &str) -> Result<Vec<u8>, MediaError> {
    if !request.allow_remote {
        return Err(MediaError::RemoteDisabled);
    }
    if !request.allow_private_networks && is_private_url(url) {
        return Err(MediaError::PrivateNetwork);
    }
    let cache = DiskCache::new(request.cache_dir.as_deref(), url);
    let cached = cache.read();
    let agent = ureq::config::Config::builder()
        .timeout_global(Some(Duration::from_secs(12)))
        .http_status_as_error(false)
        .build()
        .new_agent();
    let mut last_error = String::new();
    for attempt in 0..2 {
        let mut builder = agent.get(url).header("User-Agent", "MD-View/0.6");
        if let Some((_, etag)) = &cached
            && let Some(etag) = etag
        {
            builder = builder.header("If-None-Match", etag);
        }
        let response = match builder.call() {
            Ok(response) => response,
            Err(error) => {
                last_error = error.to_string();
                if attempt == 0 {
                    continue;
                }
                return Err(MediaError::Network(last_error));
            }
        };
        let status = response.status().as_u16();
        if status == 304
            && let Some((bytes, _)) = &cached
        {
            return Ok(bytes.clone());
        }
        if !(200..300).contains(&status) {
            last_error = format!("HTTP {status}");
            if status >= 500 && attempt == 0 {
                continue;
            }
            return Err(MediaError::Network(last_error));
        }
        let etag = response
            .headers()
            .get("etag")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let mut body = response;
        let bytes = body
            .body_mut()
            .with_config()
            .limit(MAX_SOURCE_BYTES as u64)
            .read_to_vec()
            .map_err(|error| MediaError::Network(error.to_string()))?;
        if bytes.is_empty() {
            return Err(MediaError::EmptyImage);
        }
        cache.write(&bytes, etag.as_deref());
        return Ok(bytes);
    }
    Err(MediaError::Network(last_error))
}

fn is_private_url(url: &str) -> bool {
    let Some(rest) = url.split_once("://").map(|(_, rest)| rest) else {
        return true;
    };
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(rest)
        .rsplit('@')
        .next()
        .unwrap_or(rest);
    let host = if let Some(stripped) = authority.strip_prefix('[') {
        stripped.split(']').next().unwrap_or(stripped)
    } else {
        authority.split(':').next().unwrap_or(authority)
    };
    let host = host.to_ascii_lowercase();
    if host.is_empty() {
        return true;
    }
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") {
        return true;
    }
    if host == "::1" || host.starts_with("fe80:") || host.starts_with("fc") || host.starts_with("fd") {
        return true;
    }
    let octets = host.split('.').map(str::parse::<u8>).collect::<Result<Vec<_>, _>>();
    let Ok(octets) = octets else {
        return false;
    };
    match octets.as_slice() {
        [127, ..] | [10, ..] | [0, ..] | [169, 254, ..] | [192, 168, ..] => true,
        [172, second, ..] if (16..=31).contains(second) => true,
        [100, second, ..] if (64..=127).contains(second) => true,
        _ => false,
    }
}

/// Decode image bytes or rasterize SVG bytes according to the request.
pub fn load_bytes(bytes: &[u8], request: &MediaRequest) -> Result<Raster, MediaError> {
    if bytes.is_empty() {
        return Err(MediaError::Empty);
    }
    if looks_like_svg(bytes) {
        return rasterize_svg(bytes, request);
    }
    let format = image::guess_format(bytes).map_err(|error| MediaError::Decode(error.to_string()))?;
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_SIDE);
    limits.max_image_height = Some(MAX_IMAGE_SIDE);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|error| MediaError::Decode(error.to_string()))?;
    let max_width = request.max_width.max(1);
    let max_height = request.max_height.max(1);
    let scaled = if decoded.width() > max_width || decoded.height() > max_height {
        decoded.thumbnail(max_width, max_height)
    } else {
        decoded
    };
    let rgba = scaled.to_rgba8();
    Ok(Raster {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(512)];
    let text = String::from_utf8_lossy(head).trim_start().to_ascii_lowercase();
    text.starts_with("<svg")
        || text.starts_with("<?xml")
        || text.starts_with("<!doctype svg")
        || text.starts_with("<svg:")
}

struct DiskCache {
    directory: Option<PathBuf>,
    key: String,
}

impl DiskCache {
    fn new(directory: Option<&Path>, url: &str) -> Self {
        let key = blake3::hash(url.as_bytes()).to_hex().to_string();
        Self {
            directory: directory.map(Path::to_path_buf),
            key,
        }
    }

    fn paths(&self) -> Option<(PathBuf, PathBuf)> {
        let directory = self.directory.as_ref()?;
        Some((
            directory.join(format!("{}.bin", self.key)),
            directory.join(format!("{}.etag", self.key)),
        ))
    }

    fn read(&self) -> Option<(Vec<u8>, Option<String>)> {
        let (bytes_path, etag_path) = self.paths()?;
        let bytes = std::fs::read(&bytes_path).ok()?;
        if bytes.len() > MAX_SOURCE_BYTES {
            return None;
        }
        let etag = std::fs::read_to_string(&etag_path).ok();
        Some((bytes, etag))
    }

    fn write(&self, bytes: &[u8], etag: Option<&str>) {
        let Some((bytes_path, etag_path)) = self.paths() else {
            return;
        };
        let Some(parent) = bytes_path.parent() else {
            return;
        };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        let _ = write_atomic(&bytes_path, bytes);
        if let Some(etag) = etag {
            let _ = write_atomic(&etag_path, etag.as_bytes());
        }
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = path.with_extension("tmp");
    {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MediaKind;

    #[test]
    fn private_network_detection() {
        for url in [
            "http://localhost/a.png",
            "http://127.0.0.1/a.png",
            "http://10.1.2.3/a.png",
            "http://192.168.1.1/a.png",
            "http://172.20.0.1/a.png",
            "http://[::1]/a.png",
            "http://printer.local/a.png",
        ] {
            assert!(is_private_url(url), "{url}");
        }
        for url in [
            "https://example.com/a.png",
            "https://8.8.8.8/a.png",
            "https://raw.githubusercontent.com/a.png",
        ] {
            assert!(!is_private_url(url), "{url}");
        }
    }

    #[test]
    fn local_images_decode() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("pixel.png");
        let image = image::RgbaImage::from_pixel(3, 2, image::Rgba([10, 20, 30, 255]));
        image.save(&path).expect("save png");
        let mut request = MediaRequest::new(MediaKind::Image, "pixel.png");
        request.base = Some(directory.path().to_path_buf());
        let raster = request.load().expect("load");
        assert_eq!((raster.width, raster.height), (3, 2));
        assert!(raster.is_consistent());
    }

    #[test]
    fn remote_images_cache_and_revalidate() {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpListener;

        let directory = tempfile::tempdir().expect("tempdir");
        let mut png = std::io::Cursor::new(Vec::new());
        let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255]));
        image.write_to(&mut png, image::ImageFormat::Png).expect("encode png");
        let png = png.into_inner();
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("address");
        let server = std::thread::spawn(move || {
            for _ in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut reader = BufReader::new(stream.try_clone().expect("clone"));
                let mut request = String::new();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        break;
                    }
                    request.push_str(&line);
                    if line == "\r\n" {
                        break;
                    }
                }
                if request.contains("If-None-Match: \"test-etag\"") {
                    let _ = stream.write_all(b"HTTP/1.1 304 Not Modified\r\nETag: \"test-etag\"\r\n\r\n");
                } else {
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nETag: \"test-etag\"\r\n\r\n",
                        png.len()
                    );
                    let _ = stream.write_all(header.as_bytes());
                    let _ = stream.write_all(&png);
                }
                let _ = stream.flush();
            }
        });
        let mut request = MediaRequest::new(MediaKind::Image, format!("http://{address}/pixel.png"));
        request.allow_private_networks = true;
        request.cache_dir = Some(directory.path().to_path_buf());
        let first = request.load().expect("first load");
        assert_eq!((first.width, first.height), (2, 2));
        let second = request.load().expect("cached load");
        assert_eq!((second.width, second.height), (2, 2));
        server.join().expect("server");

        let mut blocked = MediaRequest::new(MediaKind::Image, format!("http://{address}/pixel.png"));
        assert_eq!(blocked.load(), Err(MediaError::PrivateNetwork));
        blocked.allow_private_networks = true;
        blocked.allow_remote = false;
        assert_eq!(blocked.load(), Err(MediaError::RemoteDisabled));
    }

    #[test]
    fn missing_files_and_network_paths_are_errors() {
        let request = MediaRequest::new(MediaKind::Image, "\\\\server\\share\\a.png");
        assert_eq!(request.load(), Err(MediaError::RefusedPath));
        let mut request = MediaRequest::new(MediaKind::Image, "https://example.com/a.png");
        request.allow_remote = false;
        assert_eq!(request.load(), Err(MediaError::RemoteDisabled));
    }
}
