//! Capture result types and the pure image helpers (alpha fixes, rounded corners, pointer
//! drawing), shared by every backend.

// Some helpers are only called by the Windows capture paths so far.
#![cfg_attr(not(windows), allow(dead_code))]

use stinkshot_core::{PxPoint, PxRect, Rgba};

#[derive(Clone, Debug)]
pub struct Frozen {
    /// Virtual-desktop rect that `image` covers (== virtual_bounds at freeze time).
    pub bounds: PxRect,
    pub image: Rgba,
    /// Cursor position at freeze time (virtual coords).
    pub cursor: PxPoint,
    /// The pointer image at freeze time, kept apart from `image` (which has it drawn in
    /// only when `include_cursor` was set). None when the pointer was hidden.
    pub cursor_shot: Option<CursorShot>,
    /// `cursor_shot` is drawn into `image`.
    pub cursor_drawn: bool,
}

/// Premultiplied -> straight alpha. If nothing is opaque the source did not carry alpha at all
/// (some windows report 0), so treat the image as fully opaque instead of invisible.
pub(crate) fn fix_alpha(img: &mut Rgba) {
    if img.data.chunks_exact(4).all(|p| p[3] == 0) {
        for p in img.data.chunks_exact_mut(4) {
            p[3] = 255;
        }
        return;
    }
    for p in img.data.chunks_exact_mut(4) {
        let a = p[3] as u32;
        if a != 0 && a != 255 {
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct WindowCaptureOpts {
    pub include_cursor: bool,
}

/// Make the four corners of `rect` (inside `img`) transparent with a rounded, anti-aliased edge.
pub(crate) fn apply_corner_alpha(img: &mut Rgba, rect: PxRect, radius: f32) {
    let r = radius.min(rect.w as f32 / 2.0).min(rect.h as f32 / 2.0);
    if r < 1.0 {
        return;
    }
    let n = r.ceil() as i32 + 1;
    for dy in 0..n {
        for dx in 0..n {
            // Distance of this pixel centre from the corner circle's centre.
            let (px, py) = (dx as f32 + 0.5, dy as f32 + 0.5);
            let dist = ((r - px).powi(2) + (r - py).powi(2)).sqrt();
            let cover = if px >= r || py >= r {
                1.0
            } else {
                (r - dist + 0.5).clamp(0.0, 1.0)
            };
            if cover >= 1.0 {
                continue;
            }
            for (x, y) in [
                (rect.x + dx, rect.y + dy),
                (rect.right() - 1 - dx, rect.y + dy),
                (rect.x + dx, rect.bottom() - 1 - dy),
                (rect.right() - 1 - dx, rect.bottom() - 1 - dy),
            ] {
                if x >= 0 && y >= 0 && (x as u32) < img.width && (y as u32) < img.height {
                    let mut p = img.pixel(x as u32, y as u32);
                    p[3] = (p[3] as f32 * cover).round() as u8;
                    img.set_pixel(x as u32, y as u32, p);
                }
            }
        }
    }
}

/// Recover straight-alpha RGBA from the same glyph rendered over black and over white
/// (inverting pixels become black with a white outline, see `cursor_from_renders`).
pub(crate) fn combine_black_white(black: &[u8], white: &[u8], w: u32, h: u32) -> Rgba {
    stinkshot_core::annotate::cursor_from_renders(black, white, w, h)
}

/// The pointer at the moment of a capture, apart from the screen pixels.
#[derive(Clone, Debug)]
pub struct CursorShot {
    /// Straight-alpha pointer image at the size Windows draws it.
    pub image: Rgba,
    /// Where the image's top-left sits, virtual coords (position minus hotspot).
    pub top_left: PxPoint,
    /// Hotspot inside the image.
    pub hotspot: PxPoint,
}

impl CursorShot {
    /// The image's rect, virtual coords.
    pub fn rect(&self) -> PxRect {
        PxRect::new(
            self.top_left.x,
            self.top_left.y,
            self.image.width as i32,
            self.image.height as i32,
        )
    }
}

/// How much to enlarge a `bitmap_w` px cursor bitmap on a monitor at `mon_scale`: bitmaps at
/// the 96 dpi size (`base_w`, SM_CXCURSOR at 96 dpi) get the monitor's scale, larger ones
/// (accessibility pointer sizes) are drawn as they are.
pub(crate) fn cursor_draw_scale(bitmap_w: i32, base_w: i32, mon_scale: f32) -> f32 {
    if bitmap_w <= base_w && mon_scale > 1.01 {
        mon_scale
    } else {
        1.0
    }
}

/// Straight-alpha source-over of `src` onto `dst` at (x, y).
pub(crate) fn blend_over(dst: &mut Rgba, src: &Rgba, x: i32, y: i32) {
    let Some(r) = PxRect::new(x, y, src.width as i32, src.height as i32).intersect(&dst.bounds())
    else {
        return;
    };
    for yy in r.y..r.bottom() {
        for xx in r.x..r.right() {
            let s = src.pixel((xx - x) as u32, (yy - y) as u32);
            let sa = s[3] as u32;
            if sa == 0 {
                continue;
            }
            let d = dst.pixel(xx as u32, yy as u32);
            let da = d[3] as u32;
            let out_a = sa + da * (255 - sa) / 255;
            let mut o = [0u8; 4];
            for i in 0..3 {
                let num = s[i] as u32 * sa * 255 + d[i] as u32 * da * (255 - sa);
                o[i] = if out_a == 0 {
                    0
                } else {
                    (num / (out_a * 255)).min(255) as u8
                };
            }
            o[3] = out_a.min(255) as u8;
            dst.set_pixel(xx as u32, yy as u32, o);
        }
    }
}

/// Draw a pointer taken with `cursor_shot` into `img` (top-left at `origin`, virtual coords).
pub fn draw_cursor_shot(img: &mut Rgba, origin: PxPoint, c: &CursorShot) {
    blend_over(
        img,
        &c.image,
        c.top_left.x - origin.x,
        c.top_left.y - origin.y,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_fix_unpremultiplies_and_rescues_blank() {
        let mut img = Rgba::new(2, 1);
        img.set_pixel(0, 0, [50, 25, 0, 100]);
        img.set_pixel(1, 0, [10, 10, 10, 255]);
        fix_alpha(&mut img);
        assert_eq!(img.pixel(0, 0), [128, 64, 0, 100]);
        assert_eq!(img.pixel(1, 0), [10, 10, 10, 255]);
        let mut blank = Rgba::filled(2, 1, [5, 6, 7, 0]);
        fix_alpha(&mut blank);
        assert_eq!(blank.pixel(0, 0), [5, 6, 7, 255]);
    }

    #[test]
    fn black_white_alpha_recovery() {
        // Opaque red: black pass = (0,0,255) BGRA, white pass identical.
        let black = [0u8, 0, 255, 255];
        let white = [0u8, 0, 255, 255];
        let c = combine_black_white(&black, &white, 1, 1);
        assert_eq!(c.pixel(0, 0), [255, 0, 0, 255]);
        // Fully transparent: black pass 0, white pass 255.
        let c = combine_black_white(&[0, 0, 0, 255], &[255, 255, 255, 255], 1, 1);
        assert_eq!(c.pixel(0, 0)[3], 0);
        // 50% white: black pass 128, white pass 255 (C*a = 128, +127)
        let c = combine_black_white(&[128, 128, 128, 255], &[255, 255, 255, 255], 1, 1);
        let p = c.pixel(0, 0);
        assert!((p[3] as i32 - 128).abs() <= 1 && p[0] >= 250);
    }

    #[test]
    fn corner_alpha_rounds_only_corners() {
        let mut img = Rgba::filled(40, 30, [9, 9, 9, 255]);
        apply_corner_alpha(&mut img, PxRect::new(0, 0, 40, 30), 8.0);
        assert_eq!(img.pixel(0, 0)[3], 0);
        assert_eq!(img.pixel(39, 0)[3], 0);
        assert_eq!(img.pixel(0, 29)[3], 0);
        assert_eq!(img.pixel(39, 29)[3], 0);
        // edge midpoints and the interior stay opaque
        assert_eq!(img.pixel(20, 0)[3], 255);
        assert_eq!(img.pixel(0, 15)[3], 255);
        assert_eq!(img.pixel(20, 15)[3], 255);
        // the arc is anti-aliased
        let partial = (0..8).any(|d| {
            let a = img.pixel(d, 1)[3];
            a > 0 && a < 255
        });
        assert!(partial);
    }

    #[test]
    fn blend_over_basics() {
        let mut dst = Rgba::filled(2, 2, [0, 0, 0, 255]);
        let src = Rgba::filled(1, 1, [255, 255, 255, 128]);
        blend_over(&mut dst, &src, 1, 1);
        let p = dst.pixel(1, 1);
        assert!((p[0] as i32 - 128).abs() <= 1 && p[3] == 255);
        assert_eq!(dst.pixel(0, 0), [0, 0, 0, 255]);
        // fully outside is a no-op
        blend_over(&mut dst, &src, 5, 5);
    }
}

#[cfg(test)]
mod cursor_scale_tests {
    use super::cursor_draw_scale;

    #[test]
    fn standard_cursors_follow_the_monitor_scale() {
        assert_eq!(cursor_draw_scale(32, 32, 1.5), 1.5);
        assert_eq!(cursor_draw_scale(32, 32, 1.0), 1.0);
        // Already-big accessibility pointers are drawn as they are.
        assert_eq!(cursor_draw_scale(64, 32, 1.5), 1.0);
    }
}
