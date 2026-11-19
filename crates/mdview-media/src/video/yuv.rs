//! YUV to RGBA conversion, scaling, and rotation in one pass.
//!
//! Every output axis depends on one source axis only (rotations are quarter turns), so the
//! source positions and bilinear weights are computed once per axis and frame, and the
//! per-pixel work is integer table lookups. The view never asks for more than the native
//! size, so this is a mild downscale at most. BT.709 is used from 720 lines up and BT.601
//! below, the usual choice for streams that do not say, in limited range unless the picture
//! uses the full range (JPEG). Rows are split across a few scoped threads.

use super::decode::Picture;

/// Two neighbouring source samples and the weight of the second, out of 256.
#[derive(Clone, Copy)]
struct Tap {
    first: u32,
    second: u32,
    weight: u32,
}

/// Luma and chroma taps along one output axis.
struct Axis {
    luma: Vec<Tap>,
    chroma: Vec<Tap>,
}

impl Axis {
    /// Taps for `out` output samples taken from a source axis of `source` luma samples whose
    /// chroma is subsampled by `shift`, read backwards when `flip`.
    fn new(out: usize, source: u32, shift: u32, flip: bool) -> Self {
        let chroma_source = source.div_ceil(1 << shift);
        let scale = f64::from(source) / out as f64;
        let mut luma = Vec::with_capacity(out);
        let mut chroma = Vec::with_capacity(out);
        for index in 0..out {
            let index = if flip { out - 1 - index } else { index };
            let position = (index as f64 + 0.5) * scale - 0.5;
            luma.push(tap(position, source));
            chroma.push(tap((position + 0.5) / f64::from(1 << shift) - 0.5, chroma_source));
        }
        Self { luma, chroma }
    }
}

fn tap(position: f64, length: u32) -> Tap {
    let last = f64::from(length.saturating_sub(1));
    let position = position.clamp(0.0, last);
    let first = position.floor();
    Tap {
        first: first as u32,
        second: (first as u32 + 1).min(length.saturating_sub(1)),
        weight: ((position - first) * 256.0).round() as u32,
    }
}

/// Integer coefficients (×256): R from V, G from U, G from V, B from U.
type Matrix = [i32; 4];
/// Limited range (16–235 luma, 16–240 chroma).
const BT601: Matrix = [409, -100, -208, 516];
const BT709: Matrix = [459, -55, -136, 541];
/// Full range (0–255), as JPEG uses.
const BT601_FULL: Matrix = [359, -88, -183, 454];
const BT709_FULL: Matrix = [403, -48, -120, 475];

/// Convert `picture` to straight RGBA of `size` (after rotation by `rotation` degrees
/// clockwise).
pub(crate) fn to_rgba(picture: &Picture, size: (u32, u32), rotation: u32) -> Vec<u8> {
    let (width, height) = (size.0.max(1) as usize, size.1.max(1) as usize);
    let (shift_x, shift_y) = picture.chroma_shift;
    // Which source axis each output axis walks, and in which direction.
    let (across, down) = match rotation {
        90 => (
            Axis::new(width, picture.height, shift_y, true),
            Axis::new(height, picture.width, shift_x, false),
        ),
        180 => (
            Axis::new(width, picture.width, shift_x, true),
            Axis::new(height, picture.height, shift_y, true),
        ),
        270 => (
            Axis::new(width, picture.height, shift_y, false),
            Axis::new(height, picture.width, shift_x, true),
        ),
        _ => (
            Axis::new(width, picture.width, shift_x, false),
            Axis::new(height, picture.height, shift_y, false),
        ),
    };
    let job = Job {
        picture,
        across: &across,
        down: &down,
        transposed: rotation % 180 == 90,
        matrix: match (picture.height >= 720, picture.full_range) {
            (true, false) => BT709,
            (false, false) => BT601,
            (true, true) => BT709_FULL,
            (false, true) => BT601_FULL,
        },
        // Luma black level and gain (×256) to full-range RGB.
        luma: if picture.full_range { (0, 256) } else { (16, 298) },
        depth_shift: u32::from(picture.bit_depth.saturating_sub(8)),
    };
    let mut rgba = vec![0u8; width * height * 4];
    let threads = std::thread::available_parallelism().map_or(1, |count| count.get().clamp(1, 8));
    let rows_per_chunk = height.div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        for (chunk, rows) in rgba.chunks_mut(rows_per_chunk * width * 4).enumerate() {
            let job = &job;
            scope.spawn(move || {
                for (index, row) in rows.chunks_exact_mut(width * 4).enumerate() {
                    let y = chunk * rows_per_chunk + index;
                    if job.depth_shift == 0 {
                        job.row::<false>(y, row);
                    } else {
                        job.row::<true>(y, row);
                    }
                }
            });
        }
    });
    rgba
}

struct Job<'a> {
    picture: &'a Picture,
    across: &'a Axis,
    down: &'a Axis,
    /// Output columns walk source rows (quarter-turn rotations).
    transposed: bool,
    matrix: Matrix,
    luma: (i32, i32),
    depth_shift: u32,
}

