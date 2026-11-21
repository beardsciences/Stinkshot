//! Decoded frame types and the crop helper shared by every decoder backend.

// `crop_bgra` is only called by the Windows decoder so far.
#![cfg_attr(not(windows), allow(dead_code))]

pub struct VideoFrame {
    pub pts_hns: i64,
    pub dur_hns: i64,
    /// BGRA top-down, alpha forced to 255.
    pub bgra: Vec<u8>,
}

pub struct AudioChunk {
    pub pts_hns: i64,
    pub dur_hns: i64,
    /// 48 kHz stereo s16le
    pub pcm: Vec<u8>,
}

/// Crop (or pad with black) a tight BGRA buffer from `sw` x `sh` to `dw` x `dh`, top-left
/// aligned. Returns the input unchanged when the sizes match.
pub fn crop_bgra(src: Vec<u8>, sw: usize, sh: usize, dw: usize, dh: usize) -> Vec<u8> {
    if (sw, sh) == (dw, dh) {
        return src;
    }
    let mut out = vec![0u8; dw * dh * 4];
    let (cw, ch) = (sw.min(dw), sh.min(dh));
    for y in 0..ch {
        out[y * dw * 4..y * dw * 4 + cw * 4].copy_from_slice(&src[y * sw * 4..y * sw * 4 + cw * 4]);
    }
    for px in out.chunks_exact_mut(4) {
        px[3] = 255;
    }
    out
}

#[cfg(test)]
mod crop_tests {
    use super::crop_bgra;

    #[test]
    fn padded_decode_is_cropped_to_the_picture() {
        // 3x2 picture decoded into 4x2 buffers (one padding column per row).
        let mut src = Vec::new();
        for y in 0..2u8 {
            for x in 0..4u8 {
                src.extend_from_slice(&[x, y, 9, 255]);
            }
        }
        let out = crop_bgra(src, 4, 2, 3, 2);
        assert_eq!(out.len(), 3 * 2 * 4);
        for y in 0..2usize {
            for x in 0..3usize {
                assert_eq!(
                    &out[(y * 3 + x) * 4..(y * 3 + x) * 4 + 2],
                    &[x as u8, y as u8]
                );
            }
        }
    }

    #[test]
    fn same_size_is_untouched() {
        let src = vec![1, 2, 3, 255, 4, 5, 6, 255];
        assert_eq!(crop_bgra(src.clone(), 2, 1, 2, 1), src);
    }
}
