#!/usr/bin/env bash
# Build the FFmpeg that MD View ships for its video viewer.
#
# Usage: scripts/ffmpeg/build.sh <target> <outdir>
#   target: linux-x86_64 | macos-arm64 | windows-x86_64 (cross-compiled with mingw-w64), or
#   `sources` to only download and verify the source tarballs into <outdir> (the release
#   publishes them next to the packages, as the LGPL asks of anyone who ships the binaries).
#
# The build is LGPL-2.1-or-later: no GPL or nonfree component, no encoder beyond the raw
# outputs the viewer reads, no network, no hardware or system autodetection. It contains
# only what playback needs: the common demuxers, video and audio decoders (AV1 through
# dav1d), scaling and resampling, and the rotation filters. Sources are pinned by SHA-256.
# The output directory receives `ffmpeg` and `ffprobe` (`.exe` on Windows) plus the licenses
# and a BUILDINFO file recording the exact sources and configure line.
#
# Needs: a C compiler, make, nasm (x86_64), meson, ninja, pkg-config, curl, and for
# windows-x86_64 the x86_64-w64-mingw32 toolchain. MDVIEW_FFMPEG_CACHE (default
# target/ffmpeg) holds the downloads and the build trees.
set -euo pipefail

TARGET=${1:?target: linux-x86_64 | macos-arm64 | windows-x86_64 | sources}
OUT=${2:?output directory}

FFMPEG_VERSION=9.0.2
FFMPEG_SHA256=8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e
DAV1D_VERSION=1.5.5
DAV1D_SHA256=ca96b1d0ba3f01f55de7231d775892b64994ba061aaeeaaa4f662a390b3abf9e

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
CACHE=${MDVIEW_FFMPEG_CACHE:-$ROOT/target/ffmpeg}
WORK="$CACHE/$TARGET"
PREFIX="$WORK/prefix"
JOBS=${JOBS:-$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)}
mkdir -p "$CACHE/src" "$WORK" "$OUT"
OUT=$(cd "$OUT" && pwd)

fetch() { # url file sha256
    local file="$CACHE/src/$2"
    if [[ ! -f "$file" ]]; then
        curl --fail --location --silent --show-error -o "$file.part" "$1"
        mv "$file.part" "$file"
    fi
    local actual
    actual=$( (sha256sum "$file" 2>/dev/null || shasum -a 256 "$file") | cut -d' ' -f1)
    if [[ "$actual" != "$3" ]]; then
        echo "checksum mismatch for $2: $actual" >&2
        rm -f "$file"
        exit 1
    fi
}

fetch "https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VERSION.tar.xz" "ffmpeg-$FFMPEG_VERSION.tar.xz" "$FFMPEG_SHA256"
fetch "https://downloads.videolan.org/pub/videolan/dav1d/$DAV1D_VERSION/dav1d-$DAV1D_VERSION.tar.xz" \
    "dav1d-$DAV1D_VERSION.tar.xz" "$DAV1D_SHA256"

if [[ "$TARGET" == sources ]]; then
    cp "$CACHE/src/ffmpeg-$FFMPEG_VERSION.tar.xz" "$CACHE/src/dav1d-$DAV1D_VERSION.tar.xz" "$OUT/"
    echo "sources in $OUT"
    exit 0
fi

rm -rf "$WORK/ffmpeg-$FFMPEG_VERSION" "$WORK/dav1d-$DAV1D_VERSION" "$PREFIX"
tar -xJf "$CACHE/src/ffmpeg-$FFMPEG_VERSION.tar.xz" -C "$WORK"
tar -xJf "$CACHE/src/dav1d-$DAV1D_VERSION.tar.xz" -C "$WORK"

EXE=""
CROSS=()
MESON_CROSS=()
case "$TARGET" in
    linux-x86_64) ;;
    macos-arm64)
        export MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET:-11.0}
        ;;
    windows-x86_64)
        EXE=".exe"
        HOST=x86_64-w64-mingw32
        cat > "$WORK/mingw.ini" <<EOF
[binaries]
c = '$HOST-gcc'
ar = '$HOST-ar'
strip = '$HOST-strip'
windres = '$HOST-windres'
pkg-config = 'pkg-config'

[host_machine]
system = 'windows'
cpu_family = 'x86_64'
cpu = 'x86_64'
endian = 'little'
EOF
        MESON_CROSS=(--cross-file "$WORK/mingw.ini")
        CROSS=(--target-os=mingw32 --arch=x86_64 --cross-prefix="$HOST-" --pkg-config=pkg-config
            --extra-ldflags=-static)
        ;;
    *)
        echo "unknown target: $TARGET" >&2
        exit 1
        ;;
esac