impl Job<'_> {
    fn row<const WIDE: bool>(&self, y: usize, row: &mut [u8]) {
        let [r_v, g_u, g_v, b_u] = self.matrix;
        let (fixed_luma, fixed_chroma) = (self.down.luma[y], self.down.chroma[y]);
        for (x, pixel) in row.chunks_exact_mut(4).enumerate() {
            let (moving_luma, moving_chroma) = (self.across.luma[x], self.across.chroma[x]);
            let ((column, line), (chroma_column, chroma_line)) = if self.transposed {
                ((fixed_luma, moving_luma), (fixed_chroma, moving_chroma))
            } else {
                ((moving_luma, fixed_luma), (moving_chroma, fixed_chroma))
            };
            let c = self.luma.1 * (self.sample::<WIDE>(0, column, line) - self.luma.0);
            let d = self.sample::<WIDE>(1, chroma_column, chroma_line) - 128;
            let e = self.sample::<WIDE>(2, chroma_column, chroma_line) - 128;
            pixel[0] = clamp((c + r_v * e + 128) >> 8);
            pixel[1] = clamp((c + g_u * d + g_v * e + 128) >> 8);
            pixel[2] = clamp((c + b_u * d + 128) >> 8);
            pixel[3] = 255;
        }
    }

    /// Bilinear sample of plane `index` on an 8-bit scale.
    #[inline]
    fn sample<const WIDE: bool>(&self, index: usize, column: Tap, line: Tap) -> i32 {
        let plane = &self.picture.planes[index];
        let at = |x: u32, y: u32| -> u32 {
            let offset = y as usize * plane.stride;
            if WIDE {
                let offset = offset + x as usize * 2;
                plane.data.get(offset..offset + 2).map_or(0, |bytes| {
                    u32::from(u16::from_le_bytes([bytes[0], bytes[1]])) >> self.depth_shift
                })
            } else {
                plane.data.get(offset + x as usize).copied().map_or(0, u32::from)
            }
        };
        let blend = |a: u32, b: u32, weight: u32| a * (256 - weight) + b * weight;
        let top = blend(
            at(column.first, line.first),
            at(column.second, line.first),
            column.weight,
        );
        let bottom = blend(
            at(column.first, line.second),
            at(column.second, line.second),
            column.weight,
        );
        ((blend(top, bottom, line.weight) + (1 << 15)) >> 16) as i32
    }
}

fn clamp(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

#[cfg(test)]
mod tests {
    use super::super::decode::Plane;
    use super::*;

    /// A 4×2 4:2:0 picture whose left half is limited-range white and right half black.
    fn split_picture() -> Picture {
        let luma = vec![235, 235, 16, 16, 235, 235, 16, 16];
        Picture {
            width: 4,
            height: 2,
            planes: [
                Plane { data: luma, stride: 4 },
                Plane {
                    data: vec![128, 128],
                    stride: 2,
                },
                Plane {
                    data: vec![128, 128],
                    stride: 2,
                },
            ],
            chroma_shift: (1, 1),
            bit_depth: 8,
            full_range: false,
            pts: 0.0,
        }
    }

    #[test]
    fn limited_range_grey_maps_to_full_range() {
        let rgba = to_rgba(&split_picture(), (4, 2), 0);
        assert_eq!(&rgba[..4], &[255, 255, 255, 255], "white");
        assert_eq!(&rgba[12..16], &[0, 0, 0, 255], "black");
    }

    #[test]
    fn quarter_turns_move_the_left_edge() {
        // Clockwise, the white left half becomes the top; counter-clockwise, the bottom.
        let clockwise = to_rgba(&split_picture(), (2, 4), 90);
        assert_eq!((clockwise[0], clockwise[3 * 2 * 4]), (255, 0));
        let counter = to_rgba(&split_picture(), (2, 4), 270);
        assert_eq!((counter[0], counter[3 * 2 * 4]), (0, 255));
        let upside_down = to_rgba(&split_picture(), (4, 2), 180);
        assert_eq!((upside_down[0], upside_down[3 * 4]), (0, 255));
    }

    #[test]
    fn pure_red_survives_conversion() {
        let mut picture = split_picture();
        // BT.601 limited-range red: Y 81, U 90, V 240.
        picture.planes[0].data = vec![81; 8];
        picture.planes[1].data = vec![90; 2];
        picture.planes[2].data = vec![240; 2];
        let rgba = to_rgba(&picture, (4, 2), 0);
        assert!(rgba[0] > 250 && rgba[1] < 5 && rgba[2] < 5, "{:?}", &rgba[..4]);
    }

    #[test]
    fn full_range_samples_keep_their_levels() {
        let mut picture = split_picture();
        picture.full_range = true;
        picture.planes[0].data = vec![255, 255, 0, 0, 255, 255, 0, 0];
        let rgba = to_rgba(&picture, (4, 2), 0);
        assert_eq!(&rgba[..4], &[255, 255, 255, 255], "white");
        assert_eq!(&rgba[12..16], &[0, 0, 0, 255], "black");
        picture.planes[0].data = vec![128; 8];
        assert_eq!(to_rgba(&picture, (4, 2), 0)[0], 128, "mid grey stays mid grey");
    }

    #[test]
    fn ten_bit_samples_scale_to_eight_bits() {
        let mut picture = split_picture();
        let wide = |values: &[u16]| values.iter().flat_map(|value| value.to_le_bytes()).collect::<Vec<u8>>();
        picture.planes[0] = Plane {
            data: wide(&[940; 8]),
            stride: 8,
        };
        picture.planes[1] = Plane {
            data: wide(&[512; 2]),
            stride: 4,
        };
        picture.planes[2] = Plane {
            data: wide(&[512; 2]),
            stride: 4,
        };
        picture.bit_depth = 10;
        let rgba = to_rgba(&picture, (4, 2), 0);
        assert_eq!(&rgba[..4], &[255, 255, 255, 255], "10-bit white");
    }
}
