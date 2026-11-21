//! OCR result types, the engine trait and the image preparation (scaling, tiling, padding)
//! shared by every backend.

// The preparation helpers are only called by the Windows engine so far.
#![cfg_attr(not(windows), allow(dead_code))]

use stinkshot_core::{PxRect, Rgba};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OcrWord {
    pub text: String,
    /// In the input image's pixel space.
    pub rect: PxRect,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OcrLine {
    pub text: String,
    /// In the input image's pixel space.
    pub rect: PxRect,
    /// The line's words in reading order (Smart Redact boxes single words, not lines).
    pub words: Vec<OcrWord>,
}

/// One recognized line of a tile: text, bounds and its words with their bounds, all as
/// (x0, y0, x1, y1) in tile pixels.
pub(crate) type TileLine = (String, [f32; 4], Vec<(String, [f32; 4])>);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OcrResult {
    /// Lines joined with '\n', in reading order.
    pub text: String,
    pub lines: Vec<OcrLine>,
}

pub trait OcrEngine: Send + Sync {
    fn name(&self) -> &str;
    fn recognize(&self, img: &Rgba) -> anyhow::Result<OcrResult>;
}

/// Images whose longest side is below this get the 2x upscale.
const UPSCALE_BELOW: u32 = 1600;
/// Windows OCR needs at least this many pixels per side.
const MIN_SIDE: u32 = 48;
/// Vertical overlap between tiles, in recognized-image pixels.
pub(crate) const TILE_OVERLAP: u32 = 96;
/// Preferred tile height; a smaller tile keeps recognition latency predictable.
pub(crate) const TILE_TARGET: u32 = 3000;

/// Composite onto white and convert to opaque BGRA.
pub(crate) fn to_opaque_bgra(img: &Rgba) -> Vec<u8> {
    let mut out = Vec::with_capacity(img.data.len());
    for p in img.data.chunks_exact(4) {
        let a = p[3] as u32;
        let f = |c: u8| -> u8 { ((c as u32 * a + 255 * (255 - a) + 127) / 255) as u8 };
        out.extend_from_slice(&[f(p[2]), f(p[1]), f(p[0]), 255]);
    }
    out
}

/// Scale (in [1/16, 2]) applied before recognition so the result fits the engine limits.
pub(crate) fn choose_scale(w: u32, h: u32, max_dim: u32) -> f32 {
    let longest = w.max(h);
    let mut s = if longest < UPSCALE_BELOW { 2.0 } else { 1.0 };
    // The width cannot be tiled, so it bounds the scale.
    if (w as f32 * s) > max_dim as f32 {
        s = max_dim as f32 / w as f32;
    }
    s.max(1.0 / 16.0)
}

/// Vertical tile spans `(start, len)` covering `h` pixels with `overlap` between tiles.
pub(crate) fn tile_spans(h: u32, tile: u32, overlap: u32) -> Vec<(u32, u32)> {
    if h <= tile {
        return vec![(0, h)];
    }
    let step = tile.saturating_sub(overlap).max(1);
    let mut spans = Vec::new();
    let mut y = 0;
    loop {
        let len = tile.min(h - y);
        spans.push((y, len));
        if y + len >= h {
            break;
        }
        y += step;
    }
    spans
}

pub(crate) fn resize(img: &Rgba, nw: u32, nh: u32) -> Rgba {
    let buf = image::RgbaImage::from_raw(img.width, img.height, img.data.clone());
    match buf {
        Some(b) => {
            let filter = if nw >= img.width {
                image::imageops::FilterType::CatmullRom
            } else {
                image::imageops::FilterType::Lanczos3
            };
            let r = image::imageops::resize(&b, nw, nh, filter);
            Rgba::from_raw(r.width(), r.height(), r.into_raw())
        }
        None => img.clone(),
    }
}

/// Pad (bottom/right) with the image's background so both sides reach `MIN_SIDE`.
pub(crate) fn pad_min(img: &Rgba) -> (Rgba, u32, u32) {
    let (nw, nh) = (img.width.max(MIN_SIDE), img.height.max(MIN_SIDE));
    if nw == img.width && nh == img.height {
        return (img.clone(), nw, nh);
    }
    let bg = if img.data.len() >= 4 {
        img.pixel(0, 0)
    } else {
        [255, 255, 255, 255]
    };
    let mut out = Rgba::filled(nw, nh, bg);
    out.blit(img, 0, 0);
    (out, nw, nh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_rules() {
        assert_eq!(choose_scale(400, 300, 10000), 2.0);
        assert_eq!(choose_scale(2000, 1000, 10000), 1.0);
        // wide image whose 2x width would exceed the engine limit stays within it
        let s = choose_scale(1500, 200, 2000);
        assert!(1500.0 * s <= 2000.0 + 0.5);
        assert!(choose_scale(40000, 100, 10000) < 1.0);
    }

    #[test]
    fn tiles_cover_and_overlap() {
        assert_eq!(tile_spans(100, 3000, 96), vec![(0, 100)]);
        let spans = tile_spans(7000, 3000, 96);
        assert_eq!(spans[0], (0, 3000));
        let (ly, ll) = *spans.last().unwrap();
        assert_eq!(ly + ll, 7000);
        for pair in spans.windows(2) {
            let (a, al) = pair[0];
            let (b, _) = pair[1];
            assert_eq!(a + al - b, 96);
        }
    }

    #[test]
    fn padding_reaches_min_side() {
        let img = Rgba::filled(10, 20, [200, 200, 200, 255]);
        let (p, w, h) = pad_min(&img);
        assert_eq!((w, h), (MIN_SIDE, MIN_SIDE));
        assert_eq!(p.pixel(47, 47), [200, 200, 200, 255]);
    }
}