# dav1d: the AV1 decoder (FFmpeg's own AV1 decoder only drives hardware).
(
    cd "$WORK/dav1d-$DAV1D_VERSION"
    meson setup build "${MESON_CROSS[@]}" --prefix="$PREFIX" --libdir=lib --buildtype=release \
        --default-library=static -Denable_tools=false -Denable_tests=false -Denable_docs=false
    ninja -C build -j "$JOBS" install
)

THREADS=--enable-pthreads
[[ "$TARGET" == windows-x86_64 ]] && THREADS=--enable-w32threads

join() { local IFS=,; echo "$*"; }
DEMUXERS=(mov matroska avi asf flv mpegps mpegts ogg mpegvideo m4v h264 hevc ivf obu wav mp3 aac flac)
VIDEO=(h264 hevc vp8 vp9 libdav1d mpeg4 mpeg1video mpeg2video msmpeg4v1 msmpeg4v2 msmpeg4v3 wmv1 wmv2
    wmv3 vc1 h263 flv mjpeg prores theora dnxhd)
AUDIO=(aac aac_latm mp3float mp2float mp1float opus vorbis flac alac ac3 eac3 dca truehd wmav1 wmav2
    wmapro pcm_s16le pcm_s16be pcm_s24le pcm_s24be pcm_s32le pcm_f32le pcm_f64le pcm_u8 pcm_alaw
    pcm_mulaw adpcm_ima_wav adpcm_ms)
PARSERS=(h264 hevc vp8 vp9 av1 mpeg4video mpegvideo aac aac_latm mpegaudio opus vorbis flac ac3 vc1
    mjpeg dca h263 dnxhd)
FILTERS=(scale format null anull aresample aformat transpose hflip vflip rotate)

CONFIGURE=(--prefix="$PREFIX" --disable-everything --disable-autodetect --disable-network
    --disable-doc --disable-debug --disable-ffplay --enable-ffmpeg --enable-ffprobe
    --disable-gpl --disable-nonfree --enable-static --disable-shared "$THREADS"
    --enable-libdav1d --enable-protocol=file,pipe
    --enable-demuxer="$(join "${DEMUXERS[@]}")"
    --enable-decoder="$(join "${VIDEO[@]}" "${AUDIO[@]}")"
    --enable-parser="$(join "${PARSERS[@]}")"
    --enable-filter="$(join "${FILTERS[@]}")"
    --enable-muxer=rawvideo,pcm_s16le,null --enable-encoder=rawvideo,pcm_s16le,wrapped_avframe
    "${CROSS[@]}")

(
    cd "$WORK/ffmpeg-$FFMPEG_VERSION"
    PKG_CONFIG_PATH="$PREFIX/lib/pkgconfig" PKG_CONFIG_LIBDIR="$PREFIX/lib/pkgconfig" \
        ./configure "${CONFIGURE[@]}" --pkg-config-flags=--static \
        --extra-cflags="-I$PREFIX/include" --extra-ldflags="-L$PREFIX/lib"
    make -j "$JOBS"
)

SRC="$WORK/ffmpeg-$FFMPEG_VERSION"
STRIP=strip
[[ "$TARGET" == windows-x86_64 ]] && STRIP="$HOST-strip"
for tool in ffmpeg ffprobe; do
    cp "$SRC/$tool$EXE" "$OUT/$tool$EXE"
    if [[ "$TARGET" == macos-arm64 ]]; then
        strip -x "$OUT/$tool$EXE"
    else
        "$STRIP" "$OUT/$tool$EXE"
    fi
done
mkdir -p "$OUT/licenses"
cp "$SRC/COPYING.LGPLv2.1" "$OUT/licenses/FFmpeg-COPYING.LGPLv2.1"
cp "$SRC/LICENSE.md" "$OUT/licenses/FFmpeg-LICENSE.md"
cp "$WORK/dav1d-$DAV1D_VERSION/COPYING" "$OUT/licenses/dav1d-COPYING"
{
    echo "FFmpeg $FFMPEG_VERSION (LGPL-2.1-or-later) built for $TARGET by scripts/ffmpeg/build.sh"
    echo "Source: https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VERSION.tar.xz"
    echo "SHA-256: $FFMPEG_SHA256"
    echo "dav1d $DAV1D_VERSION (BSD-2-Clause)"
    echo "Source: https://downloads.videolan.org/pub/videolan/dav1d/$DAV1D_VERSION/dav1d-$DAV1D_VERSION.tar.xz"
    echo "SHA-256: $DAV1D_SHA256"
    echo "Configure: ${CONFIGURE[*]//$PREFIX/<prefix>}"
} > "$OUT/BUILDINFO"
echo "FFmpeg $FFMPEG_VERSION for $TARGET in $OUT"
