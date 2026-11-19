//! Container demuxing: one track of a video file as timed packets.
//!
//! The readers keep the file open and read one packet at a time, so a long video is never
//! loaded into memory; AVI, MPEG transport/program streams, and FLV are indexed when opened.
//! Times are seconds on the presentation timeline: MP4 edit lists and composition offsets are
//! applied, and the other containers start where their first packet does.

mod avi;
mod flv;
mod mkv;
mod mp4;
mod mpegts;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

use super::track::{Kind, Packet, TrackInfo};
use crate::MediaError;

/// Packets of one track.
pub(crate) enum Demuxer {
    Mp4(mp4::Mp4Track),
    Mkv(Box<mkv::MkvTrack>),
    Avi(avi::AviTrack),
    MpegTs(Box<mpegts::TsTrack>),
    Flv(flv::FlvTrack),
}

/// The container formats, told apart by their first bytes.
#[derive(Debug, PartialEq)]
enum Container {
    Mp4,
    Mkv,
    Avi,
    MpegTs,
    Flv,
    Asf,
    Ogg,
    Unknown,
}

fn sniff(head: &[u8]) -> Container {
    let box_type = head.get(4..8).unwrap_or_default();
    if head.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        Container::Mkv
    } else if [&b"ftyp"[..], b"moov", b"mdat", b"wide", b"free", b"skip"].contains(&box_type) {
        Container::Mp4
    } else if head.starts_with(b"RIFF") && head.get(8..11) == Some(b"AVI") {
        Container::Avi
    } else if head.starts_with(b"FLV\x01") {
        Container::Flv
    } else if head.starts_with(&[0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11]) {
        Container::Asf
    } else if head.starts_with(b"OggS") {
        Container::Ogg
    } else if filmcraft_mpegts::sniff(head).is_some() {
        Container::MpegTs
    } else {
        Container::Unknown
    }
}

impl Demuxer {
    /// Open the first track of `kind`; `Ok(None)` when the file has none.
    pub(crate) fn open(path: &Path, kind: Kind) -> Result<Option<(Self, TrackInfo)>, MediaError> {
        let mut file = File::open(path).map_err(io_error)?;
        let mut head = Vec::with_capacity(64 * 1024);
        file.by_ref().take(64 * 1024).read_to_end(&mut head).map_err(io_error)?;
        if head.len() < 8 {
            return Err(MediaError::Decode("The file is too short to be a video.".to_owned()));
        }
        file.seek(SeekFrom::Start(0)).map_err(io_error)?;
        Ok(match sniff(&head) {
            Container::Mp4 => mp4::Mp4Track::open(file, kind)?.map(|(track, info)| (Self::Mp4(track), info)),
            Container::Mkv => mkv::MkvTrack::open(file, kind)?.map(|(track, info)| (Self::Mkv(Box::new(track)), info)),
            Container::Avi => {
                avi::AviTrack::open(FileSource::new(file)?, kind)?.map(|(track, info)| (Self::Avi(track), info))
            }
            Container::MpegTs => mpegts::TsTrack::open(FileSource::new(file)?, kind)?
                .map(|(track, info)| (Self::MpegTs(Box::new(track)), info)),
            Container::Flv => flv::FlvTrack::open(file, kind)?.map(|(track, info)| (Self::Flv(track), info)),
            Container::Asf => {
                return Err(MediaError::Decode(
                    "Windows Media (WMV/ASF) files cannot be played: there is no pure-Rust Windows Media Video decoder yet."
                        .to_owned(),
                ));
            }
            Container::Ogg => {
                return Err(MediaError::Decode(
                    "Ogg video (OGV/Theora) is not supported yet. MD View plays MP4, MOV, MKV, WebM, AVI, MPEG-TS, MPEG-PS, and FLV files."
                        .to_owned(),
                ));
            }
            Container::Unknown => {
                return Err(MediaError::Decode(
                    "MD View plays MP4, MOV, MKV, WebM, AVI, MPEG-TS, MPEG-PS, and FLV files; this container is not supported yet."
                        .to_owned(),
                ));
            }
        })
    }

    pub(crate) fn next(&mut self) -> Result<Option<Packet>, MediaError> {
        match self {
            Self::Mp4(track) => track.next(),
            Self::Mkv(track) => track.next(),
            Self::Avi(track) => track.next(),
            Self::MpegTs(track) => track.next(),
            Self::Flv(track) => track.next(),
        }
    }

    /// Position the track on the last key frame at or before `at` seconds.
    pub(crate) fn seek(&mut self, at: f64) -> Result<(), MediaError> {
        match self {
            Self::Mp4(track) => track.seek(at),
            Self::Mkv(track) => return track.seek(at),
            Self::Avi(track) => track.seek(at),
            Self::MpegTs(track) => track.seek(at),
            Self::Flv(track) => track.seek(at),
        }
        Ok(())
    }
}

pub(crate) fn io_error(error: std::io::Error) -> MediaError {
    MediaError::Io(error.to_string())
}

/// Index of the last entry at or before `at` that decoding can start from, given each
/// entry's time and whether it is a key frame; the first entry when there is none.
fn last_key_at(entries: impl DoubleEndedIterator<Item = (f64, bool)> + ExactSizeIterator, at: f64) -> usize {
    let mut entries = entries;
    entries.rposition(|(time, key)| key && time <= at + 1e-6).unwrap_or(0)
}

/// A file read at any offset, shared by the indexing demuxers.
pub(crate) struct FileSource {
    file: Mutex<File>,
    len: u64,
}

impl FileSource {
    fn new(file: File) -> Result<Self, MediaError> {
        let len = file.metadata().map_err(io_error)?.len();
        Ok(Self {
            file: Mutex::new(file),
            len,
        })
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
        let mut file = self
            .file
            .lock()
            .map_err(|_| std::io::Error::other("the file reader failed earlier"))?;
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(buf)
    }
}

impl filmcraft_avi::ByteSource for FileSource {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
        FileSource::read_at(self, offset, buf)
    }
}

impl filmcraft_mpegts::ByteSource for FileSource {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
        FileSource::read_at(self, offset, buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn containers_are_told_apart_by_their_first_bytes() {
        assert_eq!(sniff(b"\x1A\x45\xDF\xA3\0\0\0\0"), Container::Mkv);
        assert_eq!(sniff(b"\0\0\0\x20ftypisom"), Container::Mp4);
        assert_eq!(sniff(b"RIFF\0\0\0\0AVI LIST"), Container::Avi);
        assert_eq!(sniff(b"RIFF\0\0\0\0WAVEfmt "), Container::Unknown);
        assert_eq!(sniff(b"FLV\x01\x05\0\0\0\x09"), Container::Flv);
        assert_eq!(sniff(b"OggS\0\x02\0\0"), Container::Ogg);
        let mut ts = vec![0u8; 188 * 6];
        for packet in ts.chunks_mut(188) {
            packet[0] = 0x47;
        }
        assert_eq!(sniff(&ts), Container::MpegTs);
    }

    #[test]
    fn seeking_starts_from_the_last_key_frame_before_the_target() {
        let entries = [(0.0, true), (1.0, false), (2.0, true), (3.0, false)];
        assert_eq!(last_key_at(entries.iter().copied(), 2.5), 2);
        assert_eq!(last_key_at(entries.iter().copied(), 1.9), 0);
        assert_eq!(last_key_at(entries.iter().copied(), -1.0), 0);
    }
}
